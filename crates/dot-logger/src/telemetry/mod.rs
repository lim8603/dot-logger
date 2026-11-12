//! Explicitly enabled, fixed producer counters and caller-driven one-second history.
//!
//! Capture only updates producer-local atomics. Sampling/history locks are general
//! path operations and never run inside capture. Missing data is not reported as
//! zero. Management report arithmetic uses explicitly supplied evidence; runtime
//! workers and device validation remain the host's responsibility.
//!
//! # Use
//!
//! `CaptureOwner::enable_telemetry` enables the fixed counters and a 301-entry
//! history; a management thread drives `Telemetry::sample`. Pass the
//! `Observed(Window)` of `Telemetry::window(10 | 60 | 300)` to
//! `report::ReportInput::from_window` and compute with
//! `Report::build(input, profile, now_ns)` (see `tc_009_telemetry.rs`). Input
//! counts cover every producer and channel; completion counts are assignments of
//! the selected output. The host supplies `OutputDelta` and the physical
//! `BufferTrend` and `DiskTrend` for the same session/revision/epoch and window.
//! Uncollected values are `Unavailable`; payload bytes never stand in for occupied
//! memory.
//!
//! # Rejections and load
//!
//! The totals `rejected`/`acquire_rejected` split into `_capacity`, `_contract` and
//! `_lifecycle`; only capacity rejections are overload evidence. The load ratio
//! compares the submission *attempt* rate `(accepted + rejected_capacity)` with
//! verified completion capacity. A retried raw loan or general producer counts
//! every attempt, so it is neither a unique-input nor a loss count. Acquire
//! failures, skips and a general producer's closing/deadline pre-rejection are
//! excluded.
//!
//! # Aligning per-target output
//!
//! After each successful `sample`, keep its session/revision/epoch/time with
//! `Completion::target_totals(id)` of the chosen target in a bounded host history
//! and use the two samples exactly at `Window.from_ns` and `to_ns`: a 10 s window
//! may start up to 2 s early, so do not derive its start from `to_ns`. Mismatched
//! target, boundary, session or epoch, a missing endpoint, overflow, Busy or a
//! decreasing total make the delta `Unavailable`. complete/failed/dropped/unknown
//! are checked differences; `pending` is the last sample's value. Concurrent
//! capture and completion reads can mix nearby instants; this is not a global
//! atomic snapshot (example:
//! `host_aligns_target_completion_history_to_actual_window_endpoints`).
//!
//! `Telemetry::history_len()` returns `Observation<usize>`: a history error is
//! distinct from length 0, and the exclusive owner's `reset_telemetry` clears the
//! history and its poison. Build `CaptureCounts` with `..Default::default()` to
//! stay compatible with new category fields.
mod aggregate;
pub mod capacity;
/// Before/after condition checks and evidence-preserving rate comparison.
pub mod compare;
mod counters;
/// Host-driven remote status, explicit apply and compare backend.
#[cfg(feature = "remote")]
pub mod operations;
pub mod report;

use crate::completion::Completion;
pub use aggregate::{Sample, TelemetryError, Window};
pub use counters::CaptureCounts;
pub(crate) use counters::{Bank, CounterCell};

/// Why an observation cannot be supplied; never a numeric zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unavailable {
    /// Producer adoption, old completion or retained payloads still span revisions.
    RevisionTransition,
    /// The host has not supplied the required resource/output observation.
    MissingMeasurement,
    /// No completion registry was supplied.
    MissingCompletion,
    /// ID was not registered before capture.
    Unregistered,
    /// The required time interval was not retained or has missing samples.
    IncompleteWindow,
    /// Count representation saturated; deltas would be misleading.
    Saturated,
    /// Another session/revision supplied the completion registry.
    ForeignSession,
    /// A management-path history lock was poisoned.
    HistoryUnavailable,
}
/// Evidence state of a value. Device validation/estimates are not fabricated here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Observation<T> {
    /// Atomically read counters; concurrent producers need not share one instant.
    Observed(T),
    /// Last retained sample is older than two seconds.
    Stale(T),
    /// No usable value exists.
    Unavailable(Unavailable),
}

