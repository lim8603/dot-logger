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
