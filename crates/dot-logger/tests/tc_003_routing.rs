//! TC-003 routing subset: bounded fanout, isolation, fairness, and ownership.
#![allow(clippy::unwrap_used)]

use dot_logger::capture::{CaptureOwner, ChannelConfig, Importance, Level, RecordMetadata};
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    ValidatedConfig, WorkerThreadingConfig,
};
use dot_logger::platform::{PlatformCapabilities, Requirement};
use dot_logger::pool::RawRecordOutcome;
use dot_logger::routing::{
    DispatchOutcome, DropReason, Route, Router, RoutingConfig, Schedule, TargetConfig,
};
use std::time::{Duration, Instant};

fn plan() -> ValidatedConfig {
    SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 128 * 1024,
            raw_bytes: 128 * 1024,
            output_bytes: 512 * 1024,
            external_bytes: 0,
            control_bytes: 128 * 1024,
            worker_bytes: 128 * 1024,
        },
        producers: vec![
            ProducerQuota {
                small: PoolQuota {
                    slots: 16,
                    slot_bytes: 16,
                    important_slots: 0
                },
                raw: Some(PoolQuota {
                    slots: 8,
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
fn config() -> RoutingConfig {
    RoutingConfig {
        targets: vec![target(10, true), target(20, false)],
        routes: vec![Route {
            channel_id: 7,
            targets: vec![10, 20],
        }],
        shared_records: 16,
        small_copy_bytes: 128,
        schedule: Schedule {
            record_quantum: 2,
            byte_quantum: 16,
            records_per_turn: 8,
            bytes_per_turn: 128,
            retries_per_target: 2,
            reclaim_slots_per_producer: 2,
        },
    }
}
fn target(id: u32, required: bool) -> TargetConfig {
    TargetConfig {
        id,
        required,
        queue_slots: 1,
        pending_slots: 2,
        retained_bytes: 192,
        max_attempts: 3,
        max_age: Duration::from_secs(1),
    }
}
fn meta() -> RecordMetadata {
    RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 0,
        real_ns: 0,
        source_time: None,
        level: Level::Info,
        importance: None,
    }
}

#[test]
fn stopped_required_a_does_not_stop_b_and_retry_exhaustion_is_explicit() {
    let plan = plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|mut scope| {
        let (mut producer, consumer) = scope.next().unwrap();
        let (_, second) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![consumer, second], config()).unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 32];
            for i in 0..3 {
                producer.try_record_small(meta(), &[i]).unwrap();
                let report = dispatch.turn(now, &mut events).unwrap();
                assert_eq!(report.records, 1);
                let delivered = outputs[1].try_next(now).unwrap().unwrap();
                assert_eq!(delivered.payload(), &[i]);
                drop(delivered);
            }
            assert_eq!(dispatch.target_snapshot(10).unwrap().queued, 1);
            let mut losses = vec![];
            for _ in 0..4 {
                let report = dispatch.turn(now, &mut events).unwrap();
                losses.extend(events[..report.events].iter().flatten().copied());
            }
            assert_eq!(
                losses
                    .iter()
                    .filter(|event| event.target_id == Some(10)
                        && event.outcome == DispatchOutcome::Dropped(DropReason::RetryExhausted))
                    .count(),
                2
            );
            assert!(losses.iter().all(|event| event.required));
            assert_eq!(outputs[0].try_next(now).unwrap().unwrap().payload(), &[0]);
            assert_eq!(dispatch.snapshot().live_records, 0);
        });
    });
}

fn with_router<R>(
    config: RoutingConfig,
    run: impl for<'a> FnOnce(
        dot_logger::capture::Producer<'a>,
        dot_logger::capture::Producer<'a>,
        Router<'a>,
    ) -> R,
) -> R {
    let plan = plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|mut scope| {
        let (first, a) = scope.next().unwrap();
        let (second, b) = scope.next().unwrap();
        run(
            first,
            second,
            Router::new(&plan, vec![a, b], config).unwrap(),
        )
    })
}