/// Borrowed view; cannot outlive the CaptureOwner's preallocated counter bank.
/// Use on management threads. Copying the handle does not allocate or start work.
#[derive(Clone, Copy)]
pub struct Telemetry<'owner> {
    pub(crate) bank: &'owner Bank,
    pub(crate) session: u128,
    pub(crate) revision: u64,
    pub(crate) updates: Option<crate::control::config_update::ConfigUpdates<'owner>>,
}
impl Telemetry<'_> {
    /// Counter generation, advanced only by explicit management reset/enable.
    pub fn epoch(self) -> u64 {
        self.bank.epoch
    }
    /// Fixed registered producer cardinality.
    pub fn producers(self) -> usize {
        self.bank.cells.len()
    }
    /// Producer-local atomic counters. Concurrent values may span nearby events;
    /// quiescent samples are exact. No retry or producer synchronization is needed.
    pub fn producer(self, id: u32) -> Observation<CaptureCounts> {
        self.bank.cells.get(id as usize).map_or(
            Observation::Unavailable(Unavailable::Unregistered),
            |cell| Observation::Observed(cell.read()),
        )
    }
    /// Sample cumulative counts at most once per second. No clock reads or worker
    /// startup: the host supplies session-monotonic nanoseconds and completion.
    pub fn sample(
        self,
        at_ns: u64,
        completion: Option<Completion<'_>>,
    ) -> Result<Sample, TelemetryError> {
        // Hold publication serialization across the sample: an apply cannot
        // race a stable generation label. RT producers never acquire this lock.
        let observation = self.updates.map(|u| u.state.observation(u.registry));
        let (revision, transition) = match &observation {
            Some(Ok((m, ready))) => (m.revision, !ready),
            Some(Err(_)) => (self.revision, true),
            None => (self.revision, false),
        };
        let mut capture = CaptureCounts::default();
        for cell in &self.bank.cells {
            capture.add(cell.read());
        }
        let completion = completion
            .map(|c| {
                if c.session_id() != self.session || c.revision() != self.revision {
                    (
                        Observation::Unavailable(Unavailable::ForeignSession),
                        Observation::Unavailable(Unavailable::ForeignSession),
                    )
                } else {
                    (
                        Observation::Observed(c.totals(false)),
                        Observation::Observed(c.totals(true)),
                    )
                }
            })
            .unwrap_or((
                Observation::Unavailable(Unavailable::MissingCompletion),
                Observation::Unavailable(Unavailable::MissingCompletion),
            ));
        let sample = Sample {
            session_id: self.session,
            revision,
            at_ns,
            epoch: self.epoch(),
            capture: if transition {
                Observation::Unavailable(Unavailable::RevisionTransition)
            } else {
                Observation::Observed(capture)
            },
            outputs: if transition {
                Observation::Unavailable(Unavailable::RevisionTransition)
            } else {
                completion.0
            },
            required_outputs: if transition {
                Observation::Unavailable(Unavailable::RevisionTransition)
            } else {
                completion.1
            },
        };
        self.bank
            .history
            .lock()
            .map_err(|_| TelemetryError::HistoryUnavailable)?
            .push(sample)?;
        Ok(sample)
    }
    /// Latest retained observation; stale data keeps its timestamp and value.
    pub fn latest(self, now_ns: u64) -> Observation<Sample> {
        let Ok(h) = self.bank.history.lock() else {
            return Observation::Unavailable(Unavailable::HistoryUnavailable);
        };
        match h.latest() {
            Some(s) if now_ns < s.at_ns || now_ns - s.at_ns > 2_000_000_000 => {
                Observation::Stale(s)
            }
            Some(s) => Observation::Observed(s),
            None => Observation::Unavailable(Unavailable::IncompleteWindow),
        }
    }
    /// Retained sample count, bounded by 301 (300 seconds plus baseline).
    /// A poisoned history is unavailable, distinct from an observed empty history.
    pub fn history_len(self) -> Observation<usize> {
        self.bank.history.lock().map_or(
            Observation::Unavailable(Unavailable::HistoryUnavailable),
            |h| Observation::Observed(h.len),
        )
    }
    /// Delta over one of the registered 10/60/300 second windows. Reset clears
    /// history; gaps and saturation make the whole interval unavailable.
    pub fn window(self, seconds: u16) -> Observation<Window> {
        self.bank.history.lock().map_or(
            Observation::Unavailable(Unavailable::HistoryUnavailable),
            |h| h.window(seconds),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_reset_recovers_poisoned_history() {
        let mut bank = Bank::new(1, 1);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = bank.history.lock().unwrap();
            panic!("inject management history failure");
        }));
        let t = Telemetry {
            bank: &bank,
            session: 42,
            revision: 1,
            updates: None,
        };
        assert_eq!(t.sample(0, None), Err(TelemetryError::HistoryUnavailable));
        assert_eq!(
            t.history_len(),
            Observation::Unavailable(Unavailable::HistoryUnavailable)
        );
        bank.reset(2);
        let t = Telemetry {
            bank: &bank,
            session: 42,
            revision: 1,
            updates: None,
        };
        assert_eq!(t.history_len(), Observation::Observed(0));
        assert!(t.sample(0, None).is_ok());
        assert_eq!(t.history_len(), Observation::Observed(1));
        assert_eq!(t.epoch(), 2);
    }
}
