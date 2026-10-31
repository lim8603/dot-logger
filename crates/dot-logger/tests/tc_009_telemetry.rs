//! TC-009 local counters/history; no throughput or device acceptance claim.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    capture::*,
    completion::{Boundary, Update},
    pool::RawRecordOutcome,
    telemetry::{Observation, TelemetryError, Unavailable},
};
use support::*;

fn observed<T: std::fmt::Debug>(value: Observation<T>) -> T {
    match value {
        Observation::Observed(value) => value,
        other => panic!("{other:?}"),
    }
}
#[test]
fn capture_small_raw_skip_and_reject_totals_are_exact_after_drain() {
    let plan = plan();
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Info;
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap();
    assert!(owner.telemetry().is_none());
    let before = owner.control_reserved_bytes();
    owner.enable_telemetry().unwrap();
    assert!(owner.control_reserved_bytes() > before);
    owner.scope(|mut s| {
        let telemetry = s.telemetry().unwrap();
        let (mut p, mut r) = s.next().unwrap();
        let mut m = meta();
        m.level = Level::Debug;
        assert_eq!(p.try_record_small(m, &[1]), Ok(RecordOutcome::Skipped));
        assert_eq!(
            p.try_record_small(meta(), &[0; 17]),
            Err(CaptureError::TooLarge)
        );
        accepted(p.try_record_small(meta(), &[1, 2, 3]).unwrap());
        r.try_read(&mut [0; 16]).unwrap().unwrap();
        let loan = p.try_acquire(4, Importance::Normal).unwrap();
        assert!(matches!(
            p.try_record_raw(meta(), loan).unwrap(),
            RawRecordOutcome::Accepted(_)
        ));
        drop(r.try_take_raw().unwrap().unwrap());
        r.reclaim_raw(2);
        assert!(p.try_acquire(65, Importance::Normal).is_err());
        let counts = observed(telemetry.producer(0));
        assert_eq!(counts.acquire_rejected, 1);
        assert_eq!(counts.rejected_contract, 1);
        assert_eq!(counts.acquire_rejected_contract, 1);
        assert_eq!(counts.rejected_capacity, 0);
        assert_eq!(
            (
                counts.accepted,
                counts.rejected,
                counts.skipped,
                counts.accepted_bytes
            ),
            (2, 1, 1, 7)
        );
        assert_eq!(
            telemetry.producer(99),
            Observation::Unavailable(Unavailable::Unregistered)
        );
        let sample = telemetry.sample(0, None).unwrap();
        assert_eq!(observed(sample.capture), counts);
        assert_eq!(
            sample.outputs,
            Observation::Unavailable(Unavailable::MissingCompletion)
        );
    });
}
#[test]
fn history_has_bounded_windows_and_reset_never_becomes_a_large_delta() {
    let mut owner = CaptureOwner::new(&plan(), 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.enable_telemetry().unwrap();
    owner.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let (mut p, mut r) = s.next().unwrap();
        for second in 0..=310 {
            accepted(p.try_record_small(meta(), &[1]).unwrap());
            r.try_read(&mut [0; 16]).unwrap();
            t.sample(second * 1_000_000_000, None).unwrap();
        }
        assert_eq!(
            t.history_len(),
            dot_logger::telemetry::Observation::Observed(301)
        );
        for seconds in [10, 60, 300] {
            let window = observed(t.window(seconds));
            assert_eq!(window.counts.accepted, u64::from(seconds));
        }
        assert_eq!(t.sample(1, None), Err(TelemetryError::TimeOrder));
        assert!(matches!(t.latest(313_000_000_000), Observation::Stale(_)));
    });
    let epoch = owner.telemetry().unwrap().epoch();
    owner.reset_telemetry().unwrap();
    let t = owner.telemetry().unwrap();
    assert!(t.epoch() > epoch);
    assert_eq!(observed(t.producer(0)).accepted, 0);
    assert_eq!(
        t.window(10),
        Observation::Unavailable(Unavailable::IncompleteWindow)
    );
    t.sample(400_000_000_000, None).unwrap();
    assert_eq!(
        t.history_len(),
        dot_logger::telemetry::Observation::Observed(1)
    );
}
#[test]
fn target_totals_reuse_completion_authority_and_preserve_failure_and_unknown() {
    let mut owner = CaptureOwner::new_tracked(
        &plan(),
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(4),
    )
    .unwrap();
    owner.enable_telemetry().unwrap();
    owner.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let c = s.completion().unwrap();
        let (mut p, mut r) = s.next().unwrap();
        for _ in 0..8 {
            let receipt = accepted(p.try_record_small(meta(), &[1]).unwrap());
            r.try_read(&mut [0; 16]).unwrap();
            c.report(receipt, 10, Update::Confirmed(Boundary::Written))
                .unwrap();
            c.report(receipt, 20, Update::Unknown).unwrap();
        }
        let all = observed(t.sample(0, Some(c)).unwrap().outputs);
        assert_eq!(
            (all.records, all.assigned, all.complete, all.unknown),
            (8, 16, 8, 8)
        );
        let a = c.target_totals(10).unwrap();
        let b = c.target_totals(20).unwrap();
        assert_eq!(
            (a.assigned, a.complete, b.assigned, b.unknown),
            (8, 8, 8, 8)
        );
        assert!(c.target_totals(999).is_none());
    });
}
#[test]
fn telemetry_reservation_checks_combined_control_budget_before_allocating() {
    use dot_logger::{config::SessionBuilder, platform::PlatformCapabilities};
    let mut config = plan().config().clone();
    config.memory.control_bytes = 4096;
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    let before = owner.control_reserved_bytes();
    assert!(owner.enable_telemetry().is_err());
    assert_eq!(owner.control_reserved_bytes(), before);
    assert!(owner.telemetry().is_none());
}

