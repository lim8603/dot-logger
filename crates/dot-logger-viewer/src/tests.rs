#![allow(clippy::unwrap_used)]

use crate::{Options, bridge::Bridge, catalog::Catalog, output::Outbox, serve};
use dot_logger::{
    adapters::remote::{AckState, RemoteHub, RemoteLimits, RemoteServer, ServerConfig},
    capture::{CaptureReceipt, CapturedRecord, Importance, Level, RecordMetadata},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    query::QueryBudget,
};
use dot_logger_client::RemoteConfig;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::{sleep, timeout},
};
use tokio_tungstenite::{
    client_async,
    tungstenite::{Message, client::IntoClientRequest},
};

const WAIT: Duration = Duration::from_secs(5);
const CATALOG: &str = r#"{"version":1,"channels":[{"id":17,"channel_id":7,
    "schema_id":"00000000000000000000000000000063","schema_version":1,"topic":"robot/pose",
    "encoding":"json","schema_name":"Pose","schema":"{\"type\":\"object\"}"},
    {"id":18,"channel_id":8,"schema_id":"00000000000000000000000000000064",
    "schema_version":1,"topic":"robot/health","encoding":"json","schema_name":"Health",
    "schema":"{\"type\":\"object\"}"}]}"#;

type ClientSocket = tokio_tungstenite::WebSocketStream<TcpStream>;

fn budget() -> QueryBudget {
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 132 * 1024 * 1024,
            input_bytes: 1024 * 1024,
            raw_bytes: 0,
            output_bytes: 1024 * 1024,
            external_bytes: 128 * 1024 * 1024,
            control_bytes: 1024 * 1024,
            worker_bytes: 1024 * 1024,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 256,
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
    .unwrap();
    QueryBudget::new(&plan)
}

async fn host(budget: &QueryBudget, address: SocketAddr) -> (RemoteServer, RemoteHub) {
    let (hub, _inbox) = RemoteHub::new(
        budget,
        RemoteLimits {
            connections: 4,
            subscriptions: 4,
            commands: 2,
            message_bytes: 32 * 1024,
            record_bytes: 64 * 1024,
            subscription_records: 1,
            subscription_bytes: 64 * 1024,
            max_age: Duration::from_secs(10),
            request_timeout: Duration::from_secs(2),
        },
    )
    .unwrap();
    let server = RemoteServer::bind(
        ServerConfig {
            enabled: true,
            address,
            ..ServerConfig::default()
        },
        hub.clone(),
    )
    .await
    .unwrap();
    (server, hub)
}

async fn subscriptions(hub: &RemoteHub, count: usize) -> [u64; 4] {
    timeout(WAIT, async {
        loop {
            let mut subscriptions = [0; 4];
            if hub.subscriptions(&mut subscriptions) == count {
                return subscriptions;
            }
            sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap()
}

fn record(sequence: u64, length: usize) -> CapturedRecord {
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
            real_ns: 1_000_000_000 + sequence as i64,
            source_time: None,
            level: Level::Info,
            importance: Some(Importance::Normal),
        },
        revision: 1,
        payload_len: length,
    }
}

async fn message(socket: &mut ClientSocket, matches: impl Fn(&Message) -> bool) -> Message {
    timeout(WAIT, async {
        loop {
            let message = socket.next().await.unwrap().unwrap();
            if matches(&message) {
                return message;
            }
        }
    })
    .await
    .unwrap()
}

fn status(message: &Message, text: &str) -> bool {
    message.to_text().ok().is_some_and(|content| {
        let event: Value = serde_json::from_str(content).unwrap();
        event["op"] == "status"
            && event["message"]
                .as_str()
                .is_some_and(|value| value.contains(text))
    })
}

async fn socket(
    address: SocketAddr,
    protocol: &str,
    origin: Option<&str>,
) -> tokio_tungstenite::tungstenite::Result<ClientSocket> {
    let mut request = format!("ws://{address}").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("sec-websocket-protocol", protocol.parse().unwrap());
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("origin", origin.parse().unwrap());
    }
    let (socket, accepted) =
        client_async(request, TcpStream::connect(address).await.unwrap()).await?;
    assert_eq!(
        accepted.headers()["sec-websocket-protocol"],
        "foxglove.websocket.v1"
    );
    Ok(socket)
}

