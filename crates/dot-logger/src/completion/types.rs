//! Fixed contracts for completion admission, observation, and bounded queries.
use crate::{capture::CaptureReceipt, config::ConfigError, routing::DropReason};
use std::{fmt, ops::BitOr};

/// A confirmed output boundary; each adapter declares its supported subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Boundary {
    /// Entered the target FIFO, without a write acknowledgment.
    Enqueued = 0,
    /// Encoded bytes reached the operating-system write boundary.
    Written = 1,
    /// Explicit synchronization covered the record.
    Synced = 2,
    /// Database API acknowledged the projection; not a disk guarantee.
    DatabaseAcknowledged = 3,
    /// Receiving application acknowledged the assembled record.
    ReceiverAcknowledged = 4,
}
/// Supported/observed boundaries, kept independent of terminal failures.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Boundaries(pub(crate) u32);
impl Boundaries {
    /// FIFO, OS write, and explicit synchronization.
    pub const FILE: Self = Self(7);
    /// FIFO and database API acknowledgment.
    pub const DATABASE: Self = Self(1 | 8);
    /// FIFO and receiving application acknowledgment.
    pub const REMOTE: Self = Self(1 | 16);
    /// One explicit boundary.
    pub const fn only(boundary: Boundary) -> Self {
        Self(1 << boundary as u8)
    }
    /// Whether this set contains the boundary.
    pub const fn contains(self, boundary: Boundary) -> bool {
        self.0 & (1 << boundary as u8) != 0
    }
}
impl BitOr for Boundaries {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
/// Immutable completion contract matching one route target.
#[derive(Clone, Debug)]
pub struct TargetContract {
    /// Stable target identifier.
    pub id: u32,
    /// Required/optional role, matching the routing plan.
    pub required: bool,
    /// Boundaries actually provided by the adapter.
    pub supported: Boundaries,
    /// Boundary at which this target's attempt is terminal success.
    pub completion: Boundary,
}
/// Explicit limits for producer windows, query work, and routing feedback.
#[derive(Clone, Debug)]
pub struct CompletionConfig {
    /// Maximum terminal-detail age; capacity reuse may retire detail earlier.
    /// Pending records never expire merely because this interval elapsed.
    pub history_retention: std::time::Duration,
    /// Fixed slots per producer; pending slots are never overwritten.
    pub window_records: usize,
    /// Concurrent borrowed query handles, including waiters.
    pub max_queries: usize,
    /// Maximum rows initialized by one query page.
    pub page_items: usize,
    /// Capacity of dispatcher feedback and each target-reader feedback FIFO.
    pub feedback_events: usize,
    /// Maximum feedback updates examined per queue in a dispatcher turn.
    pub updates_per_turn: usize,
    /// Maximum physical completion slots reconciled per dispatcher turn.
    pub reconcile_per_turn: usize,
    /// Initial target contracts, fixed with the capture revision.
    pub targets: Vec<TargetContract>,
}
/// Stable failure codes, without formatting or dynamic diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureReason {
    /// Output could not reach its write/API boundary.
    IoFailure,
    /// A confirmed write exists, but synchronization failed.
    SyncFailure,
    /// Adapter rejected or could not process this record.
    AdapterFailure,
}
/// Monotonic adapter/dispatcher observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Update {
    /// Confirmed boundary; Synced also confirms Written and Enqueued.
    Confirmed(Boundary),
    /// Terminal failure; prior confirmed boundaries remain observable.
    Failed(FailureReason),
    /// Explicit policy discard, distinct from admission rejection.
    Dropped(DropReason),
    /// Terminal uncertainty, never interpreted as successful delivery.
    Unknown,
}
/// Classification at the requested boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryOutcome {
    /// Requested boundary was confirmed, even if a later boundary failed.
    Complete,
    /// The requested boundary has not been confirmed or terminated.
    Pending,
    /// Terminal failure before the requested boundary.
    Failed(FailureReason),
    /// Explicit target discard before the requested boundary.
    Dropped(DropReason),
    /// Result cannot be confirmed, including an unreported delivery Drop.
    Unknown,
    /// This receipt's bounded detail has been retired.
    Expired,
    /// This target was not selected by this record's immutable route.
    NotRouted,
    /// Metadata is being published/updated; retry without assuming success.
    Busy,
}
/// One bounded completion observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordStatus {
    /// Confirmed boundaries, never erased by a later failure.
    pub confirmed: Boundaries,
    /// Classification at the requested boundary.
    pub outcome: QueryOutcome,
}
/// Query/update/startup error. A Busy update has changed nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompletionError {
    /// Invalid limits, routes, capabilities, or query selection.
    InvalidConfig,
    /// This stage-specific ingress was created without completion tracking.
    Untracked,
    /// Concrete metadata exceeds the enclosing control reservation.
    Config(ConfigError),
    /// Session/producer/sequence is invalid or not yet accepted.
    InvalidReceipt,
    /// Receipt detail was retired before this update.
    Expired,
    /// Unknown target identifier.
    InvalidTarget,
    /// Target does not implement the requested boundary.
    UnsupportedBoundary,
    /// Target is not selected by this receipt's plan.
    NotRouted,
    /// Publication/report ownership is held elsewhere; do not spin in RT.
    Busy,
    /// Configured concurrent-query budget is exhausted.
    QueryCapacity,
    /// A terminal outcome cannot be replaced by a conflicting observation.
    InvalidTransition,
    /// Caller buffer has insufficient capacity for the registered producer set.
    BufferTooSmall,
    /// A caller range is invalid, duplicated, or extends beyond acceptance.
    InvalidScope,
}
impl fmt::Display for CompletionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "completion error: {self:?}")
    }
}
impl std::error::Error for CompletionError {}
/// Frozen producer-local range; first=1,last=0 represents an empty producer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProducerRange {
    /// Registered producer index.
    pub producer_id: u32,
    /// First requested accepted sequence, inclusive.
    pub first: u64,
    /// Last requested accepted sequence, inclusive.
    pub last: u64,
}
/// One requested target/boundary, independent of the target's required role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetRequest {
    /// Configured target identifier.
    pub target_id: u32,
    /// Explicit observation boundary.
    pub boundary: Boundary,
}
/// Caller-owned page row; a page never starts IO or synchronization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompletionItem {
    /// Original ingress identity.
    pub receipt: CaptureReceipt,
    /// Requested target/boundary.
    pub request: TargetRequest,
    /// Status at that boundary.
    pub status: RecordStatus,
}
/// Counts accumulated over the pages of one frozen query scan.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QuerySummary {
    /// Confirmed requested boundaries.
    pub complete: u64,
    /// Nonterminal observations.
    pub pending: u64,
    /// Terminal failures.
    pub failed: u64,
    /// Policy discards.
    pub dropped: u64,
    /// Unconfirmed terminal outcomes.
    pub unknown: u64,
    /// Retired details, never treated as successful.
    pub expired: u64,
    /// Requested targets absent from individual routes.
    pub not_routed: u64,
    /// Rows observed during metadata publication/reporting.
    pub busy: u64,
    /// All rows in this frozen range have been scanned.
    pub finished: bool,
    /// Count representation saturated; full success cannot be claimed.
    pub overflowed: bool,
}
impl QuerySummary {
    /// True only after every page was scanned without a gap/failure/uncertainty.
    pub fn is_complete(&self) -> bool {
        self.finished
            && !self.overflowed
            && self.pending == 0
            && self.failed == 0
            && self.dropped == 0
            && self.unknown == 0
            && self.expired == 0
            && self.busy == 0
    }
    /// No nonterminal/unscanned rows remain, regardless of the terminal result.
    pub fn is_settled(&self) -> bool {
        self.finished && !self.overflowed && self.pending == 0 && self.busy == 0
    }
}
/// Result of initializing a bounded prefix of the caller's page buffer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PageReport {
    /// Initialized prefix length; the remainder of the buffer is untouched.
    pub items: usize,
    /// Cumulative counts, including whether more pages remain.
    pub summary: QuerySummary,
}
/// General-path wait outcome; timeout/cancellation do not alter records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaitOutcome {
    /// All requested boundaries were confirmed.
    Complete,
    /// All rows are terminal, with failures/unknown/expired possible.
    Settled,
    /// Monotonic deadline reached; includes the last bounded page/summary.
    TimedOut,
    /// Caller cancelled this query, leaving processing untouched.
    Cancelled,
}
/// Last bounded page and accumulated summary returned by a wait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitReport {
    /// Why waiting ended.
    pub outcome: WaitOutcome,
    /// Counts and initialized prefix for the last page in the caller buffer.
    pub page: PageReport,
}
/// Session counters distinguish retained detail from retired terminal outcomes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompletionTotals {
    /// Accepted records, not multiplied by fanout.
    pub records: u64,
    /// Target assignments included in this selection.
    pub assigned: u64,
    /// Assignments confirmed at their configured completion boundary.
    pub complete: u64,
    /// Terminal failures before the configured boundary.
    pub failed: u64,
    /// Terminal policy discards.
    pub dropped: u64,
    /// Terminal uncertainty or unreported output release.
    pub unknown: u64,
    /// Retained assignments that are still pending.
    pub pending: u64,
    /// Physical slots temporarily being initialized/reported.
    pub busy: u64,
    /// Retired record details; aggregate terminal counts remain available.
    pub expired_records: u64,
    /// Some counter saturated; no full-success claim is valid.
    pub overflowed: bool,
}
