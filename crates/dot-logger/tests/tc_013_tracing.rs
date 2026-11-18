//! TC-013 F12 (TASK-027 S3b): tracing/log connection through the facade.
#![cfg(feature = "tracing-adapter")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dot_logger::capture::Level;
use dot_logger::logger::{Budget, Logger, LookupError, Retention, Shutdown, WriterQuota};
use dot_logger::storage::NativeReader;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIB: usize = 1024 * 1024;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-tc013-tracing-{}-{nanos}-{}",
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
}

/// A tracing-only logger: the connection declares its own channel and writer.
fn start(temp: &Temp, quota: WriterQuota) -> Logger {
    Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .tracing("app/log", tracing::Level::INFO, quota)
        .start()
        .unwrap()
}

/// Stored (level, JSON text) of every record below one output directory.
fn stored(dir: &Path) -> Vec<(Level, String)> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "mcap"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let bytes = std::fs::read(file).unwrap();
        for message in NativeReader::new(&bytes, 4096, 4096).unwrap() {
            let record = message.unwrap();
            out.push((
                record.record.metadata.level,
                String::from_utf8(record.payload.to_vec()).unwrap(),
            ));
        }
    }
    out
}

fn closed(logger: Logger) {
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(report.is_success(), "{report:?}");
    assert_eq!(
        report.writers_outstanding, 0,
        "shutdown reclaims the sink's writer even while the sink is alive"
    );
}

#[test]
fn f12_tracing_and_log_events_are_recorded_with_level_filtering() {
    let temp = Temp::new();
    let logger = start(&temp, WriterQuota::small(64, 1024));
    let directory = logger.directories()[0].clone();
    let sink = logger.tracing().unwrap();
    tracing::subscriber::with_default(sink.clone(), || {
        tracing::info!(target: "robot", joint = 3, "moved");
        tracing::debug!("below the INFO minimum");
        tracing::error!(code = "E7", "fault");
    });
    log::Log::log(
        &sink,
        &log::Record::builder()
            .args(format_args!("from log"))
            .level(log::Level::Warn)
            .target("legacy")
            .build(),
    );
    log::Log::log(
        &sink,
        &log::Record::builder()
            .args(format_args!("filtered"))
            .level(log::Level::Trace)
            .target("legacy")
            .build(),
    );
    assert_eq!(logger.health().tracing_dropped, Some(0));
    closed(logger);
    // The sink outlives the logger: later events are dropped without panic.
    tracing::subscriber::with_default(sink, || tracing::warn!("after shutdown"));
    let records = stored(&directory);
    assert_eq!(records.len(), 3, "{records:?}");
    assert_eq!(records[0].0, Level::Info);
    assert!(records[0].1.contains("\"target\":\"robot\""), "{records:?}");
    assert!(records[0].1.contains("\"joint\":\"3\""));
    assert!(records[0].1.contains("moved"));
    assert_eq!(records[1].0, Level::Error);
    assert!(records[1].1.contains("\"code\":\"E7\""));
    assert_eq!(records[2].0, Level::Warn);
    assert!(records[2].1.contains("\"target\":\"legacy\""));
    assert!(records[2].1.contains("from log"));
}

