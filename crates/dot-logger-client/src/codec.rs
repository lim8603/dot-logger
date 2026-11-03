use crate::wire;
use dot_logger::{
    adapters::remote::*, capture::*, completion::*, query::TimeBasis, replay::ReplayState,
    routing::DropReason,
};
use prost::Message;
use wire::Record;
fn id(bytes: &[u8]) -> Result<u128, RemoteError> {
    Ok(u128::from_le_bytes(
        bytes.try_into().map_err(|_| RemoteError::Corrupt)?,
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
        _ => return Err(RemoteError::Corrupt),
    })
}
impl Record {
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
                    _ => return Err(RemoteError::Corrupt),
                }),
            },
            revision: self.revision,
            payload_len: usize::try_from(self.payload_length).map_err(|_| RemoteError::Limit)?,
        };
        if record.receipt.accepted_seq == 0 {
            return Err(RemoteError::Corrupt);
        }
        Ok(record)
    }
}
pub(crate) fn method(request: &ManagementRequest) -> (&'static str, &'static str) {
    match request {
        ManagementRequest::GetStatus => ("GetStatus", "/dot_logger.remote.v1.Logger/GetStatus"),
        ManagementRequest::Query { .. } => ("Query", "/dot_logger.remote.v1.Logger/Query"),
        ManagementRequest::ReadBlob { .. } => ("ReadBlob", "/dot_logger.remote.v1.Logger/ReadBlob"),
        ManagementRequest::Replay { .. } => ("Replay", "/dot_logger.remote.v1.Logger/Replay"),
        ManagementRequest::GetCompletion { .. } => (
            "GetCompletion",
            "/dot_logger.remote.v1.Logger/GetCompletion",
        ),
        ManagementRequest::ApplyConfig { .. } => {
            ("ApplyConfig", "/dot_logger.remote.v1.Logger/ApplyConfig")
        }
        ManagementRequest::PlanCheck { .. } => {
            ("PlanCheck", "/dot_logger.remote.v1.Logger/PlanCheck")
        }
        ManagementRequest::RunCheck { .. } => ("RunCheck", "/dot_logger.remote.v1.Logger/RunCheck"),
        ManagementRequest::Compare { .. } => ("Compare", "/dot_logger.remote.v1.Logger/Compare"),
    }
}
pub(crate) fn request(native: ManagementRequest, max: usize) -> Result<wire::Request, RemoteError> {
    let mut r = wire::Request {
        major: PROTOCOL_MAJOR,
        ..wire::Request::default()
    };
    match native {
        ManagementRequest::GetStatus => {}
        ManagementRequest::Query {
            snapshot,
            cursor,
            query,
        } => {
            if query.from >= query.to {
                return Err(RemoteError::InvalidInput);
            }
            let (basis, clock_domain) = match query.basis {
                TimeBasis::Mono => (0, 0),
                TimeBasis::Real => (1, 0),
                TimeBasis::Source(domain) if domain != 0 => (2, domain),
                _ => return Err(RemoteError::InvalidInput),
            };
            r.handle = snapshot;
            r.cursor = cursor;
            r.query = Some(wire::QueryMessage {
                basis,
                clock_domain,
                from: query.from.to_le_bytes().to_vec(),
                to: query.to.to_le_bytes().to_vec(),
                channel: query.channel,
                minimum_level: query.minimum_level as u32,
            });
        }
        ManagementRequest::ReadBlob {
            snapshot,
            blob,
            offset,
            length,
        } => {
            if length == 0
                || length as usize > max.saturating_sub(1024)
                || offset.checked_add(length as u64).is_none()
            {
                return Err(RemoteError::InvalidInput);
            }
            r.handle = snapshot;
            r.other = blob;
            r.offset = offset;
            r.length = length;
        }
        ManagementRequest::Replay {
            handle,
            action,
            generation,
            from,
            speed,
        } => {
            r.handle = handle;
            r.action = match action {
                ReplayAction::Start => 0,
                ReplayAction::Pause => 1,
                ReplayAction::Resume => 2,
                ReplayAction::Seek => 3,
                ReplayAction::Stop => 4,
                ReplayAction::Speed => 5,
            };
            r.generation = generation;
            r.from = from;
            r.numerator = speed.numerator();
            r.denominator = speed.denominator();
        }
        ManagementRequest::GetCompletion {
            receipt,
            target,
            boundary,
        } => {
            if receipt.accepted_seq == 0 {
                return Err(RemoteError::InvalidInput);
            }
            r.record = Some(wire::Record {
                session: receipt.session_id.to_le_bytes().to_vec(),
                producer: receipt.producer_id,
                sequence: receipt.accepted_seq,
                ..wire::Record::default()
            });
            r.target = target;
            r.boundary = boundary as u32;
        }
        ManagementRequest::ApplyConfig {
            base_revision,
            patch,
        } => {
            r.handle = base_revision;
            r.data = patch;
        }
        ManagementRequest::PlanCheck { plan } | ManagementRequest::RunCheck { plan } => {
            r.handle = plan;
        }
        ManagementRequest::Compare { before, after } => {
            r.handle = before;
            r.other = after;
        }
    }
    if r.encoded_len() > max {
        return Err(RemoteError::Limit);
    }
    Ok(r)
}
pub(crate) fn version(major: u32) -> Result<(), RemoteError> {
    if major == PROTOCOL_MAJOR {
        Ok(())
    } else {
        Err(RemoteError::Version)
    }
}
pub(crate) fn reply(
    r: wire::Reply,
    method: &str,
    max: usize,
    blob_limit: Option<u32>,
) -> Result<ManagementReply, RemoteError> {
    version(r.major)?;
    if r.encoded_len() > max || r.items.len() > 1000 || r.data.len() > max {
        return Err(RemoteError::Limit);
    }
    Ok(match (method, r.kind) {
        ("GetStatus", 1) => ManagementReply::Status {
            session: id(&r.session)?,
            revision: r.revision,
            report: r.data,
        },
        ("Query", 2) => {
            if r.handle == 0
                || r.items
                    .len()
                    .saturating_mul(std::mem::size_of::<RemoteItem>())
                    > max.saturating_mul(2)
            {
                return Err(RemoteError::Corrupt);
            }
            let mut items = Vec::with_capacity(r.items.len());
            for item in r.items {
                let record = item.record.ok_or(RemoteError::Corrupt)?.native()?;
                if (item.blob == 0 && item.inline.len() != record.payload_len)
                    || (item.blob != 0 && !item.inline.is_empty())
                {
                    return Err(RemoteError::Corrupt);
                }
                items.push(RemoteItem {
                    record,
                    inline: item.inline,
                    blob: item.blob,
                });
            }
            ManagementReply::Page {
                snapshot: r.handle,
                cursor: r.cursor,
                items,
            }
        }
        ("ReadBlob", 3) => {
            if r.data.len() > blob_limit.ok_or(RemoteError::Corrupt)? as usize {
                return Err(RemoteError::Limit);
            }
            ManagementReply::Blob(r.data)
        }
        ("Replay", 4) => {
            if r.generation == 0 {
                return Err(RemoteError::Corrupt);
            }
            ManagementReply::Replay {
                generation: r.generation,
                state: match r.state {
                    0 => ReplayState::Playing,
                    1 => ReplayState::Paused,
                    2 => ReplayState::Ended,
                    3 => ReplayState::Stopped,
                    _ => return Err(RemoteError::Corrupt),
                },
            }
        }
        ("GetCompletion", 5) => {
            if r.confirmed & !31 != 0 {
                return Err(RemoteError::Corrupt);
            }
            let mut confirmed = Boundaries::default();
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
                if r.confirmed & (1 << i) != 0 {
                    confirmed = confirmed | Boundaries::only(b);
                }
            }
            let outcome = match r.outcome {
                0 => QueryOutcome::Complete,
                1 => QueryOutcome::Pending,
                2 => QueryOutcome::Failed(match r.reason {
                    0 => FailureReason::IoFailure,
                    1 => FailureReason::SyncFailure,
                    2 => FailureReason::AdapterFailure,
                    _ => return Err(RemoteError::Corrupt),
                }),
                3 => QueryOutcome::Dropped(match r.reason {
                    0 => DropReason::Capacity,
                    1 => DropReason::RetryExhausted,
                    2 => DropReason::PendingFull,
                    3 => DropReason::Expired,
                    4 => DropReason::Cancelled,
                    _ => return Err(RemoteError::Corrupt),
                }),
                4 => QueryOutcome::Unknown,
                5 => QueryOutcome::Expired,
                6 => QueryOutcome::NotRouted,
                7 => QueryOutcome::Busy,
                _ => return Err(RemoteError::Corrupt),
            };
            ManagementReply::Completion(RecordStatus { confirmed, outcome })
        }
        ("ApplyConfig", 6) => ManagementReply::Applied(r.revision),
        ("PlanCheck" | "RunCheck" | "Compare", 7) => ManagementReply::Report(r.data),
        _ => return Err(RemoteError::Corrupt),
    })
}
pub(crate) fn chunk(f: wire::Frame) -> Result<Chunk, RemoteError> {
    version(f.major)?;
    if f.kind != 1 || f.data.len() > MAX_CHUNK_BYTES {
        return Err(RemoteError::Corrupt);
    }
    Ok(Chunk {
        subscription: f.subscription,
        transfer: f.transfer,
        generation: f.generation,
        record: f.record.ok_or(RemoteError::Corrupt)?.native()?,
        total: f.total,
        offset: f.offset,
        checksum: f.checksum,
        data: f.data,
        original: f.original,
        late: f.late,
    })
}
pub(crate) fn ack(a: Ack) -> wire::AckRequest {
    wire::AckRequest {
        major: PROTOCOL_MAJOR,
        subscription: a.subscription,
        transfer: a.transfer,
        generation: a.generation,
        session: a.receipt.session_id.to_le_bytes().to_vec(),
        producer: a.receipt.producer_id,
        sequence: a.receipt.accepted_seq,
    }
}
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use dot_logger::replay::Speed;
    #[test]
    fn wrong_version_kind_metadata_and_blob_range_never_become_success() {
        assert_eq!(
            reply(
                wire::Reply {
                    major: 2,
                    kind: 1,
                    ..Default::default()
                },
                "GetStatus",
                4096,
                None
            )
            .unwrap_err(),
            RemoteError::Version
        );
        assert_eq!(
            reply(
                wire::Reply {
                    major: 1,
                    kind: 3,
                    ..Default::default()
                },
                "GetStatus",
                4096,
                None
            )
            .unwrap_err(),
            RemoteError::Corrupt
        );
        assert_eq!(
            reply(
                wire::Reply {
                    major: 1,
                    kind: 1,
                    session: vec![0; 15],
                    ..Default::default()
                },
                "GetStatus",
                4096,
                None
            )
            .unwrap_err(),
            RemoteError::Corrupt
        );
        assert_eq!(
            reply(
                wire::Reply {
                    major: 1,
                    kind: 3,
                    data: vec![1, 2],
                    ..Default::default()
                },
                "ReadBlob",
                4096,
                Some(1)
            )
            .unwrap_err(),
            RemoteError::Limit
        );
        assert!(
            reply(
                wire::Reply {
                    major: 1,
                    kind: 5,
                    confirmed: 32,
                    ..Default::default()
                },
                "GetCompletion",
                4096,
                None
            )
            .is_err()
        );
        let request = request(
            ManagementRequest::Replay {
                handle: 1,
                action: ReplayAction::Speed,
                generation: 1,
                from: 0,
                speed: Speed::new(3, 2).unwrap(),
            },
            4096,
        )
        .unwrap();
        assert_eq!((request.numerator, request.denominator), (3, 2));
    }
}
