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
            let mut return_invalid = false;
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
                                    ReplayAction::Speed => {
                                        if speed != Speed::new(3, 2).unwrap() {
                                            return_invalid = true;
                                        }
                                        r.set_speed(speed, now, &no).unwrap()
                                    }
                                }
                                if std::mem::take(&mut return_invalid) {
                                    Err(RemoteError::InvalidInput)
                                } else {
                                    Ok(ManagementReply::Replay {
                                        generation: r.generation(),
                                        state: r.state(),
                                    })
                                }
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
        assert_eq!(rows[0]["data"]["blob"], "1");
        let output = command()
            .args([
                "blob",
                "--json",
                "--endpoint",
                &endpoint,
                "--snapshot",
                "9",
                "--blob",
                "1",
                "--offset",
                "1",
                "--length",
                "2",
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
        assert_eq!(rows[0]["command"], "blob");
        assert_eq!(rows[0]["kind"], "blob");
        assert_eq!(rows[0]["data"]["bytes"], 2);
        assert_eq!(rows[0]["data"]["data_hex"], "0203");
        for (action, generation, expected) in [
            ("start", "0", "1"),
            ("pause", "1", "1"),
            ("resume", "1", "1"),
            ("seek", "1", "2"),
            ("speed", "2", "2"),
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
                    "--speed",
                    "3/2",
                ])
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{action}: {}",
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
fn actual_cli_remote_completion_reports_host_outcome_and_confirmed_boundaries() {
    use dot_logger::{
        adapters::remote::*,
        completion::{Boundaries, Boundary, FailureReason, QueryOutcome, RecordStatus},
        query::QueryBudget,
        routing::DropReason,
    };
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
                ..Default::default()
            },
            hub.clone(),
        )
        .await
        .unwrap();
        let endpoint = format!("http://{}", server.local_addr().unwrap());
        let (stop, done) = tokio::sync::oneshot::channel();
        let server_task = tokio::spawn(server.serve(async {
            let _ = done.await;
        }));
        let both = Boundaries::only(Boundary::Enqueued) | Boundaries::only(Boundary::Written);
        let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = finished.clone();
        let host = tokio::spawn(async move {
            let mut served = 0;
            while !flag.load(Ordering::Acquire) {
                let Some(command) = inbox.try_next() else {
                    tokio::task::yield_now().await;
                    continue;
                };
                served += 1;
                // The host answers only the exact receipt/target/boundary the CLI was given.
                let result = match *command.request() {
                    ManagementRequest::GetCompletion {
                        receipt,
                        target: 3,
                        boundary: Boundary::Synced,
                    } if receipt.session_id == 42 && receipt.producer_id == 1 => {
                        let (confirmed, outcome) = match receipt.accepted_seq {
                            1 => (
                                both | Boundaries::only(Boundary::Synced),
                                QueryOutcome::Complete,
                            ),
                            2 => (
                                Boundaries::only(Boundary::Enqueued),
                                QueryOutcome::Dropped(DropReason::PendingFull),
                            ),
                            3 => (both, QueryOutcome::Failed(FailureReason::SyncFailure)),
                            _ => (Boundaries::default(), QueryOutcome::NotRouted),
                        };
                        Ok(ManagementReply::Completion(RecordStatus {
                            confirmed,
                            outcome,
                        }))
                    }
                    // Target 4 confirms exactly the boundary the CLI asked for.
                    ManagementRequest::GetCompletion {
                        target: 4,
                        boundary,
                        ..
                    } => Ok(ManagementReply::Completion(RecordStatus {
                        confirmed: Boundaries::only(boundary),
                        outcome: QueryOutcome::Unknown,
                    })),
                    _ => Err(RemoteError::Unsupported),
                };
                command.respond(result);
            }
            served
        });
        for (sequence, outcome, confirmed) in [
            ("1", "Complete", json!(["Enqueued", "Written", "Synced"])),
            ("2", "Dropped(PendingFull)", json!(["Enqueued"])),
            ("3", "Failed(SyncFailure)", json!(["Enqueued", "Written"])),
            ("4", "NotRouted", json!([])),
        ] {
            let output = command()
                .args([
                    "completion",
                    "--json",
                    "--endpoint",
                    &endpoint,
                    "--session",
                    "0000000000000000000000000000002a",
                    "--producer",
                    "1",
                    "--sequence",
                    sequence,
                    "--target",
                    "3",
                    "--boundary",
                    "synced",
                ])
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{sequence}: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            let rows = lines(&output.stdout);
            assert_eq!(rows[0]["command"], "completion");
            assert_eq!(rows[0]["kind"], "completion");
            assert_eq!(rows[0]["data"]["outcome"], outcome, "{sequence}");
            assert_eq!(rows[0]["data"]["confirmed"], confirmed, "{sequence}");
        }
        for (boundary, confirmed) in [
            ("enqueued", "Enqueued"),
            ("written", "Written"),
            ("synced", "Synced"),
            ("database", "DatabaseAcknowledged"),
            ("receiver", "ReceiverAcknowledged"),
        ] {
            let output = command()
                .args(["completion", "--json", "--endpoint", &endpoint])
                .args(["--session", "0000000000000000000000000000002a"])
                .args(["--sequence", "1", "--target", "4", "--boundary", boundary])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(0), "{boundary}");
            let rows = lines(&output.stdout);
            assert_eq!(rows[0]["data"]["outcome"], "Unknown", "{boundary}");
            assert_eq!(
                rows[0]["data"]["confirmed"],
                json!([confirmed]),
                "{boundary}"
            );
        }
        finished.store(true, Ordering::Release);
        assert_eq!(host.await.unwrap(), 9);
        stop.send(()).unwrap();
        server_task.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn invalid_blob_and_completion_arguments_are_input_errors_before_any_connect() {
    let receipt = [
        "--session",
        "0000000000000000000000000000002a",
        "--sequence",
    ];
    for args in [
        vec!["blob", "--snapshot", "9", "--blob", "1", "--length", "0"],
        vec![
            "blob",
            "--snapshot",
            "9",
            "--blob",
            "1",
            "--offset",
            "18446744073709551615",
            "--length",
            "2",
        ],
        [&["completion"][..], &receipt, &["0"]].concat(),
        [
            &["completion"][..],
            &receipt,
            &["1", "--boundary", "flushed"],
        ]
        .concat(),
        vec!["completion", "--session", "2a", "--sequence", "1"],
        vec!["tail", "--channel", "x"],
        vec!["query", "--limit", "0"],
        vec!["replay", "--action", "jump"],
        vec!["status", "--token-env", "BAD-NAME"],
        vec!["status", "--max-output-bytes", "1023"],
        vec!["status", "--manifest", "session.json"],
    ] {
        let output = command()
            .args(&args)
            .args(["--json", "--endpoint", "http://127.0.0.1:1"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let last = lines(&output.stdout).pop().unwrap();
        assert_eq!(last["kind"], "error", "{args:?}");
        assert_eq!(last["data"]["outcome"], "invalid_input", "{args:?}");
    }
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

/// TASK-033 S3 (UC-31): the CLI passes `--channel`, `--level` and `--basis`
/// into the query. The fixture holds two Info records on channel 7 at mono
/// 10/20 and real 2; each flag must select or exclude them, and invalid
/// values are input errors.
#[test]
fn offline_query_channel_level_and_basis_flags_reach_the_query() {
    let f = fixture([10, 20]);
    let count = |extra: &[&str]| {
        let output = command()
            .args(["query", "--json", "--manifest"])
            .arg(&f.manifest)
            .args(extra)
            .output()
            .unwrap();
        let rows = lines(&output.stdout);
        let records = rows.iter().filter(|row| row["kind"] == "record").count();
        (output.status.code(), records)
    };
    let window = ["--from", "0", "--to", "100"];
    let with = |more: &[&'static str]| [&window[..], more].concat();
    assert_eq!(count(&with(&["--channel", "7"])), (Some(0), 2));
    assert_eq!(count(&with(&["--channel", "8"])), (Some(0), 0));
    assert_eq!(count(&with(&["--level", "info"])), (Some(0), 2));
    assert_eq!(count(&with(&["--level", "warn"])), (Some(0), 0));
    // Real time is 2 for both records: inside [0, 3), outside [3, 100).
    assert_eq!(
        count(&["--basis", "real", "--from", "0", "--to", "3"]),
        (Some(0), 2)
    );
    assert_eq!(
        count(&["--basis", "real", "--from", "3", "--to", "100"]),
        (Some(0), 0)
    );
    // Mono 10 and 20: a mono window around 15 keeps only the second.
    assert_eq!(count(&["--from", "15", "--to", "100"]), (Some(0), 1));
    // The records carry no source time, so a source-time query has no
    // domain to compare and no conversion is inferred: an input error.
    assert_eq!(
        count(&with(&["--basis", "source", "--clock-domain", "5"])),
        (Some(2), 0)
    );
    for bad in [
        &["--basis", "wall", "--from", "0", "--to", "100"][..],
        &[
            "--basis",
            "source",
            "--clock-domain",
            "0",
            "--from",
            "0",
            "--to",
            "100",
        ],
        &["--from", "5", "--to", "5"],
        &["--channel", "x", "--from", "0", "--to", "100"],
        &["--level", "loud", "--from", "0", "--to", "100"],
    ] {
        assert_eq!(count(bad).0, Some(2), "{bad:?} is an input error");
    }
}

/// Writes a changed copy of the fixture manifest and returns its path.
fn manifest_with(f: &Fixture, name: &str, change: impl FnOnce(&mut Value)) -> std::path::PathBuf {
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    change(&mut manifest);
    let path = f.dir.join(name);
    std::fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    path
}
/// Runs the CLI with `--json`; returns the exit code and the NDJSON rows.
fn run(args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> (Option<i32>, Vec<Value>) {
    let output = command().arg("--json").args(args).output().unwrap();
    (output.status.code(), lines(&output.stdout))
}
/// The README contract: exit 0/3 end with `summary`, exit 2/4 with `error`,
/// and the terminal row names the outcome.
#[track_caller]
fn assert_outcome(result: &(Option<i32>, Vec<Value>), code: i32, outcome: &str) {
    let last = result.1.last().unwrap();
    assert_eq!(result.0, Some(code), "{last}");
    assert_eq!(last["data"]["outcome"], outcome, "{last}");
    assert_eq!(
        last["data"]["exit_code"],
        if code == 0 { Value::Null } else { json!(code) }
    );
    let kind = if matches!(code, 0 | 3) {
        "summary"
    } else {
        "error"
    };
    assert_eq!(last["kind"], kind, "{last}");
}
fn records(rows: &[Value]) -> Vec<&Value> {
    rows.iter().filter(|r| r["kind"] == "record").collect()
}

/// Manifest declaration errors are rejected by `config validate` as input
/// (exit 2) before any source is read; a valid descriptor is accepted.
#[test]
fn offline_manifest_declaration_errors_are_input_errors() {
    let f = fixture([10, 20]);
    let validate = |manifest: &std::path::Path| {
        run([
            std::ffi::OsStr::new("config"),
            "validate".as_ref(),
            "--manifest".as_ref(),
            manifest.as_os_str(),
        ])
    };
    fn descriptor(hex: &str) -> Value {
        json!({"encoding":"protobuf","data_hex":hex,"name":"my.Pose"})
    }
    /// Adds channel 8 with the same schema id and version as channel 7.
    fn second(m: &mut Value, hex: Option<&str>) {
        let mut other = m["channels"][0].clone();
        other["id"] = json!(8);
        other["topic"] = json!("other");
        if let Some(hex) = hex {
            other["descriptor"] = descriptor(hex);
        }
        m["channels"].as_array_mut().unwrap().push(other);
    }
    let valid = manifest_with(&f, "valid.json", |m| {
        m["channels"][0]["descriptor"] = descriptor("0a0B");
    });
    let result = validate(&valid);
    assert_outcome(&result, 0, "complete");
    assert_eq!(result.1[0]["data"]["valid"], true);
    type Change = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Change)> = vec![
        (
            "schema_version",
            Box::new(|m| m["schema_version"] = json!(2)),
        ),
        ("unknown field", Box::new(|m| m["extra"] = json!(1))),
        (
            "odd descriptor hex",
            Box::new(|m| m["channels"][0]["descriptor"] = descriptor("abc")),
        ),
        (
            "non-hex descriptor",
            Box::new(|m| m["channels"][0]["descriptor"] = descriptor("zz")),
        ),
        (
            "same schema, other descriptor",
            Box::new(|m| {
                m["channels"][0]["descriptor"] = descriptor("0a0b");
                second(m, Some("0a0c"));
            }),
        ),
        (
            "duplicate source sequence",
            Box::new(|m| {
                let mut other = m["sources"][0].clone();
                other["path"] = json!("other.mcap");
                m["sources"].as_array_mut().unwrap().push(other);
            }),
        ),
        (
            "reservation above external bytes",
            Box::new(|m| m["external_bytes"] = json!(1)),
        ),
    ];
    for (what, change) in cases {
        let path = manifest_with(&f, "bad.json", change);
        let result = validate(&path);
        assert!(result.0 == Some(2), "{what}: {:?}", result.1);
        assert_outcome(&result, 2, "invalid_input");
    }
    // Accepted for a different channel id with the same descriptor.
    let same = manifest_with(&f, "same.json", |m| {
        m["channels"][0]["descriptor"] = descriptor("0a0b");
        second(m, Some("0a0b"));
    });
    assert_outcome(&validate(&same), 0, "complete");
    // Malformed JSON, a directory and an over-4-MiB file are input errors.
    let malformed = f.dir.join("malformed.json");
    std::fs::write(&malformed, b"{").unwrap();
    let large = f.dir.join("large.json");
    std::fs::File::create(&large)
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    for path in [&malformed, &f.dir, &large] {
        assert_outcome(&validate(path), 2, "invalid_input");
    }
}

/// A relative source resolves next to the manifest. A missing source is IO
/// (exit 4), a source above `max_source_bytes` is a limit (exit 3), a broken
/// MCAP is invalid data (exit 4) and a foreign MCAP profile is unsupported
/// (exit 2).
#[test]
fn offline_source_failures_map_to_stable_exit_codes() {
    let f = fixture([10, 20]);
    let query = |manifest: &std::path::Path| {
        run([
            std::ffi::OsStr::new("query"),
            "--manifest".as_ref(),
            manifest.as_os_str(),
        ])
    };
    let relative = manifest_with(&f, "relative.json", |m| {
        m["sources"][0]["path"] = json!("source.mcap")
    });
    let result = query(&relative);
    assert_outcome(&result, 0, "complete");
    assert_eq!(records(&result.1).len(), 2);
    let missing = manifest_with(&f, "missing.json", |m| {
        m["sources"][0]["path"] = json!("missing.mcap")
    });
    assert_outcome(&query(&missing), 4, "io_or_transport");
    let small = manifest_with(&f, "small.json", |m| {
        m["limits"]["max_source_bytes"] = json!(16)
    });
    assert_outcome(&query(&small), 3, "limit");
    // A read budget that covers only the opening scan stops the first page
    // read, for query and for replay.
    let size = std::fs::metadata(&f.source).unwrap().len();
    let budget = manifest_with(&f, "budget.json", |m| {
        m["limits"]["read_bytes"] = json!(size)
    });
    assert_outcome(&query(&budget), 3, "limit");
    let result = run([
        std::ffi::OsStr::new("replay"),
        "--manifest".as_ref(),
        budget.as_os_str(),
    ]);
    assert_outcome(&result, 3, "limit");
    assert!(records(&result.1).is_empty());
    let mut broken = std::fs::read(&f.source).unwrap();
    broken[0] = 0;
    std::fs::write(f.dir.join("broken.mcap"), &broken).unwrap();
    let broken = manifest_with(&f, "broken.json", |m| {
        m["sources"][0]["path"] = json!("broken.mcap")
    });
    assert_outcome(&query(&broken), 4, "invalid_data");
    // A valid MCAP header whose profile is not `dot-logger/standard/1`.
    let mut foreign = b"\x89MCAP0\r\n\x01".to_vec();
    foreign.extend_from_slice(&9u64.to_le_bytes());
    foreign.extend_from_slice(&1u32.to_le_bytes());
    foreign.push(b'x');
    foreign.extend_from_slice(&0u32.to_le_bytes());
    foreign.extend_from_slice(b"\x89MCAP0\r\n");
    std::fs::write(f.dir.join("foreign.mcap"), &foreign).unwrap();
    let foreign = manifest_with(&f, "foreign.json", |m| {
        m["sources"][0]["path"] = json!("foreign.mcap")
    });
    assert_outcome(&query(&foreign), 2, "unsupported");
}

/// `--include-payload` returns the bytes of inline and blob records for query
/// and replay. A payload that does not fit the output cap is a partial limit
/// (exit 3) with no record row.
#[test]
fn offline_payload_output_reads_inline_and_blob_records_within_the_output_cap() {
    let f = fixture([10, 20]);
    // `inline_bytes` 16 keeps the 3-byte payloads inline; 0 stores them as blobs.
    let inline = f.manifest.clone();
    let blob = manifest_with(&f, "blob.json", |m| m["limits"]["inline_bytes"] = json!(0));
    for (command, manifest, storage) in [
        ("query", &inline, "inline"),
        ("query", &blob, "blob"),
        ("replay", &inline, ""),
        ("replay", &blob, ""),
    ] {
        let result = run([
            command.as_ref(),
            "--include-payload".as_ref(),
            "--manifest".as_ref(),
            manifest.as_os_str(),
        ]);
        assert_outcome(&result, 0, "complete");
        let rows = records(&result.1);
        assert_eq!(rows.len(), 2, "{command} {storage}");
        for row in rows {
            assert_eq!(row["data"]["data_hex"], "010203", "{command} {manifest:?}");
            if command == "query" {
                assert_eq!(row["data"]["payload_storage"], storage);
            }
        }
    }
    // A 1024-byte cap leaves no payload capacity after the terminal reserve.
    // Without payload one record row still fits; the next one does not.
    for (command, manifest, include, count) in [
        ("query", &inline, true, 0),
        ("query", &blob, true, 0),
        ("replay", &blob, true, 0),
        ("replay", &inline, false, 1),
    ] {
        let mut args = vec![command, "--max-output-bytes", "1024"];
        if include {
            args.push("--include-payload");
        }
        let result = run(args
            .iter()
            .map(std::ffi::OsStr::new)
            .chain([std::ffi::OsStr::new("--manifest"), manifest.as_os_str()]));
        assert_outcome(&result, 3, "limit");
        assert_eq!(records(&result.1).len(), count, "{command} {manifest:?}");
    }
}

/// Offline query follows page cursors; like replay, exactly `--limit`
/// records complete (exit 0) and more selected records are partial (exit 3).
#[test]
fn offline_query_pages_and_exact_limit_match_replay() {
    let f = fixture([10, 20]);
    let paged = manifest_with(&f, "paged.json", |m| m["limits"]["page_records"] = json!(1));
    for (limit, code, outcome, count) in [
        ("1", 3, "limit", 1),
        ("2", 0, "complete", 2),
        ("3", 0, "complete", 2),
    ] {
        let result = run([
            std::ffi::OsStr::new("query"),
            "--limit".as_ref(),
            limit.as_ref(),
            "--manifest".as_ref(),
            paged.as_os_str(),
        ]);
        assert_outcome(&result, code, outcome);
        assert_eq!(records(&result.1).len(), count, "--limit {limit}");
    }
    // Replay whose schedule cannot be represented in u64 ns stops as a limit.
    let far = fixture([0, 1 << 62]);
    let result = run([
        std::ffi::OsStr::new("replay"),
        "--speed".as_ref(),
        "1/1000".as_ref(),
        "--manifest".as_ref(),
        far.manifest.as_os_str(),
    ]);
    assert_outcome(&result, 3, "limit");
    assert_eq!(records(&result.1).len(), 1);
}

/// Offline mode supports only start playback on the mono basis; remote-only
/// commands without `--endpoint` and offline-only commands with it are
/// unsupported (exit 2).
#[test]
fn commands_outside_their_mode_are_unsupported() {
    let f = fixture([10, 20]);
    let manifest = f.manifest.to_str().unwrap();
    for args in [
        vec!["replay", "--basis", "real", "--manifest", manifest],
        vec!["replay", "--action", "pause", "--manifest", manifest],
        vec!["tail", "--manifest", manifest],
        vec!["blob", "--manifest", manifest],
        vec![
            "config",
            "apply",
            "--patch",
            manifest,
            "--manifest",
            manifest,
        ],
        vec!["config", "validate", "--endpoint", "http://127.0.0.1:1"],
        vec![
            "recover",
            "--output",
            "x.mcap",
            "--endpoint",
            "http://127.0.0.1:1",
        ],
    ] {
        let result = run(&args);
        assert_eq!(result.0, Some(2), "{args:?}");
        assert_outcome(&result, 2, "unsupported");
    }
}

/// `config diff` reports only whether two declarations differ, never their values.
#[test]
fn config_diff_reports_only_whether_declarations_changed() {
    let f = fixture([10, 20]);
    let changed = manifest_with(&f, "changed.json", |m| m["revision"] = json!(77));
    for (after, expected) in [(&f.manifest, false), (&changed, true)] {
        let output = command()
            .args(["config", "diff", "--json", "--before"])
            .arg(&f.manifest)
            .arg("--after")
            .arg(after)
            .output()
            .unwrap();
        let result = (output.status.code(), lines(&output.stdout));
        assert_outcome(&result, 0, "complete");
        assert_eq!(result.1[0]["kind"], "diff");
        assert_eq!(
            result.1[0]["data"],
            json!({"changed": expected, "values": "redacted"})
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("77"));
    }
    let malformed = f.dir.join("malformed.json");
    std::fs::write(&malformed, b"{").unwrap();
    let result = run([
        std::ffi::OsStr::new("config"),
        "diff".as_ref(),
        "--before".as_ref(),
        f.manifest.as_os_str(),
        "--after".as_ref(),
        malformed.as_os_str(),
    ]);
    assert_outcome(&result, 2, "invalid_input");
    let result = run([
        std::ffi::OsStr::new("config"),
        "diff".as_ref(),
        "--before".as_ref(),
        f.manifest.as_os_str(),
    ]);
    assert_outcome(&result, 2, "invalid_input");
}

/// Offline `status` shows the declared identity and no invented metrics;
/// without `--json` the same rows are human text lines.
#[test]
fn offline_status_and_human_text_output() {
    let f = fixture([10, 20]);
    let result = run([
        std::ffi::OsStr::new("status"),
        "--manifest".as_ref(),
        f.manifest.as_os_str(),
    ]);
    assert_outcome(&result, 0, "complete");
    assert_eq!(result.1[0]["kind"], "status");
    assert_eq!(
        result.1[0]["data"]["session"],
        "0000000000000000000000000000002a"
    );
    assert_eq!(result.1[0]["data"]["revision"], "1");
    assert_eq!(result.1[0]["data"]["metrics"]["state"], "Unavailable");
    let output = command()
        .args(["status", "--manifest"])
        .arg(&f.manifest)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).unwrap();
    let text: Vec<_> = text.lines().collect();
    assert_eq!(text.len(), 2, "{text:?}");
    assert!(text[0].starts_with("status status: {"), "{text:?}");
    assert!(text[1].starts_with("status summary: {"), "{text:?}");
    assert!(serde_json::from_str::<Value>(text[0]).is_err());
}

/// `recover` takes exactly one complete regular source. A broken source
/// reports the unpublished recovery and keeps the partial output (exit 4); a
/// truncated source recovers its verified prefix but is partial (exit 3).
#[test]
fn recover_rejects_unsupported_sources_and_reports_partial_recovery() {
    let f = fixture([10, 20]);
    let recover = |manifest: &std::path::Path, output: &str| {
        run([
            std::ffi::OsStr::new("recover"),
            "--manifest".as_ref(),
            manifest.as_os_str(),
            "--output".as_ref(),
            f.dir.join(output).as_os_str(),
        ])
    };
    let two = manifest_with(&f, "two.json", |m| {
        let mut other = m["sources"][0].clone();
        other["sequence"] = json!(2);
        other["path"] = json!("other.mcap");
        m["sources"].as_array_mut().unwrap().push(other);
    });
    assert_outcome(&recover(&two, "two.mcap"), 2, "invalid_input");
    let directory = manifest_with(&f, "directory.json", |m| {
        m["sources"][0]["path"] = json!(".")
    });
    assert_outcome(&recover(&directory, "directory.mcap"), 2, "invalid_input");
    let active = manifest_with(&f, "active.json", |m| {
        m["sources"][0]["safe_offset"] = json!(8)
    });
    assert_outcome(&recover(&active, "active.mcap"), 2, "unsupported");
    for name in ["two.mcap", "directory.mcap", "active.mcap"] {
        assert!(!f.dir.join(name).exists(), "{name}");
    }
    let mut bytes = std::fs::read(&f.source).unwrap();
    bytes[0] = 0;
    std::fs::write(f.dir.join("broken.mcap"), &bytes).unwrap();
    let broken = manifest_with(&f, "broken.json", |m| {
        m["sources"][0]["path"] = json!("broken.mcap")
    });
    let result = recover(&broken, "broken-out.mcap");
    assert_outcome(&result, 4, "io_or_transport");
    assert_eq!(result.1[0]["kind"], "recovery");
    assert_eq!(result.1[0]["data"]["published"], false);
    assert_eq!(result.1[0]["data"]["stop"], "Failed");
    assert!(
        f.dir.join("broken-out.mcap").exists(),
        "partial output is kept"
    );
    let bytes = std::fs::read(&f.source).unwrap();
    std::fs::write(f.dir.join("cut.mcap"), &bytes[..900]).unwrap();
    let cut = manifest_with(&f, "cut.json", |m| {
        m["sources"][0]["path"] = json!("cut.mcap")
    });
    let result = recover(&cut, "cut-out.mcap");
    assert_outcome(&result, 3, "limit");
    assert_eq!(result.1[0]["kind"], "recovery");
    assert_eq!(result.1[0]["data"]["stop"], "Truncated");
    assert_eq!(result.1[0]["data"]["excluded_to"], 900);
}

/// `calibrate` maps its failures to stable exits before writing a profile: a
/// relative directory is input (2), a reserve above the free space is a limit
/// (3) and a missing directory or profile directory is IO (4).
#[cfg(target_os = "linux")]
#[test]
fn calibrate_failures_map_to_stable_exit_codes_and_write_no_profile() {
    let f = fixture([10, 20]);
    let calibrate = |directory: &std::ffi::OsStr, profile: &std::path::Path, reserve: &str| {
        run([
            std::ffi::OsStr::new("calibrate"),
            "--directory".as_ref(),
            directory,
            "--output".as_ref(),
            profile.as_os_str(),
            "--write-limit".as_ref(),
            "4096".as_ref(),
            "--disk-reserve".as_ref(),
            reserve.as_ref(),
        ])
    };
    let profile = f.dir.join("machine.json");
    assert_outcome(
        &calibrate("relative".as_ref(), &profile, "0"),
        2,
        "invalid_input",
    );
    assert_outcome(
        &calibrate(f.dir.as_os_str(), &profile, "18446744073709551615"),
        3,
        "limit",
    );
    assert_outcome(
        &calibrate(f.dir.join("missing").as_os_str(), &profile, "0"),
        4,
        "io_or_transport",
    );
    assert!(!profile.exists());
    // The measurement (4 KiB) is reported, but the profile cannot be saved.
    let result = calibrate(
        f.dir.as_os_str(),
        &f.dir.join("missing").join("machine.json"),
        "0",
    );
    assert_outcome(&result, 4, "io_or_transport");
    assert_eq!(result.1[0]["kind"], "calibration");
}

/// Serves `reply` from a loopback host with a Manage token and runs `body`
/// (endpoint, hub) on a blocking thread; the budget is released afterwards.
fn with_host(
    reply: impl Fn(
        &dot_logger::adapters::remote::ManagementRequest,
    ) -> Result<
        dot_logger::adapters::remote::ManagementReply,
        dot_logger::adapters::remote::RemoteError,
    > + Send
    + 'static,
    body: impl FnOnce(&str, &dot_logger::adapters::remote::RemoteHub),
) {
    use dot_logger::{adapters::remote::*, query::QueryBudget};
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
            connections: 4,
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
        let endpoint = format!("http://{}", server.local_addr().unwrap());
        let (stop, done) = tokio::sync::oneshot::channel();
        let server_task = tokio::spawn(server.serve(async {
            let _ = done.await;
        }));
        let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = finished.clone();
        let host = tokio::spawn(async move {
            while !flag.load(Ordering::Acquire) {
                if let Some(command) = inbox.try_next() {
                    let result = reply(command.request());
                    command.respond(result);
                } else {
                    tokio::task::yield_now().await;
                }
            }
        });
        tokio::task::block_in_place(|| body(&endpoint, &hub));
        finished.store(true, Ordering::Release);
        host.await.unwrap();
        stop.send(()).unwrap();
        server_task.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}
fn remote_record(sequence: u64, payload_len: usize) -> CapturedRecord {
    let mut metadata = support::meta();
    metadata.importance = Some(Importance::Normal);
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: sequence,
        },
        metadata,
        revision: 1,
        payload_len,
    }
}

/// Host replies that are errors, malformed or too large for the output cap
/// map to the README exit codes. A remote IO failure keeps
/// `posted_host_io: completion_unconfirmed`; a protocol mismatch and a known
/// check outcome do not.
#[test]
fn remote_reply_failures_map_to_stable_exit_codes() {
    use dot_logger::adapters::remote::*;
    with_host(
        |request| {
            let report = |v: Value| Ok(ManagementReply::Report(serde_json::to_vec(&v).unwrap()));
            match *request {
                ManagementRequest::Compare { before: 1, .. } => Err(RemoteError::Corrupt),
                ManagementRequest::Compare { before: 2, .. } => Err(RemoteError::Version),
                ManagementRequest::Compare { before: 3, .. } => report(json!({"schema":"other"})),
                ManagementRequest::Compare { before: 4, .. } => {
                    report(json!({"schema":"dot_logger.compare.v1"}))
                }
                ManagementRequest::PlanCheck { plan: 1 } => {
                    report(json!({"schema":"dot_logger.check.v1","kind":"result"}))
                }
                ManagementRequest::RunCheck { plan: 2 } => report(
                    json!({"schema":"dot_logger.check.v1","kind":"result","outcome":"Maybe"}),
                ),
                ManagementRequest::RunCheck { plan: 3 } => report(
                    json!({"schema":"dot_logger.check.v1","kind":"result","outcome":"Aborted"}),
                ),
                // Two inline pages: cursor 7 continues at 8, which ends the query.
                ManagementRequest::Query {
                    cursor: cursor @ (7 | 8),
                    ..
                } => Ok(ManagementReply::Page {
                    snapshot: 9,
                    cursor: if cursor == 7 { 8 } else { 0 },
                    items: vec![RemoteItem {
                        record: remote_record(cursor, 3),
                        inline: vec![1, 2, 3],
                        blob: 0,
                    }],
                }),
                // A cursor that does not advance would loop forever.
                ManagementRequest::Query { cursor: 5, .. } => Ok(ManagementReply::Page {
                    snapshot: 9,
                    cursor: 5,
                    items: vec![],
                }),
                ManagementRequest::Query { .. } => Ok(ManagementReply::Page {
                    snapshot: 9,
                    cursor: 0,
                    items: vec![RemoteItem {
                        record: remote_record(1, 3),
                        inline: vec![],
                        blob: 1,
                    }],
                }),
                // An empty chunk for a non-empty blob would never finish.
                ManagementRequest::ReadBlob { snapshot: 9, .. } => {
                    Ok(ManagementReply::Blob(vec![]))
                }
                ManagementRequest::ReadBlob { .. } => Ok(ManagementReply::Blob(vec![1, 2, 3])),
                _ => Err(RemoteError::Unsupported),
            }
        },
        |endpoint, _| {
            let query = |extra: &[&str]| {
                let output = command()
                    .args(["query", "--json", "--endpoint", endpoint, "--cursor", "7"])
                    .args(extra)
                    .env("DOT_LOGGER_TOKEN", "test-only-manage-credential")
                    .output()
                    .unwrap();
                (output.status.code(), lines(&output.stdout))
            };
            let result = query(&["--include-payload"]);
            assert_outcome(&result, 0, "complete");
            let rows = records(&result.1);
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|r| r["data"]["data_hex"] == "010203"));
            assert_eq!(result.1.last().unwrap()["data"]["cursor"], "0");
            let result = query(&["--limit", "1"]);
            assert_outcome(&result, 3, "limit");
            assert_eq!(records(&result.1).len(), 1);
            let unconfirmed = json!("completion_unconfirmed");
            for (args, code, outcome, posted) in [
                (
                    &["compare", "--before", "1", "--after", "9"][..],
                    4,
                    "invalid_data",
                    &unconfirmed,
                ),
                (
                    &["compare", "--before", "2", "--after", "9"],
                    4,
                    "incompatible_protocol",
                    &Value::Null,
                ),
                (
                    &["compare", "--before", "3", "--after", "9"],
                    4,
                    "invalid_data",
                    &unconfirmed,
                ),
                (
                    &[
                        "compare",
                        "--before",
                        "4",
                        "--after",
                        "9",
                        "--max-output-bytes",
                        "1024",
                    ],
                    3,
                    "limit",
                    &Value::Null,
                ),
                (
                    &["check", "plan", "--plan", "1"],
                    4,
                    "invalid_data",
                    &unconfirmed,
                ),
                (
                    &["check", "run", "--plan", "2"],
                    4,
                    "invalid_data",
                    &unconfirmed,
                ),
                (
                    &["check", "run", "--plan", "3"],
                    3,
                    "check_aborted",
                    &Value::Null,
                ),
                (&["query", "--cursor", "5"], 4, "invalid_data", &unconfirmed),
                (
                    &["query", "--include-payload"],
                    4,
                    "invalid_data",
                    &unconfirmed,
                ),
                (
                    &["query", "--include-payload", "--max-output-bytes", "1024"],
                    3,
                    "limit",
                    &Value::Null,
                ),
                (
                    &[
                        "blob",
                        "--snapshot",
                        "7",
                        "--blob",
                        "1",
                        "--length",
                        "3",
                        "--max-output-bytes",
                        "1024",
                    ],
                    3,
                    "limit",
                    &Value::Null,
                ),
            ] {
                let output = command()
                    .args(args)
                    .args(["--json", "--endpoint", endpoint])
                    .env("DOT_LOGGER_TOKEN", "test-only-manage-credential")
                    .output()
                    .unwrap();
                let result = (output.status.code(), lines(&output.stdout));
                assert_eq!(result.0, Some(code), "{args:?}: {:?}", result.1);
                assert_outcome(&result, code, outcome);
                let last = result.1.last().unwrap();
                assert_eq!(&last["data"]["posted_host_io"], posted, "{args:?}");
                assert!(records(&result.1).is_empty(), "{args:?}");
                if outcome == "check_aborted" {
                    assert_eq!(result.1[0]["kind"], "check");
                    assert_eq!(result.1[0]["data"]["outcome"], "Aborted");
                }
            }
        },
    );
}

