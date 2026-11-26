//! TASK-023 P0 baseline runs for local-pc-v1: calibration, control loops with
//! the logger off, and the paired simple-file writer. One JSON line per fact.
#[cfg(target_os = "linux")]
#[path = "support/perf/mod.rs"]
mod perf;
#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    match linux::execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("perf baseline: {error}");
            std::process::ExitCode::from(4)
        }
    }
}
#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("perf baseline: Linux only");
    std::process::ExitCode::from(2)
}
#[cfg(target_os = "linux")]
mod linux {
    use super::perf::{self, *};
    use serde_json::{Value, json};
    use std::{
        io::{self, Write},
        path::Path,
        time::Duration,
    };
    // local-pc-v1 [control], [storage] and worker CPUs.
    const PERIOD: Duration = Duration::from_millis(10);
    const CONTROL_CPU: usize = 2;
    const SAFETY_CPU: usize = 3;
    const CONTROL_WORK: Duration = Duration::from_micros(500);
    const SAFETY_WORK: Duration = Duration::from_micros(200);
    const WORKER_CPUS: [usize; 8] = [4, 5, 6, 7, 12, 13, 14, 15];
    /// local-pc-v1 RT rows: SCHED_FIFO control 70 / safety 80 with all
    /// memory locked, selected by `DOT_LOGGER_PERF_RT=fifo` (TASK-022).
    const CONTROL_FIFO: u8 = 70;
    const SAFETY_FIFO: u8 = 80;
    fn rt_mode() -> Result<bool, Error> {
        match std::env::var("DOT_LOGGER_PERF_RT") {
            Err(std::env::VarError::NotPresent) => Ok(false),
            Ok(v) if v == "fifo" => Ok(true),
            _ => Err("DOT_LOGGER_PERF_RT must be unset or \"fifo\"".into()),
        }
    }
    fn fifo(priority: u8) -> Result<Option<u8>, Error> {
        Ok(rt_mode()?.then_some(priority))
    }
    /// Rotation of both tools: the facade segment ceiling (API §19), so P0
    /// and logger runs share the same file limit (TASK-023 S2b).
    const FILE_LIMIT: u64 = 256 << 20;
    const BLOCK: usize = 1 << 20;
    const SYNC: Duration = Duration::from_secs(1);
    const RESERVE: u64 = 64 << 30;
    const USAGE: &str = "usage: perf_baseline --plan | --calibrate | \
        --control <seconds> <control_iters> <safety_iters> | \
        --p0 <parent> <seconds> <warmup_s> <payload> <bytes_per_second> <files> <writers> <control_iters> <safety_iters> | \
        --p0-max <parent> <total_bytes> <payload> <files> <writers> | \
        --p1 <parent> <seconds> <warmup_s> <payload> <bytes_per_second> <files> <writers> <producers> <channels> <slots> <memory_mib> <telemetry> <control_iters> <safety_iters> | \
        --p1-max <parent> <total_bytes> <payload> <files> <writers> <producers> <channels> <slots> <memory_mib> | \
        --p1-cost <parent> <payload> <samples> | \
        --latency <tmpfs_parent> warm|cold|reject <payload> <calls> | \
        --latency <tmpfs_parent> sparse <payload> <hz> <seconds> <between_bytes> | \
        --latency <tmpfs_parent> multi <payload> <calls> <writers> | \
        --latency <tmpfs_parent> jitter <payload> <period_us> <seconds> <work_us> <per_release> [<load_dir> <load_bytes> <load_per_second>] | \
        --sustained <parent> <max_bytes> <files> <writers> <stable_s> | \
        --p0-mix-max <parent> <total_bytes> <small_bytes> <small_per_second> <large_bytes> <large_per_second> <streams> <files> <writers> | \
        --p1-mix-max <parent> <total_bytes> <small_bytes> <small_per_second> <large_bytes> <large_per_second> <streams> <files> <writers> <producers> <channels> <slots> <raw_slots> <memory_mib> <telemetry> | \
        --p0-mix <parent> <seconds> <warmup_s> <small_bytes> <small_per_second> <large_bytes> <large_per_second> <streams> <files> <writers> <control_iters> <safety_iters> | \
        --p1-mix <parent> <seconds> <warmup_s> <small_bytes> <small_per_second> <large_bytes> <large_per_second> <streams> <files> <writers> <producers> <channels> <slots> <raw_slots> <memory_mib> <telemetry> <control_iters> <safety_iters>";

