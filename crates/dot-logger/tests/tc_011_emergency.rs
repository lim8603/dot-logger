//! TC-011: injected power events drive the same tracked shutdown coordinator.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    capture::*, completion::*, control::emergency::*, control::*, platform::power::*, routing::*,
};
use std::time::{Duration, Instant};
use support::*;
#[path = "../examples/support/ups/mod.rs"]
mod ups;

fn emergency() -> EmergencyConfig {
    EmergencyConfig {
        files: vec![
            EmergencyFile {
                target: 20,
                rank: 2,
                protected_bytes: 64,
                estimated_drain_sync: Duration::from_millis(10),
            },
            EmergencyFile {
                target: 10,
                rank: 1,
                protected_bytes: 64,
                estimated_drain_sync: Duration::from_millis(10),
            },
        ],
        safety_margin: Duration::from_millis(200),
    }
}

#[test]
fn disabled_alarm_does_not_reserve_or_close_input_and_minutes_are_explicit_deadlines() {
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(8),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let start = Instant::now();
        let mut control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 64,
            },
            start,
        )
        .unwrap();
        let bytes = control.reserved_bytes();
        let (mut producer, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        let alarm = PowerAlarm::new(1, start, Duration::from_secs(180)).unwrap();
        assert_eq!(
            control.notify_power(alarm, start).unwrap(),
            PowerNotice::Disabled
        );
        assert_eq!(control.reserved_bytes(), bytes);
        assert!(producer.try_record_small(meta(), &[1]).is_ok());
        assert!(
            control.enable_emergency(&plan, emergency()).is_err(),
            "enable is startup-only"
        );
        router.scope(|mut d, _| assert!(control.poll_emergency(start, &mut d).unwrap().is_none()));
    });
}

#[test]
fn earliest_alarm_deadline_never_extends_and_rank_precedes_target_id() {
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(8),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let start = Instant::now();
        let mut control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 64,
            },
            start,
        )
        .unwrap();
        control.enable_emergency(&plan, emergency()).unwrap();
        let (mut p, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        router.scope(|mut d, mut outputs| {
            let receipt = written(&mut p, &mut d, &mut outputs, start);
            control
                .notify_power(
                    PowerAlarm::new(1, start, Duration::from_secs(3)).unwrap(),
                    start,
                )
                .unwrap();
            assert!(matches!(
                p.try_record_small(meta(), &[2]),
                Err(CaptureError::Closing)
            ));
            control
                .notify_power(
                    PowerAlarm::new(2, start, Duration::from_secs(180)).unwrap(),
                    start,
                )
                .unwrap();
            let first = control.poll_emergency(start, &mut d).unwrap().unwrap();
            assert_eq!(first.poweroff_deadline, start + Duration::from_secs(3));
            control.observe_protected_bytes(10, 2).unwrap();
            control.observe_protected_bytes(20, 2).unwrap();
            let token = control.shutdown_sync().unwrap();
            let mut cutoffs = [ProducerRange::default(); 2];
            assert!(
                control
                    .next_sync(token, 20, start, &mut d, &mut cutoffs)
                    .unwrap()
                    .is_none()
            );
            control
                .next_sync(token, 10, start, &mut d, &mut cutoffs)
                .unwrap()
                .unwrap()
                .complete(SyncResult::Synced);
            control.poll_emergency(start, &mut d).unwrap();
            control
                .next_sync(token, 20, start, &mut d, &mut cutoffs)
                .unwrap()
                .unwrap()
                .complete(SyncResult::Synced);
            let report = control.poll_emergency(start, &mut d).unwrap().unwrap();
            assert!(report.protected_complete);
            assert_eq!(
                completion
                    .poll(receipt, 10, Boundary::Synced)
                    .unwrap()
                    .outcome,
                QueryOutcome::Complete
            );
            control
                .notify_power(
                    PowerAlarm::new(3, start, Duration::from_millis(1500)).unwrap(),
                    start,
                )
                .unwrap();
            let tighter = control.poll_emergency(start, &mut d).unwrap().unwrap();
            assert_eq!(
                tighter.poweroff_deadline,
                start + Duration::from_millis(1500)
            );
        });
    });
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
fn alarm_tightens_normal_shutdown_and_retains_late_sync_guard_until_real_completion() {
    session(|completion, mut p, mut router, control, start| {
        router.scope(|mut d, mut outputs| {
            let receipt=written(&mut p,&mut d,&mut outputs,start);
            let mut frozen=[ProducerRange::default();2];
            let ordinary=control.request_sync(completion.capture_scope(&mut frozen).unwrap(),&[10],start,start+Duration::from_secs(180)).unwrap();
            let io=control.next_sync(ordinary,10,start,&mut d,&mut frozen).unwrap().unwrap();
            control.begin_shutdown(ShutdownPlan{sync_files:true},start,start+Duration::from_secs(180),&mut d).unwrap();
            control.notify_power(PowerAlarm::new(1,start,Duration::from_secs(3)).unwrap(),start+Duration::from_millis(1000)).unwrap();
            let expired=start+Duration::from_secs(3);
            let report=control.poll_emergency(expired,&mut d).unwrap().unwrap();
            assert_eq!(report.work_deadline,start+Duration::from_millis(2800));
            assert!(matches!(report.shutdown,ShutdownPoll::Incomplete(r) if r.sync_in_flight==1));
            assert!(!report.protected_complete);
            assert_eq!(completion.poll(receipt,10,Boundary::Synced).unwrap().outcome,QueryOutcome::Pending);
            io.complete(SyncResult::Synced);
            for _ in 0..32 { control.poll_emergency(expired,&mut d).unwrap(); }
            assert!(matches!(control.poll_emergency(expired,&mut d).unwrap().unwrap().shutdown,ShutdownPoll::Closed(r) if !r.is_success()));
            assert_eq!(completion.poll(receipt,10,Boundary::Synced).unwrap().outcome,QueryOutcome::Complete);
        });
    });
}

