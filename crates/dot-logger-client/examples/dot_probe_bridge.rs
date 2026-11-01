//! Explicit dot-probe tick export -> raw capture -> route -> socket -> queue ACK.
#[path = "support/dot_probe.rs"]
mod bridge;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, mpsc::sync_channel},
};

fn main() -> std::process::ExitCode {
    match execute() {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(3),
        Err(error) => {
            eprintln!("dot-probe bridge: {error}");
            std::process::ExitCode::from(4)
        }
    }
}
fn execute() -> Result<bool, bridge::Error> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || (args[0] != "--plan" && args[0] != "--run") {
        return Err("usage: dot_probe_bridge (--plan|--run) <PROBE_TICK_REPORT path>".into());
    }
    // Preview deliberately does not open/probe/parse the app export or bind IO.
    if args[0] == "--plan" {
        println!(
            "{{\"scope\":\"production_capture_loopback_snapshot\",\"records\":1,\"max_snapshot_bytes\":{},\"duration_ns\":\"2000000000\",\"logger_budget_bytes\":16777216,\"workers\":1,\"receiver_queue_slots\":1,\"boundary\":\"ReceiverAcknowledged\",\"capability\":\"Unprobed\",\"capacity\":\"Unavailable\"}}",
            bridge::MAX_SNAPSHOT_BYTES
        );
        return Ok(true);
    }
    let payload = bridge::read_snapshot(Path::new(&args[1]))?;
    let (queue, consumed) = sync_channel(1);
    let report = bridge::run(
        &payload,
        bridge::Options::default(),
        &AtomicBool::new(false),
        |_, bytes| {
            if queue.try_send(bytes.to_vec()).is_ok() {
                bridge::Action::Accept
            } else {
                bridge::Action::Reject
            }
        },
    )?;
    let identical = consumed.try_recv().is_ok_and(|bytes| bytes == payload);
    println!(
        "{}",
        serde_json::json!({
            "scope": "production_capture_loopback_snapshot", "source": "dot-probe.PROBE_TICK_REPORT.export",
            "source_freshness": "Unavailable", "source_bytes": payload.len(),
            "outcome": format!("{:?}", report.outcome), "accepted": report.accepted,
            "acknowledged": report.acknowledged,
            "receipt": report.receipt.map(|r| serde_json::json!({"session": format!("{:032x}", r.session_id), "producer": r.producer_id, "sequence": r.accepted_seq})),
            "revision": 1, "handoff_error": report.handoff_error.map(|e| format!("{e:?}")),
        "before_handoff": format!("{:?}", report.before_handoff),
            "completion": format!("{:?}", report.completion),
        "raw_held_before_ack": report.raw_held_before_ack, "raw_reclaimed": report.raw_reclaimed,
            "external_bytes_after_close": report.external_bytes_after_close, "payload_identical": identical,
            "network_scope": "LoopbackOnly", "capacity": "Unavailable", "network_measurement": "Unavailable",
            "rss_measurement": "Unavailable", "physical_write_bytes": 0
        })
    );
    Ok(report.outcome == bridge::BridgeOutcome::Pass && identical)
}
