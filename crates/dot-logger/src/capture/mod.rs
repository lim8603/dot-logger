//! Preallocated small/raw ingress, independent of storage and completion.
//!
//! The owner retains both rtrb endpoints. RT handles only borrow them, so even
//! the last handle's Drop cannot free queue memory. No worker is started here.
//! A receipt acknowledges admission. Tracked construction reserves completion
//! metadata before admission; neither form acknowledges dispatch/write/durability.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use crate::completion::{Completion, CompletionConfig, CompletionError, Registry};
use crate::routing::RoutingConfig;

use crate::config::{ConfigError, ImportancePolicy, MemoryBudget, Resource, ValidatedConfig};
use crate::pool::{
    Loan, PoolSnapshot, RawPool, RawRecord, RawRecordError, RawRecordOutcome, ReclaimReport,
    SlotIdentity,
};

mod types;

#[cfg(test)]
mod tests;

pub use types::{
    CaptureBuildError, CaptureError, CaptureReceipt, CapturedRecord, ChannelConfig, Importance,
    ImportanceMapping, Level, ReadError, RecordMetadata, RecordOutcome, SourceTime,
};

#[derive(Clone, Copy)]
struct QueuedRecord {
    record: CapturedRecord,
    raw: Option<SlotIdentity>,
}

struct WriteState {
    records: rtrb::Producer<QueuedRecord>,
    bytes: rtrb::Producer<u8>,
    sequence: u64,
    normal_accepted: usize,
    raw_cursor: usize,
}

struct ReadState {
    records: rtrb::Consumer<QueuedRecord>,
    bytes: rtrb::Consumer<u8>,
    normal_consumed: usize,
    reclaim_cursor: usize,
}

struct Entry {
    write: WriteState,
    read: ReadState,
    consumed: AtomicUsize,
    slot_bytes: usize,
    normal_limit: usize,
    raw: Option<RawPool>,
}

/// Owns preallocated small/raw ingress and its fixed registration metadata.
///
/// Allocate and drop on a management thread. The application must supply a new
/// session ID on restart. Targets, completion windows, workers, and
/// durability are not instantiated by `new`; `new_tracked` adds completion metadata.
pub struct CaptureOwner {
    entries: Box<[Entry]>,
    channels: Box<[ChannelConfig]>,
    closing: AtomicBool,
    session_id: u128,
    revision: u64,
    input_reserved_bytes: usize,
    raw_reserved_bytes: usize,
    control_reserved_bytes: usize,
    completion: Option<Box<Registry>>,
    memory: MemoryBudget,
    telemetry: Option<Box<crate::telemetry::Bank>>,
    telemetry_epoch: u64,
}

