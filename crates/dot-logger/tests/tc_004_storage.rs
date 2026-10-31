//! TC-004: product envelopes, bounded file writers and independent container checks.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::capture::{
    CaptureOwner, CaptureReceipt, CapturedRecord, ChannelConfig, Importance, SourceTime,
};
use dot_logger::routing::Router;
use dot_logger::storage::{
    SegmentWriter, StorageChannel, WriterConfig, decode_record, encode_record,
};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> (Self, File) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "dot-logger-{}-{}.mcap",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        (Self(path), file)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn record(seq: u64, mono: u64, real: i64, len: usize) -> CapturedRecord {
    let mut metadata = support::meta();
    metadata.importance = Some(Importance::Important);
    metadata.mono_ns = mono;
    metadata.real_ns = real;
    metadata.source_time = Some(SourceTime {
        ticks: -7,
        clock_domain: 17,
    });
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: seq,
        },
        metadata,
        revision: 1,
        payload_len: len,
    }
}
fn config(max: usize) -> WriterConfig {
    WriterConfig {
        max_payload_bytes: max,
        write_buffer_bytes: 4096,
        chunk_target_bytes: 1024,
        max_records: 8,
        max_chunks: 8,
        max_metadata_bytes: 4096,
    }
}

#[test]
fn delivery_batch_flushes_one_chunk_and_reports_every_written_record() {
    use dot_logger::completion::{Boundary, QueryOutcome};
    use std::time::Instant;
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
        let (path, file) = Temp::new();
        let mut limits = config(256);
        limits.max_chunks = 1;
        let mut writer =
            SegmentWriter::new(router.writer_budget(10).unwrap(), limits, channels(), file)
                .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let receipts: Vec<_> = (0..4)
                .map(|_| {
                    support::accepted(producer.try_record_small(support::meta(), &[1, 2]).unwrap())
                })
                .collect();
            dispatch.turn(now, &mut [None; 8]).unwrap();
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let deliveries: Vec<_> = (0..4)
                .map(|_| outputs[0].try_next(now).unwrap().unwrap())
                .collect();
            let refs: Vec<_> = deliveries.iter().collect();
            writer.write_batch(&refs).unwrap();
            assert_eq!(writer.index().len(), 1);
            assert_eq!(writer.index()[0].record_count, 4);
            for receipt in receipts {
                assert_eq!(
                    completion
                        .poll(receipt, 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
            }
            drop(deliveries);
            drop(writer.finish().unwrap());
            let bytes = fs::read(&path.0).unwrap();
            assert_eq!(independent_messages(&bytes, bytes.len() - 8), Some(4));
            assert_eq!(
                mcap::Summary::read(&bytes)
                    .unwrap()
                    .unwrap()
                    .chunk_indexes
                    .len(),
                1
            );
        });
    });
}

#[test]
fn batch_flush_failure_reports_no_written_success_and_drop_does_no_io() {
    use dot_logger::completion::{Boundary, QueryOutcome};
    use dot_logger::storage::StorageError;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize},
        },
        time::Instant,
    };
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
        let (path, file) = Temp::new();
        let fail = Arc::new(AtomicBool::new(false));
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config(256),
            channels(),
            FailingFile {
                file,
                fail: fail.clone(),
                syncs: Arc::new(AtomicUsize::new(0)),
                fail_sync: Arc::new(AtomicBool::new(false)),
            },
        )
        .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let receipts: Vec<_> = (0..2)
                .map(|_| {
                    support::accepted(producer.try_record_small(support::meta(), &[1, 2]).unwrap())
                })
                .collect();
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let a = outputs[0].try_next(now).unwrap().unwrap();
            let b = outputs[0].try_next(now).unwrap().unwrap();
            fail.store(true, Ordering::Release);
            let err = writer.write_batch(&[&a, &b]).unwrap_err();
            assert_eq!((err.appended, err.written, err.reported), (2, 0, 0));
            assert!(matches!(err.error, StorageError::Io(_)));
            assert_eq!(writer.safe_offset(), 0);
            for receipt in receipts {
                assert_eq!(
                    completion
                        .poll(receipt, 10, Boundary::Written)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Pending
                );
            }
            let before = fs::read(&path.0).unwrap();
            drop(writer);
            assert_eq!(fs::read(&path.0).unwrap(), before);
        });
    });
}

#[test]
fn partial_batch_rejection_exposes_the_appended_prefix_for_explicit_flush_and_report() {
    use dot_logger::{
        completion::{Boundary, Update},
        storage::StorageError,
    };
    use std::time::Instant;
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
        let (path, file) = Temp::new();
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config(256),
            channels(),
            file,
        )
        .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            producer.try_record_small(support::meta(), &[1, 2]).unwrap();
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let delivery = outputs[0].try_next(now).unwrap().unwrap();
            let err = writer.write_batch(&[&delivery, &delivery]).unwrap_err();
            assert_eq!((err.appended, err.written, err.reported), (1, 0, 0));
            assert!(matches!(err.error, StorageError::InvalidRecord));
            writer.flush_chunk().unwrap();
            delivery
                .report(Update::Confirmed(Boundary::Written))
                .unwrap();
            drop(delivery);
            drop(writer.finish().unwrap());
            let bytes = fs::read(&path.0).unwrap();
            assert_eq!(independent_messages(&bytes, bytes.len() - 8), Some(1));
        });
    });
}

