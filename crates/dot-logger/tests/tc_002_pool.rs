//! TC-002: raw ownership, deferred reclamation, and mixed ingress.
#![allow(clippy::unwrap_used)]

use dot_logger::capture::{
    CaptureError, CaptureOwner, ChannelConfig, Importance, Level, RecordMetadata,
};
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    WorkerThreadingConfig,
};
use dot_logger::platform::{PlatformCapabilities, Requirement};
use dot_logger::pool::{RawRecordOutcome, SlotState};

fn config() -> SessionConfig {
    SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 256 * 1024,
            raw_bytes: 256 * 1024,
            output_bytes: 256 * 1024,
            external_bytes: 0,
            control_bytes: 128 * 1024,
            worker_bytes: 128 * 1024,
        },
        producers: vec![
            ProducerQuota {
                small: PoolQuota {
                    slots: 8,
                    slot_bytes: 16,
                    important_slots: 2
                },
                raw: Some(PoolQuota {
                    slots: 4,
                    slot_bytes: 64,
                    important_slots: 1
                })
            };
            2
        ],
        importance: ImportancePolicy::PrioritizeImportant,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    }
}

fn owner_from(config: SessionConfig) -> CaptureOwner {
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap()
}
fn owner() -> CaptureOwner {
    owner_from(config())
}
fn meta(importance: Importance) -> RecordMetadata {
    RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 123,
        real_ns: 456,
        source_time: None,
        level: Level::Info,
        importance: Some(importance),
    }
}

#[test]
fn rejected_loan_is_the_same_loan_and_dropped_returns_survive_full_notification() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let loan = producer.try_acquire(64, Importance::Normal).unwrap();
        let identity = loan.identity();
        let address = loan.payload().as_ptr();
        let mut bad = meta(Importance::Normal);
        bad.schema_id = 0;
        let error = producer.try_record_raw(bad, loan).unwrap_err();
        assert_eq!(error.reason, CaptureError::SchemaMismatch);
        assert_eq!(error.loan.identity(), identity);
        assert_eq!(error.loan.payload().as_ptr(), address);
        let b = producer.try_acquire(64, Importance::Normal).unwrap();
        let c = producer.try_acquire(64, Importance::Normal).unwrap();
        let d = producer.try_acquire(64, Importance::Important).unwrap();
        drop(error.loan);
        drop(b);
        drop(c);
        drop(d);
        let snapshot = consumer.raw_snapshot().unwrap();
        assert_eq!(snapshot.return_pending, 4);
        assert!(snapshot.notification_pending); // One-slot mailbox is already full.
        assert!(matches!(
            producer.try_acquire(1, Importance::Important),
            Err(CaptureError::Capacity)
        ));
        let mut reclaimed = 0;
        for _ in 0..4 {
            reclaimed += consumer.reclaim_raw(1).reclaimed;
        }
        assert_eq!(reclaimed, 4);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
        assert_eq!(consumer.raw_snapshot().unwrap().free, 4);
        let next = producer.try_acquire(64, Importance::Normal).unwrap();
        assert_eq!(next.identity().slot_id, identity.slot_id);
        assert!(next.identity().generation > identity.generation);
    });
}

#[test]
fn accepted_payload_is_not_copied_and_cannot_be_reclaimed_while_reading() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut loan = producer.try_acquire(64, Importance::Important).unwrap();
        loan.payload_mut().fill(23);
        let identity = loan.identity();
        let address = loan.payload().as_ptr();
        let RawRecordOutcome::Accepted(receipt) = producer
            .try_record_raw(meta(Importance::Important), loan)
            .unwrap()
        else {
            panic!("unexpected skip")
        };
        assert_eq!(consumer.raw_snapshot().unwrap().queued, 1);
        let record = consumer.try_take_raw().unwrap().unwrap();
        assert_eq!(record.record().receipt, receipt);
        assert_eq!(record.identity(), identity);
        assert_eq!(record.payload().as_ptr(), address);
        assert_eq!(record.payload(), &[23; 64]);
        assert_eq!(consumer.raw_snapshot().unwrap().in_use, 1);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
        drop(record);
        assert_eq!(consumer.raw_snapshot().unwrap().return_pending, 1);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 1);
        assert_eq!(consumer.raw_snapshot().unwrap().free, 4);
    });
}

