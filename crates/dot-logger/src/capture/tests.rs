//! Boundary tests requiring otherwise unreachable counter states.

#![allow(clippy::unwrap_used)]

use super::*;

fn metadata() -> RecordMetadata {
    RecordMetadata {
        channel_id: 1,
        schema_id: 2,
        schema_version: 1,
        mono_ns: 0,
        real_ns: 0,
        source_time: None,
        level: Level::Info,
        importance: None,
    }
}

fn entry() -> (Entry, Endpoint) {
    let (record_write, record_read) = rtrb::RingBuffer::new(1);
    let (byte_write, byte_read) = rtrb::RingBuffer::new(1);
    let endpoint = Endpoint {
        write: WriteState {
            records: record_write,
            bytes: byte_write,
            sequence: 0,
            normal_accepted: 0,
            raw_cursor: 0,
            sampling: vec![0].into_boxed_slice(),
        },
        read: ReadState {
            records: record_read,
            bytes: byte_read,
            normal_consumed: 0,
            reclaim_cursor: 0,
            last_sequence: 0,
        },
    };
    let entry = Entry {
        consumed: AtomicUsize::new(0),
        slot_bytes: 1,
        normal_limit: 1,
        raw: None,
    };
    (entry, endpoint)
}

fn owner((entry, endpoint): (Entry, Endpoint)) -> CaptureOwner {
    CaptureOwner {
        endpoints: vec![endpoint].into_boxed_slice(),
        core: CaptureCore {
            entries: vec![entry].into_boxed_slice(),
            channels: vec![ChannelConfig::new(1, 2, 1)].into_boxed_slice(),
            closing: AtomicBool::new(false),
            session_id: 3,
            revision: 4,
            input_reserved_bytes: 0,
            raw_reserved_bytes: 0,
            control_reserved_bytes: 0,
            completion: None,
            telemetry: None,
            telemetry_epoch: 0,
            loss: None,
            updates: None,
            memory: crate::config::MemoryBudget {
                total_bytes: 0,
                input_bytes: 0,
                raw_bytes: 0,
                output_bytes: 0,
                external_bytes: 0,
                control_bytes: 0,
                worker_bytes: 0,
            },
        },
    }
}

#[test]
fn sequence_exhaustion_rejects_without_reusing_identity_or_publishing_bytes() {
    let mut entry = entry();
    entry.1.write.sequence = u64::MAX - 1;
    let mut owner = owner(entry);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let RecordOutcome::Accepted(receipt) = producer.try_record_small(metadata(), b"a").unwrap()
        else {
            panic!("unexpected skip")
        };
        assert_eq!(receipt.accepted_seq, u64::MAX);
        assert_eq!(
            consumer.try_read(&mut [0]).unwrap().unwrap().receipt,
            receipt
        );
        assert_eq!(
            producer.try_record_small(metadata(), b"b"),
            Err(CaptureError::SequenceExhausted)
        );
        assert!(consumer.try_read(&mut [0]).unwrap().is_none());
    });
}

#[test]
fn normal_quota_remains_correct_across_wrapping_occupancy_counters() {
    let mut entry = entry();
    entry.1.write.normal_accepted = usize::MAX - 1;
    entry.1.read.normal_consumed = usize::MAX - 1;
    entry.0.consumed.store(usize::MAX - 1, Ordering::Release);
    let mut owner = owner(entry);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        for expected in 1..=4 {
            producer.try_record_small(metadata(), b"x").unwrap();
            assert_eq!(
                producer.try_record_small(metadata(), b"y"),
                Err(CaptureError::Capacity)
            );
            assert_eq!(
                consumer
                    .try_read(&mut [0])
                    .unwrap()
                    .unwrap()
                    .receipt
                    .accepted_seq,
                expected
            );
        }
    });
}

#[test]
fn pre_submit_failures_after_closing_do_not_change_loss_watermarks() {
    let mut owner = owner(entry());
    owner.core.loss = Some(crate::loss::Bank::new(3, 1, vec![1], vec![10]).into());
    owner
        .core
        .loss_before_submit(0, Some(1), crate::loss::Cause::Encode)
        .unwrap();
    let bank = owner.core.loss.as_ref().unwrap();
    let before: Vec<_> = bank
        .sample(10, 1, 0, 0, crate::loss::Coverage::Observed)
        .counts;
    owner.core.control().close();
    for cause in [crate::loss::Cause::Encode, crate::loss::Cause::InvalidInput] {
        assert_eq!(
            owner.core.loss_before_submit(0, Some(1), cause),
            Err(CaptureError::Closing)
        );
    }
    assert_eq!(
        bank.sample(10, 2, 0, 0, crate::loss::Coverage::Observed)
            .counts,
        before
    );
}
