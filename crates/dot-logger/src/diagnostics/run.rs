//! Explicit synchronous synthetic-file check; never a device capacity certificate.
use super::file::{BoundedSink, OwnedFile};
use super::{CheckPlan, NetworkPlan, Progress, StopReason};
use crate::{
    capture::{CaptureReceipt, CapturedRecord, Importance, Level, RecordMetadata},
    completion::Boundary,
    storage::{SegmentWriter, StorageChannel, StorageError, WriterBudget, WriterConfig},
};
use std::{
    cell::RefCell,
    io,
    path::Path,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Explicit finite synthetic workload. Repeated seeded bytes do not reproduce
/// sensor content/compressibility, capture/routing work, or remote traffic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileWorkload {
    /// Number of requested records, strictly positive.
    pub records: u32,
    /// Bytes per synthetic payload, strictly positive.
    pub payload_bytes: usize,
    /// Spacing in the session clock; zero explicitly selects maximum file load.
    pub interval_ns: u64,
    /// Deterministic repeated payload byte; never read from operating records.
    pub seed: u8,
}
/// One uncompressed file, one channel and one calling worker. The supplied
/// writer authority must belong to a dedicated check session/target, not a live
/// application's output. No production capture receipt/completion is created.
#[derive(Debug)]
pub struct FileCheck {
    /// Finite synthetic input profile.
    pub workload: FileWorkload,
    /// Explicit channel/schema declaration for the temporary product MCAP file.
    pub channel: StorageChannel,
}
/// Host observations and interruptible pacing. Implementations use the selected
/// session clock, fresh free-space probes and real cancellation. The trusted
/// parent namespace must remain exclusively controlled throughout the call.
pub trait CheckRuntime {
    /// Current session-monotonic ns; reversals are rejected.
    fn now_ns(&mut self) -> io::Result<u64>;
    /// Explicit cancellation; never means OS IO has already ended.
    fn cancelled(&self) -> bool;
    /// Actual filesystem free bytes for an existing check directory or parent.
    fn available_bytes(&mut self, path: &Path) -> io::Result<u64>;
    /// Wait until the supplied session-clock instant or cancellation. Returning
    /// early without cancellation is rejected, rather than busy-retried.
    fn wait_until(&mut self, at_ns: u64) -> io::Result<()>;
}
/// Real monotonic clock and cancellation-aware pacing, with a host-supplied
/// filesystem probe. No worker is spawned and no shell command is run here.
pub struct FileCheckRuntime<'a, F> {
    start: Instant,
    cancel: &'a AtomicBool,
    free: F,
}
impl<'a, F: FnMut(&Path) -> io::Result<u64>> FileCheckRuntime<'a, F> {
    /// The origin must correspond to the plan's selected session clock. The
    /// probe owns any OS-specific free-space capability/error handling.
    pub fn new(session_start: Instant, cancel: &'a AtomicBool, free: F) -> Self {
        Self {
            start: session_start,
            cancel,
            free,
        }
    }
}
impl<F: FnMut(&Path) -> io::Result<u64>> CheckRuntime for FileCheckRuntime<'_, F> {
    fn now_ns(&mut self) -> io::Result<u64> {
        u64::try_from(self.start.elapsed().as_nanos())
            .map_err(|_| io::ErrorKind::InvalidData.into())
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
    fn available_bytes(&mut self, path: &Path) -> io::Result<u64> {
        (self.free)(path)
    }
    fn wait_until(&mut self, at_ns: u64) -> io::Result<()> {
        while !self.cancelled() {
            let now = self.now_ns()?;
            if now >= at_ns {
                break;
            }
            std::thread::sleep(Duration::from_nanos((at_ns - now).min(10_000_000)));
        }
        Ok(())
    }
}
/// Functional result for the declared file or routed snapshot scope only. Pass does not
/// certify maximum capacity, CPU/RT overhead, network, UPS or 24-hour behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckOutcome {
    /// Every requested record reached its declared boundary and resources settled.
    Pass,
    /// Operational, clock, finalization/sync or cleanup failure; see issues.
    Fail,
    /// Explicit cancellation with successful IO settlement/cleanup.
    Aborted,
    /// Budget/space or unsupported scope prevented the requested test.
    Inconclusive,
}
/// Original reason load stopped, independent of later settlement errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckStop {
    /// Requested finite workload reached its declared completion boundary.
    Completed,
    /// An admission/resource/cancellation limit stopped new work.
    Limit(StopReason),
    /// File-only runner cannot verify the requested completion boundary.
    UnsupportedBoundary,
    /// Network work requires a separate receiver/backend and remains unmeasured.
    UnsupportedNetwork,
    /// An operational or invalid-clock error stopped the run.
    Error,
}
/// Failure stage, retained separately from load-stop and final outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckStage {
    /// Production capture, routing or receiver transport failed.
    Transport,
    /// Clock, pacing or free-space observation failed.
    Observation,
    /// Creation or writer initialization failed.
    Create,
    /// Append/complete-chunk flush failed.
    Write,
    /// MCAP footer/summary failed.
    Finish,
    /// Actual file/directory sync failed.
    Sync,
    /// Owned temporary resources could not be removed safely.
    Cleanup,
}
/// Original OS cause (or an error with the storage cause as its source).
#[derive(Debug)]
pub struct CheckIssue {
    /// Operation that failed.
    pub stage: CheckStage,
    /// Original error; OS error codes remain available.
    pub error: io::Error,
}
/// Pre-IO validation failure; no check namespace has been created.
#[derive(Debug)]
pub enum CheckStartError {
    /// Plan/session/revision does not match the supplied writer authority.
    Identity,
    /// Only one synchronous calling worker is currently implemented.
    UnsupportedWorkers,
    /// This filesystem adapter requires Unix directory sync/identity support.
    UnsupportedPlatform,
    /// Empty/overflowing workload, invalid registration or writer limits.
    InvalidWorkload,
    /// Conservative reservation exceeds the plan's declared memory requirement.
    MemoryBudget {
        /// Computed requested-byte requirement, excluding host runtime/OS cache.
        required: usize,
        /// Host declaration, already bounded by the plan's memory allowance.
        declared: usize,
    },
}
impl std::fmt::Display for CheckStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "file check: {self:?}")
    }
}
impl std::error::Error for CheckStartError {}
/// Final evidence, returned after actual writer drop, file sync/close and owned
/// namespace cleanup attempts. Partial work never becomes a capacity profile.
#[derive(Debug)]
pub struct CheckReport {
    /// Original plan/provenance and budgets, moved without cloning its path.
    pub plan: CheckPlan,
    /// Exact requested synthetic input, including contents and timing definition.
    pub workload: FileWorkload,
    /// Target identity of the dedicated check writer authority.
    pub target_id: u32,
    /// File channel/schema identity; no production receipt was admitted.
    pub channel: crate::capture::ChannelConfig,
    /// Functional scope outcome after settlement.
    pub outcome: CheckOutcome,
    /// Why load ended, even if later cleanup failed.
    pub stop: CheckStop,
    /// Original session-monotonic start, or None when unsupported before IO.
    pub from_ns: Option<u64>,
    /// Last successfully observed session time, including settlement/cleanup.
    pub to_ns: Option<u64>,
    /// Records whose whole chunk reached the write boundary.
    pub written_records: u32,
    /// Records in a complete file covered by successful actual file+directory sync.
    pub synced_records: u32,
    /// Bytes accepted by Write, including repeated seek-overwrites and metadata;
    /// not SSD/device write amplification or final file length.
    pub physical_write_bytes: u64,
    /// Actual temporary file length before deletion, if measured.
    pub file_bytes: Option<u64>,
    /// Summary/footer completed successfully.
    pub file_complete: bool,
    /// Owned paths removed and parent directory sync succeeded, or nothing was
    /// created. False can also mean paths were removed but the final sync failed.
    pub cleaned: bool,
    /// First load/startup/observation error; original cause is preserved.
    pub primary_issue: Option<CheckIssue>,
    /// First footer/file/directory sync failure, independent of the load error.
    pub settlement_issue: Option<CheckIssue>,
    /// Cleanup failure, never hidden by an earlier operational failure.
    pub cleanup_issue: Option<CheckIssue>,
}
pub(super) struct Shared<'a, R> {
    pub runtime: &'a mut R,
    pub bytes: u64,
    pub reserved: u64,
    pub limit: u64,
    pub reserve: u64,
    pub last_ns: u64,
    pub blocked: Option<StopReason>,
}
impl<R: CheckRuntime> Shared<'_, R> {
    pub fn now(&mut self) -> io::Result<u64> {
        let now = self.runtime.now_ns()?;
        if now < self.last_ns {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "check clock reversed",
            ));
        }
        self.last_ns = now;
        Ok(now)
    }
}
enum Admission {
    Record(u64),
    Stop(StopReason),
}
fn next_record<R: CheckRuntime>(
    state: &Rc<RefCell<Shared<'_, R>>>,
    plan: &CheckPlan,
    directory: &Path,
    from: u64,
    desired: u64,
    per_record: u64,
) -> io::Result<Admission> {
    let mut s = state.borrow_mut();
    let now = s.now()?;
    if s.runtime.cancelled() {
        return Ok(Admission::Stop(StopReason::Cancelled));
    }
    if now - from >= plan.spec().budgets.duration_ns {
        return Ok(Admission::Stop(StopReason::TimeBudget));
    }
    if now < desired {
        s.runtime.wait_until(desired)?;
    }
    let now = s.now()?;
    if s.runtime.cancelled() {
        return Ok(Admission::Stop(StopReason::Cancelled));
    }
    if now - from >= plan.spec().budgets.duration_ns {
        return Ok(Admission::Stop(StopReason::TimeBudget));
    }
    if now < desired {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "check wait returned early",
        ));
    }
    let free = s.runtime.available_bytes(directory)?;
    // Probes are real, potentially slow IO. Admission must use the time and
    // cancellation observed after the probe, not the earlier pacing instant.
    let now = s.now()?;
    let progress = Progress {
        elapsed_ns: now - from,
        written_bytes: s.reserved,
        free_disk_bytes: free.saturating_sub(s.reserved.saturating_sub(s.bytes)),
        cancelled: s.runtime.cancelled(),
    };
    match plan.write_allowance(progress) {
        Err(reason) => Ok(Admission::Stop(reason)),
        Ok(n) if n < per_record => Ok(Admission::Stop(
            if plan.spec().budgets.write_bytes.saturating_sub(s.reserved) < per_record {
                StopReason::WriteBudget
            } else {
                StopReason::DiskReserve
            },
        )),
        Ok(_) => {
            s.reserved += per_record;
            Ok(Admission::Record(now))
        }
    }
}
fn io_error(e: StorageError) -> io::Error {
    match e {
        StorageError::Io(e) => e,
        other => io::Error::other(other),
    }
}
fn issue(stage: CheckStage, error: io::Error) -> CheckIssue {
    CheckIssue { stage, error }
}
impl CheckPlan {
    /// Explicitly run a bounded synthetic product-MCAP file check. Requires a
    /// dedicated writer authority and an existing trusted parent. Existing paths
    /// are refused; only the exact newly created file/directory may be removed.
    /// Cancellation/deadlines stop admission, then finish/sync/close/cleanup are
    /// attempted; ongoing OS IO has no forced time bound. No workers are spawned.
    pub fn run_file<R: CheckRuntime>(
        self,
        budget: WriterBudget,
        check: FileCheck,
        runtime: &mut R,
    ) -> Result<CheckReport, CheckStartError> {
        let spec = self.spec();
        let w = check.workload;
        if budget.session != spec.session_id || budget.revision != spec.revision {
            return Err(CheckStartError::Identity);
        }
        if spec.budgets.workers != 1 {
            return Err(CheckStartError::UnsupportedWorkers);
        }
        if !cfg!(unix) {
            return Err(CheckStartError::UnsupportedPlatform);
        }
        if w.records == 0
            || w.payload_bytes == 0
            || u64::from(w.records - 1)
                .checked_mul(w.interval_ns)
                .is_none()
        {
            return Err(CheckStartError::InvalidWorkload);
        }
        let metadata = [
            check.channel.topic.len(),
            check.channel.payload_encoding.len(),
            check
                .channel
                .schema
                .as_ref()
                .map_or(0, |s| s.encoding.len() + s.descriptor.len()),
        ]
        .into_iter()
        .try_fold(0usize, |a, b| a.checked_add(b))
        .ok_or(CheckStartError::InvalidWorkload)?;
        let config = WriterConfig {
            max_loss_reports: 0,
            max_payload_bytes: w.payload_bytes,
            write_buffer_bytes: 0,
            chunk_target_bytes: w.payload_bytes,
            max_records: w.records as usize,
            max_chunks: (w.records as usize)
                .checked_add(1)
                .ok_or(CheckStartError::InvalidWorkload)?,
            max_metadata_bytes: metadata.max(1),
        };
        let reservation = config
            .reservation(std::slice::from_ref(&check.channel), budget.producers)
            .map_err(|_| CheckStartError::InvalidWorkload)?;
        // Writer reservation includes maps/encoder/index/metadata. Additional
        // allowance covers payload, paths, shared state and fixed run bookkeeping.
        let required = reservation
            .checked_add(w.payload_bytes)
            .and_then(|n| n.checked_add(65536))
            .and_then(|n| n.checked_add(spec.directory.capacity().checked_mul(8)?))
            .ok_or(CheckStartError::InvalidWorkload)?;
        if required > spec.required_memory_bytes {
            return Err(CheckStartError::MemoryBudget {
                required,
                declared: spec.required_memory_bytes,
            });
        }
        // Conservative pinned uncompressed one-channel layout, with room for all
        // footer/schema metadata and per-record chunks/indexes/seek rewrites.
        // The sink also enforces actual total bytes and fresh free-space limits.
        let base = 65536u64
            .checked_add(
                (metadata as u64)
                    .checked_mul(4)
                    .ok_or(CheckStartError::InvalidWorkload)?,
            )
            .ok_or(CheckStartError::InvalidWorkload)?;
        let per_record = (w.payload_bytes as u64)
            .checked_add(4096)
            .ok_or(CheckStartError::InvalidWorkload)?;
        let mut report = CheckReport {
            target_id: budget.target,
            channel: check.channel.channel,
            plan: self,
            workload: w,
            outcome: CheckOutcome::Inconclusive,
            stop: CheckStop::Error,
            from_ns: None,
            to_ns: None,
            written_records: 0,
            synced_records: 0,
            physical_write_bytes: 0,
            file_bytes: None,
            file_complete: false,
            cleaned: true,
            primary_issue: None,
            settlement_issue: None,
            cleanup_issue: None,
        };
        if !matches!(
            report.plan.spec().boundary,
            Boundary::Written | Boundary::Synced
        ) {
            report.stop = CheckStop::UnsupportedBoundary;
            return Ok(report);
        }
        if report.plan.spec().network != NetworkPlan::UnavailableNoReceiver {
            report.stop = CheckStop::UnsupportedNetwork;
            return Ok(report);
        }
        let from = match runtime.now_ns() {
            Ok(n) => n,
            Err(e) => {
                report.primary_issue = Some(issue(CheckStage::Observation, e));
                report.outcome = CheckOutcome::Fail;
                return Ok(report);
            }
        };
        report.from_ns = Some(from);
        report.to_ns = Some(from);
        let spec = report.plan.spec();
        let state = Rc::new(RefCell::new(Shared {
            runtime,
            bytes: 0,
            reserved: base,
            limit: spec.budgets.write_bytes,
            reserve: spec.budgets.disk_reserve_bytes,
            last_ns: from,
            blocked: None,
        }));
        let mut owned = OwnedFile::new(&spec.directory);
        let operation = (|| -> Result<(), CheckIssue> {
            let mut s = state.borrow_mut();
            if s.runtime.cancelled() {
                report.stop = CheckStop::Limit(StopReason::Cancelled);
                return Ok(());
            }
            let free = s
                .runtime
                .available_bytes(owned.parent())
                .map_err(|e| issue(CheckStage::Observation, e))?;
            let now = s.now().map_err(|e| issue(CheckStage::Observation, e))?;
            let p = Progress {
                elapsed_ns: now - from,
                written_bytes: base,
                free_disk_bytes: free.saturating_sub(base),
                cancelled: s.runtime.cancelled(),
            };
            let allowance = report.plan.write_allowance(p);
            if let Err(reason) = allowance {
                report.stop = CheckStop::Limit(reason);
                return Ok(());
            }
            if allowance.is_ok_and(|n| n < per_record) {
                report.stop = CheckStop::Limit(
                    if spec.budgets.write_bytes.saturating_sub(base) < per_record {
                        StopReason::WriteBudget
                    } else {
                        StopReason::DiskReserve
                    },
                );
                return Ok(());
            }
            if from.checked_add(spec.budgets.duration_ns).is_none() {
                return Err(issue(
                    CheckStage::Observation,
                    io::ErrorKind::InvalidData.into(),
                ));
            }
            drop(s);
            owned.create().map_err(|e| issue(CheckStage::Create, e))?;
            let sink = BoundedSink::new(
                owned
                    .clone_file()
                    .map_err(|e| issue(CheckStage::Create, e))?,
                Rc::clone(&state),
                owned.directory(),
            );
            let mut writer = SegmentWriter::new(budget, config, vec![check.channel], sink)
                .map_err(|e| issue(CheckStage::Create, io_error(e)))?;
            let payload = vec![w.seed; w.payload_bytes];
            let mut writer_ok = true;
            report.stop = CheckStop::Completed;
            for i in 0..w.records {
                let desired = from
                    + u64::from(i)
                        .saturating_mul(w.interval_ns)
                        .min(spec.budgets.duration_ns);
                let now = match next_record(
                    &state,
                    &report.plan,
                    owned.directory(),
                    from,
                    desired,
                    per_record,
                ) {
                    Ok(Admission::Record(now)) => now,
                    Ok(Admission::Stop(reason)) => {
                        report.stop = CheckStop::Limit(reason);
                        break;
                    }
                    Err(error) => {
                        // Observation/pacing failed without poisoning the writer.
                        // Preserve that error, but still finish the accepted prefix.
                        report.primary_issue = Some(issue(CheckStage::Observation, error));
                        report.stop = CheckStop::Error;
                        break;
                    }
                };
                let record = CapturedRecord {
                    receipt: CaptureReceipt {
                        session_id: spec.session_id,
                        producer_id: 0,
                        accepted_seq: u64::from(i) + 1,
                    },
                    revision: spec.revision,
                    payload_len: w.payload_bytes,
                    metadata: RecordMetadata {
                        channel_id: report.channel.channel_id,
                        schema_id: report.channel.schema_id,
                        schema_version: report.channel.schema_version,
                        mono_ns: now,
                        real_ns: 0,
                        source_time: None,
                        level: Level::Info,
                        importance: Some(Importance::Normal),
                    },
                };
                if let Err(e) = writer
                    .append(record, &payload)
                    .and_then(|()| writer.flush_chunk().map(|_| ()))
                {
                    report.primary_issue = Some(issue(CheckStage::Write, io_error(e)));
                    report.stop = state
                        .borrow()
                        .blocked
                        .map_or(CheckStop::Error, CheckStop::Limit);
                    writer_ok = false;
                    break;
                }
                report.written_records += 1;
            }
            if writer_ok {
                match writer.finish() {
                    Ok(finished) => {
                        report.file_complete = true;
                        drop(finished.into_inner());
                    }
                    Err(e) => {
                        report.settlement_issue = Some(issue(CheckStage::Finish, io_error(e)));
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = operation {
            report.stop = CheckStop::Error;
            report.primary_issue = Some(e);
        }
        if let Err(e) = owned.settle(&mut report.file_bytes) {
            if report.settlement_issue.is_none() {
                report.settlement_issue = Some(issue(CheckStage::Sync, e));
            }
        }
        if report.file_complete && report.settlement_issue.is_none() {
            report.synced_records = report.written_records;
        }
        // No writer/clone remains. Close the original before removing any path.
        owned.close();
        report.cleaned = match owned.cleanup() {
            Ok(()) => true,
            Err(e) => {
                report.cleanup_issue = Some(issue(CheckStage::Cleanup, e));
                false
            }
        };
        let mut s = state.borrow_mut();
        report.physical_write_bytes = s.bytes;
        match s.now() {
            Ok(n) => report.to_ns = Some(n),
            Err(e) => {
                if report.primary_issue.is_none() {
                    report.primary_issue = Some(issue(CheckStage::Observation, e));
                }
            }
        }
        if s.runtime.cancelled() && report.primary_issue.is_none() {
            report.stop = CheckStop::Limit(StopReason::Cancelled);
        }
        report.outcome = if report.primary_issue.is_some()
            || report.settlement_issue.is_some()
            || report.cleanup_issue.is_some()
        {
            CheckOutcome::Fail
        } else {
            match report.stop {
                CheckStop::Completed
                    if report.file_complete && report.synced_records == w.records =>
                {
                    CheckOutcome::Pass
                }
                CheckStop::Limit(StopReason::Cancelled) => CheckOutcome::Aborted,
                _ => CheckOutcome::Inconclusive,
            }
        };
        Ok(report)
    }
}
