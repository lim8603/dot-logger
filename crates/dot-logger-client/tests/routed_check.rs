//! Production capture/routing through a real socket and receiving queue.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used)]
#[path = "../examples/support/dot_probe.rs"]
mod bridge;
use bridge::{Action, BridgeOutcome, Options};
use dot_logger::completion::QueryOutcome;
use std::{
    sync::{atomic::AtomicBool, mpsc::sync_channel},
    time::Duration,
};

const SNAPSHOT: &[u8] = br#"{"threads":[{"name":"ControlThread","interval_us":10000,"ticks":2}]}"#;

#[test]
fn actual_queue_handoff_confirms_the_original_capture_receipt_and_returns_the_raw_slot() {
    let (tx, rx) = sync_channel(1);
    let report = bridge::run(
        SNAPSHOT,
        Options::default(),
        &AtomicBool::new(false),
        |record, bytes| {
            assert_eq!(record.receipt.producer_id, 0);
            assert_eq!(record.receipt.accepted_seq, 1);
            assert_eq!(bytes, SNAPSHOT);
            if tx.try_send(bytes.to_vec()).is_ok() {
                Action::Accept
            } else {
                Action::Reject
            }
        },
    )
    .unwrap();
    assert_eq!(rx.try_recv().unwrap(), SNAPSHOT);
    assert_eq!(report.outcome, BridgeOutcome::Pass);
    assert_eq!(report.before_handoff, Some(QueryOutcome::Pending));
    assert_eq!(report.completion, Some(QueryOutcome::Complete));
    assert_eq!(
        (report.accepted, report.acknowledged, report.raw_reclaimed),
        (1, 1, 1)
    );
    assert!(report.raw_held_before_ack);
    assert_eq!(report.external_bytes_after_close, 0);
}

#[test]
fn a_full_receiver_queue_and_disconnect_never_confirm_delivery() {
    for disconnect in [false, true] {
        let (tx, _rx) = sync_channel(1);
        tx.try_send(vec![0]).unwrap();
        let report = bridge::run(
            SNAPSHOT,
            Options::default(),
            &AtomicBool::new(false),
            |_, bytes| {
                if disconnect {
                    Action::Disconnect
                } else if tx.try_send(bytes.to_vec()).is_err() {
                    Action::Reject
                } else {
                    panic!("queue must be full")
                }
            },
        )
        .unwrap();
        assert_eq!(report.outcome, BridgeOutcome::Inconclusive);
        assert_eq!(report.before_handoff, Some(QueryOutcome::Pending));
        assert_eq!(report.completion, Some(QueryOutcome::Unknown));
        assert_eq!(
            (report.accepted, report.acknowledged, report.raw_reclaimed),
            (1, 0, 1)
        );
        assert_eq!(report.external_bytes_after_close, 0);
    }
}

#[test]
fn no_receiver_cancel_and_budget_expiry_do_not_create_a_capture_receipt() {
    for case in 0..3 {
        let options = Options {
            receiver: case != 0,
            duration: if case == 2 {
                Duration::ZERO
            } else {
                Duration::from_secs(2)
            },
        };
        let report = bridge::run(SNAPSHOT, options, &AtomicBool::new(case == 1), |_, _| {
            panic!("no handoff")
        })
        .unwrap();
        assert_eq!(report.accepted, 0);
        assert_eq!(report.acknowledged, 0);
        assert_eq!(report.completion, None);
        assert_eq!(report.external_bytes_after_close, 0);
        assert_eq!(
            report.outcome,
            if case == 1 {
                BridgeOutcome::Aborted
            } else {
                BridgeOutcome::Inconclusive
            }
        );
    }
}

#[test]
fn source_is_bounded_and_invalid_or_empty_app_exports_are_rejected() {
    assert!(bridge::validate_snapshot(SNAPSHOT).is_ok());
    for invalid in [
        &b"{}"[..],
        &b"{\"threads\":[]}"[..],
        &b"{\"threads\":[{}]}"[..],
        &vec![b' '; bridge::MAX_SNAPSHOT_BYTES + 1][..],
    ] {
        assert!(bridge::validate_snapshot(invalid).is_err());
    }
}

#[test]
fn regular_file_import_preserves_the_actual_export_and_rejects_oversized_input() {
    let root = std::env::temp_dir().join(format!("dot-bridge-import-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let file = root.join("ticks.json");
    std::fs::write(&file, SNAPSHOT).unwrap();
    assert_eq!(bridge::read_snapshot(&file).unwrap(), SNAPSHOT);
    assert_eq!(std::fs::read(&file).unwrap(), SNAPSHOT);
    assert!(bridge::read_snapshot(&root).is_err());
    std::fs::write(&file, vec![b' '; bridge::MAX_SNAPSHOT_BYTES + 1]).unwrap();
    assert!(bridge::read_snapshot(&file).is_err());
    std::fs::remove_file(file).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn a_snapshot_larger_than_one_transport_chunk_keeps_its_original_bytes() {
    let mut value: serde_json::Value = serde_json::from_slice(SNAPSHOT).unwrap();
    value["padding"] = serde_json::Value::String("x".repeat(60_000));
    let bytes = serde_json::to_vec(&value).unwrap();
    let (tx, rx) = sync_channel(1);
    let report = bridge::run(
        &bytes,
        Options::default(),
        &AtomicBool::new(false),
        |_, received| {
            assert_eq!(received, bytes);
            tx.try_send(received.to_vec()).unwrap();
            Action::Accept
        },
    )
    .unwrap();
    assert_eq!(rx.try_recv().unwrap(), bytes);
    assert_eq!(report.outcome, BridgeOutcome::Pass);
    assert!(report.raw_held_before_ack);
    assert_eq!(
        (
            report.acknowledged,
            report.raw_reclaimed,
            report.external_bytes_after_close
        ),
        (1, 1, 0)
    );
}

#[test]
fn queue_acceptance_followed_by_connection_loss_preserves_data_without_claiming_ack() {
    let (tx, rx) = sync_channel(1);
    let report = bridge::run(
        SNAPSHOT,
        Options::default(),
        &AtomicBool::new(false),
        |_, bytes| {
            tx.try_send(bytes.to_vec()).unwrap();
            Action::AcceptThenDisconnect
        },
    )
    .unwrap();
    assert_eq!(rx.try_recv().unwrap(), SNAPSHOT);
    assert_eq!(
        report.handoff_error,
        Some(dot_logger_client::ClientError::AcknowledgementUnknown)
    );
    assert_eq!(report.completion, Some(QueryOutcome::Unknown));
    assert_eq!(report.outcome, BridgeOutcome::Inconclusive);
    assert_eq!(
        (
            report.acknowledged,
            report.raw_reclaimed,
            report.external_bytes_after_close
        ),
        (0, 1, 0)
    );
}
