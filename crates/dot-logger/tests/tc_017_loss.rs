//! TASK-035: loss attempts and assignments remain explicit without telemetry.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    capture::*,
    completion::{Boundary, FailureReason, Update},
    loss::{Cause, Scope},
    routing::Router,
};
#[test]
fn telemetry_off_retries_are_attempts_and_output_reports_count_once_per_assignment() {
    let plan = support::plan();
    let routes = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        support::tracking(16),
    )
    .unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        assert!(scope.telemetry().is_none());
        let loss = scope.loss_tracking().unwrap();
        let completion = scope.completion().unwrap();
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], routes).unwrap();
        for _ in 0..8 {
            support::accepted(producer.try_record_small(support::meta(), &[1]).unwrap());
        }
        for _ in 0..3 {
            assert_eq!(
                producer.try_record_small(support::meta(), &[1]),
                Err(CaptureError::Capacity)
            );
        }
        let count = |scope, cause, target| {
            (0..loss.len())
                .map(|i| loss.get(i).unwrap())
                .filter(|row| {
                    row.scope == scope
                        && row.cause == cause
                        && row.channel == Some(7)
                        && row.target == target
                })
                .map(|row| row.count)
                .sum::<u64>()
        };
        assert_eq!(count(Scope::SubmissionRejected, Cause::Capacity, None), 3);
        router.scope(|mut dispatch, mut outputs| {
            let now = std::time::Instant::now();
            dispatch.turn(now, &mut [None; 16]).unwrap();
            let delivery = outputs[0].try_next(now).unwrap().unwrap();
            let receipt = delivery.record().receipt;
            for _ in 0..2 {
                completion
                    .report(receipt, 10, Update::Failed(FailureReason::IoFailure))
                    .unwrap();
            }
            assert_eq!(count(Scope::OutputFailed, Cause::Io, Some(10)), 1);
            assert_ne!(
                completion
                    .poll(receipt, 10, Boundary::Written)
                    .unwrap()
                    .outcome,
                dot_logger::completion::QueryOutcome::Complete
            );
        });
        support::accepted(producer.try_record_small(support::meta(), &[1]).unwrap());
        assert_eq!(
            count(Scope::SubmissionRejected, Cause::Capacity, None),
            3,
            "later admission does not erase previous rejected attempts"
        );
    });
}

#[derive(Default)]
struct Memory(std::io::Cursor<Vec<u8>>);
impl std::io::Write for Memory {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.0, b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::io::Seek for Memory {
    fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, p)
    }
}
impl dot_logger::storage::SyncSink for Memory {
    fn sync_all(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn storage() -> Vec<dot_logger::storage::StorageChannel> {
    vec![dot_logger::storage::StorageChannel {
        channel: ChannelConfig::new(7, 99, 1),
        topic: "loss/test".into(),
        payload_encoding: "opaque".into(),
        schema: None,
    }]
}
fn wide_storage() -> Vec<dot_logger::storage::StorageChannel> {
    (7..77)
        .map(|id| dot_logger::storage::StorageChannel {
            channel: ChannelConfig::new(id, 99, 1),
            topic: format!("loss/test/{id}"),
            payload_encoding: "opaque".into(),
            schema: None,
        })
        .collect()
}
fn writer_config() -> dot_logger::storage::WriterConfig {
    dot_logger::storage::WriterConfig {
        max_loss_reports: 8,
        max_payload_bytes: 16,
        write_buffer_bytes: 0,
        chunk_target_bytes: 4096,
        max_records: 16,
        max_chunks: 16,
        max_metadata_bytes: 128,
    }
}
#[test]
fn native_baseline_periodic_and_final_parts_preserve_attempt_counts_without_user_records() {
    use dot_logger::storage::{LossReader, NativeReader, SegmentWriter};
    let plan = support::plan();
    let routes = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        support::tracking(16),
    )
    .unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let router = Router::new(&plan, vec![a, b], routes).unwrap();
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            dot_logger::storage::WriterConfig {
                max_records: 0,
                ..writer_config()
            },
            storage(),
            Memory::default(),
        )
        .unwrap();
        for _ in 0..8 {
            support::accepted(p.try_record_small(support::meta(), &[1]).unwrap());
        }
        for _ in 0..3 {
            assert_eq!(
                p.try_record_small(support::meta(), &[1]),
                Err(CaptureError::Capacity)
            );
        }
        writer.checkpoint().unwrap();
        let bytes = writer.finish().unwrap().into_inner().0.into_inner();
        let summary = mcap::Summary::read(&bytes).unwrap().unwrap();
        assert!(!summary.chunk_indexes.is_empty());
        assert!(
            summary
                .chunk_indexes
                .iter()
                .all(|chunk| !chunk.message_index_offsets.is_empty()),
            "registration-only chunks must not contribute a playback time of zero"
        );
        let reader = NativeReader::new(&bytes, 16, 2).unwrap();
        assert!(reader.info().loss_rows > 0);
        assert_eq!(
            reader.count(),
            0,
            "control messages must not consume user record identities"
        );
        let reports = LossReader::new(&bytes, 16, 2)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        // Each checkpoint is one version-2 part over every registered row, so
        // an omitted row was read as zero (ADR-013 2026-11-21).
        let counts: Vec<_> = reports
            .iter()
            .map(|r| {
                assert_eq!(r.range.map(|g| g.end == g.rows), Some(true));
                let count = r.counts.iter().find(|c| {
                    c.scope == Scope::SubmissionRejected
                        && c.cause == Cause::Capacity
                        && c.channel == Some(7)
                        && c.producer == Some(0)
                });
                (r.coverage, count.map_or(0, |c| c.count))
            })
            .collect();
        assert_eq!(
            counts,
            [
                (dot_logger::loss::Coverage::Baseline, 0),
                (dot_logger::loss::Coverage::Observed, 3),
                (dot_logger::loss::Coverage::FinalPartial, 3)
            ]
        );
    });
}

