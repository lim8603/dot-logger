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
    sync::{
        Condvar, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
mod facade;
#[allow(unused_imports)]
pub use facade::{
    CallCost, FACADE_SEGMENT_RECORDS, LoggerLoad, LoggerRun, LoggerVerified, call_cost,
    verify_logger, write_logger,
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
    /// Rotate before a file would hold more than this many records: the
    /// facade's fixed segment record limit (`FACADE_SEGMENT_RECORDS`) for
    /// logger comparisons, `u64::MAX` for none.
    pub file_record_limit: u64,
    /// Write request size; sync and rotation flush shorter tails.
    pub block_bytes: usize,
    /// Periodic flush + fdatasync of every file a writer owns.
    pub sync_interval: Duration,
    /// Paced input rate, or `None` for the device's unpaced capability.
    pub offered_bytes_per_second: Option<u64>,
}
/// Write requests (`write_all` calls); the kernel may split them further.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WriteCalls {
    /// Requests issued.
    pub calls: u64,
    /// Largest request.
    pub largest: usize,
    /// Smallest request.
    pub smallest: usize,
}
/// Why admission stopped before the offered records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Free space would fall below the declared reserve.
    Reserve,
    /// The logger's shutdown report was not a success: records were lost,
    /// failed or left unconfirmed.
    Unsuccessful,
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
    /// Write requests the tool issued; `None` when the tool cannot observe
    /// them (the logger's internal writer), never a fabricated zero.
    pub writes: Option<WriteCalls>,
    /// Periodic sync rounds over all writers; `None` when not observable.
    pub periodic_syncs: Option<u64>,
    /// Shared start (CLOCK_MONOTONIC ns) from the run's `Gate`; every
    /// other time is measured from it, setup excluded.
    pub start_ns: u64,
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
    /// The logger lost, failed or left records unconfirmed.
    Unsuccessful,
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
    /// Control and load did not share a start, or the load's admission did
    /// not cover the control window (REV-052 RV-130).
    NotConcurrent,
}
/// Lead from the last party's arrival to the shared start, so every party is
/// asleep on the absolute start before it arrives.
pub const GATE_LEAD: Duration = Duration::from_millis(50);
/// Common start of one measured run (REV-052 RV-130). Each party — P0 writer
/// or logger producer threads (`load`), control and safety loops (`others`)
/// — passes after its own setup; all then count from one absolute instant,
/// so worker start, file creation and preallocation stay outside every
/// measured interval. A party whose setup fails aborts, releasing the rest.
pub struct Gate {
    load: usize,
    parties: usize,
    state: Mutex<GateState>,
    changed: Condvar,
}
#[derive(Default)]
struct GateState {
    arrived: usize,
    start: Option<u64>,
    aborted: bool,
}
impl Gate {
    /// `load` load threads plus `others` concurrent loops.
    pub fn new(load: usize, others: usize) -> Self {
        Self {
            load,
            parties: load + others,
            state: Mutex::new(GateState::default()),
            changed: Condvar::new(),
        }
    }
    /// Load threads this gate expects; a load must match it exactly.
    pub fn load_parties(&self) -> usize {
        self.load
    }
    /// Arrive after setup, then sleep until the shared start and return it
    /// (CLOCK_MONOTONIC ns). Fails once any party aborted before the start.
    pub fn pass(&self) -> Result<u64, Error> {
        let mut s = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        s.arrived += 1;
        if s.arrived == self.parties && !s.aborted {
            s.start = Some(now_ns() + u64::try_from(GATE_LEAD.as_nanos())?);
            self.changed.notify_all();
        }
        while s.start.is_none() && !s.aborted {
            s = self.changed.wait(s).unwrap_or_else(PoisonError::into_inner);
        }
        let Some(start) = s.start else {
            return Err(invalid("another party failed before the start"));
        };
        drop(s);
        sleep_until(start)?;
        Ok(start)
    }
    /// Run a party's `setup`, then pass; a failed setup aborts the gate.
    pub fn after<T>(&self, setup: impl FnOnce() -> Result<T, Error>) -> Result<(T, u64), Error> {
        match setup() {
            Ok(value) => Ok((value, self.pass()?)),
            Err(e) => {
                self.abort();
                Err(e)
            }
        }
    }
    /// Release every waiting party without a start. No effect once started.
    pub fn abort(&self) {
        let mut s = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if s.start.is_none() {
            s.aborted = true;
            self.changed.notify_all();
        }
    }
    /// The shared start, once every party arrived.
    pub fn started(&self) -> Option<u64> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .start
    }
}
/// Control window against the load interval of one concurrent run, all from
/// the shared start. The final durability tail is kept apart: it follows the
/// load and is not load the control loops ran under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadWindow {
    /// |control origin - load start|; zero for one shared `Gate`.
    pub skew_ns: u64,
    /// Nominal control window: releases × period.
    pub control_ns: u64,
    /// Load admission interval: start to the last accepted record.
    pub load_ns: u64,
    /// Last accepted record to the final file and directory sync.
    pub tail_ns: Option<u64>,
    /// Part of the control window inside the load interval.
    pub covered_ns: u64,
}
impl LoadWindow {
    /// Concurrent when both share the start and the load covered at least
    /// 99 % of the control window (a paced load admits its last record
    /// slightly before the window ends).
    pub fn judge(&self) -> Result<(), Rejection> {
        if self.skew_ns != 0 || u128::from(self.covered_ns) * 100 < u128::from(self.control_ns) * 99
        {
            return Err(Rejection::NotConcurrent);
        }
        Ok(())
    }
}
/// Compare a control run with the load it ran beside.
pub fn load_window(control: &ControlRun, load: &StorageRun) -> LoadWindow {
    let period = u64::try_from(control.config.period.as_nanos()).unwrap_or(u64::MAX);
    let control_ns = control.releases.saturating_mul(period);
    let control_end = control.origin_ns.saturating_add(control_ns);
    let load_end = load.start_ns.saturating_add(load.accepted_ns);
    let covered_ns = control_end
        .min(load_end)
        .saturating_sub(control.origin_ns.max(load.start_ns));
    LoadWindow {
        skew_ns: control.origin_ns.abs_diff(load.start_ns),
        control_ns,
        load_ns: load.accepted_ns,
        tail_ns: load.durable_ns.map(|d| d.saturating_sub(load.accepted_ns)),
        covered_ns,
    }
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
/// Records of `payload` bytes in `total_bytes`; a zero payload is a usage
/// error rather than a division by zero (REV-051 RV-127).
pub fn records_for(total_bytes: u64, payload: usize) -> Result<u64, Error> {
    total_bytes
        .checked_div(payload as u64)
        .ok_or_else(|| "payload must be at least 1 byte".into())
}

/// Throughput up to the final file/directory sync; RAM acceptance alone and
/// truncated runs are rejected.
pub fn throughput(run: &StorageRun) -> Result<Throughput, Rejection> {
    match run.stopped {
        Some(Stop::Unsuccessful) => return Err(Rejection::Unsuccessful),
        Some(Stop::Reserve) => return Err(Rejection::Incomplete),
        None => {}
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
    /// First release (CLOCK_MONOTONIC ns), the run's shared `Gate` start.
    pub origin_ns: u64,
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
            *sample = now_ns().saturating_sub(t).max(1);
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
/// CLOCK_MONOTONIC in nanoseconds, the clock of `Gate` starts.
pub fn monotonic_ns() -> u64 {
    now_ns()
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
/// it first when configured; the first release is the `gate`'s shared start.
/// Skipped releases count as misses.
pub fn run_control(config: ControlConfig, releases: u64, gate: &Gate) -> Result<ControlRun, Error> {
    let ((period, deadline, kernel), origin) = gate.after(|| {
        if let Some(cpu) = config.cpu {
            pin(cpu)?;
        }
        let period = u64::try_from(config.period.as_nanos())?;
        let deadline = u64::try_from(config.deadline.as_nanos())?;
        if period == 0 || deadline == 0 {
            return Err(invalid("zero period or deadline"));
        }
        Ok((period, deadline, kernel()?))
    })?;
    let mut r = ControlRun {
        config,
        kernel,
        origin_ns: origin,
        releases,
        completed: 0,
        misses: 0,
        skipped: 0,
        off_cpu: 0,
        wake: Histogram::default(),
        response: Histogram::default(),
    };
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
        let c = account(k, period, deadline, done.saturating_sub(origin));
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
/// simple-file names and the `out-NNN` logger output directories this
/// harness creates, then the directory if empty.
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
                } else if is_output_dir(&e.file_name()) {
                    let _ = fs::remove_dir_all(e.path());
                }
            }
        }
        let _ = fs::remove_dir(&self.path);
    }
}
/// `out-NNN`: a logger output directory created by `write_logger`.
fn is_output_dir(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .and_then(|n| n.strip_prefix("out-"))
        .is_some_and(|n| n.len() == 3 && n.bytes().all(|b| b.is_ascii_digit()))
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
    records: u64,
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
            records: 0,
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
/// fall below `reserve_bytes`; admitted records are still synced. Each
/// writer thread is one `gate` load party: thread start, pinning and file
/// creation are setup before the shared start.
pub fn write_p0(
    dir: &Path,
    workload: StorageWorkload,
    reserve_bytes: u64,
    writer_cpus: &[usize],
    gate: &Gate,
) -> Result<StorageRun, Error> {
    let w = workload;
    let frame = (FRAME_HEADER + w.record_payload_bytes) as u64;
    if w.record_payload_bytes == 0
        || w.files == 0
        || !(1..=w.files).contains(&w.writers)
        || w.block_bytes == 0
        || frame > w.file_limit_bytes
        || w.file_record_limit == 0
        || w.offered_bytes_per_second == Some(0)
    {
        return Err(invalid("invalid P0 workload"));
    }
    if gate.load_parties() != w.writers {
        return Err(invalid("gate load parties differ from the P0 writers"));
    }
    let stop = AtomicBool::new(false);
    let results: Vec<Result<WriterStats, Error>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..w.writers)
            .map(|id| {
                let stop = &stop;
                let cpu = (!writer_cpus.is_empty()).then(|| writer_cpus[id % writer_cpus.len()]);
                scope.spawn(move || writer(dir, w, reserve_bytes, id, cpu, gate, stop))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err(invalid("writer panicked"))))
            .collect()
    });
    let start = gate
        .started()
        .ok_or_else(|| invalid("P0 writers never started"))?;
    let mut run = StorageRun {
        tool: Tool::P0SimpleFile,
        workload: w,
        payload_bytes: 0,
        framing_bytes: 0,
        files_created: 0,
        writes: None,
        periodic_syncs: None,
        start_ns: start,
        accepted_ns: 0,
        durable_ns: None,
        stopped: None,
    };
    let mut writes = WriteCalls::default();
    for r in results {
        let s = r?;
        run.payload_bytes += s.payload;
        run.framing_bytes += s.records * FRAME_HEADER as u64;
        run.files_created += s.files;
        if s.writes > 0 {
            writes.smallest = if writes.calls == 0 {
                s.smallest
            } else {
                writes.smallest.min(s.smallest)
            };
        }
        writes.calls += s.writes;
        writes.largest = writes.largest.max(s.largest);
        *run.periodic_syncs.get_or_insert(0) += s.syncs;
        run.accepted_ns = run.accepted_ns.max(s.accepted_ns);
    }
    run.writes = Some(writes);
    // Every file was synced by its writer; the directory makes names durable.
    File::open(dir)?.sync_all()?;
    run.durable_ns = Some(now_ns().saturating_sub(start));
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
    cpu: Option<usize>,
    gate: &Gate,
    stop: &AtomicBool,
) -> Result<WriterStats, Error> {
    let mut stats = WriterStats::default();
    let ((mut outputs, mut payload), start) = gate.after(|| {
        if let Some(cpu) = cpu {
            pin(cpu)?;
        }
        let mut outputs = Vec::new();
        for f in (id..w.files).step_by(w.writers) {
            outputs.push(Output::open(dir, f, 0, w.block_bytes)?);
        }
        if available_bytes(dir)? < reserve_bytes.saturating_add(w.file_limit_bytes) {
            stop.store(true, Ordering::Release);
        }
        Ok((outputs, vec![0_u8; w.record_payload_bytes]))
    })?;
    stats.files = outputs.len() as u64;
    let elapsed = || Duration::from_nanos(now_ns().saturating_sub(start));
    let share = w
        .offered_bytes_per_second
        .map(|rate| rate as f64 * outputs.len() as f64 / w.files as f64);
    let mut last_sync = Instant::now();
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
            let ahead = due.saturating_sub(elapsed());
            if ahead > Duration::from_millis(1) {
                std::thread::sleep(ahead);
            }
        }
        let out = &mut outputs[f / w.writers];
        let frame = (FRAME_HEADER + w.record_payload_bytes) as u64;
        if out.pending() + frame > w.file_limit_bytes || out.records >= w.file_record_limit {
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
        out.records += 1;
        stats.payload += w.record_payload_bytes as u64;
        stats.records += 1;
        stats.accepted_ns = u64::try_from(elapsed().as_nanos())?;
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
    /// Most records found in one file.
    pub largest_file_records: u64,
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
        largest_file_records: 0,
    };
    let mut expect: Vec<u64> = (0..w.files as u64).collect();
    let mut chunk = vec![0_u8; w.record_payload_bytes.min(1 << 20)];
    for (file, _, path) in &names {
        if *file >= w.files {
            return Err(invalid("file outside workload"));
        }
        let mut r = BufReader::with_capacity(1 << 20, File::open(path)?);
        let mut header = [0_u8; FRAME_HEADER];
        let mut in_file = 0_u64;
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
            in_file += 1;
            if in_file > w.file_record_limit {
                return Err(invalid("file holds more records than the limit"));
            }
            v.largest_file_records = v.largest_file_records.max(in_file);
            v.records += 1;
            v.payload_bytes += len;
        }
    }
    if v.payload_bytes != run.payload_bytes || v.files != run.files_created {
        return Err(invalid("read-back totals differ from the run"));
    }
    Ok(v)
}

