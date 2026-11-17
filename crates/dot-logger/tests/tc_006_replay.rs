//! TASK-011: virtual-clock scheduling and bounded replay over frozen files.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    capture::*, config::*, platform::*, query::*, replay::*, routing::Router, storage::*,
};
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
fn two_channels() -> Vec<StorageChannel> {
    let mut c = channels();
    c.push(StorageChannel {
        channel: ChannelConfig::new(8, 99, 1),
        topic: "other".into(),
        payload_encoding: "opaque".into(),
        schema: None,
    });
    c
}
fn with_writer(f: impl FnOnce(SegmentWriter<File>, File)) {
    with_writer_on(channels(), f)
}
fn with_writer_on(storage: Vec<StorageChannel>, f: impl FnOnce(SegmentWriter<File>, File)) {
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
            storage,
            file.clone(),
        )
        .unwrap();
        f(w, file);
    });
}
fn snapshot(file: File, safe: u64, c: QueryConfig) -> Snapshot<File> {
    snapshot_on(channels(), file, safe, c)
}
fn snapshot_on(
    storage: Vec<StorageChannel>,
    file: File,
    safe: u64,
    c: QueryConfig,
) -> Snapshot<File> {
    Snapshot::new(
        &budget(),
        42,
        storage,
        vec![QuerySource::active(file, 0, 1, safe)],
        c,
        0,
        100,
        &AtomicBool::new(false),
    )
    .unwrap()
}

