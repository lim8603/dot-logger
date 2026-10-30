//! TASK-010: file snapshots, time/filter ordering, bounded pages and payload reads.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{capture::*, config::*, platform::*, query::*, routing::Router, storage::*};
use std::io::{self, Cursor as IoCursor, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
#[derive(Clone, Default)]
struct File(Arc<Mutex<IoCursor<Vec<u8>>>>, u64);
impl Write for File {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let mut bytes = self.0.lock().unwrap();
        bytes.seek(SeekFrom::Start(self.1))?;
        let n = bytes.write(b)?;
        self.1 += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Read for File {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let mut bytes = self.0.lock().unwrap();
        bytes.seek(SeekFrom::Start(self.1))?;
        let n = bytes.read(b)?;
        self.1 += n as u64;
        Ok(n)
    }
}
impl Seek for File {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        let mut bytes = self.0.lock().unwrap();
        bytes.seek(SeekFrom::Start(self.1))?;
        self.1 = bytes.seek(p)?;
        Ok(self.1)
    }
}
impl SyncSink for File {
    fn sync_all(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn cfg() -> QueryConfig {
    QueryConfig {
        max_segments: 4,
        max_records: 32,
        max_record_bytes: 16384,
        max_source_bytes: 1_000_000,
        max_payload_bytes: 8192,
        max_metadata_bytes: 1024,
        producers: 2,
        page_records: 2,
        page_bytes: 1024,
        inline_bytes: 16,
        blob_read_bytes: 32,
        read_bytes: 1_000_000,
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
fn query() -> Query {
    Query {
        basis: TimeBasis::Mono,
        from: 0,
        to: u64::MAX as i128 + 1,
        channel: None,
        minimum_level: Level::Trace,
    }
}
fn budget() -> QueryBudget {
    let mut c = support::plan().config().clone();
    c.memory.external_bytes = 1024 * 1024;
    c.memory.total_bytes += 1024 * 1024;
    QueryBudget::new(
        &SessionBuilder::new(c)
            .validate(&PlatformCapabilities::default())
            .unwrap(),
    )
}
fn record(seq: u64, mono: u64, real: i64) -> CapturedRecord {
    let mut m = support::meta();
    m.mono_ns = mono;
    m.real_ns = real;
    m.importance = Some(Importance::Normal);
    m.source_time = Some(SourceTime {
        ticks: real,
        clock_domain: 9,
    });
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: seq,
        },
        revision: 1,
        metadata: m,
        payload_len: 3,
    }
}
fn with_writer(f: impl FnOnce(SegmentWriter<File>, File)) {
    let p = support::plan();
    let mut o = CaptureOwner::new(&p, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    o.scope(|s| {
        let r = Router::new(&p, s.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let file = File::default();
        let w = SegmentWriter::new(
            r.writer_budget(10).unwrap(),
            WriterConfig {
                max_payload_bytes: 8192,
                write_buffer_bytes: 0,
                chunk_target_bytes: 16384,
                max_records: 32,
                max_chunks: 16,
                max_metadata_bytes: 1024,
            },
            channels(),
            file.clone(),
        )
        .unwrap();
        f(w, file);
    });
}
fn snapshot(file: File, safe: u64, c: QueryConfig) -> Snapshot<File> {
    Snapshot::new(
        &budget(),
        42,
        1,
        channels(),
        vec![QuerySource::active(file, 1, safe)],
        c,
        0,
        100,
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn active_snapshot_hides_unflushed_and_later_complete_chunks() {
    with_writer(|mut w, f| {
        w.append(record(1, 10, 100), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let safe = w.safe_offset();
        w.append(record(2, 20, 200), &[4, 5, 6]).unwrap();
        let mut s = snapshot(f, safe, cfg());
        w.flush_chunk().unwrap();
        let c = s.start(query(), 1, &AtomicBool::new(false)).unwrap();
        let p = s.page(&c, 1, &AtomicBool::new(false)).unwrap();
        assert_eq!(p.records.len(), 1);
        assert_eq!(p.records[0].record.receipt.accepted_seq, 1);
        assert_eq!(p.inline(&p.records[0]), Some([1, 2, 3].as_slice()));
        assert!(p.next.is_none());
    });
}
#[test]
fn pages_sort_out_of_order_timestamps_and_freeze_filters() {
    with_writer(|mut w, f| {
        for (i, t) in [30, 10, 20, 10, 40].into_iter().enumerate() {
            w.append(record(i as u64 + 1, t, 100 - t as i64), &[1, 2, 3])
                .unwrap();
        }
        w.flush_chunk().unwrap();
        let mut s = snapshot(f, w.safe_offset(), cfg());
        let cancel = AtomicBool::new(false);
        let mut c = s.start(query(), 1, &cancel).unwrap();
        let mut got = vec![];
        loop {
            let p = s.page(&c, 1, &cancel).unwrap();
            assert!(p.records.len() <= 2);
            got.extend(p.records.iter().map(|x| x.record.receipt.accepted_seq));
            if let Some(next) = p.next {
                c = next
            } else {
                break;
            }
        }
        assert_eq!(got, [2, 4, 3, 1, 5]);
        let mut q = query();
        q.basis = TimeBasis::Real;
        q.from = 70;
        q.to = 90;
        let c = s.start(q, 1, &cancel).unwrap();
        let p = s.page(&c, 1, &cancel).unwrap();
        assert_eq!(
            p.records
                .iter()
                .map(|x| x.record.receipt.accepted_seq)
                .collect::<Vec<_>>(),
            [1, 3]
        );
        let mut severity = query();
        severity.minimum_level = Level::Error;
        let filtered = s.start(severity, 1, &cancel).unwrap();
        assert!(s.page(&filtered, 1, &cancel).unwrap().records.is_empty());
        let mut q = query();
        q.channel = Some(8);
        let c = s.start(q, 1, &cancel).unwrap();
        assert!(s.page(&c, 1, &cancel).unwrap().records.is_empty());
    });
}
#[test]
fn interleaved_queries_and_reused_cursors_keep_their_original_order() {
    with_writer(|mut w, f| {
        for (i, t) in [30, 10, 20, 10, 40].into_iter().enumerate() {
            let mut r = record(i as u64 + 1, t, 100 - t as i64);
            r.metadata.level = if i == 2 { Level::Error } else { Level::Info };
            w.append(r, &[i as u8; 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let mut s = snapshot(f, w.safe_offset(), cfg());
        let cancel = AtomicBool::new(false);
        let original = s.start(query(), 1, &cancel).unwrap();
        let next = s.page(&original, 1, &cancel).unwrap().next.unwrap();
        let cases = [
            (
                Query {
                    basis: TimeBasis::Real,
                    to: i64::MAX as i128 + 1,
                    ..query()
                },
                vec![5, 1],
            ),
            (
                Query {
                    basis: TimeBasis::Source(9),
                    to: i64::MAX as i128 + 1,
                    ..query()
                },
                vec![5, 1],
            ),
            (
                Query {
                    from: 20,
                    ..query()
                },
                vec![3, 1],
            ),
            (Query { to: 20, ..query() }, vec![2, 4]),
            (
                Query {
                    channel: Some(8),
                    ..query()
                },
                vec![],
            ),
            (
                Query {
                    minimum_level: Level::Error,
                    ..query()
                },
                vec![3],
            ),
        ];
        for (q, expected) in cases {
            let other = s.start(q, 1, &cancel).unwrap();
            let page = s.page(&other, 1, &cancel).unwrap();
            assert_eq!(
                page.records
                    .iter()
                    .map(|r| r.record.receipt.accepted_seq)
                    .collect::<Vec<_>>(),
                expected
            );
            let page = s.page(&next, 1, &cancel).unwrap();
            assert_eq!(page.records[0].record.receipt.accepted_seq, 3);
            assert_eq!(page.records[1].record.receipt.accepted_seq, 1);
            assert_eq!(page.inline(&page.records[0]), Some([2; 3].as_slice()));
        }
        let restarted = s.start(query(), 1, &cancel).unwrap();
        assert_eq!(
            s.page(&restarted, 1, &cancel).unwrap().records[0]
                .record
                .receipt
                .accepted_seq,
            2
        );
        cancel.store(true, std::sync::atomic::Ordering::Release);
        assert!(matches!(
            s.page(&next, 1, &cancel),
            Err(QueryError::Cancelled)
        ));
        cancel.store(false, std::sync::atomic::Ordering::Release);
        assert_eq!(
            s.page(&next, 1, &cancel).unwrap().records[0]
                .record
                .receipt
                .accepted_seq,
            3
        );
        assert!(matches!(
            s.page(&next, 100, &cancel),
            Err(QueryError::Expired)
        ));
        assert!(matches!(
            s.page(&next, 1, &cancel),
            Err(QueryError::Expired)
        ));
    });
}
#[test]
fn byte_limit_counts_descriptors_and_large_payload_reads_stay_bounded() {
    with_writer(|mut w, f| {
        for seq in 1..=3 {
            let mut r = record(seq, seq, 100);
            r.payload_len = 4096;
            w.append(r, &vec![seq as u8; 4096]).unwrap();
        }
        w.flush_chunk().unwrap();
        let mut config = cfg();
        config.page_bytes = std::mem::size_of::<QueryRecord>();
        let mut s = snapshot(f, w.safe_offset(), config);
        let cancel = AtomicBool::new(false);
        let c = s.start(query(), 1, &cancel).unwrap();
        let p = s.page(&c, 1, &cancel).unwrap();
        assert_eq!(p.records.len(), 1);
        let handle = match &p.records[0].payload {
            Payload::Blob(h) => h.clone(),
            _ => panic!("expected blob"),
        };
        assert!(p.next.is_some());
        let mut foreign = snapshot(
            File(Arc::new(Mutex::new(IoCursor::new(Vec::new()))), 0),
            0,
            cfg(),
        );
        assert!(matches!(
            foreign.read_blob(&handle, 0, 1, 1, &cancel),
            Err(QueryError::Gone)
        ));

        assert_eq!(s.read_blob(&handle, 4000, 32, 1, &cancel).unwrap(), [1; 32]);
        assert!(matches!(
            s.read_blob(&handle, 0, 33, 1, &cancel),
            Err(QueryError::Limit)
        ));
        assert!(matches!(
            s.read_blob(&handle, 4090, 32, 1, &cancel),
            Err(QueryError::Gone)
        ));
        assert!(matches!(
            s.read_blob(&handle, 0, 1, 100, &cancel),
            Err(QueryError::Gone)
        ));
        assert!(matches!(
            s.read_blob(&handle, 0, 1, 1, &cancel),
            Err(QueryError::Gone)
        ));
    });
}
#[test]
fn cursor_and_blob_identity_expiry_and_cancel_are_explicit() {
    with_writer(|mut w, f| {
        w.append(record(1, 1, 1), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let mut a = snapshot(f.clone(), w.safe_offset(), cfg());
        let mut b = snapshot(f, w.safe_offset(), cfg());
        let no = AtomicBool::new(false);
        let c = a.start(query(), 1, &no).unwrap();
        assert!(matches!(b.page(&c, 1, &no), Err(QueryError::Cursor)));

        let yes = AtomicBool::new(true);
        assert!(matches!(a.page(&c, 1, &yes), Err(QueryError::Cancelled)));
        let mut q = query();
        q.basis = TimeBasis::Source(8);
        q.to = i64::MAX as i128 + 1;
        assert!(matches!(a.start(q, 1, &no), Err(QueryError::ClockDomain)));
        q.basis = TimeBasis::Source(9);
        let c = a.start(q, 1, &no).unwrap();
        assert_eq!(a.page(&c, 1, &no).unwrap().records.len(), 1);
        assert!(matches!(a.page(&c, 100, &no), Err(QueryError::Expired)));
    });
}
#[test]
fn malformed_or_over_limit_sources_do_not_produce_partial_snapshots() {
    with_writer(|mut w, f| {
        w.append(record(1, 1, 1), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let safe = w.safe_offset();
        let no = AtomicBool::new(false);
        for end in [safe - 1, safe + 1, u64::MAX] {
            let r = Snapshot::new(
                &budget(),
                42,
                1,
                channels(),
                vec![QuerySource::active(f.clone(), 1, end)],
                cfg(),
                0,
                100,
                &no,
            );
            assert!(r.is_err());
        }
        let mut body_limit = cfg();
        body_limit.max_record_bytes = 40;
        assert!(matches!(
            Snapshot::new(
                &budget(),
                42,
                1,
                channels(),
                vec![QuerySource::active(f.clone(), 1, safe)],
                body_limit,
                0,
                100,
                &no
            ),
            Err(QueryError::Limit)
        ));
        let mut c = cfg();
        c.max_records = 1;
        w.append(record(2, 2, 2), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        assert!(matches!(
            Snapshot::new(
                &budget(),
                42,
                1,
                channels(),
                vec![QuerySource::active(f, 1, w.safe_offset())],
                c,
                0,
                100,
                &no
            ),
            Err(QueryError::Limit)
        ));
    });
}
#[test]
fn closed_and_active_segments_merge_and_shared_reservations_return() {
    with_writer(|mut w, f| {
        w.append(record(1, 20, 100), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let old = f.0.lock().unwrap().get_ref().clone();
        w.append(record(2, 10, 90), &[1, 2, 3]).unwrap();
        let finished = w.finish().unwrap();
        let bound = finished.safe_offset();
        let budget = budget();
        let no = AtomicBool::new(false);
        let mut s = Snapshot::new(
            &budget,
            42,
            1,
            channels(),
            vec![
                QuerySource::active(
                    File(Arc::new(Mutex::new(IoCursor::new(old.clone()))), 0),
                    1,
                    old.len() as u64,
                ),
                QuerySource::active(f, 2, bound),
            ],
            cfg(),
            0,
            100,
            &no,
        )
        .unwrap();
        assert!(budget.used_bytes() > 0);
        let c = s.start(query(), 1, &no).unwrap();
        let p = s.page(&c, 1, &no).unwrap();
        assert_eq!(p.records[0].record.metadata.mono_ns, 10);
        drop(s);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn byte_read_and_shared_memory_limits_reject_without_leaking_reservations() {
    with_writer(|mut w, f| {
        w.append(record(1, 1, 1), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let no = AtomicBool::new(false);
        let c = cfg();
        let needed = c.reservation(&channels()).unwrap();
        let mut plan = support::plan().config().clone();
        plan.memory.external_bytes = needed;
        plan.memory.total_bytes += needed;
        let budget = QueryBudget::new(
            &SessionBuilder::new(plan)
                .validate(&PlatformCapabilities::default())
                .unwrap(),
        );
        let source = || vec![QuerySource::active(f.clone(), 1, w.safe_offset())];
        let s = Snapshot::new(&budget, 42, 1, channels(), source(), c, 0, 100, &no).unwrap();
        assert_eq!(budget.used_bytes(), needed);
        assert!(matches!(
            Snapshot::new(&budget, 42, 1, channels(), source(), c, 0, 100, &no),
            Err(QueryError::Limit)
        ));
        drop(s);
        assert_eq!(budget.used_bytes(), 0);
        let mut c = cfg();
        c.read_bytes = w.safe_offset() - 1;
        assert!(matches!(
            Snapshot::new(&budget, 42, 1, channels(), source(), c, 0, 100, &no),
            Err(QueryError::Limit)
        ));
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn a_corrupt_complete_chunk_is_rejected_and_keeps_source_bytes_intact() {
    with_writer(|mut w, f| {
        w.append(record(1, 1, 1), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let safe = w.safe_offset();
        let mut bytes = f.0.lock().unwrap().get_ref().clone();
        let mut at = 8;
        while bytes[at] != 6 {
            at += 9 + u64::from_le_bytes(bytes[at + 1..at + 9].try_into().unwrap()) as usize;
        }
        bytes[at + 9 + 24] ^= 1;
        let original = bytes.clone();
        let bad = File(Arc::new(Mutex::new(IoCursor::new(bytes))), 0);
        assert!(matches!(
            Snapshot::new(
                &budget(),
                42,
                1,
                channels(),
                vec![QuerySource::active(bad.clone(), 1, safe)],
                cfg(),
                0,
                100,
                &AtomicBool::new(false)
            ),
            Err(QueryError::CorruptData)
        ));
        assert_eq!(*bad.0.lock().unwrap().get_ref(), original);
    });
}

#[test]
fn observed_expiry_cannot_be_reversed_with_an_older_time_argument() {
    with_writer(|mut w, f| {
        w.append(record(1, 1, 1), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let mut s = snapshot(f, w.safe_offset(), cfg());
        let no = AtomicBool::new(false);
        let c = s.start(query(), 1, &no).unwrap();
        assert!(matches!(s.page(&c, 100, &no), Err(QueryError::Expired)));
        assert!(matches!(s.page(&c, 1, &no), Err(QueryError::Expired)));
    });
}