/// With the default Gap policy an overflowed record is reported as a `gap`
/// row and later records still arrive and are ACKed. With
/// `--disconnect-on-overflow` the subscription ends without a gap and without
/// ACKing anything (exit 3).
#[test]
fn remote_tail_reports_overflow_gaps_or_disconnects() {
    with_host(
        |_| Err(dot_logger::adapters::remote::RemoteError::Unsupported),
        |endpoint, hub| {
            for disconnect in [false, true] {
                let mut child = command()
                    .args(["tail", "--json", "--endpoint", endpoint])
                    .args([
                        "--record-bytes",
                        "8",
                        "--limit",
                        "2",
                        "--timeout-ms",
                        "3000",
                    ])
                    .args(disconnect.then_some("--disconnect-on-overflow"))
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                let mut ids = [0; 2];
                let start = Instant::now();
                while hub.subscriptions(&mut ids) == 0 {
                    assert!(start.elapsed() < Duration::from_secs(3), "no subscription");
                    std::thread::sleep(Duration::from_millis(1));
                }
                // `--records 1`: the second record overflows while the first is unACKed.
                let first = hub
                    .offer(ids[0], remote_record(1, 1), &[9], false, 1)
                    .unwrap();
                let second = hub
                    .offer(ids[0], remote_record(2, 1), &[9], false, 1)
                    .unwrap();
                let mut stdout = BufReader::new(child.stdout.take().unwrap());
                let mut text = String::new();
                if !disconnect {
                    for _ in 0..2 {
                        stdout.read_line(&mut text).unwrap();
                    }
                    let rows = lines(text.as_bytes());
                    assert_eq!(rows[0]["kind"], "record", "{text}");
                    assert_eq!(rows[1]["kind"], "gap", "{text}");
                    assert_eq!(rows[1]["data"]["reason"], "overflow");
                    assert_eq!(rows[1]["data"]["first"], rows[1]["data"]["last"]);
                    let third = hub
                        .offer(ids[0], remote_record(3, 1), &[9], false, 1)
                        .unwrap();
                    stdout.read_to_string(&mut text).unwrap();
                    drop(third);
                } else {
                    stdout.read_to_string(&mut text).unwrap();
                }
                let status = child.wait().unwrap();
                let rows = lines(text.as_bytes());
                let last = rows.last().unwrap();
                let gaps = rows.iter().filter(|r| r["kind"] == "gap").count();
                if disconnect {
                    assert_eq!(status.code(), Some(3), "{text}");
                    assert_eq!(gaps, 0, "{text}");
                    assert_eq!(last["data"]["acknowledged"], 0, "{text}");
                } else {
                    assert_eq!(status.code(), Some(0), "{text}");
                    assert_eq!(records(&rows).len(), 2, "{text}");
                    assert_eq!(last["data"]["gaps"], 1);
                    assert_eq!(last["data"]["acknowledged"], 2);
                }
                drop((first, second));
            }
        },
    );
}

/// An unreachable host is a transport failure (exit 4). Nothing was posted,
/// so no `posted_host_io` notice is added.
#[test]
fn unreachable_host_is_transport_failure_without_posted_notice() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let endpoint = format!("http://127.0.0.1:{port}");
    let result = run(["status", "--endpoint", &endpoint]);
    assert_outcome(&result, 4, "io_or_transport");
    assert_eq!(result.1.len(), 1);
    assert_eq!(result.1[0]["data"]["posted_host_io"], Value::Null);
}

#[test]
fn obsolete_export_command_is_rejected_without_creating_a_file() {
    let f = fixture([10, 20]);
    let output = f.dir.join("out.mcap");
    let before = std::fs::read(&f.source).unwrap();
    let result = command()
        .args(["export", "--standard", "--input"])
        .arg(&f.source)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(!output.exists());
    assert_eq!(std::fs::read(&f.source).unwrap(), before);
}