#[test]
fn missing_intervals_and_foreign_completion_are_unavailable() {
    let plan = plan();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.enable_telemetry().unwrap();
    let other = CaptureOwner::new_tracked(
        &plan,
        43,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(4),
    )
    .unwrap();
    let t = owner.telemetry().unwrap();
    let sample = t.sample(0, other.completion()).unwrap();
    assert_eq!(
        sample.outputs,
        Observation::Unavailable(Unavailable::ForeignSession)
    );
    t.sample(10_000_000_000, None).unwrap();
    assert_eq!(
        t.window(10),
        Observation::Unavailable(Unavailable::IncompleteWindow)
    );
    assert_eq!(
        t.sample(10_000_000_001, None),
        Err(TelemetryError::TimeOrder)
    );
    assert_eq!(
        t.history_len(),
        dot_logger::telemetry::Observation::Observed(2)
    );
}
#[test]
fn concurrent_management_reads_never_modify_producer_counts() {
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Fatal;
    let mut owner = CaptureOwner::new(&plan(), 42, 1, &[channel]).unwrap();
    owner.enable_telemetry().unwrap();
    owner.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let (mut p, _) = s.next().unwrap();
        std::thread::scope(|threads| {
            let worker = threads.spawn(move || {
                for _ in 0..128 {
                    assert_eq!(p.try_record_small(meta(), &[1]), Ok(RecordOutcome::Skipped));
                }
            });
            for _ in 0..128 {
                assert!(observed(t.producer(0)).skipped <= 128);
            }
            worker.join().unwrap();
        });
        assert_eq!(observed(t.producer(0)).skipped, 128);
    });
}

#[test]
fn report_consumes_retained_window_and_preserves_unmeasured_resources() {
    use dot_logger::telemetry::{capacity::*, report::*};
    let mut owner = CaptureOwner::new(&plan(), 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.enable_telemetry().unwrap();
    owner.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let (mut p, mut r) = s.next().unwrap();
        t.sample(0, None).unwrap();
        for second in 1..=10 {
            accepted(p.try_record_small(meta(), &[1, 2, 3]).unwrap());
            r.try_read(&mut [0; 16]).unwrap().unwrap();
            t.sample(second * 1_000_000_000, None).unwrap();
        }
        let window = observed(t.window(10));
        let input = ReportInput::from_window(
            window,
            Conditions {
                environment_id: 100,
                workload_id: 200,
                revision: 1,
                target_id: 10,
                boundary: Boundary::Written,
            },
        );
        let report = Report::build(input, None, 10_000_000_000);
        assert_eq!(
            report.accepted_rate,
            Value::Observed(Rate {
                amount: 10,
                elapsed_ns: 10_000_000_000
            })
        );
        assert_eq!(
            report.accepted_bytes_rate,
            Value::Observed(Rate {
                amount: 30,
                elapsed_ns: 10_000_000_000
            })
        );
        assert_eq!(report.load, Value::Unavailable(Reason::MissingCapacity));
        assert_eq!(
            report.buffer_limit,
            Value::Unavailable(Reason::MissingMeasurement)
        );
        assert_eq!(report.window.epoch, t.epoch());
        assert_eq!(report.window.session_id, 42);
    });
}

