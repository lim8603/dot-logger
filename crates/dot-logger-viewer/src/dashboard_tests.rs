#![allow(clippy::unwrap_used)]
//! TASK-032 S2: the dashboard status contract and its loopback HTTP boundary.

use crate::dashboard::{self, Status};
use dot_logger::adapters::remote::{
    ManagementReply, ManagementRequest, RemoteError, RemoteHub, RemoteLimits, RemoteServer,
    ServerConfig,
};
use dot_logger_client::{ClientError, RemoteConfig};
use serde_json::{Value, json};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::{sleep, timeout},
};

const WAIT: Duration = Duration::from_secs(5);

fn reply(report: &Value) -> ManagementReply {
    ManagementReply::Status {
        session: 42,
        revision: 3,
        report: report.to_string().into_bytes(),
    }
}

// A status.v1 report whose latest sample is Unavailable and whose windows are
// Stale or Unavailable: nothing here may become a number or "normal".
fn unobserved() -> Value {
    json!({"schema":"dot_logger.status.v1","session":"0000000000000000000000000000002a",
        "posted_revision":null,
        "latest":{"state":"Unavailable","reason":"NoSample"},
        "windows":[
            {"seconds":10,"state":"Stale","revision":"3","epoch":"1","from_ns":"1","to_ns":"2",
             "counts":{"accepted":"5","overflowed":false}},
            {"seconds":60,"state":"Unavailable","reason":"InsufficientHistory"},
            {"seconds":300,"state":"Unavailable","reason":"InsufficientHistory"}],
        "snapshots":[]})
}

#[test]
fn unavailable_and_stale_values_pass_through_without_invented_numbers() {
    let mut status = Status::new("http://127.0.0.1:9");
    let pending = status.json(1_000);
    assert_eq!(pending["schema"], "dot_logger.viewer.dashboard.v1");
    assert_eq!(pending["poll"]["state"], "pending");
    assert!(pending["last"].is_null());

    status.record(2_000, Ok(reply(&unobserved())));
    let shown = status.json(2_500);
    assert_eq!(shown["poll"]["state"], "ok");
    assert!(shown["poll"]["failure"].is_null());
    assert_eq!(shown["last"]["received_at_unix_ms"], 2_000);
    assert_eq!(shown["last"]["session"], "0000000000000000000000000000002a");
    assert_eq!(shown["last"]["revision"], "3");
    assert_eq!(shown["last"]["metrics"], unobserved());
    assert_eq!(shown["now_unix_ms"], 2_500);
}

#[test]
fn a_host_without_metrics_is_unavailable_not_zero() {
    let mut status = Status::new("http://127.0.0.1:9");
    status.record(
        1,
        Ok(ManagementReply::Status {
            session: 1,
            revision: 1,
            report: Vec::new(),
        }),
    );
    assert_eq!(
        status.json(1)["last"]["metrics"],
        json!({"state":"Unavailable","reason":"no_runtime_metrics_backend"})
    );
}

#[test]
fn failures_keep_the_last_value_and_report_reason_and_time_separately() {
    let mut status = Status::new("http://127.0.0.1:9");
    status.record(1_000, Ok(reply(&unobserved())));
    for (at, outcome, reason) in [
        (
            2_000,
            Err(ClientError::Remote(RemoteError::PermissionDenied)),
            "permission_denied",
        ),
        (3_000, Err(ClientError::Disconnected), "disconnected"),
        (
            4_000,
            Err(ClientError::Remote(RemoteError::TimedOut)),
            "timed_out",
        ),
        (
            5_000,
            Err(ClientError::Remote(RemoteError::Unsupported)),
            "unsupported",
        ),
        // A reply that is not a status.v1 report is corrupt, not a value.
        (
            6_000,
            Ok(reply(&json!({"schema":"dot_logger.other.v1"}))),
            "corrupt",
        ),
        (7_000, Ok(ManagementReply::Blob(vec![1])), "corrupt"),
    ] {
        status.record(at, outcome);
        let shown = status.json(at + 1);
        assert_eq!(shown["poll"]["state"], "failed", "{reason}");
        assert_eq!(shown["poll"]["failure"]["reason"], reason);
        assert_eq!(shown["poll"]["failure"]["at_unix_ms"], at);
        assert_eq!(shown["last"]["received_at_unix_ms"], 1_000, "{reason}");
        assert_eq!(shown["last"]["metrics"], unobserved(), "{reason}");
    }
    status.record(8_000, Ok(reply(&unobserved())));
    let shown = status.json(8_000);
    assert_eq!(shown["poll"]["state"], "ok");
    assert!(shown["poll"]["failure"].is_null());
    assert_eq!(shown["last"]["received_at_unix_ms"], 8_000);
}

