//! Audited raw payload ownership boundary.
//!
//! # Safety invariants
//! Each slot permanently owns its allocation. An atomic word packs a 61-bit
//! generation and its state. Exactly one successful Free -> Loaned CAS grants
//! an Exclusive guard. It cannot be cloned; payload references borrow the guard.
//! Consuming that guard publishes Queued with Release, after mutable borrows end.
//! A generation-matching Queued -> InUse Acquire CAS grants one Shared guard.
//! Readers only expose immutable slices borrowing that guard. Guard Drop publishes
//! ReturnPending with Release after the final payload reference ends. Reclamation
//! acquires that state before publishing Free; the next loan acquires Free.
//! Thus payload writes, reads, and subsequent writes have happens-before edges.
//!
//! Generation and state are compared together, so a stale token cannot acquire
//! a recycled slot. Generation never wraps: an exhausted free slot is retired.
//! Return notification is a coalescing one-slot mailbox, never the ownership
//! record. A saturated mailbox cannot erase ReturnPending. Neither guard owns
//! allocations or an Arc, and Drop only performs fixed atomic operations.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{SlotIdentity, SlotState};

const STATE_BITS: u32 = 3;
const STATE_MASK: u64 = (1 << STATE_BITS) - 1;
pub(super) const MAX_GENERATION: u64 = u64::MAX >> STATE_BITS;

fn word(generation: u64, state: SlotState) -> u64 {
    (generation << STATE_BITS) | state as u64
}

fn state(word: u64) -> SlotState {
    match word & STATE_MASK {
        0 => SlotState::Free,
        1 => SlotState::Loaned,
        2 => SlotState::Queued,
        3 => SlotState::InUse,
        4 => SlotState::ReturnPending,
        _ => SlotState::Retired,
    }
}

pub(super) struct ReturnNotice(AtomicBool);

impl ReturnNotice {
    pub(super) fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    fn post(&self) -> bool {
        self.0
            .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
            .is_ok()
    }

    pub(super) fn take(&self) -> bool {
        self.0.swap(false, Ordering::AcqRel)
    }
    pub(super) fn pending(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub(super) struct Slot {
    payload: UnsafeCell<Box<[u8]>>,
    control: AtomicU64,
}

// SAFETY: Safe callers can access payload only through the uncloneable guards
// below. Atomic generation+state grants exclusive writer or immutable reader
// access, and no reuse occurs until the guard's Release return is acquired.
// The Box pointer/length never changes while shared; only its bytes are mutated.
unsafe impl Sync for Slot {}

impl Slot {
    pub(super) fn new(bytes: usize) -> Self {
        let mut payload = vec![0; bytes].into_boxed_slice();
        // Startup touch, not memory locking or a claim about real RT latency.
        std::hint::black_box(&mut payload).fill(0);
        Self {
            payload: UnsafeCell::new(payload),
            control: AtomicU64::new(word(0, SlotState::Free)),
        }
    }

    pub(super) fn state(&self) -> SlotState {
        state(self.control.load(Ordering::Acquire))
    }

    pub(super) fn acquire<'a>(
        &'a self,
        slot_id: usize,
        notice: &'a ReturnNotice,
    ) -> Option<Exclusive<'a>> {
        let current = self.control.load(Ordering::Acquire);
        if state(current) != SlotState::Free {
            return None;
        }
        let generation = current >> STATE_BITS;
        if generation == MAX_GENERATION {
            let _ = self.control.compare_exchange(
                current,
                word(generation, SlotState::Retired),
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
            return None;
        }
        let generation = generation + 1;
        self.control
            .compare_exchange(
                current,
                word(generation, SlotState::Loaned),
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()?;
        Some(Exclusive {
            slot: self,
            notice,
            identity: SlotIdentity {
                slot_id,
                generation,
            },
            armed: true,
        })
    }

    pub(super) fn read<'a>(
        &'a self,
        identity: SlotIdentity,
        notice: &'a ReturnNotice,
    ) -> Option<Shared<'a>> {
        if identity.generation > MAX_GENERATION {
            return None;
        }
        self.control
            .compare_exchange(
                word(identity.generation, SlotState::Queued),
                word(identity.generation, SlotState::InUse),
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()?;
        Some(Shared {
            slot: self,
            notice,
            identity,
        })
    }

    pub(super) fn reclaim(&self) -> bool {
        let current = self.control.load(Ordering::Acquire);
        if state(current) != SlotState::ReturnPending {
            return false;
        }
        self.control
            .compare_exchange(
                current,
                word(current >> STATE_BITS, SlotState::Free),
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
    }
}

pub(super) struct Exclusive<'a> {
    slot: &'a Slot,
    notice: &'a ReturnNotice,
    identity: SlotIdentity,
    armed: bool,
}

impl Exclusive<'_> {
    pub(super) fn identity(&self) -> SlotIdentity {
        self.identity
    }

