//! TC-009 live configuration: publication differs from producer application.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    capture::*,
    completion::{Boundary, QueryOutcome, Update},
    control::config_update::*,
    routing::Router,
};
use support::*;

fn patch() -> ConfigPatch {
    ConfigPatch {
        channels: vec![ChannelPatch {
            channel_id: 7,
            minimum_level: Level::Warn,
            sample_every: 2,
        }],
        routes: vec![],
        memory: None,
    }
}
fn restore() -> ConfigPatch {
    ConfigPatch {
        channels: vec![ChannelPatch {
            channel_id: 7,
            minimum_level: Level::Trace,
            sample_every: 1,
        }],
        routes: vec![],
        memory: None,
    }
}
fn owner(revision: u64) -> CaptureOwner {
    let mut o = CaptureOwner::new_tracked(
        &plan(),
        42,
        revision,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(8),
    )
    .unwrap();
    o.enable_config_updates(&routing()).unwrap();
    o
}
#[test]
fn validation_does_not_apply_and_stale_base_conflicts() {
    let mut o = owner(1);
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        let v = u.validate_config(1, &patch()).unwrap();
        assert_eq!(u.channel_policy(1, 7).unwrap().minimum_level, Level::Trace);
        let mut targets = [0; 2];
        assert_eq!(u.route_targets(1, 7, &mut targets), Ok(2));
        assert_eq!(targets, [10, 20]);
        let mut short = [999];
        assert_eq!(
            u.route_targets(1, 7, &mut short),
            Err(ConfigUpdateError::BufferTooSmall)
        );
        assert_eq!(short, [999]);
        assert!(v.information_loss && !v.restart_required);
        assert_eq!(u.posted_revision().unwrap(), 1);
        let (mut p, _) = s.next().unwrap();
        accepted(p.try_record_small(meta(), &[1]).unwrap());
        assert_eq!(
            u.apply_config(0, &patch()),
            Err(ConfigUpdateError::Conflict)
        );
        assert_eq!(u.apply_config(1, &patch()), Ok(2));
        assert_eq!(
            u.apply_config(1, &patch()),
            Err(ConfigUpdateError::Conflict)
        );
    });
}
#[test]
fn producer_boundaries_apply_partially_and_idle_producer_keeps_busy() {
    let mut o = owner(1);
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        let (mut a, _) = s.next().unwrap();
        let (mut b, _) = s.next().unwrap();
        assert_eq!(u.apply_config(1, &patch()), Ok(2));
        assert_eq!(u.producer_revision(0), Some(1));
        assert_eq!(a.try_record_small(meta(), &[1]), Ok(RecordOutcome::Skipped));
        assert_eq!(u.producer_revision(0), Some(2));
        assert_eq!(u.producer_revision(1), Some(1));
        assert_eq!(
            u.apply_config(2, &restore()),
            Err(ConfigUpdateError::RevisionBusy)
        );
        assert_eq!(b.apply_pending_config(), 2);
        assert_eq!(u.apply_config(2, &restore()), Ok(3));
    });
}
#[test]
fn accepted_old_route_and_active_delivery_keep_old_plan_alive() {
    let plan = plan();
    let mut o = owner(1);
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        let completion = s.completion().unwrap();
        let (mut a, ar) = s.next().unwrap();
        let (mut b, br) = s.next().unwrap();
        let old = accepted(a.try_record_small(meta(), &[1]).unwrap());
        let change = ConfigPatch {
            routes: vec![dot_logger::routing::Route {
                channel_id: 7,
                targets: vec![20],
            }],
            ..Default::default()
        };
        assert_eq!(u.apply_config(1, &change), Ok(2));
        let new = accepted(a.try_record_small(meta(), &[2]).unwrap());
        b.apply_pending_config();
        assert_eq!(
            u.apply_config(
                2,
                &ConfigPatch {
                    routes: routing().routes,
                    ..Default::default()
                }
            ),
            Err(ConfigUpdateError::RevisionBusy)
        );
        assert_eq!(
            completion.poll(new, 10, Boundary::Written).unwrap().outcome,
            QueryOutcome::NotRouted
        );
        let mut router = Router::new(&plan, vec![ar, br], routing()).unwrap();
        router.scope(|mut d, mut readers| {
            assert!(
                d.turn(std::time::Instant::now(), &mut [None; 16])
                    .unwrap()
                    .fault
                    .is_none()
            );
            let old_a = readers[0]
                .try_next(std::time::Instant::now())
                .unwrap()
                .unwrap();
            let old_b = readers[1]
                .try_next(std::time::Instant::now())
                .unwrap()
                .unwrap();
            assert_eq!(old_a.record().receipt, old);
            assert_eq!(old_b.record().revision, 1);
            completion
                .report(old, 10, Update::Confirmed(Boundary::Written))
                .unwrap();
            completion
                .report(old, 20, Update::Confirmed(Boundary::Written))
                .unwrap();
            drop(old_b);
            assert_eq!(
                u.apply_config(
                    2,
                    &ConfigPatch {
                        routes: routing().routes,
                        ..Default::default()
                    }
                ),
                Err(ConfigUpdateError::RevisionBusy)
            );
            drop(old_a);
            assert_eq!(
                u.apply_config(
                    2,
                    &ConfigPatch {
                        routes: routing().routes,
                        ..Default::default()
                    }
                ),
                Ok(3)
            );
        });
    });
}