#[test]
fn channel_updates_bypass_a_busy_tracing_sink_and_recover_after_formatter_panic() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::mpsc;

    struct InterruptedFormat {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    impl fmt::Display for InterruptedFormat {
        fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.entered.send(()).unwrap();
            self.release.recv().unwrap();
            panic!("injected formatter panic");
        }
    }

    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget().writer_quota(WriterQuota::small(64, 64)))
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("app/data")
        .writer("control")
        .tracing(
            "app/log",
            tracing::Level::INFO,
            WriterQuota::small(64, 1024),
        )
        .start()
        .unwrap();
    let directory = logger.directories()[0].clone();
    let data = logger.channel::<[u8]>("app/data").unwrap();
    let sink = logger.tracing().unwrap();

    let during_formatting = std::thread::scope(|scope| {
        let (entered, formatting) = mpsc::channel();
        let (release, wait_for_release) = mpsc::channel();
        let sink = &sink;
        scope.spawn(move || {
            let message = InterruptedFormat {
                entered,
                release: wait_for_release,
            };
            let result = catch_unwind(AssertUnwindSafe(|| {
                log::Log::log(
                    sink,
                    &log::Record::builder()
                        .args(format_args!("{message}"))
                        .level(log::Level::Warn)
                        .build(),
                );
            }));
            assert!(result.is_err());
        });
        formatting.recv_timeout(Duration::from_secs(5)).unwrap();

        let (send, posted) = mpsc::channel();
        let logger = &logger;
        scope.spawn(move || send.send(logger.set_channel(data, Level::Warn, 1)).ok());
        let result = posted.recv_timeout(Duration::from_secs(2));
        release.send(()).unwrap();
        result
    });
    assert_eq!(during_formatting.unwrap(), Ok(2));

    // The sink must acknowledge both updates even though its lock is poisoned.
    assert_eq!(logger.set_channel(data, Level::Error, 1), Ok(3));
    assert_eq!(logger.set_channel(data, Level::Info, 1), Ok(4));
    tracing::subscriber::with_default(sink, || tracing::warn!("after formatter panic"));
    closed(logger);
    let records = stored(&directory);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0].0, Level::Warn);
    assert!(records[0].1.contains("after formatter panic"));
}

#[test]
fn f12_oversized_and_recursive_events_are_dropped_and_counted() {
    // `tracing` already suppresses nested dispatch on one thread; a `log`
    // call made while formatting (e.g. from a library's Debug impl) reaches
    // the same sink and must be dropped instead of deadlocking on its lock.
    struct Recurse(dot_logger::logger::TracingSink);
    impl fmt::Debug for Recurse {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            log::Log::log(
                &self.0,
                &log::Record::builder()
                    .args(format_args!("nested"))
                    .level(log::Level::Warn)
                    .build(),
            );
            f.write_str("outer")
        }
    }
    let temp = Temp::new();
    let logger = start(&temp, WriterQuota::small(8, 128));
    let directory = logger.directories()[0].clone();
    let sink = logger.tracing().unwrap();
    let nested = Recurse(sink.clone());
    tracing::subscriber::with_default(sink, || {
        tracing::info!(text = "x".repeat(512).as_str(), "too large for 128 bytes");
        tracing::info!(value = ?nested, "recursive");
    });
    assert_eq!(logger.health().tracing_dropped, Some(2));
    closed(logger);
    let records = stored(&directory);
    assert_eq!(records.len(), 1, "{records:?}");
    assert!(records[0].1.contains("\"value\":\"outer\""));
}

#[test]
fn f12_tracing_is_absent_unless_declared() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget().writer_quota(WriterQuota::small(8, 64)))
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("control")
        .start()
        .unwrap();
    assert!(matches!(logger.tracing(), Err(LookupError::Unknown)));
    assert_eq!(logger.health().tracing_dropped, None);
    closed(logger);
}

/// TASK-033 S3: every `log` level is compared with the declared minimum on
/// the `tracing` scale, so `log` filtering matches `tracing` filtering.
#[test]
fn f12_log_levels_are_filtered_against_the_declared_minimum() {
    let temp = Temp::new();
    let logger = start(&temp, WriterQuota::small(8, 256));
    let sink = logger.tracing().unwrap();
    let enabled = |level| {
        log::Log::enabled(
            &sink,
            &log::Metadata::builder().level(level).target("t").build(),
        )
    };
    assert_eq!(
        [
            log::Level::Error,
            log::Level::Warn,
            log::Level::Info,
            log::Level::Debug,
            log::Level::Trace,
        ]
        .map(enabled),
        [true, true, true, false, false],
        "minimum is INFO"
    );
    closed(logger);
}
