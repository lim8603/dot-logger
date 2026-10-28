//! TC-003: explicit sync barriers, deadlines, and live owner/IO preservation.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::capture::{CaptureError, CaptureOwner, ChannelConfig, Importance};
use dot_logger::completion::{Boundary, ProducerRange, QueryOutcome, Update};
use dot_logger::control::{ControlConfig, ControlOwner, ShutdownPlan, ShutdownPoll, SyncResult};
use dot_logger::routing::Router;
use std::time::{Duration, Instant};
use support::*;
#[test]
fn polling_never_syncs_and_only_an_explicit_barrier_ack_confirms_synced() {
    let plan = plan();
    let routing = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        tracking(4),
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
                work_per_poll: 8,
            },
            now,
        )
        .unwrap();
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let receipt = accepted(producer.try_record_small(meta(), &[1]).unwrap());
        let mut router = Router::new(&plan, vec![a, b], routing).unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let mut events = [None; 16];
            dispatch.turn(now, &mut events).unwrap();
            for output in &mut outputs {
                let item = output.try_next(now).unwrap().unwrap();
                item.report(Update::Confirmed(Boundary::Written)).unwrap();
                drop(item);
            }
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Synced)
                    .unwrap()
                    .outcome,
                QueryOutcome::Pending
            );
            control.poll(now).unwrap();
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Synced)
                    .unwrap()
                    .outcome,
                QueryOutcome::Pending
            );
            let mut ranges = [ProducerRange::default(); 2];
            let frozen = completion.capture_scope(&mut ranges).unwrap();
            let token = control
                .request_sync(frozen, &[10], now, now + Duration::from_secs(1))
                .unwrap();
            let mut barrier = [ProducerRange::default(); 2];
            let command = control
                .next_sync(token, 10, now, &mut dispatch, &mut barrier)
                .unwrap()
                .unwrap();
            assert_eq!(barrier[0].last, receipt.accepted_seq);
            command.complete(SyncResult::Synced);
            control.poll(now).unwrap();
            assert!(
                control
                    .sync_snapshot(token, now)
                    .unwrap()
                    .barriers_succeeded()
            );
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Synced)
                    .unwrap()
                    .outcome,
                QueryOutcome::Complete
            );
            assert_eq!(
                completion
                    .poll(receipt, 20, Boundary::Synced)
                    .unwrap()
                    .outcome,
                QueryOutcome::Pending
            );
        });
    });
}
#[test]
fn shutdown_timeout_keeps_live_raw_io_and_can_finish_on_the_same_owner() {
    let plan = plan();
    let routing = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        tracking(4),
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
                work_per_poll: 16,
            },
            now,
        )
        .unwrap();
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut loan = producer.try_acquire(64, Importance::Normal).unwrap();
        loan.payload_mut().fill(81);
        let receipt = match producer.try_record_raw(meta(), loan).unwrap() {
            dot_logger::pool::RawRecordOutcome::Accepted(receipt) => receipt,
            _ => panic!("unexpected skip"),
        };
        let mut router = Router::new(&plan, vec![a, b], routing).unwrap();
        router.scope(|mut dispatch, mut outputs| {
            let mut events = [None; 16];
            dispatch.turn(now, &mut events).unwrap();
            let a = outputs[0].try_next(now).unwrap().unwrap();
            let b = outputs[1].try_next(now).unwrap().unwrap();
            let result = control
                .begin_shutdown(ShutdownPlan { sync_files: false }, now, now, &mut dispatch)
                .unwrap();
            let ShutdownPoll::Incomplete(report) = result else {
                panic!("deadline must preserve incomplete IO")
            };
            assert_eq!(report.in_flight, 2);
            assert_eq!(b.payload(), &[81; 64]);
            assert_eq!(
                producer.try_record_small(meta(), &[2]),
                Err(CaptureError::Closing)
            );
            a.report(Update::Confirmed(Boundary::Written)).unwrap();
            b.report(Update::Confirmed(Boundary::Written)).unwrap();
            drop(a);
            drop(b);
            for _ in 0..3 {
                dispatch.turn(now, &mut events).unwrap();
            }
            let ShutdownPoll::Closed(report) = control.poll_shutdown(now, &mut dispatch).unwrap()
            else {
                panic!("same owner must allow cleanup after deadline")
            };
            assert!(!report.is_success());
            assert!(report.deadline_elapsed);
            assert_eq!(report.raw_in_use, 0);
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

#[test]
fn sync_requests_for_one_target_never_overlap_actual_io() {
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    owner.scope(|scope| {
        let c = scope.completion().unwrap();
        let now = Instant::now();
        let control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 2,
                work_per_poll: 16,
            },
            now,
        )
        .unwrap();
        let inputs = scope.map(|(_, c)| c).collect();
        let mut router = Router::new(&plan, inputs, route).unwrap();
        router.scope(|mut dispatcher, _| {
            let mut ranges = [ProducerRange::default(); 2];
            let frozen = c.capture_scope(&mut ranges).unwrap();
            let a = control
                .request_sync(frozen, &[10], now, now + Duration::from_secs(1))
                .unwrap();
            let b = control
                .request_sync(frozen, &[10], now, now + Duration::from_secs(1))
                .unwrap();
            let mut cutoffs = [ProducerRange::default(); 2];
            let first = control
                .next_sync(a, 10, now, &mut dispatcher, &mut cutoffs)
                .unwrap()
                .unwrap();
            assert!(
                control
                    .next_sync(b, 10, now, &mut dispatcher, &mut cutoffs)
                    .unwrap()
                    .is_none()
            );
            first.complete(SyncResult::Synced);
            control.poll(now).unwrap();
            assert!(
                control
                    .next_sync(b, 10, now, &mut dispatcher, &mut cutoffs)
                    .unwrap()
                    .is_some()
            );
        });
    });
}

