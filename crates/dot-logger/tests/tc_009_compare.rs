//! Before/after comparison retains evidence and refuses causal comparisons of different inputs.
#![allow(clippy::unwrap_used)]
use dot_logger::{
    completion::Boundary,
    telemetry::{
        CaptureCounts, Window,
        capacity::{Conditions, Rate, Reason, Value},
        compare::*,
        report::{Report, ReportInput},
    },
};
fn snapshot(revision: u64, accepted: u64, skipped: u64) -> Snapshot {
    let from_ns = (revision - 1) * 10_000_000_000;
    let window = Window {
        session_id: 42,
        revision,
        epoch: 1,
        from_ns,
        to_ns: from_ns + 10_000_000_000,
        counts: CaptureCounts {
            accepted,
            skipped,
            rejected: 2,
            rejected_capacity: 2,
            ..Default::default()
        },
    };
    let conditions = Conditions {
        environment_id: 1,
        workload_id: 2,
        revision,
        target_id: 10,
        boundary: Boundary::Written,
    };
    Snapshot::new(
        Report::build(
            ReportInput::from_window(window, conditions),
            None,
            window.to_ns,
        ),
        99,
    )
}
#[test]
fn compares_recording_effect_and_keeps_intentional_skips_separate_from_loss() {
    let c = Comparison::build(snapshot(1, 100, 0), snapshot(2, 50, 50));
    assert!(c.differences.comparable());
    let Value::Observed(delta) = c.accepted_rate else {
        panic!("delta")
    };
    assert_eq!(delta.direction, Direction::Decrease);
    assert_eq!(delta.numerator * 10_000_000_000, delta.denominator * 50);
    assert!(matches!(
        c.intentional_skip_rate,
        Value::Observed(RateChange {
            direction: Direction::Increase,
            ..
        })
    ));
    assert!(matches!(
        c.capacity_rejection_rate,
        Value::Observed(RateChange {
            direction: Direction::Equal,
            ..
        })
    ));
    assert_eq!(c.after.report.window.revision, 2);
    assert_eq!(
        c.after.resources.cpu_time_ns,
        Value::Unavailable(Reason::MissingMeasurement)
    );
    assert_eq!(
        c.after.report.output,
        Value::Unavailable(Reason::MissingMeasurement)
    );
}
#[test]
fn changed_input_environment_or_completion_boundary_are_not_comparable() {
    let before = snapshot(1, 100, 0);
    let mut after = snapshot(2, 50, 0);
    after.application_input_id = 100;
    after.report.conditions.environment_id = 3;
    after.report.conditions.boundary = Boundary::Synced;
    let c = Comparison::build(before, after);
    assert!(!c.differences.comparable());
    assert!(c.differences.application_input && c.differences.environment && c.differences.boundary);
    assert_eq!(
        c.accepted_rate,
        Value::Unavailable(Reason::IncomparableConditions)
    );
    assert_eq!(c.after.application_input_id, 100);
}
#[test]
fn resources_require_exact_window_and_stale_measurements_are_not_zero() {
    let mut before = snapshot(1, 100, 0);
    let mut after = snapshot(2, 100, 0);
    let mut resources = Resources::missing(after.report.window);
    resources.window.revision = 999;
    resources.cpu_time_ns = Value::Observed(5);
    after.resources = resources;
    let c = Comparison::build(before, after);
    assert_eq!(c.cpu_rate, Value::Unavailable(Reason::InvalidMeasurement));
    before.resources.cpu_time_ns = Value::Observed(2_000_000_000);
    after.resources = Resources::missing(after.report.window);
    after.resources.cpu_time_ns = Value::Observed(1_000_000_000);
    let c = Comparison::build(before, after);
    assert!(matches!(
        c.cpu_rate,
        Value::Observed(RateChange {
            direction: Direction::Decrease,
            ..
        })
    ));
    after.report.accepted_rate = Value::Stale {
        value: Rate {
            amount: 100,
            elapsed_ns: 10_000_000_000,
        },
        basis: dot_logger::telemetry::capacity::Basis::Observed,
    };
    assert_eq!(
        Comparison::build(before, after).accepted_rate,
        Value::Unavailable(Reason::StaleMeasurement)
    );
}
