//! TASK-023 S2: logger-on storage runs through the public facade only
//! (`Logger::builder` → `Writer::try_log` → `shutdown`), under the same
//! `StorageWorkload` as the P0 simple-file writer.
//!
//! Mapping to P0: record `s` goes to channel `s % channels`, channel `c` is
//! routed to output `c % files` only, so record `s` lands in file `s % files`
//! exactly as in P0. P0 writer threads correspond to logger IO threads;
//! producer threads are a logger-side load (`LoggerLoad`) with record `s` sent
//! by producer `s % producers`.
use super::{
    Error, LatencyHistogram, SEQUENCE_BYTES, SequenceOracle, Stop, StorageRun, StorageWorkload,
    Tool, available_bytes, check_p1_payload, invalid, now_ns, p1_payload, payload_sequence, pin,
};
use dot_logger::capture::{CaptureError, Level, RecordMetadata, RecordOutcome};
use dot_logger::logger::{
    Budget, Channel, ChannelOptions, EncodeError, LogError, Logged, Logger, Record, Retention,
    Schema, Shutdown, WriterQuota,
};
use dot_logger::platform::Requirement;
use dot_logger::storage::decode_record;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const MIB: u64 = 1024 * 1024;
/// Facade segment bounds (API §19, `budget.rs`): segment = clamp(retention / 8,
/// floor, ceiling), floor = max(8 MiB, 2 × largest payload + 1 MiB).
const SEGMENT_FLOOR: u64 = 8 * MIB;
const SEGMENT_CEILING: u64 = 256 * MIB;
/// Shutdown is given ample time; the durable time is measured, not bounded.
const SHUTDOWN: Duration = Duration::from_secs(600);
const DROP_DEADLINE: Duration = Duration::from_secs(30);

/// Harness record: its sequence and the P0 fill pattern, written straight
/// into the writer's encode scratch (one small slot = the payload size).
struct Seq(u64);
impl Record for Seq {
    const SCHEMA: Schema = Schema::new(0x0053_4551_5045_5246, 1, "dot-logger-perf-seq");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        p1_payload(self.0, out).ok_or(EncodeError)
    }
}

/// Logger-side load around a shared `StorageWorkload`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggerLoad {
    /// Producer threads, each owning one `Writer`.
    pub producers: usize,
    /// Registered channels; must be a multiple of the workload's files.
    pub channels: usize,
    /// Small slots per producer.
    pub slots: usize,
    /// `Budget::memory_bytes`.
    pub memory_bytes: usize,
    /// Producer `p` pins to `producer_cpus[p % len]` when not empty.
    pub producer_cpus: Vec<usize>,
    /// `Budget::worker_cpus` (Optional) when not empty.
    pub worker_cpus: Vec<usize>,
}

/// Facts of one logger-on run.
#[derive(Clone, Debug)]
pub struct LoggerRun {
    /// Storage facts under the shared workload (`tool = Logger`).
    pub storage: StorageRun,
    /// Logger-side load.
    pub load: LoggerLoad,
    /// Duration of every accepted `try_log` call.
    pub accepted: LatencyHistogram,
    /// Duration of every rejected (`Capacity`) `try_log` call.
    pub rejected: LatencyHistogram,
    /// Capacity rejections; each was retried until accepted.
    pub rejections: u64,
    /// `plan().required_bytes` of the declared budget.
    pub required_bytes: usize,
    /// Shutdown report (`Debug`).
    pub report: String,
    /// Session namespace directory of every output, in file order.
    pub directories: Vec<PathBuf>,
}

/// Records and bytes read back from a logger run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoggerVerified {
    /// Records whose identity, placement and content matched the oracle.
    pub records: u64,
    /// Their payload bytes.
    pub payload_bytes: u64,
    /// Closed MCAP files read.
    pub files: u64,
    /// Largest closed file.
    pub largest_file: u64,
}

fn topic(channel: usize) -> String {
    format!("perf/{channel:03}")
}