/// Nanosecond latency histogram for RT call costs: values below 8 ns are
/// exact, larger ones fall into 8 linear sub-buckets per power of two, so a
/// quantile is reported as its bucket's upper bound with ≤ 1/8 relative
/// error, capped at the exact maximum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatencyHistogram {
    count: u64,
    max: u64,
    buckets: Vec<u64>,
}
const SUB_BITS: u32 = 3;
const SUB: u64 = 1 << SUB_BITS;
const LATENCY_BUCKETS: usize = (SUB as usize) * (64 - SUB_BITS as usize + 1);
impl Default for LatencyHistogram {
    fn default() -> Self {
        Self {
            count: 0,
            max: 0,
            buckets: vec![0; LATENCY_BUCKETS],
        }
    }
}
impl LatencyHistogram {
    fn index(ns: u64) -> usize {
        if ns < SUB {
            return ns as usize;
        }
        let exp = 63 - ns.leading_zeros();
        let sub = (ns >> (exp - SUB_BITS)) & (SUB - 1);
        (SUB + u64::from(exp - SUB_BITS) * SUB + sub) as usize
    }
    fn upper(index: usize) -> u64 {
        let index = index as u64;
        if index < SUB {
            return index;
        }
        let exp = (index - SUB) / SUB + u64::from(SUB_BITS);
        let sub = (index - SUB) % SUB;
        ((SUB + sub + 1) << (exp - u64::from(SUB_BITS))).saturating_sub(1)
    }
    /// Add one sample.
    pub fn record(&mut self, ns: u64) {
        self.buckets[Self::index(ns)] += 1;
        self.count += 1;
        self.max = self.max.max(ns);
    }
    /// Add every sample of `other`.
    pub fn merge(&mut self, other: &Self) {
        for (a, b) in self.buckets.iter_mut().zip(&other.buckets) {
            *a += b;
        }
        self.count += other.count;
        self.max = self.max.max(other.max);
    }
    /// Recorded samples.
    pub fn count(&self) -> u64 {
        self.count
    }
    /// Exact largest sample.
    pub fn max(&self) -> u64 {
        self.max
    }
    /// Upper bound of the `q` quantile (0 < q ≤ 1); `None` when empty.
    pub fn quantile(&self, q: f64) -> Option<u64> {
        if self.count == 0 {
            return None;
        }
        let rank = ((q * self.count as f64).ceil() as u64).clamp(1, self.count);
        let mut seen = 0;
        for (i, n) in self.buckets.iter().enumerate() {
            seen += n;
            if seen >= rank {
                return Some(Self::upper(i).min(self.max));
            }
        }
        Some(self.max)
    }
}

