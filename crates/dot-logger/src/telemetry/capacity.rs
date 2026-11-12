//! General-path evidence and checked capacity arithmetic. No device is measured here.
//!
//! `Profile` points at evidence the host obtained. A load ratio is computed only
//! when device, OS, features, load, codec, fanout, revision, output, completion
//! boundary and the session's valid interval all match. The denominator is
//! completion capacity comparable with global ingress, not the sum of fanout
//! assignments. `Value` separates observed, verified, assumption-based, unmeasured
//! and expired. Dropping work so a queue stabilizes still reports `Overloaded`;
//! intentional filtering is separate.
use crate::completion::Boundary;

/// Conditions identifying an externally measured completion-capacity profile.
/// IDs must change with hardware/OS/features or payload/fanout/codec/load changes.
/// Capture rates cover all registered producers/channels; target identifies the
/// output being examined. Capacity uses ingress-equivalent completed records,
/// never an unconverted sum of fanout assignments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Conditions {
    /// Host-maintained hardware, OS and enabled-feature identity.
    pub environment_id: u128,
    /// Host-maintained payload, fanout, codec and offered-load identity.
    pub workload_id: u128,
    /// Configuration used during validation.
    pub revision: u64,
    /// Selected output target; input counters remain global ingress counters.
    pub target_id: u32,
    /// Boundary actually validated by that output adapter.
    pub boundary: Boundary,
}
/// Exact rational rate. Units are documented by the containing field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rate {
    /// Count or physical bytes during the interval.
    pub amount: u64,
    /// Actual elapsed session-monotonic nanoseconds; must be positive.
    pub elapsed_ns: u64,
}
/// Exact dimensionless offered-input / validated-completion ratio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ratio {
    /// Cross-multiplied offered record rate numerator.
    pub numerator: u128,
    /// Cross-multiplied validated rate denominator; strictly positive.
    pub denominator: u128,
}
/// An externally validated profile. Supplying this asserts that the host has
/// evidence for the stated conditions, not that this library ran a benchmark.
/// Validity times must use the current session's monotonic time base.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile {
    /// Session whose monotonic clock defines this profile's validity timestamps.
    pub session_id: u128,
    /// Stable lookup key for the host's validation evidence.
    pub id: u128,
    /// Exact conditions required for reuse.
    pub conditions: Conditions,
    /// Inclusive beginning of validity in session-monotonic nanoseconds.
    pub valid_from_ns: u64,
    /// Inclusive expiration in the same clock domain.
    pub valid_until_ns: u64,
    /// Positive ingress-equivalent completed records per interval.
    pub completed_records: Rate,
}
/// Model assumption, kept with every estimate rather than promoted to validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Assumption {
    /// Observed rates and available resource limits remain constant.
    ConstantNetGrowth,
    /// All output/reclaim stops while measured physical admissions continue.
    OutputStopped,
    /// Reusable circular bytes / gross physical writes; protection, pins and
    /// configuration stay fixed. This is not a guaranteed continuous time span.
    ConstantCircularWrites,
    /// Offered non-filtered records / matching validated completion capacity.
    MatchingCapacity,
}
/// Provenance retained when a value becomes stale.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Basis {
    /// Caller-supplied or counter-derived observation.
    Observed,
    /// Evidence key of a previously valid capacity.
    Validated(u128),
    /// Assumption behind a model result.
    Estimated(Assumption),
}
/// Why a report field is unavailable; none of these mean zero or spare capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reason {
    /// Before and after have different input, environment, scope or time conditions.
    IncomparableConditions,
    /// The host has not supplied this measurement.
    MissingMeasurement,
    /// No validated completion capacity exists.
    MissingCapacity,
    /// Hardware, workload, revision, target or completion boundary differs.
    CapacityMismatch,
    /// Profile does not cover the entire observation interval and current time.
    ExpiredCapacity,
    /// Zero duration, capacity, contradictory occupancy, or invalid timestamps.
    InvalidMeasurement,
    /// Counter overflow/reset or unrepresentable result.
    Arithmetic,
    /// A model needs fresh input; stale inputs are never used as current rates.
    StaleMeasurement,
    /// A circular policy was not supplied for this output.
    NotCircular,
    /// A source counter/window explicitly reports missing or unusable data.
    Source(super::Unavailable),
}
/// Evidence classification for one field, independent of other report fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Value<T> {
    /// Supplied measurement or direct counter arithmetic.
    Observed(T),
    /// Capacity backed by the host's matching external validation evidence.
    Validated {
        /// Measured capacity.
        value: T,
        /// Validation evidence key.
        profile_id: u128,
    },
    /// Model result whose assumption is explicit.
    Estimated {
        /// Model value; no actual-device accuracy claim is implied.
        value: T,
        /// Conditions under which the prediction may be used.
        assumptions: Assumption,
    },
    /// Missing or unusable information, never a numeric zero.
    Unavailable(Reason),
    /// Value and provenance retained after its observation/validation expires.
    Stale {
        /// Original value, not extrapolated to the present.
        value: T,
        /// Original provenance.
        basis: Basis,
    },
}
impl<T> Value<T> {
    pub(crate) fn age(self, stale: bool) -> Self {
        if !stale {
            return self;
        }
        match self {
            Self::Observed(value) => Self::Stale {
                value,
                basis: Basis::Observed,
            },
            Self::Validated { value, profile_id } => Self::Stale {
                value,
                basis: Basis::Validated(profile_id),
            },
            Self::Estimated { value, assumptions } => Self::Stale {
                value,
                basis: Basis::Estimated(assumptions),
            },
            other => other,
        }
    }
    pub(crate) fn estimated(value: T, assumptions: Assumption) -> Self {
        Self::Estimated { value, assumptions }
    }
}
/// A limit horizon. NoGrowth is conditional on the observed/modelled rate,
/// not a promise of infinite capacity. Zero remaining space is always AtLimit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Forecast {
    /// The resource has no remaining room, regardless of its recent growth.
    AtLimit,
    /// Strictly positive finite duration, rounded up to nanoseconds.
    AfterNs(u64),
    /// Positive horizon exceeds u64 nanoseconds; longer than any AfterNs value.
    /// This is not missing growth or an infinite-capacity claim.
    BeyondRange,
    /// No positive growth occurred in the selected model.
    NoGrowth,
}
pub(crate) fn horizon(remaining: u64, growth: i128, elapsed: u64) -> Result<Forecast, Reason> {
    if elapsed == 0 {
        return Err(Reason::InvalidMeasurement);
    }
    if remaining == 0 {
        return Ok(Forecast::AtLimit);
    }
    if growth <= 0 {
        return Ok(Forecast::NoGrowth);
    }
    // Both factors originate as u64; their product and ceil addition fit u128.
    let numerator = u128::from(remaining) * u128::from(elapsed);
    let denominator = growth as u128;
    let ns = numerator.div_ceil(denominator);
    Ok(u64::try_from(ns).map_or(Forecast::BeyondRange, Forecast::AfterNs))
}
pub(crate) fn first(a: Forecast, b: Forecast) -> Forecast {
    match (a, b) {
        (Forecast::AtLimit, _) | (_, Forecast::AtLimit) => Forecast::AtLimit,
        (Forecast::NoGrowth, b) => b,
        (a, Forecast::NoGrowth) => a,
        (Forecast::BeyondRange, b) => b,
        (a, Forecast::BeyondRange) => a,
        (Forecast::AfterNs(a), Forecast::AfterNs(b)) => Forecast::AfterNs(a.min(b)),
    }
}
