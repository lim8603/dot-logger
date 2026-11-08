//! GAP-009 / TC-013 F5: worker panics injected through a crate-private test
//! flag. No public or feature-gated hook exists (REV-046 RV-094).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{
    Budget, Failure, HealthState, LogError, Logger, Retention, Shutdown, ShutdownError, WriterQuota,
};

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
            "dot-logger-gap009-{}-{nanos}-{}",
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

fn start(temp: &Temp) -> Logger {
    Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_millis(20))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(64, 64)),
        )
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("control")
        .start()
        .unwrap()
}

/// Retry `try_log` until the logger rejects with `Closing`, within `limit`.
fn until_closing(writer: &mut super::Writer, channel: super::Channel<[u8]>, limit: Duration) {
    let start = Instant::now();
    loop {
        match writer.try_log(channel, &[1, 2, 3][..]) {
            Err(LogError::Closing) => return,
            Ok(_) | Err(LogError::Capacity) => {}
            Err(other) => panic!("unexpected rejection {other:?}"),
        }
        assert!(start.elapsed() < limit, "admission never closed");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
// Real files, threads and the wall clock: outside Miri's isolation.
#[cfg_attr(miri, ignore)]
fn io_worker_panic_closes_admission_and_is_reported_as_failure() {
    let temp = Temp::new();
    let logger = start(&temp);
    let blob = logger.channel::<[u8]>("blob").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    for _ in 0..10 {
        while writer.try_log(blob, &[0u8; 8][..]).is_err() {
            std::thread::yield_now();
        }
    }
    logger.shared.inject.store(2, Ordering::Release);
    until_closing(&mut writer, blob, Duration::from_secs(5));
    assert!(logger.worker_failed());
    assert_eq!(
        logger.health().state,
        HealthState::Failed(Failure::WorkerFailed)
    );
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("a failed IO worker must still let shutdown close");
    };
    assert!(report.worker_failed, "{report:?}");
    assert!(report.io_failed);
    assert!(!report.is_success());
    // The writer outlived shutdown; the host reaper still owns the release.
    assert_eq!(report.writers_outstanding, 1);
    assert_eq!(writer.try_log(blob, &[0][..]), Err(LogError::Closing));
    drop(writer);
}

#[test]
// Real files, threads and the wall clock: outside Miri's isolation.
#[cfg_attr(miri, ignore)]
fn host_panic_fails_fast_and_the_reaper_still_owns_every_release() {
    let temp = Temp::new();
    let logger = start(&temp);
    let blob = logger.channel::<[u8]>("blob").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    logger.shared.inject.store(1, Ordering::Release);
    until_closing(&mut writer, blob, Duration::from_secs(5));
    assert!(logger.worker_failed());
    let begun = Instant::now();
    let result = logger.shutdown(Duration::from_secs(10));
    assert!(
        matches!(result, Err(ShutdownError::WorkerFailed)),
        "{result:?}"
    );
    assert!(
        begun.elapsed() < Duration::from_secs(1),
        "a dead host must not be awaited until the deadline"
    );
    // Dropping the writer afterwards only moves its state to the mailbox.
    drop(writer);
}

/// REV-051 RV-120: the reaper may only release a writer's mailbox after the
/// writer dropped its producer. A pushed-but-still-alive producer means the
/// RT thread is still running `Writer`'s drop glue; releasing first would make
/// that thread the second dropper and so the one that frees the ring.
#[test]
fn reaper_waits_until_the_writer_dropped_its_mailbox_producer() {
    let (mut producer, consumer) = rtrb::RingBuffer::<u8>::new(1);
    let mailboxes = [consumer];
    assert!(!super::host::writers_returned(&mailboxes));
    assert_eq!(super::host::writers_outstanding(&mailboxes), 1);
    // `Writer::drop` body has run: state pushed, fields not yet dropped.
    producer.push(7).unwrap();
    assert!(
        !super::host::writers_returned(&mailboxes),
        "state pushed but producer alive: the RT thread is still dropping fields"
    );
    assert_eq!(super::host::writers_outstanding(&mailboxes), 1);
    drop(producer);
    assert!(super::host::writers_returned(&mailboxes));
    assert_eq!(super::host::writers_outstanding(&mailboxes), 0);
}

/// REV-051 RV-121: with nothing held, an idle IO thread sleeps `idle_wait`
/// whatever `batch_wait` is; a held batch shortens the sleep to its remainder.
#[test]
fn idle_io_sleep_ignores_batch_wait_until_a_batch_is_held() {
    let idle = Duration::from_millis(2);
    let now = Instant::now();
    assert_eq!(super::host::idle_sleep(idle, Duration::ZERO, [], now), idle);
    assert_eq!(
        super::host::idle_sleep(idle, Duration::from_micros(100), [], now),
        idle
    );
    let held = now.checked_sub(Duration::from_micros(300)).unwrap();
    assert_eq!(
        super::host::idle_sleep(idle, Duration::from_millis(1), [held], now),
        Duration::from_micros(700)
    );
    assert_eq!(
        super::host::idle_sleep(idle, Duration::from_millis(10), [held], now),
        idle
    );
}

/// REV-051 RV-125: a full retention is not an IO fault, but a failing flush of
/// the written prefix is, and then the whole tail fails as IO.
#[test]
fn settle_promotes_a_failed_prefix_flush_to_io_failure() {
    use super::host::tail_reason;
    use crate::completion::FailureReason::{AdapterFailure, IoFailure};
    use crate::storage::{RetentionError, StorageError};
    assert_eq!(tail_reason(AdapterFailure, None), AdapterFailure);
    assert_eq!(
        tail_reason(AdapterFailure, Some(&RetentionError::Poisoned)),
        IoFailure
    );
    assert_eq!(
        tail_reason(
            AdapterFailure,
            Some(&RetentionError::Storage(StorageError::Poisoned))
        ),
        IoFailure
    );
    assert_eq!(tail_reason(IoFailure, None), IoFailure);
}

/// TASK-023 S2b finding ⑦: a completion report that meets Busy while another
/// thread briefly holds the slot must not give up after a fixed number of
/// yields; under CPU contention that turned written records into Unknown.
#[test]
fn busy_reports_are_retried_for_the_patience_not_a_yield_count() {
    use crate::completion::CompletionError;
    let mut calls = 0_u32;
    let result = super::host::retry_busy(
        || {
            calls += 1;
            if calls < 5000 {
                Err(CompletionError::Busy)
            } else {
                Ok(())
            }
        },
        // Generous: Miri's clock advances with interpretation time.
        Duration::from_secs(3600),
    );
    assert_eq!(result, Ok(()));
    assert_eq!(calls, 5000);
    // A slot that never frees ends as Busy once the patience is spent.
    let started = Instant::now();
    let stuck = super::host::retry_busy(|| Err(CompletionError::Busy), Duration::from_millis(20));
    assert_eq!(stuck, Err(CompletionError::Busy));
    assert!(started.elapsed() >= Duration::from_millis(20));
    // Other outcomes return at once.
    let mut once = 0;
    let other = super::host::retry_busy(
        || {
            once += 1;
            Err(CompletionError::Expired)
        },
        Duration::from_secs(5),
    );
    assert_eq!((other, once), (Err(CompletionError::Expired), 1));
}

/// TASK-023 S2d finding ⑨: the dispatcher reconciled the whole completion
/// table (window × writers slots) on every turn and every shutdown poll. With
/// 32 writers that is 262,144 slots per turn (≈3 ms), so 100k small records/s
/// fell behind, and the backlog outlived the periodic sync deadline. A turn
/// now covers one writer's window; the table is swept over `writers` turns.
#[test]
fn a_dispatcher_turn_reconciles_one_writer_window_not_the_whole_table() {
    use crate::capture::ChannelConfig;
    use crate::storage::StorageChannel;
    let budget = Budget::new()
        .memory_bytes(2048 * MIB)
        .sync_every(Duration::from_secs(1))
        .drop_deadline(Duration::from_secs(3))
        .writer_quota(WriterQuota::small(4096, 256));
    let channels = [StorageChannel {
        channel: ChannelConfig::new(1, 7, 1),
        topic: "t".into(),
        payload_encoding: "opaque".into(),
        schema: None,
    }];
    let derived = super::budget::derive(
        &budget,
        &[None; 32],
        &channels,
        &[vec![1]],
        &[Retention::ring(256 * MIB as u64)],
        super::budget::Extra::default(),
    )
    .unwrap();
    let window = derived.completion.window_records;
    assert_eq!(window, 8192);
    assert_eq!(derived.completion.reconcile_per_turn, window);
}
