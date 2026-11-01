//! TC-008 finite v2 write scope against a scripted TCP peer, not a real DB.
#![cfg(feature = "influxdb")]
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{adapters::influxdb::*, capture::CaptureReceipt, query::QueryBudget};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

fn budget() -> QueryBudget {
    let mut config = support::plan().config().clone();
    config.memory.total_bytes += 1024 * 1024;
    config.memory.external_bytes = 1024 * 1024;
    QueryBudget::new(
        &dot_logger::config::SessionBuilder::new(config)
            .validate(&Default::default())
            .unwrap(),
    )
}
fn config(endpoint: String) -> Config {
    Config {
        endpoint,
        organization: "test org".into(),
        bucket: "exclusive-bucket".into(),
        token: "test-secret-do-not-log".into(),
        session_id: 42,
        measurement: "ticks".into(),
        device: "robot 1".into(),
        channels: vec![
            Channel {
                id: 7,
                kind: Kind::Integer,
            },
            Channel {
                id: 8,
                kind: Kind::Event,
            },
        ],
        collision_policy: CollisionPolicy::RejectNonIncreasing,
        max_points: 8,
        max_batch_bytes: 4096,
        max_event_bytes: 64,
        max_attempts: 2,
        request_timeout: Duration::from_secs(1),
        retry_delay: Duration::from_millis(1),
        ttl: Duration::from_secs(3),
    }
}
fn point(channel: u32, seq: u64, real_ns: i64, value: Value<'_>) -> Point<'_> {
    Point {
        channel,
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: seq,
        },
        real_ns,
        value,
    }
}
fn run<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}
// Every response closes the connection; no daemon or background task outlives join.
fn server(replies: Vec<&'static str>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    (
        endpoint,
        std::thread::spawn(move || {
            let end = std::time::Instant::now() + Duration::from_secs(5);
            let mut requests = Vec::new();
            for reply in replies {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && std::time::Instant::now() < end =>
                        {
                            std::thread::sleep(Duration::from_millis(1))
                        }
                        other => panic!("missing request: {other:?}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut data = Vec::new();
                let mut chunk = [0; 1024];
                loop {
                    let n = stream.read(&mut chunk).unwrap();
                    assert_ne!(n, 0);
                    data.extend_from_slice(&chunk[..n]);
                    if let Some(split) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&data[..split]).to_lowercase();
                        let len: usize = headers
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length: "))
                            .unwrap()
                            .parse()
                            .unwrap();
                        if data.len() >= split + 4 + len {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(data).unwrap());
                if !reply.is_empty() {
                    stream.write_all(reply.as_bytes()).unwrap();
                }
            }
            requests
        }),
    )
}
const ACK: &str = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
#[test]
fn ack_preserves_typed_fields_receipt_real_time_and_releases_shared_budget() {
    let (url, peer) = server(vec![ACK]);
    let budget = budget();
    let mut writer = Writer::new(&budget, config(url)).unwrap();
    assert!(budget.used_bytes() > 0);
    let report = run(writer.write(
        &[
            point(7, 1, 123, Value::Integer(9)),
            point(8, 2, 124, Value::Event("a\"b\\c")),
        ],
        &AtomicBool::new(false),
    ))
    .unwrap();
    assert_eq!(report.outcome, Outcome::Acknowledged);
    assert_eq!(report.attempts, 1);
    let requests = peer.join().unwrap();
    assert!(
        requests[0]
            .starts_with("POST /api/v2/write?org=test+org&bucket=exclusive-bucket&precision=ns ")
    );
    assert!(requests[0].contains("ticks,channel=7,device=robot\\ 1 integer=9i,session=\"0000000000000000000000000000002a\",producer=0u,sequence=1u 123\n"));
    assert!(requests[0].contains("event=\"a\\\"b\\\\c\""));
    drop(writer);
    assert_eq!(budget.used_bytes(), 0);
}
#[test]
fn duplicate_across_producers_and_older_timestamps_fail_before_network() {
    let (url, peer) = server(vec![ACK]);
    let budget = budget();
    let mut writer = Writer::new(&budget, config(url)).unwrap();
    let cancel = AtomicBool::new(false);
    let first = point(7, 1, 123, Value::Integer(1));
    let mut second = point(7, 1, 123, Value::Integer(2));
    second.receipt.producer_id = 1;
    assert_eq!(
        run(writer.write(&[first, second], &cancel)),
        Err(Error::Collision)
    );
    // Failed batch preflight must not consume its timestamp.
    assert_eq!(
        run(writer.write(&[first], &cancel)).unwrap().outcome,
        Outcome::Acknowledged
    );
    assert_eq!(run(writer.write(&[second], &cancel)), Err(Error::Collision));
    assert_eq!(
        run(writer.write(&[point(7, 2, 122, Value::Integer(3))], &cancel)),
        Err(Error::Collision)
    );
    assert_eq!(peer.join().unwrap().len(), 1);
}
#[test]
fn lost_response_retry_uses_identical_body_and_partial_write_is_unknown() {
    let (url, peer) = server(vec![
        "",
        "HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 6\r\nConnection: close\r\n\r\nsecret",
    ]);
    let budget = budget();
    let mut writer = Writer::new(&budget, config(url)).unwrap();
    let report = run(writer.write(
        &[point(7, 1, 123, Value::Integer(1))],
        &AtomicBool::new(false),
    ))
    .unwrap();
    assert_eq!(report.outcome, Outcome::Unknown);
    assert_eq!(report.status, Some(422));
    assert_eq!(report.attempts, 2);
    let requests = peer.join().unwrap();
    assert_eq!(
        requests[0].split("\r\n\r\n").nth(1),
        requests[1].split("\r\n\r\n").nth(1)
    );
    assert!(!format!("{report:?}").contains("secret"));
}
#[test]
fn input_limits_types_and_cancel_are_preflight_only() {
    let budget = budget();
    let mut writer = Writer::new(&budget, config("http://127.0.0.1:1".into())).unwrap();
    let cancel = AtomicBool::new(false);
    for input in [
        point(7, 1, 1, Value::Float(f64::NAN)),
        point(8, 1, 1, Value::Event("bad\nline")),
        point(99, 1, 1, Value::Integer(1)),
        point(7, 1, i64::MIN, Value::Integer(1)),
    ] {
        assert_eq!(
            run(writer.write(&[input], &cancel)),
            Err(Error::InvalidInput)
        );
    }
    assert_eq!(
        run(writer.write(&[point(8, 1, 1, Value::Event(&"x".repeat(65)))], &cancel)),
        Err(Error::Limit)
    );
    cancel.store(true, Ordering::Release);
    let report = run(writer.write(&[point(7, 1, 1, Value::Integer(1))], &cancel)).unwrap();
    assert_eq!(
        (report.outcome, report.stop, report.attempts),
        (Outcome::NotSent, Stop::Cancelled, 0)
    );
}
#[test]
fn endpoint_credentials_redirect_and_finite_configuration_are_rejected_or_isolated() {
    let budget = budget();
    for url in [
        "http://example.com",
        "https://user:password@example.com",
        "https://example.com/path",
        "https://example.com?token=secret",
        "https://example.com/#secret",
    ] {
        assert!(matches!(
            Writer::new(&budget, config(url.into())),
            Err(Error::InvalidInput)
        ));
        assert_eq!(budget.used_bytes(), 0);
    }
    let (url, peer) = server(vec![
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:1/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    ]);
    let mut writer = Writer::new(&budget, config(url)).unwrap();
    let report = run(writer.write(
        &[point(7, 1, 1, Value::Integer(1))],
        &AtomicBool::new(false),
    ))
    .unwrap();
    assert_eq!(report.outcome, Outcome::Unknown);
    assert_eq!(report.attempts, 1);
    peer.join().unwrap();
}

