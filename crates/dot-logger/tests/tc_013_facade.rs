//! TC-013 TASK-027: facade builder, owned writers, default worker and shutdown.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dot_logger::logger::{
    Budget, EncodeError, Logged, Logger, LookupError, Missing, Record, Retention, Schema, Shutdown,
    StartError, WriterQuota,
};
use dot_logger::storage::decode_record;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIB: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    x: i32,
    y: i32,
}

impl Record for Pose {
    const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-i32x2");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let bytes = out.get_mut(..8).ok_or(EncodeError)?;
        bytes[..4].copy_from_slice(&self.x.to_le_bytes());
        bytes[4..].copy_from_slice(&self.y.to_le_bytes());
        Ok(8)
    }
}

struct Other;
impl Record for Other {
    const SCHEMA: Schema = Schema::new(0x4f54_4852, 1, "opaque");
    fn encode(&self, _: &mut [u8]) -> Result<usize, EncodeError> {
        Ok(0)
    }
}

/// Owned unique directory, removed on drop even after a failed assertion.
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-tc013-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn budget() -> Budget {
    Budget::new()
        .memory_bytes(64 * MIB)
        .sync_every(Duration::from_millis(20))
        .drop_deadline(Duration::from_secs(3))
        .writer_quota(WriterQuota::small(64, 64))
}

/// Every closed segment below one output directory, in sequence order.
fn closed_files(dir: &Path) -> (Vec<PathBuf>, usize) {
    let mut closed = Vec::new();
    let mut active = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        match path.extension().and_then(|e| e.to_str()) {
            Some("mcap") => closed.push(path),
            Some("active") => active += 1,
            _ => {}
        }
    }
    closed.sort_by_key(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse::<u64>().ok())
    });
    (closed, active)
}

fn stored_poses(dir: &Path) -> Vec<Pose> {
    let (files, active) = closed_files(dir);
    assert_eq!(active, 0, "no active segment may remain after Closed");
    let mut poses = Vec::new();
    for file in files {
        let bytes = std::fs::read(file).unwrap();
        for message in mcap::MessageStream::new(&bytes).unwrap() {
            let stored = decode_record(&message.unwrap().data, 64).unwrap();
            assert_eq!(stored.record.metadata.schema_id, Pose::SCHEMA.id());
            let p = &stored.payload;
            poses.push(Pose {
                x: i32::from_le_bytes(p[..4].try_into().unwrap()),
                y: i32::from_le_bytes(p[4..8].try_into().unwrap()),
            });
        }
    }
    poses
}

#[test]
fn f1_builder_start_try_log_shutdown_syncs_every_record_without_support_code() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .unwrap();
    let pose = logger.channel::<Pose>("robot/pose").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    // A plain 'static thread: no scope, lifetime parameter or support module.
    let rt = std::thread::spawn(move || {
        let mut sent = Vec::new();
        for i in 0..200 {
            let msg = Pose { x: i, y: -i };
            loop {
                match writer.try_log(pose, &msg) {
                    Ok(Logged::Accepted(_)) => break,
                    Ok(Logged::Skipped) => panic!("default Info channel never skips"),
                    // RT admission never waits; this test retries off the RT path.
                    Err(e) if e.is_capacity() => std::thread::yield_now(),
                    Err(e) => panic!("unexpected rejection {e:?}"),
                }
            }
            sent.push(msg);
        }
        sent
    });
    let sent = rt.join().unwrap();
    let directories = logger.directories().to_vec();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close within its deadline");
    };
    assert!(report.is_success(), "{report:?}");
    assert_eq!(report.writers_outstanding, 0);
    assert!(!report.worker_failed);
    assert_eq!(report.core.required.complete, 200);
    assert_eq!(directories.len(), 1);
    assert_eq!(stored_poses(&directories[0]), sent);
}

