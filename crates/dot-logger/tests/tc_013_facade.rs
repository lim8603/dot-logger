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
    let shutdown = logger.shutdown(Duration::from_secs(10)).unwrap();
    // TASK-029 (API §20.3): one call judges the whole shutdown.
    assert!(shutdown.is_success(), "{shutdown:?}");
    let Shutdown::Closed(report) = shutdown else {
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

/// TASK-033 coverage gap: a `Channel` is bound to the logger that issued it,
/// even when another logger registered the same topic and schema.
#[test]
fn f8_channel_from_another_logger_is_rejected_and_never_stored() {
    let (temp_a, temp_b) = (Temp::new(), Temp::new());
    let (a, b) = (start(&temp_a), start(&temp_b));
    let foreign = a.channel::<Pose>("robot/pose").unwrap();
    let mut writer = b.take_writer("control").unwrap();
    assert!(matches!(
        writer.try_log(foreign, &Pose { x: 1, y: 2 }),
        Err(dot_logger::logger::LogError::InvalidInput)
    ));
    drop(writer);
    for (logger, temp) in [(a, &temp_a), (b, &temp_b)] {
        assert!(
            logger
                .shutdown(Duration::from_secs(10))
                .unwrap()
                .is_success()
        );
        assert!(stored_poses(&temp.0).is_empty());
    }
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
    let shutdown = logger.shutdown(Duration::ZERO).unwrap();
    assert!(!shutdown.is_success(), "Incomplete is never success");
    let Shutdown::Incomplete { report, pending } = shutdown else {
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
    let shutdown = logger.shutdown(Duration::from_secs(10)).unwrap();
    assert!(!shutdown.is_success(), "a Closed failure is not success");
    let Shutdown::Closed(report) = shutdown else {
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
fn f17_continuous_load_keeps_periodic_syncs_and_drops_nothing() {
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

/// TASK-023 S2b finding ⑥ (Human 2026-11-07, recommended option): small
/// records rotate after 16,384 records, far below the segment byte limit, so
/// the segment-count limit must follow that smallest segment. Before, a
/// 64 MiB continuous output allowed 10 segments and failed records although
/// most of its bytes were unused.
#[test]
fn f18_small_record_segments_still_fill_the_configured_retention() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_millis(200))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(4096, 8)),
        )
        .file_output(&temp.0, Retention::continuous(64 * MIB as u64))
        .channel::<[u8]>("tick")
        .writer("w")
        .start()
        .unwrap();
    let tick = logger.channel::<[u8]>("tick").unwrap();
    let mut writer = logger.take_writer("w").unwrap();
    for i in 0..200_000_u64 {
        while writer.try_log(tick, &i.to_le_bytes()[..]).is_err() {
            std::hint::spin_loop();
        }
    }
    drop(writer);
    let directory = logger.directories()[0].clone();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(30)).unwrap() else {
        panic!("shutdown must close");
    };
    assert_eq!(report.core.totals.failed, 0, "{report:?}");
    assert!(report.is_success(), "{report:?}");
    let (files, _) = closed_files(&directory);
    // More segments than the old byte-derived limit of 64 MiB / 8 MiB + 2.
    assert!(files.len() > 10, "{}", files.len());
}

/// TASK-023 S2c (Human 2026-11-08 「메타데이터 헬퍼」): a facade channel
/// gives the metadata the advanced raw path needs, so a zero-copy loan from
/// `Writer::producer()` lands in that channel's file like `try_log` does.
#[test]
fn f19_raw_loans_through_the_producer_view_use_the_channel_metadata() {
    use dot_logger::capture::Importance;
    use dot_logger::pool::RawRecordOutcome;
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget().writer_quota(WriterQuota::small(64, 64).with_raw(2, MIB)))
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .channel::<[u8]>("camera")
        .writer("w")
        .start()
        .unwrap();
    let camera = logger.channel::<[u8]>("camera").unwrap();
    let mut writer = logger.take_writer("w").unwrap();
    let frames: Vec<Vec<u8>> = (0..3_u8).map(|i| vec![i + 1; MIB - 7]).collect();
    for frame in &frames {
        let metadata = camera.metadata(writer.now_ns(), 1_700_000_000_000_000_000);
        assert_eq!(metadata.schema_id, <[u8] as Record>::SCHEMA.id());
        let mut producer = writer.producer();
        let mut loan = loop {
            match producer.try_acquire(frame.len(), Importance::Normal) {
                Ok(loan) => break loan,
                Err(dot_logger::capture::CaptureError::Capacity) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("{e:?}"),
            }
        };
        loan.payload_mut().copy_from_slice(frame);
        match producer.try_record_raw(metadata, loan) {
            Ok(RawRecordOutcome::Accepted(_)) => {}
            Ok(RawRecordOutcome::Skipped(_)) => panic!("skipped"),
            Err(e) => panic!("{:?}", e.reason),
        }
    }
    drop(writer);
    let directory = logger.directories()[0].clone();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown must close");
    };
    assert!(report.is_success(), "{report:?}");
    let (files, active) = closed_files(&directory);
    assert_eq!(active, 0);
    let mut stored = Vec::new();
    for file in files {
        let bytes = std::fs::read(file).unwrap();
        for message in mcap::MessageStream::new(&bytes).unwrap() {
            let record = decode_record(&message.unwrap().data, MIB).unwrap();
            assert_eq!(record.record.metadata.channel_id, 2);
            stored.push(record.payload.to_vec());
        }
    }
    assert_eq!(stored, frames);
}

