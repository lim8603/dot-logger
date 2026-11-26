//! TASK-042: `Writer::try_log` latency distribution rows. Every row times the
//! call alone with two CLOCK_MONOTONIC reads around it; the cost of such a
//! clock pair is measured on the same thread and reported next to it.
//!
//! | Row | What is timed |
//! |---|---|
//! | H  | warm back-to-back calls, in bursts the worker drains between |
//! | C1 | one call after sweeping a buffer twice the last-level cache |
//! | C2 | one call per 1 kHz release after touching a buffer (other work) |
//! | R  | rejected calls (`Capacity`) and level-filtered calls (`Skipped`) |
//! | J  | control loop wake-up and response with 0 or N calls per release |
//! | M  | H on several writers on different CPUs at once |
//!
//! Records go to a tmpfs ring, checked from the writing process before the
//! logger starts, so the rows write nothing to an SSD. Only J's optional IO
//! load writes large records to a second output the caller chooses.
use super::facade::Seq;
use super::{
    Error, LatencyHistogram, available_bytes, calibrate, invalid, now_ns, pin, set_fifo,
    sleep_until, work,
};
use dot_logger::logger::{
    Budget, Channel, ChannelOptions, Level, LogError, Logged, Logger, Retention, Shutdown, Writer,
    WriterQuota,
};
use std::{
    hint::black_box,
    path::Path,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const MIB: u64 = 1024 * 1024;
/// tmpfs `statfs` magic (linux/magic.h).
const TMPFS_MAGIC: i64 = 0x0102_1994;
/// Calls per H burst; below the writer's slots so a drained ring never
/// rejects inside a burst.
const BURST: u64 = 1024;
/// Pause after a burst: the worker drains the ring meanwhile.
const DRAIN: Duration = Duration::from_micros(500);
const SLOTS: usize = 4096;
const SHUTDOWN: Duration = Duration::from_secs(120);

/// Output of a row: a tmpfs directory and the ring that bounds its use.
#[derive(Clone, Copy, Debug)]
pub struct Target<'a> {
    /// Logger output directory.
    pub dir: &'a Path,
    /// `Retention::ring` bytes.
    pub ring: u64,
}
/// Refuse anything but a tmpfs directory with room for twice the ring,
/// checked from the process that will write it (a mistaken path once filled
/// a host SSD, 2026-11-23).
pub fn require_tmpfs(target: Target<'_>) -> Result<(), Error> {
    let dir = target.dir;
    let s = rustix::fs::statfs(dir)?;
    // `f_type` is i64 on 64-bit Linux and i32 on 32-bit targets.
    #[allow(clippy::useless_conversion)]
    let f_type = i64::from(s.f_type);
    if f_type != TMPFS_MAGIC {
        return Err(format!("{} is not tmpfs", dir.display()).into());
    }
    let needed = target.ring.saturating_mul(2);
    if available_bytes(dir)? < needed {
        return Err(format!("{} has less than {} MiB free", dir.display(), needed / MIB).into());
    }
    Ok(())
}

/// One measured row.
#[derive(Clone, Debug, Default)]
pub struct Row {
    /// Accepted calls.
    pub accepted: LatencyHistogram,
    /// Calls rejected for `Capacity` (timed in R, only counted elsewhere).
    pub rejected: LatencyHistogram,
    /// Calls the channel level filtered (`Skipped`).
    pub skipped: LatencyHistogram,
    /// Capacity rejections outside R (retried after the drain pause).
    pub capacity: u64,
    /// Two back-to-back clock reads on the same thread.
    pub clock: LatencyHistogram,
    /// Shutdown report was a success.
    pub success: bool,
}

/// Thread setup of a measuring thread: pinned, and SCHED_FIFO in RT rows.
#[derive(Clone, Copy, Debug)]
pub struct Rt {
    /// CPU of the (first) measuring thread.
    pub cpu: usize,
    /// SCHED_FIFO priority, or the inherited policy.
    pub fifo: Option<u8>,
}
impl Rt {
    fn enter(self, cpu: usize) -> Result<(), Error> {
        pin(cpu)?;
        if let Some(priority) = self.fifo {
            set_fifo(priority)?;
        }
        Ok(())
    }
}

