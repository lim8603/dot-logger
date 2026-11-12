//! Explicit memory reservations and startup validation.
//!
//! The facade ([`crate::logger`]) derives all of this from a `Budget` or preset;
//! this module is the advanced path.
//!
//! `SessionBuilder::validate` checks the memory total and its partitions,
//! per-producer small/raw pools, Important reservations, worker stacks, CPU lists
//! and required platform capabilities. It creates no execution resource, file or
//! thread. `ValidatedConfig` keeps the checked configuration and platform state
//! read-only; an actual start must re-check applied capabilities and final
//! allocations.
//!
//! The application states every memory number: no default budget or device
//! performance is assumed. A pool's payload is `slots × slot_bytes`; Important
//! reservations protect slots and bytes together. Input, raw metadata, output
//! references/copies and completion/control metadata are checked against their
//! partitions when created. The completion window must cover the larger of a
//! producer's small/raw Important reservations, and Normal admission leaves as
//! many reusable slots as the next Important reservation needs. File encoders,
//! indexes and upstream summary copies are reserved conservatively by count and
//! matched against allocation measurements. Custom sink buffers are adapter
//! budget; real workers and allocator internals belong to separate headroom.
//! None of this bounds whole-process memory, the OS page cache or device latency.
//!
//! Platform state comes from an adapter and defaults to `Unavailable`.
//! `Supported` is a probe result, not applied success or real-time performance.

use std::fmt;

use crate::platform::{
    Capability, CapabilityRequest, CapabilityStatus, PlatformCapabilities, Requirement,
};

/// Fixed memory partitions reserved by the application before startup.
///
/// These are logger-owned reservations, not an operating system RSS limit. Page
/// cache, kernel sockets, and application allocations are outside this budget.
/// Pool storage must include unused and reclaim-pending slots. Output references
/// and copies have their own partition; original payload storage is counted once.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryBudget {
    /// Maximum sum of all partitions, in bytes; no device-derived default.
    pub total_bytes: usize,
    /// Input envelopes, small payload pools, and capture bookkeeping.
    pub input_bytes: usize,
    /// Original payload pools, including all reserved slots.
    pub raw_bytes: usize,
    /// Output queues, writer buffers, encoding, chunks, and indexes.
    pub output_bytes: usize,
    /// Database, remote, query, and replay reservations.
    pub external_bytes: usize,
    /// Completion tracking, configuration, and telemetry reservations.
    pub control_bytes: usize,
    /// Worker stacks, allocator overhead, and other explicit headroom.
    pub worker_bytes: usize,
}

/// Fixed-size payload slots for one producer and one payload kind.
///
/// Physical payload storage is `slots * slot_bytes`. Important slots reserve
/// both slots and their bytes; Normal input cannot consume them. Slot metadata,
/// queue descriptors, and allocator overhead must be reserved separately in the
/// corresponding memory partitions when the capture allocator is implemented.
/// A small-only producer uses `None` for its raw pool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolQuota {
    /// Total preallocated slots, including Important reservations.
    pub slots: usize,
    /// Maximum single payload size and physical bytes per slot.
    pub slot_bytes: usize,
    /// Slots protected from Normal input; zero under `TreatEqually`.
    pub important_slots: usize,
}

/// Separate small-record and original-payload reservations for one producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerQuota {
    /// Small records cannot consume the original-payload reservation.
    pub small: PoolQuota,
    /// Optional pool for original sensor payloads.
    pub raw: Option<PoolQuota>,
}

/// Admission policy, independent of the ordinary log level.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImportancePolicy {
    /// Protect configured Important slots and bytes during admission.
    #[default]
    PrioritizeImportant,
    /// Use equal admission; Important reservations must be zero.
    TreatEqually,
}

/// Background worker reservations; this does not configure application RT threads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerThreadingConfig {
    /// Number of background workers reserved by this plan.
    pub count: usize,
    /// Explicit stack reservation per worker, in bytes.
    pub stack_bytes: usize,
    /// Allowed CPU identifiers to request; empty leaves placement to the OS.
    pub cpu_ids: Vec<usize>,
    /// Whether failure to support the requested placement prevents startup.
    pub affinity: Requirement,
}

/// Explicit startup plan for the configuration subset implemented by TASK-002.
///
/// Device-independent numeric defaults are deliberately absent. Storage targets,
/// schemas, durability, and completion plans are added with their implementing
/// tasks. No workers, files, or optional adapters start during validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionConfig {
    /// Global memory partitions.
    pub memory: MemoryBudget,
    /// Producer-local fixed payload reservations.
    pub producers: Vec<ProducerQuota>,
    /// Admission policy for all configured producers.
    pub importance: ImportancePolicy,
    /// Background worker resource plan.
    pub workers: WorkerThreadingConfig,
    /// Additional optional or required platform operations.
    pub capabilities: Vec<CapabilityRequest>,
}