#[test]
fn sampling_is_explicit_and_important_bypasses_it_invalid_raw_keeps_loan() {
    use dot_logger::pool::RawRecordOutcome;
    let mut o = owner(1);
    o.enable_telemetry().unwrap();
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        let (mut a, mut ar) = s.next().unwrap();
        let (mut b, _) = s.next().unwrap();
        let sampling = ConfigPatch {
            channels: vec![ChannelPatch {
                channel_id: 7,
                minimum_level: Level::Trace,
                sample_every: 2,
            }],
            ..Default::default()
        };
        u.apply_config(1, &sampling).unwrap();
        b.apply_pending_config();
        accepted(a.try_record_small(meta(), &[1]).unwrap());
        ar.try_read(&mut [0; 16]).unwrap();
        let mut m = meta();
        m.schema_version = 99;
        assert_eq!(
            a.try_record_small(m, &[1]),
            Err(CaptureError::SchemaMismatch)
        );
        assert_eq!(a.try_record_small(meta(), &[1]), Ok(RecordOutcome::Skipped));
        let mut important = meta();
        important.importance = Some(Importance::Important);
        accepted(a.try_record_small(important, &[1]).unwrap());
        ar.try_read(&mut [0; 16]).unwrap();
        let loan = a.try_acquire(4, Importance::Normal).unwrap();
        let id = loan.identity();
        let error = b.try_record_raw(meta(), loan).unwrap_err();
        assert_eq!(error.reason, CaptureError::InvalidInput);
        assert_eq!(error.loan.identity(), id);
        let outcome = a.try_record_raw(meta(), error.loan).unwrap();
        assert!(matches!(outcome, RawRecordOutcome::Accepted(_)));
        let raw = ar.try_take_raw().unwrap().unwrap();
        assert_eq!(raw.record().revision, 2);
        drop(raw);
        ar.reclaim_raw(2);
        assert_eq!(ar.raw_snapshot().unwrap().free, 2);
    });
}
#[test]
fn raw_original_guard_outlives_ingress_and_blocks_reuse() {
    use dot_logger::pool::RawRecordOutcome;
    let mut o = owner(1);
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        let c = s.completion().unwrap();
        let (mut a, mut ar) = s.next().unwrap();
        let (mut b, _) = s.next().unwrap();
        let loan = a_loan(&mut a);
        let RawRecordOutcome::Accepted(receipt) = a.try_record_raw(meta(), loan).unwrap() else {
            panic!("raw")
        };
        u.apply_config(1, &patch()).unwrap();
        a.apply_pending_config();
        b.apply_pending_config();
        let raw = ar.try_take_raw().unwrap().unwrap();
        c.report(receipt, 10, Update::Confirmed(Boundary::Written))
            .unwrap();
        c.report(receipt, 20, Update::Confirmed(Boundary::Written))
            .unwrap();
        assert_eq!(
            u.apply_config(2, &restore()),
            Err(ConfigUpdateError::RevisionBusy)
        );
        drop(raw);
        assert_eq!(u.apply_config(2, &restore()), Ok(3));
    });
}
fn a_loan<'a>(a: &mut Producer<'a>) -> dot_logger::pool::Loan<'a> {
    a.try_acquire(4, Importance::Normal).unwrap()
}
#[test]
fn malformed_restart_exhaustion_and_closed_requests_do_not_publish() {
    let mut o = owner(u64::MAX);
    o.scope(|s| {
        let u = s.config_updates().unwrap();
        let mut invalid = patch();
        invalid.channels[0].sample_every = 0;
        assert_eq!(
            u.apply_config(u64::MAX, &invalid),
            Err(ConfigUpdateError::InvalidPatch)
        );
        let mut memory = plan().config().memory.clone();
        memory.control_bytes += 1;
        let restart = ConfigPatch {
            memory: Some(memory),
            ..Default::default()
        };
        assert!(
            u.validate_config(u64::MAX, &restart)
                .unwrap()
                .restart_required
        );
        assert_eq!(
            u.apply_config(u64::MAX, &restart),
            Err(ConfigUpdateError::RestartRequired)
        );
        assert_eq!(
            u.apply_config(u64::MAX, &patch()),
            Err(ConfigUpdateError::RevisionExhausted)
        );
        s.control().close();
        assert_eq!(
            u.apply_config(u64::MAX, &patch()),
            Err(ConfigUpdateError::Closing)
        );
        assert_eq!(u.posted_revision().unwrap(), u64::MAX);
    });
}
#[test]
fn telemetry_never_labels_a_transition_or_cross_revision_delta_as_stable() {
    use dot_logger::telemetry::{Observation, Unavailable};
    let mut o = owner(1);
    o.enable_telemetry().unwrap();
    o.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let c = s.completion().unwrap();
        let u = s.config_updates().unwrap();
        let (mut a, _) = s.next().unwrap();
        let (mut b, _) = s.next().unwrap();
        t.sample(0, Some(c)).unwrap();
        u.apply_config(1, &patch()).unwrap();
        a.apply_pending_config();
        assert_eq!(
            t.sample(1_000_000_000, Some(c)).unwrap().capture,
            Observation::Unavailable(Unavailable::RevisionTransition)
        );
        b.apply_pending_config();
        let sample = t.sample(2_000_000_000, Some(c)).unwrap();
        assert_eq!(sample.revision, 2);
        for sec in 3..=10 {
            t.sample(sec * 1_000_000_000, Some(c)).unwrap();
        }
        assert_eq!(
            t.window(10),
            Observation::Unavailable(Unavailable::IncompleteWindow)
        );
        for sec in 11..=12 {
            t.sample(sec * 1_000_000_000, Some(c)).unwrap();
        }
        let Observation::Observed(window) = t.window(10) else {
            panic!("window")
        };
        assert_eq!(window.revision, 2);
        assert_eq!(window.from_ns, 2_000_000_000);
    });
}

