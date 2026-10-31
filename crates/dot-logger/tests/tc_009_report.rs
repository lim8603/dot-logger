//! TC-009 report arithmetic and evidence; synthetic rates are not device capacity.
#![allow(clippy::unwrap_used)]
use dot_logger::{
    completion::Boundary,
    telemetry::{
        CaptureCounts, Observation, Window,
        capacity::{Basis, Conditions, Forecast, Profile, Rate, Reason, Value},
        report::{BufferTrend, DiskTrend, OutputDelta, Pressure, Report, ReportInput},
    },
};
const SECOND: u64 = 1_000_000_000;
fn input() -> ReportInput {
    let window = Window {
        session_id: 42,
        revision: 7,
        epoch: 1,
        from_ns: 0,
        to_ns: 10 * SECOND,
        counts: CaptureCounts {
            accepted: 100,
            accepted_bytes: 1000,
            ..CaptureCounts::default()
        },
    };
    let mut i = ReportInput::from_window(
        window,
        Conditions {
            environment_id: 1,
            workload_id: 2,
            revision: 7,
            target_id: 10,
            boundary: Boundary::Written,
        },
    );
    i.output = Observation::Observed(OutputDelta {
        complete: 100,
        pending: 0,
        failed: 0,
        dropped: 0,
        unknown: 0,
    });
    i.buffer = Observation::Observed(BufferTrend {
        resource_id: 3,
        capacity_bytes: 1000,
        start_bytes: 100,
        end_bytes: 200,
        capacity_slots: 100,
        start_slots: 10,
        end_slots: 20,
        incoming_bytes: 200,
        incoming_slots: 20,
    });
    i.disk = Observation::Observed(DiskTrend {
        free_start_bytes: 10_000,
        free_end_bytes: 9000,
        reserve_bytes: 1000,
        quota_remaining_bytes: None,
        written_bytes: 2000,
        deleted_bytes: 1000,
        circular_budget_bytes: Some(4000),
        retained_span_ns: Some(20 * SECOND),
    });
    i
}
fn estimated<T: Copy + std::fmt::Debug>(v: Value<T>) -> T {
    match v {
        Value::Estimated { value, .. } => value,
        other => panic!("{other:?}"),
    }
}
#[test]
fn loss_stabilized_queue_stays_overloaded_without_a_capacity_denominator() {
    let mut i = input();
    i.window.counts.rejected = 3;
    i.window.counts.rejected_capacity = 3;
    if let Observation::Observed(ref mut b) = i.buffer {
        b.start_bytes = b.end_bytes;
        b.start_slots = b.end_slots;
    }
    let r = Report::build(i, None, 10 * SECOND);
    assert_eq!(r.pressure, Value::Observed(Pressure::Overloaded));
    assert_eq!(r.load, Value::Unavailable(Reason::MissingCapacity));
    assert_eq!(estimated(r.buffer_limit), Forecast::NoGrowth);
    assert_eq!(r.rejected, Value::Observed(3));
}

