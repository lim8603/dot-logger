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
    }

    fn record(allocation: bool, deallocation: bool) {
        let _ = ENABLED.try_with(|enabled| {
            if enabled.get() {
                let _ = CALLS.try_with(|calls| {
                    let (a, d) = calls.get();
                    calls.set((a + usize::from(allocation), d + usize::from(deallocation)));
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
            record(true, false);
            // SAFETY: GlobalAlloc's caller supplies the valid layout.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(true, false);
            // SAFETY: Same layout contract as alloc.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            record(false, true);
            // SAFETY: GlobalAlloc's caller supplies a live allocation/layout.
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            record(true, true);
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
        ENABLED.with(|enabled| enabled.set(true));
        let observation = Observation;
        callback();
        drop(observation);
        CALLS.with(Cell::get)
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