fn loss_plan() -> dot_logger::config::ValidatedConfig {
    let mut config = support::plan().config().clone();
    config.memory.output_bytes += 4 << 20;
    config.memory.external_bytes += 4 << 20;
    config.memory.total_bytes += 8 << 20;
    dot_logger::config::SessionBuilder::new(config)
        .validate(&dot_logger::platform::PlatformCapabilities::default())
        .unwrap()
}
fn recorded_files(observers: &[u32], periodic: bool, user: bool) -> Vec<Vec<u8>> {
    recorded_files_with_writer(observers, periodic, user, writer_config())
}
fn recorded_files_with_writer(
    observers: &[u32],
    periodic: bool,
    user: bool,
    config: dot_logger::storage::WriterConfig,
) -> Vec<Vec<u8>> {
    use dot_logger::storage::SegmentWriter;
    let plan = loss_plan();
    let routes = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        support::tracking(16),
    )
    .unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let router = Router::new(&plan, vec![a, b], routes).unwrap();
        let mut first = None;
        let mut files = Vec::new();
        for (index, target) in observers.iter().enumerate() {
            let mut writer = SegmentWriter::new(
                router.writer_budget(*target).unwrap(),
                config,
                storage(),
                Memory::default(),
            )
            .unwrap();
            if index == 0 {
                for _ in 0..8 {
                    let receipt =
                        support::accepted(p.try_record_small(support::meta(), &[1]).unwrap());
                    first.get_or_insert(receipt);
                }
                for _ in 0..3 {
                    assert_eq!(
                        p.try_record_small(support::meta(), &[1]),
                        Err(CaptureError::Capacity)
                    );
                }
            }
            if user {
                let mut metadata = support::meta();
                metadata.importance = Some(Importance::Normal);
                writer
                    .append(
                        CapturedRecord {
                            receipt: first.unwrap(),
                            metadata,
                            revision: 1,
                            payload_len: 1,
                        },
                        &[1],
                    )
                    .unwrap();
            }
            if periodic {
                writer.checkpoint().unwrap();
            }
            files.push(writer.finish().unwrap().into_inner().0.into_inner());
        }
        files
    })
}
/// One observer-10 file whose final checkpoint needs two version-2 parts:
/// producer 0 is rejected on 70 channels, more than ROWS_PER_REPORT rows.
fn wide_routing() -> dot_logger::routing::RoutingConfig {
    let mut routes = support::routing();
    let route = routes.routes[0].clone();
    routes.routes = (7..77)
        .map(|channel_id| dot_logger::routing::Route {
            channel_id,
            ..route.clone()
        })
        .collect();
    routes
}
fn wide_recorded_file(config: dot_logger::storage::WriterConfig) -> Vec<u8> {
    use dot_logger::storage::SegmentWriter;
    let plan = loss_plan();
    let routes = wide_routing();
    let channels: Vec<_> = (7..77).map(|id| ChannelConfig::new(id, 99, 1)).collect();
    let mut owner =
        CaptureOwner::new_tracked(&plan, 42, 1, &channels, &routes, support::tracking(16)).unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let router = Router::new(&plan, vec![a, b], routes).unwrap();
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config,
            wide_storage(),
            Memory::default(),
        )
        .unwrap();
        let mut first = None;
        for _ in 0..8 {
            let receipt = support::accepted(p.try_record_small(support::meta(), &[1]).unwrap());
            first.get_or_insert(receipt);
        }
        for channel in 7..77 {
            let mut metadata = support::meta();
            metadata.channel_id = channel;
            for _ in 0..if channel == 7 { 3 } else { 1 } {
                assert_eq!(
                    p.try_record_small(metadata, &[1]),
                    Err(CaptureError::Capacity)
                );
            }
        }
        let mut metadata = support::meta();
        metadata.importance = Some(Importance::Normal);
        writer
            .append(
                CapturedRecord {
                    receipt: first.unwrap(),
                    metadata,
                    revision: 1,
                    payload_len: 1,
                },
                &[1],
            )
            .unwrap();
        writer.finish().unwrap().into_inner().0.into_inner()
    })
}
fn wide_writer() -> dot_logger::storage::WriterConfig {
    dot_logger::storage::WriterConfig {
        max_loss_reports: 160,
        max_chunks: 160,
        max_metadata_bytes: 16384,
        ..writer_config()
    }
}
fn query_config() -> dot_logger::query::QueryConfig {
    dot_logger::query::QueryConfig {
        max_loss_reports: 32,
        max_segments: 4,
        max_records: 1,
        max_record_bytes: 65536,
        max_source_bytes: 1000000,
        max_payload_bytes: 16,
        max_metadata_bytes: 128,
        producers: 2,
        page_records: 1,
        page_bytes: 4096,
        inline_bytes: 16,
        blob_read_bytes: 16,
        read_bytes: 1000000,
    }
}
#[test]
fn loss_only_snapshot_compares_replicas_and_validates_target_identity_without_replaying_fanout() {
    use dot_logger::query::{LossSnapshot, QueryBudget, QueryError, QuerySource, Snapshot};
    use std::{io::Cursor, sync::atomic::AtomicBool};
    let files = recorded_files(&[10, 20], true, true);
    let sources = || {
        files
            .iter()
            .zip([10, 20])
            .map(|(bytes, target)| {
                QuerySource::active(Cursor::new(bytes.clone()), 0, 1, bytes.len() as u64)
                    .expect_target(target)
            })
            .collect()
    };
    let budget = QueryBudget::new(&loss_plan());
    let no = AtomicBool::new(false);
    let snapshot = LossSnapshot::new(
        &budget,
        42,
        storage(),
        sources(),
        query_config(),
        0,
        100,
        &no,
    )
    .unwrap();
    let deltas = snapshot.deltas(1, &no).unwrap();
    let row = deltas
        .iter()
        .find(|r| {
            r.counter.producer == Some(0)
                && r.counter.channel == Some(7)
                && r.counter.scope == Scope::SubmissionRejected
                && r.counter.cause == Cause::Capacity
        })
        .unwrap();
    assert_eq!(row.counter.count, 3);
    assert_eq!(row.delta, Some(3), "replicas must not be added");
    assert!(!row.tail_confirmed);
    assert_eq!(snapshot.unavailable_sources(), 0);
    drop(snapshot);
    assert_eq!(budget.used_bytes(), 0);
    let mut data_limits = query_config();
    data_limits.max_records = 2;
    assert!(
        matches!(
            Snapshot::new(&budget, 42, storage(), sources(), data_limits, 0, 100, &no),
            Err(QueryError::CorruptData)
        ),
        "user receipts remain unique across indexed outputs"
    );
    let wrong = vec![
        QuerySource::active(Cursor::new(files[0].clone()), 0, 1, files[0].len() as u64)
            .expect_target(20),
    ];
    assert!(matches!(
        LossSnapshot::new(&budget, 42, storage(), wrong, query_config(), 0, 100, &no),
        Err(QueryError::CorruptData)
    ));
    assert_eq!(budget.used_bytes(), 0);
}
#[test]
fn recovery_preserves_original_reports_and_incomplete_parts_never_make_an_exact_delta() {
    use dot_logger::storage::*;
    use std::{io::Cursor, sync::atomic::AtomicBool};
    let source = wide_recorded_file(wide_writer());
    let original = LossReader::new(&source, 16, 2)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // An empty baseline part, then a final checkpoint of 64 + 6 rows.
    assert_eq!(
        original.iter().map(|r| r.counts.len()).collect::<Vec<_>>(),
        [0, 64, 6]
    );
    let plan = loss_plan();
    let channels: Vec<_> = (7..77).map(|id| ChannelConfig::new(id, 99, 1)).collect();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &channels).unwrap();
    owner.scope(|scope| {
        let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), wide_routing()).unwrap();
        for limit in [8, 2] {
            let mut writer = WriterConfig {
                max_metadata_bytes: 16384,
                ..writer_config()
            };
            writer.max_loss_reports = limit;
            let options = RecoveryOptions {
                source_target: Some(10),
                source_sequence: 0,
                writer,
                limits: RecoveryConfig {
                    max_source_bytes: 1000000,
                    max_record_bytes: 65536,
                    max_chunks: 32,
                },
            };
            let recovered = recover_segment(
                &mut Cursor::new(&source),
                router.writer_budget(10).unwrap(),
                wide_storage(),
                options,
                Memory::default(),
                &AtomicBool::new(false),
            )
            .unwrap();
            let stop = recovered.report.stop;
            let count = recovered.report.loss_reports;
            let bytes = recovered.segment.into_inner().0.into_inner();
            let reports = LossReader::new(&bytes, 16, 2)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            if limit == 8 {
                assert_eq!(stop, RecoveryStop::DataEnd);
                assert_eq!(reports, original);
                assert_eq!(count, 3);
                assert_eq!(NativeReader::new(&bytes, 16, 2).unwrap().count(), 1);
                let deltas = dot_logger::loss::summarize(&reports, 512).unwrap();
                assert_eq!(deltas.len(), 70);
                assert!(deltas.iter().all(|d| d.delta == Some(d.counter.count)));
            } else {
                assert_eq!(stop, RecoveryStop::Limit);
                assert_eq!(count, 2);
                let summary = dot_logger::loss::summarize(&reports, 512).unwrap();
                assert!(summary.iter().all(|row| row.delta.is_none()));
            }
        }
    });
}