impl CaptureOwner {
    /// Validate concrete ingress memory, preallocate, and touch all queue slots.
    ///
    /// The input reservation includes payloads, envelopes, endpoint state, and
    /// rtrb ring descriptors. Owner/channel/raw-slot metadata fits control;
    /// original payload allocations fit the raw partition, counted once.
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
        let mut raw_reserved_bytes = 0usize;
        let mut raw_metadata_bytes = 0usize;
        for producer in &config.producers {
            let slots = producer.small.slots;
            let bytes = slots
                .checked_mul(producer.small.slot_bytes)
                .ok_or_else(overflow)?;
            // rtrb uses positions in 0..2*capacity; Vec uses an isize-sized layout.
            if slots > isize::MAX as usize / size_of::<QueuedRecord>()
                || bytes > isize::MAX as usize
            {
                return Err(overflow());
            }
            let entry_bytes = slots
                .checked_mul(size_of::<QueuedRecord>())
                .and_then(|n| n.checked_add(bytes))
                .and_then(|n| n.checked_add(size_of::<Entry>()))
                .and_then(|n| n.checked_add(size_of::<rtrb::RingBuffer<QueuedRecord>>()))
                .and_then(|n| n.checked_add(size_of::<rtrb::RingBuffer<u8>>()))
                .ok_or_else(overflow)?;
            input_reserved_bytes = input_reserved_bytes
                .checked_add(entry_bytes)
                .ok_or_else(overflow)?;
            if let Some(quota) = &producer.raw {
                let metadata = RawPool::metadata_bytes(quota).ok_or_else(overflow)?;
                if quota.slot_bytes > isize::MAX as usize || metadata > isize::MAX as usize {
                    return Err(overflow());
                }
                raw_metadata_bytes = raw_metadata_bytes
                    .checked_add(metadata)
                    .ok_or_else(overflow)?;
                raw_reserved_bytes = raw_reserved_bytes
                    .checked_add(
                        quota
                            .slots
                            .checked_mul(quota.slot_bytes)
                            .ok_or_else(overflow)?,
                    )
                    .ok_or_else(overflow)?;
            }
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
            .and_then(|n| n.checked_add(raw_metadata_bytes))
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
            let (record_write, record_read) = warm_ring(
                quota.slots,
                QueuedRecord {
                    record: warm_record,
                    raw: None,
                },
            )?;
            let (byte_write, byte_read) = warm_ring(quota.slots * quota.slot_bytes, 0u8)?;
            entries.push(Entry {
                write: WriteState {
                    records: record_write,
                    bytes: byte_write,
                    sequence: 0,
                    normal_accepted: 0,
                    raw_cursor: 0,
                },
                read: ReadState {
                    records: record_read,
                    bytes: byte_read,
                    normal_consumed: 0,
                    reclaim_cursor: 0,
                },
                consumed: AtomicUsize::new(0),
                slot_bytes: quota.slot_bytes,
                normal_limit: match config.importance {
                    ImportancePolicy::PrioritizeImportant => quota.slots - quota.important_slots,
                    ImportancePolicy::TreatEqually => quota.slots,
                },
                raw: producer.raw.as_ref().map(RawPool::new),
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
            raw_reserved_bytes,
            control_reserved_bytes: control_bytes,
            completion: None,
            memory: config.memory.clone(),
            telemetry: None,
            telemetry_epoch: 0,
        })
    }

    /// Construct ingress with preallocated completion admission windows and a
    /// frozen target/route contract. All allocation occurs before producer use.
    /// Completion metadata shares the control partition with capture metadata.
    pub fn new_tracked(
        plan: &ValidatedConfig,
        session_id: u128,
        revision: u64,
        channels: &[ChannelConfig],
        routing: &RoutingConfig,
        config: CompletionConfig,
    ) -> Result<Self, CaptureBuildError> {
        let mut owner = Self::new(plan, session_id, revision, channels)?;
        let available = plan.config().memory.control_bytes - owner.control_reserved_bytes;
        let registry = Registry::new(
            session_id,
            revision,
            &plan.config().producers,
            channels,
            routing,
            config,
            available,
        )
        .map_err(|error| match error {
            CompletionError::Config(error) => CaptureBuildError::Config(error),
            _ => CaptureBuildError::InvalidCompletion,
        })?;
        owner.control_reserved_bytes += registry.reserved();
        owner.completion = Some(Box::new(registry));
        Ok(owner)
    }