#[test]
fn original_is_shared_once_and_last_io_guard_controls_reclamation() {
    with_router(config(), |mut producer, _, mut router| {
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut loan = producer.try_acquire(64, Importance::Normal).unwrap();
            loan.payload_mut().fill(73);
            let address = loan.payload().as_ptr();
            assert!(matches!(
                producer.try_record_raw(meta(), loan).unwrap(),
                RawRecordOutcome::Accepted(_)
            ));
            let mut events = [None; 8];
            // 64-byte FIFO head accumulates four 16-byte deficits without copying.
            for _ in 0..4 {
                dispatch.turn(now, &mut events).unwrap();
            }
            assert_eq!(dispatch.snapshot().live_records, 1);
            assert_eq!(dispatch.snapshot().raw_retained_bytes, 64);
            assert_eq!(dispatch.snapshot().raw_reserved_bytes, 2 * 8 * 64);
            assert_eq!(dispatch.snapshot().small_copy_bytes, 0);
            assert_eq!(dispatch.target_snapshot(10).unwrap().retained_bytes, 64);
            assert_eq!(dispatch.target_snapshot(20).unwrap().retained_bytes, 64);
            let first = outputs[0].try_next(now).unwrap().unwrap();
            let second = outputs[1].try_next(now).unwrap().unwrap();
            assert_eq!(first.payload().as_ptr(), address);
            assert_eq!(second.payload().as_ptr(), address);
            assert_eq!(dispatch.target_snapshot(10).unwrap().in_flight, 1);
            assert!(dispatch.cancel_target(10));
            assert!(dispatch.cancel_target(20));
            drop(first);
            assert_eq!(dispatch.snapshot().raw_retained_bytes, 64);
            assert_eq!(dispatch.turn(now, &mut events).unwrap().reclaimed, 0);
            assert_eq!(second.payload(), &[73; 64]);
            drop(second);
            assert_eq!(dispatch.snapshot().live_records, 0);
            let reclaimed: usize = (0..4)
                .map(|_| dispatch.turn(now, &mut events).unwrap().reclaimed)
                .sum();
            assert_eq!(reclaimed, 1);
        });
    });
}

#[test]
fn retries_resume_in_fifo_order_without_new_records_overtaking() {
    let mut cfg = config();
    cfg.targets[0].max_attempts = 10;
    cfg.targets[1].queue_slots = 8;
    with_router(cfg, |mut producer, _, mut router| {
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 32];
            for n in 1..=3 {
                producer.try_record_small(meta(), &[n]).unwrap();
            }
            dispatch.turn(now, &mut events).unwrap();
            dispatch.turn(now, &mut events).unwrap();
            assert_eq!(dispatch.target_snapshot(10).unwrap().pending, 2);
            assert_eq!(outputs[0].try_next(now).unwrap().unwrap().payload(), &[1]);
            producer.try_record_small(meta(), &[4]).unwrap();
            dispatch.turn(now, &mut events).unwrap();
            assert_eq!(outputs[0].try_next(now).unwrap().unwrap().payload(), &[2]);
            dispatch.turn(now, &mut events).unwrap();
            assert_eq!(outputs[0].try_next(now).unwrap().unwrap().payload(), &[3]);
            dispatch.turn(now, &mut events).unwrap();
            assert_eq!(outputs[0].try_next(now).unwrap().unwrap().payload(), &[4]);
        });
    });
}

#[test]
fn short_event_buffer_does_not_consume_ingress_or_lose_retry_outcomes() {
    with_router(config(), |mut producer, _, mut router| {
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            producer.try_record_small(meta(), &[1]).unwrap();
            let report = dispatch.turn(now, &mut [None; 1]).unwrap();
            assert_eq!(report.records, 0);
            assert!(report.backpressured);
            assert_eq!(dispatch.snapshot().live_records, 0);
            let mut events = [None; 2];
            assert_eq!(dispatch.turn(now, &mut events).unwrap().events, 2);
            producer.try_record_small(meta(), &[2]).unwrap();
            dispatch.turn(now, &mut events).unwrap();
            assert_eq!(dispatch.target_snapshot(10).unwrap().pending, 1);
            for _ in 0..10 {
                dispatch.turn(now, &mut []).unwrap();
            }
            assert_eq!(dispatch.target_snapshot(10).unwrap().pending, 1);
            drop(outputs[0].try_next(now).unwrap().unwrap());
            let report = dispatch.turn(now, &mut events).unwrap();
            assert!(
                events[..report.events]
                    .iter()
                    .flatten()
                    .any(|e| e.target_id == Some(10) && e.outcome == DispatchOutcome::Enqueued)
            );
            assert_eq!(outputs[0].try_next(now).unwrap().unwrap().payload(), &[2]);
        });
    });
}

