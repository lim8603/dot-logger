//! Preallocated small-record ingress, independent of storage and completion.
//!
//! The owner retains both rtrb endpoints. RT handles only borrow them, so even
//! the last handle's Drop cannot free queue memory. No worker is started here.
//! A capture receipt acknowledges ingress only; it cannot be used as evidence
//! of dispatch, completion tracking, file writes, or durability.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use crate::config::{ConfigError, ImportancePolicy, Resource, ValidatedConfig};

mod types;

#[cfg(test)]
mod tests;

pub use types::{
    CaptureBuildError, CaptureError, CaptureReceipt, CapturedRecord, ChannelConfig, Importance,
    ImportanceMapping, Level, ReadError, RecordMetadata, RecordOutcome, SourceTime,
};

struct WriteState {
    records: rtrb::Producer<CapturedRecord>,
    bytes: rtrb::Producer<u8>,
    sequence: u64,
    normal_accepted: usize,
}

struct ReadState {
    records: rtrb::Consumer<CapturedRecord>,
    bytes: rtrb::Consumer<u8>,
    normal_consumed: usize,
}

struct Entry {
    write: WriteState,
    read: ReadState,
    consumed: AtomicUsize,
    slot_bytes: usize,
    normal_limit: usize,
}

/// Owns only preallocated small ingress and its fixed registration metadata.
///
/// Allocate and drop on a management thread. The application must supply a new
/// session ID on restart. Raw quotas, targets, completion windows, workers, and
/// durability are not instantiated by this stage-specific constructor.
pub struct CaptureOwner {
    entries: Box<[Entry]>,
    channels: Box<[ChannelConfig]>,
    closing: AtomicBool,
    session_id: u128,
    revision: u64,
    input_reserved_bytes: usize,
}