#[test]
fn configuration_state_and_sampling_survive_new_borrow_scopes() {
    let mut o = owner(1);
    let sampling = ConfigPatch {
        channels: vec![ChannelPatch {
            channel_id: 7,
            minimum_level: Level::Trace,
            sample_every: 2,
        }],
        routes: vec![dot_logger::routing::Route {
            channel_id: 7,
            targets: vec![],
        }],
        ..Default::default()
    };
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        u.apply_config(1, &sampling).unwrap();
        let (mut a, mut ar) = s.next().unwrap();
        let (mut b, _) = s.next().unwrap();
        b.apply_pending_config();
        accepted(a.try_record_small(meta(), &[1]).unwrap());
        ar.try_read(&mut [0; 16]).unwrap();
    });
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        let (mut a, _) = s.next().unwrap();
        assert_eq!(a.try_record_small(meta(), &[1]), Ok(RecordOutcome::Skipped));
        assert_eq!(u.apply_config(2, &restore()), Ok(3));
    });
}
#[test]
fn enabling_after_earlier_capture_and_drain_uses_existing_consumed_watermark() {
    let mut o = CaptureOwner::new_tracked(
        &plan(),
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(8),
    )
    .unwrap();
    o.scope(|mut s| {
        let c = s.completion().unwrap();
        let (mut a, mut ar) = s.next().unwrap();
        let r = accepted(a.try_record_small(meta(), &[1]).unwrap());
        ar.try_read(&mut [0; 16]).unwrap();
        for t in [10, 20] {
            c.report(r, t, Update::Confirmed(Boundary::Written))
                .unwrap();
        }
    });
    o.enable_config_updates(&routing()).unwrap();
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        u.apply_config(1, &patch()).unwrap();
        let (mut a, _) = s.next().unwrap();
        let (mut b, _) = s.next().unwrap();
        a.apply_pending_config();
        b.apply_pending_config();
        assert_eq!(u.apply_config(2, &restore()), Ok(3));
    });
}
#[test]
fn rejected_patches_and_insufficient_control_reservation_leave_owner_usable() {
    let mut o = owner(1);
    o.scope(|s| {
        let u = s.config_updates().unwrap();
        for change in [
            ConfigPatch {
                channels: vec![ChannelPatch {
                    channel_id: 999,
                    ..patch().channels[0]
                }],
                ..Default::default()
            },
            ConfigPatch {
                channels: vec![patch().channels[0]; 2],
                ..Default::default()
            },
            ConfigPatch {
                routes: vec![dot_logger::routing::Route {
                    channel_id: 7,
                    targets: vec![999],
                }],
                ..Default::default()
            },
            ConfigPatch {
                routes: vec![dot_logger::routing::Route {
                    channel_id: 7,
                    targets: vec![10, 10],
                }],
                ..Default::default()
            },
            ConfigPatch::default(),
        ] {
            assert_eq!(
                u.validate_config(1, &change),
                Err(ConfigUpdateError::InvalidPatch)
            );
            assert_eq!(u.posted_revision().unwrap(), 1);
        }
    });
    let used = o.control_reserved_bytes();
    let mut config = plan().config().clone();
    config.memory.control_bytes = used - 1;
    let p = dot_logger::config::SessionBuilder::new(config)
        .validate(&dot_logger::platform::PlatformCapabilities::default())
        .unwrap();
    let mut other = CaptureOwner::new_tracked(
        &p,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(8),
    )
    .unwrap();
    assert!(matches!(
        other.enable_config_updates(&routing()),
        Err(ConfigUpdateError::Config(_))
    ));
    other.scope(|s| assert!(s.config_updates().is_none()));
}