    pub fn execute() -> Result<(), Error> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut out = io::stdout().lock();
        if rt_mode()? {
            perf::lock_memory()?;
        }
        match a.as_slice() {
            ["--plan"] => line(
                &mut out,
                json!({"scope":"P0Plan","profile":"local-pc-v1","period_us":10000,"deadline_us":10000,
                    "control":{"cpu":CONTROL_CPU,"work_us":500},"safety":{"cpu":SAFETY_CPU,"work_us":200},
                    "worker_cpus":WORKER_CPUS,"file_limit_bytes":FILE_LIMIT,"block_bytes":BLOCK,
                    "sync_interval_ms":1000,"final_file_and_directory_sync":true,"reserve_bytes":RESERVE,
                    "policy":if rt_mode()? {"SCHED_FIFO"} else {"SCHED_OTHER"},
                    "fifo_priority":{"control":fifo(CONTROL_FIFO)?,"safety":fifo(SAFETY_FIFO)?},
                    "memory_locked":rt_mode()?,"kernel":kernel()?,"realtime":perf::realtime_kernel(),
                    "throughput_denominator":"final file and directory sync",
                    "ratio_requires":"P0 run of the identical workload","repetitions":REPETITIONS}),
            ),
            ["--calibrate"] => {
                let control = calibrated(CONTROL_CPU, CONTROL_WORK, fifo(CONTROL_FIFO)?)?;
                let safety = calibrated(SAFETY_CPU, SAFETY_WORK, fifo(SAFETY_FIFO)?)?;
                let c = |cal: Calibration| {
                    json!({"iterations":cal.iterations,
                    "median_ns":cal.median_ns,"rounds":cal.rounds})
                };
                line(
                    &mut out,
                    json!({"scope":"P0Calibration","kernel":kernel()?,"logger":"off",
                        "realtime":perf::realtime_kernel(),"rt_mode":rt_mode()?,
                        "governor":governor(CONTROL_CPU),"releases_per_round":100,
                        "control":{"cpu":CONTROL_CPU,"work_us":500,"result":c(control)},
                        "safety":{"cpu":SAFETY_CPU,"work_us":200,"result":c(safety)}}),
                )
            }
            ["--control", seconds, control, safety] => {
                let ((c, ()), s) = loops(
                    seconds.parse()?,
                    control.parse()?,
                    safety.parse()?,
                    0,
                    Duration::ZERO,
                    |_| Ok(()),
                )?;
                line(&mut out, control_json("control", "off", &c))?;
                line(&mut out, control_json("safety", "off", &s))
            }
            [
                "--p0",
                parent,
                seconds,
                warmup,
                payload,
                rate,
                files,
                writers,
                control,
                safety,
            ] => {
                let seconds: u64 = seconds.parse()?;
                let w = paced(seconds, warmup, payload, rate, files, writers)?;
                let dir = RunDir::create(Path::new(parent))?;
                let ((c, run), s) = loops(
                    seconds,
                    control.parse()?,
                    safety.parse()?,
                    w.writers,
                    w.warmup,
                    |gate| write_p0(dir.path(), w, RESERVE, &WORKER_CPUS, gate),
                )?;
                line(&mut out, control_json("control", "p0", &c))?;
                line(&mut out, control_json("safety", "p0", &s))?;
                line(&mut out, window_json("p0", &c, &s, &run))?;
                line(&mut out, storage_json(dir.path(), &run)?)
            }
            ["--p0-max", parent, total, payload, files, writers] => {
                let payload: usize = payload.parse()?;
                let w = workload(
                    payload,
                    records_for(total.parse::<u64>()?, payload)?,
                    files.parse()?,
                    writers.parse()?,
                    None,
                );
                let dir = RunDir::create(Path::new(parent))?;
                let run = write_p0(
                    dir.path(),
                    w,
                    RESERVE,
                    &WORKER_CPUS,
                    &Gate::new(w.writers, 0),
                )?;
                line(&mut out, storage_json(dir.path(), &run)?)
            }
            [
                "--p1",
                parent,
                seconds,
                warmup,
                payload,
                rate,
                files,
                writers,
                producers,
                channels,
                slots,
                memory,
                telemetry,
                control,
                safety,
            ] => {
                let seconds: u64 = seconds.parse()?;
                let w = paced(seconds, warmup, payload, rate, files, writers)?;
                let mut load = logger_load(producers, channels, slots, memory)?;
                load.telemetry = flag(telemetry)?;
                let dir = RunDir::create(Path::new(parent))?;
                let ((c, run), s) = loops(
                    seconds,
                    control.parse()?,
                    safety.parse()?,
                    load.producers,
                    w.warmup,
                    |gate| write_logger(dir.path(), w, &load, RESERVE, gate),
                )?;
                line(&mut out, control_json("control", "on", &c))?;
                line(&mut out, control_json("safety", "on", &s))?;
                line(&mut out, window_json("on", &c, &s, &run.storage))?;
                line(&mut out, logger_json(&run)?)
            }
            [
                "--p1-max",
                parent,
                total,
                payload,
                files,
                writers,
                producers,
                channels,
                slots,
                memory,
            ] => {
                let payload: usize = payload.parse()?;
                let w = workload(
                    payload,
                    records_for(total.parse::<u64>()?, payload)?,
                    files.parse()?,
                    writers.parse()?,
                    None,
                );
                let load = logger_load(producers, channels, slots, memory)?;
                let dir = RunDir::create(Path::new(parent))?;
                let gate = Gate::new(load.producers, 0);
                let run = write_logger(dir.path(), w, &load, RESERVE, &gate)?;
                line(&mut out, logger_json(&run)?)
            }
            // TASK-023 S2c: P2 originals (`small_per_second` 0) and the P3
            // mix, paced, beside the control loops.
            [
                "--p0-mix",
                parent,
                seconds,
                warmup,
                small,
                small_rate,
                large,
                large_rate,
                streams,
                files,
                writers,
                control,
                safety,
            ] => {
                let seconds: u64 = seconds.parse()?;
                let w = mixed_workload(
                    seconds, warmup, small, small_rate, large, large_rate, streams, files, writers,
                )?;
                let dir = RunDir::create(Path::new(parent))?;
                let ((c, run), s) = loops(
                    seconds,
                    control.parse()?,
                    safety.parse()?,
                    w.writers,
                    w.warmup,
                    |gate| write_p0(dir.path(), w, RESERVE, &WORKER_CPUS, gate),
                )?;
                line(&mut out, control_json("control", "p0", &c))?;
                line(&mut out, control_json("safety", "p0", &s))?;
                line(&mut out, window_json("p0", &c, &s, &run))?;
                line(&mut out, storage_json(dir.path(), &run)?)
            }
            [
                "--p1-mix",
                parent,
                seconds,
                warmup,
                small,
                small_rate,
                large,
                large_rate,
                streams,
                files,
                writers,
                producers,
                channels,
                slots,
                raw_slots,
                memory,
                telemetry,
                control,
                safety,
            ] => {
                let seconds: u64 = seconds.parse()?;
                let w = mixed_workload(
                    seconds, warmup, small, small_rate, large, large_rate, streams, files, writers,
                )?;
                let mut load = logger_load(producers, channels, slots, memory)?;
                load.raw_slots = raw_slots.parse()?;
                load.telemetry = flag(telemetry)?;
                let dir = RunDir::create(Path::new(parent))?;
                let ((c, run), s) = loops(
                    seconds,
                    control.parse()?,
                    safety.parse()?,
                    load.producers,
                    w.warmup,
                    |gate| write_logger(dir.path(), w, &load, RESERVE, gate),
                )?;
                line(&mut out, control_json("control", "on", &c))?;
                line(&mut out, control_json("safety", "on", &s))?;
                line(&mut out, window_json("on", &c, &s, &run.storage))?;
                line(&mut out, logger_json(&run)?)
            }
            // Long-run rule (Human 2026-11-08): the device's write rate after
            // its SLC cache ran out.
            ["--sustained", parent, max_bytes, files, writers, stable] => {
                let dir = RunDir::create(Path::new(parent))?;
                let run = measure_sustained(
                    dir.path(),
                    files.parse()?,
                    writers.parse()?,
                    BLOCK,
                    max_bytes.parse()?,
                    stable.parse()?,
                    &WORKER_CPUS,
                )?;
                let s = run.sustained;
                line(
                    &mut out,
                    json!({"scope":"Sustained","files":files.parse::<usize>()?,"writers":writers.parse::<usize>()?,
                        "block_bytes":BLOCK,"total_bytes":run.total_bytes.to_string(),
                        "peak_bytes_per_second":s.peak_bytes_per_second.to_string(),
                        "drop_at_s":s.drop_at,"bytes_per_second":s.bytes_per_second.to_string(),
                        "stable":s.stable,"series":run.series}),
                )
            }
            // Efficiency: the unpaced mix on both tools.
            [
                "--p0-mix-max",
                parent,
                total,
                small,
                small_rate,
                large,
                large_rate,
                streams,
                files,
                writers,
            ] => {
                let w = unpaced_mix(
                    total, small, small_rate, large, large_rate, streams, files, writers,
                )?;
                let dir = RunDir::create(Path::new(parent))?;
                let run = write_p0(
                    dir.path(),
                    w,
                    RESERVE,
                    &WORKER_CPUS,
                    &Gate::new(w.writers, 0),
                )?;
                line(&mut out, storage_json(dir.path(), &run)?)
            }
            [
                "--p1-mix-max",
                parent,
                total,
                small,
                small_rate,
                large,
                large_rate,
                streams,
                files,
                writers,
                producers,
                channels,
                slots,
                raw_slots,
                memory,
                telemetry,
            ] => {
                let w = unpaced_mix(
                    total, small, small_rate, large, large_rate, streams, files, writers,
                )?;
                let mut load = logger_load(producers, channels, slots, memory)?;
                load.raw_slots = raw_slots.parse()?;
                load.telemetry = flag(telemetry)?;
                let dir = RunDir::create(Path::new(parent))?;
                let gate = Gate::new(load.producers, 0);
                let run = write_logger(dir.path(), w, &load, RESERVE, &gate)?;
                line(&mut out, logger_json(&run)?)
            }
            ["--p1-cost", parent, payload, samples] => {
                let dir = RunDir::create(Path::new(parent))?;
                let cost = std::thread::spawn({
                    let path = dir.path().to_owned();
                    let payload: usize = payload.parse()?;
                    let samples: u64 = samples.parse()?;
                    move || -> Result<CallCost, Error> {
                        perf::pin(CONTROL_CPU)?;
                        call_cost(&path, payload, samples)
                    }
                })
                .join()
                .map_err(|_| "call cost panicked")??;
                line(
                    &mut out,
                    json!({"scope":"P1CallCost","kernel":kernel()?,"cpu":CONTROL_CPU,
                        "governor":governor(CONTROL_CPU),"payload_bytes":cost.payload_bytes,
                        "try_log":latency(&cost.try_log),"encode":latency(&cost.encode),
                        "record_small":latency(&cost.record_small),"copy":latency(&cost.copy),
                        "clock":latency(&cost.clock),"success":cost.success,"report":cost.report}),
                )
            }
            ["--latency", parent, row, rest @ ..] => latency_row(&mut out, parent, row, rest),
            _ => Err(USAGE.into()),
        }
    }
    /// TASK-042 rows: CPUs of the measuring threads (the control CPU first,
    /// then other physical cores; SMT siblings are 8 apart on this PC).
    const MULTI_CPUS: [usize; 8] = [CONTROL_CPU, 3, 4, 5, 6, 7, 0, 1];
    const LATENCY_RING: u64 = 1 << 30;
    fn latency_row(
        out: &mut impl Write,
        parent: &str,
        row: &str,
        rest: &[&str],
    ) -> Result<(), Error> {
        let dir = RunDir::create(Path::new(parent))?;
        let output = dir.path().join("out-000");
        std::fs::create_dir(&output)?;
        let target = Target {
            dir: &output,
            ring: LATENCY_RING,
        };
        let rt = Rt {
            cpu: CONTROL_CPU,
            fifo: fifo(CONTROL_FIFO)?,
        };
        let env = json!({"scope":"Latency","row":row,"kernel":kernel()?,
            "realtime":perf::realtime_kernel(),"rt_mode":rt_mode()?,"fifo":rt.fifo,
            "cpu":CONTROL_CPU,"governor":governor(CONTROL_CPU),"smt":smt(),
            "ring_bytes":LATENCY_RING.to_string()});
        let number = |i: usize| -> Result<u64, Error> { Ok(rest.get(i).ok_or(USAGE)?.parse()?) };
        let payload = usize::try_from(number(0)?)?;
        let mut value = env;
        value["payload_bytes"] = json!(payload);
        match (row, rest.len()) {
            ("warm" | "cold" | "reject", 2) => {
                let calls = number(1)?;
                value["calls"] = json!(calls);
                let r = match row {
                    "warm" => warm(target, payload, calls, rt)?,
                    "cold" => {
                        let (r, swept) = cold(target, payload, calls, rt)?;
                        value["swept_bytes"] = json!(swept);
                        r
                    }
                    _ => reject(target, payload, calls, rt)?,
                };
                row_json(&mut value, &r);
            }
            ("sparse", 4) => {
                let (hz, seconds, between) = (number(1)?, number(2)?, number(3)?);
                let period = Duration::from_nanos(1_000_000_000 / hz.max(1));
                value["period_ns"] = json!(period.as_nanos().to_string());
                value["between_bytes"] = json!(between);
                let r = sparse(
                    target,
                    payload,
                    period,
                    hz * seconds,
                    usize::try_from(between)?,
                    rt,
                )?;
                row_json(&mut value, &r);
            }
            ("multi", 3) => {
                let (calls, writers) = (number(1)?, usize::try_from(number(2)?)?);
                let cpus = MULTI_CPUS.get(..writers).ok_or("at most 8 writers")?;
                value["calls"] = json!(calls);
                value["cpus"] = json!(cpus);
                let rows = multi(target, payload, calls, cpus, rt)?;
                let mut all = LatencyHistogram::default();
                for r in &rows {
                    all.merge(&r.accepted);
                }
                value["accepted"] = distribution(&all);
                value["writers"] = rows
                    .iter()
                    .map(|r| json!({"accepted":distribution(&r.accepted),"capacity":r.capacity}))
                    .collect();
                value["success"] = json!(rows.iter().all(|r| r.success));
            }
            ("jitter", 5 | 8) => {
                let period = Duration::from_micros(number(1)?);
                let seconds = number(2)?;
                let work = Duration::from_micros(number(3)?);
                let per_release = number(4)?;
                let releases = seconds * 1_000_000_000 / u64::try_from(period.as_nanos())?.max(1);
                let load_dir = rest.get(5).map(Path::new);
                let load = match load_dir {
                    Some(dir) => Some(IoLoad {
                        dir,
                        bytes: usize::try_from(number(6)?)?,
                        per_second: number(7)?,
                    }),
                    None => None,
                };
                value["period_ns"] = json!(period.as_nanos().to_string());
                value["work_ns"] = json!(work.as_nanos().to_string());
                value["per_release"] = json!(per_release);
                value["releases"] = json!(releases);
                value["load"] = json!(load.map(|l| json!({"dir":l.dir.display().to_string(),
                    "bytes":l.bytes,"per_second":l.per_second})));
                // TASK-042 diagnosis: record batches slower than this.
                let slow = std::env::var("DOT_LOGGER_PERF_SLOW_US")
                    .ok()
                    .map(|v| v.parse().map(Duration::from_micros))
                    .transpose()?;
                value["slow_threshold_ns"] = json!(slow.map(|d| d.as_nanos().to_string()));
                let j = jitter(
                    target,
                    payload,
                    period,
                    releases,
                    work,
                    per_release,
                    load,
                    slow,
                    rt,
                )?;
                value["slow"] = j
                    .slow
                    .iter()
                    .map(|b| {
                        json!({"release":b.release,"at_ns":b.at_ns.to_string(),"wake_ns":b.wake_ns,
                            "work_ns":b.work_ns,"seq":b.seq,
                            "batch_ns":b.batch_ns,"max_call_ns":b.max_call_ns,
                            "max_call_index":b.max_call_index,"calls_over_2us":b.calls_over_2us})
                    })
                    .collect();
                value["slow_dropped"] = json!(j.slow_dropped);
                value["wake"] = distribution(&j.wake);
                value["response"] = distribution(&j.response);
                value["calls_per_release"] = distribution(&j.calls);
                value["misses"] = json!(j.misses);
                value["capacity"] = json!(j.capacity);
                value["work_iterations"] = json!(j.work_iterations.to_string());
                value["load_records"] = json!(j.load_records);
                value["success"] = json!(j.success);
            }
            _ => return Err(USAGE.into()),
        }
        line(out, value)
    }
    fn row_json(value: &mut Value, r: &Row) {
        value["accepted"] = distribution(&r.accepted);
        value["rejected"] = distribution(&r.rejected);
        value["skipped"] = distribution(&r.skipped);
        value["clock_pair"] = distribution(&r.clock);
        value["capacity"] = json!(r.capacity);
        value["success"] = json!(r.success);
    }
    /// Quantiles are bucket upper bounds (≤ 1/32 above the true value); the
    /// maximum is exact; an empty histogram reports nulls.
    fn distribution(h: &LatencyHistogram) -> Value {
        let q = |q| h.quantile(q);
        json!({"count":h.count(),"p50_ns":q(0.5),"p90_ns":q(0.9),"p99_ns":q(0.99),
            "p999_ns":q(0.999),"p9999_ns":q(0.9999),"max_ns":(h.count() > 0).then(|| h.max())})
    }
    /// SMT state from sysfs ("on", "off", "forceoff", "notsupported").
    fn smt() -> Option<String> {
        std::fs::read_to_string("/sys/devices/system/cpu/smt/control")
            .ok()
            .map(|s| s.trim().to_owned())
    }
    fn workload(
        payload: usize,
        records: u64,
        files: usize,
        writers: usize,
        rate: Option<u64>,
    ) -> StorageWorkload {
        StorageWorkload {
            record_payload_bytes: payload,
            records,
            files,
            writers,
            file_limit_bytes: FILE_LIMIT,
            file_record_limit: FACADE_SEGMENT_RECORDS,
            block_bytes: BLOCK,
            sync_interval: SYNC,
            offered_bytes_per_second: rate,
            large: None,
            warmup: Duration::ZERO,
        }
    }
    /// `warmup` + `seconds` of `payload`-byte records at `rate` bytes/s; the
    /// first `warmup` seconds are left out of the results.
    fn paced(
        seconds: u64,
        warmup: &str,
        payload: &str,
        rate: &str,
        files: &str,
        writers: &str,
    ) -> Result<StorageWorkload, Error> {
        let warmup: u64 = warmup.parse()?;
        let payload: usize = payload.parse()?;
        let rate: u64 = rate.parse()?;
        let mut w = workload(
            payload,
            records_for(rate.saturating_mul(seconds + warmup), payload)?,
            files.parse()?,
            writers.parse()?,
            Some(rate),
        );
        w.warmup = Duration::from_secs(warmup);
        w.validate()?;
        Ok(w)
    }
    fn flag(value: &str) -> Result<bool, Error> {
        match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(USAGE.into()),
        }
    }
    /// `warmup` + `seconds` of `small_per_second` small and `large_per_second` large
    /// records: every `round(total / large)`-th record is large, rotating over
    /// `streams`, and the offered rate is the run's payload over `seconds`.
    #[allow(clippy::too_many_arguments)]
    fn mixed_workload(
        seconds: u64,
        warmup: &str,
        small: &str,
        small_rate: &str,
        large: &str,
        large_rate: &str,
        streams: &str,
        files: &str,
        writers: &str,
    ) -> Result<StorageWorkload, Error> {
        let warmup: u64 = warmup.parse()?;
        let total_seconds = seconds + warmup;
        let small_rate: u64 = small_rate.parse()?;
        let large_rate: u64 = large_rate.parse()?;
        let per_second = small_rate + large_rate;
        if seconds == 0 || large_rate == 0 {
            return Err(USAGE.into());
        }
        let mut w = workload(
            small.parse()?,
            per_second * total_seconds,
            files.parse()?,
            writers.parse()?,
            None,
        );
        w.large = Some(LargeRecords {
            bytes: large.parse()?,
            every: (per_second + large_rate / 2) / large_rate,
            streams: streams.parse()?,
        });
        w.offered_bytes_per_second = Some(w.total_payload_bytes() / total_seconds);
        w.warmup = Duration::from_secs(warmup);
        w.validate()?;
        Ok(w)
    }
    /// The paced mix's record ratio without pacing, sized to `total` payload
    /// bytes.
    #[allow(clippy::too_many_arguments)]
    fn unpaced_mix(
        total: &str,
        small: &str,
        small_rate: &str,
        large: &str,
        large_rate: &str,
        streams: &str,
        files: &str,
        writers: &str,
    ) -> Result<StorageWorkload, Error> {
        let mut w = mixed_workload(
            1, "0", small, small_rate, large, large_rate, streams, files, writers,
        )?;
        let per_second = w.records;
        let per_second_bytes = w.total_payload_bytes().max(1);
        w.records = (u128::from(total.parse::<u64>()?) * u128::from(per_second)
            / u128::from(per_second_bytes))
        .try_into()?;
        w.offered_bytes_per_second = None;
        w.validate()?;
        Ok(w)
    }
    /// Producers unpinned; logger IO and host threads on the worker CPUs.
    fn logger_load(
        producers: &str,
        channels: &str,
        slots: &str,
        memory_mib: &str,
    ) -> Result<LoggerLoad, Error> {
        Ok(LoggerLoad {
            producers: producers.parse()?,
            channels: channels.parse()?,
            slots: slots.parse()?,
            raw_slots: 0,
            memory_bytes: memory_mib
                .parse::<usize>()?
                .checked_mul(1 << 20)
                .ok_or("memory")?,
            producer_cpus: Vec::new(),
            worker_cpus: WORKER_CPUS.to_vec(),
            telemetry: false,
        })
    }
    /// Quantiles are bucket upper bounds (≤ 1/32 above the true value); the
    /// maximum is exact; an empty histogram reports nulls.
    fn latency(h: &LatencyHistogram) -> Value {
        let q = |q| h.quantile(q).map(|n| n.to_string());
        json!({"count":h.count().to_string(),"p50_ns":q(0.5),"p99_ns":q(0.99),
            "p999_ns":q(0.999),"max_ns":(h.count() > 0).then(|| h.max().to_string())})
    }
    /// A failed read-back is reported next to the run facts, never hidden.
    fn logger_json(r: &LoggerRun) -> Result<Value, Error> {
        let mut value = storage_facts(&r.storage);
        value["scope"] = json!("LoggerStorage");
        value["load"] = json!({"producers":r.load.producers,"channels":r.load.channels,
            "slots":r.load.slots,"memory_bytes":r.load.memory_bytes.to_string(),
            "producer_cpus":r.load.producer_cpus,"worker_cpus":r.load.worker_cpus,
            "telemetry":r.load.telemetry});
        value["required_bytes"] = json!(r.required_bytes.to_string());
        value["accepted_calls"] = latency(&r.accepted);
        value["raw_accepted_calls"] = latency(&r.raw_accepted);
        value["load"]["raw_slots"] = json!(r.load.raw_slots);
        value["rejected_calls"] = latency(&r.rejected);
        value["rejections"] = json!(r.rejections.to_string());
        value["report"] = json!(r.report);
        match verify_logger(r) {
            Ok(v) => {
                value["verified_records"] = json!(v.records.to_string());
                value["verified_payload_bytes"] = json!(v.payload_bytes.to_string());
                value["largest_file"] = json!(v.largest_file.to_string());
                value["largest_file_records"] = json!(v.largest_file_records.to_string());
            }
            Err(e) => value["verify_error"] = json!(e.to_string()),
        }
        Ok(value)
    }
    fn calibrated(
        cpu: usize,
        target: Duration,
        fifo_priority: Option<u8>,
    ) -> Result<Calibration, Error> {
        std::thread::spawn(move || {
            perf::pin(cpu)?;
            if let Some(priority) = fifo_priority {
                perf::set_fifo(priority)?;
            }
            calibrate_periodic(target, PERIOD, 100)
        })
        .join()
        .map_err(|_| "calibration panicked")?
    }
    type Loops<T> = ((ControlRun, T), ControlRun);
    /// Control and safety loops on their CPUs for `seconds`, concurrently with
    /// `load` on the calling thread, from the end of its `warmup`. All of
    /// them start together once the
    /// loops and the load's `load_parties` threads finished their setup
    /// (REV-052 RV-130); a load that fails first releases the loops.
    fn loops<T: Send>(
        seconds: u64,
        control: u64,
        safety: u64,
        load_parties: usize,
        warmup: Duration,
        load: impl FnOnce(&Gate) -> Result<T, Error>,
    ) -> Result<Loops<T>, Error> {
        let releases = seconds * 1_000_000_000 / PERIOD.as_nanos() as u64;
        let (control_fifo, safety_fifo) = (fifo(CONTROL_FIFO)?, fifo(SAFETY_FIFO)?);
        let config = |cpu, work_iterations, fifo_priority| ControlConfig {
            cpu: Some(cpu),
            period: PERIOD,
            deadline: PERIOD,
            work_iterations,
            fifo_priority,
        };
        let gate = Gate::new(load_parties, 2);
        std::thread::scope(|scope| {
            let gate = &gate;
            // The loops cover the measured window, after the load's warm-up.
            let c = scope.spawn(move || {
                run_control(
                    config(CONTROL_CPU, control, control_fifo),
                    releases,
                    gate,
                    warmup,
                )
            });
            let s = scope.spawn(move || {
                run_control(
                    config(SAFETY_CPU, safety, safety_fifo),
                    releases,
                    gate,
                    warmup,
                )
            });
            let loaded = load(gate);
            if loaded.is_err() {
                gate.abort();
            }
            let c = c.join().map_err(|_| "control panicked")?;
            let s = s.join().map_err(|_| "safety panicked")?;
            // The load's own failure first: the loops then only report
            // that the start was released.
            let loaded = loaded?;
            Ok(((c?, loaded), s?))
        })
    }
    /// Recorded, never changed: frequency scaling shapes the fixed work time.
    fn governor(cpu: usize) -> Option<String> {
        std::fs::read_to_string(format!(
            "/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor"
        ))
        .ok()
        .map(|g| g.trim().to_owned())
    }
    fn control_json(name: &str, logger: &str, r: &ControlRun) -> Value {
        json!({"scope":"P0Control","loop":name,"logger":logger,"kernel":r.kernel,
            "realtime":perf::realtime_kernel(),"policy":r.policy.policy,"priority":r.policy.priority,
            "governor":r.config.cpu.and_then(governor),
            "cpu":r.config.cpu,"period_ns":r.config.period.as_nanos().to_string(),
            "deadline_ns":r.config.deadline.as_nanos().to_string(),
            "work_iterations":r.config.work_iterations.to_string(),"releases":r.releases,
            "completed":r.completed,"misses":r.misses,"skipped":r.skipped,"off_cpu":r.off_cpu,
            "wake":histogram(&r.wake),"response":histogram(&r.response)})
    }
    /// Whether the loops observed the load: shared start, load admission over
    /// the control window, and the durability tail reported apart.
    fn window_json(logger: &str, c: &ControlRun, s: &ControlRun, run: &StorageRun) -> Value {
        let judged = |r: &ControlRun| {
            let w = load_window(r, run);
            json!({"skew_ns":w.skew_ns.to_string(),"control_ns":w.control_ns.to_string(),
                "covered_ns":w.covered_ns.to_string(),
                "concurrent":w.judge().map_or_else(|e| json!({"rejected":format!("{e:?}")}), |()| json!(true))})
        };
        let w = load_window(c, run);
        json!({"scope":"LoadWindow","logger":logger,"setup_excluded":true,
            "lead_ns":GATE_LEAD.as_nanos().to_string(),"load_ns":w.load_ns.to_string(),
            "tail_ns":w.tail_ns.map(|n| n.to_string()),"control":judged(c),"safety":judged(s)})
    }
    fn histogram(h: &Histogram) -> Value {
        json!({"count":h.count,"max_ns":h.max_ns.to_string(),"buckets_pow2_us":h.buckets})
    }
    fn storage_json(dir: &Path, r: &StorageRun) -> Result<Value, Error> {
        let v = verify_p0(dir, r)?;
        let mut value = storage_facts(r);
        value["dir"] = json!(dir.display().to_string());
        value["verified_records"] = json!(v.records.to_string());
        value["verified_payload_bytes"] = json!(v.payload_bytes.to_string());
        Ok(value)
    }
    fn storage_facts(r: &StorageRun) -> Value {
        let w = r.workload;
        let rate = match throughput(r) {
            Ok(t) => json!({"bytes_per_second":t.bytes_per_second()}),
            Err(e) => json!({"rejected":format!("{e:?}")}),
        };
        json!({"scope":"P0Storage","tool":format!("{:?}",r.tool),
        "workload":{"record_payload_bytes":w.record_payload_bytes,"records":w.records.to_string(),
            "files":w.files,"writers":w.writers,"file_limit_bytes":w.file_limit_bytes.to_string(),
            "block_bytes":w.block_bytes,"sync_interval_ms":w.sync_interval.as_millis() as u64,
            "offered_bytes_per_second":w.offered_bytes_per_second.map(|b| b.to_string()),
            "warmup_ms":w.warmup.as_millis() as u64,"first_measured":w.first_measured().to_string(),
            "large":w.large.map(|l| json!({"bytes":l.bytes,"every":l.every.to_string(),"streams":l.streams}))},
        "payload_bytes":r.payload_bytes.to_string(),"framing_bytes":r.framing_bytes.to_string(),
        "files_created":r.files_created,
        "writes":r.writes.map(|w| json!({"calls":w.calls.to_string(),"largest":w.largest,"smallest":w.smallest})),
        "periodic_syncs":r.periodic_syncs,"accepted_ns":r.accepted_ns.to_string(),
        "durable_ns":r.durable_ns.map(|n| n.to_string()),"stopped":r.stopped.map(|s| format!("{s:?}")),
        "sync_overdue":r.sync_overdue,
        "throughput":rate})
    }
    fn line(out: &mut impl Write, value: Value) -> Result<(), Error> {
        writeln!(out, "{value}")?;
        Ok(())
    }
}