    /// Borrow management completion metadata; absent for untracked ingress.
    pub fn completion(&self) -> Option<Completion<'_>> {
        self.completion
            .as_ref()
            .map(|registry| Completion { registry })
    }

    /// Preallocate fixed counters and 301 one-second samples in the remaining
    /// control budget. Explicit opt-in; call before lending any RT handles.
    pub fn enable_telemetry(&mut self) -> Result<(), CaptureBuildError> {
        if self.telemetry.is_some() {
            return Ok(());
        }
        let overflow = || CaptureBuildError::Config(ConfigError::ArithmeticOverflow);
        let epoch = self.telemetry_epoch.checked_add(1).ok_or_else(overflow)?;
        let bytes = crate::telemetry::Bank::required(self.entries.len()).ok_or_else(overflow)?;
        let required = self
            .control_reserved_bytes
            .checked_add(bytes)
            .ok_or_else(overflow)?;
        if required > self.memory.control_bytes {
            return Err(CaptureBuildError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Control,
                required,
                available: self.memory.control_bytes,
            }));
        }
        self.telemetry = Some(Box::new(crate::telemetry::Bank::new(
            self.entries.len(),
            epoch,
        )));
        self.control_reserved_bytes = required;
        self.telemetry_epoch = epoch;
        Ok(())
    }
    /// Explicit management reset after capture/telemetry borrows have ended.
    /// Clears counter history without resetting capture receipt sequences.
    pub fn reset_telemetry(&mut self) -> Result<(), crate::telemetry::TelemetryError> {
        use crate::telemetry::TelemetryError;
        let bank = self.telemetry.as_mut().ok_or(TelemetryError::Disabled)?;
        let epoch = self
            .telemetry_epoch
            .checked_add(1)
            .ok_or(TelemetryError::EpochExhausted)?;
        bank.reset(epoch);
        self.telemetry_epoch = epoch;
        Ok(())
    }
    /// Fixed management observations, absent until explicit enable.
    pub fn telemetry(&self) -> Option<crate::telemetry::Telemetry<'_>> {
        self.telemetry
            .as_deref()
            .map(|bank| crate::telemetry::Telemetry {
                bank,
                session: self.session_id,
                revision: self.revision,
            })
    }

    /// Bytes reserved for concrete input arrays, queue descriptors, and endpoints.
    pub fn input_reserved_bytes(&self) -> usize {
        self.input_reserved_bytes
    }

    /// Physical original-payload storage, counted once including unused slots.
    pub fn raw_reserved_bytes(&self) -> usize {
        self.raw_reserved_bytes
    }

    /// Concrete owner, channel, and original-slot metadata reservation.
    pub fn control_reserved_bytes(&self) -> usize {
        self.control_reserved_bytes
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
            completion: self
                .completion
                .as_ref()
                .map(|registry| Completion { registry }),
            control_reserved_bytes: self.control_reserved_bytes,
            memory: &self.memory,
            telemetry: self.telemetry.as_deref(),
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
    completion: Option<Completion<'owner>>,
    control_reserved_bytes: usize,
    memory: &'owner MemoryBudget,
    telemetry: Option<&'owner crate::telemetry::Bank>,
}

impl<'owner> CaptureScope<'owner> {
    /// Management observations sharing this scope's immutable registration.
    pub fn telemetry(&self) -> Option<crate::telemetry::Telemetry<'owner>> {
        self.telemetry.map(|bank| crate::telemetry::Telemetry {
            bank,
            session: self.session_id,
            revision: self.revision,
        })
    }
    /// Management completion handle sharing the fixed capture registration.
    pub fn completion(&self) -> Option<Completion<'owner>> {
        self.completion
    }

    /// Borrow the ingress close flag for a management thread.
    pub fn control(&self) -> CaptureControl<'owner> {
        CaptureControl {
            closing: self.closing,
            completion: self.completion,
            reserved: self.control_reserved_bytes,
            memory: self.memory,
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
                    raw: entry.raw.as_ref(),
                    completion: self.completion,
                    telemetry: self.telemetry.map(|bank| &bank.cells[id]),
                },
                CaptureConsumer {
                    read: &mut entry.read,
                    consumed: &entry.consumed,
                    raw: entry.raw.as_ref(),
                    completion: self.completion,
                    memory: self.memory,
                    slot_bytes: entry.slot_bytes,
                    channels: self.channels,
                    session_id: self.session_id,
                    producer_id: id as u32,
                    revision: self.revision,
                },
            )
        })
    }
}

/// Borrowed management control for ingress only, without drain/sync semantics.
#[derive(Clone, Copy)]
pub struct CaptureControl<'owner> {
    closing: &'owner AtomicBool,
    pub(crate) completion: Option<Completion<'owner>>,
    pub(crate) reserved: usize,
    pub(crate) memory: &'owner MemoryBudget,
}

impl CaptureControl<'_> {
    /// Close new admission. An overlapping call may already have been admitted.
    /// Existing records stay readable. Use ControlOwner for tracked drain/sync/closure.
    pub fn close(&self) {
        self.closing.store(true, Ordering::SeqCst);
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
    raw: Option<&'owner RawPool>,
    completion: Option<Completion<'owner>>,
    telemetry: Option<&'owner crate::telemetry::CounterCell>,
}

