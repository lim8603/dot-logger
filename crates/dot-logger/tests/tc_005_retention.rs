//! TC-005 Task-008: bounded rotations, durable publication, events and read pins.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::capture::{CaptureOwner, CaptureReceipt, CapturedRecord, ChannelConfig};
use dot_logger::routing::Router;
use dot_logger::storage::*;
use std::cell::RefCell;
use std::io::{self, Cursor, Seek, SeekFrom, Write};
use std::rc::Rc;

#[derive(Default)]
struct State {
    ops: Vec<String>,
    files: std::collections::BTreeMap<u64, Vec<u8>>,
    free: u64,
    free_probes: usize,
    fail: Option<&'static str>,
}
#[derive(Clone)]
struct Store(Rc<RefCell<State>>);
struct Sink {
    seq: u64,
    data: Cursor<Vec<u8>>,
    state: Rc<RefCell<State>>,
}
impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.state.borrow().fail == Some("write") {
            return Err(io::Error::other("write"));
        }
        self.data.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Seek for Sink {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.data.seek(p)
    }
}
impl SyncSink for Sink {
    fn sync_all(&mut self) -> io::Result<()> {
        let mut s = self.state.borrow_mut();
        s.ops.push(format!("sync:{}", self.seq));
        if s.fail == Some("sync") {
            return Err(io::Error::other("sync"));
        }
        s.files.insert(self.seq, self.data.get_ref().clone());
        Ok(())
    }
}
impl SegmentStore for Store {
    type Sink = Sink;
    fn directory_sync_supported(&self) -> bool {
        true
    }
    fn available_bytes(&mut self) -> io::Result<u64> {
        let mut state = self.0.borrow_mut();
        state.free_probes += 1;
        Ok(state.free)
    }
    fn create(&mut self, sequence: u64) -> io::Result<Sink> {
        self.0.borrow_mut().ops.push(format!("create:{sequence}"));
        Ok(Sink {
            seq: sequence,
            data: Cursor::new(Vec::new()),
            state: Rc::clone(&self.0),
        })
    }
    fn finalize(&mut self, sequence: u64) -> io::Result<()> {
        let mut s = self.0.borrow_mut();
        s.ops.push(format!("rename:{sequence}"));
        if s.fail == Some("rename") {
            return Err(io::Error::other("rename"));
        }
        Ok(())
    }
    fn sync_directory(&mut self) -> io::Result<()> {
        let mut s = self.0.borrow_mut();
        s.ops.push("dir".into());
        if s.fail == Some("dir") {
            return Err(io::Error::other("dir"));
        }
        Ok(())
    }
    fn remove(&mut self, sequence: u64) -> io::Result<()> {
        let mut s = self.0.borrow_mut();
        s.ops.push(format!("delete:{sequence}"));
        if s.fail == Some("delete") {
            return Err(io::Error::other("delete"));
        }
        s.files.remove(&sequence);
        Ok(())
    }
}
fn writer() -> WriterConfig {
    WriterConfig {
        max_loss_reports: 0,
        max_payload_bytes: 16,
        write_buffer_bytes: 64,
        chunk_target_bytes: 1024,
        max_records: 2,
        max_chunks: 4,
        max_metadata_bytes: 128,
    }
}
fn retention(mode: RetentionMode) -> RetentionConfig {
    RetentionConfig {
        mode,
        disk_bytes: 1_000_000,
        disk_reserve_bytes: 100,
        max_age_ns: 1000,
        max_segment_bytes: 100_000,
        max_segment_ns: 100,
        max_index_bytes: 16_384,
        protected_bytes: 300_000,
        pre_ns: 10,
        post_ns: 10,
        max_segments: 8,
        max_events: 4,
        max_pins: 4,
        max_pin_bytes: 4096,
        max_pin_ns: 100,
    }
}
fn channels() -> Vec<StorageChannel> {
    vec![StorageChannel {
        channel: ChannelConfig::new(7, 99, 1),
        topic: "a".into(),
        payload_encoding: "opaque".into(),
        schema: None,
    }]
}
fn record(seq: u64, mono: u64) -> CapturedRecord {
    let mut metadata = support::meta();
    metadata.mono_ns = mono;
    metadata.importance = Some(dot_logger::capture::Importance::Important);
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: seq,
        },
        metadata,
        revision: 1,
        payload_len: 1,
    }
}
fn with_log(
    config: RetentionConfig,
    limits: WriterConfig,
    f: impl FnOnce(SegmentLog<Store>, Rc<RefCell<State>>),
) {
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|mut scope| {
        let (_, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let router = Router::new(&plan, vec![a, b], support::routing()).unwrap();
        let state = Rc::new(RefCell::new(State {
            free: u64::MAX,
            ..State::default()
        }));
        let log = SegmentLog::new(
            router.writer_budget(10).unwrap(),
            limits,
            channels(),
            config,
            Store(Rc::clone(&state)),
            0,
        )
        .unwrap();
        f(log, state);
    });
}
#[test]
fn record_rotation_syncs_footer_renames_and_syncs_directory_before_publication() {
    with_log(
        retention(RetentionMode::Continuous),
        writer(),
        |mut log, state| {
            log.append(record(1, 1), &[1], 1).unwrap();
            log.append(record(2, 2), &[2], 2).unwrap();
            let result = log.append(record(3, 3), &[3], 3).unwrap();
            assert_eq!(result.rotated, Some(RotationReason::Records));
            assert_eq!(log.segments().len(), 1);
            assert_eq!(
                state.borrow().ops,
                ["create:0", "sync:0", "rename:0", "dir", "create:1"]
            );
            let bytes = state.borrow().files[&0].clone();
            assert_eq!(mcap::MessageStream::new(&bytes).unwrap().count(), 4);
        },
    );
}
#[test]
fn rotating_append_avoids_a_redundant_host_probe_and_later_appends_still_check_reserve() {
    with_log(
        retention(RetentionMode::Continuous),
        writer(),
        |mut log, state| {
            log.append(record(1, 1), &[1], 1).unwrap();
            log.append(record(2, 2), &[2], 2).unwrap();
            let before = state.borrow().free_probes;
            let report = log.append(record(3, 3), &[3], 3).unwrap();
            assert_eq!(report.rotated, Some(RotationReason::Records));
            // Rotation checks capacity before closing and before opening the next file.
            assert_eq!(state.borrow().free_probes - before, 2);
            assert_eq!(log.segments()[0].records, 2);

            state.borrow_mut().free = 100_099;
            let before = state.borrow().free_probes;
            assert!(matches!(
                log.append(record(4, 4), &[4], 4),
                Err(RetentionError::Capacity)
            ));
            assert_eq!(state.borrow().free_probes - before, 1);
            state.borrow_mut().free = 100_100;
            log.append(record(4, 4), &[4], 4).unwrap();
            log.finish(5).unwrap();
            let bytes = state.borrow().files[&1].clone();
            assert_eq!(mcap::MessageStream::new(&bytes).unwrap().count(), 4);
        },
    );
}
#[test]
fn lost_pin_keeps_deletion_protection_and_pin_capacity_after_its_deadline() {
    let mut cfg = retention(RetentionMode::Circular);
    cfg.max_segments = 2;
    cfg.max_pins = 1;
    with_log(cfg, writer(), |mut log, state| {
        for n in 1..=3 {
            log.append(record(n, n), &[1], n).unwrap();
        }
        let pin = log.pin(0, 1, 3, 4).unwrap();
        drop(pin);
        assert!(matches!(log.pin(0, 1, 4, 5), Err(RetentionError::PinLimit)));
        log.append(record(4, 4), &[1], 4).unwrap();
        assert!(matches!(
            log.append(record(5, 5), &[1], 5),
            Err(RetentionError::Capacity)
        ));
        assert!(!state.borrow().ops.iter().any(|op| op == "delete:0"));
        assert_eq!(log.segments()[0].sequence, 0);
    });
}
#[test]
fn continuous_stops_without_deleting_and_circular_skips_event_and_read_pins() {
    let mut cfg = retention(RetentionMode::Continuous);
    cfg.max_segments = 2;
    with_log(cfg, writer(), |mut log, state| {
        for n in 1..=4 {
            log.append(record(n, n), &[1], n).unwrap();
        }
        assert!(matches!(
            log.append(record(5, 5), &[1], 5),
            Err(RetentionError::Capacity)
        ));
        assert!(!state.borrow().ops.iter().any(|op| op.starts_with("delete")));
    });
    cfg.mode = RetentionMode::Circular;
    with_log(cfg, writer(), |mut log, state| {
        for n in 1..=3 {
            log.append(record(n, n), &[1], n).unwrap();
        }
        let pin = log.pin(0, 32, 3, 20).unwrap();
        log.append(record(4, 4), &[1], 4).unwrap();
        assert!(matches!(
            log.append(record(5, 5), &[1], 5),
            Err(RetentionError::Capacity)
        ));
        assert!(matches!(
            log.consume_pin(&pin, 1, 20),
            Err(RetentionError::Expired)
        ));
        assert!(!state.borrow().ops.iter().any(|op| op == "delete:0"));
        log.release_pin(pin).unwrap();
        log.append(record(5, 5), &[1], 21).unwrap();
        assert!(state.borrow().ops.iter().any(|op| op == "delete:0"));
    });
}
#[test]
fn overlapping_events_charge_unique_files_and_protect_future_post_window() {
    let mut cfg = retention(RetentionMode::Event);
    cfg.max_segments = 2;
    with_log(cfg, writer(), |mut log, state| {
        log.append(record(1, 15), &[1], 15).unwrap();
        log.append(record(2, 20), &[1], 20).unwrap();
        let a = log.protect_event(20, 100, 20).unwrap();
        let charged = log.protected_bytes();
        let b = log.protect_event(20, 101, 20).unwrap();
        assert_eq!(log.protected_bytes(), charged);
        assert_eq!(a.requested, (10, 30));
        assert_eq!(a.actual, Some((15, 20)));
        assert!(a.pre_truncated);
        log.append(record(3, 25), &[1], 25).unwrap();
        log.append(record(4, 30), &[1], 30).unwrap();
        assert!(matches!(
            log.append(record(5, 31), &[1], 31),
            Err(RetentionError::Capacity)
        ));
        assert!(!state.borrow().ops.iter().any(|op| op.starts_with("delete")));
        log.release_event(a.event_id).unwrap();
        assert!(log.protected_bytes() > 0);
        log.release_event(b.event_id).unwrap();
        log.append(record(5, 31), &[1], 31).unwrap();
    });
}
#[test]
fn failures_never_publish_a_closed_file_and_poison_further_writes() {
    for fail in ["sync", "rename", "dir"] {
        with_log(
            retention(RetentionMode::Continuous),
            writer(),
            |mut log, state| {
                log.append(record(1, 1), &[1], 1).unwrap();
                state.borrow_mut().fail = Some(fail);
                assert!(log.rotate(2).is_err());
                assert!(log.segments().is_empty());
                assert!(matches!(
                    log.append(record(2, 2), &[2], 2),
                    Err(RetentionError::Poisoned)
                ));
            },
        );
    }
}

