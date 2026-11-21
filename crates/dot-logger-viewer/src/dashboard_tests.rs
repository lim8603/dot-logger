#![allow(clippy::unwrap_used)]
//! TASK-032 S2: the dashboard status contract and its loopback HTTP boundary.

use crate::dashboard::{self, Query, Status};
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

// ---- S3: read-only Compare and PlanCheck lookups ----

fn compared() -> Value {
    json!({"schema":"dot_logger.compare.v1","comparable":false,
        "before":{"id":"1"},"after":{"id":"2"},
        "differences":{"environment":true},
        "changes":{"accepted_rate":{"state":"Unavailable","reason":"NotComparable"}}})
}

fn planned() -> Value {
    json!({"schema":"dot_logger.check.v1","kind":"plan","plan":"7","capability":"Unprobed"})
}

fn report(value: &Value) -> ManagementReply {
    ManagementReply::Report(value.to_string().into_bytes())
}

#[test]
fn lookups_pass_reports_through_and_name_every_failure() {
    let ok = Query::Compare.json(5, Ok(report(&compared())));
    assert_eq!(ok["schema"], "dot_logger.viewer.request.v1");
    assert_eq!(ok["request"], "compare");
    assert_eq!(ok["at_unix_ms"], 5);
    assert_eq!(ok["outcome"], "ok");
    assert_eq!(ok["report"], compared());
    assert!(ok.get("reason").is_none());
    let plan = Query::Plan.json(6, Ok(report(&planned())));
    assert_eq!(
        (plan["request"].clone(), plan["report"].clone()),
        (json!("plan"), planned())
    );
    for (query, outcome, reason) in [
        (
            Query::Compare,
            Err(ClientError::Remote(RemoteError::Gone)),
            "gone",
        ),
        (
            Query::Plan,
            Err(ClientError::Remote(RemoteError::Unsupported)),
            "unsupported",
        ),
        (
            Query::Plan,
            Err(ClientError::Remote(RemoteError::PermissionDenied)),
            "permission_denied",
        ),
        // A report with another schema, or a check result instead of a plan.
        (Query::Compare, Ok(report(&planned())), "corrupt"),
        (
            Query::Plan,
            Ok(report(
                &json!({"schema":"dot_logger.check.v1","kind":"result"}),
            )),
            "corrupt",
        ),
        (Query::Plan, Ok(ManagementReply::Applied(1)), "corrupt"),
    ] {
        let failed = query.json(9, outcome);
        assert_eq!(failed["outcome"], "failed", "{reason}");
        assert_eq!(failed["reason"], reason);
        assert!(
            failed.get("report").is_none(),
            "{reason}: no partial report"
        );
    }
}

#[tokio::test]
async fn lookup_paths_reject_malformed_ids_before_asking_the_upstream() {
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let unreachable = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let (address, stop) = start(RemoteConfig::new(unreachable)).await;
    let host = address.to_string();
    for path in [
        "/api/compare",
        "/api/compare?before=1",
        "/api/compare?before=1&after=x",
        "/api/compare?before=-1&after=2",
        "/api/compare?before=1&after=2&before=3",
        "/api/plan",
        "/api/plan?id=",
        "/api/plan?id=18446744073709551616",
    ] {
        let (head, _) = exchange(address, &get(path, &host, "")).await;
        assert!(head.starts_with("http/1.1 400"), "{path}: {head}");
    }
    // A well-formed lookup to an unreachable upstream is a named failure.
    let (head, body) = exchange(address, &get("/api/plan?id=7", &host, "")).await;
    assert!(head.starts_with("http/1.1 200"), "{head}");
    assert!(head.contains("cache-control: no-store"), "{head}");
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["outcome"], "failed");
    assert!(value["reason"].is_string());
    stop.send(()).unwrap();
}

