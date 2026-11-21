//! Read-only Foxglove WebSocket bridge for dot-logger.

mod bridge;
mod catalog;
mod dashboard;
#[cfg(test)]
mod dashboard_tests;
mod output;
mod replay;
#[cfg(test)]
mod tests;
mod websocket;

use bridge::Bridge;
use catalog::{Catalog, MAX_CATALOG_BYTES, read_file};
use clap::{Args, Parser};
use dot_logger_client::RemoteConfig;
use replay::{Playback, Rate};
use std::{error::Error, io, net::SocketAddr, path::PathBuf, process::ExitCode, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinSet,
};

const MIB: usize = 1024 * 1024;

#[derive(Parser)]
#[command(about = "Bridge live or recorded dot-logger data to a local Foxglove WebSocket")]
struct Command {
    /// Robot's authenticated HTTPS endpoint or explicit loopback HTTP endpoint.
    #[arg(long)]
    upstream: String,
    /// JSON file with local channel and schema mappings.
    #[arg(long)]
    catalog: PathBuf,
    /// Local WebSocket address. Port 0 requests an available port.
    #[arg(long, default_value = "127.0.0.1:8765", value_parser = loopback)]
    listen: SocketAddr,
    /// Also serve the read-only operations dashboard on this loopback address.
    /// Port 0 requests an available port.
    #[arg(long, value_parser = loopback)]
    dashboard: Option<SocketAddr>,
    /// CA PEM for an HTTPS upstream.
    #[arg(long)]
    ca: Option<PathBuf>,
    /// Override the upstream TLS certificate name.
    #[arg(long)]
    tls_domain: Option<String>,
    /// Start recorded playback at this session-monotonic nanosecond position.
    #[arg(long, requires = "replay_to_ns")]
    replay_from_ns: Option<u64>,
    /// Exclusive end of the recorded playback range in session-monotonic nanoseconds.
    #[arg(long, requires = "replay_from_ns")]
    replay_to_ns: Option<u64>,
    /// Initial positive rational playback rate, for example 2/1. Defaults to 1/1.
    #[arg(long, requires = "replay_from_ns")]
    replay_speed: Option<Rate>,
    #[command(flatten)]
    options: Options,
}

#[derive(Args, Clone)]
struct Options {
    /// Maximum simultaneous WebSocket clients, including handshakes.
    #[arg(long, default_value_t = 4)]
    clients: usize,
    /// Maximum distinct active upstream channels.
    #[arg(long, default_value_t = 4)]
    feeds: usize,
    /// Maximum in-flight records requested from the upstream per channel.
    #[arg(long, default_value_t = 4)]
    receiver_records: usize,
    /// Maximum payload size received from the upstream.
    #[arg(long, default_value_t = 64 * 1024)]
    record_bytes: usize,
    /// Upstream subscription lease requested in milliseconds.
    #[arg(long, default_value_t = 10_000)]
    receiver_age_ms: u64,
    /// Maximum queued data frames per WebSocket, plus one active write.
    #[arg(long, default_value_t = 32)]
    queue_records: usize,
    /// Per-WebSocket byte budget covering queued data frames and the active write.
    #[arg(long, default_value_t = 2 * MIB)]
    queue_bytes: usize,
    /// Maximum bytes requested per remote replay ReadBlob RPC.
    #[arg(long, default_value_t = 16 * 1024)]
    replay_blob_bytes: usize,
    /// Also allow this exact WebSocket Origin, for a trusted frontend.
    #[arg(long)]
    allow_origin: Option<String>,
}

#[cfg(test)]
impl Default for Options {
    fn default() -> Self {
        Command::parse_from([
            "viewer",
            "--upstream",
            "http://127.0.0.1",
            "--catalog",
            "unused",
        ])
        .options
    }
}

impl Options {
    fn validate(&self) -> io::Result<()> {
        if !(1..=16).contains(&self.clients)
            || !(1..=128).contains(&self.feeds)
            || !(1..=128).contains(&self.receiver_records)
            || !(1..=16 * MIB).contains(&self.record_bytes)
            || !(1..=3_600_000).contains(&self.receiver_age_ms)
            || !(1..=1024).contains(&self.queue_records)
            || !(1..=16 * MIB - 1024).contains(&self.replay_blob_bytes)
            || !(self.record_bytes + output::DATA_HEADER..=64 * MIB).contains(&self.queue_bytes)
            || self.clients.saturating_mul(self.queue_bytes) > 256 * MIB
            || self
                .feeds
                .saturating_mul(self.receiver_records)
                .saturating_mul(self.record_bytes + 1024)
                > 256 * MIB
            || self.allow_origin.as_ref().is_some_and(|origin| {
                origin.is_empty()
                    || origin.len() > 512
                    || !origin.bytes().all(|b| b.is_ascii_graphic())
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid viewer limits or Origin",
            ));
        }
        Ok(())
    }
}

