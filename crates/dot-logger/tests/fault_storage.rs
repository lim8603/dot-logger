//! TC-005 TASK-009: immutable source recovery and storage failure boundaries.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::capture::{
    CaptureOwner, CaptureReceipt, CapturedRecord, ChannelConfig, Importance,
};
use dot_logger::routing::Router;
use dot_logger::storage::*;
use std::cell::RefCell;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
struct State {
    bytes: Cursor<Vec<u8>>,
    ops: Vec<&'static str>,
    fail: Option<&'static str>,
}
#[derive(Clone, Default)]
struct Buffer(Rc<RefCell<State>>);
impl Write for Buffer {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let mut s = self.0.borrow_mut();
        s.ops.push("write");
        if s.fail == Some("write") {
            return Err(io::Error::from_raw_os_error(28));
        }
        s.bytes.write(b)
    }
    fn flush(&mut self) -> io::Result<()> {
        let mut s = self.0.borrow_mut();
        s.ops.push("flush");
        if s.fail == Some("flush") {
            return Err(io::Error::other("flush"));
        }
        Ok(())
    }
}
impl Seek for Buffer {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().bytes.seek(pos)
    }
}
impl SyncSink for Buffer {
    fn sync_all(&mut self) -> io::Result<()> {
        let mut s = self.0.borrow_mut();
        s.ops.push("sync");
        if s.fail == Some("sync") {
            return Err(io::Error::other("sync"));
        }
        Ok(())
    }
}
struct Store(Buffer);
impl SegmentStore for Store {
    type Sink = Buffer;
    fn directory_sync_supported(&self) -> bool {
        self.0.0.borrow().fail != Some("unsupported")
    }
    fn available_bytes(&mut self) -> io::Result<u64> {
        Ok(u64::MAX)
    }
    fn create(&mut self, _: u64) -> io::Result<Buffer> {
        self.0.0.borrow_mut().ops.push("create");
        Ok(self.0.clone())
    }
    fn finalize(&mut self, _: u64) -> io::Result<()> {
        let mut s = self.0.0.borrow_mut();
        s.ops.push("rename");
        if s.fail == Some("rename") {
            return Err(io::Error::other("rename"));
        }
        Ok(())
    }
    fn sync_directory(&mut self) -> io::Result<()> {
        let mut s = self.0.0.borrow_mut();
        s.ops.push("directory");
        if s.fail == Some("directory") {
            return Err(io::Error::other("directory"));
        }
        Ok(())
    }
    fn remove(&mut self, _: u64) -> io::Result<()> {
        panic!("recovery must never delete")
    }
}
fn config() -> WriterConfig {
    WriterConfig {
        max_loss_reports: 0,
        max_payload_bytes: 64,
        write_buffer_bytes: 0,
        chunk_target_bytes: 4096,
        max_records: 16,
        max_chunks: 8,
        max_metadata_bytes: 128,
    }
}
fn options() -> RecoveryOptions {
    RecoveryOptions {
        source_target: None,
        source_sequence: 0,
        writer: config(),
        limits: RecoveryConfig {
            max_source_bytes: 128_000,
            max_record_bytes: 8192,
            max_chunks: 8,
        },
    }
}
fn channels() -> Vec<StorageChannel> {
    vec![StorageChannel {
        channel: ChannelConfig::new(7, 99, 1),
        topic: "raw".into(),
        payload_encoding: "opaque".into(),
        schema: None,
    }]
}
fn record(seq: u64) -> CapturedRecord {
    let mut metadata = support::meta();
    metadata.mono_ns = seq;
    metadata.real_ns = 100 + seq as i64;
    metadata.importance = Some(Importance::Important);
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: seq,
        },
        revision: 1,
        metadata,
        payload_len: 3,
    }
}
fn with_budget(f: impl FnOnce(WriterBudget)) {
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|mut scope| {
        let (_, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let router = Router::new(&plan, vec![a, b], support::routing()).unwrap();
        f(router.writer_budget(10).unwrap());
    });
}
fn fixture(budget: WriterBudget, finished: bool) -> Vec<u8> {
    let sink = Buffer::default();
    let mut writer = SegmentWriter::new(budget, config(), channels(), sink.clone()).unwrap();
    for n in 1..=6 {
        writer.append(record(n), &[1, 2, n as u8]).unwrap();
        if n % 2 == 0 {
            writer.flush_chunk().unwrap();
        }
    }
    if finished {
        drop(writer.finish().unwrap());
    } else {
        drop(writer);
    }
    sink.0.borrow().bytes.get_ref().clone()
}
fn records(bytes: &[u8]) -> Vec<(u8, usize, usize)> {
    let mut p = 8;
    let mut out = Vec::new();
    while p + 9 <= bytes.len() {
        let n = u64::from_le_bytes(bytes[p + 1..p + 9].try_into().unwrap()) as usize;
        let end = p + 9 + n;
        if end > bytes.len() {
            break;
        }
        out.push((bytes[p], p, end));
        p = end;
    }
    out
}
fn chunks(bytes: &[u8]) -> Vec<(usize, usize)> {
    records(bytes)
        .into_iter()
        .filter(|r| r.0 == 6)
        .map(|r| (r.1, r.2))
        .collect()
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
fn fix_crc(bytes: &mut [u8], chunk: (usize, usize)) {
    let start = chunk.0 + 9;
    let crc = crc32(&bytes[start + 40..chunk.1]);
    bytes[start + 24..start + 28].copy_from_slice(&crc.to_le_bytes());
}
fn receipts(bytes: &[u8]) -> Vec<u64> {
    assert!(mcap::Summary::read(bytes).unwrap().is_some());
    NativeReader::new(bytes, 64, 4096)
        .unwrap()
        .map(|m| m.unwrap().record.receipt.accepted_seq)
        .collect()
}
#[test]
fn truncated_chunk_recovers_only_the_complete_prefix_and_preserves_source() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let ch = chunks(&bytes);
        let bytes = bytes[..ch[1].0 + 25].to_vec();
        let original = bytes.clone();
        let mut source = Cursor::new(bytes);
        let output = Buffer::default();
        let recovered = recover_segment(
            &mut source,
            budget,
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(source.into_inner(), original);
        assert_eq!(recovered.report.records, 2);
        assert_eq!(recovered.report.chunks, 1);
        assert_eq!(recovered.report.verified_end, ch[0].1 as u64);
        assert_eq!(
            recovered.report.excluded,
            (ch[0].1 as u64, original.len() as u64)
        );
        assert_eq!(recovered.report.stop, RecoveryStop::Truncated);
        assert_eq!(receipts(output.0.borrow().bytes.get_ref()), [1, 2]);
        assert_eq!(output.0.borrow().ops.last(), Some(&"sync"));
        assert!(!recovered.segment.index().is_empty());
    });
}
#[test]
fn crc_damage_stops_before_the_bad_chunk_without_resynchronizing_later_data() {
    with_budget(|budget| {
        let mut bytes = fixture(budget.clone(), true);
        let ch = chunks(&bytes);
        bytes[ch[1].0 + 9 + 40 + 25] ^= 1;
        let output = Buffer::default();
        let r = recover_segment(
            &mut Cursor::new(bytes),
            budget,
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.report.stop, RecoveryStop::Checksum);
        assert_eq!(r.report.records, 2);
        assert_eq!(receipts(output.0.borrow().bytes.get_ref()), [1, 2]);
    });
}
#[test]
fn unknown_channel_with_valid_crc_rejects_the_entire_chunk() {
    with_budget(|budget| {
        let mut bytes = fixture(budget.clone(), false);
        let ch = chunks(&bytes);
        let msg = ch[1].0 + 9 + 40;
        assert_eq!(bytes[msg], 5);
        bytes[msg + 9..msg + 11].copy_from_slice(&999u16.to_le_bytes());
        fix_crc(&mut bytes, ch[1]);
        let output = Buffer::default();
        let r = recover_segment(
            &mut Cursor::new(bytes),
            budget,
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.report.stop, RecoveryStop::InvalidRecord);
        assert_eq!(r.report.records, 2);
        assert_eq!(receipts(output.0.borrow().bytes.get_ref()), [1, 2]);
    });
}
#[test]
fn clean_and_unfinished_sources_rebuild_indexes_and_preserve_full_width_receipts() {
    for finished in [false, true] {
        with_budget(|budget| {
            let bytes = fixture(budget.clone(), finished);
            let output = Buffer::default();
            let r = recover_segment(
                &mut Cursor::new(bytes),
                budget,
                channels(),
                options(),
                output.clone(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(r.report.records, 6);
            assert_eq!(
                r.report.stop,
                if finished {
                    RecoveryStop::DataEnd
                } else {
                    RecoveryStop::EndOfInput
                }
            );
            assert_eq!(
                receipts(output.0.borrow().bytes.get_ref()),
                [1, 2, 3, 4, 5, 6]
            );
        });
    }
}
#[test]
fn source_record_and_chunk_limits_bound_work_without_partial_chunk_admission() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let mut opt = options();
        opt.limits.max_source_bytes = 1;
        let output = Buffer::default();
        assert!(
            recover_segment(
                &mut Cursor::new(bytes.clone()),
                budget.clone(),
                channels(),
                opt,
                output.clone(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
        assert!(output.0.borrow().ops.is_empty());
        opt = options();
        opt.limits.max_chunks = 1;
        let output = Buffer::default();
        let r = recover_segment(
            &mut Cursor::new(bytes.clone()),
            budget.clone(),
            channels(),
            opt,
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.report.records, 2);
        assert_eq!(r.report.stop, RecoveryStop::Limit);
        drop(r); // Release the finished output index before the next recovery.
        opt = options();
        opt.limits.max_record_bytes = 64;
        let output = Buffer::default();
        let r = recover_segment(
            &mut Cursor::new(bytes),
            budget.clone(),
            channels(),
            opt,
            output,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.report.records, 0);
        assert_eq!(r.report.stop, RecoveryStop::Limit);
    });
}
#[test]
fn recovery_publication_orders_sync_rename_directory_and_keeps_os_failure_causes() {
    for fail in [
        None,
        Some("write"),
        Some("flush"),
        Some("sync"),
        Some("rename"),
        Some("directory"),
        Some("unsupported"),
    ] {
        with_budget(|budget| {
            let bytes = fixture(budget.clone(), false);
            let output = Buffer::default();
            output.0.borrow_mut().fail = fail;
            let mut store = Store(output.clone());
            let result = recover_to_store(
                &mut Cursor::new(bytes),
                budget,
                channels(),
                options(),
                &mut store,
                99,
                &AtomicBool::new(false),
            );
            let state = output.0.borrow();
            match fail {
                None => {
                    assert_eq!(result.unwrap().records, 6);
                    let tail = &state.ops[state.ops.len() - 3..];
                    assert_eq!(tail, ["sync", "rename", "directory"]);
                }
                Some("write") => {
                    let err = result.err().unwrap();
                    assert!(
                        matches!(err.error,Some(StorageError::Io(ref e)) if e.raw_os_error()==Some(28))
                    );
                    assert!(!state.ops.contains(&"rename"));
                }
                Some("unsupported") => {
                    assert!(result.is_err());
                    assert!(state.ops.is_empty());
                }
                Some("directory") => {
                    assert!(result.is_err());
                    assert!(state.ops.contains(&"rename"));
                }
                Some("rename") => {
                    assert!(result.is_err());
                    assert!(state.ops.contains(&"sync"));
                    assert!(!state.ops.contains(&"directory"));
                }
                _ => {
                    assert!(result.is_err());
                    assert!(!state.ops.contains(&"rename"));
                }
            }
        });
    }
}
#[test]
fn cancellation_and_nonempty_destination_do_not_publish_or_modify_existing_data() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let output = Buffer::default();
        let cancel = AtomicBool::new(true);
        let r = recover_segment(
            &mut Cursor::new(bytes.clone()),
            budget.clone(),
            channels(),
            options(),
            output.clone(),
            &cancel,
        )
        .err()
        .unwrap();
        assert_eq!(r.report.stop, RecoveryStop::Cancelled);
        assert!(output.0.borrow().ops.is_empty());
        cancel.store(false, Ordering::Relaxed);
        output.0.borrow_mut().bytes = Cursor::new(vec![9]);
        assert!(
            recover_segment(
                &mut Cursor::new(bytes),
                budget,
                channels(),
                options(),
                output.clone(),
                &cancel
            )
            .is_err()
        );
        assert_eq!(output.0.borrow().bytes.get_ref(), &[9]);
    });
}
#[test]
fn malformed_magic_and_extreme_lengths_do_not_panic_or_allocate_from_file_lengths() {
    with_budget(|budget| {
        for bytes in [
            vec![0; 8],
            b"\x89MCAP0\r\n\x01\xff\xff\xff\xff\xff\xff\xff\xff".to_vec(),
        ] {
            assert!(
                recover_segment(
                    &mut Cursor::new(bytes),
                    budget.clone(),
                    channels(),
                    options(),
                    Buffer::default(),
                    &AtomicBool::new(false)
                )
                .is_err()
            );
        }
    });
}
#[cfg(all(unix, not(miri)))]
#[test]
fn real_readonly_file_recovery_keeps_original_and_refuses_existing_output() {
    with_budget(|budget| {
        let root = std::env::temp_dir().join(format!(
            "dot-recovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let bytes = fixture(budget.clone(), false);
        let src = root.join("original.active");
        std::fs::write(&src, &bytes).unwrap();
        let mut store = FileStore::new(&root, &budget, || Ok(u64::MAX)).unwrap();
        let r = recover_to_store(
            &mut std::fs::File::open(&src).unwrap(),
            budget.clone(),
            channels(),
            options(),
            &mut store,
            1,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.records, 6);
        assert_eq!(std::fs::read(&src).unwrap(), bytes);
        let dest = store.directory().join("1.mcap");
        let recovered = std::fs::read(&dest).unwrap();
        assert_eq!(receipts(&recovered), [1, 2, 3, 4, 5, 6]);
        // Existing active paths are rejected without truncation.
        std::fs::write(store.directory().join("1.active"), [9]).unwrap();
        assert!(
            recover_to_store(
                &mut std::fs::File::open(&src).unwrap(),
                budget,
                channels(),
                options(),
                &mut store,
                1,
                &AtomicBool::new(false)
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&dest).unwrap(), recovered);
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[test]
fn corruption_in_the_last_message_discards_earlier_messages_in_the_same_chunk() {
    with_budget(|budget| {
        let mut bytes = fixture(budget.clone(), false);
        let ch = chunks(&bytes);
        let first = ch[1].0 + 9 + 40;
        let len = u64::from_le_bytes(bytes[first + 1..first + 9].try_into().unwrap()) as usize;
        let second = first + 9 + len;
        assert_eq!(bytes[second], 5);
        bytes[second + 9..second + 11].copy_from_slice(&999u16.to_le_bytes());
        fix_crc(&mut bytes, ch[1]);
        let output = Buffer::default();
        let r = recover_segment(
            &mut Cursor::new(bytes),
            budget,
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.report.stop, RecoveryStop::InvalidRecord);
        assert_eq!(receipts(output.0.borrow().bytes.get_ref()), [1, 2]);
    });
}
#[test]
fn missing_crc_compression_and_nested_lengths_are_never_silently_trusted() {
    for failure in 0..4 {
        with_budget(|budget| {
            let mut bytes = fixture(budget.clone(), false);
            let ch = chunks(&bytes);
            let base = ch[1].0 + 9;
            match failure {
                0 => bytes[base + 24..base + 28].fill(0),
                1 => bytes[base + 28..base + 32].copy_from_slice(&1u32.to_le_bytes()),
                2 => {
                    bytes[base + 40 + 1..base + 40 + 9].copy_from_slice(&u64::MAX.to_le_bytes());
                    fix_crc(&mut bytes, ch[1]);
                }
                _ => {
                    bytes[base..base + 8].copy_from_slice(&0u64.to_le_bytes());
                }
            }
            let output = Buffer::default();
            let r = recover_segment(
                &mut Cursor::new(bytes),
                budget,
                channels(),
                options(),
                output.clone(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(r.report.records, 2);
            assert_eq!(
                r.report.stop,
                match failure {
                    0 => RecoveryStop::Checksum,
                    1 => RecoveryStop::UnsupportedCompression,
                    _ => RecoveryStop::InvalidRecord,
                }
            );
            assert_eq!(receipts(output.0.borrow().bytes.get_ref()), [1, 2]);
        });
    }
}
#[test]
fn unknown_schema_reference_is_rejected_even_when_chunk_crc_is_correct() {
    with_budget(|budget| {
        let mut bytes = fixture(budget.clone(), false);
        let ch = chunks(&bytes);
        let mut p = ch[0].0 + 9 + 40;
        while bytes[p] != 4 {
            let len = u64::from_le_bytes(bytes[p + 1..p + 9].try_into().unwrap()) as usize;
            p += 9 + len;
        }
        bytes[p + 9 + 2..p + 9 + 4].copy_from_slice(&999u16.to_le_bytes());
        fix_crc(&mut bytes, ch[0]);
        let output = Buffer::default();
        let r = recover_segment(
            &mut Cursor::new(bytes),
            budget,
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.report.stop, RecoveryStop::InvalidRecord);
        assert_eq!(r.report.records, 0);
        assert_eq!(r.report.verified_end, 0);
        assert!(receipts(output.0.borrow().bytes.get_ref()).is_empty());
    });
}
#[test]
fn full_width_receipts_and_optional_user_schema_survive_recovery() {
    with_budget(|budget| {
        let mut registrations = channels();
        registrations[0].schema = Some(PayloadSchema {
            encoding: "opaque".into(),
            descriptor: vec![0, 128, 255],
            name: Some("test.Opaque".into()),
        });
        let sink = Buffer::default();
        let mut writer = SegmentWriter::new(
            budget.clone(),
            config(),
            registrations.clone(),
            sink.clone(),
        )
        .unwrap();
        for seq in u64::MAX - 2..=u64::MAX {
            let mut r = record(1);
            r.receipt.accepted_seq = seq;
            r.metadata.mono_ns = seq;
            writer.append(r, &[1, 128, 255]).unwrap();
        }
        writer.flush_chunk().unwrap();
        drop(writer);
        let bytes = sink.0.borrow().bytes.get_ref().clone();
        let output = Buffer::default();
        let r = recover_segment(
            &mut Cursor::new(bytes),
            budget,
            registrations,
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(r.report.records, 3);
        assert_eq!(
            receipts(output.0.borrow().bytes.get_ref()),
            [u64::MAX - 2, u64::MAX - 1, u64::MAX]
        );
        assert_eq!(
            mcap::Summary::read(output.0.borrow().bytes.get_ref())
                .unwrap()
                .unwrap()
                .schemas
                .len(),
            2
        );
    });
}
struct BrokenReader(Cursor<Vec<u8>>);
impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from_raw_os_error(5))
    }
}
impl Seek for BrokenReader {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.0.seek(p)
    }
}
#[test]
fn source_io_failure_preserves_os_error_and_writes_nothing() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let output = Buffer::default();
        let r = recover_segment(
            &mut BrokenReader(Cursor::new(bytes)),
            budget.clone(),
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert!(matches!(r.error,Some(StorageError::Io(ref e)) if e.raw_os_error()==Some(5)));
        assert!(output.0.borrow().ops.is_empty());
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn bounded_work_reservation_rejects_before_output_and_returns_all_leases() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let output = Buffer::default();
        let mut opt = options();
        opt.limits.max_record_bytes = 1024 * 1024;
        let r = recover_segment(
            &mut Cursor::new(bytes),
            budget.clone(),
            channels(),
            opt,
            output.clone(),
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert!(matches!(r.error, Some(StorageError::Budget { .. })));
        assert!(output.0.borrow().ops.is_empty());
        assert_eq!(budget.used_bytes(), 0);
    });
}

struct CancelOnSync<'a>(Buffer, &'a AtomicBool);
impl Write for CancelOnSync<'_> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.write(b)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Seek for CancelOnSync<'_> {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.0.seek(p)
    }
}
impl SyncSink for CancelOnSync<'_> {
    fn sync_all(&mut self) -> io::Result<()> {
        self.0.sync_all()?;
        self.1.store(true, Ordering::Release);
        Ok(())
    }
}
#[test]
fn cancellation_arriving_during_final_sync_does_not_return_a_successful_recovery() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let cancel = AtomicBool::new(false);
        let result = recover_segment(
            &mut Cursor::new(bytes),
            budget,
            channels(),
            options(),
            CancelOnSync(Buffer::default(), &cancel),
            &cancel,
        );
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().report.stop, RecoveryStop::Cancelled);
    });
}