#[test]
fn recovery_keeps_grouped_loss_chunks_whole_at_report_and_chunk_limits() {
    use dot_logger::storage::*;
    use std::{io::Cursor, sync::atomic::AtomicBool};
    let grouped = WriterConfig {
        chunk_target_bytes: 8192,
        max_metadata_bytes: 16384,
        ..writer_config()
    };
    let source = wide_recorded_file(WriterConfig {
        max_loss_reports: 160,
        max_chunks: 60,
        ..grouped
    });
    let original = LossReader::new(&source, 16, 2)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // The final checkpoint's two parts share one grouped chunk.
    assert_eq!(original.len(), 3);
    let plan = loss_plan();
    let channels: Vec<_> = (7..77).map(|id| ChannelConfig::new(id, 99, 1)).collect();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &channels).unwrap();
    owner.scope(|scope| {
        let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), wide_routing()).unwrap();
        for (writer, expected) in [
            (
                WriterConfig {
                    max_chunks: 3,
                    ..grouped
                },
                3,
            ),
            (
                WriterConfig {
                    max_loss_reports: 2,
                    ..grouped
                },
                1,
            ),
            (
                WriterConfig {
                    max_chunks: 3,
                    max_metadata_bytes: 16384,
                    ..writer_config()
                },
                1,
            ),
        ] {
            let recovered = recover_segment(
                &mut Cursor::new(&source),
                router.writer_budget(10).unwrap(),
                wide_storage(),
                RecoveryOptions {
                    source_target: Some(10),
                    source_sequence: 0,
                    writer,
                    limits: RecoveryConfig {
                        max_source_bytes: 1_000_000,
                        max_record_bytes: 65_536,
                        max_chunks: 32,
                    },
                },
                Memory::default(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(recovered.report.records, 1);
            assert_eq!(recovered.report.loss_reports, expected);
            let stop = if expected == original.len() {
                RecoveryStop::DataEnd
            } else {
                RecoveryStop::Limit
            };
            assert_eq!(recovered.report.stop, stop);
            let bytes = recovered.segment.into_inner().0.into_inner();
            let reports = LossReader::new(&bytes, 16, 2)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(reports, &original[..expected]);
        }
    });
}
#[test]
fn policy_raw_acquisition_and_empty_routing_remain_distinct_from_capacity_losses() {
    use dot_logger::pool::RawRecordOutcome;
    let plan = support::plan();
    let mut routes = support::routing();
    routes.routes[0].targets.clear();
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Warn;
    let mut owner =
        CaptureOwner::new_tracked(&plan, 42, 1, &[channel], &routes, support::tracking(16))
            .unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        let counters = scope.loss_tracking().unwrap();
        let (mut p, _) = scope.next().unwrap();
        assert_eq!(
            p.try_record_small(support::meta(), &[1]),
            Ok(RecordOutcome::Skipped)
        );
        let a = p.try_acquire(1, Importance::Normal).unwrap();
        let b = p.try_acquire(1, Importance::Normal).unwrap();
        assert!(matches!(
            p.try_acquire(1, Importance::Normal),
            Err(CaptureError::Capacity)
        ));
        let id = a.identity();
        let RawRecordOutcome::Skipped(a) = p.try_record_raw(support::meta(), a).unwrap() else {
            panic!("filtered raw")
        };
        assert_eq!(a.identity(), id);
        let mut metadata = support::meta();
        metadata.level = Level::Warn;
        support::accepted(p.try_record_small(metadata, &[1]).unwrap());
        let rows: Vec<_> = (0..counters.len())
            .map(|i| counters.get(i).unwrap())
            .filter(|r| r.count > 0)
            .collect();
        assert!(rows.iter().any(|r| r.scope == Scope::IntentionalSkip
            && r.cause == Cause::Policy
            && r.count == 2));
        assert!(rows.iter().any(|r| r.scope == Scope::IntentionalSkip
            && r.cause == Cause::NoRoute
            && r.count == 1));
        assert!(rows.iter().any(|r| r.scope == Scope::AcquireRejected
            && r.cause == Cause::Capacity
            && r.channel.is_none()
            && r.count == 1));
        assert!(!rows.iter().any(|r| r.scope == Scope::SubmissionRejected));
        drop((a, b));
    });
}
#[test]
fn concurrent_output_reports_do_not_lose_shared_counter_updates_or_count_repeats() {
    let plan = support::plan();
    let routes = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        support::tracking(512),
    )
    .unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let counters = scope.loss_tracking().unwrap();
        let (mut p, mut read) = scope.next().unwrap();
        let receipts: Vec<_> = (0..256)
            .map(|_| {
                let r = support::accepted(p.try_record_small(support::meta(), &[1]).unwrap());
                read.try_read(&mut [0; 16]).unwrap();
                r
            })
            .collect();
        let barrier = std::sync::Barrier::new(4);
        std::thread::scope(|threads| {
            for thread in 0..4 {
                let receipts = &receipts;
                let barrier = &barrier;
                threads.spawn(move || {
                    barrier.wait();
                    for r in receipts.iter().skip(thread).step_by(4) {
                        for _ in 0..2 {
                            completion
                                .report(*r, 10, Update::Failed(FailureReason::IoFailure))
                                .unwrap();
                        }
                    }
                });
            }
        });
        let row = (0..counters.len())
            .map(|i| counters.get(i).unwrap())
            .find(|r| {
                r.scope == Scope::OutputFailed
                    && r.cause == Cause::Io
                    && r.producer == Some(0)
                    && r.channel == Some(7)
                    && r.target == Some(10)
            })
            .unwrap();
        assert_eq!(row.count, 256);
    });
}

