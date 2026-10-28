//! TC-003: admission tracking, completion gaps, and bounded history.
#![allow(clippy::unwrap_used)]
use dot_logger::capture::{CaptureError, CaptureOwner, ChannelConfig, Level, RecordOutcome};
use dot_logger::completion::{Boundaries, Boundary, QueryOutcome, Update};
use std::time::Duration;
mod support;
use support::*;
#[test]
fn seq_two_complete_does_not_hide_seq_one_pending() {
    let plan = plan();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(4),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut producer, _) = scope.next().unwrap();
        let first = accepted(producer.try_record_small(meta(), &[1]).unwrap());
        let second = accepted(producer.try_record_small(meta(), &[2]).unwrap());
        for id in [10, 20] {
            completion
                .report(second, id, Update::Confirmed(Boundary::Written))
                .unwrap();
        }
        assert_eq!(
            completion
                .poll(first, 10, Boundary::Written)
                .unwrap()
                .outcome,
            QueryOutcome::Pending
        );
        assert_eq!(
            completion
                .poll(second, 10, Boundary::Written)
                .unwrap()
                .outcome,
            QueryOutcome::Complete
        );
    });
}
#[test]
fn full_completion_window_rejects_even_after_ingress_drains_and_never_reuses_pending() {
    let plan = plan();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(2),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut producer, mut reader) = scope.next().unwrap();
        let first = accepted(producer.try_record_small(meta(), &[1]).unwrap());
        let second = accepted(producer.try_record_small(meta(), &[2]).unwrap());
        let mut scratch = [0; 16];
        for _ in 0..2 {
            reader.try_read(&mut scratch).unwrap().unwrap();
        }
        assert_eq!(
            producer.try_record_small(meta(), &[3]),
            Err(CaptureError::CompletionCapacity)
        );
        for id in [10, 20] {
            completion
                .report(first, id, Update::Confirmed(Boundary::Written))
                .unwrap();
        }
        let third = accepted(producer.try_record_small(meta(), &[3]).unwrap());
        assert_eq!(third.accepted_seq, 3);
        assert_eq!(
            completion
                .poll(first, 10, Boundary::Written)
                .unwrap()
                .outcome,
            QueryOutcome::Expired
        );
        assert_eq!(
            completion
                .poll(second, 10, Boundary::Written)
                .unwrap()
                .outcome,
            QueryOutcome::Pending
        );
    });
}

