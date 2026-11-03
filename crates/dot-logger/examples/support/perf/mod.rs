//! TASK-023 P0 baseline: result judgment, simple-file writer and synthetic
//! fixed-work control loop. Measurement harness only, never a product API.
#![allow(dead_code)]
use rustix::{
    thread::{
        ClockId, CpuSet, Timespec, clock_nanosleep_absolute, sched_getcpu, sched_setaffinity,
    },
    time::clock_gettime,
};
use std::{
    fs::{self, File, OpenOptions},
    hint::black_box,
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};
/// Ordinary harness error; no measurement result is implied.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
fn invalid(text: &'static str) -> Error {
    io::Error::other(text).into()
}
/// Simple-file framing per record: sequence and payload length, both LE u64.
pub const FRAME_HEADER: usize = 16;
/// Independent repetitions per off/on condition (local-pc-v1).
pub const REPETITIONS: usize = 3;

/// Which writer produced a storage run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Paired simple-file Rust writer (P0).
    P0SimpleFile,
    /// dot-logger under the same workload.
    Logger,
}
/// Every condition a logger run must share with its P0 baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageWorkload {
    /// Payload of every record; framing is reported separately.
    pub record_payload_bytes: usize,
    /// Records offered over the run.
    pub records: u64,
    /// Concurrently open output files; record `s` goes to file `s % files`.
    pub files: usize,
    /// Writer threads; file `f` belongs to writer `f % writers`.
    pub writers: usize,
    /// Rotate before a file would exceed this many bytes.
    pub file_limit_bytes: u64,
    /// Write request size; sync and rotation flush shorter tails.
    pub block_bytes: usize,
    /// Periodic flush + fdatasync of every file a writer owns.
    pub sync_interval: Duration,
    /// Paced input rate, or `None` for the device's unpaced capability.
    pub offered_bytes_per_second: Option<u64>,
}
/// Why admission stopped before the offered records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Free space would fall below the declared reserve.
    Reserve,
}
/// Facts of one storage run. Times are from the run start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageRun {
    /// Producing writer.
    pub tool: Tool,
    /// Declared workload.
    pub workload: StorageWorkload,
    /// Payload bytes actually admitted.
    pub payload_bytes: u64,
    /// Encoding/framing bytes written in addition to payload.
    pub framing_bytes: u64,
    /// Files created, including rotations.
    pub files_created: u64,
    /// Write requests (`write_all` calls); the kernel may split them further.
    pub write_calls: u64,
    /// Largest write request.
    pub largest_write: usize,
    /// Smallest write request.
    pub smallest_write: usize,
    /// Periodic sync rounds over all writers.
    pub periodic_syncs: u64,
    /// Last record handed to a writer in memory. Never a throughput denominator.
    pub accepted_ns: u64,
    /// Final file and directory sync completed; `None` if never reached.
    pub durable_ns: Option<u64>,
    /// Admission stopped early.
    pub stopped: Option<Stop>,
}
/// A result that must not be reported as the requested figure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// Throughput without the final file/directory sync (RAM acceptance only).
    NoFinalSync,
    /// Admission stopped before the offered workload.
    Incomplete,
    /// Durable time of zero cannot form a rate.
    ZeroDuration,
    /// A logger ratio without any P0 baseline.
    MissingBaseline,
    /// The baseline was not produced by the P0 writer.
    BaselineNotP0,
    /// Files, rotation, blocks, sync or offered rate differ.
    WorkloadMismatch,
    /// Different admitted payload bytes.
    PayloadMismatch,
    /// Fewer than three runs per condition, or unpaired counts.
    InsufficientRepetitions,
    /// Control runs differ in CPU, period, deadline, work, kernel or length.
    ConditionMismatch,
}
/// Payload bytes over the time to durability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Throughput {
    /// Admitted payload.
    pub payload_bytes: u64,
    /// Start to final file and directory sync.
    pub durable_ns: u64,
}
impl Throughput {
    /// Payload bytes per second to durability.
    pub fn bytes_per_second(&self) -> f64 {
        self.payload_bytes as f64 * 1e9 / self.durable_ns as f64
    }
}
/// Throughput up to the final file/directory sync; RAM acceptance alone and
/// truncated runs are rejected.
pub fn throughput(run: &StorageRun) -> Result<Throughput, Rejection> {
    if run.stopped.is_some() {
        return Err(Rejection::Incomplete);
    }
    let durable_ns = run.durable_ns.ok_or(Rejection::NoFinalSync)?;
    if durable_ns == 0 {
        return Err(Rejection::ZeroDuration);
    }
    Ok(Throughput {
        payload_bytes: run.payload_bytes,
        durable_ns,
    })
}
/// Logger throughput relative to a P0 baseline of the same workload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Efficiency {
    /// Logger run.
    pub logger: Throughput,
    /// Matching P0 run.
    pub baseline: Throughput,
}
impl Efficiency {
    /// Logger durable rate as a percentage of the P0 durable rate.
    pub fn percent(&self) -> f64 {
        100.0 * self.logger.bytes_per_second() / self.baseline.bytes_per_second()
    }
}
/// A ratio exists only against a P0 run with the identical workload.
pub fn efficiency(
    logger: &StorageRun,
    baseline: Option<&StorageRun>,
) -> Result<Efficiency, Rejection> {
    let baseline = baseline.ok_or(Rejection::MissingBaseline)?;
    if baseline.tool != Tool::P0SimpleFile {
        return Err(Rejection::BaselineNotP0);
    }
    if logger.workload != baseline.workload {
        return Err(Rejection::WorkloadMismatch);
    }
    if logger.payload_bytes != baseline.payload_bytes {
        return Err(Rejection::PayloadMismatch);
    }
    Ok(Efficiency {
        logger: throughput(logger)?,
        baseline: throughput(baseline)?,
    })
}

