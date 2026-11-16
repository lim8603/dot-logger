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
                assert_eq!(mcap::MessageStream::new(&bytes).unwrap().count(), 4);
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

#[cfg(feature = "influxdb")]
#[test]
fn routed_db_busy_publication_keeps_the_same_attempt_and_output_permit() {
    use crate::{adapters::influxdb::*, query::QueryBudget};
    use std::sync::atomic::AtomicBool;
    let (_, original, route) = setup();
    let mut config = original.config().clone();
    config.memory.external_bytes = 128 * 1024;
    let plan = SessionBuilder::new(config)
        .validate(&Default::default())
        .unwrap();
    let budget = QueryBudget::new(&plan);
    let tracking = CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: 8,
        max_queries: 1,
        page_items: 2,
        feedback_events: 2,
        updates_per_turn: 2,
        reconcile_per_turn: 2,
        targets: vec![TargetContract {
            id: 10,
            required: true,
            supported: Boundaries::DATABASE,
            completion: Boundary::DatabaseAcknowledged,
        }],
    };
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking,
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut producer, input) = scope.next().unwrap();
        let receipt = match producer
            .try_record_small(metadata(), &1i64.to_le_bytes())
            .unwrap()
        {
            crate::capture::RecordOutcome::Accepted(r) => r,
            _ => panic!("skipped"),
        };
        let mut router = Router::new(&plan, vec![input], route).unwrap();
        let low = Writer::new(
            &budget,
            Config {
                endpoint: "http://127.0.0.1:1".into(),
                organization: "test".into(),
                bucket: "test".into(),
                token: "test-only".into(),
                session_id: 42,
                measurement: "test".into(),
                device: "test".into(),
                channels: vec![Channel {
                    id: 7,
                    kind: Kind::Integer,
                }],
                collision_policy: CollisionPolicy::RejectNonIncreasing,
                max_points: 1,
                max_batch_bytes: 4096,
                max_event_bytes: 64,
                max_attempts: 1,
                request_timeout: Duration::from_secs(1),
                retry_delay: Duration::from_millis(1),
                ttl: Duration::from_secs(1),
            },
        )
        .unwrap();
        let mut writer = RoutedWriter::new(
            low,
            c,
            10,
            vec![ProjectionSchema {
                channel_id: 7,
                schema_id: 99,
                schema_version: 1,
                encoding: ProjectionEncoding::I64Le,
            }],
        )
        .unwrap();
        router.scope(|mut d, mut outputs| {
            d.turn(Instant::now(), &mut [None; 4]).unwrap();
            let attempt = writer.write_delivery(
                outputs[0].try_next(Instant::now()).unwrap().unwrap(),
                &AtomicBool::new(true),
            );
            assert_eq!(attempt.result().unwrap().attempts, 0);
            let slot = c.registry.locate(receipt).unwrap();
            let phase = slot.phase.load(Ordering::Acquire);
            slot.phase.store(UPDATING, Ordering::Release);
            assert_eq!(attempt.publish(), Err(CompletionError::Busy));
            assert_eq!(d.target_snapshot(10).unwrap().in_flight, 1);
            slot.phase.store(phase, Ordering::Release);
            attempt.publish().unwrap();
            assert_eq!(
                c.poll(receipt, 10, Boundary::DatabaseAcknowledged)
                    .unwrap()
                    .outcome,
                QueryOutcome::Dropped(crate::routing::DropReason::Cancelled)
            );
            assert_eq!(attempt.result().unwrap().attempts, 0);
            drop(attempt);
            assert_eq!(d.target_snapshot(10).unwrap().in_flight, 0);
        });
        drop(writer);
    });
    assert_eq!(budget.used_bytes(), 0);
}

fn accept(p: &mut crate::capture::Producer<'_>, byte: u8) -> CaptureReceipt {
    let crate::capture::RecordOutcome::Accepted(receipt) =
        p.try_record_small(metadata(), &[byte]).unwrap()
    else {
        panic!()
    };
    receipt
}

#[test]
fn reconcile_sweeps_the_window_only_after_an_abandoned_delivery() {
    // TASK-023 S3 finding 11: a sweep on every turn cost ~37 % of the host
    // thread although only an abandoned delivery needs it.
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let first = accept(&mut p, 1);
        reader.try_read(&mut [0; 16]).unwrap();
        c.report(first, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        // A settled slot left ACTIVE is what a sweep would terminalize.
        let slot = &c.registry.producers[0].slots[(first.accepted_seq % 2) as usize];
        slot.phase.store(ACTIVE, Ordering::Release);
        assert_eq!(c.reconcile(2), 0, "no abandonment, no sweep");
        let second = accept(&mut p, 2);
        reader.try_read(&mut [0; 16]).unwrap();
        drop(c.marker(second, 10).unwrap());
        assert_eq!(c.reconcile(2), 2);
        assert_eq!(c.totals(false).unknown, 1);
    });
}