#[test]
fn written_survives_sync_failure_and_adapter_boundaries_are_checked() {
    use dot_logger::completion::{CompletionError, FailureReason};
    let plan = plan();
    let mut cfg = tracking(4);
    cfg.targets[0].completion = Boundary::Synced;
    cfg.targets[1].supported = Boundaries::DATABASE;
    cfg.targets[1].completion = Boundary::DatabaseAcknowledged;
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        cfg,
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut producer, _) = scope.next().unwrap();
        let receipt = accepted(producer.try_record_small(meta(), &[1]).unwrap());
        completion
            .report(receipt, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        completion
            .report(receipt, 10, Update::Failed(FailureReason::SyncFailure))
            .unwrap();
        assert_eq!(
            completion
                .poll(receipt, 10, Boundary::Written)
                .unwrap()
                .outcome,
            QueryOutcome::Complete
        );
        assert_eq!(
            completion
                .poll(receipt, 10, Boundary::Synced)
                .unwrap()
                .outcome,
            QueryOutcome::Failed(FailureReason::SyncFailure)
        );
        assert_eq!(
            completion.report(receipt, 10, Update::Confirmed(Boundary::Synced)),
            Err(CompletionError::InvalidTransition)
        );
        assert_eq!(
            completion.poll(receipt, 20, Boundary::Written),
            Err(CompletionError::UnsupportedBoundary)
        );
        completion
            .report(
                receipt,
                20,
                Update::Confirmed(Boundary::DatabaseAcknowledged),
            )
            .unwrap();
        assert_eq!(
            completion
                .poll(receipt, 20, Boundary::DatabaseAcknowledged)
                .unwrap()
                .outcome,
            QueryOutcome::Complete
        );
    });
}
#[test]
fn bounded_pages_preserve_gaps_and_frozen_scope_excludes_later_acceptance() {
    use dot_logger::completion::{ProducerRange, TargetRequest};
    let plan = plan();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(4),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut producer, _) = scope.next().unwrap();
        let first = accepted(producer.try_record_small(meta(), &[1]).unwrap());
        let second = accepted(producer.try_record_small(meta(), &[2]).unwrap());
        let mut ranges = [ProducerRange::default(); 2];
        let frozen = completion.capture_scope(&mut ranges).unwrap();
        producer.try_record_small(meta(), &[3]).unwrap();
        for id in [10, 20] {
            completion
                .report(second, id, Update::Confirmed(Boundary::Written))
                .unwrap();
        }
        let requests = [
            TargetRequest {
                target_id: 10,
                boundary: Boundary::Written,
            },
            TargetRequest {
                target_id: 20,
                boundary: Boundary::Written,
            },
        ];
        let mut query = completion.query(frozen, &requests).unwrap();
        let mut page = [None; 8];
        let report = query.page(&mut page).unwrap();
        assert_eq!(report.items, 3);
        assert!(!report.summary.finished);
        assert!(!report.summary.is_complete());
        assert_eq!(page[3], None);
        let report = query.page(&mut page).unwrap();
        assert_eq!(report.items, 1);
        assert!(report.summary.finished);
        assert_eq!(report.summary.pending, 2);
        assert_eq!(report.summary.complete, 2);
        assert!(!report.summary.is_complete());
        for id in [10, 20] {
            completion
                .report(first, id, Update::Confirmed(Boundary::Written))
                .unwrap();
        }
        query.restart();
        query.page(&mut page).unwrap();
        assert!(query.page(&mut page).unwrap().summary.is_complete());
    });
}
#[test]
fn query_permits_timeout_and_cancellation_do_not_change_record_outcomes() {
    use dot_logger::completion::{CompletionError, ProducerRange, TargetRequest, WaitOutcome};
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;
    let plan = plan();
    let mut cfg = tracking(4);
    cfg.max_queries = 1;
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        cfg,
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, _) = scope.next().unwrap();
        let receipt = accepted(p.try_record_small(meta(), &[1]).unwrap());
        let ranges = [ProducerRange {
            producer_id: 0,
            first: 1,
            last: 1,
        }];
        let frozen = c.scope(&ranges).unwrap();
        let requests = [TargetRequest {
            target_id: 10,
            boundary: Boundary::Written,
        }];
        let mut query = c.query(frozen, &requests).unwrap();
        assert!(matches!(
            c.query(frozen, &requests),
            Err(CompletionError::QueryCapacity)
        ));
        let mut page = [None; 1];
        assert_eq!(
            query
                .wait(Instant::now(), &AtomicBool::new(false), &mut page)
                .unwrap()
                .outcome,
            WaitOutcome::TimedOut
        );
        assert_eq!(
            query
                .wait(
                    Instant::now() + Duration::from_secs(1),
                    &AtomicBool::new(true),
                    &mut page
                )
                .unwrap()
                .outcome,
            WaitOutcome::Cancelled
        );
        assert_eq!(
            c.poll(receipt, 10, Boundary::Written).unwrap().outcome,
            QueryOutcome::Pending
        );
        drop(query);
        assert!(c.query(frozen, &requests).is_ok());
    });
}
#[test]
fn retired_details_are_expired_while_terminal_session_counters_are_preserved() {
    use dot_logger::completion::CompletionError;
    let plan = plan();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(1),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, mut reader) = scope.next().unwrap();
        let first = accepted(p.try_record_small(meta(), &[1]).unwrap());
        reader.try_read(&mut [0; 16]).unwrap();
        for id in [10, 20] {
            c.report(first, id, Update::Confirmed(Boundary::Written))
                .unwrap();
        }
        let second = accepted(p.try_record_small(meta(), &[2]).unwrap());
        assert_eq!(
            c.poll(first, 10, Boundary::Written).unwrap().outcome,
            QueryOutcome::Expired
        );
        assert_eq!(
            c.report(first, 10, Update::Unknown),
            Err(CompletionError::Expired)
        );
        assert_eq!(
            c.poll(second, 10, Boundary::Written).unwrap().outcome,
            QueryOutcome::Pending
        );
        let totals = c.totals(false);
        assert_eq!(totals.records, 2);
        assert_eq!(totals.assigned, 4);
        assert_eq!(totals.complete, 2);
        assert_eq!(totals.pending, 2);
        assert_eq!(totals.expired_records, 1);
    });
}
#[test]
fn raw_rejection_returns_exact_loan_and_skips_do_not_reserve_tracking() {
    use dot_logger::capture::Importance;
    let plan = plan();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(1),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let (mut p, mut reader) = scope.next().unwrap();
        p.try_record_small(meta(), &[1]).unwrap();
        reader.try_read(&mut [0; 16]).unwrap();
        let mut loan = p.try_acquire(64, Importance::Normal).unwrap();
        loan.payload_mut().fill(11);
        let id = loan.identity();
        let ptr = loan.payload().as_ptr();
        let error = p.try_record_raw(meta(), loan).unwrap_err();
        assert_eq!(error.reason, CaptureError::CompletionCapacity);
        assert_eq!(error.loan.identity(), id);
        assert_eq!(error.loan.payload().as_ptr(), ptr);
        assert_eq!(error.loan.payload(), &[11; 64]);
        drop(error.loan);
    });
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Info;
    let mut owner =
        CaptureOwner::new_tracked(&plan, 43, 1, &[channel], &routing(), tracking(1)).unwrap();
    owner.scope(|mut scope| {
        let (mut p, _) = scope.next().unwrap();
        p.try_record_small(meta(), &[1]).unwrap();
        let mut low = meta();
        low.level = Level::Trace;
        assert_eq!(p.try_record_small(low, &[2]), Ok(RecordOutcome::Skipped));
        low.schema_id = 0;
        assert_eq!(
            p.try_record_small(low, &[2]),
            Err(CaptureError::SchemaMismatch)
        );
    });
}
#[test]
fn tracked_router_records_discards_and_unreported_output_drop_is_unknown() {
    use dot_logger::routing::{DropReason, Router};
    use std::time::Instant;
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(2),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let first = accepted(p.try_record_small(meta(), &[1]).unwrap());
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        router.scope(|mut d, mut outputs| {
            let now = Instant::now();
            let mut events = [None; 16];
            d.turn(now, &mut events).unwrap();
            let a = outputs[0].try_next(now).unwrap().unwrap();
            a.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(a);
            drop(outputs[1].try_next(now).unwrap().unwrap());
            assert_eq!(
                c.poll(first, 20, Boundary::Written).unwrap().outcome,
                QueryOutcome::Unknown
            );
            d.turn(now, &mut events).unwrap();
            let second = accepted(p.try_record_small(meta(), &[2]).unwrap());
            d.turn(now, &mut events).unwrap();
            d.cancel_target(10);
            assert!(outputs[0].try_next(now).unwrap().is_err());
            d.turn(now, &mut events).unwrap();
            assert_eq!(
                c.poll(second, 10, Boundary::Written).unwrap().outcome,
                QueryOutcome::Dropped(DropReason::Cancelled)
            );
        });
    });
}
#[test]
fn unsupported_scope_and_target_contracts_are_rejected_before_querying() {
    use dot_logger::completion::{CompletionError, ProducerRange, TargetRequest};
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(2),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, _) = scope.next().unwrap();
        p.try_record_small(meta(), &[1]).unwrap();
        assert!(matches!(
            c.scope(&[ProducerRange {
                producer_id: 0,
                first: 1,
                last: 2
            }]),
            Err(CompletionError::InvalidScope)
        ));
        assert!(matches!(
            c.scope(&[ProducerRange {
                producer_id: 9,
                first: 1,
                last: 1
            }]),
            Err(CompletionError::InvalidScope)
        ));
        let ranges = [ProducerRange {
            producer_id: 0,
            first: 1,
            last: 1,
        }];
        let frozen = c.scope(&ranges).unwrap();
        assert!(matches!(
            c.query(
                frozen,
                &[TargetRequest {
                    target_id: 10,
                    boundary: Boundary::ReceiverAcknowledged
                }]
            ),
            Err(CompletionError::UnsupportedBoundary)
        ));
    });
    let mut cfg = tracking(2);
    cfg.targets[0].required = false;
    assert!(
        CaptureOwner::new_tracked(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)], &route, cfg)
            .is_err()
    );
}

