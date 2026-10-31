#![allow(clippy::unwrap_used)]

use dot_logger::capture::{
    CaptureError, CaptureOwner, ChannelConfig, Importance, Level, RecordMetadata, RecordOutcome,
};
use dot_logger::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
    WorkerThreadingConfig,
};
use dot_logger::platform::{PlatformCapabilities, Requirement};

// Safety boundary: this module only forwards the exact GlobalAlloc arguments to
// System and updates already initialized, non-allocating thread-local Cells.
// None of this unsafe instrumentation is linked into the production library.
#[allow(unsafe_code)]
mod allocator {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ENABLED: Cell<bool> = const { Cell::new(false) };
        static CALLS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
        static BYTES: Cell<(i128, i128)> = const { Cell::new((0, 0)) };
    }

    fn record(allocation: bool, deallocation: bool, allocated: usize, released: usize) {
        let _ = ENABLED.try_with(|enabled| {
            if enabled.get() {
                let _ = CALLS.try_with(|calls| {
                    let (a, d) = calls.get();
                    calls.set((a + usize::from(allocation), d + usize::from(deallocation)));
                });
                let _ = BYTES.try_with(|bytes| {
                    let (live, peak) = bytes.get();
                    let live = live
                        .saturating_add(allocated as i128)
                        .saturating_sub(released as i128);
                    bytes.set((live, peak.max(live)));
                });
            }
        });
    }

    struct CountingAllocator;

    // SAFETY: Every allocation is delegated to System, and deallocation/realloc
    // receive the original pointer/layout unchanged. The TLS bookkeeping never
    // allocates, panics, unwinds, or takes a lock inside the allocator.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record(true, false, layout.size(), 0);
            // SAFETY: GlobalAlloc's caller supplies the valid layout.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(true, false, layout.size(), 0);
            // SAFETY: Same layout contract as alloc.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            record(false, true, 0, layout.size());
            // SAFETY: GlobalAlloc's caller supplies a live allocation/layout.
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            record(true, true, size, layout.size());
            // SAFETY: Forward the live pointer and valid new size unchanged.
            unsafe { System.realloc(ptr, layout, size) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    struct Observation;

    impl Drop for Observation {
        fn drop(&mut self) {
            ENABLED.with(|enabled| enabled.set(false));
        }
    }

    pub fn observe(callback: impl FnOnce()) -> (usize, usize) {
        CALLS.with(|calls| calls.set((0, 0)));
        BYTES.with(|bytes| bytes.set((0, 0)));
        ENABLED.with(|enabled| enabled.set(true));
        let observation = Observation;
        callback();
        drop(observation);
        CALLS.with(Cell::get)
    }

    pub fn observe_memory(callback: impl FnOnce()) -> (i128, i128) {
        observe(callback);
        BYTES.with(Cell::get)
    }
}

#[test]
fn small_capture_success_skip_errors_and_last_borrow_drop_never_call_allocator() {
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
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 512,
                important_slots: 2,
            },
            raw: None,
        }],
        importance: ImportancePolicy::PrioritizeImportant,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    };
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Info;
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap();
    let mut meta = RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 123,
        real_ns: 456,
        source_time: None,
        level: Level::Info,
        importance: None,
    };
    // Verify that the guard observes real allocations before trusting a zero.
    let mut calibration = None;
    assert!(
        allocator::observe(|| {
            calibration = Some(std::hint::black_box(vec![1u8; 37]));
        })
        .0 > 0
    );
    assert!(allocator::observe(|| drop(calibration)).1 > 0);

    assert_eq!(
        allocator::observe(|| {
            owner.scope(|mut scope| {
                let control = scope.control();
                let (mut producer, mut consumer) = scope.next().unwrap();
                for _ in 0..20 {
                    for _ in 0..6 {
                        assert!(matches!(
                            producer.try_record_small(meta, &[5; 512]),
                            Ok(RecordOutcome::Accepted(_))
                        ));
                    }
                    assert_eq!(
                        producer.try_record_small(meta, b"full"),
                        Err(CaptureError::Capacity)
                    );
                    meta.importance = Some(Importance::Important);
                    for _ in 0..2 {
                        producer.try_record_small(meta, &[6; 512]).unwrap();
                    }
                    assert_eq!(
                        producer.try_record_small(meta, b"full"),
                        Err(CaptureError::Capacity)
                    );
                    meta.importance = None;
                    let mut bytes = [0; 512];
                    for _ in 0..8 {
                        consumer.try_read(&mut bytes).unwrap().unwrap();
                    }
                }
                meta.level = Level::Debug;
                assert_eq!(
                    producer.try_record_small(meta, b"skip"),
                    Ok(RecordOutcome::Skipped)
                );
                meta.channel_id = 8;
                assert_eq!(
                    producer.try_record_small(meta, b"bad"),
                    Err(CaptureError::InvalidInput)
                );
                meta.channel_id = 7;
                meta.schema_id = 100;
                assert_eq!(
                    producer.try_record_small(meta, b"bad"),
                    Err(CaptureError::SchemaMismatch)
                );
                meta.schema_id = 99;
                assert_eq!(
                    producer.try_record_small(meta, &[0; 513]),
                    Err(CaptureError::TooLarge)
                );
                control.close();
                assert_eq!(
                    producer.try_record_small(meta, b"closed"),
                    Err(CaptureError::Closing)
                );
                // Both handles leave scope inside the observed region; owning
                // queue endpoints remain inside the management owner.
            });
        }),
        (0, 0)
    );
    assert!(allocator::observe(|| drop(owner)).1 > 0);
}