/// Synthetic periodic task; work is a calibrated iteration count, not time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlConfig {
    /// Pinned CPU, or unpinned.
    pub cpu: Option<usize>,
    /// Release period on the monotonic clock.
    pub period: Duration,
    /// Completion deadline relative to each release.
    pub deadline: Duration,
    /// Fixed work per release, calibrated once with the logger off.
    pub work_iterations: u64,
}
/// Exact maximum plus power-of-two microsecond buckets: bucket 0 holds
/// < 1 µs and bucket k holds [2^(k-1), 2^k) µs; the last bucket is open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Histogram {
    /// Recorded samples.
    pub count: u64,
    /// Largest sample.
    pub max_ns: u64,
    /// Sample counts per bucket.
    pub buckets: [u64; 32],
}
impl Histogram {
    /// Add one sample.
    pub fn record(&mut self, ns: u64) {
        let us = ns / 1000;
        let bucket = if us == 0 {
            0
        } else {
            (64 - us.leading_zeros() as usize).min(31)
        };
        self.buckets[bucket] += 1;
        self.count += 1;
        self.max_ns = self.max_ns.max(ns);
    }
}
/// Facts of one control run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlRun {
    /// Loop configuration.
    pub config: ControlConfig,
    /// Running kernel release.
    pub kernel: String,
    /// Nominal releases in the run.
    pub releases: u64,
    /// Releases completed by their deadline.
    pub completed: u64,
    /// Late completions plus skipped releases.
    pub misses: u64,
    /// Releases whose deadline passed before they could start.
    pub skipped: u64,
    /// Executed releases observed on another CPU than the pinned one.
    pub off_cpu: u64,
    /// Release to wake-up latency of executed releases.
    pub wake: Histogram,
    /// Release to completion of executed releases.
    pub response: Histogram,
}
/// Outcome of one executed release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cycle {
    /// Completed after its deadline.
    pub missed: bool,
    /// Later releases whose deadline had already passed.
    pub skipped: u64,
    /// Next release to execute.
    pub next: u64,
}
/// Account release `index` completing at `done_ns` (from the release origin).
/// Releases whose deadline is not after `done_ns` are skipped and missed.
pub fn account(index: u64, period_ns: u64, deadline_ns: u64, done_ns: u64) -> Cycle {
    let missed = done_ns > index * period_ns + deadline_ns;
    // First release k with k * period + deadline > done.
    let open = match done_ns.checked_sub(deadline_ns) {
        Some(late) => late / period_ns + 1,
        None => 0,
    };
    let next = open.max(index + 1);
    Cycle {
        missed,
        skipped: next - index - 1,
        next,
    }
}
/// Off/on miss difference per paired repetition and in total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdditionalMisses {
    /// `on[i].misses - off[i].misses`.
    pub paired: Vec<i64>,
    /// Sum of the paired differences.
    pub total: i64,
}
/// Additional deadline misses exist only for ≥3 paired runs that share CPU,
/// period, deadline, work, kernel and length.
pub fn additional_misses(
    off: &[ControlRun],
    on: &[ControlRun],
) -> Result<AdditionalMisses, Rejection> {
    if off.len() < REPETITIONS || on.len() < REPETITIONS || off.len() != on.len() {
        return Err(Rejection::InsufficientRepetitions);
    }
    let first = &off[0];
    if off.iter().chain(on).any(|r| {
        r.config != first.config || r.kernel != first.kernel || r.releases != first.releases
    }) {
        return Err(Rejection::ConditionMismatch);
    }
    let paired: Vec<i64> = off
        .iter()
        .zip(on)
        .map(|(a, b)| b.misses as i64 - a.misses as i64)
        .collect();
    Ok(AdditionalMisses {
        total: paired.iter().sum(),
        paired,
    })
}
/// Fixed integer work; the compiler cannot elide or shorten it.
pub fn work(iterations: u64) -> u64 {
    let mut x = 0x9e37_79b9_7f4a_7c15_u64;
    for i in 0..black_box(iterations) {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(i);
    }
    black_box(x)
}
/// Iterations that take about `target` on the calling thread's current CPU:
/// median of 11 timed runs. Call it pinned and with the logger off.
pub fn calibrate(target: Duration) -> u64 {
    let target_ns = target.as_nanos().max(1);
    let mut n = 1024_u64;
    while n < 1 << 40 {
        let t = Instant::now();
        work(n);
        if t.elapsed().as_nanos() * 4 >= target_ns {
            break;
        }
        n *= 2;
    }
    let mut samples = [0_u128; 11];
    for s in &mut samples {
        let t = Instant::now();
        work(n);
        *s = t.elapsed().as_nanos().max(1);
    }
    samples.sort_unstable();
    u64::try_from(u128::from(n) * target_ns / samples[5])
        .unwrap_or(u64::MAX)
        .max(1)
}
/// Calibration in the run's own duty cycle: frequency scaling makes a
/// one-off busy loop unrepeatable, so each round times `work` at real
/// releases and rescales by the median. The last round's median is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calibration {
    /// Fixed work per release for every later off/on run.
    pub iterations: u64,
    /// Median work time of the final round at `iterations`.
    pub median_ns: u64,
    /// Rounds performed.
    pub rounds: u32,
}
/// Calibrate `target` work at `period` releases on the calling thread
/// (pin it first), `releases` per round, until the median is within 5 %.
pub fn calibrate_periodic(
    target: Duration,
    period: Duration,
    releases: usize,
) -> Result<Calibration, Error> {
    let target_ns = u64::try_from(target.as_nanos())?;
    let period_ns = u64::try_from(period.as_nanos())?;
    if target_ns == 0 || period_ns == 0 || releases == 0 {
        return Err(invalid("zero calibration target, period or releases"));
    }
    let mut iterations = calibrate(target);
    let mut samples = vec![0_u64; releases];
    for round in 1..=8 {
        let origin = now_ns() + period_ns;
        for (k, sample) in samples.iter_mut().enumerate() {
            sleep_until(origin + k as u64 * period_ns)?;
            let t = now_ns();
            work(iterations);
            *sample = (now_ns() - t).max(1);
        }
        samples.sort_unstable();
        let median_ns = samples[releases / 2];
        let off = median_ns.abs_diff(target_ns);
        if off * 20 <= target_ns || round == 8 {
            return Ok(Calibration {
                iterations,
                median_ns,
                rounds: round,
            });
        }
        iterations =
            u64::try_from(u128::from(iterations) * u128::from(target_ns) / u128::from(median_ns))?
                .max(1);
    }
    Err(invalid("calibration did not run"))
}
/// Pin the calling thread to one CPU.
pub fn pin(cpu: usize) -> Result<(), Error> {
    let mut set = CpuSet::new();
    set.set(cpu);
    sched_setaffinity(None, &set)?;
    Ok(())
}
/// Running kernel release, e.g. `7.0.0-42-generic`.
pub fn kernel() -> Result<String, Error> {
    Ok(fs::read_to_string("/proc/sys/kernel/osrelease")?
        .trim()
        .to_owned())
}
fn now_ns() -> u64 {
    let t = clock_gettime(ClockId::Monotonic);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
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
/// Run `releases` absolute monotonic releases on the calling thread, pinning
/// it first when configured. Skipped releases count as misses.
pub fn run_control(config: ControlConfig, releases: u64) -> Result<ControlRun, Error> {
    if let Some(cpu) = config.cpu {
        pin(cpu)?;
    }
    let period = u64::try_from(config.period.as_nanos())?;
    let deadline = u64::try_from(config.deadline.as_nanos())?;
    if period == 0 || deadline == 0 {
        return Err(invalid("zero period or deadline"));
    }
    let mut r = ControlRun {
        config,
        kernel: kernel()?,
        releases,
        completed: 0,
        misses: 0,
        skipped: 0,
        off_cpu: 0,
        wake: Histogram::default(),
        response: Histogram::default(),
    };
    let origin = now_ns() + period;
    let mut k = 0;
    while k < releases {
        let release = origin + k * period;
        sleep_until(release)?;
        let wake = now_ns();
        work(config.work_iterations);
        let done = now_ns();
        if config.cpu.is_some_and(|c| sched_getcpu() != c) {
            r.off_cpu += 1;
        }
        r.wake.record(wake.saturating_sub(release));
        r.response.record(done.saturating_sub(release));
        let c = account(k, period, deadline, done - origin);
        if c.missed {
            r.misses += 1;
        } else {
            r.completed += 1;
        }
        // Releases beyond the run horizon are not part of this run.
        let skipped = c.skipped.min(releases - k - 1);
        r.misses += skipped;
        r.skipped += skipped;
        k = c.next;
    }
    Ok(r)
}

/// Exclusive `dot-logger-verification-*` directory. Drop removes only the
/// simple-file names this harness creates, then the directory if empty.
pub struct RunDir {
    path: PathBuf,
}
impl RunDir {
    /// Create a new exclusive directory under `parent`.
    pub fn create(parent: &Path) -> Result<Self, Error> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = parent.join(format!(
            "dot-logger-verification-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self { path })
    }
    /// Owned directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for RunDir {
    fn drop(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.path) {
            for e in entries.flatten() {
                if parse_name(&e.file_name()).is_some() {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
        let _ = fs::remove_dir(&self.path);
    }
}
fn file_name(file: usize, rotation: u32) -> String {
    format!("p0-{file:03}-{rotation:05}.bin")
}
fn parse_name(name: &std::ffi::OsStr) -> Option<(usize, u32)> {
    let (file, rotation) = name
        .to_str()?
        .strip_prefix("p0-")?
        .strip_suffix(".bin")?
        .split_once('-')?;
    Some((file.parse().ok()?, rotation.parse().ok()?))
}
fn fill(seq: u64) -> u8 {
    (seq % 251) as u8
}
fn available_bytes(dir: &Path) -> Result<u64, Error> {
    let s = rustix::fs::statvfs(dir)?;
    Ok(s.f_bavail.saturating_mul(s.f_frsize))
}
#[derive(Default)]
struct WriterStats {
    payload: u64,
    records: u64,
    files: u64,
    writes: u64,
    largest: usize,
    smallest: usize,
    syncs: u64,
    accepted_ns: u64,
}
impl WriterStats {
    fn note(&mut self, len: usize) {
        self.writes += 1;
        self.largest = self.largest.max(len);
        self.smallest = if self.writes == 1 {
            len
        } else {
            self.smallest.min(len)
        };
    }
}
struct Output {
    id: usize,
    rotation: u32,
    file: File,
    written: u64,
    buf: Vec<u8>,
    closed: Vec<File>,
}
impl Output {
    fn open(dir: &Path, id: usize, rotation: u32, block: usize) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(file_name(id, rotation)))?;
        Ok(Self {
            id,
            rotation,
            file,
            written: 0,
            buf: Vec::with_capacity(block),
            closed: Vec::new(),
        })
    }
    fn write(&mut self, stats: &mut WriterStats, bytes: &[u8]) -> Result<(), Error> {
        self.file.write_all(bytes)?;
        self.written += bytes.len() as u64;
        stats.note(bytes.len());
        Ok(())
    }
    fn push(
        &mut self,
        stats: &mut WriterStats,
        mut bytes: &[u8],
        block: usize,
    ) -> Result<(), Error> {
        while !bytes.is_empty() {
            if self.buf.is_empty() && bytes.len() >= block {
                self.write(stats, &bytes[..block])?;
                bytes = &bytes[block..];
                continue;
            }
            let n = (block - self.buf.len()).min(bytes.len());
            self.buf.extend_from_slice(&bytes[..n]);
            bytes = &bytes[n..];
            if self.buf.len() == block {
                self.flush(stats)?;
            }
        }
        Ok(())
    }
    fn flush(&mut self, stats: &mut WriterStats) -> Result<(), Error> {
        if !self.buf.is_empty() {
            let buf = std::mem::take(&mut self.buf);
            self.write(stats, &buf)?;
            self.buf = buf;
            self.buf.clear();
        }
        Ok(())
    }
    fn pending(&self) -> u64 {
        self.written + self.buf.len() as u64
    }
}
/// Run the P0 simple-file writer into the exclusive `dir`. Writers pin to
/// `writer_cpus[w % len]` when given. Admission stops when free space would
/// fall below `reserve_bytes`; admitted records are still synced.
pub fn write_p0(
    dir: &Path,
    workload: StorageWorkload,
    reserve_bytes: u64,
    writer_cpus: &[usize],
) -> Result<StorageRun, Error> {
    let w = workload;
    let frame = (FRAME_HEADER + w.record_payload_bytes) as u64;
    if w.record_payload_bytes == 0
        || w.files == 0
        || !(1..=w.files).contains(&w.writers)
        || w.block_bytes == 0
        || frame > w.file_limit_bytes
        || w.offered_bytes_per_second == Some(0)
    {
        return Err(invalid("invalid P0 workload"));
    }
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let results: Vec<Result<WriterStats, Error>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..w.writers)
            .map(|id| {
                let stop = &stop;
                let cpu = (!writer_cpus.is_empty()).then(|| writer_cpus[id % writer_cpus.len()]);
                scope.spawn(move || {
                    if let Some(cpu) = cpu {
                        pin(cpu)?;
                    }
                    writer(dir, w, reserve_bytes, id, start, stop)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err(invalid("writer panicked"))))
            .collect()
    });
    let mut run = StorageRun {
        tool: Tool::P0SimpleFile,
        workload: w,
        payload_bytes: 0,
        framing_bytes: 0,
        files_created: 0,
        write_calls: 0,
        largest_write: 0,
        smallest_write: 0,
        periodic_syncs: 0,
        accepted_ns: 0,
        durable_ns: None,
        stopped: None,
    };
    for r in results {
        let s = r?;
        run.payload_bytes += s.payload;
        run.framing_bytes += s.records * FRAME_HEADER as u64;
        run.files_created += s.files;
        if s.writes > 0 {
            run.smallest_write = if run.write_calls == 0 {
                s.smallest
            } else {
                run.smallest_write.min(s.smallest)
            };
        }
        run.write_calls += s.writes;
        run.largest_write = run.largest_write.max(s.largest);
        run.periodic_syncs += s.syncs;
        run.accepted_ns = run.accepted_ns.max(s.accepted_ns);
    }
    // Every file was synced by its writer; the directory makes names durable.
    File::open(dir)?.sync_all()?;
    run.durable_ns = Some(u64::try_from(start.elapsed().as_nanos())?);
    if stop.load(Ordering::Acquire) {
        run.stopped = Some(Stop::Reserve);
    }
    Ok(run)
}
fn writer(
    dir: &Path,
    w: StorageWorkload,
    reserve_bytes: u64,
    id: usize,
    start: Instant,
    stop: &AtomicBool,
) -> Result<WriterStats, Error> {
    let mut stats = WriterStats::default();
    let mut outputs = Vec::new();
    for f in (id..w.files).step_by(w.writers) {
        outputs.push(Output::open(dir, f, 0, w.block_bytes)?);
        stats.files += 1;
    }
    let share = w
        .offered_bytes_per_second
        .map(|rate| rate as f64 * outputs.len() as f64 / w.files as f64);
    let mut payload = vec![0_u8; w.record_payload_bytes];
    let mut last_sync = Instant::now();
    if available_bytes(dir)? < reserve_bytes.saturating_add(w.file_limit_bytes) {
        stop.store(true, Ordering::Release);
    }
    let mut seq = 0_u64;
    while seq < w.records && !stop.load(Ordering::Acquire) {
        let f = (seq % w.files as u64) as usize;
        if f % w.writers != id {
            seq += 1;
            continue;
        }
        if let Some(share) = share {
            // Sleep only when more than 1 ms ahead of the offered schedule.
            let due = Duration::from_secs_f64(stats.payload as f64 / share);
            let ahead = due.saturating_sub(start.elapsed());
            if ahead > Duration::from_millis(1) {
                std::thread::sleep(ahead);
            }
        }
        let out = &mut outputs[f / w.writers];
        let frame = (FRAME_HEADER + w.record_payload_bytes) as u64;
        if out.pending() + frame > w.file_limit_bytes {
            if available_bytes(dir)? < reserve_bytes.saturating_add(w.file_limit_bytes) {
                stop.store(true, Ordering::Release);
                break;
            }
            out.flush(&mut stats)?;
            let next = Output::open(dir, out.id, out.rotation + 1, w.block_bytes)?;
            let old = std::mem::replace(out, next);
            out.closed = old.closed;
            out.closed.push(old.file);
            stats.files += 1;
        }
        let mut header = [0_u8; FRAME_HEADER];
        header[..8].copy_from_slice(&seq.to_le_bytes());
        header[8..].copy_from_slice(&(w.record_payload_bytes as u64).to_le_bytes());
        payload.fill(fill(seq));
        out.push(&mut stats, &header, w.block_bytes)?;
        out.push(&mut stats, &payload, w.block_bytes)?;
        stats.payload += w.record_payload_bytes as u64;
        stats.records += 1;
        stats.accepted_ns = u64::try_from(start.elapsed().as_nanos())?;
        if last_sync.elapsed() >= w.sync_interval {
            for out in &mut outputs {
                out.flush(&mut stats)?;
                out.file.sync_data()?;
                for closed in out.closed.drain(..) {
                    closed.sync_data()?;
                }
            }
            stats.syncs += 1;
            last_sync = Instant::now();
        }
        seq += 1;
    }
    for out in &mut outputs {
        out.flush(&mut stats)?;
        out.file.sync_all()?;
        for closed in out.closed.drain(..) {
            closed.sync_all()?;
        }
    }
    Ok(stats)
}
/// Records and bytes read back from a P0 run directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verified {
    /// Records whose sequence, length and content matched the oracle.
    pub records: u64,
    /// Their payload bytes.
    pub payload_bytes: u64,
    /// Files read.
    pub files: u64,
}
/// Every file must hold exactly a prefix of its arithmetic sequence
/// `f, f + files, ...`, with matching length and fill bytes, across rotations.
pub fn verify_p0(dir: &Path, run: &StorageRun) -> Result<Verified, Error> {
    let w = run.workload;
    let mut names: Vec<(usize, u32, PathBuf)> = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let (file, rotation) = parse_name(&e.file_name()).ok_or(invalid("foreign file"))?;
        names.push((file, rotation, e.path()));
    }
    names.sort();
    let mut v = Verified {
        records: 0,
        payload_bytes: 0,
        files: names.len() as u64,
    };
    let mut expect: Vec<u64> = (0..w.files as u64).collect();
    let mut chunk = vec![0_u8; w.record_payload_bytes.min(1 << 20)];
    for (file, _, path) in &names {
        if *file >= w.files {
            return Err(invalid("file outside workload"));
        }
        let mut r = BufReader::with_capacity(1 << 20, File::open(path)?);
        let mut header = [0_u8; FRAME_HEADER];
        loop {
            match r.read_exact(&mut header) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let seq = u64::from_le_bytes(header[..8].try_into()?);
            let len = u64::from_le_bytes(header[8..].try_into()?);
            if seq != expect[*file] || len != w.record_payload_bytes as u64 {
                return Err(invalid("sequence or length differs from oracle"));
            }
            let mut left = w.record_payload_bytes;
            while left > 0 {
                let n = left.min(chunk.len());
                r.read_exact(&mut chunk[..n])?;
                if chunk[..n].iter().any(|&b| b != fill(seq)) {
                    return Err(invalid("payload differs from oracle"));
                }
                left -= n;
            }
            expect[*file] += w.files as u64;
            v.records += 1;
            v.payload_bytes += len;
        }
    }
    if v.payload_bytes != run.payload_bytes || v.files != run.files_created {
        return Err(invalid("read-back totals differ from the run"));
    }
    Ok(v)
}
