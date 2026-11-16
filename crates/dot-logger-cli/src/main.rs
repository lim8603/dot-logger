//! Explicit general-path dot-logger command line interface.
mod args;
mod commands;
mod model;
mod output;
use clap::error::ErrorKind;
use clap::parser::ValueSource;
use commands::Context;
use dot_logger_client::Cancellation;
use output::{Failure, Output};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
/// Time allowed after a calibration's time limit for the final sync and cleanup.
const CALIBRATE_SLACK_MS: u64 = 120_000;
fn run() -> i32 {
    let args: Vec<_> = std::env::args_os().collect();
    let json = args.iter().any(|a| a == "--json");
    let matches = match args::command().try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(error) => {
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                print!("{error}");
                return 0;
            }
            return Output::new(json, "input".into(), 4096).finish(Err(Failure::Input), false);
        }
    };
    let command = matches.subcommand_name().unwrap_or("input").to_owned();
    let timeout = matches
        .get_one::<String>("timeout-ms")
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| (1..=3_600_000).contains(n));
    // `calibrate` runs for its own time limit plus the final sync and cleanup;
    // the general 5 s default would cancel it. An explicit value still wins.
    let default_timeout = matches.value_source("timeout-ms") == Some(ValueSource::DefaultValue);
    let timeout = if command == "calibrate" && default_timeout {
        matches
            .get_one::<String>("time-limit-ms")
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|n| (1..=3_600_000).contains(n))
            .map(|n| n + CALIBRATE_SLACK_MS)
    } else {
        timeout
    };
    let cap = matches
        .get_one::<String>("max-output-bytes")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| (1024..=256 * 1024 * 1024).contains(n));
    let (Some(timeout), Some(cap)) = (timeout, cap) else {
        return Output::new(json, command, 4096).finish(Err(Failure::Input), false);
    };
    if matches.get_one::<String>("endpoint").is_some()
        && matches.get_one::<String>("manifest").is_some()
    {
        return Output::new(json, command, 4096).finish(Err(Failure::Input), false);
    }
    let output = Output::new(json, command, cap);
    let cancel = Cancellation::new();
    let timed_out = Arc::new(AtomicBool::new(false));
    let context = Context {
        args: matches,
        cancel: cancel.clone(),
        started: Instant::now(),
        deadline_ns: timeout * 1_000_000,
    };
    if let Err(error) = context.validate() {
        let mut output = output;
        return output.finish(Err(error), false);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => {
            let mut output = output;
            return output.finish(Err(Failure::Io), false);
        }
    };
    runtime.block_on(async move {
        let signal_cancel = cancel.clone();
        let signal = tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                signal_cancel.cancel();
            }
        });
        let deadline_cancel = cancel.clone();
        let deadline_flag = timed_out.clone();
        let deadline = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(timeout)).await;
            deadline_flag.store(true, Ordering::Release);
            deadline_cancel.cancel();
        });
        let (mut output, result) = if context.optional("endpoint").is_some() {
            let mut output = output;
            let result = commands::remote(&context, &mut output).await;
            (output, result)
        } else {
            let worker = tokio::task::spawn_blocking(move || {
                let mut output = output;
                let result = commands::local(&context, &mut output);
                (output, result)
            });
            match worker.await {
                Ok(result) => result,
                Err(_) => {
                    signal.abort();
                    deadline.abort();
                    return 4;
                }
            }
        };
        signal.abort();
        deadline.abort();
        output.finish(result, timed_out.load(Ordering::Acquire))
    })
}
fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(run() as u8)
}