#[test]
fn normal_loans_cannot_borrow_important_bytes_and_small_raw_are_fifo() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let a = producer.try_acquire(64, Importance::Normal).unwrap();
        let b = producer.try_acquire(64, Importance::Normal).unwrap();
        let c = producer.try_acquire(64, Importance::Normal).unwrap();
        assert!(matches!(
            producer.try_acquire(1, Importance::Normal),
            Err(CaptureError::Capacity)
        ));
        let mut important = producer.try_acquire(64, Importance::Important).unwrap();
        important.payload_mut().fill(9);
        producer
            .try_record_small(meta(Importance::Normal), b"first")
            .unwrap();
        producer
            .try_record_raw(meta(Importance::Important), important)
            .unwrap();
        producer
            .try_record_small(meta(Importance::Normal), b"last")
            .unwrap();
        let mut bytes = [0; 64];
        let first = consumer.try_read(&mut bytes).unwrap().unwrap();
        assert_eq!(first.receipt.accepted_seq, 1);
        assert_eq!(&bytes[..first.payload_len], b"first");
        let raw = consumer.try_take_raw().unwrap().unwrap();
        assert_eq!(raw.record().receipt.accepted_seq, 2);
        assert_eq!(raw.payload(), &[9; 64]);
        let last = consumer.try_read(&mut bytes).unwrap().unwrap();
        assert_eq!(last.receipt.accepted_seq, 3);
        assert_eq!(&bytes[..last.payload_len], b"last");
        drop(a);
        drop(b);
        drop(c);
        drop(raw);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 4);
        assert_eq!(consumer.raw_snapshot().unwrap().count(SlotState::Free), 4);
    });
}

#[test]
fn different_producer_pool_and_reservation_mismatch_return_the_original_loan() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut a, mut a_read) = scope.next().unwrap();
        let (mut b, _) = scope.next().unwrap();
        let mut loan = a.try_acquire(3, Importance::Normal).unwrap();
        loan.payload_mut().copy_from_slice(b"abc");
        let identity = loan.identity();
        let error = b
            .try_record_raw(meta(Importance::Normal), loan)
            .unwrap_err();
        assert_eq!(error.reason, CaptureError::InvalidInput);
        assert_eq!(error.loan.identity(), identity);
        assert_eq!(error.loan.payload(), b"abc");
        let error = a
            .try_record_raw(meta(Importance::Important), error.loan)
            .unwrap_err();
        assert_eq!(error.reason, CaptureError::InvalidInput);
        assert_eq!(error.loan.identity(), identity);
        a.try_record_raw(meta(Importance::Normal), error.loan)
            .unwrap();
        assert_eq!(a_read.try_take_raw().unwrap().unwrap().payload(), b"abc");
    });
}

#[test]
fn filtered_raw_returns_same_mutable_loan_and_advances_no_sequence() {
    let plan = SessionBuilder::new(config())
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Warn;
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut loan = producer.try_acquire(3, Importance::Normal).unwrap();
        loan.payload_mut().copy_from_slice(b"abc");
        let identity = loan.identity();
        let RawRecordOutcome::Skipped(mut loan) = producer
            .try_record_raw(meta(Importance::Normal), loan)
            .unwrap()
        else {
            panic!("unexpected acceptance")
        };
        assert_eq!(loan.identity(), identity);
        assert_eq!(loan.payload(), b"abc");
        loan.payload_mut().copy_from_slice(b"xyz");
        let mut metadata = meta(Importance::Normal);
        metadata.level = Level::Warn;
        let RawRecordOutcome::Accepted(receipt) = producer.try_record_raw(metadata, loan).unwrap()
        else {
            panic!("unexpected skip")
        };
        assert_eq!(receipt.accepted_seq, 1);
        assert_eq!(consumer.try_take_raw().unwrap().unwrap().payload(), b"xyz");
    });
}