#[test]
fn rejected_sync_work_can_be_retried_on_the_correct_active_or_finished_file() {
    use dot_logger::{
        completion::{Boundary, ProducerRange, QueryOutcome, TargetRequest},
        control::{ControlConfig, ControlOwner},
        storage::StorageError,
    };
    use std::time::{Duration, Instant};
    for finished in [false, true] {
        let mut settings = support::plan().config().clone();
        settings.memory.total_bytes = 2 * 1024 * 1024;
        settings.memory.output_bytes = 1024 * 1024;
        let plan = dot_logger::config::SessionBuilder::new(settings)
            .validate(&dot_logger::platform::PlatformCapabilities::default())
            .unwrap();
        let route = support::routing();
        let mut tracking = support::tracking(4);
        tracking.targets[0].completion = Boundary::Synced;
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
            let completion = scope.completion().unwrap();
            let now = Instant::now();
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
            let (_pa, a) = Temp::new();
            let (_pb, b) = Temp::new();
            let mut wa = SegmentWriter::new(
                router.writer_budget(10).unwrap(),
                config(256),
                channels(),
                a,
            )
            .unwrap();
            let mut wb = SegmentWriter::new(
                router.writer_budget(20).unwrap(),
                config(256),
                channels(),
                b,
            )
            .unwrap();
            router.scope(|mut dispatch, mut outputs| {
                let receipt =
                    support::accepted(producer.try_record_small(support::meta(), &[3, 4]).unwrap());
                dispatch.turn(now, &mut [None; 8]).unwrap();
                let a = outputs[0].try_next(now).unwrap().unwrap();
                wa.write_delivery(&a).unwrap();
                drop(a);
                let b = outputs[1].try_next(now).unwrap().unwrap();
                wb.write_delivery(&b).unwrap();
                drop(b);
                let mut ranges = [ProducerRange::default(); 2];
                let frozen = completion.capture_scope(&mut ranges).unwrap();
                let token = control
                    .request_sync(frozen, &[10], now, now + Duration::from_secs(1))
                    .unwrap();
                let work = control
                    .next_sync(
                        token,
                        10,
                        now,
                        &mut dispatch,
                        &mut [ProducerRange::default(); 2],
                    )
                    .unwrap()
                    .unwrap();
                // Query permits are unrelated to file coverage's direct poll path.
                let requests = [TargetRequest {
                    target_id: 10,
                    boundary: Boundary::Synced,
                }];
                let q1 = completion.query(frozen, &requests).unwrap();
                let q2 = completion.query(frozen, &requests).unwrap();
                if finished {
                    let mut wa = wa.finish().unwrap();
                    let mut wb = wb.finish().unwrap();
                    let err = wb.sync_work(work).unwrap_err();
                    assert!(matches!(err.error, StorageError::InvalidRecord));
                    let work = err.work.unwrap();
                    control.poll(now).unwrap();
                    assert_eq!(
                        completion
                            .poll(receipt, 10, Boundary::Synced)
                            .unwrap()
                            .outcome,
                        QueryOutcome::Pending
                    );
                    wa.sync_work(work).unwrap();
                } else {
                    let err = wb.sync_work(work).unwrap_err();
                    assert!(matches!(err.error, StorageError::InvalidRecord));
                    let work = err.work.unwrap();
                    control.poll(now).unwrap();
                    assert_eq!(
                        completion
                            .poll(receipt, 10, Boundary::Synced)
                            .unwrap()
                            .outcome,
                        QueryOutcome::Pending
                    );
                    wa.sync_work(work).unwrap();
                }
                drop((q1, q2));
                control.poll(now).unwrap();
                assert_eq!(
                    completion
                        .poll(receipt, 10, Boundary::Synced)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Complete
                );
            });
        });
    }
}

struct CountingFile {
    file: File,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    max_write: usize,
}
impl std::io::Write for CountingFile {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        std::io::Write::write(&mut self.file, &b[..b.len().min(self.max_write)])
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut self.file)
    }
}
impl Seek for CountingFile {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(p)
    }
}
impl dot_logger::storage::SyncSink for CountingFile {
    fn sync_all(&mut self) -> std::io::Result<()> {
        self.file.sync_all()
    }
}

