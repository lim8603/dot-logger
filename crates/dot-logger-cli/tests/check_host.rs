//! CLI -> actual gRPC -> registered host -> actual file sync/cleanup.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    adapters::remote::*, capture::*, completion::Boundary, diagnostics::*, query::QueryBudget,
    routing::Router, storage::StorageChannel,
};
use serde_json::Value;
use std::{
    net::SocketAddr,
    path::PathBuf,
    process::{Command, Output},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
struct Host {
    address: SocketAddr,
    id: u64,
    path: PathBuf,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    results: mpsc::Receiver<(CheckOutcome, u32, bool)>,
}
impl Host {
    fn new(records: u32, interval_ns: u64, message_bytes: usize, boundary: Boundary) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "dot-cli-check-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("operating"), b"keep").unwrap();
        let (ready, receive) = mpsc::channel();
        let (stop, stopped) = mpsc::channel();
        let (result, results) = mpsc::channel();
        let host_path = path.clone();
        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut config = support::plan().config().clone();
                config.memory.external_bytes = 128 * 1024 * 1024;
                config.memory.total_bytes += 128 * 1024 * 1024;
                let plan = dot_logger::config::SessionBuilder::new(config)
                    .validate(&dot_logger::platform::PlatformCapabilities::default())
                    .unwrap();
                let budget = QueryBudget::new(&plan);
                let limits = RemoteLimits {
                    connections: 4,
                    subscriptions: 1,
                    commands: 2,
                    message_bytes,
                    record_bytes: 1024,
                    subscription_records: 1,
                    subscription_bytes: 1024,
                    max_age: Duration::from_secs(10),
                    request_timeout: Duration::from_secs(5),
                };
                let (hub, mut inbox) = RemoteHub::new(&budget, limits).unwrap();
                let server = RemoteServer::bind(
                    ServerConfig {
                        enabled: true,
                        tokens: vec![
                            AccessToken::new("test-manage-check-token".into(), Permission::Manage)
                                .unwrap(),
                        ],
                        ..ServerConfig::default()
                    },
                    hub,
                )
                .await
                .unwrap();
                let addr = server.local_addr().unwrap();
                let (shutdown, done) = tokio::sync::oneshot::channel();
                let task = tokio::spawn(server.serve(async {
                    let _ = done.await;
                }));
                let channel = ChannelConfig::new(7, 99, 1);
                let mut owner = CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap();
                owner.scope(|s| {
                    let router =
                        Router::new(&plan, s.map(|(_, r)| r).collect(), support::routing())
                            .unwrap();
                    let mut checks = CheckHost::new(1).unwrap();
                    use dot_logger::telemetry::{
                        CaptureCounts, Window,
                        capacity::{Conditions, Profile, Rate},
                        report::{Report, ReportInput},
                    };
                    let conditions = Conditions {
                        environment_id: 11,
                        workload_id: 22,
                        revision: 1,
                        target_id: 10,
                        boundary,
                    };
                    let profile = Profile {
                        session_id: 4242,
                        id: 99,
                        conditions,
                        valid_from_ns: 0,
                        valid_until_ns: 0,
                        completed_records: Rate {
                            amount: 100,
                            elapsed_ns: 1,
                        },
                    };
                    let window = Window {
                        session_id: 4242,
                        revision: 1,
                        epoch: 3,
                        from_ns: 0,
                        to_ns: 1,
                        counts: CaptureCounts::default(),
                    };
                    let source = CheckSource {
                        report: Report::build(
                            ReportInput::from_window(window, conditions),
                            Some(profile),
                            2,
                        ),
                        profile: Some(profile),
                    };
                    let id = checks
                        .register(
                            CheckPlan::build(CheckSpec {
                                session_id: 42,
                                revision: 1,
                                environment_id: 11,
                                workload_id: 22,
                                directory: host_path.join("check"),
                                budgets: CheckBudgets {
                                    duration_ns: 10_000_000_000,
                                    write_bytes: 1024 * 1024,
                                    memory_bytes: 8 * 1024 * 1024,
                                    disk_reserve_bytes: 0,
                                    workers: 1,
                                },
                                required_memory_bytes: 4 * 1024 * 1024,
                                boundary,
                                network: NetworkPlan::UnavailableNoReceiver,
                            })
                            .unwrap(),
                            router.writer_budget(10).unwrap(),
                            FileCheck {
                                workload: FileWorkload {
                                    records,
                                    payload_bytes: 16,
                                    interval_ns,
                                    seed: 17,
                                },
                                channel: StorageChannel {
                                    channel,
                                    topic: "check".into(),
                                    payload_encoding: "opaque".into(),
                                    schema: None,
                                },
                            },
                            Some(source),
                        )
                        .unwrap();
                    ready.send((addr, id)).unwrap();
                    let cancel = AtomicBool::new(false);
                    let mut runtime =
                        FileCheckRuntime::new(Instant::now(), &cancel, linux_available_bytes);
                    loop {
                        match stopped.try_recv() {
                            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                            Err(mpsc::TryRecvError::Empty) => {}
                        }
                        if let Some(command) = inbox.try_next() {
                            if let Some(e) = checks.handle_command(command, &mut runtime) {
                                result
                                    .send((
                                        e.report.outcome,
                                        e.report.written_records,
                                        e.report.cleaned,
                                    ))
                                    .unwrap();
                            }
                        } else {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                    if message_bytes < 5120 {
                        assert!(
                            checks.preview(id).is_ok(),
                            "small reply must not consume capability"
                        );
                    }
                });
                let _ = shutdown.send(());
                task.await.unwrap().unwrap();
            });
        });
        let (address, id) = receive.recv_timeout(Duration::from_secs(10)).unwrap();
        Self {
            address,
            id,
            path,
            stop: Some(stop),
            thread: Some(thread),
            results,
        }
    }
    fn invoke(&self, action: &str, auth: bool, timeout: u64) -> Output {
        let mut c = Command::new(env!("CARGO_BIN_EXE_dot-logger"));
        c.args([
            "--json",
            "--endpoint",
            &format!("http://{}", self.address),
            "--plan",
            &self.id.to_string(),
            "--timeout-ms",
            &timeout.to_string(),
            "--token-env",
            "TEST_CHECK_TOKEN",
            "check",
            action,
        ]);
        if auth {
            c.env("TEST_CHECK_TOKEN", "test-manage-check-token");
        } else {
            c.env_remove("TEST_CHECK_TOKEN");
        }
        c.output().unwrap()
    }
    fn unchanged(&self) {
        assert_eq!(std::fs::read(self.path.join("operating")).unwrap(), b"keep");
        assert_eq!(std::fs::read_dir(&self.path).unwrap().count(), 1);
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            assert!(thread.join().is_ok());
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
fn rows(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
fn cli_plan_then_explicit_run_uses_real_registered_host_and_manage_permission() {
    let host = Host::new(2, 0, 5120, Boundary::Synced);
    let preview = host.invoke("plan", false, 5000); // Read-only preview is allowed.
    assert_eq!(preview.status.code(), Some(0), "{:?}", rows(&preview));
    assert_eq!(rows(&preview)[0]["data"]["kind"], "plan");
    host.unchanged();
    assert_eq!(host.invoke("run", false, 5000).status.code(), Some(2));
    host.unchanged();
    let run = host.invoke("run", true, 5000);
    assert_eq!(run.status.code(), Some(0), "{:?}", rows(&run));
    let rows = rows(&run);
    assert_eq!(rows[0]["data"]["outcome"], "Pass");
    assert_eq!(rows[0]["data"]["network_measurement"], "Unavailable");
    assert_eq!(rows[0]["data"]["capacity"], "Unavailable");
    assert_eq!(rows[0]["data"]["source"]["epoch"], "3");
    assert_eq!(
        rows[0]["data"]["source"]["session"],
        format!("{:032x}", 4242)
    );
    assert_eq!(rows[0]["data"]["session"], format!("{:032x}", 42));
    assert_eq!(rows[0]["data"]["source"]["profile"]["valid_until_ns"], "0");
    assert_eq!(
        rows[0]["data"]["source"]["profile"]["reused_as_check_capacity"],
        false
    );
    assert!(host.invoke("run", true, 5000).status.code() != Some(0));
    host.unchanged();
}
#[test]
fn remote_deadline_stops_pacing_and_host_settles_before_releasing_command() {
    let host = Host::new(100, 100_000_000, 263168, Boundary::Synced);
    let output = host.invoke("run", true, 500);
    assert_eq!(output.status.code(), Some(3), "{:?}", rows(&output));
    assert_eq!(
        rows(&output).last().unwrap()["data"]["posted_host_io"],
        "completion_unconfirmed"
    );
    let (outcome, written, cleaned) = host.results.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(outcome, CheckOutcome::Aborted);
    assert!(written > 0 && written < 100);
    assert!(cleaned);
    host.unchanged();
    assert!(host.invoke("run", true, 5000).status.code() != Some(0));
}
#[test]
fn insufficient_reply_budget_prevents_io_and_does_not_consume_plan() {
    let host = Host::new(2, 0, 2048, Boundary::Synced);
    let output = host.invoke("run", true, 5000);
    assert_eq!(output.status.code(), Some(3), "{:?}", rows(&output));
    host.unchanged();
}
#[test]
fn unsupported_boundary_is_inconclusive_and_cli_exits_three_without_load() {
    let host = Host::new(2, 0, 263168, Boundary::ReceiverAcknowledged);
    let output = host.invoke("run", true, 5000);
    assert_eq!(output.status.code(), Some(3), "{:?}", rows(&output));
    assert_eq!(rows(&output)[0]["data"]["outcome"], "Inconclusive");
    assert!(
        rows(&output).last().unwrap()["data"]
            .get("posted_host_io")
            .is_none()
    );
    host.unchanged();
}
#[test]
fn existing_namespace_is_preserved_and_file_failure_exits_four() {
    let host = Host::new(2, 0, 263168, Boundary::Synced);
    std::fs::create_dir(host.path.join("check")).unwrap();
    std::fs::write(host.path.join("check/foreign"), b"keep-foreign").unwrap();
    let output = host.invoke("run", true, 5000);
    assert_eq!(output.status.code(), Some(4), "{:?}", rows(&output));
    assert_eq!(rows(&output)[0]["data"]["outcome"], "Fail");
    assert_eq!(
        std::fs::read(host.path.join("check/foreign")).unwrap(),
        b"keep-foreign"
    );
    assert!(
        rows(&output).last().unwrap()["data"]
            .get("posted_host_io")
            .is_none()
    );
}