impl<'owner> Producer<'owner> {
    /// Copy one small record using only preallocated storage.
    ///
    /// No clock reads, I/O, callbacks, allocation, deallocation, or retry loops.
    /// The input slice is no longer borrowed when this function returns.
    pub fn try_record_small(
        &mut self,
        metadata: RecordMetadata,
        payload: &[u8],
    ) -> Result<RecordOutcome, CaptureError> {
        let result = self.record_small_inner(metadata, payload);
        if let Some(counter) = self.telemetry {
            match &result {
                Ok(RecordOutcome::Accepted(_)) => counter.record(0, payload.len()),
                Ok(RecordOutcome::Skipped) => counter.record(2, 0),
                Err(error) => counter.reject(*error, false),
            }
        }
        result
    }
    fn record_small_inner(
        &mut self,
        mut metadata: RecordMetadata,
        payload: &[u8],
    ) -> Result<RecordOutcome, CaptureError> {
        let _active = self
            .completion
            .map(|completion| completion.enter(self.producer_id));
        let (importance, skip) = self.classify(&metadata, payload.len(), self.slot_bytes)?;
        if skip {
            return Ok(RecordOutcome::Skipped);
        }
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
        let tracking = self
            .completion
            .map(|completion| {
                completion.reserve(self.producer_id, sequence, metadata.channel_id, importance)
            })
            .transpose()
            .map_err(|_| CaptureError::CompletionCapacity)?;
        let receipt = CaptureReceipt {
            session_id: self.session_id,
            producer_id: self.producer_id,
            accepted_seq: sequence,
        };
        metadata.importance = Some(importance);
        bytes.fill_from_iter(payload.iter().copied());
        if let Some(tracking) = tracking {
            tracking.commit();
        }
        self.write.sequence = sequence;
        if importance == Importance::Normal {
            self.write.normal_accepted = self.write.normal_accepted.wrapping_add(1);
        }
        // rtrb's Release publication of this envelope follows the byte copy.
        // The consumer acquires the envelope before reading the byte queue.
        records.fill_from_iter(std::iter::once(QueuedRecord {
            record: CapturedRecord {
                receipt,
                metadata,
                revision: self.revision,
                payload_len: payload.len(),
            },
            raw: None,
        }));
        Ok(RecordOutcome::Accepted(receipt))
    }