#[test]
fn contract_and_lifecycle_errors_are_not_capacity_pressure_or_load() {
    use dot_logger::telemetry::{capacity::*, report::*};
    for kind in 0..4 {
        let mut config = plan().config().clone();
        if kind == 2 {
            config.producers[0].raw = None;
        }
        let plan = dot_logger::config::SessionBuilder::new(config)
            .validate(&dot_logger::platform::PlatformCapabilities::default())
            .unwrap();
        let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        owner.enable_telemetry().unwrap();
        owner.scope(|mut s| {
            let t = s.telemetry().unwrap();
            let control = s.control();
            let (mut p, _) = s.next().unwrap();
            t.sample(0, None).unwrap();
            match kind {
                0 => assert_eq!(
                    p.try_record_small(meta(), &[0; 17]),
                    Err(CaptureError::TooLarge)
                ),
                1 => {
                    control.close();
                    assert_eq!(p.try_record_small(meta(), &[1]), Err(CaptureError::Closing));
                }
                2 => assert_eq!(
                    p.try_acquire(1, Importance::Normal).err(),
                    Some(CaptureError::Unsupported)
                ),
                _ => {
                    control.close();
                    assert_eq!(
                        p.try_acquire(1, Importance::Normal).err(),
                        Some(CaptureError::Closing)
                    );
                }
            }
            for second in 1..=10 {
                t.sample(second * 1_000_000_000, None).unwrap();
            }
            let input = ReportInput::from_window(
                observed(t.window(10)),
                Conditions {
                    environment_id: 1,
                    workload_id: 2,
                    revision: 1,
                    target_id: 10,
                    boundary: Boundary::Written,
                },
            );
            let profile = Profile {
                id: 1,
                session_id: 42,
                conditions: input.conditions,
                valid_from_ns: 0,
                valid_until_ns: 20_000_000_000,
                completed_records: Rate {
                    amount: 1,
                    elapsed_ns: 10_000_000_000,
                },
            };
            let report = Report::build(input, Some(profile), 10_000_000_000);
            assert_ne!(
                report.pressure,
                Value::Observed(Pressure::Overloaded),
                "kind={kind}"
            );
            let Value::Estimated { value: ratio, .. } = report.load else {
                panic!("load")
            };
            assert_eq!(ratio.numerator, 0, "kind={kind}");
        });
    }
}

#[test]
fn capacity_attempts_preserve_pressure_and_raw_retries_stay_explicit() {
    use dot_logger::telemetry::{capacity::*, report::*};
    let mut owner = CaptureOwner::new(&plan(), 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.enable_telemetry().unwrap();
    owner.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let control = s.control();
        let (mut p, _) = s.next().unwrap();
        t.sample(0, None).unwrap();
        for _ in 0..8 {
            accepted(p.try_record_small(meta(), &[1]).unwrap());
        }
        let mut loan = p.try_acquire(1, Importance::Normal).unwrap();
        let other = p.try_acquire(1, Importance::Normal).unwrap();
        assert_eq!(
            p.try_acquire(1, Importance::Normal).err(),
            Some(CaptureError::Capacity)
        );
        for _ in 0..3 {
            let error = p.try_record_raw(meta(), loan).unwrap_err();
            assert_eq!(error.reason, CaptureError::Capacity);
            loan = error.loan;
        }
        let mut wrong = meta();
        wrong.schema_id += 1;
        let error = p.try_record_raw(wrong, loan).unwrap_err();
        assert_eq!(error.reason, CaptureError::SchemaMismatch);
        control.close();
        let error = p.try_record_raw(meta(), error.loan).unwrap_err();
        assert_eq!(error.reason, CaptureError::Closing);
        drop((error.loan, other));
        for second in 1..=10 {
            t.sample(second * 1_000_000_000, None).unwrap();
        }
        let window = observed(t.window(10));
        let c = window.counts;
        assert_eq!((c.accepted, c.rejected, c.acquire_rejected), (8, 5, 1));
        assert_eq!(
            (
                c.rejected_capacity,
                c.rejected_contract,
                c.rejected_lifecycle
            ),
            (3, 1, 1)
        );
        assert_eq!(c.acquire_rejected_capacity, 1);
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
        let profile = Profile {
            id: 1,
            session_id: 42,
            conditions: input.conditions,
            valid_from_ns: 0,
            valid_until_ns: 20_000_000_000,
            completed_records: Rate {
                amount: 100,
                elapsed_ns: 10_000_000_000,
            },
        };
        let report = Report::build(input, Some(profile), 10_000_000_000);
        assert_eq!(report.pressure, Value::Observed(Pressure::Overloaded));
        assert_eq!(report.rejected_contract, Value::Observed(1));
        assert_eq!(report.rejected_lifecycle, Value::Observed(1));
        let Value::Estimated { value: ratio, .. } = report.load else {
            panic!("load")
        };
        // 8 accepted + 3 capacity attempts, not 13 total submissions or 12 with acquire.
        assert_eq!(ratio.numerator * 100, ratio.denominator * 11);
    });
    owner.reset_telemetry().unwrap();
    assert_eq!(
        observed(owner.telemetry().unwrap().producer(0)),
        Default::default()
    );
}

