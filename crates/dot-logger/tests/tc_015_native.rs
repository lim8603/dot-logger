//! ADR-013 / TASK-030 S2 A: native user/meta pairs, independently inspected.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::capture::{
    CaptureOwner, CaptureReceipt, CapturedRecord, ChannelConfig, Importance, SourceTime,
};
use dot_logger::routing::Router;
use dot_logger::storage::{SegmentWriter, StorageChannel, SyncSink, WriterConfig};
use std::io::Cursor;

struct Sink(Cursor<Vec<u8>>);
impl std::io::Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.0, b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::io::Seek for Sink {
    fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, p)
    }
}
impl SyncSink for Sink {
    fn sync_all(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn file(target: usize) -> Vec<u8> {
    file_channels(
        target,
        vec![StorageChannel {
            channel: ChannelConfig::new(7, 99, 1),
            topic: "robot/a".into(),
            payload_encoding: "opaque".into(),
            schema: None,
        }],
    )
}
fn file_channels(target: usize, channels: Vec<StorageChannel>) -> Vec<u8> {
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let mut w = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            WriterConfig {
                max_loss_reports: 0,
                max_payload_bytes: 256,
                write_buffer_bytes: 0,
                chunk_target_bytes: target,
                max_records: 8,
                max_chunks: 8,
                max_metadata_bytes: 1024,
            },
            channels,
            Sink(Cursor::new(Vec::new())),
        )
        .unwrap();
        for (p, seq) in [
            (0, u64::from(u32::MAX) + 1),
            (1, u64::from(u32::MAX) + 1),
            (0, u64::from(u32::MAX) + 2),
        ] {
            let mut metadata = support::meta();
            metadata.importance = Some(Importance::Important);
            metadata.source_time = Some(SourceTime {
                ticks: -7,
                clock_domain: 9,
            });
            w.append(
                CapturedRecord {
                    receipt: CaptureReceipt {
                        session_id: 42,
                        producer_id: p,
                        accepted_seq: seq,
                    },
                    metadata,
                    revision: 1,
                    payload_len: 100,
                },
                &[p as u8; 100],
            )
            .unwrap();
        }
        w.finish().unwrap().into_inner().0.into_inner()
    })
}
#[test]
fn native_writer_stores_user_payload_and_adjacent_meta() {
    let bytes = file(4096);
    let header = mcap::read::LinearReader::new(&bytes)
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    let mcap::records::Record::Header(header) = header else {
        panic!("header")
    };
    assert_eq!(header.profile, "dot-logger/standard/1");
    let messages: Vec<_> = mcap::MessageStream::new(&bytes)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(messages.len(), 6);
    for (i, pair) in messages.chunks_exact(2).enumerate() {
        assert_eq!(pair[0].channel.topic, "robot/a");
        assert!(pair[0].channel.schema.is_none());
        assert_eq!(&pair[0].data[..], &[if i == 1 { 1 } else { 0 }; 100]);
        assert_eq!(pair[1].channel.topic, "_dot_logger/meta");
        assert_eq!(
            pair[1].channel.schema.as_ref().unwrap().name,
            "dot_logger.RecordMeta"
        );
        assert_eq!(pair[0].sequence, pair[1].sequence);
    }
}
#[test]
fn a_pair_never_splits_at_the_chunk_target() {
    let bytes = file(32);
    let mut pairs = 0;
    for record in mcap::read::LinearReader::new(&bytes).unwrap() {
        if let mcap::records::Record::Chunk { header, data } = record.unwrap() {
            let mut messages = Vec::new();
            for record in mcap::read::ChunkReader::new(header, &data).unwrap() {
                if let mcap::records::Record::Message { header, .. } = record.unwrap() {
                    messages.push(header.channel_id);
                }
            }
            if !messages.is_empty() {
                assert_eq!(messages.len(), 2);
                pairs += 1;
            }
        }
    }
    assert_eq!(pairs, 3);
}

