use super::{CaptureCounts, Observation, Unavailable};
use crate::completion::CompletionTotals;
const SAMPLES: usize = 301;
/// One-second cumulative report; actual sampling time is always retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sample {
    /// Original session identity, carried with every exported sample.
    pub session_id: u128,
    /// Immutable capture revision.
    pub revision: u64,
    /// Host-supplied session-monotonic nanoseconds.
    pub at_ns: u64,
    /// Counter generation; reset never becomes a wrapping delta.
    pub epoch: u64,
    /// Ingress decisions summed once per producer.
    pub capture: Observation<CaptureCounts>,
    /// All target assignments, including failures/drops/unknown/pending.
    pub outputs: Observation<CompletionTotals>,
    /// Only required-target assignments, with the same session record count.
    pub required_outputs: Observation<CompletionTotals>,
}
/// Observed counter interval; no throughput/capacity forecast is inferred.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Window {
    /// Original session identity.
    pub session_id: u128,
    /// Immutable capture revision.
    pub revision: u64,
    /// Counter generation; independent of completion's session totals.
    pub epoch: u64,
    /// Actual first observation time.
    pub from_ns: u64,
    /// Actual last observation time.
    pub to_ns: u64,
    /// Exact differences for the retained interval; intentional skips stay separate.
    pub counts: CaptureCounts,
}
/// General-path sampling/reset failure; existing history is unchanged on rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryError {
    /// Samples must advance by at least one second in the session clock.
    TimeOrder,
    /// Enable/reset exhausted its counter generation; no wrap is performed.
    EpochExhausted,
    /// Explicit enable is required before reset.
    Disabled,
    /// A management history lock was poisoned.
    HistoryUnavailable,
}
impl std::fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "telemetry: {self:?}")
    }
}
impl std::error::Error for TelemetryError {}
pub(crate) struct History {
    slots: Box<[Option<Sample>]>,
    next: usize,
    pub(crate) len: usize,
}
impl History {
    pub(crate) fn required() -> usize {
        SAMPLES * size_of::<Option<Sample>>()
    }
    pub(crate) fn new() -> Self {
        Self {
            slots: vec![None; SAMPLES].into_boxed_slice(),
            next: 0,
            len: 0,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.slots.fill(None);
        self.next = 0;
        self.len = 0;
    }
    pub(crate) fn latest(&self) -> Option<Sample> {
        self.slots[(self.next + SAMPLES - 1) % SAMPLES]
    }
    pub(crate) fn push(&mut self, sample: Sample) -> Result<(), TelemetryError> {
        if self.latest().is_some_and(|s| {
            sample
                .at_ns
                .checked_sub(s.at_ns)
                .is_none_or(|n| n < 1_000_000_000)
        }) {
            return Err(TelemetryError::TimeOrder);
        }
        self.slots[self.next] = Some(sample);
        self.next = (self.next + 1) % SAMPLES;
        self.len = (self.len + 1).min(SAMPLES);
        Ok(())
    }
    pub(crate) fn window(&self, seconds: u16) -> Observation<Window> {
        let unavailable = Observation::Unavailable(Unavailable::IncompleteWindow);
        if ![10, 60, 300].contains(&seconds) {
            return unavailable;
        }
        let Some(last) = self.latest() else {
            return unavailable;
        };
        let Some(target) = last.at_ns.checked_sub(u64::from(seconds) * 1_000_000_000) else {
            return unavailable;
        };
        let mut newer = last;
        for offset in 1..self.len {
            let Some(old) = self.slots[(self.next + SAMPLES - 1 - offset) % SAMPLES] else {
                return unavailable;
            };
            if newer.at_ns - old.at_ns > 2_000_000_000
                || old.epoch != last.epoch
                || old.revision != last.revision
                || old.session_id != last.session_id
            {
                return unavailable;
            }
            let (Observation::Observed(_), Observation::Observed(_)) = (old.capture, newer.capture)
            else {
                return unavailable;
            };
            if old.at_ns <= target {
                let (Observation::Observed(a), Observation::Observed(b)) =
                    (last.capture, old.capture)
                else {
                    return unavailable;
                };
                return match a.delta(b) {
                    Some(counts) => Observation::Observed(Window {
                        session_id: last.session_id,
                        revision: last.revision,
                        epoch: last.epoch,
                        from_ns: old.at_ns,
                        to_ns: last.at_ns,
                        counts,
                    }),
                    None => Observation::Unavailable(Unavailable::Saturated),
                };
            }
            newer = old;
        }
        unavailable
    }
}