/// Retention per output that makes the facade segment equal the workload's
/// file limit, and still holds the run's volume.
fn retention_bytes(w: &StorageWorkload) -> Result<u64, Error> {
    let payload = w.record_payload_bytes as u64;
    let floor = SEGMENT_FLOOR.max(payload.saturating_mul(2).saturating_add(MIB));
    let ceiling = SEGMENT_CEILING.max(floor);
    if !(floor..=ceiling).contains(&w.file_limit_bytes) {
        return Err(invalid("file limit outside the facade segment range"));
    }
    // Encoding at most doubles small records; two more segments cover the
    // active one and rotation.
    let need = w
        .records
        .div_ceil(w.files as u64)
        .saturating_mul(payload)
        .saturating_mul(2)
        .saturating_add(w.file_limit_bytes.saturating_mul(2));
    let exact = w.file_limit_bytes.saturating_mul(8);
    if w.file_limit_bytes == ceiling {
        Ok(exact.max(need))
    } else if need <= exact {
        Ok(exact)
    } else {
        Err(invalid(
            "per-file volume exceeds 8 × file limit; use the ceiling",
        ))
    }
}

struct Produced {
    accepted: LatencyHistogram,
    rejected: LatencyHistogram,
    rejections: u64,
    accepted_ns: u64,
}