#[test]
fn raw_loan_rejection_coalesced_return_and_last_handle_drop_never_allocate_or_free() {
    use dot_logger::pool::RawRecordOutcome;
    let config = SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 256 * 1024,
            raw_bytes: 256 * 1024,
            output_bytes: 256 * 1024,
            external_bytes: 0,
            control_bytes: 128 * 1024,
            worker_bytes: 128 * 1024,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 16,
                important_slots: 2,
            },
            raw: Some(PoolQuota {
                slots: 4,
                slot_bytes: 4096,
                important_slots: 1,
            }),
        }],
        importance: ImportancePolicy::PrioritizeImportant,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    };
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Info;
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap();
    let mut meta = RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 123,
        real_ns: 456,
        source_time: None,
        level: Level::Info,
        importance: None,
    };
    assert_eq!(
        allocator::observe(|| {
            owner.scope(|mut scope| {
                let control = scope.control();
                let (mut producer, mut consumer) = scope.next().unwrap();
                // Saturate the one-slot return hint. The slot states retain all
                // returns, including rejections and the last loan's RT Drop.
                let a = producer.try_acquire(4096, Importance::Normal).unwrap();
                let b = producer.try_acquire(4096, Importance::Normal).unwrap();
                let c = producer.try_acquire(4096, Importance::Normal).unwrap();
                let d = producer.try_acquire(4096, Importance::Important).unwrap();
                meta.schema_id = 0;
                let error = producer.try_record_raw(meta, a).unwrap_err();
                assert_eq!(error.reason, CaptureError::SchemaMismatch);
                drop(error.loan);
                drop(b);
                drop(c);
                drop(d);
                assert_eq!(consumer.raw_snapshot().unwrap().return_pending, 4);
                assert_eq!(consumer.reclaim_raw(4).reclaimed, 4);
                meta.schema_id = 99;
                for _ in 0..10 {
                    let mut loan = producer.try_acquire(4096, Importance::Normal).unwrap();
                    loan.payload_mut().fill(7);
                    assert!(matches!(
                        producer.try_record_raw(meta, loan),
                        Ok(RawRecordOutcome::Accepted(_))
                    ));
                    let record = consumer.try_take_raw().unwrap().unwrap();
                    assert_eq!(record.payload()[0], 7);
                    assert_eq!(consumer.reclaim_raw(4).reclaimed, 0);
                    drop(record);
                    assert_eq!(consumer.reclaim_raw(4).reclaimed, 1);
                }
                let loan = producer.try_acquire(64, Importance::Normal).unwrap();
                meta.level = Level::Debug;
                let RawRecordOutcome::Skipped(loan) = producer.try_record_raw(meta, loan).unwrap()
                else {
                    panic!("unexpected acceptance")
                };
                drop(loan);
                consumer.reclaim_raw(4);
                meta.level = Level::Info;
                for _ in 0..6 {
                    producer.try_record_small(meta, b"fill").unwrap();
                }
                let loan = producer.try_acquire(4096, Importance::Normal).unwrap();
                let error = producer.try_record_raw(meta, loan).unwrap_err();
                assert_eq!(error.reason, CaptureError::Capacity);
                drop(error.loan);
                let loan = producer.try_acquire(4096, Importance::Important).unwrap();
                control.close();
                let error = producer.try_record_raw(meta, loan).unwrap_err();
                assert_eq!(error.reason, CaptureError::Closing);
                drop(error.loan);
            });
        }),
        (0, 0)
    );
    assert!(allocator::observe(|| drop(owner)).1 > 0);
}