impl CaptureOwner {
    /// Validate concrete ingress memory, preallocate, and touch all queue slots.
    ///
    /// The input reservation includes payloads, envelopes, endpoint state, and
    /// rtrb ring descriptors. Owner/channel metadata fits the control partition.
    /// Allocator bookkeeping and rounding remain in configured worker headroom.
    /// Touching storage does not lock pages into RAM or prove RT latency.
    pub fn new(
        plan: &ValidatedConfig,
        session_id: u128,
        revision: u64,
        channels: &[ChannelConfig],
    ) -> Result<Self, CaptureBuildError> {
        let config = plan.config();
        if channels.is_empty()
            || channels.iter().enumerate().any(|(i, channel)| {
                channels[..i]
                    .iter()
                    .any(|other| other.channel_id == channel.channel_id)
            })
        {
            return Err(CaptureBuildError::InvalidChannels);
        }
        if u32::try_from(config.producers.len() - 1).is_err() {
            return Err(CaptureBuildError::TooManyProducers);
        }
        let overflow = || CaptureBuildError::Config(ConfigError::ArithmeticOverflow);
        let mut input_reserved_bytes = 0usize;
        for producer in &config.producers {
            let slots = producer.small.slots;
            let bytes = slots
                .checked_mul(producer.small.slot_bytes)
                .ok_or_else(overflow)?;
            // rtrb uses positions in 0..2*capacity; Vec uses an isize-sized layout.
            if slots > isize::MAX as usize / size_of::<CapturedRecord>()
                || bytes > isize::MAX as usize
            {
                return Err(overflow());
            }
            let entry_bytes = slots
                .checked_mul(size_of::<CapturedRecord>())
                .and_then(|n| n.checked_add(bytes))
                .and_then(|n| n.checked_add(size_of::<Entry>()))
                .and_then(|n| n.checked_add(size_of::<rtrb::RingBuffer<CapturedRecord>>()))
                .and_then(|n| n.checked_add(size_of::<rtrb::RingBuffer<u8>>()))
                .ok_or_else(overflow)?;
            input_reserved_bytes = input_reserved_bytes
                .checked_add(entry_bytes)
                .ok_or_else(overflow)?;
        }
        if input_reserved_bytes > config.memory.input_bytes {
            return Err(CaptureBuildError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Input,
                required: input_reserved_bytes,
                available: config.memory.input_bytes,
            }));
        }
        let control_bytes = channels
            .len()
            .checked_mul(size_of::<ChannelConfig>())
            .and_then(|n| n.checked_add(size_of::<Self>()))
            .ok_or_else(overflow)?;
        if control_bytes > config.memory.control_bytes {
            return Err(CaptureBuildError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Control,
                required: control_bytes,
                available: config.memory.control_bytes,
            }));
        }
        let warm_record = CapturedRecord {
            receipt: CaptureReceipt {
                session_id,
                producer_id: 0,
                accepted_seq: 0,
            },
            metadata: RecordMetadata {
                channel_id: channels[0].channel_id,
                schema_id: channels[0].schema_id,
                schema_version: channels[0].schema_version,
                mono_ns: 0,
                real_ns: 0,
                source_time: None,
                level: Level::Trace,
                importance: Some(Importance::Normal),
            },
            revision,
            payload_len: 0,
        };
        let mut entries = Vec::with_capacity(config.producers.len());
        for producer in &config.producers {
            let quota = &producer.small;
            let (record_write, record_read) = warm_ring(quota.slots, warm_record)?;
            let (byte_write, byte_read) = warm_ring(quota.slots * quota.slot_bytes, 0u8)?;
            entries.push(Entry {
                write: WriteState {
                    records: record_write,
                    bytes: byte_write,
                    sequence: 0,
                    normal_accepted: 0,
                },
                read: ReadState {
                    records: record_read,
                    bytes: byte_read,
                    normal_consumed: 0,
                },
                consumed: AtomicUsize::new(0),
                slot_bytes: quota.slot_bytes,
                normal_limit: match config.importance {
                    ImportancePolicy::PrioritizeImportant => quota.slots - quota.important_slots,
                    ImportancePolicy::TreatEqually => quota.slots,
                },
            });
        }
        let entries = entries.into_boxed_slice();
        Ok(Self {
            entries,
            channels: channels.into(),
            closing: AtomicBool::new(false),
            session_id,
            revision,
            input_reserved_bytes,
        })
    }

    /// Bytes reserved for concrete input arrays, queue descriptors, and endpoints.
    pub fn input_reserved_bytes(&self) -> usize {
        self.input_reserved_bytes
    }

    /// Lend unique producer/consumer pairs without moving their owning endpoints.
    ///
    /// Allocate/register before this call; create RT threads using scoped threads.
    /// The scope and its handles cannot escape this management borrow.
    ///
    /// ```compile_fail
    /// use dot_logger::capture::CaptureOwner;
    /// fn escape(owner: &mut CaptureOwner) {
    ///     let producer = owner.scope(|mut scope| scope.next().unwrap().0);
    ///     drop(producer);
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use dot_logger::capture::CaptureOwner;
    /// fn drop_while_borrowed(mut owner: CaptureOwner) {
    ///     owner.scope(|mut scope| {
    ///         let (producer, _) = scope.next().unwrap();
    ///         drop(owner);
    ///         drop(producer);
    ///     });
    /// }
    /// ```
    pub fn scope<R>(&mut self, callback: impl FnOnce(CaptureScope<'_>) -> R) -> R {
        callback(CaptureScope {
            entries: self.entries.iter_mut().enumerate(),
            channels: &self.channels,
            closing: &self.closing,
            session_id: self.session_id,
            revision: self.revision,
        })
    }
}

fn warm_ring<T: Copy>(
    capacity: usize,
    value: T,
) -> Result<(rtrb::Producer<T>, rtrb::Consumer<T>), CaptureBuildError> {
    let (mut writer, mut reader) = rtrb::RingBuffer::new(capacity);
    writer
        .write_chunk_uninit(capacity)
        .map_err(|_| CaptureBuildError::InitializationFailed)?
        .fill_from_iter(std::iter::repeat_n(value, capacity));
    reader
        .read_chunk(capacity)
        .map_err(|_| CaptureBuildError::InitializationFailed)?
        .commit_all();
    Ok((writer, reader))
}