fn loopback(value: &str) -> Result<SocketAddr, &'static str> {
    value
        .parse::<SocketAddr>()
        .ok()
        .filter(|addr| addr.ip().is_loopback())
        .ok_or("expected a literal loopback socket address")
}

async fn serve(
    listener: TcpListener,
    upstream: RemoteConfig,
    catalog: Catalog,
    options: Options,
    stop: impl Future<Output = ()>,
) -> io::Result<()> {
    options.validate()?;
    let bridge = Arc::new(Bridge::new(upstream, &options).map_err(io::Error::other)?);
    let catalog = Arc::new(catalog);
    let options = Arc::new(options);
    let result = listen(&listener, options.clients, stop, |socket| {
        websocket::session(socket, bridge.clone(), catalog.clone(), options.clone())
    })
    .await;
    bridge.shutdown().await;
    result
}

async fn serve_replay(
    listener: TcpListener,
    mut upstream: RemoteConfig,
    catalog: Catalog,
    options: Options,
    playback: Playback,
    stop: impl Future<Output = ()>,
) -> io::Result<()> {
    options.validate()?;
    upstream.message_bytes = upstream.message_bytes.max(options.replay_blob_bytes + 1024);
    let catalog = Arc::new(catalog);
    let options = Arc::new(options);
    listen(&listener, options.clients, stop, |socket| {
        replay::session(
            socket,
            upstream.clone(),
            catalog.clone(),
            options.clone(),
            playback.clone(),
        )
    })
    .await
}

async fn listen<Fut: Future<Output = ()> + Send + 'static>(
    listener: &TcpListener,
    clients: usize,
    stop: impl Future<Output = ()>,
    session: impl Fn(TcpStream) -> Fut,
) -> io::Result<()> {
    if !listener.local_addr()?.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "viewer listener must be loopback",
        ));
    }
    let mut sockets = JoinSet::new();
    let mut stop = std::pin::pin!(stop);
    let result = loop {
        tokio::select! {
            biased;
            _ = &mut stop => break Ok(()),
            _ = sockets.join_next(), if !sockets.is_empty() => {},
            accepted = listener.accept(), if sockets.len() < clients => {
                match accepted {
                    Ok((socket, _)) => {
                        sockets.spawn(session(socket));
                    }
                    Err(error) => break Err(error),
                }
            }
        }
    };
    sockets.shutdown().await;
    result
}

async fn run(command: Command) -> Result<(), Box<dyn Error>> {
    command.options.validate()?;
    let catalog = Catalog::parse(&read_file(&command.catalog, MAX_CATALOG_BYTES)?)?;
    let mut upstream = RemoteConfig::new(command.upstream);
    upstream.token = match std::env::var("DOT_LOGGER_TOKEN") {
        Ok(token) => Some(token),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err("DOT_LOGGER_TOKEN must be valid text".into());
        }
    };
    upstream.ca_certificate = command
        .ca
        .as_ref()
        .map(|path| read_file(path, 65_536))
        .transpose()?;
    upstream.tls_domain = command.tls_domain;
    let playback = command
        .replay_from_ns
        .zip(command.replay_to_ns)
        .map(|(from, to)| Playback::new(from, to, command.replay_speed.unwrap_or(Rate::REALTIME)))
        .transpose()?;
    let listener = TcpListener::bind(command.listen).await?;
    let stop = termination()?;
    println!("ws://{}", listener.local_addr()?);
    // The dashboard polls with its own connection and stops with the viewer.
    let dashboard = match command.dashboard {
        Some(address) => {
            let listener = TcpListener::bind(address).await?;
            println!("http://{}/", listener.local_addr()?);
            Some(tokio::spawn(dashboard::serve(
                listener,
                upstream.clone(),
                command.options.allow_origin.clone(),
                std::future::pending(),
            )))
        }
        None => None,
    };
    if let Some(playback) = playback {
        playback.start_terminal()?;
        eprintln!("Replay controls: seek MONO_NS or speed N/D");
        serve_replay(listener, upstream, catalog, command.options, playback, stop).await?;
    } else {
        serve(listener, upstream, catalog, command.options, stop).await?;
    }
    if let Some(dashboard) = dashboard {
        dashboard.abort();
    }
    Ok(())
}

#[cfg(unix)]
fn termination() -> io::Result<impl Future<Output = ()>> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        tokio::select! {
            _ = interrupt.recv() => {},
            _ = terminate.recv() => {},
        }
    })
}

#[cfg(not(unix))]
fn termination() -> io::Result<impl Future<Output = ()>> {
    Ok(async {
        let _ = tokio::signal::ctrl_c().await;
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run(Command::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dot-logger-viewer: {error}");
            ExitCode::FAILURE
        }
    }
}
