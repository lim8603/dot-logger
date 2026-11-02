//! Portable capability reports for startup validation.

/// An advanced platform operation that may be requested during startup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    /// Place background workers on explicitly selected CPUs.
    WorkerAffinity,
    /// Change the scheduling priority of background workers.
    WorkerPriority,
    /// Synchronize directory metadata after file operations.
    DirectorySync,
    /// Request the platform's stronger file synchronization operation.
    FullSync,
}

/// Whether a requested operation is mandatory for startup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Requirement {
    /// Keep the reported status and allow a documented fallback.
    Optional,
    /// Reject startup unless the operation is supported.
    Required,
}

/// A platform capability together with its startup requirement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityRequest {
    /// Operation being requested.
    pub capability: Capability,
    /// Whether lack of support prevents startup.
    pub requirement: Requirement,
}

/// Observed support in the current environment, including permissions.
///
/// `Supported` describes a capability probe; it does not assert that worker
/// placement or synchronization has already occurred. Startup must still report
/// application failures when those operations are implemented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityStatus {
    /// The platform adapter verified support in this environment.
    Supported,
    /// The platform adapter does not support the operation.
    Unsupported,
    /// Support has not been established, for example before a probe runs.
    Unavailable,
    /// The probe failed, including a permission failure.
    Failed,
}

/// Platform observations supplied by an application adapter or a test double.
///
/// CPU identifiers are the actual allowed set, which can be sparse under CPU
/// affinity or container restrictions. A CPU count is insufficient to infer it.
/// No operating system is assumed to support advanced operations by default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformCapabilities {
    /// CPU identifiers available to background workers.
    pub available_cpu_ids: Vec<usize>,
    /// Background worker placement support.
    pub worker_affinity: CapabilityStatus,
    /// Background worker scheduling priority support.
    pub worker_priority: CapabilityStatus,
    /// Directory synchronization support.
    pub directory_sync: CapabilityStatus,
    /// Strong file synchronization support.
    pub full_sync: CapabilityStatus,
}

impl Default for PlatformCapabilities {
    fn default() -> Self {
        Self {
            available_cpu_ids: Vec::new(),
            worker_affinity: CapabilityStatus::Unavailable,
            worker_priority: CapabilityStatus::Unavailable,
            directory_sync: CapabilityStatus::Unavailable,
            full_sync: CapabilityStatus::Unavailable,
        }
    }
}

impl PlatformCapabilities {
    /// Return the observed status without interpreting missing information as success.
    pub fn status(&self, capability: Capability) -> CapabilityStatus {
        match capability {
            Capability::WorkerAffinity => self.worker_affinity,
            Capability::WorkerPriority => self.worker_priority,
            Capability::DirectorySync => self.directory_sync,
            Capability::FullSync => self.full_sync,
        }
    }
}
/// Explicit power alarm DTOs for host adapters and deterministic simulation.
pub mod power;