async fn wait_for_ack(accepted: &dot_logger::adapters::remote::PendingAck) {
    timeout(WAIT, async {
        while accepted.state() != AckState::Acknowledged {
            sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
}

fn subscribe() -> Message {
    Message::text(r#"{"op":"subscribe","subscriptions":[{"id":91,"channelId":17}]}"#)
}

#[tokio::test]
async fn live_bridge_reports_gap_recovers_from_host_exit_and_releases_its_subscription() {
    let budget = budget();
    let (server, hub) = host(&budget, "127.0.0.1:0".parse().unwrap()).await;
    let upstream = server.local_addr().unwrap();
    let (host_stop, host_done) = oneshot::channel();
    let host_task = tokio::spawn(server.serve(async {
        let _ = host_done.await;
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (viewer_stop, viewer_done) = oneshot::channel();
    let options = Options {
        receiver_records: 1,
        record_bytes: 64 * 1024,
        ..Options::default()
    };
    let viewer = tokio::spawn(serve(
        listener,
        RemoteConfig::new(format!("http://{upstream}")),
        Catalog::parse(CATALOG.as_bytes()).unwrap(),
        options,
        async {
            let _ = viewer_done.await;
        },
    ));
    for (protocol, origin, expected) in [
        ("foxglove.sdk.v1", None, 400),
        (
            "foxglove.websocket.v1",
            Some("https://untrusted.example"),
            403,
        ),
    ] {
        assert!(matches!(
            self::socket(address, protocol, origin).await,
            Err(tokio_tungstenite::tungstenite::Error::Http(response))
                if response.status() == expected
        ));
    }
    let origin = format!("http://{address}");
    let mut socket = self::socket(
        address,
        "foxglove.sdk.v1, foxglove.websocket.v1",
        Some(&origin),
    )
    .await
    .unwrap();
    let greeting = message(&mut socket, |message| message.is_text()).await;
    assert_eq!(
        serde_json::from_str::<Value>(greeting.to_text().unwrap()).unwrap()["op"],
        "serverInfo"
    );
    let advertisement = message(&mut socket, |message| message.is_text()).await;
    let advertisement: Value = serde_json::from_str(advertisement.to_text().unwrap()).unwrap();
    assert_eq!(advertisement["channels"][0]["id"], 17);
    assert_eq!(advertisement["channels"][0]["topic"], "robot/pose");
    subscriptions(&hub, 0).await;

    socket.send(subscribe()).await.unwrap();
    let id = subscriptions(&hub, 1).await[0];
    let payload = br#"{"x":42}"#;
    // Same-thread production fills the host queue before its stream can drain.
    let accepted = hub
        .offer(id, record(1, payload.len()), payload, false, 0)
        .unwrap();
    let dropped = hub
        .offer(id, record(2, payload.len()), payload, false, 0)
        .unwrap();
    assert_eq!(dropped.state(), AckState::Dropped);
    let data = message(&mut socket, Message::is_binary).await.into_data();
    assert_eq!(data[0], 1);
    assert_eq!(&data[1..5], &91u32.to_le_bytes());
    assert_eq!(&data[5..13], &1_000_000_001u64.to_le_bytes());
    assert_eq!(&data[13..], payload);
    message(&mut socket, |m| status(m, "Upstream Gap")).await;
    wait_for_ack(&accepted).await;
    let recovered = br#"{"x":43}"#;
    hub.offer(id, record(3, recovered.len()), recovered, false, 0)
        .unwrap();
    assert_eq!(
        &message(&mut socket, Message::is_binary).await.into_data()[13..],
        recovered
    );

    host_stop.send(()).unwrap();
    timeout(WAIT, host_task).await.unwrap().unwrap().unwrap();
    message(&mut socket, |m| status(m, "Unknown")).await;
    subscriptions(&hub, 0).await;
    drop((accepted, dropped, hub));
    assert_eq!(budget.used_bytes(), 0);

    let (server, restarted) = host(&budget, upstream).await;
    let (host_stop, host_done) = oneshot::channel();
    let host_task = tokio::spawn(server.serve(async {
        let _ = host_done.await;
    }));
    let id = subscriptions(&restarted, 1).await[0];
    message(&mut socket, |m| status(m, "Reconnected")).await;
    restarted
        .offer(id, record(1, recovered.len()), recovered, false, 0)
        .unwrap();
    assert_eq!(
        &message(&mut socket, Message::is_binary).await.into_data()[13..],
        recovered
    );
    socket
        .send(Message::text(
            r#"{"op":"unsubscribe","subscriptionIds":[91]}"#,
        ))
        .await
        .unwrap();
    subscriptions(&restarted, 0).await;

    socket.send(subscribe()).await.unwrap();
    let id = subscriptions(&restarted, 1).await[0];
    let mut changed = record(2, recovered.len());
    changed.metadata.schema_version += 1;
    let refused = restarted.offer(id, changed, recovered, false, 0).unwrap();
    message(&mut socket, |m| {
        status(m, "local schema mapping does not match")
    })
    .await;
    let retired = message(&mut socket, |m| {
        m.to_text().ok().is_some_and(|text| {
            serde_json::from_str::<Value>(text).unwrap()
                == json!({
                    "op": "unadvertise", "channelIds": [17]
                })
        })
    })
    .await;
    assert!(retired.is_text());
    subscriptions(&restarted, 0).await;
    assert_eq!(refused.state(), AckState::Unknown);
    drop(refused);

    socket
        .send(Message::text(
            r#"{"op":"subscribe","subscriptions":[{"id":92,"channelId":18}]}"#,
        ))
        .await
        .unwrap();
    subscriptions(&restarted, 1).await;
    viewer_stop.send(()).unwrap();
    timeout(WAIT, viewer).await.unwrap().unwrap().unwrap();
    subscriptions(&restarted, 0).await;
    host_stop.send(()).unwrap();
    timeout(WAIT, host_task).await.unwrap().unwrap().unwrap();
    drop(restarted);
    assert_eq!(budget.used_bytes(), 0);
}

#[tokio::test]
async fn a_full_socket_queue_does_not_hold_the_shared_feed_or_ack_a_rejected_record() {
    let budget = budget();
    let (server, hub) = host(&budget, "127.0.0.1:0".parse().unwrap()).await;
    let address = server.local_addr().unwrap();
    let (host_stop, host_done) = oneshot::channel();
    let host_task = tokio::spawn(server.serve(async {
        let _ = host_done.await;
    }));
    let options = Options {
        receiver_records: 1,
        record_bytes: 64 * 1024,
        ..Options::default()
    };
    let bridge = Bridge::new(RemoteConfig::new(format!("http://{address}")), &options).unwrap();
    let catalog = Catalog::parse(CATALOG.as_bytes()).unwrap();
    let channel = catalog.channel(17).unwrap();
    let payload = br#"{"x":42}"#;
    let frame_bytes = payload.len() + crate::output::DATA_HEADER;
    let (slow, mut slow_data, slow_notices) = Outbox::new(2, frame_bytes);
    let (fast, mut fast_data, _) = Outbox::new(1, frame_bytes);
    let slow_lease = bridge.subscribe(channel, 1, &slow).unwrap();
    let fast_lease = bridge.subscribe(channel, 2, &fast).unwrap();
    let id = subscriptions(&hub, 1).await[0];

    for sequence in 1..=2 {
        let accepted = hub
            .offer(id, record(sequence, payload.len()), payload, false, 0)
            .unwrap();
        let (value, charge) = timeout(WAIT, fast_data.recv())
            .await
            .unwrap()
            .unwrap()
            .into_send()
            .unwrap();
        assert_eq!(&value.into_data()[13..], payload);
        drop(charge);
        wait_for_ack(&accepted).await;
    }
    let mut previous = crate::output::Notices::default();
    assert!(
        slow_notices
            .borrow()
            .statuses(&mut previous)
            .iter()
            .any(|m| status(m, "Viewer Gap: 1 records"))
    );

    // Keep the first frame charged while it would be writing to the socket.
    let (_, writing_charge) = slow_data.try_recv().unwrap().into_send().unwrap();
    drop(fast_lease);
    let rejected = hub
        .offer(id, record(3, payload.len()), payload, false, 0)
        .unwrap();
    subscriptions(&hub, 0).await;
    assert_eq!(rejected.state(), AckState::Unknown);
    assert!(
        slow_notices
            .borrow()
            .statuses(&mut previous)
            .iter()
            .any(|m| status(m, "Viewer Gap: 2 records"))
    );
    drop(writing_charge);
    let resumed = subscriptions(&hub, 1).await[0];
    assert_ne!(resumed, id);
    let accepted = hub
        .offer(resumed, record(4, payload.len()), payload, false, 0)
        .unwrap();
    wait_for_ack(&accepted).await;
    drop(slow_lease);
    assert!(slow_data.try_recv().unwrap().into_send().is_none());
    subscriptions(&hub, 0).await;
    bridge.shutdown().await;
    host_stop.send(()).unwrap();
    timeout(WAIT, host_task).await.unwrap().unwrap().unwrap();
    drop((accepted, rejected, hub));
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn catalog_rejects_ambiguous_mappings_and_definitions() {
    let valid: Value = serde_json::from_str(CATALOG).unwrap();
    for (field, value) in [
        ("id", json!(0)),
        ("schema_id", json!("63")),
        ("encoding", json!("protobuf")),
        ("schema", json!(r#""not a JSON schema""#)),
    ] {
        let mut invalid = valid.clone();
        invalid["channels"][0][field] = value;
        assert!(
            Catalog::parse(invalid.to_string().as_bytes()).is_err(),
            "{field}"
        );
    }
    let mut duplicate = valid;
    let first = duplicate["channels"][0].clone();
    duplicate["channels"].as_array_mut().unwrap().push(first);
    assert!(Catalog::parse(duplicate.to_string().as_bytes()).is_err());
}
