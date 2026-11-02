//! Fixed real-socket scenario; every exit closes and joins the owned server.
use super::*;
use dot_logger::{adapters::remote::*, capture::*, config::*, platform::*, query::QueryBudget};
use dot_logger_client::{ReceiverConfig, RemoteClient, RemoteConfig};
use std::time::Duration;

/// Finite fault scenarios compiled by both the example and integration tests.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Local wait, server lease end, explicit reconnect, overflow and cancellation.
    Lifecycle,
    /// Consumer history full: no ACK and a retained terminal marker.
    HistoryFull,
    /// Parent connection closes with an unacknowledged record.
    Disconnect,
}

fn budget() -> Result<QueryBudget, Error> {
    let mib = 1024 * 1024;
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 8 * mib,
            input_bytes: mib,
            raw_bytes: 0,
            output_bytes: mib,
            external_bytes: 4 * mib,
            control_bytes: mib,
            worker_bytes: mib,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 2,
                slot_bytes: 64,
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
    .validate(&PlatformCapabilities::default())?;
    Ok(QueryBudget::new(&plan))
}

/// Run only the fixed lifecycle demonstration with at most two connections.
pub fn run(cancel: &Cancellation) -> Result<TailReport, Error> {
    run_case(Mode::Lifecycle, cancel)
}
/// General-path finite harness; no global runtime, environment, files or DB.
/// Admission/polling is bounded by 10s, server cleanup observation by another 2s.
/// Scheduling/OS IO may exceed these waits; no hard real-time deadline is claimed.
pub fn run_case(mode: Mode, cancel: &Cancellation) -> Result<TailReport, Error> {
    if cancel.is_cancelled() || tokio::runtime::Handle::try_current().is_ok() {
        return Err(invalid("cancelled or nested tail runtime"));
    }
    let budget = budget()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        let (hub, inbox) = RemoteHub::new(
            &budget,
            RemoteLimits {
                connections: 2,
                subscriptions: 2,
                commands: 1,
                message_bytes: 4096,
                record_bytes: 64,
                subscription_records: 1,
                subscription_bytes: 64,
                max_age: Duration::from_secs(2),
                request_timeout: Duration::from_secs(2),
            },
        )?;
        let server = RemoteServer::bind(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            hub.clone(),
        )
        .await?;
        let endpoint = format!("http://{}", server.local_addr()?);
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let mut task = tokio::spawn(server.serve(async {
            let _ = stopped.await;
        }));
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            scenario(&budget, &hub, &endpoint, mode, cancel),
        )
        .await;
        hub.close();
        let _ = stop.send(());
        let closed = match tokio::time::timeout(Duration::from_secs(2), &mut task).await {
            Ok(joined) => joined
                .map_err(|_| invalid("tail server task failed"))?
                .map_err(Error::from),
            Err(_) => {
                task.abort();
                let _ = task.await;
                Err(invalid("tail server close deadline"))
            }
        };
        drop((inbox, hub));
        closed?;
        result.map_err(|_| invalid("tail scenario deadline"))?
    });
    // Runtime owns HTTP/2 tasks: release it before claiming reservation cleanup.
    drop(runtime);
    let mut report = result?;
    report.external_after_close = budget.used_bytes();
    if report.external_after_close != 0 {
        return Err(invalid("tail reservation still owned"));
    }
    Ok(report)
}