#[test]
fn alarm_expires_queued_ordinary_work_instead_of_waiting_for_its_old_minutes_deadline() {
    session(|completion, mut p, mut router, control, start| {
        router.scope(|mut d, mut outputs| {
            written(&mut p, &mut d, &mut outputs, start);
            let mut ranges = [ProducerRange::default(); 2];
            let token = control
                .request_sync(
                    completion.capture_scope(&mut ranges).unwrap(),
                    &[20],
                    start,
                    start + Duration::from_secs(180),
                )
                .unwrap();
            control
                .notify_power(
                    PowerAlarm::new(1, start, Duration::from_secs(3)).unwrap(),
                    start,
                )
                .unwrap();
            control.poll_emergency(start, &mut d).unwrap();
            assert_eq!(control.sync_snapshot(token, start).unwrap().expired, 1);
            assert!(
                control
                    .next_sync(token, 20, start, &mut d, &mut ranges)
                    .unwrap()
                    .is_none()
            );
        });
    });
}

#[test]
fn concurrent_alarm_posts_keep_the_earliest_deadline_without_losing_the_event() {
    session(|_, _, mut router, control, start| {
        std::thread::scope(|threads| {
            for n in 1..=8 {
                let c = &control;
                threads.spawn(move || {
                    c.notify_power(
                        PowerAlarm::new(n, start, Duration::from_secs(n)).unwrap(),
                        start,
                    )
                    .unwrap();
                });
            }
        });
        router.scope(|mut d, _| {
            let r = control.poll_emergency(start, &mut d).unwrap().unwrap();
            assert_eq!(r.poweroff_deadline, start + Duration::from_secs(1));
            assert_eq!(r.work_deadline, start + Duration::from_millis(800));
            assert!((1..=8).contains(&r.first_event_id));
        });
    });
}

#[test]
fn payload_limit_and_missing_observation_cannot_become_zero_or_success() {
    session(|_, _, mut router, control, start| {
        router.scope(|mut d, _| {
            control
                .notify_power(
                    PowerAlarm::new(1, start, Duration::from_millis(210)).unwrap(),
                    start,
                )
                .unwrap();
            let r = control.poll_emergency(start, &mut d).unwrap().unwrap();
            assert!(r.unachievable);
            assert!(!r.protected_complete);
            control.observe_protected_bytes(10, 65).unwrap();
            assert!(control.observe_protected_bytes(10, 1).is_err());
            control.observe_protected_bytes(20, 0).unwrap();
            let r = control.poll_emergency(start, &mut d).unwrap().unwrap();
            assert!(r.unachievable && !r.protected_complete);
        });
    });
}

#[test]
fn late_alarm_has_no_new_budget_and_early_poll_cannot_precede_actual_receipt() {
    session(|_, _, mut router, control, start| {
        control
            .notify_power(
                PowerAlarm::new(1, start, Duration::from_secs(3)).unwrap(),
                start + Duration::from_secs(4),
            )
            .unwrap();
        router.scope(|mut d, _| {
            assert!(control.poll_emergency(start, &mut d).is_err());
            let report = control
                .poll_emergency(start + Duration::from_secs(4), &mut d)
                .unwrap()
                .unwrap();
            assert!(report.unachievable && !report.protected_complete);
            assert_eq!(report.poweroff_deadline, start + Duration::from_secs(3));
        });
    });
}

#[test]
#[cfg(unix)]
fn real_file_sync_recovery_and_failure_profiles_preserve_partial_receipt_facts() {
    let normal = ups::run(ups::Mode::Normal, 8192, Duration::ZERO).unwrap();
    assert!(normal.protected_complete);
    assert_eq!(normal.recovered_records, 8);
    assert_eq!(normal.storage_after_close, 0);
    for mode in [
        ups::Mode::SyncFailure,
        ups::Mode::WriteFailure,
        ups::Mode::EarlierDeadline,
        ups::Mode::Expired,
    ] {
        let report = ups::run(mode, 8192, Duration::ZERO).unwrap();
        assert!(!report.protected_complete);
        assert_eq!(report.storage_after_close, 0);
        assert!(report.partial);
    }
}

