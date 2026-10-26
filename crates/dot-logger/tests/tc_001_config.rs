//! TC-001: observable startup configuration behavior.

#![allow(clippy::unwrap_used)]

use dot_logger::config::{
    ConfigError, ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, Resource,
    SessionBuilder, SessionConfig, WorkerThreadingConfig,
};
use dot_logger::platform::{
    Capability, CapabilityRequest, CapabilityStatus, PlatformCapabilities, Requirement,
};

const MIB: usize = 1024 * 1024;

fn fixture() -> SessionConfig {
    SessionConfig {
        memory: MemoryBudget {
            total_bytes: 64 * MIB,
            input_bytes: 4 * MIB,
            raw_bytes: 32 * MIB,
            output_bytes: 8 * MIB,
            external_bytes: 4 * MIB,
            control_bytes: 4 * MIB,
            worker_bytes: 12 * MIB,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 16,
                slot_bytes: 1024,
                important_slots: 4,
            },
            raw: Some(PoolQuota {
                slots: 4,
                slot_bytes: 8 * MIB,
                important_slots: 1,
            }),
        }],
        importance: ImportancePolicy::default(),
        workers: WorkerThreadingConfig {
            count: 2,
            stack_bytes: MIB,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    }
}

fn platform() -> PlatformCapabilities {
    PlatformCapabilities {
        available_cpu_ids: vec![0, 2, 4],
        worker_affinity: CapabilityStatus::Supported,
        worker_priority: CapabilityStatus::Unsupported,
        directory_sync: CapabilityStatus::Unavailable,
        full_sync: CapabilityStatus::Failed,
    }
}

#[test]
fn rejects_65_mib_reserved_inside_64_mib_total() {
    let mut config = fixture();
    config.memory.output_bytes += MIB;
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::BudgetExceeded {
            resource: Resource::Total,
            required: 65 * MIB,
            available: 64 * MIB,
        })
    );
}

#[test]
fn rejects_unsupported_required_capability() {
    let mut config = fixture();
    config.capabilities.push(CapabilityRequest {
        capability: Capability::WorkerPriority,
        requirement: Requirement::Required,
    });
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::CapabilityRequired {
            capability: Capability::WorkerPriority,
            status: CapabilityStatus::Unsupported,
        })
    );
}

#[test]
fn exact_budget_and_single_max_raw_record_are_valid() {
    let config = fixture();
    let validated = SessionBuilder::new(config.clone())
        .validate(&platform())
        .unwrap();
    assert_eq!(validated.config(), &config);
    assert_eq!(validated.reserved_bytes(), 64 * MIB);
    // Original payload storage is counted once, independent of output budgets.
    assert_eq!(validated.raw_pool_bytes(), 32 * MIB);
    assert_eq!(validated.platform(), &platform());
}

#[test]
fn optional_unavailable_and_failed_capabilities_remain_observable() {
    let mut config = fixture();
    for capability in [Capability::DirectorySync, Capability::FullSync] {
        config.capabilities.push(CapabilityRequest {
            capability,
            requirement: Requirement::Optional,
        });
    }
    let validated = SessionBuilder::new(config).validate(&platform()).unwrap();
    assert_eq!(
        validated.platform().directory_sync,
        CapabilityStatus::Unavailable
    );
    assert_eq!(validated.platform().full_sync, CapabilityStatus::Failed);
}

#[test]
fn required_unavailable_or_failed_capability_is_rejected() {
    for (capability, status) in [
        (Capability::DirectorySync, CapabilityStatus::Unavailable),
        (Capability::FullSync, CapabilityStatus::Failed),
    ] {
        let mut config = fixture();
        config.capabilities.push(CapabilityRequest {
            capability,
            requirement: Requirement::Required,
        });
        assert_eq!(
            SessionBuilder::new(config).validate(&platform()),
            Err(ConfigError::CapabilityRequired { capability, status })
        );
    }
}

#[test]
fn producer_reservations_cannot_exceed_input_or_raw_partition() {
    for resource in [Resource::Input, Resource::Raw] {
        let mut config = fixture();
        if resource == Resource::Input {
            config.memory.input_bytes = 1;
        } else {
            config.memory.raw_bytes -= 1;
        }
        assert!(matches!(SessionBuilder::new(config).validate(&platform()),
            Err(ConfigError::BudgetExceeded { resource: actual, .. }) if actual == resource));
    }
    let mut config = fixture();
    config.producers.push(config.producers[0].clone());
    assert!(matches!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::BudgetExceeded {
            resource: Resource::Raw,
            ..
        })
    ));
}

