//! Standard MCAP export (TASK-030 S1, API §21): a `dot-logger/1` file becomes
//! a file standard tools decode — each user channel carries the
//! application's own schema and encoding instead of `dot_logger.Envelope`.
//!
//! Mapping (API §21.1): the MCAP `sequence` is the low 32 bits of the
//! accepted sequence, `log_time` and `publish_time` are the real (wall) time.
//! Every user message is followed, at the same `log_time`, by one
//! `dot_logger.RecordMeta` message on [`META_TOPIC`] holding the full
//! sequence, producer, monotonic time, revision, level, importance and
//! source time. Channels without a descriptor are exported schemaless with
//! their payload encoding label; tools show them as raw bytes.
//!
//! The input is read once, front to back, with records bounded by
//! [`ExportLimits`]. A file that ends early or stops being readable (crash,
//! truncation, a chunk failing its CRC) exports the complete chunks before
//! that point and reports why in [`ExportReport::truncated`]; a chunk is
//! checked before any of its messages is written.

use super::decode_record;
use mcap::records::{MessageHeader, Record};
use mcap::sans_io::linear_reader::{LinearReadEvent, LinearReader, LinearReaderOptions};
use prost::Message;
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Topic of the per-message metadata channel in an exported file.
pub const META_TOPIC: &str = "_dot_logger/meta";
const META_SCHEMA: &str = "dot_logger.RecordMeta";
const META_DESCRIPTOR: &[u8] = include_bytes!("record_meta.bin");
const ENVELOPE_SCHEMA: &str = "dot_logger.Envelope";
const PROFILE: &str = "dot-logger/1";

#[derive(Clone, PartialEq, Message)]
struct RecordSource {
    #[prost(sint64, tag = "1")]
    ticks: i64,
    #[prost(uint64, tag = "2")]
    clock_domain: u64,
}
#[derive(Clone, PartialEq, Message)]
struct RecordMeta {
    #[prost(uint32, tag = "1")]
    channel: u32,
    #[prost(uint32, tag = "2")]
    producer: u32,
    #[prost(uint64, tag = "3")]
    sequence: u64,
    #[prost(uint64, tag = "4")]
    mono: u64,
    #[prost(sint64, tag = "5")]
    real: i64,
    #[prost(uint64, tag = "6")]
    revision: u64,
    #[prost(uint32, tag = "7")]
    level: u32,
    #[prost(uint32, tag = "8")]
    importance: u32,
    #[prost(message, optional, tag = "9")]
    source: Option<RecordSource>,
}

/// Bounds for reading the input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExportLimits {
    max_record_bytes: usize,
}
impl ExportLimits {
    /// Largest input record (a message, or a whole chunk, which is checked
    /// before its messages are exported) and largest user payload. Larger
    /// records fail with [`ExportError::RecordTooLarge`] instead of being
    /// buffered.
    pub const fn new(max_record_bytes: usize) -> Self {
        Self { max_record_bytes }
    }
}

/// One exported user channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportedChannel {
    /// Channel topic, unchanged.
    pub topic: String,
    /// MCAP message encoding: the channel's payload encoding label.
    pub message_encoding: String,
    /// Schema type name standard tools resolve; `None` when the file has no
    /// descriptor for the channel (exported schemaless, opaque bytes).
    pub schema_name: Option<String>,
    /// Messages exported on this channel.
    pub messages: u64,
}

/// Result of a completed export.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExportReport {
    /// User messages exported (each also has one meta message).
    pub messages: u64,
    /// User channels in input order.
    pub channels: Vec<ExportedChannel>,
    /// Why reading stopped before the end of a complete file; the messages
    /// before that point were exported. `None` for a complete input.
    pub truncated: Option<String>,
    /// Input metadata and attachment records, which are not exported.
    pub skipped_records: u64,
}

