//! Fixed session counters and bounded loss checkpoints, independent of telemetry.
//!
//! Rejections are attempts, output outcomes are assignments, and intentional
//! policy omissions and unknown outcomes are separate units. Observation times
//! use this counter epoch's management monotonic clock, not record timestamps.
use crate::capture::CaptureError;
use prost::Message;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// Maximum count rows in one independently bounded protobuf report part.
pub const ROWS_PER_REPORT: usize = 64;
/// Maximum encoded report bytes, checked before protobuf allocation/decoding.
pub const MAX_REPORT_BYTES: usize = 8192;
pub(crate) const CONTROL_CHARGE: u64 = MAX_REPORT_BYTES as u64 + 1024;
pub(crate) const DESCRIPTOR: &[u8] = include_bytes!("storage/loss_report.bin");
pub(crate) const SCHEMA: &str = "dot_logger.LossReport";
const INGRESS: usize = 12;
const ACQUIRE: usize = 8;
const OUTPUT: [Cause; 9] = [
    Cause::Io,
    Cause::Sync,
    Cause::Adapter,
    Cause::Capacity,
    Cause::RetryExhausted,
    Cause::PendingFull,
    Cause::Expired,
    Cause::Cancelled,
    Cause::Unconfirmed,
];
macro_rules! codes {
    ($name:ident, $doc:literal, $($variant:ident = $value:literal => $description:literal),+ $(,)?) => {
        #[doc=$doc]
        #[derive(Clone,Copy,Debug,Eq,PartialEq,Ord,PartialOrd)]
        #[repr(u32)]
        pub enum $name { $(#[doc=$description] $variant=$value),+ }
        impl $name {
            fn decode(value:u32)->Option<Self> { match value { $($value=>Some(Self::$variant),)+ _=>None } }
        }
    }
}
codes!(Scope,"Unit and origin of a cumulative counter.",
    SubmissionRejected=0=>"Rejected submission attempts, including retried input.",
    AcquireRejected=1=>"Raw-loan acquisition attempts; no channel is supplied.",
    IntentionalSkip=2=>"Declared filtering, sampling or explicit empty routing.",
    OutputFailed=3=>"First terminal failed assignment of the original target.",
    OutputDropped=4=>"First terminal dropped assignment of the original target.",
    OutputUnknown=5=>"Unconfirmed assignment, never an inferred loss or success.");
codes!(Cause,"Fixed reason; no dynamic labels are created on a producer.",
    Capacity=0=>"Ingress, loan or target capacity.",
    CompletionCapacity=1=>"No reusable completion slot.",
    InvalidInput=2=>"Invalid or foreign identity, without attributing a channel.",
    SchemaMismatch=3=>"Registered channel with mismatching schema.",
    TooLarge=4=>"Payload or raw acquisition exceeds its class.",
    Unsupported=5=>"Requested raw operation is unavailable.",
    Closing=6=>"Admission closed; excluded from the stored closing watermark.",
    Exhausted=7=>"Sequence or generation cannot wrap.",
    Encode=8=>"Facade encoding failed before core submission.",
    Policy=9=>"Intentional level filtering or deterministic sampling.",
    NoRoute=10=>"The admitted record explicitly selected no output.",
    Deadline=11=>"General-path admission deadline elapsed.",
    Io=12=>"Output IO failed.",
    Sync=13=>"Output synchronization failed.",
    Adapter=14=>"Output adapter failed.",
    RetryExhausted=15=>"Target retry allowance exhausted.",
    PendingFull=16=>"Target pending queue full.",
    Expired=17=>"Target assignment expired.",
    Cancelled=18=>"Target assignment cancelled.",
    Unconfirmed=19=>"Guard ended without a confirmed outcome.");
codes!(Coverage,"Checkpoint coverage; closure alone never proves quiescence.",
    Baseline=0=>"First complete sample in this file; earlier attempts are unobserved.",
    Observed=1=>"Management observations, not a simultaneous global snapshot.",
    Coalesced=2=>"A previous report was deferred; observation interval widened.",
    FinalPartial=3=>"Final file sample with an explicitly unconfirmed future tail.",
    Quiescent=4=>"Producer return, drain and final sample were explicitly proven.");
/// One cumulative row; absent identifiers remain absent even when ID zero exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LossCount {
    /// Attempts, omissions, or one target's assignments.
    pub scope: Scope,
    /// Fixed cause.
    pub cause: Cause,
    /// Original producer, absent only for genuinely unattributed observations.
    pub producer: Option<u32>,
    /// Registered product channel; absent for acquisitions/foreign tokens.
    pub channel: Option<u32>,
    /// Original assignment target; absent for replicated ingress counters.
    pub target: Option<u32>,
    /// Session-epoch cumulative value, never reset with telemetry.
    pub count: u64,
    /// Saturated value; exact differences are unavailable.
    pub overflowed: bool,
}
/// One part of a cumulative checkpoint. All parts are required for exact deltas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LossReport {
    /// Original capture session.
    pub session: u128,
    /// Counter epoch, independent of telemetry reset.
    pub epoch: u64,
    /// File output that observed the shared counters.
    pub observer: u32,
    /// Monotonic checkpoint ID within this observer/epoch.
    pub checkpoint: u64,
    /// Zero-based part index.
    pub part: u32,
    /// Required part count.
    pub parts: u32,
    /// Conservative previous sample start, in counter-epoch monotonic ns.
    pub observed_from_ns: u64,
    /// End of reading these rows, in counter-epoch monotonic ns.
    pub observed_to_ns: u64,
    /// Wall-clock Unix nanoseconds used by this control Message's MCAP header.
    /// Kept separate from the counter-epoch observation interval.
    pub publish_time_unix_ns: u64,
    /// Baseline, observation, deferred interval, or proven/partial closure.
    pub coverage: Coverage,
    /// A deferred sample was coalesced into these latest cumulative values.
    pub coalesced: bool,
    /// At most ROWS_PER_REPORT cumulative rows.
    pub counts: Vec<LossCount>,
}
#[derive(Default)]
struct Cell {
    count: AtomicU64,
    overflow: AtomicBool,
}
impl Cell {
    // This producer owns every submission/acquisition cell it updates.
    fn single(&self) {
        let old = self.count.load(Ordering::Relaxed);
        if old == u64::MAX {
            self.overflow.store(true, Ordering::Relaxed);
        } else {
            self.count.store(old + 1, Ordering::Relaxed);
        }
    }
    // Independent record slots/IO reporters may update the same assignment
    // counter concurrently. This retrying RMW is general-path only.
    #[allow(deprecated)]
    fn shared(&self) {
        let old = self
            .count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_add(1))
            })
            .unwrap_or_else(|v| v);
        if old == u64::MAX {
            self.overflow.store(true, Ordering::Relaxed);
        }
    }
}
pub(crate) struct Bank {
    pub(crate) session: u128,
    producers: usize,
    channels: Box<[u32]>,
    targets: Box<[u32]>,
    cells: Box<[Cell]>,
    checkpoints: Box<[AtomicU64]>,
    origin: Instant,
    quiescent: AtomicBool,
}
impl Bank {
    pub(crate) fn rows(producers: usize, channels: usize, targets: usize) -> Option<usize> {
        let ingress = channels
            .checked_add(1)?
            .checked_mul(INGRESS)?
            .checked_add(ACQUIRE)?;
        let output = channels.checked_mul(targets)?.checked_mul(OUTPUT.len())?;
        producers.checked_mul(ingress.checked_add(output)?)
    }
    pub(crate) fn valid_rows(rows: usize, producers: usize, channels: usize) -> bool {
        if rows == 0 {
            return true;
        }
        if producers == 0 || channels == 0 || rows % producers != 0 {
            return false;
        }
        let Some(base) = channels
            .checked_add(1)
            .and_then(|v| v.checked_mul(INGRESS))
            .and_then(|v| v.checked_add(ACQUIRE))
        else {
            return false;
        };
        let Some(output) = (rows / producers).checked_sub(base) else {
            return false;
        };
        let Some(width) = channels.checked_mul(OUTPUT.len()) else {
            return false;
        };
        output > 0 && output % width == 0 && output / width <= u16::MAX as usize
    }
    pub(crate) fn required(producers: usize, channels: usize, targets: usize) -> Option<usize> {
        Self::rows(producers, channels, targets)?
            .checked_mul(size_of::<Cell>())?
            .checked_add(
                channels
                    .checked_add(targets)?
                    .checked_mul(size_of::<u32>())?,
            )?
            .checked_add(targets.checked_mul(size_of::<AtomicU64>())?)?
            .checked_add(size_of::<Self>() + 64)
            .filter(|v| *v <= isize::MAX as usize)
    }
    pub(crate) fn new(
        session: u128,
        producers: usize,
        channels: Vec<u32>,
        targets: Vec<u32>,
    ) -> Self {
        let n = Self::rows(producers, channels.len(), targets.len()).unwrap_or(0);
        let result = Self {
            session,
            producers,
            checkpoints: (0..targets.len()).map(|_| AtomicU64::new(0)).collect(),
            channels: channels.into_boxed_slice(),
            targets: targets.into_boxed_slice(),
            cells: (0..n).map(|_| Cell::default()).collect(),
            origin: Instant::now(),
            quiescent: AtomicBool::new(false),
        };
        // Dirty every allocated counter page before any producer is lent.
        for cell in &result.cells {
            cell.count.store(1, Ordering::Relaxed);
            cell.count.store(0, Ordering::Relaxed);
            cell.overflow.store(false, Ordering::Relaxed);
        }
        result
    }
    pub(crate) fn len(&self) -> usize {
        self.cells.len()
    }
    pub(crate) fn parts(&self) -> usize {
        self.len().div_ceil(ROWS_PER_REPORT)
    }
    pub(crate) fn now(&self) -> u64 {
        self.origin.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
    }
    pub(crate) fn prove_quiescent(&self) {
        self.quiescent.store(true, Ordering::Release);
    }
    pub(crate) fn quiescent(&self) -> bool {
        self.quiescent.load(Ordering::Acquire)
    }
    #[allow(deprecated)]
    pub(crate) fn next_checkpoint(&self, target: u32) -> Option<u64> {
        let i = self.targets.iter().position(|v| *v == target)?;
        self.checkpoints[i]
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .ok()?
            .checked_add(1)
    }
    pub(crate) fn ingress(&self, producer: u32, channel: Option<usize>, cause: Cause) {
        if cause == Cause::Closing {
            return;
        }
        let c = channel.unwrap_or(self.channels.len());
        let i = (producer as usize * (self.channels.len() + 1) + c) * INGRESS + cause as usize;
        if let Some(cell) = self.cells.get(i) {
            cell.single();
        }
    }
    pub(crate) fn acquire(&self, producer: u32, error: CaptureError) {
        let cause = reason(error);
        if cause == Cause::Closing {
            return;
        }
        let i = self.producers * (self.channels.len() + 1) * INGRESS
            + producer as usize * ACQUIRE
            + cause as usize;
        if (cause as usize) < ACQUIRE {
            if let Some(cell) = self.cells.get(i) {
                cell.single();
            }
        }
    }
    pub(crate) fn output(&self, producer: usize, channel: usize, target: usize, cause: Cause) {
        if let Some(r) = OUTPUT.iter().position(|v| *v == cause) {
            let base = self.producers * ((self.channels.len() + 1) * INGRESS + ACQUIRE);
            let i = base
                + ((producer * self.channels.len() + channel) * self.targets.len() + target)
                    * OUTPUT.len()
                + r;
            if let Some(cell) = self.cells.get(i) {
                cell.shared();
            }
        }
    }
    fn get(&self, index: usize) -> Option<LossCount> {
        let cell = self.cells.get(index)?;
        let capture = self.producers * (self.channels.len() + 1) * INGRESS;
        let acquire = self.producers * ACQUIRE;
        let (scope, cause, producer, channel, target) = if index < capture {
            let cause = Cause::decode((index % INGRESS) as u32)?;
            let key = index / INGRESS;
            let channel = key % (self.channels.len() + 1);
            (
                if matches!(cause, Cause::Policy | Cause::NoRoute) {
                    Scope::IntentionalSkip
                } else {
                    Scope::SubmissionRejected
                },
                cause,
                (key / (self.channels.len() + 1)) as u32,
                self.channels.get(channel).copied(),
                None,
            )
        } else if index < capture + acquire {
            let key = index - capture;
            (
                Scope::AcquireRejected,
                Cause::decode((key % ACQUIRE) as u32)?,
                (key / ACQUIRE) as u32,
                None,
                None,
            )
        } else {
            let key = index - capture - acquire;
            let cause = OUTPUT[key % OUTPUT.len()];
            let key = key / OUTPUT.len();
            let t = key % self.targets.len();
            let key = key / self.targets.len();
            (
                match cause {
                    Cause::Unconfirmed => Scope::OutputUnknown,
                    Cause::Io | Cause::Sync | Cause::Adapter => Scope::OutputFailed,
                    _ => Scope::OutputDropped,
                },
                cause,
                (key / self.channels.len()) as u32,
                Some(self.channels[key % self.channels.len()]),
                Some(self.targets[t]),
            )
        };
        let observed = cell.count.load(Ordering::Relaxed);
        let overflowed = cell.overflow.load(Ordering::Relaxed) || observed == u64::MAX;
        Some(LossCount {
            scope,
            cause,
            producer: Some(producer),
            channel,
            target,
            count: if overflowed { u64::MAX } else { observed },
            overflowed,
        })
    }
    pub(crate) fn sample(
        &self,
        target: u32,
        checkpoint: u64,
        part: usize,
        from: u64,
        coverage: Coverage,
    ) -> LossReport {
        let start = part * ROWS_PER_REPORT;
        let counts = (start..(start + ROWS_PER_REPORT).min(self.len()))
            .filter_map(|i| self.get(i))
            .collect();
        LossReport {
            session: self.session,
            epoch: 1,
            observer: target,
            checkpoint,
            part: part as u32,
            parts: self.parts() as u32,
            observed_from_ns: from,
            observed_to_ns: self.now(),
            publish_time_unix_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos().min(u128::from(u64::MAX)) as u64),
            coalesced: coverage == Coverage::Coalesced,
            coverage,
            counts,
        }
    }
}
/// Borrowed management view; copying it allocates nothing and changes no refcount.
#[derive(Clone, Copy)]
pub struct Counters<'a> {
    pub(crate) bank: &'a Bank,
}
impl Counters<'_> {
    /// Fixed registered counter row count, including zero-valued rows.
    pub fn len(&self) -> usize {
        self.bank.len()
    }
    /// Whether there are no registered counter rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Read one cumulative row. Different rows are not a simultaneous snapshot.
    pub fn get(&self, index: usize) -> Option<LossCount> {
        self.bank.get(index)
    }
}
pub(crate) fn reason(error: CaptureError) -> Cause {
    match error {
        CaptureError::Capacity => Cause::Capacity,
        CaptureError::CompletionCapacity => Cause::CompletionCapacity,
        CaptureError::InvalidInput => Cause::InvalidInput,
        CaptureError::SchemaMismatch => Cause::SchemaMismatch,
        CaptureError::TooLarge => Cause::TooLarge,
        CaptureError::Unsupported => Cause::Unsupported,
        CaptureError::Closing => Cause::Closing,
        CaptureError::SequenceExhausted | CaptureError::GenerationExhausted => Cause::Exhausted,
        CaptureError::DeadlineExceeded => Cause::Deadline,
    }
}
#[derive(Clone, PartialEq, Message)]
struct WireCount {
    #[prost(uint32, tag = "1")]
    scope: u32,
    #[prost(uint32, tag = "2")]
    cause: u32,
    #[prost(uint32, optional, tag = "3")]
    producer: Option<u32>,
    #[prost(uint32, optional, tag = "4")]
    channel: Option<u32>,
    #[prost(uint32, optional, tag = "5")]
    target: Option<u32>,
    #[prost(uint64, tag = "6")]
    count: u64,
    #[prost(bool, tag = "7")]
    overflowed: bool,
}
#[derive(Clone, PartialEq, Message)]
struct WireReport {
    #[prost(uint32, tag = "1")]
    version: u32,
    #[prost(bytes = "vec", tag = "2")]
    session: Vec<u8>,
    #[prost(uint64, tag = "3")]
    epoch: u64,
    #[prost(uint32, tag = "4")]
    observer: u32,
    #[prost(uint64, tag = "5")]
    checkpoint: u64,
    #[prost(uint32, tag = "6")]
    part: u32,
    #[prost(uint32, tag = "7")]
    parts: u32,
    #[prost(uint64, tag = "8")]
    from: u64,
    #[prost(uint64, tag = "9")]
    to: u64,
    #[prost(uint32, tag = "10")]
    coverage: u32,
    #[prost(message, repeated, tag = "11")]
    counts: Vec<WireCount>,
    #[prost(uint64, tag = "12")]
    publish_time_unix_ns: u64,
    #[prost(bool, tag = "13")]
    coalesced: bool,
}
impl LossReport {
    pub(crate) fn encode(&self, out: &mut Vec<u8>) -> Result<(), crate::storage::StorageError> {
        if self.counts.len() > ROWS_PER_REPORT {
            return Err(crate::storage::StorageError::InvalidRecord);
        }
        let report = WireReport {
            version: 1,
            session: self.session.to_le_bytes().to_vec(),
            epoch: self.epoch,
            observer: self.observer,
            checkpoint: self.checkpoint,
            part: self.part,
            parts: self.parts,
            from: self.observed_from_ns,
            to: self.observed_to_ns,
            coverage: self.coverage as u32,
            publish_time_unix_ns: self.publish_time_unix_ns,
            coalesced: self.coalesced,
            counts: self
                .counts
                .iter()
                .map(|r| WireCount {
                    scope: r.scope as u32,
                    cause: r.cause as u32,
                    producer: r.producer,
                    channel: r.channel,
                    target: r.target,
                    count: r.count,
                    overflowed: r.overflowed,
                })
                .collect(),
        };
        out.clear();
        if report.encoded_len() > MAX_REPORT_BYTES {
            return Err(crate::storage::StorageError::InvalidRecord);
        }
        report
            .encode(out)
            .map_err(|_| crate::storage::StorageError::InvalidRecord)
    }
    /// Decode one bounded part; malformed enum, range, presence or width is rejected.
    pub fn decode(bytes: &[u8]) -> Result<Self, crate::storage::StorageError> {
        let bad = || crate::storage::StorageError::CorruptData;
        if bytes.len() > MAX_REPORT_BYTES || !bounded_wire(bytes, false) {
            return Err(bad());
        }
        let r = WireReport::decode(bytes).map_err(|_| bad())?;
        if r.version != 1
            || r.session.len() != 16
            || r.epoch == 0
            || r.checkpoint == 0
            || r.parts == 0
            || r.part >= r.parts
            || r.from > r.to
            || r.counts.len() > ROWS_PER_REPORT
        {
            return Err(bad());
        }
        let counts = r
            .counts
            .into_iter()
            .map(|c| {
                let scope = Scope::decode(c.scope).ok_or_else(bad)?;
                let cause = Cause::decode(c.cause).ok_or_else(bad)?;
                let valid_cause = match scope {
                    Scope::SubmissionRejected => {
                        (cause as u32 <= Cause::Encode as u32) || cause == Cause::Deadline
                    }
                    Scope::AcquireRejected => (cause as u32) < ACQUIRE as u32,
                    Scope::IntentionalSkip => {
                        matches!(cause, Cause::Policy | Cause::NoRoute)
                    }
                    Scope::OutputFailed => {
                        matches!(cause, Cause::Io | Cause::Sync | Cause::Adapter)
                            && c.producer.is_some()
                            && c.channel.is_some()
                    }
                    Scope::OutputDropped => {
                        matches!(
                            cause,
                            Cause::Capacity
                                | Cause::RetryExhausted
                                | Cause::PendingFull
                                | Cause::Expired
                                | Cause::Cancelled
                        ) && c.producer.is_some()
                            && c.channel.is_some()
                    }
                    Scope::OutputUnknown => {
                        cause == Cause::Unconfirmed && c.producer.is_some() && c.channel.is_some()
                    }
                };
                if !valid_cause
                    || matches!(
                        scope,
                        Scope::OutputFailed | Scope::OutputDropped | Scope::OutputUnknown
                    ) != c.target.is_some()
                    || scope == Scope::AcquireRejected && c.channel.is_some()
                    || c.overflowed && c.count != u64::MAX
                {
                    return Err(bad());
                }
                Ok(LossCount {
                    scope,
                    cause,
                    producer: c.producer,
                    channel: c.channel,
                    target: c.target,
                    count: c.count,
                    overflowed: c.overflowed,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if counts.iter().enumerate().any(|(i, row)| {
            counts[..i]
                .iter()
                .any(|old| counter_key(old) == counter_key(row))
        }) {
            return Err(bad());
        }
        Ok(Self {
            session: u128::from_le_bytes(r.session.as_slice().try_into().map_err(|_| bad())?),
            epoch: r.epoch,
            observer: r.observer,
            checkpoint: r.checkpoint,
            part: r.part,
            parts: r.parts,
            observed_from_ns: r.from,
            observed_to_ns: r.to,
            coverage: Coverage::decode(r.coverage).ok_or_else(bad)?,
            publish_time_unix_ns: r.publish_time_unix_ns,
            coalesced: r.coalesced,
            counts,
        })
    }
}
fn varint(bytes: &mut &[u8]) -> Option<u64> {
    let mut n = 0u64;
    for shift in (0..70).step_by(7) {
        let (&b, rest) = bytes.split_first()?;
        *bytes = rest;
        if shift == 63 && b > 1 {
            return None;
        }
        n |= u64::from(b & 127) << shift;
        if b < 128 {
            return Some(n);
        }
    }
    None
}
fn bounded_wire(mut bytes: &[u8], row: bool) -> bool {
    let mut seen = 0u32;
    let mut count = 0;
    while !bytes.is_empty() {
        let Some(tag) = varint(&mut bytes) else {
            return false;
        };
        let field = tag >> 3;
        let wire = tag & 7;
        if field == 0 || field > if row { 7 } else { 13 } {
            return false;
        }
        let repeated = !row && field == 11;
        let bit = 1 << field;
        if !repeated && seen & bit != 0 {
            return false;
        }
        seen |= bit;
        if wire == 0 && !(!row && (field == 2 || field == 11)) {
            let Some(value) = varint(&mut bytes) else {
                return false;
            };
            if (row && field <= 5 || !row && matches!(field, 1 | 4 | 6 | 7 | 10))
                && value > u64::from(u32::MAX)
            {
                return false;
            }
            if (row && field == 7 || !row && field == 13) && value > 1 {
                return false;
            }
        } else if wire == 2 && !row && (field == 2 || field == 11) {
            let Some(len) = varint(&mut bytes).and_then(|v| usize::try_from(v).ok()) else {
                return false;
            };
            let Some(body) = bytes.get(..len) else {
                return false;
            };
            bytes = &bytes[len..];
            if field == 2 && len != 16 {
                return false;
            }
            if repeated {
                count += 1;
                if count > ROWS_PER_REPORT || !bounded_wire(body, true) {
                    return false;
                }
            }
        } else {
            return false;
        }
    }
    true
}

/// One counter's watermark comparison over the selected complete observations.
/// Ingress replicas from multiple outputs share one key and are never summed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LossDelta {
    /// Original capture session.
    pub session: u128,
    /// Counter epoch; different epochs are never subtracted.
    pub epoch: u64,
    /// Largest observed cumulative row, preserving its attempt/assignment unit.
    pub counter: LossCount,
    /// Difference between complete checkpoint watermarks; absent when the latest
    /// value exists only in incomplete parts, overflow, observer decrease, or
    /// fewer than two complete observations. Later complete parts may coalesce gaps.
    pub delta: Option<u64>,
    /// Conservative management observation envelope, not event timestamps.
    pub observed_from_ns: u64,
    /// End of the last selected observation.
    pub observed_to_ns: u64,
    /// A complete quiescent observation covers the maximum watermark.
    pub tail_confirmed: bool,
}
fn counter_key(row: &LossCount) -> (Scope, Cause, Option<u32>, Option<u32>, Option<u32>) {
    (row.scope, row.cause, row.producer, row.channel, row.target)
}
/// Compare cumulative watermarks without adding replicas. `max_rows` bounds all
/// temporary index/result rows. Incomplete parts preserve cumulative evidence and cannot extend a difference;
/// decreasing/saturated values never produce an exact difference.
pub fn summarize(
    reports: &[LossReport],
    max_rows: usize,
) -> Result<Vec<LossDelta>, crate::storage::StorageError> {
    use crate::storage::StorageError;
    let count = reports
        .iter()
        .try_fold(0usize, |n, r| n.checked_add(r.counts.len()))
        .ok_or(StorageError::InvalidConfig)?;
    if count > max_rows || reports.len() > max_rows {
        return Err(StorageError::NeedRotation);
    }
    if reports.iter().any(|r| {
        r.counts.is_empty() || r.counts.len() > ROWS_PER_REPORT || r.parts == 0 || r.part >= r.parts
    }) {
        return Err(StorageError::CorruptData);
    }
    let mut groups: Vec<_> = reports.iter().collect();
    groups.sort_unstable_by_key(|r| (r.session, r.epoch, r.observer, r.checkpoint, r.part));
    let mut samples = Vec::with_capacity(count);
    // Include groups missing this row when deciding whether a later incomplete
    // checkpoint invalidates an earlier quiescent tail.
    let mut latest = std::collections::BTreeMap::new();
    let mut at = 0;
    while at < groups.len() {
        let first = groups[at];
        let key = (first.session, first.epoch, first.observer, first.checkpoint);
        let mut end = at + 1;
        while end < groups.len()
            && (
                groups[end].session,
                groups[end].epoch,
                groups[end].observer,
                groups[end].checkpoint,
            ) == key
        {
            end += 1;
        }
        let complete = end - at == first.parts as usize
            && groups[at..end].iter().enumerate().all(|(i, r)| {
                r.parts == first.parts
                    && r.part as usize == i
                    && r.coverage == first.coverage
                    && r.observed_from_ns == first.observed_from_ns
            });
        let observed_to = groups[at..end]
            .iter()
            .map(|r| r.observed_to_ns)
            .max()
            .unwrap_or(0);
        let quiescent = complete && first.coverage == Coverage::Quiescent;
        let last = latest
            .entry((first.session, first.epoch))
            .or_insert((observed_to, quiescent));
        match observed_to.cmp(&last.0) {
            std::cmp::Ordering::Greater => *last = (observed_to, quiescent),
            std::cmp::Ordering::Equal => last.1 &= quiescent,
            std::cmp::Ordering::Less => {}
        }
        for report in &groups[at..end] {
            for row in &report.counts {
                samples.push((*report, row, complete, observed_to));
            }
        }
        at = end;
    }
    samples.sort_unstable_by_key(|(report, row, _, _)| {
        (
            report.session,
            report.epoch,
            counter_key(row),
            report.observer,
            report.checkpoint,
        )
    });
    let mut result = Vec::new();
    let mut at = 0;
    while at < samples.len() {
        let (report, row, _, _) = samples[at];
        let key = (report.session, report.epoch, counter_key(row));
        let mut end = at + 1;
        while end < samples.len()
            && (
                samples[end].0.session,
                samples[end].0.epoch,
                counter_key(samples[end].1),
            ) == key
        {
            end += 1;
        }
        let mut valid = true;
        let mut complete_count = 0usize;
        let mut min = u64::MAX;
        let mut max = 0;
        let mut complete_max = 0;
        let mut from = u64::MAX;
        let mut to = 0;
        let mut previous = None;
        let mut previous_complete = None;
        let mut interval = false;
        for (r, c, complete, _) in &samples[at..end] {
            valid &= !c.overflowed;
            if let Some((observer, checkpoint, value, observed)) = previous {
                if observer == r.observer
                    && (checkpoint == r.checkpoint
                        || c.count < value
                        || r.observed_to_ns < observed)
                {
                    valid = false;
                }
            }
            previous = Some((r.observer, r.checkpoint, c.count, r.observed_to_ns));
            max = max.max(c.count);
            if *complete {
                interval |=
                    r.coverage != Coverage::Baseline || previous_complete == Some(r.observer);
                previous_complete = Some(r.observer);
                complete_count += 1;
                min = min.min(c.count);
                complete_max = complete_max.max(c.count);
                from = from.min(r.observed_from_ns);
                to = to.max(r.observed_to_ns);
            }
        }
        let exact = valid && interval && complete_count >= 2 && max == complete_max;
        if complete_count == 0 {
            from = samples[at..end]
                .iter()
                .map(|(r, _, _, _)| r.observed_from_ns)
                .min()
                .unwrap_or(0);
            to = samples[at..end]
                .iter()
                .map(|(r, _, _, _)| r.observed_to_ns)
                .max()
                .unwrap_or(0);
            min = max;
        }
        let mut counter = row.clone();
        counter.count = max;
        counter.overflowed = samples[at..end].iter().any(|(_, r, _, _)| r.overflowed);
        let last = latest[&(report.session, report.epoch)];
        let tail = valid
            && last.1
            && samples[at..end]
                .iter()
                .any(|(r, c, complete, observed_to)| {
                    *complete
                        && r.coverage == Coverage::Quiescent
                        && c.count == max
                        && *observed_to == last.0
                });
        result.push(LossDelta {
            session: report.session,
            epoch: report.epoch,
            counter,
            delta: exact.then_some(max - min),
            observed_from_ns: from,
            observed_to_ns: to,
            tail_confirmed: tail,
        });
        at = end;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn saturation_never_wraps_or_turns_a_torn_read_into_an_exact_value() {
        let bank = Bank::new(42, 1, vec![0], vec![0]);
        bank.cells[0].count.store(u64::MAX - 1, Ordering::Relaxed);
        bank.ingress(0, Some(0), Cause::Capacity);
        bank.ingress(0, Some(0), Cause::Capacity);
        let row = bank.get(0).unwrap();
        assert_eq!(row.count, u64::MAX);
        assert!(row.overflowed);
        bank.cells[0].count.store(u64::MAX - 1, Ordering::Relaxed);
        bank.cells[0].overflow.store(true, Ordering::Relaxed);
        assert_eq!(bank.get(0).unwrap().count, u64::MAX);
        let base = bank.producers * ((bank.channels.len() + 1) * INGRESS + ACQUIRE);
        bank.cells[base].count.store(u64::MAX, Ordering::Relaxed);
        bank.output(0, 0, 0, Cause::Io);
        assert_eq!(bank.get(base).unwrap().count, u64::MAX);
        assert!(bank.get(base).unwrap().overflowed);
    }
    #[test]
    fn protobuf_preserves_zero_identifier_presence_and_rejects_semantic_or_count_abuse() {
        let bank = Bank::new(42, 1, vec![0], vec![0]);
        bank.ingress(0, Some(0), Cause::Capacity);
        let report = bank.sample(0, 1, 0, 0, Coverage::Baseline);
        let mut bytes = Vec::new();
        report.encode(&mut bytes).unwrap();
        let decoded = LossReport::decode(&bytes).unwrap();
        assert!(
            decoded
                .counts
                .iter()
                .any(|r| r.channel == Some(0) && r.producer == Some(0) && r.target == Some(0))
        );
        assert!(
            decoded
                .counts
                .iter()
                .any(|r| r.scope == Scope::AcquireRejected
                    && r.channel.is_none()
                    && r.target.is_none())
        );
        let mut wrong = report.clone();
        wrong.counts[0].scope = Scope::OutputFailed;
        wrong.encode(&mut bytes).unwrap();
        assert!(LossReport::decode(&bytes).is_err());
        let mut raw = WireReport::decode(report_to_bytes(&report).as_slice()).unwrap();
        raw.counts = (0..65).map(|_| raw.counts[0].clone()).collect();
        let bytes = raw.encode_to_vec();
        assert!(bytes.len() < MAX_REPORT_BYTES);
        assert!(LossReport::decode(&bytes).is_err());
        let mut bytes = report_to_bytes(&report);
        bytes.extend_from_slice(&[0x08, 0x01]);
        assert!(LossReport::decode(&bytes).is_err(), "duplicate scalar");
        let mut bytes = report_to_bytes(&report);
        bytes.extend_from_slice(&[0x68, 0x02]);
        assert!(LossReport::decode(&bytes).is_err(), "invalid bool");
    }
    fn report_to_bytes(report: &LossReport) -> Vec<u8> {
        let mut bytes = Vec::new();
        report.encode(&mut bytes).unwrap();
        bytes
    }
    #[test]
    fn incomplete_overflow_and_decreasing_checkpoints_never_extend_an_exact_delta() {
        let bank = Bank::new(42, 1, vec![7], vec![10]);
        let baseline = bank.sample(10, 1, 0, 0, Coverage::Baseline);
        bank.ingress(0, Some(0), Cause::Capacity);
        let sample = bank.sample(10, 2, 0, 0, Coverage::Observed);
        let target = |reports: &[LossReport]| {
            summarize(reports, 256)
                .unwrap()
                .into_iter()
                .find(|r| {
                    r.counter.scope == Scope::SubmissionRejected
                        && r.counter.cause == Cause::Capacity
                        && r.counter.channel == Some(7)
                })
                .unwrap()
        };
        assert_eq!(target(&[baseline.clone(), sample.clone()]).delta, Some(1));
        let mut missing = sample.clone();
        missing.parts = 2;
        assert!(target(&[baseline.clone(), missing]).delta.is_none());
        let mut overflow = sample.clone();
        overflow.counts[0].count = u64::MAX;
        overflow.counts[0].overflowed = true;
        assert!(target(&[baseline.clone(), overflow]).delta.is_none());
        let mut smaller = baseline.clone();
        smaller.checkpoint = 3;
        smaller.observed_to_ns = sample.observed_to_ns + 1;
        assert!(target(&[sample, smaller]).delta.is_none());
        assert!(matches!(
            summarize(&[baseline], 1),
            Err(crate::storage::StorageError::NeedRotation)
        ));
    }
    #[test]
    fn baseline_replicas_and_stale_quiescent_reports_do_not_confirm_unobserved_work() {
        let bank = Bank::new(42, 1, vec![7], vec![10, 20]);
        let mut first = bank.sample(10, 1, 0, 1, Coverage::Baseline);
        first.observed_to_ns = 10;
        let mut replica = first.clone();
        replica.observer = 20;
        assert!(
            summarize(&[first.clone(), replica], 256)
                .unwrap()
                .iter()
                .all(|d| d.delta.is_none())
        );
        let mut final_report = first.clone();
        final_report.checkpoint = 2;
        final_report.observed_to_ns = 20;
        final_report.coverage = Coverage::Quiescent;
        assert!(
            summarize(&[first.clone(), final_report.clone()], 256)
                .unwrap()
                .iter()
                .all(|d| d.tail_confirmed)
        );
        for (complete, coverage) in [(false, Coverage::Quiescent), (true, Coverage::Observed)] {
            let mut later = final_report.clone();
            later.checkpoint = 3;
            later.observed_to_ns = 30;
            later.coverage = coverage;
            if !complete {
                later.parts = 2;
            }
            assert!(
                summarize(&[first.clone(), final_report.clone(), later], 256)
                    .unwrap()
                    .iter()
                    .all(|d| !d.tail_confirmed)
            );
        }
        assert!(
            summarize(
                &[first.clone(), final_report.clone(), final_report.clone()],
                256
            )
            .unwrap()
            .iter()
            .all(|d| !d.tail_confirmed)
        );
        let mut decrease = final_report.clone();
        decrease.checkpoint = 3;
        decrease.observed_to_ns = 30;
        final_report.counts[0].count = 1;
        let key = counter_key(&decrease.counts[0]);
        assert!(
            !summarize(&[first, final_report, decrease], 256)
                .unwrap()
                .iter()
                .find(|d| counter_key(&d.counter) == key)
                .unwrap()
                .tail_confirmed
        );
    }
    #[test]
    fn registered_shape_and_memory_arithmetic_reject_impossible_counter_grids() {
        assert_eq!(Bank::rows(1, 1, 1), Some(41));
        assert!(Bank::valid_rows(41, 1, 1));
        assert!(!Bank::valid_rows(1, 1, 1));
        assert!(!Bank::valid_rows(42, 1, 1));
        assert!(Bank::required(usize::MAX, usize::MAX, usize::MAX).is_none());
        assert!(Bank::rows(2, usize::MAX, 1).is_none());
    }
}