#[test]
fn bounded_write_buffer_reduces_file_calls_and_preserves_bytes_with_short_writes() {
    use std::sync::{Arc, atomic::AtomicUsize};
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let budget = router.writer_budget(10).unwrap();
        let mut results = Vec::new();
        for (capacity, max_write) in [(0, usize::MAX), (4096, usize::MAX), (4096, 3)] {
            let (path, file) = Temp::new();
            let calls = Arc::new(AtomicUsize::new(0));
            let mut limits = config(256);
            limits.max_records = 128;
            limits.max_chunks = 2;
            limits.chunk_target_bytes = 65536;
            limits.write_buffer_bytes = capacity;
            let mut writer = SegmentWriter::new(
                budget.clone(),
                limits,
                channels(),
                CountingFile {
                    file,
                    calls: calls.clone(),
                    max_write,
                },
            )
            .unwrap();
            for seq in 1..=128 {
                writer
                    .append(record(seq, seq, seq as i64, 64), &[7; 64])
                    .unwrap();
            }
            writer.flush_chunk().unwrap();
            let safe = writer.safe_offset();
            assert_eq!(fs::metadata(&path.0).unwrap().len(), safe);
            drop(writer.finish().unwrap());
            let bytes = fs::read(&path.0).unwrap();
            assert_eq!(independent_messages(&bytes, bytes.len() - 8), Some(128));
            results.push((calls.load(Ordering::Relaxed), bytes));
        }
        println!(
            "128 records: direct writes={}, buffered writes={}, short writes={}",
            results[0].0, results[1].0, results[2].0
        );
        assert!(
            results[1].0 * 4 < results[0].0,
            "buffer must coalesce small writes"
        );
        assert_eq!(results[0].1, results[1].1);
        assert_eq!(results[0].1, results[2].1);
    });
}

#[test]
fn channel_chunk_reservation_is_bounded_by_total_records() {
    let mut limits = config(256);
    limits.max_records = 100_000;
    limits.max_chunks = 100_000;
    let channels: Vec<_> = (0..128)
        .map(|id| StorageChannel {
            channel: ChannelConfig::new(id, 99, 1),
            topic: format!("channel/{id}"),
            payload_encoding: "bytes".into(),
            schema: None,
        })
        .collect();
    let reservation = limits.reservation(&channels, 32).unwrap();
    println!("128 channels / 100000 records/chunks: reservation={reservation}");
    assert!(
        reservation < 128 * 1024 * 1024,
        "one record contributes at most one channel/chunk pair"
    );
}

#[test]
fn dropping_buffered_open_data_does_not_write_or_flush_it() {
    use std::sync::{Arc, atomic::AtomicUsize};
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let (path, file) = Temp::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config(256),
            channels(),
            CountingFile {
                file,
                calls: calls.clone(),
                max_write: usize::MAX,
            },
        )
        .unwrap();
        writer.append(record(1, 1, 100, 2), &[1, 2]).unwrap();
        assert_eq!(writer.safe_offset(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        drop(writer);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(fs::metadata(&path.0).unwrap().len(), 0);
    });
}

#[test]
fn partially_failed_buffer_drain_keeps_safe_offset_and_drop_does_not_retry() {
    use dot_logger::storage::{StorageError, SyncSink};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize},
    };
    struct FaultFile {
        inner: CountingFile,
        remaining: Arc<AtomicUsize>,
        interrupt: Arc<AtomicBool>,
    }
    impl std::io::Write for FaultFile {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if self.interrupt.swap(false, Ordering::Relaxed) {
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            let left = self.remaining.load(Ordering::Relaxed);
            if left == 0 {
                return Err(std::io::Error::other("injected partial write failure"));
            }
            let count = std::io::Write::write(&mut self.inner, &b[..b.len().min(left)])?;
            self.remaining.fetch_sub(count, Ordering::Relaxed);
            Ok(count)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            std::io::Write::flush(&mut self.inner)
        }
    }
    impl Seek for FaultFile {
        fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(p)
        }
    }
    impl SyncSink for FaultFile {
        fn sync_all(&mut self) -> std::io::Result<()> {
            self.inner.sync_all()
        }
    }
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let (path, file) = Temp::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let remaining = Arc::new(AtomicUsize::new(usize::MAX));
        let interrupt = Arc::new(AtomicBool::new(true));
        let sink = FaultFile {
            inner: CountingFile {
                file,
                calls: calls.clone(),
                max_write: usize::MAX,
            },
            remaining: remaining.clone(),
            interrupt,
        };
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config(256),
            channels(),
            sink,
        )
        .unwrap();
        writer.append(record(1, 1, 1, 2), &[1, 2]).unwrap();
        let safe = writer.flush_chunk().unwrap();
        writer.append(record(2, 2, 2, 2), &[3, 4]).unwrap();
        remaining.store(3, Ordering::Relaxed);
        assert!(matches!(writer.flush_chunk(), Err(StorageError::Io(_))));
        assert_eq!(writer.safe_offset(), safe);
        assert_eq!(writer.index().len(), 1);
        assert!(matches!(
            writer.append(record(3, 3, 3, 2), &[5, 6]),
            Err(StorageError::Poisoned)
        ));
        let before = fs::read(&path.0).unwrap();
        let writes = calls.load(Ordering::Relaxed);
        drop(writer);
        assert_eq!(calls.load(Ordering::Relaxed), writes);
        assert_eq!(fs::read(&path.0).unwrap(), before);
    });
}
fn channels() -> Vec<StorageChannel> {
    vec![StorageChannel {
        channel: ChannelConfig::new(7, 99, 1),
        topic: "sensor/opaque".into(),
        payload_encoding: "unknown/vendor-format".into(),
        schema: None,
    }]
}