    fn classify(
        &self,
        metadata: &RecordMetadata,
        length: usize,
        maximum: usize,
    ) -> Result<(Importance, bool), CaptureError> {
        if self.closing.load(Ordering::SeqCst) {
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
        if length > maximum {
            return Err(CaptureError::TooLarge);
        }
        let importance = metadata
            .importance
            .unwrap_or_else(|| channel.importance.resolve(metadata.level));
        Ok((importance, metadata.level < channel.minimum_level))
    }

    /// Borrow one fixed-class original slot. At most two finite slot scans;
    /// no allocation, free, callback, I/O, waiting, or CAS retry loop.
    pub fn try_acquire(
        &mut self,
        length: usize,
        importance: Importance,
    ) -> Result<Loan<'owner>, CaptureError> {
        let result = self.acquire_inner(length, importance);
        if let Err(error) = &result {
            if let Some(counter) = self.telemetry {
                counter.reject(*error, true);
            }
        }
        result
    }
    fn acquire_inner(
        &mut self,
        length: usize,
        importance: Importance,
    ) -> Result<Loan<'owner>, CaptureError> {
        // Loan acquisition can overlap closure just like record publication.
        // Quiescent shutdown must observe its final Loaned/return state too.
        let _active = self
            .completion
            .map(|completion| completion.enter(self.producer_id));
        if self.closing.load(Ordering::SeqCst) {
            return Err(CaptureError::Closing);
        }
        self.raw.ok_or(CaptureError::Unsupported)?.acquire(
            length,
            importance,
            &mut self.write.raw_cursor,
        )
    }

    /// Submit an original without copying payload bytes. Failure or intentional
    /// omission returns the exact same loan. Pool/producer/importance mismatches
    /// are InvalidInput, checked before a filter can hide invalid ownership.
    pub fn try_record_raw(
        &mut self,
        metadata: RecordMetadata,
        loan: Loan<'owner>,
    ) -> Result<RawRecordOutcome<'owner>, RawRecordError<'owner>> {
        let length = loan.len();
        let result = self.record_raw_inner(metadata, loan);
        if let Some(counter) = self.telemetry {
            match &result {
                Ok(RawRecordOutcome::Accepted(_)) => counter.record(0, length),
                Ok(RawRecordOutcome::Skipped(_)) => counter.record(2, 0),
                Err(error) => counter.reject(error.reason, false),
            }
        }
        result
    }
    fn record_raw_inner(
        &mut self,
        mut metadata: RecordMetadata,
        loan: Loan<'owner>,
    ) -> Result<RawRecordOutcome<'owner>, RawRecordError<'owner>> {
        let _active = self
            .completion
            .map(|completion| completion.enter(self.producer_id));
        let classify = (|| {
            let (importance, skip) = self.classify(&metadata, loan.len(), usize::MAX)?;
            let pool = self.raw.ok_or(CaptureError::Unsupported)?;
            if !loan.belongs_to(pool) || importance != loan.importance() {
                return Err(CaptureError::InvalidInput);
            }
            Ok((importance, skip))
        })();
        let (importance, skip) = match classify {
            Ok(value) => value,
            Err(reason) => return Err(RawRecordError { reason, loan }),
        };
        if skip {
            return Ok(RawRecordOutcome::Skipped(loan));
        }
        if importance == Importance::Normal
            && self
                .write
                .normal_accepted
                .wrapping_sub(self.consumed.load(Ordering::Acquire))
                >= self.normal_limit
        {
            return Err(RawRecordError {
                reason: CaptureError::Capacity,
                loan,
            });
        }
        let Some(sequence) = self.write.sequence.checked_add(1) else {
            return Err(RawRecordError {
                reason: CaptureError::SequenceExhausted,
                loan,
            });
        };
        let Ok(records) = self.write.records.write_chunk_uninit(1) else {
            return Err(RawRecordError {
                reason: CaptureError::Capacity,
                loan,
            });
        };
        let tracking = match self
            .completion
            .map(|completion| {
                completion.reserve(self.producer_id, sequence, metadata.channel_id, importance)
            })
            .transpose()
        {
            Ok(tracking) => tracking,
            Err(_) => {
                return Err(RawRecordError {
                    reason: CaptureError::CompletionCapacity,
                    loan,
                });
            }
        };
        let receipt = CaptureReceipt {
            session_id: self.session_id,
            producer_id: self.producer_id,
            accepted_seq: sequence,
        };
        let payload_len = loan.len();
        metadata.importance = Some(importance);
        // All fallible reservations finished. Publish ownership before the FIFO
        // envelope; there is no further failure path or payload copy here.
        let identity = loan.publish();
        if let Some(tracking) = tracking {
            tracking.commit();
        }
        self.write.sequence = sequence;
        if importance == Importance::Normal {
            self.write.normal_accepted = self.write.normal_accepted.wrapping_add(1);
        }
        records.fill_from_iter(std::iter::once(QueuedRecord {
            record: CapturedRecord {
                receipt,
                metadata,
                revision: self.revision,
                payload_len,
            },
            raw: Some(identity),
        }));
        Ok(RawRecordOutcome::Accepted(receipt))
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
                Err(CaptureError::Capacity | CaptureError::CompletionCapacity) => {
                    std::thread::yield_now()
                }
                result => return result,
            }
        }
    }
}

/// Single borrowed management consumer, kept separate from any file I/O.
pub struct CaptureConsumer<'owner> {
    read: &'owner mut ReadState,
    consumed: &'owner AtomicUsize,
    raw: Option<&'owner RawPool>,
    completion: Option<Completion<'owner>>,
    memory: &'owner MemoryBudget,
    slot_bytes: usize,
    channels: &'owner [ChannelConfig],
    session_id: u128,
    producer_id: u32,
    revision: u64,
}

pub(crate) struct CaptureRegistration<'owner> {
    pub(crate) session_id: u128,
    pub(crate) producer_id: u32,
    pub(crate) revision: u64,
    pub(crate) channels: &'owner [ChannelConfig],
    pub(crate) small_layout: (usize, usize),
    pub(crate) raw_layout: Option<(usize, usize)>,
    pub(crate) completion: Option<Completion<'owner>>,
    pub(crate) memory: &'owner MemoryBudget,
}