fn session<R>(
    run: impl for<'a> FnOnce(
        dot_logger::completion::Completion<'a>,
        dot_logger::capture::Producer<'a>,
        Router<'a>,
        ControlOwner<'a>,
        Instant,
    ) -> R,
) -> R {
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let now = Instant::now();
        let control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 2,
                work_per_poll: 16,
            },
            now,
        )
        .unwrap();
        let (p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        run(
            c,
            p,
            Router::new(&plan, vec![a, b], route).unwrap(),
            control,
            now,
        )
    })
}
fn written<'a>(
    producer: &mut dot_logger::capture::Producer<'a>,
    dispatcher: &mut dot_logger::routing::Dispatcher<'_, 'a>,
    outputs: &mut [dot_logger::routing::TargetConsumer<'_, 'a>],
    now: Instant,
) -> dot_logger::capture::CaptureReceipt {
    let receipt = accepted(producer.try_record_small(meta(), &[1]).unwrap());
    dispatcher.turn(now, &mut [None; 16]).unwrap();
    for reader in outputs {
        let item = reader.try_next(now).unwrap().unwrap();
        item.report(Update::Confirmed(Boundary::Written)).unwrap();
        drop(item);
    }
    receipt
}
#[test]
fn expired_sync_guard_keeps_io_alive_and_prevents_request_reuse_and_shutdown() {
    use dot_logger::control::ControlError;
    session(|c, mut p, mut router, control, now| {
        router.scope(|mut d,mut outputs|{
  let receipt=written(&mut p,&mut d,&mut outputs,now);let mut ranges=[ProducerRange::default();2];let frozen=c.capture_scope(&mut ranges).unwrap();let end=now+Duration::from_secs(1);
  let token=control.request_sync(frozen,&[10],now,end).unwrap();let mut barrier=[ProducerRange::default();2];let io=control.next_sync(token,10,now,&mut d,&mut barrier).unwrap().unwrap();
  assert_eq!(control.release_sync(token),Err(ControlError::InFlight));let late=end+Duration::from_secs(1);control.poll(late).unwrap();assert_eq!(control.sync_snapshot(token,late).unwrap().in_flight,1);
  let result=control.begin_shutdown(ShutdownPlan{sync_files:false},late,late,&mut d).unwrap();assert!(matches!(result,ShutdownPoll::Incomplete(report)if report.sync_in_flight==1));
  io.complete(SyncResult::Synced);let result=control.poll_shutdown(late,&mut d).unwrap();assert!(matches!(result,ShutdownPoll::Closed(report)if report.deadline_elapsed&&!report.is_success()));
  assert_eq!(c.poll(receipt,10,Boundary::Synced).unwrap().outcome,QueryOutcome::Complete);
 });
    });
}
#[test]
fn failed_or_unreported_sync_preserves_written_and_reports_failed_or_unknown() {
    use dot_logger::completion::FailureReason;
    for fail in [true, false] {
        session(|c, mut p, mut router, control, now| {
            router.scope(|mut d, mut outputs| {
                let receipt = written(&mut p, &mut d, &mut outputs, now);
                let mut ranges = [ProducerRange::default(); 2];
                let token = control
                    .request_sync(
                        c.capture_scope(&mut ranges).unwrap(),
                        &[10],
                        now,
                        now + Duration::from_secs(1),
                    )
                    .unwrap();
                let mut barrier = [ProducerRange::default(); 2];
                let io = control
                    .next_sync(token, 10, now, &mut d, &mut barrier)
                    .unwrap()
                    .unwrap();
                if fail {
                    io.complete(SyncResult::Failed);
                } else {
                    drop(io);
                }
                control.poll(now).unwrap();
                assert_eq!(
                    c.poll(receipt, 10, Boundary::Written).unwrap().outcome,
                    QueryOutcome::Complete
                );
                assert_eq!(
                    c.poll(receipt, 10, Boundary::Synced).unwrap().outcome,
                    if fail {
                        QueryOutcome::Failed(FailureReason::SyncFailure)
                    } else {
                        QueryOutcome::Unknown
                    }
                );
                let snapshot = control.sync_snapshot(token, now).unwrap();
                assert_eq!(snapshot.failed + snapshot.unknown, 1);
                assert!(!snapshot.barriers_succeeded());
            });
        });
    }
}
#[test]
fn barrier_ack_cannot_confirm_records_accepted_after_the_frozen_cutoff() {
    session(|c, mut p, mut router, control, now| {
        router.scope(|mut d, mut outputs| {
            let first = written(&mut p, &mut d, &mut outputs, now);
            let mut ranges = [ProducerRange::default(); 2];
            let token = control
                .request_sync(
                    c.capture_scope(&mut ranges).unwrap(),
                    &[10],
                    now,
                    now + Duration::from_secs(1),
                )
                .unwrap();
            let second = written(&mut p, &mut d, &mut outputs, now);
            let mut barrier = [ProducerRange::default(); 2];
            let io = control
                .next_sync(token, 10, now, &mut d, &mut barrier)
                .unwrap()
                .unwrap();
            assert_eq!(barrier[0].last, first.accepted_seq);
            io.complete(SyncResult::Synced);
            control.poll(now).unwrap();
            assert_eq!(
                c.poll(first, 10, Boundary::Synced).unwrap().outcome,
                QueryOutcome::Complete
            );
            assert_eq!(
                c.poll(second, 10, Boundary::Synced).unwrap().outcome,
                QueryOutcome::Pending
            );
        });
    });
}
#[test]
fn shutdown_has_a_dedicated_sync_slot_and_waits_for_explicit_file_barriers() {
    session(|c, mut p, mut router, control, now| {
        router.scope(|mut d, mut outputs| {
            let receipt = written(&mut p, &mut d, &mut outputs, now);
            let mut ranges = [ProducerRange::default(); 2];
            let frozen = c.capture_scope(&mut ranges).unwrap();
            let end = now + Duration::from_secs(1);
            let a = control.request_sync(frozen, &[10], now, end).unwrap();
            let b = control.request_sync(frozen, &[20], now, end).unwrap();
            assert!(matches!(
                control
                    .begin_shutdown(ShutdownPlan { sync_files: true }, now, end, &mut d)
                    .unwrap(),
                ShutdownPoll::Closing(_)
            ));
            let shutdown = control.shutdown_sync().unwrap();
            let mut cutoffs = [ProducerRange::default(); 2];
            for (token, target) in [(a, 10), (b, 20)] {
                control
                    .next_sync(token, target, now, &mut d, &mut cutoffs)
                    .unwrap()
                    .unwrap()
                    .complete(SyncResult::Synced);
            }
            control.poll(now).unwrap();
            for target in [10, 20] {
                control
                    .next_sync(shutdown, target, now, &mut d, &mut cutoffs)
                    .unwrap()
                    .unwrap()
                    .complete(SyncResult::Synced);
            }
            let ShutdownPoll::Closed(report) = control.poll_shutdown(now, &mut d).unwrap() else {
                panic!("explicit barriers must permit closure")
            };
            assert!(report.is_success());
            assert_eq!(
                c.poll(receipt, 10, Boundary::Synced).unwrap().outcome,
                QueryOutcome::Complete
            );
            let ShutdownPoll::Closed(later) = control
                .poll_shutdown(end + Duration::from_secs(1), &mut d)
                .unwrap()
            else {
                panic!("closed phase is stable")
            };
            assert!(later.is_success());
        });
    });
}
#[test]
fn an_application_loan_keeps_shutdown_incomplete_until_its_return_scan() {
    session(|_, mut p, mut router, control, now| {
        router.scope(|mut d,_|{
  let loan=p.try_acquire(64,Importance::Normal).unwrap();let result=control.begin_shutdown(ShutdownPlan{sync_files:false},now,now,&mut d).unwrap();assert!(matches!(result,ShutdownPoll::Incomplete(report)if report.raw_loaned==1));
  drop(loan);for _ in 0..3{d.turn(now,&mut[None;16]).unwrap();}
  assert!(matches!(control.poll_shutdown(now,&mut d).unwrap(),ShutdownPoll::Closed(report)if report.raw_loaned==0&&report.raw_return_pending==0));
 });
    });
}
#[test]
fn stale_request_generation_and_reconstructed_controller_epoch_never_alias_new_work() {
    use dot_logger::control::ControlError;
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    owner.scope(|scope| {
        let capture = scope.control();
        let c = scope.completion().unwrap();
        let now = Instant::now();
        let cfg = ControlConfig {
            sync_slots: 1,
            work_per_poll: 16,
        };
        let first = ControlOwner::new(&plan, capture, cfg, now).unwrap();
        assert!(matches!(
            ControlOwner::new(&plan, capture, cfg, now),
            Err(ControlError::Busy)
        ));
        let mut router = Router::new(&plan, scope.map(|(_, c)| c).collect(), route).unwrap();
        router.scope(|mut d, _| {
            let mut ranges = [ProducerRange::default(); 2];
            let scope = c.capture_scope(&mut ranges).unwrap();
            let mut out = [ProducerRange::default(); 2];
            let end = now + Duration::from_secs(1);
            let old = first.request_sync(scope, &[10], now, end).unwrap();
            first
                .next_sync(old, 10, now, &mut d, &mut out)
                .unwrap()
                .unwrap()
                .complete(SyncResult::Synced);
            first.poll(now).unwrap();
            first.release_sync(old).unwrap();
            let newer = first.request_sync(scope, &[10], now, end).unwrap();
            assert_ne!(old, newer);
            assert_eq!(
                first.sync_snapshot(old, now),
                Err(ControlError::InvalidToken)
            );
            drop(first);
            let second = ControlOwner::new(&plan, capture, cfg, now).unwrap();
            let newest = second.request_sync(scope, &[10], now, end).unwrap();
            assert_ne!(newer, newest);
            assert_eq!(
                second.sync_snapshot(newer, now),
                Err(ControlError::InvalidToken)
            );
        });
    });
}
#[test]
fn unsupported_sync_plan_rejects_before_closing_admission() {
    use dot_logger::control::ControlError;
    let plan = plan();
    let route = routing();
    let mut cfg = tracking(4);
    cfg.targets[0].completion = Boundary::Synced;
    let mut owner =
        CaptureOwner::new_tracked(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)], &route, cfg)
            .unwrap();
    owner.scope(|mut scope| {
        let now = Instant::now();
        let control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 16,
            },
            now,
        )
        .unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        router.scope(|mut d, _| {
            assert_eq!(
                control.begin_shutdown(
                    ShutdownPlan { sync_files: false },
                    now,
                    now + Duration::from_secs(1),
                    &mut d
                ),
                Err(ControlError::UnsupportedBoundary)
            );
            assert!(p.try_record_small(meta(), &[1]).is_ok());
        });
    });
}
#[test]
fn closure_racing_acceptance_freezes_every_pre_close_receipt_and_drains_it() {
    use std::sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    };
    session(|c, mut p, mut router, control, now| {
        router.scope(|mut d, mut outputs| {
            let ready = Barrier::new(2);
            let accepted_count = AtomicUsize::new(0);
            std::thread::scope(|threads| {
                let source = threads.spawn(|| {
                    if p.try_record_small(meta(), &[1]).is_ok() {
                        accepted_count.fetch_add(1, Ordering::SeqCst);
                    }
                    ready.wait();
                    for _ in 0..32 {
                        match p.try_record_small(meta(), &[2]) {
                            Ok(_) => {
                                accepted_count.fetch_add(1, Ordering::SeqCst);
                            }
                            Err(CaptureError::Closing) => break,
                            Err(CaptureError::Capacity | CaptureError::CompletionCapacity) => {}
                            other => panic!("unexpected source outcome {other:?}"),
                        }
                    }
                });
                ready.wait();
                control
                    .begin_shutdown(
                        ShutdownPlan { sync_files: false },
                        now,
                        now + Duration::from_secs(1),
                        &mut d,
                    )
                    .unwrap();
                for _ in 0..64 {
                    d.turn(now, &mut [None; 16]).unwrap();
                    for output in &mut outputs {
                        while let Some(item) = output.try_next(now) {
                            if let Ok(item) = item {
                                item.report(Update::Confirmed(Boundary::Written)).unwrap();
                            }
                        }
                    }
                    if matches!(
                        control.poll_shutdown(now, &mut d).unwrap(),
                        ShutdownPoll::Closed(_)
                    ) {
                        break;
                    }
                }
                source.join().unwrap();
            });
            for _ in 0..8 {
                d.turn(now, &mut [None; 16]).unwrap();
                for output in &mut outputs {
                    while let Some(item) = output.try_next(now) {
                        if let Ok(item) = item {
                            item.report(Update::Confirmed(Boundary::Written)).unwrap();
                        }
                    }
                }
            }
            let ShutdownPoll::Closed(report) = control.poll_shutdown(now, &mut d).unwrap() else {
                panic!("all admitted work must drain")
            };
            assert!(report.is_success());
            let mut frozen = [ProducerRange::default(); 2];
            control.shutdown_ranges(&mut frozen).unwrap();
            assert_eq!(frozen[0].last, accepted_count.load(Ordering::SeqCst) as u64);
            assert_eq!(c.totals(false).complete, 2 * frozen[0].last);
        });
    });
}

