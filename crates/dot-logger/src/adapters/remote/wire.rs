//! Checked-in protocol declarations; schema lives in proto/logger.proto.
use super::types::*;
use crate::{
    capture::*,
    completion::*,
    query::{Query, TimeBasis},
    replay::{ReplayState, Speed},
};
use prost::Message;
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Source {
    #[prost(sint64, tag = "1")]
    pub ticks: i64,
    #[prost(uint64, tag = "2")]
    pub clock_domain: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Record {
    #[prost(bytes = "vec", tag = "1")]
    pub session: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub producer: u32,
    #[prost(uint64, tag = "3")]
    pub sequence: u64,
    #[prost(uint32, tag = "4")]
    pub channel: u32,
    #[prost(bytes = "vec", tag = "5")]
    pub schema: Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub schema_version: u32,
    #[prost(uint64, tag = "7")]
    pub revision: u64,
    #[prost(uint64, tag = "8")]
    pub mono: u64,
    #[prost(sint64, tag = "9")]
    pub real: i64,
    #[prost(message, optional, tag = "10")]
    pub source: Option<Source>,
    #[prost(uint32, tag = "11")]
    pub level: u32,
    #[prost(uint32, tag = "12")]
    pub importance: u32,
    #[prost(uint64, tag = "13")]
    pub payload_length: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct QueryMessage {
    #[prost(uint32, tag = "1")]
    pub basis: u32,
    #[prost(uint64, tag = "2")]
    pub clock_domain: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub from: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub to: Vec<u8>,
    #[prost(uint32, optional, tag = "5")]
    pub channel: Option<u32>,
    #[prost(uint32, tag = "6")]
    pub minimum_level: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Request {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint64, tag = "2")]
    pub handle: u64,
    #[prost(uint64, tag = "3")]
    pub cursor: u64,
    #[prost(message, optional, tag = "4")]
    pub query: Option<QueryMessage>,
    #[prost(message, optional, tag = "5")]
    pub record: Option<Record>,
    #[prost(uint32, tag = "6")]
    pub target: u32,
    #[prost(uint32, tag = "7")]
    pub boundary: u32,
    #[prost(uint64, tag = "8")]
    pub offset: u64,
    #[prost(uint32, tag = "9")]
    pub length: u32,
    #[prost(uint64, tag = "10")]
    pub generation: u64,
    #[prost(uint32, tag = "11")]
    pub action: u32,
    #[prost(uint64, tag = "12")]
    pub from: u64,
    #[prost(uint32, tag = "13")]
    pub numerator: u32,
    #[prost(uint32, tag = "14")]
    pub denominator: u32,
    #[prost(bytes = "vec", tag = "15")]
    pub data: Vec<u8>,
    #[prost(uint64, tag = "16")]
    pub other: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Item {
    #[prost(message, optional, tag = "1")]
    pub record: Option<Record>,
    #[prost(bytes = "vec", tag = "2")]
    pub inline: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub blob: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Reply {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint32, tag = "2")]
    pub kind: u32,
    #[prost(bytes = "vec", tag = "3")]
    pub session: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub revision: u64,
    #[prost(uint64, tag = "5")]
    pub handle: u64,
    #[prost(uint64, tag = "6")]
    pub cursor: u64,
    #[prost(uint64, tag = "7")]
    pub generation: u64,
    #[prost(uint32, tag = "8")]
    pub state: u32,
    #[prost(message, repeated, tag = "9")]
    pub items: Vec<Item>,
    #[prost(bytes = "vec", tag = "10")]
    pub data: Vec<u8>,
    #[prost(uint32, tag = "11")]
    pub confirmed: u32,
    #[prost(uint32, tag = "12")]
    pub outcome: u32,
    #[prost(uint32, tag = "13")]
    pub reason: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct SubscribeRequest {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint32, optional, tag = "2")]
    pub channel: Option<u32>,
    #[prost(uint32, tag = "3")]
    pub minimum_level: u32,
    #[prost(bool, tag = "4")]
    pub originals: bool,
    #[prost(bool, tag = "5")]
    pub disconnect: bool,
    #[prost(uint32, tag = "6")]
    pub records: u32,
    #[prost(uint64, tag = "7")]
    pub bytes: u64,
    #[prost(uint64, tag = "8")]
    pub age_ms: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Frame {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint64, tag = "2")]
    pub subscription: u64,
    #[prost(uint64, tag = "3")]
    pub transfer: u64,
    #[prost(uint64, tag = "4")]
    pub generation: u64,
    #[prost(message, optional, tag = "5")]
    pub record: Option<Record>,
    #[prost(uint64, tag = "6")]
    pub total: u64,
    #[prost(uint64, tag = "7")]
    pub offset: u64,
    #[prost(uint32, tag = "8")]
    pub checksum: u32,
    #[prost(bytes = "vec", tag = "9")]
    pub data: Vec<u8>,
    #[prost(bool, tag = "10")]
    pub original: bool,
    #[prost(bool, tag = "11")]
    pub late: bool,
    #[prost(uint64, tag = "12")]
    pub gap_first: u64,
    #[prost(uint64, tag = "13")]
    pub gap_last: u64,
    #[prost(uint32, tag = "14")]
    pub kind: u32,
    #[prost(uint32, tag = "15")]
    pub gap_reason: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct AckRequest {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint64, tag = "2")]
    pub subscription: u64,
    #[prost(uint64, tag = "3")]
    pub transfer: u64,
    #[prost(uint64, tag = "4")]
    pub generation: u64,
    #[prost(bytes = "vec", tag = "5")]
    pub session: Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub producer: u32,
    #[prost(uint64, tag = "7")]
    pub sequence: u64,
}

