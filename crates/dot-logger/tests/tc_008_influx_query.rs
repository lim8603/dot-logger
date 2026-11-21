//! Bounded v2 numeric query parsing, scope and late-error rejection.
#![cfg(feature = "influxdb")]
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{adapters::influxdb::*, query::QueryBudget};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::AtomicBool,
    time::Duration,
};
fn budget() -> QueryBudget {
    let mut c = support::plan().config().clone();
    c.memory.total_bytes += 1024 * 1024;
    c.memory.external_bytes = 1024 * 1024;
    QueryBudget::new(
        &dot_logger::config::SessionBuilder::new(c)
            .validate(&Default::default())
            .unwrap(),
    )
}
fn config(endpoint: String) -> Config {
    Config {
        endpoint,
        organization: "test org".into(),
        bucket: "bucket\"safe".into(),
        token: "secret-not-in-errors".into(),
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
        ttl: Duration::from_secs(2),
    }
}
fn query() -> NumericQuery {
    NumericQuery {
        channel: 7,
        from_ns: 10,
        to_ns: 100,
        max_rows: 2,
        max_response_bytes: 4096,
    }
}
fn run<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}
const CSV: &str = "#datatype,string,long,unsignedLong,long,unsignedLong,string,long\r\n#group,false,false,false,false,false,false,false\r\n#default,_result,,,,,,\r\n,result,table,producer,real_ns,sequence,session,value\r\n,,0,1,12,3,0000000000000000000000000000002a,99\r\n";
fn server(body: String, status: u16) -> (String, std::thread::JoinHandle<String>) {
    server_response(format!(
        "HTTP/1.1 {status} Response\r\nContent-Type: text/csv\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ))
}
fn server_response(response: String) -> (String, std::thread::JoinHandle<String>) {
    server_action(move |stream| stream.write_all(response.as_bytes()).unwrap())
}
fn server_action(
    action: impl FnOnce(&mut std::net::TcpStream) + Send + 'static,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (
        url,
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(4);
            let mut stream = loop {
                if let Ok((s, _)) = listener.accept() {
                    // Windows accepted sockets inherit the listener's non-blocking mode.
                    s.set_nonblocking(false).unwrap();
                    break s;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut data = Vec::new();
            let mut buf = [0; 1024];
            loop {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                data.extend_from_slice(&buf[..n]);
                if let Some(end) = data.windows(4).position(|b| b == b"\r\n\r\n") {
                    let h = String::from_utf8_lossy(&data[..end]).to_lowercase();
                    let len: usize = h
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if data.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            action(&mut stream);
            String::from_utf8(data).unwrap()
        }),
    )
}
#[test]
fn typed_query_preserves_real_ns_and_receipt_and_owns_result_reservation() {
    let (url, peer) = server(CSV.into(), 200);
    let budget = budget();
    let mut writer = Writer::new(&budget, config(url)).unwrap();
    let baseline = budget.used_bytes();
    let page = run(writer.query_numeric(query(), &AtomicBool::new(false))).unwrap();
    assert_eq!(page.scope().session_id, 42);
    assert_eq!(page.scope().channel, 7);
    assert_eq!(page.scope().measurement, "ticks");
    assert_eq!(page.query().from_ns, 10);
    assert_eq!(page.points().len(), 1);
    let p = &page.points()[0];
    assert_eq!(p.real_ns, 12);
    assert_eq!(p.value, NumericValue::Integer(99));
    assert_eq!(
        (
            p.receipt.session_id,
            p.receipt.producer_id,
            p.receipt.accepted_seq
        ),
        (42, 1, 3)
    );
    assert!(budget.used_bytes() > baseline);
    drop(writer);
    assert!(budget.used_bytes() > 0);
    drop(page);
    assert_eq!(budget.used_bytes(), 0);
    let request = peer.join().unwrap();
    assert!(request.starts_with("POST /api/v2/query?org=test+org "));
    let request_json: serde_json::Value =
        serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    let flux = request_json["query"].as_str().unwrap();
    assert!(flux.contains("bucket\\\"safe"));
    assert!(flux.contains("group(columns: [])"));
    assert!(flux.contains("limit(n: 3)"));
    assert_eq!(
        request_json["dialect"]["annotations"],
        serde_json::json!(["datatype", "group", "default"])
    );
}
#[test]
fn late_error_foreign_scope_bad_type_or_overlimit_never_returns_partial_success() {
    let extra = ",,0,1,13,4,0000000000000000000000000000002a,100\r\n,,0,1,14,5,0000000000000000000000000000002a,101\r\n";
    for (body, expected) in [
        (
            format!("{CSV},error,reference\r\n,secret,123\r\n"),
            QueryError::Corrupt,
        ),
        (
            CSV.replace(
                "0000000000000000000000000000002a",
                "0000000000000000000000000000002b",
            ),
            QueryError::Corrupt,
        ),
        (
            CSV.replace("string,long\r\n#group", "string,double\r\n#group"),
            QueryError::Corrupt,
        ),
        (format!("{CSV}{extra}"), QueryError::Limit),
    ] {
        let (url, peer) = server(body, 200);
        let b = budget();
        let mut w = Writer::new(&b, config(url)).unwrap();
        let before = b.used_bytes();
        assert!(
            matches!(run(w.query_numeric(query(),&AtomicBool::new(false))),Err(e) if e==expected)
        );
        assert_eq!(b.used_bytes(), before);
        peer.join().unwrap();
    }
}
#[test]
fn query_budget_scope_and_cancellation_are_checked_before_io() {
    let b = budget();
    let mut w = Writer::new(&b, config("http://127.0.0.1:1".into())).unwrap();
    for q in [
        NumericQuery {
            channel: 8,
            ..query()
        },
        NumericQuery {
            from_ns: 100,
            ..query()
        },
        NumericQuery {
            max_rows: 0,
            ..query()
        },
        NumericQuery {
            max_response_bytes: 1024 * 1024 + 1,
            ..query()
        },
    ] {
        assert!(matches!(
            run(w.query_numeric(q, &AtomicBool::new(false))),
            Err(QueryError::InvalidInput)
        ));
    }
    assert!(matches!(
        run(w.query_numeric(query(), &AtomicBool::new(true))),
        Err(QueryError::Cancelled)
    ));
    let mut c = config("http://127.0.0.1:1".into());
    c.bucket = "${die()}".into();
    let mut w = Writer::new(&b, c).unwrap();
    assert!(matches!(
        run(w.query_numeric(query(), &AtomicBool::new(false))),
        Err(QueryError::InvalidInput)
    ));
}
#[test]
fn http_error_is_redacted_and_response_bytes_are_bounded() {
    for (body, status, expected) in [
        ("secret-not-in-errors".repeat(200), 200, QueryError::Limit),
        ("secret-not-in-errors".into(), 401, QueryError::Http(401)),
    ] {
        let (url, peer) = server(body, status);
        let b = budget();
        let mut w = Writer::new(&b, config(url)).unwrap();
        let result = run(w.query_numeric(
            NumericQuery {
                max_response_bytes: 256,
                ..query()
            },
            &AtomicBool::new(false),
        ));
        assert!(matches!(result,Err(e) if e==expected));
        peer.join().unwrap();
    }
}

#[test]
fn chunked_response_limit_and_truncated_body_never_return_a_prefix() {
    let too_large = "x".repeat(300);
    let chunked = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{too_large}\r\n0\r\n\r\n",
        too_large.len()
    );
    let truncated = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{CSV}",
        CSV.len() + 100
    );
    for (response, bytes, expected) in [
        (chunked, 256, QueryError::Limit),
        (truncated, 4096, QueryError::Transport),
    ] {
        let (url, peer) = server_response(response);
        let b = budget();
        let mut w = Writer::new(&b, config(url)).unwrap();
        assert!(
            matches!(run(w.query_numeric(NumericQuery{max_response_bytes:bytes,..query()},&AtomicBool::new(false))),Err(e)if e==expected)
        );
        peer.join().unwrap();
    }
}
#[test]
fn query_reservations_cannot_bypass_the_writer_session_partition() {
    let b = budget();
    let mut w = Writer::new(&b, config("http://127.0.0.1:1".into())).unwrap();
    let before = b.used_bytes();
    assert!(matches!(
        run(w.query_numeric(
            NumericQuery {
                max_response_bytes: 1024 * 1024,
                ..query()
            },
            &AtomicBool::new(false)
        )),
        Err(QueryError::Limit)
    ));
    assert_eq!(b.used_bytes(), before);
}
#[test]
fn malformed_schema_range_and_duplicate_rows_are_rejected() {
    for body in [
        CSV.replace(
            "#datatype,string,long,unsignedLong,long,unsignedLong,string,long",
            "#datatype,string,long,unsignedLong,long,unsignedLong,string,double",
        ),
        CSV.replace(",12,", ",100,"),
        format!("{CSV},,0,1,12,4,0000000000000000000000000000002a,1\r\n"),
        format!("#default,_result,,,,,,\r\n{CSV}"),
    ] {
        let (url, peer) = server(body, 200);
        let b = budget();
        let mut w = Writer::new(&b, config(url)).unwrap();
        assert!(matches!(
            run(w.query_numeric(query(), &AtomicBool::new(false))),
            Err(QueryError::Corrupt)
        ));
        peer.join().unwrap();
    }
}

#[test]
fn in_flight_query_cancel_request_timeout_and_ttl_drop_partial_results() {
    for mode in 0..3 {
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let (url, peer) = server_action(move |stream| {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n{CSV}"
            )
            .unwrap();
            if mode == 0 {
                signal.store(true, std::sync::atomic::Ordering::Release);
            }
            std::thread::sleep(Duration::from_millis(150));
        });
        let b = budget();
        let mut cfg = config(url);
        if mode == 1 {
            cfg.request_timeout = Duration::from_millis(100);
        }
        if mode == 2 {
            cfg.ttl = Duration::from_millis(50);
        }
        let mut w = Writer::new(&b, cfg).unwrap();
        let before = b.used_bytes();
        let expected = if mode == 0 {
            QueryError::Cancelled
        } else {
            QueryError::TimedOut
        };
        assert!(matches!(run(w.query_numeric(query(),&cancel)),Err(e)if e==expected));
        peer.join().unwrap();
        assert_eq!(b.used_bytes(), before);
        drop(w);
        assert_eq!(b.used_bytes(), 0);
    }
}
#[test]
fn nonfinite_float_and_excess_csv_cells_are_rejected() {
    for body in [
        CSV.replace("string,long\r\n#group", "string,double\r\n#group")
            .replace(",99\r\n", ",NaN\r\n"),
        ",".repeat(10000),
    ] {
        let (url, peer) = server(body, 200);
        let b = budget();
        let mut cfg = config(url);
        cfg.channels[0].kind = Kind::Float;
        let mut w = Writer::new(&b, cfg).unwrap();
        assert!(matches!(
            run(w.query_numeric(
                NumericQuery {
                    max_response_bytes: 16384,
                    ..query()
                },
                &AtomicBool::new(false)
            )),
            Err(QueryError::Corrupt)
        ));
        peer.join().unwrap();
    }
}
