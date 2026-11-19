//! Compare validated TCP and dot-logger receiver ACKs on two LAN hosts.
use dot_logger::{
    adapters::remote::{
        AccessToken, AckState, OverflowPolicy, PendingAck, Permission, RemoteHub, RemoteLimits,
        RemoteServer, ServerConfig, SubscriptionFilter, TlsIdentity,
    },
    capture::{CaptureReceipt, CapturedRecord, Importance, Level, RecordMetadata},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    query::QueryBudget,
};
use dot_logger_client::{Cancellation, ReceiverConfig, RemoteClient, RemoteConfig, RemoteEvent};
use serde_json::json;
use std::{
    collections::VecDeque,
    error::Error,
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    time::{Duration, Instant},
};
use tokio::{sync::oneshot, time::timeout};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const SMALL: usize = 256;
const BULK: usize = 8 << 20;
const WINDOW: usize = 4;
const SMALL_WINDOW: usize = 1024;
const START_WAIT: Duration = Duration::from_secs(180);
const IO_WAIT: Duration = Duration::from_secs(15);
const CHANNEL: u32 = 7;
const RECEIVER_MARKER: &[u8; 8] = b"DOTLAN01";

fn budget() -> Result<QueryBudget> {
    let mib = 1 << 20;
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 132 * mib,
            input_bytes: mib,
            raw_bytes: 0,
            output_bytes: mib,
            external_bytes: 128 * mib,
            control_bytes: mib,
            worker_bytes: mib,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: SMALL,
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
    .map_err(|error| format!("probe budget: {error:?}"))?;
    Ok(QueryBudget::new(&plan))
}

fn pattern(bytes: usize) -> Vec<u8> {
    (0..bytes).map(|index| (index % 251) as u8).collect()
}

fn verify(payload: &[u8], expected: &[u8], sequence: u64) -> bool {
    payload.get(..8) == Some(sequence.to_le_bytes().as_slice())
        && payload.get(8..) == expected.get(8..)
}

fn stamp(payload: &mut [u8], sequence: u64) {
    payload[..8].copy_from_slice(&sequence.to_le_bytes());
}

fn record(sequence: u64, bytes: usize) -> CapturedRecord {
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: sequence,
        },
        metadata: RecordMetadata {
            channel_id: CHANNEL,
            schema_id: 99,
            schema_version: 1,
            mono_ns: sequence,
            real_ns: sequence as i64,
            source_time: None,
            level: Level::Info,
            importance: Some(Importance::Normal),
        },
        revision: 1,
        payload_len: bytes,
    }
}

fn quantile(sorted: &[u64], percentile: usize) -> Option<u64> {
    let rank = (sorted.len() * percentile).div_ceil(100);
    rank.checked_sub(1)
        .and_then(|index| sorted.get(index).copied())
}

#[derive(Default)]
struct OfferStats {
    dropped: u64,
    unknown: u64,
    max_lag_ns: u64,
}

fn report(
    mode: &str,
    bytes: usize,
    count: u64,
    start: Instant,
    latencies: &mut [u64],
    offers: OfferStats,
) {
    let elapsed = start.elapsed().as_secs_f64();
    latencies.sort_unstable();
    println!(
        "{}",
        json!({
            "mode": mode, "payload_bytes": bytes, "offered": count,
            "acknowledged": latencies.len(), "dropped": offers.dropped, "unknown": offers.unknown,
            "elapsed_s": elapsed,
            "verified_MBps": latencies.len() as f64 * bytes as f64 / elapsed / 1e6,
            "p50_ack_ns": quantile(latencies, 50), "p95_ack_ns": quantile(latencies, 95),
            "p99_ack_ns": quantile(latencies, 99), "max_ack_ns": latencies.last(),
            "max_offer_lag_ns": offers.max_lag_ns
        })
    );
}

