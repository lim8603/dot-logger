//! Ordinary log hook -> bounded textual payload -> explicit capture admission.
//! Run with --features tracing-adapter. No global logger or worker is installed.
use dot_logger::{
    adapters::tracing::log_record,
    capture::{CaptureOwner, ChannelConfig, Level, RecordMetadata, RecordOutcome},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 256 * 1024,
            raw_bytes: 0,
            output_bytes: 0,
            external_bytes: 0,
            control_bytes: 256 * 1024,
            worker_bytes: 512 * 1024,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 4,
                slot_bytes: 512,
                important_slots: 0,
            },
            raw: None,
        }],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())?;
    // Explicit application registration; schema 99/v1 is the textual JSON hook.
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)])?;
    let mut scratch = [0; 512];
    let encoded = log_record(
        &mut scratch,
        &log::Record::builder()
            .target("dot_probe.host")
            .level(log::Level::Info)
            .args(format_args!("ordinary tick={}", 7))
            .build(),
    )?;
    owner.scope(|mut scope| -> Result<(), Box<dyn std::error::Error>> {
        let (mut producer, mut consumer) = scope.next().ok_or("producer unavailable")?;
        let metadata = RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: 123,
            real_ns: 456,
            source_time: None,
            level: Level::Trace,
            importance: None,
        };
        let outcome = encoded
            .capture(&mut producer, metadata)
            .map_err(|e| format!("capture: {e:?}"))?;
        let RecordOutcome::Accepted(receipt) = outcome else {
            return Err("unexpected skip".into());
        };
        let mut payload = [0; 512];
        let captured = consumer
            .try_read(&mut payload)
            .map_err(|e| format!("consumer: {e:?}"))?
            .ok_or("record unavailable")?;
        if captured.receipt != receipt || &payload[..captured.payload_len] != encoded.payload {
            return Err("capture identity/payload mismatch".into());
        }
        println!("{}", std::str::from_utf8(&payload[..captured.payload_len])?);
        println!("receipt={receipt:?}; boundary=IngressAccepted; outputs=NotRun");
        Ok(())
    })
}
