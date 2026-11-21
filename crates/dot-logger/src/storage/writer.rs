use super::*;
use crate::capture::CapturedRecord;
use crate::completion::{Boundary, Update};
use crate::control::{SyncResult, SyncWork};
use crate::routing::Delivery;
use std::collections::BTreeMap;
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, AtomicU64};

/// A seekable OS/adapter sink with explicit synchronization.
pub trait SyncSink: Write + Seek {
    /// Confirm an actual adapter synchronization, separate from Write::flush.
    fn sync_all(&mut self) -> std::io::Result<()>;
}
impl SyncSink for std::fs::File {
    fn sync_all(&mut self) -> std::io::Result<()> {
        std::fs::File::sync_all(self)
    }
}
struct Signals {
    position: AtomicU64,
    sync: AtomicBool,
}
struct Sink<W> {
    inner: W,
    buffer: Vec<u8>,
    sent: usize,
    signals: Arc<Signals>,
}
impl<W: SyncSink> Sink<W> {
    fn drain_buffer(&mut self) -> std::io::Result<()> {
        // Preserve partial progress on an IO error. Drop only releases memory;
        // it never retries a write or flushes abandoned bytes.
        while self.sent < self.buffer.len() {
            match self.inner.write(&self.buffer[self.sent..]) {
                Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
                Ok(n) => self.sent += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        self.buffer.clear();
        self.sent = 0;
        Ok(())
    }
}
impl<W: SyncSink> Write for Sink<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = if bytes.len() >= self.buffer.capacity() {
            self.drain_buffer()?;
            self.inner.write(bytes)?
        } else {
            if self.buffer.len() + bytes.len() > self.buffer.capacity() {
                self.drain_buffer()?;
            }
            self.buffer.extend_from_slice(bytes);
            bytes.len()
        };
        let pos = self
            .signals
            .position
            .load(Ordering::Relaxed)
            .checked_add(count as u64)
            .ok_or_else(|| std::io::Error::other("file offset overflow"))?;
        self.signals.position.store(pos, Ordering::Release);
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let sync = self.signals.sync.swap(false, Ordering::AcqRel);
        self.drain_buffer()?;
        self.inner.flush()?;
        if sync {
            self.inner.sync_all()?;
        }
        Ok(())
    }
}
impl<W: SyncSink> Seek for Sink<W> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.drain_buffer()?;
        let pos = self.inner.seek(pos)?;
        self.signals.position.store(pos, Ordering::Release);
        Ok(pos)
    }
    fn stream_position(&mut self) -> std::io::Result<u64> {
        // Includes accepted buffered bytes, so position probes need no IO.
        Ok(self.signals.position.load(Ordering::Acquire))
    }
}
/// `base` is what the target's files had appended when this file started.
pub(super) fn check_sync(
    budget: &WriterBudget,
    work: &SyncWork<'_, '_>,
    first: &[u64],
    last: &[u64],
    base: &[u64],
) -> Result<(), StorageError> {
    if work.target_id() != budget.target || work.token().session_id != budget.session {
        return Err(StorageError::InvalidRecord);
    }
    let files = &budget.target_state.files;
    let foreign = |p: usize| files.foreign(p, base.get(p).copied().unwrap_or(0));
    match work.covers_file(first, last, foreign) {
        Ok(true) => Ok(()),
        Err(
            crate::control::ControlError::Busy
            | crate::control::ControlError::Completion(crate::completion::CompletionError::Busy),
        ) => Err(StorageError::Busy),
        _ => Err(StorageError::InvalidRecord),
    }
}
pub(super) fn report_written(delivery: &Delivery<'_>) -> Result<(), StorageError> {
    delivery
        .report(Update::Confirmed(Boundary::Written))
        .map_err(|e| match e {
            crate::completion::CompletionError::Busy => StorageError::Busy,
            _ => StorageError::Completion(e),
        })
}
// Shared by the native reader and writer so self-described registrations cannot
// bypass reserved names, product identity uniqueness or descriptor consistency.
pub(super) fn registration_bytes(channels: &[StorageChannel]) -> Result<usize, StorageError> {
    let mut metadata = 0usize;
    for (i, c) in channels.iter().enumerate() {
        if c.topic.len() > 256
            || c.payload_encoding.len() > 64
            || c.topic.is_empty()
            || matches!(c.topic.as_str(), codec::META_TOPIC | codec::LOSS_TOPIC)
            || c.payload_encoding.is_empty()
            || channels[..i]
                .iter()
                .any(|p| p.channel.channel_id == c.channel.channel_id)
        {
            return Err(StorageError::InvalidConfig);
        }
        if c.schema.as_ref().is_some_and(|s| {
            s.name
                .as_deref()
                .is_some_and(|n| matches!(n, codec::META_SCHEMA | "dot_logger.LossReport"))
                || s.encoding.is_empty()
                || s.encoding.len() > 64
                || s.name
                    .as_ref()
                    .is_some_and(|n| n.is_empty() || n.len() > 256)
        }) {
            return Err(StorageError::InvalidConfig);
        }
        if channels[..i].iter().any(|p| {
            p.channel.schema_id == c.channel.schema_id
                && p.channel.schema_version == c.channel.schema_version
                && p.schema != c.schema
        }) {
            return Err(StorageError::InvalidConfig);
        }
        for bytes in [
            c.topic.len(),
            c.payload_encoding.len(),
            c.schema.as_ref().map_or(0, |s| s.encoding.len()),
            c.schema.as_ref().map_or(0, |s| s.descriptor.len()),
            c.schema
                .as_ref()
                .and_then(|s| s.name.as_ref())
                .map_or(0, String::len),
        ] {
            metadata = metadata
                .checked_add(bytes)
                .ok_or(StorageError::InvalidConfig)?;
        }
    }
    Ok(metadata)
}
// The smallest nonempty native file needs two message indexes, one chunk,
// and one application-channel/chunk auxiliary entry in the conservative model.
pub(super) const MIN_INDEX_BYTES: usize = 128 + 512 + 512;
const LOSS_MESSAGE_BOUND: usize = crate::loss::ENCODED_REPORT_BOUND + 31;