#[test]
fn terminal_detail_age_expires_without_erasing_totals_or_expiring_pending_records() {
    use std::time::Instant;
    let plan = plan();
    let mut cfg = tracking(4);
    cfg.history_retention = Duration::from_secs(1);
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        cfg,
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, _) = scope.next().unwrap();
        let a = accepted(p.try_record_small(meta(), &[1]).unwrap());
        let b = accepted(p.try_record_small(meta(), &[2]).unwrap());
        for id in [10, 20] {
            c.report(a, id, Update::Confirmed(Boundary::Written))
                .unwrap();
        }
        let future = Instant::now() + Duration::from_secs(2);
        assert_eq!(
            c.poll_at(a, 10, Boundary::Written, future).unwrap().outcome,
            QueryOutcome::Expired
        );
        assert_eq!(
            c.poll_at(b, 10, Boundary::Written, future).unwrap().outcome,
            QueryOutcome::Pending
        );
        let totals = c.totals(false);
        assert_eq!(totals.complete, 2);
        assert_eq!(totals.pending, 2);
    });
}

#[test]
fn completion_reservation_is_concrete_and_plan_mismatch_cannot_expand_budgets() {
    use dot_logger::config::{ConfigError, Resource, SessionBuilder};
    use dot_logger::platform::PlatformCapabilities;
    use dot_logger::routing::{Router, RoutingError};
    let checked = plan();
    let route = routing();
    let owner = CaptureOwner::new_tracked(
        &checked,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    let required = owner.control_reserved_bytes();
    drop(owner);
    for fits in [true, false] {
        let mut config = checked.config().clone();
        config.memory.control_bytes = required - usize::from(!fits);
        let checked = SessionBuilder::new(config)
            .validate(&PlatformCapabilities::default())
            .unwrap();
        let result = CaptureOwner::new_tracked(
            &checked,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &route,
            tracking(4),
        );
        if fits {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(dot_logger::capture::CaptureBuildError::Config(
                    ConfigError::BudgetExceeded {
                        resource: Resource::Completion,
                        ..
                    }
                ))
            ));
        }
    }
    let mut owner = CaptureOwner::new_tracked(
        &checked,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    let mut changed = checked.config().clone();
    changed.memory.output_bytes -= 1;
    let changed = SessionBuilder::new(changed)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    owner.scope(|scope| {
        assert!(matches!(
            Router::new(&changed, scope.map(|(_, c)| c).collect(), route),
            Err(RoutingError::InputsMismatch)
        ))
    });
}

#[test]
fn explicitly_unrouted_records_also_retire_terminal_detail_by_age_after_dispatch() {
    use dot_logger::routing::Router;
    use std::time::Instant;
    let plan = plan();
    let mut route = routing();
    route.routes[0].targets.clear();
    let mut cfg = tracking(4);
    cfg.history_retention = Duration::from_secs(1);
    let mut owner =
        CaptureOwner::new_tracked(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)], &route, cfg)
            .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let receipt = accepted(p.try_record_small(meta(), &[1]).unwrap());
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        router.scope(|mut d, _| {
            d.turn(Instant::now(), &mut [None; 4]).unwrap();
        });
        assert_eq!(
            c.poll(receipt, 10, Boundary::Written).unwrap().outcome,
            QueryOutcome::NotRouted
        );
        assert_eq!(
            c.poll_at(
                receipt,
                10,
                Boundary::Written,
                Instant::now() + Duration::from_secs(2)
            )
            .unwrap()
            .outcome,
            QueryOutcome::Expired
        );
    });
}