#[test]
fn bytes_time_and_index_limits_rotate_without_losing_the_trigger_record() {
    for reason in [
        RotationReason::Bytes,
        RotationReason::Time,
        RotationReason::Index,
    ] {
        let mut cfg = retention(RetentionMode::Continuous);
        let mut limits = writer();
        limits.max_records = 64;
        match reason {
            RotationReason::Bytes => cfg.max_segment_bytes = 24_000,
            RotationReason::Time => cfg.max_segment_ns = 2,
            RotationReason::Index => {
                limits.max_chunks = 1;
                limits.chunk_target_bytes = 1;
            }
            _ => unreachable!(),
        }
        with_log(cfg, limits, |mut log, state| {
            let mut observed = false;
            for n in 1..=8 {
                let r = log.append(record(n, n), &[n as u8], n).unwrap();
                if r.rotated == Some(reason) {
                    observed = true;
                    break;
                }
            }
            assert!(observed, "{reason:?}");
            log.finish(10).unwrap();
            let total: usize = state
                .borrow()
                .files
                .values()
                .map(|b| NativeReader::new(b, 16, 4096).unwrap().count())
                .sum();
            assert_eq!(
                total,
                log.segments().iter().map(|s| s.records).sum::<usize>()
            );
            assert!(
                log.segments()
                    .iter()
                    .all(|s| s.bytes <= cfg.max_segment_bytes)
            );
        });
    }
}
#[test]
fn event_quota_rejects_history_and_future_growth_and_expiry_is_explicit() {
    let mut cfg = retention(RetentionMode::Event);
    cfg.protected_bytes = 100_000;
    with_log(cfg, writer(), |mut log, _| {
        log.append(record(1, 15), &[1], 15).unwrap();
        let e = log.protect_event(20, 100, 20).unwrap();
        log.append(record(2, 20), &[1], 20).unwrap();
        assert!(matches!(
            log.append(record(3, 25), &[1], 25),
            Err(RetentionError::EventQuota)
        ));
        log.release_event(e.event_id).unwrap();
        log.append(record(3, 25), &[1], 25).unwrap();
        assert!(matches!(
            log.protect_event(25, 100, 25),
            Err(RetentionError::EventQuota)
        ));
        assert_eq!(log.protected_bytes(), 0);
    });
    with_log(retention(RetentionMode::Event), writer(), |mut log, _| {
        log.append(record(1, 15), &[1], 15).unwrap();
        let e = log.protect_event(20, 40, 20).unwrap();
        log.append(record(2, 30), &[1], 40).unwrap();
        log.flush().unwrap();
        let r = log.event_report(e.event_id).unwrap();
        assert_eq!(r.actual, Some((15, 30)));
        assert!(!r.post_pending);
        assert!(log.protected_bytes() > 0);
        assert_eq!(log.expire_events(40).unwrap(), 1);
        assert_eq!(log.protected_bytes(), 0);
    });
}
#[test]
fn reserve_age_and_pin_byte_limits_are_enforced_before_new_writes() {
    with_log(
        retention(RetentionMode::Continuous),
        writer(),
        |mut log, state| {
            state.borrow_mut().free = 100_099;
            assert!(matches!(
                log.append(record(1, 1), &[1], 1),
                Err(RetentionError::Capacity)
            ));
            state.borrow_mut().free = 100_100;
            log.append(record(1, 1), &[1], 1).unwrap();
            assert!(matches!(
                log.append(record(2, 2), &[1], 1000),
                Err(RetentionError::Capacity)
            ));
        },
    );
    let mut cfg = retention(RetentionMode::Circular);
    cfg.max_age_ns = 10;
    with_log(cfg, writer(), |mut log, state| {
        log.append(record(1, 1), &[1], 1).unwrap();
        log.rotate(2).unwrap();
        let pin = log.pin(0, 4, 2, 20).unwrap();
        log.consume_pin(&pin, 4, 2).unwrap();
        assert!(matches!(
            log.consume_pin(&pin, 1, 2),
            Err(RetentionError::PinLimit)
        ));
        log.append(record(2, 20), &[1], 20).unwrap();
        assert!(!state.borrow().ops.contains(&"delete:0".into()));
        log.release_pin(pin).unwrap();
        log.append(record(3, 21), &[1], 21).unwrap();
        assert!(state.borrow().ops.contains(&"delete:0".into()));
    });
}
#[test]
fn invalid_input_cross_segment_duplicates_and_drop_do_not_write_or_publish() {
    with_log(
        retention(RetentionMode::Continuous),
        writer(),
        |mut log, state| {
            log.append(record(1, 1), &[1], 1).unwrap();
            log.rotate(2).unwrap();
            let before = state.borrow().ops.clone();
            assert!(matches!(
                log.append(record(1, 1), &[1], 2),
                Err(RetentionError::InvalidRecord)
            ));
            let mut bad = record(2, 2);
            bad.metadata.real_ns = -1;
            assert!(log.append(bad, &[1], 2).is_err());
            assert_eq!(state.borrow().ops, before);
            drop(log);
            assert_eq!(state.borrow().ops, before);
        },
    );
}
#[test]
fn deletion_failure_stops_writes_and_unsupported_directory_sync_is_rejected() {
    let mut cfg = retention(RetentionMode::Circular);
    cfg.max_segments = 2;
    with_log(cfg, writer(), |mut log, state| {
        for n in 1..=4 {
            log.append(record(n, n), &[1], n).unwrap();
        }
        state.borrow_mut().fail = Some("delete");
        assert!(log.append(record(5, 5), &[1], 5).is_err());
        assert!(matches!(
            log.append(record(5, 5), &[1], 5),
            Err(RetentionError::Poisoned)
        ));
        assert_eq!(log.segments().len(), 1);
    });
    struct Unsupported(Store);
    impl SegmentStore for Unsupported {
        type Sink = Sink;
        fn directory_sync_supported(&self) -> bool {
            false
        }
        fn available_bytes(&mut self) -> io::Result<u64> {
            panic!("must reject before IO")
        }
        fn create(&mut self, _: u64) -> io::Result<Sink> {
            panic!("must reject before IO")
        }
        fn finalize(&mut self, _: u64) -> io::Result<()> {
            panic!("must reject before IO")
        }
        fn sync_directory(&mut self) -> io::Result<()> {
            panic!("must reject before IO")
        }
        fn remove(&mut self, _: u64) -> io::Result<()> {
            panic!("must reject before IO")
        }
    }
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let store = Unsupported(Store(Rc::new(RefCell::new(State::default()))));
        let _ = &store.0;
        assert!(matches!(
            SegmentLog::new(
                router.writer_budget(10).unwrap(),
                writer(),
                channels(),
                cfg,
                store,
                0
            ),
            Err(RetentionError::UnsupportedDirectorySync)
        ));
    });
}

