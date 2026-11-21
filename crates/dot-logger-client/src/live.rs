use crate::{
    Cancellation, ClientError, PendingRecord, ReceiverConfig, RemoteClient, RemoteConfig,
    RemoteEvent, RemoteSubscription,
};
use dot_logger::{
    adapters::remote::{OverflowPolicy, RemoteError, SubscriptionFilter},
    capture::{Level, RecordMetadata},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    query::QueryBudget,
};

/// Shared receiver reservation for applications that only consume remote live data.
pub struct LiveBudget(QueryBudget);

impl LiveBudget {
    /// Set the total receiver storage available to live subscriptions, including
    /// reassembly bookkeeping. Transport and downstream queues have separate limits.
    pub fn new(bytes: usize) -> Result<Self, ClientError> {
        let invalid = ClientError::Remote(RemoteError::InvalidInput);
        let plan = SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: bytes.checked_add(8192).ok_or(invalid)?,
                input_bytes: 4096,
                raw_bytes: 0,
                output_bytes: 0,
                external_bytes: bytes,
                control_bytes: 0,
                worker_bytes: 4096,
            },
            producers: vec![ProducerQuota {
                small: PoolQuota {
                    slots: 1,
                    slot_bytes: 1,
                    important_slots: 0,
                },
                raw: None,
            }],
            importance: ImportancePolicy::TreatEqually,
            workers: WorkerThreadingConfig {
                count: 1,
                stack_bytes: 4096,
                cpu_ids: vec![],
                affinity: Requirement::Optional,
            },
            capabilities: vec![],
        })
        .validate(&PlatformCapabilities::default())
        .map_err(|_| invalid)?;
        Ok(Self(QueryBudget::new(&plan)))
    }
}

/// Metadata required to select and timestamp a live display channel.
#[derive(Clone, Copy)]
pub struct LiveRecord {
    /// Registered channel identifier.
    pub channel_id: u32,
    /// Registered schema identifier.
    pub schema_id: u128,
    /// Registered schema version.
    pub schema_version: u32,
    /// Original signed Unix time, in nanoseconds.
    pub real_ns: i64,
}

impl LiveRecord {
    pub(crate) fn from_metadata(metadata: RecordMetadata) -> Self {
        Self {
            channel_id: metadata.channel_id,
            schema_id: metadata.schema_id,
            schema_version: metadata.schema_version,
            real_ns: metadata.real_ns,
        }
    }
}

/// Own a filtered subscription and the remote connection that backs it.
pub struct LiveSubscription {
    stream: RemoteSubscription,
    _connection: RemoteClient,
}

impl LiveSubscription {
    /// Open a read-only stream for one native channel, with explicit original
    /// payload opt-in and the existing upstream Gap policy.
    pub async fn connect(
        config: RemoteConfig,
        budget: &LiveBudget,
        limits: ReceiverConfig,
        channel_id: u32,
        originals: bool,
        cancel: &Cancellation,
    ) -> Result<Self, ClientError> {
        let mut connection = RemoteClient::connect(config, cancel).await?;
        let stream = connection
            .subscribe(
                &budget.0,
                SubscriptionFilter {
                    channel: Some(channel_id),
                    minimum_level: Level::Trace,
                    originals,
                    overflow: OverflowPolicy::Gap,
                },
                limits,
                cancel,
            )
            .await?;
        Ok(Self {
            stream,
            _connection: connection,
        })
    }

    /// Await a complete record, upstream Gap, cancellation, or terminal stream
    /// event. Local polling deadlines are retried on this same connection.
    pub async fn next(
        &mut self,
        cancel: &Cancellation,
    ) -> Result<Option<RemoteEvent>, ClientError> {
        loop {
            match self.stream.next(cancel).await {
                Err(ClientError::Remote(RemoteError::TimedOut)) => {}
                other => return other,
            }
        }
    }

    /// Offer a verified record to a bounded consumer. Only acceptance can
    /// produce a receiving acknowledgement on the original connection.
    pub async fn handoff(
        &mut self,
        record: PendingRecord,
        consumer: impl FnOnce(LiveRecord, &[u8]) -> bool,
        cancel: &Cancellation,
    ) -> Result<(), ClientError> {
        self.stream
            .handoff(
                record,
                |record, bytes| consumer(LiveRecord::from_metadata(record.metadata), bytes),
                cancel,
            )
            .await
    }

    /// Send queued acknowledgements and wait until the server confirmed every
    /// handoff so far. A failure reports AcknowledgementUnknown.
    pub async fn flush(&mut self, cancel: &Cancellation) -> Result<(), ClientError> {
        self.stream.flush(cancel).await
    }
}
