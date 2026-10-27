//! TC-002: small capture admission, ownership, and bounded waiting.

#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use dot_logger::capture::{
    CaptureBuildError, CaptureError, CaptureOwner, ChannelConfig, Importance, ImportanceMapping,
    Level, ReadError, RecordMetadata, RecordOutcome, SourceTime,
};
use dot_logger::config::{
    ConfigError, ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, Resource,
    SessionBuilder, SessionConfig, ValidatedConfig, WorkerThreadingConfig,
};
use dot_logger::platform::{PlatformCapabilities, Requirement};

fn plan(policy: ImportancePolicy, slots: usize, reserve: usize) -> ValidatedConfig {
    let config = SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 256 * 1024,
            raw_bytes: 0,
            output_bytes: 256 * 1024,
            external_bytes: 0,
            control_bytes: 256 * 1024,
            worker_bytes: 256 * 1024,
        },
        producers: vec![
            ProducerQuota {
                small: PoolQuota {
                    slots,
                    slot_bytes: 16,
                    important_slots: reserve
                },
                raw: None,
            };
            2
        ],
        importance: policy,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    };
    SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap()
}

fn owner(policy: ImportancePolicy, slots: usize, reserve: usize) -> CaptureOwner {
    CaptureOwner::new(
        &plan(policy, slots, reserve),
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
    )
    .unwrap()
}

fn metadata(importance: Option<Importance>) -> RecordMetadata {
    RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 123,
        real_ns: 456,
        source_time: None,
        level: Level::Info,
        importance,
    }
}

#[test]
fn normal_saturation_preserves_important_reservation_and_rejects_newest() {
    let mut owner = owner(ImportancePolicy::PrioritizeImportant, 8, 2);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        for seq in 1..=6 {
            let RecordOutcome::Accepted(receipt) = producer
                .try_record_small(metadata(None), b"normal")
                .unwrap()
            else {
                panic!("unexpected skip")
            };
            assert_eq!(receipt.accepted_seq, seq);
        }
        assert_eq!(
            producer.try_record_small(metadata(None), b"extra"),
            Err(CaptureError::Capacity)
        );
        for seq in 7..=8 {
            let RecordOutcome::Accepted(receipt) = producer
                .try_record_small(metadata(Some(Importance::Important)), b"important")
                .unwrap()
            else {
                panic!("unexpected skip")
            };
            assert_eq!(receipt.accepted_seq, seq);
        }
        assert_eq!(
            producer.try_record_small(metadata(Some(Importance::Important)), b"extra"),
            Err(CaptureError::Capacity)
        );
        let mut bytes = [0; 16];
        for seq in 1..=8 {
            let record = consumer.try_read(&mut bytes).unwrap().unwrap();
            assert_eq!(record.receipt.accepted_seq, seq);
            assert_eq!(
                &bytes[..record.payload_len],
                if seq <= 6 {
                    b"normal".as_slice()
                } else {
                    b"important".as_slice()
                }
            );
        }
        assert!(consumer.try_read(&mut bytes).unwrap().is_none());
    });
}

#[test]
fn general_producer_observes_deadline_when_full_and_before_first_attempt() {
    let mut owner = owner(ImportancePolicy::TreatEqually, 1, 0);
    owner.scope(|mut scope| {
        let (producer, _consumer) = scope.next().unwrap();
        let mut producer = producer.into_general();
        assert_eq!(
            producer.record_until(
                metadata(None),
                b"past",
                Instant::now() - Duration::from_millis(1)
            ),
            Err(CaptureError::DeadlineExceeded)
        );
        assert!(
            producer
                .record_until(
                    metadata(None),
                    b"first",
                    Instant::now() + Duration::from_secs(1)
                )
                .is_ok()
        );
        assert_eq!(
            producer.record_until(
                metadata(None),
                b"full",
                Instant::now() + Duration::from_millis(2)
            ),
            Err(CaptureError::DeadlineExceeded)
        );
    });
}

