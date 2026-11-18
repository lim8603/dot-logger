use crate::{
    Options,
    catalog::{Catalog, Channel},
    output::{Notice, data_message, status},
    websocket::{self, Control, STATUS_INTERVAL, Writer},
};
use dot_logger_client::{Cancellation, RecordedItem, RemoteConfig, RemoteRecording};
use futures_util::StreamExt;
use std::{
    collections::{BTreeMap, HashSet},
    fmt, io,
    io::{BufRead, Read},
    num::NonZeroU32,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
    time::{Instant, MissedTickBehavior, interval, sleep},
};
use tokio_tungstenite::tungstenite::Message;

const GREETING: &str =
    r#"{"op":"serverInfo","name":"dot-logger-viewer replay","capabilities":["time"]}"#;
const MAX_PAGES: usize = 4096;
const MAX_CONTROL_LINE: u64 = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rate(NonZeroU32, NonZeroU32);

impl Rate {
    pub(crate) const REALTIME: Self = Self(NonZeroU32::MIN, NonZeroU32::MIN);
}

impl FromStr for Rate {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let error = "expected a positive rational playback rate such as 2/1";
        let (numerator, denominator) = value.split_once('/').ok_or(error)?;
        Ok(Self(
            numerator.parse().map_err(|_| error)?,
            denominator.parse().map_err(|_| error)?,
        ))
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

#[derive(Clone, Copy)]
struct Position {
    generation: u64,
    from: u64,
    rate: Rate,
}

#[derive(Clone)]
pub(crate) struct Playback {
    start: u64,
    end: u64,
    position: watch::Sender<Position>,
}

impl Playback {
    pub(crate) fn new(start: u64, end: u64, rate: Rate) -> io::Result<Self> {
        if start >= end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "replay range must be nonempty",
            ));
        }
        let (position, _) = watch::channel(Position {
            generation: 0,
            from: start,
            rate,
        });
        Ok(Self {
            start,
            end,
            position,
        })
    }

    pub(crate) fn seek(&self, from: u64) -> bool {
        (self.start..self.end).contains(&from)
            && self.position.send_if_modified(|position| {
                let Some(generation) = position.generation.checked_add(1) else {
                    return false;
                };
                position.generation = generation;
                position.from = from;
                true
            })
    }

    pub(crate) fn start_terminal(&self) -> io::Result<()> {
        let playback = self.clone();
        std::thread::Builder::new()
            .name("viewer-playback-control".into())
            .spawn(move || {
                if let Err(error) = playback.controls(io::stdin().lock()) {
                    eprintln!("replay controls: {error}");
                }
            })?;
        Ok(())
    }

    fn controls(&self, mut input: impl BufRead) -> io::Result<()> {
        let mut line = Vec::new();
        loop {
            line.clear();
            let count = (&mut input)
                .take(MAX_CONTROL_LINE + 1)
                .read_until(b'\n', &mut line)?;
            if count == 0 {
                return Ok(());
            }
            if count as u64 > MAX_CONTROL_LINE {
                if line.last() != Some(&b'\n') {
                    input.skip_until(b'\n')?;
                }
                eprintln!("replay controls: line too long");
                continue;
            }
            let text = std::str::from_utf8(&line).unwrap_or_default().trim();
            if let Some(value) = text.strip_prefix("seek ") {
                if !value.parse().is_ok_and(|mono| self.seek(mono)) {
                    eprintln!("replay controls: seek position must be within the configured range");
                }
            } else if let Some(value) = text.strip_prefix("speed ") {
                match value.parse() {
                    Ok(rate) => self.position.send_modify(|position| position.rate = rate),
                    Err(reason) => eprintln!("replay controls: {reason}"),
                }
            } else if !text.is_empty() {
                eprintln!("replay controls: expected seek MONO_NS or speed N/D");
            }
        }
    }
}

struct Clock {
    media: u64,
    wall: Instant,
    rate: Rate,
}

