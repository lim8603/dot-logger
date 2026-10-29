//! Paused-publication tests for states that public operations do not expose.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::{
    capture::{CaptureOwner, ChannelConfig, Level, RecordMetadata},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    routing::{Route, Router, Schedule, TargetConfig},
};
use std::time::{Duration, Instant};
fn setup() -> (CaptureOwner, crate::config::ValidatedConfig, RoutingConfig) {
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 128 * 1024,
            raw_bytes: 0,
            output_bytes: 256 * 1024,
            external_bytes: 0,
            control_bytes: 256 * 1024,
            worker_bytes: 256 * 1024,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 16,
                important_slots: 0,
            },
            raw: None,
        }],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())
    .unwrap();
    let routing = RoutingConfig {
        targets: vec![TargetConfig {
            id: 10,
            required: true,
            queue_slots: 4,
            pending_slots: 1,
            retained_bytes: 128,
            max_attempts: 3,
            max_age: Duration::from_secs(1),
        }],
        routes: vec![Route {
            channel_id: 7,
            targets: vec![10],
        }],
        shared_records: 4,
        small_copy_bytes: 64,
        schedule: Schedule {
            record_quantum: 1,
            byte_quantum: 64,
            records_per_turn: 1,
            bytes_per_turn: 64,
            retries_per_target: 1,
            reclaim_slots_per_producer: 1,
        },
    };
    let config = CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: 2,
        max_queries: 1,
        page_items: 2,
        feedback_events: 1,
        updates_per_turn: 1,
        reconcile_per_turn: 1,
        targets: vec![TargetContract {
            id: 10,
            required: true,
            supported: Boundaries::FILE,
            completion: Boundary::Written,
        }],
    };
    let owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        config,
    )
    .unwrap();
    (owner, plan, routing)
}
fn metadata() -> RecordMetadata {
    RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 1,
        real_ns: 2,
        source_time: None,
        level: Level::Info,
        importance: None,
    }
}