#[test]
fn product_envelope_preserves_full_receipt_clocks_and_unknown_payload() {
    let expected = record(u64::MAX, u64::MAX, 100, 3);
    let mut bytes = Vec::new();
    encode_record(expected, &[0, 128, 255], &mut bytes, 3).unwrap();
    let decoded = decode_record(&bytes, 3).unwrap();
    assert_eq!(decoded.record, expected);
    assert_eq!(decoded.payload, [0, 128, 255]);
}

#[test]
fn file_writer_preserves_large_records_wall_reversal_and_safe_chunks() {
    use dot_logger::config::SessionBuilder;
    use dot_logger::platform::PlatformCapabilities;
    let mut settings = support::plan().config().clone();
    settings.memory.total_bytes = 128 * 1024 * 1024;
    settings.memory.output_bytes = 64 * 1024 * 1024;
    let plan = SessionBuilder::new(settings)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let route = support::routing();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), route).unwrap();
        let budget = router.writer_budget(10).unwrap();
        let (path, file) = Temp::new();
        let mut writer =
            SegmentWriter::new(budget, config(8 * 1024 * 1024), channels(), file).unwrap();
        let payload = vec![0x5a; 8 * 1024 * 1024];
        writer
            .append(
                record(u64::from(u32::MAX) + 7, 50, 900, payload.len()),
                &payload,
            )
            .unwrap();
        assert_eq!(writer.safe_offset(), 0);
        let safe = writer.flush_chunk().unwrap();
        assert!(safe > payload.len() as u64);
        writer
            .append(record(u64::MAX, 20, 100, 2), &[3, 4])
            .unwrap();
        assert_eq!(writer.safe_offset(), safe);
        let finished = writer.finish().unwrap();
        assert_eq!(finished.index().len(), 2);
        let mut file = finished.into_inner();
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        drop(file);
        let messages = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(independent_messages(&bytes, bytes.len() - 8), Some(2));
        let first = decode_record(&messages[0].data, payload.len()).unwrap();
        assert_eq!(first.record.receipt.accepted_seq, u64::from(u32::MAX) + 7);
        assert_eq!(first.payload, payload);
        let summary = mcap::Summary::read(&bytes).unwrap().unwrap();
        assert_eq!(summary.chunk_indexes.len(), 2);
        drop(path);
    });
}

#[test]
fn only_one_active_writer_per_target_and_finished_index_retains_its_budget() {
    use dot_logger::{
        config::SessionBuilder, platform::PlatformCapabilities, storage::StorageError,
    };
    let mut settings = support::plan().config().clone();
    settings.memory.total_bytes = 8 * 1024 * 1024;
    settings.memory.output_bytes = 4 * 1024 * 1024;
    let plan = SessionBuilder::new(settings)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let budget = router.writer_budget(10).unwrap();
        let (_a, a) = Temp::new();
        let (_b, b) = Temp::new();
        let (_c, c) = Temp::new();
        let writer = SegmentWriter::new(budget.clone(), config(256), channels(), a).unwrap();
        assert!(matches!(
            SegmentWriter::new(budget.clone(), config(256), channels(), b),
            Err(StorageError::Busy)
        ));
        let finished = writer.finish().unwrap();
        assert!(budget.used_bytes() > 0);
        let next = SegmentWriter::new(budget.clone(), config(256), channels(), c).unwrap();
        drop(next);
        assert!(budget.used_bytes() > 0);
        drop(finished);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn a_new_segment_cannot_acknowledge_an_older_segments_unsynced_records() {
    use dot_logger::{
        completion::{Boundary, ProducerRange, QueryOutcome, Update},
        control::{ControlConfig, ControlOwner},
        storage::StorageError,
    };
    use std::time::{Duration, Instant};
    let plan = support::plan();
    let route = support::routing();
    let mut tracking = support::tracking(4);
    tracking.targets[0].completion = Boundary::Synced;
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
        let completion = scope.completion().unwrap();
        let now = Instant::now();
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
        let (_old, old) = Temp::new();
        let (_new, new) = Temp::new();
        let mut first = SegmentWriter::new(budget.clone(), config(256), channels(), old).unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let receipt =
                support::accepted(producer.try_record_small(support::meta(), &[3, 4]).unwrap());
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let delivery = outputs[0].try_next(now).unwrap().unwrap();
            first.write_delivery(&delivery).unwrap();
            drop(delivery);
            let optional = outputs[1].try_next(now).unwrap().unwrap();
            optional
                .report(Update::Confirmed(Boundary::Written))
                .unwrap();
            drop(optional);
            drop(first.finish().unwrap());
            let mut second = SegmentWriter::new(budget, config(256), channels(), new).unwrap();
            let mut ranges = [ProducerRange::default(); 2];
            let frozen = completion.capture_scope(&mut ranges).unwrap();
            let token = control
                .request_sync(frozen, &[10], now, now + Duration::from_secs(1))
                .unwrap();
            let work = control
                .next_sync(
                    token,
                    10,
                    now,
                    &mut dispatch,
                    &mut [ProducerRange::default(); 2],
                )
                .unwrap()
                .unwrap();
            assert!(matches!(
                second.sync_work(work).map_err(|e| e.error),
                Err(StorageError::InvalidRecord)
            ));
            control.poll(now).unwrap();
            assert_ne!(
                completion
                    .poll(receipt, 10, Boundary::Synced)
                    .unwrap()
                    .outcome,
                QueryOutcome::Complete
            );
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Written)
                    .unwrap()
                    .outcome,
                QueryOutcome::Complete
            );
        });
    });
}

