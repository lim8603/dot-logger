//! Bounded control commands and honest shutdown reports.
use crate::completion::{CompletionError, CompletionTotals};
use crate::config::ConfigError;
use crate::routing::RoutingError;
use std::fmt;
/// Explicit control reservations; a separate slot is always reserved for shutdown.
#[derive(Clone, Copy, Debug)]
pub struct ControlConfig {
    /// Concurrent ordinary sync requests; each freezes all producer range storage.
    pub sync_slots: usize,
    /// Maximum physical completion cells examined by one control poll.
    pub work_per_poll: usize,
}
/// Non-owning sync-request identity. Generations never wrap or identify a new request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncToken {
    pub(crate) session_id: u128,
    pub(crate) owner_epoch: u64,
    pub(crate) slot: usize,
    pub(crate) generation: u64,
}
/// Actual adapter result for one explicit barrier, without conflating write and sync.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncResult {
    /// Writer confirms the requested producer cutoffs were synchronized.
    Synced,
    /// Explicit synchronization failed; earlier Written observations remain valid.
    Failed,
    /// Result is unknown, including an operation guard released without acknowledgment.
    Unknown,
}
/// Per-request progress; successful barriers alone do not erase record failures.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SyncSnapshot {
    /// Commands waiting for prior ingress/IO to drain.
    pub queued: usize,
    /// Actual operations still held by writer guards, including after deadline.
    pub in_flight: usize,
    /// Acknowledged barriers whose retained receipt details are being updated.
    pub applying: usize,
    /// Successful target barriers with metadata publication finished.
    pub succeeded: usize,
    /// Failed target barriers.
    pub failed: usize,
    /// Unknown target barrier outcomes.
    pub unknown: usize,
    /// Commands that expired before starting IO.
    pub expired: usize,
    /// Monotonic request deadline has been reached.
    pub deadline_elapsed: bool,
    /// Every selected command is done/expired, with no active guard.
    pub finished: bool,
}
impl SyncSnapshot {
    /// Barrier execution succeeded on time. Query record scopes separately to
    /// observe prior drops/failures/expired detail; a barrier is not a record count.
    pub fn barriers_succeeded(&self) -> bool {
        self.finished
            && !self.deadline_elapsed
            && self.failed == 0
            && self.unknown == 0
            && self.expired == 0
    }
}
/// Owner-held control progress, without starting backend IO itself.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ControlProgress {
    /// Physical receipt cells examined while applying acknowledged barriers.
    pub examined: usize,
    /// Receipt boundary/outcome updates published this poll.
    pub applied: usize,
}
/// Concrete shutdown phase; an Incomplete result keeps the coordinator usable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShutdownPhase {
    /// Input remains open; shutdown has not been requested.
    Running,
    /// Admission is closed; producers/queues/IO/sync/reclamation still need progress.
    Closing,
    /// Resources drained and target attempts settled; inspect failures separately.
    Closed,
}
/// Frozen plan for normal shutdown, excluding the optional UPS protocol.
#[derive(Clone, Copy, Debug)]
pub struct ShutdownPlan {
    /// Execute explicit file synchronization before closure. Required when any
    /// configured completion boundary is Synced. No automatic success on Drop.
    pub sync_files: bool,
}
/// Bounded aggregate snapshot. Per-target/receipt details remain available through
/// completion queries and target snapshots rather than an unbounded report vector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShutdownReport {
    /// Current admission/drain phase.
    pub phase: ShutdownPhase,
    /// Original monotonic deadline was reached, even if later cleanup completes.
    pub deadline_elapsed: bool,
    /// All target terminal outcomes, including retired-history aggregate counts.
    pub totals: CompletionTotals,
    /// Required-target outcomes, without optional target successes masking failures.
    pub required: CompletionTotals,
    /// Input envelopes still queued.
    pub ingress: usize,
    /// Output envelopes still queued.
    pub queued: usize,
    /// Pending enqueue retries.
    pub pending: usize,
    /// Output payload guards held by consumers/IO.
    pub in_flight: usize,
    /// Completion feedback awaiting publication.
    pub feedback: usize,
    /// Originals currently loaned to the application.
    pub raw_loaned: usize,
    /// Originals retained by readers/outputs.
    pub raw_in_use: usize,
    /// Originals still in ingress.
    pub raw_queued: usize,
    /// Originals marked for the bounded return scan.
    pub raw_return_pending: usize,
    /// Producer calls that may still publish pre-close admission or loan state.
    pub producer_calls: usize,
    /// Ordinary or shutdown sync guards still held by writers.
    pub sync_in_flight: usize,
    /// Any explicit shutdown/ordinary sync request failed, expired, or became unknown.
    pub sync_failed: bool,
}
impl ShutdownReport {
    /// True only for on-time closure with all required boundaries confirmed and
    /// no failed synchronization. Optional failures remain visible in totals.
    pub fn is_success(&self) -> bool {
        self.phase == ShutdownPhase::Closed
            && !self.deadline_elapsed
            && !self.sync_failed
            && !self.required.overflowed
            && self.required.pending == 0
            && self.required.busy == 0
            && self.required.failed == 0
            && self.required.dropped == 0
            && self.required.unknown == 0
    }
}
/// Explicit poll result; neither incomplete variant consumes or drops its owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShutdownPoll {
    /// Drain is still progressing before the deadline.
    Closing(ShutdownReport),
    /// Deadline reached; keep driving the same owner/IO until access actually ends.
    Incomplete(ShutdownReport),
    /// Resource cleanup completed. The report can still contain target failures.
    Closed(ShutdownReport),
}
/// Startup, command, or lifecycle error with bounded fixed metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlError {
    /// Control requires tracked ingress and positive explicit reservations.
    InvalidConfig,
    /// Control metadata does not fit the remaining control partition.
    Config(ConfigError),
    /// Completion query/update rejected the command.
    Completion(CompletionError),
    /// Dispatcher does not match this owner, or its resource view failed.
    Routing(RoutingError),
    /// Ordinary request slots are exhausted; shutdown keeps its dedicated slot.
    Capacity,
    /// Request token is stale, unknown, or reserved for internal shutdown use.
    InvalidToken,
    /// Request is being published/queried elsewhere; retry on the general path.
    Busy,
    /// Sync is not implemented for a requested target or shutdown omits required sync.
    UnsupportedBoundary,
    /// Supplied monotonic time moved backwards or predates construction.
    ClockWentBackwards,
    /// A deadline cannot be represented in the explicit control clock domain.
    InvalidDeadline,
    /// An ordinary sync request was already expired before submission.
    DeadlineExceeded,
    /// Caller storage cannot hold every registered producer cutoff.
    BufferTooSmall,
    /// Cannot retire a request while queued/applying/active IO still exists.
    InFlight,
    /// New ordinary control work is closed during shutdown.
    Closing,
}
impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "control error: {self:?}")
    }
}
impl std::error::Error for ControlError {}
impl From<CompletionError> for ControlError {
    fn from(error: CompletionError) -> Self {
        Self::Completion(error)
    }
}
impl From<RoutingError> for ControlError {
    fn from(error: RoutingError) -> Self {
        Self::Routing(error)
    }
}
