use crate::{
    Options,
    bridge::Bridge,
    catalog::{Catalog, Channel, MAX_CATALOG_BYTES},
    output::{DATA_HEADER, Frame, Notices, Outbox, status},
};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    net::IpAddr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
    time::{MissedTickBehavior, interval, timeout},
};
use tokio_tungstenite::{
    WebSocketStream, accept_hdr_async_with_config,
    tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
        http::{HeaderValue, StatusCode, Uri},
        protocol::WebSocketConfig,
    },
};

const PROTOCOL: &str = "foxglove.websocket.v1";
const COMMAND_BYTES: usize = 16 * 1024;
const MAX_SUBSCRIPTIONS: usize = 128;
const WRITE_DEADLINE: Duration = Duration::from_secs(2);
const HANDSHAKE_DEADLINE: Duration = Duration::from_secs(5);
pub(crate) const STATUS_INTERVAL: Duration = Duration::from_millis(100);
const GREETING: &str = r#"{"op":"serverInfo","name":"dot-logger-viewer","capabilities":[]}"#;

pub(crate) type Socket = WebSocketStream<TcpStream>;
pub(crate) type Writer = SplitSink<Socket, Message>;

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum Command {
    Subscribe {
        subscriptions: Vec<Subscription>,
    },
    Unsubscribe {
        #[serde(rename = "subscriptionIds")]
        subscription_ids: Vec<u32>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Subscription {
    id: u32,
    channel_id: u32,
}

pub(crate) enum Control {
    Status(Message),
    Flush,
    Close,
}

pub(crate) async fn session(
    tcp: TcpStream,
    bridge: Arc<Bridge>,
    catalog: Arc<Catalog>,
    options: Arc<Options>,
) {
    let Some(socket) = open(tcp, &catalog, &options, GREETING).await else {
        return;
    };
    let (writer, reader) = socket.split();
    let (outbox, data, notices) = Outbox::new(options.queue_records, options.queue_bytes);
    let (commands, control) = mpsc::channel(8);
    tokio::select! {
        _ = read(reader, &catalog, &commands, |channel, id| {
            let bridge = &bridge;
            let outbox = &outbox;
            async move {
                bridge.subscribe(&channel, id, outbox).await.ok_or("upstream channel limit reached")
            }
        }) => {},
        _ = write(writer, data, notices, control, &catalog) => {},
    }
}

pub(crate) async fn open(
    tcp: TcpStream,
    catalog: &Catalog,
    options: &Options,
    server_info: &'static str,
) -> Option<Socket> {
    if tcp.set_nodelay(true).is_err() {
        return None;
    }
    let config = WebSocketConfig::default()
        .read_buffer_size(4096)
        .max_message_size(Some(COMMAND_BYTES))
        .max_frame_size(Some(COMMAND_BYTES))
        .write_buffer_size(0)
        .max_write_buffer_size(MAX_CATALOG_BYTES.max(options.record_bytes + DATA_HEADER) + 32);
    #[allow(clippy::result_large_err)] // The callback uses tungstenite's HTTP response type.
    let handshake = accept_hdr_async_with_config(
        tcp,
        |request: &Request, response: Response| {
            negotiate(request, response, options.allow_origin.as_deref())
        },
        Some(config),
    );
    let mut socket = timeout(HANDSHAKE_DEADLINE, handshake).await.ok()?.ok()?;
    let greeting = async {
        socket.send(Message::text(server_info)).await?;
        socket.send(Message::text(catalog.advertisement())).await
    };
    timeout(WRITE_DEADLINE, greeting).await.ok()?.ok()?;
    Some(socket)
}

#[allow(clippy::result_large_err)] // The handshake callback returns tungstenite's HTTP response.
fn negotiate(
    request: &Request,
    mut response: Response,
    explicit_origin: Option<&str>,
) -> Result<Response, ErrorResponse> {
    let protocols = request.headers().get_all("sec-websocket-protocol");
    if !protocols.iter().any(|header| {
        header
            .to_str()
            .is_ok_and(|value| value.split(',').any(|part| part.trim() == PROTOCOL))
    }) {
        return Err(reject(StatusCode::BAD_REQUEST));
    }
    let origins = request.headers().get_all("origin");
    let mut origins = origins.iter();
    if let Some(origin) = origins.next() {
        if origins.next().is_some()
            || !origin
                .to_str()
                .is_ok_and(|value| Some(value) == explicit_origin || local_origin(value))
        {
            return Err(reject(StatusCode::FORBIDDEN));
        }
    }
    response
        .headers_mut()
        .insert("sec-websocket-protocol", HeaderValue::from_static(PROTOCOL));
    Ok(response)
}

pub(crate) fn local_origin(origin: &str) -> bool {
    let Ok(uri) = origin.parse::<Uri>() else {
        return false;
    };
    matches!(uri.scheme_str(), Some("http" | "https"))
        && uri
            .authority()
            .is_some_and(|authority| !authority.as_str().contains('@'))
        && uri
            .path_and_query()
            .is_some_and(|path| path.as_str() == "/")
        && uri.host().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .trim_matches(['[', ']'])
                    .parse::<IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        })
}

