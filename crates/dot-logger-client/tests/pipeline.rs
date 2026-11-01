//! TASK-020 S2: same-receipt file/DB/subscription and explicit config comparison.
#![cfg(feature = "integration-example")]
#![allow(clippy::unwrap_used)]
#[path = "../examples/support/pipeline/mod.rs"]
mod pipeline;
use pipeline::*;
use std::sync::atomic::AtomicBool;

fn config() -> dot_logger::adapters::influxdb::Config {
    database_config(
        "http://127.0.0.1:1".into(),
        "org".into(),
        "bucket".into(),
        "test-only-secret".into(),
        "s2_test".into(),
        42,
    )
}
#[test]
fn db_transport_failure_preserves_file_subscription_and_explicit_workload_difference() {
    let report = run(config(), Options::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(report.before.accepted, 30);
    assert_eq!(report.before.skipped, 0);
    assert_eq!(report.after.accepted, 15);
    assert_eq!(report.after.skipped, 15);
    assert_eq!(report.file_verified, 45);
    assert_eq!(report.remote_verified, 45);
    assert_eq!(report.raw_reclaimed, 20);
    assert_eq!(report.before.file.complete, 30);
    assert_eq!(report.after.file.complete, 15);
    assert_eq!(report.before.database.unknown, 20);
    assert_eq!(report.after.database.assigned, 0);
    assert_eq!(report.external_after_close, 0);
    assert!(!report.comparison.differences.comparable());
    assert!(report.comparison.differences.workload);
    assert!(!report.is_complete());
    assert!(matches!(
        report.comparison.accepted_rate,
        dot_logger::telemetry::capacity::Value::Unavailable(_)
    ));
}
#[test]
fn full_last_consumer_queue_never_acknowledges_but_file_remains_complete() {
    let report = run(
        config(),
        Options {
            receiver: ReceiverMode::FullOnLast,
            ..Default::default()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report.file_verified, 45);
    assert_eq!(report.remote_verified, 44);
    assert_eq!(report.after.receiver.complete, 14);
    assert_eq!(report.after.receiver.unknown, 1);
    assert_eq!(report.raw_reclaimed, 20);
    assert_eq!(report.external_after_close, 0);
}
#[test]
fn pre_cancel_and_zero_deadline_admit_no_work() {
    assert!(run(config(), Options::default(), &AtomicBool::new(true)).is_err());
    assert!(
        run(
            config(),
            Options {
                duration: std::time::Duration::ZERO,
                ..Default::default()
            },
            &AtomicBool::new(false)
        )
        .is_err()
    );
}
#[test]
#[ignore = "requires owned isolated InfluxDB; scripts/check-influxdb.py --pipeline-example"]
fn real_db_before_after_file_subscription_and_numeric_query_match_originals() {
    let report = run(
        database_from_env("tc_pipeline_live").unwrap(),
        Options {
            verify_query: true,
            ..Default::default()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report.before.database.complete, 20);
    assert_eq!(report.after.database.assigned, 0);
    assert_eq!(report.numeric_verified, Some(10));
    assert!(report.is_complete());
    assert_eq!(report.file_verified, 45);
    assert_eq!(report.remote_verified, 45);
    assert_eq!(report.external_after_close, 0);
}

#[test]
fn invalid_db_and_nested_runtime_fail_without_implicit_worker_or_secret_text() {
    let mut db = config();
    db.endpoint = "http://credential@example.invalid".into();
    let error = run(db, Options::default(), &AtomicBool::new(false)).unwrap_err();
    assert!(!error.to_string().contains("credential"));
    assert!(!error.to_string().contains("test-only-secret"));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        assert!(run(config(), Options::default(), &AtomicBool::new(false)).is_err());
    });
}
