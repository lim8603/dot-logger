//! Default worker: one management host thread plus file IO threads
//! (ADR-012 §4–5). Every core borrow lives on these threads' stacks; the RT
//! side only holds `Writer`s, so the existing scoped contracts are unchanged.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::capture::{CaptureCore, ReadState};
use crate::completion::{Boundary, CompletionError, FailureReason, ProducerRange, Update};
use crate::config::ValidatedConfig;
use crate::control::{
    ControlConfig, ControlOwner, ShutdownPlan, ShutdownPoll, ShutdownReport, SyncResult, SyncToken,
    SyncWork,
};
use crate::diagnostics::linux_available_bytes;
use crate::platform::Requirement;
use crate::routing::{Delivery, Router, RoutingConfig, TargetConsumer};
use crate::storage::{
    FileStore, RetentionBatchError, RetentionConfig, RetentionError, SegmentLog, StorageChannel,
    StorageError, WriterConfig,
};

use super::affinity::pin_current;
use super::budget::{self, STACK_BYTES, Tuning};
use super::health::{AFFINITY_APPLIED, AFFINITY_FAILED, AFFINITY_NONE, OutputStatus, Shared};
use super::remote::{Remote, Tap};
use super::writer::Owned;
use super::{FacadeReport, StartError};

type Probe = Box<dyn FnMut() -> std::io::Result<u64> + Send>;
type Log = SegmentLog<FileStore<Probe>>;

/// Management command from the `Logger` handle.
pub(crate) enum Command {
    Shutdown {
        deadline: Instant,
        reply: Sender<Progress>,
    },
}

/// Shutdown progress sent back to the waiting handle.
pub(crate) enum Progress {
    Incomplete(FacadeReport),
    Closed(FacadeReport),
}

/// Everything the host thread owns. Moved in once; nothing returns.
pub(crate) struct HostPlan {
    pub(crate) core: Arc<CaptureCore>,
    pub(crate) reads: Vec<ReadState>,
    pub(crate) mailboxes: Vec<rtrb::Consumer<Owned>>,
    pub(crate) plan: ValidatedConfig,
    pub(crate) routing: RoutingConfig,
    pub(crate) control: ControlConfig,
    pub(crate) writer: WriterConfig,
    pub(crate) retention: Vec<RetentionConfig>,
    pub(crate) channels: Vec<StorageChannel>,
    pub(crate) outputs: Vec<PathBuf>,
    pub(crate) tuning: Tuning,
    pub(crate) origin: Instant,
    pub(crate) shared: Arc<Shared>,
    /// Validated worker CPU list.
    pub(crate) cpus: Option<(Vec<usize>, Requirement)>,
    /// Running remote server, stopped by the host before it reports Closed.
    pub(crate) remote: Remote,
}

/// A host that failed to start. When `host` is present the thread is still
/// cleaning up (listener, reaper) and must be joined once the caller dropped
/// every writer it still holds (REV-051 RV-122).
pub(crate) struct SpawnFailed {
    pub(crate) error: StartError,
    pub(crate) host: Option<JoinHandle<()>>,
}

/// Start the host and wait until it built every component or failed.
pub(crate) fn spawn(
    plan: HostPlan,
    commands: Receiver<Command>,
) -> Result<(Vec<PathBuf>, JoinHandle<()>), SpawnFailed> {
    let (ready, started) = mpsc::sync_channel(1);
    let handle = std::thread::Builder::new()
        .name("dot-logger-host".into())
        .stack_size(STACK_BYTES)
        .spawn(move || host_main(plan, commands, ready))
        .map_err(|error| SpawnFailed {
            error: StartError::Io(error),
            host: None,
        })?;
    let error = match started.recv() {
        Ok(Ok(directories)) => return Ok((directories, handle)),
        Ok(Err(error)) => error,
        Err(_) => StartError::WorkerFailed,
    };
    Err(SpawnFailed {
        error,
        host: Some(handle),
    })
}

