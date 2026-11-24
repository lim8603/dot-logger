use super::types::*;
use crate::query::{QueryBudget, Reservation};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct Ledger {
    used: AtomicUsize,
    generations: Mutex<Vec<(u64, u64)>>,
    _reservation: Reservation,
}
struct Permit(Arc<Ledger>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.used.fetch_sub(1, Ordering::Release);
    }
}
struct Assembly {
    header: Chunk,
    data: Vec<u8>,
    expires: Instant,
    permit: Permit,
}
/// Synchronous receiver with bounded incomplete and handed-off-but-unaccepted records.
/// Transport-decoded fragment storage is separate; each input is capped at 256 KiB.
/// Own one receiver per actual transport connection. On reconnect, stop routing old
/// fragments and drop old unaccepted records; do not send their ACKs on the new
/// connection. Old owners retain their reservation until actually dropped.
pub struct Reassembler {
    ledger: Arc<Ledger>,
    frames: Vec<Assembly>,
    records: usize,
    bytes: usize,
    ttl: Duration,
    observed: Option<Instant>,
}
/// Complete verified record; retaining it retains one receiver permit.
/// No ACK is available until the consumer explicitly accepts the handoff.
pub struct ReceivedRecord {
    header: Chunk,
    data: Vec<u8>,
    expires: Instant,
    _permit: Permit,
}
impl ReceivedRecord {
    /// Attempt consumer handoff. Rejection/expiry returns this owner with no ACK.
    /// Consumer copies/queues, if any, belong to its separately budgeted memory.
    // Return the original bounded owner without allocating another error box.
    #[allow(clippy::result_large_err)]
    pub fn handoff(
        self,
        consumer: impl FnOnce(&crate::capture::CapturedRecord, &[u8]) -> bool,
    ) -> Result<Ack, Self> {
        let valid = self._permit.0.generations.lock().is_ok_and(|g| {
            !g.iter()
                .any(|(s, v)| *s == self.header.subscription && *v != self.header.generation)
        });
        if !valid || Instant::now() >= self.expires || !consumer(&self.header.record, &self.data) {
            return Err(self);
        }
        Ok(Ack {
            subscription: self.header.subscription,
            transfer: self.header.transfer,
            generation: self.header.generation,
            receipt: self.header.record.receipt,
        })
    }
}
impl Reassembler {
    /// External partition bytes one receiver reserves, including bookkeeping
    /// that differs by platform. Size a shared budget from this, never from
    /// the payload bytes alone (REV-072 RV-199 added Apple lock storage).
    pub fn reservation_bytes(
        records: usize,
        max_record_bytes: usize,
    ) -> Result<usize, RemoteError> {
        if records == 0 || max_record_bytes == 0 {
            return Err(RemoteError::InvalidInput);
        }
        max_record_bytes
            .checked_add(1024)
            .and_then(|n| n.checked_mul(records))
            .and_then(|n| n.checked_add(crate::sync::MUTEX_BYTES))
            .ok_or(RemoteError::Limit)
    }
    /// Reserve all potential record storage against the shared external partition.
    pub fn new(
        budget: &QueryBudget,
        records: usize,
        max_record_bytes: usize,
        ttl: Duration,
    ) -> Result<Self, RemoteError> {
        if records == 0
            || max_record_bytes == 0
            || ttl.is_zero()
            || ttl > Duration::from_secs(86400)
        {
            return Err(RemoteError::InvalidInput);
        }
        let bytes = Self::reservation_bytes(records, max_record_bytes)?;
        let reservation = budget.reserve(bytes).map_err(|_| RemoteError::Limit)?;
        Ok(Self {
            ledger: Arc::new(Ledger {
                used: AtomicUsize::new(0),
                generations: crate::sync::warm_mutex(Vec::with_capacity(records)),
                _reservation: reservation,
            }),
            frames: Vec::with_capacity(records),
            records,
            bytes: max_record_bytes,
            ttl,
            observed: None,
        })
    }
    /// Expire incomplete assemblies using monotonically clamped observations.
    pub fn expire(&mut self, now: Instant) {
        let now = self.observed.map_or(now, |old| old.max(now));
        self.observed = Some(now);
        self.frames.retain(|f| now < f.expires);
    }
    /// Validate contiguous fragments and CRC; corrupt matching assemblies are discarded.
    /// Complete records keep their capacity permit until accepted or dropped.
    pub fn push(
        &mut self,
        mut chunk: Chunk,
        now: Instant,
    ) -> Result<Option<ReceivedRecord>, RemoteError> {
        self.expire(now);
        if self
            .ledger
            .generations
            .lock()
            .map_err(|_| RemoteError::Transport)?
            .iter()
            .any(|(s, g)| *s == chunk.subscription && *g != chunk.generation)
        {
            return Err(RemoteError::Gone);
        }
        let now = self.observed.ok_or(RemoteError::InvalidInput)?;
        let key = |f: &Assembly| {
            f.header.subscription == chunk.subscription && f.header.transfer == chunk.transfer
        };
        let mut index = self.frames.iter().position(key);
        let valid = valid_record(chunk.record)
            && chunk.subscription != 0
            && chunk.transfer != 0
            && chunk.data.len() <= MAX_CHUNK_BYTES
            && chunk.total == chunk.record.payload_len as u64
            && chunk.total <= self.bytes as u64
            && chunk
                .offset
                .checked_add(chunk.data.len() as u64)
                .is_some_and(|n| n <= chunk.total)
            && (!chunk.data.is_empty() || chunk.total == 0);
        if !valid {
            if let Some(i) = index {
                self.frames.swap_remove(i);
            }
            return Err(RemoteError::Corrupt);
        }
        if index.is_none() {
            if chunk.offset != 0 {
                return Err(RemoteError::Corrupt);
            }
            if self.ledger.used.load(Ordering::Acquire) >= self.records {
                return Err(RemoteError::Limit);
            }
            let expires = now.checked_add(self.ttl).ok_or(RemoteError::Limit)?;
            self.ledger.used.fetch_add(1, Ordering::AcqRel);
            let data = std::mem::take(&mut chunk.data);
            let mut buffer = Vec::with_capacity(chunk.total as usize);
            buffer.extend_from_slice(&data);
            self.frames.push(Assembly {
                header: chunk,
                data: buffer,
                expires,
                permit: Permit(self.ledger.clone()),
            });
            index = Some(self.frames.len() - 1);
        } else {
            let i = index.ok_or(RemoteError::Corrupt)?;
            let h = &self.frames[i].header;
            if h.record != chunk.record
                || h.total != chunk.total
                || h.generation != chunk.generation
                || h.checksum != chunk.checksum
                || h.original != chunk.original
                || h.late != chunk.late
                || chunk.offset != self.frames[i].data.len() as u64
            {
                self.frames.swap_remove(i);
                return Err(RemoteError::Corrupt);
            }
            self.frames[i].data.extend_from_slice(&chunk.data);
        }
        let i = index.ok_or(RemoteError::Corrupt)?;
        if self.frames[i].data.len() as u64 != self.frames[i].header.total {
            return Ok(None);
        }
        let frame = self.frames.swap_remove(i);
        if checksum(&frame.data) != frame.header.checksum {
            return Err(RemoteError::Corrupt);
        }
        Ok(Some(ReceivedRecord {
            header: frame.header,
            data: frame.data,
            expires: frame.expires,
            _permit: frame.permit,
        }))
    }
    /// Advance a bounded generation gate and discard old incomplete records.
    /// Future fragments and ready-but-unaccepted records also check this gate.
    /// Tracking is limited to the configured record count; recreate the receiver
    /// for a new connection. Already accepted consumer data cannot be revoked.
    /// Distinct subscription gates persist for this receiver's lifetime, even
    /// after their incomplete records expire. Repeated advances of the same gate
    /// do not consume another slot. On `Limit`, reconnect with a new receiver;
    /// erasing a gate could allow late stale fragments or handoffs through.
    pub fn discard_generations(
        &mut self,
        subscription: u64,
        generation: u64,
    ) -> Result<(), RemoteError> {
        if subscription == 0 || generation == 0 {
            return Err(RemoteError::InvalidInput);
        }
        let mut gates = self
            .ledger
            .generations
            .lock()
            .map_err(|_| RemoteError::Transport)?;
        if let Some((_, old)) = gates.iter_mut().find(|(s, _)| *s == subscription) {
            if generation < *old {
                return Err(RemoteError::Gone);
            }
            *old = generation;
        } else {
            if gates.len() >= self.records {
                return Err(RemoteError::Limit);
            }
            gates.push((subscription, generation));
        }
        drop(gates);
        self.frames
            .retain(|f| f.header.subscription != subscription || f.header.generation == generation);
        Ok(())
    }
}