async fn exchange(address: SocketAddr, request: &[u8]) -> (String, String) {
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(request).await.unwrap();
    let mut bytes = Vec::new();
    timeout(WAIT, socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    (head.to_ascii_lowercase(), body.to_owned())
}

fn get(path: &str, host: &str, extra: &str) -> Vec<u8> {
    format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n{extra}\r\n").into_bytes()
}

async fn status_json(address: SocketAddr) -> Value {
    let (head, body) = exchange(address, &get("/api/status", &address.to_string(), "")).await;
    assert!(head.starts_with("http/1.1 200"), "{head}");
    assert!(head.contains("content-type: application/json"), "{head}");
    assert!(head.contains("cache-control: no-store"), "{head}");
    serde_json::from_str(&body).unwrap()
}

async fn wait_status(address: SocketAddr, done: impl Fn(&Value) -> bool) -> Value {
    timeout(WAIT, async {
        loop {
            let value = status_json(address).await;
            if done(&value) {
                return value;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}

async fn start(upstream: RemoteConfig) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::spawn(dashboard::serve(listener, upstream, None, async {
        let _ = stopped.await;
    }));
    (address, stop)
}

#[tokio::test]
async fn http_serves_only_loopback_get_requests_with_a_restrictive_policy() {
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let unreachable = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let (address, stop) = start(RemoteConfig::new(unreachable.clone())).await;
    let host = address.to_string();

    // The upstream is unreachable: a failure with no value, never zeros.
    let value = wait_status(address, |v| v["poll"]["state"] == "failed").await;
    assert!(value["last"].is_null());
    assert!(value["poll"]["failure"]["reason"].is_string());
    assert_eq!(value["upstream"], unreachable);

    let (head, body) = exchange(address, &get("/", &host, "")).await;
    assert!(head.starts_with("http/1.1 200"), "{head}");
    assert!(head.contains("content-type: text/html"), "{head}");
    assert!(
        head.contains("content-security-policy: default-src 'none'"),
        "{head}"
    );
    assert!(head.contains("x-content-type-options: nosniff"), "{head}");
    assert!(body.contains("dashboard.js"));
    for path in ["/dashboard.js", "/dashboard.css"] {
        let (head, _) = exchange(address, &get(path, &host, "")).await;
        assert!(head.starts_with("http/1.1 200"), "{path}: {head}");
    }
    let localhost = format!("localhost:{}", address.port());
    let (head, _) = exchange(address, &get("/api/status", &localhost, "")).await;
    assert!(head.starts_with("http/1.1 200"), "{head}");
    let same_origin = format!("Origin: http://{host}\r\n");
    let (head, _) = exchange(address, &get("/api/status", &host, &same_origin)).await;
    assert!(head.starts_with("http/1.1 200"), "{head}");

    let rejected: [(Vec<u8>, &str); 7] = [
        // DNS rebinding: a foreign Host name resolving to loopback.
        (
            get(
                "/api/status",
                &format!("evil.example:{}", address.port()),
                "",
            ),
            "403",
        ),
        (get("/api/status", "", ""), "400"),
        (
            get("/api/status", &host, "Origin: http://evil.example\r\n"),
            "403",
        ),
        (
            format!("POST /api/status HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\n\r\n")
                .into_bytes(),
            "405",
        ),
        (get("/api/status", &host, "Content-Length: 4\r\n"), "400"),
        (get("/missing", &host, ""), "404"),
        (
            get("/", &host, &format!("X-Fill: {}\r\n", "a".repeat(9_000))),
            "431",
        ),
    ];
    for (request, code) in rejected {
        let (head, body) = exchange(address, &request).await;
        assert!(
            head.starts_with(&format!("http/1.1 {code}")),
            "{code}: {head}"
        );
        assert!(!body.contains("schema"), "{code}: no status in a rejection");
    }
    stop.send(()).unwrap();
}

#[tokio::test]
async fn http_status_follows_the_actual_host_reply_and_keeps_it_after_a_failure() {
    let budget = crate::tests::budget();
    let (hub, mut inbox) = RemoteHub::new(
        &budget,
        RemoteLimits {
            connections: 2,
            subscriptions: 1,
            commands: 2,
            message_bytes: 32 * 1024,
            record_bytes: 1024,
            subscription_records: 1,
            subscription_bytes: 1024,
            max_age: Duration::from_secs(10),
            request_timeout: Duration::from_secs(2),
        },
    )
    .unwrap();
    let server = RemoteServer::bind(
        ServerConfig {
            enabled: true,
            address: "127.0.0.1:0".parse().unwrap(),
            ..ServerConfig::default()
        },
        hub.clone(),
    )
    .await
    .unwrap();
    let upstream = format!("http://{}", server.local_addr().unwrap());
    let (server_stop, server_stopped) = oneshot::channel::<()>();
    let served = tokio::spawn(server.serve(async {
        let _ = server_stopped.await;
    }));
    let (address, stop) = start(RemoteConfig::new(upstream)).await;

    // Answer status requests: the first with a report, later ones denied.
    let host = tokio::spawn(async move {
        let mut answered = 0;
        loop {
            let Some(command) = inbox.try_next() else {
                sleep(Duration::from_millis(2)).await;
                continue;
            };
            assert!(matches!(command.request(), ManagementRequest::GetStatus));
            answered += 1;
            command.respond(if answered == 1 {
                Ok(reply(&unobserved()))
            } else {
                Err(RemoteError::PermissionDenied)
            });
            if answered == 3 {
                return inbox;
            }
        }
    });
    let ok = wait_status(address, |v| v["poll"]["state"] == "ok").await;
    assert_eq!(ok["last"]["metrics"], unobserved());
    let denied = wait_status(address, |v| v["poll"]["state"] == "failed").await;
    assert_eq!(denied["poll"]["failure"]["reason"], "permission_denied");
    assert_eq!(
        denied["last"]["received_at_unix_ms"],
        ok["last"]["received_at_unix_ms"]
    );
    assert_eq!(denied["last"]["metrics"], unobserved());
    let inbox = timeout(WAIT, host).await.unwrap().unwrap();

    // Without the host, the last value stays and the failure changes.
    server_stop.send(()).unwrap();
    served.await.unwrap().unwrap();
    let gone = wait_status(address, |v| {
        v["poll"]["failure"]["reason"] != "permission_denied" && v["poll"]["state"] == "failed"
    })
    .await;
    assert_eq!(gone["last"]["metrics"], unobserved());
    stop.send(()).unwrap();
    drop((inbox, hub));
}