#[test]
fn routing_general_path_allocations_stay_bounded_and_return_after_repeated_saturation() {
    use dot_logger::routing::{Route, Router, RoutingConfig, Schedule, TargetConfig};
    use std::time::{Duration, Instant};
    let calibration = allocator::observe_memory(|| {
        let bytes = vec![1u8; 37];
        std::hint::black_box(&bytes);
        drop(bytes);
    });
    assert_eq!(calibration, (0, 37));
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
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 4,
                slot_bytes: 16,
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
    };
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
    owner.scope(|mut scope| {
        let (mut producer, consumer) = scope.next().unwrap();
        let mut reservation = 0;
        let metadata = RecordMetadata {
            channel_id: 7,
            schema_id: 99,
            schema_version: 1,
            mono_ns: 0,
            real_ns: 0,
            source_time: None,
            level: Level::Info,
            importance: None,
        };
        let now = Instant::now();
        let mut events = [None; 8];
        let (remaining, peak) = allocator::observe_memory(|| {
        let config = RoutingConfig {
            targets: (0..2)
                .map(|id| TargetConfig {
                    id,
                    required: id == 0,
                    queue_slots: 1,
                    pending_slots: 1,
                    retained_bytes: 32,
                    max_attempts: 2,
                    max_age: Duration::from_secs(1),
                })
                .collect(),
            routes: vec![Route {
                channel_id: 7,
                targets: vec![0, 1],
            }],
            shared_records: 2,
            small_copy_bytes: 32,
            schedule: Schedule {
                record_quantum: 2,
                byte_quantum: 32,
                records_per_turn: 2,
                bytes_per_turn: 32,
                retries_per_target: 1,
                reclaim_slots_per_producer: 1,
            },
        };
        let mut router = Router::new(&plan, vec![consumer], config).unwrap();
        reservation = router.scope(|dispatch, _| dispatch.snapshot().output_reserved_bytes);
            router.scope(|mut dispatch, mut outputs| {
                for _ in 0..256 {
                    for _ in 0..2 {
                        producer.try_record_small(metadata, &[5; 16]).unwrap();
                    }
                    assert_eq!(dispatch.turn(now, &mut events).unwrap().records, 2);
                    // Both full queues exhaust their pending retry, without leaking shared copies.
                    dispatch.turn(now, &mut events).unwrap();
                    for output in &mut outputs {
                        drop(output.try_next(now).unwrap().unwrap());
                    }
                    assert_eq!(dispatch.snapshot().live_records, 0);
                }
            });
        });
        assert_eq!(remaining, 0);
        assert!(peak > 0 && peak <= reservation as i128);
        println!("routing saturation: 256 cycles, 512 records, requested-byte delta={remaining}, peak={peak}, output reservation={reservation}");
    });
}

#[path = "../../../crates/dot-logger/tests/support/mod.rs"]
mod tracked_fixtures;

#[test]
fn tracked_rt_admission_full_window_raw_rejection_skip_and_last_drop_never_allocate_or_free() {
    use dot_logger::completion::{Boundary, Update};
    let plan = tracked_fixtures::plan();
    let route = tracked_fixtures::routing();
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = Level::Info;
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[channel],
        &route,
        tracked_fixtures::tracking(2),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let completion = scope.completion().unwrap();
        let (mut producer, mut reader) = scope.next().unwrap();
        let mut receipts = [None; 2];
        let mut scratch = [0; 64];
        let calls = allocator::observe(|| {
            let mut loan = producer.try_acquire(64, Importance::Normal).unwrap();
            loan.payload_mut().fill(31);
            let dot_logger::pool::RawRecordOutcome::Accepted(raw) = producer
                .try_record_raw(tracked_fixtures::meta(), loan)
                .unwrap()
            else {
                panic!("unexpected skip")
            };
            receipts[0] = Some(raw);
            let RecordOutcome::Accepted(small) = producer
                .try_record_small(tracked_fixtures::meta(), &[1])
                .unwrap()
            else {
                panic!("unexpected skip")
            };
            receipts[1] = Some(small);
            reader.try_read(&mut scratch).unwrap().unwrap();
            reader.try_read(&mut scratch).unwrap().unwrap();
            assert_eq!(
                producer.try_record_small(tracked_fixtures::meta(), &[2]),
                Err(CaptureError::CompletionCapacity)
            );
            let loan = producer.try_acquire(64, Importance::Normal).unwrap();
            let identity = loan.identity();
            let ptr = loan.payload().as_ptr();
            let error = producer
                .try_record_raw(tracked_fixtures::meta(), loan)
                .unwrap_err();
            assert_eq!(error.reason, CaptureError::CompletionCapacity);
            assert_eq!(error.loan.identity(), identity);
            assert_eq!(error.loan.payload().as_ptr(), ptr);
            drop(error.loan);
            let mut skipped = tracked_fixtures::meta();
            skipped.level = Level::Trace;
            assert_eq!(
                producer.try_record_small(skipped, &[0]),
                Ok(RecordOutcome::Skipped)
            );
            assert_eq!(reader.reclaim_raw(2).reclaimed, 2);
            let _last_producer = producer;
        });
        assert_eq!(calls, (0, 0));
        for receipt in receipts.into_iter().flatten() {
            for id in [10, 20] {
                completion
                    .report(receipt, id, Update::Confirmed(Boundary::Written))
                    .unwrap();
            }
        }
    });
}