// In-memory storage exercises the public writer paths under deterministic
// metadata-publication contention, including under Miri without filesystem IO.
struct MemorySink(std::io::Cursor<Vec<u8>>);
impl std::io::Write for MemorySink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.0, bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::io::Seek for MemorySink {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, pos)
    }
}
impl crate::storage::SyncSink for MemorySink {
    fn sync_all(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn storage_config() -> crate::storage::WriterConfig {
    crate::storage::WriterConfig {
        max_payload_bytes: 16,
        write_buffer_bytes: 256,
        chunk_target_bytes: 1024,
        max_records: 2,
        max_chunks: 2,
        max_metadata_bytes: 64,
    }
}
fn storage_channels() -> Vec<crate::storage::StorageChannel> {
    vec![crate::storage::StorageChannel {
        channel: ChannelConfig::new(7, 99, 1),
        topic: "test".into(),
        payload_encoding: "bytes".into(),
        schema: None,
    }]
}

#[test]
fn storage_written_busy_retries_reports_without_appending_again() {
    use crate::storage::{SegmentWriter, StorageError};
    for batch in [false, true] {
        let (mut owner, plan, routing) = setup();
        owner.scope(|mut scope| {
            let completion = scope.completion().unwrap();
            let (mut producer, reader) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![reader], routing).unwrap();
            let mut writer = SegmentWriter::new(
                router.writer_budget(10).unwrap(),
                storage_config(),
                storage_channels(),
                MemorySink(std::io::Cursor::new(Vec::new())),
            )
            .unwrap();
            router.scope(|mut dispatcher, mut outputs| {
                let now = Instant::now();
                producer.try_record_small(metadata(), &[1]).unwrap();
                dispatcher.turn(now, &mut [None; 4]).unwrap();
                let first = outputs[0].try_next(now).unwrap().unwrap();
                producer.try_record_small(metadata(), &[2]).unwrap();
                dispatcher.turn(now, &mut [None; 4]).unwrap();
                let second = outputs[0].try_next(now).unwrap().unwrap();
                let slot = completion.registry.locate(second.record().receipt).unwrap();
                let phase = slot.phase.swap(UPDATING, Ordering::AcqRel);
                if batch {
                    let error = writer.write_batch(&[&first, &second]).unwrap_err();
                    assert!(matches!(error.error, StorageError::Busy));
                    assert_eq!((error.appended, error.written, error.reported), (2, 2, 1));
                } else {
                    writer.write_delivery(&first).unwrap();
                    assert!(matches!(
                        writer.write_delivery(&second),
                        Err(StorageError::Busy)
                    ));
                }
                let safe = writer.safe_offset();
                slot.phase.store(phase, Ordering::Release);
                second.report(Update::Confirmed(Boundary::Written)).unwrap();
                assert_eq!(writer.safe_offset(), safe);
                assert_eq!(
                    completion
                        .poll(second.record().receipt, 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
                drop((first, second));
                let bytes = writer.finish().unwrap().into_inner().0.into_inner();
                assert_eq!(mcap::MessageStream::new(&bytes).unwrap().count(), 2);
            });
        });
    }
}

#[test]
fn storage_sync_busy_returns_work_for_active_and_finished_files() {
    use crate::{
        control::{ControlConfig, ControlOwner},
        storage::{SegmentWriter, StorageError},
    };
    for finished in [false, true] {
        let (mut owner, plan, routing) = setup();
        owner.scope(|mut scope| {
            let completion = scope.completion().unwrap();
            let now = Instant::now();
            let control = ControlOwner::new(
                &plan,
                scope.control(),
                ControlConfig {
                    sync_slots: 1,
                    work_per_poll: 8,
                },
                now,
            )
            .unwrap();
            let (mut producer, reader) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![reader], routing).unwrap();
            let budget = router.writer_budget(10).unwrap();
            let mut writer = SegmentWriter::new(
                budget.clone(),
                storage_config(),
                storage_channels(),
                MemorySink(std::io::Cursor::new(Vec::new())),
            )
            .unwrap();
            router.scope(|mut dispatcher, mut outputs| {
                producer.try_record_small(metadata(), &[1]).unwrap();
                dispatcher.turn(now, &mut [None; 4]).unwrap();
                let delivery = outputs[0].try_next(now).unwrap().unwrap();
                let receipt = delivery.record().receipt;
                writer.write_delivery(&delivery).unwrap();
                delivery
                    .report(Update::Confirmed(Boundary::Synced))
                    .unwrap();
                drop(delivery);
                drop(writer.finish().unwrap());
                let mut next = SegmentWriter::new(
                    budget,
                    storage_config(),
                    storage_channels(),
                    MemorySink(std::io::Cursor::new(Vec::new())),
                )
                .unwrap();
                let mut ranges = [ProducerRange::default(); 1];
                let frozen = completion.capture_scope(&mut ranges).unwrap();
                let token = control
                    .request_sync(frozen, &[10], now, now + Duration::from_secs(1))
                    .unwrap();
                let work = control
                    .next_sync(
                        token,
                        10,
                        now,
                        &mut dispatcher,
                        &mut [ProducerRange::default(); 1],
                    )
                    .unwrap()
                    .unwrap();
                let slot = completion.registry.locate(receipt).unwrap();
                let phase = slot.phase.swap(UPDATING, Ordering::AcqRel);
                if finished {
                    let mut next = next.finish().unwrap();
                    let error = next.sync_work(work).unwrap_err();
                    assert!(matches!(error.error, StorageError::Busy));
                    let work = error.work.unwrap();
                    slot.phase.store(phase, Ordering::Release);
                    next.sync_work(work).unwrap();
                } else {
                    let error = next.sync_work(work).unwrap_err();
                    assert!(matches!(error.error, StorageError::Busy));
                    let work = error.work.unwrap();
                    slot.phase.store(phase, Ordering::Release);
                    next.sync_work(work).unwrap();
                }
                control.poll(now).unwrap();
                assert_eq!(
                    completion
                        .poll(receipt, 10, Boundary::Synced)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
            });
        });
    }
}
#[test]
fn busy_report_publication_preserves_feedback_and_backpressures_without_losing_an_event() {
    let (mut owner, plan, routing) = setup();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut p, c) = scope.next().unwrap();
        p.try_record_small(metadata(), &[1]).unwrap();
        p.try_record_small(metadata(), &[2]).unwrap();
        let slot = &completion.registry.producers[0].slots[1];
        slot.phase.store(UPDATING, Ordering::Release);
        let mut router = Router::new(&plan, vec![c], routing).unwrap();
        router.scope(|mut d, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 4];
            let report = d.turn(now, &mut events).unwrap();
            assert_eq!(report.records, 1);
            assert_eq!(d.drain_state().feedback, 1);
            assert_eq!(d.turn(now, &mut events).unwrap().records, 0);
            let first = outputs[0].try_next(now).unwrap().unwrap();
            assert_eq!(
                first.report(Update::Confirmed(Boundary::Written)),
                Err(CompletionError::Busy)
            );
            slot.phase.store(ACTIVE, Ordering::Release);
            assert_eq!(d.turn(now, &mut events).unwrap().records, 1);
            first.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(first);
            let second = outputs[0].try_next(now).unwrap().unwrap();
            second.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(second);
            assert_eq!(completion.totals(false).complete, 2);
            assert_eq!(d.drain_state().feedback, 0);
        });
    });
}
#[test]
fn a_stale_drop_marker_cannot_overwrite_the_new_generations_unknown_outcome() {
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let crate::capture::RecordOutcome::Accepted(first) =
            p.try_record_small(metadata(), &[1]).unwrap()
        else {
            panic!()
        };
        reader.try_read(&mut [0; 16]).unwrap();
        let old = c.marker(first, 10).unwrap();
        c.report(first, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        let crate::capture::RecordOutcome::Accepted(second) =
            p.try_record_small(metadata(), &[2]).unwrap()
        else {
            panic!()
        };
        reader.try_read(&mut [0; 16]).unwrap();
        c.report(second, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        let crate::capture::RecordOutcome::Accepted(third) =
            p.try_record_small(metadata(), &[3]).unwrap()
        else {
            panic!()
        };
        let new = c.marker(third, 10).unwrap();
        drop(new);
        drop(old);
        assert_eq!(
            c.poll(third, 10, Boundary::Written).unwrap().outcome,
            QueryOutcome::Unknown
        );
        c.reconcile(2);
        assert_eq!(c.totals(false).unknown, 1);
    });
}

