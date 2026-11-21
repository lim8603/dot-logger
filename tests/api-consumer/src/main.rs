//! Independent application: tracked admission, routing, writer acknowledgments, sync, and shutdown.

use dot_logger::capture::{CaptureOwner, ChannelConfig, Level, RecordMetadata, RecordOutcome};
use dot_logger::completion::{
    Boundaries, Boundary, CompletionConfig, ProducerRange, QueryOutcome, TargetContract,
};
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    WorkerThreadingConfig,
};
use dot_logger::control::{ControlConfig, ControlOwner, ShutdownPlan, ShutdownPoll};
use dot_logger::platform::{PlatformCapabilities, Requirement};
use dot_logger::routing::{Route, Router, RoutingConfig, Schedule, TargetConfig};
use dot_logger::storage::{SegmentWriter, StorageChannel, WriterConfig};
use std::time::{Duration, Instant};

struct TempPath(std::path::PathBuf);
impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const MIB: usize = 1024 * 1024;
    let config = SessionConfig {
        memory: MemoryBudget {
            total_bytes: 9 * MIB,
            input_bytes: MIB,
            raw_bytes: 0,
            output_bytes: 2 * MIB,
            external_bytes: MIB,
            control_bytes: MIB,
            worker_bytes: 4 * MIB,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 64,
                slot_bytes: 256,
                important_slots: 16,
            },
            raw: None,
        }],
        importance: ImportancePolicy::default(),
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: MIB,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        // Default capability observations are Unavailable, never assumed supported.
        capabilities: vec![],
    };
    let plan = SessionBuilder::new(config).validate(&PlatformCapabilities::default())?;
    assert_eq!(plan.reserved_bytes(), 9 * MIB);
    assert_eq!(plan.raw_pool_bytes(), 0);
    let routing = RoutingConfig {
        targets: vec![TargetConfig {
            id: 1,
            required: true,
            queue_slots: 2,
            pending_slots: 1,
            retained_bytes: 1024,
            max_attempts: 3,
            max_age: Duration::from_secs(1),
        }],
        routes: vec![Route {
            channel_id: 7,
            targets: vec![1],
        }],
        shared_records: 4,
        small_copy_bytes: 1024,
        schedule: Schedule {
            record_quantum: 2,
            byte_quantum: 256,
            records_per_turn: 2,
            bytes_per_turn: 512,
            retries_per_target: 1,
            reclaim_slots_per_producer: 1,
        },
    };
    let tracking = CompletionConfig {
        history_retention: Duration::from_secs(60),
        // Preserve sixteen Important opportunities while admitting Normal input.
        window_records: 32,
        max_queries: 2,
        page_items: 8,
        feedback_events: 8,
        updates_per_turn: 8,
        reconcile_per_turn: 4,
        targets: vec![TargetContract {
            id: 1,
            required: true,
            supported: Boundaries::FILE,
            completion: Boundary::Written,
        }],
    };
    // This consumer writes and syncs a local file. Device/power-loss durability
    // remains a separate hardware test.
    let mut ingress = CaptureOwner::new_tracked(
        &plan,
        1,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        tracking,
    )?;
    ingress.scope(|mut scope| -> Result<(), Box<dyn std::error::Error>> {
        let completion = scope.completion().ok_or("tracked completion missing")?;
        let now = Instant::now();
        let control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 32,
            },
            now,
        )?;
        let Some((mut producer, consumer)) = scope.next() else {
            panic!("registered producer missing");
        };
        let metadata = RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: 123,
            real_ns: 456,
            source_time: None,
            level: Level::Info,
            importance: None,
        };
        let Ok(RecordOutcome::Accepted(receipt)) = producer.try_record_small(metadata, b"example")
        else {
            panic!("example admission must fit");
        };
        let Ok(RecordOutcome::Accepted(second)) = producer.try_record_small(metadata, b"batch")
        else {
            panic!("second example admission must fit");
        };
        let mut router = Router::new(&plan, vec![consumer], routing)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = TempPath(std::env::temp_dir().join(format!(
            "dot-logger-consumer-{}-{stamp}.mcap",
            std::process::id()
        )));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&path.0)?;
        let mut writer = SegmentWriter::new(
            router.writer_budget(1)?,
            WriterConfig {
                max_loss_reports: 0,
                max_payload_bytes: 256,
                write_buffer_bytes: 4096,
                chunk_target_bytes: 1024,
                max_records: 4,
                max_chunks: 4,
                max_metadata_bytes: 1024,
            },
            vec![StorageChannel {
                channel: ChannelConfig::new(7, 99, 1),
                topic: "example".into(),
                payload_encoding: "application/octet-stream".into(),
                schema: None,
            }],
            file,
        )?;
        router.scope(
            |mut dispatch, mut outputs| -> Result<(), Box<dyn std::error::Error>> {
                let mut events = [None; 4];
                let report = dispatch.turn(now, &mut events)?;
                assert_eq!(report.records, 2);
                assert!(report.fault.is_none());
                let Some(Ok(delivery)) = outputs[0].try_next(now) else {
                    panic!("expected output guard");
                };
                assert_eq!(delivery.payload(), b"example");
                let Some(Ok(other)) = outputs[0].try_next(now) else {
                    panic!("expected second output guard");
                };
                writer.write_batch(&[&delivery, &other])?;
                assert_eq!(writer.index().len(), 1);
                assert_eq!(writer.index()[0].record_count, 2);
                drop((delivery, other));
                assert_eq!(
                    completion.poll(receipt, 1, Boundary::Synced)?.outcome,
                    QueryOutcome::Pending
                );
                let mut ranges = [ProducerRange::default(); 1];
                let frozen = completion.capture_scope(&mut ranges)?;
                let token =
                    control.request_sync(frozen, &[1], now, now + Duration::from_secs(1))?;
                let mut barrier = [ProducerRange::default(); 1];
                let io = control
                    .next_sync(token, 1, now, &mut dispatch, &mut barrier)?
                    .ok_or("barrier not ready")?;
                assert_eq!(barrier[0].last, second.accepted_seq);
                let mut finished = writer.finish()?;
                // This example ends on rejection; a live host can retry the
                // returned error.work on the correct segment before dropping it.
                finished.sync_work(io).map_err(|error| error.error)?;
                control.poll(now)?;
                assert_eq!(
                    completion.poll(receipt, 1, Boundary::Synced)?.outcome,
                    QueryOutcome::Complete
                );
                assert_eq!(
                    completion.poll(second, 1, Boundary::Synced)?.outcome,
                    QueryOutcome::Complete
                );
                // An independent read descriptor observes only the captured boundary.
                let query_budget = dot_logger::query::QueryBudget::new(&plan);
                let cancel = std::sync::atomic::AtomicBool::new(false);
                let mut snapshot = dot_logger::query::Snapshot::new(
                    &query_budget,
                    1,
                    vec![StorageChannel {
                        channel: ChannelConfig::new(7, 99, 1),
                        topic: "example".into(),
                        payload_encoding: "application/octet-stream".into(),
                        schema: None,
                    }],
                    vec![dot_logger::query::QuerySource::active(
                        std::fs::File::open(&path.0)?,
                        0,
                        1,
                        finished.safe_offset(),
                    )],
                    dot_logger::query::QueryConfig {
                        max_loss_reports: 0,
                        max_segments: 1,
                        max_records: 4,
                        max_record_bytes: 16384,
                        max_source_bytes: 65536,
                        max_payload_bytes: 256,
                        max_metadata_bytes: 1024,
                        producers: 1,
                        page_records: 4,
                        page_bytes: 4096,
                        inline_bytes: 256,
                        blob_read_bytes: 256,
                        read_bytes: 131072,
                    },
                    0,
                    100,
                    &cancel,
                )?;
                let cursor = snapshot.start(
                    dot_logger::query::Query {
                        basis: dot_logger::query::TimeBasis::Mono,
                        from: 0,
                        to: u64::MAX as i128 + 1,
                        channel: Some(7),
                        minimum_level: Level::Trace,
                    },
                    1,
                    &cancel,
                )?;
                let page = snapshot.page(&cursor, 1, &cancel)?;
                assert_eq!(page.records.len(), 2);
                assert_eq!(page.records[0].record.receipt, receipt);
                assert_eq!(page.inline(&page.records[0]), Some(b"example".as_slice()));
                // Replay owns the same actual file reader and bounded reservation.
                let mut replay = dot_logger::replay::Replay::new(
                    snapshot,
                    dot_logger::query::Query {
                        basis: dot_logger::query::TimeBasis::Mono,
                        from: metadata.mono_ns as i128,
                        to: metadata.mono_ns as i128 + 1,
                        channel: Some(7),
                        minimum_level: Level::Trace,
                    },
                    dot_logger::replay::Speed::new(1, 1)?,
                    1,
                    &cancel,
                )?;
                for (expected, payload) in [
                    (receipt, b"example".as_slice()),
                    (second, b"batch".as_slice()),
                ] {
                    match replay.poll(1, true, &cancel)? {
                        dot_logger::replay::ReplayPoll::Ready(frame) => {
                            assert_eq!(frame.record.receipt, expected);
                            match frame.payload {
                                dot_logger::replay::ReplayPayload::Inline(bytes) => {
                                    assert_eq!(bytes, payload)
                                }
                                _ => panic!("expected inline replay"),
                            }
                        }
                        _ => panic!("expected replay frame"),
                    }
                }
                assert!(matches!(
                    replay.poll(1, true, &cancel)?,
                    dot_logger::replay::ReplayPoll::End
                ));
                replay.stop();
                assert_eq!(query_budget.used_bytes(), 0);
                control.release_sync(token)?;
                let result = control.begin_shutdown(
                    ShutdownPlan { sync_files: false },
                    now,
                    now + Duration::from_secs(1),
                    &mut dispatch,
                )?;
                assert!(matches!(result,ShutdownPoll::Closed(report)if report.is_success()));
                assert_eq!(dispatch.snapshot().live_records, 0);
                Ok(())
            },
        )
    })?;
    report_contract();
    // The facade's file output needs the Linux free-space probe (RV-172).
    #[cfg(target_os = "linux")]
    facade_contract()?;
    #[cfg(feature = "remote")]
    remote_contract(&plan)?;
    println!(
        "tracked admission/routing/MCAP file sync/shutdown and facade passed; {} bytes reserved",
        plan.reserved_bytes()
    );
    Ok(())
}

