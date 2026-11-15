//! One explicit snapshot over production capture/routing and real loopback IO.
use dot_logger::{
    adapters::remote::*, capture::*, completion::*, config::*, platform::*, pool::RawRecordOutcome,
    query::QueryBudget, routing::*,
};
use dot_logger_client::{
    Cancellation, ClientError, ReceiverConfig, RemoteClient, RemoteConfig, RemoteEvent,
};
use std::{
    error::Error as StdError,
    fs::File,
    io::{self, Read},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Original general-path IO, configuration or transport failure.
pub type Error = Box<dyn StdError + Send + Sync>;

/// Maximum raw snapshot bytes, also the single source/queue/pool slot limit.
pub const MAX_SNAPSHOT_BYTES: usize = 64 * 1024;
pub const TARGET: u32 = 10;
const REVISION: u64 = 1;

/// Explicit receiver decision after complete assembly, before ACK.
#[derive(Clone, Copy)]
pub enum Action {
    /// The bounded consumer accepted ownership.
    Accept,
    /// The bounded consumer refused ownership; no ACK.
    Reject,
    #[cfg(test)]
    /// Fault injection: close the transport before handoff.
    Disconnect,
    #[cfg(test)]
    /// Fault injection: accept bytes then lose ACK confirmation.
    AcceptThenDisconnect,
}
/// Example limits; not logger defaults or a hardware capability profile.
pub struct Options {
    /// Whether a real receiving connection should be established.
    pub receiver: bool,
    /// Finite new-load admission interval, at most two seconds.
    pub duration: Duration,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            receiver: true,
            duration: Duration::from_secs(2),
        }
    }
}
/// Functional outcome for this one exported snapshot and loopback connection.
#[derive(Debug, PartialEq, Eq)]
pub enum BridgeOutcome {
    /// Original receipt acknowledged after consumer handoff.
    Pass,
    /// Operational failure; original error is retained.
    Fail,
    /// Explicit cancellation after actual IO settlement.
    Aborted,
    /// Missing receiver, exhausted budget or unconfirmed ACK.
    Inconclusive,
}
/// Actual ingress receipt and observed completion, without inferred capacity.
#[derive(Debug)]
pub struct BridgeReport {
    /// Original operational failure, never an inferred completion.
    pub issue: Option<Error>,
    /// Functional result for this one finite snapshot.
    pub outcome: BridgeOutcome,
    /// A new capture or handoff was actually stopped by its admission deadline.
    pub budget_expired: bool,
    /// Original admitted raw capture receipt.
    pub receipt: Option<CaptureReceipt>,
    /// Actual admitted production records.
    pub accepted: u64,
    /// Receipts confirmed at ReceiverAcknowledged.
    pub acknowledged: u64,
    /// Rejection or unconfirmed ACK after consumer invocation.
    pub handoff_error: Option<ClientError>,
    /// Completion observed after assembly and before handoff.
    pub before_handoff: Option<QueryOutcome>,
    /// Completion observed after delivery settlement.
    pub completion: Option<QueryOutcome>,
    /// Whether the original raw slot remained unavailable before ACK.
    pub raw_held_before_ack: bool,
    /// Raw slots actually reclaimed by dispatcher turns.
    pub raw_reclaimed: usize,
    /// Shared external reservations after actual server close.
    pub external_bytes_after_close: usize,
}
impl Default for BridgeReport {
    fn default() -> Self {
        Self {
            issue: None,
            budget_expired: false,
            outcome: BridgeOutcome::Inconclusive,
            receipt: None,
            accepted: 0,
            acknowledged: 0,
            handoff_error: None,
            before_handoff: None,
            completion: None,
            raw_held_before_ack: false,
            raw_reclaimed: 0,
            external_bytes_after_close: 0,
        }
    }
}
fn invalid(message: &'static str) -> Error {
    io::Error::new(io::ErrorKind::InvalidData, message).into()
}

