//! Finite screen transcript for subscription boundaries; no real dot-probe UI.
#[path = "support/tail/mod.rs"]
mod tail;
use dot_logger_client::Cancellation;
use std::io::{self, Write};

fn main() -> std::process::ExitCode {
    match execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dot-probe tail: {error}");
            std::process::ExitCode::from(4)
        }
    }
}
fn execute() -> Result<(), tail::Error> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut out = io::stdout().lock();
    let report = if args.len() == 1 && args[0] == "--plan" {
        serde_json::json!({"scope": "SyntheticScreenExample", "connections": 2,
            "subscriptions": 2, "screen_rows": 32, "record_bytes": 64,
            "scenario_wait_ms": 10000, "cleanup_wait_ms": 2000,
            "network_scope": "LoopbackOnly", "missing_between_subscriptions": null,
            "capacity": "Unavailable", "capability": "Unprobed"})
    } else if args.len() == 1 && args[0] == "--run" {
        let report = tail::run(&Cancellation::new())?;
        report.screen.render(&mut out)?;
        serde_json::json!({"scope": "SyntheticScreenExample", "rows": report.screen.rows(),
            "acknowledged": report.acknowledged, "subscriptions": report.subscriptions,
            "external_after_close": report.external_after_close,
            "network_scope": "LoopbackOnly", "missing_between_subscriptions": null,
            "capacity": "Unavailable"})
    } else {
        return Err("usage: dot_probe_tail (--plan|--run)".into());
    };
    serde_json::to_writer(&mut out, &report)?;
    writeln!(out)?;
    Ok(())
}
