//! Bounded, synchronous queries over frozen complete-chunk file snapshots.
use crate::capture::{CapturedRecord, Level};
use crate::config::ValidatedConfig;
use crate::storage::{
    ReadPermit, StorageChannel, decode_record,
    recovery::{Fields, Validation, crc_update, read_frame},
};
use std::cell::Cell;
use std::io::SeekFrom;
use std::io::{Read, Seek};
use std::sync::atomic::AtomicBool;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Time axis; source ticks require an explicit matching clock domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeBasis {
    /// Session monotonic nanoseconds.
    Mono,
    /// Signed Unix nanoseconds, with no monotonicity assumption.
    Real,
    /// Original ticks in exactly this domain; no implicit clock conversion.
    Source(u64),
}
/// Immutable half-open query filter. Bounds must fit the selected time axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Query {
    /// Timestamp axis and explicit source domain.
    pub basis: TimeBasis,
    /// Inclusive lower bound.
    pub from: i128,
    /// Exclusive upper bound.
    pub to: i128,
    /// Optional product channel selector.
    pub channel: Option<u32>,
    /// Inclusive minimum ordinary log level.
    pub minimum_level: Level,
}
/// Caller supplied limits; page sizes charge metadata as well as inline payload.
#[derive(Clone, Copy, Debug)]
pub struct QueryConfig {
    /// Frozen segment count limit.
    pub max_segments: usize,
    /// Maximum indexed messages across all frozen segments.
    pub max_records: usize,
    /// Maximum container body/chunk bytes, checked before reading.
    pub max_record_bytes: usize,
    /// Maximum frozen length per source, checked before scanning.
    pub max_source_bytes: u64,
    /// Maximum decoded application payload bytes.
    pub max_payload_bytes: usize,
    /// Registration metadata bytes across all channels.
    pub max_metadata_bytes: usize,
    /// Registered producer count from the original session.
    pub producers: usize,
    /// Maximum records in one borrowed page.
    pub page_records: usize,
    /// Maximum metadata plus inline bytes in one page.
    pub page_bytes: usize,
    /// Payloads larger than this return blob handles.
    pub inline_bytes: usize,
    /// Maximum bytes returned by one blob read.
    pub blob_read_bytes: usize,
    /// Cumulative read allowance: the sum of all frozen source safe offsets for
    /// the initial scan, plus every subsequent page inline and blob reread.
    /// A budget equal to the scanned prefixes leaves no allowance for payload
    /// rereads; repeated pages/blob ranges consume allowance again.
    pub read_bytes: u64,
}
/// Structured query failure; IO causes retain the original OS error.
#[derive(Debug)]
pub enum QueryError {
    /// Invalid limits, filter or deadline.
    InvalidInput,
    /// Shared query partition is full or source/index/read bounds were reached.
    Limit,
    /// Source framing, CRC or product metadata was invalid.
    CorruptData,
    /// Unsupported compression or product format.
    Unsupported,
    /// Source domain is absent or differs; no conversion is inferred.
    ClockDomain,
    /// Cursor belongs to a different snapshot.
    Cursor,
    /// Snapshot or pin lifetime ended.
    Expired,
    /// Explicit cancellation; synchronous IO has already returned.
    Cancelled,
    /// Blob has expired, belongs elsewhere, or its range is invalid.
    Gone,
    /// Original reader error.
    Io(std::io::Error),
}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "query error: {self:?}")
    }
}
impl std::error::Error for QueryError {}
/// Shared reservation authority for the session's query/external partition.
pub struct QueryBudget {
    state: Arc<Ledger>,
}
struct Ledger {
    used: AtomicUsize,
    limit: usize,
}
pub(crate) struct Reservation {
    state: Arc<Ledger>,
    bytes: usize,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.state.used.fetch_sub(self.bytes, Ordering::Release);
    }
}
impl QueryBudget {
    /// Create once per session and share this authority with query/remote adapters.
    /// All their reservations are charged to the same external partition.
    pub fn new(plan: &ValidatedConfig) -> Self {
        Self {
            state: Arc::new(Ledger {
                used: AtomicUsize::new(0),
                limit: plan.config().memory.external_bytes,
            }),
        }
    }
    /// Currently reserved query bytes, including snapshots whose owner was dropped.
    pub fn used_bytes(&self) -> usize {
        self.state.used.load(Ordering::Acquire)
    }
    // try_update requires Rust 1.95; keep fetch_update for MSRV 1.85.
    // Nightly deprecates the alias. Suppress only this reservation operation.
    #[allow(deprecated)]
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Reservation, QueryError> {
        self.state
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_add(bytes).filter(|v| *v <= self.state.limit)
            })
            .map_err(|_| QueryError::Limit)?;
        Ok(Reservation {
            state: Arc::clone(&self.state),
            bytes,
        })
    }
}
/// One opened reader and its frozen OS-visible limit. The host preserves that prefix.
pub struct QuerySource<R> {
    // Reader must close before the permit advertises that actual reads ended.
    reader: R,
    sequence: u64,
    safe_offset: u64,
    permit: Option<ReadPermit>,
}
impl<R> QuerySource<R> {
    /// Freeze an active source at a writer-published complete-chunk offset.
    /// The host prevents truncation/replacement and deletion for this reader's lifetime.
    pub fn active(reader: R, sequence: u64, safe_offset: u64) -> Self {
        Self {
            reader,
            sequence,
            safe_offset,
            permit: None,
        }
    }
    pub(crate) fn pinned(reader: R, sequence: u64, safe_offset: u64, permit: ReadPermit) -> Self {
        Self {
            reader,
            sequence,
            safe_offset,
            permit: Some(permit),
        }
    }
    fn authorize(&mut self, bytes: u64, now: u64) -> Result<(), QueryError> {
        if let Some(p) = &mut self.permit {
            p.authorize(bytes, now).map_err(|e| match e {
                crate::storage::RetentionError::Expired => QueryError::Expired,
                _ => QueryError::Limit,
            })?;
        }
        Ok(())
    }
}
/// Snapshot-local original payload reference. It cannot be forged or moved across snapshots.
#[derive(Clone, Debug)]
pub struct BlobHandle {
    identity: Arc<()>,
    row: usize,
}
/// Bounded payload representation in a borrowed page.
#[derive(Clone, Debug)]
pub enum Payload {
    /// Byte range local to the page that returned this record.
    /// Obtain payload bytes with [`Page::inline`]; these coordinates are not
    /// source-file offsets and must not be reused with another page.
    Inline {
        /// Start in the page's borrowed inline bytes.
        offset: usize,
        /// Application payload byte count.
        length: usize,
    },
    /// Original file bytes, available through bounded reads.
    Blob(BlobHandle),
}
/// Product metadata and bounded payload descriptor.
#[derive(Clone, Debug)]
pub struct QueryRecord {
    /// Original receipt, clocks, revision, channel and payload length.
    pub record: CapturedRecord,
    /// Inline range or snapshot-local original blob handle.
    pub payload: Payload,
}
/// Opaque immutable continuation retaining the original filter.
#[derive(Clone, Debug)]
pub struct Cursor {
    identity: Arc<()>,
    query: Query,
    position: usize,
}
/// Borrowed page; the snapshot cannot reuse buffers while the page is live.
pub struct Page<'a> {
    /// Records charged against both page limits.
    pub records: &'a [QueryRecord],
    /// Continuation, absent after the last matching message.
    pub next: Option<Cursor>,
    data: &'a [u8],
}
impl Page<'_> {
    /// Obtain inline bytes for a record from this page, or None for a blob
    /// descriptor or invalid range. Pass records from this page only: ranges
    /// are bounds-checked but do not carry page identity.
    pub fn inline(&self, item: &QueryRecord) -> Option<&[u8]> {
        match item.payload {
            Payload::Inline { offset, length } => {
                self.data.get(offset..offset.checked_add(length)?)
            }
            Payload::Blob(_) => None,
        }
    }
}
/// Owns readers, fixed index/buffers and their reservation until actual reads end.
pub struct Snapshot<R> {
    sources: Vec<QuerySource<R>>,
    rows: Vec<Row>,
    order: Vec<usize>,
    ordered_query: Option<Query>,
    items: Vec<QueryRecord>,
    data: Vec<u8>,
    blob: Vec<u8>,
    identity: Arc<()>,
    config: QueryConfig,
    deadline: u64,
    observed: Cell<u64>,
    remaining: u64,
    // Last: allocation buffers and sources finish before reservation release.
    _reservation: Reservation,
}
struct Row {
    record: CapturedRecord,
    source: usize,
    payload_offset: u64,
}
fn check(now: u64, deadline: u64, cancel: &AtomicBool) -> Result<(), QueryError> {
    if now >= deadline {
        return Err(QueryError::Expired);
    }
    if cancel.load(Ordering::Acquire) {
        return Err(QueryError::Cancelled);
    }
    Ok(())
}
fn invalid(_: impl std::fmt::Debug) -> QueryError {
    QueryError::CorruptData
}
fn scan_stop(stop: crate::storage::RecoveryStop) -> QueryError {
    match stop {
        crate::storage::RecoveryStop::Limit => QueryError::Limit,
        crate::storage::RecoveryStop::UnsupportedCompression => QueryError::Unsupported,
        _ => QueryError::CorruptData,
    }
}
impl QueryConfig {
    /// Conservative requested-byte bound for one snapshot, including decode temporaries.
    pub fn reservation(&self, channels: &[StorageChannel]) -> Result<usize, QueryError> {
        if self.max_segments == 0
            || self.max_records == 0
            || self.max_record_bytes < 40
            || self.max_payload_bytes == 0
            || self.max_source_bytes < 8
            || self.producers == 0
            || self.page_records == 0
            || self.page_bytes < std::mem::size_of::<QueryRecord>()
            || self.inline_bytes > self.page_bytes
            || self.blob_read_bytes == 0
            || self.read_bytes == 0
            || channels.is_empty()
            || channels.len() > u16::MAX as usize
        {
            return Err(QueryError::InvalidInput);
        }
        let metadata = channels
            .iter()
            .try_fold(0usize, |n, c| {
                n.checked_add(c.topic.len())?
                    .checked_add(c.payload_encoding.len())?
                    .checked_add(
                        c.schema
                            .as_ref()
                            .map_or(0, |s| s.encoding.len().saturating_add(s.descriptor.len())),
                    )
            })
            .ok_or(QueryError::InvalidInput)?;
        if metadata > self.max_metadata_bytes {
            return Err(QueryError::Limit);
        }
        let mut result = 65536usize;
        for (count, size) in [
            (self.max_record_bytes, 4),
            (self.max_payload_bytes, 4),
            (
                self.max_records,
                std::mem::size_of::<Row>() + std::mem::size_of::<usize>(),
            ),
            (self.page_records, std::mem::size_of::<QueryRecord>()),
            (self.page_bytes, 1),
            (self.blob_read_bytes, 1),
            (self.max_metadata_bytes, 8),
            (channels.len(), 128),
            (self.producers, 16),
            (
                self.max_segments,
                std::mem::size_of::<QuerySource<()>>() + 128,
            ),
        ] {
            result = result
                .checked_add(count.checked_mul(size).ok_or(QueryError::InvalidInput)?)
                .ok_or(QueryError::InvalidInput)?;
        }
        if result > isize::MAX as usize {
            return Err(QueryError::InvalidInput);
        }
        Ok(result)
    }
}
impl<R: Read + Seek> Snapshot<R> {
    /// Validate and index the frozen source ranges. Sources must be independently
    /// opened, uncompressed dot-logger/1 files with the supplied original registrations.
    /// Large payloads are decoded only in bounded scan scratch, then referenced on disk.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        budget: &QueryBudget,
        session: u128,
        revision: u64,
        channels: Vec<StorageChannel>,
        sources: Vec<QuerySource<R>>,
        config: QueryConfig,
        now: u64,
        deadline: u64,
        cancel: &AtomicBool,
    ) -> Result<Self, QueryError> {
        let deadline = sources
            .iter()
            .filter_map(|s| s.permit.as_ref().map(ReadPermit::deadline))
            .fold(deadline, u64::min);
        check(now, deadline, cancel)?;
        let reservation = budget.reserve(config.reservation(&channels)?)?;
        if sources.len() > config.max_segments {
            return Err(QueryError::Limit);
        }
        for (i, s) in sources.iter().enumerate() {
            if s.safe_offset > config.max_source_bytes {
                return Err(QueryError::Limit);
            }
            if sources[..i].iter().any(|a| a.sequence == s.sequence) {
                return Err(QueryError::InvalidInput);
            }
        }
        for (i, c) in channels.iter().enumerate() {
            if channels[..i]
                .iter()
                .any(|a| a.channel.channel_id == c.channel.channel_id)
            {
                return Err(QueryError::InvalidInput);
            }
        }
        let mut snapshot = Self {
            sources: sources.into_boxed_slice().into_vec(),
            rows: Vec::with_capacity(config.max_records),
            order: Vec::with_capacity(config.max_records),
            ordered_query: None,
            items: Vec::with_capacity(config.page_records),
            data: Vec::with_capacity(config.page_bytes),
            blob: vec![0; config.blob_read_bytes],
            identity: Arc::new(()),
            config,
            deadline,
            observed: Cell::new(now),
            remaining: config.read_bytes,
            _reservation: reservation,
        };
        let mut buffer = vec![0; config.max_record_bytes];
        for source in 0..snapshot.sources.len() {
            snapshot.scan(
                source,
                (session, revision),
                &channels,
                &mut buffer,
                now,
                cancel,
            )?;
        }
        check(now, deadline, cancel)?;
        Ok(snapshot)
    }
    fn charge(&mut self, source: usize, bytes: u64, now: u64) -> Result<(), QueryError> {
        let remaining = self.remaining.checked_sub(bytes).ok_or(QueryError::Limit)?;
        self.sources[source].authorize(bytes, now)?;
        self.remaining = remaining;
        Ok(())
    }
    fn scan(
        &mut self,
        index: usize,
        identity: (u128, u64),
        channels: &[StorageChannel],
        buffer: &mut [u8],
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<(), QueryError> {
        let end = self.sources[index].safe_offset;
        if end == 0 {
            return Ok(());
        }
        if end < 8 {
            return Err(QueryError::CorruptData);
        }
        // Charge a conservative full prefix once. Later page/blob rereads charge again.
        self.charge(index, end, now)?;
        let reader = &mut self.sources[index].reader;
        if reader.seek(SeekFrom::End(0)).map_err(QueryError::Io)? < end {
            return Err(QueryError::CorruptData);
        }
        reader.seek(SeekFrom::Start(0)).map_err(QueryError::Io)?;
        let mut magic = [0; 8];
        reader.read_exact(&mut magic).map_err(QueryError::Io)?;
        if &magic != b"\x89MCAP0\r\n" {
            return Err(QueryError::CorruptData);
        }
        let mut position = 8;
        let (op, len, header) = read_frame(reader, buffer, end, &mut position)
            .map_err(QueryError::Io)?
            .map_err(scan_stop)?;
        let mut f = Fields(&buffer[..len]);
        if op != 1
            || f.string().map_err(invalid)? != b"dot-logger/1"
            || std::str::from_utf8(f.string().map_err(invalid)?).is_err()
            || !f.0.is_empty()
        {
            return Err(QueryError::Unsupported);
        }
        let mut crc = crc_update(crc_update(!0, &magic), &header);
        crc = crc_update(crc, &buffer[..len]);
        let mut validation = Validation {
            session: identity.0,
            revision: identity.1,
            channels,
            envelope: None,
            payload_schemas: vec![None; channels.len()],
            seen: vec![false; channels.len()],
            last: vec![0; self.config.producers],
            max_payload: self.config.max_payload_bytes,
        };
        while position < end {
            check(now, self.deadline, cancel)?;
            let start = position;
            let (op, len, header) = read_frame(reader, buffer, end, &mut position)
                .map_err(QueryError::Io)?
                .map_err(scan_stop)?;
            let body = &buffer[..len];
            match op {
                3 => validation.schema(body).map_err(invalid)?,
                4 => validation.channel(body).map_err(invalid)?,
                6 => {
                    validation.chunk(body).map_err(|e| match e {
                        crate::storage::RecoveryStop::UnsupportedCompression => {
                            QueryError::Unsupported
                        }
                        _ => QueryError::CorruptData,
                    })?;
                    let mut nested = Fields(body.get(40..).ok_or(QueryError::CorruptData)?);
                    while !nested.0.is_empty() {
                        check(now, self.deadline, cancel)?;
                        let at = len - nested.0.len();
                        let (op, record) = nested.record().map_err(invalid)?;
                        if op != 5 {
                            continue;
                        }
                        if self.rows.len() == self.config.max_records {
                            return Err(QueryError::Limit);
                        }
                        let envelope = record.get(22..).ok_or(QueryError::CorruptData)?;
                        let stored = decode_record(envelope, self.config.max_payload_bytes)
                            .map_err(invalid)?;
                        let (payload_start, payload_len) = payload_range(envelope)?;
                        if envelope.get(payload_start..payload_start + payload_len)
                            != Some(stored.payload.as_slice())
                        {
                            return Err(QueryError::CorruptData);
                        }
                        let absolute = start
                            .checked_add(9)
                            .and_then(|n| n.checked_add(at as u64))
                            .and_then(|n| n.checked_add(9 + 22))
                            .and_then(|n| n.checked_add(payload_start as u64))
                            .ok_or(QueryError::CorruptData)?;
                        self.rows.push(Row {
                            record: stored.record,
                            source: index,
                            payload_offset: absolute,
                        });
                    }
                }
                7 => {}
                15 => {
                    if body.len() != 4 || (body != [0; 4] && body != (!crc).to_le_bytes()) {
                        return Err(QueryError::CorruptData);
                    }
                    break;
                }
                _ => return Err(QueryError::CorruptData),
            }
            crc = crc_update(crc_update(crc, &header), body);
        }
        Ok(())
    }
    fn alive(&self, now: u64, cancel: &AtomicBool) -> Result<(), QueryError> {
        // A stale host timestamp must never revive an already observed expiry.
        let observed = now.max(self.observed.get());
        self.observed.set(observed);
        check(observed, self.deadline, cancel)
    }
    /// Start a cursor with a frozen filter. Bounds are native ticks/ns; conversion
    /// between unrelated source clock domains is deliberately unsupported.
    pub fn start(&self, query: Query, now: u64, cancel: &AtomicBool) -> Result<Cursor, QueryError> {
        self.alive(now, cancel)?;
        let (min, max) = match query.basis {
            TimeBasis::Mono => (0, u64::MAX as i128 + 1),
            _ => (i64::MIN as i128, i64::MAX as i128 + 1),
        };
        if query.from >= query.to || query.from < min || query.to > max {
            return Err(QueryError::InvalidInput);
        }
        for row in &self.rows {
            if matches_filter(row, query) {
                timestamp(row, query.basis)?;
            }
        }
        Ok(Cursor {
            identity: Arc::clone(&self.identity),
            query,
            position: 0,
        })
    }
    /// Reuse the finite ordering index while the cursor's filter is unchanged;
    /// switching filters rebuilds it in the same reserved buffer. Payloads remain
    /// on disk; selected inline bytes are reread for each borrowed page.
    pub fn page(
        &mut self,
        cursor: &Cursor,
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<Page<'_>, QueryError> {
        self.page_limit(cursor, now, cancel, self.config.page_records)
    }
    pub(crate) fn page_one(
        &mut self,
        cursor: &Cursor,
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<Page<'_>, QueryError> {
        self.page_limit(cursor, now, cancel, 1)
    }
    pub(crate) fn check_alive(&self, now: u64, cancel: &AtomicBool) -> Result<(), QueryError> {
        self.alive(now, cancel)
    }
    pub(crate) fn inline_payload(&self, record: &QueryRecord) -> Option<&[u8]> {
        match record.payload {
            Payload::Inline { offset, length } => {
                self.data.get(offset..offset.checked_add(length)?)
            }
            Payload::Blob(_) => None,
        }
    }
    pub(crate) fn blob_payload(&self, length: usize) -> Option<&[u8]> {
        self.blob.get(..length)
    }
    fn page_limit(
        &mut self,
        cursor: &Cursor,
        now: u64,
        cancel: &AtomicBool,
        records: usize,
    ) -> Result<Page<'_>, QueryError> {
        self.alive(now, cancel)?;
        if !Arc::ptr_eq(&cursor.identity, &self.identity) {
            return Err(QueryError::Cursor);
        }
        self.items.clear();
        self.data.clear();
        if self.ordered_query != Some(cursor.query) {
            // A failed/cancelled rebuild must never publish a partial index as
            // the previous query's cache. Rows and source boundaries are frozen.
            self.ordered_query = None;
            self.order.clear();
            for (i, row) in self.rows.iter().enumerate() {
                self.alive(now, cancel)?;
                if matches_filter(row, cursor.query) {
                    let time = timestamp(row, cursor.query.basis)?;
                    if cursor.query.from <= time && time < cursor.query.to {
                        self.order.push(i);
                    }
                }
            }
            self.order.sort_unstable_by_key(|i| {
                let r = &self.rows[*i];
                (
                    timestamp(r, cursor.query.basis).unwrap_or(0),
                    r.record.receipt.producer_id,
                    r.record.receipt.accepted_seq,
                    self.sources[r.source].sequence,
                    r.payload_offset,
                )
            });
            self.alive(now, cancel)?;
            self.ordered_query = Some(cursor.query);
        }
        let mut at = cursor.position;
        let mut charged = 0usize;
        while let Some(&row_id) = self.order.get(at) {
            self.alive(now, cancel)?;
            let row = &self.rows[row_id];
            let len = row.record.payload_len;
            let base = std::mem::size_of::<QueryRecord>();
            let inline = len <= self.config.inline_bytes
                && base
                    .checked_add(len)
                    .is_some_and(|v| v <= self.config.page_bytes);
            let cost = base + if inline { len } else { 0 };
            if self.items.len() == records || charged + cost > self.config.page_bytes {
                break;
            }
            let record = row.record;
            let source = row.source;
            let offset = row.payload_offset;
            let payload = if inline {
                self.charge(source, len as u64, now)?;
                let begin = self.data.len();
                self.data.resize(begin + len, 0);
                self.sources[source]
                    .reader
                    .seek(SeekFrom::Start(offset))
                    .map_err(QueryError::Io)?;
                self.sources[source]
                    .reader
                    .read_exact(&mut self.data[begin..])
                    .map_err(QueryError::Io)?;
                self.alive(now, cancel)?;
                Payload::Inline {
                    offset: begin,
                    length: len,
                }
            } else {
                Payload::Blob(BlobHandle {
                    identity: Arc::clone(&self.identity),
                    row: row_id,
                })
            };
            self.items.push(QueryRecord { record, payload });
            charged += cost;
            at += 1;
        }
        self.alive(now, cancel)?;
        let next = (at < self.order.len()).then(|| Cursor {
            identity: Arc::clone(&self.identity),
            query: cursor.query,
            position: at,
        });
        Ok(Page {
            records: &self.items,
            data: &self.data,
            next,
        })
    }
    /// Read a bounded original payload range. Handles expire with the snapshot;
    /// expiry/cancellation never releases source protection before actual IO returns.
    pub fn read_blob(
        &mut self,
        handle: &BlobHandle,
        offset: u64,
        length: usize,
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<&[u8], QueryError> {
        self.alive(now, cancel).map_err(|e| match e {
            QueryError::Expired => QueryError::Gone,
            other => other,
        })?;
        if !Arc::ptr_eq(&handle.identity, &self.identity) {
            return Err(QueryError::Gone);
        }
        if length > self.config.blob_read_bytes {
            return Err(QueryError::Limit);
        }
        let row = self.rows.get(handle.row).ok_or(QueryError::Gone)?;
        let end = offset.checked_add(length as u64).ok_or(QueryError::Gone)?;
        if end > row.record.payload_len as u64 {
            return Err(QueryError::Gone);
        }
        let absolute = row
            .payload_offset
            .checked_add(offset)
            .ok_or(QueryError::Gone)?;
        let source = row.source;
        self.charge(source, length as u64, now)?;
        self.sources[source]
            .reader
            .seek(SeekFrom::Start(absolute))
            .map_err(QueryError::Io)?;
        self.sources[source]
            .reader
            .read_exact(&mut self.blob[..length])
            .map_err(QueryError::Io)?;
        self.alive(now, cancel).map_err(|e| match e {
            QueryError::Expired => QueryError::Gone,
            other => other,
        })?;
        Ok(&self.blob[..length])
    }
}
fn matches_filter(row: &Row, query: Query) -> bool {
    query
        .channel
        .is_none_or(|c| c == row.record.metadata.channel_id)
        && row.record.metadata.level as u8 >= query.minimum_level as u8
}
fn timestamp(row: &Row, basis: TimeBasis) -> Result<i128, QueryError> {
    Ok(match basis {
        TimeBasis::Mono => row.record.metadata.mono_ns as i128,
        TimeBasis::Real => row.record.metadata.real_ns as i128,
        TimeBasis::Source(domain) => {
            let t = row
                .record
                .metadata
                .source_time
                .ok_or(QueryError::ClockDomain)?;
            if t.clock_domain != domain {
                return Err(QueryError::ClockDomain);
            }
            t.ticks as i128
        }
    })
}
// Locate the exact on-disk protobuf bytes field after the bounded full decoder has
// validated the envelope. Reject unsupported group wire types and duplicate payloads
// rather than guessing a contiguous blob location for an ambiguous envelope.
fn payload_range(bytes: &[u8]) -> Result<(usize, usize), QueryError> {
    fn varint(bytes: &[u8], at: &mut usize) -> Result<u64, QueryError> {
        let mut n = 0;
        for shift in (0..70).step_by(7) {
            let b = *bytes.get(*at).ok_or(QueryError::CorruptData)?;
            *at += 1;
            if shift == 63 && b > 1 {
                return Err(QueryError::CorruptData);
            }
            n |= u64::from(b & 127) << shift;
            if b & 128 == 0 {
                return Ok(n);
            }
        }
        Err(QueryError::CorruptData)
    }
    let mut at = 0;
    let mut payload = None;
    while at < bytes.len() {
        let key = varint(bytes, &mut at)?;
        if key >> 3 == 0 {
            return Err(QueryError::CorruptData);
        }
        let size = match key & 7 {
            0 => {
                varint(bytes, &mut at)?;
                continue;
            }
            1 => 8,
            2 => usize::try_from(varint(bytes, &mut at)?).map_err(invalid)?,
            5 => 4,
            _ => return Err(QueryError::CorruptData),
        };
        let end = at
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or(QueryError::CorruptData)?;
        if key >> 3 == 20 {
            if key & 7 != 2 || payload.is_some() {
                return Err(QueryError::CorruptData);
            }
            payload = Some((at, size));
        }
        at = end;
    }
    // A protobuf empty bytes field may be absent; it still has a valid empty range.
    Ok(payload.unwrap_or((bytes.len(), 0)))
}