#[test]
fn target_byte_quota_includes_active_io_and_other_output_continues() {
    let mut cfg = config();
    cfg.targets[0].queue_slots = 4;
    cfg.targets[0].pending_slots = 0;
    cfg.targets[0].retained_bytes = 2;
    with_router(cfg, |mut producer, _, mut router| {
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 8];
            producer.try_record_small(meta(), &[1, 2]).unwrap();
            dispatch.turn(now, &mut events).unwrap();
            let held = outputs[0].try_next(now).unwrap().unwrap();
            drop(outputs[1].try_next(now).unwrap().unwrap());
            producer.try_record_small(meta(), &[3]).unwrap();
            let report = dispatch.turn(now, &mut events).unwrap();
            assert!(
                events[..report.events]
                    .iter()
                    .flatten()
                    .any(|e| e.target_id == Some(10)
                        && e.outcome == DispatchOutcome::Dropped(DropReason::Capacity))
            );
            assert_eq!(dispatch.target_snapshot(10).unwrap().retained_bytes, 2);
            assert_eq!(outputs[1].try_next(now).unwrap().unwrap().payload(), &[3]);
            drop(held);
            producer.try_record_small(meta(), &[4, 5]).unwrap();
            dispatch.turn(now, &mut events).unwrap();
            assert_eq!(
                outputs[0].try_next(now).unwrap().unwrap().payload(),
                &[4, 5]
            );
        });
    });
}

#[test]
fn shared_small_copies_are_counted_once_and_block_until_last_reference_ends() {
    let mut cfg = config();
    cfg.small_copy_bytes = 16;
    cfg.targets.iter_mut().for_each(|t| t.queue_slots = 4);
    with_router(cfg, |mut producer, _, mut router| {
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 8];
            producer.try_record_small(meta(), &[1; 16]).unwrap();
            dispatch.turn(now, &mut events).unwrap();
            let a = outputs[0].try_next(now).unwrap().unwrap();
            let b = outputs[1].try_next(now).unwrap().unwrap();
            assert_eq!(a.payload().as_ptr(), b.payload().as_ptr());
            assert_eq!(dispatch.snapshot().small_copy_bytes, 16);
            producer.try_record_small(meta(), &[2]).unwrap();
            assert_eq!(dispatch.turn(now, &mut events).unwrap().records, 0);
            drop(a);
            assert_eq!(dispatch.snapshot().small_copy_bytes, 16);
            assert_eq!(dispatch.turn(now, &mut events).unwrap().records, 0);
            drop(b);
            assert_eq!(dispatch.snapshot().small_copy_bytes, 0);
            assert_eq!(dispatch.turn(now, &mut events).unwrap().records, 1);
        });
    });
}

#[test]
fn shared_record_limit_includes_dequeued_guards_and_survives_scope_reentry() {
    let mut cfg = config();
    cfg.shared_records = 1;
    with_router(cfg, |mut producer, _, mut router| {
        let now = Instant::now();
        let mut events = [None; 8];
        producer.try_record_small(meta(), &[1]).unwrap();
        let held = router.scope(|mut dispatch, mut outputs| {
            dispatch.turn(now, &mut events).unwrap();
            let held = outputs[0].try_next(now).unwrap().unwrap();
            drop(outputs[1].try_next(now).unwrap().unwrap());
            held
        });
        producer.try_record_small(meta(), &[2]).unwrap();
        router.scope(|mut dispatch, _| {
            assert_eq!(dispatch.turn(now, &mut events).unwrap().records, 0);
            drop(held);
            assert_eq!(dispatch.turn(now, &mut events).unwrap().records, 1);
        });
    });
}