#[test]
fn abandoned_assignments_are_unknown_once_and_telemetry_reset_preserves_loss_epoch() {
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
    owner.enable_telemetry().unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let loss = scope.loss_tracking().unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], routes).unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let receipt = support::accepted(p.try_record_small(support::meta(), &[1]).unwrap());
            let mut invalid = support::meta();
            invalid.schema_version = 2;
            assert_eq!(
                p.try_record_small(invalid, &[1]),
                Err(CaptureError::SchemaMismatch)
            );
            let now = std::time::Instant::now();
            dispatch.turn(now, &mut [None; 16]).unwrap();
            let unknown = outputs[0].try_next(now).unwrap().unwrap();
            let known = outputs[1].try_next(now).unwrap().unwrap();
            known.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(known);
            drop(unknown);
            completion.reconcile(32);
            completion.reconcile(32);
            completion.report(receipt, 10, Update::Unknown).unwrap();
            completion.report(receipt, 10, Update::Unknown).unwrap();
            assert_eq!(
                (0..loss.len())
                    .map(|i| loss.get(i).unwrap())
                    .find(|r| r.scope == Scope::OutputUnknown
                        && r.target == Some(10)
                        && r.producer == Some(0))
                    .unwrap()
                    .count,
                1
            );
        });
    });
    owner.reset_telemetry().unwrap();
    let loss = owner.loss_tracking().unwrap();
    assert_eq!(
        (0..loss.len())
            .map(|i| loss.get(i).unwrap())
            .find(|r| r.scope == Scope::OutputUnknown
                && r.target == Some(10)
                && r.producer == Some(0))
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        (0..loss.len())
            .map(|i| loss.get(i).unwrap())
            .find(|r| r.scope == Scope::SubmissionRejected
                && r.cause == Cause::SchemaMismatch
                && r.channel == Some(7)
                && r.producer == Some(0))
            .unwrap()
            .count,
        1
    );
    let dot_logger::telemetry::Observation::Observed(counts) =
        owner.telemetry().unwrap().producer(0)
    else {
        panic!("telemetry")
    };
    assert_eq!(counts.accepted, 0);
}
#[test]
fn loss_control_reservation_checks_the_exact_control_budget_before_enabling() {
    let plan = support::plan();
    let routes = support::routing();
    let build = |p: &dot_logger::config::ValidatedConfig| {
        CaptureOwner::new_tracked(
            p,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &routes,
            support::tracking(8),
        )
        .unwrap()
    };
    let mut measured = build(&plan);
    measured.enable_loss_tracking().unwrap();
    let required = measured.control_reserved_bytes();
    measured.enable_loss_tracking().unwrap();
    assert_eq!(measured.control_reserved_bytes(), required);
    for available in [required - 1, required] {
        let mut config = plan.config().clone();
        config.memory.control_bytes = available;
        let small = dot_logger::config::SessionBuilder::new(config)
            .validate(&dot_logger::platform::PlatformCapabilities::default())
            .unwrap();
        let mut owner = build(&small);
        if available < required {
            assert!(matches!(
                owner.enable_loss_tracking(),
                Err(CaptureBuildError::Config(
                    dot_logger::config::ConfigError::BudgetExceeded { .. }
                ))
            ));
            assert!(owner.loss_tracking().is_none());
        } else {
            owner.enable_loss_tracking().unwrap();
            assert_eq!(owner.control_reserved_bytes(), required);
        }
    }
}
fn frames(bytes: &[u8], start: usize, end: usize) -> Vec<(u8, usize, usize)> {
    let mut at = start;
    let mut result = Vec::new();
    while at < end {
        let len = u64::from_le_bytes(bytes[at + 1..at + 9].try_into().unwrap()) as usize;
        result.push((bytes[at], at, at + 9 + len));
        at += 9 + len;
    }
    result
}
fn crc(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for b in bytes {
        c ^= u32::from(*b);
        for _ in 0..8 {
            c = (c >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(c & 1));
        }
    }
    !c
}
#[test]
fn a_bad_loss_control_later_in_the_same_chunk_exposes_no_user_pair() {
    use dot_logger::storage::*;
    use std::{io::Cursor, sync::atomic::AtomicBool};
    let bytes = recorded_files(&[10], false, true).remove(0);
    let chunks: Vec<_> = frames(&bytes, 8, bytes.len() - 8)
        .into_iter()
        .filter(|r| r.0 == 6)
        .collect();
    let user = chunks
        .iter()
        .find(|(_, a, b)| {
            frames(&bytes, a + 49, *b).iter().any(|(op, p, _)| {
                *op == 5 && u16::from_le_bytes(bytes[p + 9..p + 11].try_into().unwrap()) == 1
            })
        })
        .copied()
        .unwrap();
    let control = *chunks.last().unwrap();
    let mut data = bytes[user.1 + 49..user.2].to_vec();
    let mut bad = bytes[control.1 + 49..control.2].to_vec();
    let message = frames(&bad, 0, bad.len())
        .into_iter()
        .find(|r| r.0 == 5)
        .unwrap();
    let payload = message.1 + 9 + 22;
    assert_eq!(&bad[payload..payload + 2], &[8, 2], "version-2 report");
    bad[payload + 1] = 99;
    data.extend_from_slice(&bad);
    let mut header = bytes[user.1 + 9..user.1 + 49].to_vec();
    let control_end = u64::from_le_bytes(bytes[control.1 + 17..control.1 + 25].try_into().unwrap());
    header[8..16].copy_from_slice(&control_end.to_le_bytes());
    header[16..24].copy_from_slice(&(data.len() as u64).to_le_bytes());
    header[24..28].copy_from_slice(&crc(&data).to_le_bytes());
    header[32..40].copy_from_slice(&(data.len() as u64).to_le_bytes());
    let mut chunk = vec![6];
    chunk.extend_from_slice(&((40 + data.len()) as u64).to_le_bytes());
    chunk.extend_from_slice(&header);
    chunk.extend_from_slice(&data);
    let mut damaged = bytes.clone();
    damaged.splice(user.1..user.2, chunk);
    let mut reader = NativeReader::new(&damaged, 16, 2).unwrap();
    assert!(reader.next().unwrap().is_err());
    assert!(reader.next().is_none());
    let plan = loss_plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let recovered = recover_segment(
            &mut Cursor::new(&damaged),
            router.writer_budget(10).unwrap(),
            storage(),
            RecoveryOptions {
                source_target: Some(10),
                source_sequence: 0,
                writer: writer_config(),
                limits: RecoveryConfig {
                    max_source_bytes: 1000000,
                    max_record_bytes: 65536,
                    max_chunks: 32,
                },
            },
            Memory::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(recovered.report.stop, RecoveryStop::InvalidRecord);
        assert_eq!(recovered.report.records, 0);
        let output = recovered.segment.into_inner().0.into_inner();
        assert_eq!(NativeReader::new(&output, 16, 2).unwrap().count(), 0);
    });
}

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
impl dot_logger::storage::SyncSink for Sink {
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
impl dot_logger::storage::SegmentStore for Store {
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

#[test]
fn saturated_control_budget_keeps_the_active_file_and_coalesces_its_final_sample() {
    use dot_logger::storage::*;
    // Reservations stay dense (two parts per checkpoint here) while each
    // written version-2 checkpoint uses one part, so five reports admit the
    // baseline and one periodic sample but not a second (ADR-013 2026-11-21).
    for (report_limit, index_limit, periodic) in [(4, 16384, 0), (5, 4096, 1)] {
        let plan = loss_plan();
        let routes = support::routing();
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &routes,
            support::tracking(16),
        )
        .unwrap();
        owner.enable_loss_tracking().unwrap();
        owner.scope(|mut scope| {
            let (mut p, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let router = Router::new(&plan, vec![a, b], routes).unwrap();
            let mut config = writer_config();
            config.max_loss_reports = report_limit;
            for bad in [
                WriterConfig {
                    max_loss_reports: 3,
                    ..config
                },
                WriterConfig {
                    max_chunks: 5,
                    ..config
                },
            ] {
                assert!(
                    SegmentWriter::new(
                        router.writer_budget(10).unwrap(),
                        bad,
                        storage(),
                        Memory::default()
                    )
                    .is_err()
                );
            }
            let state = Rc::new(RefCell::new(State {
                free: 1_000_000,
                ..State::default()
            }));
            let retention = RetentionConfig {
                mode: RetentionMode::Continuous,
                disk_bytes: 1_000_000,
                disk_reserve_bytes: 0,
                max_age_ns: 1000,
                max_segment_bytes: 200_000,
                max_segment_ns: 1000,
                max_index_bytes: index_limit,
                protected_bytes: 300_000,
                pre_ns: 0,
                post_ns: 0,
                max_segments: 1,
                max_events: 1,
                max_pins: 1,
                max_pin_bytes: 4096,
                max_pin_ns: 100,
            };
            let mut log = SegmentLog::new(
                router.writer_budget(10).unwrap(),
                config,
                storage(),
                retention,
                Store(Rc::clone(&state)),
                0,
            )
            .unwrap();
            for _ in 0..8 {
                support::accepted(p.try_record_small(support::meta(), &[1]).unwrap());
            }
            for now in 1..=3 {
                assert_eq!(
                    p.try_record_small(support::meta(), &[1]),
                    Err(CaptureError::Capacity)
                );
                assert_eq!(log.checkpoint(now).is_ok(), now <= periodic);
            }
            log.finish(4).unwrap();
            let state = state.borrow();
            assert_eq!(
                state
                    .ops
                    .iter()
                    .filter(|op| op.starts_with("create:"))
                    .count(),
                1
            );
            let bytes = state.files.get(&0).unwrap();
            let reports = LossReader::new(bytes, 16, 2)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(
                reports
                    .iter()
                    .filter(|r| r.coverage == dot_logger::loss::Coverage::Baseline)
                    .all(|r| r.observed_from_ns > 0)
            );
            // One version-2 part per checkpoint: only one nonzero row exists.
            assert_eq!(reports.len(), 2 + periodic as usize);
            assert!(reports[reports.len() - 1].coalesced);
            let counts = dot_logger::loss::summarize(&reports, 512).unwrap();
            assert!(
                counts
                    .iter()
                    .any(|d| d.counter.scope == Scope::SubmissionRejected
                        && d.counter.cause == Cause::Capacity
                        && d.counter.producer == Some(0)
                        && d.counter.count == 3
                        && d.delta == Some(3))
            );
        });
    }
}

#[test]
fn facade_encode_failures_are_persisted_and_only_returned_writers_confirm_the_tail() {
    use dot_logger::logger::{Budget, EncodeError, Logger, Record, Retention, Schema, WriterQuota};
    use dot_logger::loss::Coverage;
    use std::time::Duration;
    struct Bad;
    impl Record for Bad {
        const SCHEMA: Schema = Schema::new(99, 1, "opaque");
        fn encode(&self, _: &mut [u8]) -> Result<usize, EncodeError> {
            Err(EncodeError)
        }
    }
    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    for outstanding in [false, true] {
        let temp = Temp(std::env::temp_dir().join(format!(
            "dot-loss-tail-{}-{outstanding}",
            std::process::id()
        )));
        std::fs::create_dir(&temp.0).unwrap();
        let logger = Logger::builder()
            .budget(
                Budget::new()
                    .memory_bytes(32 << 20)
                    .sync_every(Duration::from_millis(20))
                    .drop_deadline(Duration::from_secs(3))
                    .writer_quota(WriterQuota::small(8, 16)),
            )
            .file_output(&temp.0, Retention::continuous(64 << 20))
            .channel::<Bad>("bad")
            .writer("p")
            .start()
            .unwrap();
        let channel = logger.channel::<Bad>("bad").unwrap();
        let mut writer = Some(logger.take_writer("p").unwrap());
        assert!(writer.as_mut().unwrap().try_log(channel, &Bad).is_err());
        let directory = logger.directories()[0].clone();
        if !outstanding {
            drop(writer.take());
        }
        logger.shutdown(Duration::from_secs(5)).unwrap();
        let mut reports = Vec::new();
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "mcap") {
                reports.extend(
                    dot_logger::storage::LossReader::new(&std::fs::read(path).unwrap(), 16, 1)
                        .unwrap()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap(),
                );
            }
        }
        assert!(!reports.is_empty());
        assert_eq!(
            reports.iter().any(|r| r.coverage == Coverage::Quiescent),
            !outstanding
        );
        assert!(
            reports
                .iter()
                .flat_map(|r| &r.counts)
                .any(|c| c.cause == Cause::Encode && c.count == 1)
        );
        if outstanding {
            assert_eq!(
                writer.as_mut().unwrap().try_log(channel, &Bad),
                Err(dot_logger::logger::LogError::Closing)
            );
        }
    }
}