#[test]
fn noncapacity_failures_remain_diagnostic_without_inventing_pressure_or_load() {
    let mut i = input();
    i.window.counts.rejected = 1000;
    i.window.counts.rejected_contract = 600;
    i.window.counts.rejected_lifecycle = 400;
    i.window.counts.acquire_rejected = 1000;
    i.window.counts.acquire_rejected_contract = 700;
    i.window.counts.acquire_rejected_lifecycle = 300;
    if let Observation::Observed(ref mut b) = i.buffer {
        b.start_bytes = b.end_bytes;
        b.start_slots = b.end_slots;
    }
    let profile = Profile {
        session_id: 42,
        id: 9,
        conditions: i.conditions,
        valid_from_ns: 0,
        valid_until_ns: 100 * SECOND,
        completed_records: Rate {
            amount: 200,
            elapsed_ns: 10 * SECOND,
        },
    };
    let report = Report::build(i, Some(profile), 10 * SECOND);
    assert_eq!(
        report.pressure,
        Value::Observed(Pressure::NoPressureObserved)
    );
    assert_eq!(
        estimated(report.load).numerator * 2,
        estimated(report.load).denominator
    );
    assert_eq!(report.rejected, Value::Observed(1000));
    assert_eq!(report.rejected_contract, Value::Observed(600));
    assert_eq!(report.rejected_lifecycle, Value::Observed(400));
    assert_eq!(report.acquire_rejected_contract, Value::Observed(700));
    assert_eq!(report.acquire_rejected_lifecycle, Value::Observed(300));
    let stale = Report::build(i, Some(profile), 20 * SECOND);
    assert!(matches!(
        stale.rejected_contract,
        Value::Stale { value: 600, .. }
    ));
    i.window.counts.overflowed = true;
    let invalid = Report::build(i, Some(profile), 10 * SECOND);
    assert_eq!(
        invalid.rejected_contract,
        Value::Unavailable(Reason::Arithmetic)
    );
    assert_eq!(
        invalid.acquire_rejected_lifecycle,
        Value::Unavailable(Reason::Arithmetic)
    );
}
#[test]
fn zero_or_negative_growth_never_invents_a_finite_deadline() {
    let mut i = input();
    if let Observation::Observed(ref mut b) = i.buffer {
        b.start_bytes = 300;
        b.start_slots = 30;
    }
    if let Observation::Observed(ref mut d) = i.disk {
        d.free_start_bytes = d.free_end_bytes;
    }
    let r = Report::build(i, None, 10 * SECOND);
    assert_eq!(estimated(r.buffer_limit), Forecast::NoGrowth);
    assert_eq!(estimated(r.disk_exhaustion), Forecast::NoGrowth);
    assert_eq!(
        estimated(r.output_stopped_limit),
        Forecast::AfterNs(40 * SECOND)
    );
}
#[test]
fn byte_slot_disk_and_circular_retention_times_have_separate_meanings() {
    let mut i = input();
    if let Observation::Observed(ref mut b) = i.buffer {
        b.end_slots = 60;
    }
    let r = Report::build(i, None, 10 * SECOND);
    assert_eq!(
        estimated(r.buffer_bytes_limit),
        Forecast::AfterNs(80 * SECOND)
    );
    assert_eq!(
        estimated(r.buffer_slots_limit),
        Forecast::AfterNs(8 * SECOND)
    );
    assert_eq!(estimated(r.buffer_limit), Forecast::AfterNs(8 * SECOND));
    assert_eq!(estimated(r.disk_exhaustion), Forecast::AfterNs(80 * SECOND));
    assert_eq!(
        estimated(r.circular_retention),
        Forecast::AfterNs(20 * SECOND)
    );
    assert_eq!(r.retained_span_ns, Value::Observed(20 * SECOND));
}
#[test]
fn capacity_must_match_conditions_and_be_current() {
    let i = input();
    let p = Profile {
        session_id: 42,
        id: 9,
        conditions: i.conditions,
        valid_from_ns: 0,
        valid_until_ns: 100 * SECOND,
        completed_records: Rate {
            amount: 200,
            elapsed_ns: 10 * SECOND,
        },
    };
    let r = Report::build(i, Some(p), 10 * SECOND);
    assert!(matches!(r.capacity, Value::Validated { profile_id: 9, .. }));
    let ratio = estimated(r.load);
    assert_eq!(ratio.numerator * 2, ratio.denominator);
    let mut changed = p;
    changed.conditions.workload_id += 1;
    assert_eq!(
        Report::build(i, Some(changed), 10 * SECOND).load,
        Value::Unavailable(Reason::CapacityMismatch)
    );
    changed = p;
    changed.valid_until_ns = 9 * SECOND;
    let r = Report::build(i, Some(changed), 10 * SECOND);
    assert!(matches!(
        r.capacity,
        Value::Stale {
            basis: Basis::Validated(9),
            ..
        }
    ));
    assert_eq!(r.load, Value::Unavailable(Reason::ExpiredCapacity));
}

#[test]
fn quota_fill_uses_file_growth_even_when_other_filesystem_users_free_space() {
    let mut i = input();
    if let Observation::Observed(ref mut d) = i.disk {
        d.free_start_bytes = d.free_end_bytes - 100;
        d.quota_remaining_bytes = Some(500);
    }
    let r = Report::build(i, None, 10 * SECOND);
    assert_eq!(estimated(r.disk_exhaustion), Forecast::AfterNs(5 * SECOND));
}
#[test]
fn a_profile_from_another_session_cannot_reuse_monotonic_validity_times() {
    let i = input();
    let p = Profile {
        id: 9,
        session_id: 43,
        conditions: i.conditions,
        valid_from_ns: 0,
        valid_until_ns: 100 * SECOND,
        completed_records: Rate {
            amount: 200,
            elapsed_ns: 10 * SECOND,
        },
    };
    assert_eq!(
        Report::build(i, Some(p), 10 * SECOND).load,
        Value::Unavailable(Reason::CapacityMismatch)
    );
}

