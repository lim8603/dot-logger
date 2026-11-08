//! Management-path file storage. Payload encoding and file IO never run on RT handles.
mod catalog;
mod codec;
mod index;
pub(crate) mod recovery;
mod retention;
pub(crate) use retention::{log_required, smallest_rotated_segment};
mod writer;
use crate::capture::ChannelConfig;
use crate::routing::{SharedLedger, TargetLedger};
pub(crate) use catalog::ReadPermit;
pub use catalog::{EventReport, ReadPin, RetentionConfig, RetentionMode, SegmentInfo};
pub use codec::{StoredRecord, decode_record, encode_record};
pub use index::ChunkIndexEntry;
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
pub use writer::{FinishedSegment, SegmentWriter, SyncSink};

/// Opaque reservation authority for one configured target and a shared output partition.
#[derive(Clone)]
pub struct WriterBudget {
    pub(crate) ledger: Arc<SharedLedger>,
    pub(crate) session: u128,
    pub(crate) revision: u64,
    pub(crate) target: u32,
    pub(crate) producers: usize,
    pub(crate) target_state: Arc<TargetLedger>,
}
impl WriterBudget {
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
#[derive(Default)]
struct Owed {
    /// Released files whose appended records were never proven synced.
    files: usize,
    /// Per producer: hull of those files' ranges, `(0, 0)` when none.
    ranges: Box<[(u64, u64)]>,
}
impl FileLedger {
    pub(crate) fn new(producers: usize) -> Self {
        Self {
            appended: (0..producers).map(|_| Default::default()).collect(),
            owed: std::sync::Mutex::new(Owed {
                files: 0,
                ranges: vec![(0, 0); producers].into_boxed_slice(),
            }),
        }
    }
    /// Bytes `new` allocates per producer beyond the struct itself.
    pub(crate) const PRODUCER_BYTES: usize =
        size_of::<std::sync::atomic::AtomicU64>() + size_of::<(u64, u64)>();
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
    /// A file released its writer while holding unsynced records.
    pub(crate) fn owe(&self, first: &[u64], last: &[u64]) {
        let mut owed = self.owed();
        owed.files += 1;
        for (range, (&first, &last)) in owed.ranges.iter_mut().zip(first.iter().zip(last)) {
            if first == 0 {
                continue;
            }
            *range = if range.0 == 0 {
                (first, last)
            } else {
                (range.0.min(first), range.1.max(last))
            };
        }
    }
    /// A released file that owed its records was synced.
    pub(crate) fn settle(&self) {
        let mut owed = self.owed();
        owed.files = owed.files.saturating_sub(1);
        if owed.files == 0 {
            owed.ranges.fill((0, 0));
        }
    }
    /// The records of `producer` another file may hold unsynced, as seen by a
    /// file created when this ledger's appended marks were `base`.
    pub(crate) fn foreign(&self, producer: usize, base: u64) -> crate::control::Foreign {
        let owed = self.owed();
        crate::control::Foreign {
            owed: if owed.files == 0 {
                (0, 0)
            } else {
                owed.ranges.get(producer).copied().unwrap_or((0, 0))
            },
            base,
            appended: self
                .appended
                .get(producer)
                .map_or(0, |mark| mark.load(Ordering::Acquire)),
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
}
/// Immutable mapping from a product channel to a file-local channel.
#[derive(Clone, Debug)]
pub struct StorageChannel {
    /// Product schema identity and version, independent of MCAP's u16 IDs.
    pub channel: ChannelConfig,
    /// Bounded display topic, not a filesystem path.
    pub topic: String,
    /// User payload encoding inside the product envelope.
    pub payload_encoding: String,
    /// Optional descriptor repeated within each segment.
    pub schema: Option<PayloadSchema>,
}
/// Explicit per-segment limits; no disk retention or capacity defaults are inferred.
#[derive(Clone, Copy, Debug)]
pub struct WriterConfig {
    /// Largest payload, separate from envelope overhead.
    pub max_payload_bytes: usize,
    /// Explicit management-path write buffer capacity; zero selects direct IO.
    pub write_buffer_bytes: usize,
    /// Chunk target; a single larger permitted record uses its own chunk.
    pub chunk_target_bytes: usize,
    /// Hard segment message count; reaching it requires a new segment.
    pub max_records: usize,
    /// Hard chunk/index count, including a metadata-only flushed chunk.
    pub max_chunks: usize,
    /// Total input topic/encoding/descriptor metadata bytes.
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
    /// Invalid, overflowing or unsupported startup limits.
    InvalidConfig,
    /// Record/session/channel/schema/time/length does not match the frozen contract.
    InvalidRecord,
    /// Encoded envelope is malformed or exceeds decoder limits.
    CorruptData,
    /// The envelope major version is unsupported.
    UnsupportedVersion(u32),
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
            target: 1,
            producers: 1,
            target_state: Arc::new(TargetLedger::default()),
        };
        let busy = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let budget = budget.clone();
                    scope.spawn(move || {
                        let mut busy = 0;
                        for _ in 0..20_000 {
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