/// Smallest logger payload: it starts with the record's sequence.
pub const SEQUENCE_BYTES: usize = 8;
/// Encode the logger record `seq`: its sequence (LE u64) then the P0 fill
/// byte. Returns the payload length, or `None` below eight bytes.
pub fn p1_payload(seq: u64, out: &mut [u8]) -> Option<usize> {
    if out.len() < SEQUENCE_BYTES {
        return None;
    }
    let (head, tail) = out.split_at_mut(SEQUENCE_BYTES);
    head.copy_from_slice(&seq.to_le_bytes());
    tail.fill(fill(seq));
    Some(out.len())
}
/// Sequence carried by a logger payload.
pub fn payload_sequence(payload: &[u8]) -> Option<u64> {
    Some(u64::from_le_bytes(
        payload.get(..SEQUENCE_BYTES)?.try_into().ok()?,
    ))
}
/// The payload is exactly what `p1_payload(seq, ..)` wrote.
pub fn check_p1_payload(seq: u64, payload: &[u8]) -> bool {
    payload_sequence(payload) == Some(seq)
        && payload[SEQUENCE_BYTES..].iter().all(|&b| b == fill(seq))
}

/// Read-back oracle for runs where record `seq` comes from producer
/// `seq % producers` and lands in file `seq % files`.
pub struct SequenceOracle {
    records: u64,
    files: u64,
    producers: u64,
    seen: Vec<u64>,
    count: u64,
    /// Next allowed sequence per (producer, file).
    next: Vec<u64>,
}
impl SequenceOracle {
    /// Expect sequences `0..records`.
    pub fn new(records: u64, files: usize, producers: usize) -> Self {
        Self {
            records,
            files: files.max(1) as u64,
            producers: producers.max(1) as u64,
            seen: vec![0; records.div_ceil(64) as usize],
            count: 0,
            next: vec![0; files.max(1) * producers.max(1)],
        }
    }
    /// Account one stored record of `file`.
    pub fn see(&mut self, file: usize, seq: u64) -> Result<(), Error> {
        if seq >= self.records {
            return Err(invalid("sequence outside the workload"));
        }
        if seq % self.files != file as u64 {
            return Err(invalid("record stored in the wrong file"));
        }
        let (word, bit) = ((seq / 64) as usize, 1_u64 << (seq % 64));
        if self.seen[word] & bit != 0 {
            return Err(invalid("duplicate record"));
        }
        let slot = (seq % self.producers * self.files + file as u64) as usize;
        if seq < self.next[slot] {
            return Err(invalid("producer order broken within a file"));
        }
        self.seen[word] |= bit;
        self.next[slot] = seq + 1;
        self.count += 1;
        Ok(())
    }
    /// Every expected record was seen; returns their number.
    pub fn finish(&self) -> Result<u64, Error> {
        if self.count != self.records {
            return Err(invalid("records missing from the files"));
        }
        Ok(self.count)
    }
}
