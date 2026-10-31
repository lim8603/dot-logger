//! Borrowed, fixed-size original-payload slots and deferred reclamation.
//!
//! No external SDK buffers or callbacks enter the RT ownership path. Slot state
//! is authoritative; the one-slot return notification may coalesce arbitrarily.

use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;

use crate::capture::{CaptureError, CapturedRecord, Importance};
use crate::config::PoolQuota;

#[allow(unsafe_code)]
mod storage;

/// Quota-owned slot lifecycle, including permanently exhausted generations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SlotState {
    /// Reclaimed and eligible for a new loan.
    Free = 0,
    /// A unique caller may read/write the payload.
    Loaned = 1,
    /// Accepted, immutable payload awaiting a consumer.
    Queued = 2,
    /// An immutable reader still holds the payload.
    InUse = 3,
    /// All references ended; background reclamation is required.
    ReturnPending = 4,
    /// Generation space exhausted; storage remains reserved until owner Drop.
    Retired = 5,
}

/// Opaque identity information, not an authority to access or return a slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlotIdentity {
    /// Producer-local physical slot index.
    pub slot_id: usize,
    /// Generation, never wrapped or reused by the same slot.
    pub generation: u64,
}

/// Management snapshot; slots are sampled individually, not atomically together.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PoolSnapshot {
    /// Reclaimed slots.
    pub free: usize,
    /// Caller-held loans.
    pub loaned: usize,
    /// Accepted, queued originals.
    pub queued: usize,
    /// Originals still held by readers.
    pub in_use: usize,
    /// Returns waiting for background scan.
    pub return_pending: usize,
    /// Generation-exhausted slots.
    pub retired: usize,
    /// Whether the coalescing one-slot notification is full.
    pub notification_pending: bool,
}

impl PoolSnapshot {
    /// Read the sampled count for one lifecycle state.
    pub fn count(&self, state: SlotState) -> usize {
        match state {
            SlotState::Free => self.free,
            SlotState::Loaned => self.loaned,
            SlotState::Queued => self.queued,
            SlotState::InUse => self.in_use,
            SlotState::ReturnPending => self.return_pending,
            SlotState::Retired => self.retired,
        }
    }
}

/// Result of one bounded management scan; reclamation never frees payloads.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReclaimReport {
    /// Number of slot states inspected, at most the requested limit.
    pub inspected: usize,
    /// Slots transitioned exactly once from ReturnPending to Free.
    pub reclaimed: usize,
    /// A pending/coalesced hint was acknowledged; ownership did not depend on it.
    pub notification_received: bool,
}

/// Unique mutable original payload borrowed from the session's fixed pool.
/// Drop publishes ReturnPending without allocation, free, callbacks, or waiting.
///
/// ```compile_fail
/// use dot_logger::capture::{CaptureOwner, Importance};
/// fn escape(owner: &mut CaptureOwner) {
///     let loan = owner.scope(|mut scope| {
///         scope.next().unwrap().0.try_acquire(1, Importance::Normal).unwrap()
///     });
///     drop(loan);
/// }
/// ```
///
/// ```compile_fail
/// use dot_logger::pool::Loan;
/// fn duplicate(loan: Loan<'_>) { let _ = loan.clone(); }
/// ```
///
/// ```compile_fail
/// use dot_logger::pool::Loan;
/// fn require_sync<T: Sync>() {}
/// require_sync::<Loan<'static>>();
/// ```
///
/// ```compile_fail
/// use dot_logger::pool::Loan;
/// fn access_after_return(mut loan: Loan<'_>) {
///     let bytes = loan.payload_mut();
///     drop(loan);
///     bytes[0] = 1;
/// }
/// ```
///
/// ```compile_fail
/// use dot_logger::capture::{Producer, RecordMetadata};
/// use dot_logger::pool::Loan;
/// fn reuse_after_submit<'a>(producer: &mut Producer<'a>, meta: RecordMetadata, mut loan: Loan<'a>) {
///     let _ = producer.try_record_raw(meta, loan);
///     loan.payload_mut()[0] = 1;
/// }
/// ```
pub struct Loan<'owner> {
    access: storage::Exclusive<'owner>,
    pool: &'owner RawPool,
    length: usize,
    importance: Importance,
    not_sync: PhantomData<Cell<()>>,
}

impl Loan<'_> {
    /// This physical slot's identity; metadata alone grants no access.
    pub fn identity(&self) -> SlotIdentity {
        self.access.identity()
    }
    /// Requested, bounded valid payload length.
    pub fn len(&self) -> usize {
        self.length
    }
    /// Whether the valid payload is empty.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    /// Admission class fixed at acquisition, including protected bytes.
    pub fn importance(&self) -> Importance {
        self.importance
    }
    /// Read the valid loan bytes.
    pub fn payload(&self) -> &[u8] {
        &self.access.payload()[..self.length]
    }
    /// Exclusively fill the valid bytes before submission.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.access.payload_mut()[..self.length]
    }
    pub(crate) fn belongs_to(&self, pool: &RawPool) -> bool {
        std::ptr::eq(self.pool, pool)
    }
    pub(crate) fn publish(self) -> SlotIdentity {
        self.access.publish()
    }
}

impl fmt::Debug for Loan<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Loan")
            .field("identity", &self.identity())
            .field("length", &self.length)
            .field("importance", &self.importance)
            .finish_non_exhaustive()
    }
}