#[test]
fn concrete_control_reservation_accepts_exact_fit_and_rejects_one_byte_short() {
    use dot_logger::config::{ConfigError, Resource, SessionBuilder};
    use dot_logger::control::ControlError;
    use dot_logger::platform::PlatformCapabilities;
    let original = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &original,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    let capture = owner.control_reserved_bytes();
    let reserved = owner.scope(|scope| {
        ControlOwner::new(
            &original,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 16,
            },
            Instant::now(),
        )
        .unwrap()
        .reserved_bytes()
    });
    for fits in [true, false] {
        let mut config = original.config().clone();
        config.memory.control_bytes = capture + reserved - usize::from(!fits);
        let checked = SessionBuilder::new(config)
            .validate(&PlatformCapabilities::default())
            .unwrap();
        let mut owner = CaptureOwner::new_tracked(
            &checked,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &route,
            tracking(4),
        )
        .unwrap();
        owner.scope(|scope| {
            let result = ControlOwner::new(
                &checked,
                scope.control(),
                ControlConfig {
                    sync_slots: 1,
                    work_per_poll: 16,
                },
                Instant::now(),
            );
            if fits {
                assert!(result.is_ok());
            } else {
                assert!(matches!(
                    result,
                    Err(ControlError::Config(ConfigError::BudgetExceeded {
                        resource: Resource::Control,
                        ..
                    }))
                ));
            }
        });
    }
}
#[test]
fn a_queued_barrier_expiry_never_creates_an_io_guard() {
    session(|c, _, mut router, control, now| {
        router.scope(|mut d, _| {
            let mut ranges = [ProducerRange::default(); 2];
            let end = now + Duration::from_secs(1);
            let token = control
                .request_sync(c.capture_scope(&mut ranges).unwrap(), &[10], now, end)
                .unwrap();
            control.poll(end).unwrap();
            let mut cutoffs = [ProducerRange::default(); 2];
            assert!(
                control
                    .next_sync(token, 10, end, &mut d, &mut cutoffs)
                    .unwrap()
                    .is_none()
            );
            let snapshot = control.sync_snapshot(token, end).unwrap();
            assert_eq!(snapshot.expired, 1);
            assert_eq!(snapshot.in_flight, 0);
            assert!(!snapshot.barriers_succeeded());
            control.release_sync(token).unwrap();
        });
    });
}