#[test]
fn heterogeneous_channels_use_distinct_field_types_in_one_measurement() {
    let (url, peer) = server(vec![ACK]);
    let budget = budget();
    let mut cfg = config(url);
    cfg.channels.extend([
        Channel {
            id: 9,
            kind: Kind::Float,
        },
        Channel {
            id: 10,
            kind: Kind::Boolean,
        },
    ]);
    let mut writer = Writer::new(&budget, cfg).unwrap();
    let report = run(writer.write(
        &[
            point(7, 1, 1, Value::Integer(1)),
            point(9, 2, 1, Value::Float(2.5)),
            point(10, 3, 1, Value::Boolean(true)),
        ],
        &AtomicBool::new(false),
    ))
    .unwrap();
    assert_eq!(report.outcome, Outcome::Acknowledged);
    let requests = peer.join().unwrap();
    for field in [" integer=1i,", " float=2.5,", " boolean=true,"] {
        assert!(requests[0].contains(field), "{field}");
    }
}

#[test]
fn exhausted_response_loss_and_later_rejection_preserve_uncertainty() {
    for responses in [
        vec!["", ""],
        vec![
            "",
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ],
    ] {
        let (url, peer) = server(responses);
        let budget = budget();
        let mut writer = Writer::new(&budget, config(url)).unwrap();
        let cancel = AtomicBool::new(false);
        let input = point(7, 1, 123, Value::Integer(1));
        let report = run(writer.write(&[input], &cancel)).unwrap();
        assert_eq!(report.outcome, Outcome::Unknown);
        assert_eq!(report.attempts, 2);
        // Never resubmit an uncertain timestamp as a new batch.
        assert_eq!(run(writer.write(&[input], &cancel)), Err(Error::Collision));
        assert_eq!(peer.join().unwrap().len(), 2);
    }
}
#[test]
fn retry_after_cannot_extend_ttl_and_initial_auth_rejection_is_definitive() {
    for (response, outcome, stop) in [
        (
            "HTTP/1.1 503 Unavailable\r\nRetry-After: 18446744073709551615\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Outcome::Unknown,
            Stop::Deadline,
        ),
        (
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Outcome::Rejected,
            Stop::Response,
        ),
    ] {
        let (url, peer) = server(vec![response]);
        let budget = budget();
        let mut cfg = config(url);
        cfg.ttl = Duration::from_millis(50);
        let mut writer = Writer::new(&budget, cfg).unwrap();
        let report = run(writer.write(
            &[point(7, 1, 123, Value::Integer(1))],
            &AtomicBool::new(false),
        ))
        .unwrap();
        assert_eq!(
            (report.outcome, report.stop, report.attempts),
            (outcome, stop, 1)
        );
        peer.join().unwrap();
    }
}
#[test]
fn shared_partition_and_encoded_body_limits_reject_without_io() {
    let budget = budget();
    let mut config = config("http://127.0.0.1:1".into());
    config.max_batch_bytes = 256;
    let mut writer = Writer::new(&budget, config).unwrap();
    let cancel = AtomicBool::new(false);
    let points: Vec<_> = (1..=8)
        .map(|seq| point(7, seq, seq as i64, Value::Integer(1)))
        .collect();
    assert_eq!(run(writer.write(&points, &cancel)), Err(Error::Limit));
    drop(writer);
    assert_eq!(budget.used_bytes(), 0);
    let no_budget = QueryBudget::new(&support::plan());
    assert!(matches!(
        Writer::new(&no_budget, crate::config("http://127.0.0.1:1".into())),
        Err(Error::Limit)
    ));
    assert_eq!(no_budget.used_bytes(), 0);
}
#[test]
fn in_flight_cancel_or_timeout_never_claims_remote_rollback() {
    for cancellation in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let peer_cancel = cancel.clone();
        listener.set_nonblocking(true).unwrap();
        let peer = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                if let Ok((stream, _)) = listener.accept() {
                    break stream;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut data = [0; 4096];
            assert!(stream.read(&mut data).unwrap() > 0);
            if cancellation {
                peer_cancel.store(true, Ordering::Release);
            }
            // Keep the remote operation open past local cancellation/timeout.
            std::thread::sleep(Duration::from_millis(150));
        });
        let budget = budget();
        let mut cfg = config(url);
        cfg.max_attempts = 1;
        cfg.request_timeout = Duration::from_millis(100);
        let mut writer = Writer::new(&budget, cfg).unwrap();
        let input = point(7, 1, 1, Value::Integer(1));
        let report = run(writer.write(&[input], &cancel)).unwrap();
        assert_eq!(
            (report.outcome, report.stop),
            (
                Outcome::Unknown,
                if cancellation {
                    Stop::Cancelled
                } else {
                    Stop::Attempts
                }
            )
        );
        peer.join().unwrap();
        cancel.store(false, Ordering::Release);
        assert_eq!(run(writer.write(&[input], &cancel)), Err(Error::Collision));
        drop(writer);
        assert_eq!(budget.used_bytes(), 0);
    }
}