// Independent uncompressed MCAP scanner: no mcap/prost parsing code is used for
// record lengths, chunk CRC, message offsets or message-index validation.
fn u32_at(b: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(p..p.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn u64_at(b: &[u8], p: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        b.get(p..p.checked_add(8)?)?.try_into().ok()?,
    ))
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
fn raw_records(bytes: &[u8]) -> Option<Vec<(u8, usize, &[u8])>> {
    let mut p = 0;
    let mut records = Vec::new();
    while p < bytes.len() {
        let opcode = *bytes.get(p)?;
        let len = usize::try_from(u64_at(bytes, p + 1)?).ok()?;
        let end = p.checked_add(9)?.checked_add(len)?;
        records.push((opcode, p, bytes.get(p + 9..end)?));
        p = end;
    }
    Some(records)
}
fn independent_messages(bytes: &[u8], end: usize) -> Option<usize> {
    if bytes.get(..8)? != b"\x89MCAP0\r\n" {
        return None;
    }
    let mut messages = 0;
    let mut expected = std::collections::BTreeMap::<u16, Vec<(u64, u64)>>::new();
    for (op, _, body) in raw_records(bytes.get(8..end)?)? {
        if op == 6 {
            if u32_at(body, 28)? != 0 {
                return None;
            }
            let n = usize::try_from(u64_at(body, 32)?).ok()?;
            let records = body.get(40..40usize.checked_add(n)?)?;
            if n as u64 != u64_at(body, 16)? || crc32(records) != u32_at(body, 24)? {
                return None;
            }
            expected.clear();
            let mut times = Vec::new();
            for (nested, offset, data) in raw_records(records)? {
                if nested == 5 {
                    let channel = u16::from_le_bytes(data.get(..2)?.try_into().ok()?);
                    let time = u64_at(data, 6)?;
                    if data.len() < 22 {
                        return None;
                    }
                    expected
                        .entry(channel)
                        .or_default()
                        .push((time, offset as u64));
                    times.push(time);
                    messages += 1;
                }
            }
            if let (Some(min), Some(max)) = (times.iter().min(), times.iter().max()) {
                if *min != u64_at(body, 0)? || *max != u64_at(body, 8)? {
                    return None;
                }
            }
        } else if op == 7 {
            let channel = u16::from_le_bytes(body.get(..2)?.try_into().ok()?);
            let count = u32_at(body, 2)? as usize;
            if count % 16 != 0 || count.checked_add(6)? != body.len() {
                return None;
            }
            let actual = (6..body.len())
                .step_by(16)
                .map(|p| Some((u64_at(body, p)?, u64_at(body, p + 8)?)))
                .collect::<Option<Vec<_>>>()?;
            if expected.get(&channel)? != &actual {
                return None;
            }
        }
    }
    Some(messages)
}

#[test]
fn envelope_golden_and_invalid_lengths_versions_are_checked() {
    use dot_logger::storage::StorageError;
    let golden = include_bytes!("fixtures/envelope_v1.bin");
    let expected = record(u64::MAX, u64::MAX, 100, 3);
    assert_eq!(decode_record(golden, 3).unwrap().record, expected);
    let mut encoded = Vec::new();
    encode_record(expected, &[0, 128, 255], &mut encoded, 3).unwrap();
    assert_eq!(encoded, golden);
    encoded.extend_from_slice(&[0x98, 0x06, 1]);
    assert_eq!(decode_record(&encoded, 3).unwrap().record, expected);
    let mut future = golden.to_vec();
    future[1] = 2;
    assert!(matches!(
        decode_record(&future, 3),
        Err(StorageError::UnsupportedVersion(2))
    ));
    let mut bad = golden.to_vec();
    bad.extend_from_slice(&[0x70, 127]);
    assert!(matches!(
        decode_record(&bad, 3),
        Err(StorageError::CorruptData)
    ));
    assert!(decode_record(golden, 2).is_err());
    assert!(decode_record(&golden[..golden.len() - 1], 3).is_err());
    let mut negative = expected;
    negative.metadata.real_ns = -1;
    assert!(encode_record(negative, &[0, 128, 255], &mut encoded, 3).is_err());
    assert_eq!(crc32(b"123456789"), 0xcbf43926);
}

#[test]
fn bounded_indexes_minmax_rejections_and_drop_do_not_publish_extra_data() {
    use dot_logger::storage::StorageError;
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let budget = router.writer_budget(10).unwrap();
        let (path, file) = Temp::new();
        let mut limits = config(4);
        limits.chunk_target_bytes = usize::MAX;
        limits.max_records = 2;
        limits.max_chunks = 1;
        let mut writer = SegmentWriter::new(budget.clone(), limits, channels(), file).unwrap();
        let initial = fs::read(&path.0).unwrap();
        assert!(writer.append(record(1, 1, -1, 2), &[1, 2]).is_err());
        assert_eq!(fs::read(&path.0).unwrap(), initial);
        writer.append(record(1, 50, 900, 2), &[1, 2]).unwrap();
        writer.append(record(2, 20, 100, 2), &[3, 4]).unwrap();
        assert!(writer.index().is_empty());
        let before = fs::read(&path.0).unwrap();
        assert!(matches!(
            writer.append(record(3, 70, 1000, 2), &[5, 6]),
            Err(StorageError::NeedRotation)
        ));
        assert_eq!(fs::read(&path.0).unwrap(), before);
        let safe = writer.flush_chunk().unwrap();
        let index = writer.index()[0];
        assert_eq!(
            (
                index.mono_min,
                index.mono_max,
                index.real_min,
                index.real_max,
                index.record_count
            ),
            (20, 50, 100, 900, 2)
        );
        let bytes = fs::read(&path.0).unwrap();
        assert_eq!(independent_messages(&bytes, safe as usize), Some(2));
        let mut corrupt = bytes.clone();
        let at = corrupt.len() - 20;
        corrupt[at] ^= 1;
        assert_ne!(independent_messages(&corrupt, safe as usize), Some(2));
        drop(writer);
        assert_eq!(fs::read(&path.0).unwrap(), bytes);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn output_reservations_accept_exact_fit_and_reject_one_byte_short() {
    use dot_logger::{
        config::SessionBuilder, platform::PlatformCapabilities, storage::StorageError,
    };
    let original = support::plan();
    let mut first = CaptureOwner::new(&original, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    let router_bytes = first.scope(|scope| {
        let router = Router::new(
            &original,
            scope.map(|(_, r)| r).collect(),
            support::routing(),
        )
        .unwrap();
        original.config().memory.output_bytes - router.writer_budget(10).unwrap().limit_bytes()
    });
    let required = config(256).reservation(&channels(), 2).unwrap();
    for fits in [true, false] {
        let mut settings = original.config().clone();
        settings.memory.output_bytes = router_bytes + required - usize::from(!fits);
        let plan = SessionBuilder::new(settings)
            .validate(&PlatformCapabilities::default())
            .unwrap();
        let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        owner.scope(|scope| {
            let router =
                Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
            let budget = router.writer_budget(10).unwrap();
            let (path, file) = Temp::new();
            let result = SegmentWriter::new(budget.clone(), config(256), channels(), file);
            if fits {
                assert!(result.is_ok());
            } else {
                assert!(matches!(result, Err(StorageError::Budget { .. })));
                assert_eq!(fs::metadata(&path.0).unwrap().len(), 0);
            }
            drop(result);
            assert_eq!(budget.used_bytes(), 0);
        });
    }
    let mut limits = config(256);
    limits.max_chunks = usize::MAX;
    assert!(limits.reservation(&channels(), 2).is_err());
    limits = config(usize::MAX);
    assert!(limits.reservation(&channels(), 2).is_err());
    if let Some(too_wide) = (u32::MAX as usize).checked_add(1) {
        limits = config(256);
        limits.max_metadata_bytes = too_wide;
        assert!(limits.reservation(&channels(), 2).is_err());
    }
}

#[test]
fn schema_versions_and_wide_channel_ids_use_distinct_file_local_ids() {
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let (path, file) = Temp::new();
        let mut mapping = channels();
        mapping[0].schema = Some(dot_logger::storage::PayloadSchema {
            encoding: "vendor-schema".into(),
            descriptor: vec![1, 2],
        });
        let mut second = mapping[0].clone();
        second.channel.channel_id = u32::MAX;
        second.channel.schema_version = 2;
        second.topic = "version2".into();
        second.schema.as_mut().unwrap().descriptor = vec![3, 4];
        mapping.push(second);
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config(256),
            mapping,
            file,
        )
        .unwrap();
        writer.append(record(1, 1, 100, 2), &[1, 2]).unwrap();
        let mut v2 = record(2, 2, 99, 2);
        v2.metadata.channel_id = u32::MAX;
        v2.metadata.schema_version = 2;
        writer.append(v2, &[3, 4]).unwrap();
        drop(writer.finish().unwrap());
        let bytes = fs::read(&path.0).unwrap();
        let messages = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_ne!(messages[0].channel.id, messages[1].channel.id);
        assert_eq!(
            mcap::Summary::read(&bytes).unwrap().unwrap().schemas.len(),
            3
        );
        assert_eq!(decode_record(&messages[1].data, 256).unwrap().record, v2);
        assert_eq!(independent_messages(&bytes, bytes.len() - 8), Some(2));
    });
}