#[test]
fn copies_input_preserves_metadata_and_does_not_consume_on_short_read() {
    let mut owner = owner(ImportancePolicy::PrioritizeImportant, 2, 1);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut input = [10, 20, 30];
        let mut meta = metadata(None);
        meta.real_ns = -123;
        meta.source_time = Some(SourceTime {
            ticks: 17,
            clock_domain: 8,
        });
        producer.try_record_small(meta, &input).unwrap();
        input.fill(0);
        assert_eq!(
            consumer.try_read(&mut [0; 2]),
            Err(ReadError::BufferTooSmall(3))
        );
        let mut bytes = [0; 16];
        let record = consumer.try_read(&mut bytes).unwrap().unwrap();
        assert_eq!(&bytes[..record.payload_len], &[10, 20, 30]);
        meta.importance = Some(Importance::Normal);
        assert_eq!(record.metadata, meta);
        assert_eq!(record.revision, 1);
        assert_eq!(record.receipt.session_id, 42);
        producer.try_record_small(metadata(None), &[5; 16]).unwrap();
        assert_eq!(
            consumer
                .try_read(&mut bytes)
                .unwrap()
                .unwrap()
                .receipt
                .accepted_seq,
            2
        );
    });
}

#[test]
fn equal_policy_has_no_priority_and_producer_quotas_are_independent() {
    let mut owner = owner(ImportancePolicy::TreatEqually, 2, 0);
    owner.scope(|mut scope| {
        let (mut a, mut a_read) = scope.next().unwrap();
        let (mut b, mut b_read) = scope.next().unwrap();
        a.try_record_small(metadata(None), &[1; 16]).unwrap();
        a.try_record_small(metadata(None), &[2; 16]).unwrap();
        assert_eq!(
            a.try_record_small(metadata(Some(Importance::Important)), b"x"),
            Err(CaptureError::Capacity)
        );
        b.try_record_small(metadata(None), b"b").unwrap();
        let record = b_read.try_read(&mut [0; 16]).unwrap().unwrap();
        assert_eq!(record.receipt.producer_id, 1);
        assert_eq!(record.receipt.accepted_seq, 1);
        a_read.try_read(&mut [0; 16]).unwrap().unwrap();
        a.try_record_small(metadata(None), b"a").unwrap();
    });
}

#[test]
fn explicit_importance_overrides_channel_default_and_complete_level_mapping() {
    let plan = plan(ImportancePolicy::PrioritizeImportant, 2, 1);
    let mut mapped = ChannelConfig::new(7, 99, 1);
    mapped.importance = ImportanceMapping::ByLevel([
        Importance::Normal,
        Importance::Normal,
        Importance::Normal,
        Importance::Important,
        Importance::Important,
        Importance::Important,
    ]);
    let mut defaulted = ChannelConfig::new(8, 100, 2);
    defaulted.importance = ImportanceMapping::Default(Importance::Important);
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[mapped, defaulted]).unwrap();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut meta = metadata(Some(Importance::Normal));
        meta.level = Level::Error;
        producer.try_record_small(meta, b"explicit normal").unwrap();
        assert_eq!(
            producer.try_record_small(meta, b"x"),
            Err(CaptureError::Capacity)
        );
        meta.importance = None;
        producer
            .try_record_small(meta, b"mapped important")
            .unwrap();
        let mut bytes = [0; 16];
        assert_eq!(
            consumer
                .try_read(&mut bytes)
                .unwrap()
                .unwrap()
                .metadata
                .importance,
            Some(Importance::Normal)
        );
        assert_eq!(
            consumer
                .try_read(&mut bytes)
                .unwrap()
                .unwrap()
                .metadata
                .importance,
            Some(Importance::Important)
        );
        meta.channel_id = 8;
        meta.schema_id = 100;
        meta.schema_version = 2;
        producer.try_record_small(meta, b"default").unwrap();
        assert_eq!(
            consumer
                .try_read(&mut bytes)
                .unwrap()
                .unwrap()
                .metadata
                .importance,
            Some(Importance::Important)
        );
    });
}