impl Clock {
    fn set_rate(&mut self, rate: Rate, now: Instant) {
        if self.rate != rate {
            let elapsed = now.duration_since(self.wall).as_nanos();
            let media_elapsed =
                elapsed * u128::from(self.rate.0.get()) / u128::from(self.rate.1.get());
            self.media = self
                .media
                .saturating_add(u64::try_from(media_elapsed).unwrap_or(u64::MAX));
            self.wall = now;
            self.rate = rate;
        }
    }

    fn remaining(&self, media: u64, now: Instant) -> Duration {
        let delta = u128::from(media.saturating_sub(self.media));
        let wall_ns =
            (delta * u128::from(self.rate.1.get())).div_ceil(u128::from(self.rate.0.get()));
        let remaining = wall_ns.saturating_sub(now.duration_since(self.wall).as_nanos());
        Duration::from_nanos(u64::try_from(remaining).unwrap_or(u64::MAX))
    }

    fn delivered(&mut self, media: u64, now: Instant) {
        // Move the anchor to a late delivery so slow IO does not bunch up
        // later samples that were separated in the recording.
        self.media = media;
        self.wall = now;
    }
}

type Interests = BTreeMap<u32, Arc<Channel>>;

struct Interest {
    id: u32,
    channels: watch::Sender<Interests>,
}

impl Drop for Interest {
    fn drop(&mut self) {
        self.channels.send_modify(|channels| {
            channels.remove(&self.id);
        });
    }
}

struct Output {
    generation: u64,
    message: Message,
    subscription: Option<(u32, Arc<Channel>)>,
}

pub(crate) async fn session(
    tcp: TcpStream,
    remote: RemoteConfig,
    catalog: Arc<Catalog>,
    options: Arc<Options>,
    playback: Playback,
) {
    let Some(socket) = websocket::open(tcp, &catalog, &options, GREETING).await else {
        return;
    };
    let (writer, reader) = socket.split();
    let (channels, interests) = watch::channel(Interests::new());
    let (commands, controls) = mpsc::channel(8);
    let (out, output) = mpsc::channel(1);
    let position = playback.position.subscribe();
    let driver = Driver {
        position: position.clone(),
        interests: interests.clone(),
        out,
        end: playback.end,
        record_bytes: options.record_bytes,
    };
    tokio::select! {
        _ = websocket::read(reader, &catalog, &commands, |channel, id| {
            channels.send_modify(|active| { active.insert(id, Arc::new(channel)); });
            std::future::ready(Ok(Interest { id, channels: channels.clone() }))
        }) => {},
        _ = write(writer, controls, output, interests, position, &catalog) => {},
        _ = driver.run(remote, options.replay_blob_bytes) => {},
    }
}