#[test]
fn host_aligns_target_completion_history_to_actual_window_endpoints() {
    use dot_logger::telemetry::{capacity::*, report::*};
    use dot_logger::{completion::FailureReason, routing::DropReason};
    let mut owner = CaptureOwner::new_tracked(
        &plan(),
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(8),
    )
    .unwrap();
    owner.enable_telemetry().unwrap();
    owner.scope(|mut s| {
        let t = s.telemetry().unwrap();
        let c = s.completion().unwrap();
        let (mut p, mut r) = s.next().unwrap();
        // Baseline already contains pending work: final pending must not be differenced.
        accepted(p.try_record_small(meta(), &[1]).unwrap());
        r.try_read(&mut [0; 16]).unwrap().unwrap();
        let baseline = t.sample(0, Some(c)).unwrap();
        let mut host_history = vec![(baseline, c.target_totals(10).unwrap())];
        for second in [2, 4, 6, 8, 10, 11] {
            let receipt = accepted(p.try_record_small(meta(), &[1]).unwrap());
            r.try_read(&mut [0; 16]).unwrap().unwrap();
            let update = match second {
                2 | 11 => Some(Update::Confirmed(Boundary::Written)),
                4 => Some(Update::Failed(FailureReason::IoFailure)),
                6 => Some(Update::Dropped(DropReason::Capacity)),
                8 => Some(Update::Unknown),
                _ => None,
            };
            if let Some(update) = update {
                c.report(receipt, 10, update).unwrap();
            }
            c.report(receipt, 20, Update::Unknown).unwrap();
            let sample = t.sample(second * 1_000_000_000, Some(c)).unwrap();
            host_history.push((sample, c.target_totals(10).unwrap()));
        }
        let window = observed(t.window(10));
        assert_eq!((window.from_ns, window.to_ns), (0, 11_000_000_000));
        let endpoint = |at_ns| {
            host_history.iter().find(|(sample, _)| {
                sample.session_id == window.session_id
                    && sample.revision == window.revision
                    && sample.epoch == window.epoch
                    && sample.at_ns == at_ns
            })
        };
        let (Some((_, start)), Some((_, end))) = (endpoint(window.from_ns), endpoint(window.to_ns))
        else {
            panic!("missing endpoints must remain unavailable")
        };
        assert!(!start.overflowed && !end.overflowed && start.busy == 0 && end.busy == 0);
        let mut input = ReportInput::from_window(
            window,
            Conditions {
                environment_id: 1,
                workload_id: 2,
                revision: 1,
                target_id: 10,
                boundary: Boundary::Written,
            },
        );
        // Both snapshots came from target 10's configured Written boundary.
        let delta = OutputDelta {
            complete: end.complete.checked_sub(start.complete).unwrap(),
            failed: end.failed.checked_sub(start.failed).unwrap(),
            dropped: end.dropped.checked_sub(start.dropped).unwrap(),
            unknown: end.unknown.checked_sub(start.unknown).unwrap(),
            pending: end.pending,
        };
        input.output = Observation::Observed(delta);
        let report = Report::build(input, None, window.to_ns);
        assert_eq!(
            delta,
            OutputDelta {
                complete: 2,
                failed: 1,
                dropped: 1,
                unknown: 1,
                pending: 2
            }
        );
        assert_eq!(
            report.completed_rate,
            Value::Observed(Rate {
                amount: 2,
                elapsed_ns: 11_000_000_000
            })
        );
        assert_eq!(
            report.accepted_rate,
            Value::Observed(Rate {
                amount: 6,
                elapsed_ns: 11_000_000_000
            })
        );
        assert_eq!(report.output, Value::Observed(delta));
        assert_eq!(report.pressure, Value::Observed(Pressure::Overloaded));
        // A host that did not retain the real start cannot substitute to_ns - 10s.
        assert!(endpoint(window.to_ns - 10_000_000_000).is_none());
    });
}
