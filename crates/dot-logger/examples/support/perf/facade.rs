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
    Error, Gate, LatencyHistogram, SEQUENCE_BYTES, SequenceOracle, Stop, StorageRun,
    StorageWorkload, Tool, available_bytes, check_p1_payload, invalid, now_ns, p1_payload, pace,
    payload_sequence, pin,
};
use dot_logger::capture::{CaptureError, Importance, RecordOutcome};
use dot_logger::logger::{
    Budget, Channel, ChannelOptions, EncodeError, LogError, Logged, Logger, Record, Retention,
    Schema, Shutdown, Writer, WriterQuota,
};
use dot_logger::platform::Requirement;
use dot_logger::pool::RawRecordOutcome;
use dot_logger::storage::NativeReader;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MIB: u64 = 1024 * 1024;
/// Facade segment bounds (API §19, `budget.rs`): segment = clamp(retention / 8,
/// floor, ceiling), floor = max(8 MiB, 2 × largest payload + 1 MiB).
const SEGMENT_FLOOR: u64 = 8 * MIB;
const SEGMENT_CEILING: u64 = 256 * MIB;
/// Records per facade segment (`WriterConfig::max_records` in the facade
/// budget): the facade rotates at this count whatever the byte limit, so a
/// comparable P0 workload rotates at it too (TASK-023 S2b ⑤).
pub const FACADE_SEGMENT_RECORDS: u64 = 16_384;
/// Pause of a load producer after a Capacity rejection before it retries.
/// A bare `yield_now` let 32 rejected producers spin on 16 cores and starve
/// the logger's host thread, so unpaced runs measured the scheduler rather
/// than the logger (EV-061).
const RETRY_BACKOFF: Duration = Duration::from_micros(50);
/// Shutdown is given ample time; the durable time is measured, not bounded.
const SHUTDOWN: Duration = Duration::from_secs(600);
const DROP_DEADLINE: Duration = Duration::from_secs(30);

/// Producer of record `s`: large stream `j` has its own raw producer `j`;
/// small records rotate over the remaining producers.
fn producer_of(w: &StorageWorkload, producers: usize, s: u64) -> usize {
    let streams = w.large.map_or(0, |l| l.streams);
    match w.stream_of(s) {
        Some(j) => j,
        None => streams + (s % (producers - streams).max(1) as u64) as usize,
    }
}
/// Channel index of record `s`: routed to output `channel % files`, which is
/// `file_of(s)` for both kinds.
fn channel_of(w: &StorageWorkload, channels: usize, s: u64) -> usize {
    match w.stream_of(s) {
        Some(_) => w.file_of(s),
        None => (s % channels as u64) as usize,
    }
}
/// Records of producer `p` in increasing order, without scanning the others.
fn records_of(w: StorageWorkload, producers: usize, p: usize) -> Box<dyn Iterator<Item = u64>> {
    match w.large {
        Some(l) if p < l.streams => Box::new(
            (p as u64..)
                .step_by(l.streams)
                .map(move |n| n * l.every)
                .take_while(move |&s| s < w.records),
        ),
        large => {
            let streams = large.map_or(0, |l| l.streams);
            let share = (producers - streams) as u64;
            Box::new(
                ((p - streams) as u64..w.records)
                    .step_by(share as usize)
                    .filter(move |&s| w.stream_of(s).is_none()),
            )
        }
    }
}
fn wall_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
}

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
    /// Raw (zero-copy original) slots of each large-stream producer.
    pub raw_slots: usize,
    /// `Budget::memory_bytes`.
    pub memory_bytes: usize,
    /// Producer `p` pins to `producer_cpus[p % len]` when not empty.
    pub producer_cpus: Vec<usize>,
    /// `Budget::worker_cpus` (Optional) when not empty.
    pub worker_cpus: Vec<usize>,
    /// Opt-in telemetry (`LoggerBuilder::telemetry`), for the instrumentation
    /// cost comparison (TASK-023 S3, ≤ 5 %).
    pub telemetry: bool,
}