/// Raw admission result; intentional omission returns the same loan.
#[derive(Debug)]
pub enum RawRecordOutcome<'owner> {
    /// Only ingress was accepted, with the original slot retained by the owner.
    Accepted(crate::capture::CaptureReceipt),
    /// Filter omission; the caller retains the original loan.
    Skipped(Loan<'owner>),
}

/// Failed admission with ownership of the unchanged original loan.
#[derive(Debug)]
pub struct RawRecordError<'owner> {
    /// Fixed reason; constructing it never formats or allocates diagnostics.
    pub reason: CaptureError,
    /// Same slot, generation, contents, and pointer as the submitted loan.
    pub loan: Loan<'owner>,
}

/// Immutable original retained until every borrow of this guard ends.
///
/// ```compile_fail
/// use dot_logger::pool::RawRecord;
/// fn mutate(record: &mut RawRecord<'_>) { record.payload()[0] = 1; }
/// ```
///
/// ```compile_fail
/// use dot_logger::capture::CaptureOwner;
/// fn escape_reader(owner: &mut CaptureOwner) {
///     let record = owner.scope(|mut scope| scope.next().unwrap().1.try_take_raw().unwrap());
///     drop(record);
/// }
/// ```
pub struct RawRecord<'owner> {
    access: storage::Shared<'owner>,
    record: CapturedRecord,
    pub(crate) plan_guard: Option<crate::control::config_update::PlanGuard<'owner>>,
}

impl RawRecord<'_> {
    /// Metadata and ingress identity associated with the immutable payload.
    pub fn record(&self) -> CapturedRecord {
        self.record
    }
    /// Physical identity, including generation.
    pub fn identity(&self) -> SlotIdentity {
        self.access.identity()
    }
    /// Original bytes, without capture/consumer payload copying.
    pub fn payload(&self) -> &[u8] {
        &self.access.payload()[..self.record.payload_len]
    }
}

pub(crate) struct RawPool {
    slots: Box<[storage::Slot]>,
    notice: storage::ReturnNotice,
    slot_bytes: usize,
    normal_slots: usize,
}

impl RawPool {
    pub(crate) fn layout(&self) -> (usize, usize) {
        (self.slots.len(), self.slot_bytes)
    }

    pub(crate) fn metadata_bytes(quota: &PoolQuota) -> Option<usize> {
        quota.slots.checked_mul(size_of::<storage::Slot>())
    }

    pub(crate) fn new(quota: &PoolQuota) -> Self {
        Self {
            slots: (0..quota.slots)
                .map(|_| storage::Slot::new(quota.slot_bytes))
                .collect(),
            notice: storage::ReturnNotice::new(),
            slot_bytes: quota.slot_bytes,
            normal_slots: quota.slots - quota.important_slots,
        }
    }

    pub(crate) fn acquire(
        &self,
        length: usize,
        importance: Importance,
        cursor: &mut usize,
    ) -> Result<Loan<'_>, CaptureError> {
        if length > self.slot_bytes {
            return Err(CaptureError::TooLarge);
        }
        let mut eligible = 0;
        let mut retired = 0;
        // Important uses its reservation first, then may use Normal capacity.
        // Normal never borrows the protected physical slots or their bytes.
        for reserved in [true, false] {
            if reserved && importance == Importance::Normal {
                continue;
            }
            for offset in 0..self.slots.len() {
                let index = (*cursor + offset) % self.slots.len();
                if (index >= self.normal_slots) != reserved {
                    continue;
                }
                eligible += 1;
                if let Some(access) = self.slots[index].acquire(index, &self.notice) {
                    *cursor = (index + 1) % self.slots.len();
                    return Ok(Loan {
                        access,
                        pool: self,
                        length,
                        importance,
                        not_sync: PhantomData,
                    });
                }
                retired += usize::from(self.slots[index].state() == SlotState::Retired);
            }
        }
        Err(if eligible > 0 && retired == eligible {
            CaptureError::GenerationExhausted
        } else {
            CaptureError::Capacity
        })
    }

    pub(crate) fn take(
        &self,
        identity: SlotIdentity,
        record: CapturedRecord,
    ) -> Option<RawRecord<'_>> {
        let access = self
            .slots
            .get(identity.slot_id)?
            .read(identity, &self.notice)?;
        Some(RawRecord {
            access,
            record,
            plan_guard: None,
        })
    }

    pub(crate) fn snapshot(&self) -> PoolSnapshot {
        let mut snapshot = PoolSnapshot {
            notification_pending: self.notice.pending(),
            ..PoolSnapshot::default()
        };
        for slot in &self.slots {
            match slot.state() {
                SlotState::Free => snapshot.free += 1,
                SlotState::Loaned => snapshot.loaned += 1,
                SlotState::Queued => snapshot.queued += 1,
                SlotState::InUse => snapshot.in_use += 1,
                SlotState::ReturnPending => snapshot.return_pending += 1,
                SlotState::Retired => snapshot.retired += 1,
            }
        }
        snapshot
    }

    pub(crate) fn reclaim(&self, limit: usize, cursor: &mut usize) -> ReclaimReport {
        if limit == 0 {
            return ReclaimReport::default();
        }
        let mut report = ReclaimReport {
            notification_received: self.notice.take(),
            ..ReclaimReport::default()
        };
        // Always advance the scan, including without a notification. Repeated
        // coalesced hints cannot starve an older unnotified ReturnPending slot.
        for _ in 0..limit.min(self.slots.len()) {
            report.inspected += 1;
            report.reclaimed += usize::from(self.slots[*cursor].reclaim());
            *cursor = (*cursor + 1) % self.slots.len();
        }
        report
    }
}
