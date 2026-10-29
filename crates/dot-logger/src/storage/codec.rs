use super::*;
use crate::capture::CapturedRecord;
use crate::capture::{CaptureReceipt, Importance, Level, RecordMetadata, SourceTime};
use prost::Message;
pub(super) const OVERHEAD: usize = 512;
#[derive(Clone, PartialEq, Message)]
struct Source {
    #[prost(sint64, tag = "1")]
    ticks: i64,
    #[prost(uint64, tag = "2")]
    clock_domain: u64,
}
#[derive(Clone, PartialEq, Message)]
struct Envelope {
    #[prost(uint32, tag = "1")]
    major: u32,
    #[prost(bytes = "vec", tag = "2")]
    session: Vec<u8>,
    #[prost(uint32, tag = "3")]
    producer: u32,
    #[prost(uint64, tag = "4")]
    sequence: u64,
    #[prost(uint32, tag = "5")]
    channel: u32,
    #[prost(bytes = "vec", tag = "6")]
    schema: Vec<u8>,
    #[prost(uint32, tag = "7")]
    schema_version: u32,
    #[prost(uint64, tag = "8")]
    revision: u64,
    #[prost(uint64, tag = "9")]
    mono: u64,
    #[prost(sint64, tag = "10")]
    real: i64,
    #[prost(message, optional, tag = "11")]
    source: Option<Source>,
    #[prost(uint32, tag = "12")]
    level: u32,
    #[prost(uint32, tag = "13")]
    importance: u32,
    #[prost(uint64, tag = "14")]
    payload_length: u64,
    #[prost(bytes = "vec", tag = "20")]
    payload: Vec<u8>,
}
/// Decoded product identity and opaque payload; no schema conversion is implied.
#[derive(Debug)]
pub struct StoredRecord {
    /// Original accepted metadata and full-width receipt.
    pub record: CapturedRecord,
    /// Opaque application bytes.
    pub payload: Vec<u8>,
}
/// Encode one versioned product envelope into caller-owned scratch storage.
pub fn encode_record(
    record: CapturedRecord,
    payload: &[u8],
    out: &mut Vec<u8>,
    max: usize,
) -> Result<(), StorageError> {
    let m = record.metadata;
    if payload.len() != record.payload_len
        || payload.len() > max
        || m.real_ns < 0
        || record.receipt.accepted_seq == 0
    {
        return Err(StorageError::InvalidRecord);
    }
    let importance = match m.importance {
        Some(Importance::Normal) => 0,
        Some(Importance::Important) => 1,
        None => return Err(StorageError::InvalidRecord),
    };
    let header = Envelope {
        major: 1,
        session: record.receipt.session_id.to_le_bytes().to_vec(),
        producer: record.receipt.producer_id,
        sequence: record.receipt.accepted_seq,
        channel: m.channel_id,
        schema: m.schema_id.to_le_bytes().to_vec(),
        schema_version: m.schema_version,
        revision: record.revision,
        mono: m.mono_ns,
        real: m.real_ns,
        source: m.source_time.map(|s| Source {
            ticks: s.ticks,
            clock_domain: s.clock_domain,
        }),
        level: m.level as u32,
        importance,
        payload_length: payload.len() as u64,
        payload: Vec::new(),
    };
    let size = payload
        .len()
        .checked_add(OVERHEAD)
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(StorageError::InvalidRecord)?;
    out.clear();
    if out.capacity() < size {
        out.try_reserve_exact(size)
            .map_err(|_| StorageError::InvalidConfig)?;
    }
    header
        .encode(&mut *out)
        .map_err(|_| StorageError::InvalidRecord)?;
    // Append the bytes field directly, avoiding a second owned copy of an 8 MiB loan.
    out.extend_from_slice(&[0xa2, 0x01]);
    let mut length = payload.len() as u64;
    while length >= 128 {
        out.push((length as u8 & 127) | 128);
        length >>= 7;
    }
    out.push(length as u8);
    out.extend_from_slice(payload);
    Ok(())
}
/// Decode a bounded versioned envelope, preserving unknown user payload bytes.
pub fn decode_record(bytes: &[u8], max: usize) -> Result<StoredRecord, StorageError> {
    if bytes.len() > max.checked_add(OVERHEAD).ok_or(StorageError::CorruptData)? {
        return Err(StorageError::CorruptData);
    }
    let w = Envelope::decode(bytes).map_err(|_| StorageError::CorruptData)?;
    if w.major != 1 {
        return Err(StorageError::UnsupportedVersion(w.major));
    }
    if w.sequence == 0
        || w.real < 0
        || w.payload.len() > max
        || w.payload_length != w.payload.len() as u64
    {
        return Err(StorageError::CorruptData);
    }
    let session = u128::from_le_bytes(
        w.session
            .try_into()
            .map_err(|_| StorageError::CorruptData)?,
    );
    let schema = u128::from_le_bytes(w.schema.try_into().map_err(|_| StorageError::CorruptData)?);
    let level = match w.level {
        0 => Level::Trace,
        1 => Level::Debug,
        2 => Level::Info,
        3 => Level::Warn,
        4 => Level::Error,
        5 => Level::Fatal,
        _ => return Err(StorageError::CorruptData),
    };
    let importance = match w.importance {
        0 => Importance::Normal,
        1 => Importance::Important,
        _ => return Err(StorageError::CorruptData),
    };
    let record = CapturedRecord {
        receipt: CaptureReceipt {
            session_id: session,
            producer_id: w.producer,
            accepted_seq: w.sequence,
        },
        metadata: RecordMetadata {
            channel_id: w.channel,
            schema_id: schema,
            schema_version: w.schema_version,
            mono_ns: w.mono,
            real_ns: w.real,
            source_time: w.source.map(|s| SourceTime {
                ticks: s.ticks,
                clock_domain: s.clock_domain,
            }),
            level,
            importance: Some(importance),
        },
        revision: w.revision,
        payload_len: w.payload.len(),
    };
    Ok(StoredRecord {
        record,
        payload: w.payload,
    })
}
