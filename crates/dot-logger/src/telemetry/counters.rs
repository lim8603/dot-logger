use super::aggregate::History;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

/// Cumulative immediate capture decisions; target fanout never multiplies them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaptureCounts {
    /// Accepted record submissions (small plus raw).
    pub accepted: u64,
    /// Rejected record submissions, including invalid input and closed ingress.
    pub rejected: u64,
    /// Intentional filter omissions, distinct from rejection/loss.
    pub skipped: u64,
    /// Payload bytes accepted at ingress, not physical memory or durable bytes.
    pub accepted_bytes: u64,
    /// Failed raw-slot acquisition attempts, separate from record submissions.
    pub acquire_rejected: u64,
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
            overflowed: false,
        })
    }
}
// Avoid adjacent producers updating the same cache line on common platforms.
// Alignment is a layout choice, not a universal cache-isolation guarantee.
#[repr(align(128))]
pub(crate) struct CounterCell {
    values: [AtomicU64; 5],
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
        CaptureCounts {
            accepted: values[0],
            rejected: values[1],
            skipped: values[2],
            accepted_bytes: values[3],
            acquire_rejected: values[4],
            overflowed: self.overflowed.load(Ordering::Relaxed),
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
}