    pub(super) fn payload(&self) -> &[u8] {
        // SAFETY: The Loaned state and unique guard exclude all other access.
        // The returned borrow ends before this guard can publish or be dropped.
        let payload = unsafe { &*self.slot.payload.get() };
        payload.as_ref()
    }

    pub(super) fn payload_mut(&mut self) -> &mut [u8] {
        // SAFETY: This uncloneable guard exclusively owns the Loaned generation.
        // &mut self prevents another live payload borrow through this guard.
        let payload = unsafe { &mut *self.slot.payload.get() };
        payload.as_mut()
    }

    pub(super) fn publish(mut self) -> SlotIdentity {
        self.armed = false;
        self.slot.control.store(
            word(self.identity.generation, SlotState::Queued),
            Ordering::Release,
        );
        self.identity
    }
}

impl Drop for Exclusive<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.slot.control.store(
                word(self.identity.generation, SlotState::ReturnPending),
                Ordering::Release,
            );
            let _ = self.notice.post();
        }
    }
}

pub(super) struct Shared<'a> {
    slot: &'a Slot,
    notice: &'a ReturnNotice,
    identity: SlotIdentity,
}

impl Shared<'_> {
    pub(super) fn identity(&self) -> SlotIdentity {
        self.identity
    }

    pub(super) fn payload(&self) -> &[u8] {
        // SAFETY: A generation-matching CAS granted the unique InUse guard;
        // all mutable loan borrows ended before the Release publish it acquired.
        // Reclaim cannot succeed before this guard's Drop and borrows ending.
        let payload = unsafe { &*self.slot.payload.get() };
        payload.as_ref()
    }
}

impl Drop for Shared<'_> {
    fn drop(&mut self) {
        self.slot.control.store(
            word(self.identity.generation, SlotState::ReturnPending),
            Ordering::Release,
        );
        let _ = self.notice.post();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn out_of_range_generation_cannot_alias_a_current_queued_slot() {
        let slot = Slot::new(1);
        let notice = ReturnNotice::new();
        let identity = slot.acquire(0, &notice).unwrap().publish();
        let forged = SlotIdentity {
            slot_id: 0,
            generation: identity.generation + MAX_GENERATION + 1,
        };
        assert!(slot.read(forged, &notice).is_none());
        assert_eq!(slot.state(), SlotState::Queued);
        let read = slot.read(identity, &notice).unwrap();
        drop(read);
        assert!(slot.reclaim());
    }

    #[test]
    fn stale_and_duplicate_identities_cannot_steal_a_recycled_generation() {
        let slot = Slot::new(1);
        let notice = ReturnNotice::new();
        let old = slot.acquire(0, &notice).unwrap().publish();
        let view = slot.read(old, &notice).unwrap();
        assert!(slot.read(old, &notice).is_none());
        assert!(!slot.reclaim());
        drop(view);
        assert!(slot.reclaim());
        assert!(!slot.reclaim());
        let mut loan = slot.acquire(0, &notice).unwrap();
        loan.payload_mut()[0] = 42;
        let new = loan.publish();
        assert!(new.generation > old.generation);
        assert!(slot.read(old, &notice).is_none());
        let view = slot.read(new, &notice).unwrap();
        assert_eq!(view.payload(), &[42]);
        drop(view);
        assert!(slot.reclaim());
    }

    #[test]
    fn final_generation_is_valid_once_then_slot_retires_without_wrapping() {
        let slot = Slot::new(1);
        let notice = ReturnNotice::new();
        slot.control
            .store(word(MAX_GENERATION - 1, SlotState::Free), Ordering::Release);
        let identity = slot.acquire(0, &notice).unwrap().publish();
        assert_eq!(identity.generation, MAX_GENERATION);
        let view = slot.read(identity, &notice).unwrap();
        drop(view);
        assert!(slot.reclaim());
        assert!(slot.acquire(0, &notice).is_none());
        assert_eq!(slot.state(), SlotState::Retired);
        assert!(slot.acquire(0, &notice).is_none());
        assert!(!slot.reclaim());
    }

    #[test]
    fn saturated_notification_does_not_replace_slot_return_state() {
        let notice = ReturnNotice::new();
        assert!(notice.post());
        assert!(!notice.post());
        let slot = Slot::new(1);
        let loan = slot.acquire(0, &notice).unwrap();
        drop(loan);
        assert_eq!(slot.state(), SlotState::ReturnPending);
        assert!(notice.take());
        assert!(!notice.take());
        assert!(slot.reclaim());
        assert!(!slot.reclaim());
    }
}
