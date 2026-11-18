//! Native pair metadata. Application bytes are never wrapped in a product envelope.
use super::{StorageChannel, StorageError};
use crate::capture::{
    CaptureReceipt, CapturedRecord, Importance, Level, RecordMetadata, SourceTime,
};
use prost::Message;

pub(super) const OVERHEAD: usize = 512;
pub(crate) const PROFILE: &str = "dot-logger/standard/2";
pub(crate) const FILE_FORMAT_VERSION: &str = "2";
pub(crate) const META_TOPIC: &str = ".logger/meta";
pub(crate) const LOSS_TOPIC: &str = ".logger/loss";
pub(crate) const META_SCHEMA: &str = "dot_logger.RecordMeta";
pub(crate) const META_DESCRIPTOR: &[u8] = include_bytes!("record_meta.bin");
#[derive(Clone, PartialEq, Message)]
struct Source {
    #[prost(sint64, tag = "1")]
    ticks: i64,
    #[prost(uint64, tag = "2")]
    clock_domain: u64,
}
#[derive(Clone, PartialEq, Message)]
struct Meta {
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
    source: Option<Source>,
}
/// One native logical record. The application payload borrows the source bytes.
#[derive(Debug)]
pub struct StoredRecord<'a> {
    /// Original product identity and clocks reconstructed from file/channel/meta.
    pub record: CapturedRecord,
    /// Original application bytes, without any container or product header.
    pub payload: &'a [u8],
}
pub(crate) fn validate_record(
    r: CapturedRecord,
    payload: &[u8],
    max: usize,
) -> Result<(), StorageError> {
    if r.payload_len != payload.len()
        || payload.len() > max
        || r.metadata.real_ns < 0
        || r.receipt.accepted_seq == 0
        || r.metadata.importance.is_none()
    {
        return Err(StorageError::InvalidRecord);
    }
    Ok(())
}
pub(crate) fn encode_meta(
    r: CapturedRecord,
    channel: u16,
    out: &mut Vec<u8>,
) -> Result<(), StorageError> {
    out.clear();
    Meta {
        channel: u32::from(channel),
        producer: r.receipt.producer_id,
        sequence: r.receipt.accepted_seq,
        mono: r.metadata.mono_ns,
        real: r.metadata.real_ns,
        revision: r.revision,
        level: r.metadata.level as u32,
        importance: u32::from(r.metadata.importance == Some(Importance::Important)),
        source: r.metadata.source_time.map(|s| Source {
            ticks: s.ticks,
            clock_domain: s.clock_domain,
        }),
    }
    .encode(out)
    .map_err(|_| StorageError::InvalidRecord)
}
pub(crate) fn decode_meta(
    bytes: &[u8],
    session: u128,
    channel: u16,
    c: &StorageChannel,
    len: usize,
) -> Result<CapturedRecord, StorageError> {
    if bytes.len() > OVERHEAD {
        return Err(StorageError::CorruptData);
    }
    let m = Meta::decode(bytes).map_err(|_| StorageError::CorruptData)?;
    if m.channel != u32::from(channel) || m.sequence == 0 || m.real < 0 {
        return Err(StorageError::CorruptData);
    }
    let level = match m.level {
        0 => Level::Trace,
        1 => Level::Debug,
        2 => Level::Info,
        3 => Level::Warn,
        4 => Level::Error,
        5 => Level::Fatal,
        _ => return Err(StorageError::CorruptData),
    };
    let importance = match m.importance {
        0 => Importance::Normal,
        1 => Importance::Important,
        _ => return Err(StorageError::CorruptData),
    };
    Ok(CapturedRecord {
        receipt: CaptureReceipt {
            session_id: session,
            producer_id: m.producer,
            accepted_seq: m.sequence,
        },
        metadata: RecordMetadata {
            channel_id: c.channel.channel_id,
            schema_id: c.channel.schema_id,
            schema_version: c.channel.schema_version,
            mono_ns: m.mono,
            real_ns: m.real,
            source_time: m.source.map(|s| SourceTime {
                ticks: s.ticks,
                clock_domain: s.clock_domain,
            }),
            level,
            importance: Some(importance),
        },
        revision: m.revision,
        payload_len: len,
    })
}