#[test]
fn f6_missing_loss_values_are_listed_and_nothing_starts() {
    let temp = Temp::new();
    let error = Logger::builder()
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .expect_err("missing budget must be rejected");
    assert!(matches!(&error, StartError::Missing(m) if m.contains(&Missing::Budget)));

    let partial = Budget::new()
        .memory_bytes(64 * MIB)
        .writer_quota(WriterQuota::small(64, 64));
    let StartError::Missing(missing) = Logger::builder()
        .budget(partial)
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .expect_err("missing loss values must be rejected")
    else {
        panic!("expected a missing-value list");
    };
    assert!(missing.contains(&Missing::SyncEvery));
    assert!(missing.contains(&Missing::DropDeadline));
    assert!(!missing.contains(&Missing::Memory));
    // Rejection happens before any session namespace is created.
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn f6_memory_shortfall_names_the_resource_and_amount() {
    let temp = Temp::new();
    let error = Logger::builder()
        .budget(budget().memory_bytes(4096))
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .expect_err("an undersized budget must be rejected");
    let StartError::Budget {
        required,
        available,
        ..
    } = error
    else {
        panic!("expected a budget shortfall, got {error:?}");
    };
    assert!(required > available);
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn f8_lookup_errors_are_explicit() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .unwrap();
    assert!(matches!(
        logger.channel::<Pose>("robot/missing"),
        Err(LookupError::Unknown)
    ));
    assert!(matches!(
        logger.channel::<Other>("robot/pose"),
        Err(LookupError::SchemaMismatch)
    ));
    assert!(matches!(
        logger.take_writer("missing"),
        Err(LookupError::Unknown)
    ));
    let first = logger.take_writer("control").unwrap();
    assert!(matches!(
        logger.take_writer("control"),
        Err(LookupError::AlreadyTaken)
    ));
    drop(first);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("idle shutdown must close");
    };
    assert!(report.is_success(), "{report:?}");
}

fn start(temp: &Temp) -> Logger {
    Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .unwrap()
}

/// Log `count` poses on this thread, retrying capacity off the RT path.
fn log_poses(logger: &Logger, count: i32) -> Vec<Pose> {
    let pose = logger.channel::<Pose>("robot/pose").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    let mut sent = Vec::new();
    for i in 0..count {
        let msg = Pose { x: i, y: 2 * i };
        while let Err(e) = writer.try_log(pose, &msg) {
            assert!(e.is_capacity(), "{e:?}");
            std::thread::yield_now();
        }
        sent.push(msg);
    }
    sent
}

#[test]
fn f4_expired_deadline_reports_incomplete_then_closes_without_forced_release() {
    let temp = Temp::new();
    let logger = start(&temp);
    let directory = logger.directories()[0].clone();
    let sent = log_poses(&logger, 100);
    let Shutdown::Incomplete { report, pending } = logger.shutdown(Duration::ZERO).unwrap() else {
        panic!("an already expired deadline cannot close on time");
    };
    assert!(report.core.deadline_elapsed);
    assert!(!report.is_success());
    // The worker keeps draining after the deadline; nothing was force-released.
    let Shutdown::Closed(last) = pending.wait(Duration::from_secs(10)).unwrap() else {
        panic!("cleanup must finish after the deadline");
    };
    assert!(!last.is_success(), "a missed deadline is never success");
    assert!(last.core.deadline_elapsed);
    assert!(!last.worker_failed);
    // Records were still written and published; only their timing failed.
    assert_eq!(stored_poses(&directory), sent);
}

#[test]
fn f5_file_failure_is_reported_and_never_success() {
    let temp = Temp::new();
    let logger = start(&temp);
    let directory = logger.directories()[0].clone();
    let _ = log_poses(&logger, 10);
    // The namespace vanishes underneath the writer: directory sync and the
    // final publication fail even though the open file still accepts writes.
    std::fs::remove_dir_all(&directory).unwrap();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("a failed file must still settle to Closed");
    };
    assert!(report.io_failed, "{report:?}");
    assert!(!report.worker_failed);
    assert!(!report.is_success());
}

#[test]
fn f7_drop_without_shutdown_syncs_and_publishes_within_its_deadline() {
    let temp = Temp::new();
    let logger = start(&temp);
    let directory = logger.directories()[0].clone();
    let sent = log_poses(&logger, 100);
    drop(logger);
    assert_eq!(stored_poses(&directory), sent);
}