#[test]
fn invalid_startup_plans_do_not_consume_an_enabled_reservation() {
    let plan = plan();
    let route = routing();
    let mut tracked = tracking(8);
    tracked.targets[0].completion = Boundary::Synced;
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracked,
    )
    .unwrap();
    owner.scope(|scope| {
        let start = Instant::now();
        let mut control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 64,
            },
            start,
        )
        .unwrap();
        let initial = control.reserved_bytes();
        for n in 0..5 {
            let mut config = emergency();
            match n {
                0 => config.files.clear(),
                1 => config.files[0].target = 10,
                2 => config.files[0].target = 99,
                3 => config.files[0].protected_bytes = u64::MAX,
                _ => config.files.retain(|f| f.target != 10),
            }
            assert!(control.enable_emergency(&plan, config).is_err());
            assert_eq!(control.reserved_bytes(), initial);
        }
        control.enable_emergency(&plan, emergency()).unwrap();
        assert!(control.reserved_bytes() > initial);
        assert!(control.enable_emergency(&plan, emergency()).is_err());
    });
}

#[test]
fn equal_rank_uses_target_order_and_failure_still_allows_the_next_file() {
    let plan = plan();
    let route = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &route,
        tracking(8),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let start = Instant::now();
        let mut control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 64,
            },
            start,
        )
        .unwrap();
        let mut config = emergency();
        for file in &mut config.files {
            file.rank = 1;
        }
        control.enable_emergency(&plan, config).unwrap();
        let (_, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![a, b], route).unwrap();
        router.scope(|mut d, _| {
            let now = start + Duration::from_secs(1);
            control
                .notify_power(
                    PowerAlarm::new(1, start, Duration::from_secs(180)).unwrap(),
                    now,
                )
                .unwrap();
            let r = control.poll_emergency(now, &mut d).unwrap().unwrap();
            assert_eq!(r.work_deadline, start + Duration::from_millis(179800));
            let token = control.shutdown_sync().unwrap();
            let mut ranges = [ProducerRange::default(); 2];
            assert!(
                control
                    .next_sync(token, 20, now, &mut d, &mut ranges)
                    .unwrap()
                    .is_none()
            );
            control
                .next_sync(token, 10, now, &mut d, &mut ranges)
                .unwrap()
                .unwrap()
                .complete(SyncResult::Failed);
            control.poll_emergency(now, &mut d).unwrap();
            control
                .next_sync(token, 20, now, &mut d, &mut ranges)
                .unwrap()
                .unwrap()
                .complete(SyncResult::Synced);
            assert!(
                !control
                    .poll_emergency(now, &mut d)
                    .unwrap()
                    .unwrap()
                    .protected_complete
            );
        });
    });
}

#[test]
fn alarm_deadline_never_reclaims_a_raw_payload_still_owned_by_write_io() {
    use dot_logger::pool::RawRecordOutcome;
    session(|completion, mut p, mut router, control, start| {
        let mut loan = p.try_acquire(64, Importance::Normal).unwrap();
        loan.payload_mut().fill(81);
        let RawRecordOutcome::Accepted(receipt) = p.try_record_raw(meta(), loan).unwrap() else {
            panic!()
        };
        router.scope(|mut d,mut outputs| {
            d.turn(start,&mut[None;16]).unwrap();
            let active=outputs[0].try_next(start).unwrap().unwrap();
            let other=outputs[1].try_next(start).unwrap().unwrap();
            other.report(Update::Confirmed(Boundary::Written)).unwrap();drop(other);
            control.notify_power(PowerAlarm::new(1,start,Duration::from_secs(3)).unwrap(),start).unwrap();
            control.poll_emergency(start,&mut d).unwrap();
            let late=start+Duration::from_secs(4);
            let report=control.poll_emergency(late,&mut d).unwrap().unwrap();
            assert!(matches!(report.shutdown,ShutdownPoll::Incomplete(r) if r.in_flight==1 && r.raw_in_use==1));
            assert_eq!(active.payload(),&[81;64]);
            active.report(Update::Confirmed(Boundary::Written)).unwrap();drop(active);
            for _ in 0..32 {d.turn(late,&mut[None;16]).unwrap();control.poll_emergency(late,&mut d).unwrap();}
            let report=control.poll_emergency(late,&mut d).unwrap().unwrap();
            assert!(matches!(report.shutdown,ShutdownPoll::Closed(r) if r.raw_in_use==0 && r.raw_return_pending==0));
            assert!(!report.protected_complete);
            assert_eq!(completion.poll(receipt,10,Boundary::Written).unwrap().outcome,QueryOutcome::Complete);
            assert_eq!(completion.poll(receipt,10,Boundary::Synced).unwrap().outcome,QueryOutcome::Unknown);
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
        tracking(8),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let c = scope.completion().unwrap();
        let now = Instant::now();
        let mut control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 2,
                work_per_poll: 16,
            },
            now,
        )
        .unwrap();
        control.enable_emergency(&plan, emergency()).unwrap();
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