#[test]
fn multi_segment_control_sync_preserves_rejected_work_and_reports_directory_failure() {
    use dot_logger::completion::{Boundary, ProducerRange, QueryOutcome};
    use dot_logger::control::{ControlConfig, ControlOwner};
    use std::time::{Duration, Instant};
    for fail in [false, true] {
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
            let now = Instant::now();
            let completion = scope.completion().unwrap();
            let control = ControlOwner::new(
                &plan,
                scope.control(),
                ControlConfig {
                    sync_slots: 1,
                    work_per_poll: 32,
                },
                now,
            )
            .unwrap();
            let (mut producer, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![a, b], route).unwrap();
            let state = Rc::new(RefCell::new(State {
                free: u64::MAX,
                ..State::default()
            }));
            let mut limits = writer();
            limits.max_records = 1;
            let mut log = SegmentLog::new(
                router.writer_budget(10).unwrap(),
                limits,
                channels(),
                retention(RetentionMode::Continuous),
                Store(Rc::clone(&state)),
                0,
            )
            .unwrap();
            let wrong_budget = router.writer_budget(20).unwrap();
            router.scope(|mut dispatch, mut outputs| {
                let receipts: Vec<_> = (0..3)
                    .map(|_| {
                        support::accepted(producer.try_record_small(support::meta(), &[1]).unwrap())
                    })
                    .collect();
                dispatch.turn(now, &mut [None; 8]).unwrap();
                dispatch.turn(now, &mut [None; 8]).unwrap();
                let deliveries: Vec<_> = (0..3)
                    .map(|_| outputs[0].try_next(now).unwrap().unwrap())
                    .collect();
                log.write_batch(&deliveries.iter().collect::<Vec<_>>(), 1)
                    .unwrap();
                drop(deliveries);
                assert_eq!(log.segments().len(), 2);
                for r in &receipts {
                    assert_eq!(
                        completion.poll(*r, 10, Boundary::Synced).unwrap().outcome,
                        QueryOutcome::Pending
                    );
                }
                let mut ranges = [ProducerRange::default(); 2];
                let frozen = completion.capture_scope(&mut ranges).unwrap();
                let token = control
                    .request_sync(frozen, &[10], now, now + Duration::from_secs(1))
                    .unwrap();
                let work = control
                    .next_sync(token, 10, now, &mut dispatch, &mut ranges)
                    .unwrap()
                    .unwrap();
                let wrong_state = Rc::new(RefCell::new(State {
                    free: u64::MAX,
                    ..State::default()
                }));
                let mut wrong = SegmentLog::new(
                    wrong_budget,
                    limits,
                    channels(),
                    retention(RetentionMode::Continuous),
                    Store(wrong_state),
                    0,
                )
                .unwrap();
                let rejected = wrong.sync_work(work).unwrap_err();
                assert!(matches!(rejected.error, StorageError::InvalidRecord));
                let work = rejected.work.unwrap();
                drop(wrong);
                if fail {
                    state.borrow_mut().fail = Some("dir");
                }
                let result = log.sync_work(work);
                if fail {
                    assert!(result.unwrap_err().work.is_none());
                } else {
                    result.unwrap();
                }
                control.poll(now).unwrap();
                for r in &receipts {
                    assert_eq!(
                        completion.poll(*r, 10, Boundary::Written).unwrap().outcome,
                        QueryOutcome::Complete
                    );
                    assert_eq!(
                        completion.poll(*r, 10, Boundary::Synced).unwrap().outcome,
                        if fail {
                            QueryOutcome::Failed(dot_logger::completion::FailureReason::SyncFailure)
                        } else {
                            QueryOutcome::Complete
                        }
                    );
                }
                control.release_sync(token).unwrap();
                while let Some(result) = outputs[1].try_next(now) {
                    let delivery = result.unwrap();
                    delivery
                        .report(dot_logger::completion::Update::Confirmed(Boundary::Written))
                        .unwrap();
                }
                use dot_logger::control::{ShutdownPlan, ShutdownPoll};
                let mut shutdown = control
                    .begin_shutdown(
                        ShutdownPlan { sync_files: false },
                        now,
                        now + Duration::from_secs(1),
                        &mut dispatch,
                    )
                    .unwrap();
                for _ in 0..16 {
                    if let ShutdownPoll::Closed(report) = shutdown {
                        // Shutdown asks only for Written; the failed optional sync
                        // boundary remains observable in the Synced queries above.
                        assert!(report.is_success());
                        return;
                    }
                    dispatch.turn(now, &mut [None; 8]).unwrap();
                    shutdown = control.poll_shutdown(now, &mut dispatch).unwrap();
                }
                panic!("shutdown did not settle");
            });
        });
    }
}