#[test]
fn deficit_round_robin_preserves_mixed_fifo_and_gives_large_heads_progress() {
    let mut cfg = config();
    cfg.schedule.records_per_turn = 2;
    cfg.schedule.bytes_per_turn = 64;
    cfg.targets.iter_mut().for_each(|t| {
        t.queue_slots = 32;
        t.retained_bytes = 2048;
    });
    with_router(cfg, |mut first, mut second, mut router| {
        let loan = first.try_acquire(64, Importance::Normal).unwrap();
        first.try_record_raw(meta(), loan).unwrap();
        first.try_record_small(meta(), &[9]).unwrap();
        for _ in 0..8 {
            second.try_record_small(meta(), &[1]).unwrap();
        }
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 32];
            let mut receipts = vec![];
            for turn in 0..8 {
                let report = dispatch.turn(now, &mut events).unwrap();
                assert!(report.records <= 2 && report.bytes <= 64);
                while let Some(item) = outputs[0].try_next(now) {
                    receipts.push(item.unwrap().record().receipt);
                }
                while let Some(item) = outputs[1].try_next(now) {
                    drop(item.unwrap());
                }
                if turn == 0 {
                    assert!(receipts.iter().all(|r| r.producer_id == 1));
                    assert!(!receipts.is_empty());
                }
            }
            let first: Vec<_> = receipts
                .iter()
                .filter(|r| r.producer_id == 0)
                .map(|r| r.accepted_seq)
                .collect();
            let second: Vec<_> = receipts
                .iter()
                .filter(|r| r.producer_id == 1)
                .map(|r| r.accepted_seq)
                .collect();
            assert_eq!(first, vec![1, 2]);
            assert_eq!(second, (1..=8).collect::<Vec<_>>());
        });
    });
}

#[test]
fn queued_and_pending_expiry_are_explicit_but_active_io_remains_valid() {
    with_router(config(), |mut producer, _, mut router| {
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let late = now + Duration::from_secs(1);
            let mut events = [None; 16];
            for n in 0..2 {
                producer.try_record_small(meta(), &[n]).unwrap();
            }
            dispatch.turn(now, &mut events).unwrap();
            let active = outputs[1].try_next(now).unwrap().unwrap();
            let report = dispatch.turn(late, &mut events).unwrap();
            assert_eq!(
                events[..report.events]
                    .iter()
                    .flatten()
                    .filter(|e| e.outcome == DispatchOutcome::Dropped(DropReason::Expired))
                    .count(),
                2
            );
            let event = outputs[0].try_next(late).unwrap().err().unwrap();
            assert_eq!(event.outcome, DispatchOutcome::Dropped(DropReason::Expired));
            assert_eq!(active.payload(), &[0]);
            drop(active);
            assert_eq!(dispatch.snapshot().live_records, 0);
        });
    });
}

#[test]
fn target_cancellation_reports_pending_queued_and_future_drops() {
    with_router(config(), |mut producer, _, mut router| {
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 16];
            for n in 0..2 {
                producer.try_record_small(meta(), &[n]).unwrap();
            }
            dispatch.turn(now, &mut events).unwrap();
            assert!(dispatch.cancel_target(10));
            assert!(!dispatch.cancel_target(99));
            let report = dispatch.turn(now, &mut events).unwrap();
            assert!(
                events[..report.events]
                    .iter()
                    .flatten()
                    .any(|e| e.target_id == Some(10)
                        && e.outcome == DispatchOutcome::Dropped(DropReason::Cancelled))
            );
            assert_eq!(
                outputs[0].try_next(now).unwrap().err().unwrap().outcome,
                DispatchOutcome::Dropped(DropReason::Cancelled)
            );
            producer.try_record_small(meta(), &[3]).unwrap();
            let report = dispatch.turn(now, &mut events).unwrap();
            assert!(
                events[..report.events]
                    .iter()
                    .flatten()
                    .any(|e| e.target_id == Some(10)
                        && e.outcome == DispatchOutcome::Dropped(DropReason::Cancelled))
            );
            assert_eq!(dispatch.target_snapshot(10).unwrap().retained_records, 0);
        });
    });
}

#[test]
fn close_leaves_accepted_ingress_available_for_later_management() {
    let plan = plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|mut scope| {
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        p.try_record_small(meta(), &[7]).unwrap();
        let mut router = Router::new(&plan, vec![a, b], config()).unwrap();
        router.scope(|mut dispatch, _| {
            dispatch.close();
            assert_eq!(
                dispatch
                    .turn(Instant::now(), &mut [None; 8])
                    .unwrap()
                    .records,
                0
            );
        });
    });
    owner.scope(|mut scope| {
        let (_, mut c) = scope.next().unwrap();
        let mut bytes = [0; 16];
        assert_eq!(c.peek().unwrap().unwrap().payload_len, 1);
        assert!(c.try_read(&mut bytes).unwrap().is_some());
        assert_eq!(bytes[0], 7);
    });
}