#[test]
fn delayed_enqueue_feedback_after_writer_failure_does_not_stall_dispatch() {
    let (mut owner, plan, routing) = setup();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut producer, consumer) = scope.next().unwrap();
        producer.try_record_small(metadata(), &[1]).unwrap();
        producer.try_record_small(metadata(), &[2]).unwrap();
        let slot = &completion.registry.producers[0].slots[1];
        // Hold only metadata publication; the output FIFO remains readable.
        slot.phase.store(UPDATING, Ordering::Release);
        let mut router = Router::new(&plan, vec![consumer], routing).unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 4];
            assert_eq!(dispatch.turn(now, &mut events).unwrap().records, 1);
            assert_eq!(dispatch.drain_state().feedback, 1);
            let first = outputs[0].try_next(now).unwrap().unwrap();
            let receipt = first.record().receipt;
            slot.phase.store(ACTIVE, Ordering::Release);
            first
                .report(Update::Failed(FailureReason::IoFailure))
                .unwrap();
            drop(first);
            let report = dispatch.turn(now, &mut events).unwrap();
            assert_eq!(report.fault, None);
            assert_eq!(report.records, 1);
            assert_eq!(dispatch.drain_state().feedback, 0);
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Written)
                    .unwrap()
                    .outcome,
                QueryOutcome::Failed(FailureReason::IoFailure)
            );
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Enqueued)
                    .unwrap()
                    .outcome,
                QueryOutcome::Complete
            );
            let second = outputs[0].try_next(now).unwrap().unwrap();
            second.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(second);
            let totals = completion.totals(false);
            assert_eq!((totals.complete, totals.failed, totals.pending), (1, 1, 0));
        });
    });
}
