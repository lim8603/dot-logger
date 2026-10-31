//! Synchronous offline replay with a caller-driven monotonic nanosecond clock.
//! A single prefetched descriptor and payload use the owned snapshot's reservation.
//! No timer, worker, network send or RT capture callback is started here.
use crate::capture::CapturedRecord;
use crate::query::{
    BlobHandle, Cursor, Payload, Query, QueryError, QueryRecord, Snapshot, TimeBasis,
};
use std::io::{Read, Seek};
use std::sync::atomic::AtomicBool;

/// Finite positive rational playback rate; e.g. 1/2, 1/1 or 2/1.
/// Components are preserved: equality compares the supplied numerator and
/// denominator. Use [`Speed::is_equivalent`] to compare playback rates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Speed {
    numerator: u32,
    denominator: u32,
}
impl Speed {
    /// Original-time playback rate (1/1).
    pub const REALTIME: Self = Self {
        numerator: 1,
        denominator: 1,
    };
    /// Reject zero components. Integer ratios avoid floating-point clock drift.
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, ReplayError> {
        if numerator == 0 || denominator == 0 {
            return Err(ReplayError::InvalidInput);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }
    /// Compare rates without changing their original components.
    pub fn is_equivalent(&self, other: &Self) -> bool {
        // Two u32 factors always fit in u64, including u32::MAX squared.
        u64::from(self.numerator) * u64::from(other.denominator)
            == u64::from(other.numerator) * u64::from(self.denominator)
    }
    /// Positive numerator as supplied, without reducing the ratio.
    pub fn numerator(&self) -> u32 {
        self.numerator
    }
    /// Positive denominator as supplied, without reducing the ratio.
    pub fn denominator(&self) -> u32 {
        self.denominator
    }
}
impl std::fmt::Display for Speed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}
/// Replay lifecycle; Ended keeps readers alive for the final record's blob reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayState {
    /// Clock advances at the selected speed.
    Playing,
    /// Media clock is frozen; no prefetch or delivery is performed.
    Paused,
    /// No more matching records. Seek can restart within the frozen snapshot.
    Ended,
    /// Readers and their reservations have been released. This state is terminal.
    Stopped,
}
/// Replay validation, clock/generation arithmetic, or original query failure.
#[derive(Debug)]
pub enum ReplayError {
    /// Non-mono filter or invalid rate.
    InvalidInput,
    /// Operation is incompatible with the current lifecycle state.
    InvalidState,
    /// Scheduling deadline or seek generation cannot be represented.
    Limit,
    /// A retained blob reference predates the current seek generation.
    StaleGeneration,
    /// Original structured query failure, including the original OS IO cause.
    Query(QueryError),
}
impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("non-mono filter or invalid rate"),
            Self::InvalidState => {
                f.write_str("operation is incompatible with the current replay state")
            }
            Self::Limit => f.write_str("scheduling deadline or seek generation limit exceeded"),
            Self::StaleGeneration => {
                f.write_str("blob reference predates the current seek generation")
            }
            Self::Query(error) => write!(f, "replay query failed: {error}"),
        }
    }
}
impl std::error::Error for ReplayError {}
/// Synchronous seek/stop outcome. Already returned deliveries cannot be revoked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transition {
    /// Generation attached to subsequent deliveries.
    pub generation: u64,
    /// A prefetched but not yet delivered record was discarded.
    pub discarded_prefetch: bool,
}
/// Borrowed original payload; large payloads stay on disk.
#[derive(Debug)]
pub enum ReplayPayload<'a> {
    /// Bytes valid for this frame's borrow only.
    Inline(&'a [u8]),
    /// Use a cloned handle with the frame's generation in [`Replay::read_blob`].
    Blob(&'a BlobHandle),
}
/// One delivered record. Its borrow prevents seek/stop during synchronous use.
#[derive(Debug)]
pub struct ReplayFrame<'a> {
    /// Original receipt, clocks, schema and channel metadata.
    pub record: &'a CapturedRecord,
    /// Original inline bytes or bounded blob descriptor.
    pub payload: ReplayPayload<'a>,
    /// Seek generation. Adapters must discard queued sends from older generations.
    pub generation: u64,
    /// Delivery delay relative to the slowed offline clock, in host nanoseconds.
    pub late_ns: u64,
}
/// One bounded turn, with no sleeping or retry loop.
#[derive(Debug)]
pub enum ReplayPoll<'a> {
    /// Consumer readiness was accepted; one record is delivered.
    Ready(ReplayFrame<'a>),
    /// Not yet due or the consumer declined. No record was delivered.
    Waiting {
        /// Host monotonic nanosecond deadline for the prefetched record.
        due_ns: u64,
        /// Current delay, including time spent waiting for consumer readiness.
        late_ns: u64,
    },
    /// Clock is paused and no read was issued.
    Paused,
    /// Frozen selection is empty or has been fully consumed; no loss is inferred.
    End,
    /// Terminal stop; all actual reader IO has returned and readers were closed.
    Stopped,
}
struct Pending {
    record: QueryRecord,
    next: Option<Cursor>,
    delivered: bool,
}
/// Owns one snapshot and at most one prefetched record, using its existing buffers.
/// The host supplies readiness and times in the same clock domain as snapshot pins.
/// Old host timestamps are clamped to the maximum observed value.
pub struct Replay<R> {
    snapshot: Option<Snapshot<R>>,
    query: Query,
    cursor: Option<Cursor>,
    pending: Option<Pending>,
    state: ReplayState,
    generation: u64,
    speed: Speed,
    anchor_host: u64,
    anchor_media: u128,
    observed: u64,
}
impl<R> Drop for Replay<R> {
    fn drop(&mut self) {
        // Release copied descriptors before the snapshot's reader/ledger lease.
        self.pending = None;
        self.cursor = None;
        self.snapshot = None;
    }
}
impl<R: Read + Seek> Replay<R> {
    /// Start at query.from on the mono axis. Leading gaps retain their relative
    /// duration; query validation and snapshot/pin expiry remain in force.
    pub fn new(
        snapshot: Snapshot<R>,
        query: Query,
        speed: Speed,
        now_ns: u64,
        cancel: &AtomicBool,
    ) -> Result<Self, ReplayError> {
        if query.basis != TimeBasis::Mono {
            return Err(ReplayError::InvalidInput);
        }
        let cursor = snapshot
            .start(query, now_ns, cancel)
            .map_err(ReplayError::Query)?;
        Ok(Self {
            snapshot: Some(snapshot),
            query,
            cursor: Some(cursor),
            pending: None,
            state: ReplayState::Playing,
            generation: 1,
            speed,
            anchor_host: now_ns,
            anchor_media: query.from as u128,
            observed: now_ns,
        })
    }
    /// Current lifecycle state; query errors and explicit stop are terminal.
    pub fn state(&self) -> ReplayState {
        self.state
    }
    /// Generation starts at one and increments on each successful seek.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    fn fail<T>(&mut self, error: ReplayError) -> Result<T, ReplayError> {
        self.stop();
        Err(error)
    }
    fn live(&mut self, now_ns: u64, cancel: &AtomicBool) -> Result<u64, ReplayError> {
        if self.state == ReplayState::Stopped {
            return Err(ReplayError::InvalidState);
        }
        let now = now_ns.max(self.observed);
        self.observed = now;
        let result = self
            .snapshot
            .as_ref()
            .ok_or(ReplayError::InvalidState)?
            .check_alive(now, cancel);
        if let Err(error) = result {
            return self.fail(ReplayError::Query(error));
        }
        Ok(now)
    }
    fn media_at(&self, now: u64) -> u128 {
        let elapsed = if self.state == ReplayState::Playing {
            u128::from(now.saturating_sub(self.anchor_host)) * u128::from(self.speed.numerator)
                / u128::from(self.speed.denominator)
        } else {
            0
        };
        // Beyond the selection's end there are no representable future records.
        (self.anchor_media + elapsed).min(self.query.to as u128)
    }
    /// Freeze the current media position. Repeated pause is idempotent.
    pub fn pause(&mut self, now_ns: u64, cancel: &AtomicBool) -> Result<(), ReplayError> {
        let now = self.live(now_ns, cancel)?;
        if self.state == ReplayState::Ended {
            return Err(ReplayError::InvalidState);
        }
        self.anchor_media = self.media_at(now);
        self.anchor_host = now;
        self.state = ReplayState::Paused;
        Ok(())
    }
    /// Resume at the frozen media position; repeated resume does not reset time.
    pub fn resume(&mut self, now_ns: u64, cancel: &AtomicBool) -> Result<(), ReplayError> {
        let now = self.live(now_ns, cancel)?;
        match self.state {
            ReplayState::Paused => {
                self.anchor_host = now;
                self.state = ReplayState::Playing;
            }
            ReplayState::Playing => {}
            _ => return Err(ReplayError::InvalidState),
        }
        Ok(())
    }
    /// Change speed at the current integer-nanosecond media position, including
    /// while paused. Transitions round fractional media nanoseconds down.
    pub fn set_speed(
        &mut self,
        speed: Speed,
        now_ns: u64,
        cancel: &AtomicBool,
    ) -> Result<(), ReplayError> {
        let now = self.live(now_ns, cancel)?;
        self.anchor_media = self.media_at(now);
        self.anchor_host = now;
        self.speed = speed;
        Ok(())
    }
    /// Reposition within the frozen query, preserving filters and pause state.
    /// Validation failure leaves the previous selection and prefetch intact.
    pub fn seek(
        &mut self,
        from_ns: u64,
        now_ns: u64,
        cancel: &AtomicBool,
    ) -> Result<Transition, ReplayError> {
        let now = self.live(now_ns, cancel)?;
        let generation = self.generation.checked_add(1).ok_or(ReplayError::Limit)?;
        let query = Query {
            from: from_ns as i128,
            ..self.query
        };
        let result = self
            .snapshot
            .as_ref()
            .ok_or(ReplayError::InvalidState)?
            .start(query, now, cancel);
        let cursor = match result {
            Ok(cursor) => cursor,
            Err(QueryError::InvalidInput) => {
                return Err(ReplayError::Query(QueryError::InvalidInput));
            }
            Err(error) => return self.fail(ReplayError::Query(error)),
        };
        let discarded_prefetch = self.pending.as_ref().is_some_and(|p| !p.delivered);
        self.pending = None;
        self.cursor = Some(cursor);
        self.query = query;
        self.generation = generation;
        self.anchor_media = from_ns as u128;
        self.anchor_host = now;
        if self.state == ReplayState::Ended {
            self.state = ReplayState::Playing;
        }
        Ok(Transition {
            generation,
            discarded_prefetch,
        })
    }
    /// Discard undelivered prefetch and close actual readers before releasing pins.
    /// Idempotent; no background IO or external send is assumed to have completed.
    pub fn stop(&mut self) -> Transition {
        let discarded_prefetch = self.pending.as_ref().is_some_and(|p| !p.delivered);
        self.pending = None;
        self.cursor = None;
        self.snapshot = None;
        self.state = ReplayState::Stopped;
        Transition {
            generation: self.generation,
            discarded_prefetch,
        }
    }
    /// Prefetch at most one record, then deliver only if due and consumer_ready.
    /// Late delivery slows the offline clock to retain subsequent record spacing.
    pub fn poll(
        &mut self,
        now_ns: u64,
        consumer_ready: bool,
        cancel: &AtomicBool,
    ) -> Result<ReplayPoll<'_>, ReplayError> {
        if self.state == ReplayState::Stopped {
            return Ok(ReplayPoll::Stopped);
        }
        let now = self.live(now_ns, cancel)?;
        if self.state == ReplayState::Paused {
            return Ok(ReplayPoll::Paused);
        }
        if self.pending.as_ref().is_some_and(|p| p.delivered) {
            self.pending = None;
        }
        if self.pending.is_none() {
            let Some(cursor) = self.cursor.as_ref() else {
                self.state = ReplayState::Ended;
                return Ok(ReplayPoll::End);
            };
            let loaded = self
                .snapshot
                .as_mut()
                .ok_or(ReplayError::InvalidState)?
                .page_one(cursor, now, cancel)
                .map(|p| {
                    p.records.first().map(|record| Pending {
                        record: record.clone(),
                        next: p.next,
                        delivered: false,
                    })
                });
            match loaded {
                Ok(Some(pending)) => self.pending = Some(pending),
                Ok(None) => {
                    self.cursor = None;
                    self.state = ReplayState::Ended;
                    return Ok(ReplayPoll::End);
                }
                Err(error) => return self.fail(ReplayError::Query(error)),
            }
        }
        let pending = self.pending.as_ref().ok_or(ReplayError::InvalidState)?;
        let distance =
            u128::from(pending.record.record.metadata.mono_ns).saturating_sub(self.anchor_media);
        let delay = (distance * u128::from(self.speed.denominator))
            .div_ceil(u128::from(self.speed.numerator));
        let Some(due) = u64::try_from(delay)
            .ok()
            .and_then(|n| self.anchor_host.checked_add(n))
        else {
            return self.fail(ReplayError::Limit);
        };
        let late = now.saturating_sub(due);
        if now < due || !consumer_ready {
            return Ok(ReplayPoll::Waiting {
                due_ns: due,
                late_ns: late,
            });
        }
        let media = u128::from(pending.record.record.metadata.mono_ns);
        if self.anchor_media > media {
            // A pause/rate change may have advanced the virtual playhead past
            // a blocked record. Resume the offline clock at its actual delivery
            // so later records retain their spacing instead of catching up.
            self.anchor_media = media;
            self.anchor_host = now;
        } else {
            // Here anchor_host <= due <= now, so anchor_host + late <= now.
            // Keep defensive failures terminal if scheduling changes later.
            let Some(anchor_host) = self.anchor_host.checked_add(late) else {
                return self.fail(ReplayError::Limit);
            };
            self.anchor_host = anchor_host;
        }
        let pending = self.pending.as_mut().ok_or(ReplayError::InvalidState)?;
        pending.delivered = true;
        self.cursor = pending.next.take();
        let payload = match &pending.record.payload {
            Payload::Inline { .. } => ReplayPayload::Inline(
                self.snapshot
                    .as_ref()
                    .and_then(|s| s.inline_payload(&pending.record))
                    .ok_or(ReplayError::Query(QueryError::CorruptData))?,
            ),
            Payload::Blob(handle) => ReplayPayload::Blob(handle),
        };
        Ok(ReplayPoll::Ready(ReplayFrame {
            record: &pending.record.record,
            payload,
            generation: self.generation,
            late_ns: late,
        }))
    }
    /// Read a bounded original blob for the current generation. Seek invalidates
    /// prior generations; query IO/read limits and expiry still apply.
    #[allow(clippy::too_many_arguments)]
    pub fn read_blob(
        &mut self,
        generation: u64,
        handle: &BlobHandle,
        offset: u64,
        length: usize,
        now_ns: u64,
        cancel: &AtomicBool,
    ) -> Result<&[u8], ReplayError> {
        let now = self.live(now_ns, cancel)?;
        if generation != self.generation {
            return Err(ReplayError::StaleGeneration);
        }
        let result = self
            .snapshot
            .as_mut()
            .ok_or(ReplayError::InvalidState)?
            .read_blob(handle, offset, length, now, cancel)
            .map(|_| ());
        if let Err(error) = result {
            return self.fail(ReplayError::Query(error));
        }
        self.snapshot
            .as_ref()
            .and_then(|s| s.blob_payload(length))
            .ok_or(ReplayError::Query(QueryError::CorruptData))
    }
}