/// Run the workload through the facade into the exclusive `dir`. Every
/// rejected call is retried, so a completed run admitted every record.
pub fn write_logger(
    dir: &Path,
    w: StorageWorkload,
    load: &LoggerLoad,
    reserve_bytes: u64,
) -> Result<LoggerRun, Error> {
    if w.record_payload_bytes < SEQUENCE_BYTES
        || w.files == 0
        || !(1..=w.files).contains(&w.writers)
        || load.producers == 0
        || load.channels < w.files
        || load.channels % w.files != 0
        || load.slots == 0
        || w.offered_bytes_per_second == Some(0)
    {
        return Err(invalid("invalid logger workload"));
    }
    let cap = retention_bytes(&w)?;
    let total = cap.saturating_mul(w.files as u64);
    if available_bytes(dir)?
        < reserve_bytes.saturating_add(
            total.min(
                w.records
                    .saturating_mul(w.record_payload_bytes as u64)
                    .saturating_mul(2),
            ),
        )
    {
        return Err(invalid("not enough free space above the reserve"));
    }
    let mut budget = Budget::new()
        .memory_bytes(load.memory_bytes)
        .sync_every(w.sync_interval)
        .drop_deadline(DROP_DEADLINE)
        .writer_quota(WriterQuota::small(load.slots, w.record_payload_bytes))
        .io_threads(w.writers);
    if !load.worker_cpus.is_empty() {
        budget = budget.worker_cpus(&load.worker_cpus, Requirement::Optional);
    }
    let mut builder = Logger::builder().budget(budget);
    for f in 0..w.files {
        let out = dir.join(format!("out-{f:03}"));
        fs::create_dir(&out)?;
        builder = builder.file_output(&out, Retention::continuous(cap).reserve(reserve_bytes));
    }
    for c in 0..load.channels {
        builder =
            builder.channel_with::<Seq>(&topic(c), ChannelOptions::new().outputs(&[c % w.files]));
    }
    for p in 0..load.producers {
        builder = builder.writer(&format!("p{p:03}"));
    }
    let required_bytes = builder.plan()?.required_bytes;
    let logger = builder.start()?;
    let directories = logger.directories().to_vec();
    let channels = (0..load.channels)
        .map(|c| logger.channel::<Seq>(&topic(c)))
        .collect::<Result<Vec<Channel<Seq>>, _>>()
        .map_err(|e| format!("channel lookup: {e:?}"))?;
    let mut writers = Vec::with_capacity(load.producers);
    for p in 0..load.producers {
        writers.push(
            logger
                .take_writer(&format!("p{p:03}"))
                .map_err(|e| format!("writer lookup: {e:?}"))?,
        );
    }
    // The run starts with a running logger: preallocation and namespace
    // creation are setup, as opening files is for P0.
    let start = Instant::now();
    let produced: Vec<Result<Produced, Error>> = std::thread::scope(|scope| {
        let handles: Vec<_> = writers
            .into_iter()
            .enumerate()
            .map(|(p, mut writer)| {
                let channels = &channels;
                let cpu = (!load.producer_cpus.is_empty())
                    .then(|| load.producer_cpus[p % load.producer_cpus.len()]);
                let producers = load.producers as u64;
                scope.spawn(move || -> Result<Produced, Error> {
                    if let Some(cpu) = cpu {
                        pin(cpu)?;
                    }
                    let share = w
                        .offered_bytes_per_second
                        .map(|rate| rate as f64 / producers as f64);
                    let mut out = Produced {
                        accepted: LatencyHistogram::default(),
                        rejected: LatencyHistogram::default(),
                        rejections: 0,
                        accepted_ns: 0,
                    };
                    let mut sent = 0_u64;
                    let mut seq = p as u64;
                    while seq < w.records {
                        if let Some(share) = share {
                            // Sleep only when more than 1 ms ahead of schedule.
                            let due = Duration::from_secs_f64(sent as f64 / share);
                            let ahead = due.saturating_sub(start.elapsed());
                            if ahead > Duration::from_millis(1) {
                                std::thread::sleep(ahead);
                            }
                        }
                        let channel = channels[(seq % channels.len() as u64) as usize];
                        loop {
                            let t0 = now_ns();
                            let result = writer.try_log(channel, &Seq(seq));
                            let spent = now_ns().saturating_sub(t0);
                            match result {
                                Ok(Logged::Accepted(_)) => {
                                    out.accepted.record(spent);
                                    break;
                                }
                                Ok(Logged::Skipped) => return Err(invalid("record skipped")),
                                Err(e) if e.is_capacity() => {
                                    out.rejected.record(spent);
                                    out.rejections += 1;
                                    std::thread::yield_now();
                                }
                                Err(LogError::Closing) => {
                                    return Err(invalid("logger closed during the run"));
                                }
                                Err(e) => return Err(format!("try_log: {e:?}").into()),
                            }
                        }
                        out.accepted_ns = u64::try_from(start.elapsed().as_nanos())?;
                        sent += w.record_payload_bytes as u64;
                        seq += producers;
                    }
                    drop(writer);
                    Ok(out)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(invalid("producer panicked")))
            })
            .collect()
    });
    let mut accepted = LatencyHistogram::default();
    let mut rejected = LatencyHistogram::default();
    let mut rejections = 0;
    let mut accepted_ns = 0;
    let mut failure = None;
    for p in produced {
        match p {
            Ok(p) => {
                accepted.merge(&p.accepted);
                rejected.merge(&p.rejected);
                rejections += p.rejections;
                accepted_ns = accepted_ns.max(p.accepted_ns);
            }
            Err(e) => failure = failure.or(Some(e)),
        }
    }
    // Shutdown runs even after a producer failure, so nothing is left running.
    let shutdown = logger.shutdown(SHUTDOWN);
    let durable_ns = u64::try_from(start.elapsed().as_nanos())?;
    if let Some(e) = failure {
        return Err(e);
    }
    let report = match shutdown {
        Ok(Shutdown::Closed(report)) => report,
        Ok(Shutdown::Incomplete { report, .. }) => {
            return Err(format!("shutdown incomplete: {report:?}").into());
        }
        Err(e) => return Err(format!("shutdown failed: {e:?}").into()),
    };
    let (files_created, file_bytes, _) = closed_files(&directories)?;
    let payload_bytes = w.records * w.record_payload_bytes as u64;
    Ok(LoggerRun {
        storage: StorageRun {
            tool: Tool::Logger,
            workload: w,
            payload_bytes,
            framing_bytes: file_bytes.saturating_sub(payload_bytes),
            files_created,
            writes: None,
            periodic_syncs: None,
            accepted_ns,
            durable_ns: Some(durable_ns),
            stopped: (!report.is_success()).then_some(Stop::Unsuccessful),
        },
        load: load.clone(),
        accepted,
        rejected,
        rejections,
        required_bytes,
        report: format!("{report:?}"),
        directories,
    })
}

/// Closed MCAP files of every output: count, total bytes, per-output lists in
/// segment order. An `.active` segment left behind is an error.
fn closed_files(directories: &[PathBuf]) -> Result<(u64, u64, Vec<Vec<PathBuf>>), Error> {
    let mut count = 0;
    let mut bytes = 0;
    let mut lists = Vec::with_capacity(directories.len());
    for dir in directories {
        let mut files = Vec::new();
        for e in fs::read_dir(dir)? {
            let path = e?.path();
            match path.extension().and_then(|x| x.to_str()) {
                Some("mcap") => {
                    bytes += fs::metadata(&path)?.len();
                    files.push(path);
                }
                Some("active") => return Err(invalid("active segment after Closed")),
                _ => {}
            }
        }
        files.sort_by_key(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<u64>().ok())
        });
        count += files.len() as u64;
        lists.push(files);
    }
    Ok((count, bytes, lists))
}

