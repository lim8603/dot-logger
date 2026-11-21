//! A loopback gRPC host with an advanced-path `OperationsHost` for the viewer
//! dashboard: GetStatus and Compare answer from two held snapshots. PlanCheck
//! is not served here, so the dashboard shows it as unsupported.
//!
//! ```sh
//! cargo run --locked -p dot-logger-viewer --example operations_host
//! ```
//!
//! It prints the upstream URL and serves for ten minutes or until stopped.
#![allow(clippy::unwrap_used)]

use dot_logger::{
    adapters::remote::{RemoteHub, RemoteLimits, RemoteServer, ServerConfig},
    capture::{CaptureOwner, ChannelConfig, Level, RecordMetadata, RecordOutcome},
    completion::{Boundaries, Boundary, CompletionConfig, TargetContract},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    query::QueryBudget,
    routing::{Route, RoutingConfig, Schedule, TargetConfig},
    telemetry::{
        Observation, Window,
        capacity::Conditions,
        compare::Snapshot,
        operations::OperationsHost,
        report::{Report, ReportInput},
    },
};
use std::time::{Duration, Instant};

const SECOND: u64 = 1_000_000_000;
const SERVE_FOR: Duration = Duration::from_secs(600);
const MIB: usize = 1024 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 8 * MIB,
            input_bytes: 128 * 1024,
            raw_bytes: 0,
            output_bytes: 256 * 1024,
            external_bytes: 4 * MIB,
            control_bytes: 256 * 1024,
            worker_bytes: 256 * 1024,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 16,
                important_slots: 0,
            },
            raw: None,
        }],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    let budget = QueryBudget::new(&plan);
    let (hub, mut inbox) = RemoteHub::new(
        &budget,
        RemoteLimits {
            connections: 4,
            subscriptions: 1,
            commands: 2,
            message_bytes: 64 * 1024,
            record_bytes: 1024,
            subscription_records: 1,
            subscription_bytes: 1024,
            max_age: Duration::from_secs(10),
            request_timeout: Duration::from_secs(5),
        },
    )?;
    let server = runtime.block_on(RemoteServer::bind(
        ServerConfig {
            enabled: true,
            ..ServerConfig::default()
        },
        hub,
    ))?;
    println!("http://{}", server.local_addr()?);
    let (shutdown, done) = tokio::sync::oneshot::channel::<()>();
    let served = runtime.spawn(server.serve(async {
        let _ = done.await;
    }));

    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(),
    )?;
    owner.enable_config_updates(&routing())?;
    owner.enable_telemetry()?;
    owner.scope(|mut scope| {
        let telemetry = scope.telemetry().unwrap();
        let updates = scope.config_updates();
        let mut operations = OperationsHost::new(4).unwrap();
        let (mut producer, _) = scope.next().unwrap();
        // Four records in the first ten-second window, two in the second.
        for at in 0..=20 {
            if (1..=4).contains(&at) || at == 12 || at == 16 {
                assert!(matches!(
                    producer.try_record_small(metadata(), &[1]).unwrap(),
                    RecordOutcome::Accepted(_)
                ));
            }
            telemetry.sample(at * SECOND, None).unwrap();
            if at % 10 == 0 && at > 0 {
                let Observation::Observed(window) = telemetry.window(10) else {
                    panic!("a stable ten-second window")
                };
                operations.register_snapshot(snapshot(window)).unwrap();
            }
        }
        let started = Instant::now();
        while started.elapsed() < SERVE_FOR {
            if let Some(command) = inbox.try_next() {
                operations.handle_command(command, telemetry, updates, 20 * SECOND);
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    });
    let _ = shutdown.send(());
    runtime.block_on(served)??;
    Ok(())
}

fn snapshot(window: Window) -> Snapshot {
    let input = ReportInput::from_window(
        window,
        Conditions {
            environment_id: 1,
            workload_id: 1,
            revision: window.revision,
            target_id: 10,
            boundary: Boundary::Written,
        },
    );
    Snapshot::new(Report::build(input, None, window.to_ns), 1)
}

fn metadata() -> RecordMetadata {
    RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 1,
        real_ns: 2,
        source_time: None,
        level: Level::Info,
        importance: None,
    }
}

fn routing() -> RoutingConfig {
    RoutingConfig {
        targets: vec![TargetConfig {
            id: 10,
            required: true,
            queue_slots: 4,
            pending_slots: 2,
            retained_bytes: 256,
            max_attempts: 3,
            max_age: Duration::from_secs(1),
        }],
        routes: vec![Route {
            channel_id: 7,
            targets: vec![10],
        }],
        shared_records: 8,
        small_copy_bytes: 128,
        schedule: Schedule {
            record_quantum: 2,
            byte_quantum: 64,
            records_per_turn: 4,
            bytes_per_turn: 128,
            retries_per_target: 2,
            reclaim_slots_per_producer: 2,
        },
    }
}

fn tracking() -> CompletionConfig {
    CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: 8,
        max_queries: 2,
        page_items: 3,
        feedback_events: 8,
        updates_per_turn: 8,
        reconcile_per_turn: 4,
        targets: vec![TargetContract {
            id: 10,
            required: true,
            supported: Boundaries::FILE,
            completion: Boundary::Written,
        }],
    }
}