#[test]
fn observed_before_after_windows_measure_recording_reduction_not_generation_work() {
    use dot_logger::telemetry::{
        Observation,
        capacity::{Conditions, Value},
        compare::{Comparison, Direction, RateChange, Snapshot},
        report::{Report, ReportInput},
    };
    let mut routes = routing();
    routes.routes[0].targets.clear();
    let mut o = CaptureOwner::new_tracked(
        &plan(),
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        tracking(8),
    )
    .unwrap();
    o.enable_config_updates(&routes).unwrap();
    o.enable_telemetry().unwrap();
    o.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let c = s.completion().unwrap();
        let u = s.config_updates().unwrap();
        let (mut a, mut ar) = s.next().unwrap();
        let (mut b, _) = s.next().unwrap();
        t.sample(0, Some(c)).unwrap();
        for sec in 1..=10 {
            for _ in 0..2 {
                accepted(a.try_record_small(meta(), &[1]).unwrap());
                ar.try_read(&mut [0; 16]).unwrap();
            }
            t.sample(sec * 1_000_000_000, Some(c)).unwrap();
        }
        let Observation::Observed(before) = t.window(10) else {
            panic!("before")
        };
        u.apply_config(
            1,
            &ConfigPatch {
                channels: vec![ChannelPatch {
                    channel_id: 7,
                    minimum_level: Level::Trace,
                    sample_every: 2,
                }],
                ..Default::default()
            },
        )
        .unwrap();
        a.apply_pending_config();
        b.apply_pending_config();
        t.sample(11_000_000_000, Some(c)).unwrap();
        for sec in 12..=21 {
            for _ in 0..2 {
                let _ = a.try_record_small(meta(), &[1]).unwrap();
                ar.try_read(&mut [0; 16]).unwrap();
            }
            t.sample(sec * 1_000_000_000, Some(c)).unwrap();
        }
        let Observation::Observed(after) = t.window(10) else {
            panic!("after")
        };
        assert_eq!(
            (
                before.counts.accepted,
                after.counts.accepted,
                after.counts.skipped
            ),
            (20, 10, 10)
        );
        let report = |window: dot_logger::telemetry::Window| {
            Report::build(
                ReportInput::from_window(
                    window,
                    Conditions {
                        environment_id: 1,
                        workload_id: 2,
                        revision: window.revision,
                        target_id: 10,
                        boundary: Boundary::Written,
                    },
                ),
                None,
                window.to_ns,
            )
        };
        let comparison = Comparison::build(
            Snapshot::new(report(before), 99),
            Snapshot::new(report(after), 99),
        );
        assert!(comparison.differences.comparable());
        assert!(matches!(
            comparison.accepted_rate,
            Value::Observed(RateChange {
                direction: Direction::Decrease,
                ..
            })
        ));
        assert!(matches!(
            comparison.intentional_skip_rate,
            Value::Observed(RateChange {
                direction: Direction::Increase,
                ..
            })
        ));
        assert!(matches!(comparison.cpu_rate, Value::Unavailable(_)));
    });
}

