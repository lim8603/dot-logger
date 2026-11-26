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
//! For fault injection (TASK-024 S2) `run` also takes `--sample-ms`,
//! `--reserve-mib` and, with the `remote` feature, `--remote <port>`; samples
//! then carry each output's failed records and time since its last sync.
//! `inspect` reports what a directory holds (closed and, with `--active`,
//! unfinished files) and the latest cumulative loss checkpoint, without a
//! pass/fail judgment: the S2 driver (`scripts/soak-faults.py`) judges.
//!
//! Heap samples also split allocations and frees per thread, and a `run`
//! fails when the RT threads (control, frame) touch the heap once warm
//! (`rt_heap` in the end line, TASK-039).
//!
//! Measurement harness only, never a product API. RT rows set
//! `DOT_LOGGER_PERF_RT=fifo` (control thread SCHED_FIFO 70 on CPU 2, memory
//! locked), as `perf_baseline` does; the control thread is always pinned.
#[cfg(target_os = "linux")]
#[path = "support/perf/mod.rs"]
mod perf;

/// Heap accounting for the RSS judgment (TASK-024 S4, EV-120): live bytes
/// and their lowest point between samples, so a leak (live bytes rising)
/// can be told apart from allocator retention (RSS rising, live bytes flat);
/// and per thread, so the RT threads can be held to zero (TASK-039).
/// Harness only; the unsafe here forwards to `System` unchanged.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod heap {
    use serde_json::{Value, json};
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed};

    struct Counting;
    static LIVE: AtomicUsize = AtomicUsize::new(0);
    static LOW: AtomicUsize = AtomicUsize::new(usize::MAX);
    static CALLS: AtomicU64 = AtomicU64::new(0);

    /// Per-thread counts (TASK-039): which thread allocates or frees. Each
    /// thread takes the next slot on its first call and records its OS tid;
    /// the last slot also takes every thread beyond it. Nothing here
    /// allocates: a const, drop-free thread local and fixed static arrays.
    const SLOTS: usize = 64;
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    static TID: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];
    static ALLOCS: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    static ALLOC_BYTES: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    static FREES: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    thread_local! {
        static SLOT: Cell<usize> = const { Cell::new(usize::MAX) };
    }

    fn slot() -> usize {
        SLOT.try_with(|slot| {
            if slot.get() == usize::MAX {
                let next = NEXT.fetch_add(1, Relaxed).min(SLOTS - 1);
                if TID[next].load(Relaxed) == 0 {
                    TID[next].store(
                        rustix::thread::gettid().as_raw_nonzero().get() as u32,
                        Relaxed,
                    );
                }
                slot.set(next);
            }
            slot.get()
        })
        .unwrap_or(SLOTS - 1)
    }

    fn grow(by: usize) {
        LIVE.fetch_add(by, Relaxed);
        CALLS.fetch_add(1, Relaxed);
        let slot = slot();
        ALLOCS[slot].fetch_add(1, Relaxed);
        ALLOC_BYTES[slot].fetch_add(by as u64, Relaxed);
    }
    fn shrink(by: usize) {
        // Wrapping: an allocator must never panic.
        let now = LIVE.fetch_sub(by, Relaxed).wrapping_sub(by);
        LOW.fetch_min(now, Relaxed);
        FREES[slot()].fetch_add(1, Relaxed);
    }

    /// RT threads (control, frame) registered at the start of their steady
    /// loop, with their counts at that moment (TASK-039).
    const RT_THREADS: usize = 2;
    static RT_SLOT: [AtomicUsize; RT_THREADS] =
        [const { AtomicUsize::new(usize::MAX) }; RT_THREADS];
    static RT_BASE: [AtomicU64; RT_THREADS * 2] = [const { AtomicU64::new(0) }; RT_THREADS * 2];

    /// Called by an RT thread once warm-up (calibration, first use) is done:
    /// from here on it must neither allocate nor free.
    pub fn rt_begin(index: usize) {
        let slot = slot();
        RT_BASE[index * 2].store(ALLOCS[slot].load(Relaxed), Relaxed);
        RT_BASE[index * 2 + 1].store(FREES[slot].load(Relaxed), Relaxed);
        RT_SLOT[index].store(slot, Relaxed);
    }

    /// Allocations and frees on the RT threads since `rt_begin`, and how many
    /// of them registered. Read while they still run, before thread exit.
    pub fn rt_since_begin() -> Value {
        let (mut allocs, mut frees, mut threads) = (0, 0, 0);
        for (index, slot) in RT_SLOT.iter().enumerate() {
            let slot = slot.load(Relaxed);
            if slot == usize::MAX {
                continue;
            }
            threads += 1;
            allocs += ALLOCS[slot].load(Relaxed) - RT_BASE[index * 2].load(Relaxed);
            frees += FREES[slot].load(Relaxed) - RT_BASE[index * 2 + 1].load(Relaxed);
        }
        json!({"threads":threads,"allocs":allocs,"frees":frees})
    }

    /// Cumulative counts per thread that ever allocated, named from
    /// `/proc` while the thread lives (reading it allocates on the caller).
    fn threads() -> Vec<Value> {
        let used = NEXT.load(Relaxed).min(SLOTS);
        (0..used)
            .map(|i| {
                let tid = TID[i].load(Relaxed);
                let name = std::fs::read_to_string(format!("/proc/self/task/{tid}/comm"))
                    .map_or_else(|_| "(exited)".to_owned(), |n| n.trim_end().to_owned());
                json!({"tid":tid,"name":name,"allocs":ALLOCS[i].load(Relaxed),
                    "alloc_bytes":ALLOC_BYTES[i].load(Relaxed),"frees":FREES[i].load(Relaxed),
                    "shared":i == SLOTS - 1})
            })
            .collect()
    }

    // SAFETY: every method passes the caller's pointer, layout and size to
    // `System` unchanged and returns its result; the counters only observe.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
            let ptr = unsafe { System.alloc(layout) };
            if !ptr.is_null() {
                grow(layout.size());
            }
            ptr
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            // SAFETY: as for `alloc`.
            let ptr = unsafe { System.alloc_zeroed(layout) };
            if !ptr.is_null() {
                grow(layout.size());
            }
            ptr
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: `ptr` came from this allocator (that is, `System`) with `layout`.
            unsafe { System.dealloc(ptr, layout) };
            shrink(layout.size());
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: as for `dealloc`, and the caller checked `new_size`.
            let new = unsafe { System.realloc(ptr, layout, new_size) };
            if !new.is_null() {
                if new_size >= layout.size() {
                    grow(new_size - layout.size());
                } else {
                    shrink(layout.size() - new_size);
                    CALLS.fetch_add(1, Relaxed);
                    let slot = slot();
                    ALLOCS[slot].fetch_add(1, Relaxed);
                    FREES[slot].fetch_sub(1, Relaxed);
                }
            }
            new
        }
    }
    #[global_allocator]
    static GLOBAL: Counting = Counting;

    /// Live bytes now, their lowest point since the last sample, the
    /// allocator calls so far (`alloc`, `alloc_zeroed` and `realloc`; frees
    /// are not counted) and glibc's own view of its arenas.
    pub fn sample() -> Value {
        let live = LIVE.load(Relaxed);
        let low = LOW.swap(live, Relaxed).min(live);
        json!({"live_bytes":live,"low_bytes":low,"calls":CALLS.load(Relaxed),
            "glibc":glibc::arenas(),"threads":threads()})
    }
    /// Return free heap pages to the kernel; true when some were released.
    pub fn trim() -> bool {
        glibc::trim()
    }

    /// `mallinfo2` and `malloc_trim` are glibc extensions; other Linux C
    /// libraries such as musl report no arenas and trim nothing (REV-072
    /// RV-197), so the counting allocator still builds there.
    #[cfg(target_env = "gnu")]
    mod glibc {
        use serde_json::{Value, json};

        /// glibc `struct mallinfo2` (all arenas, glibc 2.33+).
        #[repr(C)]
        struct MallInfo2 {
            arena: usize,
            ordblks: usize,
            smblks: usize,
            hblks: usize,
            hblkhd: usize,
            usmblks: usize,
            fsmblks: usize,
            uordblks: usize,
            fordblks: usize,
            keepcost: usize,
        }
        unsafe extern "C" {
            fn mallinfo2() -> MallInfo2;
            fn malloc_trim(pad: usize) -> i32;
        }
        pub fn arenas() -> Value {
            // SAFETY: mallinfo2 takes no arguments and only reads allocator state.
            let info = unsafe { mallinfo2() };
            let _ = (info.smblks, info.usmblks, info.fsmblks, info.keepcost);
            json!({"arena":info.arena,"chunks_free":info.ordblks,"mmap_chunks":info.hblks,
                "mmap_bytes":info.hblkhd,"in_use":info.uordblks,"free":info.fordblks})
        }
        pub fn trim() -> bool {
            // SAFETY: malloc_trim takes a plain size and is thread-safe.
            unsafe { malloc_trim(0) == 1 }
        }
    }
    #[cfg(not(target_env = "gnu"))]
    mod glibc {
        pub fn arenas() -> serde_json::Value {
            serde_json::Value::Null
        }
        pub fn trim() -> bool {
            false
        }
    }
}
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
        Channel, EncodeError, EventMark, EventWindow, LogError, Logged, Logger, Preset, Record,
        Retention, Schema, Writer,
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
    const EVENT_SIDE: Duration = Duration::from_secs(30);
    const EVENT_LIMIT: usize = 6;
    const CONTROL_STEP: u64 = 31;
    const FRAME_STEP: u64 = 7;
    // Power cut: signal to cut-off, and the margin the host keeps (EV-043).
    const POWEROFF: Duration = Duration::from_secs(3);
    const SAFETY: Duration = Duration::from_millis(200);
    const POWER_RING_MIB: u64 = 1024;
    const MAX_PRODUCERS: usize = 8;
    const USAGE: &str = "usage: soak run <parent> <seconds> <sample_s> <ring_mib> \
        [--sample-ms <ms>] [--reserve-mib <mib>] [--remote <port>] \
        [--events <every_s>,<hold_s>,<release_s>] | \
        power <parent> <delay_s>[,<delay_s>...] | \
        verify <dir> <control_records> <frames> [--ring] | \
        inspect <dir> <control_records> <frames> [--active] | \
        reader <dir> <interval_s> <seconds>";

    /// Fault-injection options of `run` (TASK-024 S2).
    #[derive(Default)]
    struct Options {
        sample_ms: Option<u64>,
        reserve_mib: u64,
        remote: Option<u16>,
        /// TASK-040: mark an event every `every`, protect it for `hold`
        /// after its window, and release odd-numbered marks after `release`.
        events: Option<Events>,
    }
    #[derive(Clone, Copy)]
    struct Events {
        every: Duration,
        hold: Duration,
        release: Duration,
    }
    impl Options {
        fn parse(rest: &[&str]) -> Result<Self, Error> {
            let mut options = Self::default();
            let mut it = rest.iter();
            while let Some(flag) = it.next() {
                let value = it.next().ok_or_else(|| invalid(USAGE))?;
                match *flag {
                    "--sample-ms" => options.sample_ms = Some(value.parse()?),
                    "--reserve-mib" => options.reserve_mib = value.parse()?,
                    "--remote" => options.remote = Some(value.parse()?),
                    "--events" => {
                        let parts: Vec<u64> =
                            value.split(',').map(str::parse).collect::<Result<_, _>>()?;
                        let [every, hold, release] = parts[..] else {
                            return Err(invalid(USAGE));
                        };
                        if every == 0 || hold == 0 || release == 0 {
                            return Err(invalid(USAGE));
                        }
                        options.events = Some(Events {
                            every: Duration::from_secs(every),
                            hold: Duration::from_secs(hold),
                            release: Duration::from_secs(release),
                        });
                    }
                    _ => return Err(invalid(USAGE)),
                }
            }
            if options.remote.is_some() && !cfg!(feature = "remote") {
                return Err(invalid("--remote needs the remote feature"));
            }
            Ok(options)
        }
    }

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
        super::heap::rt_begin(0);
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
        super::heap::rt_begin(1);
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
    /// Closed `.mcap` files and, with `active`, unfinished `.active` ones.
    fn log_files(dir: &Path, active: bool) -> io::Result<Vec<PathBuf>> {
        let mut all = Vec::new();
        files(dir, &mut all)?;
        all.retain(|p| {
            p.extension()
                .is_some_and(|e| e == "mcap" || (active && e == "active"))
        });
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

    fn run(
        parent: &Path,
        seconds: u64,
        sample_s: u64,
        ring_mib: u64,
        options: &Options,
    ) -> Result<(), Error> {
        let rt = rt_mode()?;
        if rt {
            perf::lock_memory()?;
        }
        let mut retention = Retention::ring(ring_mib << 20).reserve(options.reserve_mib << 20);
        if let Some(events) = options.events {
            // 30 s either side of each mark; up to EVENT_LIMIT marks and
            // half the ring protected at once (the ring keeps the rest).
            retention = retention.events(
                EventWindow::new(EVENT_SIDE, EVENT_SIDE)
                    .hold(events.hold)
                    .limits(EVENT_LIMIT, (ring_mib << 20) / 2),
            );
        }
        let builder = Logger::builder()
            .preset(Preset::sensor(FRAME_HZ, FRAME_BYTES).small(CONTROL_HZ, CONTROL_BYTES))
            .file_output(parent, retention)
            .channel::<Control>("soak/control")
            .channel::<[u8]>("soak/frame")
            .writer("control")
            .writer("frame");
        #[cfg(feature = "remote")]
        let builder = match options.remote {
            Some(port) => builder.remote(dot_logger::adapters::remote::ServerConfig {
                enabled: true,
                address: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                ..Default::default()
            }),
            None => builder,
        };
        let logger = builder.start()?;
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
            "reserve_mib":options.reserve_mib,"remote":options.remote,
            "seconds":seconds,"dirs":dirs,
            "load":{"control_hz":CONTROL_HZ,"control_bytes":CONTROL_BYTES,
                "frame_hz":FRAME_HZ,"frame_bytes":FRAME_BYTES}}))?;
        let workers = [
            {
                let (stats, stop) = (stats.clone(), stop.clone());
                thread::Builder::new()
                    .name("soak-control".into())
                    .spawn(move || control_loop(control_writer, control, &stats, &stop, rt))?
            },
            {
                let (stats, stop) = (stats.clone(), stop.clone());
                thread::Builder::new()
                    .name("soak-frame".into())
                    .spawn(move || frame_loop(frame_writer, frame, &stats, &stop))?
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
        let sample = options
            .sample_ms
            .map_or(Duration::from_secs(sample_s.max(1)), |ms| {
                Duration::from_millis(ms.max(10))
            });
        let mut next_sample = started + sample;
        let mut next_mark = options.events.map(|e| started + e.every);
        let mut marks = 0_u64;
        let mut releases: Vec<(Instant, u64, EventMark)> = Vec::new();
        let alarm = loop {
            let now = Instant::now();
            if now >= end {
                break None;
            }
            // Releases first, so a mark due at the same time finds room.
            while let Some(index) = releases.iter().position(|(at, _, _)| now >= *at) {
                let (_, n, mark) = releases.swap_remove(index);
                let result = logger.release_event(mark);
                emit(json!({"event":"release","n":n,
                    "t_ms":started.elapsed().as_millis() as u64,
                    "ok":result.is_ok(),"error":result.err().map(|e| format!("{e:?}"))}))?;
            }
            if let (Some(at), Some(events)) = (next_mark, options.events) {
                if now >= at {
                    next_mark = Some(at + events.every);
                    marks += 1;
                    let result = logger.mark_event();
                    emit(json!({"event":"mark","n":marks,
                        "t_ms":started.elapsed().as_millis() as u64,
                        "control":stats.control.load(Ordering::Acquire),
                        "ok":result.is_ok(),"error":result.as_ref().err().map(|e| format!("{e:?}"))}))?;
                    if let (Ok(mark), true) = (result, marks % 2 == 1) {
                        releases.push((now + events.release, marks, mark));
                    }
                }
            }
            let due = releases
                .iter()
                .map(|(at, _, _)| *at)
                .chain(next_mark)
                .fold(next_sample.min(end), Instant::min);
            let wait = due.saturating_duration_since(Instant::now());
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
                let health = logger.health();
                let outputs: Vec<Value> = health
                    .outputs
                    .iter()
                    .map(|o| {
                        json!({"failed_records":o.failed_records,"failed":o.failed,
                            "since_sync_ms":o.since_sync.map(|d| d.as_millis() as u64)})
                    })
                    .collect();
                emit(json!({"event":"sample","t_s":started.elapsed().as_secs(),
                    "t_ms":started.elapsed().as_millis() as u64,"outputs":outputs,
                    "rss_kib":status_kib("VmRSS:"),"hwm_kib":status_kib("VmHWM:"),
                    "rss_anon_kib":status_kib("RssAnon:"),"rss_file_kib":status_kib("RssFile:"),
                    "heap":super::heap::sample(),"fds":fds(),"disk_bytes":bytes,"files":count,
                    "health":format!("{:?}",health.state),
                    "worker_failed":logger.worker_failed(),"counts":counts}))?;
            }
        };
        if alarm.is_none() {
            // Still under load: how much of the RSS is free heap glibc keeps.
            let rss = |f| status_kib(f);
            let before = json!({"rss_kib":rss("VmRSS:"),"rss_anon_kib":rss("RssAnon:"),
                "heap":super::heap::sample()});
            let released = super::heap::trim();
            emit(json!({"event":"trim","released":released,"before":before,
                "after":{"rss_kib":rss("VmRSS:"),"rss_anon_kib":rss("RssAnon:"),
                    "heap":super::heap::sample()}}))?;
        }
        // RT threads must not touch the heap once warm (TASK-039); read
        // before they stop, so thread exit is not counted.
        let rt_heap = super::heap::rt_since_begin();
        stop.store(true, Ordering::Release);
        let mut failures = Vec::new();
        if alarm.is_none() && (rt_heap["allocs"] != json!(0) || rt_heap["frees"] != json!(0)) {
            failures.push(format!("RT threads used the heap under load: {rt_heap}"));
        }
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
        // Producers are stopped; let records still in the pipeline settle so
        // the failed count compares with the shutdown report (TASK-024 S2).
        let mut health = logger.health();
        let mut steady = 0;
        for _ in 0..50 {
            thread::sleep(Duration::from_millis(100));
            let next = logger.health();
            let same = next
                .outputs
                .iter()
                .map(|o| o.failed_records)
                .eq(health.outputs.iter().map(|o| o.failed_records));
            health = next;
            steady = if same { steady + 1 } else { 0 };
            if steady == 5 {
                break;
            }
        }
        let outputs: Vec<Value> = health
            .outputs
            .iter()
            .map(|o| json!({"failed_records":o.failed_records,"failed":o.failed}))
            .collect();
        let shutdown = logger
            .shutdown(Duration::from_secs(30))
            .map_err(|e| invalid(format!("shutdown: {e:?}")))?;
        let (bytes, count) = disk(&dirs);
        emit(json!({"event":"end","success":shutdown.is_success(),
            "elapsed_s":started.elapsed().as_secs(),"counts":stats.counts(),
            "failures":failures,"rt_heap":rt_heap,"disk_bytes":bytes,"files":count,
            "health":format!("{:?}",health.state),"outputs":outputs,
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
        /// Loss parts that failed to decode in closed files (a failure), and
        /// in `.active` files (a killed writer's torn tail; observed only).
        loss_errors: u64,
        loss_errors_active: u64,
        /// `.active` files read (only with `--active`) and how many end torn.
        active_files: usize,
        torn_active: usize,
        /// Latest cumulative checkpoint per observer: id and "scope/cause" sums.
        losses: std::collections::BTreeMap<u32, (u64, std::collections::BTreeMap<String, u64>)>,
    }
    /// Missing indices: oldest prefix (allowed for a ring) and gaps after it.
    fn missing(seen: &[bool]) -> (usize, usize) {
        let prefix = seen.iter().take_while(|s| !**s).count();
        let gaps = seen[prefix..].iter().filter(|s| !**s).count();
        (prefix, gaps)
    }
    fn scan(dir: &Path, control: u64, frames: u64) -> Result<Found, Error> {
        scan_files(dir, control, frames, false)
    }
    fn scan_files(dir: &Path, control: u64, frames: u64, active: bool) -> Result<Found, Error> {
        let mut found = Found {
            control: vec![false; usize::try_from(control)?],
            frames: vec![false; usize::try_from(frames)?],
            ..Found::default()
        };
        for path in log_files(dir, active)? {
            let is_active = path.extension().is_some_and(|e| e == "active");
            if is_active {
                found.active_files += 1;
            } else {
                found.files += 1;
            }
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
                    // A killed writer leaves a torn tail; recovery drops it.
                    if is_active {
                        found.torn_active += 1;
                    } else {
                        found.torn_files += 1;
                    }
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
            // Decode errors are counted, never skipped (REV-073 RV-206).
            let mut loss_error = false;
            match LossReader::new(&bytes, FRAME_BYTES, MAX_PRODUCERS) {
                Err(_) => loss_error = true,
                Ok(losses) => {
                    for report in losses {
                        let Ok(report) = report else {
                            loss_error = true;
                            break;
                        };
                        found.loss_reports += 1;
                        found.nonzero_loss_rows +=
                            report.counts.iter().filter(|c| c.count != 0).count() as u64;
                        let latest = found
                            .losses
                            .entry(report.observer)
                            .or_insert_with(|| (report.checkpoint, Default::default()));
                        if report.checkpoint > latest.0 {
                            *latest = (report.checkpoint, Default::default());
                        }
                        if report.checkpoint == latest.0 {
                            for c in &report.counts {
                                *latest
                                    .1
                                    .entry(format!("{:?}/{:?}", c.scope, c.cause))
                                    .or_default() += c.count;
                            }
                        }
                    }
                }
            }
            if loss_error && is_active {
                found.loss_errors_active += 1;
            } else if loss_error {
                found.loss_errors += 1;
            }
        }
        Ok(found)
    }
    /// Indices seen: count, first, last, and holes between first and last.
    fn span(seen: &[bool]) -> Value {
        let first = seen.iter().position(|s| *s);
        let last = seen.iter().rposition(|s| *s);
        let holes = match (first, last) {
            (Some(a), Some(b)) => seen[a..=b].iter().filter(|s| !**s).count(),
            _ => 0,
        };
        json!({"seen":seen.iter().filter(|s| **s).count(),"first":first,"last":last,
            "holes":holes})
    }
    /// Contiguous index runs `[first, last]`, at most `MAX_RUNS` of them:
    /// event-protected islands before the ring's suffix (TASK-040).
    fn runs(seen: &[bool]) -> Value {
        const MAX_RUNS: usize = 256;
        let mut out = Vec::new();
        let mut start = None;
        for (i, s) in seen.iter().enumerate() {
            match (start, *s) {
                (None, true) => start = Some(i),
                (Some(a), false) => {
                    out.push(json!([a, i - 1]));
                    start = None;
                }
                _ => {}
            }
            if out.len() == MAX_RUNS {
                break;
            }
        }
        if let (Some(a), true) = (start, out.len() < MAX_RUNS) {
            out.push(json!([a, seen.len() - 1]));
        }
        Value::Array(out)
    }
    fn inspect(dir: &Path, control: u64, frames: u64, active: bool) -> Result<(), Error> {
        let found = scan_files(dir, control, frames, active)?;
        let losses: serde_json::Map<String, Value> = found
            .losses
            .iter()
            .map(|(observer, (checkpoint, sums))| {
                (
                    observer.to_string(),
                    json!({"checkpoint":checkpoint,"counts":sums}),
                )
            })
            .collect();
        emit(json!({"event":"inspect","dir":dir,"active":active,
            "found":found_json(&found),"active_files":found.active_files,
            "torn_active":found.torn_active,"losses":losses,
            "control":span(&found.control),"frames":span(&found.frames),
            "runs":{"control":runs(&found.control),"frames":runs(&found.frames)}}))
    }

    /// One external reader pass's findings (TASK-040).
    #[derive(Default)]
    struct Pass {
        files: u64,
        bytes: u64,
        vanished: u64,
        deleted_while_open: u64,
        unreadable: u64,
        torn: u64,
        corrupt: u64,
        foreign: u64,
        file_holes: u64,
        control: u64,
        frames: u64,
        nonzero_loss_rows: u64,
        loss_errors: u64,
    }
    impl Pass {
        fn intact(&self) -> bool {
            self.unreadable
                + self.torn
                + self.corrupt
                + self.foreign
                + self.file_holes
                + self.nonzero_loss_rows
                + self.loss_errors
                == 0
        }
        fn json(&self) -> Value {
            json!({"files":self.files,"bytes":self.bytes,"vanished":self.vanished,
                "deleted_while_open":self.deleted_while_open,"unreadable":self.unreadable,
                "torn":self.torn,"corrupt":self.corrupt,"foreign":self.foreign,
                "file_holes":self.file_holes,"control":self.control,"frames":self.frames,
                "nonzero_loss_rows":self.nonzero_loss_rows,"loss_errors":self.loss_errors})
        }
    }

    /// Read every closed file newest first, as an operator tool would while
    /// the logger writes and the ring deletes. Each file must be whole and
    /// hold contiguous records per channel; files the ring removed before
    /// they were opened, or while they were being read, are only counted.
    fn read_pass(dir: &Path) -> Result<Pass, Error> {
        let mut pass = Pass::default();
        let mut paths = log_files(dir, false)?;
        paths.reverse();
        for path in paths {
            let mut file = match fs::File::open(&path) {
                Ok(file) => file,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    pass.vanished += 1;
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let mut bytes = Vec::new();
            io::Read::read_to_end(&mut file, &mut bytes)?;
            drop(file);
            if !path.exists() {
                pass.deleted_while_open += 1;
            }
            pass.files += 1;
            pass.bytes += bytes.len() as u64;
            let Ok(reader) = NativeReader::new(&bytes, FRAME_BYTES, MAX_PRODUCERS) else {
                pass.unreadable += 1;
                continue;
            };
            let topics: Vec<(u32, String)> = reader
                .channels()
                .iter()
                .map(|c| (c.channel.channel_id, c.topic.clone()))
                .collect();
            let mut last: [Option<u64>; 2] = [None, None];
            for item in reader {
                let Ok(stored) = item else {
                    pass.torn += 1;
                    break;
                };
                let id = stored.record.metadata.channel_id;
                let topic = topics
                    .iter()
                    .find(|(c, _)| *c == id)
                    .map(|(_, t)| t.as_str());
                let (slot, len, step) = match topic {
                    Some("soak/control") => (0, CONTROL_BYTES, CONTROL_STEP),
                    Some("soak/frame") => (1, FRAME_BYTES, FRAME_STEP),
                    _ => {
                        pass.foreign += 1;
                        continue;
                    }
                };
                let Some(index) = index_of(stored.payload, len, step) else {
                    pass.corrupt += 1;
                    continue;
                };
                if last[slot].is_some_and(|prev| index != prev + 1) {
                    pass.file_holes += 1;
                }
                last[slot] = Some(index);
                if slot == 0 {
                    pass.control += 1;
                } else {
                    pass.frames += 1;
                }
            }
            // Decode errors are counted, never skipped (REV-073 RV-206).
            match LossReader::new(&bytes, FRAME_BYTES, MAX_PRODUCERS) {
                Ok(losses) => {
                    for report in losses {
                        let Ok(report) = report else {
                            pass.loss_errors += 1;
                            break;
                        };
                        pass.nonzero_loss_rows +=
                            report.counts.iter().filter(|c| c.count != 0).count() as u64;
                    }
                }
                Err(_) => pass.loss_errors += 1,
            }
        }
        Ok(pass)
    }

    fn reader(dir: &Path, interval: Duration, seconds: u64) -> Result<(), Error> {
        let started = Instant::now();
        let end = started + Duration::from_secs(seconds);
        let (mut passes, mut failed) = (0_u64, 0_u64);
        let mut total = Pass::default();
        let mut min_free = u64::MAX;
        while Instant::now() < end {
            let begun = Instant::now();
            let pass = read_pass(dir)?;
            let free = dot_logger::diagnostics::available_bytes(dir).unwrap_or(0);
            min_free = min_free.min(free);
            passes += 1;
            if !pass.intact() {
                failed += 1;
            }
            emit(
                json!({"event":"pass","n":passes,"t_ms":started.elapsed().as_millis() as u64,
                "read_ms":begun.elapsed().as_millis() as u64,"intact":pass.intact(),
                "free_bytes":free,"found":pass.json()}),
            )?;
            for (sum, add) in [
                (&mut total.files, pass.files),
                (&mut total.bytes, pass.bytes),
                (&mut total.vanished, pass.vanished),
                (&mut total.deleted_while_open, pass.deleted_while_open),
                (&mut total.unreadable, pass.unreadable),
                (&mut total.torn, pass.torn),
                (&mut total.corrupt, pass.corrupt),
                (&mut total.foreign, pass.foreign),
                (&mut total.file_holes, pass.file_holes),
                (&mut total.control, pass.control),
                (&mut total.frames, pass.frames),
                (&mut total.nonzero_loss_rows, pass.nonzero_loss_rows),
                (&mut total.loss_errors, pass.loss_errors),
            ] {
                *sum += add;
            }
            let next = (begun + interval).min(end);
            thread::sleep(next.saturating_duration_since(Instant::now()));
        }
        emit(
            json!({"event":"reader_end","passes":passes,"failed_passes":failed,
            "min_free_bytes":min_free,"total":total.json()}),
        )?;
        if failed == 0 && passes > 0 {
            Ok(())
        } else {
            Err(invalid("a reader pass found a damaged file"))
        }
    }
    fn found_json(found: &Found) -> Value {
        let (control_prefix, control_gaps) = missing(&found.control);
        let (frame_prefix, frame_gaps) = missing(&found.frames);
        json!({"files":found.files,"unreadable_files":found.unreadable_files,
            "torn_files":found.torn_files,"foreign":found.foreign,
            "duplicates":found.duplicates,"corrupt_payloads":found.corrupt_payloads,
            "loss_reports":found.loss_reports,"nonzero_loss_rows":found.nonzero_loss_rows,
            "loss_errors":found.loss_errors,"loss_errors_active":found.loss_errors_active,
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
            && found.loss_errors == 0
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
            ["run", parent, seconds, sample, ring, rest @ ..] => run(
                Path::new(parent),
                seconds.parse()?,
                sample.parse()?,
                ring.parse()?,
                &Options::parse(rest)?,
            ),
            ["inspect", dir, control, frames] => {
                inspect(Path::new(dir), control.parse()?, frames.parse()?, false)
            }
            ["inspect", dir, control, frames, "--active"] => {
                inspect(Path::new(dir), control.parse()?, frames.parse()?, true)
            }
            ["power", parent, delays] => power(Path::new(parent), delays),
            ["reader", dir, interval, seconds] => reader(
                Path::new(dir),
                Duration::from_secs(interval.parse()?),
                seconds.parse()?,
            ),
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