/// REV-052 RV-128: a log dropped with written but unsynced records leaves
/// them owed; a later log of the same target cannot take a barrier over them
/// once their Written details expired, unless the dropped log synced first.
#[test]
fn a_dropped_unsynced_log_keeps_its_records_owed_to_a_later_log() {
    use dot_logger::completion::{Boundary, ProducerRange, QueryOutcome, Update};
    use dot_logger::control::{ControlConfig, ControlOwner};
    use std::time::{Duration, Instant};
    for synced in [false, true] {
        let plan = support::plan();
        let route = support::routing();
        let mut tracking = support::tracking(4);
        tracking.history_retention = Duration::from_millis(1);
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
            let now = Instant::now();
            let completion = scope.completion().unwrap();
            let control = ControlOwner::new(
                &plan,
                scope.control(),
                ControlConfig {
                    sync_slots: 1,
                    work_per_poll: 32,
                },
                now,
            )
            .unwrap();
            let (mut producer, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![a, b], route).unwrap();
            let budget = router.writer_budget(10).unwrap();
            let log = |budget| {
                let state = Rc::new(RefCell::new(State {
                    free: u64::MAX,
                    ..State::default()
                }));
                SegmentLog::new(
                    budget,
                    writer(),
                    channels(),
                    retention(RetentionMode::Continuous),
                    Store(state),
                    0,
                )
                .unwrap()
            };
            let mut first = log(budget.clone());
            router.scope(|mut dispatch, mut outputs| {
                let barrier =
                    |target: &mut SegmentLog<Store>,
                     dispatch: &mut dot_logger::routing::Dispatcher<'_, '_>| {
                        let mut ranges = [ProducerRange::default(); 2];
                        let frozen = completion.capture_scope(&mut ranges).unwrap();
                        let token = control
                            .request_sync(frozen, &[10], now, now + Duration::from_secs(5))
                            .unwrap();
                        let work = control
                            .next_sync(token, 10, now, dispatch, &mut ranges)
                            .unwrap()
                            .unwrap();
                        let result = target.sync_work(work).map_err(|e| e.error);
                        control.poll(now).unwrap();
                        control.release_sync(token).unwrap();
                        result
                    };
                let receipt =
                    support::accepted(producer.try_record_small(support::meta(), &[1]).unwrap());
                dispatch.turn(now, &mut [None; 8]).unwrap();
                let delivery = outputs[0].try_next(now).unwrap().unwrap();
                first.write_batch(&[&delivery], 1).unwrap();
                drop(delivery);
                let optional = outputs[1].try_next(now).unwrap().unwrap();
                optional
                    .report(Update::Confirmed(Boundary::Written))
                    .unwrap();
                drop(optional);
                if synced {
                    barrier(&mut first, &mut dispatch).unwrap();
                }
                drop(first);
                std::thread::sleep(Duration::from_millis(10));
                assert_eq!(
                    completion
                        .poll(receipt, 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Expired
                );
                let mut second = log(budget);
                let result = barrier(&mut second, &mut dispatch);
                if synced {
                    result.unwrap();
                } else {
                    assert!(matches!(result, Err(StorageError::InvalidRecord)));
                }
            });
        });
    }
}