/// TASK-033 S3 (UC-16): a facade ring output small enough to wrap deletes
/// its oldest closed segment files, stays within the configured bytes and
/// keeps the newest records readable. A ring needs room for the rotating
/// segment's bound plus the next segment (2 × 8 MiB here), so 17 MiB
/// retains about 1 MiB of closed files; 30,000 poses write about 4 MiB.
#[test]
fn f20_small_ring_retention_deletes_the_oldest_segment_files() {
    const RING: u64 = 17 * MIB as u64;
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(RING))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .unwrap();
    let directory = logger.directories()[0].clone();
    let sent = log_poses(&logger, 30_000);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown must close");
    };
    // A ring never rejects for space; it deletes instead.
    assert_eq!(report.core.totals.failed, 0, "{report:?}");
    assert!(report.is_success(), "{report:?}");
    let (files, _) = closed_files(&directory);
    let sequences: Vec<u64> = files
        .iter()
        .map(|f| f.file_stem().unwrap().to_str().unwrap().parse().unwrap())
        .collect();
    assert!(sequences.len() >= 2, "{sequences:?}");
    // Segment 0 and its successors were deleted; the survivors are the
    // newest, contiguous run of segments.
    assert!(sequences[0] > 0, "the oldest segment file must be deleted");
    assert!(
        sequences.windows(2).all(|w| w[1] == w[0] + 1),
        "{sequences:?}"
    );
    let total: u64 = files
        .iter()
        .map(|f| std::fs::metadata(f).unwrap().len())
        .sum();
    assert!(total <= RING, "{total} > {RING}");
    // The newest records survive in order, the oldest are gone.
    let stored = stored_poses(&directory);
    assert!(!stored.is_empty() && stored.len() < sent.len());
    assert_eq!(stored, sent[sent.len() - stored.len()..]);
}

/// TASK-033 S3 (UC-16): `Retention::reserve` is the free-space floor of the
/// real filesystem probe. A floor no filesystem can satisfy (4 EiB) is
/// rejected before anything starts and leaves nothing on disk, while a small
/// floor on the same output starts and stores records. The probe is not
/// injectable through the facade, so the runtime case (free space shrinking
/// while recording) is not tested here.
#[test]
fn f21_retention_reserve_is_enforced_against_the_free_space_probe() {
    let temp = Temp::new();
    let build = |reserve: u64| {
        Logger::builder()
            .budget(budget())
            .file_output(&temp.0, Retention::ring(256 * MIB as u64).reserve(reserve))
            .channel::<Pose>("robot/pose")
            .writer("control")
    };
    match build(1 << 62).start() {
        Err(StartError::Storage(dot_logger::storage::RetentionError::Capacity)) => {}
        Err(e) => panic!("unexpected error {e:?}"),
        Ok(_) => panic!("an unsatisfiable free-space floor must not start"),
    }
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);

    let logger = build(4096).start().unwrap();
    let directory = logger.directories()[0].clone();
    let sent = log_poses(&logger, 10);
    assert!(
        logger
            .shutdown(Duration::from_secs(10))
            .unwrap()
            .is_success()
    );
    assert_eq!(stored_poses(&directory), sent);
}