pub(super) fn loss_chunks(parts: usize, chunk_target_bytes: usize) -> usize {
    parts.div_ceil(loss_parts_per_chunk(chunk_target_bytes))
}

fn loss_parts_per_chunk(chunk_target_bytes: usize) -> usize {
    // Keep these control chunks within the existing report decoder's scale,
    // even when the writer uses a much larger target for application payloads.
    (chunk_target_bytes.min(crate::loss::MAX_REPORT_BYTES) / LOSS_MESSAGE_BOUND).max(1)
}

impl WriterConfig {
    pub(crate) fn validate_loss(&self, rows: usize) -> Result<(), StorageError> {
        let parts = rows.div_ceil(crate::loss::ROWS_PER_REPORT);
        if rows > 0
            && (self.max_loss_reports < parts.checked_mul(2).ok_or(StorageError::InvalidConfig)?
                || self.max_chunks
                    < loss_chunks(parts, self.chunk_target_bytes)
                        .checked_mul(2)
                        .and_then(|n| n.checked_add(2))
                        .ok_or(StorageError::InvalidConfig)?)
        {
            return Err(StorageError::InvalidConfig);
        }
        Ok(())
    }

    fn index_entries(&self, channels: usize) -> usize {
        // Each nonempty channel/chunk pair requires at least one message.
        self.max_chunks
            .saturating_mul(channels)
            .min(self.max_records)
    }
    /// Conservative requested-byte reservation for the pinned uncompressed MCAP
    /// implementation, including encoder, maps, Vec growth and summary clones.
    /// Sink-owned buffers, allocator bookkeeping and OS page cache are separate.
    pub fn reservation(
        &self,
        channels: &[StorageChannel],
        producers: usize,
    ) -> Result<usize, StorageError> {
        if self.max_payload_bytes == 0
            || self.chunk_target_bytes == 0
            || self.max_records == 0 && self.max_loss_reports == 0
            || self.max_chunks == 0
            || self.max_records > u32::MAX as usize
            || self.max_chunks > u32::MAX as usize
            || channels.is_empty()
            || channels.len() > u16::MAX as usize - 2
            || self.max_metadata_bytes == 0
            || self.max_metadata_bytes > u32::MAX as usize
            || producers == 0
        {
            return Err(StorageError::InvalidConfig);
        }
        if self.max_loss_reports > u32::MAX as usize {
            return Err(StorageError::InvalidConfig);
        }
        let metadata = registration_bytes(channels)?;
        if metadata > self.max_metadata_bytes {
            return Err(StorageError::InvalidConfig);
        }
        let max = self
            .max_payload_bytes
            .checked_add(codec::OVERHEAD)
            .ok_or(StorageError::InvalidConfig)?;
        let mut total = 65536usize;
        // Direct seeking avoids an accumulating chunk payload buffer. Reserve an
        // extra max-record allowance; chunk target is not a hard memory cap.
        for (n, size) in [
            (2, max),
            (self.write_buffer_bytes, 1),
            (self.max_records, 128),
            (self.max_loss_reports, 64),
            (
                usize::from(self.max_loss_reports > 0),
                8 * crate::loss::MAX_REPORT_BYTES + 16384,
            ),
            (self.max_chunks, 512),
            (self.index_entries(channels.len()), 512),
            (channels.len() + 1, 8192),
            (self.max_metadata_bytes, 16),
            (producers, 64),
        ] {
            total = total
                .checked_add(n.checked_mul(size).ok_or(StorageError::InvalidConfig)?)
                .ok_or(StorageError::InvalidConfig)?;
        }
        if total > isize::MAX as usize {
            return Err(StorageError::InvalidConfig);
        }
        Ok(total)
    }
}
/// Exclusive general-path owner of one bounded uncompressed segment.
/// Drop releases resources without claiming write/sync success or writing a footer.
pub struct SegmentWriter<W: SyncSink> {
    writer: Option<mcap::Writer<Sink<W>>>,
    signals: Arc<Signals>,
    config: WriterConfig,
    channels: Box<[StorageChannel]>,
    current: Box<[Option<ChunkIndexEntry>]>,
    index: Vec<ChunkIndexEntry>,
    last: Box<[u64]>,
    first: Box<[u64]>,
    /// The target's appended marks when this file started.
    base: Box<[u64]>,
    /// Appended records not covered by a successful sync of this file.
    dirty: bool,
    /// Owns its target claim; a SegmentLog accounts for its own files.
    standalone: bool,
    encoder: Vec<u8>,
    meta_channel: u16,
    loss_channel: u16,
    loss_parts: usize,
    reserved_final_parts: usize,
    reserved_final_chunks: usize,
    loss_count: usize,
    pending_loss_parts: usize,
    sample_start: u64,
    deferred: bool,
    records: usize,
    chunks: usize,
    pending_records: usize,
    pending_bytes: usize,
    chunk_start: u64,
    open: bool,
    safe: u64,
    poisoned: bool,
    _target: TargetLease,
    // Last: allocations are released before the budget lease.
    lease: Option<Lease>,
}
/// Closed container; its index reservation remains live until this owner is dropped.
pub struct FinishedSegment<W: SyncSink> {
    sink: W,
    index: Vec<ChunkIndexEntry>,
    safe: u64,
    first: Box<[u64]>,
    last: Box<[u64]>,
    base: Box<[u64]>,
    /// Counted in the target's owed files until synced.
    owes: bool,
    _lease: Lease,
}
impl<W: SyncSink> FinishedSegment<W> {
    pub(super) fn sync_recovered(&mut self) -> Result<(), StorageError> {
        self.sink
            .flush()
            .and_then(|()| self.sink.sync_all())
            .map_err(StorageError::Io)?;
        self.settle();
        Ok(())
    }
    fn settle(&mut self) {
        if std::mem::take(&mut self.owes) {
            self._lease.budget.target_state.files.settle();
        }
    }
    /// Retained auxiliary index.
    pub fn index(&self) -> &[ChunkIndexEntry] {
        &self.index
    }
    /// End of the complete file, without an implicit durability acknowledgment.
    pub fn safe_offset(&self) -> u64 {
        self.safe
    }
    /// Return the sink, releasing logger-owned metadata reservations.
    pub fn into_inner(self) -> W {
        self.sink
    }
    /// Synchronize the closed file including its summary/footer for a matching
    /// admitted barrier. Other segments require separate coordinated handling.
    /// Validation/Busy returns the original work in SyncWorkError::work; retry
    /// that guard on the correct file. An IO failure acknowledges Failed and
    /// returns no work. Discarding returned work records Unknown.
    pub fn sync_work<'c, 'o>(
        &mut self,
        work: SyncWork<'c, 'o>,
    ) -> Result<(), SyncWorkError<'c, 'o>> {
        if let Err(error) = check_sync(
            &self._lease.budget,
            &work,
            &self.first,
            &self.last,
            &self.base,
        ) {
            return Err(SyncWorkError {
                error,
                work: Some(work),
            });
        }
        let result = self.sink.flush().and_then(|()| self.sink.sync_all());
        if result.is_ok() {
            self.settle();
        }
        work.complete(if result.is_ok() {
            SyncResult::Synced
        } else {
            SyncResult::Failed
        });
        result.map_err(|e| SyncWorkError {
            error: StorageError::Io(e),
            work: None,
        })
    }
}
impl<W: SyncSink> SegmentWriter<W> {
    /// Validate and reserve before emitting any file bytes. The sink must be empty
    /// and positioned at zero. Custom sink buffering is the adapter's own budget.
    pub fn new(
        budget: WriterBudget,
        config: WriterConfig,
        channels: Vec<StorageChannel>,
        sink: W,
    ) -> Result<Self, StorageError> {
        config.reservation(&channels, budget.producers)?;
        config.validate_loss(budget.loss_rows())?;
        let target_lease = TargetLease::enter(&budget)?;
        Self::with_target(budget, config, channels, sink, target_lease, true, 0)
    }
    /// `standalone` writers record their unsynced records in the target's
    /// file ledger when released; a SegmentLog does that for its own files.
    pub(super) fn with_target(
        budget: WriterBudget,
        config: WriterConfig,
        channels: Vec<StorageChannel>,
        mut sink: W,
        target_lease: TargetLease,
        standalone: bool,
        sequence: u64,
    ) -> Result<Self, StorageError> {
        let bytes = config.reservation(&channels, budget.producers)?;
        let loss_rows = budget.loss_rows();
        let loss_parts = loss_rows.div_ceil(crate::loss::ROWS_PER_REPORT);
        let reserved_final_parts = if budget.loss.is_some() { loss_parts } else { 0 };
        if budget.recovered_file.is_some() {
            if loss_rows > 0 && config.max_loss_reports == 0 {
                return Err(StorageError::NeedRotation);
            }
        } else {
            config.validate_loss(loss_rows)?;
        }
        let lease = budget.reserve(bytes)?;
        if sink.stream_position().map_err(StorageError::Io)? != 0
            || sink.seek(SeekFrom::End(0)).map_err(StorageError::Io)? != 0
        {
            return Err(StorageError::InvalidConfig);
        }
        sink.seek(SeekFrom::Start(0)).map_err(StorageError::Io)?;
        // Normalize caller spare capacities before retaining the registrations.
        let channels: Box<[_]> = channels
            .into_iter()
            .map(|mut c| {
                c.topic = c.topic.into_boxed_str().into_string();
                c.payload_encoding = c.payload_encoding.into_boxed_str().into_string();
                if let Some(s) = &mut c.schema {
                    s.encoding = std::mem::take(&mut s.encoding)
                        .into_boxed_str()
                        .into_string();
                    s.descriptor = std::mem::take(&mut s.descriptor)
                        .into_boxed_slice()
                        .into_vec();
                    s.name = s.name.take().map(|n| n.into_boxed_str().into_string());
                }
                c
            })
            .collect();
        let signals = Arc::new(Signals {
            position: AtomicU64::new(0),
            sync: AtomicBool::new(false),
        });
        let mut writer = mcap::WriteOptions::new()
            .compression(None)
            .disable_seeking(false)
            .chunk_size(None)
            .chunk_registrations(false)
            .profile(codec::PROFILE)
            .library("dot-logger/0.1.0")
            .calculate_chunk_crcs(true)
            .calculate_data_section_crc(true)
            .calculate_summary_section_crc(true)
            .emit_message_indexes(true)
            .emit_chunk_indexes(true)
            .create(Sink {
                inner: sink,
                buffer: Vec::with_capacity(config.write_buffer_bytes),
                sent: 0,
                signals: Arc::clone(&signals),
            })?;
        writer.write_metadata(
            &super::FileInfo {
                loss_rows,
                session: budget.session,
                revision: budget.revision,
                target: budget
                    .recovered_file
                    .map_or(budget.target, |file| file.target),
                sequence,
                producers: budget.producers,
                channels: channels.len(),
            }
            .metadata(),
        )?;
        let meta_schema =
            writer.add_schema(codec::META_SCHEMA, "protobuf", codec::META_DESCRIPTOR)?;
        let meta_channel = (channels.len() + 1) as u16;
        writer.add_channel_with_id(
            meta_channel,
            meta_schema,
            codec::META_TOPIC,
            "protobuf",
            &BTreeMap::new(),
        )?;
        let loss_channel = (channels.len() + 2) as u16;
        if loss_rows > 0 {
            let schema =
                writer.add_schema(crate::loss::SCHEMA, "protobuf", crate::loss::DESCRIPTOR)?;
            writer.add_channel_with_id(
                loss_channel,
                schema,
                codec::LOSS_TOPIC,
                "protobuf",
                &BTreeMap::new(),
            )?;
        }
        for (i, c) in channels.iter().enumerate() {
            let mut metadata = BTreeMap::from([
                ("session_id".into(), format!("{:032x}", budget.session)),
                ("revision".into(), budget.revision.to_string()),
                ("product_channel".into(), c.channel.channel_id.to_string()),
                (
                    "payload_schema_id".into(),
                    format!("{:032x}", c.channel.schema_id),
                ),
                (
                    "payload_schema_version".into(),
                    c.channel.schema_version.to_string(),
                ),
                ("payload_encoding".into(), c.payload_encoding.clone()),
            ]);
            let schema = if let Some(d) = &c.schema {
                let fallback = format!(
                    "payload.{:032x}.v{}",
                    c.channel.schema_id, c.channel.schema_version
                );
                if let Some(name) = &d.name {
                    metadata.insert("payload_schema_name".into(), name.clone());
                }
                writer.add_schema(
                    d.name.as_deref().unwrap_or(&fallback),
                    &d.encoding,
                    &d.descriptor,
                )?
            } else {
                0
            };
            writer.add_channel_with_id(
                (i + 1) as u16,
                schema,
                &c.topic,
                &c.payload_encoding,
                &metadata,
            )?;
        }
        let count = channels.len();
        let mut result = Self {
            writer: Some(writer),
            signals,
            config,
            channels,
            current: vec![None; count].into_boxed_slice(),
            index: Vec::with_capacity(config.index_entries(count)),
            last: vec![0; budget.producers].into_boxed_slice(),
            first: vec![0; budget.producers].into_boxed_slice(),
            base: budget.target_state.files.base(),
            dirty: false,
            standalone,
            encoder: Vec::with_capacity(codec::OVERHEAD),
            meta_channel,
            loss_channel,
            loss_parts,
            reserved_final_parts,
            reserved_final_chunks: loss_chunks(reserved_final_parts, config.chunk_target_bytes),
            loss_count: 0,
            pending_loss_parts: 0,
            sample_start: 0,
            deferred: false,
            records: 0,
            chunks: 0,
            pending_records: 0,
            pending_bytes: 0,
            chunk_start: 0,
            open: false,
            safe: 0,
            poisoned: false,
            _target: target_lease,
            lease: Some(lease),
        };
        result.collect_loss(crate::loss::Coverage::Baseline, false)?;
        Ok(result)
    }
    /// Conservative bytes reserved until this writer or its finished index is dropped.
    pub fn reserved_bytes(&self) -> usize {
        self.lease.as_ref().map_or(0, |l| l.bytes)
    }
    /// Encode and append without reporting durability. Invalid input and capacity
    /// rejection leave the file untouched; an IO failure poisons further appends.
    pub fn append(&mut self, record: CapturedRecord, payload: &[u8]) -> Result<(), StorageError> {
        if self.poisoned {
            return Err(StorageError::Poisoned);
        }
        let budget = &self.lease.as_ref().ok_or(StorageError::Poisoned)?.budget;
        let p = record.receipt.producer_id as usize;
        let channel = self
            .channels
            .iter()
            .position(|c| c.channel.channel_id == record.metadata.channel_id)
            .ok_or(StorageError::InvalidRecord)?;
        let c = &self.channels[channel].channel;
        if record.receipt.session_id != budget.session
            || record.revision != budget.revision
            || p >= self.last.len()
            || record.receipt.accepted_seq <= self.last[p]
            || record.metadata.schema_id != c.schema_id
            || record.metadata.schema_version != c.schema_version
        {
            return Err(StorageError::InvalidRecord);
        }
        if self.records == self.config.max_records
            || (!self.open
                && self.chunks.saturating_add(1 + self.reserved_final_chunks)
                    > self.config.max_chunks)
        {
            return Err(StorageError::NeedRotation);
        }
        codec::validate_record(record, payload, self.config.max_payload_bytes)?;
        codec::encode_meta(record, (channel + 1) as u16, &mut self.encoder)?;
        let bytes = payload.len() + self.encoder.len() + 62;
        let split = self.pending_loss_parts > 0
            || (self.pending_records > 0
                && self.pending_bytes.saturating_add(bytes) > self.config.chunk_target_bytes);
        if split
            && self.chunks.saturating_add(2 + self.reserved_final_chunks) > self.config.max_chunks
        {
            return Err(StorageError::NeedRotation);
        }
        if split {
            self.flush_chunk()?;
        }
        if !self.open {
            self.chunk_start = self.signals.position.load(Ordering::Acquire);
        }
        let header = mcap::records::MessageHeader {
            channel_id: (channel + 1) as u16,
            sequence: record.receipt.accepted_seq as u32,
            log_time: record.metadata.real_ns as u64,
            publish_time: record.metadata.real_ns as u64,
        };
        let writer = self.writer.as_mut().ok_or(StorageError::Poisoned)?;
        let result = writer
            .write_to_known_channel(&header, payload)
            .and_then(|()| {
                writer.write_to_known_channel(
                    &mcap::records::MessageHeader {
                        channel_id: self.meta_channel,
                        ..header
                    },
                    &self.encoder,
                )
            });
        if let Err(e) = result {
            self.poisoned = true;
            return Err(e.into());
        }
        self.open = true;
        self.records += 1;
        self.pending_records += 1;
        self.pending_bytes += bytes;
        self.last[p] = record.receipt.accepted_seq;
        if self.first[p] == 0 {
            self.first[p] = record.receipt.accepted_seq;
        }
        self.dirty = true;
        if let Some(lease) = &self.lease {
            let files = &lease.budget.target_state.files;
            files.appended(p, record.receipt.accepted_seq);
        }
        let entry = self.current[channel].get_or_insert(ChunkIndexEntry {
            channel_id: record.metadata.channel_id,
            mono_min: record.metadata.mono_ns,
            mono_max: record.metadata.mono_ns,
            real_min: record.metadata.real_ns,
            real_max: record.metadata.real_ns,
            chunk_offset: self.chunk_start,
            record_count: 0,
        });
        entry.mono_min = entry.mono_min.min(record.metadata.mono_ns);
        entry.mono_max = entry.mono_max.max(record.metadata.mono_ns);
        entry.real_min = entry.real_min.min(record.metadata.real_ns);
        entry.real_max = entry.real_max.max(record.metadata.real_ns);
        entry.record_count += 1;
        Ok(())
    }
    /// Collect and flush a complete bounded cumulative checkpoint, when enabled.
    /// NeedRotation preserves all counters for a later file/coalesced observation.
    pub fn checkpoint(&mut self) -> Result<(), StorageError> {
        let coverage = if self.deferred {
            crate::loss::Coverage::Coalesced
        } else {
            crate::loss::Coverage::Observed
        };
        self.collect_loss(coverage, false)
    }
    pub(super) fn defer_loss(&mut self) {
        self.deferred = true;
    }
    fn collect_loss(
        &mut self,
        coverage: crate::loss::Coverage,
        final_sample: bool,
    ) -> Result<(), StorageError> {
        let budget = &self.lease.as_ref().ok_or(StorageError::Poisoned)?.budget;
        let Some(bank) = budget.loss.clone() else {
            return Ok(());
        };
        let target = budget.target;
        let parts = bank.parts();
        let reserve = if final_sample { 0 } else { parts };
        let chunks = loss_chunks(parts, self.config.chunk_target_bytes);
        if self
            .loss_count
            .checked_add(parts)
            .and_then(|n| n.checked_add(reserve))
            .is_none_or(|n| n > self.config.max_loss_reports)
            || self
                .chunks
                .checked_add(usize::from(self.open))
                .and_then(|n| n.checked_add(chunks))
                .and_then(|n| n.checked_add(if final_sample { 0 } else { chunks }))
                .is_none_or(|n| n > self.config.max_chunks)
        {
            self.deferred = true;
            return Err(StorageError::NeedRotation);
        }
        let started = bank.now();
        let checkpoint = bank
            .next_checkpoint(target)
            .ok_or(StorageError::InvalidRecord)?;
        // Version-2 parts list only nonzero rows; the reserved dense part
        // count above remains the upper bound.
        let from = if coverage == crate::loss::Coverage::Baseline {
            started
        } else {
            self.sample_start
        };
        let mut start = 0;
        for part in 0..parts {
            let mut report = bank.sparse(target, checkpoint, part, start, from, coverage);
            report.coalesced = self.deferred;
            self.append_loss_report(&report)?;
            start = report.range.map_or(bank.len(), |r| r.end as usize);
            if start == bank.len() {
                break;
            }
        }
        self.flush_chunk()?;
        self.sample_start = started;
        self.deferred = false;
        Ok(())
    }
    // Simulate output chunk splits before recovery admits any part of a source
    // chunk. Re-encode metadata because source protobuf widths need not be canonical.
    pub(super) fn recovery_chunk_fits(
        &mut self,
        data: &[u8],
        state: &super::recovery::Validation<'_>,
    ) -> Result<bool, StorageError> {
        use super::recovery::Fields;
        let mut nested = Fields(data);
        let mut chunks = self.chunks;
        let mut pending = self.pending_records;
        let mut pending_loss = self.pending_loss_parts;
        let parts_per_chunk = loss_parts_per_chunk(self.config.chunk_target_bytes);
        let mut bytes = self.pending_bytes;
        let mut controls = self.loss_count;
        let mut records = self.records;
        while !nested.0.is_empty() {
            let (opcode, record) = nested.record().map_err(|_| StorageError::CorruptData)?;
            if opcode != 5 {
                continue;
            }
            if state
                .loss_report(record)
                .map_err(|_| StorageError::CorruptData)?
                .is_some()
            {
                if pending > 0 || pending_loss == parts_per_chunk {
                    chunks = chunks.saturating_add(1);
                    pending_loss = 0;
                }
                controls = controls.saturating_add(1);
                pending = 0;
                bytes = 0;
                pending_loss += 1;
            } else {
                let (_, meta) = nested.record().map_err(|_| StorageError::CorruptData)?;
                let stored = state
                    .pair(record, meta)
                    .map_err(|_| StorageError::CorruptData)?;
                let mut fields = Fields(record);
                let channel = fields.u16().map_err(|_| StorageError::CorruptData)?;
                codec::encode_meta(stored.record, channel, &mut self.encoder)?;
                let size = stored.payload.len() + self.encoder.len() + 62;
                if pending_loss > 0
                    || (pending > 0 && bytes.saturating_add(size) > self.config.chunk_target_bytes)
                {
                    chunks = chunks.saturating_add(1);
                    pending_loss = 0;
                    pending = 0;
                    bytes = 0;
                }
                pending += 1;
                bytes = bytes.saturating_add(size);
                records = records.saturating_add(1);
            }
            if chunks.saturating_add(usize::from(pending > 0 || pending_loss > 0))
                > self.config.max_chunks
                || controls > self.config.max_loss_reports
                || records > self.config.max_records
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(super) fn append_loss_report(
        &mut self,
        report: &crate::loss::LossReport,
    ) -> Result<(), StorageError> {
        if self.poisoned {
            return Err(StorageError::Poisoned);
        }
        let split = self.pending_records > 0
            || self.pending_loss_parts == loss_parts_per_chunk(self.config.chunk_target_bytes);
        if self.loss_parts == 0
            || self.loss_count >= self.config.max_loss_reports
            || self.chunks + usize::from(split) + 1 > self.config.max_chunks
        {
            return Err(StorageError::NeedRotation);
        }
        report.encode(&mut self.encoder)?;
        if split {
            self.flush_chunk()?;
        }
        let header = mcap::records::MessageHeader {
            channel_id: self.loss_channel,
            sequence: report.checkpoint as u32,
            log_time: report.publish_time_unix_ns,
            publish_time: report.publish_time_unix_ns,
        };
        let result = self
            .writer
            .as_mut()
            .ok_or(StorageError::Poisoned)?
            .write_to_known_channel(&header, &self.encoder);
        if let Err(error) = result {
            self.poisoned = true;
            return Err(error.into());
        }
        self.open = true;
        self.loss_count += 1;
        self.pending_loss_parts += 1;
        Ok(())
    }

    /// Last complete chunk flushed to the adapter write boundary, not disk durability.
    pub fn safe_offset(&self) -> u64 {
        self.safe
    }
    /// Complete-chunk auxiliary entries; open-chunk bounds are never exposed.
    pub fn index(&self) -> &[ChunkIndexEntry] {
        &self.index
    }
    /// Finish the current chunk and publish its safe offset only after flush succeeds.
    pub fn flush_chunk(&mut self) -> Result<u64, StorageError> {
        if self.poisoned {
            return Err(StorageError::Poisoned);
        }
        let result = self.writer.as_mut().ok_or(StorageError::Poisoned)?.flush();
        if let Err(e) = result {
            self.poisoned = true;
            return Err(e.into());
        }
        if self.open {
            for entry in &mut self.current {
                if let Some(entry) = entry.take() {
                    self.index.push(entry);
                }
            }
            self.chunks += 1;
            self.open = false;
            self.pending_records = 0;
            self.pending_loss_parts = 0;
            self.pending_bytes = 0;
        }
        self.safe = self.signals.position.load(Ordering::Acquire);
        Ok(self.safe)
    }
    // Include both this observation and the already reserved final sample.
    pub(super) fn projected_loss_index_bytes(&self) -> usize {
        let reports = self.loss_parts.saturating_add(self.reserved_final_parts);
        let control_chunks = loss_chunks(self.loss_parts, self.config.chunk_target_bytes)
            .saturating_add(self.reserved_final_chunks);
        self.records
            .saturating_mul(128)
            .saturating_add(self.loss_count.saturating_add(reports).saturating_mul(64))
            .saturating_add(
                self.chunks
                    .saturating_add(usize::from(self.open))
                    .saturating_add(control_chunks)
                    .saturating_mul(512),
            )
            .saturating_add(
                self.index
                    .len()
                    .saturating_add(self.current.iter().filter(|e| e.is_some()).count())
                    .saturating_mul(512),
            )
    }
    pub(super) fn projected_index_bytes(
        &self,
        record: CapturedRecord,
        payload_len: usize,
    ) -> usize {
        let bytes = payload_len
            .saturating_add(codec::OVERHEAD)
            .saturating_add(62);
        // Use the maximum metadata width, so a possible split is counted even
        // when the actual pair would fit the current chunk.
        // Same split rule as `append`: a record never joins an open loss chunk.
        let split = self.pending_loss_parts > 0
            || (self.pending_records > 0
                && self.pending_bytes.saturating_add(bytes) > self.config.chunk_target_bytes);
        let new_pair = split
            || self
                .channels
                .iter()
                .position(|c| c.channel.channel_id == record.metadata.channel_id)
                .is_none_or(|i| self.current[i].is_none());
        let pairs = self
            .index
            .len()
            .saturating_add(self.current.iter().filter(|e| e.is_some()).count())
            .saturating_add(usize::from(new_pair));
        self.records
            .saturating_add(1)
            .saturating_mul(128)
            .saturating_add(
                self.loss_count
                    .saturating_add(self.reserved_final_parts)
                    .saturating_mul(64),
            )
            .saturating_add(
                self.chunks
                    .saturating_add(1 + usize::from(split) + self.reserved_final_chunks)
                    .saturating_mul(512),
            )
            .saturating_add(pairs.saturating_mul(512))
    }
    pub(super) fn synchronize(&mut self) -> Result<(), StorageError> {
        self.signals.sync.store(true, Ordering::Release);
        let result = self.flush_chunk();
        self.signals.sync.store(false, Ordering::Release);
        result.map(|_| ())
    }
    /// Write a routing guard and flush its complete chunk before reporting Written.
    /// Prefer write_batch for throughput; this method explicitly flushes one record.
    /// On Busy the bytes are already written: retry only
    /// `delivery.report(Update::Confirmed(Boundary::Written))` with the same guard.
    /// Calling write_delivery again would append twice and is rejected by sequence.
    pub fn write_delivery(&mut self, delivery: &Delivery<'_>) -> Result<(), StorageError> {
        if delivery.target_id()
            != self
                .lease
                .as_ref()
                .ok_or(StorageError::Poisoned)?
                .budget
                .target
        {
            return Err(StorageError::InvalidRecord);
        }
        self.append(delivery.record(), delivery.payload())?;
        self.flush_chunk()?;
        report_written(delivery)
    }
    /// Append caller-owned guards using the chunk target, flush the final chunk
    /// once, then report Written in slice order. An empty batch performs no IO.
    /// The caller bounds batch size/latency and retains guards through reporting.
    ///
    /// On append failure, no reports are made: `error.appended` records were
    /// accepted and must not be appended again. Flush explicitly before reporting
    /// that prefix. IO failure poisons the writer; never claim Written for it.
    /// After a successful flush, `written` equals the batch length; on report Busy,
    /// retry only `Delivery::report(Update::Confirmed(Boundary::Written))` for
    /// `deliveries[error.reported..error.written]`. Do not retry file writes.
    pub fn write_batch(&mut self, deliveries: &[&Delivery<'_>]) -> Result<(), BatchWriteError> {
        if deliveries.is_empty() {
            return Ok(());
        }
        let mut progress = BatchWriteError {
            error: StorageError::Poisoned,
            appended: 0,
            written: 0,
            reported: 0,
        };
        let target = self.lease.as_ref().ok_or(progress)?.budget.target;
        progress = BatchWriteError {
            error: StorageError::InvalidRecord,
            appended: 0,
            written: 0,
            reported: 0,
        };
        for (i, delivery) in deliveries.iter().enumerate() {
            if delivery.target_id() != target {
                return Err(progress);
            }
            if let Err(error) = self.append(delivery.record(), delivery.payload()) {
                progress.error = error;
                return Err(progress);
            }
            progress.appended = i + 1;
        }
        if let Err(error) = self.flush_chunk() {
            progress.error = error;
            return Err(progress);
        }
        progress.written = deliveries.len();
        for (i, delivery) in deliveries.iter().enumerate() {
            if let Err(error) = report_written(delivery) {
                progress.error = error;
                return Err(progress);
            }
            progress.reported = i + 1;
        }
        Ok(())
    }
    /// Synchronize the actual sink for an admitted control barrier and acknowledge
    /// its result. ControlOwner validates preceding guards and cutoffs first.
    /// Pre-IO validation/Busy/Poisoned returns ownership in SyncWorkError::work.
    /// Retry that work on a matching healthy segment. Actual flush/sync failure
    /// acknowledges Failed and returns no work; dropping rejected work is Unknown.
    pub fn sync_work<'c, 'o>(
        &mut self,
        work: SyncWork<'c, 'o>,
    ) -> Result<(), SyncWorkError<'c, 'o>> {
        let check = self
            .lease
            .as_ref()
            .ok_or(StorageError::Poisoned)
            .and_then(|lease| check_sync(&lease.budget, &work, &self.first, &self.last, &self.base))
            .and({
                if self.poisoned {
                    Err(StorageError::Poisoned)
                } else {
                    Ok(())
                }
            });
        if let Err(error) = check {
            return Err(SyncWorkError {
                error,
                work: Some(work),
            });
        }
        self.signals.sync.store(true, Ordering::Release);
        let result = self.flush_chunk();
        self.signals.sync.store(false, Ordering::Release);
        if result.is_ok() {
            self.dirty = false;
        }
        work.complete(if result.is_ok() {
            SyncResult::Synced
        } else {
            SyncResult::Failed
        });
        result
            .map(|_| ())
            .map_err(|error| SyncWorkError { error, work: None })
    }
    /// Write the standard summary/footer; finish alone does not synchronize storage.
    pub fn finish(mut self) -> Result<FinishedSegment<W>, StorageError> {
        let coverage = if self
            .lease
            .as_ref()
            .and_then(|l| l.budget.loss.as_ref())
            .is_some_and(|bank| bank.quiescent())
        {
            crate::loss::Coverage::Quiescent
        } else {
            crate::loss::Coverage::FinalPartial
        };
        self.collect_loss(coverage, true)?;
        self.flush_chunk()?;
        let mut writer = self.writer.take().ok_or(StorageError::Poisoned)?;
        if let Err(e) = writer.finish() {
            drop(writer.into_inner());
            return Err(e.into());
        }
        let mut sink = writer.into_inner();
        sink.flush().map_err(StorageError::Io)?;
        let safe = self.signals.position.load(Ordering::Acquire);
        let lease = self.lease.take().ok_or(StorageError::Poisoned)?;
        // Releasing the target claim: a later file of this target must not
        // stand in for these records until this file is synced.
        let owes = self.standalone && std::mem::take(&mut self.dirty);
        if owes {
            lease.budget.target_state.files.owe(&self.first, &self.last);
        }
        Ok(FinishedSegment {
            sink: sink.inner,
            index: std::mem::take(&mut self.index),
            safe,
            first: std::mem::take(&mut self.first),
            last: std::mem::take(&mut self.last),
            base: std::mem::take(&mut self.base),
            owes,
            _lease: lease,
        })
    }
}
impl<W: SyncSink> Drop for SegmentWriter<W> {
    fn drop(&mut self) {
        // Extracting suppresses MCAP's usual Drop-time footer IO. Only explicit
        // finish/sync methods produce completion observations.
        if let Some(writer) = self.writer.take() {
            drop(writer.into_inner());
        }
        // Unsynced records stay owed: no later file stands in for them.
        if self.standalone && self.dirty {
            if let Some(lease) = &self.lease {
                lease.budget.target_state.files.owe(&self.first, &self.last);
            }
        }
    }
}