struct FailingFile {
    file: File,
    fail: std::sync::Arc<std::sync::atomic::AtomicBool>,
    syncs: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    fail_sync: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl std::io::Write for FailingFile {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.file, b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.fail.load(Ordering::Acquire) {
            Err(std::io::Error::other("injected flush failure"))
        } else {
            std::io::Write::flush(&mut self.file)
        }
    }
}
impl Seek for FailingFile {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(p)
    }
}
impl dot_logger::storage::SyncSink for FailingFile {
    fn sync_all(&mut self) -> std::io::Result<()> {
        self.syncs.fetch_add(1, Ordering::Relaxed);
        if self.fail_sync.load(Ordering::Acquire) {
            return Err(std::io::Error::other("injected sync failure"));
        }
        self.file.sync_all()
    }
}
#[test]
fn failed_flush_never_advances_safe_offset_and_poisoned_drop_does_no_io() {
    use dot_logger::storage::StorageError;
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let (path, file) = Temp::new();
        let fail = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let syncs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config(256),
            channels(),
            FailingFile {
                file,
                fail: fail.clone(),
                syncs: syncs.clone(),
                fail_sync: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            },
        )
        .unwrap();
        writer.append(record(1, 1, 100, 2), &[1, 2]).unwrap();
        let safe = writer.flush_chunk().unwrap();
        writer.append(record(2, 2, 99, 2), &[3, 4]).unwrap();
        fail.store(true, Ordering::Release);
        assert!(matches!(writer.flush_chunk(), Err(StorageError::Io(_))));
        assert_eq!(writer.safe_offset(), safe);
        assert_eq!(writer.index().len(), 1);
        assert!(matches!(
            writer.append(record(3, 3, 98, 2), &[5, 6]),
            Err(StorageError::Poisoned)
        ));
        let before = fs::read(&path.0).unwrap();
        drop(writer);
        assert_eq!(fs::read(&path.0).unwrap(), before);
        assert_eq!(syncs.load(Ordering::Relaxed), 0);
    });
}