#[test]
fn general_path_expired_deadline_is_one_unsubmitted_attempt() {
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
    owner.enable_loss_tracking().unwrap();
    owner.scope(|mut scope| {
        let loss = scope.loss_tracking().unwrap();
        let (p, _) = scope.next().unwrap();
        assert_eq!(
            p.into_general()
                .record_until(support::meta(), &[1], std::time::Instant::now()),
            Err(CaptureError::DeadlineExceeded)
        );
        assert_eq!(
            (0..loss.len())
                .map(|i| loss.get(i).unwrap())
                .filter(|c| c.cause == Cause::Deadline)
                .map(|c| c.count)
                .sum::<u64>(),
            1
        );
    });
}
#[test]
fn control_io_failure_keeps_only_the_prefix_and_a_surviving_output_observes_the_failed_origin() {
    use dot_logger::storage::*;
    struct Observed {
        bytes: Rc<RefCell<Cursor<Vec<u8>>>>,
        failed: Rc<std::cell::Cell<bool>>,
    }
    impl Write for Observed {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.failed.get() {
                Err(io::Error::other("injected"))
            } else {
                self.bytes.borrow_mut().write(b)
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Seek for Observed {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            self.bytes.borrow_mut().seek(p)
        }
    }
    impl SyncSink for Observed {
        fn sync_all(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for all_failed in [false, true] {
        let plan = loss_plan();
        let routes = support::routing();
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &routes,
            support::tracking(16),
        )
        .unwrap();
        owner.enable_loss_tracking().unwrap();
        owner.scope(|mut scope| {
            let completion = scope.completion().unwrap();
            let (mut p, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let router = Router::new(&plan, vec![a, b], routes).unwrap();
            let receipt = support::accepted(p.try_record_small(support::meta(), &[1]).unwrap());
            let mut prefixes = Vec::new();
            for target in [10, 20] {
                let bytes = Rc::new(RefCell::new(Cursor::new(Vec::new())));
                let failed = Rc::new(std::cell::Cell::new(false));
                let mut writer = SegmentWriter::new(
                    router.writer_budget(target).unwrap(),
                    writer_config(),
                    storage(),
                    Observed {
                        bytes: Rc::clone(&bytes),
                        failed: Rc::clone(&failed),
                    },
                )
                .unwrap();
                let safe = writer.safe_offset() as usize;
                if target == 10 || all_failed {
                    completion
                        .report(receipt, target, Update::Failed(FailureReason::IoFailure))
                        .unwrap();
                    failed.set(true);
                    assert!(writer.checkpoint().is_err());
                    assert_eq!(writer.safe_offset() as usize, safe);
                    assert!(writer.checkpoint().is_err());
                    drop(writer);
                    prefixes.push(bytes.borrow().get_ref()[..safe].to_vec());
                } else {
                    writer.checkpoint().unwrap();
                    drop(writer.finish().unwrap());
                    prefixes.push(bytes.borrow().get_ref().clone());
                }
            }
            let reports = prefixes
                .iter()
                .flat_map(|bytes| {
                    LossReader::new(bytes, 16, 2)
                        .unwrap()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert!(
                !reports
                    .iter()
                    .any(|r| r.coverage == dot_logger::loss::Coverage::Quiescent)
            );
            if !all_failed {
                assert!(
                    reports
                        .iter()
                        .flat_map(|r| &r.counts)
                        .any(|c| c.scope == Scope::OutputFailed
                            && c.target == Some(10)
                            && c.count == 1)
                );
            }
            // All failed output files preserve their existing prefix; RAM-only
            // terminal changes after that watermark are explicitly unobserved.
            if all_failed {
                assert!(
                    dot_logger::loss::summarize(&reports, 1024)
                        .unwrap()
                        .iter()
                        .all(|d| d.delta.is_none())
                );
            }
            assert!(
                dot_logger::loss::summarize(&reports, 1024)
                    .unwrap()
                    .iter()
                    .all(|d| !d.tail_confirmed)
            );
        });
    }
}

#[test]
fn loss_report_observer_must_match_the_original_file_target() {
    let mut bytes = recorded_files(&[10], false, false).remove(0);
    let chunks = frames(&bytes, 8, bytes.len() - 8);
    let (a, b, payload) = chunks
        .iter()
        .filter(|r| r.0 == 6)
        .find_map(|(_, a, b)| {
            frames(&bytes, a + 49, *b)
                .iter()
                .find(|(op, p, _)| {
                    *op == 5 && u16::from_le_bytes(bytes[p + 9..p + 11].try_into().unwrap()) == 3
                })
                .map(|(_, p, _)| (*a, *b, p + 31))
        })
        .unwrap();
    assert_eq!(&bytes[payload + 22..payload + 24], &[32, 10]);
    bytes[payload + 23] = 20;
    let sum = crc(&bytes[a + 49..b]);
    bytes[a + 33..a + 37].copy_from_slice(&sum.to_le_bytes());
    let mut reader = dot_logger::storage::LossReader::new(&bytes, 16, 2).unwrap();
    assert!(reader.next().unwrap().is_err());
}

#[test]
fn rotated_file_baselines_start_at_their_own_observation() {
    use dot_logger::storage::*;
    let plan = loss_plan();
    let routes = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        support::tracking(16),
    )
    .unwrap();
    owner.enable_loss_tracking().unwrap();
    owner.scope(|scope| {
        let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), routes).unwrap();
        let state = Rc::new(RefCell::new(State {
            free: 10_000_000,
            ..State::default()
        }));
        let retention = RetentionConfig {
            mode: RetentionMode::Continuous,
            disk_bytes: 10_000_000,
            disk_reserve_bytes: 0,
            max_age_ns: 1000,
            max_segment_bytes: 200_000,
            max_segment_ns: 1000,
            max_index_bytes: 16384,
            protected_bytes: 300_000,
            pre_ns: 0,
            post_ns: 0,
            max_segments: 4,
            max_events: 1,
            max_pins: 1,
            max_pin_bytes: 4096,
            max_pin_ns: 100,
        };
        let mut log = SegmentLog::new(
            router.writer_budget(10).unwrap(),
            writer_config(),
            storage(),
            retention,
            Store(Rc::clone(&state)),
            0,
        )
        .unwrap();
        log.rotate(1).unwrap();
        log.checkpoint(2).unwrap();
        log.finish(3).unwrap();
        let state = state.borrow();
        let bytes = state.files.get(&1).unwrap();
        assert_eq!(NativeReader::new(bytes, 16, 2).unwrap().info().sequence, 1);
        let reports = LossReader::new(bytes, 16, 2)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let start = reports[0].observed_from_ns;
        assert!(start > 0);
        assert!(
            dot_logger::loss::summarize(&reports, 1024)
                .unwrap()
                .iter()
                .all(|d| d.observed_from_ns == start)
        );
    });
}
