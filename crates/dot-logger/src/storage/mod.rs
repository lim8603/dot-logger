//! Management-path file storage. Payload encoding and file IO never run on RT handles.
//!
//! # Single segment
//!
//! `SegmentWriter` keeps one active writer per target and checks the stated
//! maximum payload, record, chunk and metadata limits. The default writer is
//! uncompressed with chunk CRC and indexes and publishes the safe offset of a
//! complete chunk only after `flush_chunk` succeeds. `write_batch(&[&Delivery])`
//! accumulates up to the chunk target and reports Written together after the last
//! flush; a batch error keeps the append/flush/report progress, and after `Busy`
//! only the same guard's `report` is retried. `write_delivery` is the single-record
//! path for an immediate Written. A `WriterConfig::write_buffer_bytes` buffer is
//! charged to the shared output budget (0 writes directly). `finish` writes the
//! summary and footer; `FinishedSegment::sync_work` confirms a control request
//! after the real file sync. A sync validation error before IO returns the
//! original work in `SyncWorkError::work`, so it can be retried on the right file.
//! A new file's sync never confirms records of another, already finished segment.
//! Apart from the `compression-zstd` dependency option, the product writer is
//! uncompressed.
//!
//! # Rotation and retention
//!
//! `SegmentLog` rotates on bytes, monotonic time, message count, index
//! bytes/chunk limits and delivery revision boundaries. `append_with_budget`
//! explicitly selects a retained budget of the same router/target; `write_batch`
//! derives it from the live delivery guard. Files remain single-revision, while
//! sequence, receipt watermarks, events, pins and sync debt survive rotations. `RetentionConfig` states continuous, ring or event mode and
//! the file/disk/protection/query-pin budgets. Rotation publishes the closed
//! catalog only after footer → file sync → final name → directory sync.
//! Overlapping events do not charge protected bytes twice and report the real
//! pre/post record ranges and any missing history. Query pins bound their deadline
//! and read bytes and must be released explicitly after reading before deletion can
//! proceed. A partial `write_batch` failure keeps the appended / written / reported
//! prefixes and the original error in `RetentionBatchError`. Deliveries of one
//! target arrive in producer FIFO order and an admitted prefix is never rewritten.
//! File/index byte limits use conservative bounds of the pinned MCAP and may rotate
//! earlier than the exact size requires.
//!
//! `SegmentStore` requires a fresh free-space observation from the host and
//! directory sync. The standard `FileStore` creates fresh session/target
//! directories and implements Unix directory sync; elsewhere the primitive is an
//! explicit `Unsupported`, never an automatic success. Real free-space discovery,
//! space contention with other processes, restart recovery, active readers and
//! device durability are host or later verification scope.
//!
//! # Recovering an interrupted segment
//!
//! `recover_segment` reads an unchanged original through `Read + Seek` into an
//! independent empty sink. The caller supplies the original session, revision and
//! channel registrations, `RecoveryOptions` (input size, record/chunk limits,
//! output `WriterConfig`) and a cancel flag. It targets the product's uncompressed
//! `dot-logger/standard/2` format with nonzero chunk CRC and copies a complete chunk only
//! after its length, CRC, references, user/meta pairs and producer sequences all check.
//! It does not resynchronize after damage: `RecoveryReport` names the verified
//! prefix, the excluded suffix and the cause. Original and opaque payloads are
//! kept under a new standard summary and auxiliary indexes, and the new file is
//! synced. Written/Synced answers of the earlier process are neither restored nor
//! guessed.
//!
//! `recover_to_store` drives new file → recovery and file sync → close → final
//! name → directory sync, using `SegmentStore`'s exclusive namespace and
//! no-overwrite contract (`FileStore`'s directory sync is Unix-only). A failed
//! active or renamed file is left for investigation: no automatic deletion, retry
//! or catalog registration. The host opens the original read-only, keeps it
//! unchanged during recovery and links per-target IO errors to the existing
//! completion failure reports.
mod catalog;
pub(crate) mod codec;
mod index;
mod read;
pub(crate) mod recovery;
mod retention;
pub(crate) use retention::{
    log_required, loss_checkpoint_bounds, native_file_minimum, smallest_rotated_segment,
};
mod writer;
use crate::capture::ChannelConfig;
use crate::routing::{SharedLedger, TargetLedger};
pub(crate) use catalog::ReadPermit;
pub use catalog::{EventReport, ReadPin, RetentionConfig, RetentionMode, SegmentInfo};
pub use codec::StoredRecord;
pub use index::ChunkIndexEntry;
pub use read::{FileInfo, LossReader, NativeReader};
pub use recovery::{
    RecoveredSegment, RecoveryConfig, RecoveryError, RecoveryOptions, RecoveryReport, RecoveryStop,
    recover_segment, recover_to_store,
};
pub use retention::{
    AppendReport, FileStore, RetentionBatchError, RetentionError, RotationReason, SegmentLog,
    SegmentStore,
};
use std::sync::Arc;
use std::sync::atomic::Ordering;
pub(crate) use writer::registration_bytes;
pub use writer::{FinishedSegment, SegmentWriter, SyncSink};

