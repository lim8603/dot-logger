//! Explicit real DB tests; scripts/check-influxdb.py owns setup and cleanup.
//! They target InfluxDB OSS 2.9.1. On InfluxDB 3 Core (`--server v3`) every
//! write assertion holds and each test fails at its Flux read-back (HTTP 404);
//! the harness checks that and compares the stored rows with SQL instead.
#![cfg(feature = "influxdb")]
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{adapters::influxdb::*, capture::CaptureReceipt, query::QueryBudget};
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn budget() -> QueryBudget {
    let mut c = support::plan().config().clone();
    c.memory.total_bytes += 2 * 1024 * 1024;
    c.memory.external_bytes = 2 * 1024 * 1024;
    QueryBudget::new(
        &dot_logger::config::SessionBuilder::new(c)
            .validate(&Default::default())
            .unwrap(),
    )
}
fn config(measurement: &str) -> Config {
    Config {
        endpoint: std::env::var("DOT_LOGGER_TEST_INFLUX_URL").unwrap(),
        organization: std::env::var("DOT_LOGGER_TEST_INFLUX_ORG").unwrap(),
        bucket: std::env::var("DOT_LOGGER_TEST_INFLUX_BUCKET").unwrap(),
        token: std::env::var("DOT_LOGGER_TEST_INFLUX_TOKEN").unwrap(),
        session_id: 42,
        measurement: measurement.into(),
        device: "robot_1".into(),
        channels: vec![
            Channel {
                id: 7,
                kind: Kind::Integer,
            },
            Channel {
                id: 8,
                kind: Kind::Boolean,
            },
            Channel {
                id: 9,
                kind: Kind::Float,
            },
            Channel {
                id: 10,
                kind: Kind::Event,
            },
        ],
        collision_policy: CollisionPolicy::RejectNonIncreasing,
        max_points: 8,
        max_batch_bytes: 4096,
        max_event_bytes: 64,
        max_attempts: 2,
        request_timeout: Duration::from_secs(10),
        retry_delay: Duration::from_millis(10),
        ttl: Duration::from_secs(20),
    }
}
fn point(channel: u32, seq: u64, t: i64, value: Value<'_>) -> Point<'_> {
    Point {
        channel,
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: (seq % 2) as u32,
            accepted_seq: seq,
        },
        real_ns: t,
        value,
    }
}
fn query(channel: u32, t: i64) -> NumericQuery {
    NumericQuery {
        channel,
        from_ns: t,
        to_ns: t + 100,
        max_rows: 8,
        max_response_bytes: 8192,
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .try_into()
        .unwrap()
}
fn run(f: impl std::future::Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}
async fn raw_write(measurement: &str, body: String) -> u16 {
    let c = config(measurement);
    let response = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
        .post(format!("{}/api/v2/write", c.endpoint))
        .query(&[
            ("org", c.organization),
            ("bucket", c.bucket),
            ("precision", "ns".into()),
        ])
        .header("Authorization", format!("Token {}", c.token))
        .body(body)
        .send()
        .await
        .unwrap();
    response.status().as_u16()
}
#[test]
#[ignore = "requires digest-pinned isolated DB; run scripts/check-influxdb.py"]
fn real_v2_roundtrip_preserves_typed_values_receipts_and_nanoseconds() {
    run(async {
        let b = budget();
        let mut w = Writer::new(&b, config("tc_roundtrip")).unwrap();
        let t = now();
        let cancel = AtomicBool::new(false);
        let mut points = [
            point(7, 1, t, Value::Integer(i64::MAX)),
            point(7, 2, t + 1, Value::Integer(i64::MIN)),
            point(8, 3, t + 2, Value::Boolean(true)),
            point(9, 4, t + 3, Value::Float(2.5)),
            point(10, 5, t + 4, Value::Event("quoted\"event")),
        ];
        points[3].receipt.accepted_seq = u64::MAX;
        points[3].receipt.producer_id = u32::MAX;
        assert_eq!(
            w.write(&points, &cancel).await.unwrap().outcome,
            Outcome::Acknowledged
        );
        for (channel, expected) in [
            (
                7,
                vec![
                    NumericValue::Integer(i64::MAX),
                    NumericValue::Integer(i64::MIN),
                ],
            ),
            (8, vec![NumericValue::Boolean(true)]),
            (9, vec![NumericValue::Float(2.5)]),
        ] {
            let page = w.query_numeric(query(channel, t), &cancel).await.unwrap();
            assert_eq!(
                page.points().iter().map(|p| p.value).collect::<Vec<_>>(),
                expected
            );
            for row in page.points() {
                let original = points.iter().find(|p| p.receipt == row.receipt).unwrap();
                assert_eq!(row.real_ns, original.real_ns);
            }
        }
        assert!(matches!(
            w.query_numeric(
                NumericQuery {
                    max_rows: 1,
                    ..query(7, t)
                },
                &cancel
            )
            .await,
            Err(QueryError::Limit)
        ));
        assert!(
            w.query_numeric(
                NumericQuery {
                    from_ns: t + 20,
                    ..query(7, t)
                },
                &cancel
            )
            .await
            .unwrap()
            .points()
            .is_empty()
        );
        drop(w);
        assert_eq!(b.used_bytes(), 0);
    });
}
#[test]
#[ignore = "requires digest-pinned isolated DB; run scripts/check-influxdb.py"]
fn real_v2_duplicate_merge_is_not_original_record_preservation() {
    run(async {
        let b = budget();
        let mut w = Writer::new(&b, config("tc_duplicate")).unwrap();
        let t = now();
        let cancel = AtomicBool::new(false);
        let original = point(7, 1, t, Value::Integer(1));
        assert_eq!(
            w.write(&[original], &cancel).await.unwrap().outcome,
            Outcome::Acknowledged
        );
        assert_eq!(
            w.write(&[point(7, 2, t, Value::Integer(9))], &cancel).await,
            Err(Error::Collision)
        );
        assert_eq!(
            w.write(&[point(7, 2, t - 1, Value::Integer(9))], &cancel)
                .await,
            Err(Error::Collision)
        );
        // Test-only bypass demonstrates why fresh/exclusive ownership is mandatory.
        assert_eq!(
            raw_write(
                "tc_duplicate",
                format!("tc_duplicate,channel=7,device=robot_1 integer=9i {t}")
            )
            .await,
            204
        );
        let page = w.query_numeric(query(7, t), &cancel).await.unwrap();
        assert_eq!(page.points().len(), 1);
        assert_eq!(page.points()[0].value, NumericValue::Integer(9));
        assert_eq!(page.points()[0].receipt, original.receipt);
        drop((page, w));
        assert_eq!(b.used_bytes(), 0);
    });
}
#[test]
#[ignore = "requires digest-pinned isolated DB; run scripts/check-influxdb.py"]
fn real_v2_partial_type_failure_reports_unknown_and_retains_valid_rows() {
    run(async {
        let b = budget();
        let mut w = Writer::new(&b, config("tc_partial")).unwrap();
        let t = now();
        let cancel = AtomicBool::new(false);
        assert_eq!(
            raw_write(
                "tc_partial",
                format!(
                    "tc_partial,channel=7,device=robot_1 integer=\"conflict\" {}",
                    t - 1
                )
            )
            .await,
            204
        );
        let report = w
            .write(
                &[
                    point(7, 1, t, Value::Integer(1)),
                    point(8, 2, t + 1, Value::Boolean(true)),
                ],
                &cancel,
            )
            .await
            .unwrap();
        assert_eq!(report.outcome, Outcome::Unknown);
        assert!(matches!(report.status, Some(400 | 422)));
        assert_eq!(report.attempts, 1);
        let page = w.query_numeric(query(8, t), &cancel).await.unwrap();
        assert_eq!(page.points().len(), 1);
        assert_eq!(page.points()[0].receipt.accepted_seq, 2);
        assert_eq!(page.points()[0].value, NumericValue::Boolean(true));
        drop((page, w));
        assert_eq!(b.used_bytes(), 0);
    });
}

#[path = "support/influx_routed.rs"]
mod routed_fixture;
#[test]
#[ignore = "requires digest-pinned isolated DB; run scripts/check-influxdb.py"]
fn real_v2_delivery_ack_and_failure_keep_original_completion_identity() {
    use dot_logger::{
        capture::{CaptureOwner, ChannelConfig, Importance},
        completion::{Boundary, QueryOutcome, Update},
        pool::RawRecordOutcome,
        routing::Router,
    };
    use std::time::Instant;
    for failure in [false, true] {
        let plan = routed_fixture::plan();
        let route = routed_fixture::route(true);
        let budget = QueryBudget::new(&plan);
        let measurement = if failure {
            "tc_routed_failed"
        } else {
            "tc_routed"
        };
        let mut cfg = config(measurement);
        cfg.channels.retain(|c| c.id == 7);
        let t = now();
        if failure {
            assert_eq!(
                routed_fixture::runtime(raw_write(
                    measurement,
                    format!(
                        "{measurement},channel=7,device=robot_1 integer=\"conflict\" {}",
                        t - 1
                    )
                )),
                204
            );
        }
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &route,
            routed_fixture::tracking(),
        )
        .unwrap();
        owner.scope(|mut scope| {
            let c = scope.completion().unwrap();
            let (mut producer, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![a, b], route).unwrap();
            let mut db = RoutedWriter::new(
                Writer::new(&budget, cfg).unwrap(),
                c,
                20,
                routed_fixture::schemas(),
            )
            .unwrap();
            router.scope(|mut dispatch, mut outputs| {
                let mut loan = producer.try_acquire(8, Importance::Normal).unwrap();
                loan.payload_mut().copy_from_slice(&(-123i64).to_le_bytes());
                let mut metadata = support::meta();
                metadata.real_ns = t;
                let receipt = match producer.try_record_raw(metadata, loan).unwrap() {
                    RawRecordOutcome::Accepted(r) => r,
                    _ => panic!("skipped"),
                };
                dispatch.turn(Instant::now(), &mut [None; 8]).unwrap();
                let file = outputs[0].try_next(Instant::now()).unwrap().unwrap();
                file.report(Update::Confirmed(Boundary::Written)).unwrap();
                drop(file);
                let attempt = db.write_delivery(
                    outputs[1].try_next(Instant::now()).unwrap().unwrap(),
                    &AtomicBool::new(false),
                );
                assert_eq!(
                    attempt.result().unwrap().outcome,
                    if failure {
                        Outcome::Unknown
                    } else {
                        Outcome::Acknowledged
                    }
                );
                assert_eq!(
                    c.poll(receipt, 20, Boundary::DatabaseAcknowledged)
                        .unwrap()
                        .outcome,
                    QueryOutcome::Pending
                );
                if failure {
                    assert!(matches!(attempt.result().unwrap().status, Some(400 | 422)));
                    assert_eq!(attempt.result().unwrap().attempts, 1);
                }
                attempt.publish().unwrap();
                assert_eq!(
                    c.poll(receipt, 20, Boundary::DatabaseAcknowledged)
                        .unwrap()
                        .outcome,
                    if failure {
                        QueryOutcome::Unknown
                    } else {
                        QueryOutcome::Complete
                    }
                );
                assert_eq!(dispatch.snapshot().raw_retained_bytes, 8);
                drop(attempt);
                assert_eq!(
                    dispatch
                        .turn(Instant::now(), &mut [None; 8])
                        .unwrap()
                        .reclaimed,
                    1
                );
                let mut low = db.into_writer();
                if !failure {
                    let page = routed_fixture::runtime(
                        low.query_numeric(query(7, t), &AtomicBool::new(false)),
                    )
                    .unwrap();
                    assert_eq!(page.points().len(), 1);
                    assert_eq!(page.points()[0].receipt, receipt);
                    assert_eq!(page.points()[0].real_ns, t);
                    assert_eq!(page.points()[0].value, NumericValue::Integer(-123));
                }
                drop(low);
            });
        });
        assert_eq!(budget.used_bytes(), 0);
    }
}
