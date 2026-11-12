//! Versioned output, offline real files, cancellation and stable exits.
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{capture::*, config::*, platform::*, routing::Router, storage::*};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
struct Fixture {
    dir: std::path::PathBuf,
    manifest: std::path::PathBuf,
    source: std::path::PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
fn fixture(times: [u64; 2]) -> Fixture {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "dot-logger-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let source = dir.join("source.mcap");
    let manifest = dir.join("session.json");
    let mut config = support::plan().config().clone();
    config.memory.output_bytes = 8 * 1024 * 1024;
    config.memory.total_bytes += 8 * 1024 * 1024;
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let channel = ChannelConfig::new(7, 99, 1);
    let channels = vec![StorageChannel {
        channel,
        topic: "raw".into(),
        payload_encoding: "opaque".into(),
        schema: None,
    }];
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap();
    owner.scope(|s| {
        let router = Router::new(&plan, s.map(|(_, r)| r).collect(), support::routing()).unwrap();
        let mut writer = SegmentWriter::new(
            router.writer_budget(10).unwrap(),
            WriterConfig {
                max_payload_bytes: 8192,
                write_buffer_bytes: 0,
                chunk_target_bytes: 16384,
                max_records: 32,
                max_chunks: 16,
                max_metadata_bytes: 1024,
            },
            channels,
            std::fs::File::create(&source).unwrap(),
        )
        .unwrap();
        for (i, mono) in times.into_iter().enumerate() {
            let mut metadata = support::meta();
            metadata.mono_ns = mono;
            metadata.importance = Some(Importance::Normal);
            writer
                .append(
                    CapturedRecord {
                        receipt: CaptureReceipt {
                            session_id: 42,
                            producer_id: 0,
                            accepted_seq: i as u64 + 1,
                        },
                        metadata,
                        revision: 1,
                        payload_len: 3,
                    },
                    &[1, 2, 3],
                )
                .unwrap();
        }
        drop(writer.finish().unwrap());
    });
    let data = json!({"schema_version":1,"session":"0000000000000000000000000000002a","revision":1,"producers":2,"external_bytes":8388608,
        "channels":[{"id":7,"schema":"00000000000000000000000000000063","version":1,"topic":"raw","encoding":"opaque"}],
        "sources":[{"path":source,"sequence":1}],"limits":{"max_segments":4,"max_records":32,"max_record_bytes":16384,"max_source_bytes":1000000,"max_payload_bytes":8192,"max_metadata_bytes":1024,"page_records":2,"page_bytes":1024,"inline_bytes":16,"blob_read_bytes":32,"read_bytes":1000000}});
    std::fs::write(&manifest, serde_json::to_vec(&data).unwrap()).unwrap();
    Fixture {
        dir,
        manifest,
        source,
    }
}
fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dot-logger"));
    command.env_remove("DOT_LOGGER_TOKEN");
    command
}
fn lines(output: &[u8]) -> Vec<Value> {
    std::str::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
#[test]
fn json_input_errors_are_exit_two_without_echoing_secret_arguments() {
    let output = command()
        .args(["--json", "--not-an-option", "test-only-secret-credential"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("test-only-secret-credential"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("test-only-secret-credential"));
    let rows = lines(&output.stdout);
    assert_eq!(rows[0]["schema_version"], 1);
    assert_eq!(rows[0]["kind"], "error");
}
#[test]
fn real_offline_query_replay_and_validation_share_versioned_output() {
    let f = fixture([10, 20]);
    let before = std::fs::read(&f.source).unwrap();
    for action in ["query", "replay"] {
        let output = command()
            .args([action, "--json", "--manifest"])
            .arg(&f.manifest)
            .args(["--from", "0", "--to", "100"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let rows = lines(&output.stdout);
        assert!(rows.iter().all(|row| row["schema_version"] == 1));
        assert_eq!(rows.iter().filter(|row| row["kind"] == "record").count(), 2);
        assert_eq!(rows.last().unwrap()["kind"], "summary");
    }
    let output = command()
        .args(["config", "validate", "--json", "--manifest"])
        .arg(&f.manifest)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(std::fs::read(&f.source).unwrap(), before);
}
#[cfg(unix)]
#[test]
fn ctrl_c_replay_reports_partial_and_closes_readers() {
    let f = fixture([0, 60_000_000_000]);
    let mut child = command()
        .args(["replay", "--json", "--manifest"])
        .arg(&f.manifest)
        .args(["--timeout-ms", "10000"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut first = String::new();
        reader.read_line(&mut first).unwrap();
        tx.send(first.clone()).unwrap();
        reader.read_to_string(&mut first).unwrap();
        first
    });
    if rx.recv_timeout(Duration::from_secs(3)).is_err() {
        let _ = child.kill();
        panic!("first replay output timed out");
    }
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("cancelled replay did not finish");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(status.code(), Some(3));
    let text = reader.join().unwrap();
    let rows = lines(text.as_bytes());
    assert_eq!(rows.iter().filter(|row| row["kind"] == "record").count(), 1);
    assert_eq!(rows.last().unwrap()["kind"], "summary");
    assert_eq!(rows.last().unwrap()["data"]["outcome"], "cancelled");
}

#[test]
fn invalid_replay_rate_is_input_error_before_any_connect() {
    let output = command()
        .args([
            "replay",
            "--json",
            "--endpoint",
            "http://127.0.0.1:1",
            "--speed",
            "0/1",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn tail_external_budget_is_validated_before_connect() {
    let output = command()
        .args([
            "tail",
            "--json",
            "--endpoint",
            "http://127.0.0.1:1",
            "--external-bytes",
            "3221225472",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        lines(&output.stdout).last().unwrap()["data"]["outcome"],
        "invalid_input"
    );
}

#[test]
fn offline_replay_exact_limit_is_complete_and_extra_records_are_partial() {
    for (limit, exit, records) in [(1, 3, 1), (2, 0, 2), (3, 0, 2)] {
        let f = fixture([10, 20]);
        let output = command()
            .args(["replay", "--json", "--manifest"])
            .arg(&f.manifest)
            .args(["--limit", &limit.to_string()])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(
            lines(&output.stdout)
                .iter()
                .filter(|r| r["kind"] == "record")
                .count(),
            records
        );
    }
}

#[test]
fn dropped_mutating_host_command_marks_completion_unconfirmed() {
    use dot_logger::{adapters::remote::*, query::QueryBudget};
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut c = support::plan().config().clone();
        c.memory.external_bytes = 8 * 1024 * 1024;
        c.memory.total_bytes += 8 * 1024 * 1024;
        let plan = SessionBuilder::new(c)
            .validate(&PlatformCapabilities::default())
            .unwrap();
        let budget = QueryBudget::new(&plan);
        let (hub, mut inbox) = RemoteHub::new(
            &budget,
            RemoteLimits {
                connections: 2,
                subscriptions: 2,
                commands: 2,
                message_bytes: 4096,
                record_bytes: 8192,
                subscription_records: 2,
                subscription_bytes: 16384,
                max_age: Duration::from_secs(2),
                request_timeout: Duration::from_secs(2),
            },
        )
        .unwrap();
        let server = RemoteServer::bind(
            ServerConfig {
                enabled: true,
                tokens: vec![
                    AccessToken::new("test-only-manage-credential".into(), Permission::Manage)
                        .unwrap(),
                ],
                ..Default::default()
            },
            hub.clone(),
        )
        .await
        .unwrap();
        let endpoint = format!("http://{}", server.local_addr().unwrap());
        let (stop, done) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(server.serve(async {
            let _ = done.await;
        }));
        let host = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    if let Some(command) = inbox.try_next() {
                        drop(command);
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        });
        let output = command()
            .args([
                "check",
                "run",
                "--plan",
                "1",
                "--json",
                "--endpoint",
                &endpoint,
            ])
            .env("DOT_LOGGER_TOKEN", "test-only-manage-credential")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let rows = lines(&output.stdout);
        let data = &rows.last().unwrap()["data"];
        assert_eq!(data["outcome"], "gone_or_expired");
        assert_eq!(data["posted_host_io"], "completion_unconfirmed");
        host.await.unwrap();
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn duplicate_offline_channel_is_rejected_by_config_validation() {
    let f = fixture([10, 20]);
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    let duplicate = manifest["channels"][0].clone();
    manifest["channels"].as_array_mut().unwrap().push(duplicate);
    std::fs::write(&f.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = command()
        .args(["config", "validate", "--json", "--manifest"])
        .arg(&f.manifest)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
#[test]
fn output_cap_is_partial_and_recovery_never_overwrites_source() {
    let f = fixture([10, 20]);
    let original = std::fs::read(&f.source).unwrap();
    let output = command()
        .args(["query", "--json", "--manifest"])
        .arg(&f.manifest)
        .args(["--max-output-bytes", "1024"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.len() <= 1024);
    assert_eq!(
        lines(&output.stdout).last().unwrap()["data"]["outcome"],
        "limit"
    );
    let output = command()
        .args(["recover", "--json", "--manifest"])
        .arg(&f.manifest)
        .arg("--output")
        .arg(&f.source)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(std::fs::read(&f.source).unwrap(), original);
    let recovered = f.dir.join("recovered.mcap");
    let output = command()
        .args(["recover", "--json", "--manifest"])
        .arg(&f.manifest)
        .arg("--output")
        .arg(&recovered)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    manifest["sources"][0]["path"] = json!(recovered);
    let recovered_manifest = f.dir.join("recovered.json");
    std::fs::write(&recovered_manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = command()
        .args(["query", "--json", "--manifest"])
        .arg(&recovered_manifest)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        lines(&output.stdout)
            .iter()
            .filter(|v| v["kind"] == "record")
            .count(),
        2
    );
    assert_eq!(std::fs::read(&f.source).unwrap(), original);
}

#[test]
fn actual_cli_remote_query_blob_and_replay_control_use_real_file_primitives() {
    use dot_logger::{adapters::remote::*, query::*, replay::*};
    use dot_logger_client::{Cancellation, OfflineConfig, OfflineSource, open_snapshot};
    use std::fs::File;
    use std::sync::{Arc, atomic::AtomicBool};
    let f = fixture([10, 20]);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut c = support::plan().config().clone();
        c.memory.external_bytes = 8 * 1024 * 1024;
        c.memory.total_bytes += 8 * 1024 * 1024;
        let plan = SessionBuilder::new(c)
            .validate(&PlatformCapabilities::default())
            .unwrap();
        let budget = QueryBudget::new(&plan);
        let limits = RemoteLimits {
            connections: 8,
            subscriptions: 2,
            commands: 4,
            message_bytes: 4096,
            record_bytes: 8192,
            subscription_records: 2,
            subscription_bytes: 16384,
            max_age: Duration::from_secs(10),
            request_timeout: Duration::from_secs(2),
        };
        let (hub, mut inbox) = RemoteHub::new(&budget, limits).unwrap();
        let server = RemoteServer::bind(
            ServerConfig {
                enabled: true,
                tokens: vec![
                    AccessToken::new("test-only-manage-credential".into(), Permission::Manage)
                        .unwrap(),
                ],
                ..Default::default()
            },
            hub.clone(),
        )
        .await
        .unwrap();
        let address = server.local_addr().unwrap();
        let endpoint = format!("http://{address}");
        let (stop, done) = tokio::sync::oneshot::channel();
        let server_task = tokio::spawn(server.serve(async {
            let _ = done.await;
        }));
        let native = QueryConfig {
            max_segments: 1,
            max_records: 32,
            max_record_bytes: 16384,
            max_source_bytes: 1000000,
            max_payload_bytes: 8192,
            max_metadata_bytes: 1024,
            producers: 2,
            page_records: 2,
            page_bytes: 1024,
            inline_bytes: 0,
            blob_read_bytes: 32,
            read_bytes: 1000000,
        };
        let snapshot = open_snapshot(
            &budget,
            OfflineConfig {
                session: 42,
                revision: 1,
                channels: vec![StorageChannel {
                    channel: ChannelConfig::new(7, 99, 1),
                    topic: "raw".into(),
                    payload_encoding: "opaque".into(),
                    schema: None,
                }],
                limits: native,
                deadline_ns: 10_000_000_000,
            },
            vec![OfflineSource {
                path: f.source.clone(),
                sequence: 1,
                safe_offset: None,
            }],
            0,
            &Cancellation::new(),
        )
        .unwrap();
        let finished = Arc::new(AtomicBool::new(false));
        let flag = finished.clone();
        let host = tokio::spawn(async move {
            let origin = Instant::now();
            let no = std::sync::atomic::AtomicBool::new(false);
            let mut snapshot = Some(snapshot);
            let mut replay: Option<Replay<File>> = None;
            let mut blobs = Vec::new();
            let mut query = Query {
                basis: TimeBasis::Mono,
                from: 0,
                to: 100,
                channel: None,
                minimum_level: Level::Trace,
            };
            while !flag.load(Ordering::Acquire) {
                if let Some(command) = inbox.try_next() {
                    let now = origin.elapsed().as_nanos() as u64;
                    let result = match command.request().clone() {
                        ManagementRequest::GetStatus => Ok(ManagementReply::Status {
                            session: 42,
                            revision: 1,
                            report: Vec::new(),
                        }),
                        ManagementRequest::Query { snapshot, .. }
                            if snapshot != 0 && snapshot != 9 =>
                        {
                            Err(RemoteError::Gone)
                        }
                        ManagementRequest::Query { query: q, .. } => {
                            query = q;
                            let snap = snapshot.as_mut().unwrap();
                            let cursor = snap.start(q, now, &no).unwrap();
                            let page = snap.page(&cursor, now, &no).unwrap();
                            blobs.clear();
                            let items = page
                                .records
                                .iter()
                                .map(|r| {
                                    let Payload::Blob(blob) = &r.payload else {
                                        panic!("blob")
                                    };
                                    blobs.push(blob.clone());
                                    RemoteItem {
                                        record: r.record,
                                        inline: vec![],
                                        blob: blobs.len() as u64,
                                    }
                                })
                                .collect();
                            Ok(ManagementReply::Page {
                                snapshot: 9,
                                cursor: 0,
                                items,
                            })
                        }
                        ManagementRequest::ReadBlob {
                            snapshot: 9,
                            blob,
                            offset,
                            length,
                        } => Ok(ManagementReply::Blob(
                            snapshot
                                .as_mut()
                                .unwrap()
                                .read_blob(
                                    &blobs[blob as usize - 1],
                                    offset,
                                    length as usize,
                                    now,
                                    &no,
                                )
                                .unwrap()
                                .to_vec(),
                        )),
                        ManagementRequest::Replay {
                            handle: 9,
                            action,
                            generation,
                            from,
                            speed,
                        } => {
                            if action == ReplayAction::Start {
                                replay = Some(
                                    Replay::new(snapshot.take().unwrap(), query, speed, now, &no)
                                        .unwrap(),
                                );
                            }
                            let r = replay.as_mut().unwrap();
                            if generation != 0 && generation != r.generation() {
                                Err(RemoteError::Gone)
                            } else {
                                match action {
                                    ReplayAction::Start => {}
                                    ReplayAction::Pause => r.pause(now, &no).unwrap(),
                                    ReplayAction::Resume => r.resume(now, &no).unwrap(),
                                    ReplayAction::Seek => {
                                        r.seek(from, now, &no).unwrap();
                                    }
                                    ReplayAction::Stop => {
                                        r.stop();
                                    }
                                    ReplayAction::Speed => r.set_speed(speed, now, &no).unwrap(),
                                }
                                Ok(ManagementReply::Replay {
                                    generation: r.generation(),
                                    state: r.state(),
                                })
                            }
                        }
                        ManagementRequest::GetCompletion { .. } => Ok(ManagementReply::Completion(
                            dot_logger::completion::RecordStatus {
                                confirmed: dot_logger::completion::Boundaries::default(),
                                outcome: dot_logger::completion::QueryOutcome::Unknown,
                            },
                        )),
                        ManagementRequest::ApplyConfig {
                            base_revision: 1,
                            patch,
                        } if patch == b"{}" => Ok(ManagementReply::Applied(2)),
                        _ => Err(RemoteError::Unsupported),
                    };
                    command.respond(result);
                } else {
                    tokio::task::yield_now().await;
                }
            }
        });
        let output = command()
            .args(["status", "--json", "--endpoint", &endpoint])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            lines(&output.stdout)[0]["data"]["metrics"]["state"],
            "Unavailable"
        );
        let expired = command()
            .args([
                "query",
                "--json",
                "--endpoint",
                &endpoint,
                "--snapshot",
                "999",
            ])
            .output()
            .unwrap();
        assert_eq!(expired.status.code(), Some(3));
        assert_eq!(
            lines(&expired.stdout).last().unwrap()["data"]["outcome"],
            "gone_or_expired"
        );
        let output = command()
            .args([
                "query",
                "--json",
                "--endpoint",
                &endpoint,
                "--to",
                "100",
                "--include-payload",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let rows = lines(&output.stdout);
        assert_eq!(rows[0]["data"]["data_hex"], "010203");
        assert_eq!(rows.last().unwrap()["data"]["snapshot"], "9");
        for (action, generation, expected) in [
            ("start", "0", "1"),
            ("pause", "1", "1"),
            ("resume", "1", "1"),
            ("seek", "1", "2"),
            ("stop", "2", "2"),
        ] {
            let output = command()
                .args([
                    "replay",
                    "--json",
                    "--endpoint",
                    &endpoint,
                    "--handle",
                    "9",
                    "--action",
                    action,
                    "--generation",
                    generation,
                    "--from",
                    "0",
                ])
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            assert_eq!(lines(&output.stdout)[0]["data"]["generation"], expected);
        }
        let patch = f.dir.join("patch.json");
        std::fs::write(&patch, b"{}").unwrap();
        let output = command()
            .args([
                "config",
                "apply",
                "--json",
                "--endpoint",
                &endpoint,
                "--base-revision",
                "1",
                "--patch",
            ])
            .arg(patch)
            .args(["--token-env", "DOT_LOGGER_TEST_TOKEN"])
            .env("DOT_LOGGER_TEST_TOKEN", "test-only-manage-credential")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(lines(&output.stdout)[0]["data"]["revision"], "2");
        let output = command()
            .args([
                "check",
                "run",
                "--json",
                "--endpoint",
                &endpoint,
                "--token-env",
                "DOT_LOGGER_TEST_TOKEN",
            ])
            .env("DOT_LOGGER_TEST_TOKEN", "test-only-manage-credential")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(lines(&output.stdout)[0]["data"]["outcome"], "unsupported");
        finished.store(true, Ordering::Release);
        host.await.unwrap();
        stop.send(()).unwrap();
        server_task.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn non_regular_offline_source_is_rejected_as_input() {
    let f = fixture([10, 20]);
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    manifest["sources"][0]["path"] = json!(f.dir);
    std::fs::write(&f.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = command()
        .args(["query", "--json", "--manifest"])
        .arg(&f.manifest)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn cli_tail_acks_only_after_successful_stdout_handoff() {
    use dot_logger::{adapters::remote::*, query::QueryBudget};
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        for broken in [false, true] {
            let mut c = support::plan().config().clone();
            c.memory.external_bytes = 8 * 1024 * 1024;
            c.memory.total_bytes += 8 * 1024 * 1024;
            let plan = SessionBuilder::new(c)
                .validate(&PlatformCapabilities::default())
                .unwrap();
            let budget = QueryBudget::new(&plan);
            let (hub, inbox) = RemoteHub::new(
                &budget,
                RemoteLimits {
                    connections: 2,
                    subscriptions: 2,
                    commands: 2,
                    message_bytes: 4096,
                    record_bytes: 8192,
                    subscription_records: 2,
                    subscription_bytes: 16384,
                    max_age: Duration::from_secs(2),
                    request_timeout: Duration::from_secs(2),
                },
            )
            .unwrap();
            let server = RemoteServer::bind(
                ServerConfig {
                    enabled: true,
                    ..Default::default()
                },
                hub.clone(),
            )
            .await
            .unwrap();
            let endpoint = format!("http://{}", server.local_addr().unwrap());
            let (stop, done) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(server.serve(async {
                let _ = done.await;
            }));
            let mut child = command()
                .args([
                    "tail",
                    "--json",
                    "--endpoint",
                    &endpoint,
                    "--record-bytes",
                    "8",
                    "--age-ms",
                    "1000",
                    "--limit",
                    "1",
                    "--timeout-ms",
                    "2000",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            if broken {
                drop(child.stdout.take());
            }
            let mut ids = [0; 2];
            if tokio::time::timeout(Duration::from_secs(2), async {
                while hub.subscriptions(&mut ids) == 0 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .is_err()
            {
                let _ = child.kill();
                panic!("CLI subscription did not start");
            }
            let mut metadata = support::meta();
            metadata.importance = Some(Importance::Normal);
            let ack = hub
                .offer(
                    ids[0],
                    CapturedRecord {
                        receipt: CaptureReceipt {
                            session_id: 42,
                            producer_id: 0,
                            accepted_seq: 1,
                        },
                        metadata,
                        revision: 1,
                        payload_len: 1,
                    },
                    &[9],
                    false,
                    1,
                )
                .unwrap();
            let output = child.wait_with_output().unwrap();
            assert_eq!(
                output.status.code(),
                Some(if broken { 4 } else { 0 }),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            tokio::time::timeout(Duration::from_secs(2), async {
                while ack.state() == AckState::Pending {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            assert_eq!(
                ack.state(),
                if broken {
                    AckState::Unknown
                } else {
                    AckState::Acknowledged
                }
            );
            if !broken {
                assert_eq!(
                    lines(&output.stdout).last().unwrap()["data"]["acknowledged"],
                    1
                );
            }
            drop((ack, inbox));
            stop.send(()).unwrap();
            task.await.unwrap().unwrap();
            drop(hub);
            assert_eq!(budget.used_bytes(), 0);
        }
    });
}

#[cfg(unix)]
#[test]
fn non_unicode_credential_environment_is_not_downgraded_to_anonymous() {
    use std::os::unix::ffi::OsStringExt;
    let output = command()
        .args([
            "status",
            "--json",
            "--endpoint",
            "http://127.0.0.1:1",
            "--token-env",
            "DOT_LOGGER_TEST_TOKEN",
        ])
        .env(
            "DOT_LOGGER_TEST_TOKEN",
            std::ffi::OsString::from_vec(vec![0xff]),
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
/// TASK-028 S2 (API §19.8): `calibrate` measures once, saves a new profile and
/// leaves no probe file; an existing profile is kept and nothing is measured.
/// Measuring needs the Linux free-space check (API §19.9).
#[cfg(target_os = "linux")]
#[test]
fn calibrate_saves_a_new_profile_and_never_overwrites() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "dot-logger-cli-calibrate-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let profile = dir.join("machine.json");
    let run = || {
        command()
            .args(["calibrate", "--json", "--directory"])
            .arg(&dir)
            .arg("--output")
            .arg(&profile)
            .args(["--write-limit", "2097152", "--disk-reserve", "0"])
            .output()
            .unwrap()
    };
    let output = run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let rows = lines(&output.stdout);
    assert_eq!(rows[0]["kind"], "calibration");
    assert_eq!(rows[0]["data"]["write"]["bytes"], 2_097_152);
    assert!(rows[0]["data"]["derived"]["sustained_bytes_per_second"]["assumption"].is_string());
    assert_eq!(rows.last().unwrap()["kind"], "summary");
    let saved: Value = serde_json::from_slice(&std::fs::read(&profile).unwrap()).unwrap();
    assert_eq!(saved["format"], "dot-logger-calibration");
    let names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("machine.json")]);

    let before = std::fs::read(&profile).unwrap();
    let output = run();
    assert_eq!(
        output.status.code(),
        Some(3),
        "existing profile is a conflict"
    );
    assert!(
        !lines(&output.stdout)
            .iter()
            .any(|r| r["kind"] == "calibration")
    );
    assert_eq!(std::fs::read(&profile).unwrap(), before);

    let output = command()
        .args(["calibrate", "--json", "--output"])
        .arg(dir.join("other.json"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "--directory is required");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// REV-053 RV-132: elsewhere the free-space check is `Unsupported` (API
/// §19.9); `calibrate` fails as IO, writes nothing and still keeps an
/// existing profile.
#[cfg(not(target_os = "linux"))]
#[test]
fn calibrate_on_other_platforms_fails_without_writing() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "dot-logger-cli-calibrate-other-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let profile = dir.join("machine.json");
    let run = || {
        command()
            .args(["calibrate", "--json", "--directory"])
            .arg(&dir)
            .arg("--output")
            .arg(&profile)
            .args(["--disk-reserve", "0"])
            .output()
            .unwrap()
    };
    let output = run();
    assert_eq!(
        output.status.code(),
        Some(4),
        "unsupported free-space check"
    );
    assert!(
        !lines(&output.stdout)
            .iter()
            .any(|r| r["kind"] == "calibration")
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);

    std::fs::write(&profile, b"kept").unwrap();
    assert_eq!(
        run().status.code(),
        Some(3),
        "existing profile is a conflict"
    );
    assert_eq!(std::fs::read(&profile).unwrap(), b"kept");
    std::fs::remove_dir_all(&dir).unwrap();
}