struct FailAtReader {
    inner: Cursor<Vec<u8>>,
    at: u64,
}
impl Read for FailAtReader {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let left = self.at.saturating_sub(self.inner.position());
        if left == 0 {
            return Err(io::Error::from_raw_os_error(5));
        }
        let n = b.len().min(left as usize);
        self.inner.read(&mut b[..n])
    }
}
impl Seek for FailAtReader {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.inner.seek(p)
    }
}
#[test]
fn input_failure_after_a_verified_chunk_reports_progress_without_sync_or_publication() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let ch = chunks(&bytes);
        let output = Buffer::default();
        let mut source = FailAtReader {
            inner: Cursor::new(bytes.clone()),
            at: ch[1].0 as u64 + 5,
        };
        let error = recover_segment(
            &mut source,
            budget.clone(),
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert_eq!(error.report.records, 2);
        assert_eq!(error.report.verified_end, ch[0].1 as u64);
        assert!(matches!(error.error,Some(StorageError::Io(ref e)) if e.raw_os_error()==Some(5)));
        assert!(!output.0.borrow().ops.contains(&"sync"));
        assert_eq!(source.inner.into_inner(), bytes);
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn disk_full_on_one_output_preserves_the_other_output_and_further_capture_admission() {
    use dot_logger::completion::{Boundary, FailureReason, QueryOutcome, Update};
    let plan = support::plan();
    let routing = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        support::tracking(8),
    )
    .unwrap();
    owner.scope(|mut scope|{
        let completion=scope.completion().unwrap();
        let (mut producer,a)=scope.next().unwrap();let (_,b)=scope.next().unwrap();
        let mut router=Router::new(&plan,vec![a,b],routing).unwrap();
        let failed=Buffer::default();let healthy=Buffer::default();
        let mut a=SegmentWriter::new(router.writer_budget(10).unwrap(),config(),channels(),failed.clone()).unwrap();
        let mut b=SegmentWriter::new(router.writer_budget(20).unwrap(),config(),channels(),healthy.clone()).unwrap();
        failed.0.borrow_mut().fail=Some("write");
        router.scope(|mut dispatch,mut outputs|{
            let now=std::time::Instant::now();let receipt=support::accepted(producer.try_record_small(support::meta(),&[1,2,3]).unwrap());
            dispatch.turn(now,&mut[None;8]).unwrap();dispatch.turn(now,&mut[None;8]).unwrap();
            let delivery_a=outputs[0].try_next(now).unwrap().unwrap();let delivery_b=outputs[1].try_next(now).unwrap().unwrap();
            assert!(matches!(a.write_delivery(&delivery_a),Err(StorageError::Io(ref e)) if e.raw_os_error()==Some(28)));
            delivery_a.report(Update::Failed(FailureReason::IoFailure)).unwrap();
            b.write_delivery(&delivery_b).unwrap();
            assert_eq!(completion.poll(receipt,10,Boundary::Written).unwrap().outcome,QueryOutcome::Failed(FailureReason::IoFailure));
            assert_eq!(completion.poll(receipt,20,Boundary::Written).unwrap().outcome,QueryOutcome::Complete);
            support::accepted(producer.try_record_small(support::meta(),&[4]).unwrap());
            assert!(matches!(a.append(record(3),&[1,2,3]),Err(StorageError::Poisoned)));
        });
    });
}