use dot_logger::storage::{NativeReader, StorageError};
fn frames(bytes: &[u8], start: usize, end: usize) -> Vec<(u8, usize, usize)> {
    let mut p = start;
    let mut records = Vec::new();
    while p < end {
        let len = u64::from_le_bytes(bytes[p + 1..p + 9].try_into().unwrap()) as usize;
        records.push((bytes[p], p, p + 9 + len));
        p += 9 + len;
    }
    records
}
fn chunks(bytes: &[u8]) -> Vec<(u8, usize, usize)> {
    // Stop before the trailing magic, whose first byte is not a record.
    frames(bytes, 8, bytes.len() - 8)
        .into_iter()
        .filter(|r| r.0 == 6)
        .collect()
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for b in bytes {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
fn fix_chunk(bytes: &mut [u8], chunk: (u8, usize, usize)) {
    let crc = crc32(&bytes[chunk.1 + 49..chunk.2]);
    bytes[chunk.1 + 33..chunk.1 + 37].copy_from_slice(&crc.to_le_bytes());
}
#[test]
fn native_reader_preserves_full_identity_and_borrows_payload() {
    let bytes = file(4096);
    let mut reader = NativeReader::new(&bytes, 100, 2).unwrap();
    assert_eq!(reader.info().session, 42);
    assert_eq!(reader.info().revision, 1);
    assert_eq!(reader.info().target, 10);
    assert_eq!(reader.info().sequence, 0);
    assert_eq!(reader.info().producers, 2);
    assert_eq!(reader.channels()[0].channel.schema_id, 99);
    let records = reader.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 3);
    for (i, r) in records.iter().enumerate() {
        assert_eq!(r.record.receipt.session_id, 42);
        assert_eq!(
            r.record.receipt.accepted_seq,
            u64::from(u32::MAX) + if i == 2 { 2 } else { 1 }
        );
        assert_eq!(r.record.receipt.producer_id, if i == 1 { 1 } else { 0 });
        assert_eq!(
            r.record.metadata.source_time,
            Some(SourceTime {
                ticks: -7,
                clock_domain: 9
            })
        );
        assert_eq!(r.record.metadata.importance, Some(Importance::Important));
        assert!(r.payload.as_ptr() >= bytes.as_ptr());
        assert!(r.payload.as_ptr_range().end <= bytes.as_ptr_range().end);
    }
    assert!(reader.next().is_none());
}
#[test]
fn reader_rejects_legacy_profiles_and_caller_limits() {
    let mut bytes = file(4096);
    assert!(NativeReader::new(&bytes, 100, 1).is_err());
    assert!(NativeReader::new(&bytes, 0, 2).is_err());
    assert!(NativeReader::new(&bytes, 100, 0).is_err());
    assert!(
        NativeReader::new(&bytes, 99, 2)
            .unwrap()
            .next()
            .unwrap()
            .is_err()
    );
    let profile = bytes
        .windows(b"dot-logger/standard/1".len())
        .position(|w| w == b"dot-logger/standard/1")
        .unwrap();
    bytes[profile] = b'x';
    assert!(matches!(
        NativeReader::new(&bytes, 100, 2),
        Err(StorageError::UnsupportedFormat)
    ));
}
#[test]
fn semantic_damage_anywhere_in_a_chunk_exposes_no_pair_from_it() {
    let original = file(4096);
    let chunk = chunks(&original)[0];
    let messages: Vec<_> = frames(&original, chunk.1 + 49, chunk.2)
        .into_iter()
        .filter(|r| r.0 == 5)
        .collect();
    // Last pair: wrong metadata channel, sequence, log/publish clocks, or payload
    // metadata channel reference. Rehash so CRC alone cannot reject the file.
    for offset in [0, 2, 6, 14, 24] {
        let mut bytes = original.clone();
        bytes[messages[5].1 + 9 + offset] ^= 1;
        fix_chunk(&mut bytes, chunk);
        let mut reader = NativeReader::new(&bytes, 100, 2).unwrap();
        assert!(reader.next().unwrap().is_err(), "offset {offset}");
        assert!(reader.next().is_none());
    }
}
#[test]
fn crc_missing_pair_and_truncated_tail_keep_only_complete_chunks() {
    let original = file(32);
    let all = chunks(&original);
    assert_eq!(all.len(), 3);
    for mode in 0..4 {
        let mut bytes = original.clone();
        let c = all[1];
        match mode {
            0 => bytes[c.1 + 33..c.1 + 37].fill(0),
            1 => bytes[c.2 - 1] ^= 1,
            2 => {
                let messages = frames(&bytes, c.1 + 49, c.2);
                bytes[messages[1].1] = 4;
                fix_chunk(&mut bytes, c);
            }
            _ => bytes.truncate(c.2 - 1),
        }
        let mut reader = NativeReader::new(&bytes, 100, 2).unwrap();
        assert_eq!(
            reader.next().unwrap().unwrap().record.receipt.producer_id,
            0
        );
        assert!(reader.next().unwrap().is_err());
        assert!(reader.next().is_none());
    }
}
#[test]
fn impossible_lengths_and_late_sequence_reversal_never_panic_or_leak_a_pair() {
    let original = file(4096);
    let chunk = chunks(&original)[0];
    let messages: Vec<_> = frames(&original, chunk.1 + 49, chunk.2)
        .into_iter()
        .filter(|r| r.0 == 5)
        .collect();
    for op in [3, 4] {
        let mut bytes = original.clone();
        let registration = frames(&bytes, chunk.1 + 49, chunk.2)
            .into_iter()
            .find(|r| r.0 == op)
            .unwrap();
        let at = registration.1 + 9 + if op == 3 { 2 } else { 4 };
        bytes[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        fix_chunk(&mut bytes, chunk);
        assert!(
            NativeReader::new(&bytes, 100, 2).is_err(),
            "forged registration string length"
        );
    }
    let mut bytes = original.clone();
    bytes[messages[5].1 + 1..messages[5].1 + 9].copy_from_slice(&u64::MAX.to_le_bytes());
    fix_chunk(&mut bytes, chunk);
    assert!(
        NativeReader::new(&bytes, 100, 2)
            .unwrap()
            .next()
            .unwrap()
            .is_err()
    );
    // Copy the first producer's complete pair over its later pair: fields and
    // CRC remain valid, but per-producer monotonicity fails before first yield.
    let mut bytes = original.clone();
    let first = original[messages[0].1..messages[1].2].to_vec();
    assert_eq!(first.len(), messages[5].2 - messages[4].1);
    bytes[messages[4].1..messages[5].2].copy_from_slice(&first);
    fix_chunk(&mut bytes, chunk);
    assert!(
        NativeReader::new(&bytes, 100, 2)
            .unwrap()
            .next()
            .unwrap()
            .is_err()
    );
}
fn replace_file_metadata(bytes: &mut Vec<u8>, entries: Vec<(String, String)>) {
    fn string(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }
    let root = frames(bytes, 8, bytes.len() - 8)
        .into_iter()
        .find(|r| r.0 == 12)
        .unwrap();
    let mut pairs = Vec::new();
    for (k, v) in entries {
        string(&mut pairs, &k);
        string(&mut pairs, &v);
    }
    let mut body = Vec::new();
    string(&mut body, "dot_logger.file");
    body.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    body.extend_from_slice(&pairs);
    bytes[root.1 + 1..root.1 + 9].copy_from_slice(&(body.len() as u64).to_le_bytes());
    bytes.splice(root.1 + 9..root.2, body);
}
fn file_metadata(bytes: &[u8]) -> Vec<(String, String)> {
    mcap::read::LinearReader::new(bytes)
        .unwrap()
        .find_map(|r| match r.unwrap() {
            mcap::records::Record::Metadata(m) if m.name == "dot_logger.file" => {
                Some(m.metadata.into_iter().collect())
            }
            _ => None,
        })
        .unwrap()
}
#[test]
fn untrusted_file_metadata_rejects_duplicate_keys_and_unallocatable_producer_counts() {
    let original = file(4096);
    let mut bytes = original.clone();
    let mut entries = file_metadata(&bytes);
    entries.push(entries[0].clone());
    replace_file_metadata(&mut bytes, entries);
    assert!(
        NativeReader::new(&bytes, 100, 2).is_err(),
        "duplicate root keys"
    );
    let mut bytes = original.clone();
    let mut entries = file_metadata(&bytes);
    entries.iter_mut().find(|(k, _)| k == "revision").unwrap().1 = "0".repeat(4096) + "1";
    replace_file_metadata(&mut bytes, entries);
    assert!(
        NativeReader::new(&bytes, 100, 2).is_err(),
        "file Metadata exceeds its fixed reservation"
    );
    let mut bytes = original;
    let mut entries = file_metadata(&bytes);
    entries
        .iter_mut()
        .find(|(k, _)| k == "producers")
        .unwrap()
        .1 = usize::MAX.to_string();
    replace_file_metadata(&mut bytes, entries);
    assert!(
        NativeReader::new(&bytes, 100, usize::MAX).is_err(),
        "overflowing producer allocation"
    );
}

#[test]
fn reserved_native_topics_and_schemas_are_rejected_before_any_sink_write() {
    use dot_logger::storage::PayloadSchema;
    let config = WriterConfig {
        max_loss_reports: 0,
        max_payload_bytes: 256,
        write_buffer_bytes: 0,
        chunk_target_bytes: 4096,
        max_records: 8,
        max_chunks: 8,
        max_metadata_bytes: 1024,
    };
    let make = |topic: &str| StorageChannel {
        channel: ChannelConfig::new(7, 99, 1),
        topic: topic.into(),
        payload_encoding: "opaque".into(),
        schema: None,
    };
    for topic in ["_dot_logger/meta", "_dot_logger/loss"] {
        assert!(matches!(
            config.reservation(&[make(topic)], 2),
            Err(StorageError::InvalidConfig)
        ));
    }
    for name in ["dot_logger.RecordMeta", "dot_logger.LossReport"] {
        let mut channel = make("robot/a");
        channel.schema = Some(PayloadSchema {
            name: Some(name.into()),
            encoding: "protobuf".into(),
            descriptor: vec![1],
        });
        assert!(matches!(
            config.reservation(&[channel], 2),
            Err(StorageError::InvalidConfig)
        ));
    }
}

#[test]
fn reader_rejects_two_file_channels_with_the_same_product_identity() {
    let mut bytes = file_channels(
        4096,
        vec![
            StorageChannel {
                channel: ChannelConfig::new(7, 99, 1),
                topic: "robot/a".into(),
                payload_encoding: "opaque".into(),
                schema: None,
            },
            StorageChannel {
                channel: ChannelConfig::new(8, 99, 1),
                topic: "robot/b".into(),
                payload_encoding: "opaque".into(),
                schema: None,
            },
        ],
    );
    let chunk = chunks(&bytes)[0];
    let channel = frames(&bytes, chunk.1 + 49, chunk.2)
        .into_iter()
        .filter(|r| r.0 == 4)
        .nth(2)
        .unwrap();
    // Meta channel is registered first. The second user channel's identity is 8.
    let p = bytes[channel.1..channel.2]
        .windows(b"product_channel".len())
        .position(|s| s == b"product_channel")
        .unwrap()
        + channel.1
        + b"product_channel".len()
        + 4;
    assert_eq!(bytes[p], b'8');
    bytes[p] = b'7';
    fix_chunk(&mut bytes, chunk);
    assert!(
        NativeReader::new(&bytes, 100, 2).is_err(),
        "duplicate product channel accepted"
    );
}

/// The sink accepts the whole user payload, then fails the first metadata write.
/// Neither the safe prefix nor Written may include that unpaired user message.
#[test]
fn metadata_io_failure_never_publishes_a_partial_pair_or_written_completion() {
    use dot_logger::completion::{Boundary, QueryOutcome};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Instant,
    };
    struct Fault {
        bytes: Rc<RefCell<Cursor<Vec<u8>>>>,
        armed: Rc<Cell<bool>>,
        failed: Rc<Cell<bool>>,
        calls: Rc<Cell<usize>>,
    }
    impl std::io::Write for Fault {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.calls.set(self.calls.get() + 1);
            if self.failed.get() {
                return Err(std::io::ErrorKind::StorageFull.into());
            }
            let n = std::io::Write::write(&mut *self.bytes.borrow_mut(), b)?;
            if self.armed.get() && b == [0xab; 8] {
                self.failed.set(true);
            }
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl std::io::Seek for Fault {
        fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
            std::io::Seek::seek(&mut *self.bytes.borrow_mut(), p)
        }
    }
    impl SyncSink for Fault {
        fn sync_all(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let plan = support::plan();
    let route = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        support::tracking(8),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        let bytes = Rc::new(RefCell::new(Cursor::new(Vec::new())));
        let armed = Rc::new(Cell::new(false));
        let failed = Rc::new(Cell::new(false));
        let calls = Rc::new(Cell::new(0));
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            WriterConfig {
                max_loss_reports: 0,
                max_payload_bytes: 16,
                write_buffer_bytes: 0,
                chunk_target_bytes: 4096,
                max_records: 8,
                max_chunks: 8,
                max_metadata_bytes: 1024,
            },
            vec![StorageChannel {
                channel: ChannelConfig::new(7, 99, 1),
                topic: "robot/a".into(),
                payload_encoding: "opaque".into(),
                schema: None,
            }],
            Fault {
                bytes: bytes.clone(),
                armed: armed.clone(),
                failed: failed.clone(),
                calls: calls.clone(),
            },
        )
        .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            support::accepted(producer.try_record_small(support::meta(), &[1; 8]).unwrap());
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let first = outputs[0].try_next(now).unwrap().unwrap();
            writer.write_delivery(&first).unwrap();
            drop(first);
            let safe = writer.safe_offset();
            let before = bytes.borrow().get_ref()[..safe as usize].to_vec();
            let receipt = support::accepted(
                producer
                    .try_record_small(support::meta(), &[0xab; 8])
                    .unwrap(),
            );
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let second = outputs[0].try_next(now).unwrap().unwrap();
            armed.set(true);
            assert!(matches!(
                writer.write_delivery(&second),
                Err(StorageError::Io(_))
            ));
            assert!(failed.get(), "whole user payload was accepted first");
            assert_ne!(
                completion
                    .poll(receipt, 10, Boundary::Written)
                    .unwrap()
                    .outcome,
                QueryOutcome::Complete
            );
            assert_eq!(writer.safe_offset(), safe);
            assert_eq!(writer.index()[0].record_count, 1);
            assert!(matches!(writer.flush_chunk(), Err(StorageError::Poisoned)));
            let writes = calls.get();
            drop(writer);
            assert_eq!(calls.get(), writes, "Drop must not retry IO");
            assert_eq!(&bytes.borrow().get_ref()[..safe as usize], before);
            let prefix = bytes.borrow();
            let records = NativeReader::new(&prefix.get_ref()[..safe as usize], 16, 2)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].payload, &[1; 8]);
        });
    });
}

