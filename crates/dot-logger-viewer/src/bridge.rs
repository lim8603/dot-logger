use crate::{
    Options,
    catalog::Channel,
    output::{Destination, Notice, Outbox},
};
use dot_logger_client::{
    Cancellation, ClientError, LiveBudget, LiveRecord, LiveSubscription, ReceiverConfig,
    RemoteConfig, RemoteEvent,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Mutex as AsyncMutex, Notify},
    task::JoinHandle,
    time::sleep,
};

const RETRY_FIRST: Duration = Duration::from_millis(250);
const RETRY_MAX: Duration = Duration::from_secs(5);

pub(crate) struct Bridge {
    upstream: Arc<Upstream>,
    capacity: usize,
    workers: AsyncMutex<HashMap<u32, Worker>>,
}

struct Upstream {
    remote: RemoteConfig,
    budget: LiveBudget,
    limits: ReceiverConfig,
}

struct Worker {
    feed: Arc<Feed>,
    task: JoinHandle<()>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Feed {
    channel: Channel,
    state: Mutex<FeedState>,
    stop: Notify,
}

#[derive(Default)]
struct FeedState {
    next: u64,
    listeners: BTreeMap<u64, Arc<Destination>>,
    connected: bool,
    stopping: bool,
}

impl FeedState {
    fn notify(&mut self, notice: Notice) {
        match notice {
            Notice::Connected { .. } => self.connected = true,
            Notice::Unknown { .. } => self.connected = false,
            _ => {}
        }
        for destination in self.listeners.values() {
            destination.notice(notice);
        }
    }
}

pub(crate) struct Lease {
    feed: Arc<Feed>,
    destination: Arc<Destination>,
    key: u64,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Bridge {
    pub(crate) fn new(remote: RemoteConfig, options: &Options) -> Result<Self, ClientError> {
        let budget = LiveBudget::new(
            options.feeds * options.receiver_records * (options.record_bytes + 1024),
        )?;
        Ok(Self {
            upstream: Arc::new(Upstream {
                remote,
                budget,
                limits: ReceiverConfig {
                    records: options.receiver_records,
                    record_bytes: options.record_bytes,
                    ttl: Duration::from_millis(options.receiver_age_ms),
                },
            }),
            capacity: options.feeds,
            workers: AsyncMutex::new(HashMap::new()),
        })
    }

    pub(crate) async fn subscribe(
        &self,
        channel: &Channel,
        subscription: u32,
        outbox: &Outbox,
    ) -> Option<Lease> {
        let mut workers = self.workers.lock().await;
        workers.retain(|_, worker| !worker.task.is_finished());
        if channel.retired() {
            return None;
        }
        let destination = Arc::new(Destination {
            subscription,
            active: AtomicBool::new(true),
            channel: channel.clone(),
            outbox: outbox.clone(),
        });
        let channel_id = channel.native_id;
        let mut reclaim = None;
        if let Some(worker) = workers.get(&channel_id) {
            let mut state = lock(&worker.feed.state);
            if channel.retired() {
                return None;
            }
            if !state.stopping {
                return worker.feed.add(&mut state, destination);
            }
            reclaim = Some(channel_id);
        } else if workers.len() == self.capacity {
            reclaim = Some(workers.iter().find_map(|(&id, worker)| {
                let state = lock(&worker.feed.state);
                (state.listeners.is_empty() || worker.feed.channel.retired()).then_some(id)
            })?);
        }
        if let Some(id) = reclaim {
            let worker = workers.get_mut(&id)?;
            worker.task.abort();
            // The task owns the subscription and receiver reservation. Wait for
            // both to drop before reusing its slot, even on a channel switch.
            let _ = (&mut worker.task).await;
            workers.remove(&id);
        }
        if channel.retired() {
            return None;
        }
        let feed = Arc::new(Feed {
            channel: channel.clone(),
            state: Mutex::new(FeedState::default()),
            stop: Notify::new(),
        });
        let lease = feed.add(&mut lock(&feed.state), destination)?;
        let upstream = self.upstream.clone();
        let running = feed.clone();
        let task = tokio::spawn(async move {
            tokio::select! {
                biased;
                _ = running.wait_until_idle() => {},
                _ = upstream.run(&running) => {},
            }
        });
        workers.insert(channel_id, Worker { feed, task });
        Some(lease)
    }

    pub(crate) async fn shutdown(&self) {
        let mut workers = std::mem::take(&mut *self.workers.lock().await);
        for worker in workers.values() {
            worker.task.abort();
        }
        for worker in workers.values_mut() {
            let _ = (&mut worker.task).await;
        }
    }
}

impl Feed {
    async fn wait_until_idle(&self) {
        loop {
            self.stop.notified().await;
            let mut state = lock(&self.state);
            if state.listeners.is_empty() {
                state.stopping = true;
                return;
            }
        }
    }