#[test]
fn validates_before_filter_and_only_accepted_records_advance_sequence() {
    let plan = plan(ImportancePolicy::PrioritizeImportant, 2, 1);
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Warn;
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap();
    owner.scope(|mut scope| {
        let (mut producer, _consumer) = scope.next().unwrap();
        let mut meta = metadata(None);
        assert_eq!(
            producer.try_record_small(meta, b"skip"),
            Ok(RecordOutcome::Skipped)
        );
        meta.channel_id = 9;
        assert_eq!(
            producer.try_record_small(meta, b"x"),
            Err(CaptureError::InvalidInput)
        );
        meta.channel_id = 7;
        meta.schema_version = 2;
        assert_eq!(
            producer.try_record_small(meta, b"x"),
            Err(CaptureError::SchemaMismatch)
        );
        meta.schema_version = 1;
        meta.schema_id = 1;
        assert_eq!(
            producer.try_record_small(meta, b"x"),
            Err(CaptureError::SchemaMismatch)
        );
        meta.schema_id = 99;
        assert_eq!(
            producer.try_record_small(meta, &[0; 17]),
            Err(CaptureError::TooLarge)
        );
        meta.level = Level::Warn;
        let RecordOutcome::Accepted(receipt) = producer.try_record_small(meta, b"").unwrap() else {
            panic!("unexpected skip")
        };
        assert_eq!(receipt.accepted_seq, 1);
    });
}

#[test]
fn closing_rejects_new_input_and_preserves_accepted_records_across_scopes() {
    let mut owner = owner(ImportancePolicy::TreatEqually, 2, 0);
    owner.scope(|mut scope| {
        let control = scope.control();
        let (mut producer, _consumer) = scope.next().unwrap();
        producer.try_record_small(metadata(None), b"keep").unwrap();
        control.close();
        assert_eq!(
            producer.try_record_small(metadata(None), b"new"),
            Err(CaptureError::Closing)
        );
        assert_eq!(
            producer.into_general().record_until(
                metadata(None),
                b"new",
                Instant::now() + Duration::from_secs(1)
            ),
            Err(CaptureError::Closing)
        );
    });
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut bytes = [0; 16];
        assert_eq!(
            consumer
                .try_read(&mut bytes)
                .unwrap()
                .unwrap()
                .receipt
                .accepted_seq,
            1
        );
        assert_eq!(&bytes[..4], b"keep");
        assert_eq!(
            producer.try_record_small(metadata(None), b"new"),
            Err(CaptureError::Closing)
        );
    });
}

#[test]
fn repeated_wraparound_and_mixed_importance_keep_bytes_and_fifo_intact() {
    let mut owner = owner(ImportancePolicy::PrioritizeImportant, 3, 1);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut bytes = [0; 16];
        for batch in 0..200 {
            for item in 0..3 {
                let len = (batch + item) % 17;
                let importance = if item == 2 {
                    Importance::Important
                } else {
                    Importance::Normal
                };
                producer
                    .try_record_small(metadata(Some(importance)), &[item as u8; 16][..len])
                    .unwrap();
            }
            for item in 0..3 {
                let record = consumer.try_read(&mut bytes).unwrap().unwrap();
                assert_eq!(record.receipt.accepted_seq, (batch * 3 + item + 1) as u64);
                assert_eq!(record.payload_len, (batch + item) % 17);
                assert!(
                    bytes[..record.payload_len]
                        .iter()
                        .all(|&byte| byte == item as u8)
                );
            }
        }
    });
}

#[test]
fn registration_rejects_missing_or_duplicate_channels_and_concrete_budget_overflow() {
    let plan = plan(ImportancePolicy::TreatEqually, 1, 0);
    assert!(matches!(
        CaptureOwner::new(&plan, 42, 1, &[]),
        Err(CaptureBuildError::InvalidChannels)
    ));
    let channel = ChannelConfig::new(7, 99, 1);
    assert!(matches!(
        CaptureOwner::new(&plan, 42, 1, &[channel, channel]),
        Err(CaptureBuildError::InvalidChannels)
    ));
    let mut config = plan.config().clone();
    config.memory.input_bytes = 32; // Payloads fit; actual envelope/queue metadata does not.
    let plan = SessionBuilder::new(config.clone())
        .validate(&PlatformCapabilities::default())
        .unwrap();
    assert!(matches!(
        CaptureOwner::new(&plan, 42, 1, &[channel]),
        Err(CaptureBuildError::Config(ConfigError::BudgetExceeded {
            resource: Resource::Input,
            ..
        }))
    ));
    config.memory.input_bytes = 256 * 1024;
    config.memory.control_bytes = 0;
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    assert!(matches!(
        CaptureOwner::new(&plan, 42, 1, &[channel]),
        Err(CaptureBuildError::Config(ConfigError::BudgetExceeded {
            resource: Resource::Control,
            ..
        }))
    ));
}

