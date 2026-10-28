//! Explicit management-path routing contracts.

use crate::capture::{CaptureReceipt, ReadError};
use crate::config::ConfigError;
use std::fmt;
use std::time::Duration;

/// One immutable initial-revision channel route. Empty targets explicitly omit output.
#[derive(Clone, Debug)]
pub struct Route {
    /// Registered ingress channel.
    pub channel_id: u32,
    /// Unique identifiers from the configured target set.
    pub targets: Vec<u32>,
}

/// Finite retention and retry limits for one independent output.
#[derive(Clone, Debug)]
pub struct TargetConfig {
    /// Unique output identifier.
    pub id: u32,
    /// Preserved in outcomes; a required-target drop must not become overall success.
    pub required: bool,
    /// Nonblocking output FIFO capacity.
    pub queue_slots: usize,
    /// Separate FIFO for bounded enqueue retries; zero disables retention on full.
    pub pending_slots: usize,
    /// Logical payload bytes held by this target, including pending and active IO.
    /// Shared originals are charged here as retention causes, not physical copies.
    pub retained_bytes: usize,
    /// Total enqueue attempts including the initial attempt; positive.
    pub max_attempts: usize,
    /// Maximum pending/queued age; an active IO guard remains valid past this age.
    pub max_age: Duration,
}

/// Explicit finite work limits. Numbers are application choices, not product defaults.
#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    /// Maximum records consumed from one producer during one visit.
    pub record_quantum: usize,
    /// Deficit credit added per producer visit, allowing large heads to make progress.
    pub byte_quantum: usize,
    /// Maximum new records consumed across all producers per turn.
    pub records_per_turn: usize,
    /// Maximum new payload bytes per turn; must fit the largest configured record.
    pub bytes_per_turn: usize,
    /// Maximum pending FIFO entries examined per target per turn.
    pub retries_per_target: usize,
    /// Raw slots inspected per producer per turn, independent of notifications.
    pub reclaim_slots_per_producer: usize,
}

/// Startup-only route plan, with physical output reservations separate from raw pools.
#[derive(Clone, Debug)]
pub struct RoutingConfig {
    /// Fixed target set, including explicit required/optional roles.
    pub targets: Vec<TargetConfig>,
    /// Exactly one route for every registered channel, frozen for this revision.
    pub routes: Vec<Route>,
    /// Maximum distinct payloads retained across all target references and active IO.
    pub shared_records: usize,
    /// Aggregate small-payload copy bytes. Originals never consume this reservation.
    pub small_copy_bytes: usize,
    /// Work and fairness limits.
    pub schedule: Schedule,
}

/// Startup or invariant failure. A failed head is left in ingress for diagnosis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutingError {
    /// Invalid or inconsistent targets, routes, quotas, or work limits.
    InvalidConfig,
    /// Consumers do not form this plan's ordered, single-session producer set.
    InputsMismatch,
    /// Concrete output reservations do not fit, or arithmetic overflowed.
    Config(ConfigError),
    /// Ingress publication invariant failed.
    Read(ReadError),
    /// Tracked routing could not publish a completion observation.
    Completion(crate::completion::CompletionError),
    /// Supplied management time moved backwards; the turn performed no work.
    ClockWentBackwards,
    /// Head identity, revision, or payload size does not match the initial plan.
    RecordMismatch,
}
impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "routing error: {self:?}")
    }
}
impl std::error::Error for RoutingError {}
impl From<ReadError> for RoutingError {
    fn from(value: ReadError) -> Self {
        Self::Read(value)
    }
}

/// Explicit reason for terminating this target's dispatch attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DropReason {
    /// Target's combined queued/pending/active-IO slots or bytes were exhausted.
    Capacity,
    /// Configured enqueue-attempt limit was exhausted.
    RetryExhausted,
    /// No bounded pending slot was available.
    PendingFull,
    /// Pending/queued age reached its limit.
    Expired,
    /// Management explicitly cancelled this target.
    Cancelled,
}

/// Dispatch-stage result, without storage/completion acknowledgments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchOutcome {
    /// Entered this target FIFO; this is not a write or durability acknowledgment.
    Enqueued,
    /// This target's processing attempt ended with an explicit cause.
    Dropped(DropReason),
    /// Route explicitly selected no outputs; target_id is absent.
    Unrouted,
}

/// Caller-owned event used to connect routing to future completion/telemetry stages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchEvent {
    /// Original ingress identity, never retrospectively rejected.
    pub receipt: CaptureReceipt,
    /// Immutable ingress/route revision.
    pub revision: u64,
    /// None only for an explicitly empty route.
    pub target_id: Option<u32>,
    /// Configured role of the affected target.
    pub required: bool,
    /// Dispatch-stage outcome.
    pub outcome: DispatchOutcome,
}

/// Bounded work and pressure observed in one dispatcher turn.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TurnReport {
    /// Ingress invariant failure after any reported work; the failing head is retained.
    pub fault: Option<RoutingError>,
    /// New ingress records consumed.
    pub records: usize,
    /// New ingress payload bytes consumed, counted once across fanout.
    pub bytes: usize,
    /// Initialized prefix of the caller's event buffer.
    pub events: usize,
    /// Pending FIFO entries examined, including failed retries.
    pub retries: usize,
    /// Raw slots inspected across producer pools.
    pub reclaim_inspected: usize,
    /// Raw slots returned to Free during this turn.
    pub reclaimed: usize,
    /// Some work was left for another turn due to a bounded resource/work limit.
    pub backpressured: bool,
}

/// Shared reservations and live physical-payload usage; concurrent samples are approximate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RoutingSnapshot {
    /// Full raw pool allocation, including idle and return-pending slots, counted once.
    pub raw_reserved_bytes: usize,
    /// Output reservation including queues, route metadata, shared headers, and copies.
    pub output_reserved_bytes: usize,
    /// Distinct originals/small copies retained, irrespective of target count.
    pub live_records: usize,
    /// Actual bytes in live small copies, counted once across targets.
    pub small_copy_bytes: usize,
    /// Valid original bytes held by shared guards; part of raw_reserved_bytes, not extra.
    pub raw_retained_bytes: usize,
}

/// One target's retention causes, including IO guards after queue removal.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetSnapshot {
    /// Records in this target's output FIFO.
    pub queued: usize,
    /// Records in its bounded retry FIFO.
    pub pending: usize,
    /// Records removed from the FIFO but still held by consumers/IO.
    pub in_flight: usize,
    /// Combined queued, pending, and active-IO record count.
    pub retained_records: usize,
    /// Combined logical payload bytes; sharing across targets does not multiply RAM.
    pub retained_bytes: usize,
    /// Explicit management cancellation has been requested.
    pub cancelled: bool,
}
