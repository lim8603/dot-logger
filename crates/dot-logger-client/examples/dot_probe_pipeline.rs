//! Finite synthetic dot-probe-style inputs through real file/DB/loopback paths.
#[path = "support/pipeline/mod.rs"]
mod pipeline;
use std::sync::atomic::AtomicBool;
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--plan"] {
        println!("{}", pipeline::plan_description());
        return std::process::ExitCode::SUCCESS;
    }
    if args.len() != 2 || args[0] != "--run" {
        eprintln!("usage: dot_probe_pipeline --plan | --run <fresh-exclusive-measurement>");
        return std::process::ExitCode::from(4);
    }
    let run = pipeline::database_from_env(&args[1]).and_then(|db| {
        pipeline::run(
            db,
            pipeline::Options {
                verify_query: true,
                ..Default::default()
            },
            &AtomicBool::new(false),
        )
    });
    match run {
        Ok(report) => {
            println!("{}", report.json());
            if report.is_complete() {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::from(3)
            }
        }
        Err(error) => {
            eprintln!("pipeline example: {error}");
            std::process::ExitCode::from(4)
        }
    }
}