struct Session {
    logger: Logger,
    channel: Channel<Seq>,
    writers: Vec<Writer>,
}
fn session(
    target: Target<'_>,
    payload: usize,
    slots: usize,
    writers: usize,
    minimum: Option<Level>,
) -> Result<Session, Error> {
    require_tmpfs(target)?;
    let options = minimum.map_or_else(ChannelOptions::new, |level| {
        ChannelOptions::new().minimum(level)
    });
    let mut builder = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB as usize * writers.max(1))
                .sync_every(Duration::from_secs(1))
                .drop_deadline(Duration::from_secs(30))
                .writer_quota(WriterQuota::small(slots, payload)),
        )
        .file_output(target.dir, Retention::ring(target.ring))
        .channel_with::<Seq>("latency/seq", options);
    for w in 0..writers {
        builder = builder.writer(&format!("w{w}"));
    }
    let logger = builder.start()?;
    let channel = logger
        .channel::<Seq>("latency/seq")
        .map_err(|e| format!("channel lookup: {e:?}"))?;
    let writers = (0..writers)
        .map(|w| {
            logger
                .take_writer(&format!("w{w}"))
                .map_err(|e| format!("writer lookup: {e:?}").into())
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Session {
        logger,
        channel,
        writers,
    })
}
fn close(session: Session, row: &mut Row) -> Result<(), Error> {
    drop(session.writers);
    match session.logger.shutdown(SHUTDOWN) {
        Ok(Shutdown::Closed(report)) => {
            row.success = report.is_success();
            Ok(())
        }
        other => Err(format!("shutdown: {other:?}").into()),
    }
}

/// One timed call; the outcome decides the histogram.
fn timed(
    writer: &mut Writer,
    channel: Channel<Seq>,
    seq: u64,
    row: &mut Row,
) -> Result<bool, Error> {
    let t0 = now_ns();
    let result = writer.try_log(channel, &Seq(seq));
    let spent = now_ns().saturating_sub(t0);
    match result {
        Ok(Logged::Accepted(_)) => row.accepted.record(spent),
        Ok(Logged::Skipped) => row.skipped.record(spent),
        Err(LogError::Capacity) => {
            row.rejected.record(spent);
            return Ok(false);
        }
        Err(e) => return Err(format!("try_log: {e:?}").into()),
    }
    Ok(true)
}
fn clock_pairs(row: &mut Row, n: u64) {
    for _ in 0..n {
        let t0 = now_ns();
        row.clock.record(now_ns().saturating_sub(t0));
    }
}
/// Accepted calls in bursts with drain pauses; a rejected call is counted,
/// not timed, and retried after the pause.
fn warm_loop(
    writer: &mut Writer,
    channel: Channel<Seq>,
    calls: u64,
    row: &mut Row,
) -> Result<(), Error> {
    let mut seq = 0;
    while seq < calls {
        let end = (seq + BURST).min(calls);
        while seq < end {
            if timed(writer, channel, seq, row)? {
                seq += 1;
            } else {
                row.capacity += 1;
                break;
            }
        }
        std::thread::sleep(DRAIN);
    }
    // Rejections were only counted here.
    row.rejected = LatencyHistogram::default();
    Ok(())
}

/// Row H: `calls` warm calls of `payload` bytes.
pub fn warm(target: Target<'_>, payload: usize, calls: u64, rt: Rt) -> Result<Row, Error> {
    let mut s = session(target, payload, SLOTS, 1, None)?;
    let mut row = Row::default();
    let channel = s.channel;
    let mut writer = s.writers.pop().ok_or(invalid("no writer"))?;
    let result = std::thread::scope(|scope| {
        scope
            .spawn(|| -> Result<(), Error> {
                rt.enter(rt.cpu)?;
                warm_loop(&mut writer, channel, calls, &mut row)?;
                clock_pairs(&mut row, calls);
                Ok(())
            })
            .join()
            .map_err(|_| "warm row panicked")?
    });
    s.writers.push(writer);
    result?;
    close(s, &mut row)?;
    Ok(row)
}

/// Last-level cache of `cpu` from sysfs, in bytes.
pub fn llc_bytes(cpu: usize) -> Result<usize, Error> {
    let mut largest = 0;
    for index in 0..8 {
        let base = format!("/sys/devices/system/cpu/cpu{cpu}/cache/index{index}");
        let Ok(size) = std::fs::read_to_string(format!("{base}/size")) else {
            break;
        };
        let size = size.trim();
        let bytes = match size.strip_suffix('K') {
            Some(k) => k.parse::<usize>()? * 1024,
            None => match size.strip_suffix('M') {
                Some(m) => m.parse::<usize>()? * 1024 * 1024,
                None => size.parse()?,
            },
        };
        largest = largest.max(bytes);
    }
    if largest == 0 {
        return Err(invalid("no cache size in sysfs"));
    }
    Ok(largest)
}
/// Write one byte per 64-byte line of `buffer`.
fn sweep(buffer: &mut [u8]) {
    for line in buffer.chunks_mut(64) {
        line[0] = line[0].wrapping_add(1);
    }
    black_box(buffer);
}