#[test]
fn waiting_general_producer_resumes_when_consumer_releases_capacity() {
    let mut owner = owner(ImportancePolicy::TreatEqually, 1, 0);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        producer.try_record_small(metadata(None), b"first").unwrap();
        std::thread::scope(|threads| {
            let producer = threads.spawn(move || {
                producer.into_general().record_until(
                    metadata(None),
                    b"second",
                    Instant::now() + Duration::from_secs(5),
                )
            });
            consumer.try_read(&mut [0; 16]).unwrap().unwrap();
            assert!(matches!(
                producer.join().unwrap(),
                Ok(RecordOutcome::Accepted(_))
            ));
            let mut bytes = [0; 16];
            let record = consumer.try_read(&mut bytes).unwrap().unwrap();
            assert_eq!(record.receipt.accepted_seq, 2);
            assert_eq!(&bytes[..record.payload_len], b"second");
        });
    });
}

#[test]
fn close_interrupts_full_general_wait_without_consuming_or_accepting() {
    let mut owner = owner(ImportancePolicy::TreatEqually, 1, 0);
    owner.scope(|mut scope| {
        let control = scope.control();
        let (mut producer, mut consumer) = scope.next().unwrap();
        producer.try_record_small(metadata(None), b"keep").unwrap();
        std::thread::scope(|threads| {
            let producer = threads.spawn(move || {
                producer.into_general().record_until(
                    metadata(None),
                    b"cancel",
                    Instant::now() + Duration::from_secs(5),
                )
            });
            control.close();
            assert_eq!(producer.join().unwrap(), Err(CaptureError::Closing));
            let mut bytes = [0; 16];
            let record = consumer.try_read(&mut bytes).unwrap().unwrap();
            assert_eq!(&bytes[..record.payload_len], b"keep");
            assert!(consumer.try_read(&mut bytes).unwrap().is_none());
        });
    });
}

#[test]
fn two_concurrent_producers_keep_independent_fifo_through_queue_wraps() {
    let mut owner = owner(ImportancePolicy::PrioritizeImportant, 8, 2);
    owner.scope(|mut scope| {
        let pairs = [scope.next().unwrap(), scope.next().unwrap()];
        std::thread::scope(|threads| {
            let mut readers = Vec::new();
            let mut writers = Vec::new();
            for (mut producer, consumer) in pairs {
                readers.push(consumer);
                writers.push(threads.spawn(move || {
                    let deadline = Instant::now() + Duration::from_secs(10);
                    for value in 1u64..=4000 {
                        let importance = if value % 3 == 0 {
                            Importance::Important
                        } else {
                            Importance::Normal
                        };
                        loop {
                            match producer
                                .try_record_small(metadata(Some(importance)), &value.to_le_bytes())
                            {
                                Ok(RecordOutcome::Accepted(receipt)) => {
                                    assert_eq!(receipt.accepted_seq, value);
                                    break;
                                }
                                Err(CaptureError::Capacity) => {
                                    assert!(
                                        Instant::now() < deadline,
                                        "consumer failed to progress"
                                    );
                                    std::thread::yield_now();
                                }
                                other => panic!("unexpected capture result: {other:?}"),
                            }
                        }
                    }
                }));
            }
            let mut expected = [1u64; 2];
            let deadline = Instant::now() + Duration::from_secs(10);
            while expected.iter().any(|&next| next <= 4000) {
                for (index, consumer) in readers.iter_mut().enumerate() {
                    let mut bytes = [0; 16];
                    if let Some(record) = consumer.try_read(&mut bytes).unwrap() {
                        assert_eq!(record.receipt.producer_id, index as u32);
                        assert_eq!(record.receipt.accepted_seq, expected[index]);
                        assert_eq!(&bytes[..record.payload_len], expected[index].to_le_bytes());
                        expected[index] += 1;
                    }
                }
                assert!(Instant::now() < deadline, "producer failed to progress");
            }
            for writer in writers {
                writer.join().unwrap();
            }
        });
    });
}

