use super::aggregate::History;
use crate::capture::CaptureError;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

/// Cumulative immediate capture decisions; target fanout never multiplies them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaptureCounts {
    /// Accepted record submissions (small plus raw).
    pub accepted: u64,
    /// All rejected submission attempts; retries count again. Sum of the three
    /// rejection categories below, not a unique-record loss count.
    pub rejected: u64,
    /// Intentional filter omissions, distinct from rejection/loss.
    pub skipped: u64,
    /// Payload bytes accepted at ingress, not physical memory or durable bytes.
    pub accepted_bytes: u64,
    /// Failed raw-slot acquisition attempts, separate from record submissions.
    pub acquire_rejected: u64,
    /// Submission attempts rejected for capacity, including completion capacity.
    pub rejected_capacity: u64,
    /// Submission attempts rejected for invalid/unsupported caller input.
    pub rejected_contract: u64,
    /// Submission attempts rejected for closure or exhausted identities.
    pub rejected_lifecycle: u64,
    /// Raw acquisition attempts rejected for capacity; not additional records.
    pub acquire_rejected_capacity: u64,
    /// Raw acquisition attempts rejected for invalid/unsupported caller input.
    pub acquire_rejected_contract: u64,
    /// Raw acquisition attempts rejected for closure or exhausted identities.
    pub acquire_rejected_lifecycle: u64,
    /// A count saturated; full-success/rate claims are invalid.
    pub overflowed: bool,
}
impl CaptureCounts {
    pub(crate) fn add(&mut self, other: Self) {
        self.overflowed |= other.overflowed;
        for (a, b) in [
            (&mut self.accepted, other.accepted),
            (&mut self.rejected, other.rejected),
            (&mut self.skipped, other.skipped),
            (&mut self.accepted_bytes, other.accepted_bytes),
            (&mut self.acquire_rejected, other.acquire_rejected),
            (&mut self.rejected_capacity, other.rejected_capacity),
            (&mut self.rejected_contract, other.rejected_contract),
            (&mut self.rejected_lifecycle, other.rejected_lifecycle),
            (
                &mut self.acquire_rejected_capacity,
                other.acquire_rejected_capacity,
            ),
            (
                &mut self.acquire_rejected_contract,
                other.acquire_rejected_contract,
            ),
            (
                &mut self.acquire_rejected_lifecycle,
                other.acquire_rejected_lifecycle,
            ),
        ] {
            match a.checked_add(b) {
                Some(n) => *a = n,
                None => {
                    *a = u64::MAX;
                    self.overflowed = true;
                }
            }
        }
    }
    pub(crate) fn delta(self, old: Self) -> Option<Self> {
        if self.overflowed || old.overflowed {
            return None;
        }
        Some(Self {
            accepted: self.accepted.checked_sub(old.accepted)?,
            rejected: self.rejected.checked_sub(old.rejected)?,
            skipped: self.skipped.checked_sub(old.skipped)?,
            accepted_bytes: self.accepted_bytes.checked_sub(old.accepted_bytes)?,
            acquire_rejected: self.acquire_rejected.checked_sub(old.acquire_rejected)?,
            rejected_capacity: self.rejected_capacity.checked_sub(old.rejected_capacity)?,
            rejected_contract: self.rejected_contract.checked_sub(old.rejected_contract)?,
            rejected_lifecycle: self
                .rejected_lifecycle
                .checked_sub(old.rejected_lifecycle)?,
            acquire_rejected_capacity: self
                .acquire_rejected_capacity
                .checked_sub(old.acquire_rejected_capacity)?,
            acquire_rejected_contract: self
                .acquire_rejected_contract
                .checked_sub(old.acquire_rejected_contract)?,
            acquire_rejected_lifecycle: self
                .acquire_rejected_lifecycle
                .checked_sub(old.acquire_rejected_lifecycle)?,
            overflowed: false,
        })
    }
}
// Avoid adjacent producers updating the same cache line on common platforms.
// Alignment is a layout choice, not a universal cache-isolation guarantee.
#[repr(align(128))]
pub(crate) struct CounterCell {
    values: [AtomicU64; 9],
    overflowed: AtomicBool,
}
impl CounterCell {
    fn new() -> Self {
        Self {
            values: std::array::from_fn(|_| AtomicU64::new(0)),
            overflowed: AtomicBool::new(false),
        }
    }
    // Exactly one Producer writes each cell. Reset requires exclusive owner
    // access after all capture/telemetry borrows end. Counters carry no payload
    // publication ordering: relaxed loads/stores suffice, with no RMW or retry.
    // A concurrent sample can mix nearby event counts; after quiescence it is
    // exact. This explicitly avoids imposing a global/seqlock cost on RT calls.
    pub(crate) fn record(&self, kind: usize, bytes: usize) {
        self.increment(kind, 1);
        if kind == 0 {
            self.increment(3, bytes as u64);
        }
    }
    /// One category store per failure; total counts are derived on management reads.
    pub(crate) fn reject(&self, error: CaptureError, acquisition: bool) {
        let index = match (error, acquisition) {
            (
                CaptureError::Capacity
                | CaptureError::CompletionCapacity
                | CaptureError::DeadlineExceeded,
                false,
            ) => 1,
            (
                CaptureError::Capacity
                | CaptureError::CompletionCapacity
                | CaptureError::DeadlineExceeded,
                true,
            ) => 4,
            (
                CaptureError::InvalidInput
                | CaptureError::SchemaMismatch
                | CaptureError::TooLarge
                | CaptureError::Unsupported,
                false,
            ) => 5,
            (
                CaptureError::InvalidInput
                | CaptureError::SchemaMismatch
                | CaptureError::TooLarge
                | CaptureError::Unsupported,
                true,
            ) => 7,
            (
                CaptureError::Closing
                | CaptureError::SequenceExhausted
                | CaptureError::GenerationExhausted,
                false,
            ) => 6,
            (
                CaptureError::Closing
                | CaptureError::SequenceExhausted
                | CaptureError::GenerationExhausted,
                true,
            ) => 8,
        };
        self.increment(index, 1);
    }
    fn increment(&self, index: usize, count: u64) {
        let old = self.values[index].load(Ordering::Relaxed);
        let (next, overflow) = old.overflowing_add(count);
        self.values[index].store(if overflow { u64::MAX } else { next }, Ordering::Relaxed);
        if overflow {
            self.overflowed.store(true, Ordering::Relaxed);
        }
    }
    pub(crate) fn read(&self) -> CaptureCounts {
        let values = self.values.each_ref().map(|x| x.load(Ordering::Relaxed));
        let mut overflowed = self.overflowed.load(Ordering::Relaxed);
        let mut total = |indices: [usize; 3]| {
            indices
                .into_iter()
                .fold(0_u64, |sum, i| match sum.checked_add(values[i]) {
                    Some(n) => n,
                    None => {
                        overflowed = true;
                        u64::MAX
                    }
                })
        };
        let rejected = total([1, 5, 6]);
        let acquire_rejected = total([4, 7, 8]);
        CaptureCounts {
            accepted: values[0],
            rejected,
            skipped: values[2],
            accepted_bytes: values[3],
            acquire_rejected,
            rejected_capacity: values[1],
            rejected_contract: values[5],
            rejected_lifecycle: values[6],
            acquire_rejected_capacity: values[4],
            acquire_rejected_contract: values[7],
            acquire_rejected_lifecycle: values[8],
            overflowed,
        }
    }
}

