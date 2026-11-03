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
    const FILE_LIMIT: u64 = 1 << 30;
    const BLOCK: usize = 1 << 20;
    const SYNC: Duration = Duration::from_secs(1);
    const RESERVE: u64 = 64 << 30;
    const USAGE: &str = "usage: perf_baseline --plan | --calibrate | \
        --control <seconds> <control_iters> <safety_iters> | \
        --p0 <parent> <seconds> <payload> <bytes_per_second> <files> <writers> <control_iters> <safety_iters> | \
        --p0-max <parent> <total_bytes> <payload> <files> <writers>";

    pub fn execute() -> Result<(), Error> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut out = io::stdout().lock();
        match a.as_slice() {
            ["--plan"] => line(
                &mut out,
                json!({"scope":"P0Plan","profile":"local-pc-v1","period_us":10000,"deadline_us":10000,
                    "control":{"cpu":CONTROL_CPU,"work_us":500},"safety":{"cpu":SAFETY_CPU,"work_us":200},
                    "worker_cpus":WORKER_CPUS,"file_limit_bytes":FILE_LIMIT,"block_bytes":BLOCK,
                    "sync_interval_ms":1000,"final_file_and_directory_sync":true,"reserve_bytes":RESERVE,
                    "policy":"SCHED_OTHER","throughput_denominator":"final file and directory sync",
                    "ratio_requires":"P0 run of the identical workload","repetitions":REPETITIONS}),
            ),
            ["--calibrate"] => {
                let control = calibrated(CONTROL_CPU, CONTROL_WORK)?;
                let safety = calibrated(SAFETY_CPU, SAFETY_WORK)?;
                let c = |cal: Calibration| {
                    json!({"iterations":cal.iterations,
                    "median_ns":cal.median_ns,"rounds":cal.rounds})
                };
                line(
                    &mut out,
                    json!({"scope":"P0Calibration","kernel":kernel()?,"logger":"off",
                        "governor":governor(CONTROL_CPU),"releases_per_round":100,
                        "control":{"cpu":CONTROL_CPU,"work_us":500,"result":c(control)},
                        "safety":{"cpu":SAFETY_CPU,"work_us":200,"result":c(safety)}}),
                )
            }
            ["--control", seconds, control, safety] => {
                let ((c, ()), s) =
                    loops(seconds.parse()?, control.parse()?, safety.parse()?, || {
                        Ok(())
                    })?;
                line(&mut out, control_json("control", "off", &c))?;
                line(&mut out, control_json("safety", "off", &s))
            }
            [
                "--p0",
                parent,
                seconds,
                payload,
                rate,
                files,
                writers,
                control,
                safety,
            ] => {
                let seconds: u64 = seconds.parse()?;
                let payload: usize = payload.parse()?;
                let rate: u64 = rate.parse()?;
                let w = workload(
                    payload,
                    rate.saturating_mul(seconds) / payload as u64,
                    files.parse()?,
                    writers.parse()?,
                    Some(rate),
                );
                let dir = RunDir::create(Path::new(parent))?;
                let ((c, run), s) = loops(seconds, control.parse()?, safety.parse()?, || {
                    write_p0(dir.path(), w, RESERVE, &WORKER_CPUS)
                })?;
                line(&mut out, control_json("control", "p0", &c))?;
                line(&mut out, control_json("safety", "p0", &s))?;
                line(&mut out, storage_json(dir.path(), &run)?)
            }
            ["--p0-max", parent, total, payload, files, writers] => {
                let payload: usize = payload.parse()?;
                let w = workload(
                    payload,
                    total.parse::<u64>()? / payload as u64,
                    files.parse()?,
                    writers.parse()?,
                    None,
                );
                let dir = RunDir::create(Path::new(parent))?;
                let run = write_p0(dir.path(), w, RESERVE, &WORKER_CPUS)?;
                line(&mut out, storage_json(dir.path(), &run)?)
            }
            _ => Err(USAGE.into()),
        }
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
            block_bytes: BLOCK,
            sync_interval: SYNC,
            offered_bytes_per_second: rate,
        }
    }
    fn calibrated(cpu: usize, target: Duration) -> Result<Calibration, Error> {
        std::thread::spawn(move || {
            perf::pin(cpu)?;
            calibrate_periodic(target, PERIOD, 100)
        })
        .join()
        .map_err(|_| "calibration panicked")?
    }
    type Loops<T> = ((ControlRun, T), ControlRun);
    /// Control and safety loops on their CPUs for `seconds`, concurrently with
    /// `load` on the calling thread.
    fn loops<T: Send>(
        seconds: u64,
        control: u64,
        safety: u64,
        load: impl FnOnce() -> Result<T, Error>,
    ) -> Result<Loops<T>, Error> {
        let releases = seconds * 1_000_000_000 / PERIOD.as_nanos() as u64;
        let config = |cpu, work_iterations| ControlConfig {
            cpu: Some(cpu),
            period: PERIOD,
            deadline: PERIOD,
            work_iterations,
        };
        std::thread::scope(|scope| {
            let c = scope.spawn(move || run_control(config(CONTROL_CPU, control), releases));
            let s = scope.spawn(move || run_control(config(SAFETY_CPU, safety), releases));
            let loaded = load();
            let c = c.join().map_err(|_| "control panicked")??;
            let s = s.join().map_err(|_| "safety panicked")??;
            Ok(((c, loaded?), s))
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
            "governor":r.config.cpu.and_then(governor),
            "cpu":r.config.cpu,"period_ns":r.config.period.as_nanos().to_string(),
            "deadline_ns":r.config.deadline.as_nanos().to_string(),
            "work_iterations":r.config.work_iterations.to_string(),"releases":r.releases,
            "completed":r.completed,"misses":r.misses,"skipped":r.skipped,"off_cpu":r.off_cpu,
            "wake":histogram(&r.wake),"response":histogram(&r.response)})
    }
    fn histogram(h: &Histogram) -> Value {
        json!({"count":h.count,"max_ns":h.max_ns.to_string(),"buckets_pow2_us":h.buckets})
    }
    fn storage_json(dir: &Path, r: &StorageRun) -> Result<Value, Error> {
        let v = verify_p0(dir, r)?;
        let w = r.workload;
        let rate = match throughput(r) {
            Ok(t) => json!({"bytes_per_second":t.bytes_per_second()}),
            Err(e) => json!({"rejected":format!("{e:?}")}),
        };
        Ok(
            json!({"scope":"P0Storage","tool":format!("{:?}",r.tool),"dir":dir.display().to_string(),
            "workload":{"record_payload_bytes":w.record_payload_bytes,"records":w.records.to_string(),
                "files":w.files,"writers":w.writers,"file_limit_bytes":w.file_limit_bytes.to_string(),
                "block_bytes":w.block_bytes,"sync_interval_ms":w.sync_interval.as_millis() as u64,
                "offered_bytes_per_second":w.offered_bytes_per_second.map(|b| b.to_string())},
            "payload_bytes":r.payload_bytes.to_string(),"framing_bytes":r.framing_bytes.to_string(),
            "files_created":r.files_created,"write_calls":r.write_calls.to_string(),
            "largest_write":r.largest_write,"smallest_write":r.smallest_write,
            "periodic_syncs":r.periodic_syncs,"accepted_ns":r.accepted_ns.to_string(),
            "durable_ns":r.durable_ns.map(|n| n.to_string()),"stopped":r.stopped.map(|s| format!("{s:?}")),
            "throughput":rate,"verified_records":v.records.to_string(),"verified_payload_bytes":v.payload_bytes.to_string()}),
        )
    }
    fn line(out: &mut impl Write, value: Value) -> Result<(), Error> {
        writeln!(out, "{value}")?;
        Ok(())
    }
}
