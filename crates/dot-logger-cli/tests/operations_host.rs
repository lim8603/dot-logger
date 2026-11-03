//! CLI -> actual gRPC -> OperationsHost -> live ConfigUpdates/Telemetry.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    adapters::remote::*,
    capture::*,
    completion::Boundary,
    query::QueryBudget,
    telemetry::{
        Observation,
        capacity::Conditions,
        compare::Snapshot,
        operations::OperationsHost,
        report::{Report, ReportInput},
    },
};
use serde_json::Value;
use std::{
    net::SocketAddr,
    path::PathBuf,
    process::{Command, Output},
    sync::mpsc,
    thread::JoinHandle,
    time::Duration,
};
const SECOND: u64 = 1_000_000_000;
const TOKEN: &str = "test-manage-operations-token";
struct Host {
    address: SocketAddr,
    dir: PathBuf,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}
impl Host {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("dot-cli-ops-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (ready, receive) = mpsc::channel();
        let (stop, stopped) = mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut config = support::plan().config().clone();
                config.memory.external_bytes = 64 * 1024 * 1024;
                config.memory.total_bytes += 64 * 1024 * 1024;
                let plan = dot_logger::config::SessionBuilder::new(config)
                    .validate(&dot_logger::platform::PlatformCapabilities::default())
                    .unwrap();
                let budget = QueryBudget::new(&plan);
                let limits = RemoteLimits {
                    connections: 4,
                    subscriptions: 1,
                    commands: 2,
                    message_bytes: 64 * 1024,
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
                        tokens: vec![AccessToken::new(TOKEN.into(), Permission::Manage).unwrap()],
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
                let mut owner = CaptureOwner::new_tracked(
                    &plan,
                    42,
                    1,
                    &[ChannelConfig::new(7, 99, 1)],
                    &support::routing(),
                    support::tracking(8),
                )
                .unwrap();
                owner.enable_config_updates(&support::routing()).unwrap();
                owner.enable_telemetry().unwrap();
                owner.scope(|mut s| {
                    let telemetry = s.telemetry().unwrap();
                    let updates = s.config_updates();
                    let mut ops = OperationsHost::new(4).unwrap();
                    let (mut p, _) = s.next().unwrap();
                    // Four records in the first stable window, two in the second.
                    for at in 0..=20 {
                        if (1..=4).contains(&at) || at == 12 || at == 16 {
                            assert!(matches!(
                                p.try_record_small(support::meta(), &[1]).unwrap(),
                                RecordOutcome::Accepted(_)
                            ));
                        }
                        telemetry.sample(at * SECOND, None).unwrap();
                        if at % 10 == 0 && at > 0 {
                            let Observation::Observed(window) = telemetry.window(10) else {
                                panic!("stable window")
                            };
                            ops.register_snapshot(snapshot(window)).unwrap();
                        }
                    }
                    ready.send(addr).unwrap();
                    loop {
                        match stopped.try_recv() {
                            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                            Err(mpsc::TryRecvError::Empty) => {}
                        }
                        if let Some(command) = inbox.try_next() {
                            ops.handle_command(command, telemetry, updates, 20 * SECOND);
                        } else {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                });
                let _ = shutdown.send(());
                task.await.unwrap().unwrap();
            });
        });
        let address = receive.recv_timeout(Duration::from_secs(10)).unwrap();
        Self {
            address,
            dir,
            stop: Some(stop),
            thread: Some(thread),
        }
    }
    fn invoke(&self, args: &[&str]) -> Output {
        let mut c = Command::new(env!("CARGO_BIN_EXE_dot-logger"));
        c.args([
            "--json",
            "--endpoint",
            &format!("http://{}", self.address),
            "--token-env",
            "TEST_OPERATIONS_TOKEN",
        ])
        .args(args)
        .env("TEST_OPERATIONS_TOKEN", TOKEN);
        c.output().unwrap()
    }
    fn apply(&self, base: &str, name: &str, patch: &str) -> Output {
        let path = self.dir.join(name);
        std::fs::write(&path, patch).unwrap();
        self.invoke(&[
            "--base-revision",
            base,
            "--patch",
            path.to_str().unwrap(),
            "config",
            "apply",
        ])
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
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
fn snapshot(window: dot_logger::telemetry::Window) -> Snapshot {
    let input = ReportInput::from_window(
        window,
        Conditions {
            environment_id: 1,
            workload_id: 1,
            revision: window.revision,
            target_id: 10,
            boundary: Boundary::Written,
        },
    );
    Snapshot::new(Report::build(input, None, window.to_ns), 1)
}
fn rows(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
const WARN: &str = r#"{"schema":"dot_logger.config_patch.v1","channels":[{"channel":7,"minimum_level":"warn","sample_every":2}]}"#;
const TRACE: &str = r#"{"schema":"dot_logger.config_patch.v1","channels":[{"channel":7,"minimum_level":"trace","sample_every":1}]}"#;

#[test]
fn cli_status_apply_and_compare_use_live_host_state() {
    let host = Host::new();
    let status = host.invoke(&["status"]);
    assert_eq!(status.status.code(), Some(0), "{:?}", rows(&status));
    let metrics = &rows(&status)[0]["data"]["metrics"];
    assert_eq!(metrics["schema"], "dot_logger.status.v1");
    assert_eq!(metrics["posted_revision"], "1");
    assert_eq!(metrics["latest"]["state"], "Observed");
    assert_eq!(metrics["latest"]["capture"]["accepted"], "6");
    assert_eq!(metrics["windows"][0]["state"], "Observed");
    assert_eq!(metrics["windows"][0]["counts"]["accepted"], "2");
    assert_eq!(metrics["windows"][2]["state"], "Unavailable");
    assert_eq!(metrics["snapshots"].as_array().unwrap().len(), 2);

    let conflict = host.apply("0", "warn.json", WARN);
    assert_eq!(conflict.status.code(), Some(3), "{:?}", rows(&conflict));
    let last = rows(&conflict).pop().unwrap();
    assert_eq!(last["data"]["outcome"], "revision_conflict");
    assert!(last["data"].get("posted_host_io").is_none());

    let applied = host.apply("1", "warn.json", WARN);
    assert_eq!(applied.status.code(), Some(0), "{:?}", rows(&applied));
    assert_eq!(rows(&applied)[0]["data"]["revision"], "2");

    // The producer has not polled revision 2, so the old generation is retained.
    let busy = host.apply("2", "trace.json", TRACE);
    assert_eq!(busy.status.code(), Some(3), "{:?}", rows(&busy));
    assert_eq!(
        rows(&busy).pop().unwrap()["data"]["outcome"],
        "revision_busy"
    );

    let restart = host.apply(
        "2",
        "memory.json",
        r#"{"schema":"dot_logger.config_patch.v1","memory":{"total_bytes":1}}"#,
    );
    assert_eq!(restart.status.code(), Some(2), "{:?}", rows(&restart));
    assert_eq!(
        rows(&restart).pop().unwrap()["data"]["outcome"],
        "restart_required"
    );
    let invalid = host.apply("2", "bad.json", r#"{"schema":"other"}"#);
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(
        rows(&invalid).pop().unwrap()["data"]["outcome"],
        "invalid_input"
    );

    let compare = host.invoke(&["--before", "1", "--after", "2", "compare"]);
    assert_eq!(compare.status.code(), Some(0), "{:?}", rows(&compare));
    let row = &rows(&compare)[0];
    assert_eq!(row["kind"], "compare");
    assert_eq!(row["data"]["schema"], "dot_logger.compare.v1");
    assert_eq!(row["data"]["comparable"], true);
    assert_eq!(
        row["data"]["changes"]["accepted_rate"]["direction"],
        "Decrease"
    );
    assert_eq!(row["data"]["changes"]["cpu_rate"]["state"], "Unavailable");
    let gone = host.invoke(&["--before", "1", "--after", "99", "compare"]);
    assert_eq!(gone.status.code(), Some(3));
    assert_eq!(
        rows(&gone).pop().unwrap()["data"]["outcome"],
        "gone_or_expired"
    );

    let status = host.invoke(&["status"]);
    let data = &rows(&status)[0]["data"];
    assert_eq!(data["revision"], "2");
    assert_eq!(data["metrics"]["posted_revision"], "2");
}