/// Row C1: `calls` calls, each after sweeping `2 × LLC` bytes, so the call's
/// code, the writer state and its ring slots come from memory.
pub fn cold(target: Target<'_>, payload: usize, calls: u64, rt: Rt) -> Result<(Row, usize), Error> {
    let swept = 2 * llc_bytes(rt.cpu)?;
    let mut s = session(target, payload, SLOTS, 1, None)?;
    let mut row = Row::default();
    let channel = s.channel;
    let mut writer = s.writers.pop().ok_or(invalid("no writer"))?;
    let mut buffer = vec![0_u8; swept];
    let result = std::thread::scope(|scope| {
        scope
            .spawn(|| -> Result<(), Error> {
                rt.enter(rt.cpu)?;
                let mut seq = 0;
                while seq < calls {
                    sweep(&mut buffer);
                    if timed(&mut writer, channel, seq, &mut row)? {
                        seq += 1;
                    } else {
                        row.capacity += 1;
                    }
                }
                row.rejected = LatencyHistogram::default();
                clock_pairs(&mut row, calls);
                Ok(())
            })
            .join()
            .map_err(|_| "cold row panicked")?
    });
    s.writers.push(writer);
    result?;
    close(s, &mut row)?;
    Ok((row, swept))
}

/// Row C2: one call per release of a `period` loop for `releases` releases,
/// after touching `between` bytes as the loop's other work.
pub fn sparse(
    target: Target<'_>,
    payload: usize,
    period: Duration,
    releases: u64,
    between: usize,
    rt: Rt,
) -> Result<Row, Error> {
    let mut s = session(target, payload, SLOTS, 1, None)?;
    let mut row = Row::default();
    let channel = s.channel;
    let mut writer = s.writers.pop().ok_or(invalid("no writer"))?;
    let mut buffer = vec![0_u8; between];
    let period = u64::try_from(period.as_nanos())?;
    let result = std::thread::scope(|scope| {
        scope
            .spawn(|| -> Result<(), Error> {
                rt.enter(rt.cpu)?;
                let origin = now_ns() + period;
                for k in 0..releases {
                    sleep_until(origin + k * period)?;
                    sweep(&mut buffer);
                    if !timed(&mut writer, channel, k, &mut row)? {
                        row.capacity += 1;
                    }
                }
                row.rejected = LatencyHistogram::default();
                clock_pairs(&mut row, releases);
                Ok(())
            })
            .join()
            .map_err(|_| "sparse row panicked")?
    });
    s.writers.push(writer);
    result?;
    close(s, &mut row)?;
    Ok(row)
}

/// Row R: `calls` level-filtered calls, then calls into a 64-slot ring
/// without pausing until `calls` of them were rejected for `Capacity`.
pub fn reject(target: Target<'_>, payload: usize, calls: u64, rt: Rt) -> Result<Row, Error> {
    let mut row = Row::default();
    for filtered in [true, false] {
        let minimum = filtered.then_some(Level::Error);
        let mut s = session(target, payload, 64, 1, minimum)?;
        let channel = s.channel;
        let mut writer = s.writers.pop().ok_or(invalid("no writer"))?;
        let result = std::thread::scope(|scope| {
            scope
                .spawn(|| -> Result<(), Error> {
                    rt.enter(rt.cpu)?;
                    let mut seq = 0;
                    if filtered {
                        while row.skipped.count() < calls {
                            timed(&mut writer, channel, seq, &mut row)?;
                            seq += 1;
                        }
                    } else {
                        while row.rejected.count() < calls {
                            timed(&mut writer, channel, seq, &mut row)?;
                            seq += 1;
                        }
                    }
                    Ok(())
                })
                .join()
                .map_err(|_| "reject row panicked")?
        });
        s.writers.push(writer);
        result?;
        close(s, &mut row)?;
    }
    clock_pairs(&mut row, calls);
    Ok(row)
}

