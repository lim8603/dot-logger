//! Owned finite UPS simulation with actual capture, file sync and prefix recovery.
#![allow(dead_code)]
use dot_logger::{
    capture::*, completion::*, config::*, control::emergency::*, control::*, platform::power::*,
    platform::*, pool::RawRecordOutcome, routing::*, storage::*,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
/// Ordinary host error; no physical UPS certification is implied.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
fn invalid(text: &'static str) -> Error {
    io::Error::other(text).into()
}
/// Deterministic finite fault selection, not a product setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// All protected files must sync before the effective cutoff.
    Normal,
    /// First file sync returns an OS failure; lower ranks still get a chance.
    SyncFailure,
    /// First file append returns ENOSPC, preserving its failed receipt scope.
    WriteFailure,
    /// Shorten cutoff after rank one, so rank two cannot start.
    EarlierDeadline,
    /// Deliver an alarm after its deadline; never start new protected IO.
    Expired,
}
/// Real file facts and conservative partial status for one simulated event.
#[derive(Debug)]
pub struct CaseReport {
    /// Selected scenario.
    pub mode: Mode,
    /// Total preaccepted protected payload.
    pub payload_bytes: usize,
    /// Requested delivery delay, included in the original 3s budget.
    pub delay_ms: u128,
    /// Actual source-to-receipt interval.
    pub actual_delivery_ns: u128,
    /// Actual source-to-last protected confirmation interval.
    pub protection_ns: u128,
    /// Every requested file/receipt was protected before the effective cutoff.
    pub protected_complete: bool,
    /// Explicit partial/failure scope, never inferred from successful cleanup.
    pub partial: bool,
    /// Independent recovered source identities/bytes checked against the oracle.
    pub recovered_records: usize,
    /// Frozen receipt count; oracle exists independently from files/recovery.
    pub offered_records: usize,
    /// Actual ordered sync attempts.
    pub sync_order: Vec<u32>,
    /// Requested output reservations after writer/recovery owners actually drop.
    pub storage_after_close: usize,
    /// Actual final resource state on the original coordinator.
    pub shutdown: ShutdownPoll,
}
struct Temp {
    path: PathBuf,
    cleanup: bool,
}
struct ChildOwner(std::process::Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
impl Temp {
    fn new() -> Result<Self, Error> {
        Self::at(&std::env::temp_dir())
    }
    fn at(parent: &std::path::Path) -> Result<Self, Error> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = parent.join(format!(
            "dot-logger-ups-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self {
            path,
            cleanup: true,
        })
    }
    fn file(&self, id: u32, recovered: bool) -> PathBuf {
        self.path.join(format!(
            "{}-{id}.mcap",
            if recovered { "recovered" } else { "source" }
        ))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        if !self.cleanup {
            return;
        }
        // Only fixed files created with create_new in this exclusive namespace.
        for id in [10, 20] {
            for recovered in [false, true] {
                let _ = fs::remove_file(self.file(id, recovered));
            }
        }
        let _ = fs::remove_dir(&self.path);
    }
}