async fn write(
    mut writer: Writer,
    mut controls: mpsc::Receiver<Control>,
    mut output: mpsc::Receiver<Output>,
    interests: watch::Receiver<Interests>,
    position: watch::Receiver<Position>,
    catalog: &Catalog,
) {
    let mut tick = interval(STATUS_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut retired = HashSet::new();
    loop {
        tokio::select! {
            biased;
            command = controls.recv() => {
                if !websocket::apply_control(&mut writer, command).await { break; }
            }
            _ = tick.tick() => {
                if !websocket::send_retired(&mut writer, catalog, &mut retired).await { break; }
            }
            frame = output.recv() => {
                let Some(frame) = frame else { break; };
                if frame.generation != position.borrow().generation {
                    continue;
                }
                if let Some((id, expected)) = &frame.subscription {
                    let active = interests.borrow();
                    if expected.retired()
                        || !active.get(id).is_some_and(|entry| Arc::ptr_eq(entry, expected))
                    {
                        continue;
                    }
                }
                if !websocket::send(&mut writer, frame.message).await { break; }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Progress {
    Seek,
    End,
    Closed,
}

struct Driver {
    position: watch::Receiver<Position>,
    interests: watch::Receiver<Interests>,
    out: mpsc::Sender<Output>,
    end: u64,
    record_bytes: usize,
}

impl Driver {
    async fn run(mut self, remote: RemoteConfig, blob_bytes: usize) {
        let cancel = Cancellation::new();
        let mut recording = None;
        loop {
            if self
                .interests
                .wait_for(|channels| !channels.is_empty())
                .await
                .is_err()
            {
                return;
            }
            let position = *self.position.borrow_and_update();
            if recording.is_none() {
                match RemoteRecording::connect(remote.clone(), blob_bytes, &cancel).await {
                    Ok(connection) => recording = Some(connection),
                    Err(error) => {
                        if !self.failed(position.generation, &error.to_string()).await {
                            return;
                        }
                        continue;
                    }
                }
            }
            let Some(connection) = recording.as_mut() else {
                return;
            };
            match self.play(connection, position, &cancel).await {
                Ok(Progress::Closed) => return,
                Ok(Progress::Seek) => continue,
                Ok(Progress::End) => {
                    if !self
                        .emit(
                            position.generation,
                            status("playback", 0, "Replay ended"),
                            None,
                        )
                        .await
                    {
                        return;
                    }
                    if newer_seek(&mut self.position, position.generation)
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                Err(error) => {
                    // A failed playback may have an unusable connection or snapshot.
                    // Open a fresh one when the operator next seeks.
                    recording = None;
                    if !self.failed(position.generation, &error).await {
                        return;
                    }
                }
            }
        }
    }

    async fn play(
        &mut self,
        connection: &mut RemoteRecording,
        start: Position,
        cancel: &Cancellation,
    ) -> Result<Progress, String> {
        let generation = start.generation;
        if !self.emit(generation, time(start.from), None).await
            || !self
                .emit(
                    generation,
                    status("playback", 0, format!("Replay at {} ns", start.from)),
                    None,
                )
                .await
        {
            return Ok(Progress::Closed);
        }
        let mut clock = Clock {
            media: start.from,
            wall: Instant::now(),
            rate: start.rate,
        };
        let mut cursor = 0;
        let mut previous = None;
        for _ in 0..MAX_PAGES {
            if let Some(progress) = self.ready(generation).await {
                return Ok(progress);
            }
            // Let an in-flight first page return its snapshot capability even
            // after a seek. Drop its records, then reuse the same snapshot.
            let page = connection
                .page(start.from, self.end, cursor, cancel)
                .await
                .map_err(|error| error.to_string())?;
            if self.position.borrow().generation != generation {
                return Ok(Progress::Seek);
            }
            for item in page.items {
                let key = item.order_key();
                if previous.is_some_and(|last| key <= last) {
                    return Err("Query returned duplicate or out-of-order records".into());
                }
                previous = Some(key);
                if let Some(progress) = self.ready(generation).await {
                    return Ok(progress);
                }
                if let Some(progress) = self
                    .record(connection, item, generation, &mut clock, cancel)
                    .await?
                {
                    return Ok(progress);
                }
            }
            cursor = page.next_cursor;
            if cursor == 0 {
                return Ok(Progress::End);
            }
        }
        Err("Replay exceeded the page limit".into())
    }

    async fn record(
        &mut self,
        connection: &mut RemoteRecording,
        item: RecordedItem,
        generation: u64,
        clock: &mut Clock,
        cancel: &Cancellation,
    ) -> Result<Option<Progress>, String> {
        let metadata = item.metadata();
        let mono = item.order_key().0;
        let channel = self
            .interests
            .borrow()
            .values()
            .find(|channel| channel.native_id == metadata.channel_id && !channel.retired())
            .cloned();
        let Some(channel) = channel else {
            return Ok(None);
        };
        if !channel.matches(metadata) {
            channel.retire();
            let notice = Notice::Unsupported {
                channel: metadata.channel_id,
                reason: "local schema mapping does not match",
            };
            return Ok(
                (!self.emit(generation, notice.message(1), None).await).then_some(Progress::Closed)
            );
        }
        let mut seek = self.position.clone();
        let payload = tokio::select! {
            biased;
            change = newer_seek(&mut seek, generation) => return Ok(Some(progress(change))),
            result = connection.payload(item, self.record_bytes, cancel) => result.map_err(|error| error.to_string())?,
        };
        loop {
            let position = *self.position.borrow_and_update();
            if generation != position.generation {
                return Ok(Some(Progress::Seek));
            }
            let now = Instant::now();
            clock.set_rate(position.rate, now);
            let wait = clock.remaining(mono, now);
            if wait.is_zero() {
                break;
            }
            tokio::select! {
                biased;
                changed = self.position.changed() => {
                    if changed.is_err() { return Ok(Some(Progress::Closed)); }
                }
                _ = sleep(wait.min(Duration::from_secs(60))) => {},
            }
        }
        let destinations: Vec<_> = self
            .interests
            .borrow()
            .iter()
            .filter(|(_, channel)| channel.native_id == metadata.channel_id)
            .map(|(&id, channel)| (id, channel.clone()))
            .collect();
        if destinations.is_empty() {
            return Ok(None);
        }
        if !self.emit(generation, time(mono), None).await {
            return Ok(Some(Progress::Closed));
        }
        for (id, channel) in destinations {
            if !self
                .emit(
                    generation,
                    data_message(id, mono, &payload),
                    Some((id, channel)),
                )
                .await
            {
                return Ok(Some(Progress::Closed));
            }
        }
        clock.delivered(mono, Instant::now());
        Ok(None)
    }

    async fn ready(&mut self, generation: u64) -> Option<Progress> {
        tokio::select! {
            biased;
            change = newer_seek(&mut self.position, generation) => Some(progress(change)),
            interest = self.interests.wait_for(|channels| !channels.is_empty()) => {
                interest.err().map(|_| Progress::Closed)
            }
        }
    }

    async fn failed(&mut self, generation: u64, error: &str) -> bool {
        self.emit(
            generation,
            status("playback", 2, format!("Replay failed: {error}")),
            None,
        )
        .await
            && newer_seek(&mut self.position, generation).await.is_ok()
    }

    async fn emit(
        &self,
        generation: u64,
        message: Message,
        subscription: Option<(u32, Arc<Channel>)>,
    ) -> bool {
        self.out
            .send(Output {
                generation,
                message,
                subscription,
            })
            .await
            .is_ok()
    }
}

async fn newer_seek(
    position: &mut watch::Receiver<Position>,
    generation: u64,
) -> Result<(), watch::error::RecvError> {
    position
        .wait_for(|value| value.generation != generation)
        .await
        .map(drop)
}

fn progress(change: Result<(), watch::error::RecvError>) -> Progress {
    if change.is_ok() {
        Progress::Seek
    } else {
        Progress::Closed
    }
}

fn time(mono_ns: u64) -> Message {
    let mut bytes = [0u8; 9];
    bytes[0] = 2;
    bytes[1..].copy_from_slice(&mono_ns.to_le_bytes());
    Message::binary(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn changing_rate_preserves_position_and_late_delivery_preserves_the_next_gap() {
        let start = Instant::now();
        let mut clock = Clock {
            media: 100,
            wall: start,
            rate: Rate::REALTIME,
        };
        let transition = start + Duration::from_nanos(50);
        clock.set_rate("2/1".parse().unwrap(), transition);
        assert_eq!(clock.remaining(250, transition), Duration::from_nanos(50));
        let late = start + Duration::from_nanos(1000);
        assert_eq!(clock.remaining(250, late), Duration::ZERO);
        clock.delivered(250, late);
        assert_eq!(clock.remaining(350, late), Duration::from_nanos(50));
        clock.set_rate("1/2".parse().unwrap(), late);
        assert_eq!(clock.remaining(350, late), Duration::from_nanos(200));
    }

    #[test]
    fn terminal_seeks_and_speeds_preserve_the_last_valid_controls() {
        let playback = Playback::new(10, 200, Rate::REALTIME).unwrap();
        playback
            .controls(&b"seek 100\nspeed 2/1\nseek 200\nspeed 0/1\n"[..])
            .unwrap();
        let position = *playback.position.borrow();
        assert_eq!(position.from, 100);
        assert_eq!(position.generation, 1);
        assert_eq!(position.rate.to_string(), "2/1");
    }
}
