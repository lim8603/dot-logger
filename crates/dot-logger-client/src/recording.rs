use crate::{Cancellation, ClientError, LiveRecord, RemoteClient, RemoteConfig};
use dot_logger::{
    adapters::remote::{ManagementReply, ManagementRequest, RemoteError, RemoteItem},
    capture::Level,
    query::{Query, TimeBasis},
};
use std::sync::Arc;

/// Read pages and original payloads from one frozen remote snapshot.
/// The host owns snapshot expiry and reader cleanup.
pub struct RemoteRecording {
    client: RemoteClient,
    snapshot: u64,
    session: Option<u128>,
    chunk_bytes: usize,
    identity: Arc<()>,
}

/// One bounded page, including the opaque continuation chosen by the host.
pub struct RecordedPage {
    /// Zero at the end of the range.
    pub next_cursor: u64,
    /// Records in the host's monotonic query order.
    pub items: Vec<RecordedItem>,
}

/// Metadata and a payload capability tied to its originating connection.
pub struct RecordedItem {
    item: RemoteItem,
    owner: Arc<()>,
}

impl RecordedItem {
    /// Product channel, schema identity, and original wall time.
    pub fn metadata(&self) -> LiveRecord {
        LiveRecord::from_metadata(self.item.record.metadata)
    }

    /// Stable leading order components for a monotonic query within the snapshot.
    pub fn order_key(&self) -> (u64, u32, u64) {
        let record = &self.item.record;
        (
            record.metadata.mono_ns,
            record.receipt.producer_id,
            record.receipt.accepted_seq,
        )
    }
}

impl RemoteRecording {
    /// Create a read connection without opening a snapshot.
    /// Bound each blob range request separately from the protocol codec budget.
    pub async fn connect(
        config: RemoteConfig,
        blob_chunk_bytes: usize,
        cancel: &Cancellation,
    ) -> Result<Self, ClientError> {
        let chunk_bytes = blob_chunk_bytes.min(config.message_bytes.saturating_sub(1024));
        if chunk_bytes == 0 {
            return Err(ClientError::Remote(RemoteError::InvalidInput));
        }
        let client = RemoteClient::connect(config, cancel).await?;
        Ok(Self {
            client,
            snapshot: 0,
            session: None,
            chunk_bytes,
            identity: Arc::new(()),
        })
    }

    /// Read the monotonic half-open range. Start or seek with cursor zero.
    /// The first successful page binds this client to the returned snapshot.
    pub async fn page(
        &mut self,
        from_ns: u64,
        to_ns: u64,
        cursor: u64,
        cancel: &Cancellation,
    ) -> Result<RecordedPage, ClientError> {
        if from_ns >= to_ns || (self.snapshot == 0 && cursor != 0) {
            return Err(ClientError::Remote(RemoteError::InvalidInput));
        }
        let reply = self
            .client
            .request(
                ManagementRequest::Query {
                    snapshot: self.snapshot,
                    cursor,
                    query: Query {
                        basis: TimeBasis::Mono,
                        from: from_ns.into(),
                        to: to_ns.into(),
                        channel: None,
                        minimum_level: Level::Trace,
                    },
                },
                cancel,
            )
            .await?;
        let ManagementReply::Page {
            snapshot,
            cursor: next_cursor,
            items,
        } = reply
        else {
            return Err(corrupt());
        };
        if (self.snapshot != 0 && self.snapshot != snapshot)
            || (next_cursor != 0 && next_cursor == cursor)
        {
            return Err(corrupt());
        }
        let mut session = self.session;
        for item in &items {
            let record = item.record;
            if !(from_ns..to_ns).contains(&record.metadata.mono_ns)
                || session.is_some_and(|previous| previous != record.receipt.session_id)
            {
                return Err(corrupt());
            }
            session = Some(record.receipt.session_id);
        }
        self.snapshot = snapshot;
        self.session = session;
        Ok(RecordedPage {
            next_cursor,
            items: items
                .into_iter()
                .map(|item| RecordedItem {
                    item,
                    owner: self.identity.clone(),
                })
                .collect(),
        })
    }

    /// Read a complete original payload with an explicit caller size bound.
    /// Reject foreign items, zero progress and inconsistent byte counts.
    pub async fn payload(
        &mut self,
        item: RecordedItem,
        record_bytes: usize,
        cancel: &Cancellation,
    ) -> Result<Vec<u8>, ClientError> {
        if !Arc::ptr_eq(&item.owner, &self.identity) {
            return Err(ClientError::StaleConnection);
        }
        let RemoteItem {
            record,
            inline,
            blob,
        } = item.item;
        if record.payload_len > record_bytes {
            return Err(ClientError::Remote(RemoteError::Limit));
        }
        if blob == 0 {
            return Ok(inline);
        }
        if record.payload_len == 0 {
            return Err(corrupt());
        }
        let mut result = Vec::with_capacity(record.payload_len);
        while result.len() < record.payload_len {
            let length = (record.payload_len - result.len()).min(self.chunk_bytes);
            let ManagementReply::Blob(part) = self
                .client
                .request(
                    ManagementRequest::ReadBlob {
                        snapshot: self.snapshot,
                        blob,
                        offset: result.len() as u64,
                        length: u32::try_from(length).map_err(|_| corrupt())?,
                    },
                    cancel,
                )
                .await?
            else {
                return Err(corrupt());
            };
            if part.is_empty() || part.len() > length {
                return Err(corrupt());
            }
            result.extend(part);
        }
        Ok(result)
    }
}

fn corrupt() -> ClientError {
    ClientError::Remote(RemoteError::Corrupt)
}
