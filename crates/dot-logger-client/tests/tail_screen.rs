//! TASK-020 S3: bounded screen history must expose subscription discontinuity.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used)]
#[path = "../examples/support/tail/mod.rs"]
mod tail;
use dot_logger::{adapters::remote::RemoteError, capture::*};
use dot_logger_client::{Cancellation, ClientError};
use tail::*;

fn record(sequence: u64) -> CapturedRecord {
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
    }
}

#[test]
fn same_subscription_timeout_is_pending_but_new_subscription_always_breaks_the_line() {
    let mut screen = Screen::new(16).unwrap();
    let first = screen.begin().unwrap();
    screen.record(first, record(1), &[1]).unwrap();
    screen.wait(first).unwrap();
    screen.record(first, record(2), &[2]).unwrap();
    screen.end(first, EndReason::StreamEnded).unwrap();
    let second = screen.begin().unwrap();
    screen.record(second, record(3), &[3]).unwrap();
    let rows = screen.rows();
    assert_eq!(rows[1]["span"], rows[3]["span"]);
    assert_eq!(rows[2]["continuity"], "Unconfirmed");
    assert_eq!(rows[5]["kind"], "subscription_started");
    assert_eq!(rows[5]["connect_from_previous"], false);
    assert!(rows[5]["missing_records"].is_null());
    assert_ne!(rows[3]["segment"], rows[6]["segment"]);
    assert!(screen.record(first, record(4), &[4]).is_err());
}

#[test]
fn overflow_gap_is_subscription_scoped_and_splits_the_plot_without_guessing_capture_loss() {
    let mut screen = Screen::new(16).unwrap();
    let id = screen.begin().unwrap();
    screen.record(id, record(7), &[7]).unwrap();
    screen.gap(id, 2, 4).unwrap();
    screen.record(id, record(8), &[8]).unwrap();
    assert_eq!(screen.rows()[2]["first_transfer"], "2");
    assert_eq!(screen.rows()[2]["last_transfer"], "4");
    assert!(screen.rows()[2]["missing_records"].is_null());
    assert_ne!(screen.rows()[1]["span"], screen.rows()[3]["span"]);
    assert!(screen.gap(id, 0, 4).is_err());
    assert!(screen.gap(id, 5, 4).is_err());
    assert!(screen.begin().is_err());
}

#[test]
fn history_full_keeps_terminal_marker_and_rejects_new_record_without_erasing_history() {
    let mut screen = Screen::new(3).unwrap();
    let id = screen.begin().unwrap();
    screen.record(id, record(1), &[1]).unwrap();
    assert!(screen.record(id, record(2), &[2]).is_err());
    screen
        .end(id, EndReason::Client(ClientError::ConsumerRejected))
        .unwrap();
    assert_eq!(screen.rows().len(), 3);
    assert_eq!(screen.rows()[2]["kind"], "subscription_ended");
    assert_eq!(screen.rows()[2]["reason"], "ConsumerRejected");
    assert!(screen.begin().is_err());
    assert!(Screen::new(usize::MAX).is_err());
}

#[test]
fn actual_loopback_timeout_lease_reconnect_gap_cancel_and_cleanup_have_visible_boundaries() {
    let report = run(&Cancellation::new()).unwrap();
    assert_eq!(report.acknowledged, 3);
    assert_eq!(report.subscriptions, 2);
    assert_eq!(report.external_after_close, 0);
    let rows = report.screen.rows();
    let records: Vec<_> = rows.iter().filter(|r| r["kind"] == "record").collect();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["segment"], records[1]["segment"]);
    assert_eq!(records[0]["receipt"]["accepted_seq"], "1");
    assert_eq!(records[1]["receipt"]["accepted_seq"], "2");
    assert_ne!(records[1]["segment"], records[2]["segment"]);
    assert_eq!(records[2]["receipt"]["accepted_seq"], "99");
    assert_eq!(records[2]["payload"], serde_json::json!([9]));
    let gaps: Vec<_> = rows.iter().filter(|r| r["kind"] == "gap").collect();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0]["first_transfer"], "2");
    assert_eq!(gaps[0]["last_transfer"], "2");
    assert!(rows.iter().any(|r| r["kind"] == "wait_timed_out"));
    let ends: Vec<_> = rows
        .iter()
        .filter(|r| r["kind"] == "subscription_ended")
        .collect();
    assert_eq!(ends.len(), 2);
    assert_eq!(ends[0]["reason"], "StreamEnded");
    assert_eq!(ends[1]["reason"], "Cancelled");
    assert!(
        run(&{
            let c = Cancellation::new();
            c.cancel();
            c
        })
        .is_err()
    );
}