#[test]
fn a_sync_token_from_another_session_cannot_alias_an_identical_slot_generation() {
    use dot_logger::control::ControlError;
    let plan = plan();
    let route = routing();
    let now = Instant::now();
    let cfg = ControlConfig {
        sync_slots: 1,
        work_per_poll: 16,
    };
    let mut first = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    let old = first.scope(|scope| {
        let c = scope.completion().unwrap();
        let control = ControlOwner::new(&plan, scope.control(), cfg, now).unwrap();
        let mut ranges = [ProducerRange::default(); 2];
        control
            .request_sync(
                c.capture_scope(&mut ranges).unwrap(),
                &[10],
                now,
                now + Duration::from_secs(1),
            )
            .unwrap()
    });
    let mut second = CaptureOwner::new_tracked(
        &plan,
        43,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(4),
    )
    .unwrap();
    second.scope(|scope| {
        let c = scope.completion().unwrap();
        let control = ControlOwner::new(&plan, scope.control(), cfg, now).unwrap();
        let mut ranges = [ProducerRange::default(); 2];
        let new = control
            .request_sync(
                c.capture_scope(&mut ranges).unwrap(),
                &[10],
                now,
                now + Duration::from_secs(1),
            )
            .unwrap();
        assert_ne!(old, new);
        assert_eq!(
            control.sync_snapshot(old, now),
            Err(ControlError::InvalidToken)
        );
    });
}