#[test]
fn ingress_saturation_preserves_loan_then_important_raw_uses_input_reservation() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        for _ in 0..6 {
            producer
                .try_record_small(meta(Importance::Normal), b"s")
                .unwrap();
        }
        let loan = producer.try_acquire(64, Importance::Normal).unwrap();
        let identity = loan.identity();
        let error = producer
            .try_record_raw(meta(Importance::Normal), loan)
            .unwrap_err();
        assert_eq!(error.reason, CaptureError::Capacity);
        assert_eq!(error.loan.identity(), identity);
        let important = producer.try_acquire(64, Importance::Important).unwrap();
        producer
            .try_record_raw(meta(Importance::Important), important)
            .unwrap();
        producer
            .try_record_small(meta(Importance::Important), b"last")
            .unwrap();
        assert_eq!(
            producer
                .try_record_raw(meta(Importance::Normal), error.loan)
                .unwrap_err()
                .reason,
            CaptureError::Capacity
        );
        for _ in 0..6 {
            consumer.try_read(&mut [0; 16]).unwrap().unwrap();
        }
        assert_eq!(
            consumer
                .try_take_raw()
                .unwrap()
                .unwrap()
                .record()
                .receipt
                .accepted_seq,
            7
        );
        assert_eq!(
            consumer
                .try_read(&mut [0; 16])
                .unwrap()
                .unwrap()
                .receipt
                .accepted_seq,
            8
        );
    });
}

#[test]
fn short_raw_copy_and_wrong_kind_leave_fifo_and_ownership_untouched() {
    use dot_logger::capture::ReadError;
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        producer
            .try_record_small(meta(Importance::Normal), b"small")
            .unwrap();
        assert!(matches!(
            consumer.try_take_raw(),
            Err(ReadError::PayloadKindMismatch)
        ));
        assert_eq!(
            consumer
                .try_read(&mut [0; 16])
                .unwrap()
                .unwrap()
                .payload_len,
            5
        );
        let mut loan = producer.try_acquire(64, Importance::Normal).unwrap();
        loan.payload_mut().fill(37);
        producer
            .try_record_raw(meta(Importance::Normal), loan)
            .unwrap();
        assert_eq!(
            consumer.try_read(&mut [0; 63]),
            Err(ReadError::BufferTooSmall(64))
        );
        assert_eq!(consumer.raw_snapshot().unwrap().queued, 1);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
        let mut bytes = [0; 64];
        assert_eq!(
            consumer.try_read(&mut bytes).unwrap().unwrap().payload_len,
            64
        );
        assert_eq!(bytes, [37; 64]);
        assert_eq!(consumer.raw_snapshot().unwrap().return_pending, 1);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 1);
    });
}

#[test]
fn queued_and_outstanding_loans_survive_close_and_scope_reentry() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let control = scope.control();
        let (mut producer, _) = scope.next().unwrap();
        let mut accepted = producer.try_acquire(64, Importance::Normal).unwrap();
        accepted.payload_mut().fill(3);
        producer
            .try_record_raw(meta(Importance::Normal), accepted)
            .unwrap();
        let pending = producer.try_acquire(64, Importance::Normal).unwrap();
        control.close();
        let error = producer
            .try_record_raw(meta(Importance::Normal), pending)
            .unwrap_err();
        assert_eq!(error.reason, CaptureError::Closing);
        drop(error.loan);
        assert!(matches!(
            producer.try_acquire(1, Importance::Important),
            Err(CaptureError::Closing)
        ));
    });
    owner.scope(|mut scope| {
        let (_, mut consumer) = scope.next().unwrap();
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 1); // Queued remains protected.
        let record = consumer.try_take_raw().unwrap().unwrap();
        assert_eq!(record.payload(), &[3; 64]);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
        drop(record);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 1);
    });
}

