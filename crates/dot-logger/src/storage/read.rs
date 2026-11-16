//! Borrowed native-file iteration. A whole chunk is checked before its first record.
use super::codec::{META_TOPIC, PROFILE};
use super::recovery::{Fields, Validation, chunk_data, crc_update};
use super::{PayloadSchema, StorageChannel, StorageError, StoredRecord};
use crate::capture::ChannelConfig;
use mcap::records::Record;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

/// Identity and registration bounds repeated in every native file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileInfo {
    /// Session identity, independent of file rotation.
    pub session: u128,
    /// Revision of application records in this file.
    pub revision: u64,
    /// Original file output identifier.
    pub target: u32,
    /// File sequence within that output; standalone writers start at zero.
    pub sequence: u64,
    /// Registered producer count.
    pub producers: usize,
    /// Registered application channel count; excludes control topics.
    pub channels: usize,
}
impl FileInfo {
    pub(crate) fn metadata(self) -> mcap::records::Metadata {
        mcap::records::Metadata {
            name: "dot_logger.file".into(),
            metadata: BTreeMap::from([
                ("format_version".into(), "1".into()),
                ("session_id".into(), format!("{:032x}", self.session)),
                ("revision".into(), self.revision.to_string()),
                ("target_id".into(), self.target.to_string()),
                ("segment_sequence".into(), self.sequence.to_string()),
                ("producers".into(), self.producers.to_string()),
                ("channels".into(), self.channels.to_string()),
                ("loss_tracking".into(), "unavailable".into()),
            ]),
        }
    }
    pub(crate) fn parse(body: &[u8]) -> Result<Self, StorageError> {
        // Eight fixed numeric/control fields fit below 1 KiB, even at their
        // maximum widths. Check before MCAP allocates its string/map temporaries:
        // hostile Metadata must stay inside the fixed control reservation.
        if body.len() > 1024 {
            return Err(StorageError::CorruptData);
        }
        let mut fields = Fields(body);
        if fields.string().map_err(corrupt)? != b"dot_logger.file" {
            return Err(StorageError::CorruptData);
        }
        let mut pairs = Fields(fields.string().map_err(corrupt)?);
        if !fields.0.is_empty() {
            return Err(StorageError::CorruptData);
        }
        let mut metadata = BTreeMap::new();
        while !pairs.0.is_empty() {
            if metadata.len() == 8 {
                return Err(StorageError::CorruptData);
            }
            let key = std::str::from_utf8(pairs.string().map_err(corrupt)?).map_err(corrupt)?;
            let value = std::str::from_utf8(pairs.string().map_err(corrupt)?).map_err(corrupt)?;
            if metadata.insert(key, value).is_some() {
                return Err(StorageError::CorruptData);
            }
        }
        let bad = || StorageError::CorruptData;
        let get = |key: &str| metadata.get(key).copied().ok_or_else(bad);
        if metadata.len() != 8
            || get("format_version")? != "1"
            || get("loss_tracking")? != "unavailable"
        {
            return Err(bad());
        }
        let session = get("session_id")?;
        if session.len() != 32 {
            return Err(bad());
        }
        let value = Self {
            session: u128::from_str_radix(session, 16).map_err(|_| bad())?,
            revision: get("revision")?.parse().map_err(|_| bad())?,
            target: get("target_id")?.parse().map_err(|_| bad())?,
            sequence: get("segment_sequence")?.parse().map_err(|_| bad())?,
            producers: get("producers")?.parse().map_err(|_| bad())?,
            channels: get("channels")?.parse().map_err(|_| bad())?,
        };
        if value.producers == 0 || value.channels == 0 || value.channels > u16::MAX as usize - 2 {
            return Err(bad());
        }
        Ok(value)
    }
}
fn corrupt(_: impl std::fmt::Debug) -> StorageError {
    StorageError::CorruptData
}
fn header(bytes: &[u8]) -> Result<Fields<'_>, StorageError> {
    if !bytes.starts_with(mcap::MAGIC) {
        return Err(StorageError::CorruptData);
    }
    let mut frames = Fields(&bytes[8..]);
    let (op, body) = frames.record().map_err(corrupt)?;
    let mut fields = Fields(body);
    if op != 1 {
        return Err(StorageError::CorruptData);
    }
    if fields.string().map_err(corrupt)? != PROFILE.as_bytes() {
        return Err(StorageError::UnsupportedFormat);
    }
    std::str::from_utf8(fields.string().map_err(corrupt)?).map_err(corrupt)?;
    if !fields.0.is_empty() {
        return Err(StorageError::CorruptData);
    }
    Ok(frames)
}
/// A bounded-memory view over caller-owned native file bytes.
///
/// Registration memory is bounded by the input prefix and u16 channel IDs;
/// producer state is capped by `max_producers`. Payload bytes are borrowed.
/// Unlike an ordinary MCAP message stream, this validates the complete chunk's
/// pairs before yielding any of them. A corrupt suffix ends with one error.
/// OS durability is not implied by an otherwise valid or truncated prefix.
pub struct NativeReader<'a> {
    frames: Fields<'a>,
    chunk: Fields<'a>,
    validation: Validation<'static>,
    info: FileInfo,
    crc: u32,
    ended: bool,
}
impl<'a> NativeReader<'a> {
    /// Read a native file, rejecting obsolete profiles, oversized payloads and
    /// producer registrations beyond the caller's bound before allocating them.
    pub fn new(
        bytes: &'a [u8],
        max_payload_bytes: usize,
        max_producers: usize,
    ) -> Result<Self, StorageError> {
        if max_payload_bytes == 0 || max_producers == 0 {
            return Err(StorageError::InvalidConfig);
        }
        let frames = header(bytes)?;
        let (info, channels) = registrations(frames.0, max_producers)?;
        let consumed = bytes.len() - frames.0.len();
        Ok(Self {
            frames,
            chunk: Fields(&[]),
            validation: Validation::new(
                info.session,
                info.revision,
                Cow::Owned(channels),
                info.producers,
                max_payload_bytes,
            ),
            info,
            crc: crc_update(!0, &bytes[..consumed]),
            ended: false,
        })
    }
    /// Self-described file identity; loss collection is unavailable until the
    /// dedicated checkpoint collector is enabled by its implementation stage.
    pub fn info(&self) -> FileInfo {
        self.info
    }
    /// Validated original application registrations in file-local channel order.
    pub fn channels(&self) -> &[StorageChannel] {
        &self.validation.channels
    }
    fn next_record(&mut self) -> Result<Option<StoredRecord<'a>>, StorageError> {
        loop {
            if !self.chunk.0.is_empty() {
                let (op, user) = self.chunk.record().map_err(corrupt)?;
                if op != 5 {
                    continue;
                }
                let (op, meta) = self.chunk.record().map_err(corrupt)?;
                if op != 5 {
                    return Err(StorageError::CorruptData);
                }
                return self.validation.pair(user, meta).map(Some).map_err(corrupt);
            }
            if self.frames.0.is_empty() {
                return Ok(None);
            }
            let before = self.frames.0;
            let (op, body) = self.frames.record().map_err(corrupt)?;
            match op {
                3 => self.validation.schema(body).map_err(corrupt)?,
                4 => self.validation.channel(body).map_err(corrupt)?,
                12 => self.validation.metadata(body).map_err(corrupt)?,
                6 => {
                    self.validation.chunk(body).map_err(corrupt)?;
                    // chunk() already verified the fixed uncompressed header and CRC.
                    self.chunk = Fields(&body[40..]);
                }
                7 => {}
                15 => {
                    if body.len() != 4 || (body != [0; 4] && body != (!self.crc).to_le_bytes()) {
                        return Err(StorageError::CorruptData);
                    }
                    return Ok(None);
                }
                _ => return Err(StorageError::CorruptData),
            }
            self.crc = crc_update(self.crc, &before[..before.len() - self.frames.0.len()]);
        }
    }
}
impl<'a> Iterator for NativeReader<'a> {
    type Item = Result<StoredRecord<'a>, StorageError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        match self.next_record() {
            Ok(Some(r)) => Some(Ok(r)),
            Ok(None) => {
                self.ended = true;
                None
            }
            Err(e) => {
                self.ended = true;
                Some(Err(e))
            }
        }
    }
}
fn registrations(
    bytes: &[u8],
    max_producers: usize,
) -> Result<(FileInfo, Vec<StorageChannel>), StorageError> {
    let mut outer = Fields(bytes);
    let mut info = None;
    let mut schemas = HashMap::new();
    let mut channels = BTreeMap::new();
    while !outer.0.is_empty() {
        let (op, body) = outer.record().map_err(corrupt)?;
        if op == 12 {
            let f = FileInfo::parse(body)?;
            if info.is_some()
                || f.producers > max_producers
                || f.producers > isize::MAX as usize / std::mem::size_of::<u64>()
            {
                return Err(StorageError::CorruptData);
            }
            info = Some(f);
            continue;
        }
        let data = if op == 6 {
            chunk_data(body).map_err(corrupt)?
        } else {
            continue;
        };
        let file = info.ok_or(StorageError::CorruptData)?;
        let mut nested = Fields(data);
        while !nested.0.is_empty() {
            let (op, body) = nested.record().map_err(corrupt)?;
            // Validate all file-controlled lengths with borrowed slices before
            // MCAP constructs owned strings or a descriptor. A small record with
            // a forged u32 string length must never drive a giant allocation.
            let mut bounds = Fields(body);
            match op {
                3 => {
                    bounds.u16().map_err(corrupt)?;
                    for _ in 0..3 {
                        bounds.string().map_err(corrupt)?;
                    }
                }
                4 => {
                    bounds.u16().map_err(corrupt)?;
                    bounds.u16().map_err(corrupt)?;
                    bounds.string().map_err(corrupt)?;
                    bounds.string().map_err(corrupt)?;
                    let mut map = Fields(bounds.string().map_err(corrupt)?);
                    while !map.0.is_empty() {
                        map.string().map_err(corrupt)?;
                        map.string().map_err(corrupt)?;
                    }
                }
                5 => break,
                _ => return Err(StorageError::CorruptData),
            }
            if !bounds.0.is_empty() {
                return Err(StorageError::CorruptData);
            }
            match mcap::parse_record(op, body).map_err(corrupt)? {
                Record::Schema { header, data } => {
                    if schemas.len() > file.channels + 1 {
                        return Err(StorageError::CorruptData);
                    }
                    schemas.insert(header.id, (header.name, header.encoding, data.into_owned()));
                }
                Record::Channel(c) => {
                    if c.topic == META_TOPIC {
                        continue;
                    }
                    if c.id == 0 || usize::from(c.id) > file.channels {
                        return Err(StorageError::CorruptData);
                    }
                    let get = |k: &str| c.metadata.get(k).ok_or(StorageError::CorruptData);
                    let channel = ChannelConfig::new(
                        get("product_channel")?.parse().map_err(corrupt)?,
                        u128::from_str_radix(get("payload_schema_id")?, 16).map_err(corrupt)?,
                        get("payload_schema_version")?.parse().map_err(corrupt)?,
                    );
                    let schema = if c.schema_id == 0 {
                        None
                    } else {
                        let (_name, encoding, data) =
                            schemas.get(&c.schema_id).ok_or(StorageError::CorruptData)?;
                        Some(PayloadSchema {
                            name: c.metadata.get("payload_schema_name").cloned(),
                            encoding: encoding.clone(),
                            descriptor: data.clone(),
                        })
                    };
                    channels.insert(
                        c.id,
                        StorageChannel {
                            channel,
                            topic: c.topic,
                            payload_encoding: c.message_encoding,
                            schema,
                        },
                    );
                }
                Record::Message { .. } => break,
                _ => return Err(StorageError::CorruptData),
            }
        }
        if channels.len() == file.channels {
            let channels: Vec<_> = channels.into_values().collect();
            super::writer::registration_bytes(&channels).map_err(corrupt)?;
            return Ok((file, channels));
        }
    }
    Err(StorageError::CorruptData)
}