#[cfg(feature = "remote")]
fn remote_contract(
    plan: &dot_logger::config::ValidatedConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use dot_logger::{
        adapters::remote::*,
        capture::{CaptureReceipt, CapturedRecord, Importance},
        query::QueryBudget,
    };
    let budget = QueryBudget::new(plan);
    let limits = RemoteLimits {
        connections: 1,
        subscriptions: 1,
        commands: 1,
        message_bytes: 4096,
        record_bytes: 256,
        subscription_records: 2,
        subscription_bytes: 512,
        max_age: Duration::from_secs(1),
        request_timeout: Duration::from_secs(1),
    };
    let (hub, inbox) = RemoteHub::new(&budget, limits)?;
    let pending_bind = RemoteServer::bind(ServerConfig::default(), hub.clone());
    drop(pending_bind);
    let mut receiver = Reassembler::new(&budget, 1, 256, Duration::from_secs(1))?;
    let record = CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: 1,
        },
        metadata: RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: 1,
            real_ns: 1,
            source_time: None,
            level: Level::Info,
            importance: Some(Importance::Normal),
        },
        revision: 1,
        payload_len: 3,
    };
    let ready = receiver
        .push(
            Chunk::single(1, 1, 1, record, b"ack", false),
            Instant::now(),
        )?
        .ok_or("incomplete receiver")?;
    let ack = ready
        .handoff(|r, data| r.receipt == record.receipt && data == b"ack")
        .map_err(|_| "receiver refused")?;
    assert_eq!(ack.receipt, record.receipt);
    drop((receiver, hub, inbox));
    assert_eq!(budget.used_bytes(), 0);
    Ok(())
}

