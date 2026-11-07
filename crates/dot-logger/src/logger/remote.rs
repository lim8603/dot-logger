//! Optional remote connection owned by the default worker (API §18.11).
//!
//! Subscribers receive best-effort copies of what the first file output wrote;
//! they are not completion targets, so the network never delays or fails file
//! recording. Management answers GetStatus only. Without the `remote` feature
//! every type here is an inert stub so the host code stays free of `cfg`.

use crate::routing::Delivery;
use crate::telemetry::Telemetry;

#[cfg(feature = "remote")]
pub(crate) use imp::{Runtime, default_limits};

/// The running remote server, or nothing.
#[derive(Default)]
pub(crate) struct Remote {
    #[cfg(feature = "remote")]
    pub(crate) runtime: Option<Runtime>,
}

/// Copies written records to active subscriptions; held by one IO target.
pub(crate) struct Tap {
    #[cfg(feature = "remote")]
    hub: crate::adapters::remote::RemoteHub,
    #[cfg(feature = "remote")]
    ids: Vec<u64>,
}

impl Remote {
    /// A tap for the first file output, when remote is running.
    pub(crate) fn tap(&self) -> Option<Tap> {
        #[cfg(feature = "remote")]
        {
            self.runtime.as_ref().map(|r| Tap {
                hub: r.hub.clone(),
                ids: vec![0; r.subscriptions],
            })
        }
        #[cfg(not(feature = "remote"))]
        {
            None
        }
    }

    /// Answer queued management commands; one nonwaiting host turn.
    #[cfg_attr(not(feature = "remote"), allow(unused_variables))]
    pub(crate) fn serve(&mut self, telemetry: Option<Telemetry<'_>>, now_ns: u64) {
        #[cfg(feature = "remote")]
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.serve(telemetry, now_ns);
        }
    }

    /// Close subscriptions, stop the listener and join its thread. Idempotent.
    pub(crate) fn stop(&mut self) {
        #[cfg(feature = "remote")]
        if let Some(runtime) = self.runtime.take() {
            runtime.stop();
        }
    }
}

impl Tap {
    /// Offer a copy of each written record to every active subscription.
    /// Subscription overflow is that stream's Gap/Disconnect; nothing here
    /// waits or reports completion.
    #[cfg_attr(not(feature = "remote"), allow(unused_variables))]
    pub(crate) fn offer(&mut self, written: &[&Delivery<'_>]) {
        #[cfg(feature = "remote")]
        {
            let count = self.hub.subscriptions(&mut self.ids);
            for delivery in written {
                for &id in &self.ids[..count] {
                    // Filter mismatch, a closed stream or overflow only affect
                    // that subscriber, which observes Gap or Disconnect.
                    let _ = self.hub.offer(
                        id,
                        delivery.record(),
                        delivery.payload(),
                        delivery.is_raw(),
                        1,
                    );
                }
            }
        }
    }
}

#[cfg(feature = "remote")]
mod imp {
    use std::net::SocketAddr;
    use std::sync::mpsc;
    use std::thread::JoinHandle;
    use std::time::Duration;

    use crate::adapters::remote::{
        CommandInbox, ManagementReply, ManagementRequest, RemoteError, RemoteHub, RemoteLimits,
        RemoteServer, ServerConfig,
    };
    use crate::config::ValidatedConfig;
    use crate::platform::Requirement;
    use crate::query::QueryBudget;
    use crate::telemetry::Telemetry;
    use crate::telemetry::operations::OperationsHost;

    use super::super::StartError;
    use super::super::affinity::pin_current;
    use super::super::budget::{STACK_BYTES, WriterQuota};

    /// Deterministic limits from the writer quotas (API §18.11): four
    /// connections/subscriptions/commands, one record = the largest slot, a
    /// subscription holds every slot's records up to 4 MiB, 32 KiB messages.
    pub(crate) fn default_limits(quotas: &[WriterQuota]) -> RemoteLimits {
        let record = quotas
            .iter()
            .map(|q| q.small_slot_bytes.max(q.raw_slot_bytes))
            .max()
            .unwrap_or(1)
            .max(1);
        // Saturating: these run before the quota check rejects an overflowing
        // quota as an invalid budget (RV-123).
        let records = quotas.iter().fold(0usize, |sum, q| {
            sum.saturating_add(q.small_slots.saturating_add(q.raw_slots))
        });
        let bytes = quotas.iter().fold(0usize, |sum, q| {
            sum.saturating_add(
                q.small_slots
                    .saturating_mul(q.small_slot_bytes)
                    .saturating_add(q.raw_slots.saturating_mul(q.raw_slot_bytes)),
            )
        });
        RemoteLimits {
            connections: 4,
            subscriptions: 4,
            commands: 4,
            message_bytes: 32 * 1024,
            record_bytes: record,
            subscription_records: records.max(1),
            subscription_bytes: bytes.min(4 * 1024 * 1024).max(record),
            max_age: Duration::from_secs(10),
            request_timeout: Duration::from_secs(5),
        }
    }