/// Read one regular-file export, bounded even if it grows after metadata lookup.
/// The app's atomic rename means this handle reads one original inode. Its age,
/// boot/session identity and relationship to other snapshots are not inferred.
pub fn read_snapshot(path: &Path) -> Result<Vec<u8>, Error> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("snapshot must be a regular file"));
    }
    let file = File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > MAX_SNAPSHOT_BYTES as u64 {
        return Err(invalid("snapshot must be a bounded regular file"));
    }
    let mut bytes = Vec::with_capacity(MAX_SNAPSHOT_BYTES + 1);
    file.take(MAX_SNAPSHOT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    validate_snapshot(&bytes)?;
    Ok(bytes)
}
/// Check the app export shape without treating its counters as logger timing,
/// fresh operational Report evidence, or a new capacity Profile.
pub fn validate_snapshot(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() || bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(invalid("snapshot byte limit"));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let threads = value
        .get("threads")
        .and_then(|v| v.as_array())
        .ok_or_else(|| invalid("missing threads"))?;
    if threads.is_empty() || threads.len() > 128 {
        return Err(invalid("thread count limit"));
    }
    for thread in threads {
        if !thread
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty() && s.len() <= 128)
            || thread
                .get("interval_us")
                .and_then(|v| v.as_u64())
                .is_none_or(|v| v == 0)
            || thread.get("ticks").and_then(|v| v.as_u64()).is_none()
        {
            return Err(invalid("invalid thread observation"));
        }
    }
    Ok(())
}
fn config() -> Result<ValidatedConfig, Error> {
    let mib = 1024 * 1024;
    Ok(SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 16 * mib,
            input_bytes: mib,
            raw_bytes: mib,
            output_bytes: mib,
            external_bytes: 8 * mib,
            control_bytes: mib,
            worker_bytes: 4 * mib,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 2,
                slot_bytes: 16,
                important_slots: 0,
            },
            raw: Some(PoolQuota {
                slots: 1,
                slot_bytes: MAX_SNAPSHOT_BYTES,
                important_slots: 0,
            }),
        }],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 1024 * 1024,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())?)
}
fn routing() -> RoutingConfig {
    RoutingConfig {
        targets: vec![TargetConfig {
            id: TARGET,
            required: true,
            queue_slots: 1,
            pending_slots: 1,
            retained_bytes: MAX_SNAPSHOT_BYTES,
            max_attempts: 1,
            max_age: Duration::from_secs(2),
        }],
        routes: vec![Route {
            channel_id: 7,
            targets: vec![TARGET],
        }],
        shared_records: 1,
        small_copy_bytes: 16,
        schedule: Schedule {
            record_quantum: 1,
            byte_quantum: MAX_SNAPSHOT_BYTES,
            records_per_turn: 1,
            bytes_per_turn: MAX_SNAPSHOT_BYTES,
            retries_per_target: 1,
            reclaim_slots_per_producer: 1,
        },
    }
}
fn tracking() -> CompletionConfig {
    CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: 2,
        max_queries: 1,
        page_items: 1,
        feedback_events: 4,
        updates_per_turn: 4,
        reconcile_per_turn: 2,
        targets: vec![TargetContract {
            id: TARGET,
            required: true,
            supported: Boundaries::REMOTE,
            completion: Boundary::ReceiverAcknowledged,
        }],
    }
}

