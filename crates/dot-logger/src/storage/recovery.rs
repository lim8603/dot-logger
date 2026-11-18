//! Bounded management-path recovery of an immutable dot-logger/standard/2 segment.
use super::*;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};

/// Explicit source-side bounds; output limits remain in WriterConfig.
#[derive(Clone, Copy, Debug)]
pub struct RecoveryConfig {
    /// Maximum source length inspected; larger inputs are rejected before writing.
    pub max_source_bytes: u64,
    /// Maximum body length of any scanned container record or chunk.
    pub max_record_bytes: usize,
    /// Maximum complete source chunks recovered, including metadata-only chunks.
    pub max_chunks: usize,
}
/// Frozen output configuration and finite source bounds.
#[derive(Clone, Copy, Debug)]
pub struct RecoveryOptions {
    /// Optional original target identity, required to disambiguate equal sequences.
    pub source_target: Option<u32>,
    /// Expected original file sequence. Independent sink recovery preserves it;
    /// `recover_to_store` instead names its output with the supplied new sequence.
    pub source_sequence: u64,
    /// Original session's registered output format and capacities.
    pub writer: WriterConfig,
    /// Source-side bounds, supplied explicitly by the caller.
    pub limits: RecoveryConfig,
}
/// Why scanning ended; no resynchronization is attempted after this boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryStop {
    /// Input ended at a record boundary; no footer durability is inferred.
    EndOfInput,
    /// Data section ended; its old summary/index is deliberately rebuilt.
    DataEnd,
    /// A container header or body was cut short.
    Truncated,
    /// A required nonzero chunk CRC did not match, or was absent.
    Checksum,
    /// A record, schema/channel reference, metadata pair or sequence was invalid.
    InvalidRecord,
    /// Only the product's uncompressed writer format is supported.
    UnsupportedCompression,
    /// A configured record/chunk bound was reached.
    Limit,
    /// An IO/ownership/configuration error interrupted recovery.
    Failed,
    /// Caller cancellation stopped recovery without publishing success.
    Cancelled,
}
/// Source evidence only; never reconstructs a past process's completion acknowledgments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryReport {
    /// Frozen observed source length; caller must keep the source immutable.
    pub source_bytes: u64,
    /// End offset of the last fully validated and copied source chunk, or zero.
    pub verified_end: u64,
    /// Source suffix not copied, including replaced indexes/summary, as [start,end).
    pub excluded: (u64, u64),
    /// Validated source chunks copied to the new output.
    pub chunks: usize,
    /// Recovered product messages, preserving their original receipts.
    pub records: usize,
    /// Copied physical loss-report parts; excluded from logical user records.
    pub loss_reports: usize,
    /// Stop cause; a successful prefix can still exclude corrupt input.
    pub stop: RecoveryStop,
}
/// Recovery failure retaining both its original cause and fully copied source prefix.
#[derive(Debug)]
pub struct RecoveryError {
    /// Storage/OS cause, absent only for explicit cancellation.
    pub error: Option<StorageError>,
    /// Last copied prefix; failure never authorizes publishing the output.
    pub report: RecoveryReport,
}
impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "recovery failed: {:?}: {:?}",
            self.report.stop, self.error
        )
    }
}
impl std::error::Error for RecoveryError {}
/// Synced new container with rebuilt standard and auxiliary indexes.
/// Its synchronization says nothing about acknowledgments made before the crash.
pub struct RecoveredSegment<W: SyncSink> {
    /// Valid complete new segment; namespace publication is a separate operation.
    pub segment: FinishedSegment<W>,
    /// Source prefix and excluded suffix evidence.
    pub report: RecoveryReport,
}
/// Recover into an independent empty sink. The source is immutable and read-only;
/// the caller supplies the original session/revision/channel registrations.
/// Failures can leave an incomplete sink but never modify the source or retry IO.
pub fn recover_segment<R: Read + Seek, W: SyncSink>(
    source: &mut R,
    budget: WriterBudget,
    channels: Vec<StorageChannel>,
    options: RecoveryOptions,
    sink: W,
    cancel: &AtomicBool,
) -> Result<RecoveredSegment<W>, RecoveryError> {
    recover_at(
        source,
        budget,
        channels,
        options,
        sink,
        cancel,
        options.source_sequence,
    )
}
fn recover_at<R: Read + Seek, W: SyncSink>(
    source: &mut R,
    budget: WriterBudget,
    channels: Vec<StorageChannel>,
    options: RecoveryOptions,
    sink: W,
    cancel: &AtomicBool,
    output_sequence: u64,
) -> Result<RecoveredSegment<W>, RecoveryError> {
    let mut report = RecoveryReport::empty();
    let result = recover_inner(
        source,
        budget,
        channels,
        (options, output_sequence),
        sink,
        cancel,
        &mut report,
    );
    result
        .map(|segment| RecoveredSegment { segment, report })
        .map_err(|error| {
            if report.stop != RecoveryStop::Cancelled {
                report.stop = if matches!(
                    error,
                    Some(StorageError::NeedRotation) | Some(StorageError::Budget { .. })
                ) {
                    RecoveryStop::Limit
                } else {
                    RecoveryStop::Failed
                };
            }
            RecoveryError { error, report }
        })
}
/// Create a fresh independent output, recover and sync it, close the file, then
/// finalize its name and sync the directory. No catalog is published on failure.
/// Store implementations must honor SegmentStore's exclusive/no-overwrite contract.
/// A failed publication can leave an active or renamed file for later investigation;
/// it is neither deleted nor silently retried. Host adapter allocations are separate.
pub fn recover_to_store<R: Read + Seek, T: SegmentStore>(
    source: &mut R,
    budget: WriterBudget,
    channels: Vec<StorageChannel>,
    options: RecoveryOptions,
    store: &mut T,
    sequence: u64,
    cancel: &AtomicBool,
) -> Result<RecoveryReport, RecoveryError> {
    let mut report = RecoveryReport::empty();
    let fail = |error| RecoveryError {
        error: Some(error),
        report,
    };
    if cancel.load(Ordering::Acquire) {
        report.stop = RecoveryStop::Cancelled;
        return Err(RecoveryError {
            error: None,
            report,
        });
    }
    if !store.directory_sync_supported() {
        return Err(fail(StorageError::Io(
            std::io::ErrorKind::Unsupported.into(),
        )));
    }
    // Validate configuration before creating an output namespace entry.
    options
        .reservation(&channels, budget.producers)
        .map_err(fail)?;
    let sink = store
        .create(sequence)
        .map_err(|e| fail(StorageError::Io(e)))?;
    let recovered = recover_at(source, budget, channels, options, sink, cancel, sequence)?;
    report = recovered.report;
    // Close the file before rename; do not rely on platform-specific open-file rename.
    drop(recovered.segment.into_inner());
    if cancel.load(Ordering::Acquire) {
        report.stop = RecoveryStop::Cancelled;
        return Err(RecoveryError {
            error: None,
            report,
        });
    }
    let result = store
        .finalize(sequence)
        .and_then(|()| store.sync_directory());
    result.map(|()| report).map_err(|e| {
        report.stop = RecoveryStop::Failed;
        RecoveryError {
            error: Some(StorageError::Io(e)),
            report,
        }
    })
}
impl RecoveryReport {
    fn empty() -> Self {
        Self {
            source_bytes: 0,
            verified_end: 0,
            excluded: (0, 0),
            chunks: 0,
            records: 0,
            loss_reports: 0,
            stop: RecoveryStop::Failed,
        }
    }
}
impl RecoveryOptions {
    /// Conservative extra recovery reservation, in addition to WriterConfig's
    /// writer reservation. Covers a fixed body buffer, bounded decode/metadata
    /// temporaries and source registries; source/sink adapter memory is separate.
    pub fn reservation(
        &self,
        channels: &[StorageChannel],
        producers: usize,
    ) -> Result<usize, StorageError> {
        self.writer.reservation(channels, producers)?;
        if self.limits.max_source_bytes < 8
            || self.limits.max_record_bytes < 8
            || self.limits.max_chunks == 0
        {
            return Err(StorageError::InvalidConfig);
        }
        self.limits
            .max_record_bytes
            .checked_mul(4)
            .and_then(|n| n.checked_add(self.writer.max_metadata_bytes.checked_mul(16)?))
            .and_then(|n| n.checked_add(channels.len().checked_mul(64)?))
            .and_then(|n| n.checked_add(producers.checked_mul(16)?))
            .and_then(|n| n.checked_add(65536))
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(StorageError::InvalidConfig)
    }
}
fn cancelled(cancel: &AtomicBool, report: &mut RecoveryReport) -> Result<(), Option<StorageError>> {
    if cancel.load(Ordering::Acquire) {
        report.stop = RecoveryStop::Cancelled;
        Err(None)
    } else {
        Ok(())
    }
}
fn recover_inner<R: Read + Seek, W: SyncSink>(
    source: &mut R,
    mut budget: WriterBudget,
    channels: Vec<StorageChannel>,
    setup: (RecoveryOptions, u64),
    sink: W,
    cancel: &AtomicBool,
    report: &mut RecoveryReport,
) -> Result<FinishedSegment<W>, Option<StorageError>> {
    let (options, output_sequence) = setup;
    cancelled(cancel, report)?;
    let extra = options.reservation(&channels, budget.producers)?;
    // Declared before all recovery allocations so it outlives their deallocation.
    let _lease = budget.reserve(extra)?;
    report.source_bytes = source.seek(SeekFrom::End(0)).map_err(StorageError::Io)?;
    report.excluded = (0, report.source_bytes);
    if report.source_bytes > options.limits.max_source_bytes {
        return Err(Some(StorageError::InvalidRecord));
    }
    source.seek(SeekFrom::Start(0)).map_err(StorageError::Io)?;
    let mut magic = [0; 8];
    source.read_exact(&mut magic).map_err(StorageError::Io)?;
    if &magic != b"\x89MCAP0\r\n" {
        return Err(Some(StorageError::CorruptData));
    }
    let mut buffer = vec![0; options.limits.max_record_bytes];
    let mut position = 8u64;
    let mut data_crc = crc_update(!0, &magic);
    let header = read_frame(source, &mut buffer, report.source_bytes, &mut position)
        .map_err(StorageError::Io)?;
    let (op, len, header_bytes) = header.map_err(|_| Some(StorageError::CorruptData))?;
    let mut fields = Fields(&buffer[..len]);
    if op != 1
        || fields.string() != Ok(super::codec::PROFILE.as_bytes())
        || !fields
            .string()
            .is_ok_and(|s| std::str::from_utf8(s).is_ok())
        || !fields.0.is_empty()
    {
        return Err(Some(StorageError::CorruptData));
    }
    data_crc = crc_update(crc_update(data_crc, &header_bytes), &buffer[..len]);
    // Inspect only the fixed-size root identity before creating the new writer.
    // Recovery copies original cumulative reports; it never samples today's bank.
    budget.loss = None;
    let root_start = position;
    if let Ok((12, len, _)) = read_frame(source, &mut buffer, report.source_bytes, &mut position)
        .map_err(StorageError::Io)?
    {
        if let Ok(file) = super::FileInfo::parse(&buffer[..len]) {
            if file.session == budget.session
                && file.revision == budget.revision
                && file.producers == budget.producers
                && file.channels == channels.len()
                && file.sequence == options.source_sequence
                && options.source_target.is_none_or(|id| id == file.target)
            {
                budget.recovered_file = Some(file);
            }
        }
    }
    source
        .seek(SeekFrom::Start(root_start))
        .map_err(StorageError::Io)?;
    position = root_start;
    // Normalize moved caller registrations before retaining/copying them.
    let channels: Vec<_> = channels
        .into_iter()
        .map(|mut c| {
            c.topic = c.topic.into_boxed_str().into_string();
            c.payload_encoding = c.payload_encoding.into_boxed_str().into_string();
            if let Some(s) = &mut c.schema {
                s.name = s.name.take().map(|n| n.into_boxed_str().into_string());
                s.encoding = std::mem::take(&mut s.encoding)
                    .into_boxed_str()
                    .into_string();
                s.descriptor = std::mem::take(&mut s.descriptor)
                    .into_boxed_slice()
                    .into_vec();
            }
            c
        })
        .collect();
    let mut state = Validation::new(
        budget.session,
        budget.revision,
        std::borrow::Cow::Borrowed(&channels),
        budget.producers,
        options.writer.max_payload_bytes,
    );
    let target = TargetLease::enter(&budget)?;
    let mut writer = SegmentWriter::with_target(
        budget,
        options.writer,
        channels.clone(),
        sink,
        target,
        true,
        output_sequence,
    )?;
    'scan: loop {
        cancelled(cancel, report)?;
        if position == report.source_bytes {
            report.stop = RecoveryStop::EndOfInput;
            break;
        }
        let frame = read_frame(source, &mut buffer, report.source_bytes, &mut position)
            .map_err(StorageError::Io)?;
        let (op, len, header) = match frame {
            Ok(frame) => frame,
            Err(stop) => {
                report.stop = stop;
                break;
            }
        };
        let body = &buffer[..len];
        let result = match op {
            12 => state.metadata(body).and_then(|()| {
                if state.file_sequence() == Some(options.source_sequence)
                    && options
                        .source_target
                        .is_none_or(|id| state.file_info().is_some_and(|file| file.target == id))
                {
                    Ok(())
                } else {
                    Err(RecoveryStop::InvalidRecord)
                }
            }),
            3 | 4 => state.registration(op, body),
            6 => {
                if report.chunks == options.limits.max_chunks {
                    report.stop = RecoveryStop::Limit;
                    break;
                }
                match state.chunk(body) {
                    Ok(count) => {
                        // Bound the complete chunk before adopting any of its records.
                        let mut inspect =
                            Fields(chunk_data(body).map_err(|_| Some(StorageError::CorruptData))?);
                        let mut controls = 0usize;
                        while !inspect.0.is_empty() {
                            let (opcode, record) = inspect
                                .record()
                                .map_err(|_| Some(StorageError::CorruptData))?;
                            if opcode == 5 {
                                if state
                                    .loss_report(record)
                                    .map_err(|_| Some(StorageError::CorruptData))?
                                    .is_some()
                                {
                                    controls += 1;
                                } else {
                                    inspect
                                        .record()
                                        .map_err(|_| Some(StorageError::CorruptData))?;
                                }
                            }
                        }
                        if report
                            .records
                            .checked_add(count)
                            .is_none_or(|n| n > options.writer.max_records)
                            || !writer.recovery_chunk_fits(
                                chunk_data(body).map_err(|_| Some(StorageError::CorruptData))?,
                                &state,
                            )?
                            || report
                                .loss_reports
                                .checked_add(controls)
                                .is_none_or(|n| n > options.writer.max_loss_reports)
                        {
                            report.stop = RecoveryStop::Limit;
                            break 'scan;
                        }
                        // Validation of every nested message precedes the first append.
                        let mut nested = Fields(&body[40..]);
                        while !nested.0.is_empty() {
                            cancelled(cancel, report)?;
                            let (opcode, record) = nested
                                .record()
                                .map_err(|_| Some(StorageError::CorruptData))?;
                            if opcode == 5 {
                                if let Some(control) = state
                                    .loss_report(record)
                                    .map_err(|_| Some(StorageError::CorruptData))?
                                {
                                    writer.append_loss_report(&control)?;
                                    continue;
                                }
                                let (op, meta) = nested
                                    .record()
                                    .map_err(|_| Some(StorageError::CorruptData))?;
                                if op != 5 {
                                    return Err(Some(StorageError::CorruptData));
                                }
                                let stored = state
                                    .pair(record, meta)
                                    .map_err(|_| Some(StorageError::CorruptData))?;
                                writer.append(stored.record, stored.payload)?;
                            }
                        }
                        writer.flush_chunk()?;
                        report.chunks += 1;
                        report.records += count;
                        report.loss_reports += controls;
                        report.verified_end = position;
                        report.excluded = (position, report.source_bytes);
                        Ok(())
                    }
                    Err(stop) => Err(stop),
                }
            }
            // Source indexes are replaceable, never evidence of recovered messages.
            7 => Ok(()),
            15 => {
                report.stop =
                    if body.len() == 4 && (body == [0; 4] || body == (!data_crc).to_le_bytes()) {
                        RecoveryStop::DataEnd
                    } else {
                        RecoveryStop::Checksum
                    };
                break;
            }
            _ => Err(RecoveryStop::InvalidRecord),
        };
        if let Err(stop) = result {
            report.stop = stop;
            break;
        }
        data_crc = crc_update(crc_update(data_crc, &header), body);
    }
    if matches!(
        report.stop,
        RecoveryStop::EndOfInput | RecoveryStop::DataEnd
    ) && state.registrations_complete().is_err()
    {
        report.stop = RecoveryStop::InvalidRecord;
    }
    cancelled(cancel, report)?;
    let mut finished = writer.finish()?;
    finished.sync_recovered()?;
    cancelled(cancel, report)?;
    Ok(finished)
}
// Bounds are checked against the frozen source length and fixed buffer before IO
// or any file-controlled allocation. A short read after that is an actual IO error.
type Frame = Result<(u8, usize, [u8; 9]), RecoveryStop>;
pub(crate) fn read_frame<R: Read>(
    source: &mut R,
    buffer: &mut [u8],
    end: u64,
    pos: &mut u64,
) -> std::io::Result<Frame> {
    if end.saturating_sub(*pos) < 9 {
        return Ok(Err(RecoveryStop::Truncated));
    }
    let mut header = [0; 9];
    source.read_exact(&mut header)?;
    let len = u64::from_le_bytes(
        header[1..]
            .try_into()
            .map_err(|_| std::io::ErrorKind::InvalidData)?,
    );
    let next = match pos.checked_add(9).and_then(|p| p.checked_add(len)) {
        Some(n) => n,
        None => return Ok(Err(RecoveryStop::InvalidRecord)),
    };
    if next > end {
        return Ok(Err(RecoveryStop::Truncated));
    }
    let len = match usize::try_from(len) {
        Ok(n) if n <= buffer.len() => n,
        _ => return Ok(Err(RecoveryStop::Limit)),
    };
    source.read_exact(&mut buffer[..len])?;
    *pos = next;
    Ok(Ok((header[0], len, header)))
}
pub(crate) struct Fields<'a>(pub(crate) &'a [u8]);
impl<'a> Fields<'a> {
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], RecoveryStop> {
        let value = self.0.get(..n).ok_or(RecoveryStop::InvalidRecord)?;
        self.0 = &self.0[n..];
        Ok(value)
    }
    pub(crate) fn u16(&mut self) -> Result<u16, RecoveryStop> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| RecoveryStop::InvalidRecord)?,
        ))
    }
    pub(crate) fn u32(&mut self) -> Result<u32, RecoveryStop> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| RecoveryStop::InvalidRecord)?,
        ))
    }
    pub(crate) fn u64(&mut self) -> Result<u64, RecoveryStop> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| RecoveryStop::InvalidRecord)?,
        ))
    }
    pub(crate) fn string(&mut self) -> Result<&'a [u8], RecoveryStop> {
        let n = self.u32()? as usize;
        self.take(n)
    }
    pub(crate) fn record(&mut self) -> Result<(u8, &'a [u8]), RecoveryStop> {
        let op = self.take(1)?[0];
        let n = usize::try_from(self.u64()?).map_err(|_| RecoveryStop::InvalidRecord)?;
        Ok((op, self.take(n)?))
    }
}
pub(crate) struct Validation<'a> {
    pub(crate) session: u128,
    pub(crate) revision: u64,
    pub(crate) channels: std::borrow::Cow<'a, [StorageChannel]>,
    meta_schema: Option<u16>,
    meta_seen: bool,
    loss_schema: Option<u16>,
    loss_seen: bool,
    payload_schemas: Vec<Option<u16>>,
    seen: Vec<bool>,
    last: Vec<u64>,
    top_level_registrations: bool,
    chunk_seen: bool,
    max_payload: usize,
    file: Option<super::FileInfo>,
}
fn matches_text(actual: &[u8], expected: std::fmt::Arguments<'_>) -> bool {
    let mut buffer = [0; 256];
    let mut cursor = std::io::Cursor::new(buffer.as_mut_slice());
    std::io::Write::write_fmt(&mut cursor, expected).is_ok()
        && actual == &cursor.get_ref()[..cursor.position() as usize]
}
impl<'a> Validation<'a> {
    pub(crate) fn new(
        session: u128,
        revision: u64,
        channels: std::borrow::Cow<'a, [StorageChannel]>,
        producers: usize,
        max_payload: usize,
    ) -> Self {
        let n = channels.len();
        Self {
            session,
            revision,
            channels,
            meta_schema: None,
            meta_seen: false,
            loss_schema: None,
            loss_seen: false,
            payload_schemas: vec![None; n],
            seen: vec![false; n],
            last: vec![0; producers],
            top_level_registrations: false,
            chunk_seen: false,
            max_payload,
            file: None,
        }
    }
    pub(crate) fn file_info(&self) -> Option<super::FileInfo> {
        self.file
    }
    pub(crate) fn file_sequence(&self) -> Option<u64> {
        self.file.map(|file| file.sequence)
    }
    pub(crate) fn metadata(&mut self, body: &[u8]) -> Result<(), RecoveryStop> {
        let file = super::FileInfo::parse(body).map_err(|_| RecoveryStop::InvalidRecord)?;
        if self.file.is_some()
            || self.meta_schema.is_some()
            || self.payload_schemas.iter().any(Option::is_some)
            || file.session != self.session
            || file.revision != self.revision
            || file.producers != self.last.len()
            || file.channels != self.channels.len()
        {
            return Err(RecoveryStop::InvalidRecord);
        }
        self.file = Some(file);
        Ok(())
    }
    pub(crate) fn registration(&mut self, op: u8, body: &[u8]) -> Result<(), RecoveryStop> {
        if self.chunk_seen {
            return Err(RecoveryStop::InvalidRecord);
        }
        self.top_level_registrations = true;
        self.validate_registration(op, body)
    }
    pub(crate) fn validate_registration(
        &mut self,
        op: u8,
        body: &[u8],
    ) -> Result<(), RecoveryStop> {
        match op {
            3 => self.schema(body),
            4 => self.channel(body),
            _ => Err(RecoveryStop::InvalidRecord),
        }
    }
    pub(crate) fn registrations_complete(&self) -> Result<(), RecoveryStop> {
        if self
            .file
            .is_some_and(|file| self.loss_seen == (file.loss_rows > 0))
            && self.meta_seen
            && self.seen.iter().all(|seen| *seen)
        {
            Ok(())
        } else {
            Err(RecoveryStop::InvalidRecord)
        }
    }
    fn schema(&mut self, body: &[u8]) -> Result<(), RecoveryStop> {
        use super::codec::{META_DESCRIPTOR, META_SCHEMA};
        if self.file.is_none() {
            return Err(RecoveryStop::InvalidRecord);
        }
        let mut f = Fields(body);
        let id = f.u16()?;
        let name = f.string()?;
        let encoding = f.string()?;
        let data = f.string()?;
        if id == 0 || !f.0.is_empty() {
            return Err(RecoveryStop::InvalidRecord);
        }
        if name == crate::loss::SCHEMA.as_bytes()
            && encoding == b"protobuf"
            && data == crate::loss::DESCRIPTOR
        {
            if self.file.is_none_or(|file| file.loss_rows == 0)
                || self.loss_schema.is_some()
                || self.meta_schema == Some(id)
                || self.payload_schemas.contains(&Some(id))
            {
                return Err(RecoveryStop::InvalidRecord);
            }
            self.loss_schema = Some(id);
            return Ok(());
        }
        if name == META_SCHEMA.as_bytes() && encoding == b"protobuf" && data == META_DESCRIPTOR {
            if self.loss_schema == Some(id)
                || self.meta_schema.is_some()
                || self.payload_schemas.contains(&Some(id))
            {
                return Err(RecoveryStop::InvalidRecord);
            }
            self.meta_schema = Some(id);
            return Ok(());
        }
        if self.meta_schema == Some(id)
            || self.loss_schema == Some(id)
            || self.payload_schemas.contains(&Some(id))
        {
            return Err(RecoveryStop::InvalidRecord);
        }
        let mut found = false;
        for (i, c) in self.channels.iter().enumerate() {
            let matches = c.schema.as_ref().is_some_and(|s| {
                let named = match &s.name {
                    Some(n) => name == n.as_bytes(),
                    None => matches_text(
                        name,
                        format_args!(
                            "payload.{:032x}.v{}",
                            c.channel.schema_id, c.channel.schema_version
                        ),
                    ),
                };
                named && encoding == s.encoding.as_bytes() && data == s.descriptor
            });
            if matches {
                if self.payload_schemas[i].is_some() {
                    return Err(RecoveryStop::InvalidRecord);
                }
                self.payload_schemas[i] = Some(id);
                found = true;
            }
        }
        if found {
            Ok(())
        } else {
            Err(RecoveryStop::InvalidRecord)
        }
    }
    fn channel(&mut self, body: &[u8]) -> Result<(), RecoveryStop> {
        let mut f = Fields(body);
        let id = f.u16()?;
        let schema = f.u16()?;
        if usize::from(id) == self.channels.len() + 2 {
            if self.loss_seen
                || self.file.is_none_or(|file| file.loss_rows == 0)
                || Some(schema) != self.loss_schema
                || f.string()? != super::codec::LOSS_TOPIC.as_bytes()
                || f.string()? != b"protobuf"
                || !f.string()?.is_empty()
                || !f.0.is_empty()
            {
                return Err(RecoveryStop::InvalidRecord);
            }
            self.loss_seen = true;
            return Ok(());
        }
        if usize::from(id) == self.channels.len() + 1 {
            if self.meta_seen
                || Some(schema) != self.meta_schema
                || f.string()? != super::codec::META_TOPIC.as_bytes()
                || f.string()? != b"protobuf"
                || !f.string()?.is_empty()
                || !f.0.is_empty()
            {
                return Err(RecoveryStop::InvalidRecord);
            }
            self.meta_seen = true;
            return Ok(());
        }
        let i = usize::from(id)
            .checked_sub(1)
            .ok_or(RecoveryStop::InvalidRecord)?;
        let c = self.channels.get(i).ok_or(RecoveryStop::InvalidRecord)?;
        let expected_schema = if c.schema.is_some() {
            self.payload_schemas[i].ok_or(RecoveryStop::InvalidRecord)?
        } else {
            0
        };
        if self.seen[i]
            || self.file.is_none()
            || schema != expected_schema
            || f.string()? != c.topic.as_bytes()
            || f.string()? != c.payload_encoding.as_bytes()
        {
            return Err(RecoveryStop::InvalidRecord);
        }
        let mut meta = Fields(f.string()?);
        let mut seen = 0u8;
        let named = c.schema.as_ref().and_then(|s| s.name.as_deref());
        while !meta.0.is_empty() {
            let key = meta.string()?;
            let value = meta.string()?;
            let (bit, matches) = match key {
                b"session_id" => (
                    1,
                    matches_text(value, format_args!("{:032x}", self.session)),
                ),
                b"revision" => (2, matches_text(value, format_args!("{}", self.revision))),
                b"product_channel" => (
                    4,
                    matches_text(value, format_args!("{}", c.channel.channel_id)),
                ),
                b"payload_schema_id" => (
                    8,
                    matches_text(value, format_args!("{:032x}", c.channel.schema_id)),
                ),
                b"payload_schema_version" => (
                    16,
                    matches_text(value, format_args!("{}", c.channel.schema_version)),
                ),
                b"payload_encoding" => (32, value == c.payload_encoding.as_bytes()),
                b"payload_schema_name" if named.is_some() => {
                    (128, Some(value) == named.map(str::as_bytes))
                }
                _ => return Err(RecoveryStop::InvalidRecord),
            };
            if seen & bit != 0 || !matches {
                return Err(RecoveryStop::InvalidRecord);
            }
            seen |= bit;
        }
        if seen != (63 | if named.is_some() { 128 } else { 0 }) || !f.0.is_empty() {
            return Err(RecoveryStop::InvalidRecord);
        }
        self.seen[i] = true;
        Ok(())
    }
    pub(crate) fn pair<'b>(
        &self,
        user: &'b [u8],
        meta: &[u8],
    ) -> Result<super::StoredRecord<'b>, RecoveryStop> {
        if self.file.is_none() || !self.meta_seen {
            return Err(RecoveryStop::InvalidRecord);
        }
        let mut u = Fields(user);
        let id = u.u16()?;
        let seq = u.u32()?;
        let log = u.u64()?;
        let publish = u.u64()?;
        let i = usize::from(id)
            .checked_sub(1)
            .ok_or(RecoveryStop::InvalidRecord)?;
        let c = self.channels.get(i).ok_or(RecoveryStop::InvalidRecord)?;
        if !self.seen[i] || u.0.len() > self.max_payload {
            return Err(RecoveryStop::InvalidRecord);
        }
        let mut m = Fields(meta);
        if usize::from(m.u16()?) != self.channels.len() + 1
            || m.u32()? != seq
            || m.u64() != Ok(log)
            || m.u64() != Ok(log)
            || publish != log
        {
            return Err(RecoveryStop::InvalidRecord);
        }
        let r = super::codec::decode_meta(m.0, self.session, id, c, u.0.len())
            .map_err(|_| RecoveryStop::InvalidRecord)?;
        if r.revision != self.revision
            || r.receipt.producer_id as usize >= self.last.len()
            || r.metadata.real_ns as u64 != log
            || r.receipt.accepted_seq as u32 != seq
        {
            return Err(RecoveryStop::InvalidRecord);
        }
        Ok(super::StoredRecord {
            record: r,
            payload: u.0,
        })
    }
    pub(crate) fn loss_report(
        &self,
        body: &[u8],
    ) -> Result<Option<crate::loss::LossReport>, RecoveryStop> {
        let mut f = Fields(body);
        if usize::from(f.u16()?) != self.channels.len() + 2 {
            return Ok(None);
        }
        let file = self
            .file
            .filter(|f| f.loss_rows > 0)
            .ok_or(RecoveryStop::InvalidRecord)?;
        if !self.loss_seen {
            return Err(RecoveryStop::InvalidRecord);
        }
        let sequence = f.u32()?;
        let log = f.u64()?;
        let publish = f.u64()?;
        let report =
            crate::loss::LossReport::decode(f.0).map_err(|_| RecoveryStop::InvalidRecord)?;
        let expected_parts = file.loss_rows.div_ceil(crate::loss::ROWS_PER_REPORT);
        if report.session != self.session
            || report.observer != file.target
            || report.parts as usize != expected_parts
            || report.checkpoint as u32 != sequence
            || report.publish_time_unix_ns != log
            || log != publish
            || report.counts.len()
                != if report.part as usize + 1 == expected_parts {
                    file.loss_rows - (expected_parts - 1) * crate::loss::ROWS_PER_REPORT
                } else {
                    crate::loss::ROWS_PER_REPORT
                }
            || report.counts.iter().any(|r| {
                r.producer.is_some_and(|p| p as usize >= self.last.len())
                    || r.channel
                        .is_some_and(|id| !self.channels.iter().any(|c| c.channel.channel_id == id))
            })
        {
            return Err(RecoveryStop::InvalidRecord);
        }
        Ok(Some(report))
    }
    pub(crate) fn chunk(&mut self, body: &[u8]) -> Result<usize, RecoveryStop> {
        if self.top_level_registrations && !self.chunk_seen {
            self.registrations_complete()?;
        }
        let legacy_prefix = !self.top_level_registrations && !self.chunk_seen;
        let data = chunk_data(body)?;
        let mut f = Fields(data);
        let mut count = 0;
        let mut range: Option<(u64, u64)> = None;
        while !f.0.is_empty() {
            let (op, body) = f.record()?;
            match op {
                3 | 4 if legacy_prefix && range.is_none() => {
                    self.validate_registration(op, body)?
                }
                5 => {
                    if legacy_prefix && range.is_none() {
                        self.registrations_complete()?;
                    }
                    if let Some(report) = self.loss_report(body)? {
                        let time = report.publish_time_unix_ns;
                        range =
                            Some(range.map_or((time, time), |(a, b)| (a.min(time), b.max(time))));
                        continue;
                    }
                    let (op, meta) = f.record()?;
                    if op != 5 {
                        return Err(RecoveryStop::InvalidRecord);
                    }
                    let r = self.pair(body, meta)?.record;
                    let p = r.receipt.producer_id as usize;
                    if r.receipt.accepted_seq <= self.last[p] {
                        return Err(RecoveryStop::InvalidRecord);
                    }
                    self.last[p] = r.receipt.accepted_seq;
                    let time = r.metadata.real_ns as u64;
                    range = Some(range.map_or((time, time), |(a, b)| (a.min(time), b.max(time))));
                    count += 1;
                }
                _ => return Err(RecoveryStop::InvalidRecord),
            }
        }
        if legacy_prefix {
            self.registrations_complete()?;
        }
        self.chunk_seen = true;
        let mut h = Fields(body);
        if range.unwrap_or((0, 0)) != (h.u64()?, h.u64()?) {
            return Err(RecoveryStop::InvalidRecord);
        }
        Ok(count)
    }
}
pub(crate) fn chunk_data(body: &[u8]) -> Result<&[u8], RecoveryStop> {
    if body.len() < 40 {
        return Err(RecoveryStop::Truncated);
    }
    let mut f = Fields(body);
    f.u64()?;
    f.u64()?;
    let size = f.u64()?;
    let crc = f.u32()?;
    if !f.string()?.is_empty() {
        return Err(RecoveryStop::UnsupportedCompression);
    }
    let stored = f.u64()?;
    if size != stored || stored != f.0.len() as u64 {
        return Err(RecoveryStop::InvalidRecord);
    }
    if crc == 0 || !crc_update(!0, f.0) != crc {
        return Err(RecoveryStop::Checksum);
    }
    Ok(f.0)
}
// IEEE CRC-32, also used by the pinned MCAP writer. A fixed table keeps large
// management-path scans linear without a per-byte eight-step interpreter loop.
const CRC_TABLE: [u32; 256] = {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};
pub(crate) fn crc_update(mut crc: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        crc = (crc >> 8) ^ CRC_TABLE[usize::from(crc as u8 ^ byte)];
    }
    crc
}