/// Read every stored record back: each sequence exactly once, in its file and
/// channel, increasing per (producer, file), with the exact payload.
pub fn verify_logger(run: &LoggerRun) -> Result<LoggerVerified, Error> {
    let w = run.storage.workload;
    let channels = run.load.channels as u64;
    let mut oracle = SequenceOracle::new(w.records, w.files, run.load.producers);
    let (files, _, lists) = closed_files(&run.directories)?;
    let mut v = LoggerVerified {
        records: 0,
        payload_bytes: 0,
        files,
        largest_file: 0,
    };
    for (f, list) in lists.iter().enumerate() {
        for path in list {
            let bytes = fs::read(path)?;
            v.largest_file = v.largest_file.max(bytes.len() as u64);
            for message in mcap::MessageStream::new(&bytes)? {
                let message = message?;
                let stored = decode_record(&message.data, w.record_payload_bytes)
                    .map_err(|e| format!("decode: {e:?}"))?;
                let payload = &stored.payload;
                let seq = payload_sequence(payload).ok_or(invalid("short payload"))?;
                let channel = u64::from(stored.record.metadata.channel_id);
                if payload.len() != w.record_payload_bytes
                    || !check_p1_payload(seq, payload)
                    || channel != seq % channels + 1
                {
                    return Err(invalid("record differs from the oracle"));
                }
                oracle.see(f, seq)?;
                v.records += 1;
                v.payload_bytes += payload.len() as u64;
            }
        }
    }
    oracle.finish()?;
    if v.payload_bytes != run.storage.payload_bytes || v.files != run.storage.files_created {
        return Err(invalid("read-back totals differ from the run"));
    }
    Ok(v)
}

/// Per-call cost split for backlog T4 (slot direct encode). Each round takes
/// one sample of every kind, interleaved, on one producer thread.
#[derive(Clone, Debug)]
pub struct CallCost {
    /// Payload size of every call.
    pub payload_bytes: usize,
    /// Accepted `Writer::try_log` (clock reads + encode + admission).
    pub try_log: LatencyHistogram,
    /// `Record::encode` alone into a payload-sized buffer.
    pub encode: LatencyHistogram,
    /// Accepted `producer().try_record_small` with a pre-encoded payload
    /// (admission with its copy into the byte ring, no encode, no clock).
    pub record_small: LatencyHistogram,
    /// `copy_from_slice` of one payload: the copy slot encode would remove.
    pub copy: LatencyHistogram,
    /// Two consecutive clock reads: the timing overhead in every sample.
    pub clock: LatencyHistogram,
    /// Shutdown report was a success.
    pub success: bool,
    /// Shutdown report (`Debug`).
    pub report: String,
}