pub(crate) fn version(major: u32) -> Result<(), RemoteError> {
    if major == PROTOCOL_MAJOR {
        Ok(())
    } else {
        Err(RemoteError::Version)
    }
}
fn id(bytes: &[u8]) -> Result<u128, RemoteError> {
    Ok(u128::from_le_bytes(
        bytes.try_into().map_err(|_| RemoteError::InvalidInput)?,
    ))
}
fn time(bytes: &[u8]) -> Result<i128, RemoteError> {
    Ok(i128::from_le_bytes(
        bytes.try_into().map_err(|_| RemoteError::InvalidInput)?,
    ))
}
pub(crate) fn level(value: u32) -> Result<Level, RemoteError> {
    Ok(match value {
        0 => Level::Trace,
        1 => Level::Debug,
        2 => Level::Info,
        3 => Level::Warn,
        4 => Level::Error,
        5 => Level::Fatal,
        _ => return Err(RemoteError::InvalidInput),
    })
}
pub(crate) fn record(r: CapturedRecord) -> Record {
    Record {
        session: r.receipt.session_id.to_le_bytes().to_vec(),
        producer: r.receipt.producer_id,
        sequence: r.receipt.accepted_seq,
        channel: r.metadata.channel_id,
        schema: r.metadata.schema_id.to_le_bytes().to_vec(),
        schema_version: r.metadata.schema_version,
        revision: r.revision,
        mono: r.metadata.mono_ns,
        real: r.metadata.real_ns,
        source: r.metadata.source_time.map(|s| Source {
            ticks: s.ticks,
            clock_domain: s.clock_domain,
        }),
        level: r.metadata.level as u32,
        importance: u32::from(r.metadata.importance == Some(Importance::Important)),
        payload_length: r.payload_len as u64,
    }
}
impl Record {
    #[cfg(test)]
    pub fn native(&self) -> Result<CapturedRecord, RemoteError> {
        let record = CapturedRecord {
            receipt: CaptureReceipt {
                session_id: id(&self.session)?,
                producer_id: self.producer,
                accepted_seq: self.sequence,
            },
            metadata: RecordMetadata {
                channel_id: self.channel,
                schema_id: id(&self.schema)?,
                schema_version: self.schema_version,
                mono_ns: self.mono,
                real_ns: self.real,
                source_time: self.source.as_ref().map(|s| SourceTime {
                    ticks: s.ticks,
                    clock_domain: s.clock_domain,
                }),
                level: level(self.level)?,
                importance: Some(match self.importance {
                    0 => Importance::Normal,
                    1 => Importance::Important,
                    _ => return Err(RemoteError::InvalidInput),
                }),
            },
            revision: self.revision,
            payload_len: usize::try_from(self.payload_length).map_err(|_| RemoteError::Limit)?,
        };
        if !valid_record(record) {
            return Err(RemoteError::InvalidInput);
        }
        Ok(record)
    }
}
impl Request {
    pub fn native(self, method: &str, max: usize) -> Result<ManagementRequest, RemoteError> {
        version(self.major)?;
        Ok(match method {
            "GetStatus" => ManagementRequest::GetStatus,
            "Query" => {
                let q = self.query.ok_or(RemoteError::InvalidInput)?;
                let basis = match q.basis {
                    0 => TimeBasis::Mono,
                    1 => TimeBasis::Real,
                    2 => TimeBasis::Source(q.clock_domain),
                    _ => return Err(RemoteError::InvalidInput),
                };
                let query = Query {
                    basis,
                    from: time(&q.from)?,
                    to: time(&q.to)?,
                    channel: q.channel,
                    minimum_level: level(q.minimum_level)?,
                };
                if query.from >= query.to {
                    return Err(RemoteError::InvalidInput);
                }
                ManagementRequest::Query {
                    snapshot: self.handle,
                    cursor: self.cursor,
                    query,
                }
            }
            "ReadBlob" => {
                if self.length == 0
                    || self.length as usize > max.saturating_sub(1024)
                    || self.offset.checked_add(u64::from(self.length)).is_none()
                {
                    return Err(RemoteError::InvalidInput);
                }
                ManagementRequest::ReadBlob {
                    snapshot: self.handle,
                    blob: self.other,
                    offset: self.offset,
                    length: self.length,
                }
            }
            "Replay" => ManagementRequest::Replay {
                handle: self.handle,
                action: match self.action {
                    0 => ReplayAction::Start,
                    1 => ReplayAction::Pause,
                    2 => ReplayAction::Resume,
                    3 => ReplayAction::Seek,
                    4 => ReplayAction::Stop,
                    5 => ReplayAction::Speed,
                    _ => return Err(RemoteError::InvalidInput),
                },
                generation: self.generation,
                from: self.from,
                speed: Speed::new(self.numerator, self.denominator)
                    .map_err(|_| RemoteError::InvalidInput)?,
            },
            "GetCompletion" => {
                let r = self.record.ok_or(RemoteError::InvalidInput)?;
                let receipt = CaptureReceipt {
                    session_id: id(&r.session)?,
                    producer_id: r.producer,
                    accepted_seq: r.sequence,
                };
                if receipt.accepted_seq == 0 {
                    return Err(RemoteError::InvalidInput);
                }
                ManagementRequest::GetCompletion {
                    receipt,
                    target: self.target,
                    boundary: match self.boundary {
                        0 => Boundary::Enqueued,
                        1 => Boundary::Written,
                        2 => Boundary::Synced,
                        3 => Boundary::DatabaseAcknowledged,
                        4 => Boundary::ReceiverAcknowledged,
                        _ => return Err(RemoteError::InvalidInput),
                    },
                }
            }
            "ApplyConfig" => ManagementRequest::ApplyConfig {
                base_revision: self.handle,
                patch: self.data,
            },
            "PlanCheck" => ManagementRequest::PlanCheck { plan: self.handle },
            "RunCheck" => ManagementRequest::RunCheck { plan: self.handle },
            "Compare" => ManagementRequest::Compare {
                before: self.handle,
                after: self.other,
            },
            _ => return Err(RemoteError::Unsupported),
        })
    }
}
impl Reply {
    pub fn native(
        reply: ManagementReply,
        method: &str,
        max: usize,
        max_records: usize,
    ) -> Result<Self, RemoteError> {
        reply.check_buffer_limit(max)?;
        let mut wire = Self {
            major: PROTOCOL_MAJOR,
            ..Self::default()
        };
        match (method, reply) {
            (
                "GetStatus",
                ManagementReply::Status {
                    session,
                    revision,
                    report,
                },
            ) => {
                if report.len() > max {
                    return Err(RemoteError::Limit);
                }
                wire.kind = 1;
                wire.session = session.to_le_bytes().to_vec();
                wire.revision = revision;
                wire.data = report;
            }
            (
                "Query",
                ManagementReply::Page {
                    snapshot,
                    cursor,
                    items,
                },
            ) => {
                if items.len() > max_records
                    || items.iter().any(|i| {
                        !valid_record(i.record)
                            || (i.blob == 0 && i.inline.len() != i.record.payload_len)
                            || (i.blob != 0 && !i.inline.is_empty())
                    })
                    || items
                        .iter()
                        .try_fold(0usize, |n, i| {
                            n.checked_add(i.inline.len())
                                .and_then(|v| v.checked_add(512))
                        })
                        .is_none_or(|n| n > max)
                {
                    return Err(RemoteError::Limit);
                }
                wire.kind = 2;
                wire.handle = snapshot;
                wire.cursor = cursor;
                wire.items = items
                    .into_iter()
                    .map(|i| Item {
                        record: Some(record(i.record)),
                        inline: i.inline,
                        blob: i.blob,
                    })
                    .collect();
            }
            ("ReadBlob", ManagementReply::Blob(data)) => {
                if data.len() > max {
                    return Err(RemoteError::Limit);
                }
                wire.kind = 3;
                wire.data = data;
            }
            ("Replay", ManagementReply::Replay { generation, state }) => {
                wire.kind = 4;
                wire.generation = generation;
                wire.state = match state {
                    ReplayState::Playing => 0,
                    ReplayState::Paused => 1,
                    ReplayState::Ended => 2,
                    ReplayState::Stopped => 3,
                };
            }
            ("GetCompletion", ManagementReply::Completion(status)) => {
                wire.kind = 5;
                for (i, b) in [
                    Boundary::Enqueued,
                    Boundary::Written,
                    Boundary::Synced,
                    Boundary::DatabaseAcknowledged,
                    Boundary::ReceiverAcknowledged,
                ]
                .into_iter()
                .enumerate()
                {
                    if status.confirmed.contains(b) {
                        wire.confirmed |= 1 << i;
                    }
                }
                wire.outcome = match status.outcome {
                    QueryOutcome::Complete => 0,
                    QueryOutcome::Pending => 1,
                    QueryOutcome::Failed(reason) => {
                        wire.reason = reason as u32;
                        2
                    }
                    QueryOutcome::Dropped(reason) => {
                        wire.reason = reason as u32;
                        3
                    }
                    QueryOutcome::Unknown => 4,
                    QueryOutcome::Expired => 5,
                    QueryOutcome::NotRouted => 6,
                    QueryOutcome::Busy => 7,
                };
            }
            ("ApplyConfig", ManagementReply::Applied(revision)) => {
                wire.kind = 6;
                wire.revision = revision;
            }
            ("PlanCheck" | "RunCheck" | "Compare", ManagementReply::Report(data)) => {
                if data.len() > max {
                    return Err(RemoteError::Limit);
                }
                wire.kind = 7;
                wire.data = data;
            }
            _ => return Err(RemoteError::InvalidInput),
        }
        if wire.encoded_len() > max {
            return Err(RemoteError::Limit);
        }
        Ok(wire)
    }
}
impl Frame {
    pub fn chunk(c: Chunk) -> Self {
        Self {
            major: PROTOCOL_MAJOR,
            subscription: c.subscription,
            transfer: c.transfer,
            generation: c.generation,
            record: Some(record(c.record)),
            total: c.total,
            offset: c.offset,
            checksum: c.checksum,
            data: c.data,
            original: c.original,
            late: c.late,
            gap_first: 0,
            gap_last: 0,
            kind: 1,
            gap_reason: 0,
        }
    }
    #[cfg(test)]
    pub fn native(self) -> Result<Chunk, RemoteError> {
        version(self.major)?;
        Ok(Chunk {
            subscription: self.subscription,
            transfer: self.transfer,
            generation: self.generation,
            record: self.record.ok_or(RemoteError::Corrupt)?.native()?,
            total: self.total,
            offset: self.offset,
            checksum: self.checksum,
            data: self.data,
            original: self.original,
            late: self.late,
        })
    }
}
impl AckRequest {
    pub fn native(self) -> Result<Ack, RemoteError> {
        version(self.major)?;
        Ok(Ack {
            subscription: self.subscription,
            transfer: self.transfer,
            generation: self.generation,
            receipt: CaptureReceipt {
                session_id: id(&self.session)?,
                producer_id: self.producer,
                accepted_seq: self.sequence,
            },
        })
    }
}