fn actual_sync(fail: bool) {
    use dot_logger::{
        completion::{Boundary, FailureReason, ProducerRange, QueryOutcome},
        config::SessionBuilder,
        control::{ControlConfig, ControlOwner, ShutdownPlan, ShutdownPoll},
        platform::PlatformCapabilities,
        storage::StorageError,
    };
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize},
        },
        time::{Duration, Instant},
    };
    let mut settings = support::plan().config().clone();
    settings.memory.total_bytes = 8 * 1024 * 1024;
    settings.memory.output_bytes = 4 * 1024 * 1024;
    let plan = SessionBuilder::new(settings)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let route = support::routing();
    let mut tracking = support::tracking(4);
    tracking.targets[0].completion = Boundary::Synced;
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
        let completion=scope.completion().unwrap();let now=Instant::now();let end=now+Duration::from_secs(10);
        let control=ControlOwner::new(&plan,scope.control(),ControlConfig{sync_slots:1,work_per_poll:32},now).unwrap();
        let (mut producer,a)=scope.next().unwrap();let (_,b)=scope.next().unwrap();let mut router=Router::new(&plan,vec![a,b],route).unwrap();
        let (_pa,a)=Temp::new();let (_pb,b)=Temp::new();let syncs=Arc::new(AtomicUsize::new(0));let rejected=Arc::new(AtomicBool::new(fail));
        let mut wa=SegmentWriter::new(router.writer_budget(10).unwrap(),config(256),channels(),FailingFile{file:a,fail:Arc::new(AtomicBool::new(false)),syncs:syncs.clone(),fail_sync:rejected}).unwrap();
        let mut wb=SegmentWriter::new(router.writer_budget(20).unwrap(),config(256),channels(),b).unwrap();
        router.scope(|mut dispatch,mut outputs| {
            let receipt=support::accepted(producer.try_record_small(support::meta(),&[3,4]).unwrap());dispatch.turn(now,&mut[None;8]).unwrap();
            let delivery=outputs[0].try_next(now).unwrap().unwrap();wa.write_delivery(&delivery).unwrap();drop(delivery);
            let delivery=outputs[1].try_next(now).unwrap().unwrap();wb.write_delivery(&delivery).unwrap();drop(delivery);
            let mut wa=wa.finish().unwrap();let mut wb=wb.finish().unwrap();assert_eq!(syncs.load(Ordering::Relaxed),0);
            let mut ranges=[ProducerRange::default();2];let frozen=completion.capture_scope(&mut ranges).unwrap();
            let token=control.request_sync(frozen,&[10],now,end).unwrap();
            let work=control.next_sync(token,10,now,&mut dispatch,&mut[ProducerRange::default();2]).unwrap().unwrap();
            let result=wa.sync_work(work);control.poll(now).unwrap();
            assert_eq!(syncs.load(Ordering::Relaxed),1);
            assert_eq!(completion.poll(receipt,10,Boundary::Written).unwrap().outcome,QueryOutcome::Complete);
            if fail {
                assert!(matches!(result,Err(e) if matches!(e.error, StorageError::Io(_)) && e.work.is_none()));
                assert_eq!(completion.poll(receipt,10,Boundary::Synced).unwrap().outcome,QueryOutcome::Failed(FailureReason::SyncFailure));
            } else {
                result.unwrap();assert_eq!(completion.poll(receipt,10,Boundary::Synced).unwrap().outcome,QueryOutcome::Complete);
                control.release_sync(token).unwrap();control.begin_shutdown(ShutdownPlan{sync_files:true},now,end,&mut dispatch).unwrap();
                let shutdown=control.shutdown_sync().unwrap();
                let a=control.next_sync(shutdown,10,now,&mut dispatch,&mut[ProducerRange::default();2]).unwrap().unwrap();wa.sync_work(a).unwrap();
                let b=control.next_sync(shutdown,20,now,&mut dispatch,&mut[ProducerRange::default();2]).unwrap().unwrap();wb.sync_work(b).unwrap();
                assert!(matches!(control.poll_shutdown(now,&mut dispatch).unwrap(),ShutdownPoll::Closed(report) if report.is_success()));
                assert_eq!(syncs.load(Ordering::Relaxed),2);
            }
        });
    });
}
#[test]
fn actual_file_sync_after_footer_drives_completion_and_shutdown() {
    actual_sync(false);
}
#[test]
fn actual_sync_failure_preserves_written_and_never_reports_synced() {
    actual_sync(true);
}