fn ns(origin: Instant, now: Instant) -> u64 {
    u64::try_from(now.saturating_duration_since(origin).as_nanos()).unwrap_or(u64::MAX)
}

/// Closes RT admission on every host exit path, including unwinding.
struct CloseOnExit<'a>(&'a CaptureCore);
impl Drop for CloseOnExit<'_> {
    fn drop(&mut self) {
        self.0.control().close();
    }
}

/// Stops IO threads on every host-loop exit path, including unwinding.
struct StopOnExit<'a>(&'a Shared);
impl Drop for StopOnExit<'_> {
    fn drop(&mut self) {
        self.0.stop.store(true, Ordering::Release);
    }
}

/// True once every writer finished dropping, i.e. released its mailbox
/// producer. A pushed state alone is not enough: the writer's drop glue may
/// still be running, and whichever side drops its ring end last frees the ring
/// and the state in it. Waiting for abandonment keeps this thread the last
/// dropper (REV-051 RV-120).
pub(crate) fn writers_returned<T>(mailboxes: &[rtrb::Consumer<T>]) -> bool {
    mailboxes.iter().all(rtrb::Consumer::is_abandoned)
}

/// Writers that have not finished dropping yet.
pub(crate) fn writers_outstanding<T>(mailboxes: &[rtrb::Consumer<T>]) -> usize {
    mailboxes.iter().filter(|m| !m.is_abandoned()).count()
}

/// Sleep of an idle IO thread: `idle_wait`, shortened only to the remaining
/// hold time of a batch that is already held.
pub(crate) fn idle_sleep(
    idle_wait: Duration,
    batch_wait: Duration,
    held: impl IntoIterator<Item = Instant>,
    now: Instant,
) -> Duration {
    held.into_iter()
        .map(|since| batch_wait.saturating_sub(now.saturating_duration_since(since)))
        .fold(idle_wait, Duration::min)
}

fn host_main(
    plan: HostPlan,
    commands: Receiver<Command>,
    ready: SyncSender<Result<Vec<PathBuf>, StartError>>,
) {
    let HostPlan {
        core,
        mut reads,
        mut mailboxes,
        plan,
        routing,
        control,
        writer,
        retention,
        channels,
        outputs,
        tuning,
        origin,
        shared,
        cpus,
        mut remote,
    } = plan;
    let mut ready = Some(ready);
    {
        let close = CloseOnExit(&core);
        let flags = &*shared;
        let config = Config {
            plan: &plan,
            control,
            writer,
            retention: &retention,
            channels: &channels,
            outputs: &outputs,
            tuning,
            origin,
            flags,
            cpus: cpus.as_ref(),
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            pin(&config)?;
            run(
                &core,
                &mut reads,
                &mut mailboxes,
                routing,
                &config,
                Links {
                    commands: &commands,
                    ready: &mut ready,
                    remote: &mut remote,
                },
            )
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                flags.worker_failed.store(true, Ordering::Release);
                if let Some(ready) = ready.take() {
                    let _ = ready.send(Err(error));
                }
            }
            Err(_) => flags.worker_failed.store(true, Ordering::Release),
        }
        if let Some(ready) = ready.take() {
            let _ = ready.send(Err(StartError::WorkerFailed));
        }
        drop(close);
    }
    // No more commands are read: a later `shutdown` fails fast as WorkerFailed
    // instead of waiting for a reply that never comes.
    drop(commands);
    // Every exit path (including failure and panic) stops the listener.
    remote.stop();
    // Reaper: an outstanding Writer still points into `core` and its rings.
    // Keep this thread as their last owner until every writer finished
    // dropping, so no RT thread ever performs the final release (ADR-012 §3).
    let mut wait = tuning.idle_wait.max(Duration::from_millis(1));
    while !writers_returned(&mailboxes) {
        std::thread::sleep(wait);
        wait = wait.saturating_mul(2).min(Duration::from_millis(100));
    }
    drop(mailboxes);
    drop(reads);
    drop(core);
}