/// Spawn an owned child that exits without Drop; reopen and recover in the parent.
/// Returns verified receipt count. The external oracle is synced before spawn.
pub fn process_crash(
    executable: &std::path::Path,
    parent: &std::path::Path,
) -> Result<usize, Error> {
    let temp = Temp::at(parent)?;
    let oracle_path = temp.path.join("oracle.json");
    let mut oracle_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&oracle_path)?;
    writeln!(
        oracle_file,
        "{}",
        serde_json::json!({"session":"0000000000000000000000000000002a","revision":"1","sequences":[1,2,3,4,5,6,7,8],"payload_bytes_each":1024,"source_clock_domain":"9"})
    )?;
    oracle_file.sync_all()?;
    drop(oracle_file);
    File::open(&temp.path)?.sync_all()?;
    let result = (|| -> Result<usize, Error> {
        let mut child = ChildOwner(
            std::process::Command::new(executable)
                .arg("--crash-child")
                .arg(&temp.path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?,
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.0.kill();
                let _ = child.0.wait();
                return Err(invalid("owned UPS child deadline"));
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        if status.code() != Some(17) {
            return Err(invalid("UPS child did not exit at simulated interruption"));
        }
        let external: serde_json::Value = serde_json::from_slice(&fs::read(&oracle_path)?)?;
        let expected_sequences: Vec<u64> = external["sequences"]
            .as_array()
            .ok_or_else(|| invalid("oracle sequences missing"))?
            .iter()
            .map(|n| n.as_u64().ok_or_else(|| invalid("invalid oracle sequence")))
            .collect::<Result<_, _>>()?;
        let (plan, route, tracking) = setup(1024)?;
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1), ChannelConfig::new(8, 99, 1)],
            &route,
            tracking,
        )?;
        owner.scope(|mut scope| -> Result<usize, Error> {
            let completion = scope
                .completion()
                .ok_or_else(|| invalid("missing fresh completion"))?;
            let (_, input) = scope.next().ok_or_else(|| invalid("missing fresh input"))?;
            let router = Router::new(&plan, vec![input], route)?;
            let mut seen = Vec::new();
            for target in [10, 20] {
                let source_path = temp.file(target, false);
                let before = fs::read(&source_path)?;
                let mut source = File::open(&source_path)?;
                let recovered = recover_segment(
                    &mut source,
                    router.writer_budget(target)?,
                    channels(),
                    RecoveryOptions {
                        source_target: None,
                        source_sequence: 0,
                        writer: writer_config(1024),
                        limits: RecoveryConfig {
                            max_source_bytes: before.len() as u64,
                            max_record_bytes: 65536,
                            max_chunks: 32,
                        },
                    },
                    new_file(temp.file(target, true))?,
                    &AtomicBool::new(false),
                )?;
                if target == 10 && recovered.report.stop != RecoveryStop::Truncated {
                    return Err(invalid("torn child tail not excluded"));
                }
                drop(recovered.segment.into_inner());
                if fs::read(&source_path)? != before {
                    return Err(invalid("child recovery changed read-only source"));
                }
                let recovered_bytes = fs::read(temp.file(target, true))?;
                for message in NativeReader::new(&recovered_bytes, 1024, 4096)? {
                    let stored = message?;
                    let seq = stored.record.receipt.accepted_seq;
                    if !(1..=8).contains(&seq)
                        || stored.record.receipt.session_id != 42
                        || stored.record.revision != 1
                        || stored.record.metadata != metadata(seq)
                        || stored.payload.len() != 1024
                        || !stored.payload.iter().all(|b| *b == seq as u8)
                        || seen.contains(&seq)
                    {
                        return Err(invalid("child recovery differs from precrash oracle"));
                    }
                    if completion
                        .poll(stored.record.receipt, target, Boundary::Synced)
                        .is_ok_and(|s| s.outcome == QueryOutcome::Complete)
                    {
                        return Err(invalid("recovery fabricated previous completion"));
                    }
                    seen.push(seq);
                }
            }
            seen.sort_unstable();
            if seen != expected_sequences {
                return Err(invalid("child recovery missed oracle receipts"));
            }
            Ok(seen.len())
        })
    })();
    fs::remove_file(oracle_path)?;
    let path = temp.path.clone();
    drop(temp);
    if path.exists() {
        return Err(invalid("UPS child namespace not cleaned"));
    }
    result
}
struct FaultFile {
    file: File,
    fail_write: Arc<AtomicBool>,
    fail_sync: bool,
}
impl Write for FaultFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.fail_write.load(Ordering::Acquire) {
            return Err(io::Error::from_raw_os_error(28));
        }
        self.file.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