#[cfg(all(unix, not(miri)))]
#[cfg(unix)]
#[test]
fn filesystem_store_rotates_into_independently_readable_files_and_refuses_reuse() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let root = Temp(std::env::temp_dir().join(format!(
        "dot-logger-retention-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir(&root.0).unwrap();
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let budget = router.writer_budget(10).unwrap();
        // Deterministic space observation fixture; this is not an OS-capacity test.
        let store = FileStore::new(&root.0, &budget, || Ok(1_000_000)).unwrap();
        let directory = store.directory().to_owned();
        assert!(FileStore::new(&root.0, &budget, || Ok(1_000_000)).is_err());
        let mut log = SegmentLog::new(
            budget.clone(),
            writer(),
            channels(),
            retention(RetentionMode::Circular),
            store,
            0,
        )
        .unwrap();
        for n in 1..=7 {
            log.append(record(n, 8 - n), &[n as u8], n).unwrap();
        }
        log.finish(8).unwrap();
        let mut sequences = Vec::new();
        for segment in log.segments() {
            let bytes =
                std::fs::read(directory.join(format!("{}.mcap", segment.sequence))).unwrap();
            assert_eq!(bytes.len() as u64, segment.bytes);
            assert!(
                !directory
                    .join(format!("{}.active", segment.sequence))
                    .exists()
            );
            let summary = mcap::Summary::read(&bytes).unwrap().unwrap();
            assert!(!summary.chunk_indexes.is_empty());
            for message in NativeReader::new(&bytes, 16, 4096).unwrap() {
                sequences.push(message.unwrap().record.receipt.accepted_seq);
            }
        }
        assert_eq!(sequences, vec![1, 2, 3, 4, 5, 6, 7]);
        assert!(budget.used_bytes() > 0);
        drop(log);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn deletion_directory_failure_removes_unlinked_files_from_the_live_catalog() {
    let mut cfg = retention(RetentionMode::Circular);
    cfg.max_segments = 2;
    with_log(cfg, writer(), |mut log, state| {
        for n in 1..=4 {
            log.append(record(n, n), &[1], n).unwrap();
        }
        state.borrow_mut().fail = Some("dir");
        assert!(log.append(record(5, 5), &[1], 5).is_err());
        assert!(!state.borrow().files.contains_key(&0));
        assert!(!log.segments().iter().any(|s| s.sequence == 0));
        assert!(matches!(
            log.pin(0, 1, 5, 6),
            Err(RetentionError::UnknownToken)
        ));
    });
}

#[test]
fn index_byte_budget_rotates_before_the_message_count_or_chunk_count_limit() {
    let mut cfg = retention(RetentionMode::Continuous);
    cfg.max_index_bytes = 1152;
    let mut limits = writer();
    limits.max_records = 64;
    limits.max_chunks = 64;
    limits.chunk_target_bytes = 1024 * 1024;
    with_log(cfg, limits, |mut log, _| {
        log.append(record(1, 1), &[1], 1).unwrap();
        let second = log.append(record(2, 2), &[2], 2).unwrap();
        assert_eq!(second.rotated, Some(RotationReason::Index));
        assert_eq!(log.segments()[0].records, 1);
    });
}
#[test]
fn disk_byte_quota_stops_continuous_and_reclaims_only_closed_circular_files() {
    for mode in [RetentionMode::Continuous, RetentionMode::Circular] {
        let mut cfg = retention(mode);
        cfg.disk_bytes = 130_000;
        cfg.max_segments = 64;
        cfg.max_age_ns = 10_000;
        with_log(cfg, writer(), |mut log, state| {
            let mut stopped = false;
            for n in 1..=40 {
                if let Err(error) = log.append(record(n, n), &[1], n) {
                    assert!(matches!(error, RetentionError::Capacity));
                    stopped = true;
                    break;
                }
                assert!(
                    log.segments().iter().map(|s| s.bytes).sum::<u64>() + cfg.max_segment_bytes
                        <= cfg.disk_bytes
                );
            }
            assert_eq!(stopped, mode == RetentionMode::Continuous);
            assert_eq!(
                state.borrow().ops.iter().any(|s| s.starts_with("delete")),
                mode == RetentionMode::Circular
            );
        });
    }
}
#[test]
fn batch_capacity_failure_preserves_accepted_prefix_and_retention_cause() {
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
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        let state = Rc::new(RefCell::new(State {
            free: u64::MAX,
            ..State::default()
        }));
        let mut limits = writer();
        limits.max_records = 1;
        let mut cfg = retention(RetentionMode::Continuous);
        cfg.max_segments = 2;
        let mut log = SegmentLog::new(
            router.writer_budget(10).unwrap(),
            limits,
            channels(),
            cfg,
            Store(state),
            0,
        )
        .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = std::time::Instant::now();
            for _ in 0..3 {
                support::accepted(producer.try_record_small(support::meta(), &[1]).unwrap());
            }
            dispatch.turn(now, &mut [None; 8]).unwrap();
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let deliveries: Vec<_> = (0..3)
                .map(|_| outputs[0].try_next(now).unwrap().unwrap())
                .collect();
            let error = log
                .write_batch(&deliveries.iter().collect::<Vec<_>>(), 1)
                .unwrap_err();
            assert!(matches!(error.error, RetentionError::Capacity));
            assert_eq!((error.appended, error.written, error.reported), (2, 1, 1));
            log.flush().unwrap();
            for delivery in &deliveries[..error.appended] {
                delivery
                    .report(dot_logger::completion::Update::Confirmed(
                        dot_logger::completion::Boundary::Written,
                    ))
                    .unwrap();
            }
        });
    });
}

