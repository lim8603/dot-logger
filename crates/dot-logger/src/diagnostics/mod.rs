//! Load-free check plans and explicitly requested synchronous file checks.
//!
//! `CheckPlan::build`/`write_allowance` perform no IO, clock reads, worker startup
//! or network work. `CheckPlan::run_file` explicitly exercises a finite synthetic
//! product-MCAP file in a dedicated namespace, then syncs, closes and cleans it.
//! Results cover only the declared file scope, not capture/routing/remote work,
//! hardware capacity, RT, UPS or long-duration behavior. The host supplies a real
//! space probe, cancellation and a stable exclusively controlled trusted parent.
mod file;
mod host;
mod run;
pub use host::{CheckEvidence, CheckHost, CheckSource, HostError, linux_available_bytes};
pub use run::{
    CheckIssue, CheckOutcome, CheckReport, CheckRuntime, CheckStage, CheckStartError, CheckStop,
    FileCheck, FileCheckRuntime, FileWorkload,
};

use crate::completion::Boundary;
use std::path::{Component, PathBuf};

/// Explicit limits, with no product defaults or inferred hardware capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckBudgets {
    /// Maximum elapsed session-monotonic nanoseconds for new load admission.
    /// In-flight OS IO and final drain/sync may outlast this admission deadline.
    pub duration_ns: u64,
    /// Maximum physical output bytes, including envelopes/index/footer overhead.
    /// Payload bytes alone must not be supplied as physical write accounting.
    pub write_bytes: u64,
    /// Total declared check memory allowance, including buffers/stacks/metadata.
    pub memory_bytes: usize,
    /// Filesystem space that must remain available to other operations.
    pub disk_reserve_bytes: u64,
    /// Explicit CPU worker count; not a utilization or throughput estimate.
    pub workers: u16,
}

/// Planned network scope. No variant constitutes a measurement or verification
/// of receiver availability; actual remote completion requires receiver ACKs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPlan {
    /// No receiver: leave network performance unavailable, never zero or Pass.
    UnavailableNoReceiver,
    /// Only local testing requested; cannot validate remote performance.
    LoopbackOnly,
    /// Host-selected receiver identity, still unmeasured and unverified.
    RemoteUnmeasured {
        /// Bounded host receiver identity, rather than an arbitrary remote path.
        peer_id: u128,
    },
}

/// Reproducible host declarations preserved verbatim by the plan. The workload
/// ID identifies the host's bounded input profile; this API generates no input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckSpec {
    /// Original session identity.
    pub session_id: u128,
    /// Configuration revision selected for this check.
    pub revision: u64,
    /// Hardware/OS/feature identity supplied by the host.
    pub environment_id: u128,
    /// Payload/fanout/codec/offered-load profile identity supplied by the host.
    pub workload_id: u128,
    /// Absolute dedicated check directory. Planning does not create it, resolve
    /// symlinks or authorize deleting an existing directory or operating files.
    pub directory: PathBuf,
    /// Explicit upper bounds displayed before a separate run request.
    pub budgets: CheckBudgets,
    /// Host-calculated total reservation, including plan/path, buffers, worker
    /// stacks and metadata. Checked against the allowance, not measured here.
    pub required_memory_bytes: usize,
    /// Requested completion boundary, not proof of platform support/completion.
    pub boundary: Boundary,
    /// Requested receiver scope; missing receivers never become local evidence.
    pub network: NetworkPlan,
}

/// A declaration failed validation before any check workload or IO.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanError {
    /// Empty limits/reservation, no workers, or declared memory exceeds budget.
    InvalidBudget,
    /// Empty/relative/root path or parent traversal is ambiguous for later run.
    InvalidDirectory,
}
impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "check plan: {self:?}")
    }
}
impl std::error::Error for PlanError {}

/// Host observations at a new-load boundary. Include outstanding write
/// reservations in `written_bytes`; actual owners retain them until IO ends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Progress {
    /// Elapsed nanoseconds from the run's original session-monotonic start.
    pub elapsed_ns: u64,
    /// Physical bytes written plus reserved outstanding bytes, including final
    /// drain/index/footer. Do not subtract removed files or use payload length.
    pub written_bytes: u64,
    /// Fresh filesystem free bytes, reduced by outstanding unwritten reservations.
    /// Space shared with other processes cannot be guaranteed by a snapshot.
    pub free_disk_bytes: u64,
    /// Explicit host cancellation; does not mean in-flight IO has stopped.
    pub cancelled: bool,
}

/// Why new load must stop. These are admission decisions, not final check
/// outcomes: the host still drains, syncs, closes and reports actual scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopReason {
    /// Explicit cancellation.
    Cancelled,
    /// No filesystem bytes remain above the declared reserve.
    DiskReserve,
    /// Elapsed admission time reached the limit.
    TimeBudget,
    /// Written/reserved physical bytes reached the limit.
    WriteBudget,
}

/// Validated declarations only. Construction consumes the specification without
/// cloning its path or accessing runtime/operating resources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckPlan {
    spec: CheckSpec,
}
impl CheckPlan {
    /// Validate declarations without filesystem access or load generation.
    /// Existence/ownership, platform capability and actual reservations must be
    /// checked again by the host on a separate, explicit execution request.
    pub fn build(spec: CheckSpec) -> Result<Self, PlanError> {
        let b = spec.budgets;
        if b.duration_ns == 0
            || b.write_bytes == 0
            || b.memory_bytes == 0
            || b.workers == 0
            || spec.required_memory_bytes == 0
            || spec.required_memory_bytes > b.memory_bytes
        {
            return Err(PlanError::InvalidBudget);
        }
        if !spec.directory.is_absolute()
            || spec.directory.file_name().is_none()
            || spec
                .directory
                .components()
                .any(|c| c == Component::ParentDir)
        {
            return Err(PlanError::InvalidDirectory);
        }
        Ok(Self { spec })
    }

    /// Original declarations; no runtime resources were acquired by the plan.
    pub fn spec(&self) -> &CheckSpec {
        &self.spec
    }

    /// Maximum additional physical bytes that may be reserved for new load.
    /// Never schedule an indivisible write larger than this allowance. The host
    /// serializes admission/reservations and includes finalization overhead;
    /// this stateless calculation does not reserve space or cancel existing IO.
    /// Simultaneous stop observations prefer cancellation, reserve, time, bytes;
    /// no ordering of events between host observations is inferred.
    pub fn write_allowance(&self, progress: Progress) -> Result<u64, StopReason> {
        if progress.cancelled {
            return Err(StopReason::Cancelled);
        }
        let b = self.spec.budgets;
        let disk = progress
            .free_disk_bytes
            .saturating_sub(b.disk_reserve_bytes);
        if disk == 0 {
            return Err(StopReason::DiskReserve);
        }
        if progress.elapsed_ns >= b.duration_ns {
            return Err(StopReason::TimeBudget);
        }
        let bytes = b.write_bytes.saturating_sub(progress.written_bytes);
        if bytes == 0 {
            return Err(StopReason::WriteBudget);
        }
        Ok(bytes.min(disk))
    }
}