#[test]
fn concurrent_publication_preserves_whole_revision_routes_and_completion() {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;
    let p = plan();
    let mut o = owner(1);
    o.scope(|mut s| {
        let u = s.config_updates().unwrap();
        let c = s.completion().unwrap();
        let (mut a, ar) = s.next().unwrap();
        let (mut b, br) = s.next().unwrap();
        let mut router = Router::new(&p, vec![ar, br], routing()).unwrap();
        let stage = AtomicU64::new(1);
        let ack_a = AtomicU64::new(1);
        let ack_b = AtomicU64::new(1);
        std::thread::scope(|threads| {
            let produce = |producer: &mut Producer<'_>, ack: &AtomicU64| {
                for revision in 2..=6 {
                    while stage.load(Ordering::Acquire) < revision {
                        std::thread::yield_now();
                    }
                    let mut m = meta();
                    m.level = Level::Warn;
                    for _ in 0..2 {
                        accepted(producer.try_record_small(m, &[1]).unwrap());
                    }
                    ack.store(revision, Ordering::Release);
                }
            };
            let ack_first = &ack_a;
            let ack_second = &ack_b;
            let first = threads.spawn(move || produce(&mut a, ack_first));
            let second = threads.spawn(move || produce(&mut b, ack_second));
            router.scope(|mut d, mut readers| {
                for revision in 2..=6 {
                    let patch = ConfigPatch {
                        channels: vec![ChannelPatch {
                            channel_id: 7,
                            minimum_level: if revision % 2 == 0 {
                                Level::Info
                            } else {
                                Level::Warn
                            },
                            sample_every: 1,
                        }],
                        routes: vec![dot_logger::routing::Route {
                            channel_id: 7,
                            targets: vec![if revision % 2 == 0 { 20 } else { 10 }],
                        }],
                        ..Default::default()
                    };
                    assert_eq!(u.apply_config(revision - 1, &patch), Ok(revision));
                    stage.store(revision, Ordering::Release);
                    while ack_a.load(Ordering::Acquire) < revision
                        || ack_b.load(Ordering::Acquire) < revision
                    {
                        std::thread::yield_now();
                    }
                    let mut delivered = 0;
                    for _ in 0..4 {
                        let report = d.turn(Instant::now(), &mut [None; 16]).unwrap();
                        assert!(report.fault.is_none());
                        for (index, reader) in readers.iter_mut().enumerate() {
                            while let Some(result) = reader.try_next(Instant::now()) {
                                let delivery = result.unwrap();
                                assert_eq!(delivery.record().revision, revision);
                                assert_eq!(index, if revision % 2 == 0 { 1 } else { 0 });
                                c.report(
                                    delivery.record().receipt,
                                    if index == 0 { 10 } else { 20 },
                                    Update::Confirmed(Boundary::Written),
                                )
                                .unwrap();
                                delivered += 1;
                            }
                        }
                    }
                    assert_eq!(delivered, 4);
                }
            });
            first.join().unwrap();
            second.join().unwrap();
        });
        assert_eq!(c.totals(false).complete, 20);
    });
}

#[test]
fn concurrent_same_base_applies_have_one_winner_and_one_conflict() {
    let mut o = owner(1);
    o.scope(|s| {
        let u = s.config_updates().unwrap();
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|threads| {
            let first = threads.spawn(|| {
                barrier.wait();
                u.apply_config(1, &patch())
            });
            let second = threads.spawn(|| {
                barrier.wait();
                u.apply_config(1, &patch())
            });
            let mut results = vec![first.join().unwrap(), second.join().unwrap()];
            assert_eq!(results.iter().filter(|r| **r == Ok(2)).count(), 1);
            results.retain(|r| r.is_err());
            assert_eq!(results, vec![Err(ConfigUpdateError::Conflict)]);
        });
    });
}
