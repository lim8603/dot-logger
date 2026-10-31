#![allow(clippy::unwrap_used)]
use super::hub::Event;
use super::*;
use crate::{capture::*, config::*, platform::*, query::QueryBudget};
use std::{
    task::Waker,
    time::{Duration, Instant},
};
fn limits() -> RemoteLimits {
    RemoteLimits {
        connections: 4,
        subscriptions: 4,
        commands: 4,
        message_bytes: MAX_CHUNK_BYTES + 1024,
        record_bytes: 8 * 1024 * 1024,
        subscription_records: 2,
        subscription_bytes: 8 * 1024 * 1024,
        max_age: Duration::from_secs(10),
        request_timeout: Duration::from_secs(1),
    }
}
fn plan() -> ValidatedConfig {
    SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 260 * 1024 * 1024,
            input_bytes: 1024 * 1024,
            raw_bytes: 0,
            output_bytes: 1024 * 1024,
            external_bytes: 256 * 1024 * 1024,
            control_bytes: 1024 * 1024,
            worker_bytes: 1024 * 1024,
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
    .unwrap()
}
fn budget() -> QueryBudget {
    QueryBudget::new(&plan())
}
fn record(seq: u64, len: usize) -> CapturedRecord {
    CapturedRecord {
        receipt: CaptureReceipt {
            session_id: 42,
            producer_id: 0,
            accepted_seq: seq,
        },
        metadata: RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: seq,
            real_ns: seq as i64,
            source_time: None,
            level: Level::Info,
            importance: Some(Importance::Normal),
        },
        revision: 1,
        payload_len: len,
    }
}
fn filter(overflow: OverflowPolicy) -> SubscriptionFilter {
    SubscriptionFilter {
        channel: None,
        minimum_level: Level::Trace,
        originals: true,
        overflow,
    }
}
fn waker() -> Waker {
    Waker::noop().clone()
}

