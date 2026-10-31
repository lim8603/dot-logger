//! Reconnect, generation, ACK and actual-owner reservation boundaries.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used)]
use dot_logger::{adapters::remote::*, capture::*, config::*, platform::*, query::QueryBudget};
use dot_logger_client::{ClientError, ClientReceiver, ReceiverConfig};
use std::time::{Duration, Instant};

fn budget() -> QueryBudget {
    let mib = 1024 * 1024;
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 8 * mib,
            input_bytes: mib,
            raw_bytes: 0,
            output_bytes: mib,
            external_bytes: 4 * mib,
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
fn config() -> ReceiverConfig {
    ReceiverConfig {
        records: 2,
        record_bytes: 8,
        ttl: Duration::from_secs(10),
    }
}
fn chunk(transfer: u64, generation: u64) -> Chunk {
    Chunk::single(
        1,
        transfer,
        generation,
        CapturedRecord {
            receipt: CaptureReceipt {
                session_id: 42,
                producer_id: 0,
                accepted_seq: transfer,
            },
            metadata: RecordMetadata {
                channel_id: 7,
                schema_id: 99,
                schema_version: 1,
                mono_ns: transfer,
                real_ns: transfer as i64,
                source_time: None,
                level: Level::Info,
                importance: Some(Importance::Normal),
            },
            revision: 1,
            payload_len: 1,
        },
        &[9],
        false,
    )
}

#[test]
fn reconnect_blocks_old_fragments_handoffs_and_acks_until_owners_drop() {
    let budget = budget();
    let mut client = ClientReceiver::new();
    let first = client.begin_connection(&budget, config()).unwrap();
    let ready = client
        .push(&first, chunk(1, 1), Instant::now())
        .unwrap()
        .unwrap();
    let old_ack = client.handoff(ready, |_, bytes| bytes == [9]).ok().unwrap();
    let retained = client
        .push(&first, chunk(2, 1), Instant::now())
        .unwrap()
        .unwrap();
    client.end_connection(&first).unwrap();
    let retained_bytes = budget.used_bytes();
    assert!(retained_bytes > 0);
    let second = client.begin_connection(&budget, config()).unwrap();
    assert!(budget.used_bytes() > retained_bytes);
    assert!(matches!(
        client.push(&first, chunk(3, 1), Instant::now()),
        Err(ClientError::StaleConnection)
    ));
    assert_eq!(
        client.end_connection(&first),
        Err(ClientError::StaleConnection)
    );
    assert_eq!(
        client.take_ack(&second, old_ack),
        Err(ClientError::StaleConnection)
    );
    let retained = client
        .handoff(retained, |_, _| panic!("old consumer must not run"))
        .err()
        .unwrap();
    drop(retained);
    assert_eq!(budget.used_bytes(), retained_bytes);
    let fresh = client
        .push(&second, chunk(1, 1), Instant::now())
        .unwrap()
        .unwrap();
    let ack = client.handoff(fresh, |_, _| true).ok().unwrap();
    assert_eq!(client.take_ack(&second, ack).unwrap().transfer, 1);
    client.end_connection(&second).unwrap();
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn generation_advance_and_consumer_rejection_cannot_create_ack() {
    let budget = budget();
    let mut client = ClientReceiver::new();
    let id = client.begin_connection(&budget, config()).unwrap();
    let ready = client
        .push(&id, chunk(1, 1), Instant::now())
        .unwrap()
        .unwrap();
    let ready = client.handoff(ready, |_, _| false).err().unwrap();
    client.discard_generations(&id, 1, 2).unwrap();
    assert!(
        client
            .handoff(ready, |_, _| panic!("stale consumer must not run"))
            .is_err()
    );
    assert_eq!(
        client.discard_generations(&id, 1, 1),
        Err(ClientError::Remote(RemoteError::Gone))
    );
    let ready = client
        .push(&id, chunk(2, 2), Instant::now())
        .unwrap()
        .unwrap();
    let ack = client.handoff(ready, |_, _| true).ok().unwrap();
    client.end_connection(&id).unwrap();
    assert_eq!(client.take_ack(&id, ack), Err(ClientError::Disconnected));
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn invalid_or_foreign_connection_does_not_replace_current_receiver() {
    let budget = budget();
    let mut client = ClientReceiver::new();
    let mut invalid = config();
    invalid.records = 0;
    assert!(matches!(
        client.begin_connection(&budget, invalid),
        Err(ClientError::Remote(RemoteError::InvalidInput))
    ));
    assert_eq!(budget.used_bytes(), 0);
    let id = client.begin_connection(&budget, config()).unwrap();
    let used = budget.used_bytes();
    assert!(matches!(
        client.begin_connection(&budget, config()),
        Err(ClientError::AlreadyConnected)
    ));
    assert_eq!(budget.used_bytes(), used);
    let mut other = ClientReceiver::new();
    let foreign = other.begin_connection(&budget, config()).unwrap();
    assert_eq!(
        client.end_connection(&foreign),
        Err(ClientError::StaleConnection)
    );
    assert_eq!(
        client.discard_generations(&foreign, 1, 2),
        Err(ClientError::StaleConnection)
    );
    let ready = client
        .push(&id, chunk(1, 1), Instant::now())
        .unwrap()
        .unwrap();
    let ready = other
        .handoff(ready, |_, _| panic!("foreign consumer must not run"))
        .err()
        .unwrap();
    let ack = client.handoff(ready, |_, _| true).ok().unwrap();
    assert_eq!(
        client.take_ack(&foreign, ack),
        Err(ClientError::StaleConnection)
    );
    other.end_connection(&foreign).unwrap();
    client.end_connection(&id).unwrap();
    assert_eq!(budget.used_bytes(), 0);
}
