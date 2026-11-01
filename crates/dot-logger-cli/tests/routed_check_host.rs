//! CLI management permission and single-use routed host over real TCP.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{adapters::remote::*, completion::Boundary, diagnostics::*, query::QueryBudget};
use dot_logger_client::diagnostics::SnapshotCheck;
use std::{
    net::SocketAddr,
    process::{Command, Output},
    sync::{atomic::AtomicBool, mpsc},
    time::{Duration, Instant},
};
const SNAPSHOT: &[u8] = br#"{"threads":[{"name":"ControlThread","interval_us":10000,"ticks":2}]}"#;
struct Host {
    address: SocketAddr,
    id: u64,
    stop: mpsc::Sender<()>,
    join: Option<std::thread::JoinHandle<()>>,
    results: mpsc::Receiver<HostCheckEvidence>,
}
impl Host {
    fn new(receiver: bool, message_bytes: usize) -> Self {
        Self::delayed(receiver, message_bytes, Duration::ZERO)
    }
    fn delayed(receiver: bool, message_bytes: usize, delay: Duration) -> Self {
        let (ready, rx) = mpsc::channel();
        let (stop, stopped) = mpsc::channel();
        let (done, results) = mpsc::channel();
        let join = std::thread::spawn(move || {
            let mut config = support::plan().config().clone();
            config.memory.external_bytes = 128 * 1024 * 1024;
            config.memory.total_bytes += 128 * 1024 * 1024;
            let config = dot_logger::config::SessionBuilder::new(config)
                .validate(&dot_logger::platform::PlatformCapabilities::default())
                .unwrap();
            let budget = QueryBudget::new(&config);
            let (hub, mut inbox) = RemoteHub::new(
                &budget,
                RemoteLimits {
                    connections: 2,
                    subscriptions: 1,
                    commands: 1,
                    message_bytes,
                    record_bytes: 1024,
                    subscription_records: 1,
                    subscription_bytes: 1024,
                    max_age: Duration::from_secs(3),
                    request_timeout: Duration::from_secs(3),
                },
            )
            .unwrap();
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            let server = runtime
                .block_on(RemoteServer::bind(
                    ServerConfig {
                        enabled: true,
                        tokens: vec![
                            AccessToken::new("test-routed-check-token".into(), Permission::Manage)
                                .unwrap(),
                        ],
                        ..Default::default()
                    },
                    hub.clone(),
                ))
                .unwrap();
            let address = server.local_addr().unwrap();
            let (shutdown, ended) = tokio::sync::oneshot::channel();
            let server = runtime.spawn(server.serve(async {
                let _ = ended.await;
            }));
            let mut host = CheckHost::new(1).unwrap();
            let id = host
                .register_routed(
                    CheckPlan::build(CheckSpec {
                        session_id: 941,
                        revision: 1,
                        environment_id: 11,
                        workload_id: 22,
                        directory: "/nonexistent/routed-cli-never-created".into(),
                        budgets: CheckBudgets {
                            duration_ns: 2_000_000_000,
                            write_bytes: 1,
                            memory_bytes: 32 * 1024 * 1024,
                            disk_reserve_bytes: 0,
                            workers: 1,
                        },
                        required_memory_bytes: 32 * 1024 * 1024,
                        boundary: Boundary::ReceiverAcknowledged,
                        network: NetworkPlan::LoopbackOnly,
                    })
                    .unwrap(),
                    10,
                    Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), receiver).unwrap()),
                    None,
                )
                .unwrap();
            ready.send((address, id)).unwrap();
            let cancel = AtomicBool::new(false);
            let mut clock = FileCheckRuntime::new(Instant::now(), &cancel, |_| {
                panic!("routed check must not probe disk")
            });
            while stopped.try_recv().is_err() {
                if let Some(command) = inbox.try_next() {
                    if matches!(command.request(), ManagementRequest::RunCheck { .. }) {
                        std::thread::sleep(delay);
                    }
                    if let Some(evidence) = host.handle_command(command, &mut clock) {
                        done.send(evidence).unwrap();
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            hub.close();
            let _ = shutdown.send(());
            runtime.block_on(server).unwrap().unwrap();
        });
        let (address, id) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        Self {
            address,
            id,
            stop,
            join: Some(join),
            results,
        }
    }
    fn invoke(&self, action: &str, auth: bool) -> Output {
        self.invoke_timeout(action, auth, 5000)
    }
    fn invoke_timeout(&self, action: &str, auth: bool, timeout: u64) -> Output {
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
            "ROUTED_CHECK_TOKEN",
            "check",
            action,
        ]);
        if auth {
            c.env("ROUTED_CHECK_TOKEN", "test-routed-check-token");
        } else {
            c.env_remove("ROUTED_CHECK_TOKEN");
        }
        c.output().unwrap()
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        assert!(self.join.take().unwrap().join().is_ok());
    }
}
fn rows(o: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
fn cli_preview_is_read_only_and_manage_runs_actual_routing_exactly_once() {
    let host = Host::new(true, 8192);
    let preview = host.invoke("plan", false);
    assert_eq!(preview.status.code(), Some(0), "{:?}", rows(&preview));
    assert_eq!(
        rows(&preview)[0]["data"]["scope"],
        "production_capture_loopback_snapshot"
    );
    assert!(host.results.try_recv().is_err());
    assert_eq!(host.invoke("run", false).status.code(), Some(2));
    let run = host.invoke("run", true);
    assert_eq!(run.status.code(), Some(0), "{:?}", rows(&run));
    assert_eq!(rows(&run)[0]["data"]["acknowledged"], 1);
    assert_eq!(rows(&run)[0]["data"]["capacity"], "Unavailable");
    assert_eq!(rows(&run)[0]["data"]["physical_write_bytes"], 0);
    let HostCheckEvidence::Routed(e) = host.results.recv_timeout(Duration::from_secs(3)).unwrap()
    else {
        panic!("wrong backend")
    };
    assert_eq!(e.report.receipt.unwrap().session_id, 941);
    assert_eq!(
        (e.report.raw_reclaimed, e.report.external_bytes_after_close),
        (1, 0)
    );
    assert_ne!(host.invoke("run", true).status.code(), Some(0));
}
#[test]
fn missing_receiver_is_inconclusive_and_insufficient_response_budget_never_runs() {
    let host = Host::new(false, 8192);
    let run = host.invoke("run", true);
    assert_eq!(run.status.code(), Some(3), "{:?}", rows(&run));
    assert_eq!(rows(&run)[0]["data"]["accepted"], 0);
    let host = Host::new(true, 2048);
    assert_eq!(host.invoke("run", true).status.code(), Some(3));
    assert!(host.results.try_recv().is_err());
}

#[test]
fn timed_out_remote_waiter_keeps_native_evidence_and_stops_capture() {
    let host = Host::delayed(true, 8192, Duration::from_millis(800));
    let run = host.invoke_timeout("run", true, 500);
    assert_eq!(run.status.code(), Some(3), "{:?}", rows(&run));
    let HostCheckEvidence::Routed(e) = host.results.recv_timeout(Duration::from_secs(3)).unwrap()
    else {
        panic!("wrong backend")
    };
    assert_eq!(e.report.outcome, CheckOutcome::Aborted);
    assert_eq!(e.report.accepted, 0);
    assert_eq!(e.report.external_bytes_after_close, 0);
    assert_ne!(host.invoke("run", true).status.code(), Some(0));
}
