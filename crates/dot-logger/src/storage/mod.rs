//! Management-path file storage. Payload encoding and file IO never run on RT handles.
mod catalog;
mod codec;
mod index;
pub(crate) mod recovery;
mod retention;
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
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Lease, StorageError> {
        let used = self.used_bytes();
        let next = used.checked_add(bytes).ok_or(StorageError::InvalidConfig)?;
        if next > self.limit_bytes() {
            return Err(StorageError::Budget {
                required: bytes,
                available: self.limit_bytes().saturating_sub(used),
            });
        }
        self.ledger
            .storage_used
            .compare_exchange(used, next, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| StorageError::Busy)?;
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