#[test]
fn checked_arithmetic_rejects_sum_product_and_accumulation_overflow() {
    let mut config = fixture();
    config.memory.input_bytes = usize::MAX;
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::ArithmeticOverflow)
    );

    let mut config = fixture();
    config.producers[0].raw.as_mut().unwrap().slots = usize::MAX;
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::ArithmeticOverflow)
    );

    let mut config = fixture();
    config.workers.count = usize::MAX;
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::ArithmeticOverflow)
    );

    let mut config = fixture();
    let raw = config.producers[0].raw.as_mut().unwrap();
    raw.slots = 2;
    raw.slot_bytes = usize::MAX / 2;
    config.producers.push(config.producers[0].clone());
    config.producers.push(config.producers[0].clone());
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::ArithmeticOverflow)
    );
}

#[test]
fn priority_reservations_are_bounded_and_policy_consistent() {
    assert_eq!(
        ImportancePolicy::default(),
        ImportancePolicy::PrioritizeImportant
    );
    let mut config = fixture();
    config.producers[0].small.important_slots = 17;
    assert!(matches!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::InvalidPool { .. })
    ));

    let mut config = fixture();
    config.importance = ImportancePolicy::TreatEqually;
    assert!(matches!(
        SessionBuilder::new(config.clone()).validate(&platform()),
        Err(ConfigError::InvalidPool { .. })
    ));
    config.producers[0].small.important_slots = 0;
    config.producers[0].raw.as_mut().unwrap().important_slots = 0;
    assert!(SessionBuilder::new(config).validate(&platform()).is_ok());
}

#[test]
fn zero_sized_or_absent_required_resources_fail_before_startup() {
    let mut configs = vec![];
    let mut config = fixture();
    config.memory.total_bytes = 0;
    configs.push(config);
    let mut config = fixture();
    config.producers.clear();
    configs.push(config);
    let mut config = fixture();
    config.producers[0].small.slots = 0;
    configs.push(config);
    let mut config = fixture();
    config.producers[0].small.slot_bytes = 0;
    configs.push(config);
    let mut config = fixture();
    config.producers[0].raw.as_mut().unwrap().slots = 0;
    configs.push(config);
    let mut config = fixture();
    config.workers.count = 0;
    configs.push(config);
    let mut config = fixture();
    config.workers.stack_bytes = 0;
    configs.push(config);
    for config in configs {
        assert!(SessionBuilder::new(config).validate(&platform()).is_err());
    }
}

#[test]
fn small_only_producer_does_not_need_a_raw_reservation() {
    let mut config = fixture();
    config.producers[0].raw = None;
    config.memory.raw_bytes = 0;
    let validated = SessionBuilder::new(config).validate(&platform()).unwrap();
    assert_eq!(validated.raw_pool_bytes(), 0);
}

#[test]
fn worker_stacks_are_charged_and_cpu_ids_respect_sparse_allowed_set() {
    let mut config = fixture();
    config.memory.worker_bytes = MIB;
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::BudgetExceeded {
            resource: Resource::Workers,
            required: 2 * MIB,
            available: MIB
        })
    );

    let mut config = fixture();
    config.workers.cpu_ids = vec![1];
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::InvalidCpu { cpu_id: 1 })
    );

    let mut config = fixture();
    config.workers.cpu_ids = vec![2, 2];
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::DuplicateCpu { cpu_id: 2 })
    );

    let mut config = fixture();
    config.workers.cpu_ids = vec![2, 4];
    config.workers.affinity = Requirement::Required;
    assert!(
        SessionBuilder::new(config.clone())
            .validate(&platform())
            .is_ok()
    );
    let mut unsupported = platform();
    unsupported.worker_affinity = CapabilityStatus::Unsupported;
    assert!(matches!(
        SessionBuilder::new(config.clone()).validate(&unsupported),
        Err(ConfigError::CapabilityRequired { .. })
    ));
    config.workers.affinity = Requirement::Optional;
    assert!(SessionBuilder::new(config).validate(&unsupported).is_ok());
}

#[test]
fn required_affinity_needs_an_explicit_cpu_list() {
    let mut config = fixture();
    config.workers.affinity = Requirement::Required;
    assert_eq!(
        SessionBuilder::new(config).validate(&platform()),
        Err(ConfigError::MissingAffinityCpus)
    );
}

#[test]
fn validation_is_repeatable() {
    let builder = SessionBuilder::new(fixture());
    assert_eq!(builder.validate(&platform()), builder.validate(&platform()));
}