async fn grpc_server(address: SocketAddr, bytes: usize, count: u64, identity: &Path) -> Result<()> {
    let budget = budget()?;
    let records = if bytes == BULK { WINDOW } else { SMALL_WINDOW };
    let limits = RemoteLimits {
        connections: 1,
        subscriptions: 1,
        commands: 1,
        message_bytes: (256 << 10) + 1024,
        record_bytes: bytes,
        subscription_records: records,
        subscription_bytes: bytes * records,
        max_age: IO_WAIT,
        request_timeout: IO_WAIT,
    };
    let (hub, inbox) = RemoteHub::new(&budget, limits)?;
    let server = RemoteServer::bind(
        ServerConfig {
            enabled: true,
            address,
            tokens: vec![AccessToken::new(
                fs::read_to_string(identity.join("token.txt"))?
                    .trim()
                    .to_owned(),
                Permission::Read,
            )?],
            tls: Some(TlsIdentity::new(
                fs::read(identity.join("server.pem"))?,
                fs::read(identity.join("server.key"))?,
            )?),
            ..ServerConfig::default()
        },
        hub.clone(),
    )
    .await?;
    println!("ready grpc {address}");
    let (stop, stopped) = oneshot::channel();
    let serving = tokio::spawn(server.serve(async {
        let _ = stopped.await;
    }));
    let result = grpc_offers(&hub, bytes, count, records).await;
    if result.is_ok() {
        let _ = timeout(IO_WAIT, async {
            while hub.subscriptions(&mut [0]) != 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await;
    }
    let _ = stop.send(());
    timeout(IO_WAIT, serving).await???;
    drop((hub, inbox));
    if budget.used_bytes() != 0 {
        return Err("server reservation remained after shutdown".into());
    }
    result
}

async fn grpc_offers(hub: &RemoteHub, bytes: usize, count: u64, window: usize) -> Result<()> {
    let subscription = timeout(START_WAIT, async {
        let mut ids = [0];
        while hub.subscriptions(&mut ids) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        ids[0]
    })
    .await?;

    let mut payload = pattern(bytes);
    let mut pending: VecDeque<(Instant, PendingAck)> = VecDeque::with_capacity(window);
    let mut latencies = Vec::with_capacity(count as usize);
    let mut sequence = 1;
    let mut offers = OfferStats::default();
    let start = Instant::now();
    let deadline = start
        + IO_WAIT
        + Duration::from_secs(count.div_ceil(if bytes == SMALL { 10_000 } else { 4 }));

    while sequence <= count || !pending.is_empty() {
        while let Some((sent, ack)) = pending.front() {
            match ack.state() {
                AckState::Pending => break,
                AckState::Acknowledged => latencies.push(sent.elapsed().as_nanos() as u64),
                AckState::Dropped => offers.dropped += 1,
                AckState::Unknown => offers.unknown += 1,
            }
            pending.pop_front();
        }
        if Instant::now() >= deadline {
            offers.unknown += pending.len() as u64;
            break;
        }
        let scheduled = Duration::from_micros((sequence - 1) * 100);
        if sequence <= count
            && (bytes == SMALL || pending.len() < window)
            && (bytes == BULK || start.elapsed() >= scheduled)
        {
            stamp(&mut payload, sequence);
            let sent = Instant::now();
            if bytes == SMALL {
                let lag = sent
                    .duration_since(start)
                    .saturating_sub(scheduled)
                    .as_nanos() as u64;
                offers.max_lag_ns = offers.max_lag_ns.max(lag);
            }
            let ack = hub.offer(
                subscription,
                record(sequence, bytes),
                &payload,
                bytes == BULK,
                0,
            )?;
            if ack.state() == AckState::Dropped {
                offers.dropped += 1;
            } else {
                pending.push_back((sent, ack));
            }
            sequence += 1;
            continue;
        }
        if sequence <= count || !pending.is_empty() {
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
    }
    report("grpc", bytes, sequence - 1, start, &mut latencies, offers);
    if latencies.len() as u64 != count {
        return Err("not all offered records were acknowledged".into());
    }
    Ok(())
}

async fn grpc_receiver(
    address: SocketAddr,
    bytes: usize,
    count: u64,
    identity: &Path,
) -> Result<()> {
    let cancellation = Cancellation::new();
    let mut config = RemoteConfig::new(format!("https://{address}"));
    config.token = Some(
        fs::read_to_string(identity.join("token.txt"))?
            .trim()
            .into(),
    );
    config.ca_certificate = Some(fs::read(identity.join("ca.pem"))?);
    config.timeout = IO_WAIT;
    let mut client = RemoteClient::connect(config, &cancellation).await?;
    let mut subscription = client
        .subscribe(
            &budget()?,
            SubscriptionFilter {
                channel: Some(CHANNEL),
                minimum_level: Level::Trace,
                originals: bytes == BULK,
                overflow: OverflowPolicy::Gap,
            },
            ReceiverConfig {
                records: if bytes == BULK { WINDOW } else { SMALL_WINDOW },
                record_bytes: bytes,
                ttl: IO_WAIT,
            },
            &cancellation,
        )
        .await?;
    let expected = pattern(bytes);
    let mut next = 1;
    let mut verified = 0;
    let mut gaps = 0;
    let start = Instant::now();
    while next <= count {
        match subscription.next(&cancellation).await? {
            Some(RemoteEvent::Record(pending)) => {
                subscription
                    .handoff(
                        pending,
                        |record, payload| {
                            record.receipt.accepted_seq == next && verify(payload, &expected, next)
                        },
                        &cancellation,
                    )
                    .await?;
                verified += 1;
                next += 1;
            }
            Some(RemoteEvent::Gap { first, last }) if first == next && last <= count => {
                gaps += last - first + 1;
                next = last + 1;
            }
            _ => return Err(format!("stream ended or reordered at sequence {next}").into()),
        }
    }
    println!(
        "{}",
        json!({
            "mode": "grpc-receiver", "verified": verified, "gaps": gaps,
            "elapsed_s": start.elapsed().as_secs_f64(), "payload_bytes": bytes
        })
    );
    Ok(())
}

fn tcp_io(stream: &TcpStream) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(IO_WAIT))?;
    stream.set_write_timeout(Some(IO_WAIT))
}

fn tcp_server(address: SocketAddr, count: u64, peer: std::net::IpAddr) -> Result<()> {
    let listener = TcpListener::bind(address)?;
    listener.set_nonblocking(true)?;
    println!("ready tcp {address}");
    let deadline = Instant::now() + START_WAIT;
    let mut stream = loop {
        match listener.accept() {
            Ok((mut stream, source)) if source.ip() == peer => {
                stream.set_nonblocking(false)?;
                tcp_io(&stream)?;
                let mut marker = [0; 8];
                stream.read_exact(&mut marker)?;
                if &marker == RECEIVER_MARKER {
                    break stream;
                }
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err("TCP receiver did not connect".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    };
    let mut payload = pattern(BULK);
    let mut latencies = Vec::with_capacity(count as usize);
    let mut pending = Vec::with_capacity(WINDOW);
    let start = Instant::now();
    for first in (1..=count).step_by(WINDOW) {
        for sequence in first..=(first + WINDOW as u64 - 1).min(count) {
            stamp(&mut payload, sequence);
            let at = Instant::now();
            stream.write_all(&payload)?;
            pending.push((sequence, at));
        }
        for (sequence, at) in pending.drain(..) {
            let mut ack = [0; 8];
            stream.read_exact(&mut ack)?;
            if sequence != u64::from_le_bytes(ack) {
                return Err("invalid TCP receiver acknowledgement".into());
            }
            latencies.push(at.elapsed().as_nanos() as u64);
        }
    }
    report(
        "tcp",
        BULK,
        count,
        start,
        &mut latencies,
        OfferStats::default(),
    );
    Ok(())
}

fn tcp_receiver(address: SocketAddr, count: u64) -> Result<()> {
    let mut stream = TcpStream::connect_timeout(&address, IO_WAIT)?;
    tcp_io(&stream)?;
    let expected = pattern(BULK);
    let mut payload = vec![0; BULK];
    stream.write_all(RECEIVER_MARKER)?;
    let start = Instant::now();
    for sequence in 1..=count {
        stream.read_exact(&mut payload)?;
        if !verify(&payload, &expected, sequence) {
            return Err(format!("invalid TCP payload at sequence {sequence}").into());
        }
        stream.write_all(&sequence.to_le_bytes())?;
    }
    println!(
        "{}",
        json!({
            "mode": "tcp-receiver", "verified": count, "gaps": 0,
            "elapsed_s": start.elapsed().as_secs_f64(), "payload_bytes": BULK
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [role, mode, address, count, identity] if mode == "small" || mode == "bulk" => {
            let count = count.parse::<u64>()?;
            if !(1..=120_000).contains(&count) || mode == "bulk" && count > 1024 {
                return Err("record count outside probe bounds".into());
            }
            let bytes = if mode == "small" { SMALL } else { BULK };
            match role.as_str() {
                "send" => grpc_server(address.parse()?, bytes, count, Path::new(identity)).await,
                "recv" => grpc_receiver(address.parse()?, bytes, count, Path::new(identity)).await,
                _ => Err("role must be send or recv".into()),
            }
        }
        [role, mode, address, count, peer] if role == "send" && mode == "tcp" => {
            tcp_server(address.parse()?, tcp_count(count)?, peer.parse()?)
        }
        [role, mode, address, count] if role == "recv" && mode == "tcp" => {
            tcp_receiver(address.parse()?, tcp_count(count)?)
        }
        _ => Err("usage: lan_probe send|recv small|bulk ADDRESS COUNT TLS_DIR | lan_probe send tcp ADDRESS COUNT PEER_IP | lan_probe recv tcp ADDRESS COUNT".into()),
    }
}

fn tcp_count(count: &str) -> Result<u64> {
    let count = count.parse()?;
    if !(1..=1024).contains(&count) {
        return Err("TCP count outside probe bounds".into());
    }
    Ok(count)
}
