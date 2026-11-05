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