/// TASK-027 facade (API §18): the default path an application uses, without
/// support modules, scopes or lifetimes. With `remote`, one more line serves
/// subscribers and status (§18.11).
#[cfg(target_os = "linux")]
fn facade_contract() -> Result<(), Box<dyn std::error::Error>> {
    use dot_logger::logger::{
        Budget, EncodeError, HealthState, Logger, Record, Retention, Schema, Shutdown, WriterQuota,
    };
    struct Pose(i32, i32);
    impl Record for Pose {
        const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-i32x2");
        fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
            let bytes = out.get_mut(..8).ok_or(EncodeError)?;
            bytes[..4].copy_from_slice(&self.0.to_le_bytes());
            bytes[4..].copy_from_slice(&self.1.to_le_bytes());
            Ok(8)
        }
    }
    let dir = std::env::temp_dir().join(format!("dot-logger-api-consumer-{}", std::process::id()));
    let _cleanup = Cleanup(dir.clone());
    std::fs::create_dir_all(&dir)?;
    let builder = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 << 20)
                .sync_every(Duration::from_millis(50))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(64, 64)),
        )
        .file_output(&dir, Retention::ring(64 << 20))
        .channel::<Pose>("robot/pose")
        .writer("control");
    #[cfg(feature = "remote")]
    let builder = builder.remote(dot_logger::adapters::remote::ServerConfig {
        enabled: true,
        ..Default::default()
    });
    let logger = builder.start()?;
    #[cfg(feature = "remote")]
    assert!(logger.remote_addr().is_some_and(|a| a.ip().is_loopback()));
    let pose = logger.channel::<Pose>("robot/pose")?;
    let mut writer = logger.take_writer("control")?;
    std::thread::spawn(move || {
        for i in 0..10 {
            while writer.try_log(pose, &Pose(i, -i)).is_err() {
                std::thread::yield_now();
            }
        }
    })
    .join()
    .map_err(|_| "writer thread panicked")?;
    assert_eq!(logger.health().state, HealthState::Running);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(5))? else {
        return Err("facade shutdown did not close".into());
    };
    assert!(report.is_success(), "{report:?}");
    assert_eq!(report.core.required.complete, 10);
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

