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

fn entry() -> Entry {
    let (record_write, record_read) = rtrb::RingBuffer::new(1);
    let (byte_write, byte_read) = rtrb::RingBuffer::new(1);
    Entry {
        write: WriteState {
            records: record_write,
            bytes: byte_write,
            sequence: 0,
            normal_accepted: 0,
            raw_cursor: 0,
        },
        read: ReadState {
            records: record_read,
            bytes: byte_read,
            normal_consumed: 0,
            reclaim_cursor: 0,
        },
        consumed: AtomicUsize::new(0),
        slot_bytes: 1,
        normal_limit: 1,
        raw: None,
    }
}

fn owner(entry: Entry) -> CaptureOwner {
    CaptureOwner {
        entries: vec![entry].into_boxed_slice(),
        channels: vec![ChannelConfig::new(1, 2, 1)].into_boxed_slice(),
        closing: AtomicBool::new(false),
        session_id: 3,
        revision: 4,
        input_reserved_bytes: 0,
        raw_reserved_bytes: 0,
        control_reserved_bytes: 0,
        completion: None,
        memory: crate::config::MemoryBudget {
            total_bytes: 0,
            input_bytes: 0,
            raw_bytes: 0,
            output_bytes: 0,
            external_bytes: 0,
            control_bytes: 0,
            worker_bytes: 0,
        },
    }
}

#[test]
fn sequence_exhaustion_rejects_without_reusing_identity_or_publishing_bytes() {
    let mut entry = entry();
    entry.write.sequence = u64::MAX - 1;
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
    entry.write.normal_accepted = usize::MAX - 1;
    entry.read.normal_consumed = usize::MAX - 1;
    entry.consumed.store(usize::MAX - 1, Ordering::Release);
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