#[test]
fn chunk_limit_rejects_before_record_limit_without_flushing_or_writing() {
    use dot_logger::storage::StorageError;
    let plan = support::plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let router =
            Router::new(&plan, scope.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let (path, file) = Temp::new();
        let mut limits = config(256);
        limits.max_chunks = 1;
        limits.chunk_target_bytes = 1;
        let mut writer =
            SegmentWriter::new(router.writer_budget(10).unwrap(), limits, channels(), file)
                .unwrap();
        writer.append(record(1, 1, 100, 2), &[1, 2]).unwrap();
        let before = fs::read(&path.0).unwrap();
        assert!(matches!(
            writer.append(record(2, 2, 101, 2), &[3, 4]),
            Err(StorageError::NeedRotation)
        ));
        assert_eq!(writer.safe_offset(), 0);
        assert_eq!(fs::read(&path.0).unwrap(), before);
        writer.flush_chunk().unwrap();
        let before = fs::read(&path.0).unwrap();
        assert!(matches!(
            writer.append(record(2, 2, 101, 2), &[3, 4]),
            Err(StorageError::NeedRotation)
        ));
        assert_eq!(fs::read(&path.0).unwrap(), before);
    });
}

#[test]
fn expired_completion_after_a_successful_write_remains_distinct_from_invalid_input() {
    use dot_logger::{
        completion::{Boundary, CompletionError},
        storage::StorageError,
    };
    use std::time::Instant;
    let plan = support::plan();
    let route = support::routing();
    let mut tracking = support::tracking(1);
    for target in &mut tracking.targets {
        target.completion = Boundary::Enqueued;
    }
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
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        let (path, file) = Temp::new();
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            config(256),
            channels(),
            file,
        )
        .unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            producer.try_record_small(support::meta(), &[1, 2]).unwrap();
            dispatch.turn(now, &mut [None; 8]).unwrap();
            let old = outputs[0].try_next(now).unwrap().unwrap();
            producer.try_record_small(support::meta(), &[3, 4]).unwrap();
            dispatch.turn(now, &mut [None; 8]).unwrap();
            assert!(matches!(
                writer.write_delivery(&old),
                Err(StorageError::Completion(CompletionError::Expired))
            ));
            drop(old);
            let next = outputs[0].try_next(now).unwrap().unwrap();
            writer.write_delivery(&next).unwrap();
            drop(next);
            drop(writer.finish().unwrap());
            let bytes = fs::read(&path.0).unwrap();
            assert_eq!(mcap::MessageStream::new(&bytes).unwrap().count(), 2);
        });
    });
}

#[test]
fn live_revision_uses_explicit_separate_writer_metadata_and_roundtrips() {
    use dot_logger::control::config_update::{ChannelPatch, ConfigPatch};
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
    owner.enable_config_updates(&route).unwrap();
    owner.scope(|mut scope| {
        let updates = scope.config_updates().unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (mut p2, b) = scope.next().unwrap();
        updates
            .apply_config(
                1,
                &ConfigPatch {
                    channels: vec![ChannelPatch {
                        channel_id: 7,
                        minimum_level: dot_logger::capture::Level::Info,
                        sample_every: 1,
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
        p2.apply_pending_config();
        let receipt = support::accepted(p.try_record_small(support::meta(), &[4, 5]).unwrap());
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        assert!(router.writer_budget_for_revision(10, 99).is_err());
        let (path, file) = Temp::new();
        let mut writer = SegmentWriter::new(
            router.writer_budget_for_revision(10, 2).unwrap(),
            config(64),
            channels(),
            file,
        )
        .unwrap();
        router.scope(|mut d, mut readers| {
            d.turn(std::time::Instant::now(), &mut [None; 8]).unwrap();
            let delivery = readers[0]
                .try_next(std::time::Instant::now())
                .unwrap()
                .unwrap();
            assert_eq!(delivery.record().revision, 2);
            assert_eq!(delivery.record().receipt, receipt);
            writer.write_batch(&[&delivery]).unwrap();
        });
        writer.finish().unwrap();
        let bytes = fs::read(&path.0).unwrap();
        let mut messages = mcap::MessageStream::new(&bytes).unwrap();
        let message = messages.next().unwrap().unwrap();
        let decoded = decode_record(&message.data, 64).unwrap();
        assert_eq!(decoded.record.revision, 2);
        assert_eq!(decoded.record.receipt, receipt);
        assert_eq!(decoded.payload, &[4, 5]);
        assert_eq!(message.channel.metadata.get("revision").unwrap(), "2");
    });
}
