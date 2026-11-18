//! Bounded, synchronous queries over frozen complete-chunk file snapshots.
use crate::capture::{CapturedRecord, Level};
use crate::config::ValidatedConfig;
use crate::storage::{
    ReadPermit, StorageChannel,
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
    /// Maximum retained loss-report parts; zero validates and skips control data.
    pub max_loss_reports: usize,
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
#[cfg(feature = "influxdb")]
impl Reservation {
    // Reserve from the same session authority, including after its public handle
    // is dropped. Every sibling owns its own charge until its actual Drop.
    pub(crate) fn reserve_extra(&self, bytes: usize) -> Result<Self, QueryError> {
        QueryBudget {
            state: Arc::clone(&self.state),
        }
        .reserve(bytes)
    }
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
    revision: u64,
    expected_target: Option<u32>,
    actual_target: Option<u32>,
    loss_rows: usize,
    safe_offset: u64,
    permit: Option<ReadPermit>,
}
impl<R> QuerySource<R> {
    /// Require this original target identity. Needed to distinguish equal file
    /// sequences from different outputs; it is validated against file Metadata.
    pub fn expect_target(mut self, target: u32) -> Self {
        self.expected_target = Some(target);
        self
    }

    /// Freeze an active source at a writer-published complete-chunk offset.
    /// Sequence and revision are checked against the original file metadata.
    /// The host prevents truncation/replacement and deletion for this reader's lifetime.
    pub fn active(reader: R, sequence: u64, revision: u64, safe_offset: u64) -> Self {
        Self {
            reader,
            sequence,
            revision,
            expected_target: None,
            actual_target: None,
            loss_rows: 0,
            safe_offset,
            permit: None,
        }
    }
    pub(crate) fn pinned(
        reader: R,
        sequence: u64,
        revision: u64,
        safe_offset: u64,
        permit: ReadPermit,
    ) -> Self {
        Self {
            reader,
            sequence,
            revision,
            expected_target: None,
            actual_target: None,
            loss_rows: 0,
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
    loss_only: bool,
    sources: Vec<QuerySource<R>>,
    rows: Vec<Row>,
    losses: Vec<crate::loss::LossReport>,
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
            (
                self.max_loss_reports,
                256 + crate::loss::ROWS_PER_REPORT * 512,
            ),
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
    /// opened, uncompressed dot-logger/standard/2 files with the supplied original registrations.
    /// Each source declares its own revision and sequence; receipts must be unique
    /// across the entire session snapshot, even across different revisions.
    /// Large payloads are decoded only in bounded scan scratch, then referenced on disk.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        budget: &QueryBudget,
        session: u128,
        channels: Vec<StorageChannel>,
        sources: Vec<QuerySource<R>>,
        config: QueryConfig,
        now: u64,
        deadline: u64,
        cancel: &AtomicBool,
    ) -> Result<Self, QueryError> {
        Self::build(
            budget, session, channels, sources, config, now, deadline, cancel, false,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn build(
        budget: &QueryBudget,
        session: u128,
        channels: Vec<StorageChannel>,
        sources: Vec<QuerySource<R>>,
        config: QueryConfig,
        now: u64,
        deadline: u64,
        cancel: &AtomicBool,
        loss_only: bool,
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
            if sources[..i].iter().any(|a| {
                a.sequence == s.sequence
                    && (a.expected_target.is_none()
                        || s.expected_target.is_none()
                        || a.expected_target == s.expected_target)
            }) {
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
            loss_only,
            sources: sources.into_boxed_slice().into_vec(),
            rows: Vec::with_capacity(if loss_only { 0 } else { config.max_records }),
            losses: Vec::with_capacity(config.max_loss_reports),
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
                (session, snapshot.sources[source].revision),
                &channels,
                &mut buffer,
                now,
                cancel,
            )?;
        }
        for (i, source) in snapshot.sources.iter().enumerate() {
            if source.actual_target.is_some()
                && snapshot.sources[..i].iter().any(|old| {
                    old.sequence == source.sequence && old.actual_target == source.actual_target
                })
            {
                return Err(QueryError::InvalidInput);
            }
        }
        // Reuse the already reserved ordering buffer. Receipt identity is session
        // scoped, so a second source/revision may not relabel the same capture.
        snapshot.order.extend(0..snapshot.rows.len());
        snapshot.order.sort_unstable_by_key(|i| {
            let receipt = snapshot.rows[*i].record.receipt;
            (receipt.producer_id, receipt.accepted_seq)
        });
        if snapshot
            .order
            .windows(2)
            .any(|w| snapshot.rows[w[0]].record.receipt == snapshot.rows[w[1]].record.receipt)
        {
            return Err(QueryError::CorruptData);
        }
        snapshot.order.clear();
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
        let loss_only = self.loss_only;
        let end = self.sources[index].safe_offset;
        let expected_target = self.sources[index].expected_target;
        let sequence = self.sources[index].sequence;
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
            || f.string().map_err(invalid)? != crate::storage::codec::PROFILE.as_bytes()
            || std::str::from_utf8(f.string().map_err(invalid)?).is_err()
            || !f.0.is_empty()
        {
            return Err(QueryError::Unsupported);
        }
        let mut crc = crc_update(crc_update(!0, &magic), &header);
        crc = crc_update(crc, &buffer[..len]);
        let mut validation = Validation::new(
            identity.0,
            identity.1,
            std::borrow::Cow::Borrowed(channels),
            self.config.producers,
            self.config.max_payload_bytes,
        );
        while position < end {
            check(now, self.deadline, cancel)?;
            let start = position;
            let (op, len, header) = read_frame(reader, buffer, end, &mut position)
                .map_err(QueryError::Io)?
                .map_err(scan_stop)?;
            let body = &buffer[..len];
            match op {
                12 => {
                    validation.metadata(body).map_err(invalid)?;
                    if validation.file_sequence() != Some(sequence)
                        || expected_target
                            .is_some_and(|id| validation.file_info().is_none_or(|f| f.target != id))
                    {
                        return Err(QueryError::CorruptData);
                    }
                }
                3 | 4 => validation.registration(op, body).map_err(invalid)?,
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
                        if let Some(report) = validation.loss_report(record).map_err(invalid)? {
                            if self.config.max_loss_reports > 0 {
                                if self.losses.len() == self.config.max_loss_reports {
                                    return Err(QueryError::Limit);
                                }
                                self.losses.push(report);
                            }
                            continue;
                        }
                        if loss_only {
                            nested.record().map_err(invalid)?;
                            continue;
                        }
                        if self.rows.len() == self.config.max_records {
                            return Err(QueryError::Limit);
                        }
                        let (op, meta) = nested.record().map_err(invalid)?;
                        if op != 5 {
                            return Err(QueryError::CorruptData);
                        }
                        let stored = validation.pair(record, meta).map_err(invalid)?;
                        let absolute = start
                            .checked_add(9)
                            .and_then(|n| n.checked_add(at as u64))
                            .and_then(|n| n.checked_add(9 + 22))
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
        validation.registrations_complete().map_err(invalid)?;
        self.sources[index].loss_rows = validation.file_info().map_or(0, |file| file.loss_rows);
        self.sources[index].actual_target = validation.file_info().map(|file| file.target);
        Ok(())
    }
    /// Number of frozen sources that explicitly have no loss collection.
    /// These sources provide no evidence of zero loss.
    pub fn loss_unavailable_sources(&self) -> usize {
        self.sources.iter().filter(|s| s.loss_rows == 0).count()
    }
    /// Return the separately bounded control parts. No fake QueryRecord or receipt
    /// is created. A positive max_loss_reports is required for enabled files.
    pub fn loss_reports(
        &self,
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<&[crate::loss::LossReport], QueryError> {
        self.alive(now, cancel)?;
        if self.config.max_loss_reports == 0 && self.sources.iter().any(|s| s.loss_rows > 0) {
            return Err(QueryError::Limit);
        }
        Ok(&self.losses)
    }
    /// Compare complete counter watermarks without adding replicated ingress rows.
    pub fn loss_deltas(
        &self,
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<Vec<crate::loss::LossDelta>, QueryError> {
        let reports = self.loss_reports(now, cancel)?;
        crate::loss::summarize(
            reports,
            self.config
                .max_loss_reports
                .checked_mul(crate::loss::ROWS_PER_REPORT)
                .ok_or(QueryError::Limit)?,
        )
        .map_err(|_| QueryError::CorruptData)
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

/// Separately bounded loss-only snapshot. User pairs are validated but never
/// indexed, replayed or converted into synthetic loss records. Equal source
/// sequences across outputs require explicit expected target identities.
pub struct LossSnapshot<R> {
    inner: Snapshot<R>,
}
impl<R: Read + Seek> LossSnapshot<R> {
    /// Validate frozen source prefixes and retain only bounded control reports.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        budget: &QueryBudget,
        session: u128,
        channels: Vec<StorageChannel>,
        sources: Vec<QuerySource<R>>,
        config: QueryConfig,
        now: u64,
        deadline: u64,
        cancel: &AtomicBool,
    ) -> Result<Self, QueryError> {
        Ok(Self {
            inner: Snapshot::build(
                budget, session, channels, sources, config, now, deadline, cancel, true,
            )?,
        })
    }
    /// Sources with unavailable collection provide no evidence of zero loss.
    pub fn unavailable_sources(&self) -> usize {
        self.inner.loss_unavailable_sources()
    }
    /// Bounded parts, after validating every record in their complete chunk.
    pub fn reports(
        &self,
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<&[crate::loss::LossReport], QueryError> {
        self.inner.loss_reports(now, cancel)
    }
    /// Replicated ingress values are compared as watermarks, never summed.
    pub fn deltas(
        &self,
        now: u64,
        cancel: &AtomicBool,
    ) -> Result<Vec<crate::loss::LossDelta>, QueryError> {
        self.inner.loss_deltas(now, cancel)
    }
}