#[test]
fn raw_size_is_fixed_at_acquisition_and_reclaim_zero_does_no_work() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        assert!(matches!(
            producer.try_acquire(65, Importance::Normal),
            Err(CaptureError::TooLarge)
        ));
        let empty = producer.try_acquire(0, Importance::Normal).unwrap();
        assert!(empty.is_empty());
        drop(empty);
        assert_eq!(consumer.reclaim_raw(0).inspected, 0);
        assert!(consumer.raw_snapshot().unwrap().notification_pending);
        assert_eq!(consumer.reclaim_raw(usize::MAX).inspected, 4);
        assert_eq!(consumer.raw_snapshot().unwrap().free, 4);
    });
}

#[test]
fn equal_policy_and_raw_only_reservations_are_independent_of_small_storage() {
    let mut config = config();
    config.importance = ImportancePolicy::TreatEqually;
    for quota in &mut config.producers {
        quota.small.important_slots = 0;
        quota.raw.as_mut().unwrap().important_slots = 0;
    }
    let mut owner = owner_from(config);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let loans = [
            producer.try_acquire(64, Importance::Important).unwrap(),
            producer.try_acquire(64, Importance::Normal).unwrap(),
            producer.try_acquire(64, Importance::Normal).unwrap(),
            producer.try_acquire(64, Importance::Normal).unwrap(),
        ];
        assert!(matches!(
            producer.try_acquire(1, Importance::Important),
            Err(CaptureError::Capacity)
        ));
        producer
            .try_record_small(meta(Importance::Normal), b"unaffected")
            .unwrap();
        drop(loans);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 4);
    });
}

#[test]
fn raw_payload_and_slot_metadata_are_budgeted_once_before_allocation() {
    use dot_logger::capture::CaptureBuildError;
    use dot_logger::config::{ConfigError, Resource};
    let mut cfg = config();
    let owner = owner_from(cfg.clone());
    assert_eq!(owner.raw_reserved_bytes(), 2 * 4 * 64);
    let required = owner.control_reserved_bytes();
    drop(owner);
    cfg.memory.control_bytes = required - 1;
    let plan = SessionBuilder::new(cfg.clone())
        .validate(&PlatformCapabilities::default())
        .unwrap();
    assert!(
        matches!(CaptureOwner::new(&plan,42,1,&[ChannelConfig::new(7,99,1)]), Err(CaptureBuildError::Config(ConfigError::BudgetExceeded {resource:Resource::Control, required:r, ..})) if r==required)
    );
    cfg.memory.control_bytes = required;
    cfg.memory.raw_bytes = 2 * 4 * 64;
    let owner = owner_from(cfg);
    assert_eq!(owner.raw_reserved_bytes(), 2 * 4 * 64);
}

#[test]
fn optional_raw_pool_does_not_allocate_or_expose_slots() {
    let mut config = config();
    for producer in &mut config.producers {
        producer.raw = None;
    }
    let mut owner = owner_from(config);
    assert_eq!(owner.raw_reserved_bytes(), 0);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        assert!(matches!(
            producer.try_acquire(1, Importance::Normal),
            Err(CaptureError::Unsupported)
        ));
        assert!(consumer.raw_snapshot().is_none());
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
    });
}

#[test]
fn concurrent_loan_drops_coalesce_without_losing_a_return() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let loans = [
            producer.try_acquire(64, Importance::Normal).unwrap(),
            producer.try_acquire(64, Importance::Normal).unwrap(),
            producer.try_acquire(64, Importance::Normal).unwrap(),
            producer.try_acquire(64, Importance::Important).unwrap(),
        ];
        std::thread::scope(|threads| {
            for loan in loans {
                threads.spawn(move || drop(loan));
            }
        });
        assert_eq!(consumer.raw_snapshot().unwrap().return_pending, 4);
        let first = consumer.reclaim_raw(1);
        assert!(first.notification_received);
        assert_eq!(first.inspected, 1);
        let mut reclaimed = first.reclaimed;
        for _ in 0..3 {
            let r = consumer.reclaim_raw(1);
            assert!(!r.notification_received);
            reclaimed += r.reclaimed;
        }
        assert_eq!(reclaimed, 4);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
    });
}

