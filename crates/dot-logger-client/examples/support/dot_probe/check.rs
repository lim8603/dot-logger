//! dot-probe snapshot check for a trusted `CheckHost` (moved out of the
//! library into example support, TASK-033 S3 Human decision 2026-11-15).
use super::implementation as bridge;
use dot_logger::diagnostics::*;
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

/// One prevalidated dot-probe export, retained by a single-use host capability.
/// Register with [`Self::TARGET_ID`] and declare at least
/// [`Self::REQUIRED_MEMORY_BYTES`] (24 MiB) in the host's `CheckSpec` and memory
/// budget: 16 MiB for the core logger reservation plus 8 MiB of conservative
/// input/JSON/receiver/runtime bookkeeping. This planning allowance is neither
/// measured process RSS nor a hard OS memory limit.
/// This finite check never writes files, retries, starts a daemon or establishes
/// snapshot freshness, hardware capacity, RT performance or app UI integration.
pub struct SnapshotCheck {
    payload: Vec<u8>,
    receiver: bool,
}
impl SnapshotCheck {
    /// Fixed production route required by `CheckHost::register_routed`.
    pub const TARGET_ID: u32 = bridge::TARGET;
    /// Minimum declared plan memory: 16 MiB logger plus 8 MiB bookkeeping.
    pub const REQUIRED_MEMORY_BYTES: usize = 24 * 1024 * 1024;

    /// Validate a bounded export before host registration. Input is host-owned;
    /// remote callers cannot choose a source path, receiver or payload.
    pub fn new(payload: Vec<u8>, receiver: bool) -> Result<Self, bridge::Error> {
        bridge::validate_snapshot(&payload)?;
        Ok(Self {
            payload: payload.into_boxed_slice().into_vec(),
            receiver,
        })
    }
}
impl RoutedCheckBackend for SnapshotCheck {
    fn target_id(&self) -> u32 {
        Self::TARGET_ID
    }
    fn required_memory_bytes(&self) -> usize {
        Self::REQUIRED_MEMORY_BYTES
    }
    fn run(
        self: Box<Self>,
        plan: &CheckPlan,
        runtime: &mut dyn CheckRuntime,
    ) -> Result<RoutedCheckReport, CheckStartError> {
        let spec = plan.spec();
        if spec.revision == 0 || spec.budgets.duration_ns > 2_000_000_000 {
            return Err(CheckStartError::InvalidWorkload);
        }
        let runtime = RefCell::new(runtime);
        let start = runtime.borrow_mut().now_ns();
        let from = match start {
            Ok(from) => from,
            Err(error) => {
                return Ok(RoutedCheckReport {
                    outcome: CheckOutcome::Fail,
                    stop: CheckStop::Error,
                    issue: Some(CheckIssue {
                        stage: CheckStage::Observation,
                        error,
                    }),
                    receipt: None,
                    completion: None,
                    from_ns: None,
                    to_ns: None,
                    accepted: 0,
                    acknowledged: 0,
                    raw_reclaimed: 0,
                    external_bytes_after_close: 0,
                });
            }
        };
        let last = Cell::new(from);
        let observation = || -> Result<u64, bridge::Error> {
            let now = runtime.borrow_mut().now_ns()?;
            if now < last.get() {
                return Err(std::io::Error::from(std::io::ErrorKind::InvalidData).into());
            }
            last.set(now);
            Ok(now)
        };
        let (queue, consumed) = std::sync::mpsc::sync_channel(1);
        let result = bridge::execute(
            &self.payload,
            bridge::Options {
                receiver: self.receiver,
                duration: Duration::from_nanos(spec.budgets.duration_ns),
            },
            &|| runtime.borrow().cancelled(),
            |_, bytes| {
                // A single bounded queue owns an exact original copy before ACK.
                // The consumer is this check; no claim about a rendered app screen.
                if queue.try_send(bytes.to_vec()).is_ok() {
                    bridge::Action::Accept
                } else {
                    bridge::Action::Reject
                }
            },
            Some(bridge::Identity {
                session: spec.session_id,
                revision: spec.revision,
                observe: &observation,
                admit: &|| Ok(observation()? - from < spec.budgets.duration_ns),
            }),
        );
        let (to, clock_issue) = match observation() {
            Ok(to) => (to, None),
            Err(error) => (
                last.get(),
                Some(CheckIssue {
                    stage: CheckStage::Observation,
                    error: original_io_error(error),
                }),
            ),
        };
        match result {
            Ok(r) => {
                let identical = consumed.try_recv().is_ok_and(|bytes| bytes == self.payload);
                let issue = r
                    .issue
                    .map(|error| CheckIssue {
                        stage: CheckStage::Transport,
                        error: original_io_error(error),
                    })
                    .or(clock_issue);
                Ok(RoutedCheckReport {
                    outcome: if issue.is_some() {
                        CheckOutcome::Fail
                    } else {
                        match r.outcome {
                            bridge::BridgeOutcome::Pass if identical => CheckOutcome::Pass,
                            bridge::BridgeOutcome::Pass | bridge::BridgeOutcome::Fail => {
                                CheckOutcome::Fail
                            }
                            bridge::BridgeOutcome::Aborted => CheckOutcome::Aborted,
                            bridge::BridgeOutcome::Inconclusive => CheckOutcome::Inconclusive,
                        }
                    },
                    stop: if issue.is_some() {
                        CheckStop::Error
                    } else if runtime.borrow().cancelled() {
                        CheckStop::Limit(StopReason::Cancelled)
                    } else if r.budget_expired {
                        CheckStop::Limit(StopReason::TimeBudget)
                    } else if r.acknowledged == 1 {
                        CheckStop::Completed
                    } else {
                        CheckStop::UnsupportedNetwork
                    },
                    receipt: r.receipt,
                    completion: r.completion,
                    from_ns: Some(from),
                    to_ns: Some(to),
                    accepted: r.accepted,
                    acknowledged: r.acknowledged,
                    raw_reclaimed: r.raw_reclaimed,
                    external_bytes_after_close: r.external_bytes_after_close,
                    issue,
                })
            }
            // Operational failure stays an explicit failed scope. It cannot be
            // promoted to capacity or successful receiver completion.
            Err(error) => Ok(RoutedCheckReport {
                outcome: CheckOutcome::Fail,
                stop: CheckStop::Error,
                receipt: None,
                completion: None,
                from_ns: Some(from),
                to_ns: Some(to),
                accepted: 0,
                acknowledged: 0,
                raw_reclaimed: 0,
                external_bytes_after_close: 0,
                issue: Some(CheckIssue {
                    stage: CheckStage::Transport,
                    error: original_io_error(error),
                }),
            }),
        }
    }
}

fn original_io_error(error: bridge::Error) -> std::io::Error {
    match error.downcast::<std::io::Error>() {
        Ok(error) => *error,
        Err(error) => std::io::Error::other(error),
    }
}