#[test]
fn bounded_gap_ack_generation_and_filtering_are_explicit() {
    let b = budget();
    let (hub, inbox) = RemoteHub::new(&b, limits()).unwrap();
    let peer = "127.0.0.1:1".parse().unwrap();
    let sub = hub
        .subscribe(
            peer,
            filter(OverflowPolicy::Gap),
            2,
            8192,
            Duration::from_secs(2),
        )
        .unwrap();
    let first = hub
        .offer(sub.id, record(1, 3), &[1, 2, 3], false, 1)
        .unwrap();
    let second = hub
        .offer(sub.id, record(2, 3), &[4, 5, 6], false, 1)
        .unwrap();
    let dropped = hub
        .offer(sub.id, record(3, 3), &[7, 8, 9], false, 1)
        .unwrap();
    assert_eq!(dropped.state(), AckState::Dropped);
    let early = Ack {
        subscription: sub.id,
        transfer: 1,
        generation: 1,
        receipt: record(1, 3).receipt,
    };
    assert_eq!(hub.ack(early, peer), Err(RemoteError::InvalidInput));
    let Event::Chunk(c) = sub.next(&waker()).unwrap().unwrap() else {
        panic!("chunk")
    };
    assert_eq!(c.transfer, 1);
    assert_eq!(first.state(), AckState::Pending);
    assert_eq!(
        hub.ack(early, "127.0.0.1:2".parse().unwrap()),
        Err(RemoteError::PermissionDenied)
    );
    hub.ack(early, peer).unwrap();
    assert_eq!(first.state(), AckState::Acknowledged);
    assert!(matches!(sub.next(&waker()).unwrap(), Some(Event::Chunk(_))));
    assert!(matches!(
        sub.next(&waker()).unwrap(),
        Some(Event::Gap {
            first: 3,
            last: 3,
            ..
        })
    ));
    hub.discard_generations(sub.id, 2).unwrap();
    assert_eq!(second.state(), AckState::Unknown);
    let mut filtered = filter(OverflowPolicy::Gap);
    filtered.originals = false;
    let other = hub
        .subscribe(peer, filtered, 1, 8192, Duration::from_secs(2))
        .unwrap();
    assert!(matches!(
        hub.offer(other.id, record(4, 3), &[1, 2, 3], true, 0),
        Err(RemoteError::Unsupported)
    ));
    drop((sub, other, first, second, dropped, hub, inbox));
    assert_eq!(b.used_bytes(), 0);
}
#[test]
fn disconnect_overflow_is_local_and_fragmented_records_need_final_ack() {
    let b = budget();
    let (hub, _inbox) = RemoteHub::new(&b, limits()).unwrap();
    let peer = "127.0.0.1:3".parse().unwrap();
    let slow = hub
        .subscribe(
            peer,
            filter(OverflowPolicy::Disconnect),
            1,
            8192,
            Duration::from_secs(2),
        )
        .unwrap();
    let fast = hub
        .subscribe(
            peer,
            filter(OverflowPolicy::Gap),
            1,
            8192,
            Duration::from_secs(2),
        )
        .unwrap();
    let a = hub
        .offer(slow.id, record(1, 3), &[1, 2, 3], false, 0)
        .unwrap();
    let _ = hub
        .offer(slow.id, record(2, 3), &[4, 5, 6], false, 0)
        .unwrap();
    assert_eq!(a.state(), AckState::Unknown);
    assert!(slow.next(&waker()).is_err());
    assert!(
        hub.offer(fast.id, record(3, 3), &[7, 8, 9], false, 0)
            .is_ok()
    );
    assert!(matches!(fast.next(&waker()), Ok(Some(Event::Chunk(_)))));
    let large = hub
        .subscribe(
            peer,
            filter(OverflowPolicy::Gap),
            1,
            8 * 1024 * 1024,
            Duration::from_secs(2),
        )
        .unwrap();
    let payload = vec![7; MAX_CHUNK_BYTES + 1];
    let ack = hub
        .offer(large.id, record(4, payload.len()), &payload, true, 0)
        .unwrap();
    let Event::Chunk(c) = large.next(&waker()).unwrap().unwrap() else {
        panic!("chunk")
    };
    assert_eq!(c.data.len(), MAX_CHUNK_BYTES);
    let receipt = Ack {
        subscription: large.id,
        transfer: c.transfer,
        generation: 0,
        receipt: c.record.receipt,
    };
    assert_eq!(hub.ack(receipt, peer), Err(RemoteError::InvalidInput));
    assert_eq!(ack.state(), AckState::Pending);
    let Event::Chunk(c) = large.next(&waker()).unwrap().unwrap() else {
        panic!("chunk")
    };
    assert_eq!(c.offset, MAX_CHUNK_BYTES as u64);
    assert_eq!(c.data.len(), 1);
    hub.ack(receipt, peer).unwrap();
    assert_eq!(ack.state(), AckState::Acknowledged);
}
#[test]
fn receiver_identity_ttl_and_shared_reservation_survive_owner_drop() {
    let b = budget();
    let now = Instant::now();
    let mut rx = Reassembler::new(&b, 1, 8, Duration::from_secs(1)).unwrap();
    let mut a = Chunk::single(1, 1, 1, record(1, 4), &[1, 2], true);
    a.total = 4;
    a.checksum = super::types::checksum(&[1, 2, 3, 4]);
    assert!(rx.push(a.clone(), now).unwrap().is_none());
    let mut bad = a.clone();
    bad.offset = 2;
    bad.data = vec![3, 4];
    bad.record.revision = 2;
    assert!(matches!(rx.push(bad, now), Err(RemoteError::Corrupt)));
    assert!(rx.push(a.clone(), now).unwrap().is_none());
    rx.expire(now + Duration::from_secs(1));
    let mut tail = a;
    tail.offset = 2;
    tail.data = vec![3, 4];
    assert!(rx.push(tail, now).is_err());
    let ready = rx
        .push(
            Chunk::single(1, 2, 2, record(2, 1), &[9], true),
            now + Duration::from_secs(1),
        )
        .unwrap()
        .unwrap();
    drop(rx);
    assert!(b.used_bytes() > 0);
    drop(ready);
    assert_eq!(b.used_bytes(), 0);
}

#[test]
fn receiver_generation_change_rejects_late_old_network_fragments() {
    let b = budget();
    let mut rx = Reassembler::new(&b, 1, 8, Duration::from_secs(1)).unwrap();
    let old = rx
        .push(
            Chunk::single(1, 9, 1, record(9, 1), &[9], true),
            Instant::now(),
        )
        .unwrap()
        .unwrap();
    rx.discard_generations(1, 2).unwrap();
    assert!(
        old.handoff(|_, _| panic!("stale handoff must not run"))
            .is_err()
    );
    assert_eq!(rx.discard_generations(1, 1), Err(RemoteError::Gone));
    assert!(matches!(
        rx.push(
            Chunk::single(1, 1, 1, record(1, 1), &[1], true),
            Instant::now()
        ),
        Err(RemoteError::Gone)
    ));
    assert!(
        rx.push(
            Chunk::single(1, 2, 2, record(2, 1), &[2], true),
            Instant::now()
        )
        .unwrap()
        .is_some()
    );
}