fn replay(file: File, safe: u64, speed: Speed) -> Replay<File> {
    Replay::new(
        snapshot(file, safe, cfg()),
        query(),
        speed,
        0,
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn delivered(r: &mut Replay<File>, now: u64) -> (u64, u64, u64) {
    match r.poll(now, true, &AtomicBool::new(false)).unwrap() {
        ReplayPoll::Ready(frame) => {
            assert!(matches!(frame.payload, ReplayPayload::Inline(_)));
            (
                frame.record.receipt.accepted_seq,
                frame.generation,
                frame.late_ns,
            )
        }
        _ => panic!("expected delivery"),
    }
}
#[test]
fn mono_order_speed_pause_and_late_consumer_preserve_spacing() {
    with_writer(|mut w, f| {
        for (i, t) in [30, 10, 20].into_iter().enumerate() {
            w.append(record(i as u64 + 1, t, 100 - t as i64), &[1, 2, 3])
                .unwrap();
        }
        w.flush_chunk().unwrap();
        let mut r = replay(f, w.safe_offset(), Speed::new(1, 1).unwrap());
        let no = AtomicBool::new(false);
        assert!(matches!(
            r.poll(9, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 10, .. }
        ));
        assert_eq!(delivered(&mut r, 10), (2, 1, 0));
        r.pause(15, &no).unwrap();
        assert!(matches!(r.poll(40, true, &no).unwrap(), ReplayPoll::Paused));
        r.resume(50, &no).unwrap();
        assert!(matches!(
            r.poll(54, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 55, .. }
        ));
        assert_eq!(delivered(&mut r, 55), (3, 1, 0));
        assert!(matches!(
            r.poll(70, false, &no).unwrap(),
            ReplayPoll::Waiting {
                due_ns: 65,
                late_ns: 5
            }
        ));
        assert_eq!(delivered(&mut r, 75), (1, 1, 10));
        assert!(matches!(r.poll(76, true, &no).unwrap(), ReplayPoll::End));
    });
}
#[test]
fn seek_discards_prefetch_increments_generation_and_preserves_pause() {
    with_writer(|mut w, f| {
        for i in 1..=3 {
            w.append(record(i, i * 10, 100), &[i as u8; 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let mut r = replay(f, w.safe_offset(), Speed::new(1, 1).unwrap());
        let no = AtomicBool::new(false);
        r.poll(1, false, &no).unwrap();
        r.pause(2, &no).unwrap();
        let transition = r.seek(20, 3, &no).unwrap();
        assert_eq!(
            (transition.generation, transition.discarded_prefetch),
            (2, true)
        );
        assert!(matches!(r.poll(4, true, &no).unwrap(), ReplayPoll::Paused));
        r.resume(5, &no).unwrap();
        assert_eq!(delivered(&mut r, 5), (2, 2, 0));
        assert_eq!(delivered(&mut r, 15), (3, 2, 0));
        r.seek(0, 16, &no).unwrap();
        r.set_speed(Speed::new(2, 1).unwrap(), 16, &no).unwrap();
        assert_eq!(delivered(&mut r, 21), (1, 3, 0));
        assert!(matches!(
            r.poll(22, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 26, .. }
        ));
        r.stop();
        assert!(matches!(
            r.poll(23, true, &no).unwrap(),
            ReplayPoll::Stopped
        ));
    });
}
#[test]
fn finite_rate_changes_are_continuous_and_backwards_host_time_never_rewinds() {
    assert!(Speed::new(0, 1).is_err());
    assert!(Speed::new(1, 0).is_err());
    with_writer(|mut w, f| {
        w.append(record(1, 20, 0), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let mut r = replay(f, w.safe_offset(), Speed::new(1, 2).unwrap());
        let no = AtomicBool::new(false);
        assert!(matches!(
            r.poll(10, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 40, .. }
        ));
        r.set_speed(Speed::new(2, 1).unwrap(), 10, &no).unwrap();
        assert!(matches!(
            r.poll(1, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 18, .. }
        ));
        assert_eq!(delivered(&mut r, 18), (1, 1, 0));
    });
}
#[test]
fn speed_and_borrowed_deliveries_support_public_diagnostics() {
    let speed = Speed::new(6, 4).unwrap();
    assert_eq!((speed.numerator(), speed.denominator()), (6, 4));
    assert_eq!(speed.to_string(), "6/4");
    assert_eq!(Speed::REALTIME, Speed::new(1, 1).unwrap());
    with_writer(|mut w, f| {
        w.append(record(1, 10, 0), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let mut r = replay(f, w.safe_offset(), Speed::REALTIME);
        let no = AtomicBool::new(false);
        let waiting = r.poll(9, true, &no).unwrap();
        assert!(format!("{waiting:?}").contains("Waiting"));
        let ready = r.poll(10, true, &no).unwrap();
        assert!(format!("{ready:?}").contains("Ready"));
        match ready {
            ReplayPoll::Ready(frame) => {
                assert!(format!("{frame:?}").contains("generation: 1"));
                assert_eq!(format!("{:?}", frame.payload), "Inline([1, 2, 3])");
            }
            other => panic!("expected delivery: {other:?}"),
        }
    });
}

#[test]
fn late_delivery_near_host_limit_preserves_spacing_or_stops_and_releases_budget() {
    with_writer(|mut w, f| {
        for i in 1..=2 {
            w.append(record(i, i * 10, 0), &[1, 2, 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let no = AtomicBool::new(false);
        let start = u64::MAX - 100;
        for now in [u64::MAX - 11, u64::MAX - 2] {
            let budget = budget();
            let s = Snapshot::new(
                &budget,
                42,
                channels(),
                vec![QuerySource::active(f.clone(), 0, 1, w.safe_offset())],
                cfg(),
                start,
                u64::MAX,
                &no,
            )
            .unwrap();
            let mut r = Replay::new(s, query(), Speed::new(1, 1).unwrap(), start, &no).unwrap();
            assert_eq!(delivered(&mut r, now), (1, 1, now - (start + 10)));
            assert_eq!(r.state(), ReplayState::Playing);
            assert!(budget.used_bytes() > 0);
            if let Some(due) = now.checked_add(10) {
                assert!(matches!(
                    r.poll(due - 1, true, &no).unwrap(),
                    ReplayPoll::Waiting { due_ns, .. } if due_ns == due
                ));
                assert_eq!(delivered(&mut r, due), (2, 1, 0));
                r.stop();
            } else {
                assert!(matches!(r.poll(now, true, &no), Err(ReplayError::Limit)));
                assert_eq!(r.state(), ReplayState::Stopped);
                assert!(matches!(
                    r.poll(now, true, &no).unwrap(),
                    ReplayPoll::Stopped
                ));
            }
            assert_eq!(budget.used_bytes(), 0);
        }
    });
}
#[test]
fn late_delivery_slows_next_due_without_scanning_ahead_and_stop_releases_budget() {
    with_writer(|mut w, f| {
        for i in 1..=3 {
            w.append(record(i, i * 10, 100), &[i as u8; 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let budget = budget();
        let no = AtomicBool::new(false);
        let s = Snapshot::new(
            &budget,
            42,
            channels(),
            vec![QuerySource::active(f, 0, 1, w.safe_offset())],
            cfg(),
            0,
            100,
            &no,
        )
        .unwrap();
        let mut r = Replay::new(s, query(), Speed::new(1, 1).unwrap(), 0, &no).unwrap();
        assert!(budget.used_bytes() > 0);
        assert!(matches!(
            r.poll(40, false, &no).unwrap(),
            ReplayPoll::Waiting {
                due_ns: 10,
                late_ns: 30
            }
        ));
        assert_eq!(delivered(&mut r, 40), (1, 1, 30));
        assert!(matches!(
            r.poll(49, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 50, .. }
        ));
        assert_eq!(delivered(&mut r, 50), (2, 1, 0));
        r.stop();
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn speed_change_while_overdue_does_not_collapse_future_record_spacing() {
    with_writer(|mut w, f| {
        for i in 1..=3 {
            w.append(record(i, i * 10, 0), &[1, 2, 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let mut r = replay(f, w.safe_offset(), Speed::new(1, 1).unwrap());
        let no = AtomicBool::new(false);
        r.poll(1, false, &no).unwrap();
        r.set_speed(Speed::new(2, 1).unwrap(), 40, &no).unwrap();
        assert_eq!(delivered(&mut r, 40).0, 1);
        assert!(matches!(
            r.poll(44, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 45, .. }
        ));
        assert_eq!(delivered(&mut r, 45).0, 2);
    });
}
#[test]
fn failed_blob_io_is_terminal_and_releases_the_owned_snapshot() {
    with_writer(|mut w, f| {
        let mut rec = record(1, 1, 0);
        rec.payload_len = 64;
        w.append(rec, &[3; 64]).unwrap();
        w.flush_chunk().unwrap();
        let no = AtomicBool::new(false);
        let budget = budget();
        let source = IoFailure {
            file: f.clone(),
            fail: Arc::new(AtomicBool::new(false)),
        };
        let fail = Arc::clone(&source.fail);
        let s = Snapshot::new(
            &budget,
            42,
            channels(),
            vec![QuerySource::active(source, 0, 1, w.safe_offset())],
            cfg(),
            0,
            100,
            &no,
        )
        .unwrap();
        let mut r = Replay::new(s, query(), Speed::new(1, 1).unwrap(), 0, &no).unwrap();
        let handle = match r.poll(1, true, &no).unwrap() {
            ReplayPoll::Ready(frame) => match frame.payload {
                ReplayPayload::Blob(h) => h.clone(),
                _ => panic!("blob"),
            },
            _ => panic!("ready"),
        };
        fail.store(true, std::sync::atomic::Ordering::Release);
        assert!(matches!(
            r.read_blob(1, &handle, 0, 1, 2, &no),
            Err(ReplayError::Query(QueryError::Io(_)))
        ));
        assert_eq!(r.state(), ReplayState::Stopped);
        assert_eq!(budget.used_bytes(), 0);
    });
}
struct IoFailure {
    file: File,
    fail: Arc<AtomicBool>,
}
impl Read for IoFailure {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        if self.fail.load(std::sync::atomic::Ordering::Acquire) {
            return Err(io::Error::other("injected replay read"));
        }
        self.file.read(b)
    }
}
impl Seek for IoFailure {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.file.seek(p)
    }
}

#[test]
fn one_record_prefetch_does_not_reread_while_blocked_and_read_exhaustion_closes() {
    with_writer(|mut w, f| {
        for i in 1..=3 {
            w.append(record(i, i * 10, 0), &[i as u8; 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let no = AtomicBool::new(false);
        let budget = budget();
        let mut config = cfg();
        config.read_bytes = w.safe_offset() + 3;
        let s = Snapshot::new(
            &budget,
            42,
            channels(),
            vec![QuerySource::active(f, 0, 1, w.safe_offset())],
            config,
            0,
            100,
            &no,
        )
        .unwrap();
        let mut r = Replay::new(s, query(), Speed::new(1, 1).unwrap(), 0, &no).unwrap();
        for now in 1..=40 {
            assert!(matches!(
                r.poll(now, false, &no).unwrap(),
                ReplayPoll::Waiting { .. }
            ));
        }
        assert_eq!(delivered(&mut r, 40), (1, 1, 30));
        assert!(matches!(
            r.poll(41, true, &no),
            Err(ReplayError::Query(QueryError::Limit))
        ));
        assert_eq!(r.state(), ReplayState::Stopped);
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn stale_blob_generation_and_invalid_seek_do_not_change_a_valid_replay() {
    with_writer(|mut w, f| {
        let mut rec = record(1, 10, 0);
        rec.payload_len = 64;
        w.append(rec, &[8; 64]).unwrap();
        w.flush_chunk().unwrap();
        let mut selection = query();
        selection.to = 100;
        let mut r = Replay::new(
            snapshot(f, w.safe_offset(), cfg()),
            selection,
            Speed::new(1, 1).unwrap(),
            0,
            &AtomicBool::new(false),
        )
        .unwrap();
        let no = AtomicBool::new(false);
        let handle = match r.poll(10, true, &no).unwrap() {
            ReplayPoll::Ready(frame) => match frame.payload {
                ReplayPayload::Blob(h) => h.clone(),
                _ => panic!("blob"),
            },
            _ => panic!("ready"),
        };
        assert_eq!(r.read_blob(1, &handle, 60, 4, 11, &no).unwrap(), [8; 4]);
        assert!(matches!(r.poll(12, true, &no).unwrap(), ReplayPoll::End));
        r.seek(0, 13, &no).unwrap();
        assert_eq!(r.generation(), 2);
        assert!(matches!(
            r.read_blob(1, &handle, 0, 1, 14, &no),
            Err(ReplayError::StaleGeneration)
        ));
        assert_eq!(r.state(), ReplayState::Playing);
        r.poll(15, false, &no).unwrap();
        assert!(matches!(
            r.seek(u64::MAX, 16, &no),
            Err(ReplayError::Query(QueryError::InvalidInput))
        ));
        assert_eq!(r.generation(), 2);
        let frame = match r.poll(23, true, &no).unwrap() {
            ReplayPoll::Ready(f) => f,
            _ => panic!("ready"),
        };
        assert_eq!(frame.record.receipt.accepted_seq, 1);
    });
}
#[test]
fn cancellation_and_expiry_close_readers_even_when_paused_or_waiting() {
    with_writer(|mut w, f| {
        w.append(record(1, 10, 0), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        for paused in [false, true] {
            for expired in [false, true] {
                let no = AtomicBool::new(false);
                let budget = budget();
                let s = Snapshot::new(
                    &budget,
                    42,
                    channels(),
                    vec![QuerySource::active(f.clone(), 0, 1, w.safe_offset())],
                    cfg(),
                    0,
                    100,
                    &no,
                )
                .unwrap();
                let mut r = Replay::new(s, query(), Speed::new(1, 1).unwrap(), 0, &no).unwrap();
                r.poll(1, false, &no).unwrap();
                if paused {
                    r.pause(2, &no).unwrap();
                }
                let cancel = AtomicBool::new(!expired);
                assert!(matches!(
                    r.poll(if expired { 100 } else { 3 }, true, &cancel),
                    Err(ReplayError::Query(
                        QueryError::Expired | QueryError::Cancelled
                    ))
                ));
                assert_eq!(budget.used_bytes(), 0);
                assert!(matches!(r.poll(0, true, &no).unwrap(), ReplayPoll::Stopped));
                assert!(!r.stop().discarded_prefetch);
            }
        }
    });
}
#[test]
fn empty_selection_gaps_bad_axis_and_schedule_overflow_are_explicit() {
    with_writer(|mut w, f| {
        w.append(record(1, 30, 0), &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let no = AtomicBool::new(false);
        let mut empty = query();
        empty.channel = Some(8);
        let mut r = Replay::new(
            snapshot(f.clone(), w.safe_offset(), cfg()),
            empty,
            Speed::new(1, 1).unwrap(),
            0,
            &no,
        )
        .unwrap();
        assert!(matches!(r.poll(0, true, &no).unwrap(), ReplayPoll::End));
        let mut q = query();
        q.basis = TimeBasis::Real;
        q.to = 100;
        assert!(matches!(
            Replay::new(
                snapshot(f.clone(), w.safe_offset(), cfg()),
                q,
                Speed::new(1, 1).unwrap(),
                0,
                &no
            ),
            Err(ReplayError::InvalidInput)
        ));
        let mut r = replay(f.clone(), w.safe_offset(), Speed::new(1, 1).unwrap());
        assert!(matches!(
            r.poll(29, true, &no).unwrap(),
            ReplayPoll::Waiting { due_ns: 30, .. }
        ));
        assert_eq!(delivered(&mut r, 30), (1, 1, 0));
        let mut rec = record(2, u64::MAX, 0);
        rec.metadata.source_time = None;
        w.append(rec, &[1, 2, 3]).unwrap();
        w.flush_chunk().unwrap();
        let mut q = query();
        q.from = 31;
        let mut r = Replay::new(
            snapshot(f, w.safe_offset(), cfg()),
            q,
            Speed::new(1, u32::MAX).unwrap(),
            0,
            &no,
        )
        .unwrap();
        assert!(matches!(r.poll(0, true, &no), Err(ReplayError::Limit)));
        assert_eq!(r.state(), ReplayState::Stopped);
    });
}

#[test]
fn speed_equivalence_preserves_components_and_handles_u32_products() {
    let one = Speed::REALTIME;
    let unreduced = Speed::new(u32::MAX, u32::MAX).unwrap();
    assert_ne!(one, unreduced);
    assert!(one.is_equivalent(&unreduced));
    assert!(
        Speed::new(3, 2)
            .unwrap()
            .is_equivalent(&Speed::new(6, 4).unwrap())
    );
    let fast = Speed::new(u32::MAX, u32::MAX - 1).unwrap();
    assert!(fast.is_equivalent(&fast));
    assert!(!fast.is_equivalent(&one));
    assert!(!one.is_equivalent(&fast));
    assert_eq!(unreduced.numerator(), u32::MAX);
    assert_eq!(unreduced.denominator(), u32::MAX);
}

#[test]
fn replay_error_display_explains_failures_and_preserves_io_diagnostics() {
    for (error, explanation) in [
        (ReplayError::InvalidInput, "non-mono filter or invalid rate"),
        (
            ReplayError::InvalidState,
            "operation is incompatible with the current replay state",
        ),
        (
            ReplayError::Limit,
            "scheduling deadline or seek generation limit exceeded",
        ),
        (
            ReplayError::StaleGeneration,
            "blob reference predates the current seek generation",
        ),
    ] {
        assert_eq!(error.to_string(), explanation);
    }
    let error = ReplayError::Query(QueryError::Io(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "replay file access denied",
    )));
    let message = error.to_string();
    assert!(message.starts_with("replay query failed: query error:"));
    assert!(message.contains("replay file access denied"));
    assert!(
        matches!(error, ReplayError::Query(QueryError::Io(ref e)) if e.kind() == io::ErrorKind::PermissionDenied)
    );
}

/// Poll at each record's due time until End, returning `(channel, seq, mono)`
/// per delivery. Fails if the replay waits past `limit_ns` without ending.
fn run_to_end(r: &mut Replay<File>, limit_ns: u64) -> Vec<(u32, u64, u64)> {
    let no = AtomicBool::new(false);
    let mut got = vec![];
    let mut now = 0;
    loop {
        assert!(now <= limit_ns, "replay did not end by {limit_ns}");
        match r.poll(now, true, &no).unwrap() {
            ReplayPoll::Ready(frame) => {
                let m = &frame.record.metadata;
                got.push((m.channel_id, frame.record.receipt.accepted_seq, m.mono_ns));
            }
            ReplayPoll::Waiting { due_ns, .. } => now = due_ns,
            ReplayPoll::End => return got,
            other => panic!("unexpected poll: {other:?}"),
        }
    }
}

/// TASK-033 S3 (UC-24): replay with a finite `to` delivers only records before
/// that exclusive bound and then ends normally, without waiting for later ones.
#[test]
fn replay_with_upper_bound_stops_before_to_and_ends_normally() {
    with_writer(|mut w, f| {
        for (seq, t) in [(1, 10), (2, 20), (3, 30), (4, 40)] {
            w.append(record(seq, t, 0), &[seq as u8; 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let no = AtomicBool::new(false);
        let bounded = Query { to: 30, ..query() };
        let mut r = Replay::new(
            snapshot(f, w.safe_offset(), cfg()),
            bounded,
            Speed::REALTIME,
            0,
            &no,
        )
        .unwrap();
        assert_eq!(run_to_end(&mut r, 99), [(7, 1, 10), (7, 2, 20)]);
        assert_eq!(r.state(), ReplayState::Ended);
        assert!(matches!(r.poll(99, true, &no).unwrap(), ReplayPoll::End));
    });
}

/// TASK-033 S3 (UC-24): replay with a positive channel selector delivers only
/// that channel, keeping original spacing, and ends normally.
#[test]
fn replay_with_positive_channel_filter_delivers_only_that_channel() {
    with_writer_on(two_channels(), |mut w, f| {
        for (seq, (channel, t)) in [(7, 10), (8, 20), (7, 30), (8, 40)].into_iter().enumerate() {
            let mut r = record(seq as u64 + 1, t, 0);
            r.metadata.channel_id = channel;
            w.append(r, &[channel as u8; 3]).unwrap();
        }
        w.flush_chunk().unwrap();
        let no = AtomicBool::new(false);
        let only = Query {
            channel: Some(8),
            ..query()
        };
        let mut r = Replay::new(
            snapshot_on(two_channels(), f, w.safe_offset(), cfg()),
            only,
            Speed::REALTIME,
            0,
            &no,
        )
        .unwrap();
        assert_eq!(run_to_end(&mut r, 40), [(8, 2, 20), (8, 4, 40)]);
        assert_eq!(r.state(), ReplayState::Ended);
    });
}
