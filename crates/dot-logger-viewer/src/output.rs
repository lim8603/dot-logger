use crate::catalog::Channel;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

pub(crate) const DATA_HEADER: usize = 13;

pub(crate) fn data_message(subscription: u32, timestamp: u64, payload: &[u8]) -> Message {
    let mut data = Vec::with_capacity(DATA_HEADER + payload.len());
    data.push(1);
    data.extend_from_slice(&subscription.to_le_bytes());
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(payload);
    Message::binary(data)
}

#[derive(Clone, Copy)]
pub(crate) enum Notice {
    Connected { channel: u32, resumed: bool },
    Unknown { channel: u32, reason: &'static str },
    UpstreamGap { channel: u32, first: u64, last: u64 },
    QueueGap { channel: u32 },
    Unsupported { channel: u32, reason: &'static str },
}

const NOTICE_KINDS: usize = 5;

#[derive(Clone, Copy, Default)]
pub(crate) struct Notices([Option<(u64, Notice)>; NOTICE_KINDS]);

impl Notices {
    fn update(&mut self, notice: Notice) {
        let (index, count) = match notice {
            Notice::Unknown { .. } => (0, 1),
            Notice::Connected { .. } => (1, 1),
            Notice::UpstreamGap { first, last, .. } => (2, last - first + 1),
            Notice::QueueGap { .. } => (3, 1),
            Notice::Unsupported { .. } => (4, 1),
        };
        let total = self.0[index]
            .map_or(0, |(total, _)| total)
            .saturating_add(count);
        self.0[index] = Some((total, notice));
    }

    pub(crate) fn statuses(self, previous: &mut Self) -> Vec<Message> {
        let messages = self
            .0
            .into_iter()
            .zip(previous.0)
            .filter_map(|(new, old)| {
                let (total, item) = new?;
                (total != old.map_or(0, |(last, _)| last)).then(|| item.message(total))
            })
            .collect();
        *previous = self;
        messages
    }
}

impl Notice {
    fn message(self, total: u64) -> Message {
        let (id, level, message) = match self {
            Self::Connected { channel, resumed } => {
                let state = if resumed { "Reconnected" } else { "Connected" };
                (
                    "upstream-connected",
                    0,
                    format!("{state}: channel {channel}"),
                )
            }
            Self::Unknown { channel, reason } => (
                "upstream-unknown",
                1,
                format!("Unknown: channel {channel}, {reason}; missing range unavailable"),
            ),
            Self::UpstreamGap {
                channel,
                first,
                last,
            } => (
                "upstream-gap",
                1,
                format!(
                    "Upstream Gap: {total} missing transfers; last channel {channel}, {first}..={last} (subscription-local)"
                ),
            ),
            Self::QueueGap { channel } => (
                "viewer-gap",
                1,
                format!(
                    "Viewer Gap: {total} records dropped by this socket's queue; last channel {channel}"
                ),
            ),
            Self::Unsupported { channel, reason } => (
                "unsupported-record",
                1,
                format!("Unsupported record: channel {channel}, {reason}; {total} observed"),
            ),
        };
        status(id, level, message)
    }
}

pub(crate) fn status(id: &str, level: u8, message: impl AsRef<str>) -> Message {
    Message::text(
        json!({"op":"status","id":id,"level":level,"message":message.as_ref()}).to_string(),
    )
}

#[derive(Clone)]
pub(crate) struct Outbox {
    data: mpsc::Sender<Frame>,
    bytes: Arc<Semaphore>,
    notices: watch::Sender<Notices>,
}

impl Outbox {
    pub(crate) fn new(
        frames: usize,
        bytes: usize,
    ) -> (Self, mpsc::Receiver<Frame>, watch::Receiver<Notices>) {
        let (data, receiver) = mpsc::channel(frames);
        let (notices, watcher) = watch::channel(Notices::default());
        (
            Self {
                data,
                bytes: Arc::new(Semaphore::new(bytes)),
                notices,
            },
            receiver,
            watcher,
        )
    }
}

pub(crate) struct Destination {
    pub(crate) subscription: u32,
    pub(crate) active: AtomicBool,
    pub(crate) channel: Channel,
    pub(crate) outbox: Outbox,
}

impl Destination {
    pub(crate) fn notice(&self, notice: Notice) {
        self.outbox
            .notices
            .send_modify(|notices| notices.update(notice));
    }

    pub(crate) fn offer(self: &Arc<Self>, channel: u32, timestamp: u64, payload: &[u8]) -> bool {
        let Ok(slot) = self.outbox.data.try_reserve() else {
            if !self.outbox.data.is_closed() {
                self.notice(Notice::QueueGap { channel });
            }
            return false;
        };
        let size = DATA_HEADER + payload.len();
        let Ok(permit) = self
            .outbox
            .bytes
            .clone()
            .try_acquire_many_owned(size as u32)
        else {
            self.notice(Notice::QueueGap { channel });
            return false;
        };
        slot.send(Frame {
            destination: self.clone(),
            message: data_message(self.subscription, timestamp, payload),
            _permit: permit,
        });
        true
    }
}

pub(crate) struct Frame {
    destination: Arc<Destination>,
    message: Message,
    _permit: OwnedSemaphorePermit,
}

impl Frame {
    pub(crate) fn into_send(self) -> Option<(Message, OwnedSemaphorePermit)> {
        (self.destination.active.load(Ordering::Acquire) && !self.destination.channel.retired())
            .then_some((self.message, self._permit))
    }
}