/// Facts of one logger-on run.
#[derive(Clone, Debug)]
pub struct LoggerRun {
    /// Storage facts under the shared workload (`tool = Logger`).
    pub storage: StorageRun,
    /// Logger-side load.
    pub load: LoggerLoad,
    /// Duration of every accepted `try_log` call (small records).
    pub accepted: LatencyHistogram,
    /// Duration of every accepted `try_record_raw` call (large records;
    /// acquiring and filling the loan are the application's own work).
    pub raw_accepted: LatencyHistogram,
    /// Duration of every rejected (`Capacity`) call of either kind.
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
    /// Most records found in one file.
    pub largest_file_records: u64,
}

fn topic(channel: usize) -> String {
    format!("perf/{channel:03}")
}

/// Upper bound of the payload one file receives: its share of the small
/// records plus, on a stream's file, that stream's large records.
fn max_file_payload(w: &StorageWorkload) -> u64 {
    let small = w
        .records
        .div_ceil(w.files as u64)
        .saturating_mul(w.record_payload_bytes as u64);
    let large = w.large.map_or(0, |l| {
        w.records
            .div_ceil(l.every)
            .div_ceil(l.streams as u64)
            .saturating_mul(l.bytes as u64)
    });
    small.saturating_add(large)
}

/// Retention per output that makes the facade segment equal the workload's
/// file limit, and still holds the run's volume.
fn retention_bytes(w: &StorageWorkload) -> Result<u64, Error> {
    let payload = w.max_payload() as u64;
    let floor = SEGMENT_FLOOR.max(payload.saturating_mul(2).saturating_add(MIB));
    let ceiling = SEGMENT_CEILING.max(floor);
    if !(floor..=ceiling).contains(&w.file_limit_bytes) {
        return Err(invalid("file limit outside the facade segment range"));
    }
    // Encoding at most doubles small records; two more segments cover the
    // active one and rotation.
    let need = max_file_payload(w)
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

#[derive(Default)]
struct Produced {
    accepted: LatencyHistogram,
    raw_accepted: LatencyHistogram,
    rejected: LatencyHistogram,
    rejections: u64,
    accepted_ns: u64,
}

/// One small record through `try_log`, retried while `Capacity`.
fn log_small(
    writer: &mut Writer,
    channel: Channel<Seq>,
    seq: u64,
    out: &mut Produced,
) -> Result<(), Error> {
    loop {
        let t0 = now_ns();
        let result = writer.try_log(channel, &Seq(seq));
        let spent = now_ns().saturating_sub(t0);
        match result {
            Ok(Logged::Accepted(_)) => {
                out.accepted.record(spent);
                return Ok(());
            }
            Ok(Logged::Skipped) => return Err(invalid("record skipped")),
            Err(e) if e.is_capacity() => {
                out.rejected.record(spent);
                out.rejections += 1;
                std::thread::sleep(RETRY_BACKOFF);
            }
            Err(LogError::Closing) => return Err(invalid("logger closed during the run")),
            Err(e) => return Err(format!("try_log: {e:?}").into()),
        }
    }
}
/// One large record as a zero-copy loan from `Writer::producer()`, filled in
/// place like a sensor frame and submitted with the channel's metadata
/// (API §18.3); both steps retried while `Capacity`.
fn log_raw(
    writer: &mut Writer,
    channel: Channel<Seq>,
    seq: u64,
    len: usize,
    out: &mut Produced,
) -> Result<(), Error> {
    let metadata = channel.metadata(writer.now_ns(), wall_ns());
    let mut producer = writer.producer();
    let mut loan = loop {
        match producer.try_acquire(len, Importance::Normal) {
            Ok(loan) => break loan,
            Err(CaptureError::Capacity) => {
                out.rejections += 1;
                std::thread::sleep(RETRY_BACKOFF);
            }
            Err(e) => return Err(format!("try_acquire: {e:?}").into()),
        }
    };
    p1_payload(seq, loan.payload_mut()).ok_or(invalid("short raw payload"))?;
    loop {
        let t0 = now_ns();
        let result = producer.try_record_raw(metadata, loan);
        let spent = now_ns().saturating_sub(t0);
        match result {
            Ok(RawRecordOutcome::Accepted(_)) => {
                out.raw_accepted.record(spent);
                return Ok(());
            }
            Ok(RawRecordOutcome::Skipped(_)) => return Err(invalid("record skipped")),
            Err(e)
                if matches!(
                    e.reason,
                    CaptureError::Capacity | CaptureError::CompletionCapacity
                ) =>
            {
                out.rejected.record(spent);
                out.rejections += 1;
                loan = e.loan;
                std::thread::sleep(RETRY_BACKOFF);
            }
            Err(e) => return Err(format!("try_record_raw: {:?}", e.reason).into()),
        }
    }
}

/// Run the workload through the facade into the exclusive `dir`. Every
/// rejected call is retried, so a completed run admitted every record.
/// Each producer thread is one `gate` load party: planning, logger start,
/// file creation, lookups, thread start and pinning are setup before the
/// shared start, exactly as P0's writer threads open their files first.
pub fn write_logger(
    dir: &Path,
    w: StorageWorkload,
    load: &LoggerLoad,
    reserve_bytes: u64,
    gate: &Gate,
) -> Result<LoggerRun, Error> {
    w.validate()?;
    let streams = w.large.map_or(0, |l| l.streams);
    // Raw streams need their own producers and slots; small records need a
    // producer besides them unless every record is large.
    let small_records = w.large.is_none_or(|l| l.every > 1);
    if w.record_payload_bytes < SEQUENCE_BYTES
        || w.max_payload() < SEQUENCE_BYTES
        || !(1..=w.files).contains(&w.writers)
        || load.producers == 0
        || load.producers < streams + usize::from(small_records)
        || (streams > 0 && load.raw_slots == 0)
        || load.channels < w.files
        || load.channels % w.files != 0
        || load.slots == 0
        || w.offered_bytes_per_second == Some(0)
        || w.file_record_limit != FACADE_SEGMENT_RECORDS
        || gate.load_parties() != load.producers
    {
        return Err(invalid("invalid logger workload"));
    }
    let cap = retention_bytes(&w)?;
    let total = cap.saturating_mul(w.files as u64);
    if available_bytes(dir)?
        < reserve_bytes.saturating_add(total.min(w.total_payload_bytes().saturating_mul(2)))
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
    if load.telemetry {
        builder = builder.telemetry();
    }
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
        let name = format!("p{p:03}");
        builder = match w.large {
            Some(l) if p < l.streams => builder.writer_with(
                &name,
                WriterQuota::small(load.slots, w.record_payload_bytes)
                    .with_raw(load.raw_slots, l.bytes),
            ),
            _ => builder.writer(&name),
        };
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
    let produced: Vec<Result<Produced, Error>> = std::thread::scope(|scope| {
        let handles: Vec<_> = writers
            .into_iter()
            .enumerate()
            .map(|(p, mut writer)| {
                let channels = &channels;
                let cpu = (!load.producer_cpus.is_empty())
                    .then(|| load.producer_cpus[p % load.producer_cpus.len()]);
                let producers = load.producers;
                scope.spawn(move || -> Result<Produced, Error> {
                    let ((), start) = gate.after(|| match cpu {
                        Some(cpu) => pin(cpu),
                        None => Ok(()),
                    })?;
                    let mut out = Produced {
                        accepted: LatencyHistogram::default(),
                        raw_accepted: LatencyHistogram::default(),
                        rejected: LatencyHistogram::default(),
                        rejections: 0,
                        accepted_ns: 0,
                    };
                    let first = w.first_measured();
                    let mut ignored = Produced::default();
                    for seq in records_of(w, producers, p) {
                        pace(&w, seq, start)?;
                        let channel = channels[channel_of(&w, channels.len(), seq)];
                        // Warm-up records are sent alike but not timed.
                        let stats = if seq < first { &mut ignored } else { &mut out };
                        if w.stream_of(seq).is_some() {
                            log_raw(&mut writer, channel, seq, w.payload_of(seq), stats)?;
                        } else {
                            log_small(&mut writer, channel, seq, stats)?;
                        }
                        out.accepted_ns = now_ns().saturating_sub(start);
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
    let mut raw_accepted = LatencyHistogram::default();
    let mut rejected = LatencyHistogram::default();
    let mut rejections = 0;
    let mut accepted_ns = 0;
    let mut failure = None;
    for p in produced {
        match p {
            Ok(p) => {
                accepted.merge(&p.accepted);
                raw_accepted.merge(&p.raw_accepted);
                rejected.merge(&p.rejected);
                rejections += p.rejections;
                accepted_ns = accepted_ns.max(p.accepted_ns);
            }
            Err(e) => failure = failure.or(Some(e)),
        }
    }
    // Shutdown runs even after a producer failure, so nothing is left running.
    let shutdown = logger.shutdown(SHUTDOWN);
    let done = now_ns();
    if let Some(e) = failure {
        return Err(e);
    }
    let start = gate
        .started()
        .ok_or_else(|| invalid("producers never started"))?;
    let durable_ns = done.saturating_sub(start);
    let report = match shutdown {
        Ok(Shutdown::Closed(report)) => report,
        Ok(Shutdown::Incomplete { report, .. }) => {
            return Err(format!("shutdown incomplete: {report:?}").into());
        }
        Err(e) => return Err(format!("shutdown failed: {e:?}").into()),
    };
    // A leftover active segment means a failed output: keep the report.
    let (files_created, file_bytes, _) =
        closed_files(&directories).map_err(|e| format!("{e}; report {report:?}"))?;
    let payload_bytes = w.total_payload_bytes();
    Ok(LoggerRun {
        storage: StorageRun {
            tool: Tool::Logger,
            workload: w,
            payload_bytes,
            framing_bytes: file_bytes.saturating_sub(payload_bytes),
            files_created,
            writes: None,
            periodic_syncs: None,
            start_ns: start,
            accepted_ns,
            durable_ns: Some(durable_ns),
            stopped: (!report.is_success() && !overdue_only(&report)).then_some(Stop::Unsuccessful),
            sync_overdue: overdue_only(&report),
        },
        load: load.clone(),
        accepted,
        raw_accepted,
        rejected,
        rejections,
        required_bytes,
        report: format!("{report:?}"),
        directories,
    })
}

/// The only shortfall is a periodic sync past its deadline: every record
/// completed, nothing failed, dropped or stayed unknown, the shutdown met its
/// deadline and no output or worker failed. Read-back still has to pass.
fn overdue_only(report: &dot_logger::logger::FacadeReport) -> bool {
    let core = &report.core;
    let t = &core.required;
    core.sync_failed
        && !core.deadline_elapsed
        && !report.io_failed
        && !report.worker_failed
        && report.writers_outstanding == 0
        && t.failed == 0
        && t.dropped == 0
        && t.unknown == 0
        && t.pending == 0
        && t.complete == t.assigned
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
                Some("active") => {
                    return Err(format!("active segment after Closed: {}", path.display()).into());
                }
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
    let channels = run.load.channels;
    let producers = run.load.producers;
    let mut oracle = SequenceOracle::new(w.records, w.files, producers);
    let (files, _, lists) = closed_files(&run.directories)?;
    let mut v = LoggerVerified {
        records: 0,
        payload_bytes: 0,
        files,
        largest_file: 0,
        largest_file_records: 0,
    };
    for (f, list) in lists.iter().enumerate() {
        for path in list {
            let bytes = fs::read(path)?;
            v.largest_file = v.largest_file.max(bytes.len() as u64);
            let mut in_file = 0_u64;
            for message in NativeReader::new(&bytes, w.max_payload(), 4096)? {
                let stored = message.map_err(|e| format!("decode: {e:?}"))?;
                let payload = &stored.payload;
                let seq = payload_sequence(payload).ok_or(invalid("short payload"))?;
                let channel = stored.record.metadata.channel_id as usize;
                if seq >= w.records
                    || payload.len() != w.payload_of(seq)
                    || !check_p1_payload(seq, payload)
                    || channel != channel_of(&w, channels, seq) + 1
                {
                    return Err(invalid("record differs from the oracle"));
                }
                oracle.see_at(f, seq, w.file_of(seq), producer_of(&w, producers, seq))?;
                in_file += 1;
                if in_file > w.file_record_limit {
                    return Err(invalid("file holds more records than the limit"));
                }
                v.largest_file_records = v.largest_file_records.max(in_file);
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
            let metadata = channel.metadata(writer.now_ns(), 0);
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
