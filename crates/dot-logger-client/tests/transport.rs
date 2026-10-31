//! Actual socket/native DTO, handoff ACK and cancellation contracts.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used)]
use dot_logger::{adapters::remote::*, capture::*, config::*, platform::*, query::QueryBudget};
use dot_logger_client::{
    Cancellation, ClientError, ReceiverConfig, RemoteClient, RemoteConfig, RemoteEvent,
};
use std::time::Duration;

fn budget() -> QueryBudget {
    let mib = 1024 * 1024;
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 132 * mib,
            input_bytes: mib,
            raw_bytes: 0,
            output_bytes: mib,
            external_bytes: 128 * mib,
            control_bytes: mib,
            worker_bytes: mib,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 256,
                important_slots: 0,
            },
            raw: None,
        }],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())
    .unwrap();
    QueryBudget::new(&plan)
}
fn limits() -> RemoteLimits {
    RemoteLimits {
        connections: 2,
        subscriptions: 2,
        commands: 2,
        message_bytes: MAX_CHUNK_BYTES + 1024,
        record_bytes: 8 * 1024 * 1024,
        subscription_records: 2,
        subscription_bytes: 16 * 1024 * 1024,
        max_age: Duration::from_secs(10),
        request_timeout: Duration::from_secs(2),
    }
}
fn record(sequence: u64, length: usize) -> CapturedRecord {
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: sequence,
        },
        metadata: RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: sequence,
            real_ns: sequence as i64,
            source_time: None,
            level: Level::Info,
            importance: Some(Importance::Normal),
        },
        revision: 1,
        payload_len: length,
    }
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn native_status_and_large_subscription_ack_use_the_actual_connection() {
    runtime().block_on(async {
        let budget = budget();
        let (hub, mut inbox) = RemoteHub::new(&budget, limits()).unwrap();
        let server = RemoteServer::bind(
            ServerConfig {
                enabled: true,
                ..ServerConfig::default()
            },
            hub.clone(),
        )
        .await
        .unwrap();
        let address = server.local_addr().unwrap();
        let (stop, done) = tokio::sync::oneshot::channel();
        let server_task = tokio::spawn(server.serve(async {
            let _ = done.await;
        }));
        let cancel = Cancellation::new();
        let mut client =
            RemoteClient::connect(RemoteConfig::new(format!("http://{address}")), &cancel)
                .await
                .unwrap();
        let host = tokio::spawn(async move {
            loop {
                if let Some(command) = inbox.try_next() {
                    assert!(matches!(command.request(), ManagementRequest::GetStatus));
                    command.respond(Ok(ManagementReply::Status {
                        session: 42,
                        revision: 1,
                    }));
                    break;
                }
                tokio::task::yield_now().await;
            }
        });
        assert!(matches!(
            client
                .request(ManagementRequest::GetStatus, &cancel)
                .await
                .unwrap(),
            ManagementReply::Status {
                session: 42,
                revision: 1
            }
        ));
        host.await.unwrap();
        let mut stream = client
            .subscribe(
                &budget,
                SubscriptionFilter {
                    channel: Some(7),
                    minimum_level: Level::Info,
                    originals: true,
                    overflow: OverflowPolicy::Gap,
                },
                ReceiverConfig {
                    records: 1,
                    record_bytes: 8 * 1024 * 1024,
                    ttl: Duration::from_secs(10),
                },
                &cancel,
            )
            .await
            .unwrap();
        let mut ids = [0; 2];
        assert_eq!(hub.subscriptions(&mut ids), 1);
        let payload = vec![7u8; 8 * 1024 * 1024];
        let ack = hub
            .offer(ids[0], record(1, payload.len()), &payload, true, 1)
            .unwrap();
        let RemoteEvent::Record(ready) = stream.next(&cancel).await.unwrap().unwrap() else {
            panic!("record")
        };
        assert_eq!(ack.state(), AckState::Pending);
        stream
            .handoff(
                ready,
                |r, bytes| r.receipt.accepted_seq == 1 && bytes == payload,
                &cancel,
            )
            .await
            .unwrap();
        assert_eq!(ack.state(), AckState::Acknowledged);
        let pending = hub.offer(ids[0], record(2, 1), &[9], false, 1).unwrap();
        client.disconnect();
        assert!(stream.next(&cancel).await.is_err());
        drop(stream);
        tokio::time::timeout(Duration::from_secs(2), async {
            while pending.state() == AckState::Pending {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(pending.state(), AckState::Unknown);
        drop(client);
        stop.send(()).unwrap();
        server_task.await.unwrap().unwrap();
        drop((ack, pending));
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn unsafe_endpoint_and_cancelled_connect_are_rejected_without_secret_output() {
    runtime().block_on(async {
        let cancel = Cancellation::new();
        assert!(matches!(
            RemoteClient::connect(RemoteConfig::new("http://192.0.2.1:1"), &cancel).await,
            Err(ClientError::Remote(RemoteError::PermissionDenied))
        ));
        cancel.cancel();
        assert!(matches!(
            RemoteClient::connect(RemoteConfig::new("http://127.0.0.1:1"), &cancel).await,
            Err(ClientError::Cancelled)
        ));
    });
}

#[test]
fn handoff_preflight_expiry_and_generation_are_not_consumer_rejections() {
    runtime().block_on(async {
        for expire in [false, true] {
            let budget = budget();
            let (hub, inbox, addr, stop, server) = start(
                ServerConfig {
                    enabled: true,
                    ..Default::default()
                },
                limits(),
                &budget,
            )
            .await;
            let cancel = Cancellation::new();
            let mut client =
                RemoteClient::connect(RemoteConfig::new(format!("http://{addr}")), &cancel)
                    .await
                    .unwrap();
            let mut stream = client
                .subscribe(
                    &budget,
                    SubscriptionFilter {
                        channel: None,
                        minimum_level: Level::Trace,
                        originals: false,
                        overflow: OverflowPolicy::Gap,
                    },
                    ReceiverConfig {
                        records: 1,
                        record_bytes: 8,
                        ttl: Duration::from_millis(300),
                    },
                    &cancel,
                )
                .await
                .unwrap();
            let mut ids = [0; 2];
            assert_eq!(hub.subscriptions(&mut ids), 1);
            let ack = hub.offer(ids[0], record(1, 1), &[1], false, 1).unwrap();
            let RemoteEvent::Record(ready) = stream.next(&cancel).await.unwrap().unwrap() else {
                panic!("record")
            };
            if expire {
                tokio::time::sleep(Duration::from_millis(350)).await;
            } else {
                stream.discard_generations(2).unwrap();
            }
            let mut called = false;
            assert_eq!(
                stream
                    .handoff(
                        ready,
                        |_, _| {
                            called = true;
                            true
                        },
                        &cancel
                    )
                    .await,
                Err(ClientError::Remote(RemoteError::Gone))
            );
            assert!(!called);
            assert_ne!(ack.state(), AckState::Acknowledged);
            drop((stream, client, inbox, ack));
            stop.send(()).unwrap();
            server.await.unwrap().unwrap();
            drop(hub);
            assert_eq!(budget.used_bytes(), 0);
        }
    });
}

#[test]
fn local_next_timeout_preserves_subscription_but_server_lease_and_cancel_close_it() {
    runtime().block_on(async {
        let budget = budget();
        let (hub, inbox, addr, stop, server) = start(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            limits(),
            &budget,
        )
        .await;
        let cancel = Cancellation::new();
        let mut config = RemoteConfig::new(format!("http://{addr}"));
        config.timeout = Duration::from_millis(150);
        let mut client = RemoteClient::connect(config, &cancel).await.unwrap();
        let filter = SubscriptionFilter {
            channel: None,
            minimum_level: Level::Trace,
            originals: false,
            overflow: OverflowPolicy::Gap,
        };
        let mut stream = client
            .subscribe(
                &budget,
                filter,
                ReceiverConfig {
                    records: 1,
                    record_bytes: 8,
                    ttl: Duration::from_secs(2),
                },
                &cancel,
            )
            .await
            .unwrap();
        for _ in 0..2 {
            assert!(matches!(
                stream.next(&cancel).await,
                Err(ClientError::Remote(RemoteError::TimedOut))
            ));
        }
        let mut ids = [0; 2];
        assert_eq!(hub.subscriptions(&mut ids), 1);
        let ack = hub.offer(ids[0], record(1, 1), &[1], false, 1).unwrap();
        let RemoteEvent::Record(ready) = stream.next(&cancel).await.unwrap().unwrap() else {
            panic!("record")
        };
        stream.handoff(ready, |_, _| true, &cancel).await.unwrap();
        assert_eq!(ack.state(), AckState::Acknowledged);
        drop(stream);
        let mut stream = client
            .subscribe(
                &budget,
                filter,
                ReceiverConfig {
                    records: 1,
                    record_bytes: 8,
                    ttl: Duration::from_millis(100),
                },
                &cancel,
            )
            .await
            .unwrap();
        assert!(matches!(
            stream.next(&cancel).await,
            Err(ClientError::Remote(RemoteError::TimedOut))
        ));
        assert!(stream.next(&cancel).await.unwrap().is_none());
        drop(stream);
        let mut stream = client
            .subscribe(
                &budget,
                filter,
                ReceiverConfig {
                    records: 1,
                    record_bytes: 8,
                    ttl: Duration::from_secs(2),
                },
                &cancel,
            )
            .await
            .unwrap();
        cancel.cancel();
        assert!(matches!(
            stream.next(&cancel).await,
            Err(ClientError::Cancelled)
        ));
        drop((stream, client, inbox, ack));
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn corrupt_and_oversized_host_responses_keep_their_failure_category() {
    runtime().block_on(async {
        let budget = budget();
        let token = "test-only-manage-credential";
        let (hub, mut inbox, addr, stop, server) = start(
            ServerConfig {
                enabled: true,
                tokens: vec![AccessToken::new(token.into(), Permission::Manage).unwrap()],
                ..Default::default()
            },
            limits(),
            &budget,
        )
        .await;
        let cancel = Cancellation::new();
        let mut config = RemoteConfig::new(format!("http://{addr}"));
        config.token = Some(token.into());
        config.message_bytes = 2048;
        let mut client = RemoteClient::connect(config, &cancel).await.unwrap();
        let host = tokio::spawn(async move {
            command(&mut inbox).await.respond(Err(RemoteError::Corrupt));
            command(&mut inbox)
                .await
                .respond(Ok(ManagementReply::Report(vec![0; 4096])));
        });
        assert!(matches!(
            client.request(ManagementRequest::GetStatus, &cancel).await,
            Err(ClientError::Remote(RemoteError::Corrupt))
        ));
        assert!(matches!(
            client
                .request(ManagementRequest::RunCheck { plan: 1 }, &cancel)
                .await,
            Err(ClientError::Remote(RemoteError::Limit))
        ));
        host.await.unwrap();
        drop(client);
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}

#[test]
fn next_timeout_during_partial_tcp_frame_retains_decoder_and_record() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    runtime().block_on(async {
        let budget = budget();
        let (hub, inbox, addr, stop, server) = start(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            limits(),
            &budget,
        )
        .await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = listener.local_addr().unwrap();
        let pause = Arc::new(AtomicBool::new(false));
        let pause_copy = pause.clone();
        let proxy = tokio::spawn(async move {
            let (downstream, _) = listener.accept().await.unwrap();
            let upstream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (mut down_read, mut down_write) = downstream.into_split();
            let (mut up_read, mut up_write) = upstream.into_split();
            let upload = tokio::spawn(async move {
                let _ = tokio::io::copy(&mut down_read, &mut up_write).await;
            });
            let mut bytes = [0; 1024];
            loop {
                let n = up_read.read(&mut bytes).await.unwrap();
                if n == 0 {
                    break;
                }
                if pause_copy.swap(false, Ordering::AcqRel) {
                    down_write.write_all(&bytes[..1]).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    down_write.write_all(&bytes[1..n]).await.unwrap();
                } else {
                    down_write.write_all(&bytes[..n]).await.unwrap();
                }
            }
            upload.abort();
            let _ = upload.await;
        });
        let cancel = Cancellation::new();
        let mut config = RemoteConfig::new(format!("http://{proxy_addr}"));
        config.timeout = Duration::from_millis(150);
        let mut client = RemoteClient::connect(config, &cancel).await.unwrap();
        let mut stream = client
            .subscribe(
                &budget,
                SubscriptionFilter {
                    channel: None,
                    minimum_level: Level::Trace,
                    originals: false,
                    overflow: OverflowPolicy::Gap,
                },
                ReceiverConfig {
                    records: 1,
                    record_bytes: 8192,
                    ttl: Duration::from_secs(2),
                },
                &cancel,
            )
            .await
            .unwrap();
        let mut ids = [0; 2];
        assert_eq!(hub.subscriptions(&mut ids), 1);
        pause.store(true, Ordering::Release);
        let payload = vec![7; 8192];
        let ack = hub
            .offer(ids[0], record(1, payload.len()), &payload, false, 1)
            .unwrap();
        assert!(matches!(
            stream.next(&cancel).await,
            Err(ClientError::Remote(RemoteError::TimedOut))
        ));
        let RemoteEvent::Record(ready) = stream.next(&cancel).await.unwrap().unwrap() else {
            panic!("record")
        };
        stream
            .handoff(ready, |_, data| data == payload, &cancel)
            .await
            .unwrap();
        assert_eq!(ack.state(), AckState::Acknowledged);
        drop((stream, client, ack, inbox));
        proxy.abort();
        let _ = proxy.await;
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}

async fn start(
    config: ServerConfig,
    limits: RemoteLimits,
    budget: &QueryBudget,
) -> (
    RemoteHub,
    CommandInbox,
    std::net::SocketAddr,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), RemoteError>>,
) {
    let (hub, inbox) = RemoteHub::new(budget, limits).unwrap();
    let server = RemoteServer::bind(config, hub.clone()).await.unwrap();
    let address = server.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve(async {
        let _ = rx.await;
    }));
    (hub, inbox, address, tx, task)
}
async fn command(inbox: &mut CommandInbox) -> dot_logger::adapters::remote::Command {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(c) = inbox.try_next() {
                return c;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}
#[test]
fn completion_claim_without_requested_boundary_is_rejected() {
    runtime().block_on(async {
        use dot_logger::completion::*;
        let budget = budget();
        let (hub, mut inbox, addr, stop, server) = start(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            limits(),
            &budget,
        )
        .await;
        let cancel = Cancellation::new();
        let mut client =
            RemoteClient::connect(RemoteConfig::new(format!("http://{addr}")), &cancel)
                .await
                .unwrap();
        let host = tokio::spawn(async move {
            command(&mut inbox)
                .await
                .respond(Ok(ManagementReply::Completion(RecordStatus {
                    confirmed: Boundaries::only(Boundary::Written),
                    outcome: QueryOutcome::Complete,
                })));
        });
        let result = client
            .request(
                ManagementRequest::GetCompletion {
                    receipt: record(1, 1).receipt,
                    target: 1,
                    boundary: Boundary::ReceiverAcknowledged,
                },
                &cancel,
            )
            .await;
        assert!(matches!(
            result,
            Err(ClientError::Remote(RemoteError::Corrupt))
        ));
        host.await.unwrap();
        drop(client);
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn stale_replay_generation_reply_is_rejected() {
    runtime().block_on(async {
        let budget = budget();
        let (hub, mut inbox, addr, stop, server) = start(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            limits(),
            &budget,
        )
        .await;
        let cancel = Cancellation::new();
        let mut client =
            RemoteClient::connect(RemoteConfig::new(format!("http://{addr}")), &cancel)
                .await
                .unwrap();
        let host = tokio::spawn(async move {
            command(&mut inbox)
                .await
                .respond(Ok(ManagementReply::Replay {
                    generation: 1,
                    state: dot_logger::replay::ReplayState::Paused,
                }));
        });
        let result = client
            .request(
                ManagementRequest::Replay {
                    handle: 9,
                    action: ReplayAction::Pause,
                    generation: 9,
                    from: 0,
                    speed: dot_logger::replay::Speed::REALTIME,
                },
                &cancel,
            )
            .await;
        assert!(matches!(
            result,
            Err(ClientError::Remote(RemoteError::Corrupt))
        ));
        host.await.unwrap();
        drop(client);
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn tls_read_manage_permissions_and_fixed_errors_do_not_expose_credentials() {
    runtime().block_on(async {
        const CERT: &[u8] =
            include_bytes!("../../dot-logger/tests/fixtures/remote/test-only-cert.pem");
        const KEY: &[u8] =
            include_bytes!("../../dot-logger/tests/fixtures/remote/test-only-key.pem");
        let budget = budget();
        let mut l = limits();
        l.connections = 4;
        let config = ServerConfig {
            enabled: true,
            address: "0.0.0.0:0".parse().unwrap(),
            tokens: vec![
                AccessToken::new("test-only-read-credential".into(), Permission::Read).unwrap(),
                AccessToken::new("test-only-manage-credential".into(), Permission::Manage).unwrap(),
            ],
            tls: Some(TlsIdentity::new(CERT.to_vec(), KEY.to_vec()).unwrap()),
            ..Default::default()
        };
        let (hub, mut inbox, addr, stop, server) = start(config, l, &budget).await;
        let cancel = Cancellation::new();
        for token in ["test-only-bad-credential", "test-only-read-credential"] {
            let mut cfg = RemoteConfig::new(format!("https://127.0.0.1:{}", addr.port()));
            cfg.ca_certificate = Some(CERT.to_vec());
            cfg.tls_domain = Some("localhost".into());
            cfg.token = Some(token.into());
            let mut client = RemoteClient::connect(cfg, &cancel).await.unwrap();
            let result = client
                .request(ManagementRequest::RunCheck { plan: 1 }, &cancel)
                .await;
            let error = result.unwrap_err();
            assert_eq!(error, ClientError::Remote(RemoteError::PermissionDenied));
            assert!(!error.to_string().contains(token));
            assert!(inbox.try_next().is_none());
        }
        let mut cfg = RemoteConfig::new(format!("https://127.0.0.1:{}", addr.port()));
        cfg.ca_certificate = Some(CERT.to_vec());
        cfg.tls_domain = Some("localhost".into());
        cfg.token = Some("test-only-manage-credential".into());
        let mut client = RemoteClient::connect(cfg, &cancel).await.unwrap();
        let host = tokio::spawn(async move {
            let c = command(&mut inbox).await;
            assert!(matches!(
                c.request(),
                ManagementRequest::RunCheck { plan: 1 }
            ));
            c.respond(Err(RemoteError::Unsupported));
        });
        assert!(matches!(
            client
                .request(ManagementRequest::RunCheck { plan: 1 }, &cancel)
                .await,
            Err(ClientError::Remote(RemoteError::Unsupported))
        ));
        host.await.unwrap();
        drop(client);
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn cancellation_keeps_actual_host_command_capacity_until_its_owner_finishes() {
    runtime().block_on(async {
        let budget = budget();
        let mut l = limits();
        l.commands = 1;
        let (hub, mut inbox, addr, stop, server) = start(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            l,
            &budget,
        )
        .await;
        let cancel = Cancellation::new();
        let token = cancel.clone();
        let mut client =
            RemoteClient::connect(RemoteConfig::new(format!("http://{addr}")), &cancel)
                .await
                .unwrap();
        let wait =
            tokio::spawn(async move { client.request(ManagementRequest::GetStatus, &token).await });
        let retained = command(&mut inbox).await;
        cancel.cancel();
        assert!(matches!(wait.await.unwrap(), Err(ClientError::Cancelled)));
        tokio::time::timeout(Duration::from_secs(2), async {
            while !retained.cancelled() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let fresh = Cancellation::new();
        let mut client = RemoteClient::connect(RemoteConfig::new(format!("http://{addr}")), &fresh)
            .await
            .unwrap();
        assert!(matches!(
            client.request(ManagementRequest::GetStatus, &fresh).await,
            Err(ClientError::Remote(RemoteError::Limit))
        ));
        retained.respond(Err(RemoteError::Unsupported));
        let host = tokio::spawn(async move {
            command(&mut inbox)
                .await
                .respond(Ok(ManagementReply::Status {
                    session: 42,
                    revision: 1,
                }));
        });
        assert!(
            client
                .request(ManagementRequest::GetStatus, &fresh)
                .await
                .is_ok()
        );
        host.await.unwrap();
        drop(client);
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}
#[test]
fn gap_consumer_rejection_and_cancel_after_handoff_never_claim_ack_success() {
    runtime().block_on(async {
        let budget = budget();
        let (hub, inbox, addr, stop, server) = start(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            limits(),
            &budget,
        )
        .await;
        let cancel = Cancellation::new();
        let mut client =
            RemoteClient::connect(RemoteConfig::new(format!("http://{addr}")), &cancel)
                .await
                .unwrap();
        let filter = SubscriptionFilter {
            channel: None,
            minimum_level: Level::Trace,
            originals: false,
            overflow: OverflowPolicy::Gap,
        };
        let mut stream = client
            .subscribe(
                &budget,
                filter,
                ReceiverConfig {
                    records: 1,
                    record_bytes: 8,
                    ttl: Duration::from_secs(10),
                },
                &cancel,
            )
            .await
            .unwrap();
        let mut ids = [0; 2];
        assert_eq!(hub.subscriptions(&mut ids), 1);
        let first = hub.offer(ids[0], record(1, 1), &[1], false, 1).unwrap();
        let discarded = hub.offer(ids[0], record(2, 1), &[2], false, 1).unwrap();
        assert_eq!(discarded.state(), AckState::Dropped);
        let RemoteEvent::Record(ready) = stream.next(&cancel).await.unwrap().unwrap() else {
            panic!("record")
        };
        assert_eq!(
            stream.handoff(ready, |_, _| false, &cancel).await,
            Err(ClientError::ConsumerRejected)
        );
        assert!(matches!(
            stream.next(&cancel).await.unwrap(),
            Some(RemoteEvent::Gap { first: 2, last: 2 })
        ));
        assert_eq!(first.state(), AckState::Pending);
        drop(stream);
        tokio::time::timeout(Duration::from_secs(2), async {
            while first.state() == AckState::Pending {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(first.state(), AckState::Unknown);
        let mut stream = client
            .subscribe(
                &budget,
                filter,
                ReceiverConfig {
                    records: 1,
                    record_bytes: 8,
                    ttl: Duration::from_secs(10),
                },
                &cancel,
            )
            .await
            .unwrap();
        assert_eq!(hub.subscriptions(&mut ids), 1);
        let pending = hub.offer(ids[0], record(3, 1), &[3], false, 1).unwrap();
        let RemoteEvent::Record(ready) = stream.next(&cancel).await.unwrap().unwrap() else {
            panic!("record")
        };
        assert_eq!(
            stream
                .handoff(
                    ready,
                    |_, _| {
                        cancel.cancel();
                        true
                    },
                    &cancel
                )
                .await,
            Err(ClientError::AcknowledgementUnknown)
        );
        drop(stream);
        tokio::time::timeout(Duration::from_secs(2), async {
            while pending.state() == AckState::Pending {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(pending.state(), AckState::Unknown);
        drop((first, discarded, pending, inbox, client));
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        drop(hub);
        assert_eq!(budget.used_bytes(), 0);
    });
}