/// Row J: a control loop of `releases` releases with `work` of fixed
/// computation and `per_release` calls after it. The IO load, when given,
/// writes `load_bytes` records `load_per_second` times a second from an
/// unpinned SCHED_OTHER thread to a second output in `load_dir`.
pub struct Jitter {
    /// Release to wake-up.
    pub wake: LatencyHistogram,
    /// Release to completion (work and calls).
    pub response: LatencyHistogram,
    /// The release's calls together.
    pub calls: LatencyHistogram,
    /// Completions after the period.
    pub misses: u64,
    /// Capacity rejections (not retried: an RT loop moves on).
    pub capacity: u64,
    /// Calibrated work iterations.
    pub work_iterations: u64,
    /// Load records written.
    pub load_records: u64,
    /// Batches slower than the `slow` threshold, when one was given.
    pub slow: Vec<SlowBatch>,
    /// Slow batches beyond the preallocated capacity (counted only).
    pub slow_dropped: u64,
    /// Shutdown report was a success.
    pub success: bool,
}
/// A J batch slower than the diagnostic threshold: when it ran and whether
/// one call or many made it slow.
#[derive(Clone, Copy, Debug, Default)]
pub struct SlowBatch {
    /// Release index.
    pub release: u64,
    /// CLOCK_MONOTONIC at the batch start.
    pub at_ns: u64,
    /// Release to wake-up of this cycle.
    pub wake_ns: u64,
    /// The cycle's fixed computation (no memory traffic): slower too when
    /// the whole core was slow, unchanged when only the calls were.
    pub work_ns: u64,
    /// Sequence of the batch's first call.
    pub seq: u64,
    /// The whole batch.
    pub batch_ns: u64,
    /// Slowest single call.
    pub max_call_ns: u64,
    /// Position of the slowest call in the batch.
    pub max_call_index: u64,
    /// Calls over 2 µs.
    pub calls_over_2us: u64,
}
/// Most slow batches one J row keeps; more are only counted.
const SLOW_CAPACITY: usize = 4096;
/// IO load of row J.
#[derive(Clone, Copy, Debug)]
pub struct IoLoad<'a> {
    /// Output directory of the load (an SSD when IO is meant).
    pub dir: &'a Path,
    /// Bytes per load record.
    pub bytes: usize,
    /// Load records per second.
    pub per_second: u64,
}
#[allow(clippy::too_many_arguments)]
pub fn jitter(
    target: Target<'_>,
    payload: usize,
    period: Duration,
    releases: u64,
    work_time: Duration,
    per_release: u64,
    load: Option<IoLoad<'_>>,
    slow: Option<Duration>,
    rt: Rt,
) -> Result<Jitter, Error> {
    let slow_ns = slow.map(|d| u64::try_from(d.as_nanos())).transpose()?;
    require_tmpfs(target)?;
    let options = ChannelOptions::new().outputs(&[0]);
    let mut builder = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(256 * MIB as usize)
                .sync_every(Duration::from_secs(1))
                .drop_deadline(Duration::from_secs(30))
                .writer_quota(WriterQuota::small(SLOTS, payload)),
        )
        .file_output(target.dir, Retention::ring(target.ring))
        .channel_with::<Seq>("latency/seq", options)
        .writer("control");
    if let Some(l) = load {
        builder = builder
            .file_output(l.dir, Retention::ring(target.ring))
            .channel_with::<[u8]>("latency/load", ChannelOptions::new().outputs(&[1]))
            .writer_with("load", WriterQuota::small(16, l.bytes));
    }
    let logger = builder.start()?;
    let channel = logger
        .channel::<Seq>("latency/seq")
        .map_err(|e| format!("channel lookup: {e:?}"))?;
    let mut control = logger
        .take_writer("control")
        .map_err(|e| format!("writer lookup: {e:?}"))?;
    let stop = AtomicBool::new(false);
    let mut j = Jitter {
        wake: LatencyHistogram::default(),
        response: LatencyHistogram::default(),
        calls: LatencyHistogram::default(),
        misses: 0,
        capacity: 0,
        work_iterations: 0,
        load_records: 0,
        slow: Vec::with_capacity(if slow_ns.is_some() { SLOW_CAPACITY } else { 0 }),
        slow_dropped: 0,
        success: false,
    };
    let period_ns = u64::try_from(period.as_nanos())?;
    let j_ref = &mut j;
    let result = std::thread::scope(|scope| -> Result<u64, Error> {
        let loader = match load {
            Some(l) => {
                let lane = logger
                    .channel::<[u8]>("latency/load")
                    .map_err(|e| format!("channel lookup: {e:?}"))?;
                let mut w = logger
                    .take_writer("load")
                    .map_err(|e| format!("writer lookup: {e:?}"))?;
                let stop = &stop;
                Some(scope.spawn(move || -> Result<u64, Error> {
                    let frame = vec![7_u8; l.bytes];
                    let every = 1_000_000_000 / l.per_second.max(1);
                    let mut next = now_ns();
                    let mut written = 0;
                    while !stop.load(Ordering::Relaxed) {
                        next += every;
                        sleep_until(next)?;
                        match w.try_log(lane, &frame[..]) {
                            Ok(_) => written += 1,
                            Err(LogError::Capacity) => {}
                            Err(e) => return Err(format!("load: {e:?}").into()),
                        }
                    }
                    Ok(written)
                }))
            }
            None => None,
        };
        let j = j_ref;
        let control = &mut control;
        let measured = scope
            .spawn(move || -> Result<(), Error> {
                rt.enter(rt.cpu)?;
                j.work_iterations = calibrate(work_time);
                let origin = now_ns() + period_ns;
                let mut seq = 0;
                for k in 0..releases {
                    let release = origin + k * period_ns;
                    sleep_until(release)?;
                    let wake = now_ns();
                    work(j.work_iterations);
                    let t0 = now_ns();
                    let first = seq;
                    // Diagnostic mode times each call too (one extra clock
                    // read per call); the slowest call says whether one stall
                    // or a general slowdown made a batch slow.
                    let (mut max_call, mut max_index, mut over) = (0, 0, 0);
                    let mut last = t0;
                    for i in 0..per_release {
                        match control.try_log(channel, &Seq(seq)) {
                            Ok(_) => {}
                            Err(LogError::Capacity) => j.capacity += 1,
                            Err(e) => return Err(format!("try_log: {e:?}").into()),
                        }
                        seq += 1;
                        if slow_ns.is_some() {
                            let now = now_ns();
                            let call = now - last;
                            last = now;
                            if call > max_call {
                                (max_call, max_index) = (call, i);
                            }
                            over += u64::from(call > 2_000);
                        }
                    }
                    let done = now_ns();
                    if per_release > 0 {
                        j.calls.record(done - t0);
                    }
                    // No let chain: the MSRV is 1.85.
                    if slow_ns.is_some_and(|limit| per_release > 0 && done - t0 > limit) {
                        let b = SlowBatch {
                            release: k,
                            at_ns: t0,
                            wake_ns: wake.saturating_sub(release),
                            work_ns: t0 - wake,
                            seq: first,
                            batch_ns: done - t0,
                            max_call_ns: max_call,
                            max_call_index: max_index,
                            calls_over_2us: over,
                        };
                        // Never grows past the preallocation inside the loop.
                        if j.slow.len() < SLOW_CAPACITY {
                            j.slow.push(b);
                        } else {
                            j.slow_dropped += 1;
                        }
                    }
                    j.wake.record(wake.saturating_sub(release));
                    j.response.record(done.saturating_sub(release));
                    if done > release + period_ns {
                        j.misses += 1;
                    }
                }
                Ok(())
            })
            .join()
            .map_err(|_| "jitter row panicked")?;
        stop.store(true, Ordering::Relaxed);
        let written = match loader {
            Some(loader) => loader.join().map_err(|_| "load panicked")??,
            None => 0,
        };
        measured.map(|()| written)
    });
    drop(control);
    j.load_records = result?;
    match logger.shutdown(SHUTDOWN) {
        Ok(Shutdown::Closed(report)) => j.success = report.is_success(),
        other => return Err(format!("shutdown: {other:?}").into()),
    }
    Ok(j)
}