/// TASK-033 S3 (UC-11): `try_log_at` stores exactly the caller-supplied
/// session monotonic and wall-clock times (log time = wall time) instead of
/// reading a clock.
#[test]
fn f22_try_log_at_stores_the_caller_supplied_times() {
    let temp = Temp::new();
    let logger = start(&temp);
    let directory = logger.directories()[0].clone();
    let pose = logger.channel::<Pose>("robot/pose").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    let times: Vec<(u64, i64)> = (0..5_u64)
        .map(|i| {
            (
                1_000 + i * 1_000_000_007,
                1_234_567_890_123_456_789 + 3 * i as i64,
            )
        })
        .collect();
    for (i, &(mono, real)) in times.iter().enumerate() {
        let msg = Pose { x: i as i32, y: 0 };
        while let Err(e) = writer.try_log_at(pose, &msg, mono, real) {
            assert!(e.is_capacity(), "{e:?}");
            std::thread::yield_now();
        }
    }
    drop(writer);
    assert!(
        logger
            .shutdown(Duration::from_secs(10))
            .unwrap()
            .is_success()
    );
    let (files, _) = closed_files(&directory);
    let mut stored = Vec::new();
    for file in files {
        let bytes = std::fs::read(file).unwrap();
        for message in mcap::MessageStream::new(&bytes).unwrap() {
            let message = message.unwrap();
            let record = decode_record(&message.data, 64).unwrap();
            let m = record.record.metadata;
            assert_eq!(message.log_time, m.real_ns as u64);
            stored.push((m.mono_ns, m.real_ns));
        }
    }
    assert_eq!(stored, times);
}

/// TASK-033 S3 (builder validation): a missing output directory, or a path
/// that is not a directory, is `Io(NotFound)` from the up-front check (the
/// namespace creation would report `NotADirectory` for the file), and a
/// duplicate or empty writer name is `Duplicate`; in every case nothing
/// starts and nothing is created on disk.
#[test]
fn f23_missing_directory_and_bad_writer_names_are_rejected_before_start() {
    let temp = Temp::new();
    let missing = temp.0.join("missing");
    let file = temp.0.join("file");
    std::fs::write(&file, b"").unwrap();
    let builder = |dir: &Path, writers: &[&str]| {
        writers.iter().fold(
            Logger::builder()
                .budget(budget())
                .file_output(dir, Retention::ring(256 * MIB as u64))
                .channel::<Pose>("robot/pose"),
            |b, name| b.writer(name),
        )
    };
    match builder(&missing, &["control"]).start() {
        Err(StartError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        Err(e) => panic!("unexpected error {e:?}"),
        Ok(_) => panic!("a missing output directory must not start"),
    }
    assert!(
        !missing.exists(),
        "the missing directory must not be created"
    );
    match builder(&file, &["control"]).start() {
        Err(StartError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        Err(e) => panic!("unexpected error {e:?}"),
        Ok(_) => panic!("a file as output directory must not start"),
    }
    assert!(file.is_file());
    std::fs::remove_file(&file).unwrap();
    for writers in [&["a", "b", "a"][..], &[""], &["a", ""]] {
        match builder(&temp.0, writers).start() {
            Err(StartError::Duplicate) => {}
            Err(e) => panic!("{writers:?}: unexpected error {e:?}"),
            Ok(_) => panic!("{writers:?} must not start"),
        }
    }
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

/// TASK-033 S3 (UC-16) defect: a ring smaller than two segments can never
/// rotate (the room for the closing segment plus the next one never fits), so
/// almost every record failed at the file stage after a successful start.
/// Start must refuse it with the required size instead. A continuous output
/// of the same size stays valid (it stops when full by design).
#[test]
fn f24_a_ring_below_two_segments_is_rejected_before_start() {
    let temp = Temp::new();
    let start = |retention: Retention| {
        Logger::builder()
            .budget(budget())
            .file_output(&temp.0, retention)
            .channel::<Pose>("robot/pose")
            .writer("control")
            .start()
    };
    match start(Retention::ring(15 * MIB as u64)) {
        Err(StartError::Retention {
            required,
            available,
        }) => {
            assert_eq!(available, 15 * MIB as u64);
            assert_eq!(required, 16 * MIB as u64, "two 8 MiB segments");
        }
        other => panic!("a ring that cannot rotate must not start: {other:?}"),
    }
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
    let logger = start(Retention::continuous(15 * MIB as u64)).unwrap();
    assert!(
        logger
            .shutdown(Duration::from_secs(5))
            .unwrap()
            .is_success()
    );
    let logger = start(Retention::ring(16 * MIB as u64)).unwrap();
    assert!(
        logger
            .shutdown(Duration::from_secs(5))
            .unwrap()
            .is_success()
    );
}