/// Why nothing usable could be exported.
#[derive(Debug)]
pub enum ExportError {
    /// The input is not a `dot-logger/1` file.
    NotDotLogger,
    /// A user channel uses [`META_TOPIC`].
    TopicConflict,
    /// An input record exceeds [`ExportLimits`].
    RecordTooLarge,
    /// `cancel` was set; the output is incomplete and must be discarded.
    Cancelled,
    /// A product record or channel does not match the `dot-logger/1` contract.
    InvalidRecord,
    /// Reading the input or writing the output failed.
    Io(std::io::Error),
    /// Writing the output container failed.
    Output(mcap::McapError),
}
impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "export failed: {e}"),
            Self::Output(e) => write!(f, "export output failed: {e}"),
            other => write!(f, "export failed: {other:?}"),
        }
    }
}
impl std::error::Error for ExportError {}
impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<mcap::McapError> for ExportError {
    fn from(e: mcap::McapError) -> Self {
        match e {
            mcap::McapError::Io(e) => Self::Io(e),
            e => Self::Output(e),
        }
    }
}

struct Mapped {
    out: u16,
    report: usize,
    input: mcap::records::Channel,
}

/// Export one `dot-logger/1` file to a standard MCAP file. `cancel` is
/// checked before every input read and every exported message.
///
/// On any `Err`, `output` may already contain bytes and must be discarded.
/// `ExportLimits` bounds individual records/chunks, not the aggregate size of
/// the schema/channel maps; use trusted input or impose an input-size bound.
pub fn export<R: Read, W: Write + Seek>(
    mut input: R,
    output: W,
    limits: ExportLimits,
    cancel: &AtomicBool,
) -> Result<ExportReport, ExportError> {
    let mut reader = LinearReader::new_with_options(
        LinearReaderOptions::default()
            .with_prevalidate_chunk_crcs(true)
            .with_record_length_limit(limits.max_record_bytes),
    );
    let mut writer: Option<mcap::Writer<W>> = None;
    let mut output = Some(output);
    let mut meta_channel = 0u16;
    let mut schemas: HashMap<u16, (String, String, Vec<u8>)> = HashMap::new();
    let mut channels: HashMap<u16, Mapped> = HashMap::new();
    let mut report = ExportReport::default();
    let mut scratch = Vec::new();
    while let Some(event) = reader.next_event() {
        if cancel.load(Ordering::Acquire) {
            return Err(ExportError::Cancelled);
        }
        let (opcode, data) = match event {
            Ok(LinearReadEvent::ReadRequest(n)) => {
                let read = input.read(reader.insert(n))?;
                reader.notify_read(read);
                continue;
            }
            Ok(LinearReadEvent::Record { data, opcode }) => (opcode, data),
            Err(
                mcap::McapError::RecordTooLarge { .. }
                | mcap::McapError::ChunkTooLarge(_)
                | mcap::McapError::ChunkBufferTooLarge(_),
            ) => return Err(ExportError::RecordTooLarge),
            Err(mcap::McapError::Io(e)) => return Err(ExportError::Io(e)),
            Err(e) if writer.is_some() => {
                report.truncated = Some(e.to_string());
                break;
            }
            Err(_) => return Err(ExportError::NotDotLogger),
        };
        let record = match mcap::parse_record(opcode, data) {
            Ok(record) => record,
            Err(e) if writer.is_some() => {
                report.truncated = Some(e.to_string());
                break;
            }
            Err(_) => return Err(ExportError::NotDotLogger),
        };
        match record {
            Record::Header(h) => {
                if writer.is_some() || h.profile != PROFILE {
                    return Err(ExportError::NotDotLogger);
                }
                let mut w = mcap::WriteOptions::new()
                    .compression(None)
                    .profile("")
                    .library("dot-logger/0.1.0 export --standard")
                    .create(output.take().ok_or(ExportError::InvalidRecord)?)?;
                let schema = w.add_schema(META_SCHEMA, "protobuf", META_DESCRIPTOR)?;
                meta_channel = w.add_channel(schema, META_TOPIC, "protobuf", &BTreeMap::new())?;
                writer = Some(w);
            }
            _ if writer.is_none() => return Err(ExportError::NotDotLogger),
            Record::Schema { header, data } => {
                let value = (header.name, header.encoding, data.into_owned());
                if schemas.get(&header.id).is_some_and(|old| *old != value) {
                    return Err(ExportError::InvalidRecord);
                }
                schemas.insert(header.id, value);
            }
            Record::Channel(c) => {
                if let Some(old) = channels.get(&c.id) {
                    if old.input != c {
                        return Err(ExportError::InvalidRecord);
                    }
                    continue;
                }
                let envelope = schemas.get(&c.schema_id);
                if !envelope.is_some_and(|(n, e, _)| n == ENVELOPE_SCHEMA && e == "protobuf")
                    || c.message_encoding != "protobuf"
                {
                    return Err(ExportError::NotDotLogger);
                }
                if c.topic == META_TOPIC {
                    return Err(ExportError::TopicConflict);
                }
                let encoding = c
                    .metadata
                    .get("payload_encoding")
                    .ok_or(ExportError::InvalidRecord)?
                    .clone();
                let payload_schema = match c.metadata.get("payload_schema_file_id") {
                    None if c.metadata.contains_key("payload_schema_name") => {
                        return Err(ExportError::InvalidRecord);
                    }
                    None => None,
                    Some(id) => {
                        let id: u16 = id.parse().map_err(|_| ExportError::InvalidRecord)?;
                        let (name, schema_encoding, data) =
                            schemas.get(&id).ok_or(ExportError::InvalidRecord)?;
                        if id == c.schema_id || name == ENVELOPE_SCHEMA {
                            return Err(ExportError::InvalidRecord);
                        }
                        let name = c.metadata.get("payload_schema_name").unwrap_or(name);
                        if name.is_empty() || name == ENVELOPE_SCHEMA {
                            return Err(ExportError::InvalidRecord);
                        }
                        Some((name.clone(), schema_encoding.clone(), data.clone()))
                    }
                };
                let metadata: BTreeMap<String, String> = c
                    .metadata
                    .iter()
                    .filter(|(k, _)| !matches!(k.as_str(), "payload_schema_file_id"))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let w = writer.as_mut().ok_or(ExportError::InvalidRecord)?;
                let schema = match &payload_schema {
                    None => 0,
                    Some((name, encoding, data)) => w.add_schema(name, encoding, data)?,
                };
                let out = w.add_channel(schema, &c.topic, &encoding, &metadata)?;
                channels.insert(
                    c.id,
                    Mapped {
                        out,
                        report: report.channels.len(),
                        input: c.clone(),
                    },
                );
                report.channels.push(ExportedChannel {
                    topic: c.topic.clone(),
                    message_encoding: encoding,
                    schema_name: payload_schema.map(|(name, _, _)| name),
                    messages: 0,
                });
            }
            Record::Message { header, data } => {
                let mapped = channels
                    .get(&header.channel_id)
                    .ok_or(ExportError::InvalidRecord)?;
                let stored = decode_record(&data, limits.max_record_bytes)
                    .map_err(|_| ExportError::InvalidRecord)?;
                let m = stored.record.metadata;
                let r = stored.record.receipt;
                let log_time = u64::try_from(m.real_ns).map_err(|_| ExportError::InvalidRecord)?;
                let out = MessageHeader {
                    channel_id: mapped.out,
                    sequence: r.accepted_seq as u32,
                    log_time,
                    publish_time: log_time,
                };
                let w = writer.as_mut().ok_or(ExportError::InvalidRecord)?;
                w.write_to_known_channel(&out, &stored.payload)?;
                scratch.clear();
                RecordMeta {
                    channel: u32::from(mapped.out),
                    producer: r.producer_id,
                    sequence: r.accepted_seq,
                    mono: m.mono_ns,
                    real: m.real_ns,
                    revision: stored.record.revision,
                    level: m.level as u32,
                    importance: match m.importance {
                        Some(crate::capture::Importance::Important) => 1,
                        _ => 0,
                    },
                    source: m.source_time.map(|s| RecordSource {
                        ticks: s.ticks,
                        clock_domain: s.clock_domain,
                    }),
                }
                .encode(&mut scratch)
                .map_err(|_| ExportError::InvalidRecord)?;
                w.write_to_known_channel(
                    &MessageHeader {
                        channel_id: meta_channel,
                        ..out
                    },
                    &scratch,
                )?;
                report.messages += 1;
                report.channels[mapped.report].messages += 1;
            }
            Record::Metadata(_) | Record::Attachment { .. } => report.skipped_records += 1,
            _ => {}
        }
    }
    let mut w = writer.ok_or(ExportError::NotDotLogger)?;
    w.finish()?;
    Ok(report)
}
