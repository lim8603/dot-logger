use super::types::*;
use crate::{
    completion::{Boundary, CompletionError, Update},
    query::{QueryBudget, Reservation},
    routing::{Delivery, DropReason},
};
use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    task::Waker,
    time::Instant,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot};

/// Cloneable general-path admission authority. Dropping the last hub does not
/// reclaim reservations still owned by streams, pending commands or ACK handles.
#[derive(Clone)]
pub struct RemoteHub {
    pub(crate) shared: Arc<Shared>,
}
pub(crate) struct Shared {
    pub limits: RemoteLimits,
    pub closed: AtomicBool,
    subs: Mutex<Registry>,
    pub commands: mpsc::Sender<Command>,
    pub command_permits: Arc<Semaphore>,
    _reservation: Reservation,
}
struct Registry {
    next: u64,
    slots: Vec<Weak<Subscription>>,
}
/// Host-driven bounded command inbox, independent of the application's runtime.
pub struct CommandInbox {
    receiver: mpsc::Receiver<Command>,
}
/// One host operation. Keep this owner through actual IO/reader cleanup, including
/// after client timeout. Unsupported operations must be explicitly rejected.
pub struct Command {
    request: ManagementRequest,
    reply: Option<oneshot::Sender<Result<ManagementReply, RemoteError>>>,
    _permit: OwnedSemaphorePermit,
    _shared: Arc<Shared>,
}
impl Command {
    /// Borrow the validated request.
    pub fn request(&self) -> &ManagementRequest {
        &self.request
    }
    /// Whether the remote waiter was cancelled or timed out. This does not
    /// certify that application IO ended or permit early reader close.
    pub fn cancelled(&self) -> bool {
        self.reply.as_ref().is_none_or(|tx| tx.is_closed())
    }
    /// Return a real result after actual work/cleanup; late replies are discarded.
    /// Excess reply allocation capacity is rejected before entering the reply queue.
    pub fn respond(mut self, result: Result<ManagementReply, RemoteError>) {
        let result = result.and_then(|reply| {
            reply.check_buffer_limit(self._shared.limits.message_bytes)?;
            if let (ManagementRequest::ReadBlob { length, .. }, ManagementReply::Blob(data)) =
                (&self.request, &reply)
            {
                if data.len() > *length as usize {
                    return Err(RemoteError::Limit);
                }
            }
            Ok(reply)
        });
        if let Some(tx) = self.reply.take() {
            let _ = tx.send(result);
        }
    }
}
impl CommandInbox {
    /// One nonwaiting host turn; returning None never implies completion.
    pub fn try_next(&mut self) -> Option<Command> {
        self.receiver.try_recv().ok()
    }
}
/// Observation handle for one transfer. Sending bytes alone leaves Pending.
pub struct PendingAck {
    state: Arc<AtomicU8>,
    expires: Instant,
    _shared: Arc<Shared>,
}
impl PendingAck {
    /// Observe expiration without inventing a receiving ACK.
    pub fn state(&self) -> AckState {
        if Instant::now() >= self.expires {
            let _ = self
                .state
                .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
        }
        match self.state.load(Ordering::Acquire) {
            1 => AckState::Acknowledged,
            2 => AckState::Unknown,
            3 => AckState::Dropped,
            _ => AckState::Pending,
        }
    }
}
/// Retains the actual routing delivery until its terminal observation can be
/// published. Dropping an unreported guard preserves the core's Unknown behavior.
pub struct RemoteDelivery<'owner> {
    delivery: Option<Delivery<'owner>>,
    ack: PendingAck,
}
impl RemoteDelivery<'_> {
    /// Publish ReceiverAcknowledged only after validated receiver ACK. Busy keeps
    /// the delivery for a later bounded host turn; no spin loop is performed.
    pub fn poll(&mut self) -> Result<AckState, CompletionError> {
        let state = self.ack.state();
        if let Some(delivery) = self.delivery.as_ref() {
            let update = match state {
                AckState::Pending => return Ok(state),
                AckState::Acknowledged => Update::Confirmed(Boundary::ReceiverAcknowledged),
                AckState::Unknown => Update::Unknown,
                AckState::Dropped => Update::Dropped(DropReason::Capacity),
            };
            delivery.report(update)?;
            self.delivery = None;
        }
        Ok(state)
    }
}
struct Transfer {
    chunk: Chunk,
    offset: usize,
    sent: bool,
    expires: Instant,
    status: Arc<AtomicU8>,
}
pub(crate) struct Subscription {
    pub id: u64,
    pub peer: SocketAddr,
    shared: Arc<Shared>,
    filter: SubscriptionFilter,
    records: usize,
    bytes: usize,
    age: std::time::Duration,
    state: Mutex<SubState>,
}
struct SubState {
    next: u64,
    records: VecDeque<Transfer>,
    used: usize,
    gap: Option<(u64, u64)>,
    closed: bool,
    waker: Option<Waker>,
    last_mono: Option<u64>,
}
pub(crate) enum Event {
    Chunk(Chunk),
    Gap {
        subscription: u64,
        first: u64,
        last: u64,
    },
}
impl Subscription {
    pub fn close(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.closed = true;
            for t in s.records.drain(..) {
                let _ = t
                    .status
                    .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
            }
            s.used = 0;
            if let Some(w) = s.waker.take() {
                w.wake();
            }
        }
    }
    fn expire(s: &mut SubState, now: Instant) {
        if s.records.iter().any(|t| now >= t.expires) {
            s.closed = true;
            for t in &s.records {
                let _ = t
                    .status
                    .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
            }
        }
        s.records.retain(|t| {
            if now >= t.expires {
                let _ = t
                    .status
                    .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
            }
            t.status.load(Ordering::Acquire) == 0
        });
        s.used = s.records.iter().map(|t| t.chunk.data.len()).sum();
    }
    pub fn next(&self, waker: &Waker) -> Result<Option<Event>, RemoteError> {
        let mut s = self.state.lock().map_err(|_| RemoteError::Transport)?;
        Self::expire(&mut s, Instant::now());
        if s.closed || self.shared.closed.load(Ordering::Acquire) {
            return Err(RemoteError::Gone);
        }
        if let Some(i) = s.records.iter().position(|t| !t.sent) {
            let max = self.shared.limits.chunk_bytes();
            let mono = s.records[i].chunk.record.metadata.mono_ns;
            let late = s.last_mono.is_some_and(|last| mono < last);
            s.last_mono = Some(s.last_mono.map_or(mono, |old| old.max(mono)));
            let t = &mut s.records[i];
            let end = t.offset.saturating_add(max).min(t.chunk.data.len());
            let chunk = Chunk {
                subscription: t.chunk.subscription,
                transfer: t.chunk.transfer,
                generation: t.chunk.generation,
                record: t.chunk.record,
                total: t.chunk.total,
                offset: t.offset as u64,
                checksum: t.chunk.checksum,
                data: t.chunk.data[t.offset..end].to_vec(),
                original: t.chunk.original,
                late,
            };
            t.offset = end;
            t.sent = end == t.chunk.data.len();
            return Ok(Some(Event::Chunk(chunk)));
        }
        if let Some((first, last)) = s.gap.take() {
            return Ok(Some(Event::Gap {
                subscription: self.id,
                first,
                last,
            }));
        }
        s.waker = Some(waker.clone());
        Ok(None)
    }
    pub fn ack(&self, ack: Ack, peer: SocketAddr) -> Result<(), RemoteError> {
        if peer != self.peer {
            return Err(RemoteError::PermissionDenied);
        }
        let mut s = self.state.lock().map_err(|_| RemoteError::Transport)?;
        Self::expire(&mut s, Instant::now());
        if s.closed {
            return Err(RemoteError::Gone);
        }
        let i = s
            .records
            .iter()
            .position(|t| t.chunk.transfer == ack.transfer)
            .ok_or(RemoteError::Gone)?;
        let t = &s.records[i];
        if !t.sent || t.chunk.generation != ack.generation || t.chunk.record.receipt != ack.receipt
        {
            return Err(RemoteError::InvalidInput);
        }
        let t = s.records.remove(i).ok_or(RemoteError::Gone)?;
        s.used -= t.chunk.data.len();
        t.status
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| RemoteError::Gone)?;
        Ok(())
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        self.close();
    }
}
impl RemoteHub {
    /// Pre-reserve all adapter-owned queue/buffer limits; never binds a listener.
    pub fn new(
        budget: &QueryBudget,
        limits: RemoteLimits,
    ) -> Result<(Self, CommandInbox), RemoteError> {
        let bytes = limits.reservation()?;
        let reservation = budget.reserve(bytes).map_err(|_| RemoteError::Limit)?;
        let (commands, receiver) = mpsc::channel(limits.commands);
        let permits = Arc::new(Semaphore::new(limits.commands));
        let slots = Vec::with_capacity(limits.subscriptions);
        let shared = Arc::new(Shared {
            limits,
            closed: AtomicBool::new(false),
            subs: Mutex::new(Registry { next: 1, slots }),
            commands,
            command_permits: permits,
            _reservation: reservation,
        });
        Ok((Self { shared }, CommandInbox { receiver }))
    }
    /// Copy active subscription identities into bounded host storage.
    pub fn subscriptions(&self, out: &mut [u64]) -> usize {
        let Ok(reg) = self.shared.subs.lock() else {
            return 0;
        };
        let mut count = 0;
        for sub in reg.slots.iter().filter_map(Weak::upgrade) {
            if count == out.len() {
                break;
            }
            out[count] = sub.id;
            count += 1;
        }
        count
    }
    pub(crate) fn subscribe(
        &self,
        peer: SocketAddr,
        filter: SubscriptionFilter,
        records: usize,
        bytes: usize,
        age: std::time::Duration,
    ) -> Result<Arc<Subscription>, RemoteError> {
        if self.shared.closed.load(Ordering::Acquire) {
            return Err(RemoteError::Gone);
        }
        let l = &self.shared.limits;
        if records == 0
            || records > l.subscription_records
            || bytes == 0
            || bytes > l.subscription_bytes
            || age.is_zero()
            || age > l.max_age
        {
            return Err(RemoteError::InvalidInput);
        }
        let mut r = self
            .shared
            .subs
            .lock()
            .map_err(|_| RemoteError::Transport)?;
        r.slots.retain(|s| s.strong_count() > 0);
        if r.slots.len() >= l.subscriptions {
            return Err(RemoteError::Limit);
        }
        let id = r.next;
        r.next = r.next.checked_add(1).ok_or(RemoteError::Limit)?;
        let sub = Arc::new(Subscription {
            id,
            peer,
            shared: self.shared.clone(),
            filter,
            records,
            bytes,
            age,
            state: Mutex::new(SubState {
                next: 1,
                records: VecDeque::with_capacity(records),
                used: 0,
                gap: None,
                closed: false,
                waker: None,
                last_mono: None,
            }),
        });
        r.slots.push(Arc::downgrade(&sub));
        Ok(sub)
    }
    fn find(&self, id: u64) -> Result<Arc<Subscription>, RemoteError> {
        self.shared
            .subs
            .lock()
            .map_err(|_| RemoteError::Transport)?
            .slots
            .iter()
            .filter_map(Weak::upgrade)
            .find(|s| s.id == id)
            .ok_or(RemoteError::Gone)
    }
    /// Offer one copy on the general path. Raw opt-out/filter mismatch returns
    /// Unsupported without consuming capacity. Overflow affects this stream only.
    pub fn offer(
        &self,
        subscription: u64,
        record: crate::capture::CapturedRecord,
        payload: &[u8],
        original: bool,
        generation: u64,
    ) -> Result<PendingAck, RemoteError> {
        if !valid_record(record)
            || record.payload_len != payload.len()
            || payload.len() > self.shared.limits.record_bytes
        {
            return Err(RemoteError::InvalidInput);
        }
        let sub = self.find(subscription)?;
        if (original && !sub.filter.originals)
            || sub
                .filter
                .channel
                .is_some_and(|c| c != record.metadata.channel_id)
            || record.metadata.level < sub.filter.minimum_level
        {
            return Err(RemoteError::Unsupported);
        }
        let now = Instant::now();
        let expires = now.checked_add(sub.age).ok_or(RemoteError::Limit)?;
        let status = Arc::new(AtomicU8::new(0));
        let pending = PendingAck {
            state: status.clone(),
            expires,
            _shared: self.shared.clone(),
        };
        let mut s = sub.state.lock().map_err(|_| RemoteError::Transport)?;
        Subscription::expire(&mut s, now);
        if s.closed || self.shared.closed.load(Ordering::Acquire) {
            return Err(RemoteError::Gone);
        }
        let transfer = s.next;
        s.next = s.next.checked_add(1).ok_or(RemoteError::Limit)?;
        if s.gap.is_some()
            || s.records.len() >= sub.records
            || payload.len() > sub.bytes.saturating_sub(s.used)
        {
            match sub.filter.overflow {
                OverflowPolicy::Gap => {
                    status.store(3, Ordering::Release);
                    s.gap = Some(
                        s.gap
                            .map_or((transfer, transfer), |(first, _)| (first, transfer)),
                    );
                }
                OverflowPolicy::Disconnect => {
                    s.closed = true;
                    status.store(2, Ordering::Release);
                    for t in s.records.drain(..) {
                        let _ =
                            t.status
                                .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
                    }
                    s.used = 0;
                }
            }
        } else {
            let chunk = Chunk::single(
                subscription,
                transfer,
                generation,
                record,
                payload,
                original,
            );
            s.records.push_back(Transfer {
                chunk,
                offset: 0,
                sent: false,
                expires,
                status,
            });
            s.used += payload.len();
        }
        if let Some(w) = s.waker.take() {
            w.wake();
        }
        Ok(pending)
    }
    /// Keep a real routing guard until receiver ACK/Unknown can be published.
    /// Failure returns the unchanged delivery for explicit host handling.
    pub fn offer_delivery<'owner>(
        &self,
        subscription: u64,
        delivery: Delivery<'owner>,
        original: bool,
        generation: u64,
    ) -> Result<RemoteDelivery<'owner>, (RemoteError, Delivery<'owner>)> {
        match self.offer(
            subscription,
            delivery.record(),
            delivery.payload(),
            original,
            generation,
        ) {
            Ok(ack) => Ok(RemoteDelivery {
                delivery: Some(delivery),
                ack,
            }),
            Err(error) => Err((error, delivery)),
        }
    }
    /// Discard queued/unacknowledged old generations; already copied/in-flight
    /// network bytes cannot be recalled. Receiver must also filter generations.
    pub fn discard_generations(
        &self,
        subscription: u64,
        generation: u64,
    ) -> Result<(), RemoteError> {
        let sub = self.find(subscription)?;
        let mut s = sub.state.lock().map_err(|_| RemoteError::Transport)?;
        for t in &s.records {
            if t.chunk.generation != generation {
                let _ = t
                    .status
                    .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
            }
        }
        Subscription::expire(&mut s, Instant::now());
        Ok(())
    }
    pub(crate) fn ack(&self, ack: Ack, peer: SocketAddr) -> Result<(), RemoteError> {
        self.find(ack.subscription)?.ack(ack, peer)
    }
    pub(crate) async fn request(
        &self,
        request: ManagementRequest,
    ) -> Result<ManagementReply, RemoteError> {
        if self.shared.closed.load(Ordering::Acquire) {
            return Err(RemoteError::Gone);
        }
        let permit = self
            .shared
            .command_permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| RemoteError::Limit)?;
        let (tx, rx) = oneshot::channel();
        self.shared
            .commands
            .try_send(Command {
                request,
                reply: Some(tx),
                _permit: permit,
                _shared: self.shared.clone(),
            })
            .map_err(|_| RemoteError::Limit)?;
        tokio::time::timeout(self.shared.limits.request_timeout, rx)
            .await
            .map_err(|_| RemoteError::TimedOut)?
            .map_err(|_| RemoteError::Gone)?
    }
    /// Stop stream admission and mark all pending transfers Unknown. Host-owned
    /// commands/actual IO still retain their permits until the host drops them.
    pub fn close(&self) {
        self.shared.closed.store(true, Ordering::Release);
        if let Ok(r) = self.shared.subs.lock() {
            for s in r.slots.iter().filter_map(Weak::upgrade) {
                s.close();
            }
        }
    }
}
