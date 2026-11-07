//! General-path worker status (API §18.5). Values the worker has not
//! observed stay `None`/`Unavailable`; nothing unmeasured is reported as zero.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::Duration;

use crate::telemetry::{Observation, Sample, Unavailable};

/// Overall state, most severe first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthState {
    /// Every output is writing and synchronizing on schedule.
    Running,
    /// Recording continues but some records were lost or durability is late.
    Degraded(Degradation),
    /// Required recording cannot succeed any more; shut down and inspect.
    Failed(Failure),
}

/// Why a running logger is degraded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Degradation {
    /// An output rejected records without an IO fault (for example a full
    /// continuous retention). `OutputHealth::failed_records` counts them.
    RecordsFailed,
    /// An output has not completed a periodic sync for longer than
    /// `4 × sync_every + 1 s`. Records since its last sync are not durable yet.
    SyncOverdue,
}

/// Why required recording stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    /// A worker thread panicked or stopped; admission is closed (`Closing`).
    WorkerFailed,
    /// An output's write, sync or publication failed and that output stopped.
    /// Every later record fails on it because every output is required.
    IoPoisoned,
}

/// One file output, in `file_output` declaration order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputHealth {
    /// Time since this output's last successful periodic or final sync;
    /// `None` until the first one succeeds.
    pub since_sync: Option<Duration>,
    /// Records that reached this output but were reported Failed.
    pub failed_records: u64,
    /// The output stopped after an IO fault.
    pub failed: bool,
}

/// Point-in-time status from [`Logger::health`](super::Logger::health).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Health {
    /// Overall state.
    pub state: HealthState,
    /// Per-output detail.
    pub outputs: Vec<OutputHealth>,
    /// Worker CPU placement: `None` when no CPU list was requested,
    /// `Some(false)` when an `Optional` placement could not be applied.
    pub affinity: Option<bool>,
    /// Latest one-second capture sample; `None` unless enabled with
    /// `LoggerBuilder::telemetry`. Older than two seconds is `Stale`.
    pub telemetry: Option<Observation<Sample>>,
}

/// Affinity result codes in [`Shared::affinity`].
pub(crate) const AFFINITY_NONE: u8 = 0;
pub(crate) const AFFINITY_APPLIED: u8 = 1;
pub(crate) const AFFINITY_FAILED: u8 = 2;

const NEVER: u64 = u64::MAX;

/// Status one IO thread publishes for one output.
pub(crate) struct OutputStatus {
    synced_ns: AtomicU64,
    failed_records: AtomicU64,
    failed: AtomicBool,
}

impl OutputStatus {
    fn new() -> Self {
        Self {
            synced_ns: AtomicU64::new(NEVER),
            failed_records: AtomicU64::new(0),
            failed: AtomicBool::new(false),
        }
    }
    pub(crate) fn synced(&self, at_ns: u64) {
        self.synced_ns.store(at_ns, Ordering::Release);
    }
    pub(crate) fn records_failed(&self, count: usize) {
        if count > 0 {
            self.failed_records
                .fetch_add(count as u64, Ordering::AcqRel);
        }
    }
    pub(crate) fn fail(&self) {
        self.failed.store(true, Ordering::Release);
    }
}

/// State shared by the host, IO threads and the `Logger` handle.
pub(crate) struct Shared {
    pub(crate) worker_failed: AtomicBool,
    pub(crate) io_failed: AtomicBool,
    pub(crate) stop: AtomicBool,
    pub(crate) finish: AtomicBool,
    pub(crate) outputs: Box<[OutputStatus]>,
    pub(crate) affinity: AtomicU8,
    /// Latest telemetry sample; present only when telemetry is enabled.
    pub(crate) telemetry: Option<Mutex<Option<Sample>>>,
    /// Test-only fault injection: 1 panics the host, 2 panics an IO thread.
    #[cfg(test)]
    pub(crate) inject: AtomicU8,
}

impl Shared {
    pub(crate) fn new(outputs: usize, telemetry: bool) -> Self {
        Self {
            worker_failed: AtomicBool::new(false),
            io_failed: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            finish: AtomicBool::new(false),
            outputs: (0..outputs).map(|_| OutputStatus::new()).collect(),
            affinity: AtomicU8::new(AFFINITY_NONE),
            telemetry: telemetry.then(|| Mutex::new(None)),
            #[cfg(test)]
            inject: AtomicU8::new(0),
        }
    }

    /// Evaluate at session-monotonic `now_ns`; `overdue` is the sync lateness bound.
    pub(crate) fn health(&self, now_ns: u64, overdue: Duration) -> Health {
        let overdue = u64::try_from(overdue.as_nanos()).unwrap_or(u64::MAX);
        let mut late = false;
        let outputs: Vec<OutputHealth> = self
            .outputs
            .iter()
            .map(|o| {
                let synced = o.synced_ns.load(Ordering::Acquire);
                // Before the first sync, lateness counts from session start.
                let since = now_ns.saturating_sub(if synced == NEVER { 0 } else { synced });
                late |= since > overdue;
                OutputHealth {
                    since_sync: (synced != NEVER).then(|| Duration::from_nanos(since)),
                    failed_records: o.failed_records.load(Ordering::Acquire),
                    failed: o.failed.load(Ordering::Acquire),
                }
            })
            .collect();
        let state = if self.worker_failed.load(Ordering::Acquire) {
            HealthState::Failed(Failure::WorkerFailed)
        } else if outputs.iter().any(|o| o.failed) {
            HealthState::Failed(Failure::IoPoisoned)
        } else if outputs.iter().any(|o| o.failed_records > 0) {
            HealthState::Degraded(Degradation::RecordsFailed)
        } else if late {
            HealthState::Degraded(Degradation::SyncOverdue)
        } else {
            HealthState::Running
        };
        let telemetry = self.telemetry.as_ref().map(|latest| {
            let Ok(latest) = latest.lock() else {
                return Observation::Unavailable(Unavailable::HistoryUnavailable);
            };
            match *latest {
                Some(s) if now_ns.saturating_sub(s.at_ns) > 2_000_000_000 => Observation::Stale(s),
                Some(s) => Observation::Observed(s),
                None => Observation::Unavailable(Unavailable::IncompleteWindow),
            }
        });
        Health {
            state,
            outputs,
            affinity: match self.affinity.load(Ordering::Acquire) {
                AFFINITY_APPLIED => Some(true),
                AFFINITY_FAILED => Some(false),
                _ => None,
            },
            telemetry,
        }
    }
}
