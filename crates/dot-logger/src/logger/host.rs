//! Default worker: one management host thread plus file IO threads
//! (ADR-012 §4–5). Every core borrow lives on these threads' stacks; the RT
//! side only holds `Writer`s, so the existing scoped contracts are unchanged.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
use crate::routing::{Delivery, Router, RoutingConfig, TargetConsumer};
use crate::storage::{
    FileStore, RetentionBatchError, RetentionConfig, RetentionError, SegmentLog, StorageChannel,
    StorageError, WriterConfig,
};

use super::budget::{STACK_BYTES, Tuning};
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

/// Failure flags shared by the host, IO threads and the handle.
#[derive(Default)]
pub(crate) struct Flags {
    pub(crate) worker_failed: AtomicBool,
    pub(crate) io_failed: AtomicBool,
    stop: AtomicBool,
    finish: AtomicBool,
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
    pub(crate) flags: Arc<Flags>,
}

/// Start the host and wait until it built every component or failed.
pub(crate) fn spawn(
    plan: HostPlan,
    commands: Receiver<Command>,
) -> Result<(Vec<PathBuf>, JoinHandle<()>), StartError> {
    let (ready, started) = mpsc::sync_channel(1);
    let handle = std::thread::Builder::new()
        .name("dot-logger-host".into())
        .stack_size(STACK_BYTES)
        .spawn(move || host_main(plan, commands, ready))
        .map_err(StartError::Io)?;
    match started.recv() {
        Ok(Ok(directories)) => Ok((directories, handle)),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(StartError::WorkerFailed),
    }
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
struct StopOnExit<'a>(&'a Flags);
impl Drop for StopOnExit<'_> {
    fn drop(&mut self) {
        self.0.stop.store(true, Ordering::Release);
    }
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
        flags,
    } = plan;
    let mut ready = Some(ready);
    {
        let close = CloseOnExit(&core);
        let config = Config {
            plan: &plan,
            control,
            writer,
            retention: &retention,
            channels: &channels,
            outputs: &outputs,
            tuning,
            origin,
            flags: &flags,
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            run(
                &core,
                &mut reads,
                &mut mailboxes,
                routing,
                &config,
                &commands,
                &mut ready,
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
    // Reaper: an outstanding Writer still points into `core` and its rings.
    // Keep this thread as their last owner until every writer state returned,
    // so no RT thread ever performs the final release (ADR-012 §3).
    let mut wait = tuning.idle_wait.max(Duration::from_millis(1));
    while mailboxes.iter().any(rtrb::Consumer::is_empty) {
        std::thread::sleep(wait);
        wait = (wait * 2).min(Duration::from_millis(100));
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
    flags: &'a Flags,
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
    commands: &Receiver<Command>,
    ready: &mut Option<SyncSender<Result<Vec<PathBuf>, StartError>>>,
) -> Result<(), StartError> {
    let consumers = reads
        .iter_mut()
        .enumerate()
        .map(|(id, read)| core.consumer(id, read))
        .collect();
    let control = ControlOwner::new(config.plan, core.control(), config.control, config.origin)?;
    let mut router = Router::new(config.plan, consumers, routing)?;
    let (mut logs, directories) = open_logs(&router, config)?;
    if let Some(ready) = ready.take() {
        let _ = ready.send(Ok(directories));
    }
    let completion = core.completion().ok_or(StartError::InvalidBudget)?;
    let producers = core.producers();
    router.scope(|mut dispatcher, outputs| {
        let mut targets: Vec<IoTarget<'_, '_>> = outputs
            .into_iter()
            .zip(logs.iter_mut())
            .enumerate()
            .map(|(index, (consumer, log))| IoTarget {
                id: index as u32 + 1,
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
            for chunk in targets.chunks_mut(per_thread) {
                let (send, receive) = mpsc::channel::<SyncWork<'_, '_>>();
                senders.push(send);
                let tuning = config.tuning;
                let flags = config.flags;
                let origin = config.origin;
                std::thread::Builder::new()
                    .name("dot-logger-io".into())
                    .stack_size(STACK_BYTES)
                    .spawn_scoped(scope, move || {
                        io_main(chunk, &receive, tuning, flags, origin)
                    })
                    .map_err(StartError::Io)?;
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
            let mut events = vec![None; 256];
            let mut shutdown: Option<ShutdownState> = None;
            let closed = loop {
                let now = Instant::now();
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
                            Some((now + config.tuning.drop_deadline, None))
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
                    match control.poll_shutdown(now, &mut dispatcher)? {
                        ShutdownPoll::Closing(_) => {}
                        ShutdownPoll::Incomplete(report) => {
                            if !state.incomplete_sent {
                                state.incomplete_sent = true;
                                let report = facade_report(report, &control, mailboxes, config);
                                if let Some(reply) = &state.reply {
                                    let _ = reply.send(Progress::Incomplete(report));
                                }
                            }
                        }
                        ShutdownPoll::Closed(report) => {
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
            let mut report = facade_report(report, &control, mailboxes, config);
            report.unconfirmed = unconfirmed;
            if let Some(reply) = reply {
                let _ = reply.send(Progress::Closed(report));
            }
        })
    })
}

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
        let deadline = now + self.tuning.sync_every * 4 + Duration::from_secs(1);
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
        writers_outstanding: mailboxes.iter().filter(|m| m.is_empty()).count(),
        worker_failed: config.flags.worker_failed.load(Ordering::Acquire),
        io_failed: config.flags.io_failed.load(Ordering::Acquire),
        unconfirmed: Vec::new(),
    }
}

/// One file target served by an IO thread. The consumer and log are borrowed
/// from the host, so a panicking IO thread returns them for draining.
struct IoTarget<'s, 'o> {
    id: u32,
    consumer: TargetConsumer<'s, 'o>,
    log: &'s mut Log,
    batch: Vec<Delivery<'o>>,
    held_since: Option<Instant>,
}

fn io_main(
    targets: &mut [IoTarget<'_, '_>],
    work: &Receiver<SyncWork<'_, '_>>,
    tuning: Tuning,
    flags: &Flags,
    origin: Instant,
) {
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
    flags: &Flags,
    origin: Instant,
) {
    let mut syncs: Vec<SyncWork<'_, '_>> = Vec::new();
    loop {
        let now = Instant::now();
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
                Ok(()) => {}
                Err(error) => {
                    if let Some(work) = error.work {
                        if matches!(error.error, StorageError::Busy) {
                            syncs.push(work);
                        } else {
                            work.complete(SyncResult::Failed);
                            flags.io_failed.store(true, Ordering::Release);
                        }
                    } else {
                        flags.io_failed.store(true, Ordering::Release);
                    }
                }
            }
            busy = true;
        }
        if flags.finish.load(Ordering::Acquire) {
            let now = ns(origin, Instant::now());
            for target in targets.iter_mut() {
                if !target.batch.is_empty() {
                    write_batch(target, now, flags);
                }
                if target.log.finish(now).is_err() {
                    flags.io_failed.store(true, Ordering::Release);
                }
            }
            for work in syncs.drain(..) {
                work.complete(SyncResult::Failed);
            }
            return;
        }
        if flags.stop.load(Ordering::Acquire) {
            return;
        }
        if !busy {
            std::thread::sleep(tuning.idle_wait.min(tuning.batch_wait));
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
    for _ in 0..1024 {
        match delivery.report(update) {
            Err(CompletionError::Busy) => std::thread::yield_now(),
            _ => return,
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

fn write_batch(target: &mut IoTarget<'_, '_>, now: u64, flags: &Flags) {
    let refs: Vec<&Delivery<'_>> = target.batch.iter().collect();
    if let Err(error) = target.log.write_batch(&refs, now) {
        settle(target.log, &refs, error, flags);
    }
    drop(refs);
    target.batch.clear();
    target.held_since = None;
}

/// Finish a partially failed batch without ever appending its prefix again.
fn settle(log: &mut Log, batch: &[&Delivery<'_>], error: RetentionBatchError, flags: &Flags) {
    let RetentionBatchError {
        error,
        appended,
        written,
        reported,
    } = error;
    for delivery in &batch[reported..written] {
        report(delivery, Update::Confirmed(Boundary::Written));
    }
    let reason = failure(&error);
    let mut done = written;
    if reason == FailureReason::AdapterFailure && appended > written && log.flush().is_ok() {
        for delivery in &batch[written..appended] {
            report(delivery, Update::Confirmed(Boundary::Written));
        }
        done = appended;
    }
    if done < batch.len() {
        flags.io_failed.store(true, Ordering::Release);
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
    flags: &Flags,
) {
    loop {
        let now = Instant::now();
        for target in targets.iter_mut() {
            target.batch.clear();
            while let Some(next) = target.consumer.try_next(now) {
                if let Ok(delivery) = next {
                    report(&delivery, Update::Failed(FailureReason::AdapterFailure));
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