/// Management-created iterator over uniquely borrowed producer/consumer pairs.
pub struct CaptureScope<'owner> {
    entries: std::iter::Enumerate<std::slice::IterMut<'owner, Entry>>,
    channels: &'owner [ChannelConfig],
    closing: &'owner AtomicBool,
    session_id: u128,
    revision: u64,
}

impl<'owner> CaptureScope<'owner> {
    /// Borrow the ingress close flag for a management thread.
    pub fn control(&self) -> CaptureControl<'owner> {
        CaptureControl {
            closing: self.closing,
        }
    }
}

impl<'owner> Iterator for CaptureScope<'owner> {
    type Item = (Producer<'owner>, CaptureConsumer<'owner>);

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(|(id, entry)| {
            (
                Producer {
                    write: &mut entry.write,
                    consumed: &entry.consumed,
                    channels: self.channels,
                    closing: self.closing,
                    slot_bytes: entry.slot_bytes,
                    normal_limit: entry.normal_limit,
                    session_id: self.session_id,
                    producer_id: id as u32,
                    revision: self.revision,
                },
                CaptureConsumer {
                    read: &mut entry.read,
                    consumed: &entry.consumed,
                },
            )
        })
    }
}

/// Borrowed management control for ingress only, without drain/sync semantics.
#[derive(Clone, Copy)]
pub struct CaptureControl<'owner> {
    closing: &'owner AtomicBool,
}

impl CaptureControl<'_> {
    /// Close new admission. An overlapping call may already have been admitted.
    /// Existing records stay readable. Full shutdown/join belongs to TASK-006.
    pub fn close(&self) {
        self.closing.store(true, Ordering::Release);
    }
}

/// Unique, non-cloneable RT handle. Drop only releases borrows.
///
/// ```compile_fail
/// use dot_logger::capture::Producer;
/// fn clone_handle(producer: Producer<'_>) { let _ = producer.clone(); }
/// ```
///
/// ```compile_fail
/// use dot_logger::capture::Producer;
/// fn require_sync<T: Sync>() {}
/// require_sync::<Producer<'static>>();
/// ```
pub struct Producer<'owner> {
    write: &'owner mut WriteState,
    consumed: &'owner AtomicUsize,
    channels: &'owner [ChannelConfig],
    closing: &'owner AtomicBool,
    slot_bytes: usize,
    normal_limit: usize,
    session_id: u128,
    producer_id: u32,
    revision: u64,
}

impl<'owner> Producer<'owner> {
    /// Copy one small record using only preallocated storage.
    ///
    /// No clock reads, I/O, callbacks, allocation, deallocation, or retry loops.
    /// The input slice is no longer borrowed when this function returns.
    pub fn try_record_small(
        &mut self,
        mut metadata: RecordMetadata,
        payload: &[u8],
    ) -> Result<RecordOutcome, CaptureError> {
        if self.closing.load(Ordering::Acquire) {
            return Err(CaptureError::Closing);
        }
        let channel = self
            .channels
            .iter()
            .find(|channel| channel.channel_id == metadata.channel_id)
            .ok_or(CaptureError::InvalidInput)?;
        if metadata.schema_id != channel.schema_id
            || metadata.schema_version != channel.schema_version
        {
            return Err(CaptureError::SchemaMismatch);
        }
        if payload.len() > self.slot_bytes {
            return Err(CaptureError::TooLarge);
        }
        if metadata.level < channel.minimum_level {
            return Ok(RecordOutcome::Skipped);
        }
        let importance = metadata
            .importance
            .unwrap_or_else(|| channel.importance.resolve(metadata.level));
        if importance == Importance::Normal {
            let consumed = self.consumed.load(Ordering::Acquire);
            // Only this producer increments accepted and only its consumer
            // increments consumed. Outstanding records never exceed the quota;
            // wrapping subtraction remains valid across usize counter rollover.
            if self.write.normal_accepted.wrapping_sub(consumed) >= self.normal_limit {
                return Err(CaptureError::Capacity);
            }
        }
        let sequence = self
            .write
            .sequence
            .checked_add(1)
            .ok_or(CaptureError::SequenceExhausted)?;
        // Reserve both queues without publishing either. A byte reservation
        // failure drops an uncommitted envelope reservation, with no side effect.
        let records = self
            .write
            .records
            .write_chunk_uninit(1)
            .map_err(|_| CaptureError::Capacity)?;
        let bytes = self
            .write
            .bytes
            .write_chunk_uninit(payload.len())
            .map_err(|_| CaptureError::Capacity)?;
        let receipt = CaptureReceipt {
            session_id: self.session_id,
            producer_id: self.producer_id,
            accepted_seq: sequence,
        };
        metadata.importance = Some(importance);
        bytes.fill_from_iter(payload.iter().copied());
        self.write.sequence = sequence;
        if importance == Importance::Normal {
            self.write.normal_accepted = self.write.normal_accepted.wrapping_add(1);
        }
        // rtrb's Release publication of this envelope follows the byte copy.
        // The consumer acquires the envelope before reading the byte queue.
        records.fill_from_iter(std::iter::once(CapturedRecord {
            receipt,
            metadata,
            revision: self.revision,
            payload_len: payload.len(),
        }));
        Ok(RecordOutcome::Accepted(receipt))
    }

