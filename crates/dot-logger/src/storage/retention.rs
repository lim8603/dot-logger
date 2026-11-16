use super::catalog::Catalog;
use super::*;
use crate::capture::CapturedRecord;
use crate::control::{SyncResult, SyncWork};
use crate::routing::Delivery;
use std::io;
use std::path::{Path, PathBuf};

/// Host-owned general-path file operations. Implementations must not overwrite
/// existing files, and must keep one exclusive session/target namespace.
/// The logger only removes closed sequences it published in its own catalog.
pub trait SegmentStore {
    /// Seekable file/adapter sink; logger buffering is separately reserved.
    type Sink: SyncSink;
    /// Whether directory metadata synchronization is implemented by this adapter.
    fn directory_sync_supported(&self) -> bool;
    /// Fresh observed filesystem free bytes. Concurrent external writes can race it.
    fn available_bytes(&mut self) -> io::Result<u64>;
    /// Create a new empty sequence.active file, refusing existing paths.
    fn create(&mut self, sequence: u64) -> io::Result<Self::Sink>;
    /// Rename the synced, closed active file to sequence.mcap without replacement.
    fn finalize(&mut self, sequence: u64) -> io::Result<()>;
    /// Synchronize directory metadata; unsupported operations must return an error.
    fn sync_directory(&mut self) -> io::Result<()>;
    /// Remove only the named previously published closed file.
    fn remove(&mut self, sequence: u64) -> io::Result<()>;
}
/// Standard filesystem store in a fresh system-named session/target directory.
/// The host supplies an actual free-space probe; there is no invented capacity.
/// This adapter currently requires Unix directory synchronization. Other hosts
/// can implement SegmentStore with their own capability-aware primitives.
pub struct FileStore<F: FnMut() -> io::Result<u64>> {
    directory: PathBuf,
    free: F,
}
fn sync_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(io::ErrorKind::Unsupported.into())
    }
}
impl<F: FnMut() -> io::Result<u64>> FileStore<F> {
    /// Create a fresh target namespace below an existing trusted base directory.
    /// Existing target directories are rejected; startup/recovery is TASK-009.
    /// An existing session path must be a real directory, not a symbolic link:
    /// the metadata check rejects symlink traversal into a different namespace.
    /// The host must keep the trusted base and session namespace exclusively
    /// controlled during use; this check does not prevent concurrent path replacement.
    pub fn new(base: &Path, budget: &WriterBudget, free: F) -> io::Result<Self> {
        if !cfg!(unix) {
            return Err(io::ErrorKind::Unsupported.into());
        }
        let session = base.join(format!("{:032x}", budget.session));
        match std::fs::create_dir(&session) {
            Ok(()) => {}
            Err(e)
                if e.kind() == io::ErrorKind::AlreadyExists
                    && std::fs::symlink_metadata(&session)?.file_type().is_dir() => {}
            Err(e) => return Err(e),
        }
        sync_dir(base)?;
        let directory = session.join(budget.target.to_string());
        std::fs::create_dir(&directory)?;
        sync_dir(&session)?;
        sync_dir(&directory)?;
        Ok(Self { directory, free })
    }
    /// Owned namespace containing only system-generated segment filenames.
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}
impl<F: FnMut() -> io::Result<u64>> SegmentStore for FileStore<F> {
    type Sink = std::fs::File;
    fn directory_sync_supported(&self) -> bool {
        cfg!(unix)
    }
    fn available_bytes(&mut self) -> io::Result<u64> {
        (self.free)()
    }
    fn create(&mut self, seq: u64) -> io::Result<Self::Sink> {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(self.directory.join(format!("{seq}.active")))
    }
    fn finalize(&mut self, seq: u64) -> io::Result<()> {
        let dest = self.directory.join(format!("{seq}.mcap"));
        if dest.try_exists()? {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        // The namespace is exclusively owned by this adapter/host; external
        // mutation of generated filenames is outside the SegmentStore contract.
        std::fs::rename(self.directory.join(format!("{seq}.active")), dest)
    }
    fn sync_directory(&mut self) -> io::Result<()> {
        sync_dir(&self.directory)
    }
    fn remove(&mut self, seq: u64) -> io::Result<()> {
        std::fs::remove_file(self.directory.join(format!("{seq}.mcap")))
    }
}
/// Fixed general-path retention failures; file IO errors preserve the OS cause.
#[derive(Debug)]
pub enum RetentionError {
    /// Invalid, overflowing, or undersized explicit configuration.
    InvalidConfig,
    /// Record identity/schema/sequence/length is invalid.
    InvalidRecord,
    /// Host management time moved backwards.
    TimeBackwards,
    /// Disk reserve, retention bytes/age, or catalog count cannot be satisfied.
    Capacity,
    /// Event slots or unique protected-byte quota is exhausted.
    EventQuota,
    /// Pin count, byte, or lifetime limit is exhausted.
    PinLimit,
    /// Further reads on a pin have expired; release still ends deletion protection.
    Expired,
    /// An event/pin/file identity does not belong to this live catalog.
    UnknownToken,
    /// Adapter cannot synchronize directory metadata; never treated as success.
    UnsupportedDirectorySync,
    /// An earlier IO/publication failure stopped this owner.
    Poisoned,
    /// Underlying single-segment storage error.
    Storage(StorageError),
    /// Filesystem/adapter error.
    Io(io::Error),
}
impl std::fmt::Display for RetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "retention error: {self:?}")
    }
}
impl std::error::Error for RetentionError {}
impl From<StorageError> for RetentionError {
    fn from(e: StorageError) -> Self {
        Self::Storage(e)
    }
}
impl From<io::Error> for RetentionError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
/// Retention batch failure with accepted, flushed, and reported prefixes.
/// A partial append rejection never authorizes replaying accepted file writes.
#[derive(Debug)]
pub struct RetentionBatchError {
    /// Capacity/protection/IO/completion cause, preserved without collapsing categories.
    pub error: RetentionError,
    /// Accepted prefix, possibly spanning already closed files.
    pub appended: usize,
    /// Prefix guaranteed by the successful final batch flush; zero on earlier failure.
    pub written: usize,
    /// Prefix with successful Written completion reports.
    pub reported: usize,
}
impl std::fmt::Display for RetentionBatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "retention batch error: {}", self.error)
    }
}
impl std::error::Error for RetentionBatchError {}
/// Limit responsible for rotating before accepting the next message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RotationReason {
    /// Conservative encoded bytes including summary/footer.
    Bytes,
    /// Host monotonic segment age.
    Time,
    /// Per-segment message count.
    Records,
    /// Chunk/index count from the bounded MCAP writer.
    Index,
}
/// Accepted file location and optional preceding rotation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppendReport {
    /// Sequence that accepted the record.
    pub segment: u64,
    /// Limit that forced a rotation before this append.
    pub rotated: Option<RotationReason>,
}
struct Active {
    sequence: u64,
    opened: u64,
    records: usize,
    upper: u64,
    range: Option<(u64, u64)>,
    visible: Option<(u64, u64)>,
}
/// Exclusive management-path owner of a finite rotating file set.
/// Feed target deliveries in producer FIFO order; never skip a target delivery
/// and then claim its sync barrier. All time arguments share a host/session
/// monotonic nanosecond domain; record timestamps may still arrive out of order.
/// Drop performs no footer, rename, deletion, or synchronization IO.
pub struct SegmentLog<S: SegmentStore> {
    store: S,
    writer: Option<SegmentWriter<S::Sink>>,
    active: Option<Active>,
    config: WriterConfig,
    channels: Vec<StorageChannel>,
    first: Box<[u64]>,
    last: Box<[u64]>,
    /// The target's appended marks when this log started.
    base: Box<[u64]>,
    /// Appended records not yet covered by a sync of their file.
    dirty: bool,
    now: u64,
    next: u64,
    poisoned: bool,
    sealed: bool,
    budget: WriterBudget,
    target: TargetLease,
    // Last: retained logger allocations are released before their reservation.
    catalog: Catalog,
}
impl<S: SegmentStore> Drop for SegmentLog<S> {
    fn drop(&mut self) {
        // Unsynced records stay owed: no later file of this target stands in
        // for them (REV-052 RV-128).
        if self.dirty {
            self.budget.target_state.files.owe(&self.first, &self.last);
        }
    }
}
fn catalog_extra(
    config: WriterConfig,
    channels: usize,
    producers: usize,
) -> Result<usize, RetentionError> {
    config
        .max_metadata_bytes
        .checked_mul(4)
        .and_then(|n| n.checked_add(channels.checked_mul(1024)?))
        .and_then(|n| n.checked_add(producers.checked_mul(32)?))
        .and_then(|n| n.checked_add(config.max_payload_bytes.checked_add(codec::OVERHEAD)?))
        .ok_or(RetentionError::InvalidConfig)
}
/// Peak output bytes one log holds: its catalog plus one active writer
/// (rotation publishes the old writer before opening the next). Shared
/// with the facade sizing so both use the constructors' formulas.
/// Bytes an MCAP message record adds besides its data: opcode, length,
/// channel, sequence, log time and publish time.
const MESSAGE_HEADER_BYTES: u64 = 1 + 8 + 2 + 4 + 8 + 8;