    /// Longest wait for clients to close after the stop signal.
    const STOP_GRACE: Duration = Duration::from_secs(2);

    /// Hub, command inbox and the thread running the gRPC listener.
    pub(crate) struct Runtime {
        pub(crate) hub: RemoteHub,
        pub(crate) subscriptions: usize,
        pub(crate) address: SocketAddr,
        inbox: Option<CommandInbox>,
        operations: OperationsHost,
        session: u128,
        stop: Option<tokio::sync::oneshot::Sender<()>>,
        thread: Option<JoinHandle<()>>,
    }

    impl Runtime {
        /// Reserve the hub in the plan's external partition, then bind and
        /// serve on a `dot-logger-remote` thread. Nothing runs on error.
        pub(crate) fn start(
            plan: &ValidatedConfig,
            server: ServerConfig,
            limits: RemoteLimits,
            session: u128,
            cpus: Option<(Vec<usize>, Requirement)>,
        ) -> Result<(Self, Option<bool>), StartError> {
            let subscriptions = limits.subscriptions;
            let (hub, inbox) =
                RemoteHub::new(&QueryBudget::new(plan), limits).map_err(StartError::Remote)?;
            let (ready, started) = mpsc::sync_channel(1);
            let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
            let serving = hub.clone();
            let thread = std::thread::Builder::new()
                .name("dot-logger-remote".into())
                .stack_size(STACK_BYTES)
                .spawn(move || {
                    let placed = match &cpus {
                        None => None,
                        Some((cpus, requirement)) => match pin_current(cpus) {
                            Ok(()) => Some(true),
                            Err(error) if *requirement == Requirement::Required => {
                                let _ = ready.send(Err(StartError::Io(error)));
                                return;
                            }
                            Err(_) => Some(false),
                        },
                    };
                    let runtime = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(runtime) => runtime,
                        Err(error) => {
                            let _ = ready.send(Err(StartError::Io(error)));
                            return;
                        }
                    };
                    let bound = runtime.block_on(RemoteServer::bind(server, serving));
                    let server = match bound.and_then(|s| Ok((s.local_addr()?, s))) {
                        Ok((address, server)) => {
                            let _ = ready.send(Ok((address, placed)));
                            server
                        }
                        Err(error) => {
                            let _ = ready.send(Err(StartError::Remote(error)));
                            return;
                        }
                    };
                    // Graceful stop waits for clients to close their connections;
                    // a stalled client must not hold logger shutdown, so after
                    // STOP_GRACE the runtime is dropped, aborting what is left.
                    let (signal, signalled) = tokio::sync::oneshot::channel::<()>();
                    let serving = runtime.spawn(server.serve(async {
                        let _ = signalled.await;
                    }));
                    runtime.block_on(async {
                        let _ = stopped.await;
                        let _ = signal.send(());
                        let _ = tokio::time::timeout(STOP_GRACE, serving).await;
                    });
                    drop(runtime);
                })
                .map_err(StartError::Io)?;
            match started.recv() {
                Ok(Ok((address, placed))) => Ok((
                    Self {
                        hub,
                        subscriptions,
                        address,
                        inbox: Some(inbox),
                        operations: OperationsHost::new(1).map_err(StartError::Remote)?,
                        session,
                        stop: Some(stop),
                        thread: Some(thread),
                    },
                    placed,
                )),
                Ok(Err(error)) => {
                    let _ = thread.join();
                    Err(error)
                }
                Err(_) => {
                    let _ = thread.join();
                    Err(StartError::WorkerFailed)
                }
            }
        }

        /// Answer every queued command. With telemetry the operations backend
        /// serves GetStatus (`dot_logger.status.v1`), ApplyConfig (Unsupported:
        /// no live configuration path) and Compare (no snapshots: Gone). Without
        /// it GetStatus carries an empty report, meaning no metrics backend.
        /// Every other request is Unsupported.
        pub(crate) fn serve(&mut self, telemetry: Option<Telemetry<'_>>, now_ns: u64) {
            let Some(inbox) = self.inbox.as_mut() else {
                return;
            };
            while let Some(command) = inbox.try_next() {
                match telemetry {
                    Some(telemetry) => {
                        self.operations
                            .handle_command(command, telemetry, None, now_ns);
                    }
                    None => {
                        let reply = match command.request() {
                            ManagementRequest::GetStatus => Ok(ManagementReply::Status {
                                session: self.session,
                                revision: 1,
                                report: Vec::new(),
                            }),
                            _ => Err(RemoteError::Unsupported),
                        };
                        command.respond(reply);
                    }
                }
            }
        }

        /// Pending commands are answered Gone, streams end, the listener stops
        /// and its thread is joined before this returns.
        pub(crate) fn stop(self) {
            drop(self);
        }
    }

    impl Drop for Runtime {
        fn drop(&mut self) {
            drop(self.inbox.take());
            self.hub.close();
            if let Some(stop) = self.stop.take() {
                let _ = stop.send(());
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