#[test]
fn empty_route_reports_intentional_omission_without_retaining_payload() {
    let mut cfg = config();
    cfg.routes[0].targets.clear();
    with_router(cfg, |mut producer, _, mut router| {
        producer.try_record_small(meta(), &[1]).unwrap();
        router.scope(|mut dispatch, _| {
            let mut events = [None; 1];
            let report = dispatch.turn(Instant::now(), &mut events).unwrap();
            assert_eq!(report.events, 1);
            assert_eq!(events[0].unwrap().outcome, DispatchOutcome::Unrouted);
            assert_eq!(events[0].unwrap().target_id, None);
            assert_eq!(dispatch.snapshot().live_records, 0);
        });
    });
}

#[test]
fn zero_length_records_still_obey_record_work_limits_and_clock_never_regresses() {
    let mut cfg = config();
    cfg.schedule.records_per_turn = 1;
    with_router(cfg, |mut producer, _, mut router| {
        for _ in 0..3 {
            producer.try_record_small(meta(), &[]).unwrap();
        }
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 8];
            for _ in 0..3 {
                let r = dispatch.turn(now, &mut events).unwrap();
                assert_eq!(r.records, 1);
                assert_eq!(r.bytes, 0);
                for output in &mut outputs {
                    drop(output.try_next(now).unwrap().unwrap());
                }
            }
            assert!(
                dispatch
                    .turn(now + Duration::from_secs(1), &mut events)
                    .is_ok()
            );
            assert_eq!(
                dispatch.turn(now, &mut events),
                Err(dot_logger::routing::RoutingError::ClockWentBackwards)
            );
        });
    });
}

#[test]
fn startup_rejects_unusable_small_copy_budget_before_dispatch_can_deadlock() {
    let plan = plan();
    let mut cfg = config();
    cfg.small_copy_bytes = 15;
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        assert!(matches!(
            Router::new(&plan, scope.map(|(_, c)| c).collect(), cfg),
            Err(dot_logger::routing::RoutingError::InvalidConfig)
        ));
    });
}

#[test]
fn output_reservation_accepts_exact_fit_rejects_one_byte_short_and_overflow() {
    use dot_logger::config::{ConfigError, Resource};
    use dot_logger::routing::RoutingError;
    let required = with_router(config(), |_, _, mut router| {
        router.scope(|dispatch, _| dispatch.snapshot().output_reserved_bytes)
    });
    for (available, pass) in [(required, true), (required - 1, false)] {
        let mut cfg = plan().config().clone();
        cfg.memory.output_bytes = available;
        let plan = SessionBuilder::new(cfg)
            .validate(&PlatformCapabilities::default())
            .unwrap();
        let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        owner.scope(|scope| {
            let result=Router::new(&plan,scope.map(|(_,c)| c).collect(),config());
            if pass { assert!(result.is_ok()); }
            else { assert!(matches!(result,Err(RoutingError::Config(ConfigError::BudgetExceeded {resource:Resource::Output,required:r,available:a})) if r==required && a==available)); }
        });
    }
    let plan = plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let mut cfg = config();
        cfg.shared_records = usize::MAX;
        assert!(matches!(
            Router::new(&plan, scope.map(|(_, c)| c).collect(), cfg),
            Err(RoutingError::Config(ConfigError::ArithmeticOverflow))
        ));
    });
}

#[test]
fn invalid_targets_routes_work_limits_and_misordered_inputs_are_rejected() {
    use dot_logger::routing::RoutingError;
    let mut invalid = vec![];
    let mut cfg = config();
    cfg.targets[1].id = 10;
    invalid.push(cfg);
    let mut cfg = config();
    cfg.targets[0].max_age = Duration::ZERO;
    invalid.push(cfg);
    let mut cfg = config();
    cfg.targets[0].max_attempts = 0;
    invalid.push(cfg);
    let mut cfg = config();
    cfg.targets[0].queue_slots = 0;
    invalid.push(cfg);
    let mut cfg = config();
    cfg.routes.clear();
    invalid.push(cfg);
    let mut cfg = config();
    cfg.routes[0].targets = vec![10, 10];
    invalid.push(cfg);
    let mut cfg = config();
    cfg.routes[0].targets = vec![99];
    invalid.push(cfg);
    let mut cfg = config();
    cfg.schedule.bytes_per_turn = 63;
    invalid.push(cfg);
    let mut cfg = config();
    cfg.schedule.record_quantum = 0;
    invalid.push(cfg);
    let mut cfg = config();
    cfg.shared_records = 0;
    invalid.push(cfg);
    for cfg in invalid {
        let plan = plan();
        let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        owner.scope(|scope| {
            assert!(matches!(
                Router::new(&plan, scope.map(|(_, c)| c).collect(), cfg),
                Err(RoutingError::InvalidConfig)
            ));
        });
    }
    let plan = plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|scope| {
        let mut inputs: Vec<_> = scope.map(|(_, c)| c).collect();
        inputs.reverse();
        assert!(matches!(
            Router::new(&plan, inputs, config()),
            Err(RoutingError::InputsMismatch)
        ));
    });
}