// Keep the report facade usable without network/runtime/compression features.
fn report_contract() {
    use dot_logger::{
        completion::Boundary,
        telemetry::{CaptureCounts, Window, capacity::*, report::*},
    };
    let window = Window {
        session_id: 42,
        revision: 1,
        epoch: 1,
        from_ns: 0,
        to_ns: 10_000_000_000,
        counts: CaptureCounts::default(),
    };
    let input = ReportInput::from_window(
        window,
        Conditions {
            environment_id: 1,
            workload_id: 2,
            revision: 1,
            target_id: 10,
            boundary: Boundary::Written,
        },
    );
    let r = Report::build(input, None, window.to_ns);
    use dot_logger::telemetry::compare::{Comparison, Snapshot};
    let mut next = input;
    next.window.revision = 2;
    next.conditions.revision = 2;
    next.window.from_ns = 10_000_000_000;
    next.window.to_ns = 20_000_000_000;
    let comparison = Comparison::build(
        Snapshot::new(r, 99),
        Snapshot::new(Report::build(next, None, 20_000_000_000), 99),
    );
    assert!(comparison.differences.comparable());
    assert_eq!(r.rejected_capacity, Value::Observed(0));
    assert_eq!(r.rejected_contract, Value::Observed(0));
    assert_eq!(r.rejected_lifecycle, Value::Observed(0));
    assert_eq!(r.acquire_rejected_contract, Value::Observed(0));
    assert_eq!(r.capacity, Value::Unavailable(Reason::MissingCapacity));
    assert_eq!(
        r.disk_exhaustion,
        Value::Unavailable(Reason::MissingMeasurement)
    );
}

/// Removes a temporary path even when the test panics (REV-071 RV-188).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0).or_else(|_| std::fs::remove_file(&self.0));
    }
}
