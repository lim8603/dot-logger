//! General-path before/after observations. Logger filtering changes recording
//! volume; it does not remove application-side input generation work.
use super::{
    Window,
    capacity::{Rate, Reason, Value},
    report::Report,
};

/// Optional host measurements tied to exactly one report window. Missing fields
/// stay unavailable; this API starts no timers, profiler, worker or allocator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resources {
    /// Exact session/revision/epoch/interval of the host observation.
    pub window: Window,
    /// Host identity of CPU/memory scope and latency measurement definition.
    pub measurement_scope_id: u128,
    /// Observed latency in ns, using the same definition on both sides.
    pub latency_ns: Value<u64>,
    /// CPU ns during the window, compared as CPU ns per elapsed ns.
    pub cpu_time_ns: Value<u64>,
    /// Logger/application memory bytes under the host's declared measurement scope.
    pub memory_bytes: Value<u64>,
}
impl Resources {
    /// Explicitly missing measurements associated with the given interval.
    pub fn missing(window: Window) -> Self {
        Self {
            window,
            measurement_scope_id: 0,
            latency_ns: Value::Unavailable(Reason::MissingMeasurement),
            cpu_time_ns: Value::Unavailable(Reason::MissingMeasurement),
            memory_bytes: Value::Unavailable(Reason::MissingMeasurement),
        }
    }
}
/// One side of a comparison, preserving profile provenance, revision, window,
/// output outcomes and all rejection categories in the original report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    /// Report with its original Observed/Validated/Estimated/Stale values.
    pub report: Report,
    /// Host identity for input generation before logger filters/sampling. Change
    /// this when the application changes generation period, payload or input work.
    pub application_input_id: u128,
    /// Optional CPU/memory/latency measurements; never inferred from record volume.
    pub resources: Resources,
}
impl Snapshot {
    /// Preserve a report and explicitly mark unprovided host metrics unavailable.
    pub fn new(report: Report, application_input_id: u128) -> Self {
        Self {
            report,
            application_input_id,
            resources: Resources::missing(report.window),
        }
    }
}
/// All observed condition differences, rather than the first mismatch only.
/// Revisions are expected to differ; their IDs remain in each snapshot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Differences {
    /// Different session clock or counter generation cannot be joined causally.
    pub session_or_epoch: bool,
    /// Different hardware/OS/features.
    pub environment: bool,
    /// Different payload/fanout/codec/offered-load profile.
    pub workload: bool,
    /// Different pre-logger input generation profile.
    pub application_input: bool,
    /// Different selected output target.
    pub target: bool,
    /// Different output completion boundary.
    pub boundary: bool,
    /// CPU/memory scope or latency measurement definitions differ.
    pub measurement_scope: bool,
    /// Contradictory revision IDs, empty/reversed/overlapping time windows.
    pub invalid_window: bool,
}
impl Differences {
    /// Whether rate differences can be compared under the declared conditions.
    pub fn comparable(self) -> bool {
        !(self.session_or_epoch
            || self.environment
            || self.workload
            || self.application_input
            || self.target
            || self.boundary
            || self.measurement_scope
            || self.invalid_window)
    }
}
/// Sign of after minus before, kept separate from unsigned exact arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// After is larger.
    Increase,
    /// Rates or scalar measurements are exactly equal.
    Equal,
    /// After is smaller.
    Decrease,
}
/// Exact magnitude of after minus before, without float rounding or signed casts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RateChange {
    /// Sign of the difference.
    pub direction: Direction,
    /// Nonnegative cross-multiplied magnitude.
    pub numerator: u128,
    /// Strictly positive common denominator; time unit is nanoseconds.
    pub denominator: u128,
}
/// Preserved evidence and normalized observed changes. A comparable result is
/// not a claim of measured capacity, RT accuracy or reduced application work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Comparison {
    /// Original before evidence, including missing and stale measurements.
    pub before: Snapshot,
    /// Original after evidence, including missing and stale measurements.
    pub after: Snapshot,
    /// Every condition mismatch.
    pub differences: Differences,
    /// Admitted records per ns, independent of generation cost.
    pub accepted_rate: Value<RateChange>,
    /// Recorded payload bytes per ns, not physical IO bytes.
    pub accepted_bytes_rate: Value<RateChange>,
    /// Completed target assignments per ns.
    pub completed_rate: Value<RateChange>,
    /// Intentional filter/sampling omissions per ns.
    pub intentional_skip_rate: Value<RateChange>,
    /// Capacity admission rejection attempts per ns; retries are separate attempts.
    pub capacity_rejection_rate: Value<RateChange>,
    /// CPU ns per elapsed ns. Missing/stale metrics never imply savings.
    pub cpu_rate: Value<RateChange>,
    /// Scalar latency difference, represented with denominator 1.
    pub latency_ns: Value<RateChange>,
    /// Scalar memory byte difference, represented with denominator 1.
    pub memory_bytes: Value<RateChange>,
}
impl Comparison {
    /// Compare declared conditions and observations without applying settings.
    pub fn build(before: Snapshot, after: Snapshot) -> Self {
        let b = &before.report;
        let a = &after.report;
        let differences = Differences {
            session_or_epoch: b.window.session_id != a.window.session_id
                || b.window.epoch != a.window.epoch,
            environment: b.conditions.environment_id != a.conditions.environment_id,
            workload: b.conditions.workload_id != a.conditions.workload_id,
            application_input: before.application_input_id != after.application_input_id,
            target: b.conditions.target_id != a.conditions.target_id,
            boundary: b.conditions.boundary != a.conditions.boundary,
            measurement_scope: before.resources.measurement_scope_id
                != after.resources.measurement_scope_id,
            invalid_window: b.window.from_ns >= b.window.to_ns
                || a.window.from_ns >= a.window.to_ns
                || b.window.to_ns > a.window.from_ns
                || b.conditions.revision != b.window.revision
                || a.conditions.revision != a.window.revision,
        };
        let comparable = differences.comparable();
        let elapsed = |s: &Snapshot| {
            s.report
                .window
                .to_ns
                .saturating_sub(s.report.window.from_ns)
        };
        let rates = |b: Value<u64>, a: Value<u64>| {
            change(
                map(b, |amount| Rate {
                    amount,
                    elapsed_ns: elapsed(&before),
                }),
                map(a, |amount| Rate {
                    amount,
                    elapsed_ns: elapsed(&after),
                }),
                comparable,
            )
        };
        let valid_resources =
            before.resources.window == b.window && after.resources.window == a.window;
        let missing = Value::Unavailable(Reason::InvalidMeasurement);
        Self {
            before,
            after,
            differences,
            accepted_rate: change(b.accepted_rate, a.accepted_rate, comparable),
            accepted_bytes_rate: change(b.accepted_bytes_rate, a.accepted_bytes_rate, comparable),
            completed_rate: change(b.completed_rate, a.completed_rate, comparable),
            intentional_skip_rate: rates(b.skipped, a.skipped),
            capacity_rejection_rate: rates(b.rejected_capacity, a.rejected_capacity),
            cpu_rate: if valid_resources {
                rates(before.resources.cpu_time_ns, after.resources.cpu_time_ns)
            } else {
                missing
            },
            latency_ns: if valid_resources {
                scalar(
                    before.resources.latency_ns,
                    after.resources.latency_ns,
                    comparable,
                )
            } else {
                missing
            },
            memory_bytes: if valid_resources {
                scalar(
                    before.resources.memory_bytes,
                    after.resources.memory_bytes,
                    comparable,
                )
            } else {
                missing
            },
        }
    }
}
fn scalar(b: Value<u64>, a: Value<u64>, comparable: bool) -> Value<RateChange> {
    change(
        map(b, |amount| Rate {
            amount,
            elapsed_ns: 1,
        }),
        map(a, |amount| Rate {
            amount,
            elapsed_ns: 1,
        }),
        comparable,
    )
}
fn change(b: Value<Rate>, a: Value<Rate>, comparable: bool) -> Value<RateChange> {
    if !comparable {
        return Value::Unavailable(Reason::IncomparableConditions);
    }
    let (b, a) = match (b, a) {
        (Value::Observed(b), Value::Observed(a)) => (b, a),
        (Value::Stale { .. }, _) | (_, Value::Stale { .. }) => {
            return Value::Unavailable(Reason::StaleMeasurement);
        }
        (Value::Unavailable(r), _) | (_, Value::Unavailable(r)) => return Value::Unavailable(r),
        _ => return Value::Unavailable(Reason::MissingMeasurement),
    };
    if b.elapsed_ns == 0 || a.elapsed_ns == 0 {
        return Value::Unavailable(Reason::InvalidMeasurement);
    }
    let before = u128::from(b.amount) * u128::from(a.elapsed_ns);
    let after = u128::from(a.amount) * u128::from(b.elapsed_ns);
    let direction = match after.cmp(&before) {
        std::cmp::Ordering::Less => Direction::Decrease,
        std::cmp::Ordering::Equal => Direction::Equal,
        std::cmp::Ordering::Greater => Direction::Increase,
    };
    Value::Observed(RateChange {
        direction,
        numerator: after.abs_diff(before),
        denominator: u128::from(a.elapsed_ns) * u128::from(b.elapsed_ns),
    })
}

fn map<T, U>(value: Value<T>, f: impl FnOnce(T) -> U) -> Value<U> {
    match value {
        Value::Observed(v) => Value::Observed(f(v)),
        Value::Validated { value, profile_id } => Value::Validated {
            value: f(value),
            profile_id,
        },
        Value::Estimated { value, assumptions } => Value::Estimated {
            value: f(value),
            assumptions,
        },
        Value::Stale { value, basis } => Value::Stale {
            value: f(value),
            basis,
        },
        Value::Unavailable(r) => Value::Unavailable(r),
    }
}