#[test]
fn receiver_gate_limit_preserves_stale_rejection_and_connection_reservations() {
    let b = budget();
    let mut old = Reassembler::new(&b, 1, 8, Duration::from_secs(1)).unwrap();
    let ready = old
        .push(
            Chunk::single(1, 1, 1, record(1, 1), &[1], true),
            Instant::now(),
        )
        .unwrap()
        .unwrap();
    old.discard_generations(1, 2).unwrap();
    assert_eq!(old.discard_generations(2, 1), Err(RemoteError::Limit));
    old.discard_generations(1, 3).unwrap();
    assert!(
        ready
            .handoff(|_, _| panic!("old handoff must stay rejected"))
            .is_err()
    );
    let mut fresh = Reassembler::new(&b, 1, 8, Duration::from_secs(1)).unwrap();
    fresh.discard_generations(2, 1).unwrap();
    let both = b.used_bytes();
    drop(old);
    assert!(b.used_bytes() > 0 && b.used_bytes() < both);
    drop(fresh);
    assert_eq!(b.used_bytes(), 0);
}

#[cfg(not(miri))]
mod sockets {
    use super::*;
    use crate::completion::*;
    use std::sync::atomic::AtomicBool;
    use tonic::{
        Request,
        client::Grpc,
        codegen::http::uri::PathAndQuery,
        transport::{Channel, Endpoint},
    };
    type Task = tokio::task::JoinHandle<Result<(), RemoteError>>;
    async fn start(
        config: ServerConfig,
        l: RemoteLimits,
    ) -> (
        RemoteHub,
        CommandInbox,
        std::net::SocketAddr,
        tokio::sync::oneshot::Sender<()>,
        Task,
        QueryBudget,
    ) {
        let b = budget();
        let (hub, inbox) = RemoteHub::new(&b, l).unwrap();
        let server = RemoteServer::bind(config, hub.clone()).await.unwrap();
        let address = server.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(server.serve(async {
            let _ = rx.await;
        }));
        (hub, inbox, address, tx, task, b)
    }
    fn config() -> ServerConfig {
        ServerConfig {
            enabled: true,
            ..ServerConfig::default()
        }
    }
    async fn client(address: std::net::SocketAddr) -> Grpc<Channel> {
        Grpc::new(
            Endpoint::from_shared(format!("http://{address}"))
                .unwrap()
                .connect()
                .await
                .unwrap(),
        )
    }
    fn req() -> wire::Request {
        wire::Request {
            major: PROTOCOL_MAJOR,
            ..wire::Request::default()
        }
    }
    async fn unary(
        c: &mut Grpc<Channel>,
        method: &'static str,
        r: wire::Request,
    ) -> Result<wire::Reply, tonic::Status> {
        c.ready().await.unwrap();
        c.unary(
            Request::new(r),
            PathAndQuery::from_static(method),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .map(tonic::Response::into_inner)
    }
    async fn subscribe(c: &mut Grpc<Channel>) -> tonic::Streaming<wire::Frame> {
        c.ready().await.unwrap();
        c.server_streaming(
            Request::new(wire::SubscribeRequest {
                major: 1,
                minimum_level: 0,
                originals: true,
                records: 2,
                bytes: 8 * 1024 * 1024,
                age_ms: 5000,
                ..wire::SubscribeRequest::default()
            }),
            PathAndQuery::from_static("/dot_logger.remote.v1.Logger/Subscribe"),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .unwrap()
        .into_inner()
    }
    async fn stop(tx: tokio::sync::oneshot::Sender<()>, task: Task) {
        let _ = tx.send(());
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
    }
    #[test]
    fn actual_grpc_large_record_handoff_ack_and_disconnect_unknown() {
        runtime().block_on(async {
            let (hub, inbox, addr, tx, task, b) = start(config(), limits()).await;
            let mut c = client(addr).await;
            let mut stream = subscribe(&mut c).await;
            let started = stream.message().await.unwrap().unwrap();
            let payload = vec![17; 8 * 1024 * 1024];
            let pending = hub
                .offer(
                    started.subscription,
                    record(1, payload.len()),
                    &payload,
                    true,
                    3,
                )
                .unwrap();
            let mut rx = Reassembler::new(&b, 1, payload.len(), Duration::from_secs(10)).unwrap();
            let mut chunks = 0;
            let ready = loop {
                let f = stream.message().await.unwrap().unwrap();
                assert!(f.data.len() <= MAX_CHUNK_BYTES);
                chunks += 1;
                if let Some(r) = rx.push(f.native().unwrap(), Instant::now()).unwrap() {
                    break r;
                }
            };
            assert_eq!(chunks, 32);
            assert_eq!(pending.state(), AckState::Pending);
            let ack = ready
                .handoff(|r, b| r.receipt == record(1, payload.len()).receipt && b == payload)
                .ok()
                .unwrap();
            c.ready().await.unwrap();
            let _: tonic::Response<wire::Reply> = c
                .unary(
                    Request::new(wire::AckRequest {
                        major: 1,
                        subscription: ack.subscription,
                        transfer: ack.transfer,
                        generation: ack.generation,
                        session: ack.receipt.session_id.to_le_bytes().to_vec(),
                        producer: ack.receipt.producer_id,
                        sequence: ack.receipt.accepted_seq,
                    }),
                    PathAndQuery::from_static("/dot_logger.remote.v1.Logger/Ack"),
                    tonic::codec::ProstCodec::default(),
                )
                .await
                .unwrap();
            assert_eq!(pending.state(), AckState::Acknowledged);
            let unknown = hub
                .offer(started.subscription, record(2, 1), &[7], false, 3)
                .unwrap();
            let second_unknown = hub
                .offer(started.subscription, record(3, 1), &[8], false, 3)
                .unwrap();
            let dropped = hub
                .offer(started.subscription, record(4, 1), &[9], false, 3)
                .unwrap();
            assert_eq!(dropped.state(), AckState::Dropped);
            assert_eq!(stream.message().await.unwrap().unwrap().kind, 1);
            assert_eq!(stream.message().await.unwrap().unwrap().kind, 1);
            let gap = stream.message().await.unwrap().unwrap();
            assert_eq!(
                (gap.kind, gap.gap_first, gap.gap_last, gap.gap_reason),
                (2, 4, 4, 1)
            );
            drop(stream);
            drop(c);
            for _ in 0..100 {
                if unknown.state() == AckState::Unknown {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert_eq!(unknown.state(), AckState::Unknown);
            stop(tx, task).await;
            drop((hub, inbox, pending, unknown, second_unknown, dropped, rx));
            assert_eq!(b.used_bytes(), 0);
        });
    }
    #[test]
    fn version_permissions_response_cap_and_host_timeout_do_not_fabricate_results() {
        runtime().block_on(async {
            let mut l = limits();
            l.commands = 1;
            l.request_timeout = Duration::from_millis(30);
            let (hub, mut inbox, addr, tx, task, _) = start(config(), l).await;
            let mut c = client(addr).await;
            let mut bad = req();
            bad.major = 2;
            assert_eq!(
                unary(&mut c, "/dot_logger.remote.v1.Logger/GetStatus", bad)
                    .await
                    .unwrap_err()
                    .code(),
                tonic::Code::FailedPrecondition
            );
            assert_eq!(
                unary(&mut c, "/dot_logger.remote.v1.Logger/ApplyConfig", req())
                    .await
                    .unwrap_err()
                    .code(),
                tonic::Code::PermissionDenied
            );
            assert!(inbox.try_next().is_none());
            c.ready().await.unwrap();
            let mut malformed = Request::new(req());
            malformed
                .metadata_mut()
                .insert("authorization", "Basic malformed".parse().unwrap());
            let result: Result<tonic::Response<wire::Reply>, _> = c
                .unary(
                    malformed,
                    PathAndQuery::from_static("/dot_logger.remote.v1.Logger/GetStatus"),
                    tonic::codec::ProstCodec::default(),
                )
                .await;
            assert_eq!(result.unwrap_err().code(), tonic::Code::PermissionDenied);
            let future = tokio::spawn(async move {
                let result = unary(&mut c, "/dot_logger.remote.v1.Logger/GetStatus", req()).await;
                (c, result)
            });
            let command = next_command(&mut inbox).await;
            let (mut c, result) = future.await.unwrap();
            assert_eq!(result.unwrap_err().code(), tonic::Code::DeadlineExceeded);
            assert!(command.cancelled());
            assert_eq!(
                unary(&mut c, "/dot_logger.remote.v1.Logger/GetStatus", req())
                    .await
                    .unwrap_err()
                    .code(),
                tonic::Code::ResourceExhausted
            );
            command.respond(Ok(ManagementReply::Status {
                session: 42,
                revision: 1,
            }));
            let future = tokio::spawn(async move {
                let result = unary(&mut c, "/dot_logger.remote.v1.Logger/GetStatus", req()).await;
                (c, result)
            });
            let cmd = next_command(&mut inbox).await;
            assert!(matches!(cmd.request(), ManagementRequest::GetStatus));
            cmd.respond(Ok(ManagementReply::Status {
                session: 42,
                revision: 2,
            }));
            let (c, result) = future.await.unwrap();
            assert_eq!(result.unwrap().revision, 2);
            drop(c);
            stop(tx, task).await;
            drop(hub);
        });
    }
    #[test]
    fn wire_response_limits_and_typed_commands_are_enforced() {
        let mut spare = Vec::with_capacity(4097);
        spare.push(1);
        assert_eq!(
            wire::Reply::native(ManagementReply::Blob(spare), "ReadBlob", 4096, 1000),
            Err(RemoteError::Limit)
        );
        assert_eq!(
            wire::Reply::native(ManagementReply::Blob(vec![1; 4096]), "ReadBlob", 4096, 1000),
            Err(RemoteError::Limit)
        );
        assert!(
            wire::Reply::native(
                ManagementReply::Status {
                    session: 42,
                    revision: 1
                },
                "Query",
                4096,
                1000
            )
            .is_err()
        );
        let mut r = req();
        r.query = Some(wire::QueryMessage {
            basis: 0,
            from: 0i128.to_le_bytes().to_vec(),
            to: 10i128.to_le_bytes().to_vec(),
            ..wire::QueryMessage::default()
        });
        assert!(matches!(
            r.native("Query", 4096),
            Ok(ManagementRequest::Query { .. })
        ));
    }
    async fn next_command(inbox: &mut CommandInbox) -> Command {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(cmd) = inbox.try_next() {
                    return cmd;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    }
    #[test]
    fn tls_external_listener_enforces_read_manage_and_secret_redaction() {
        runtime().block_on(async {
            const CERT: &[u8] = include_bytes!("../../../tests/fixtures/remote/test-only-cert.pem");
            const KEY: &[u8] = include_bytes!("../../../tests/fixtures/remote/test-only-key.pem");
            let mut cfg = config();
            cfg.address = "0.0.0.0:0".parse().unwrap();
            cfg.tls = Some(TlsIdentity::new(CERT.to_vec(), KEY.to_vec()).unwrap());
            cfg.tokens = vec![
                AccessToken::new("test-only-read-credential".into(), Permission::Read).unwrap(),
                AccessToken::new("test-only-manage-credential".into(), Permission::Manage).unwrap(),
            ];
            let (hub, mut inbox, addr, tx, task, _) = start(cfg, limits()).await;
            let endpoint = Endpoint::from_shared(format!("https://127.0.0.1:{}", addr.port()))
                .unwrap()
                .tls_config(
                    tonic::transport::ClientTlsConfig::new()
                        .ca_certificate(tonic::transport::Certificate::from_pem(CERT))
                        .domain_name("localhost"),
                )
                .unwrap();
            let mut c = Grpc::new(endpoint.connect().await.unwrap());
            assert_eq!(
                unary(&mut c, "/dot_logger.remote.v1.Logger/GetStatus", req())
                    .await
                    .unwrap_err()
                    .code(),
                tonic::Code::PermissionDenied
            );
            let mut request = Request::new(req());
            request.metadata_mut().insert(
                "authorization",
                "Bearer test-only-read-credential".parse().unwrap(),
            );
            c.ready().await.unwrap();
            let result: Result<tonic::Response<wire::Reply>, _> = c
                .unary(
                    request,
                    PathAndQuery::from_static("/dot_logger.remote.v1.Logger/RunCheck"),
                    tonic::codec::ProstCodec::default(),
                )
                .await;
            let error = result.unwrap_err();
            assert_eq!(error.code(), tonic::Code::PermissionDenied);
            assert!(!error.to_string().contains("test-only-read-credential"));
            assert!(inbox.try_next().is_none());
            let mut request = Request::new(req());
            request.metadata_mut().insert(
                "authorization",
                "Bearer test-only-manage-credential".parse().unwrap(),
            );
            let future = tokio::spawn(async move {
                c.ready().await.unwrap();
                let result: Result<tonic::Response<wire::Reply>, _> = c
                    .unary(
                        request,
                        PathAndQuery::from_static("/dot_logger.remote.v1.Logger/RunCheck"),
                        tonic::codec::ProstCodec::default(),
                    )
                    .await;
                (c, result)
            });
            let cmd = next_command(&mut inbox).await;
            assert!(matches!(
                cmd.request(),
                ManagementRequest::RunCheck { plan: 0 }
            ));
            cmd.respond(Err(RemoteError::Unsupported));
            let (c, result) = future.await.unwrap();
            assert_eq!(result.unwrap_err().code(), tonic::Code::Unimplemented);
            drop(c);
            stop(tx, task).await;
            drop(hub);
        });
    }

    #[test]
    fn blob_reply_cannot_exceed_requested_range() {
        runtime().block_on(async {
            let (hub, mut inbox, addr, tx, task, _) = start(config(), limits()).await;
            let mut c = client(addr).await;
            let mut request = req();
            request.length = 1;
            let future = tokio::spawn(async move {
                let result = unary(&mut c, "/dot_logger.remote.v1.Logger/ReadBlob", request).await;
                (c, result)
            });
            let cmd = next_command(&mut inbox).await;
            assert!(matches!(
                cmd.request(),
                ManagementRequest::ReadBlob { length: 1, .. }
            ));
            cmd.respond(Ok(ManagementReply::Blob(vec![1, 2])));
            let (c, result) = future.await.unwrap();
            assert_eq!(result.unwrap_err().code(), tonic::Code::ResourceExhausted);
            drop(c);
            stop(tx, task).await;
            drop(hub);
        });
    }

    async fn hosted_call(
        c: Grpc<Channel>,
        method: &'static str,
        r: wire::Request,
        inbox: &mut CommandInbox,
        handler: impl FnOnce(&ManagementRequest) -> Result<ManagementReply, RemoteError>,
    ) -> (Grpc<Channel>, wire::Reply) {
        let future = tokio::spawn(async move {
            let mut c = c;
            let reply = unary(&mut c, method, r).await;
            (c, reply)
        });
        let cmd = next_command(inbox).await;
        let result = handler(cmd.request());
        cmd.respond(result);
        let (c, result) = future.await.unwrap();
        (c, result.unwrap())
    }
    #[test]
    fn host_bridge_queries_real_file_replays_and_preserves_completion() {
        use crate::{query::*, replay::*, routing::*, storage::*};
        for acknowledge in [false, true] {
            let p = plan();
            let routing = RoutingConfig {
                targets: vec![10, 20]
                    .into_iter()
                    .map(|id| TargetConfig {
                        id,
                        required: true,
                        queue_slots: 2,
                        pending_slots: 1,
                        retained_bytes: 1024,
                        max_attempts: 1,
                        max_age: Duration::from_secs(10),
                    })
                    .collect(),
                routes: vec![Route {
                    channel_id: 7,
                    targets: vec![10, 20],
                }],
                shared_records: 4,
                small_copy_bytes: 1024,
                schedule: Schedule {
                    record_quantum: 2,
                    byte_quantum: 1024,
                    records_per_turn: 2,
                    bytes_per_turn: 2048,
                    retries_per_target: 1,
                    reclaim_slots_per_producer: 1,
                },
            };
            let tracking = CompletionConfig {
                history_retention: Duration::from_secs(60),
                window_records: 8,
                max_queries: 2,
                page_items: 4,
                feedback_events: 4,
                updates_per_turn: 4,
                reconcile_per_turn: 4,
                targets: vec![
                    TargetContract {
                        id: 10,
                        required: true,
                        supported: Boundaries::FILE,
                        completion: Boundary::Written,
                    },
                    TargetContract {
                        id: 20,
                        required: true,
                        supported: Boundaries::REMOTE,
                        completion: Boundary::ReceiverAcknowledged,
                    },
                ],
            };
            let mut owner = CaptureOwner::new_tracked(
                &p,
                42,
                1,
                &[ChannelConfig::new(7, 99, 1)],
                &routing,
                tracking,
            )
            .unwrap();
            owner.scope(|mut scope| {
                let completion = scope.completion().unwrap();
                let (mut producer, consumer) = scope.next().unwrap();
                let receipt = match producer
                    .try_record_small(record(1, 3).metadata, &[1, 2, 3])
                    .unwrap()
                {
                    RecordOutcome::Accepted(r) => r,
                    _ => panic!("accepted"),
                };
                let mut router = Router::new(&p, vec![consumer], routing).unwrap();
                let path = std::env::temp_dir().join(format!(
                    "dot-logger-remote-{}-{}.mcap",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
                struct Remove(std::path::PathBuf);
                impl Drop for Remove {
                    fn drop(&mut self) {
                        let _ = std::fs::remove_file(&self.0);
                    }
                }
                let path = Remove(path);
                let file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .read(true)
                    .write(true)
                    .open(&path.0)
                    .unwrap();
                let channels = vec![StorageChannel {
                    channel: ChannelConfig::new(7, 99, 1),
                    topic: "remote-fixture".into(),
                    payload_encoding: "opaque".into(),
                    schema: None,
                }];
                let mut writer = SegmentWriter::new(
                    router.writer_budget(10).unwrap(),
                    WriterConfig {
                        max_payload_bytes: 256,
                        write_buffer_bytes: 0,
                        chunk_target_bytes: 1024,
                        max_records: 8,
                        max_chunks: 4,
                        max_metadata_bytes: 1024,
                    },
                    channels.clone(),
                    file,
                )
                .unwrap();
                router.scope(|mut dispatcher, mut consumers| {
                    let mut events = [None; 8];
                    assert!(
                        dispatcher
                            .turn(Instant::now(), &mut events)
                            .unwrap()
                            .fault
                            .is_none()
                    );
                    let file_delivery = consumers[0].try_next(Instant::now()).unwrap().unwrap();
                    let remote_delivery = consumers[1].try_next(Instant::now()).unwrap().unwrap();
                    writer.write_delivery(&file_delivery).unwrap();
                    writer.flush_chunk().unwrap();
                    assert_eq!(
                        completion
                            .poll(receipt, 10, Boundary::Written)
                            .unwrap()
                            .outcome,
                        QueryOutcome::Complete
                    );
                    let safe = writer.safe_offset();
                    runtime().block_on(async {
                        let (hub, mut inbox, addr, tx, task, b) = start(config(), limits()).await;
                        let mut c = client(addr).await;
                        let mut stream = subscribe(&mut c).await;
                        let sub = stream.message().await.unwrap().unwrap().subscription;
                        let mut pending = match hub.offer_delivery(sub, remote_delivery, false, 1) {
                            Ok(p) => p,
                            Err(_) => panic!("offer"),
                        };
                        let sent = stream.message().await.unwrap().unwrap();
                        assert_eq!(pending.poll().unwrap(), AckState::Pending);
                        assert_eq!(
                            completion
                                .poll(receipt, 20, Boundary::ReceiverAcknowledged)
                                .unwrap()
                                .outcome,
                            QueryOutcome::Pending
                        );
                        let q = Query {
                            basis: TimeBasis::Mono,
                            from: 0,
                            to: 100,
                            channel: None,
                            minimum_level: Level::Trace,
                        };
                        let snapshot = Snapshot::new(
                            &b,
                            42,
                            1,
                            channels,
                            vec![QuerySource::active(
                                std::fs::File::open(&path.0).unwrap(),
                                1,
                                safe,
                            )],
                            QueryConfig {
                                max_segments: 1,
                                max_records: 8,
                                max_record_bytes: 16384,
                                max_source_bytes: 1_000_000,
                                max_payload_bytes: 256,
                                max_metadata_bytes: 1024,
                                producers: 1,
                                page_records: 2,
                                page_bytes: 1024,
                                inline_bytes: 16,
                                blob_read_bytes: 16,
                                read_bytes: 1_000_000,
                            },
                            0,
                            1000,
                            &AtomicBool::new(false),
                        )
                        .unwrap();
                        let mut replay =
                            Replay::new(snapshot, q, Speed::REALTIME, 0, &AtomicBool::new(false))
                                .unwrap();
                        let mut request = req();
                        request.query = Some(wire::QueryMessage {
                            basis: 0,
                            from: 0i128.to_le_bytes().to_vec(),
                            to: 100i128.to_le_bytes().to_vec(),
                            ..wire::QueryMessage::default()
                        });
                        // A distinct frozen snapshot provides the query path while Replay owns its reader.
                        let mut query_snapshot = Snapshot::new(
                            &b,
                            42,
                            1,
                            vec![StorageChannel {
                                channel: ChannelConfig::new(7, 99, 1),
                                topic: "remote-fixture".into(),
                                payload_encoding: "opaque".into(),
                                schema: None,
                            }],
                            vec![QuerySource::active(
                                std::fs::File::open(&path.0).unwrap(),
                                1,
                                safe,
                            )],
                            QueryConfig {
                                max_segments: 1,
                                max_records: 8,
                                max_record_bytes: 16384,
                                max_source_bytes: 1_000_000,
                                max_payload_bytes: 256,
                                max_metadata_bytes: 1024,
                                producers: 1,
                                page_records: 2,
                                page_bytes: 1024,
                                inline_bytes: 0,
                                blob_read_bytes: 16,
                                read_bytes: 1_000_000,
                            },
                            0,
                            1000,
                            &AtomicBool::new(false),
                        )
                        .unwrap();
                        let mut blob_handle = None;
                        let (result, page) = hosted_call(
                            c,
                            "/dot_logger.remote.v1.Logger/Query",
                            request,
                            &mut inbox,
                            |request| {
                                let ManagementRequest::Query { query, .. } = request else {
                                    return Err(RemoteError::InvalidInput);
                                };
                                let cursor = query_snapshot
                                    .start(*query, 0, &AtomicBool::new(false))
                                    .unwrap();
                                let page = query_snapshot
                                    .page(&cursor, 0, &AtomicBool::new(false))
                                    .unwrap();
                                let mut items = Vec::new();
                                for r in page.records {
                                    let (inline, blob) = match &r.payload {
                                        Payload::Blob(handle) => {
                                            blob_handle = Some(handle.clone());
                                            (Vec::new(), 1)
                                        }
                                        Payload::Inline { .. } => {
                                            (page.inline(r).unwrap().to_vec(), 0)
                                        }
                                    };
                                    items.push(RemoteItem {
                                        record: r.record,
                                        inline,
                                        blob,
                                    });
                                }
                                Ok(ManagementReply::Page {
                                    snapshot: 1,
                                    cursor: 0,
                                    items,
                                })
                            },
                        )
                        .await;
                        c = result;
                        assert_eq!(page.items.len(), 1);
                        assert!(page.items[0].inline.is_empty());
                        assert_eq!(page.items[0].blob, 1);
                        let mut request = req();
                        request.handle = 1;
                        request.other = 1;
                        request.length = 3;
                        let (result, data) = hosted_call(
                            c,
                            "/dot_logger.remote.v1.Logger/ReadBlob",
                            request,
                            &mut inbox,
                            |request| {
                                let ManagementRequest::ReadBlob { offset, length, .. } = request
                                else {
                                    return Err(RemoteError::InvalidInput);
                                };
                                let bytes = query_snapshot
                                    .read_blob(
                                        blob_handle.as_ref().unwrap(),
                                        *offset,
                                        *length as usize,
                                        0,
                                        &AtomicBool::new(false),
                                    )
                                    .unwrap();
                                Ok(ManagementReply::Blob(bytes.to_vec()))
                            },
                        )
                        .await;
                        c = result;
                        assert_eq!(data.data, &[1, 2, 3]);
                        let mut request = req();
                        request.record = Some(wire::record(record(1, 3)));
                        request.target = 20;
                        request.boundary = 4;
                        let (result, status) = hosted_call(
                            c,
                            "/dot_logger.remote.v1.Logger/GetCompletion",
                            request,
                            &mut inbox,
                            |request| {
                                let ManagementRequest::GetCompletion {
                                    receipt,
                                    target,
                                    boundary,
                                } = request
                                else {
                                    return Err(RemoteError::InvalidInput);
                                };
                                Ok(ManagementReply::Completion(
                                    completion.poll(*receipt, *target, *boundary).unwrap(),
                                ))
                            },
                        )
                        .await;
                        c = result;
                        assert_eq!(status.outcome, 1);
                        let mut request = req();
                        request.action = 1;
                        request.numerator = 1;
                        request.denominator = 1;
                        let (result, status) = hosted_call(
                            c,
                            "/dot_logger.remote.v1.Logger/Replay",
                            request,
                            &mut inbox,
                            |request| {
                                assert!(matches!(
                                    request,
                                    ManagementRequest::Replay {
                                        action: ReplayAction::Pause,
                                        ..
                                    }
                                ));
                                replay.pause(0, &AtomicBool::new(false)).unwrap();
                                Ok(ManagementReply::Replay {
                                    generation: replay.generation(),
                                    state: replay.state(),
                                })
                            },
                        )
                        .await;
                        c = result;
                        assert_eq!(status.state, 1);
                        assert_eq!(replay.state(), ReplayState::Paused);
                        if acknowledge {
                            c.ready().await.unwrap();
                            let _: tonic::Response<wire::Reply> = c
                                .unary(
                                    Request::new(wire::AckRequest {
                                        major: 1,
                                        subscription: sub,
                                        transfer: sent.transfer,
                                        generation: sent.generation,
                                        session: receipt.session_id.to_le_bytes().to_vec(),
                                        producer: receipt.producer_id,
                                        sequence: receipt.accepted_seq,
                                    }),
                                    PathAndQuery::from_static("/dot_logger.remote.v1.Logger/Ack"),
                                    tonic::codec::ProstCodec::default(),
                                )
                                .await
                                .unwrap();
                            assert_eq!(pending.poll().unwrap(), AckState::Acknowledged);
                        }
                        hub.close();
                        assert_eq!(
                            pending.poll().unwrap(),
                            if acknowledge {
                                AckState::Acknowledged
                            } else {
                                AckState::Unknown
                            }
                        );
                        assert_eq!(
                            completion
                                .poll(receipt, 20, Boundary::ReceiverAcknowledged)
                                .unwrap()
                                .outcome,
                            if acknowledge {
                                QueryOutcome::Complete
                            } else {
                                QueryOutcome::Unknown
                            }
                        );
                        assert_eq!(
                            completion
                                .poll(receipt, 10, Boundary::Written)
                                .unwrap()
                                .outcome,
                            QueryOutcome::Complete
                        );
                        drop((c, stream, replay, query_snapshot, pending));
                        stop(tx, task).await;
                        drop((hub, inbox));
                        assert_eq!(b.used_bytes(), 0);
                    });
                });
            });
        }
    }
}
