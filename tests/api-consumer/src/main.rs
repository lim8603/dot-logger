//! Independent application consuming dot-logger startup and small capture APIs.

use dot_logger::capture::{CaptureOwner, ChannelConfig, Level, RecordMetadata, RecordOutcome};
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
    // Scope registration happens before creating application RT threads.
    let mut ingress = CaptureOwner::new(&plan, 1, 1, &[ChannelConfig::new(7, 99, 1)])?;
    ingress.scope(|mut scope| {
        let Some((mut producer, mut consumer)) = scope.next() else {
            panic!("validated plan must provide its registered producer");
        };
        let metadata = RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: 123,
            real_ns: 456,
            source_time: None,
            level: Level::Info,
            importance: None,
        };
        assert!(matches!(
            producer.try_record_small(metadata, b"example"),
            Ok(RecordOutcome::Accepted(_))
        ));
        let mut payload = [0; 256];
        let Ok(Some(record)) = consumer.try_read(&mut payload) else {
            panic!("accepted ingress record must be readable");
        };
        assert_eq!(&payload[..record.payload_len], b"example");
    });
    println!(
        "startup plan validated: {} bytes reserved",
        plan.reserved_bytes()
    );
    Ok(())
}