impl<'owner> CaptureConsumer<'owner> {
    /// Inspect the FIFO head without copying payloads or releasing ingress quota.
    /// Used by the management scheduler to check its record and byte budgets.
    pub fn peek(&mut self) -> Result<Option<CapturedRecord>, ReadError> {
        let Ok(records) = self.read.records.read_chunk(1) else {
            return Ok(None);
        };
        Ok(Some(queued_head(&records)?.record))
    }

    pub(crate) fn registration(&self) -> CaptureRegistration<'owner> {
        CaptureRegistration {
            session_id: self.session_id,
            producer_id: self.producer_id,
            revision: self.revision,
            channels: self.channels,
            small_layout: (self.read.records.buffer().capacity(), self.slot_bytes),
            raw_layout: self.raw.map(RawPool::layout),
            completion: self.completion,
            memory: self.memory,
        }
    }

    /// Copy the next small or raw payload into caller-owned management storage.
    /// A short destination consumes nothing. For raw no-copy access, use try_take_raw.
    pub fn try_read(
        &mut self,
        destination: &mut [u8],
    ) -> Result<Option<CapturedRecord>, ReadError> {
        let Ok(records) = self.read.records.read_chunk(1) else {
            return Ok(None);
        };
        let queued = queued_head(&records)?;
        let record = queued.record;
        if destination.len() < record.payload_len {
            return Err(ReadError::BufferTooSmall(record.payload_len));
        }
        if let Some(identity) = queued.raw {
            let raw = self
                .raw
                .and_then(|pool| pool.take(identity, record))
                .ok_or(ReadError::InconsistentQueue)?;
            destination[..record.payload_len].copy_from_slice(raw.payload());
        } else {
            let bytes = self
                .read
                .bytes
                .read_chunk(record.payload_len)
                .map_err(|_| ReadError::InconsistentQueue)?;
            let (first, second) = bytes.as_slices();
            destination[..first.len()].copy_from_slice(first);
            destination[first.len()..record.payload_len].copy_from_slice(second);
            bytes.commit_all();
        }
        // Return bytes before the envelope, then release the Normal ingress quota.
        records.commit_all();
        release_normal(self.read, self.consumed, record);
        Ok(Some(record))
    }

    /// Retain the next raw original without copying its bytes. A small record at
    /// the head returns PayloadKindMismatch and remains queued, preserving FIFO.
    pub fn try_take_raw(&mut self) -> Result<Option<RawRecord<'owner>>, ReadError> {
        let Ok(records) = self.read.records.read_chunk(1) else {
            return Ok(None);
        };
        let queued = queued_head(&records)?;
        let identity = queued.raw.ok_or(ReadError::PayloadKindMismatch)?;
        let raw = self
            .raw
            .and_then(|pool| pool.take(identity, queued.record))
            .ok_or(ReadError::InconsistentQueue)?;
        records.commit_all();
        release_normal(self.read, self.consumed, queued.record);
        Ok(Some(raw))
    }

    pub(crate) fn pending_len(&mut self) -> usize {
        self.read.records.slots()
    }

    /// Sample raw slot lifecycle counts in the management path.
    pub fn raw_snapshot(&self) -> Option<PoolSnapshot> {
        self.raw.map(RawPool::snapshot)
    }

    /// Inspect at most limit raw slots, advancing a fair round-robin cursor.
    /// Notification absence/fullness never prevents the authoritative scan.
    pub fn reclaim_raw(&mut self, limit: usize) -> ReclaimReport {
        self.raw
            .map(|pool| pool.reclaim(limit, &mut self.read.reclaim_cursor))
            .unwrap_or_default()
    }
}

fn queued_head(
    records: &rtrb::chunks::ReadChunk<'_, QueuedRecord>,
) -> Result<QueuedRecord, ReadError> {
    let (first, second) = records.as_slices();
    first
        .iter()
        .chain(second)
        .next()
        .copied()
        .ok_or(ReadError::InconsistentQueue)
}

fn release_normal(read: &mut ReadState, consumed: &AtomicUsize, record: CapturedRecord) {
    if record.metadata.importance == Some(Importance::Normal) {
        read.normal_consumed = read.normal_consumed.wrapping_add(1);
        consumed.store(read.normal_consumed, Ordering::Release);
    }
}