#[test]
fn all_reserved_and_no_reserved_boundaries_have_no_hidden_capacity() {
    for reserve in [0, 2] {
        let mut owner = owner(ImportancePolicy::PrioritizeImportant, 2, reserve);
        owner.scope(|mut scope| {
            let (mut producer, mut consumer) = scope.next().unwrap();
            if reserve == 2 {
                assert_eq!(
                    producer.try_record_small(metadata(None), b"normal"),
                    Err(CaptureError::Capacity)
                );
            }
            let importance = if reserve == 2 {
                Importance::Important
            } else {
                Importance::Normal
            };
            for _ in 0..2 {
                producer
                    .try_record_small(metadata(Some(importance)), &[0; 16])
                    .unwrap();
            }
            assert_eq!(
                producer.try_record_small(metadata(Some(Importance::Important)), b"extra"),
                Err(CaptureError::Capacity)
            );
            for _ in 0..2 {
                consumer.try_read(&mut [0; 16]).unwrap().unwrap();
            }
        });
    }
}

#[test]
fn payload_limit_is_configured_and_concrete_input_budget_is_enforced_exactly() {
    let mut config = plan(ImportancePolicy::PrioritizeImportant, 8, 2)
        .config()
        .clone();
    for producer in &mut config.producers {
        producer.small.slot_bytes = 513;
    }
    let validated = SessionBuilder::new(config.clone())
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let channel = ChannelConfig::new(7, 99, 1);
    let initial = CaptureOwner::new(&validated, 42, 1, &[channel]).unwrap();
    let required = initial.input_reserved_bytes();
    drop(initial);
    config.memory.input_bytes = required - 1;
    let validated = SessionBuilder::new(config.clone())
        .validate(&PlatformCapabilities::default())
        .unwrap();
    assert!(
        matches!(CaptureOwner::new(&validated, 42, 1, &[channel]), Err(CaptureBuildError::Config(ConfigError::BudgetExceeded { resource: Resource::Input, required: observed, available })) if observed == required && available == required - 1)
    );
    config.memory.input_bytes = required;
    let validated = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut owner = CaptureOwner::new(&validated, 42, 1, &[channel]).unwrap();
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        producer
            .try_record_small(metadata(None), &[123; 513])
            .unwrap();
        assert_eq!(
            producer.try_record_small(metadata(None), &[0; 514]),
            Err(CaptureError::TooLarge)
        );
        let mut bytes = [0; 513];
        assert_eq!(
            consumer.try_read(&mut bytes).unwrap().unwrap().payload_len,
            513
        );
        assert_eq!(bytes, [123; 513]);
    });
}

#[test]
fn borrowed_handles_can_be_registered_again_without_reusing_sequences() {
    let mut owner = owner(ImportancePolicy::TreatEqually, 2, 0);
    for seq in 1..=3 {
        owner.scope(|mut scope| {
            let (mut producer, mut consumer) = scope.next().unwrap();
            let RecordOutcome::Accepted(receipt) = producer
                .try_record_small(metadata(None), b"keep sequence")
                .unwrap()
            else {
                panic!("unexpected skip")
            };
            assert_eq!(receipt.accepted_seq, seq);
            assert_eq!(
                consumer.try_read(&mut [0; 16]).unwrap().unwrap().receipt,
                receipt
            );
        });
    }
}

#[test]
fn rejects_unrepresentable_queue_layout_before_attempting_allocation() {
    let mut config = plan(ImportancePolicy::TreatEqually, 1, 0).config().clone();
    config.producers.truncate(1);
    config.producers[0].small.slots = isize::MAX as usize;
    config.producers[0].small.slot_bytes = 1;
    config.memory.input_bytes = isize::MAX as usize;
    config.memory.total_bytes = usize::MAX;
    let validated = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    assert!(matches!(
        CaptureOwner::new(&validated, 42, 1, &[ChannelConfig::new(7, 99, 1)]),
        Err(CaptureBuildError::Config(ConfigError::ArithmeticOverflow))
    ));
}