    fn add(
        self: &Arc<Self>,
        state: &mut FeedState,
        destination: Arc<Destination>,
    ) -> Option<Lease> {
        let key = state.next.checked_add(1)?;
        state.next = key;
        if state.connected {
            destination.notice(Notice::Connected {
                channel: self.channel.native_id,
                resumed: false,
            });
        }
        state.listeners.insert(key, destination.clone());
        Some(Lease {
            feed: self.clone(),
            destination,
            key,
        })
    }

    fn notify(&self, notice: Notice) {
        lock(&self.state).notify(notice);
    }

    fn offer(&self, record: LiveRecord, payload: &[u8]) -> bool {
        let channel = self.channel.native_id;
        if !self.channel.matches(record) {
            let mut state = lock(&self.state);
            self.channel.retire();
            state.notify(Notice::Unsupported {
                channel,
                reason: "local schema mapping does not match",
            });
            return false;
        }
        let Ok(timestamp) = u64::try_from(record.real_ns) else {
            self.notify(Notice::Unsupported {
                channel,
                reason: "timestamp precedes the Unix epoch",
            });
            return false;
        };
        let mut accepted = false;
        for destination in lock(&self.state).listeners.values() {
            // Every interested socket gets a chance even when an earlier one accepted.
            accepted |= destination.offer(channel, timestamp, payload);
        }
        accepted
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let mut state = lock(&self.feed.state);
        self.destination.active.store(false, Ordering::Release);
        state.listeners.remove(&self.key);
        if state.listeners.is_empty() {
            self.feed.stop.notify_one();
        }
    }
}

impl Upstream {
    async fn run(&self, feed: &Feed) {
        let channel = feed.channel.native_id;
        let cancel = Cancellation::new();
        let mut attempted = false;
        let mut delay = RETRY_FIRST;
        loop {
            match LiveSubscription::connect(
                self.remote.clone(),
                &self.budget,
                self.limits,
                channel,
                feed.channel.originals,
                &cancel,
            )
            .await
            {
                Ok(mut subscription) => {
                    feed.notify(Notice::Connected {
                        channel,
                        resumed: attempted,
                    });
                    let end = Self::forward(&mut subscription, feed, &cancel).await;
                    drop(subscription);
                    if feed.channel.retired() {
                        return;
                    }
                    feed.notify(Notice::Unknown {
                        channel,
                        reason: end,
                    });
                    delay = RETRY_FIRST;
                }
                Err(_) => feed.notify(Notice::Unknown {
                    channel,
                    reason: "upstream unavailable or subscription rejected",
                }),
            }
            attempted = true;
            sleep(delay).await;
            delay = delay.saturating_mul(2).min(RETRY_MAX);
        }
    }

    async fn forward(
        subscription: &mut LiveSubscription,
        feed: &Feed,
        cancel: &Cancellation,
    ) -> &'static str {
        loop {
            match subscription.next(cancel).await {
                Ok(Some(RemoteEvent::Record(record))) => {
                    let result = subscription
                        .handoff(record, |meta, bytes| feed.offer(meta, bytes), cancel)
                        .await;
                    match result {
                        Ok(()) => {}
                        // An unacknowledged head occupies upstream credit until
                        // expiry. Close promptly so a new lease can make progress.
                        Err(ClientError::ConsumerRejected) => return "no viewer accepted a record",
                        Err(_) => return "upstream acknowledgement was unconfirmed",
                    }
                }
                Ok(Some(RemoteEvent::Gap { first, last })) => feed.notify(Notice::UpstreamGap {
                    channel: feed.channel.native_id,
                    first,
                    last,
                }),
                Ok(None) | Err(_) => return "upstream stream ended",
            }
        }
    }
}