#[test]
fn output_loss_and_raw_acquisition_failure_are_not_hidden_by_flat_occupancy() {
    for kind in 0..4 {
        let mut i = input();
        if let Observation::Observed(ref mut b) = i.buffer {
            b.start_bytes = b.end_bytes;
            b.start_slots = b.end_slots;
        }
        if let Observation::Observed(ref mut o) = i.output {
            match kind {
                0 => o.failed = 1,
                1 => o.dropped = 1,
                2 => o.unknown = 1,
                _ => {
                    i.window.counts.acquire_rejected = 1;
                    i.window.counts.acquire_rejected_capacity = 1;
                }
            }
        }
        assert_eq!(
            Report::build(i, None, 10 * SECOND).pressure,
            Value::Observed(Pressure::Overloaded)
        );
    }
}
#[test]
fn intentional_skips_do_not_count_as_loss_or_offered_capacity_load() {
    let mut i = input();
    i.window.counts.skipped = 10_000;
    if let Observation::Observed(ref mut b) = i.buffer {
        b.start_bytes = b.end_bytes;
        b.start_slots = b.end_slots;
    }
    let p = Profile {
        session_id: 42,
        id: 9,
        conditions: i.conditions,
        valid_from_ns: 0,
        valid_until_ns: 100 * SECOND,
        completed_records: Rate {
            amount: 100,
            elapsed_ns: 10 * SECOND,
        },
    };
    let r = Report::build(i, Some(p), 10 * SECOND);
    assert_eq!(r.pressure, Value::Observed(Pressure::NoPressureObserved));
    assert_eq!(r.skipped, Value::Observed(10_000));
    let load = estimated(r.load);
    assert_eq!(load.numerator, load.denominator);
    i.window.counts.accepted = 101;
    assert_eq!(
        Report::build(i, Some(p), 10 * SECOND).pressure,
        Value::Estimated {
            value: Pressure::Overloaded,
            assumptions: dot_logger::telemetry::capacity::Assumption::MatchingCapacity
        }
    );
}
#[test]
fn missing_stale_and_changed_revision_never_turn_into_zero_or_current_estimates() {
    let i = input();
    let r = Report::build(i, None, 13 * SECOND);
    assert!(matches!(
        r.accepted_rate,
        Value::Stale {
            basis: Basis::Observed,
            ..
        }
    ));
    assert!(matches!(
        r.buffer_limit,
        Value::Stale {
            basis: Basis::Estimated(_),
            ..
        }
    ));
    assert_eq!(r.window, i.window);
    assert_eq!(r.measurements, i);
    let reversed = Report::build(i, None, 9 * SECOND);
    assert!(matches!(reversed.pressure, Value::Stale { .. }));
    let mut missing = ReportInput::from_window(i.window, i.conditions);
    let r = Report::build(missing, None, 10 * SECOND);
    assert_eq!(
        r.buffer_limit,
        Value::Unavailable(Reason::MissingMeasurement)
    );
    assert_eq!(r.pressure, Value::Unavailable(Reason::MissingMeasurement));
    if let Observation::Observed(b) = i.buffer {
        missing.buffer = Observation::Stale(b);
    }
    assert_eq!(
        Report::build(missing, None, 10 * SECOND).buffer_limit,
        Value::Unavailable(Reason::StaleMeasurement)
    );
    missing.conditions.revision += 1;
    assert_eq!(
        Report::build(missing, None, 10 * SECOND).accepted_rate,
        Value::Unavailable(Reason::InvalidMeasurement)
    );
}
#[test]
fn full_resource_or_disk_reserve_is_at_limit_even_when_growth_is_zero() {
    let mut i = input();
    if let Observation::Observed(ref mut b) = i.buffer {
        b.start_bytes = b.capacity_bytes;
        b.end_bytes = b.capacity_bytes;
    }
    if let Observation::Observed(ref mut d) = i.disk {
        d.free_start_bytes = 500;
        d.free_end_bytes = 500;
        d.circular_budget_bytes = None;
        d.retained_span_ns = None;
    }
    let r = Report::build(i, None, 10 * SECOND);
    assert_eq!(estimated(r.buffer_limit), Forecast::AtLimit);
    assert_eq!(estimated(r.output_stopped_limit), Forecast::AtLimit);
    assert_eq!(estimated(r.disk_exhaustion), Forecast::AtLimit);
    assert_eq!(
        r.circular_retention,
        Value::Unavailable(Reason::NotCircular)
    );
    assert_eq!(
        r.retained_span_ns,
        Value::Unavailable(Reason::MissingMeasurement)
    );
    assert_eq!(r.pressure, Value::Observed(Pressure::Overloaded));
}
#[test]
fn invalid_durations_saturation_and_unrepresentable_horizons_are_explicit() {
    for kind in 0..3 {
        let mut i = input();
        match kind {
            0 => i.window.to_ns = 0,
            1 => i.window.from_ns = i.window.to_ns + 1,
            _ => i.window.counts.overflowed = true,
        }
        let r = Report::build(i, None, 10 * SECOND);
        assert!(matches!(r.accepted_rate, Value::Unavailable(_)));
        assert!(matches!(r.disk_exhaustion, Value::Unavailable(_)));
    }
    let mut i = input();
    if let Observation::Observed(ref mut b) = i.buffer {
        b.capacity_bytes = u64::MAX;
        b.start_bytes = 0;
        b.end_bytes = 1;
    }
    assert_eq!(
        Report::build(i, None, 10 * SECOND).buffer_bytes_limit,
        Value::Estimated {
            value: Forecast::BeyondRange,
            assumptions: dot_logger::telemetry::capacity::Assumption::ConstantNetGrowth
        }
    );
    if let Observation::Observed(ref mut b) = i.buffer {
        b.capacity_slots = 0;
    }
    assert_eq!(
        Report::build(i, None, 10 * SECOND).buffer_limit,
        Value::Unavailable(Reason::InvalidMeasurement)
    );
}
#[test]
fn all_capacity_conditions_future_validity_and_zero_denominators_are_checked() {
    let i = input();
    let p = Profile {
        session_id: 42,
        id: 9,
        conditions: i.conditions,
        valid_from_ns: 0,
        valid_until_ns: 100 * SECOND,
        completed_records: Rate {
            amount: 200,
            elapsed_ns: 10 * SECOND,
        },
    };
    for kind in 0..5 {
        let mut q = p;
        match kind {
            0 => q.conditions.environment_id += 1,
            1 => q.conditions.workload_id += 1,
            2 => q.conditions.revision += 1,
            3 => q.conditions.target_id += 1,
            _ => q.conditions.boundary = Boundary::Synced,
        }
        assert_eq!(
            Report::build(i, Some(q), 10 * SECOND).capacity,
            Value::Unavailable(Reason::CapacityMismatch)
        );
    }
    for kind in 0..2 {
        let mut q = p;
        if kind == 0 {
            q.completed_records.amount = 0;
        } else {
            q.completed_records.elapsed_ns = 0;
        }
        assert_eq!(
            Report::build(i, Some(q), 10 * SECOND).load,
            Value::Unavailable(Reason::InvalidMeasurement)
        );
    }
    let mut q = p;
    q.valid_from_ns = SECOND;
    assert_eq!(
        Report::build(i, Some(q), 10 * SECOND).load,
        Value::Unavailable(Reason::ExpiredCapacity)
    );
}
#[test]
fn stable_synthetic_forecasts_match_step_simulation_within_twenty_percent() {
    // Independent one-second simulation, not a PC/RT IO performance measurement.
    for rate in [1, 7, 31, 100] {
        for horizon_seconds in [5, 13, 60] {
            let mut i = input();
            if let Observation::Observed(ref mut b) = i.buffer {
                b.start_bytes = 100;
                b.end_bytes = 100 + rate * 10;
                b.capacity_bytes = b.end_bytes + rate * horizon_seconds;
                b.start_slots = 20;
                b.end_slots = 20;
            }
            if let Observation::Observed(ref mut d) = i.disk {
                d.free_start_bytes = 100_000;
                d.free_end_bytes = 100_000 - rate * 10;
                d.reserve_bytes = d.free_end_bytes - rate * horizon_seconds;
                d.quota_remaining_bytes = None;
            }
            let r = Report::build(i, None, 10 * SECOND);
            let mut remaining = rate * horizon_seconds;
            let mut actual_seconds = 0;
            while remaining > 0 {
                remaining = remaining.saturating_sub(rate);
                actual_seconds += 1;
            }
            for prediction in [r.buffer_limit, r.disk_exhaustion] {
                let Forecast::AfterNs(ns) = estimated(prediction) else {
                    panic!("{prediction:?}")
                };
                let actual_ns = actual_seconds * SECOND;
                assert!(u128::from(ns.abs_diff(actual_ns)) * 100 <= u128::from(actual_ns) * 20);
            }
        }
    }
}

#[test]
fn a_known_quota_limit_is_not_hidden_by_a_horizon_beyond_u64_nanoseconds() {
    let mut i = input();
    if let Observation::Observed(ref mut d) = i.disk {
        d.free_start_bytes = u64::MAX;
        d.free_end_bytes = u64::MAX - 1;
        d.quota_remaining_bytes = Some(0);
    }
    let r = Report::build(i, None, 10 * SECOND);
    assert_eq!(estimated(r.disk_exhaustion), Forecast::AtLimit);
    assert_eq!(r.pressure, Value::Observed(Pressure::Overloaded));
}