#[test]
fn important_completion_protection_small_and_raw_paths_never_allocate_or_free() {
    let mut config = tracked_fixtures::plan().config().clone();
    config.importance = ImportancePolicy::PrioritizeImportant;
    config.producers[0].small.important_slots = 1;
    config.producers[0].raw.as_mut().unwrap().important_slots = 1;
    let plan = SessionBuilder::new(config)
        .validate(&PlatformCapabilities::default())
        .unwrap();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &tracked_fixtures::routing(),
        tracked_fixtures::tracking(4),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let (mut p, _) = scope.next().unwrap();
        let calls = allocator::observe(|| {
            for _ in 0..3 {
                p.try_record_small(tracked_fixtures::meta(), &[1]).unwrap();
            }
            assert_eq!(
                p.try_record_small(tracked_fixtures::meta(), &[0]),
                Err(CaptureError::CompletionCapacity)
            );
            let mut important = tracked_fixtures::meta();
            important.importance = Some(Importance::Important);
            let loan = p.try_acquire(64, Importance::Important).unwrap();
            assert!(matches!(
                p.try_record_raw(important, loan).unwrap(),
                dot_logger::pool::RawRecordOutcome::Accepted(_)
            ));
            let loan = p.try_acquire(64, Importance::Important).unwrap();
            let identity = loan.identity();
            let pointer = loan.payload().as_ptr();
            let error = p.try_record_raw(important, loan).unwrap_err();
            assert_eq!(error.reason, CaptureError::CompletionCapacity);
            assert_eq!(error.loan.identity(), identity);
            assert_eq!(error.loan.payload().as_ptr(), pointer);
            drop(error.loan);
            let _last_producer = p;
        });
        assert_eq!(calls, (0, 0));
    });
}

#[test]
fn tracked_session_repeated_query_sync_and_shutdown_stay_within_concrete_reservations() {
    use dot_logger::completion::{Boundary, ProducerRange, TargetRequest, Update};
    use dot_logger::control::{
        ControlConfig, ControlOwner, ShutdownPlan, ShutdownPoll, SyncResult,
    };
    use dot_logger::routing::Router;
    use std::time::{Duration, Instant};
    let plan = tracked_fixtures::plan();
    let route = tracked_fixtures::routing();
    let mut concrete = 0;
    let now = Instant::now();
    let (remaining, peak) = allocator::observe_memory(|| {
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            42,
            1,
            &[ChannelConfig::new(7, 99, 1)],
            &route,
            tracked_fixtures::tracking(2),
        )
        .unwrap();
        concrete = owner.input_reserved_bytes()
            + owner.raw_reserved_bytes()
            + owner.control_reserved_bytes();
        owner.scope(|mut scope| {
            let completion = scope.completion().unwrap();
            let control = ControlOwner::new(
                &plan,
                scope.control(),
                ControlConfig {
                    sync_slots: 1,
                    work_per_poll: 8,
                },
                now,
            )
            .unwrap();
            concrete += control.reserved_bytes();
            let (mut producer, a) = scope.next().unwrap();
            let (_, b) = scope.next().unwrap();
            let mut router = Router::new(&plan, vec![a, b], route.clone()).unwrap();
            router.scope(|mut dispatcher, mut outputs| {
                concrete += dispatcher.snapshot().output_reserved_bytes;
                let requests = [
                    TargetRequest {
                        target_id: 10,
                        boundary: Boundary::Written,
                    },
                    TargetRequest {
                        target_id: 20,
                        boundary: Boundary::Written,
                    },
                ];
                let mut events = [None; 8];
                let mut page = [None; 2];
                let mut barrier = [ProducerRange::default(); 2];
                for _ in 0..128 {
                    let RecordOutcome::Accepted(receipt) = producer
                        .try_record_small(tracked_fixtures::meta(), &[5; 16])
                        .unwrap()
                    else {
                        panic!()
                    };
                    dispatcher.turn(now, &mut events).unwrap();
                    for output in &mut outputs {
                        let item = output.try_next(now).unwrap().unwrap();
                        item.report(Update::Confirmed(Boundary::Written)).unwrap();
                        drop(item);
                    }
                    let range = [ProducerRange {
                        producer_id: 0,
                        first: receipt.accepted_seq,
                        last: receipt.accepted_seq,
                    }];
                    let frozen = completion.scope(&range).unwrap();
                    let mut query = completion.query(frozen, &requests).unwrap();
                    assert!(query.page(&mut page).unwrap().summary.is_complete());
                    drop(query);
                    let token = control
                        .request_sync(frozen, &[10, 20], now, now + Duration::from_secs(1))
                        .unwrap();
                    for id in [10, 20] {
                        control
                            .next_sync(token, id, now, &mut dispatcher, &mut barrier)
                            .unwrap()
                            .unwrap()
                            .complete(SyncResult::Synced);
                    }
                    control.poll(now).unwrap();
                    assert!(
                        control
                            .sync_snapshot(token, now)
                            .unwrap()
                            .barriers_succeeded()
                    );
                    control.release_sync(token).unwrap();
                }
                let result = control
                    .begin_shutdown(
                        ShutdownPlan { sync_files: false },
                        now,
                        now + Duration::from_secs(1),
                        &mut dispatcher,
                    )
                    .unwrap();
                assert!(matches!(result,ShutdownPoll::Closed(report)if report.is_success()));
            });
        });
    });
    assert_eq!(remaining, 0);
    assert!(peak > 0 && peak <= concrete as i128);
    println!(
        "tracked session: 128 query/sync cycles, requested-byte peak={peak}, concrete reservation={concrete}, final delta={remaining}"
    );
}

