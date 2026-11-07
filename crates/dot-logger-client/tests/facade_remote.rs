//! TC-013 F13 (TASK-027 S3c): remote connection owned by the facade.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dot_logger::adapters::remote::{
    ManagementReply, ManagementRequest, OverflowPolicy, RemoteError, RemoteLimits, ServerConfig,
    SubscriptionFilter,
};
use dot_logger::capture::Level;
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, Resource, SessionBuilder,
    SessionConfig, WorkerThreadingConfig,
};
use dot_logger::logger::{Budget, Logger, Retention, Shutdown, StartError, WriterQuota};
use dot_logger::platform::{PlatformCapabilities, Requirement};
use dot_logger::query::QueryBudget;
use dot_logger_client::{
    Cancellation, ClientError, ReceiverConfig, RemoteClient, RemoteConfig, RemoteEvent,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIB: usize = 1024 * 1024;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-tc013-remote-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Client-side receive budget (the client owns its own reservation).
fn client_budget() -> QueryBudget {
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 36 * MIB,
            input_bytes: MIB,
            raw_bytes: 0,
            output_bytes: MIB,
            external_bytes: 32 * MIB,
            control_bytes: MIB,
            worker_bytes: MIB,
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

fn budget() -> Budget {
    Budget::new()
        .memory_bytes(64 * MIB)
        .sync_every(Duration::from_millis(20))
        .drop_deadline(Duration::from_secs(3))
        .writer_quota(WriterQuota::small(64, 64))
}

fn server() -> ServerConfig {
    ServerConfig {
        enabled: true,
        ..Default::default()
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn f13_subscribers_receive_copies_status_is_served_and_shutdown_closes_the_server() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("control")
        .remote(server())
        .start()
        .unwrap();
    let address = logger.remote_addr().expect("remote declared");
    assert!(address.ip().is_loopback());
    let rt = runtime();
    let cancel = Cancellation::new();
    let receive = client_budget();
    let mut client = rt
        .block_on(RemoteClient::connect(
            RemoteConfig::new(format!("http://{address}")),
            &cancel,
        ))
        .unwrap();
    let mut stream = rt
        .block_on(client.subscribe(
            &receive,
            SubscriptionFilter {
                channel: None,
                minimum_level: Level::Trace,
                originals: true,
                overflow: OverflowPolicy::Gap,
            },
            ReceiverConfig {
                records: 16,
                record_bytes: 64,
                ttl: Duration::from_secs(5),
            },
            &cancel,
        ))
        .unwrap();
    let blob = logger.channel::<[u8]>("blob").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    for i in 0..5u8 {
        while writer.try_log(blob, &[i, i, i][..]).is_err() {
            std::thread::yield_now();
        }
    }
    let mut received = Vec::new();
    while received.len() < 5 {
        let event = rt.block_on(stream.next(&cancel)).unwrap().unwrap();
        let RemoteEvent::Record(record) = event else {
            panic!("no overflow expected");
        };
        rt.block_on(stream.handoff(
            record,
            |r, payload| {
                received.push((r.metadata.channel_id, payload.to_vec()));
                true
            },
            &cancel,
        ))
        .unwrap();
    }
    assert_eq!(
        received,
        (0..5u8).map(|i| (1, vec![i, i, i])).collect::<Vec<_>>()
    );
    // Status answers with the logger session; other operations are explicit.
    let status = rt
        .block_on(client.request(ManagementRequest::GetStatus, &cancel))
        .unwrap();
    let ManagementReply::Status { revision, .. } = status else {
        panic!("{status:?}");
    };
    assert_eq!(revision, 1);
    let query = rt.block_on(client.request(
        ManagementRequest::ReadBlob {
            snapshot: 1,
            blob: 1,
            offset: 0,
            length: 16,
        },
        &cancel,
    ));
    assert!(
        matches!(query, Err(ClientError::Remote(RemoteError::Unsupported))),
        "{query:?}"
    );
    drop(writer);
    // A live but stalled subscriber (this client's runtime is not polled while
    // shutdown blocks) neither holds shutdown beyond the stop grace nor
    // affects its success.
    let begun = std::time::Instant::now();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(
        begun.elapsed() < Duration::from_secs(4),
        "stalled client held shutdown for {:?}",
        begun.elapsed()
    );
    assert!(report.is_success(), "{report:?}");
    let ended = rt.block_on(stream.next(&cancel));
    assert!(
        !matches!(ended, Ok(Some(_))),
        "the stream must end with the server"
    );
    assert!(
        std::net::TcpStream::connect(address).is_err(),
        "the listener must be closed when shutdown returns"
    );
}

#[test]
fn f13_remote_buffers_are_charged_to_the_memory_budget() {
    let temp = Temp::new();
    let error = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("control")
        .remote_with(
            server(),
            RemoteLimits {
                connections: 16,
                subscriptions: 16,
                commands: 16,
                message_bytes: 16 * MIB,
                record_bytes: 64,
                subscription_records: 16,
                subscription_bytes: 64,
                max_age: Duration::from_secs(5),
                request_timeout: Duration::from_secs(2),
            },
        )
        .start()
        .unwrap_err();
    assert!(
        matches!(
            error,
            StartError::Budget {
                // Remote buffers come out of the total before the core split.
                resource: Resource::Total,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn f13_bind_failure_rejects_start_and_leaves_nothing_behind() {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let temp = Temp::new();
    let error = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("control")
        .remote(ServerConfig {
            address: taken.local_addr().unwrap(),
            ..server()
        })
        .start()
        .unwrap_err();
    assert!(matches!(error, StartError::Remote(_)), "{error:?}");
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn f13_status_carries_the_operations_report_when_telemetry_is_enabled() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(budget())
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<[u8]>("blob")
        .writer("control")
        .telemetry()
        .remote(server())
        .start()
        .unwrap();
    let address = logger.remote_addr().unwrap();
    let rt = runtime();
    let cancel = Cancellation::new();
    let mut client = rt
        .block_on(RemoteClient::connect(
            RemoteConfig::new(format!("http://{address}")),
            &cancel,
        ))
        .unwrap();
    let status = rt
        .block_on(client.request(ManagementRequest::GetStatus, &cancel))
        .unwrap();
    let ManagementReply::Status { report, .. } = status else {
        panic!("{status:?}");
    };
    let report: serde_json::Value = serde_json::from_slice(&report).unwrap();
    assert_eq!(report["schema"], "dot_logger.status.v1");
    client.disconnect();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(report.is_success(), "{report:?}");
}
