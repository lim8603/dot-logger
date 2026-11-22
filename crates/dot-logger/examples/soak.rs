//! TASK-024 long-run and power-cut harness on the facade, local-pc-v1 load.
//!
//! `run` records the representative load (256 B control records 5,000/s from
//! a 10 ms control loop, 64 KiB frames 10/s) into a ring and prints one JSON
//! line per sample. A line `alarm` on stdin stands for a UPS warning: the
//! host stops its producers and shuts the logger down within the 3 s
//! cut-off minus a 200 ms margin, then waits to be killed.
//!
//! `power` drives that external power cut: it starts `run`, sends the alarm
//! after each delay, kills the child (SIGKILL) exactly 3 s later and checks
//! every record accepted before the alarm in the files left behind.
//!
//! `verify` checks a finished `run` directory; `--ring` allows the ring to
//! have deleted the oldest records, as long as the rest is a gapless suffix.
//!
//! Measurement harness only, never a product API. RT rows set
//! `DOT_LOGGER_PERF_RT=fifo` (control thread SCHED_FIFO 70 on CPU 2, memory
//! locked), as `perf_baseline` does; the control thread is always pinned.
#[cfg(target_os = "linux")]
#[path = "support/perf/mod.rs"]
mod perf;
#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    match linux::execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("soak: {error}");
            std::process::ExitCode::from(4)
        }
    }
}
#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("soak: Linux only");
    std::process::ExitCode::from(2)
}
#[cfg(target_os = "linux")]
mod linux {
    use super::perf::{self, Error};
    use dot_logger::capture::Importance;
    use dot_logger::logger::{
        Channel, EncodeError, LogError, Logged, Logger, Preset, Record, Retention, Schema, Writer,
    };
    use dot_logger::pool::RawRecordOutcome;
    use dot_logger::storage::{LossReader, NativeReader};
    use rustix::thread::{ClockId, Timespec, clock_nanosleep_absolute};
    use serde_json::{Value, json};
    use std::{
        fs,
        io::{self, BufRead, BufReader, Write},
        path::{Path, PathBuf},
        process::{Command, Stdio},
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU64, Ordering},
            mpsc::{self, RecvTimeoutError},
        },
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    // local-pc-v1 control loop (TASK-022) and the TASK-024 representative load.
    const PERIOD_NS: u64 = 10_000_000;
    const CONTROL_WORK: Duration = Duration::from_micros(500);
    const CONTROL_CPU: usize = 2;
    const CONTROL_FIFO: u8 = 70;
    const CONTROL_PER_PERIOD: u64 = 50;
    const CONTROL_HZ: u32 = 5_000;
    const CONTROL_BYTES: usize = 256;
    const FRAME_HZ: u32 = 10;
    const FRAME_BYTES: usize = 64 << 10;
    const FRAME_PERIOD: Duration = Duration::from_millis(100);
    const CONTROL_STEP: u64 = 31;
    const FRAME_STEP: u64 = 7;
    // Power cut: signal to cut-off, and the margin the host keeps (EV-043).
    const POWEROFF: Duration = Duration::from_secs(3);
    const SAFETY: Duration = Duration::from_millis(200);
    const POWER_RING_MIB: u64 = 1024;
    const MAX_PRODUCERS: usize = 8;
    const USAGE: &str = "usage: soak run <parent> <seconds> <sample_s> <ring_mib> | \
        power <parent> <delay_s>[,<delay_s>...] | \
        verify <dir> <control_records> <frames> [--ring]";

    fn invalid(text: impl Into<String>) -> Error {
        io::Error::other(text.into()).into()
    }

    /// One control record: its index and a pattern derived from it.
    struct Control(u64);
    impl Record for Control {
        const SCHEMA: Schema = Schema::new(0x534f_414b_4354_4c00, 1, "soak-pattern");
        fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
            fill(
                self.0,
                CONTROL_STEP,
                out.get_mut(..CONTROL_BYTES).ok_or(EncodeError)?,
            );
            Ok(CONTROL_BYTES)
        }
    }
    fn fill(index: u64, step: u64, out: &mut [u8]) {
        out[..8].copy_from_slice(&index.to_le_bytes());
        for (k, byte) in out.iter_mut().enumerate().skip(8) {
            *byte = index.wrapping_mul(step).wrapping_add(k as u64) as u8;
        }
    }
    /// The index of a payload written by `fill`, or None for any other bytes.
    fn index_of(payload: &[u8], len: usize, step: u64) -> Option<u64> {
        if payload.len() != len {
            return None;
        }
        let index = u64::from_le_bytes(payload[..8].try_into().ok()?);
        let pattern = payload
            .iter()
            .enumerate()
            .skip(8)
            .all(|(k, b)| *b == index.wrapping_mul(step).wrapping_add(k as u64) as u8);
        pattern.then_some(index)
    }

    #[derive(Default)]
    struct Stats {
        control: AtomicU64,
        control_rejected: AtomicU64,
        frames: AtomicU64,
        frames_rejected: AtomicU64,
        periods: AtomicU64,
        misses: AtomicU64,
        interval_max_ns: AtomicU64,
        worst_ns: AtomicU64,
        /// Synthetic-work iterations calibrated on the control thread itself.
        iterations: AtomicU64,
    }
    impl Stats {
        fn counts(&self) -> Value {
            json!({
                "control": self.control.load(Ordering::Acquire),
                "control_rejected": self.control_rejected.load(Ordering::Acquire),
                "frames": self.frames.load(Ordering::Acquire),
                "frames_rejected": self.frames_rejected.load(Ordering::Acquire),
                "periods": self.periods.load(Ordering::Acquire),
                "misses": self.misses.load(Ordering::Acquire),
                "worst_response_us": self.worst_ns.load(Ordering::Acquire) / 1000,
                "control_iterations": self.iterations.load(Ordering::Acquire),
            })
        }
    }

    fn rt_mode() -> Result<bool, Error> {
        match std::env::var("DOT_LOGGER_PERF_RT") {
            Err(std::env::VarError::NotPresent) => Ok(false),
            Ok(v) if v == "fifo" => Ok(true),
            _ => Err(invalid("DOT_LOGGER_PERF_RT must be unset or \"fifo\"")),
        }
    }
    fn sleep_until(ns: u64) -> Result<(), Error> {
        let at = Timespec {
            tv_sec: (ns / 1_000_000_000) as _,
            tv_nsec: (ns % 1_000_000_000) as _,
        };
        loop {
            match clock_nanosleep_absolute(ClockId::Monotonic, &at) {
                Ok(()) => return Ok(()),
                Err(rustix::io::Errno::INTR) => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    fn real_ns() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
    }

    /// 10 ms releases: fixed synthetic work, then 50 control records. A
    /// response past the period, or a release skipped, is a deadline miss.
    fn control_loop(
        mut writer: Writer,
        channel: Channel<Control>,
        stats: &Stats,
        stop: &AtomicBool,
        rt: bool,
    ) -> Result<(), Error> {
        perf::pin(CONTROL_CPU)?;
        if rt {
            perf::set_fifo(CONTROL_FIFO)?;
        }
        // Calibrate where and how the loop runs: same CPU, policy and 10 ms
        // duty cycle, since frequency scaling follows the duty (perf_baseline).
        let iterations =
            perf::calibrate_periodic(CONTROL_WORK, Duration::from_nanos(PERIOD_NS), 100)?
                .iterations;
        stats.iterations.store(iterations, Ordering::Release);
        let mut index = 0_u64;
        let mut release = perf::monotonic_ns() + PERIOD_NS;
        while !stop.load(Ordering::Acquire) {
            sleep_until(release)?;
            std::hint::black_box(perf::work(iterations));
            for _ in 0..CONTROL_PER_PERIOD {
                match writer.try_log(channel, &Control(index)) {
                    Ok(Logged::Accepted(_)) => {
                        index += 1;
                        stats.control.store(index, Ordering::Release);
                    }
                    Ok(Logged::Skipped) => return Err(invalid("control record skipped")),
                    Err(LogError::Closing) => return Err(invalid("logger closed under load")),
                    Err(_) => {
                        stats.control_rejected.fetch_add(1, Ordering::AcqRel);
                    }
                }
            }
            let response = perf::monotonic_ns().saturating_sub(release);
            stats.interval_max_ns.fetch_max(response, Ordering::AcqRel);
            stats.worst_ns.fetch_max(response, Ordering::AcqRel);
            stats.periods.fetch_add(1, Ordering::AcqRel);
            if response > PERIOD_NS {
                stats.misses.fetch_add(1, Ordering::AcqRel);
            }
            release += PERIOD_NS;
            // Releases whose whole period already passed were never served.
            let now = perf::monotonic_ns();
            while release + PERIOD_NS <= now {
                stats.periods.fetch_add(1, Ordering::AcqRel);
                stats.misses.fetch_add(1, Ordering::AcqRel);
                release += PERIOD_NS;
            }
        }
        Ok(())
    }

    /// 10 frames/s as zero-copy loans.
    fn frame_loop(
        mut writer: Writer,
        channel: Channel<[u8]>,
        stats: &Stats,
        stop: &AtomicBool,
    ) -> Result<(), Error> {
        let mut index = 0_u64;
        let mut next = Instant::now();
        while !stop.load(Ordering::Acquire) {
            next += FRAME_PERIOD;
            thread::sleep(next.saturating_duration_since(Instant::now()));
            let metadata = channel.metadata(writer.now_ns(), real_ns());
            let mut producer = writer.producer();
            let Ok(mut loan) = producer.try_acquire(FRAME_BYTES, Importance::Normal) else {
                stats.frames_rejected.fetch_add(1, Ordering::AcqRel);
                continue;
            };
            fill(index, FRAME_STEP, loan.payload_mut());
            match producer.try_record_raw(metadata, loan) {
                Ok(RawRecordOutcome::Accepted(_)) => {
                    index += 1;
                    stats.frames.store(index, Ordering::Release);
                }
                Ok(RawRecordOutcome::Skipped(_)) => return Err(invalid("frame skipped")),
                Err(_) => {
                    stats.frames_rejected.fetch_add(1, Ordering::AcqRel);
                }
            }
        }
        Ok(())
    }

    fn files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                files(&path, out)?;
            } else {
                out.push(path);
            }
        }
        Ok(())
    }
    fn mcap_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut all = Vec::new();
        files(dir, &mut all)?;
        all.retain(|p| p.extension().is_some_and(|e| e == "mcap"));
        all.sort();
        Ok(all)
    }
    fn disk(dirs: &[PathBuf]) -> (u64, usize) {
        let mut all = Vec::new();
        for dir in dirs {
            let _ = files(dir, &mut all);
        }
        let bytes = all
            .iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        (bytes, all.len())
    }
    fn status_kib(field: &str) -> Option<u64> {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with(field))?;
        line[field.len()..].split_whitespace().next()?.parse().ok()
    }
    fn fds() -> Option<usize> {
        Some(fs::read_dir("/proc/self/fd").ok()?.count())
    }
    fn emit(value: Value) -> Result<(), Error> {
        let mut out = io::stdout().lock();
        writeln!(out, "{value}")?;
        out.flush()?;
        Ok(())
    }

    fn run(parent: &Path, seconds: u64, sample_s: u64, ring_mib: u64) -> Result<(), Error> {
        let rt = rt_mode()?;
        if rt {
            perf::lock_memory()?;
        }
        let logger = Logger::builder()
            .preset(Preset::sensor(FRAME_HZ, FRAME_BYTES).small(CONTROL_HZ, CONTROL_BYTES))
            .file_output(parent, Retention::ring(ring_mib << 20))
            .channel::<Control>("soak/control")
            .channel::<[u8]>("soak/frame")
            .writer("control")
            .writer("frame")
            .start()?;
        let control = logger.channel::<Control>("soak/control")?;
        let frame = logger.channel::<[u8]>("soak/frame")?;
        let control_writer = logger.take_writer("control")?;
        let frame_writer = logger.take_writer("frame")?;
        let dirs = logger.directories().to_vec();
        let stats = Arc::new(Stats::default());
        let stop = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        emit(json!({"event":"start","kernel":perf::kernel()?,
            "realtime":perf::realtime_kernel(),"rt_mode":rt,"ring_mib":ring_mib,
            "seconds":seconds,"dirs":dirs,
            "load":{"control_hz":CONTROL_HZ,"control_bytes":CONTROL_BYTES,
                "frame_hz":FRAME_HZ,"frame_bytes":FRAME_BYTES}}))?;
        let workers = [
            {
                let (stats, stop) = (stats.clone(), stop.clone());
                thread::spawn(move || control_loop(control_writer, control, &stats, &stop, rt))
            },
            {
                let (stats, stop) = (stats.clone(), stop.clone());
                thread::spawn(move || frame_loop(frame_writer, frame, &stats, &stop))
            },
        ];
        let (alarm_tx, alarm_rx) = mpsc::channel();
        thread::spawn(move || {
            for line in io::stdin().lock().lines() {
                match line {
                    Ok(line) if line.trim() == "alarm" => {
                        let _ = alarm_tx.send(Instant::now());
                    }
                    Ok(_) => {}
                    Err(_) => return,
                }
            }
        });
        let mut alarms = Some(alarm_rx);
        let end = started + Duration::from_secs(seconds);
        let sample = Duration::from_secs(sample_s.max(1));
        let mut next_sample = started + sample;
        let alarm = loop {
            let now = Instant::now();
            if now >= end {
                break None;
            }
            let wait = next_sample.min(end).saturating_duration_since(now);
            let received = match &alarms {
                Some(rx) => match rx.recv_timeout(wait) {
                    Ok(at) => Some(at),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => {
                        alarms = None;
                        continue;
                    }
                },
                None => {
                    thread::sleep(wait);
                    None
                }
            };
            if received.is_some() {
                break received;
            }
            if workers.iter().any(|w| w.is_finished()) {
                break None;
            }
            if Instant::now() >= next_sample {
                next_sample += sample;
                let (bytes, count) = disk(&dirs);
                let mut counts = stats.counts();
                counts["interval_max_response_us"] =
                    json!(stats.interval_max_ns.swap(0, Ordering::AcqRel) / 1000);
                emit(json!({"event":"sample","t_s":started.elapsed().as_secs(),
                    "rss_kib":status_kib("VmRSS:"),"hwm_kib":status_kib("VmHWM:"),
                    "fds":fds(),"disk_bytes":bytes,"files":count,
                    "health":format!("{:?}",logger.health().state),
                    "worker_failed":logger.worker_failed(),"counts":counts}))?;
            }
        };
        stop.store(true, Ordering::Release);
        let mut failures = Vec::new();
        for worker in workers {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => failures.push(e.to_string()),
                Err(_) => failures.push("worker panicked".into()),
            }
        }
        if let Some(received) = alarm {
            emit(
                json!({"event":"alarm","counts":stats.counts(),"failures":failures,
                "producers_stopped_ms":received.elapsed().as_millis()}),
            )?;
            // The rest of the cut-off budget, keeping the safety margin.
            let budget = (POWEROFF - SAFETY).saturating_sub(received.elapsed());
            let shutdown = logger
                .shutdown(budget)
                .map_err(|e| invalid(format!("shutdown: {e:?}")))?;
            emit(json!({"event":"shutdown","success":shutdown.is_success(),
                "since_alarm_ms":received.elapsed().as_millis(),"budget_ms":budget.as_millis(),
                "report":format!("{shutdown:?}")}))?;
            // Power is about to fail: wait to be killed, never exit cleanly first.
            thread::sleep(Duration::from_secs(120));
            return Err(invalid("not killed after the alarm"));
        }
        let shutdown = logger
            .shutdown(Duration::from_secs(30))
            .map_err(|e| invalid(format!("shutdown: {e:?}")))?;
        let (bytes, count) = disk(&dirs);
        emit(json!({"event":"end","success":shutdown.is_success(),
            "elapsed_s":started.elapsed().as_secs(),"counts":stats.counts(),
            "failures":failures,"disk_bytes":bytes,"files":count,
            "rss_kib":status_kib("VmRSS:"),"hwm_kib":status_kib("VmHWM:"),"fds":fds(),
            "report":format!("{shutdown:?}")}))?;
        if shutdown.is_success() && failures.is_empty() {
            Ok(())
        } else {
            Err(invalid("run did not end cleanly"))
        }
    }

    /// What the files below a directory hold, checked record by record.
    #[derive(Debug, Default)]
    struct Found {
        files: usize,
        unreadable_files: usize,
        torn_files: usize,
        control: Vec<bool>,
        frames: Vec<bool>,
        foreign: u64,
        duplicates: u64,
        corrupt_payloads: u64,
        loss_reports: u64,
        nonzero_loss_rows: u64,
    }
    /// Missing indices: oldest prefix (allowed for a ring) and gaps after it.
    fn missing(seen: &[bool]) -> (usize, usize) {
        let prefix = seen.iter().take_while(|s| !**s).count();
        let gaps = seen[prefix..].iter().filter(|s| !**s).count();
        (prefix, gaps)
    }
    fn scan(dir: &Path, control: u64, frames: u64) -> Result<Found, Error> {
        let mut found = Found {
            control: vec![false; usize::try_from(control)?],
            frames: vec![false; usize::try_from(frames)?],
            ..Found::default()
        };
        for path in mcap_files(dir)? {
            found.files += 1;
            let bytes = fs::read(&path)?;
            let reader = match NativeReader::new(&bytes, FRAME_BYTES, MAX_PRODUCERS) {
                Ok(reader) => reader,
                Err(_) => {
                    found.unreadable_files += 1;
                    continue;
                }
            };
            let topics: Vec<(u32, String)> = reader
                .channels()
                .iter()
                .map(|c| (c.channel.channel_id, c.topic.clone()))
                .collect();
            for item in reader {
                let Ok(stored) = item else {
                    found.torn_files += 1;
                    break;
                };
                let id = stored.record.metadata.channel_id;
                let topic = topics
                    .iter()
                    .find(|(c, _)| *c == id)
                    .map(|(_, t)| t.as_str());
                let (seen, len, step) = match topic {
                    Some("soak/control") => (&mut found.control, CONTROL_BYTES, CONTROL_STEP),
                    Some("soak/frame") => (&mut found.frames, FRAME_BYTES, FRAME_STEP),
                    _ => {
                        found.foreign += 1;
                        continue;
                    }
                };
                let Some(index) = index_of(stored.payload, len, step) else {
                    found.corrupt_payloads += 1;
                    continue;
                };
                match usize::try_from(index).ok().and_then(|i| seen.get_mut(i)) {
                    Some(slot) if *slot => found.duplicates += 1,
                    Some(slot) => *slot = true,
                    // Accepted after the counts were taken: not an oracle record.
                    None => found.foreign += 1,
                }
            }
            if let Ok(losses) = LossReader::new(&bytes, FRAME_BYTES, MAX_PRODUCERS) {
                for report in losses {
                    let Ok(report) = report else { break };
                    found.loss_reports += 1;
                    found.nonzero_loss_rows +=
                        report.counts.iter().filter(|c| c.count != 0).count() as u64;
                }
            }
        }
        Ok(found)
    }
    fn found_json(found: &Found) -> Value {
        let (control_prefix, control_gaps) = missing(&found.control);
        let (frame_prefix, frame_gaps) = missing(&found.frames);
        json!({"files":found.files,"unreadable_files":found.unreadable_files,
            "torn_files":found.torn_files,"foreign":found.foreign,
            "duplicates":found.duplicates,"corrupt_payloads":found.corrupt_payloads,
            "loss_reports":found.loss_reports,"nonzero_loss_rows":found.nonzero_loss_rows,
            "control":{"expected":found.control.len(),"missing_prefix":control_prefix,
                "missing_gaps":control_gaps},
            "frames":{"expected":found.frames.len(),"missing_prefix":frame_prefix,
                "missing_gaps":frame_gaps}})
    }
    /// Every check except missing records, which differ between ring and power cut.
    fn intact(found: &Found) -> bool {
        found.files > 0
            && found.unreadable_files == 0
            && found.torn_files == 0
            && found.foreign == 0
            && found.duplicates == 0
            && found.corrupt_payloads == 0
            && found.loss_reports > 0
            && found.nonzero_loss_rows == 0
    }

    fn verify(dir: &Path, control: u64, frames: u64, ring: bool) -> Result<(), Error> {
        let found = scan(dir, control, frames)?;
        let (cp, cg) = missing(&found.control);
        let (fp, fg) = missing(&found.frames);
        let complete = if ring {
            cg == 0 && fg == 0
        } else {
            cp + cg + fp + fg == 0
        };
        let pass = intact(&found) && complete;
        emit(json!({"event":"verify","dir":dir,"ring":ring,"pass":pass,
            "found":found_json(&found)}))?;
        if pass {
            Ok(())
        } else {
            Err(invalid("verification failed"))
        }
    }

    fn field(line: &Value, key: &str) -> u64 {
        line["counts"][key].as_u64().unwrap_or(u64::MAX)
    }

    /// One external power cut: alarm after `delay`, SIGKILL 3 s after the alarm.
    fn power_trial(parent: &Path, trial: usize, delay: Duration) -> Result<bool, Error> {
        let dir = parent.join(format!("power-{trial}"));
        fs::create_dir(&dir)?;
        let mut child = Command::new(std::env::current_exe()?)
            .arg("run")
            .arg(&dir)
            .args(["3600", "5", &POWER_RING_MIB.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| invalid("no child stdout"))?;
        let (lines_tx, lines_rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if lines_tx.send((Instant::now(), line)).is_err() {
                    return;
                }
            }
        });
        thread::sleep(delay);
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| invalid("no child stdin"))?;
        let sent = Instant::now();
        writeln!(stdin, "alarm")?;
        stdin.flush()?;
        thread::sleep((sent + POWEROFF).saturating_duration_since(Instant::now()));
        let killed_ms = sent.elapsed().as_millis();
        let still_running = child.try_wait()?.is_none();
        child.kill()?;
        let status = child.wait()?;
        drop(stdin);
        let _ = reader.join();
        let mut alarm = None;
        let mut shutdown = None;
        let mut samples = 0;
        for (at, line) in lines_rx.try_iter() {
            let value: Value = serde_json::from_str(&line)?;
            match value["event"].as_str() {
                Some("alarm") => alarm = Some(value),
                Some("shutdown") => {
                    shutdown = Some((at.saturating_duration_since(sent).as_millis(), value))
                }
                Some("sample") => samples += 1,
                _ => {}
            }
        }
        let alarm = alarm.ok_or_else(|| invalid("child reported no alarm"))?;
        let (control, frames) = (field(&alarm, "control"), field(&alarm, "frames"));
        let rejected = field(&alarm, "control_rejected") + field(&alarm, "frames_rejected");
        let found = scan(&dir, control, frames)?;
        let (cp, cg) = missing(&found.control);
        let (fp, fg) = missing(&found.frames);
        let shutdown_ok = shutdown
            .as_ref()
            .is_some_and(|(ms, v)| v["success"] == json!(true) && *ms < POWEROFF.as_millis());
        let pass = still_running
            && shutdown_ok
            && rejected == 0
            && alarm["failures"] == json!([])
            && intact(&found)
            && cp + cg + fp + fg == 0;
        emit(
            json!({"event":"power_trial","trial":trial,"delay_s":delay.as_secs(),
            "pass":pass,"killed_after_alarm_ms":killed_ms,"alive_at_kill":still_running,
            "exit":format!("{status:?}"),"samples":samples,
            "alarm":alarm,"shutdown_seen_ms":shutdown.as_ref().map(|(ms, _)| *ms),
            "shutdown":shutdown.map(|(_, v)| v),"found":found_json(&found)}),
        )?;
        if pass {
            fs::remove_dir_all(&dir)?;
        }
        Ok(pass)
    }

    fn power(parent: &Path, delays: &str) -> Result<(), Error> {
        let delays: Vec<u64> = delays
            .split(',')
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        let mut passed = 0;
        for (trial, delay) in delays.iter().enumerate() {
            if power_trial(parent, trial, Duration::from_secs(*delay))? {
                passed += 1;
            }
        }
        emit(json!({"event":"power_summary","trials":delays.len(),"passed":passed}))?;
        if passed == delays.len() {
            Ok(())
        } else {
            Err(invalid("power-cut trials failed"))
        }
    }

    pub fn execute() -> Result<(), Error> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        match a.as_slice() {
            ["run", parent, seconds, sample, ring] => run(
                Path::new(parent),
                seconds.parse()?,
                sample.parse()?,
                ring.parse()?,
            ),
            ["power", parent, delays] => power(Path::new(parent), delays),
            ["verify", dir, control, frames] => {
                verify(Path::new(dir), control.parse()?, frames.parse()?, false)
            }
            ["verify", dir, control, frames, "--ring"] => {
                verify(Path::new(dir), control.parse()?, frames.parse()?, true)
            }
            _ => Err(invalid(USAGE)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn payload_patterns_round_trip_and_reject_other_bytes() {
            let mut control = [0_u8; CONTROL_BYTES];
            fill(42, CONTROL_STEP, &mut control);
            assert_eq!(index_of(&control, CONTROL_BYTES, CONTROL_STEP), Some(42));
            assert_eq!(index_of(&control, CONTROL_BYTES, FRAME_STEP), None);
            control[100] ^= 1;
            assert_eq!(index_of(&control, CONTROL_BYTES, CONTROL_STEP), None);
            assert_eq!(index_of(&control[..10], CONTROL_BYTES, CONTROL_STEP), None);
        }

        #[test]
        fn missing_splits_a_ring_prefix_from_gaps() {
            assert_eq!(missing(&[false, false, true, true]), (2, 0));
            assert_eq!(missing(&[true, false, true]), (0, 1));
            assert_eq!(missing(&[false, true, false]), (1, 1));
            assert_eq!(missing(&[]), (0, 0));
        }
    }
}