#[test]
fn cancelled_output_cannot_release_an_original_still_used_on_another_thread() {
    use dot_logger::capture::CaptureError;
    use std::sync::Barrier;
    let mut setup = plan().config().clone();
    setup.producers[0].raw.as_mut().unwrap().slots = 1;
    let plan = SessionBuilder::new(setup)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|mut scope| {
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut loan = producer.try_acquire(64, Importance::Normal).unwrap();
        loan.payload_mut().fill(91);
        producer.try_record_raw(meta(), loan).unwrap();
        let mut router = Router::new(&plan, vec![a, b], config()).unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 16];
            for _ in 0..4 {
                dispatch.turn(now, &mut events).unwrap();
            }
            let a = outputs[0].try_next(now).unwrap().unwrap();
            let mut reader = outputs.pop().unwrap();
            let ready = Barrier::new(2);
            let release = Barrier::new(2);
            std::thread::scope(|threads| {
                let task = threads.spawn(|| {
                    let held = reader.try_next(now).unwrap().unwrap();
                    ready.wait();
                    release.wait();
                    assert_eq!(held.payload(), &[91; 64]);
                    drop(held);
                });
                ready.wait();
                dispatch.cancel_target(20);
                drop(a);
                assert_eq!(dispatch.turn(now, &mut events).unwrap().reclaimed, 0);
                assert!(matches!(
                    producer.try_acquire(64, Importance::Normal),
                    Err(CaptureError::Capacity)
                ));
                release.wait();
                task.join().unwrap();
            });
            let reclaimed: usize = (0..4)
                .map(|_| dispatch.turn(now, &mut events).unwrap().reclaimed)
                .sum();
            assert_eq!(reclaimed, 1);
            assert!(producer.try_acquire(64, Importance::Normal).is_ok());
        });
    });
}

#[test]
fn event_totals_reconcile_all_three_records_across_required_and_optional_targets() {
    with_router(config(), |mut producer, _, mut router| {
        router.scope(|mut dispatch,mut outputs| {
            let now=Instant::now(); let mut events=[None;32]; let mut all=vec![];
            for n in 0..3 {
                producer.try_record_small(meta(),&[n]).unwrap();
                let report=dispatch.turn(now,&mut events).unwrap();
                all.extend(events[..report.events].iter().flatten().copied());
                drop(outputs[1].try_next(now).unwrap().unwrap());
            }
            for _ in 0..6 { let report=dispatch.turn(now,&mut events).unwrap(); all.extend(events[..report.events].iter().flatten().copied()); }
            assert_eq!(all.len(),6);
            for seq in 1..=3 { for target in [10,20] { assert_eq!(all.iter().filter(|e| e.receipt.accepted_seq==seq && e.target_id==Some(target)).count(),1); } }
            assert_eq!(all.iter().filter(|e| e.required && matches!(e.outcome,DispatchOutcome::Dropped(_))).count(),2);
            assert_eq!(dispatch.target_snapshot(10).unwrap().retained_records,1);
            drop(outputs[0].try_next(now).unwrap().unwrap());
            assert_eq!(dispatch.snapshot().live_records,0);
        });
    });
}

#[test]
fn startup_rejects_different_pool_layouts_instead_of_underreporting_physical_raw_bytes() {
    use dot_logger::routing::RoutingError;
    let actual = plan();
    for raw in [true, false] {
        let mut incorrect = actual.config().clone();
        if raw {
            incorrect.producers[0].raw.as_mut().unwrap().slots = 1;
        } else {
            incorrect.producers[0].small.slot_bytes = 8;
        }
        let incorrect = SessionBuilder::new(incorrect)
            .validate(&PlatformCapabilities::default())
            .unwrap();
        let mut owner = CaptureOwner::new(&actual, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        owner.scope(|scope| {
            assert!(matches!(
                Router::new(&incorrect, scope.map(|(_, c)| c).collect(), config()),
                Err(RoutingError::InputsMismatch)
            ));
        });
    }
}