    /// Move this borrow into an explicitly general-path handle with deadline waits.
    pub fn into_general(self) -> GeneralProducer<'owner> {
        GeneralProducer { producer: self }
    }
}

/// General-path producer; its waiting API never appears on the RT handle.
pub struct GeneralProducer<'owner> {
    producer: Producer<'owner>,
}

impl GeneralProducer<'_> {
    /// Retry capacity failures until admission, closure, or monotonic deadline.
    /// Checks the deadline before every attempt, including the first.
    pub fn record_until(
        &mut self,
        metadata: RecordMetadata,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<RecordOutcome, CaptureError> {
        loop {
            if self.producer.closing.load(Ordering::Acquire) {
                return Err(CaptureError::Closing);
            }
            if Instant::now() >= deadline {
                return Err(CaptureError::DeadlineExceeded);
            }
            match self.producer.try_record_small(metadata, payload) {
                Err(CaptureError::Capacity) => std::thread::yield_now(),
                result => return result,
            }
        }
    }
}

/// Single borrowed management consumer, kept separate from any file I/O.
pub struct CaptureConsumer<'owner> {
    read: &'owner mut ReadState,
    consumed: &'owner AtomicUsize,
}

impl CaptureConsumer<'_> {
    /// Copy the oldest accepted record into a caller-owned buffer.
    /// Does not consume anything if the destination is too small.
    pub fn try_read(
        &mut self,
        destination: &mut [u8],
    ) -> Result<Option<CapturedRecord>, ReadError> {
        let Ok(records) = self.read.records.read_chunk(1) else {
            return Ok(None);
        };
        let (first, second) = records.as_slices();
        let record = first
            .iter()
            .chain(second)
            .next()
            .copied()
            .ok_or(ReadError::InconsistentQueue)?;
        if destination.len() < record.payload_len {
            return Err(ReadError::BufferTooSmall(record.payload_len));
        }
        let bytes = self
            .read
            .bytes
            .read_chunk(record.payload_len)
            .map_err(|_| ReadError::InconsistentQueue)?;
        let (first, second) = bytes.as_slices();
        destination[..first.len()].copy_from_slice(first);
        destination[first.len()..record.payload_len].copy_from_slice(second);
        bytes.commit_all();
        // Release bytes before the envelope: observing a free envelope also
        // establishes that the corresponding byte capacity has been returned.
        records.commit_all();
        if record.metadata.importance == Some(Importance::Normal) {
            self.read.normal_consumed = self.read.normal_consumed.wrapping_add(1);
            self.consumed
                .store(self.read.normal_consumed, Ordering::Release);
        }
        Ok(Some(record))
    }
}
