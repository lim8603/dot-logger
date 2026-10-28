//! Independent application: tracked admission, routing, writer acknowledgments, sync, and shutdown.

use dot_logger::capture::{CaptureOwner, ChannelConfig, Level, RecordMetadata, RecordOutcome};
use dot_logger::completion::{
    Boundaries, Boundary, CompletionConfig, ProducerRange, QueryOutcome, TargetContract, Update,
};
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    WorkerThreadingConfig,
};
use dot_logger::control::{ControlConfig, ControlOwner, ShutdownPlan, ShutdownPoll, SyncResult};
use dot_logger::platform::{PlatformCapabilities, Requirement};
use dot_logger::routing::{Route, Router, RoutingConfig, Schedule, TargetConfig};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const MIB: usize = 1024 * 1024;
    let config = SessionConfig {
        memory: MemoryBudget {
            total_bytes: 8 * MIB,
            input_bytes: MIB,
            raw_bytes: 0,
            output_bytes: 2 * MIB,
            external_bytes: 0,
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
    assert_eq!(plan.reserved_bytes(), 8 * MIB);
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
    // Allocate/register before RT use. Writer acknowledgments below are a test
    // adapter; this example does not create a file or prove disk durability.
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
        let mut router = Router::new(&plan, vec![consumer], routing)?;
        router.scope(
            |mut dispatch, mut outputs| -> Result<(), Box<dyn std::error::Error>> {
                let mut events = [None; 4];
                let report = dispatch.turn(now, &mut events)?;
                assert_eq!(report.records, 1);
                assert!(report.fault.is_none());
                let Some(Ok(delivery)) = outputs[0].try_next(now) else {
                    panic!("expected output guard");
                };
                assert_eq!(delivery.payload(), b"example");
                delivery.report(Update::Confirmed(Boundary::Written))?;
                drop(delivery);
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
                assert_eq!(barrier[0].last, receipt.accepted_seq);
                io.complete(SyncResult::Synced);
                control.poll(now)?;
                assert_eq!(
                    completion.poll(receipt, 1, Boundary::Synced)?.outcome,
                    QueryOutcome::Complete
                );
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
    println!(
        "tracked admission/routing/sync/shutdown passed; {} bytes reserved",
        plan.reserved_bytes()
    );
    Ok(())
}
