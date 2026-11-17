//! TASK-030 S2 B: revision rotations and source identities.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::capture::{CaptureOwner, ChannelConfig};
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
        let fail = self.state.borrow().fail;
        if fail == Some("write")
            || (self.seq > 0 && (fail == Some("header") || (fail == Some("pair") && buf == [2])))
        {
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
        if self.0.borrow().fail == Some("create") {
            return Err(io::Error::other("create"));
        }
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
        max_payload_bytes: 16,
        write_buffer_bytes: 64,
        chunk_target_bytes: 1024,
        max_records: 8,
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
#[test]
fn interleaved_deliveries_rotate_without_reordering_or_changing_receipts() {
    run_interleaved(None);
}
#[test]
fn failed_revision_create_header_or_pair_reports_only_the_published_prefix() {
    for fault in ["create", "header", "pair"] {
        run_interleaved(Some(fault));
    }
}
fn run_interleaved(fault: Option<&'static str>) {
    use dot_logger::capture::Level;
    use dot_logger::completion::{Boundary, QueryOutcome};
    use dot_logger::control::config_update::{ChannelPatch, ConfigPatch};
    use std::time::Instant;
    let plan = support::plan();
    let mut routes = support::routing();
    routes.schedule.record_quantum = 1;
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        support::tracking(8),
    )
    .unwrap();
    owner.enable_config_updates(&routes).unwrap();
    owner.scope(|mut scope| {
        let updates = scope.config_updates().unwrap();
        let completion = scope.completion().unwrap();
        let control = dot_logger::control::ControlOwner::new(
            &plan,
            scope.control(),
            dot_logger::control::ControlConfig {
                sync_slots: 1,
                work_per_poll: 32,
            },
            Instant::now(),
        )
        .unwrap();
        let (mut a, ar) = scope.next().unwrap();
        let (mut b, br) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![ar, br], routes).unwrap();
        let state = Rc::new(RefCell::new(State {
            free: u64::MAX,
            ..State::default()
        }));
        let mut limits = writer();
        limits.write_buffer_bytes = 0;
        let mut log = SegmentLog::new(
            router.writer_budget(10).unwrap(),
            limits,
            channels(),
            retention(RetentionMode::Continuous),
            Store(state.clone()),
            0,
        )
        .unwrap();
        let old = [
            support::accepted(a.try_record_small(support::meta(), &[1]).unwrap()),
            support::accepted(a.try_record_small(support::meta(), &[3]).unwrap()),
        ];
        updates
            .apply_config(
                1,
                &ConfigPatch {
                    channels: vec![ChannelPatch {
                        channel_id: 7,
                        minimum_level: Level::Debug,
                        sample_every: 1,
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
        let new = [
            support::accepted(b.try_record_small(support::meta(), &[2]).unwrap()),
            support::accepted(b.try_record_small(support::meta(), &[4]).unwrap()),
        ];
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut deliveries = Vec::new();
            for _ in 0..4 {
                dispatch.turn(now, &mut [None; 16]).unwrap();
                while let Some(item) = outputs[0].try_next(now) {
                    deliveries.push(item.unwrap());
                }
            }
            assert_eq!(
                deliveries
                    .iter()
                    .map(|d| d.record().revision)
                    .collect::<Vec<_>>(),
                [1, 2, 1, 2]
            );
            if let Some(fault) = fault {
                state.borrow_mut().fail = Some(fault);
                let error = log
                    .write_batch(&deliveries.iter().collect::<Vec<_>>(), 1)
                    .unwrap_err();
                assert_eq!(
                    (error.appended, error.written, error.reported),
                    (1, 1, 1),
                    "{fault}"
                );
                assert_eq!(log.segments().len(), 1);
                assert_eq!(
                    completion
                        .poll(old[0], 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
                assert_ne!(
                    completion
                        .poll(new[0], 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
                assert_eq!(
                    NativeReader::new(&state.borrow().files[&0], 16, 2)
                        .unwrap()
                        .count(),
                    1
                );
                return;
            }
            log.write_batch(&deliveries.iter().collect::<Vec<_>>(), 1)
                .unwrap();
            log.finish(2).unwrap();
            assert_eq!(log.segments().len(), 4);
            let mut ranges = [dot_logger::completion::ProducerRange::default(); 2];
            let frozen = completion.capture_scope(&mut ranges).unwrap();
            let token = control
                .request_sync(frozen, &[10], now, now + std::time::Duration::from_secs(1))
                .unwrap();
            let work = control
                .next_sync(token, 10, now, &mut dispatch, &mut ranges)
                .unwrap()
                .unwrap();
            log.sync_work(work).unwrap();
            control.poll(now).unwrap();
            for receipt in old.iter().chain(new.iter()) {
                assert_eq!(
                    completion
                        .poll(*receipt, 10, Boundary::Synced)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
            }

            use dot_logger::query::*;
            let mut config = plan.config().clone();
            config.memory.external_bytes = 1024 * 1024;
            config.memory.total_bytes += 1024 * 1024;
            let query_plan = dot_logger::config::SessionBuilder::new(config)
                .validate(&dot_logger::platform::PlatformCapabilities::default())
                .unwrap();
            let budget = QueryBudget::new(&query_plan);
            let limits = QueryConfig {
                max_segments: 4,
                max_records: 8,
                max_record_bytes: 16384,
                max_source_bytes: 100000,
                max_payload_bytes: 16,
                max_metadata_bytes: 128,
                producers: 2,
                page_records: 8,
                page_bytes: 4096,
                inline_bytes: 16,
                blob_read_bytes: 16,
                read_bytes: 1000000,
            };
            let sources = log
                .segments()
                .iter()
                .map(|segment| {
                    QuerySource::active(
                        std::io::Cursor::new(state.borrow().files[&segment.sequence].clone()),
                        segment.sequence,
                        segment.revision,
                        segment.bytes,
                    )
                })
                .collect();
            let cancel = std::sync::atomic::AtomicBool::new(false);
            let mut snapshot =
                Snapshot::new(&budget, 42, channels(), sources, limits, 2, 100, &cancel).unwrap();
            let cursor = snapshot
                .start(
                    Query {
                        basis: TimeBasis::Mono,
                        from: 0,
                        to: u64::MAX as i128 + 1,
                        channel: None,
                        minimum_level: Level::Trace,
                    },
                    2,
                    &cancel,
                )
                .unwrap();
            let page = snapshot.page(&cursor, 2, &cancel).unwrap();
            assert_eq!(page.records.len(), 4);
            assert_eq!(
                page.records
                    .iter()
                    .map(|r| r.record.revision)
                    .collect::<Vec<_>>(),
                [1, 1, 2, 2]
            );

            for (i, segment) in log.segments().iter().enumerate() {
                assert_eq!(segment.sequence, i as u64);
                let state = state.borrow();
                let bytes = &state.files[&segment.sequence];
                let mut reader = NativeReader::new(bytes, 16, 2).unwrap();
                assert_eq!(reader.info().revision, if i % 2 == 0 { 1 } else { 2 });
                let item = reader.next().unwrap().unwrap();
                let expected = if i % 2 == 0 { old[i / 2] } else { new[i / 2] };
                assert_eq!(item.record.receipt, expected);
                assert_eq!(item.payload, &[(i + 1) as u8]);
                assert!(reader.next().is_none());
                assert_eq!(
                    completion
                        .poll(expected, 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
            }
        });
    });
}

fn with_revision_budgets(
    f: impl FnOnce(SegmentLog<Store>, Rc<RefCell<State>>, WriterBudget, WriterBudget),
) {
    with_revision_config(retention(RetentionMode::Event), f);
}
fn with_revision_config(
    config: RetentionConfig,
    f: impl FnOnce(SegmentLog<Store>, Rc<RefCell<State>>, WriterBudget, WriterBudget),
) {
    use dot_logger::control::config_update::{ChannelPatch, ConfigPatch};
    let plan = support::plan();
    let routes = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        support::tracking(8),
    )
    .unwrap();
    owner.enable_config_updates(&routes).unwrap();
    owner.scope(|scope| {
        let updates = scope.config_updates().unwrap();
        updates
            .apply_config(
                1,
                &ConfigPatch {
                    channels: vec![ChannelPatch {
                        channel_id: 7,
                        minimum_level: dot_logger::capture::Level::Debug,
                        sample_every: 1,
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
        let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), routes).unwrap();
        let old = router.writer_budget_for_revision(10, 1).unwrap();
        let new = router.writer_budget_for_revision(10, 2).unwrap();
        let state = Rc::new(RefCell::new(State {
            free: u64::MAX,
            ..State::default()
        }));
        let log = SegmentLog::new(
            old.clone(),
            writer(),
            channels(),
            config,
            Store(state.clone()),
            0,
        )
        .unwrap();
        f(log, state, old, new);
    });
}
fn record(seq: u64, revision: u64) -> dot_logger::capture::CapturedRecord {
    let mut metadata = support::meta();
    metadata.mono_ns = seq;
    metadata.importance = Some(dot_logger::capture::Importance::Normal);
    dot_logger::capture::CapturedRecord {
        receipt: dot_logger::capture::CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: seq,
        },
        metadata,
        revision,
        payload_len: 1,
    }
}
#[test]
fn explicit_revision_budgets_preserve_event_windows_and_active_pins() {
    with_revision_budgets(|mut log, state, old, new| {
        log.append(record(1, 1), &[1], 1).unwrap();
        log.protect_event(1, 50, 1).unwrap();
        let source = log
            .active_query_source(Cursor::new(Vec::<u8>::new()), 100, 1, 50)
            .unwrap();
        assert_eq!(
            log.append_with_budget(new.clone(), record(2, 2), &[2], 2)
                .unwrap()
                .rotated,
            Some(RotationReason::Revision)
        );
        log.append_with_budget(old, record(3, 1), &[3], 3).unwrap();
        log.append_with_budget(new, record(4, 2), &[4], 4).unwrap();
        assert_eq!(
            log.segments()
                .iter()
                .map(|s| s.revision)
                .collect::<Vec<_>>(),
            [1, 2, 1]
        );
        assert_eq!(log.event_report(1).unwrap().actual, Some((1, 3)));
        assert!(
            !state
                .borrow()
                .ops
                .iter()
                .any(|op| op.starts_with("delete:"))
        );
        drop(source);
        log.finish(5).unwrap();
    });
}
#[test]
fn invalid_revision_or_foreign_budget_is_rejected_without_io() {
    with_revision_budgets(|mut log, state, old, new| {
        log.append(record(1, 1), &[1], 1).unwrap();
        let ops = state.borrow().ops.clone();
        assert!(matches!(
            log.append_with_budget(old, record(2, 2), &[2], 2),
            Err(RetentionError::InvalidRecord)
        ));
        let plan = support::plan();
        let mut owner = CaptureOwner::new(&plan, 42, 2, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        owner.scope(|s| {
            let router =
                Router::new(&plan, s.map(|(_, r)| r).collect(), support::routing()).unwrap();
            assert!(matches!(
                log.append_with_budget(router.writer_budget(10).unwrap(), record(2, 2), &[2], 2),
                Err(RetentionError::InvalidRecord)
            ));
        });
        assert_eq!(state.borrow().ops, ops);
        log.append_with_budget(new.clone(), record(2, 2), &[2], 2)
            .unwrap();
        let ops = state.borrow().ops.clone();
        assert!(matches!(
            log.append_with_budget(new, record(1, 2), &[1], 3),
            Err(RetentionError::InvalidRecord)
        ));
        assert_eq!(state.borrow().ops, ops);
    });
}
#[test]
fn revision_capacity_rejection_keeps_the_previous_active_file_writable() {
    with_revision_budgets(|mut log, state, _old, new| {
        log.append(record(1, 1), &[1], 1).unwrap();
        log.flush().unwrap();
        let boundary = log.active_boundary();
        let ops = state.borrow().ops.clone();
        state.borrow_mut().free = 0;
        assert!(matches!(
            log.append_with_budget(new.clone(), record(2, 2), &[2], 2),
            Err(RetentionError::Capacity)
        ));
        assert_eq!(log.active_boundary(), boundary);
        assert_eq!(state.borrow().ops, ops);
        state.borrow_mut().free = u64::MAX;
        log.append(record(2, 1), &[2], 3).unwrap();
        log.append_with_budget(new, record(3, 2), &[3], 4).unwrap();
        assert_eq!(log.segments()[0].records, 2);
    });
}
#[test]
fn revision_create_failure_preserves_the_published_old_file_and_poisons_the_output() {
    with_revision_budgets(|mut log, state, _old, new| {
        log.append(record(1, 1), &[1], 1).unwrap();
        state.borrow_mut().fail = Some("create");
        assert!(log.append_with_budget(new, record(2, 2), &[2], 2).is_err());
        assert_eq!(log.segments().len(), 1);
        assert_eq!(log.segments()[0].revision, 1);
        assert_eq!(
            NativeReader::new(&state.borrow().files[&0], 16, 2)
                .unwrap()
                .count(),
            1
        );
        let ops = state.borrow().ops.clone();
        assert!(matches!(
            log.append(record(3, 2), &[3], 3),
            Err(RetentionError::Poisoned)
        ));
        drop(log);
        assert_eq!(state.borrow().ops, ops, "Drop must not retry failed IO");
    });
}

#[test]
fn a_receipt_cannot_be_relabelled_as_another_revision_in_a_snapshot() {
    use dot_logger::query::{QueryBudget, QueryConfig, QueryError, QuerySource, Snapshot};
    with_revision_budgets(|log, state, old, new| {
        drop(log);
        let make = |budget: WriterBudget, revision| {
            let sink = Sink {
                seq: 0,
                data: Cursor::new(Vec::new()),
                state: state.clone(),
            };
            let mut writer = SegmentWriter::new(budget, writer(), channels(), sink).unwrap();
            writer.append(record(1, revision), &[1]).unwrap();
            writer.finish().unwrap().into_inner().data.into_inner()
        };
        let first = make(old, 1);
        let second = make(new.clone(), 2);
        let options = RecoveryOptions {
            source_sequence: 0,
            writer: writer(),
            limits: RecoveryConfig {
                max_source_bytes: 100000,
                max_record_bytes: 16384,
                max_chunks: 8,
            },
        };
        recover_to_store(
            &mut Cursor::new(second),
            new,
            channels(),
            options,
            &mut Store(state.clone()),
            1,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        let second = state.borrow().files[&1].clone();
        let mut config = support::plan().config().clone();
        config.memory.external_bytes = 1024 * 1024;
        config.memory.total_bytes += 1024 * 1024;
        let plan = dot_logger::config::SessionBuilder::new(config)
            .validate(&dot_logger::platform::PlatformCapabilities::default())
            .unwrap();
        let budget = QueryBudget::new(&plan);
        let sources = vec![
            QuerySource::active(Cursor::new(first.clone()), 0, 1, first.len() as u64),
            QuerySource::active(Cursor::new(second.clone()), 1, 2, second.len() as u64),
        ];
        let result = Snapshot::new(
            &budget,
            42,
            channels(),
            sources,
            QueryConfig {
                max_segments: 2,
                max_records: 8,
                max_record_bytes: 16384,
                max_source_bytes: 100000,
                max_payload_bytes: 16,
                max_metadata_bytes: 128,
                producers: 2,
                page_records: 8,
                page_bytes: 4096,
                inline_bytes: 16,
                blob_read_bytes: 16,
                read_bytes: 1000000,
            },
            0,
            100,
            &std::sync::atomic::AtomicBool::new(false),
        );
        assert!(matches!(result, Err(QueryError::CorruptData)));
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn revision_rotation_cannot_delete_event_protected_or_pinned_history() {
    for event in [false, true] {
        let mut config = retention(RetentionMode::Event);
        config.max_segments = 2;
        with_revision_config(config, |mut log, state, old, new| {
            log.append(record(1, 1), &[1], 1).unwrap();
            log.flush().unwrap();
            let pin = if event {
                log.protect_event(1, 50, 1).unwrap();
                None
            } else {
                Some(
                    log.active_query_source(Cursor::new(Vec::<u8>::new()), 100, 1, 50)
                        .unwrap(),
                )
            };
            log.append_with_budget(new, record(2, 2), &[2], 2).unwrap();
            let ops = state.borrow().ops.clone();
            assert!(matches!(
                log.append_with_budget(old.clone(), record(3, 1), &[3], 3),
                Err(RetentionError::Capacity)
            ));
            assert_eq!(state.borrow().ops, ops);
            assert_eq!(log.segments()[0].revision, 1);
            if event {
                log.release_event(1).unwrap();
            }
            drop(pin);
            log.append_with_budget(old, record(3, 1), &[3], 4).unwrap();
            assert!(state.borrow().ops.contains(&"delete:0".into()));
            assert_eq!(log.segments()[0].revision, 2);
        });
    }
}
