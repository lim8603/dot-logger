//! Bounded remote queues and validated receiver handoff.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used)]
use dot_logger::adapters::remote::*;
use dot_logger::capture::*;
use dot_logger::query::QueryBudget;
use std::time::{Duration, Instant};
mod support;

fn limits() -> RemoteLimits {
    RemoteLimits {
        connections: 2,
        subscriptions: 2,
        commands: 2,
        message_bytes: 4096,
        record_bytes: 8192,
        subscription_records: 2,
        subscription_bytes: 16384,
        max_age: Duration::from_secs(2),
        request_timeout: Duration::from_secs(1),
    }
}
fn budget() -> QueryBudget {
    let mut c = support::plan().config().clone();
    c.memory.external_bytes = 4 * 1024 * 1024;
    c.memory.total_bytes += c.memory.external_bytes;
    QueryBudget::new(
        &dot_logger::config::SessionBuilder::new(c)
            .validate(&dot_logger::platform::PlatformCapabilities::default())
            .unwrap(),
    )
}
fn record(sequence: u64, len: usize) -> CapturedRecord {
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
        payload_len: len,
    }
}

#[test]
fn receiver_rejects_bad_offset_checksum_and_defers_ack_until_handoff() {
    let b = budget();
    let now = Instant::now();
    let mut receiver = Reassembler::new(&b, 1, 8192, Duration::from_secs(1)).unwrap();
    let mut chunk = Chunk::single(1, 1, 1, record(1, 3), &[1, 2, 3], false);
    chunk.offset = 1;
    assert!(receiver.push(chunk, now).is_err());
    let mut chunk = Chunk::single(1, 1, 1, record(1, 3), &[1, 2, 3], false);
    chunk.checksum ^= 1;
    assert!(receiver.push(chunk, now).is_err());
    let ready = receiver
        .push(Chunk::single(1, 1, 1, record(1, 3), &[1, 2, 3], false), now)
        .unwrap()
        .unwrap();
    assert!(
        receiver
            .push(Chunk::single(1, 2, 1, record(2, 3), &[4, 5, 6], false), now)
            .is_err()
    );
    let ready = ready.handoff(|_, _| false).err().unwrap();
    let ack = ready
        .handoff(|metadata, bytes| {
            assert_eq!(metadata.receipt.accepted_seq, 1);
            assert_eq!(bytes, &[1, 2, 3]);
            true
        })
        .ok()
        .unwrap();
    assert_eq!(ack.transfer, 1);
    assert!(
        receiver
            .push(Chunk::single(1, 2, 1, record(2, 0), &[], false), now)
            .unwrap()
            .is_some()
    );
    drop(receiver);
    assert_eq!(b.used_bytes(), 0);
}

#[test]
fn server_is_disabled_by_default_and_external_bind_requires_tls_and_auth() {
    assert!(!ServerConfig::default().enabled);
    let mut config = ServerConfig {
        enabled: true,
        address: "0.0.0.0:0".parse().unwrap(),
        ..ServerConfig::default()
    };
    assert!(config.validate().is_err());
    config.address = "127.0.0.1:0".parse().unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn remote_and_query_share_external_budget_and_release_only_after_all_owners() {
    let b = budget();
    let (hub, inbox) = RemoteHub::new(&b, limits()).unwrap();
    let used = b.used_bytes();
    assert!(used > 0);
    assert!(Reassembler::new(&b, 1024, 8192, Duration::from_secs(1)).is_err());
    let retained = hub.clone();
    drop(hub);
    assert!(b.used_bytes() > 0);
    drop((retained, inbox));
    assert_eq!(b.used_bytes(), 0);
}