#[test]
fn an_abandoned_slot_skipped_while_updating_is_reconciled_later() {
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let first = accept(&mut p, 1);
        reader.try_read(&mut [0; 16]).unwrap();
        drop(c.marker(first, 10).unwrap());
        let slot = &c.registry.producers[0].slots[(first.accepted_seq % 2) as usize];
        slot.phase.store(UPDATING, Ordering::Release);
        c.reconcile(2);
        assert_eq!(c.totals(false).unknown, 0);
        slot.phase.store(ACTIVE, Ordering::Release);
        c.reconcile(2);
        assert_eq!(c.totals(false).unknown, 1);
    });
}

#[test]
fn an_abandonment_after_a_finished_sweep_starts_another_sweep() {
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let first = accept(&mut p, 1);
        reader.try_read(&mut [0; 16]).unwrap();
        drop(c.marker(first, 10).unwrap());
        c.reconcile(2);
        assert_eq!(c.totals(false).unknown, 1);
        assert_eq!(c.reconcile(2), 0);
        let second = accept(&mut p, 2);
        reader.try_read(&mut [0; 16]).unwrap();
        drop(c.marker(second, 10).unwrap());
        c.reconcile(1);
        c.reconcile(1);
        assert_eq!(c.totals(false).unknown, 2);
    });
}

#[test]
fn dropping_a_reported_delivery_does_not_start_a_sweep() {
    // Every IO guard drops its marker armed after reporting; only an
    // unreported one is an abandonment that needs a sweep.
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let first = accept(&mut p, 1);
        reader.try_read(&mut [0; 16]).unwrap();
        let marker = c.marker(first, 10).unwrap();
        c.report(first, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        drop(marker);
        let slot = &c.registry.producers[0].slots[(first.accepted_seq % 2) as usize];
        slot.phase.store(ACTIVE, Ordering::Release);
        assert_eq!(c.reconcile(2), 0, "a reported delivery is no abandonment");
    });
}

#[test]
fn a_stale_sweep_cannot_erase_a_new_abandonment() {
    // REV-053 RV-131: reconciler A paused before storing its remainder while
    // B started the sweep of a newer abandonment; A then stored the old
    // remainder, the newer sweep never finished and its slot stayed ACTIVE.
    use std::sync::{Arc, Barrier};
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let first = accept(&mut p, 1); // slot 1
        reader.try_read(&mut [0; 16]).unwrap();
        drop(c.marker(first, 10).unwrap());
        let second = accept(&mut p, 2); // slot 0
        reader.try_read(&mut [0; 16]).unwrap();
        c.report(second, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        c.reconcile(1); // starts the sweep, visits slot 0 only
        let paused = Arc::new(Barrier::new(2));
        let resume = Arc::new(Barrier::new(2));
        std::thread::scope(|threads| {
            let (hold, wake) = (paused.clone(), resume.clone());
            let a = threads.spawn(move || {
                BEFORE_SWEEP_STORE.with(|hook| {
                    *hook.borrow_mut() = Some(Box::new(move || {
                        hold.wait();
                        wake.wait();
                    }));
                });
                c.reconcile(1) // terminalizes the first record in slot 1
            });
            paused.wait();
            let third = accept(&mut p, 3); // reuses slot 1
            reader.try_read(&mut [0; 16]).unwrap();
            drop(c.marker(third, 10).unwrap());
            c.reconcile(1); // B
            resume.wait();
            assert_eq!(a.join().unwrap(), 1);
        });
        for _ in 0..4 {
            c.reconcile(2);
        }
        let fourth = accept(&mut p, 4); // slot 0
        reader.try_read(&mut [0; 16]).unwrap();
        c.report(fourth, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        let fifth = p.try_record_small(metadata(), &[5]);
        assert!(
            matches!(fifth, Ok(crate::capture::RecordOutcome::Accepted(_))),
            "an abandoned slot must be reusable after reconciliation, got {fifth:?}"
        );
        assert_eq!(c.totals(false).unknown, 2);
    });
}
/// TASK-033 S3: a row under publication is reported Busy in its page and
/// summary instead of failing the page, and the scan cannot claim success.
#[test]
fn a_query_page_reports_a_row_under_publication_as_busy() {
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut p, _) = scope.next().unwrap();
        p.try_record_small(metadata(), &[1]).unwrap();
        p.try_record_small(metadata(), &[2]).unwrap();
        // Sequence 2 lives in slot 2 % window (2).
        let slot = &completion.registry.producers[0].slots[0];
        slot.phase.store(UPDATING, Ordering::Release);
        let ranges = [ProducerRange {
            producer_id: 0,
            first: 1,
            last: 2,
        }];
        let requests = [TargetRequest {
            target_id: 10,
            boundary: Boundary::Written,
        }];
        let mut query = completion
            .query(completion.scope(&ranges).unwrap(), &requests)
            .unwrap();
        let mut page = [None; 2];
        let report = query.page(&mut page).unwrap();
        slot.phase.store(ACTIVE, Ordering::Release);
        assert_eq!(report.items, 2);
        assert_eq!(
            page[1].map(|item| (item.receipt.accepted_seq, item.status.outcome)),
            Some((2, QueryOutcome::Busy))
        );
        assert_eq!((report.summary.pending, report.summary.busy), (1, 1));
        assert!(report.summary.finished);
        assert!(!report.summary.is_settled());
    });
}