#[test]
fn exhausted_output_capacity_finishes_only_whole_source_chunks() {
    with_budget(|budget| {
        let bytes = fixture(budget.clone(), false);
        let ch = chunks(&bytes);
        for (max_records, max_chunks, chunk_bytes, admitted) in [
            (1, 8, 4096, 0),
            (3, 8, 4096, 2),
            (16, 1, 4096, 2),
            (16, 3, 1, 2),
            (16, 6, 1, 6),
        ] {
            let output = Buffer::default();
            let mut opt = options();
            opt.writer.max_records = max_records;
            opt.writer.max_chunks = max_chunks;
            opt.writer.chunk_target_bytes = chunk_bytes;
            let recovered = recover_segment(
                &mut Cursor::new(bytes.clone()),
                budget.clone(),
                channels(),
                opt,
                output.clone(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(recovered.report.records, admitted);
            assert_eq!(
                recovered.report.stop,
                if admitted == 6 {
                    RecoveryStop::EndOfInput
                } else {
                    RecoveryStop::Limit
                }
            );
            if admitted > 0 {
                assert_eq!(recovered.report.verified_end, ch[admitted / 2 - 1].1 as u64);
            }
            let state = output.0.borrow();
            assert!(state.ops.contains(&"sync"));
            assert_eq!(
                receipts(state.bytes.get_ref()),
                (1..=admitted as u64).collect::<Vec<_>>()
            );
            drop(state);
            drop(recovered);
            assert_eq!(budget.used_bytes(), 0);
        }
    });
}

#[test]
fn damaged_root_metadata_returns_a_finished_invalid_record_prefix() {
    with_budget(|budget| {
        let mut bytes = fixture(budget.clone(), false);
        let root = records(&bytes).into_iter().find(|r| r.0 == 12).unwrap();
        bytes[root.1 + 13] ^= 1;
        let output = Buffer::default();
        let recovered = recover_segment(
            &mut Cursor::new(bytes),
            budget,
            channels(),
            options(),
            output.clone(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(recovered.report.stop, RecoveryStop::InvalidRecord);
        assert_eq!(recovered.report.records, 0);
        assert!(output.0.borrow().ops.contains(&"sync"));
        assert!(
            mcap::Summary::read(output.0.borrow().bytes.get_ref())
                .unwrap()
                .is_some()
        );
    });
}
// T5 (EV-068): the vendored MCAP writer hashes an uncompressed chunk once and
// reuses that CRC for the data section; both must stay exact, since recovery
// requires the chunk CRC and checks a nonzero data section CRC.
#[test]
fn uncompressed_chunks_keep_exact_chunk_and_data_section_crcs() {
    with_budget(|budget| {
        let bytes = fixture(budget, true);
        let ch = chunks(&bytes);
        assert_eq!(ch.len(), 3);
        for (start, end) in ch {
            let header = start + 9;
            let stored = u32::from_le_bytes(bytes[header + 24..header + 28].try_into().unwrap());
            assert_ne!(stored, 0);
            assert_eq!(stored, crc32(&bytes[header + 40..end]));
        }
        let data_end = records(&bytes).into_iter().find(|r| r.0 == 0x0f).unwrap();
        let stored = u32::from_le_bytes(bytes[data_end.1 + 9..data_end.1 + 13].try_into().unwrap());
        assert_ne!(stored, 0);
        assert_eq!(stored, crc32(&bytes[..data_end.1]));
    });
}