#[test]
fn query_owned_pin_waits_for_reader_close_on_error_cancel_and_expiry() {
    use dot_logger::query::{Query, QueryBudget, QueryConfig, QueryError, Snapshot, TimeBasis};
    use std::cell::Cell;
    use std::io::Read;
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Reader {
        bytes: Cursor<Vec<u8>>,
        log: Rc<RefCell<SegmentLog<Store>>>,
        sequence: u64,
        fail: Rc<Cell<bool>>,
        cancel_on_read: Rc<Cell<bool>>,
        cancel: Rc<AtomicBool>,
        closed: Rc<Cell<bool>>,
    }
    impl Read for Reader {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            if self.fail.get() {
                return Err(io::Error::from_raw_os_error(5));
            }
            let n = self.bytes.read(b)?;
            if self.cancel_on_read.get() {
                self.cancel.store(true, Ordering::Release);
            }
            Ok(n)
        }
    }
    impl Seek for Reader {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            self.bytes.seek(p)
        }
    }
    impl Drop for Reader {
        fn drop(&mut self) {
            // The pin remains occupied while the actual reader is being closed.
            assert!(matches!(
                self.log.borrow_mut().pin(self.sequence, 1, 22, 30),
                Err(RetentionError::PinLimit)
            ));
            self.closed.set(true);
        }
    }
    for case in 0..8 {
        let mut retention = retention(RetentionMode::Circular);
        retention.max_pins = 1;
        with_log(retention, writer(), |mut log, state| {
            log.append(record(1, 1), &[7], 0).unwrap();
            log.rotate(1).unwrap();
            let sequence = log.segments()[0].sequence;
            let bytes = state.borrow().files[&sequence].clone();
            let log = Rc::new(RefCell::new(log));
            let fail = Rc::new(Cell::new(false));
            let cancel_on_read = Rc::new(Cell::new(false));
            let cancel = Rc::new(AtomicBool::new(false));
            let closed = Rc::new(Cell::new(false));
            let reader = Reader {
                bytes: Cursor::new(bytes),
                log: log.clone(),
                sequence,
                fail: fail.clone(),
                cancel_on_read: cancel_on_read.clone(),
                cancel: cancel.clone(),
                closed: closed.clone(),
            };
            let source = log
                .borrow_mut()
                .query_source(reader, sequence, 4096, 2, 20)
                .unwrap();
            let mut c = support::plan().config().clone();
            c.memory.external_bytes = 1024 * 1024;
            c.memory.total_bytes += 1024 * 1024;
            let p = dot_logger::config::SessionBuilder::new(c)
                .validate(&dot_logger::platform::PlatformCapabilities::default())
                .unwrap();
            let budget = QueryBudget::new(&p);
            let mut snapshot = Snapshot::new(
                &budget,
                42,
                channels(),
                vec![source],
                QueryConfig {
                    max_loss_reports: 0,
                    max_segments: 1,
                    max_records: 4,
                    max_record_bytes: 8192,
                    max_source_bytes: 4096,
                    max_payload_bytes: 16,
                    max_metadata_bytes: 128,
                    producers: 2,
                    page_records: 4,
                    page_bytes: 1024,
                    inline_bytes: 16,
                    blob_read_bytes: 16,
                    read_bytes: 8192,
                },
                2,
                100,
                &cancel,
            )
            .unwrap();
            if case >= 4 {
                use dot_logger::replay::{Replay, ReplayError, ReplayPoll, Speed};
                let mut replay = Replay::new(
                    snapshot,
                    Query {
                        basis: TimeBasis::Mono,
                        from: 0,
                        to: 10,
                        channel: None,
                        minimum_level: dot_logger::capture::Level::Trace,
                    },
                    Speed::new(1, 1).unwrap(),
                    3,
                    &cancel,
                )
                .unwrap();
                fail.set(case == 5);
                cancel_on_read.set(case == 6);
                let result = replay.poll(if case == 7 { 20 } else { 4 }, true, &cancel);
                match case {
                    4 => assert!(matches!(result, Ok(ReplayPoll::Ready(_)))),
                    5 => assert!(
                        matches!(result, Err(ReplayError::Query(QueryError::Io(ref e))) if e.raw_os_error()==Some(5))
                    ),
                    6 => assert!(matches!(
                        result,
                        Err(ReplayError::Query(QueryError::Cancelled))
                    )),
                    _ => assert!(matches!(
                        result,
                        Err(ReplayError::Query(QueryError::Expired))
                    )),
                }
                if case == 4 {
                    assert!(!closed.get());
                }
                replay.stop();
                assert!(closed.get());
                assert_eq!(budget.used_bytes(), 0);
                let pin = log.borrow_mut().pin(sequence, 1, 23, 30).unwrap();
                log.borrow_mut().release_pin(pin).unwrap();
                return;
            }
            let cursor = snapshot
                .start(
                    Query {
                        basis: TimeBasis::Mono,
                        from: 0,
                        to: 10,
                        channel: None,
                        minimum_level: dot_logger::capture::Level::Trace,
                    },
                    3,
                    &cancel,
                )
                .unwrap();
            fail.set(case == 1);
            cancel_on_read.set(case == 2);
            let result = snapshot.page(&cursor, if case == 3 { 20 } else { 4 }, &cancel);
            match case {
                0 => assert_eq!(result.unwrap().records.len(), 1),
                1 => assert!(
                    matches!(result,Err(QueryError::Io(ref e)) if e.raw_os_error()==Some(5))
                ),
                2 => assert!(matches!(result, Err(QueryError::Cancelled))),
                _ => assert!(matches!(result, Err(QueryError::Expired))),
            }
            assert!(!closed.get());
            assert!(matches!(
                log.borrow_mut().pin(sequence, 1, 21, 30),
                Err(RetentionError::PinLimit)
            ));
            drop(snapshot);
            assert!(closed.get());
            assert_eq!(budget.used_bytes(), 0);
            let pin = log.borrow_mut().pin(sequence, 1, 23, 30).unwrap();
            log.borrow_mut().release_pin(pin).unwrap();
        });
    }
}