pub(crate) struct Bank {
    pub(crate) cells: Box<[CounterCell]>,
    pub(crate) history: Mutex<History>,
    pub(crate) epoch: u64,
}
impl Bank {
    pub(crate) fn required(producers: usize) -> Option<usize> {
        producers
            .checked_mul(size_of::<CounterCell>())?
            .checked_add(size_of::<Self>())?
            .checked_add(History::required())
    }
    pub(crate) fn new(producers: usize, epoch: u64) -> Self {
        Self {
            cells: (0..producers).map(|_| CounterCell::new()).collect(),
            history: Mutex::new(History::new()),
            epoch,
        }
    }
    pub(crate) fn reset(&mut self, epoch: u64) {
        for cell in &mut self.cells {
            *cell = CounterCell::new();
        }
        // Exclusive owner access: no telemetry or capture borrows can still exist.
        match self.history.get_mut() {
            Ok(h) => h.clear(),
            Err(e) => e.into_inner().clear(),
        }
        self.history.clear_poison();
        self.epoch = epoch;
    }
}
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn saturation_never_wraps_or_looks_successful() {
        let c = CounterCell::new();
        c.values[0].store(u64::MAX, Ordering::Relaxed);
        c.record(0, 1);
        assert_eq!(c.read().accepted, u64::MAX);
        assert!(c.read().overflowed);
        assert_eq!(c.read().delta(CaptureCounts::default()), None);
    }

    #[test]
    fn rejection_categories_survive_aggregation_delta_and_total_overflow() {
        let c = CounterCell::new();
        for acquisition in [false, true] {
            for error in [
                CaptureError::Capacity,
                CaptureError::CompletionCapacity,
                CaptureError::DeadlineExceeded,
            ] {
                c.reject(error, acquisition);
            }
            for error in [
                CaptureError::InvalidInput,
                CaptureError::SchemaMismatch,
                CaptureError::TooLarge,
                CaptureError::Unsupported,
            ] {
                c.reject(error, acquisition);
            }
            for error in [
                CaptureError::Closing,
                CaptureError::SequenceExhausted,
                CaptureError::GenerationExhausted,
            ] {
                c.reject(error, acquisition);
            }
        }
        let old = c.read();
        assert_eq!((old.rejected, old.acquire_rejected), (10, 10));
        assert_eq!(
            (
                old.rejected_capacity,
                old.rejected_contract,
                old.rejected_lifecycle
            ),
            (3, 4, 3)
        );
        assert_eq!(
            (
                old.acquire_rejected_capacity,
                old.acquire_rejected_contract,
                old.acquire_rejected_lifecycle
            ),
            (3, 4, 3)
        );
        let mut combined = old;
        combined.add(old);
        assert_eq!(combined.delta(old), Some(old));
        c.values[1].store(u64::MAX, Ordering::Relaxed);
        let saturated = c.read();
        assert_eq!(saturated.rejected, u64::MAX);
        assert!(saturated.overflowed);
        assert_eq!(saturated.delta(old), None);
        combined.add(saturated);
        assert!(combined.overflowed);
        assert_eq!(combined.rejected, u64::MAX);
        assert_eq!(size_of::<CounterCell>(), 128);
    }
}
