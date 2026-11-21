//! Repeatable enabled/disabled fixture, not a calibrated RT/performance verdict.
//! Run in release with controlled CPU/IO, repeating runs in reversed order.
use dot_logger::{capture::*, config::*, platform::*};
use std::{
    error::Error,
    hint::black_box,
    time::{Duration, Instant},
};
fn trial(enabled: bool, rounds: u64) -> Result<Duration, Box<dyn Error>> {
    let mib = 1024 * 1024;
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: mib,
            input_bytes: mib / 4,
            raw_bytes: 0,
            output_bytes: mib / 4,
            external_bytes: 0,
            control_bytes: mib / 4,
            worker_bytes: mib / 4,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 64,
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
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)])?;
    if enabled {
        owner.enable_telemetry()?;
    }
    owner.scope(|mut s| -> Result<Duration, Box<dyn Error>> {
        let t = s.telemetry();
        let Some((mut p, mut r)) = s.next() else {
            return Err("missing producer".into());
        };
        let metadata = RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: 0,
            real_ns: 0,
            source_time: None,
            level: Level::Info,
            importance: None,
        };
        let payload = [9; 64];
        let mut scratch = [0; 64];
        let start = Instant::now();
        for _ in 0..rounds {
            black_box(
                p.try_record_small(black_box(metadata), black_box(&payload))
                    .map_err(|_| "fixture capture failed")?,
            );
            black_box(
                r.try_read(&mut scratch)
                    .map_err(|_| "fixture drain failed")?,
            );
        }
        let elapsed = start.elapsed();
        if let Some(t) = t {
            let sample = t.sample(0, None)?;
            black_box(sample);
        }
        Ok(elapsed)
    })
}
fn main() -> Result<(), Box<dyn Error>> {
    let rounds = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u64>())
        .transpose()?
        .unwrap_or(1_000_000);
    if !(1..=100_000_000).contains(&rounds) {
        return Err("rounds must be 1..=100000000".into());
    }
    for enabled_first in [false, true] {
        for enabled in [enabled_first, !enabled_first] {
            let elapsed = trial(enabled, rounds)?;
            println!(
                "enabled={enabled} rounds={rounds} elapsed_ns={} boundary=ingress_and_drain device_acceptance=NotRun",
                elapsed.as_nanos()
            );
        }
    }
    Ok(())
}