#[test]
fn storage_encoder_indexes_and_finish_remain_within_shared_output_reservations() {
    use dot_logger::{
        capture::{CaptureReceipt, CapturedRecord},
        routing::Router,
        storage::{SegmentWriter, StorageChannel, WriterConfig},
    };
    use std::time::{SystemTime, UNIX_EPOCH};
    for (max, channel_count, records, flush_each) in [
        (256usize, 1, 129usize, true),
        (8 * 1024 * 1024, 1, 129, true),
        (256, 128, 4096, true),
        (256, 128, 4096, false),
    ] {
        let plan = SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: 128 * 1024 * 1024,
                input_bytes: 128 * 1024,
                raw_bytes: 0,
                output_bytes: 64 * 1024 * 1024,
                external_bytes: 0,
                control_bytes: 256 * 1024,
                worker_bytes: 256 * 1024,
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
        let registrations: Vec<_> = (0..channel_count)
            .map(|id| ChannelConfig::new(7 + id, 99, 1))
            .collect();
        let mut owner = CaptureOwner::new(&plan, 42, 1, &registrations).unwrap();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-memory-{}-{stamp}.mcap",
            std::process::id()
        ));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let payload = vec![81; max];
        owner.scope(|scope| {
            use dot_logger::routing::{RoutingConfig,TargetConfig,Route,Schedule};
            let route=RoutingConfig{targets:vec![TargetConfig{id:10,required:true,queue_slots:2,pending_slots:0,retained_bytes:512,max_attempts:1,max_age:std::time::Duration::from_secs(1)}],
                routes:registrations.iter().map(|c|Route{channel_id:c.channel_id,targets:vec![10]}).collect(),shared_records:2,small_copy_bytes:512,
                schedule:Schedule{record_quantum:1,byte_quantum:256,records_per_turn:1,bytes_per_turn:256,retries_per_target:1,reclaim_slots_per_producer:1}};
            let router=Router::new(&plan,scope.map(|(_,r)|r).collect(),route).unwrap();
            let budget=router.writer_budget(10).unwrap();let mut concrete=0;let mut returned=None;
            let (remaining,peak)=allocator::observe_memory(|| {
                let config=WriterConfig{max_payload_bytes:max,write_buffer_bytes:4096,chunk_target_bytes:65536,max_records:records,max_chunks:records,max_metadata_bytes:(128*channel_count as usize).max(1024)};
                let channels:Vec<_>=registrations.iter().map(|&channel|StorageChannel{channel,topic:format!("opaque/{}",channel.channel_id),payload_encoding:"application/octet-stream".into(),schema:None}).collect();
                let mut writer=SegmentWriter::new(budget.clone(),config,channels,file).unwrap();concrete=writer.reserved_bytes();
                for sequence in 1..=records as u64 {
                    let bytes=if sequence==1{&payload[..]}else{&payload[..1]};
                    let record=CapturedRecord{receipt:CaptureReceipt{session_id:42,producer_id:0,accepted_seq:sequence},
                        metadata:RecordMetadata{channel_id:7+(sequence as u32-1)%channel_count,schema_id:99,schema_version:1,mono_ns:sequence,real_ns:sequence as i64,source_time:None,level:Level::Info,importance:Some(Importance::Normal)},revision:1,payload_len:bytes.len()};
                    writer.append(record,bytes).unwrap();if flush_each{writer.flush_chunk().unwrap();}
                }
                let finished=writer.finish().unwrap();assert_eq!(finished.index().iter().map(|e|e.record_count).sum::<u64>(),records as u64);
                if flush_each{assert_eq!(finished.index().len(),records);}
                returned=Some(finished.into_inner());
            });
            drop(returned);assert_eq!(remaining,0);assert_eq!(budget.used_bytes(),0);assert!(peak>0 && peak<=concrete as i128);
            println!("storage max_payload={max}, channels={channel_count}, records={records}, flush_each={flush_each}: requested peak={peak}, reservation={concrete}, final delta={remaining}");
        });
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn rotating_catalog_events_and_pins_stay_inside_shared_memory_reservations() {
    use dot_logger::{
        capture::{CaptureReceipt, CapturedRecord},
        routing::{Route, Router, RoutingConfig, Schedule, TargetConfig},
        storage::{
            FileStore, RetentionConfig, RetentionMode, SegmentLog, StorageChannel, WriterConfig,
        },
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    for max in [256usize, 8 * 1024 * 1024] {
        let plan = SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: 128 * 1024 * 1024,
                input_bytes: 128 * 1024,
                raw_bytes: 0,
                output_bytes: 64 * 1024 * 1024,
                external_bytes: 0,
                control_bytes: 256 * 1024,
                worker_bytes: 256 * 1024,
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
        let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "dot-logger-retention-memory-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let payload = vec![42; max];
        owner.scope(|scope| {
            let route=RoutingConfig{targets:vec![TargetConfig{id:10,required:true,queue_slots:2,pending_slots:0,retained_bytes:512,max_attempts:1,max_age:Duration::from_secs(1)}],
                routes:vec![Route{channel_id:7,targets:vec![10]}],shared_records:2,small_copy_bytes:512,
                schedule:Schedule{record_quantum:1,byte_quantum:256,records_per_turn:1,bytes_per_turn:256,retries_per_target:1,reclaim_slots_per_producer:1}};
            let router=Router::new(&plan,scope.map(|(_,r)|r).collect(),route).unwrap();let budget=router.writer_budget(10).unwrap();
            let mut concrete=0;
            let (remaining,peak)=allocator::observe_memory(|| {
                let store=FileStore::new(&root,&budget,||Ok(1024*1024*1024)).unwrap();
                let config=WriterConfig{max_payload_bytes:max,write_buffer_bytes:4096,chunk_target_bytes:65536,max_records:2,max_chunks:2,max_metadata_bytes:1024};
                let retention=RetentionConfig{mode:RetentionMode::Event,disk_bytes:512*1024*1024,disk_reserve_bytes:1024,max_age_ns:1000,max_segment_bytes:32*1024*1024,max_segment_ns:100,
                    max_index_bytes:16384,protected_bytes:256*1024*1024,pre_ns:10,post_ns:10,max_segments:8,max_events:4,max_pins:4,max_pin_bytes:4096,max_pin_ns:100};
                let mut log=SegmentLog::new(budget.clone(),config,vec![StorageChannel{channel:ChannelConfig::new(7,99,1),topic:"opaque".into(),payload_encoding:"opaque".into(),schema:None}],retention,store,0).unwrap();
                concrete=budget.used_bytes();
                for sequence in 1..=5 {
                    let bytes=if sequence==1{&payload[..]}else{&payload[..1]};
                    let record=CapturedRecord{receipt:CaptureReceipt{session_id:42,producer_id:0,accepted_seq:sequence},metadata:RecordMetadata{channel_id:7,schema_id:99,schema_version:1,mono_ns:sequence,real_ns:sequence as i64,source_time:None,level:Level::Info,importance:Some(Importance::Normal)},revision:1,payload_len:bytes.len()};
                    log.append(record,bytes,sequence).unwrap();
                    if sequence==2 {let a=log.protect_event(2,50,2).unwrap();let b=log.protect_event(2,50,2).unwrap();assert_eq!(a.actual,b.actual);}
                    if sequence==3 {let pin=log.pin(0,8,3,10).unwrap();log.consume_pin(&pin,8,3).unwrap();log.release_pin(pin).unwrap();}
                    assert!(budget.used_bytes()<=concrete);
                }
                log.finish(6).unwrap();drop(log);
            });
            assert_eq!(remaining,0);assert_eq!(budget.used_bytes(),0);assert!(peak>0&&peak<=concrete as i128);
            println!("retention payload={max}: peak={peak}, reservation={concrete}, final delta={remaining}");
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn recovery_scratch_and_rebuilt_indexes_stay_inside_shared_output_reservations() {
    use dot_logger::{
        capture::{CaptureReceipt, CapturedRecord},
        routing::{Route, Router, RoutingConfig, Schedule, TargetConfig},
        storage::{
            RecoveryConfig, RecoveryOptions, SegmentWriter, StorageChannel, WriterConfig,
            recover_segment,
        },
    };
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    for max in [256usize, 8 * 1024 * 1024] {
        let plan = SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: 128 * 1024 * 1024,
                input_bytes: 128 * 1024,
                raw_bytes: 0,
                output_bytes: 64 * 1024 * 1024,
                external_bytes: 0,
                control_bytes: 256 * 1024,
                worker_bytes: 256 * 1024,
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
        let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        let root = std::env::temp_dir().join(format!(
            "dot-recovery-memory-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let source_path = root.join("source.active");
        let output_path = root.join("recovered.mcap");
        let payload = vec![128; max];
        owner.scope(|scope| {
            let route=RoutingConfig{targets:vec![TargetConfig{id:10,required:true,queue_slots:2,pending_slots:0,retained_bytes:512,max_attempts:1,max_age:Duration::from_secs(1)}],
                routes:vec![Route{channel_id:7,targets:vec![10]}],shared_records:2,small_copy_bytes:512,
                schedule:Schedule{record_quantum:1,byte_quantum:256,records_per_turn:1,bytes_per_turn:256,retries_per_target:1,reclaim_slots_per_producer:1}};
            let router=Router::new(&plan,scope.map(|(_,r)|r).collect(),route).unwrap();let budget=router.writer_budget(10).unwrap();
            let config=WriterConfig{max_payload_bytes:max,write_buffer_bytes:4096,chunk_target_bytes:65536,max_records:2,max_chunks:2,max_metadata_bytes:1024};
            let channels=||vec![StorageChannel{channel:ChannelConfig::new(7,99,1),topic:"opaque".into(),payload_encoding:"opaque".into(),schema:None}];
            let file=std::fs::OpenOptions::new().read(true).write(true).create_new(true).open(&source_path).unwrap();
            let mut writer=SegmentWriter::new(budget.clone(),config,channels(),file).unwrap();
            writer.append(CapturedRecord{receipt:CaptureReceipt{session_id:42,producer_id:0,accepted_seq:u64::MAX},metadata:RecordMetadata{channel_id:7,schema_id:99,schema_version:1,mono_ns:2,real_ns:2,source_time:None,level:Level::Info,importance:Some(Importance::Normal)},revision:1,payload_len:max},&payload).unwrap();
            writer.flush_chunk().unwrap();drop(writer);
            let mut source=std::fs::File::open(&source_path).unwrap();
            let output=std::fs::OpenOptions::new().read(true).write(true).create_new(true).open(&output_path).unwrap();
            let options=RecoveryOptions{writer:config,limits:RecoveryConfig{max_source_bytes:(max+65536) as u64,max_record_bytes:max+8192,max_chunks:2}};
            let mut concrete=0;let mut returned=None;
            let (remaining,peak)=allocator::observe_memory(||{
                let channels=channels();
                concrete=options.reservation(&channels,1).unwrap()+config.reservation(&channels,1).unwrap();
                let recovered=recover_segment(&mut source,budget.clone(),channels,options,output,&AtomicBool::new(false)).unwrap();
                assert_eq!(recovered.report.records,1);
                returned=Some(recovered.segment.into_inner());
            });
            drop(returned);
            assert_eq!(remaining,0);assert_eq!(budget.used_bytes(),0);assert!(peak>0&&peak<=concrete as i128);
            println!("recovery payload={max}: peak={peak}, reservation={concrete}, final delta={remaining}");
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn query_and_replay_pages_and_large_blobs_stay_inside_external_reservation() {
    use dot_logger::{
        capture::{CaptureReceipt, CapturedRecord},
        query::*,
        routing::{Route, Router, RoutingConfig, Schedule, TargetConfig},
        storage::{SegmentWriter, StorageChannel, WriterConfig},
    };
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;
    for max in [256usize, 8 * 1024 * 1024] {
        let plan = SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: 256 * 1024 * 1024,
                input_bytes: 128 * 1024,
                raw_bytes: 0,
                output_bytes: 64 * 1024 * 1024,
                external_bytes: 128 * 1024 * 1024,
                control_bytes: 256 * 1024,
                worker_bytes: 256 * 1024,
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
        let mut owner = CaptureOwner::new(&plan, 42, 1, &[ChannelConfig::new(7, 99, 1)]).unwrap();
        let path =
            std::env::temp_dir().join(format!("dot-query-memory-{}-{max}", std::process::id()));
        let payload = vec![128; max];
        owner.scope(|scope| {
            let routing = RoutingConfig {
                targets: vec![TargetConfig {
                    id: 10,
                    required: true,
                    queue_slots: 2,
                    pending_slots: 0,
                    retained_bytes: 512,
                    max_attempts: 1,
                    max_age: Duration::from_secs(1),
                }],
                routes: vec![Route {
                    channel_id: 7,
                    targets: vec![10],
                }],
                shared_records: 2,
                small_copy_bytes: 512,
                schedule: Schedule {
                    record_quantum: 1,
                    byte_quantum: 256,
                    records_per_turn: 1,
                    bytes_per_turn: 256,
                    retries_per_target: 1,
                    reclaim_slots_per_producer: 1,
                },
            };
            let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), routing).unwrap();
            let channels = || {
                vec![StorageChannel {
                    channel: ChannelConfig::new(7, 99, 1),
                    topic: "blob".into(),
                    payload_encoding: "opaque".into(),
                    schema: None,
                }]
            };
            let file = std::fs::OpenOptions::new()
                .create_new(true)
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            let mut writer = SegmentWriter::new(
                router.writer_budget(10).unwrap(),
                WriterConfig {
                    max_payload_bytes: max,
                    write_buffer_bytes: 4096,
                    chunk_target_bytes: 65536,
                    max_records: 2,
                    max_chunks: 2,
                    max_metadata_bytes: 1024,
                },
                channels(),
                file,
            )
            .unwrap();
            writer
                .append(
                    CapturedRecord {
                        receipt: CaptureReceipt {
                            session_id: 42,
                            producer_id: 0,
                            accepted_seq: 1,
                        },
                        metadata: RecordMetadata {
                            channel_id: 7,
                            schema_id: 99,
                            schema_version: 1,
                            mono_ns: 1,
                            real_ns: 1,
                            source_time: None,
                            level: Level::Info,
                            importance: Some(Importance::Normal),
                        },
                        revision: 1,
                        payload_len: max,
                    },
                    &payload,
                )
                .unwrap();
            writer.flush_chunk().unwrap();
            let safe = writer.safe_offset();
            drop(writer);
            let source = std::fs::File::open(&path).unwrap();
            let budget = QueryBudget::new(&plan);
            let config = QueryConfig {
                max_segments: 1,
                max_records: 2,
                max_record_bytes: max + 8192,
                max_source_bytes: (max + 65536) as u64,
                max_payload_bytes: max,
                max_metadata_bytes: 1024,
                producers: 1,
                page_records: 2,
                page_bytes: 1024,
                inline_bytes: 256,
                blob_read_bytes: 256,
                read_bytes: (max + 131072) as u64,
            };
            let mut concrete = 0;
            let (remaining, peak) = allocator::observe_memory(|| {
                let registrations = channels();
                concrete = config.reservation(&registrations).unwrap();
                let cancel = AtomicBool::new(false);
                let mut snapshot = Snapshot::new(
                    &budget,
                    42,
                    1,
                    registrations,
                    vec![QuerySource::active(source, 1, safe)],
                    config,
                    0,
                    100,
                    &cancel,
                )
                .unwrap();
                let cursor = snapshot
                    .start(
                        Query {
                            basis: TimeBasis::Mono,
                            from: 0,
                            to: 2,
                            channel: None,
                            minimum_level: Level::Trace,
                        },
                        1,
                        &cancel,
                    )
                    .unwrap();
                let page = snapshot.page(&cursor, 1, &cancel).unwrap();
                assert_eq!(page.records.len(), 1);
                match &page.records[0].payload {
                    Payload::Inline { .. } => {
                        assert_eq!(page.inline(&page.records[0]).unwrap(), payload)
                    }
                    Payload::Blob(handle) => {
                        let handle = handle.clone();
                        assert_eq!(
                            snapshot.read_blob(&handle, 0, 256, 1, &cancel).unwrap(),
                            &payload[..256]
                        );
                    }
                }
                let mut replay = dot_logger::replay::Replay::new(
                    snapshot,
                    Query { basis: TimeBasis::Mono, from: 1, to: 2, channel: None, minimum_level: Level::Trace },
                    dot_logger::replay::Speed::new(1, 1).unwrap(), 1, &cancel,
                ).unwrap();
                for _ in 0..16 {
                    replay.poll(1, false, &cancel).unwrap();
                }
                match replay.poll(1, true, &cancel).unwrap() {
                    dot_logger::replay::ReplayPoll::Ready(frame) => match frame.payload {
                        dot_logger::replay::ReplayPayload::Inline(bytes) => assert_eq!(bytes, payload),
                        dot_logger::replay::ReplayPayload::Blob(handle) => {
                            let handle=handle.clone();let generation=frame.generation;
                            assert_eq!(replay.read_blob(generation,&handle,0,256,1,&cancel).unwrap(),&payload[..256]);
                        }
                    },
                    _=>panic!("expected replay frame"),
                }
                replay.stop();
            });
            assert_eq!(remaining, 0);
            assert_eq!(budget.used_bytes(), 0);
            assert!(peak > 0 && peak <= concrete as i128);
            println!(
                "query/replay payload={max}: peak={peak}, reservation={concrete}, final delta={remaining}"
            );
        });
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(feature = "remote")]
#[test]
fn remote_receiver_and_hub_buffers_stay_in_shared_external_reservation() {
    use dot_logger::{
        adapters::remote::*,
        capture::{CaptureReceipt, CapturedRecord},
        query::QueryBudget,
    };
    use std::time::{Duration, Instant};
    for max in [256, 8 * 1024 * 1024] {
        let plan = SessionBuilder::new(SessionConfig {
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
                    slots: 2,
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
        let payload = vec![9; max];
        let record = CapturedRecord {
            receipt: CaptureReceipt {
                session_id: 42,
                producer_id: 0,
                accepted_seq: 1,
            },
            metadata: RecordMetadata {
                channel_id: 7,
                schema_id: 99,
                schema_version: 1,
                mono_ns: 1,
                real_ns: 1,
                source_time: None,
                level: Level::Info,
                importance: Some(Importance::Normal),
            },
            revision: 1,
            payload_len: max,
        };
        let checksum = Chunk::single(1, 1, 1, record, &payload, true).checksum;
        let budget = QueryBudget::new(&plan);
        let mut concrete = 0;
        let (remaining, peak) = allocator::observe_memory(|| {
            let limits = RemoteLimits {
                connections: 1,
                subscriptions: 1,
                commands: 1,
                message_bytes: MAX_CHUNK_BYTES + 1024,
                record_bytes: max,
                subscription_records: 2,
                subscription_bytes: max * 2,
                max_age: Duration::from_secs(10),
                request_timeout: Duration::from_secs(1),
            };
            let (hub, inbox) = RemoteHub::new(&budget, limits).unwrap();
            let mut receiver = Reassembler::new(&budget, 1, max, Duration::from_secs(10)).unwrap();
            concrete = budget.used_bytes();
            for (i, bytes) in payload.chunks(MAX_CHUNK_BYTES).enumerate() {
                let mut chunk = Chunk::single(1, 1, 1, record, bytes, true);
                chunk.total = max as u64;
                chunk.offset = (i * MAX_CHUNK_BYTES) as u64;
                chunk.checksum = checksum;
                if let Some(ready) = receiver.push(chunk, Instant::now()).unwrap() {
                    let ready = ready.handoff(|_, _| false).err().unwrap();
                    let ack = ready
                        .handoff(|r, data| r.receipt == record.receipt && data == payload)
                        .ok()
                        .unwrap();
                    assert_eq!(ack.transfer, 1);
                }
            }
            drop((receiver, hub, inbox));
        });
        assert_eq!(remaining, 0);
        assert_eq!(budget.used_bytes(), 0);
        assert!(peak > 0 && peak <= concrete as i128);
        println!(
            "remote core payload={max}: peak={peak}, reservation={concrete}, final delta={remaining}"
        );
    }
}