/// Opaque reservation authority for one configured target and a shared output partition.
#[derive(Clone)]
pub struct WriterBudget {
    pub(crate) ledger: Arc<SharedLedger>,
    pub(crate) session: u128,
    pub(crate) revision: u64,
    pub(crate) loss: Option<Arc<crate::loss::Bank>>,
    pub(crate) recovered_file: Option<FileInfo>,
    pub(crate) target: u32,
    pub(crate) producers: usize,
    pub(crate) target_state: Arc<TargetLedger>,
}
impl WriterBudget {
    pub(crate) fn loss_rows(&self) -> usize {
        self.loss.as_ref().map_or_else(
            || self.recovered_file.map_or(0, |f| f.loss_rows),
            |bank| bank.len(),
        )
    }
    pub(crate) fn same_target(&self, other: &Self) -> bool {
        self.session == other.session
            && self.target == other.target
            && self.producers == other.producers
            && Arc::ptr_eq(&self.ledger, &other.ledger)
            && Arc::ptr_eq(&self.target_state, &other.target_state)
    }

    /// All currently live writer reservations from this router.
    pub fn used_bytes(&self) -> usize {
        self.ledger.storage_used.load(Ordering::Acquire)
    }
    /// Total output bytes remaining after routing allocations.
    pub fn limit_bytes(&self) -> usize {
        self.ledger.storage_limit
    }
    /// Management path. Writers of one router share the ledger and rotate
    /// together, so a lost CAS race is retried against the new total: only
    /// a real shortfall fails (TASK-023 S3 finding ⑩). Some contender always
    /// succeeds, so the retry is lock-free and bounded by contention.
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Lease, StorageError> {
        let mut used = self.used_bytes();
        loop {
            let next = used.checked_add(bytes).ok_or(StorageError::InvalidConfig)?;
            if next > self.limit_bytes() {
                return Err(StorageError::Budget {
                    required: bytes,
                    available: self.limit_bytes().saturating_sub(used),
                });
            }
            match self.ledger.storage_used.compare_exchange_weak(
                used,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => used = actual,
            }
        }
        Ok(Lease {
            budget: self.clone(),
            bytes,
        })
    }
}
pub(crate) struct Lease {
    budget: WriterBudget,
    bytes: usize,
}
/// Files one target wrote, kept across its successive writers. Under a Written
/// completion contract an ended or expired record was only written, so the
/// completion history cannot show which file still owes its sync; a barrier
/// on one file asks this ledger instead (REV-052 RV-128). Management path only.
#[derive(Default)]
pub(crate) struct FileLedger {
    /// Per producer: highest sequence any file of this target appended.
    appended: Box<[std::sync::atomic::AtomicU64]>,
    owed: std::sync::Mutex<Owed>,
}
/// Per producer `(first, last)`, `(0, 0)` when none.
type Ranges = Box<[(u64, u64)]>;
/// Closed files whose owed ranges are kept exactly; more share one hull.
const PENDING_FILES: usize = 4;
#[derive(Default)]
struct Owed {
    /// Files released unsynced that nothing will sync later (dropped writers).
    stuck: usize,
    /// Per producer: hull of those files' unsynced ranges, `(0, 0)` when none.
    stuck_ranges: Ranges,
    /// Closed files awaiting their own sync, one slot each (id 0 = free), so
    /// settling one removes only its ranges (REV-071 RV-175). Preallocated:
    /// owing and settling never allocate.
    slot_ids: Box<[u64]>,
    /// `PENDING_FILES` rows of `producers` ranges, one row per slot.
    slot_ranges: Ranges,
    /// Closed files beyond the slots, sharing one hull until all settle.
    overflow: usize,
    overflow_ranges: Ranges,
    next: u64,
}
impl Owed {
    fn producers(&self) -> usize {
        self.stuck_ranges.len()
    }
    fn slot(&mut self, at: usize) -> &mut [(u64, u64)] {
        let n = self.producers();
        &mut self.slot_ranges[at * n..(at + 1) * n]
    }
    fn fold_into_stuck(&mut self, ranges: impl IntoIterator<Item = (u64, u64)>) {
        self.stuck += 1;
        for (range, (first, last)) in self.stuck_ranges.iter_mut().zip(ranges) {
            widen(range, first, last);
        }
    }
    fn release_overflow(&mut self) {
        self.overflow = self.overflow.saturating_sub(1);
        if self.overflow == 0 {
            self.overflow_ranges.fill((0, 0));
        }
    }
}
fn widen(range: &mut (u64, u64), first: u64, last: u64) {
    if first == 0 {
        return;
    }
    *range = if range.0 == 0 {
        (first, last)
    } else {
        (range.0.min(first), range.1.max(last))
    };
}
impl FileLedger {
    pub(crate) fn new(producers: usize) -> Self {
        Self {
            appended: (0..producers).map(|_| Default::default()).collect(),
            owed: crate::sync::warm_mutex(Owed {
                stuck: 0,
                stuck_ranges: vec![(0, 0); producers].into_boxed_slice(),
                slot_ids: vec![0; PENDING_FILES].into_boxed_slice(),
                slot_ranges: vec![(0, 0); PENDING_FILES * producers].into_boxed_slice(),
                overflow: 0,
                overflow_ranges: vec![(0, 0); producers].into_boxed_slice(),
                next: 0,
            }),
        }
    }
    /// Bytes `new` allocates per producer beyond the struct itself (the slot
    /// ids are charged per producer too, which over-reserves).
    pub(crate) const PRODUCER_BYTES: usize = size_of::<std::sync::atomic::AtomicU64>()
        + (2 + PENDING_FILES) * size_of::<(u64, u64)>()
        + PENDING_FILES * size_of::<u64>();
    /// Bytes `new` allocates once per ledger beyond the struct: its warmed
    /// lock on Apple targets (TASK-036 S5).
    pub(crate) const LEDGER_BYTES: usize = crate::sync::MUTEX_BYTES;
    fn owed(&self) -> std::sync::MutexGuard<'_, Owed> {
        // Owed is updated in place without panicking paths; a poisoned
        // guard still holds consistent counts.
        self.owed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(crate) fn appended(&self, producer: usize, seq: u64) {
        if let Some(mark) = self.appended.get(producer) {
            mark.fetch_max(seq, Ordering::AcqRel);
        }
    }
    /// What this target's files appended so far, as a new file's baseline.
    pub(crate) fn base(&self) -> Box<[u64]> {
        self.appended
            .iter()
            .map(|mark| mark.load(Ordering::Acquire))
            .collect()
    }
    /// A file released its writer while holding unsynced records that no
    /// later sync can reach. `first` is each producer's first unsynced
    /// sequence (0 when none), `last` its last appended one.
    pub(crate) fn owe(&self, first: &[u64], last: &[u64]) {
        self.owed()
            .fold_into_stuck(first.iter().copied().zip(last.iter().copied()));
    }
    /// A closed file holding unsynced records that its own later sync may
    /// settle; returns the handle for [`Self::settle`].
    pub(crate) fn owe_until_settled(&self, first: &[u64], last: &[u64]) -> u64 {
        let mut owed = self.owed();
        owed.next += 1;
        let id = owed.next;
        let free = owed.slot_ids.iter().position(|&slot| slot == 0);
        let pairs = first.iter().copied().zip(last.iter().copied());
        match free {
            Some(at) => {
                owed.slot_ids[at] = id;
                for (range, (first, last)) in owed.slot(at).iter_mut().zip(pairs) {
                    *range = if first == 0 { (0, 0) } else { (first, last) };
                }
            }
            None => {
                owed.overflow += 1;
                for (range, (first, last)) in owed.overflow_ranges.iter_mut().zip(pairs) {
                    widen(range, first, last);
                }
            }
        }
        id
    }
    /// The closed file `id` was synced: only its own ranges stop being owed
    /// (an overflowed file's share clears once every overflowed file settled).
    pub(crate) fn settle(&self, id: u64) {
        let mut owed = self.owed();
        match owed.slot_ids.iter().position(|&slot| slot == id) {
            Some(at) => {
                owed.slot_ids[at] = 0;
                owed.slot(at).fill((0, 0));
            }
            None => owed.release_overflow(),
        }
    }
    /// The closed file `id` was dropped unsynced: its ranges stay owed for
    /// good, folded into the stuck hull.
    pub(crate) fn abandon(&self, id: u64) {
        let mut owed = self.owed();
        match owed.slot_ids.iter().position(|&slot| slot == id) {
            Some(at) => {
                owed.slot_ids[at] = 0;
                let n = owed.producers();
                for p in 0..n {
                    let (first, last) = std::mem::take(&mut owed.slot(at)[p]);
                    widen(&mut owed.stuck_ranges[p], first, last);
                }
                owed.stuck += 1;
            }
            None if owed.overflow > 0 => {
                // Its share is not separable: keep the whole overflow hull.
                let hull = std::mem::take(&mut owed.overflow_ranges);
                owed.fold_into_stuck(hull.iter().copied());
                owed.overflow_ranges = hull;
                owed.release_overflow();
            }
            None => {}
        }
    }
    /// The records of `producer` another file may hold unsynced, as seen by a
    /// file created when this ledger's appended marks were `base`.
    pub(crate) fn foreign(&self, producer: usize, base: u64) -> crate::control::Foreign {
        let owed = self.owed();
        let mut hull = (0, 0);
        let mut add = |range: Option<&(u64, u64)>| {
            if let Some(&(first, last)) = range {
                widen(&mut hull, first, last);
            }
        };
        if owed.stuck > 0 {
            add(owed.stuck_ranges.get(producer));
        }
        if owed.overflow > 0 {
            add(owed.overflow_ranges.get(producer));
        }
        let n = owed.producers();
        for (at, &id) in owed.slot_ids.iter().enumerate() {
            if id != 0 {
                add(owed.slot_ranges.get(at * n + producer));
            }
        }
        crate::control::Foreign {
            owed: hull,
            base,
            appended: self
                .appended
                .get(producer)
                .map_or(0, |mark| mark.load(Ordering::Acquire)),
        }
    }
}
/// A closed file's entry in its target's owed ledger: settled by its own
/// sync, otherwise kept owed for good when the file is dropped (RV-175).
pub(crate) struct OwedFile {
    state: Arc<crate::routing::TargetLedger>,
    id: u64,
    settled: bool,
}
impl OwedFile {
    pub(crate) fn new(
        state: &Arc<crate::routing::TargetLedger>,
        first: &[u64],
        last: &[u64],
    ) -> Self {
        Self {
            state: Arc::clone(state),
            id: state.files.owe_until_settled(first, last),
            settled: false,
        }
    }
    pub(crate) fn settle(mut self) {
        self.state.files.settle(self.id);
        self.settled = true;
    }
}
impl Drop for OwedFile {
    fn drop(&mut self) {
        if !self.settled {
            self.state.files.abandon(self.id);
        }
    }
}
#[derive(Clone)]
struct TargetLease {
    _claim: Arc<TargetClaim>,
}
struct TargetClaim(Arc<TargetLedger>);
impl TargetLease {
    fn enter(budget: &WriterBudget) -> Result<Self, StorageError> {
        budget
            .target_state
            .writer_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| StorageError::Busy)?;
        Ok(Self {
            _claim: Arc::new(TargetClaim(Arc::clone(&budget.target_state))),
        })
    }
}
impl Drop for TargetClaim {
    fn drop(&mut self) {
        self.0.writer_active.store(false, Ordering::Release);
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.budget
            .ledger
            .storage_used
            .fetch_sub(self.bytes, Ordering::Release);
    }
}
/// Optional descriptor for a user payload; unknown encodings remain opaque bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadSchema {
    /// Schema descriptor encoding, such as protobuf or a vendor-specific encoding.
    pub encoding: String,
    /// Descriptor bytes, bounded by the configured total metadata limit.
    pub descriptor: Vec<u8>,
    /// Type name standard tools resolve in the descriptor (for protobuf the
    /// fully-qualified message name). Kept in the channel metadata as
    /// `payload_schema_name` (API §21.1 ⑥); at most 256 bytes.
    pub name: Option<String>,
}
/// Immutable mapping from a product channel to a file-local channel.
#[derive(Clone, Debug)]
pub struct StorageChannel {
    /// Product schema identity and version, independent of MCAP's u16 IDs.
    pub channel: ChannelConfig,
    /// Bounded display topic, not a filesystem path.
    pub topic: String,
    /// MCAP message encoding of the original application payload.
    pub payload_encoding: String,
    /// Optional descriptor repeated within each segment.
    pub schema: Option<PayloadSchema>,
}
/// Explicit per-segment limits; no disk retention or capacity defaults are inferred.
#[derive(Clone, Copy, Debug)]
pub struct WriterConfig {
    /// Maximum physical loss-report parts in this file; zero for unavailable collection.
    pub max_loss_reports: usize,
    /// Largest payload, separate from metadata pair overhead.
    pub max_payload_bytes: usize,
    /// Explicit management-path write buffer capacity; zero selects direct IO.
    pub write_buffer_bytes: usize,
    /// Chunk target; a single larger permitted record uses its own chunk.
    /// Loss-report parts are grouped separately within this target, capped at 8 KiB.
    pub chunk_target_bytes: usize,
    /// Hard logical user-record count; each record writes two MCAP messages.
    /// Zero permits only controls when max_loss_reports is positive.
    pub max_records: usize,
    /// Hard message chunk/index count, including loss checkpoints.
    pub max_chunks: usize,
    /// Total caller topic/encoding/descriptor bytes. Native control metadata
    /// is included separately in the fixed portion of the reservation.
    pub max_metadata_bytes: usize,
}
/// Progress retained when a batch cannot finish. Never append its accepted prefix again.
#[derive(Debug)]
pub struct BatchWriteError {
    /// Underlying append, flush or completion-report failure.
    pub error: StorageError,
    /// Prefix accepted by append, possibly still in an incomplete chunk.
    pub appended: usize,
    /// Prefix covered by the successful final batch flush (zero on flush failure).
    pub written: usize,
    /// Prefix whose Written reports succeeded. Retry reports from this index only.
    pub reported: usize,
}
impl std::fmt::Display for BatchWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "batch write failed: {}", self.error)
    }
}
impl std::error::Error for BatchWriteError {}
/// Sync failure that preserves an admitted work guard when IO has not started.
pub struct SyncWorkError<'control, 'owner> {
    /// Validation, coverage, flush or sync error.
    pub error: StorageError,
    /// Retryable ownership on pre-IO rejection; None after an actual IO attempt.
    /// Dropping returned work without retrying explicitly records Unknown.
    pub work: Option<crate::control::SyncWork<'control, 'owner>>,
}
impl std::fmt::Debug for SyncWorkError<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncWorkError")
            .field("error", &self.error)
            .field("work_returned", &self.work.is_some())
            .finish()
    }
}
impl std::fmt::Display for SyncWorkError<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sync work failed: {}", self.error)
    }
}
impl std::error::Error for SyncWorkError<'_, '_> {}
/// Fixed storage failure categories, retaining ordinary OS IO errors.
#[derive(Debug)]
pub enum StorageError {
    /// The file profile is not the native product format.
    UnsupportedFormat,
    /// Invalid, overflowing or unsupported startup limits.
    InvalidConfig,
    /// Record/session/channel/schema/time/length does not match the frozen contract.
    InvalidRecord,
    /// Native framing or metadata is malformed or exceeds decoder limits.
    CorruptData,
    /// Finite writer or index capacity requires rotation before accepting a record.
    NeedRotation,
    /// Shared startup reservation is being changed; retry on the general path.
    Busy,
    /// Insufficient output memory reservation.
    Budget {
        /// Requested writer bytes.
        required: usize,
        /// Currently available bytes.
        available: usize,
    },
    /// An earlier write/flush failure invalidated further appends.
    Poisoned,
    /// Container encoding/decoding failed.
    Container,
    /// File bytes were written, but the retained completion observation failed.
    Completion(crate::completion::CompletionError),
    /// Operating-system IO failed.
    Io(std::io::Error),
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "storage error: {self:?}")
    }
}
impl std::error::Error for StorageError {}
impl From<mcap::McapError> for StorageError {
    fn from(e: mcap::McapError) -> Self {
        match e {
            mcap::McapError::Io(e) => Self::Io(e),
            _ => Self::Container,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod reserve_tests {
    use super::*;
    use crate::routing::{SharedLedger, TargetLedger};

    /// REV-071 RV-175 (C): settling one closed file removes only its own
    /// range; the others it overlapped stay owed until they settle too.
    #[test]
    fn settling_one_owed_file_shrinks_the_owed_range() {
        let ledger = FileLedger::new(1);
        let a = ledger.owe_until_settled(&[1], &[5]);
        let b = ledger.owe_until_settled(&[6], &[15]);
        assert_eq!(ledger.foreign(0, 0).owed, (1, 15));
        ledger.settle(a);
        assert_eq!(ledger.foreign(0, 0).owed, (6, 15));
        ledger.settle(b);
        assert_eq!(ledger.foreign(0, 0).owed, (0, 0));
        // A file dropped unsynced stays owed for good.
        ledger.owe(&[20], &[21]);
        let c = ledger.owe_until_settled(&[30], &[31]);
        ledger.settle(c);
        assert_eq!(ledger.foreign(0, 0).owed, (20, 21));
        // Beyond the preallocated slots, files share one hull that clears
        // once all of them settled; owing and settling never allocate.
        let fresh = FileLedger::new(1);
        let ids: Vec<_> = (0..PENDING_FILES as u64 + 1)
            .map(|i| fresh.owe_until_settled(&[10 * i + 1], &[10 * i + 5]))
            .collect();
        assert_eq!(fresh.foreign(0, 0).owed, (1, 45));
        fresh.settle(ids[0]);
        assert_eq!(fresh.foreign(0, 0).owed, (11, 45));
        for &id in &ids[1..] {
            fresh.settle(id);
        }
        assert_eq!(fresh.foreign(0, 0).owed, (0, 0));
    }

    /// TASK-023 S3 finding ⑩: outputs of one router rotate at the same moment
    /// and reserve their next writer from one shared ledger. A lost CAS race
    /// is not a lack of budget; it must never surface as `Busy`, which made
    /// `SegmentLog::open` fail and poisoned the output.
    #[test]
    fn concurrent_reservations_within_the_limit_never_report_busy() {
        let mut ledger = SharedLedger::default();
        ledger.storage_limit = usize::MAX / 2;
        let budget = WriterBudget {
            ledger: Arc::new(ledger),
            session: 1,
            revision: 1,
            loss: None,
            recovered_file: None,
            target: 1,
            producers: 1,
            target_state: Arc::new(TargetLedger::default()),
        };
        // Miri preempts threads at random points, so a few hundred rounds
        // per seed already lose CAS races; 20,000 took about 30 minutes of
        // the CI Miri job for the same check.
        let rounds = if cfg!(miri) { 300 } else { 20_000 };
        let busy = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let budget = budget.clone();
                    scope.spawn(move || {
                        let mut busy = 0;
                        for _ in 0..rounds {
                            match budget.reserve(1) {
                                Ok(lease) => drop(lease),
                                Err(StorageError::Busy) => busy += 1,
                                Err(e) => panic!("{e:?}"),
                            }
                        }
                        busy
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).sum::<u32>()
        });
        assert_eq!(busy, 0);
        assert_eq!(budget.used_bytes(), 0);
    }
}