#[test]
fn producer_consumer_and_reclaimer_reuse_raw_payloads_across_threads() {
    use std::time::{Duration, Instant};
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        std::thread::scope(|threads| {
            let count = if cfg!(miri) { 8 } else { 400 };
            let writer = threads.spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(15);
                for value in 1..=count {
                    let importance = if value % 3 == 0 {
                        Importance::Important
                    } else {
                        Importance::Normal
                    };
                    let mut loan = loop {
                        match producer.try_acquire(64, importance) {
                            Ok(loan) => break loan,
                            Err(CaptureError::Capacity) => {
                                assert!(Instant::now() < deadline);
                                std::thread::yield_now();
                            }
                            other => panic!("unexpected loan: {other:?}"),
                        }
                    };
                    loan.payload_mut().fill(value as u8);
                    producer.try_record_raw(meta(importance), loan).unwrap();
                }
            });
            let deadline = Instant::now() + Duration::from_secs(15);
            for expected in 1..=count {
                let record = loop {
                    if let Some(record) = consumer.try_take_raw().unwrap() {
                        break record;
                    }
                    consumer.reclaim_raw(1);
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                };
                assert_eq!(record.record().receipt.accepted_seq, expected);
                assert_eq!(record.payload(), &[expected as u8; 64]);
                assert_eq!(consumer.raw_snapshot().unwrap().in_use, 1);
                consumer.reclaim_raw(1); // The live view must remain immutable/owned.
                assert_eq!(record.payload(), &[expected as u8; 64]);
                drop(record);
                consumer.reclaim_raw(1);
            }
            writer.join().unwrap();
            assert_eq!(consumer.raw_snapshot().unwrap().queued, 0);
            consumer.reclaim_raw(4);
            assert_eq!(consumer.raw_snapshot().unwrap().free, 4);
        });
    });
}

#[test]
fn loan_from_another_owner_is_rejected_even_with_identical_session_metadata() {
    let mut a = owner();
    let mut b = owner();
    a.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let loan = producer.try_acquire(64, Importance::Normal).unwrap();
        let identity = loan.identity();
        b.scope(|mut other| {
            let (mut foreign, _) = other.next().unwrap();
            let error = foreign
                .try_record_raw(meta(Importance::Normal), loan)
                .unwrap_err();
            assert_eq!(error.reason, CaptureError::InvalidInput);
            assert_eq!(error.loan.identity(), identity);
            drop(error.loan);
        });
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 1);
    });
}

#[test]
fn fully_reserved_raw_pool_preserves_every_byte_for_important_loans() {
    let mut cfg = config();
    for p in &mut cfg.producers {
        p.raw.as_mut().unwrap().important_slots = 4;
    }
    let mut owner = owner_from(cfg);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        assert!(matches!(
            producer.try_acquire(1, Importance::Normal),
            Err(CaptureError::Capacity)
        ));
        let loans = [
            producer.try_acquire(64, Importance::Important).unwrap(),
            producer.try_acquire(64, Importance::Important).unwrap(),
            producer.try_acquire(64, Importance::Important).unwrap(),
            producer.try_acquire(64, Importance::Important).unwrap(),
        ];
        assert!(matches!(
            producer.try_acquire(1, Importance::Important),
            Err(CaptureError::Capacity)
        ));
        drop(loans);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 4);
    });
}

#[test]
fn immutable_original_can_be_borrowed_by_multiple_scoped_background_readers() {
    let mut owner = owner();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut loan = producer.try_acquire(64, Importance::Normal).unwrap();
        loan.payload_mut().fill(51);
        producer
            .try_record_raw(meta(Importance::Normal), loan)
            .unwrap();
        let original = consumer.try_take_raw().unwrap().unwrap();
        std::thread::scope(|threads| {
            let reference = &original;
            for _ in 0..2 {
                threads.spawn(move || assert_eq!(reference.payload(), &[51; 64]));
            }
            assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
            assert_eq!(consumer.raw_snapshot().unwrap().in_use, 1);
        });
        assert_eq!(original.payload(), &[51; 64]);
        drop(original);
        assert_eq!(consumer.reclaim_raw(4).reclaimed, 1);
    });
}