/// Smallest closed segment a size- or count-based rotation can produce. A
/// segment rotates once its per-record upper bounds (`record_bound`) reach
/// `max_segment_bytes` or it holds `max_records`, while each record really
/// adds at least its MCAP message header. Time rotation and shutdown can
/// close smaller segments. Shared with the facade's segment-count sizing.
pub(crate) fn smallest_rotated_segment(max_segment_bytes: u64, max_records: usize) -> u64 {
    let by_bytes = max_segment_bytes.saturating_mul(MESSAGE_HEADER_BYTES) / RECORD_BOUND_OVERHEAD;
    let by_records = (max_records as u64).saturating_mul(MESSAGE_HEADER_BYTES);
    by_bytes.min(by_records).max(1)
}

/// `record_bound` of an empty payload.
const RECORD_BOUND_OVERHEAD: u64 = codec::OVERHEAD as u64 + 2048;

pub(crate) fn log_required(
    config: WriterConfig,
    channels: &[StorageChannel],
    producers: usize,
    retention: &RetentionConfig,
) -> Result<usize, RetentionError> {
    let writer = config.reservation(channels, producers)?;
    let catalog = Catalog::required(retention, catalog_extra(config, channels.len(), producers)?)?;
    writer
        .checked_add(catalog)
        .ok_or(RetentionError::InvalidConfig)
}
impl<S: SegmentStore> SegmentLog<S> {
    /// Validate, reserve finite metadata, and create the first active file.
    pub fn new(
        budget: WriterBudget,
        config: WriterConfig,
        channels: Vec<StorageChannel>,
        retention: RetentionConfig,
        store: S,
        now: u64,
    ) -> Result<Self, RetentionError> {
        retention.validate()?;
        config.reservation(&channels, budget.producers)?;
        if !store.directory_sync_supported() {
            return Err(RetentionError::UnsupportedDirectorySync);
        }
        let base = Self::base_bound(config, channels.len())?;
        if base
            .checked_add(Self::record_bound(config.max_payload_bytes)?)
            .is_none_or(|n| n > retention.max_segment_bytes)
        {
            return Err(RetentionError::InvalidConfig);
        }
        let extra = catalog_extra(config, channels.len(), budget.producers)?;
        let catalog = Catalog::new(&budget, retention, extra)?;
        let target = TargetLease::enter(&budget)?;
        let producers = budget.producers;
        let channels: Vec<_> = channels
            .into_iter()
            .map(|mut c| {
                c.topic = c.topic.into_boxed_str().into_string();
                c.payload_encoding = c.payload_encoding.into_boxed_str().into_string();
                if let Some(schema) = c.schema.as_mut() {
                    schema.name = schema.name.take().map(|n| n.into_boxed_str().into_string());
                    schema.encoding = std::mem::take(&mut schema.encoding)
                        .into_boxed_str()
                        .into_string();
                    schema.descriptor = std::mem::take(&mut schema.descriptor)
                        .into_boxed_slice()
                        .into_vec();
                }
                c
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
            .into_vec();
        let mut log = Self {
            store,
            writer: None,
            active: None,
            catalog,
            config,
            channels,
            first: vec![0; producers].into_boxed_slice(),
            last: vec![0; producers].into_boxed_slice(),
            base: budget.target_state.files.base(),
            dirty: false,
            now,
            next: 0,
            poisoned: false,
            sealed: false,
            budget,
            target,
        };
        log.open(now)?;
        Ok(log)
    }
    fn base_bound(config: WriterConfig, channels: usize) -> Result<u64, RetentionError> {
        16384u64
            .checked_add(include_bytes!("record_meta.bin").len() as u64 * 2)
            .and_then(|n| n.checked_add((config.max_metadata_bytes as u64).checked_mul(4)?))
            .and_then(|n| n.checked_add((channels as u64).checked_mul(1024)?))
            .ok_or(RetentionError::InvalidConfig)
    }
    fn record_bound(bytes: usize) -> Result<u64, RetentionError> {
        (bytes as u64)
            .checked_add(RECORD_BOUND_OVERHEAD)
            .ok_or(RetentionError::InvalidRecord)
    }
    fn clock(&mut self, now: u64) -> Result<(), RetentionError> {
        if now < self.now {
            return Err(RetentionError::TimeBackwards);
        }
        self.now = now;
        Ok(())
    }
    fn healthy(&self) -> Result<(), RetentionError> {
        if self.poisoned || self.sealed {
            Err(RetentionError::Poisoned)
        } else {
            Ok(())
        }
    }
    fn room(&mut self, now: u64, rotating: bool) -> Result<(), RetentionError> {
        loop {
            let cfg = self.catalog.config;
            let old = self
                .catalog
                .closed
                .iter()
                .any(|s| now.saturating_sub(s.opened_ns) >= cfg.max_age_ns);
            let closed = self.catalog.closed.iter().map(|s| s.bytes).sum::<u64>();
            let active = self.active.as_ref().map_or(0, |a| {
                if rotating {
                    a.upper
                } else {
                    cfg.max_segment_bytes
                }
            });
            let needed = if self.active.is_none() || rotating {
                cfg.max_segment_bytes
            } else {
                0
            };
            let count = self.catalog.closed.len()
                + usize::from(self.active.is_some())
                + usize::from(self.active.is_none() || rotating);
            let free = self.store.available_bytes()?;
            let fits = closed
                .checked_add(active)
                .and_then(|n| n.checked_add(needed))
                .is_some_and(|n| n <= cfg.disk_bytes)
                && count <= cfg.max_segments
                && free >= cfg.disk_reserve_bytes + cfg.max_segment_bytes;
            if cfg.mode == RetentionMode::Continuous {
                let active_old = self
                    .active
                    .as_ref()
                    .is_some_and(|a| now.saturating_sub(a.opened) >= cfg.max_age_ns);
                return if fits && !old && !active_old {
                    Ok(())
                } else {
                    Err(RetentionError::Capacity)
                };
            }
            let candidate = self.catalog.closed.iter().position(|s| {
                now.saturating_sub(s.opened_ns) >= cfg.max_age_ns
                    && !self.catalog.pinned(s.sequence)
                    && !self.catalog.protected(s.mono_range)
            });
            let delete = if !fits {
                self.catalog.oldest_deletable()
            } else {
                candidate
            };
            if let Some(index) = delete {
                let seq = self.catalog.closed[index].sequence;
                if let Err(e) = self.store.remove(seq) {
                    self.poisoned = true;
                    return Err(e.into());
                }
                // Unlink already changed the live namespace. Do not offer a pin
                // to a missing file when the subsequent durability step fails.
                self.catalog.closed.remove(index);
                if let Err(e) = self.store.sync_directory() {
                    self.poisoned = true;
                    return Err(e.into());
                }
            } else {
                return if fits {
                    Ok(())
                } else {
                    Err(RetentionError::Capacity)
                };
            }
        }
    }
    fn open(&mut self, now: u64) -> Result<(), RetentionError> {
        self.room(now, false)?;
        let sequence = self.next;
        let next = sequence.checked_add(1).ok_or(RetentionError::Capacity)?;
        let sink = self.store.create(sequence)?;
        let writer = SegmentWriter::with_target(
            self.budget.clone(),
            self.config,
            self.channels.clone(),
            sink,
            self.target.clone(),
            false,
            sequence,
        )?;
        self.writer = Some(writer);
        self.active = Some(Active {
            sequence,
            opened: now,
            records: 0,
            upper: Self::base_bound(self.config, self.channels.len())?,
            range: None,
            visible: None,
        });
        self.next = next;
        Ok(())
    }
    /// Published closed files only. Active/uncommitted files are never included.
    pub fn segments(&self) -> &[SegmentInfo] {
        &self.catalog.closed
    }
    /// Unique event-protected file bytes, reserving the entire overlapping active file.
    pub fn protected_bytes(&self) -> u64 {
        self.catalog
            .protected_bytes(self.active.as_ref().and_then(|a| a.range))
    }
    /// Append without reporting completion. Invalid inputs/capacity are rejected
    /// before appending; a rotation may have durably published earlier messages.
    pub fn append(
        &mut self,
        record: CapturedRecord,
        payload: &[u8],
        now: u64,
    ) -> Result<AppendReport, RetentionError> {
        self.healthy()?;
        self.clock(now)?;
        let p = record.receipt.producer_id as usize;
        let channel = self
            .channels
            .iter()
            .find(|c| c.channel.channel_id == record.metadata.channel_id)
            .ok_or(RetentionError::InvalidRecord)?;
        if record.receipt.session_id != self.budget.session
            || record.revision != self.budget.revision
            || p >= self.last.len()
            || record.receipt.accepted_seq <= self.last[p]
            || record.metadata.schema_id != channel.channel.schema_id
            || record.metadata.schema_version != channel.channel.schema_version
        {
            return Err(RetentionError::InvalidRecord);
        }
        codec::validate_record(record, payload, self.config.max_payload_bytes)?;
        let charge = Self::record_bound(payload.len())?;
        let a = self.active.as_ref().ok_or(RetentionError::Poisoned)?;
        let mut rotated = if a.records >= self.config.max_records {
            Some(RotationReason::Records)
        } else if a.records > 0 && now - a.opened >= self.catalog.config.max_segment_ns {
            Some(RotationReason::Time)
        } else if a
            .upper
            .checked_add(charge)
            .is_none_or(|n| n > self.catalog.config.max_segment_bytes)
        {
            Some(RotationReason::Bytes)
        } else if self
            .writer
            .as_ref()
            .ok_or(RetentionError::Poisoned)?
            .projected_index_bytes(record, payload.len())
            > self.catalog.config.max_index_bytes
        {
            Some(RotationReason::Index)
        } else {
            None
        };
        if rotated.is_some() {
            self.rotate(now)?;
        } else {
            self.room(now, false)?;
        }
        let mono = record.metadata.mono_ns;
        let range = |old: Option<(u64, u64)>| {
            Some(old.map_or((mono, mono), |(lo, hi)| (lo.min(mono), hi.max(mono))))
        };
        self.catalog
            .check_protection(range(self.active.as_ref().and_then(|a| a.range)))?;
        let result = self
            .writer
            .as_mut()
            .ok_or(RetentionError::Poisoned)?
            .append(record, payload);
        if matches!(result, Err(StorageError::NeedRotation)) {
            self.rotate(now)?;
            rotated = Some(RotationReason::Index);
            self.catalog.check_protection(range(None))?;
            if let Err(e) = self
                .writer
                .as_mut()
                .ok_or(RetentionError::Poisoned)?
                .append(record, payload)
            {
                self.poisoned = true;
                return Err(e.into());
            }
        } else if let Err(e) = result {
            self.poisoned = true;
            return Err(e.into());
        }
        let a = self.active.as_mut().ok_or(RetentionError::Poisoned)?;
        a.upper = a
            .upper
            .checked_add(charge)
            .ok_or(RetentionError::InvalidRecord)?;
        a.records += 1;
        a.range = range(a.range);
        if self.first[p] == 0 {
            self.first[p] = record.receipt.accepted_seq;
        }
        self.last[p] = record.receipt.accepted_seq;
        self.dirty = true;
        Ok(AppendReport {
            segment: a.sequence,
            rotated,
        })
    }
    /// Flush the final complete chunk; this is Written, not durable synchronization.
    pub fn flush(&mut self) -> Result<(), RetentionError> {
        self.healthy()?;
        if let Some(writer) = self.writer.as_mut() {
            if let Err(e) = writer.flush_chunk() {
                self.poisoned = true;
                return Err(e.into());
            }
        }
        if let Some(a) = self.active.as_mut() {
            a.visible = a.range;
        }
        Ok(())
    }
    fn publish(&mut self, now: u64) -> Result<(), RetentionError> {
        let Some(writer) = self.writer.take() else {
            return Ok(());
        };
        self.poisoned = true;
        let finished = writer.finish()?;
        let bytes = finished.safe_offset();
        let mut sink = finished.into_inner();
        sink.sync_all()?;
        drop(sink);
        let a = self.active.as_ref().ok_or(RetentionError::Poisoned)?;
        if bytes > a.upper || bytes > self.catalog.config.max_segment_bytes {
            return Err(RetentionError::InvalidConfig);
        }
        self.store.finalize(a.sequence)?;
        self.store.sync_directory()?;
        self.catalog.closed.push(SegmentInfo {
            sequence: a.sequence,
            bytes,
            opened_ns: a.opened,
            closed_ns: now,
            records: a.records,
            mono_range: a.range,
        });
        self.active = None;
        // Earlier files were synced at their own publication.
        self.dirty = false;
        self.poisoned = false;
        Ok(())
    }
    /// Explicit rotation. Capacity rejection preserves the current active writer.
    pub fn rotate(&mut self, now: u64) -> Result<(), RetentionError> {
        self.healthy()?;
        self.clock(now)?;
        self.room(now, true)?;
        self.publish(now)?;
        if let Err(e) = self.open(now) {
            self.poisoned = true;
            return Err(e);
        }
        Ok(())
    }
    /// Close and publish the last file; retain this catalog for event/pin access.
    pub fn finish(&mut self, now: u64) -> Result<(), RetentionError> {
        self.healthy()?;
        self.clock(now)?;
        self.publish(now)?;
        self.sealed = true;
        Ok(())
    }
    /// Protect retained history and future records until explicit release/expiry.
    /// Flushes the active chunk first so actual bounds refer to complete file data.
    pub fn protect_event(
        &mut self,
        at: u64,
        expires: u64,
        now: u64,
    ) -> Result<EventReport, RetentionError> {
        self.clock(now)?;
        if !self.sealed {
            self.flush()?;
        }
        self.catalog.add_event(
            at,
            expires,
            self.active.as_ref().and_then(|a| a.visible),
            now,
        )
    }
    /// Current retained record extents; no claim of gap-free event coverage.
    pub fn event_report(&self, id: u64) -> Result<EventReport, RetentionError> {
        self.catalog
            .event_report(id, self.active.as_ref().and_then(|a| a.visible), self.now)
    }
    /// Explicitly release one event's references; overlapping windows remain protected.
    pub fn release_event(&mut self, id: u64) -> Result<(), RetentionError> {
        self.catalog.release_event(id)
    }
    /// Explicit expiry sweep. Other operations never silently expire protection.
    pub fn expire_events(&mut self, now: u64) -> Result<usize, RetentionError> {
        self.clock(now)?;
        Ok(self.catalog.expire_events(now))
    }
    /// Pin a published file for a bounded read; callers release after actual IO ends.
    /// Keep the returned permit until it can be passed to `release_pin`, including
    /// error/cancellation paths. Dropping it does not release catalog protection.
    pub fn pin(
        &mut self,
        seq: u64,
        bytes: u64,
        now: u64,
        deadline: u64,
    ) -> Result<ReadPin, RetentionError> {
        self.clock(now)?;
        self.catalog.pin(seq, bytes, now, deadline)
    }
    /// Freeze the active segment's sequence and complete-chunk OS write boundary.
    /// This is not a durability acknowledgment. Open an independent reader before
    /// allowing publication/deletion; a bare active source requires host protection.
    pub fn active_boundary(&self) -> Option<(u64, u64)> {
        Some((
            self.active.as_ref()?.sequence,
            self.writer.as_ref()?.safe_offset(),
        ))
    }
    /// Pin the current active file at its frozen complete-chunk offset. Protection
    /// follows the sequence through rotation until the actual reader is dropped.
    /// Open a separate descriptor for this exact active file before calling.
    pub fn active_query_source<R: std::io::Read + std::io::Seek>(
        &mut self,
        reader: R,
        bytes: u64,
        now: u64,
        deadline: u64,
    ) -> Result<crate::query::QuerySource<R>, RetentionError> {
        self.clock(now)?;
        let (sequence, safe) = self.active_boundary().ok_or(RetentionError::UnknownToken)?;
        let pin = self.catalog.pin_known(sequence, bytes, now, deadline)?;
        Ok(crate::query::QuerySource::pinned(
            reader,
            sequence,
            safe,
            super::ReadPermit::new(pin, bytes, deadline),
        ))
    }
    /// Transfer a published file's pin to a synchronous query source. The supplied
    /// reader must be independently opened and refer to this exact catalog file.
    /// Prefix mutation/replacement is forbidden; closing the reader precedes unpin.
    /// A failed construction drops the reader; no read is left in flight.
    pub fn query_source<R: std::io::Read + std::io::Seek>(
        &mut self,
        reader: R,
        seq: u64,
        bytes: u64,
        now: u64,
        deadline: u64,
    ) -> Result<crate::query::QuerySource<R>, RetentionError> {
        let pin = self.pin(seq, bytes, now, deadline)?;
        let safe = self
            .catalog
            .closed
            .iter()
            .find(|s| s.sequence == seq)
            .map(|s| s.bytes)
            .ok_or(RetentionError::UnknownToken)?;
        Ok(crate::query::QuerySource::pinned(
            reader,
            seq,
            safe,
            super::ReadPermit::new(pin, bytes, deadline),
        ))
    }
    /// Authorize the next read against this pin's remaining bytes and deadline.
    pub fn consume_pin(
        &mut self,
        pin: &ReadPin,
        bytes: u64,
        now: u64,
    ) -> Result<(), RetentionError> {
        self.clock(now)?;
        self.catalog.consume_pin(pin, bytes, now)
    }
    /// End actual read ownership; even expired permits keep files pinned until here.
    /// Call only after all reads using this permit have ended. A lost permit keeps
    /// its pin slot and file protection for this catalog's remaining lifetime.
    pub fn release_pin(&mut self, pin: ReadPin) -> Result<(), RetentionError> {
        self.catalog.release_pin(pin)
    }
    /// Synchronize all accepted files for one control barrier, including active data
    /// and directory metadata. Previously rotated files were synced before publication.
    /// Pre-IO failures return the original work; IO failure completes Failed.
    pub fn sync_work<'c, 'o>(
        &mut self,
        work: SyncWork<'c, 'o>,
    ) -> Result<(), SyncWorkError<'c, 'o>> {
        let check = if self.poisoned {
            Err(StorageError::Poisoned)
        } else {
            super::writer::check_sync(&self.budget, &work, &self.first, &self.last, &self.base)
        };
        if let Err(error) = check {
            return Err(SyncWorkError {
                error,
                work: Some(work),
            });
        }
        let result = self
            .writer
            .as_mut()
            .map_or(Ok(()), |w| w.synchronize())
            .and_then(|()| self.store.sync_directory().map_err(StorageError::Io));
        if result.is_err() {
            self.poisoned = true;
        }
        if result.is_ok() {
            self.dirty = false;
            if let Some(a) = self.active.as_mut() {
                a.visible = a.range;
            }
        }
        work.complete(if result.is_ok() {
            SyncResult::Synced
        } else {
            SyncResult::Failed
        });
        result.map_err(|error| SyncWorkError { error, work: None })
    }
    /// Write a finite target batch across rotations, flush, then report Written.
    /// Prefix/error meanings match SegmentWriter::write_batch; never append the
    /// accepted prefix again, and retry only reports after successful final flush.
    pub fn write_batch(
        &mut self,
        deliveries: &[&Delivery<'_>],
        now: u64,
    ) -> Result<(), RetentionBatchError> {
        let mut progress = RetentionBatchError {
            error: RetentionError::InvalidRecord,
            appended: 0,
            written: 0,
            reported: 0,
        };
        for delivery in deliveries {
            if delivery.target_id() != self.budget.target {
                return Err(progress);
            }
            if let Err(error) = self.append(delivery.record(), delivery.payload(), now) {
                progress.error = error;
                return Err(progress);
            }
            progress.appended += 1;
        }
        if deliveries.is_empty() {
            return Ok(());
        }
        if let Err(error) = self.flush() {
            progress.error = error;
            return Err(progress);
        }
        progress.written = deliveries.len();
        for delivery in deliveries {
            if let Err(e) = super::writer::report_written(delivery) {
                progress.error = RetentionError::Storage(e);
                return Err(progress);
            }
            progress.reported += 1;
        }
        Ok(())
    }
}