impl Seek for FaultFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}
impl SyncSink for FaultFile {
    fn sync_all(&mut self) -> io::Result<()> {
        if self.fail_sync {
            return Err(io::ErrorKind::Other.into());
        }
        self.file.sync_all()
    }
}
fn setup(block: usize) -> Result<(ValidatedConfig, RoutingConfig, CompletionConfig), Error> {
    let mib = 1024 * 1024;
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 256 * mib,
            input_bytes: 8 * mib,
            raw_bytes: 128 * mib,
            output_bytes: 96 * mib,
            external_bytes: 8 * mib,
            control_bytes: 8 * mib,
            worker_bytes: 8 * mib,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 16,
                slot_bytes: 64,
                important_slots: 8,
            },
            raw: Some(PoolQuota {
                slots: 8,
                slot_bytes: block,
                important_slots: 8,
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
    })
    .validate(&PlatformCapabilities::default())?;
    let routing = RoutingConfig {
        targets: [10, 20, 30]
            .into_iter()
            .map(|id| TargetConfig {
                id,
                required: id != 30,
                queue_slots: 8,
                pending_slots: 0,
                retained_bytes: 8 * block,
                max_attempts: 1,
                max_age: Duration::from_secs(180),
            })
            .collect(),
        routes: vec![
            Route {
                channel_id: 7,
                targets: vec![10, 30],
            },
            Route {
                channel_id: 8,
                targets: vec![20, 30],
            },
        ],
        shared_records: 8,
        small_copy_bytes: 64,
        schedule: Schedule {
            record_quantum: 8,
            byte_quantum: 8 * block,
            records_per_turn: 8,
            bytes_per_turn: 8 * block,
            retries_per_target: 1,
            reclaim_slots_per_producer: 8,
        },
    };
    let completion = CompletionConfig {
        history_retention: Duration::from_secs(180),
        window_records: 16,
        max_queries: 1,
        page_items: 16,
        feedback_events: 32,
        updates_per_turn: 32,
        reconcile_per_turn: 32,
        targets: [10, 20, 30]
            .into_iter()
            .map(|id| TargetContract {
                id,
                required: id != 30,
                supported: Boundaries::FILE,
                completion: if id == 30 {
                    Boundary::Written
                } else {
                    Boundary::Synced
                },
            })
            .collect(),
    };
    Ok((plan, routing, completion))
}
fn channels() -> Vec<StorageChannel> {
    [7, 8]
        .into_iter()
        .map(|id| StorageChannel {
            channel: ChannelConfig::new(id, 99, 1),
            topic: format!("ups/{id}"),
            payload_encoding: "raw".into(),
            schema: None,
        })
        .collect()
}
fn writer_config(block: usize) -> WriterConfig {
    WriterConfig {
        max_loss_reports: 0,
        max_payload_bytes: block,
        write_buffer_bytes: 4096,
        chunk_target_bytes: block,
        max_records: 16,
        max_chunks: 32,
        max_metadata_bytes: 4096,
    }
}
fn new_file(path: PathBuf) -> Result<File, Error> {
    Ok(OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(path)?)
}
fn metadata(seq: u64) -> RecordMetadata {
    RecordMetadata {
        channel_id: if seq <= 6 { 7 } else { 8 },
        schema_id: 99,
        schema_version: 1,
        mono_ns: seq,
        real_ns: seq as i64,
        source_time: Some(SourceTime {
            ticks: seq as i64,
            clock_domain: 9,
        }),
        level: Level::Info,
        importance: Some(Importance::Important),
    }
}
/// Execute a bounded real-file simulation. Supported payloads are divisible by8,
/// 8KiB..=64MiB; delay≤1s. No hardware, reboot, global service or existing file is used.
pub fn run(mode: Mode, total: usize, delay: Duration) -> Result<CaseReport, Error> {
    run_inner(mode, total, delay, None, None)
}
/// Run the same profile in an explicit trusted filesystem parent, such as the
/// approved local-pc-v1 ext4 volume. Existing data is never overwritten.
pub fn run_at(
    mode: Mode,
    total: usize,
    delay: Duration,
    parent: &std::path::Path,
) -> Result<CaseReport, Error> {
    run_inner(mode, total, delay, None, Some(parent))
}
/// Child-only abrupt exit after real sync, with active writers and no destructors.
pub fn crash_child(path: PathBuf) -> Result<(), Error> {
    run_inner(Mode::Normal, 8192, Duration::ZERO, Some(path), None)?;
    Err(invalid("crash child unexpectedly returned"))
}
fn run_inner(
    mode: Mode,
    total: usize,
    delay: Duration,
    crash_path: Option<PathBuf>,
    parent: Option<&std::path::Path>,
) -> Result<CaseReport, Error> {
    if !(8192..=64 * 1024 * 1024).contains(&total)
        || total % 8 != 0
        || delay > Duration::from_secs(1)
    {
        return Err(invalid("invalid UPS simulation bounds"));
    }
    let block = total / 8;
    let crash = crash_path.is_some();
    // The parent owns this directory; a failed internal child must never delete
    // supplied paths. Normal runs create their own exclusive namespace.
    let temp = if let Some(path) = crash_path {
        Temp {
            path,
            cleanup: false,
        }
    } else {
        Temp::at(parent.unwrap_or(&std::env::temp_dir()))?
    };
    let (plan, route, tracking) = setup(block)?;
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1), ChannelConfig::new(8, 99, 1)],
        &route,
        tracking,
    )?;
    let report = owner.scope(|mut scope| -> Result<CaseReport, Error> {
        let completion = scope
            .completion()
            .ok_or_else(|| invalid("missing completion"))?;
        let origin = Instant::now();
        let mut control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 256,
            },
            origin,
        )?;
        control.enable_emergency(
            &plan,
            EmergencyConfig {
                files: vec![
                    EmergencyFile {
                        target: 20,
                        rank: 2,
                        protected_bytes: (2 * block) as u64,
                        estimated_drain_sync: Duration::from_millis(100),
                    },
                    EmergencyFile {
                        target: 10,
                        rank: 1,
                        protected_bytes: (6 * block) as u64,
                        estimated_drain_sync: Duration::from_millis(100),
                    },
                ],
                safety_margin: Duration::from_millis(200),
            },
        )?;
        let (mut producer, input) = scope.next().ok_or_else(|| invalid("missing producer"))?;
        let mut oracle = Vec::with_capacity(8);
        for seq in 1..=8 {
            let mut loan = producer
                .try_acquire(block, Importance::Important)
                .map_err(|_| invalid("snapshot pool capacity"))?;
            loan.payload_mut().fill(seq as u8);
            let RawRecordOutcome::Accepted(receipt) = producer
                .try_record_raw(metadata(seq), loan)
                .map_err(|_| invalid("snapshot capture rejected"))?
            else {
                return Err(invalid("snapshot unexpectedly skipped"));
            };
            oracle.push(CapturedRecord {
                receipt,
                metadata: metadata(seq),
                revision: 1,
                payload_len: block,
            });
        }
        let mut router = Router::new(&plan, vec![input], route)?;
        let budget = router.writer_budget(10)?;
        let fail = Arc::new(AtomicBool::new(false));
        let mut writers = Vec::new();
        for target in [10, 20] {
            writers.push(Some(SegmentWriter::new(
                router.writer_budget(target)?,
                writer_config(block),
                channels(),
                FaultFile {
                    file: new_file(temp.file(target, false))?,
                    fail_write: if target == 10 {
                        fail.clone()
                    } else {
                        Arc::new(AtomicBool::new(false))
                    },
                    fail_sync: target == 10 && mode == Mode::SyncFailure,
                },
            )?));
        }
        // Persist the directory entries before the simulated alarm, independently
        // from the per-file emergency sync boundary.
        File::open(&temp.path)?.sync_all()?;
        let generated = Instant::now();
        if !delay.is_zero() {
            std::thread::sleep(delay)
        }
        let received = Instant::now();
        control.notify_power(
            PowerAlarm::new(
                1,
                generated,
                if mode == Mode::Expired {
                    Duration::from_nanos(1)
                } else {
                    Duration::from_secs(3)
                },
            )
            .ok_or_else(|| invalid("alarm overflow"))?,
            received,
        )?;
        if mode == Mode::WriteFailure {
            fail.store(true, Ordering::Release)
        }
        let mut written = [0usize; 2];
        let mut sync_order = Vec::new();
        let last = router.scope(|mut d, mut outputs| -> Result<EmergencyReport, Error> {
            let first = control
                .poll_emergency(Instant::now(), &mut d)?
                .ok_or_else(|| invalid("missing power event"))?;
            control.observe_protected_bytes(10, (6 * block) as u64)?;
            control.observe_protected_bytes(20, (2 * block) as u64)?;
            let token = control
                .shutdown_sync()
                .ok_or_else(|| invalid("missing frozen cutoff"))?;
            let mut ranges = [ProducerRange::default(); 1];
            let mut report = first;
            d.turn(Instant::now(), &mut [None; 32])?;
            // Drop cancelled optional work by polling its own FIFO; active guards
            // would remain valid. Its outcome is not required-file success.
            for _ in 0..8 {
                if outputs[2].try_next(Instant::now()).is_none() {
                    break;
                }
            }
            for (index, target) in [10, 20].into_iter().enumerate() {
                if index == 1 && mode == Mode::EarlierDeadline {
                    let now = Instant::now();
                    control.notify_power(
                        PowerAlarm::new(2, now, Duration::from_millis(200))
                            .ok_or_else(|| invalid("alarm overflow"))?,
                        now,
                    )?;
                    report = control
                        .poll_emergency(Instant::now(), &mut d)?
                        .ok_or_else(|| invalid("missing alarm"))?;
                }
                if Instant::now() >= report.work_deadline {
                    d.cancel_target(target);
                }
                let writer = writers[index]
                    .as_mut()
                    .ok_or_else(|| invalid("writer closed"))?;
                for _ in 0..8 {
                    let Some(next) = outputs[index].try_next(Instant::now()) else {
                        break;
                    };
                    let delivery = match next {
                        Ok(delivery) => delivery,
                        Err(_) => continue,
                    };
                    if Instant::now() >= report.work_deadline {
                        delivery.report(Update::Dropped(DropReason::Cancelled))?;
                        continue;
                    }
                    match writer.write_delivery(&delivery) {
                        Ok(()) => written[index] += 1,
                        Err(_) => {
                            delivery.report(Update::Failed(FailureReason::IoFailure))?;
                        }
                    }
                }
                d.turn(Instant::now(), &mut [None; 32])?;
                if let Some(work) =
                    control.next_sync(token, target, Instant::now(), &mut d, &mut ranges)?
                {
                    sync_order.push(target);
                    if written[index] > 0 {
                        // Failure is published by the actual adapter. Preflight
                        // failure retains a guard, which is explicitly Unknown.
                        let _ = writer.sync_work(work);
                    } else {
                        work.complete(SyncResult::Failed);
                    }
                }
                report = control
                    .poll_emergency(Instant::now(), &mut d)?
                    .ok_or_else(|| invalid("missing alarm"))?;
            }
            for _ in 0..256 {
                d.turn(Instant::now(), &mut [None; 32])?;
                report = control
                    .poll_emergency(Instant::now(), &mut d)?
                    .ok_or_else(|| invalid("missing alarm"))?;
                if matches!(report.shutdown, ShutdownPoll::Closed(_)) {
                    return Ok(report);
                }
            }
            Err(invalid("UPS cleanup did not quiesce within bounded turns"))
        })?;
        let protection_ns = Instant::now().duration_since(generated).as_nanos();
        if crash {
            if !last.protected_complete {
                return Err(invalid("child did not protect all receipts"));
            }
            // A torn trailing container record is outside the last complete chunk.
            // Parent's preexisting receipt oracle is independent of crash diagnostics.
            let mut torn = OpenOptions::new().append(true).open(temp.file(10, false))?;
            torn.write_all(&[6, 255, 255])?;
            torn.sync_all()?;
            std::process::exit(17);
        }
        // Do not finalize: a power-interrupted active segment has no footer.
        // Synced complete chunks must recover independently from the old owner.
        drop(writers);
        fail.store(false, Ordering::Release);
        let mut recovered_records = 0;
        for (index, target) in [10, 20].into_iter().enumerate() {
            if written[index] == 0 {
                continue;
            }
            let mut source = File::open(temp.file(target, false))?;
            let source_len = source.metadata()?.len();
            let source_hash = fs::read(temp.file(target, false))?;
            let recovered = recover_segment(
                &mut source,
                router.writer_budget(target)?,
                channels(),
                RecoveryOptions {
                    source_target: None,
                    source_sequence: 0,
                    writer: writer_config(block),
                    limits: RecoveryConfig {
                        max_source_bytes: source_len,
                        max_record_bytes: 2 * block + 65536,
                        max_chunks: 32,
                    },
                },
                new_file(temp.file(target, true))?,
                &AtomicBool::new(false),
            )?;
            if recovered.report.records != written[index] {
                return Err(invalid("recovery record count changed"));
            }
            drop(recovered.segment.into_inner());
            if fs::read(temp.file(target, false))? != source_hash {
                return Err(invalid("recovery modified source"));
            }
            let bytes = fs::read(temp.file(target, true))?;
            let mut seen = Vec::new();
            for message in NativeReader::new(&bytes, block, 4096)? {
                let stored = message?;
                let expected = oracle
                    .iter()
                    .find(|r| r.receipt == stored.record.receipt)
                    .ok_or_else(|| invalid("foreign recovered receipt"))?;
                if *expected != stored.record
                    || stored.payload.len() != block
                    || !stored
                        .payload
                        .iter()
                        .all(|byte| *byte == expected.receipt.accepted_seq as u8)
                    || seen.contains(&expected.receipt.accepted_seq)
                {
                    return Err(invalid(
                        "recovered metadata/payload differs from external oracle",
                    ));
                }
                seen.push(expected.receipt.accepted_seq);
                recovered_records += 1;
            }
            if seen.len() != written[index] {
                return Err(invalid("duplicate or missing recovered receipt"));
            }
        }
        let storage_after_close = budget.used_bytes();
        if storage_after_close != 0 {
            return Err(invalid("UPS output reservation still owned"));
        }
        for original in &oracle {
            let target = if original.metadata.channel_id == 7 {
                10
            } else {
                20
            };
            if mode == Mode::Normal
                && completion
                    .poll(original.receipt, target, Boundary::Synced)?
                    .outcome
                    != QueryOutcome::Complete
            {
                return Err(invalid("normal UPS receipt not synced"));
            }
        }
        if mode == Mode::Normal && !last.protected_complete {
            return Err(invalid("normal UPS protection missed deadline"));
        }
        // Preserve original result, not the elapsed recovery/cleanup observation.
        let protected_complete = last.protected_complete;
        Ok(CaseReport {
            mode,
            payload_bytes: total,
            delay_ms: delay.as_millis(),
            actual_delivery_ns: received.duration_since(generated).as_nanos(),
            protection_ns,
            protected_complete,
            partial: !protected_complete,
            recovered_records,
            offered_records: oracle.len(),
            sync_order,
            storage_after_close,
            shutdown: last.shutdown,
        })
    })?;
    // Temp cleanup has a fixed owned file list; successful runs verify deletion.
    let path = temp.path.clone();
    drop(temp);
    if path.exists() {
        return Err(invalid("owned UPS files not cleaned"));
    }
    Ok(report)
}