/// Row M: H on `cpus.len()` writers, writer `i` pinned to `cpus[i]`, all
/// released together. Returns every writer's row.
pub fn multi(
    target: Target<'_>,
    payload: usize,
    calls: u64,
    cpus: &[usize],
    rt: Rt,
) -> Result<Vec<Row>, Error> {
    let mut s = session(target, payload, SLOTS, cpus.len(), None)?;
    let channel = s.channel;
    let start = Arc::new(Barrier::new(cpus.len()));
    let rows = std::thread::scope(|scope| {
        let handles: Vec<_> = s
            .writers
            .iter_mut()
            .zip(cpus)
            .map(|(writer, &cpu)| {
                let start = start.clone();
                scope.spawn(move || -> Result<Row, Error> {
                    rt.enter(cpu)?;
                    let mut row = Row::default();
                    start.wait();
                    warm_loop(writer, channel, calls, &mut row)?;
                    clock_pairs(&mut row, calls);
                    Ok(row)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().map_err(|_| "multi row panicked")?)
            .collect::<Result<Vec<_>, Error>>()
    })?;
    let mut total = Row::default();
    close(s, &mut total)?;
    Ok(rows
        .into_iter()
        .map(|mut r| {
            r.success = total.success;
            r
        })
        .collect())
}