#[test]
fn active_query_pin_survives_rotation_and_is_reclaimed_after_source_drop() {
    let mut config = retention(RetentionMode::Circular);
    config.max_segments = 2;
    config.max_pins = 1;
    with_log(config, writer(), |mut log, _| {
        log.append(record(1, 1), &[1], 0).unwrap();
        log.flush().unwrap();
        let source = log
            .active_query_source(Cursor::new(Vec::<u8>::new()), 4096, 1, 20)
            .unwrap();
        log.rotate(2).unwrap();
        assert!(matches!(log.rotate(21), Err(RetentionError::Capacity)));
        drop(source);
        log.rotate(22).unwrap();
    });
}

#[test]
fn index_budget_below_one_native_pair_is_rejected_before_creating_a_file() {
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let state = Rc::new(RefCell::new(State {
            free: u64::MAX,
            ..State::default()
        }));
        let mut config = retention(RetentionMode::Continuous);
        config.max_index_bytes = 1151; // 128 (two messages) + 512 (chunk) + 512 (channel/chunk).
        let result = SegmentLog::new(
            router.writer_budget(10).unwrap(),
            writer(),
            channels(),
            config,
            Store(state.clone()),
            0,
        );
        assert!(matches!(result, Err(RetentionError::InvalidConfig)));
        assert!(
            state.borrow().ops.is_empty(),
            "invalid native budget created a file"
        );
    });
}
