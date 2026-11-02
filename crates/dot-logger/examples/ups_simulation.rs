//! Finite real-file UPS alarm simulation; no hardware UPS is required.
#[path = "support/ups/mod.rs"]
mod ups;
use std::{
    io::{self, Write},
    time::Duration,
};
fn main() -> std::process::ExitCode {
    match execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("UPS simulation: {error}");
            std::process::ExitCode::from(4)
        }
    }
}
fn execute() -> Result<(), ups::Error> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--crash-child" {
        return ups::crash_child((&args[1]).into());
    }
    let mut out = io::stdout().lock();
    if args.len() == 1 && args[0] == "--plan" {
        writeln!(
            out,
            "{}",
            serde_json::json!({"scope":"UpsSimulation","hardware_required":false,"poweroff_ms":3000,"safety_margin_ms":200,"delays_ms":[0,50,250,1000],"protected_mib":[8,64],"rank_payload_mib_64":[48,16],"post_alarm_snapshots":0,"capability":"Unprobed"})
        )?;
        return Ok(());
    }
    if args.len() != 2 || args[0] != "--run" {
        return Err("usage: ups_simulation --plan | --run <trusted-parent>".into());
    }
    let parent = std::path::Path::new(&args[1]);
    for mib in [8, 64] {
        for delay in [0, 50, 250, 1000] {
            let r = ups::run_at(
                ups::Mode::Normal,
                mib * 1024 * 1024,
                Duration::from_millis(delay),
                parent,
            )?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"scope":"UpsSimulation","payload_mib":mib,"delay_ms":delay,"actual_delivery_ns":r.actual_delivery_ns.to_string(),"protection_ns":r.protection_ns.to_string(),"protected_complete":r.protected_complete,"partial":r.partial,"offered":r.offered_records,"recovered":r.recovered_records,"sync_order":r.sync_order,"storage_after_close":r.storage_after_close,"physical_power_loss_observed":false})
            )?;
        }
    }
    let recovered = ups::process_crash(&std::env::current_exe()?, parent)?;
    writeln!(
        out,
        "{}",
        serde_json::json!({"scope":"UpsSimulationProcessInterruption","child_exit":17,"external_oracle_before_child":true,"recovered":recovered,"torn_tail_excluded":true,"past_completion_restored":false,"physical_power_loss_observed":false})
    )?;
    Ok(())
}