/// Resource partition associated with a configuration error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Resource {
    /// Sum of the complete logger memory plan.
    Total,
    /// Input envelopes and small payloads.
    Input,
    /// Capture registration, completion, and management metadata.
    Control,
    /// Original payload storage.
    Raw,
    /// Output queues, shared metadata, and small-payload copies.
    Output,
    /// Completion metadata within the control partition.
    Completion,
    /// Background worker reservations.
    Workers,
}

/// Structured failure returned before any runtime resources are started.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// A configured amount cannot be represented on the target architecture.
    ArithmeticOverflow,
    /// A reservation does not fit its enclosing partition.
    BudgetExceeded {
        /// Partition that cannot fit the reservation.
        resource: Resource,
        /// Required bytes.
        required: usize,
        /// Configured available bytes.
        available: usize,
    },
    /// A required platform operation was not verified as supported.
    CapabilityRequired {
        /// Required operation.
        capability: Capability,
        /// Observed reason for rejection.
        status: CapabilityStatus,
    },
    /// Total memory must be explicitly positive.
    EmptyMemoryBudget,
    /// At least one producer is needed for a capture plan.
    NoProducers,
    /// A pool is empty or has reservations inconsistent with its policy.
    InvalidPool {
        /// Index of the affected producer.
        producer: usize,
        /// Input or original-payload partition.
        resource: Resource,
    },
    /// Worker count and per-worker stack reservation must both be positive.
    InvalidWorkers,
    /// Required CPU placement must identify its requested CPUs.
    MissingAffinityCpus,
    /// A requested CPU is not in the platform's allowed CPU set.
    InvalidCpu {
        /// Invalid CPU identifier.
        cpu_id: usize,
    },
    /// A requested CPU identifier occurs more than once.
    DuplicateCpu {
        /// Duplicate CPU identifier.
        cpu_id: usize,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => f.write_str("configuration arithmetic overflow"),
            Self::BudgetExceeded {
                resource,
                required,
                available,
            } => write!(
                f,
                "{resource:?} requires {required} bytes but only {available} are reserved"
            ),
            Self::CapabilityRequired { capability, status } => write!(
                f,
                "required capability {capability:?} has status {status:?}"
            ),
            Self::EmptyMemoryBudget => f.write_str("total memory budget must be positive"),
            Self::NoProducers => f.write_str("at least one producer is required"),
            Self::InvalidPool { producer, resource } => {
                write!(f, "producer {producer} has an invalid {resource:?} pool")
            }
            Self::InvalidWorkers => f.write_str("worker count and stack bytes must be positive"),
            Self::MissingAffinityCpus => {
                f.write_str("required affinity needs an explicit CPU list")
            }
            Self::InvalidCpu { cpu_id } => {
                write!(f, "CPU {cpu_id} is not available to this process")
            }
            Self::DuplicateCpu { cpu_id } => write!(f, "CPU {cpu_id} is listed more than once"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Immutable result of startup validation; does not own a running session.
///
/// A validated configuration cannot be changed without another validation:
///
/// ```compile_fail
/// use dot_logger::config::ValidatedConfig;
/// fn change_budget(plan: &ValidatedConfig) {
///     plan.config().memory.total_bytes = 0;
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedConfig {
    config: SessionConfig,
    platform: PlatformCapabilities,
    reserved_bytes: usize,
    raw_pool_bytes: usize,
}

impl ValidatedConfig {
    /// Read the validated configuration without allowing unchecked mutation.
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Read all observed capability statuses, including optional failures.
    pub fn platform(&self) -> &PlatformCapabilities {
        &self.platform
    }

    /// Sum of all logger memory partitions, including unused reservations.
    pub fn reserved_bytes(&self) -> usize {
        self.reserved_bytes
    }

    /// Physical original-payload bytes, counted once across all producers.
    pub fn raw_pool_bytes(&self) -> usize {
        self.raw_pool_bytes
    }
}

/// Management-path entry point for constructing a checked startup plan.
#[derive(Clone, Debug)]
pub struct SessionBuilder {
    config: SessionConfig,
}

impl SessionBuilder {
    /// Create a builder from an explicit plan without allocating runtime pools.
    pub fn new(config: SessionConfig) -> Self {
        Self { config }
    }

    /// Validate limits and capabilities without starting workers or performing I/O.
    ///
    /// Runtime startup must recheck capabilities when applying platform operations;
    /// this snapshot cannot guarantee future permissions or CPU availability.
    pub fn validate(
        &self,
        platform: &PlatformCapabilities,
    ) -> Result<ValidatedConfig, ConfigError> {
        let config = &self.config;
        let memory = &config.memory;
        if memory.total_bytes == 0 {
            return Err(ConfigError::EmptyMemoryBudget);
        }
        if config.producers.is_empty() {
            return Err(ConfigError::NoProducers);
        }
        let reserved_bytes = checked_sum([
            memory.input_bytes,
            memory.raw_bytes,
            memory.output_bytes,
            memory.external_bytes,
            memory.control_bytes,
            memory.worker_bytes,
        ])?;
        require_budget(Resource::Total, reserved_bytes, memory.total_bytes)?;

        let workers = &config.workers;
        if workers.count == 0 || workers.stack_bytes == 0 {
            return Err(ConfigError::InvalidWorkers);
        }
        let stack_bytes = workers
            .count
            .checked_mul(workers.stack_bytes)
            .ok_or(ConfigError::ArithmeticOverflow)?;
        require_budget(Resource::Workers, stack_bytes, memory.worker_bytes)?;

        let mut input_pool_bytes: usize = 0;
        let mut raw_pool_bytes: usize = 0;
        for (index, producer) in config.producers.iter().enumerate() {
            let input = pool_bytes(&producer.small, index, Resource::Input, config.importance)?;
            input_pool_bytes = input_pool_bytes
                .checked_add(input)
                .ok_or(ConfigError::ArithmeticOverflow)?;
            if let Some(raw) = &producer.raw {
                let raw = pool_bytes(raw, index, Resource::Raw, config.importance)?;
                raw_pool_bytes = raw_pool_bytes
                    .checked_add(raw)
                    .ok_or(ConfigError::ArithmeticOverflow)?;
            }
        }
        require_budget(Resource::Input, input_pool_bytes, memory.input_bytes)?;
        require_budget(Resource::Raw, raw_pool_bytes, memory.raw_bytes)?;

        if workers.affinity == Requirement::Required && workers.cpu_ids.is_empty() {
            return Err(ConfigError::MissingAffinityCpus);
        }
        for (index, &cpu_id) in workers.cpu_ids.iter().enumerate() {
            if !platform.available_cpu_ids.contains(&cpu_id) {
                return Err(ConfigError::InvalidCpu { cpu_id });
            }
            if workers.cpu_ids[..index].contains(&cpu_id) {
                return Err(ConfigError::DuplicateCpu { cpu_id });
            }
        }
        if !workers.cpu_ids.is_empty() {
            require_capability(
                platform,
                CapabilityRequest {
                    capability: Capability::WorkerAffinity,
                    requirement: workers.affinity,
                },
            )?;
        }
        for &request in &config.capabilities {
            require_capability(platform, request)?;
        }

        Ok(ValidatedConfig {
            config: config.clone(),
            platform: platform.clone(),
            reserved_bytes,
            raw_pool_bytes,
        })
    }
}

fn checked_sum(values: impl IntoIterator<Item = usize>) -> Result<usize, ConfigError> {
    values.into_iter().try_fold(0_usize, |sum, value| {
        sum.checked_add(value)
            .ok_or(ConfigError::ArithmeticOverflow)
    })
}

fn require_budget(
    resource: Resource,
    required: usize,
    available: usize,
) -> Result<(), ConfigError> {
    if required > available {
        return Err(ConfigError::BudgetExceeded {
            resource,
            required,
            available,
        });
    }
    Ok(())
}

fn pool_bytes(
    pool: &PoolQuota,
    producer: usize,
    resource: Resource,
    policy: ImportancePolicy,
) -> Result<usize, ConfigError> {
    if pool.slots == 0
        || pool.slot_bytes == 0
        || pool.important_slots > pool.slots
        || (policy == ImportancePolicy::TreatEqually && pool.important_slots != 0)
    {
        return Err(ConfigError::InvalidPool { producer, resource });
    }
    pool.slots
        .checked_mul(pool.slot_bytes)
        .ok_or(ConfigError::ArithmeticOverflow)
}

fn require_capability(
    platform: &PlatformCapabilities,
    request: CapabilityRequest,
) -> Result<(), ConfigError> {
    let status = platform.status(request.capability);
    if request.requirement == Requirement::Required && status != CapabilityStatus::Supported {
        return Err(ConfigError::CapabilityRequired {
            capability: request.capability,
            status,
        });
    }
    Ok(())
}
