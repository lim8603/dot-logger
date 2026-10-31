#![allow(dead_code)]
use dot_logger::capture::{Level, RecordMetadata, RecordOutcome};
use dot_logger::completion::{Boundaries, Boundary, CompletionConfig, TargetContract};
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    ValidatedConfig, WorkerThreadingConfig,
};
use dot_logger::platform::{PlatformCapabilities, Requirement};
use dot_logger::routing::{Route, RoutingConfig, Schedule, TargetConfig};
use std::time::Duration;
pub fn plan() -> ValidatedConfig {
    SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 128 * 1024,
            raw_bytes: 128 * 1024,
            output_bytes: 256 * 1024,
            external_bytes: 0,
            control_bytes: 256 * 1024,
            worker_bytes: 256 * 1024,
        },
        producers: vec![
            ProducerQuota {
                small: PoolQuota {
                    slots: 8,
                    slot_bytes: 16,
                    important_slots: 0
                },
                raw: Some(PoolQuota {
                    slots: 2,
                    slot_bytes: 64,
                    important_slots: 0
                })
            };
            2
        ],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())
    .unwrap()
}
pub fn routing() -> RoutingConfig {
    RoutingConfig {
        targets: vec![10, 20]
            .into_iter()
            .map(|id| TargetConfig {
                id,
                required: id == 10,
                queue_slots: 4,
                pending_slots: 2,
                retained_bytes: 256,
                max_attempts: 3,
                max_age: Duration::from_secs(1),
            })
            .collect(),
        routes: vec![Route {
            channel_id: 7,
            targets: vec![10, 20],
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
pub fn tracking(window: usize) -> CompletionConfig {
    CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: window,
        max_queries: 2,
        page_items: 3,
        feedback_events: 8,
        updates_per_turn: 8,
        reconcile_per_turn: 4,
        targets: vec![10, 20]
            .into_iter()
            .map(|id| TargetContract {
                id,
                required: id == 10,
                supported: Boundaries::FILE,
                completion: Boundary::Written,
            })
            .collect(),
    }
}
pub fn meta() -> RecordMetadata {
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
pub fn accepted(result: RecordOutcome) -> dot_logger::capture::CaptureReceipt {
    let RecordOutcome::Accepted(r) = result else {
        panic!("expected acceptance")
    };
    r
}