/// Synchronous ordinary-path host. No daemon, reconnect/retry, file writes or RT
/// thread hooks. The callback must synchronously try a bounded consuming queue;
/// Accept means queue ownership transferred, not that a screen was rendered.
/// Runtime/OS RSS is not measured by the explicit 16 MiB logger budget.
pub fn run(
    payload: &[u8],
    options: Options,
    cancelled: &AtomicBool,
    receiver: impl FnOnce(&CapturedRecord, &[u8]) -> Action,
) -> Result<BridgeReport, Error> {
    execute(
        payload,
        options,
        &|| cancelled.load(Ordering::Acquire),
        receiver,
        None,
    )
}
pub(super) struct Identity<'a> {
    pub session: u128,
    pub revision: u64,
    pub observe: &'a dyn Fn() -> Result<u64, Error>,
    pub admit: &'a dyn Fn() -> Result<bool, Error>,
}
pub(super) fn execute(
    payload: &[u8],
    options: Options,
    cancelled: &dyn Fn() -> bool,
    receiver: impl FnOnce(&CapturedRecord, &[u8]) -> Action,
    identity: Option<Identity<'_>>,
) -> Result<BridgeReport, Error> {
    validate_snapshot(payload)?;
    let started = Instant::now();
    let deadline = started
        .checked_add(options.duration)
        .ok_or_else(|| invalid("duration overflow"))?;
    let mut report = BridgeReport::default();
    if cancelled() {
        report.outcome = BridgeOutcome::Aborted;
        return Ok(report);
    }
    if options.duration > Duration::from_secs(2) {
        return Err(invalid("duration exceeds this bridge profile"));
    }
    if !options.receiver {
        return Ok(report);
    }
    if Instant::now() >= deadline {
        report.budget_expired = true;
        return Ok(report);
    }
    if let Some(i) = &identity {
        if !(i.admit)()? {
            report.budget_expired = true;
            return Ok(report);
        }
    }
    let plan = config()?;
    let session = if let Some(i) = &identity {
        i.session
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
            | (u128::from(std::process::id()) << 96)
    };
    let revision = identity.as_ref().map_or(REVISION, |i| i.revision);
    let budget = QueryBudget::new(&plan);
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(invalid(
            "run snapshot checks on a general worker outside Tokio",
        ));
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let routes = routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        session,
        revision,
        &[ChannelConfig::new(7, 99, 1)],
        &routes,
        tracking(),
    )?;
    let operation = owner.scope(|mut scope| -> Result<(), Error> {
        let completion = scope
            .completion()
            .ok_or_else(|| invalid("missing completion"))?;
        let (mut producer, consumer) = scope.next().ok_or_else(|| invalid("missing producer"))?;
        let mut router = Router::new(&plan, vec![consumer], routes)?;
        router.scope(|mut dispatcher, mut outputs| -> Result<(), Error> {
            rt.block_on(async {
                let limits = RemoteLimits {
                    connections: 1,
                    subscriptions: 1,
                    commands: 1,
                    message_bytes: 8192,
                    record_bytes: MAX_SNAPSHOT_BYTES,
                    subscription_records: 1,
                    subscription_bytes: MAX_SNAPSHOT_BYTES,
                    max_age: Duration::from_secs(2),
                    request_timeout: Duration::from_secs(2),
                };
                let (hub, inbox) = RemoteHub::new(&budget, limits)?;
                let server = RemoteServer::bind(
                    ServerConfig {
                        enabled: true,
                        ..Default::default()
                    },
                    hub.clone(),
                )
                .await?;
                let address = server.local_addr()?;
                let (stop, stopped) = tokio::sync::oneshot::channel();
                let server = tokio::spawn(server.serve(async {
                    let _ = stopped.await;
                }));
                // Always close the hub/server and wait for actual IO below, even
                // when connect, capture, callback or transport reports failure.
                let operation = async {
                    let cancel = Cancellation::new();
                    let mut client = RemoteClient::connect(
                        {
                            let mut config = RemoteConfig::new(format!("http://{address}"));
                            config.timeout = deadline
                                .saturating_duration_since(Instant::now())
                                .max(Duration::from_nanos(1));
                            config
                        },
                        &cancel,
                    )
                    .await?;
                    let mut stream = client
                        .subscribe(
                            &budget,
                            SubscriptionFilter {
                                channel: Some(7),
                                minimum_level: Level::Info,
                                originals: true,
                                overflow: OverflowPolicy::Disconnect,
                            },
                            ReceiverConfig {
                                records: 1,
                                record_bytes: MAX_SNAPSHOT_BYTES,
                                ttl: Duration::from_secs(2),
                            },
                            &cancel,
                        )
                        .await?;
                    if cancelled() {
                        report.outcome = BridgeOutcome::Aborted;
                        return Ok::<(), Error>(());
                    }
                    if Instant::now() >= deadline {
                        report.budget_expired = true;
                        return Ok(());
                    }
                    if let Some(i) = &identity {
                        if !(i.admit)()? {
                            report.budget_expired = true;
                            return Ok(());
                        }
                    }
                    let mut ids = [0; 1];
                    if hub.subscriptions(&mut ids) != 1 {
                        return Err(invalid("receiver missing after handshake"));
                    }
                    let mut loan = producer
                        .try_acquire(payload.len(), Importance::Normal)
                        .map_err(|_| invalid("raw loan rejected"))?;
                    loan.payload_mut().copy_from_slice(payload);
                    if cancelled() {
                        report.outcome = BridgeOutcome::Aborted;
                        return Ok(());
                    }
                    if Instant::now() >= deadline {
                        report.budget_expired = true;
                        return Ok(());
                    }
                    // Capture time belongs to this dedicated logger session, not
                    // the app's accumulated tick counters or wall-clock time.
                    let metadata = RecordMetadata {
                        channel_id: 7,
                        schema_id: 99,
                        schema_version: 1,
                        mono_ns: match &identity {
                            Some(i) => (i.observe)()?,
                            None => u64::try_from(started.elapsed().as_nanos())?,
                        },
                        real_ns: 0,
                        source_time: None,
                        level: Level::Info,
                        importance: Some(Importance::Normal),
                    };
                    if let Some(i) = &identity {
                        if !(i.admit)()? {
                            report.budget_expired = true;
                            return Ok(());
                        }
                    }
                    let receipt = match producer.try_record_raw(metadata, loan).map_err(|e| {
                        invalid(match e.reason {
                            CaptureError::Capacity => "capture capacity",
                            _ => "capture rejected",
                        })
                    })? {
                        RawRecordOutcome::Accepted(r) => r,
                        RawRecordOutcome::Skipped(_) => return Err(invalid("snapshot skipped")),
                    };
                    report.receipt = Some(receipt);
                    report.accepted = 1;
                    let mut events = [None; 4];
                    dispatcher.turn(Instant::now(), &mut events)?;
                    let delivery = outputs[0]
                        .try_next(Instant::now())
                        .ok_or_else(|| invalid("routing produced no delivery"))?
                        .map_err(|_| invalid("routing dropped the snapshot"))?;
                    if delivery.record().receipt != receipt || delivery.payload() != payload {
                        return Err(invalid("routed identity mismatch"));
                    }
                    let mut delivery = hub
                        .offer_delivery(ids[0], delivery, true, 1)
                        .map_err(|(e, _)| e)?;
                    let received = tokio::time::timeout_at(deadline.into(), stream.next(&cancel))
                        .await
                        .map_err(|_| invalid("receive deadline"))??
                        .ok_or_else(|| invalid("stream ended"))?;
                    let RemoteEvent::Record(record) = received else {
                        return Err(invalid("unexpected gap"));
                    };
                    // Assembly/read alone must not complete ingress or release
                    // the original retained by the routing delivery.
                    if delivery.poll()? != AckState::Pending {
                        return Err(invalid("ACK before handoff"));
                    }
                    report.before_handoff = Some(
                        completion
                            .poll(receipt, TARGET, Boundary::ReceiverAcknowledged)?
                            .outcome,
                    );
                    report.raw_held_before_ack = matches!(
                        producer.try_acquire(payload.len(), Importance::Normal),
                        Err(CaptureError::Capacity)
                    );
                    let result = stream
                        .handoff(
                            record,
                            |record, bytes| {
                                if cancelled() {
                                    return false;
                                }
                                if Instant::now() >= deadline {
                                    report.budget_expired = true;
                                    return false;
                                }
                                if let Some(i) = &identity {
                                    match (i.admit)() {
                                        Ok(true) => {}
                                        Ok(false) => {
                                            report.budget_expired = true;
                                            return false;
                                        }
                                        Err(error) => {
                                            report.issue = Some(error);
                                            return false;
                                        }
                                    }
                                }
                                let action = receiver(record, bytes);
                                #[cfg(test)]
                                if matches!(
                                    action,
                                    Action::Disconnect | Action::AcceptThenDisconnect
                                ) {
                                    client.disconnect();
                                }
                                match action {
                                    Action::Accept => true,
                                    #[cfg(test)]
                                    Action::AcceptThenDisconnect => true,
                                    _ => false,
                                }
                            },
                            &cancel,
                        )
                        .await;
                    match result {
                        Ok(()) => {}
                        Err(
                            e @ (ClientError::ConsumerRejected
                            | ClientError::AcknowledgementUnknown),
                        ) => {
                            report.handoff_error = Some(e);
                        }
                        Err(e) => return Err(e.into()),
                    }
                    // Rejected records never ACK. Closing this exact connection
                    // makes the pending delivery Unknown; no synthetic success.
                    drop(stream);
                    client.disconnect();
                    let settle = Instant::now() + Duration::from_secs(3);
                    loop {
                        match delivery.poll()? {
                            AckState::Pending if Instant::now() < settle => {
                                tokio::time::sleep(Duration::from_millis(1)).await
                            }
                            AckState::Pending => return Err(invalid("ACK settlement deadline")),
                            _ => break,
                        }
                    }
                    report.raw_reclaimed += dispatcher.turn(Instant::now(), &mut events)?.reclaimed;
                    report.completion = Some(
                        completion
                            .poll(receipt, TARGET, Boundary::ReceiverAcknowledged)?
                            .outcome,
                    );
                    report.acknowledged =
                        u64::from(report.completion == Some(QueryOutcome::Complete));
                    report.raw_reclaimed += dispatcher.turn(Instant::now(), &mut events)?.reclaimed;
                    report.outcome = if report.issue.is_some() {
                        BridgeOutcome::Fail
                    } else if cancelled() {
                        BridgeOutcome::Aborted
                    } else if report.acknowledged == 1 {
                        BridgeOutcome::Pass
                    } else {
                        BridgeOutcome::Inconclusive
                    };
                    Ok(())
                }
                .await;
                hub.close();
                let _ = stop.send(());
                let closed = server.await?;
                drop((hub, inbox));
                operation?;
                closed?;
                Ok(())
            })
        })
    });
    if let Err(error) = operation {
        report.issue = Some(error);
        report.outcome = BridgeOutcome::Fail;
    }
    report.external_bytes_after_close = budget.used_bytes();
    Ok(report)
}
