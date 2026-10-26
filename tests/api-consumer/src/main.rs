//! Independent application consuming only dot-logger's public startup API.

use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    WorkerThreadingConfig,
};
use dot_logger::platform::{PlatformCapabilities, Requirement};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const MIB: usize = 1024 * 1024;
    let config = SessionConfig {
        memory: MemoryBudget {
            total_bytes: 8 * MIB,
            input_bytes: MIB,
            raw_bytes: 0,
            output_bytes: 2 * MIB,
            external_bytes: 0,
            control_bytes: MIB,
            worker_bytes: 4 * MIB,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 64,
                slot_bytes: 256,
                important_slots: 16,
            },
            raw: None,
        }],
        importance: ImportancePolicy::default(),
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: MIB,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        // Default capability observations are Unavailable, never assumed supported.
        capabilities: vec![],
    };
    let plan = SessionBuilder::new(config).validate(&PlatformCapabilities::default())?;
    assert_eq!(plan.reserved_bytes(), 8 * MIB);
    assert_eq!(plan.raw_pool_bytes(), 0);
    println!(
        "startup plan validated: {} bytes reserved",
        plan.reserved_bytes()
    );
    Ok(())
}