#[tokio::test]
async fn lookups_share_the_single_upstream_request_slot_with_status_polling() {
    let budget = crate::tests::budget();
    let (hub, mut inbox) = RemoteHub::new(
        &budget,
        RemoteLimits {
            connections: 2,
            subscriptions: 1,
            commands: 4,
            message_bytes: 32 * 1024,
            record_bytes: 1024,
            subscription_records: 1,
            subscription_bytes: 1024,
            max_age: Duration::from_secs(10),
            request_timeout: Duration::from_secs(3),
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
    let host = address.to_string();

    async fn next(
        inbox: &mut dot_logger::adapters::remote::CommandInbox,
    ) -> dot_logger::adapters::remote::Command {
        timeout(WAIT, async {
            loop {
                if let Some(command) = inbox.try_next() {
                    return command;
                }
                sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .unwrap()
    }
    // Hold the first GetStatus while the browser asks for a comparison: no
    // second upstream request may start until it is answered.
    let first = next(&mut inbox).await;
    assert!(matches!(first.request(), ManagementRequest::GetStatus));
    let lookup = tokio::spawn(exchange(
        address,
        get("/api/compare?before=1&after=2", &host, "").leak(),
    ));
    sleep(Duration::from_millis(300)).await;
    assert!(
        inbox.try_next().is_none(),
        "a second upstream request started"
    );
    first.respond(Ok(reply(&unobserved())));
    // Status polls may come first; the comparison must follow within WAIT.
    let compare = timeout(WAIT, async {
        loop {
            let command = next(&mut inbox).await;
            if !matches!(command.request(), ManagementRequest::GetStatus) {
                return command;
            }
            command.respond(Ok(reply(&unobserved())));
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        compare.request(),
        ManagementRequest::Compare {
            before: 1,
            after: 2
        }
    ));
    compare.respond(Ok(report(&compared())));
    let (head, body) = timeout(WAIT, lookup).await.unwrap().unwrap();
    assert!(head.starts_with("http/1.1 200"), "{head}");
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["outcome"], "ok");
    assert_eq!(value["report"], compared());

    // A host without plans: Unsupported is a named failure, and the
    // connection stays usable for the next lookup.
    let host_task = tokio::spawn(async move {
        let mut plans = 0;
        while plans < 2 {
            let command = next(&mut inbox).await;
            match command.request() {
                ManagementRequest::GetStatus => command.respond(Ok(reply(&unobserved()))),
                ManagementRequest::PlanCheck { plan: 7 } => {
                    plans += 1;
                    command.respond(if plans == 1 {
                        Err(RemoteError::Unsupported)
                    } else {
                        Ok(report(&planned()))
                    });
                }
                _ => panic!("only status and plan lookups are expected"),
            }
        }
        inbox
    });
    let (_, body) = exchange(address, &get("/api/plan?id=7", &host, "")).await;
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (value["outcome"].clone(), value["reason"].clone()),
        (json!("failed"), json!("unsupported"))
    );
    let (_, body) = exchange(address, &get("/api/plan?id=7", &host, "")).await;
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["report"], planned());
    let inbox = timeout(WAIT, host_task).await.unwrap().unwrap();
    stop.send(()).unwrap();
    server_stop.send(()).unwrap();
    served.await.unwrap().unwrap();
    drop((inbox, hub));
}

// ---- REV-070 ----

// RV-166: an upstream with userinfo or a query never reaches the status
// document; a valid one is shown as its checked authority.
#[tokio::test]
async fn the_status_document_shows_only_the_checked_upstream_authority() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let refused = timeout(
        WAIT,
        dashboard::serve(
            listener,
            RemoteConfig::new("http://user:secret@127.0.0.1:9"),
            None,
            std::future::pending(),
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(!refused.to_string().contains("secret"), "{refused}");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::spawn(dashboard::serve(
        listener,
        RemoteConfig::new("http://127.0.0.1:9/"),
        None,
        async {
            let _ = stopped.await;
        },
    ));
    assert_eq!(status_json(address).await["upstream"], "http://127.0.0.1:9");
    stop.send(()).unwrap();
}

// RV-167: a status.v1 report whose consumed fields have other types is
// corrupt; the last valid value stays.
#[test]
fn a_status_report_with_unexpected_field_types_is_corrupt() {
    let mut status = Status::new("http://127.0.0.1:9");
    status.record(1, Ok(reply(&unobserved())));
    for (field, value) in [
        ("latest", json!("Observed")),
        ("windows", json!({"seconds": 10})),
        ("windows", json!([1])),
        ("snapshots", json!("none")),
        ("snapshots", json!([{"id": 1}])),
    ] {
        let mut report = unobserved();
        report[field] = value;
        status.record(2, Ok(reply(&report)));
        let shown = status.json(2);
        assert_eq!(shown["poll"]["failure"]["reason"], "corrupt", "{field}");
        assert_eq!(shown["last"]["metrics"], unobserved(), "{field}");
    }
}

// RV-168: a browser omits the default port 80; IPv6 needs brackets.
#[test]
fn host_headers_default_to_port_80_and_parse_bracketed_ipv6() {
    use dashboard::loopback_host;
    for (host, port, expected) in [
        ("127.0.0.1", 80, Some(true)),
        ("localhost", 80, Some(true)),
        ("LOCALHOST:80", 80, Some(true)),
        ("[::1]", 80, Some(true)),
        ("[::1]:8080", 8080, Some(true)),
        ("127.0.0.1", 8080, Some(false)),
        ("[::1]:80", 8080, Some(false)),
        ("evil.example", 80, Some(false)),
        ("[::2]:80", 80, Some(false)),
        ("::1", 80, None),
        ("[::1", 80, None),
        ("127.0.0.1:", 80, None),
        (":80", 80, None),
        ("127.0.0.1:x", 80, None),
    ] {
        assert_eq!(loopback_host(host, port), expected, "{host} on {port}");
    }
}

// RV-170: after a slow first connection, the host still sees GetStatus
// requests at least one interval apart.
#[tokio::test]
async fn status_requests_stay_one_interval_apart_after_a_slow_connection() {
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
    let target = server.local_addr().unwrap();
    let (server_stop, server_stopped) = oneshot::channel::<()>();
    let served = tokio::spawn(server.serve(async {
        let _ = server_stopped.await;
    }));
    // A relay that holds the first connection for 1.3 s before forwarding.
    let relay = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_address = relay.local_addr().unwrap();
    let relaying = tokio::spawn(async move {
        let mut first = true;
        loop {
            let (mut inbound, _) = relay.accept().await.unwrap();
            let hold = std::mem::take(&mut first);
            tokio::spawn(async move {
                if hold {
                    sleep(Duration::from_millis(1300)).await;
                }
                let mut outbound = TcpStream::connect(target).await.unwrap();
                let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
            });
        }
    });
    let mut config = RemoteConfig::new(format!("http://{relay_address}"));
    config.timeout = Duration::from_secs(5);
    let (_, stop) = start(config).await;
    let mut starts = Vec::new();
    timeout(WAIT, async {
        while starts.len() < 2 {
            if let Some(command) = inbox.try_next() {
                assert!(matches!(command.request(), ManagementRequest::GetStatus));
                starts.push(std::time::Instant::now());
                command.respond(Ok(reply(&unobserved())));
            } else {
                sleep(Duration::from_millis(1)).await;
            }
        }
    })
    .await
    .unwrap();
    let gap = starts[1] - starts[0];
    assert!(
        gap >= Duration::from_millis(900),
        "GetStatus 1 s apart: {gap:?}"
    );
    stop.send(()).unwrap();
    relaying.abort();
    server_stop.send(()).unwrap();
    served.await.unwrap().unwrap();
    drop((inbox, hub));
}

// REV-071 RV-179: an Origin with an out-of-range port is not a local origin.
#[test]
fn origins_with_an_invalid_port_are_not_local() {
    use crate::websocket::local_origin;
    for (origin, expected) in [
        ("http://localhost:8080/", true),
        ("http://127.0.0.1/", true),
        ("http://[::1]:65535/", true),
        ("http://localhost:99999/", false),
        ("http://localhost:0/", false),
        ("http://localhost:/", false),
        ("http://[::1]:70000/", false),
    ] {
        assert_eq!(local_origin(origin), expected, "{origin}");
    }
}

// REV-071 RV-179: the WebSocket handshake checks Host like the dashboard
// does, so a rebinding name with a loopback Origin is still refused.
#[test]
fn websocket_handshake_requires_a_loopback_host_on_its_port() {
    use crate::websocket::negotiate_for_tests as negotiate;
    let request = |host: &str| {
        tokio_tungstenite::tungstenite::handshake::server::Request::builder()
            .uri("/")
            .header("host", host)
            .header("sec-websocket-protocol", "foxglove.websocket.v1")
            .header("origin", "http://localhost:8765/")
            .body(())
            .unwrap()
    };
    assert!(negotiate(&request("127.0.0.1:8765"), 8765).is_ok());
    assert!(negotiate(&request("localhost:8765"), 8765).is_ok());
    assert!(negotiate(&request("evil.example:8765"), 8765).is_err());
    assert!(negotiate(&request("127.0.0.1:9999"), 8765).is_err());
}

// REV-071 RV-179: the status worker ends once every job sender is gone
// instead of polling forever.
#[tokio::test]
async fn status_worker_ends_when_its_queue_closes() {
    let (jobs, queue) = tokio::sync::mpsc::channel(1);
    drop(jobs);
    let status = std::sync::Arc::new(std::sync::Mutex::new(Status::new("http://127.0.0.1:1")));
    let upstream = RemoteConfig::new("http://127.0.0.1:1");
    let ended = tokio::time::timeout(
        Duration::from_secs(5),
        dashboard::work(status, upstream, queue),
    )
    .await;
    assert!(
        ended.is_ok(),
        "the worker kept polling after its queue closed"
    );
}