async fn connect(endpoint: &str, cancel: &Cancellation) -> Result<RemoteClient, Error> {
    let mut config = RemoteConfig::new(endpoint);
    config.timeout = Duration::from_millis(250);
    config.message_bytes = 4096;
    Ok(RemoteClient::connect(config, cancel).await?)
}
async fn subscribe(
    client: &mut RemoteClient,
    budget: &QueryBudget,
    cancel: &Cancellation,
) -> Result<RemoteSubscription, Error> {
    Ok(client
        .subscribe(
            budget,
            SubscriptionFilter {
                channel: Some(7),
                minimum_level: Level::Info,
                originals: false,
                overflow: OverflowPolicy::Gap,
            },
            ReceiverConfig {
                records: 1,
                record_bytes: 64,
                ttl: Duration::from_millis(1500),
            },
            cancel,
        )
        .await?)
}
fn subscription(hub: &RemoteHub) -> Result<u64, Error> {
    let mut ids = [0; 2];
    if hub.subscriptions(&mut ids) != 1 {
        return Err(invalid("expected one active subscription"));
    }
    Ok(ids[0])
}
fn offer(hub: &RemoteHub, id: u64, sequence: u64, byte: u8) -> Result<PendingAck, Error> {
    // Synthetic source metadata; no capture sequence or missing count is inferred
    // from subscription transfer positions. S2 verifies production capture wiring.
    Ok(hub.offer(
        id,
        CapturedRecord {
            receipt: CaptureReceipt {
                session_id: 42,
                producer_id: 0,
                accepted_seq: sequence,
            },
            metadata: RecordMetadata {
                channel_id: 7,
                schema_id: 99,
                schema_version: 1,
                mono_ns: sequence,
                real_ns: sequence as i64,
                source_time: None,
                level: Level::Info,
                importance: Some(Importance::Normal),
            },
            revision: 1,
            payload_len: 1,
        },
        &[byte],
        false,
        1,
    )?)
}
fn expect_step(actual: Step, expected: Step) -> Result<(), Error> {
    if actual != expected {
        return Err(invalid("unexpected tail screen outcome"));
    }
    Ok(())
}
async fn scenario(
    budget: &QueryBudget,
    hub: &RemoteHub,
    endpoint: &str,
    mode: Mode,
    cancel: &Cancellation,
) -> Result<TailReport, Error> {
    let mut client = connect(endpoint, cancel).await?;
    let mut stream = subscribe(&mut client, budget, cancel).await?;
    let mut screen = Screen::new(if mode == Mode::HistoryFull { 3 } else { 32 })?;
    let first = screen.begin()?;
    let id = subscription(hub)?;
    let mut acknowledged = 0;
    expect_step(
        receive(&mut screen, first, &mut stream, cancel).await?,
        Step::Waiting,
    )?;
    // A local wait must not resubscribe, reset transfer positions or fabricate Gap.
    if subscription(hub)? != id {
        return Err(invalid("local wait replaced subscription"));
    }
    let ack = offer(hub, id, 1, 1)?;
    if mode != Mode::Lifecycle {
        if mode == Mode::Disconnect {
            client.disconnect();
        }
        expect_step(
            receive(&mut screen, first, &mut stream, cancel).await?,
            Step::Ended,
        )?;
        if ack.state() == AckState::Acknowledged {
            return Err(invalid("unexpected ACK on rejected screen record"));
        }
        drop(stream);
        client.disconnect();
        drop(client);
        // Keep the real unconfirmed delivery alive until the server observes close.
        tokio::time::timeout(Duration::from_secs(2), async {
            while ack.state() == AckState::Pending {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .map_err(|_| invalid("unknown settlement deadline"))?;
        if ack.state() != AckState::Unknown {
            return Err(invalid("unacknowledged close was not Unknown"));
        }
        return Ok(TailReport {
            screen,
            acknowledged,
            subscriptions: 1,
            external_after_close: usize::MAX,
        });
    }
    expect_step(
        receive(&mut screen, first, &mut stream, cancel).await?,
        Step::Acknowledged,
    )?;
    if ack.state() != AckState::Acknowledged {
        return Err(invalid("first ACK unconfirmed"));
    }
    acknowledged += 1;
    let ack = offer(hub, id, 2, 2)?;
    expect_step(
        receive(&mut screen, first, &mut stream, cancel).await?,
        Step::Acknowledged,
    )?;
    if ack.state() != AckState::Acknowledged {
        return Err(invalid("second ACK unconfirmed"));
    }
    acknowledged += 1;
    let mut ended = false;
    for _ in 0..12 {
        match receive(&mut screen, first, &mut stream, cancel).await? {
            Step::Waiting => {}
            Step::Ended => {
                ended = true;
                break;
            }
            _ => return Err(invalid("unexpected data during lease wait")),
        }
    }
    if !ended {
        return Err(invalid("lease did not end within finite polls"));
    }
    drop(stream);
    client.disconnect();
    drop(client);
    // Explicit second connection/subscription. No automatic reconnect loop.
    let mut client = connect(endpoint, cancel).await?;
    let mut stream = subscribe(&mut client, budget, cancel).await?;
    let second = screen.begin()?;
    let id = subscription(hub)?;
    let ack = offer(hub, id, 99, 9)?;
    // One slot is occupied until ACK: transfer2 is dropped with an explicit Gap.
    let dropped = offer(hub, id, 100, 10)?;
    if dropped.state() != AckState::Dropped {
        return Err(invalid("overflow was not Dropped"));
    }
    expect_step(
        receive(&mut screen, second, &mut stream, cancel).await?,
        Step::Acknowledged,
    )?;
    if ack.state() != AckState::Acknowledged {
        return Err(invalid("fresh subscription ACK unconfirmed"));
    }
    acknowledged += 1;
    expect_step(
        receive(&mut screen, second, &mut stream, cancel).await?,
        Step::Gap,
    )?;
    let stopped = Cancellation::new();
    stopped.cancel();
    expect_step(
        receive(&mut screen, second, &mut stream, &stopped).await?,
        Step::Ended,
    )?;
    drop(stream);
    client.disconnect();
    Ok(TailReport {
        screen,
        acknowledged,
        subscriptions: 2,
        external_after_close: usize::MAX,
    })
}