/// Poll `health()` until `ready` holds or `limit` passes; returns the last value.
fn health_until(
    logger: &Logger,
    limit: Duration,
    ready: impl Fn(&dot_logger::logger::Health) -> bool,
) -> dot_logger::logger::Health {
    let start = std::time::Instant::now();
    loop {
        let health = logger.health();
        if ready(&health) || start.elapsed() > limit {
            return health;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn f9_health_is_running_and_tracks_periodic_sync_per_output() {
    use dot_logger::logger::HealthState;
    let temp = Temp::new();
    let logger = start(&temp);
    let first = logger.health();
    assert_eq!(first.state, HealthState::Running);
    assert_eq!(first.outputs.len(), 1);
    assert_eq!(first.affinity, None, "no CPU list was requested");
    assert!(first.telemetry.is_none(), "telemetry is opt-in");
    let _ = log_poses(&logger, 20);
    let synced = health_until(&logger, Duration::from_secs(3), |h| {
        h.outputs[0].since_sync.is_some()
    });
    assert!(
        synced.outputs[0].since_sync.is_some(),
        "a 20 ms periodic barrier must succeed within 3 s: {synced:?}"
    );
    assert_eq!(synced.state, HealthState::Running);
    assert_eq!(synced.outputs[0].failed_records, 0);
    assert!(!synced.outputs[0].failed);
    // Idle past the overdue bound (4 × 20 ms + 1 s): barriers keep succeeding
    // without new records, so an idle logger is never reported as late.
    std::thread::sleep(Duration::from_millis(1500));
    let idle = logger.health();
    assert_eq!(idle.state, HealthState::Running, "{idle:?}");
    assert!(idle.outputs[0].since_sync.unwrap() < Duration::from_millis(500));
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(report.is_success(), "{report:?}");
}

#[test]
fn f9_health_reports_a_poisoned_output_as_failed() {
    use dot_logger::logger::{Failure, HealthState};
    let temp = Temp::new();
    let logger = start(&temp);
    let directory = logger.directories()[0].clone();
    let _ = log_poses(&logger, 10);
    // Periodic directory sync fails once the namespace disappears (as in F5).
    std::fs::remove_dir_all(&directory).unwrap();
    let health = health_until(&logger, Duration::from_secs(5), |h| {
        h.state != HealthState::Running
    });
    assert_eq!(
        health.state,
        HealthState::Failed(Failure::IoPoisoned),
        "{health:?}"
    );
    assert!(health.outputs[0].failed);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("a failed file must still settle to Closed");
    };
    assert!(!report.is_success());
}

#[test]
fn f9_health_reports_records_failed_when_continuous_retention_is_full() {
    use dot_logger::logger::{Degradation, HealthState};
    let temp = Temp::new();
    let big = 64 * 1024;
    let logger = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_millis(20))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(16, big)),
        )
        // Smallest valid retention for 64 KiB records; 12 MiB cannot fit.
        .file_output(&temp.0, Retention::continuous(8 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("control")
        .start()
        .unwrap();
    let blob = logger.channel::<[u8]>("blob").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    let payload = vec![7u8; big];
    for _ in 0..192 {
        while let Err(e) = writer.try_log(blob, &payload[..]) {
            assert!(e.is_capacity(), "{e:?}");
            std::thread::yield_now();
        }
    }
    let health = health_until(&logger, Duration::from_secs(5), |h| {
        h.outputs[0].failed_records > 0
    });
    assert!(health.outputs[0].failed_records > 0, "{health:?}");
    assert!(
        !health.outputs[0].failed,
        "a full retention is not an IO fault"
    );
    assert_eq!(
        health.state,
        HealthState::Degraded(Degradation::RecordsFailed)
    );
    drop(writer);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(!report.is_success(), "lost records are never success");
    // REV-051 RV-125: the report agrees with health — no IO fault.
    assert!(!report.io_failed, "{report:?}");
}

#[test]
fn f10_opt_in_telemetry_samples_capture_counts_once_per_second() {
    use dot_logger::telemetry::Observation;
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .telemetry()
        .start()
        .unwrap();
    let _ = log_poses(&logger, 30);
    let health = health_until(&logger, Duration::from_secs(4), |h| {
        matches!(
            h.telemetry,
            Some(Observation::Observed(s))
                if matches!(s.capture, Observation::Observed(c) if c.accepted >= 30)
        )
    });
    let Some(Observation::Observed(sample)) = health.telemetry else {
        panic!("no fresh sample within 4 s: {health:?}");
    };
    let Observation::Observed(counts) = sample.capture else {
        panic!("capture counts unavailable: {sample:?}");
    };
    assert_eq!(counts.accepted, 30);
    assert_eq!(counts.rejected_contract, 0);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(report.is_success(), "{report:?}");
}

#[test]
fn f11_unknown_worker_cpu_is_rejected_before_anything_starts() {
    use dot_logger::platform::Requirement;
    let temp = Temp::new();
    let error = Logger::builder()
        .budget(budget().worker_cpus(&[usize::MAX], Requirement::Optional))
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .unwrap_err();
    assert!(
        matches!(
            error,
            StartError::Config(dot_logger::config::ConfigError::InvalidCpu { .. })
        ),
        "{error:?}"
    );
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

/// Start once with too little memory and return the reported requirement.
fn reported_requirement(make: &dyn Fn(usize) -> dot_logger::logger::LoggerBuilder) -> usize {
    match make(4096).start() {
        Err(StartError::Budget { required, .. }) => required,
        other => panic!("expected a budget shortfall, got {other:?}"),
    }
}

#[test]
fn f14_reported_memory_requirement_is_exact_and_sufficient() {
    let temp = Temp::new();
    // Several shapes: writers, raw pools, outputs, channels and telemetry all
    // change the control/output partitions the facade must reserve.
    let shapes: Vec<Box<dyn Fn(usize) -> dot_logger::logger::LoggerBuilder>> = vec![
        Box::new(|memory| {
            Logger::builder()
                .budget(budget().memory_bytes(memory))
                .file_output(&temp.0, Retention::ring(256 * MIB as u64))
                .channel::<Pose>("robot/pose")
                .writer("control")
        }),
        Box::new(|memory| {
            Logger::builder()
                .budget(
                    budget()
                        .memory_bytes(memory)
                        .writer_quota(WriterQuota::small(512, 256).with_raw(4, MIB)),
                )
                .file_output(&temp.0, Retention::ring(256 * MIB as u64))
                .file_output(&temp.0, Retention::continuous(64 * MIB as u64))
                .channel::<Pose>("robot/pose")
                .channel::<[u8]>("camera")
                .writer("a")
                .writer("b")
                .writer("c")
                .telemetry()
        }),
    ];
    for make in &shapes {
        let required = reported_requirement(make.as_ref());
        // One byte short is still rejected with the same requirement.
        match make(required - 1).start() {
            Err(StartError::Budget {
                required: again, ..
            }) => assert_eq!(again, required),
            other => panic!("one byte short must fail: {other:?}"),
        }
        // Exactly the requirement starts and closes successfully.
        let logger = make(required)
            .start()
            .unwrap_or_else(|e| panic!("the reported {required} bytes must be sufficient: {e:?}"));
        let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
            panic!("shutdown did not close");
        };
        assert!(report.is_success(), "{report:?}");
    }
}

/// TASK-023 S2b findings ② and ④: an unpaced writer is held back by RT
/// `Capacity`, never by required-target drops, and the periodic sync keeps
/// happening under that continuous load instead of waiting for an empty
/// queue that never comes and then expiring (Human 2026-11-07: readiness is
/// "every record up to the cutoff routed here was written").
#[test]
fn f6_continuous_load_keeps_periodic_syncs_and_drops_nothing() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_millis(50))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(16, 64)),
        )
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("w")
        .start()
        .unwrap();
    let blob = logger.channel::<[u8]>("blob").unwrap();
    let mut writer = logger.take_writer("w").unwrap();
    let payload = [5u8; 64];
    let started = std::time::Instant::now();
    let mut freshest = None;
    // Keep the writer saturated for 1.5 s, sampling how stale the last
    // successful sync is while the load continues.
    while started.elapsed() < Duration::from_millis(1500) {
        for _ in 0..256 {
            loop {
                match writer.try_log(blob, &payload[..]) {
                    Ok(_) => break,
                    Err(e) if e.is_capacity() => std::hint::spin_loop(),
                    Err(e) => panic!("{e:?}"),
                }
            }
        }
        if started.elapsed() > Duration::from_millis(1000) {
            freshest = logger.health().outputs[0].since_sync;
        }
    }
    drop(writer);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(30)).unwrap() else {
        panic!("shutdown must close");
    };
    // A sync succeeded within the last second of continuous load.
    let since = freshest.expect("no periodic sync succeeded under load");
    assert!(since < Duration::from_millis(500), "{since:?}");
    assert_eq!(report.core.totals.dropped, 0, "{report:?}");
    assert!(!report.core.sync_failed, "{report:?}");
    assert!(report.is_success(), "{report:?}");
}