fn reject(status: StatusCode) -> ErrorResponse {
    let mut response = ErrorResponse::new(None);
    *response.status_mut() = status;
    response
}

pub(crate) async fn read<L, F: Future<Output = Result<L, &'static str>>>(
    mut reader: SplitStream<Socket>,
    catalog: &Catalog,
    controls: &mpsc::Sender<Control>,
    mut subscribe: impl FnMut(Channel, u32) -> F,
) {
    let mut leases = HashMap::new();
    while let Some(Ok(message)) = reader.next().await {
        let control = match message {
            Message::Text(text) => handle(&text, catalog, &mut leases, &mut subscribe)
                .await
                .map(|reason| Control::Status(status("request-rejected", 1, reason))),
            Message::Ping(_) => Some(Control::Flush),
            Message::Close(_) => {
                leases.clear();
                if controls.try_send(Control::Close).is_ok() {
                    // The writer flushes tungstenite's queued close response.
                    std::future::pending::<()>().await;
                }
                break;
            }
            Message::Pong(_) => None,
            _ => Some(Control::Status(status(
                "request-rejected",
                1,
                "only read-only subscribe and unsubscribe requests are supported",
            ))),
        };
        if control.is_some_and(|item| controls.try_send(item).is_err()) {
            break;
        }
    }
}

async fn handle<L, F: Future<Output = Result<L, &'static str>>>(
    text: &str,
    catalog: &Catalog,
    leases: &mut HashMap<u32, L>,
    subscribe: &mut impl FnMut(Channel, u32) -> F,
) -> Option<&'static str> {
    match serde_json::from_str(text) {
        Ok(Command::Subscribe { subscriptions }) if subscriptions.len() <= MAX_SUBSCRIPTIONS => {
            let mut error = None;
            for subscription in subscriptions {
                if leases.len() == MAX_SUBSCRIPTIONS || leases.contains_key(&subscription.id) {
                    error = Some("duplicate subscription ID or socket subscription limit reached");
                } else if let Some(channel) = catalog.channel(subscription.channel_id) {
                    if channel.retired() {
                        error =
                            Some("channel schema changed; update the local catalog and restart");
                    } else {
                        match subscribe(channel.clone(), subscription.id).await {
                            Ok(lease) => {
                                leases.insert(subscription.id, lease);
                            }
                            Err(reason) => error = Some(reason),
                        }
                    }
                } else {
                    error = Some("unknown advertised channel ID");
                }
            }
            error
        }
        Ok(Command::Unsubscribe { subscription_ids })
            if subscription_ids.len() <= MAX_SUBSCRIPTIONS =>
        {
            for subscription in subscription_ids {
                leases.remove(&subscription);
            }
            None
        }
        _ => Some("invalid or unsupported read-only WebSocket request"),
    }
}

async fn write(
    mut writer: Writer,
    mut data: mpsc::Receiver<Frame>,
    notices: watch::Receiver<Notices>,
    mut controls: mpsc::Receiver<Control>,
    catalog: &Catalog,
) {
    let mut tick = interval(STATUS_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut previous = Notices::default();
    let mut retired = HashSet::new();
    loop {
        tokio::select! {
            biased;
            control = controls.recv() => {
                if !apply_control(&mut writer, control).await { break; }
            }
            _ = tick.tick() => {
                let messages = notices.borrow().statuses(&mut previous);
                for message in messages {
                    if !send(&mut writer, message).await { return; }
                }
                if !send_retired(&mut writer, catalog, &mut retired).await {
                    break;
                }
            }
            frame = data.recv() => {
                let Some(frame) = frame else { break; };
                if let Some((message, _byte_charge)) = frame.into_send() {
                    if !send(&mut writer, message).await { break; }
                }
            }
        }
    }
}

pub(crate) async fn send_retired(
    writer: &mut Writer,
    catalog: &Catalog,
    retired: &mut HashSet<u32>,
) -> bool {
    let ids: Vec<_> = catalog
        .retired_ids()
        .filter(|id| retired.insert(*id))
        .collect();
    ids.is_empty()
        || send(
            writer,
            Message::text(serde_json::json!({"op":"unadvertise","channelIds":ids}).to_string()),
        )
        .await
}

pub(crate) async fn apply_control(writer: &mut Writer, control: Option<Control>) -> bool {
    match control {
        Some(Control::Status(message)) => send(writer, message).await,
        Some(Control::Flush) => timeout(WRITE_DEADLINE, writer.flush())
            .await
            .is_ok_and(|result| result.is_ok()),
        Some(Control::Close) => {
            let _ = timeout(WRITE_DEADLINE, writer.flush()).await;
            false
        }
        None => false,
    }
}

pub(crate) async fn send(writer: &mut Writer, message: Message) -> bool {
    timeout(WRITE_DEADLINE, writer.send(message))
        .await
        .is_ok_and(|result| result.is_ok())
}