#[test]
fn transport_error_and_ack_unknown_close_display_without_claiming_unreceived_data() {
    for error in [
        ClientError::Disconnected,
        ClientError::Remote(RemoteError::Transport),
        ClientError::AcknowledgementUnknown,
    ] {
        let mut screen = Screen::new(8).unwrap();
        let first = screen.begin().unwrap();
        screen.record(first, record(1), &[1]).unwrap();
        screen.end(first, EndReason::Client(error)).unwrap();
        let second = screen.begin().unwrap();
        assert_ne!(first, second);
        assert_eq!(screen.rows()[2]["connect_from_previous"], false);
        assert!(screen.rows()[2]["missing_records"].is_null());
        assert_eq!(
            screen.rows()[1]["receipt"]["session_id"],
            "0000000000000000000000000000002a"
        );
    }
}

#[test]
fn actual_full_history_and_disconnect_never_ack_and_release_actual_connection_owners() {
    for (mode, reason) in [
        (scenario::Mode::HistoryFull, "ConsumerRejected"),
        (scenario::Mode::Disconnect, "Disconnected"),
    ] {
        let report = scenario::run_case(mode, &Cancellation::new()).unwrap();
        assert_eq!(report.acknowledged, 0);
        assert_eq!(report.subscriptions, 1);
        assert_eq!(report.external_after_close, 0);
        assert_eq!(report.screen.rows().len(), 3);
        assert_eq!(report.screen.rows()[2]["reason"], reason);
        assert!(!report.screen.rows().iter().any(|r| r["kind"] == "record"));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        assert!(run(&Cancellation::new()).is_err());
    });
}

#[test]
fn rendered_screen_shows_terminal_and_fresh_subscription_markers_between_records() {
    let mut screen = Screen::new(10).unwrap();
    let first = screen.begin().unwrap();
    screen.record(first, record(1), &[1]).unwrap();
    screen.end(first, EndReason::StreamEnded).unwrap();
    let second = screen.begin().unwrap();
    screen.record(second, record(2), &[2]).unwrap();
    let mut output = Vec::new();
    screen.render(&mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let first_record = text.find("seq=1").unwrap();
    let end = text.find("구독 종료").unwrap();
    let restart = text.rfind("새 구독").unwrap();
    let second_record = text.find("seq=2").unwrap();
    assert!(first_record < end && end < restart && restart < second_record);
    assert!(text.contains("연속성 미확인"));
    assert!(screen.render(&mut std::io::sink()).is_ok());
}

#[test]
fn large_native_identifiers_and_signed_source_clocks_are_exact_in_screen_json() {
    let mut screen = Screen::new(8).unwrap();
    let id = screen.begin().unwrap();
    let mut original = record(u64::MAX);
    original.metadata.schema_id = u128::MAX;
    original.metadata.mono_ns = u64::MAX;
    original.metadata.real_ns = i64::MIN;
    original.metadata.source_time = Some(SourceTime {
        ticks: i64::MIN,
        clock_domain: u64::MAX,
    });
    screen.record(id, original, &[1]).unwrap();
    let json = &screen.rows()[1];
    assert_eq!(json["schema_id"], "ffffffffffffffffffffffffffffffff");
    assert_eq!(json["receipt"]["accepted_seq"], u64::MAX.to_string());
    assert_eq!(json["mono_ns"], u64::MAX.to_string());
    assert_eq!(json["real_ns"], i64::MIN.to_string());
    assert_eq!(json["source_time"]["ticks"], i64::MIN.to_string());
    assert_eq!(json["source_time"]["clock_domain"], u64::MAX.to_string());
}
