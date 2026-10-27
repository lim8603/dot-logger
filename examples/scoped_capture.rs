//! Lifecycle prototype for an application that returns a 'static host JoinHandle.
//! The host owns all logger memory, lends handles to scoped RT workers, joins
//! them, drains/reclaims, then drops the owner on this ordinary host thread.
//! The caller can join this host from its existing async shutdown bridge.
//! This is not a dot-probe integration or a real-time timing benchmark.

use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use dot_logger::capture::{
    CaptureError, CaptureOwner, ChannelConfig, Importance, Level, RecordMetadata,
};
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    WorkerThreadingConfig,
};
use dot_logger::platform::{PlatformCapabilities, Requirement};
use dot_logger::pool::RawRecordOutcome;

type HostError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Default)]
struct WorkerCounts {
    accepted: usize,
    rejected: usize,
    invalid: usize,
}

fn run_host() -> Result<WorkerCounts, HostError> {
    let config = SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 256 * 1024,
            raw_bytes: 256 * 1024,
            output_bytes: 256 * 1024,
            external_bytes: 0,
            control_bytes: 128 * 1024,
            worker_bytes: 128 * 1024,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 16,
                important_slots: 2,
            },
            raw: Some(PoolQuota {
                slots: 4,
                slot_bytes: 64,
                important_slots: 1,
            }),
        }],
        importance: ImportancePolicy::PrioritizeImportant,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    };
    let plan = SessionBuilder::new(config).validate(&PlatformCapabilities::default())?;
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)])?;
    let done = AtomicBool::new(false);
    let published = AtomicUsize::new(0);
    owner.scope(|mut capture| -> Result<WorkerCounts, HostError> {
        let Some((mut producer, mut consumer)) = capture.next() else {
            return Err("missing producer".into());
        };
        thread::scope(|threads| {
            let done_ref = &done;
            let published_ref = &published;
            let worker = threads.spawn(move || {
                let mut counts = WorkerCounts::default();
                for value in 0..8 {
                    // A rejected frame is counted and the next frame proceeds;
                    // this RT worker never waits/retries for capture capacity.
                    let mut loan = match producer.try_acquire(64, Importance::Normal) {
                        Ok(loan) => loan,
                        Err(CaptureError::Capacity) => {
                            counts.rejected += 1;
                            continue;
                        }
                        Err(_) => {
                            counts.invalid += 1;
                            continue;
                        }
                    };
                    loan.payload_mut().fill(value);
                    let metadata = RecordMetadata {
                        channel_id: 7,
                        schema_id: 99,
                        schema_version: 1,
                        mono_ns: u64::from(value),
                        real_ns: i64::from(value),
                        source_time: None,
                        level: Level::Info,
                        importance: None,
                    };
                    match producer.try_record_raw(metadata, loan) {
                        Ok(RawRecordOutcome::Accepted(_)) => {
                            counts.accepted += 1;
                            published_ref.store(counts.accepted, Ordering::Release);
                        }
                        Ok(RawRecordOutcome::Skipped(loan)) => {
                            counts.rejected += 1;
                            drop(loan);
                        }
                        Err(error) => {
                            counts.rejected += 1;
                            drop(error.loan);
                        }
                    }
                }
                done_ref.store(true, Ordering::Release);
                counts
            });
            let mut received = 0;
            loop {
                match consumer.try_take_raw() {
                    Ok(Some(record)) => {
                        received += 1;
                        assert_eq!(record.record().receipt.accepted_seq, received as u64);
                        assert_eq!(record.payload().len(), 64);
                        drop(record);
                    }
                    Ok(None) => {
                        if done.load(Ordering::Acquire)
                            && received == published.load(Ordering::Acquire)
                        {
                            break;
                        }
                        thread::yield_now(); // Management path only.
                    }
                    Err(error) => {
                        return Err(format!("consumer invariant failed: {error:?}").into());
                    }
                }
                consumer.reclaim_raw(4);
            }
            let counts = worker
                .join()
                .map_err(|_| HostError::from("worker panicked"))?;
            consumer.reclaim_raw(4);
            assert_eq!(counts.accepted, received);
            assert_eq!(counts.accepted + counts.rejected + counts.invalid, 8);
            assert_eq!(counts.invalid, 0);
            assert_eq!(
                consumer.raw_snapshot().map(|snapshot| snapshot.free),
                Some(4)
            );
            Ok(counts)
        })
    })
}

fn main() -> Result<(), HostError> {
    // 'static application handle; all borrowed logger handles stay inside host.
    let host = thread::spawn(run_host);
    let counts = host
        .join()
        .map_err(|_| HostError::from("host panicked"))??;
    println!(
        "scoped capture: accepted={}, rejected={}, owner dropped after worker join",
        counts.accepted, counts.rejected
    );
    Ok(())
}
