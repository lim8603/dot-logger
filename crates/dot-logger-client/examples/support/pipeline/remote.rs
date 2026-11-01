//! Same-receipt gRPC transfer with explicit synchronous bounded-queue handoff.
use super::*;
use dot_logger::adapters::remote::*;
use dot_logger_client::{
    Cancellation, ClientError, ReceiverConfig, RemoteClient, RemoteConfig, RemoteEvent,
    RemoteSubscription,
};
use std::sync::mpsc::sync_channel;

pub(super) struct Remote {
    hub: RemoteHub,
    _inbox: CommandInbox,
    runtime: tokio::runtime::Runtime,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    server: Option<tokio::task::JoinHandle<Result<(), RemoteError>>>,
    client: Option<RemoteClient>,
    stream: Option<RemoteSubscription>,
    id: u64,
}
impl Remote {
    pub(super) fn new(budget: &QueryBudget) -> Result<Self, Error> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let limits = RemoteLimits {
            connections: 1,
            subscriptions: 1,
            commands: 1,
            message_bytes: 4096,
            record_bytes: 512,
            subscription_records: 1,
            subscription_bytes: 512,
            max_age: Duration::from_secs(5),
            request_timeout: Duration::from_secs(2),
        };
        let (hub, inbox) = RemoteHub::new(budget, limits)?;
        let mut remote = Self {
            hub,
            _inbox: inbox,
            runtime,
            stop: None,
            server: None,
            client: None,
            stream: None,
            id: 0,
        };
        let server = remote.runtime.block_on(RemoteServer::bind(
            ServerConfig {
                enabled: true,
                ..Default::default()
            },
            remote.hub.clone(),
        ))?;
        let address = server.local_addr()?;
        let (stop, stopped) = tokio::sync::oneshot::channel();
        remote.stop = Some(stop);
        remote.server = Some(remote.runtime.spawn(server.serve(async {
            let _ = stopped.await;
        })));
        let cancel = Cancellation::new();
        let (client, stream) = remote.runtime.block_on(async {
            let mut client =
                RemoteClient::connect(RemoteConfig::new(format!("http://{address}")), &cancel)
                    .await?;
            let stream = client
                .subscribe(
                    budget,
                    SubscriptionFilter {
                        channel: None,
                        minimum_level: Level::Trace,
                        originals: true,
                        overflow: OverflowPolicy::Disconnect,
                    },
                    ReceiverConfig {
                        records: 1,
                        record_bytes: 512,
                        ttl: Duration::from_secs(5),
                    },
                    &cancel,
                )
                .await?;
            Ok::<_, Error>((client, stream))
        })?;
        remote.client = Some(client);
        remote.stream = Some(stream);
        let mut ids = [0; 1];
        if remote.hub.subscriptions(&mut ids) != 1 {
            return Err(invalid("subscription missing"));
        }
        remote.id = ids[0];
        Ok(remote)
    }
    pub(super) fn receive(&mut self, original: Delivery<'_>, full: bool) -> Result<bool, Error> {
        let expected = (original.record(), original.payload().to_vec());
        let mut delivery = self
            .hub
            .offer_delivery(self.id, original, true, 1)
            .map_err(|(e, _)| e)?;
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| invalid("subscription closed"))?;
        let cancel = Cancellation::new();
        let result = self.runtime.block_on(async {
            let received = tokio::time::timeout(Duration::from_secs(3), stream.next(&cancel))
                .await??
                .ok_or_else(|| invalid("subscription ended"))?;
            let RemoteEvent::Record(record) = received else {
                return Err(invalid("unexpected gap"));
            };
            if delivery.poll()? != AckState::Pending {
                return Err(invalid("ACK before consumer handoff"));
            }
            let (queue, consumer) = sync_channel(1);
            if full {
                queue.try_send((expected.0, Vec::new()))?;
            }
            let mut identical = false;
            let handoff = stream
                .handoff(
                    record,
                    |record, payload| {
                        identical = *record == expected.0 && payload == expected.1;
                        identical && queue.try_send((*record, payload.to_vec())).is_ok()
                    },
                    &cancel,
                )
                .await;
            if !identical {
                return Err(invalid("remote metadata/payload changed"));
            }
            match handoff {
                Ok(()) => {
                    let received = consumer.try_recv()?;
                    if received != expected {
                        return Err(invalid("consumer queue bytes changed"));
                    }
                    Ok(true)
                }
                Err(ClientError::ConsumerRejected) if full => Ok(false),
                Err(e) => Err(e.into()),
            }
        })?;
        if !result {
            self.stream = None;
            if let Some(client) = &mut self.client {
                client.disconnect();
            }
        }
        self.runtime.block_on(async {
            let end = Instant::now() + Duration::from_secs(3);
            loop {
                if delivery.poll()? != AckState::Pending {
                    break;
                }
                if Instant::now() >= end {
                    return Err(invalid("ACK settlement deadline"));
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Ok::<_, Error>(())
        })?;
        Ok(result)
    }
    pub(super) fn query(
        &self,
        writer: &mut db::Writer,
        query: db::NumericQuery,
        cancel: &AtomicBool,
    ) -> Result<db::NumericPage, db::QueryError> {
        self.runtime.block_on(writer.query_numeric(query, cancel))
    }
    pub(super) fn close(&mut self) -> Result<(), Error> {
        self.stream = None;
        if let Some(mut client) = self.client.take() {
            client.disconnect();
        }
        self.hub.close();
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(mut server) = self.server.take() {
            self.runtime.block_on(async {
                match tokio::time::timeout(Duration::from_secs(3), &mut server).await {
                    Ok(result) => result??,
                    Err(_) => {
                        server.abort();
                        let _ = server.await;
                        return Err(invalid("server close deadline"));
                    }
                }
                Ok::<_, Error>(())
            })?;
        }
        Ok(())
    }
}
impl Drop for Remote {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
