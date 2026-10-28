//! Paused-publication tests for states that public operations do not expose.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::{
    capture::{CaptureOwner, ChannelConfig, Level, RecordMetadata},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    routing::{Route, Router, Schedule, TargetConfig},
};
use std::time::{Duration, Instant};
fn setup() -> (CaptureOwner, crate::config::ValidatedConfig, RoutingConfig) {
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 128 * 1024,
            raw_bytes: 0,
            output_bytes: 256 * 1024,
            external_bytes: 0,
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
    .validate(&PlatformCapabilities::default())
    .unwrap();
    let routing = RoutingConfig {
        targets: vec![TargetConfig {
            id: 10,
            required: true,
            queue_slots: 4,
            pending_slots: 1,
            retained_bytes: 128,
            max_attempts: 3,
            max_age: Duration::from_secs(1),
        }],
        routes: vec![Route {
            channel_id: 7,
            targets: vec![10],
        }],
        shared_records: 4,
        small_copy_bytes: 64,
        schedule: Schedule {
            record_quantum: 1,
            byte_quantum: 64,
            records_per_turn: 1,
            bytes_per_turn: 64,
            retries_per_target: 1,
            reclaim_slots_per_producer: 1,
        },
    };
    let config = CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: 2,
        max_queries: 1,
        page_items: 2,
        feedback_events: 1,
        updates_per_turn: 1,
        reconcile_per_turn: 1,
        targets: vec![TargetContract {
            id: 10,
            required: true,
            supported: Boundaries::FILE,
            completion: Boundary::Written,
        }],
    };
    let owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        config,
    )
    .unwrap();
    (owner, plan, routing)
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
#[test]
fn busy_report_publication_preserves_feedback_and_backpressures_without_losing_an_event() {
    let (mut owner, plan, routing) = setup();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut p, c) = scope.next().unwrap();
        p.try_record_small(metadata(), &[1]).unwrap();
        p.try_record_small(metadata(), &[2]).unwrap();
        let slot = &completion.registry.producers[0].slots[1];
        slot.phase.store(UPDATING, Ordering::Release);
        let mut router = Router::new(&plan, vec![c], routing).unwrap();
        router.scope(|mut d, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 4];
            let report = d.turn(now, &mut events).unwrap();
            assert_eq!(report.records, 1);
            assert_eq!(d.drain_state().feedback, 1);
            assert_eq!(d.turn(now, &mut events).unwrap().records, 0);
            let first = outputs[0].try_next(now).unwrap().unwrap();
            assert_eq!(
                first.report(Update::Confirmed(Boundary::Written)),
                Err(CompletionError::Busy)
            );
            slot.phase.store(ACTIVE, Ordering::Release);
            assert_eq!(d.turn(now, &mut events).unwrap().records, 1);
            first.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(first);
            let second = outputs[0].try_next(now).unwrap().unwrap();
            second.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(second);
            assert_eq!(completion.totals(false).complete, 2);
            assert_eq!(d.drain_state().feedback, 0);
        });
    });
}
#[test]
fn a_stale_drop_marker_cannot_overwrite_the_new_generations_unknown_outcome() {
    let (mut owner, _, _) = setup();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let crate::capture::RecordOutcome::Accepted(first) =
            p.try_record_small(metadata(), &[1]).unwrap()
        else {
            panic!()
        };
        reader.try_read(&mut [0; 16]).unwrap();
        let old = c.marker(first, 10).unwrap();
        c.report(first, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        let crate::capture::RecordOutcome::Accepted(second) =
            p.try_record_small(metadata(), &[2]).unwrap()
        else {
            panic!()
        };
        reader.try_read(&mut [0; 16]).unwrap();
        c.report(second, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        let crate::capture::RecordOutcome::Accepted(third) =
            p.try_record_small(metadata(), &[3]).unwrap()
        else {
            panic!()
        };
        let new = c.marker(third, 10).unwrap();
        drop(new);
        drop(old);
        assert_eq!(
            c.poll(third, 10, Boundary::Written).unwrap().outcome,
            QueryOutcome::Unknown
        );
        c.reconcile(2);
        assert_eq!(c.totals(false).unknown, 1);
    });
}
