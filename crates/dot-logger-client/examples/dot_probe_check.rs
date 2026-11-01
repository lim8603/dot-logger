//! Explicit finite CheckHost execution against one host-selected app export.
use dot_logger::{completion::Boundary, diagnostics::*};
use dot_logger_client::diagnostics::{SnapshotCheck, bridge};
use std::{
    path::Path,
    sync::atomic::AtomicBool,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
fn main() -> std::process::ExitCode {
    match execute() {
        Ok(code) => std::process::ExitCode::from(code),
        Err(e) => {
            eprintln!("dot-probe check: {e}");
            std::process::ExitCode::from(4)
        }
    }
}
fn execute() -> Result<u8, bridge::Error> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || (args[0] != "--plan" && args[0] != "--run") {
        return Err("usage: dot_probe_check (--plan|--run) <PROBE_TICK_REPORT path>".into());
    }
    if args[0] == "--plan" {
        // No path/probe/clock/runtime work, even when the input does not exist.
        println!(
            "{}",
            serde_json::json!({"schema":"dot_logger.check.v1","kind":"plan",
            "scope":"production_capture_loopback_snapshot","records":1,"max_snapshot_bytes":65536,
            "duration_ns":"2000000000","memory_bytes":"33554432",
            "required_memory_bytes":SnapshotCheck::REQUIRED_MEMORY_BYTES.to_string(),
            "write_bytes":"1","physical_write_bytes":0,"disk_reserve_bytes":"0","workers":1,
            "target":SnapshotCheck::TARGET_ID,"boundary":"ReceiverAcknowledged","network":"LoopbackOnly",
            "capability":"Unprobed","capacity":"Unavailable"})
        );
        return Ok(0);
    }
    let backend = SnapshotCheck::new(bridge::read_snapshot(Path::new(&args[1]))?, true)?;
    let session = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        | (u128::from(std::process::id()) << 96);
    let origin = Instant::now();
    let mut host = CheckHost::new(1)?;
    let id = host.register_routed(
        CheckPlan::build(CheckSpec {
            session_id: session,
            revision: 1,
            environment_id: 1,
            workload_id: 1,
            // This network-only scope never accesses the unused file declaration.
            directory: std::env::temp_dir().join("dot-probe-check-unused"),
            budgets: CheckBudgets {
                duration_ns: 2_000_000_000,
                write_bytes: 1,
                memory_bytes: 32 * 1024 * 1024,
                disk_reserve_bytes: 0,
                workers: 1,
            },
            required_memory_bytes: SnapshotCheck::REQUIRED_MEMORY_BYTES,
            boundary: Boundary::ReceiverAcknowledged,
            network: NetworkPlan::LoopbackOnly,
        })?,
        SnapshotCheck::TARGET_ID,
        Box::new(backend),
        None,
    )?;
    let cancel = AtomicBool::new(false);
    let evidence = host.run_routed(
        id,
        &mut FileCheckRuntime::new(origin, &cancel, linux_available_bytes),
    )?;
    println!("{}", std::str::from_utf8(&evidence.summary(id))?);
    Ok(match evidence.report.outcome {
        CheckOutcome::Pass => 0,
        CheckOutcome::Fail => 4,
        _ => 3,
    })
}