#[test]
fn registrations_reject_duplicate_ids_names_and_control_schema_aliases_at_construction() {
    let base = file_channels(
        4096,
        vec![StorageChannel {
            channel: ChannelConfig::new(7, 99, 1),
            topic: "robot/a".into(),
            payload_encoding: "opaque".into(),
            schema: Some(dot_logger::storage::PayloadSchema {
                name: Some("example.Payload".into()),
                encoding: "opaque".into(),
                descriptor: vec![1, 2],
            }),
        }],
    );
    for damage in ["schema-id", "channel-id", "name", "control-alias"] {
        let mut bytes = base.clone();
        let chunk = chunks(&bytes)[0];
        let nested = frames(&bytes, chunk.1 + 49, chunk.2);
        let schema = nested.iter().find(|r| r.0 == 3).unwrap();
        let channel = nested.iter().filter(|r| r.0 == 4).nth(1).unwrap();
        match damage {
            "schema-id" | "channel-id" => {
                let record = if damage == "schema-id" {
                    schema
                } else {
                    channel
                };
                let copy = bytes[record.1..record.2].to_vec();
                let n = copy.len();
                bytes.splice(channel.1..channel.1, copy);
                for offset in [1, 25, 41] {
                    let at = chunk.1 + offset;
                    let old = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
                    bytes[at..at + 8].copy_from_slice(&(old + n as u64).to_le_bytes());
                }
                fix_chunk(&mut bytes, (6, chunk.1, chunk.2 + n));
            }
            "name" => {
                let at = bytes
                    .windows(b"example.Payload".len())
                    .position(|b| b == b"example.Payload")
                    .unwrap();
                bytes[at] = b'X';
                fix_chunk(&mut bytes, chunk);
            }
            _ => {
                let id = bytes[schema.1 + 9..schema.1 + 11].to_vec();
                bytes[channel.1 + 11..channel.1 + 13].copy_from_slice(&id);
                fix_chunk(&mut bytes, chunk);
            }
        }
        assert!(
            NativeReader::new(&bytes, 100, 2).is_err(),
            "accepted {damage}"
        );
    }
}
