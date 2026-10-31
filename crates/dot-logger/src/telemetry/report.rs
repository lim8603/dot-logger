//! Pure, bounded management-path reporting. The host supplies resource samples
//! for exactly the window and conditions below; no clock, IO or worker runs here.
//! Missing/stale resource measurements must stay explicit. Ingress bytes are not
//! substituted for physical buffer/disk bytes (fanout/compression/raw pins differ).
use super::capacity::{
    self, Assumption, Conditions, Forecast, Profile, Rate, Ratio, Reason, Value,
};
use super::{Observation, Window};

/// Selected output's interval outcomes and final pending assignments. The host
/// must use target-specific completion counters, preserving terminal outcomes.
/// At each successful sample, retain the sample identity/time with target_totals
/// in a bounded host history. Match window.from_ns/to_ns exactly (not to_ns minus
/// the requested seconds), reject foreign epochs/targets/boundaries, overflow,
/// Busy or decreasing terminal counters, and use the final pending value.
/// Missing endpoints stay Unavailable; all-target Sample.outputs is not a target
/// snapshot. Concurrent capture/completion observations can mix nearby instants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputDelta {
    /// Assignments completing at Conditions::boundary during the interval.
    pub complete: u64,
    /// Assignments still pending at the interval end; not a counter delta.
    pub pending: u64,
    /// Terminal failures during the interval.
    pub failed: u64,
    /// Policy discards during the interval, separate from intentional filtering.
    pub dropped: u64,
    /// Terminal unknown results during the interval.
    pub unknown: u64,
}
/// One physical resource's occupancy over the report window. Include memory
/// held by outstanding raw references and work/reclaim reservations in the
/// applicable physical pool; do not equate accepted payload bytes to occupancy.
/// Produce separate reports for independent queues/pools rather than averaging.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BufferTrend {
    /// Host-stable identity of the physical queue/pool being examined.
    pub resource_id: u32,
    /// Usable byte ceiling for this resource.
    pub capacity_bytes: u64,
    /// Physical bytes held at window start.
    pub start_bytes: u64,
    /// Physical bytes held at window end, including retained references.
    pub end_bytes: u64,
    /// Usable slot ceiling; a separate constraint from bytes.
    pub capacity_slots: u64,
    /// Slots held at window start.
    pub start_slots: u64,
    /// Slots held at window end.
    pub end_slots: u64,
    /// Gross physical byte admissions, including bytes later reclaimed/dropped.
    pub incoming_bytes: u64,
    /// Gross slot admissions; not the global accepted record count.
    pub incoming_slots: u64,
}
/// Physical disk observations after encoding, compression, fanout and deletion.
/// Free-space trend includes other filesystem users; it is not a writer rate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiskTrend {
    /// Filesystem free bytes at window start.
    pub free_start_bytes: u64,
    /// Filesystem free bytes at window end.
    pub free_end_bytes: u64,
    /// Space to keep unused; reaching it is AtLimit even without recent growth.
    pub reserve_bytes: u64,
    /// Optional independent output-quota room; restricts usable space further.
    pub quota_remaining_bytes: Option<u64>,
    /// Gross encoded physical file bytes written in the window, before deletion.
    pub written_bytes: u64,
    /// File bytes deleted in the window; retained as an observation, not loss.
    pub deleted_bytes: u64,
    /// Reusable circular-policy bytes after fixed protected/pinned reservations.
    /// None means continuous/no circular model. This is not filesystem free space.
    pub circular_budget_bytes: Option<u64>,
    /// Observed oldest-to-newest retained timestamp span, if known. May contain
    /// gaps; do not claim that every record inside this span remains available.
    pub retained_span_ns: Option<u64>,
}
/// Input measurements share the exact window's session/revision/epoch/clock and
/// Conditions. The host must reject mismatched resource/output snapshots before
/// constructing this value. This API never silently samples a different source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReportInput {
    /// Global producer/channel ingress interval from Telemetry::window.
    pub window: Window,
    /// Device/load/revision and selected output/actual completion boundary.
    pub conditions: Conditions,
    /// Selected output measurements for the same interval, or explicit absence.
    pub output: Observation<OutputDelta>,
    /// Physical resource measurements for the same interval, or explicit absence.
    pub buffer: Observation<BufferTrend>,
    /// Filesystem/output-policy measurements for the same interval.
    pub disk: Observation<DiskTrend>,
}
impl ReportInput {
    /// Start with a retained global ingress window and no resource measurements.
    /// No default zeros, output capacity, profile or worker are fabricated.
    pub fn from_window(window: Window, conditions: Conditions) -> Self {
        Self {
            window,
            conditions,
            output: Observation::Unavailable(super::Unavailable::MissingMeasurement),
            buffer: Observation::Unavailable(super::Unavailable::MissingMeasurement),
            disk: Observation::Unavailable(super::Unavailable::MissingMeasurement),
        }
    }
}
/// Pressure of the reported interval. Absence of loss is not device headroom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pressure {
    /// Capacity rejection/acquisition failure/output loss, a full resource or attempt load
    /// beyond validated capacity. Queue stability after discards does not clear it.
    Overloaded,
    /// Occupancy grew; finite buffer capacity is being consumed.
    Growing,
    /// No loss, growth, full resource or measured capacity exceedance was seen.
    /// This is an observation only, not validated spare capacity.
    NoPressureObserved,
}
/// One report with explicit units, provenance and original measurement scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report {
    /// Original window, including session/revision/epoch and actual timestamps.
    pub window: Window,
    /// Device/load and selected output identity; ingress still covers all channels.
    pub conditions: Conditions,
    /// Caller-supplied current time; stale when reversed or over two seconds old.
    pub generated_at_ns: u64,
    /// Full source input including resource identity, capacities and raw counts.
    pub measurements: ReportInput,
    /// Accepted ingress records per nanosecond (rational).
    pub accepted_rate: Value<Rate>,
    /// Ingress payload bytes per nanosecond, never physical occupied bytes.
    pub accepted_bytes_rate: Value<Rate>,
    /// Intentional filter omissions, not unexpected loss.
    pub skipped: Value<u64>,
    /// Rejected ingress submissions; raw acquisition failures remain separate.
    pub rejected: Value<u64>,
    /// Raw acquisition failures, not necessarily additional attempted records.
    pub acquire_rejected: Value<u64>,
    /// Capacity-rejected submission attempts.
    pub rejected_capacity: Value<u64>,
    /// Contract-rejected submission attempts; not capacity pressure.
    pub rejected_contract: Value<u64>,
    /// Lifecycle-rejected submission attempts; not capacity pressure.
    pub rejected_lifecycle: Value<u64>,
    /// Capacity-rejected raw acquisition attempts.
    pub acquire_rejected_capacity: Value<u64>,
    /// Contract-rejected raw acquisition attempts; not capacity pressure.
    pub acquire_rejected_contract: Value<u64>,
    /// Lifecycle-rejected raw acquisition attempts; not capacity pressure.
    pub acquire_rejected_lifecycle: Value<u64>,
    /// Selected output completion rate in assignments per nanosecond.
    pub completed_rate: Value<Rate>,
    /// Selected output's final pending count and terminal interval outcomes.
    pub output: Value<OutputDelta>,
    /// Matching externally validated ingress-equivalent completion rate.
    pub capacity: Value<Rate>,
    /// Accepted plus capacity-rejected submission attempts / validated completion.
    /// Retries count again: this is attempt demand, not unique offered records.
    /// Contract/lifecycle failures, skips and all raw acquisitions are excluded.
    pub load: Value<Ratio>,
    /// Interval pressure; missing evidence never becomes a healthy zero.
    pub pressure: Value<Pressure>,
    /// Time until byte exhaustion under measured physical net occupancy growth.
    pub buffer_bytes_limit: Value<Forecast>,
    /// Time until slot exhaustion under measured net slot growth.
    pub buffer_slots_limit: Value<Forecast>,
    /// Earlier usable byte/slot limit; both constraints must be known.
    pub buffer_limit: Value<Forecast>,
    /// Earlier byte/slot horizon assuming all output/reclaim stops.
    pub output_stopped_limit: Value<Forecast>,
    /// Time until free-space reserve/quota under observed net filesystem growth.
    pub disk_exhaustion: Value<Forecast>,
    /// Circular byte budget / gross physical write rate, distinct from disk fill.
    pub circular_retention: Value<Forecast>,
    /// Observed retained timestamp span, with no continuous-coverage guarantee.
    pub retained_span_ns: Value<u64>,
}
fn read<T: Copy>(source: Observation<T>) -> Result<T, Reason> {
    match source {
        Observation::Observed(v) => Ok(v),
        Observation::Stale(_) => Err(Reason::StaleMeasurement),
        Observation::Unavailable(super::Unavailable::MissingMeasurement) => {
            Err(Reason::MissingMeasurement)
        }
        Observation::Unavailable(r) => Err(Reason::Source(r)),
    }
}
fn value<T>(v: Result<T, Reason>, assumption: Assumption, stale: bool) -> Value<T> {
    match v {
        Ok(v) => Value::estimated(v, assumption).age(stale),
        Err(r) => Value::Unavailable(r),
    }
}
fn observed<T: Copy>(v: Observation<T>, stale: bool) -> Value<T> {
    match v {
        Observation::Observed(v) => Value::Observed(v).age(stale),
        Observation::Stale(v) => Value::Observed(v).age(true),
        other => Value::Unavailable(read(other).err().unwrap_or(Reason::MissingMeasurement)),
    }
}
fn buffer(b: BufferTrend) -> Result<BufferTrend, Reason> {
    if b.capacity_bytes == 0
        || b.capacity_slots == 0
        || b.start_bytes > b.capacity_bytes
        || b.end_bytes > b.capacity_bytes
        || b.start_slots > b.capacity_slots
        || b.end_slots > b.capacity_slots
    {
        return Err(Reason::InvalidMeasurement);
    }
    Ok(b)
}
impl Report {
    /// Calculate without IO, allocation, clocks, locks or unbounded iteration.
    /// Invalid windows invalidate every derived field, while original inputs stay
    /// available for diagnostics. No configuration change or benchmark is run.
    pub fn build(input: ReportInput, profile: Option<Profile>, now_ns: u64) -> Self {
        let w = input.window;
        let elapsed = w.to_ns.checked_sub(w.from_ns).filter(|n| *n > 0);
        let invalid = if w.revision != input.conditions.revision || elapsed.is_none() {
            Some(Reason::InvalidMeasurement)
        } else if w.counts.overflowed {
            Some(Reason::Arithmetic)
        } else {
            None
        };
        let elapsed = elapsed.unwrap_or(1);
        let stale = now_ns < w.to_ns || now_ns - w.to_ns > 2_000_000_000;
        let rate = |amount| {
            Value::Observed(Rate {
                amount,
                elapsed_ns: elapsed,
            })
            .age(stale)
        };
        let count = |n| Value::Observed(n).age(stale);
        let mut capacity = Value::Unavailable(Reason::MissingCapacity);
        let mut load = Value::Unavailable(Reason::MissingCapacity);
        let mut exceeds = false;
        if let Some(p) = profile {
            let c = p.completed_records;
            let reason = if p.session_id != w.session_id || p.conditions != input.conditions {
                Some(Reason::CapacityMismatch)
            } else if c.amount == 0 || c.elapsed_ns == 0 || p.valid_from_ns > p.valid_until_ns {
                Some(Reason::InvalidMeasurement)
            } else {
                None
            };
            if let Some(r) = reason {
                capacity = Value::Unavailable(r);
                load = Value::Unavailable(r);
            } else {
                let expired = p.valid_from_ns > w.from_ns
                    || p.valid_until_ns < w.to_ns
                    || now_ns > p.valid_until_ns;
                capacity = Value::Validated {
                    value: c,
                    profile_id: p.id,
                }
                .age(expired || stale);
                load = if expired {
                    Value::Unavailable(Reason::ExpiredCapacity)
                } else if let Some(offered) =
                    w.counts.accepted.checked_add(w.counts.rejected_capacity)
                {
                    let ratio = Ratio {
                        numerator: u128::from(offered) * u128::from(c.elapsed_ns),
                        denominator: u128::from(c.amount) * u128::from(elapsed),
                    };
                    exceeds = ratio.numerator > ratio.denominator;
                    Value::estimated(ratio, Assumption::MatchingCapacity).age(stale)
                } else {
                    Value::Unavailable(Reason::Arithmetic)
                };
            }
        }
        let b = read(input.buffer).and_then(buffer);
        let bytes = b.and_then(|b| {
            capacity::horizon(
                b.capacity_bytes - b.end_bytes,
                i128::from(b.end_bytes) - i128::from(b.start_bytes),
                elapsed,
            )
        });
        let slots = b.and_then(|b| {
            capacity::horizon(
                b.capacity_slots - b.end_slots,
                i128::from(b.end_slots) - i128::from(b.start_slots),
                elapsed,
            )
        });
        let stopped = b.and_then(|b| {
            Ok(capacity::first(
                capacity::horizon(
                    b.capacity_bytes - b.end_bytes,
                    i128::from(b.incoming_bytes),
                    elapsed,
                )?,
                capacity::horizon(
                    b.capacity_slots - b.end_slots,
                    i128::from(b.incoming_slots),
                    elapsed,
                )?,
            ))
        });
        let disk = read(input.disk);
        let disk_time = disk.and_then(|d| {
            let filesystem = capacity::horizon(
                d.free_end_bytes.saturating_sub(d.reserve_bytes),
                i128::from(d.free_start_bytes) - i128::from(d.free_end_bytes),
                elapsed,
            )?;
            // Quota is consumed by this output, whereas filesystem free space
            // can grow when other users delete files. Model both constraints.
            match d.quota_remaining_bytes {
                Some(room) => Ok(capacity::first(
                    filesystem,
                    capacity::horizon(
                        room,
                        i128::from(d.written_bytes) - i128::from(d.deleted_bytes),
                        elapsed,
                    )?,
                )),
                None => Ok(filesystem),
            }
        });
        let retention = disk.and_then(|d| {
            capacity::horizon(
                d.circular_budget_bytes.ok_or(Reason::NotCircular)?,
                i128::from(d.written_bytes),
                elapsed,
            )
        });
        let span = match input.disk {
            Observation::Observed(d) => d
                .retained_span_ns
                .map_or(Value::Unavailable(Reason::MissingMeasurement), count),
            Observation::Stale(d) => d
                .retained_span_ns
                .map_or(Value::Unavailable(Reason::MissingMeasurement), |n| {
                    Value::Observed(n).age(true)
                }),
            other => Value::Unavailable(read(other).err().unwrap_or(Reason::MissingMeasurement)),
        };
        let o = read(input.output);
        let loss = w.counts.rejected_capacity > 0
            || w.counts.acquire_rejected_capacity > 0
            || o.is_ok_and(|o| o.failed > 0 || o.dropped > 0 || o.unknown > 0);
        let full = b
            .is_ok_and(|b| b.end_bytes == b.capacity_bytes || b.end_slots == b.capacity_slots)
            || disk_time == Ok(Forecast::AtLimit);
        let pressure = if loss || full {
            Value::Observed(Pressure::Overloaded).age(stale)
        } else if exceeds {
            Value::estimated(Pressure::Overloaded, Assumption::MatchingCapacity).age(stale)
        } else if b.is_ok_and(|b| b.end_bytes > b.start_bytes || b.end_slots > b.start_slots) {
            Value::Observed(Pressure::Growing).age(stale)
        } else if let Err(r) = b {
            Value::Unavailable(r)
        } else if let Err(r) = o {
            Value::Unavailable(r)
        } else if let Err(r) = disk {
            Value::Unavailable(r)
        } else {
            Value::Observed(Pressure::NoPressureObserved).age(stale)
        };
        let mut report = Self {
            window: w,
            conditions: input.conditions,
            generated_at_ns: now_ns,
            measurements: input,
            accepted_rate: rate(w.counts.accepted),
            accepted_bytes_rate: rate(w.counts.accepted_bytes),
            skipped: count(w.counts.skipped),
            rejected: count(w.counts.rejected),
            acquire_rejected: count(w.counts.acquire_rejected),
            rejected_capacity: count(w.counts.rejected_capacity),
            rejected_contract: count(w.counts.rejected_contract),
            rejected_lifecycle: count(w.counts.rejected_lifecycle),
            acquire_rejected_capacity: count(w.counts.acquire_rejected_capacity),
            acquire_rejected_contract: count(w.counts.acquire_rejected_contract),
            acquire_rejected_lifecycle: count(w.counts.acquire_rejected_lifecycle),
            completed_rate: match o {
                Ok(o) => rate(o.complete),
                Err(r) => Value::Unavailable(r),
            },
            output: observed(input.output, stale),
            capacity,
            load,
            pressure,
            buffer_bytes_limit: value(bytes, Assumption::ConstantNetGrowth, stale),
            buffer_slots_limit: value(slots, Assumption::ConstantNetGrowth, stale),
            buffer_limit: value(
                bytes.and_then(|a| slots.map(|b| capacity::first(a, b))),
                Assumption::ConstantNetGrowth,
                stale,
            ),
            output_stopped_limit: value(stopped, Assumption::OutputStopped, stale),
            disk_exhaustion: value(disk_time, Assumption::ConstantNetGrowth, stale),
            circular_retention: value(retention, Assumption::ConstantCircularWrites, stale),
            retained_span_ns: span,
        };
        if let Some(r) = invalid {
            report.invalidate(r);
        }
        report
    }
    fn invalidate(&mut self, r: Reason) {
        self.accepted_rate = Value::Unavailable(r);
        self.accepted_bytes_rate = Value::Unavailable(r);
        self.skipped = Value::Unavailable(r);
        self.rejected = Value::Unavailable(r);
        self.acquire_rejected = Value::Unavailable(r);
        self.rejected_capacity = Value::Unavailable(r);
        self.rejected_contract = Value::Unavailable(r);
        self.rejected_lifecycle = Value::Unavailable(r);
        self.acquire_rejected_capacity = Value::Unavailable(r);
        self.acquire_rejected_contract = Value::Unavailable(r);
        self.acquire_rejected_lifecycle = Value::Unavailable(r);
        self.completed_rate = Value::Unavailable(r);
        self.output = Value::Unavailable(r);
        self.capacity = Value::Unavailable(r);
        self.load = Value::Unavailable(r);
        self.pressure = Value::Unavailable(r);
        self.buffer_bytes_limit = Value::Unavailable(r);
        self.buffer_slots_limit = Value::Unavailable(r);
        self.buffer_limit = Value::Unavailable(r);
        self.output_stopped_limit = Value::Unavailable(r);
        self.disk_exhaustion = Value::Unavailable(r);
        self.circular_retention = Value::Unavailable(r);
        self.retained_span_ns = Value::Unavailable(r);
    }
}