struct Config<'a> {
    plan: &'a ValidatedConfig,
    control: ControlConfig,
    writer: WriterConfig,
    retention: &'a [RetentionConfig],
    channels: &'a [StorageChannel],
    outputs: &'a [PathBuf],
    tuning: Tuning,
    origin: Instant,
    flags: &'a Shared,
    cpus: Option<&'a (Vec<usize>, Requirement)>,
}

/// The host's management connections: handle commands, the start-up report
/// and the remote server.
struct Links<'a> {
    commands: &'a Receiver<Command>,
    ready: &'a mut Option<SyncSender<Result<Vec<PathBuf>, StartError>>>,
    remote: &'a mut Remote,
}

/// Restrict the calling worker thread to the configured CPUs. A `Required`
/// placement that fails rejects startup; an `Optional` one is reported.
fn pin(config: &Config<'_>) -> Result<(), StartError> {
    let Some((cpus, requirement)) = config.cpus else {
        return Ok(());
    };
    match pin_current(cpus) {
        Ok(()) => {
            // Another worker may already have recorded a failure; keep it.
            let _ = config.flags.affinity.compare_exchange(
                AFFINITY_NONE,
                AFFINITY_APPLIED,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            Ok(())
        }
        Err(error) if *requirement == Requirement::Required => Err(StartError::Io(error)),
        Err(_) => {
            config
                .flags
                .affinity
                .store(AFFINITY_FAILED, Ordering::Release);
            Ok(())
        }
    }
}

/// Remove namespaces created by a failed start; never touches other paths.
fn remove_created(directories: &[PathBuf]) {
    for dir in directories {
        let _ = std::fs::remove_dir_all(dir);
        if let Some(session) = dir.parent() {
            let _ = std::fs::remove_dir(session);
        }
    }
}

fn open_logs(
    router: &Router<'_>,
    config: &Config<'_>,
) -> Result<(Vec<Log>, Vec<PathBuf>), StartError> {
    let mut logs = Vec::with_capacity(config.outputs.len());
    let mut directories = Vec::with_capacity(config.outputs.len());
    let now = ns(config.origin, Instant::now());
    let result = (|| {
        for (index, (base, retention)) in config.outputs.iter().zip(config.retention).enumerate() {
            let target = u32::try_from(index + 1).map_err(|_| StartError::InvalidBudget)?;
            let budget = router.writer_budget(target)?;
            let probe_path = base.clone();
            let probe: Probe = Box::new(move || linux_available_bytes(&probe_path));
            let store = FileStore::new(base, &budget, probe).map_err(StartError::Io)?;
            directories.push(store.directory().to_path_buf());
            logs.push(SegmentLog::new(
                budget,
                config.writer,
                config.channels.to_vec(),
                *retention,
                store,
                now,
            )?);
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok((logs, directories)),
        Err(error) => {
            drop(logs);
            remove_created(&directories);
            Err(error)
        }
    }
}

fn run(
    core: &CaptureCore,
    reads: &mut [ReadState],
    mailboxes: &mut [rtrb::Consumer<Owned>],
    routing: RoutingConfig,
    config: &Config<'_>,
    links: Links<'_>,
) -> Result<(), StartError> {
    let Links {
        commands,
        ready,
        remote,
    } = links;
    let consumers = reads
        .iter_mut()
        .enumerate()
        .map(|(id, read)| core.consumer(id, read))
        .collect();
    let control = ControlOwner::new(config.plan, core.control(), config.control, config.origin)?;
    let mut router = Router::new(config.plan, consumers, routing)?;
    let (mut logs, directories) = open_logs(&router, config)?;
    let completion = core.completion().ok_or(StartError::InvalidBudget)?;
    let producers = core.producers();
    let telemetry = core.telemetry();
    let result = router.scope(|mut dispatcher, outputs| {
        let mut targets: Vec<IoTarget<'_, '_>> = outputs
            .into_iter()
            .zip(logs.iter_mut())
            .enumerate()
            .map(|(index, (consumer, log))| IoTarget {
                id: index as u32 + 1,
                status: &config.flags.outputs[index],
                // Subscribers see what the first output wrote, once per record.
                tap: if index == 0 { remote.tap() } else { None },
                consumer,
                log,
                batch: Vec::with_capacity(config.tuning.batch_records),
                held_since: None,
            })
            .collect();
        let count = targets.len();
        let threads = config.tuning.io_threads.clamp(1, count.max(1));
        let per_thread = count.div_ceil(threads);
        std::thread::scope(|scope| -> Result<Closed, StartError> {
            let stop = StopOnExit(config.flags);
            let mut senders = Vec::with_capacity(threads);
            let (pinned, placed) = mpsc::sync_channel(threads);
            for chunk in targets.chunks_mut(per_thread) {
                let (send, receive) = mpsc::channel::<SyncWork<'_, '_>>();
                senders.push(send);
                let pinned = pinned.clone();
                std::thread::Builder::new()
                    .name("dot-logger-io".into())
                    .stack_size(STACK_BYTES)
                    .spawn_scoped(scope, move || {
                        let placed = pin(config);
                        let ok = placed.is_ok();
                        let _ = pinned.send(placed);
                        if ok {
                            io_main(chunk, &receive, config);
                        }
                    })
                    .map_err(StartError::Io)?;
            }
            drop(pinned);
            for _ in 0..senders.len() {
                placed.recv().map_err(|_| StartError::WorkerFailed)??;
            }
            if let Some(ready) = ready.take() {
                let _ = ready.send(Ok(directories.clone()));
            }
            let send = |target: u32, work| {
                let index = (target as usize - 1) / per_thread;
                // A dead IO thread drops the work, which records Unknown.
                let _ = senders[index].send(work);
            };
            let mut host = Host {
                control: &control,
                targets: (1..=count as u32).collect(),
                ranges: vec![ProducerRange::default(); producers],
                cutoffs: vec![ProducerRange::default(); producers],
                ordinary: None,
                last_sync: Instant::now(),
                tuning: config.tuning,
            };
            let mut events = vec![None; budget::HOST_EVENTS];
            let mut shutdown: Option<ShutdownState> = None;
            let mut sampled: Option<Instant> = None;
            let closed = loop {
                let now = Instant::now();
                #[cfg(test)]
                if config.flags.inject.load(Ordering::Acquire) == 1 {
                    panic!("injected host panic");
                }
                if let (Some(telemetry), Some(latest)) = (telemetry, &config.flags.telemetry) {
                    if sampled.is_none_or(|t| now.saturating_duration_since(t) >= SAMPLE_PERIOD) {
                        // TimeOrder cannot occur at this period; a poisoned
                        // history leaves the last value to age into Stale.
                        if let Ok(sample) =
                            telemetry.sample(ns(config.origin, now), Some(completion))
                        {
                            if let Ok(mut latest) = latest.lock() {
                                *latest = Some(sample);
                            }
                        }
                        sampled = Some(now);
                    }
                }
                remote.serve(telemetry, ns(config.origin, now));
                let turn = dispatcher.turn(now, &mut events)?;
                if let Some(fault) = turn.fault {
                    return Err(StartError::Routing(fault));
                }
                control.poll(now)?;
                if config.flags.worker_failed.load(Ordering::Acquire) {
                    // An IO thread failed: stop admitting records it cannot write.
                    core.control().close();
                }
                host.ordinary_sync(&mut dispatcher, completion, shutdown.is_none(), now, &send)?;
                if shutdown.is_none() {
                    let command = match commands.try_recv() {
                        Ok(Command::Shutdown { deadline, reply }) => Some((deadline, Some(reply))),
                        // The handle vanished without shutdown (leaked or panicked).
                        Err(TryRecvError::Disconnected) => {
                            Some((budget::later(now, config.tuning.drop_deadline), None))
                        }
                        Err(TryRecvError::Empty) => None,
                    };
                    if let Some((deadline, reply)) = command {
                        control.begin_shutdown(
                            ShutdownPlan { sync_files: true },
                            now,
                            deadline,
                            &mut dispatcher,
                        )?;
                        shutdown = Some(ShutdownState {
                            reply,
                            token: None,
                            pending: Vec::new(),
                            incomplete_sent: false,
                        });
                    }
                }
                if let Some(state) = shutdown.as_mut() {
                    if state.token.is_none() {
                        state.token = control.shutdown_sync();
                        state.pending = host.targets.clone();
                    }
                    if let Some(token) = state.token {
                        host.hand(token, &mut state.pending, &mut dispatcher, now, &send);
                    }
                    // Counting the ledger on every draining turn starved
                    // dispatch (T5 ⑫, EV-066); count only when it can matter.
                    let due = !state.incomplete_sent;
                    match control.poll_shutdown_when_due(now, &mut dispatcher, due)? {
                        None | Some(ShutdownPoll::Closing(_)) => {}
                        Some(ShutdownPoll::Incomplete(report)) => {
                            if !state.incomplete_sent {
                                state.incomplete_sent = true;
                                let report = facade_report(report, &control, mailboxes, config);
                                if let Some(reply) = &state.reply {
                                    let _ = reply.send(Progress::Incomplete(report));
                                }
                            }
                        }
                        Some(ShutdownPoll::Closed(report)) => {
                            let unconfirmed = unconfirmed(&report, &control, producers);
                            break (report, unconfirmed, state.reply.take());
                        }
                    }
                }
                if turn.records == 0 && turn.events == 0 {
                    std::thread::sleep(config.tuning.idle_wait);
                }
            };
            // Closed: let IO threads publish their last segments, then report.
            config.flags.finish.store(true, Ordering::Release);
            drop(stop);
            drop(senders);
            Ok(closed)
        })
        .map(|(report, unconfirmed, reply)| {
            // IO threads are joined here; finish errors are already flagged.
            // The listener closes before Closed is reported.
            remote.stop();
            let mut report = facade_report(report, &control, mailboxes, config);
            report.unconfirmed = unconfirmed;
            if let Some(reply) = reply {
                let _ = reply.send(Progress::Closed(report));
            }
        })
    });
    if result.is_err() && ready.is_some() {
        // Startup failed after the namespaces were created: remove them.
        drop(logs);
        remove_created(&directories);
    }
    result
}

/// Telemetry samples are retained at one-second resolution.
const SAMPLE_PERIOD: Duration = Duration::from_secs(1);

type Closed = (ShutdownReport, Vec<ProducerRange>, Option<Sender<Progress>>);

struct ShutdownState {
    reply: Option<Sender<Progress>>,
    token: Option<SyncToken>,
    pending: Vec<u32>,
    incomplete_sent: bool,
}

struct Host<'c, 'o> {
    control: &'c ControlOwner<'o>,
    targets: Vec<u32>,
    ranges: Vec<ProducerRange>,
    cutoffs: Vec<ProducerRange>,
    ordinary: Option<(SyncToken, Vec<u32>)>,
    last_sync: Instant,
    tuning: Tuning,
}

impl<'c, 'o> Host<'c, 'o> {
    /// Hand ready barrier work to IO threads; drop targets whose command ended.
    fn hand(
        &mut self,
        token: SyncToken,
        pending: &mut Vec<u32>,
        dispatcher: &mut crate::routing::Dispatcher<'_, 'o>,
        now: Instant,
        send: &impl Fn(u32, SyncWork<'c, 'o>),
    ) {
        let control = self.control;
        let cutoffs = &mut self.cutoffs;
        pending.retain(|&target| {
            match control.next_sync(token, target, now, dispatcher, cutoffs) {
                Ok(Some(work)) => {
                    send(target, work);
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        });
    }

    /// Periodic ordinary barrier: request, hand out, then retire when finished.
    fn ordinary_sync(
        &mut self,
        dispatcher: &mut crate::routing::Dispatcher<'_, 'o>,
        completion: crate::completion::Completion<'o>,
        running: bool,
        now: Instant,
        send: &impl Fn(u32, SyncWork<'c, 'o>),
    ) -> Result<(), StartError> {
        if let Some((token, mut pending)) = self.ordinary.take() {
            self.hand(token, &mut pending, dispatcher, now, send);
            let snapshot = self.control.sync_snapshot(token, now)?;
            if snapshot.finished && pending.is_empty() {
                let _ = self.control.release_sync(token);
            } else {
                self.ordinary = Some((token, pending));
            }
            return Ok(());
        }
        if !running || now.saturating_duration_since(self.last_sync) < self.tuning.sync_every {
            return Ok(());
        }
        self.last_sync = now;
        let scope = completion.capture_scope(&mut self.ranges)?;
        let deadline = budget::later(now, budget::sync_overdue(self.tuning.sync_every));
        // Busy/Capacity/Closing: try again at the next period.
        if let Ok(token) = self
            .control
            .request_sync(scope, &self.targets, now, deadline)
        {
            self.ordinary = Some((token, self.targets.clone()));
        }
        Ok(())
    }
}

fn unconfirmed(
    report: &ShutdownReport,
    control: &ControlOwner<'_>,
    producers: usize,
) -> Vec<ProducerRange> {
    if report.is_success() {
        return Vec::new();
    }
    let mut ranges = vec![ProducerRange::default(); producers];
    match control.shutdown_ranges(&mut ranges) {
        Ok(n) => {
            ranges.truncate(n);
            ranges
        }
        Err(_) => Vec::new(),
    }
}

fn facade_report(
    core: ShutdownReport,
    _control: &ControlOwner<'_>,
    mailboxes: &[rtrb::Consumer<Owned>],
    config: &Config<'_>,
) -> FacadeReport {
    FacadeReport {
        core,
        writers_outstanding: writers_outstanding(mailboxes),
        worker_failed: config.flags.worker_failed.load(Ordering::Acquire),
        io_failed: config.flags.io_failed.load(Ordering::Acquire),
        unconfirmed: Vec::new(),
    }
}

/// One file target served by an IO thread. The consumer and log are borrowed
/// from the host, so a panicking IO thread returns them for draining.
struct IoTarget<'s, 'o> {
    id: u32,
    status: &'s OutputStatus,
    tap: Option<Tap>,
    consumer: TargetConsumer<'s, 'o>,
    log: &'s mut Log,
    batch: Vec<Delivery<'o>>,
    held_since: Option<Instant>,
}

fn io_main(
    targets: &mut [IoTarget<'_, '_>],
    work: &Receiver<SyncWork<'_, '_>>,
    config: &Config<'_>,
) {
    let (tuning, flags, origin) = (config.tuning, config.flags, config.origin);
    let result = catch_unwind(AssertUnwindSafe(|| {
        io_loop(targets, work, tuning, flags, origin)
    }));
    if result.is_err() {
        // Held guards were released by unwinding and observe Unknown. Close
        // admission through the shared flag path and keep draining as Failed
        // so the coordinator can still reach Closed.
        flags.worker_failed.store(true, Ordering::Release);
        flags.io_failed.store(true, Ordering::Release);
        drain_failed(targets, work, tuning, flags);
    }
}

fn io_loop(
    targets: &mut [IoTarget<'_, '_>],
    work: &Receiver<SyncWork<'_, '_>>,
    tuning: Tuning,
    flags: &Shared,
    origin: Instant,
) {
    let mut syncs: Vec<SyncWork<'_, '_>> = Vec::new();
    loop {
        let now = Instant::now();
        #[cfg(test)]
        if flags.inject.load(Ordering::Acquire) == 2 {
            panic!("injected IO panic");
        }
        let mut busy = false;
        while let Ok(next) = work.try_recv() {
            syncs.push(next);
            busy = true;
        }
        for target in targets.iter_mut() {
            while target.batch.len() < tuning.batch_records {
                match target.consumer.try_next(now) {
                    Some(Ok(delivery)) => {
                        target.held_since.get_or_insert(now);
                        target.batch.push(delivery);
                        busy = true;
                    }
                    // Dispatcher-side drop/expiry, already reported there.
                    Some(Err(_)) => busy = true,
                    None => break,
                }
            }
            let target_id = target.log_target();
            let due = !target.batch.is_empty()
                && (target.batch.len() >= tuning.batch_records
                    || target
                        .held_since
                        .is_some_and(|t| now.saturating_duration_since(t) >= tuning.batch_wait)
                    || syncs.iter().any(|w| w.target_id() == target_id)
                    || flags.stop.load(Ordering::Acquire));
            if due {
                write_batch(target, ns(origin, now), flags);
                busy = true;
            }
        }
        let mut index = 0;
        while index < syncs.len() {
            let target_id = syncs[index].target_id();
            let Some(target) = targets.iter_mut().find(|t| t.log_target() == target_id) else {
                syncs.swap_remove(index).complete(SyncResult::Failed);
                continue;
            };
            if !target.batch.is_empty() {
                index += 1;
                continue;
            }
            match target.log.sync_work(syncs.swap_remove(index)) {
                Ok(()) => target.status.synced(ns(origin, Instant::now())),
                Err(error) => {
                    if let Some(work) = error.work {
                        if matches!(error.error, StorageError::Busy) {
                            syncs.push(work);
                        } else {
                            work.complete(SyncResult::Failed);
                            // A barrier over records this output never wrote
                            // (e.g. a full retention) fails the barrier, not
                            // the file (RV-125).
                            if matches!(error.error, StorageError::Io(_) | StorageError::Poisoned) {
                                flags.io_failed.store(true, Ordering::Release);
                                target.status.fail();
                            }
                        }
                    } else {
                        // The sync itself failed; the log is now poisoned.
                        flags.io_failed.store(true, Ordering::Release);
                        target.status.fail();
                    }
                }
            }
            busy = true;
        }
        // Read `stop` before `finish`: the host stores `finish` before it
        // releases `stop`, so a seen stop implies a seen finish. Reading them
        // the other way round could see neither finish nor, a moment later,
        // stop alone, and leave the last segment active after Closed.
        let stopping = flags.stop.load(Ordering::Acquire);
        if flags.finish.load(Ordering::Acquire) {
            let now = ns(origin, Instant::now());
            for target in targets.iter_mut() {
                if !target.batch.is_empty() {
                    write_batch(target, now, flags);
                }
                if target.log.finish(now).is_err() {
                    flags.io_failed.store(true, Ordering::Release);
                    target.status.fail();
                }
            }
            for work in syncs.drain(..) {
                work.complete(SyncResult::Failed);
            }
            return;
        }
        if stopping {
            return;
        }
        if !busy {
            let held = targets
                .iter()
                .filter(|t| !t.batch.is_empty())
                .filter_map(|t| t.held_since);
            let wait = idle_sleep(tuning.idle_wait, tuning.batch_wait, held, Instant::now());
            std::thread::sleep(wait);
        }
    }
}

impl IoTarget<'_, '_> {
    fn log_target(&self) -> u32 {
        self.id
    }
}

/// Report with a bounded Busy retry; a failed report leaves the guard Unknown.
fn report(delivery: &Delivery<'_>, update: Update) {
    let _ = retry_busy(|| delivery.report(update), REPORT_PATIENCE);
}

/// How long an IO thread keeps retrying a Busy completion report.
const REPORT_PATIENCE: Duration = Duration::from_secs(1);

/// Retry `attempt` while it answers Busy, until `patience` elapsed.
pub(crate) fn retry_busy(
    mut attempt: impl FnMut() -> Result<(), CompletionError>,
    patience: Duration,
) -> Result<(), CompletionError> {
    let started = Instant::now();
    let mut tries = 0_u32;
    loop {
        match attempt() {
            Err(CompletionError::Busy) => {}
            other => return other,
        }
        if started.elapsed() >= patience {
            return Err(CompletionError::Busy);
        }
        // The holder's critical section is short unless it was preempted:
        // yield first, then sleep briefly so a starved holder can run.
        tries = tries.saturating_add(1);
        if tries < 64 {
            std::thread::yield_now();
        } else {
            std::thread::sleep(Duration::from_micros(50));
        }
    }
}

fn failure(error: &RetentionError) -> FailureReason {
    match error {
        RetentionError::Io(_)
        | RetentionError::Poisoned
        | RetentionError::Storage(StorageError::Io(_) | StorageError::Poisoned) => {
            FailureReason::IoFailure
        }
        _ => FailureReason::AdapterFailure,
    }
}

/// Reason the unconfirmed tail of a failed batch is reported with: a prefix
/// flush that hit the file itself turns any failure into an IO failure.
pub(crate) fn tail_reason(write: FailureReason, flush: Option<&RetentionError>) -> FailureReason {
    match flush {
        Some(error) if failure(error) == FailureReason::IoFailure => FailureReason::IoFailure,
        _ => write,
    }
}

fn write_batch(target: &mut IoTarget<'_, '_>, now: u64, flags: &Shared) {
    let refs: Vec<&Delivery<'_>> = target.batch.iter().collect();
    match target.log.write_batch(&refs, now) {
        Ok(()) => {
            if let Some(tap) = target.tap.as_mut() {
                tap.offer(&refs);
            }
        }
        Err(error) => {
            if let Some(tap) = target.tap.as_mut() {
                tap.offer(&refs[..error.written]);
            }
            settle(target.log, &refs, error, flags, target.status);
        }
    }
    drop(refs);
    target.batch.clear();
    target.held_since = None;
}

/// Finish a partially failed batch without ever appending its prefix again.
fn settle(
    log: &mut Log,
    batch: &[&Delivery<'_>],
    error: RetentionBatchError,
    flags: &Shared,
    status: &OutputStatus,
) {
    let RetentionBatchError {
        error,
        appended,
        written,
        reported,
    } = error;
    for delivery in &batch[reported..written] {
        report(delivery, Update::Confirmed(Boundary::Written));
    }
    let mut reason = failure(&error);
    let mut done = written;
    if reason == FailureReason::AdapterFailure && appended > written {
        match log.flush() {
            Ok(()) => {
                for delivery in &batch[written..appended] {
                    report(delivery, Update::Confirmed(Boundary::Written));
                }
                done = appended;
            }
            Err(flush) => reason = tail_reason(reason, Some(&flush)),
        }
    }
    if done < batch.len() {
        status.records_failed(batch.len() - done);
    }
    // Only a fault of the file itself is an IO failure; a full retention or a
    // rejected record fails records without failing the output (RV-125).
    if reason == FailureReason::IoFailure {
        flags.io_failed.store(true, Ordering::Release);
        status.fail();
    }
    for delivery in &batch[done..] {
        report(delivery, Update::Failed(reason));
    }
}

/// After an IO panic: fail every remaining delivery and barrier until stopped.
fn drain_failed(
    targets: &mut [IoTarget<'_, '_>],
    work: &Receiver<SyncWork<'_, '_>>,
    tuning: Tuning,
    flags: &Shared,
) {
    loop {
        let now = Instant::now();
        for target in targets.iter_mut() {
            target.batch.clear();
            while let Some(next) = target.consumer.try_next(now) {
                if let Ok(delivery) = next {
                    report(&delivery, Update::Failed(FailureReason::AdapterFailure));
                    target.status.records_failed(1);
                }
            }
        }
        loop {
            match work.recv_timeout(tuning.idle_wait) {
                Ok(next) => next.complete(SyncResult::Failed),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        if flags.stop.load(Ordering::Acquire) || flags.finish.load(Ordering::Acquire) {
            return;
        }
    }
}