/// Measure `samples` rounds of the call cost split with one output, one
/// channel and one writer whose ring is drained by the default worker.
pub fn call_cost(dir: &Path, payload: usize, samples: u64) -> Result<CallCost, Error> {
    if payload < SEQUENCE_BYTES || samples == 0 {
        return Err(invalid("invalid call cost run"));
    }
    let out = dir.join("out-000");
    fs::create_dir(&out)?;
    let logger = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB as usize)
                .sync_every(Duration::from_secs(1))
                .drop_deadline(DROP_DEADLINE)
                .writer_quota(WriterQuota::small(4096, payload)),
        )
        .file_output(&out, Retention::ring(1024 * MIB))
        .channel::<Seq>(&topic(0))
        .writer("cost")
        .start()?;
    let channel = logger
        .channel::<Seq>(&topic(0))
        .map_err(|e| format!("channel lookup: {e:?}"))?;
    let mut writer = logger
        .take_writer("cost")
        .map_err(|e| format!("writer lookup: {e:?}"))?;
    let mut cost = CallCost {
        payload_bytes: payload,
        try_log: LatencyHistogram::default(),
        encode: LatencyHistogram::default(),
        record_small: LatencyHistogram::default(),
        copy: LatencyHistogram::default(),
        clock: LatencyHistogram::default(),
        success: false,
        report: String::new(),
    };
    let mut buf = vec![0_u8; payload];
    let mut dst = vec![0_u8; payload];
    let mut seq = 0_u64;
    for _ in 0..samples {
        // try_log as an application calls it.
        loop {
            let t0 = now_ns();
            let r = writer.try_log(channel, &Seq(seq));
            let spent = now_ns().saturating_sub(t0);
            match r {
                Ok(Logged::Accepted(_)) => {
                    cost.try_log.record(spent);
                    break;
                }
                Err(e) if e.is_capacity() => std::thread::yield_now(),
                other => return Err(format!("try_log: {other:?}").into()),
            }
        }
        seq += 1;
        // Encode alone.
        let t0 = now_ns();
        let n = Seq(seq).encode(&mut buf);
        cost.encode.record(now_ns().saturating_sub(t0));
        n.map_err(|_| invalid("encode failed"))?;
        // Admission of the pre-encoded payload.
        loop {
            let metadata = RecordMetadata {
                channel_id: 1,
                schema_id: Seq::SCHEMA.id(),
                schema_version: Seq::SCHEMA.version(),
                mono_ns: writer.now_ns(),
                real_ns: 0,
                source_time: None,
                level: Level::Info,
                importance: None,
            };
            let mut producer = writer.producer();
            let t0 = now_ns();
            let r = producer.try_record_small(metadata, &buf);
            let spent = now_ns().saturating_sub(t0);
            match r {
                Ok(RecordOutcome::Accepted(_)) => {
                    cost.record_small.record(spent);
                    break;
                }
                Err(CaptureError::Capacity | CaptureError::CompletionCapacity) => {
                    std::thread::yield_now();
                }
                other => return Err(format!("try_record_small: {other:?}").into()),
            }
        }
        seq += 1;
        // The copy slot direct encode would remove.
        let t0 = now_ns();
        dst.copy_from_slice(std::hint::black_box(&buf));
        std::hint::black_box(&mut dst);
        cost.copy.record(now_ns().saturating_sub(t0));
        // Clock pair overhead.
        let t0 = now_ns();
        cost.clock.record(now_ns().saturating_sub(t0));
    }
    drop(writer);
    match logger.shutdown(SHUTDOWN) {
        Ok(Shutdown::Closed(report)) => {
            cost.success = report.is_success();
            cost.report = format!("{report:?}");
        }
        other => return Err(format!("shutdown: {other:?}").into()),
    }
    Ok(cost)
}
