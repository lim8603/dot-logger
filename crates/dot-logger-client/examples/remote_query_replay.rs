//! Query a host snapshot and exercise replay lifecycle through native Rust DTOs.
//! Run with an explicitly enabled test host; no server or daemon is started here.
use dot_logger::{
    adapters::remote::*,
    capture::Level,
    query::{Query, TimeBasis},
    replay::Speed,
};
use dot_logger_client::{Cancellation, ClientError, RemoteClient, RemoteConfig};
use std::io::Read;
async fn run(endpoint: String) -> Result<(), ClientError> {
    let cancel = Cancellation::new();
    let mut config = RemoteConfig::new(endpoint);
    config.token = match std::env::var("DOT_LOGGER_TOKEN") {
        Ok(token) => Some(token),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(ClientError::Remote(RemoteError::InvalidInput));
        }
    };
    if let Ok(path) = std::env::var("DOT_LOGGER_CA") {
        let mut bytes = Vec::new();
        let file =
            std::fs::File::open(path).map_err(|_| ClientError::Remote(RemoteError::Transport))?;
        std::io::Read::take(file, 65537)
            .read_to_end(&mut bytes)
            .map_err(|_| ClientError::Remote(RemoteError::Transport))?;
        if bytes.len() > 65536 {
            return Err(ClientError::Remote(RemoteError::InvalidInput));
        }
        config.ca_certificate = Some(bytes);
    }
    config.tls_domain = std::env::var("DOT_LOGGER_TLS_DOMAIN").ok();
    let mut client = RemoteClient::connect(config, &cancel).await?;
    let page = client
        .request(
            ManagementRequest::Query {
                snapshot: 0,
                cursor: 0,
                query: Query {
                    basis: TimeBasis::Mono,
                    from: 0,
                    to: u64::MAX as i128 + 1,
                    channel: None,
                    minimum_level: Level::Trace,
                },
            },
            &cancel,
        )
        .await?;
    let ManagementReply::Page {
        snapshot,
        cursor,
        items,
    } = page
    else {
        return Err(ClientError::Remote(RemoteError::Corrupt));
    };
    // A screen receives the same native record/descriptor data as the CLI. It must
    // retain the host snapshot/cursor capabilities instead of inventing file paths.
    println!("page records={}, more={}", items.len(), cursor != 0);
    let started = client
        .request(
            ManagementRequest::Replay {
                handle: snapshot,
                action: ReplayAction::Start,
                generation: 0,
                from: 0,
                speed: Speed::REALTIME,
            },
            &cancel,
        )
        .await?;
    let ManagementReply::Replay { generation, .. } = started else {
        return Err(ClientError::Remote(RemoteError::Corrupt));
    };
    for action in [
        ReplayAction::Pause,
        ReplayAction::Resume,
        ReplayAction::Stop,
    ] {
        client
            .request(
                ManagementRequest::Replay {
                    handle: snapshot,
                    action,
                    generation,
                    from: 0,
                    speed: Speed::REALTIME,
                },
                &cancel,
            )
            .await?;
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    let Some(endpoint) = std::env::args().nth(1) else {
        eprintln!("usage: remote_query_replay <http(s) endpoint>");
        return std::process::ExitCode::from(2);
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(_) => return std::process::ExitCode::from(4),
    };
    match runtime.block_on(run(endpoint)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(4)
        }
    }
}
