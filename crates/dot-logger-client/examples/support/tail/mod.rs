//! Finite screen adapter example, shared with integration tests; ordinary path only.
#![allow(dead_code)]
use dot_logger::capture::CapturedRecord;
use dot_logger_client::{Cancellation, ClientError, RemoteEvent, RemoteSubscription};
use serde_json::{Value, json};

pub mod scenario;
pub use scenario::run;

/// General-path example failure; never contains endpoint credentials.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
fn invalid(message: &'static str) -> Error {
    std::io::Error::other(message).into()
}

/// Observed terminal reason, distinct from the ambiguous `next` timeout.
#[derive(Clone, Copy, Debug)]
pub enum EndReason {
    /// SDK confirmed the stream has ended.
    StreamEnded,
    /// Explicit stop of this finite example.
    Stopped,
    /// SDK failure; ACK unknown never means the consumer did not receive data.
    Client(ClientError),
}

/// Bounded receiving application history, including visible boundary markers.
/// A real chart groups by segment/span AND the original source series identity.
pub struct Screen {
    rows: Vec<Value>,
    limit: usize,
    current: Option<u64>,
    last: u64,
    span: u64,
}
impl Screen {
    /// At most 64 rows of at most 64 payload bytes each; retain one terminal slot.
    /// Runtime/JSON metadata/allocator storage is ordinary host headroom, not RT.
    pub fn new(limit: usize) -> Result<Self, Error> {
        if !(3..=64).contains(&limit) {
            return Err(invalid("screen row limit must be 3..=64"));
        }
        Ok(Self {
            rows: Vec::with_capacity(limit),
            limit,
            current: None,
            last: 0,
            span: 0,
        })
    }
    fn live(&self, segment: u64) -> Result<(), Error> {
        if self.current != Some(segment) {
            return Err(invalid("stale screen subscription"));
        }
        Ok(())
    }
    fn push(&mut self, row: Value) -> Result<(), Error> {
        if self.rows.len() >= self.limit - 1 {
            return Err(invalid("screen history full; stop without ACK"));
        }
        self.rows.push(row);
        Ok(())
    }
    /// Call only after successful explicit subscribe. Never joins the old segment,
    /// even on the same TCP connection or with adjacent capture sequence numbers.
    pub fn begin(&mut self) -> Result<u64, Error> {
        if self.current.is_some() {
            return Err(invalid("screen subscription still active"));
        }
        let segment = self
            .last
            .checked_add(1)
            .ok_or_else(|| invalid("segment overflow"))?;
        self.push(json!({
            "kind": "subscription_started", "segment": segment,
            "previous_segment": (self.last != 0).then_some(self.last),
            "connect_from_previous": false, "missing_records": null,
            "marker": "새 구독 — 이전 구독과 연속성 미확인"
        }))?;
        self.current = Some(segment);
        self.last = segment;
        self.span = 1;
        Ok(segment)
    }
    /// Ambiguous local/server timeout: retain the same stream, display uncertainty,
    /// and let a subsequent next/explicit stop resolve its lifecycle.
    pub fn wait(&mut self, segment: u64) -> Result<(), Error> {
        self.live(segment)?;
        self.push(json!({"kind": "wait_timed_out", "segment": segment,
            "continuity": "Unconfirmed", "marker": "대기 만료 — 같은 구독에서 확인 필요"}))
    }
    /// Preserve the original record and bytes only on bounded consumer admission.
    /// Receipt acceptance here is not a claim about the ACK or actual rendering.
    pub fn record(
        &mut self,
        segment: u64,
        record: CapturedRecord,
        bytes: &[u8],
    ) -> Result<(), Error> {
        self.live(segment)?;
        if bytes.len() > 64 || bytes.len() != record.payload_len {
            return Err(invalid("screen payload limit or length mismatch"));
        }
        self.push(json!({"kind": "record", "segment": segment, "span": self.span,
            "receipt": {"session_id": format!("{:032x}", record.receipt.session_id),
                "producer_id": record.receipt.producer_id, "accepted_seq": record.receipt.accepted_seq.to_string()},
            "revision": record.revision.to_string(), "channel_id": record.metadata.channel_id,
            "schema_id": format!("{:032x}", record.metadata.schema_id), "schema_version": record.metadata.schema_version,
            "mono_ns": record.metadata.mono_ns.to_string(), "real_ns": record.metadata.real_ns.to_string(),
            "source_time": record.metadata.source_time.map(|source| json!({
                "ticks": source.ticks.to_string(), "clock_domain": source.clock_domain.to_string()})),
            "level": format!("{:?}", record.metadata.level),
            "importance": format!("{:?}", record.metadata.importance), "payload": bytes,
            "boundary": "ConsumerAccepted"}))
    }
    /// Server overflow positions are scoped to this subscription, not capture IDs.
    pub fn gap(&mut self, segment: u64, first: u64, last: u64) -> Result<(), Error> {
        self.live(segment)?;
        if first == 0 || last < first {
            return Err(invalid("invalid transfer gap"));
        }
        let span = self
            .span
            .checked_add(1)
            .ok_or_else(|| invalid("span overflow"))?;
        self.push(json!({"kind": "gap", "segment": segment,
            "first_transfer": first.to_string(), "last_transfer": last.to_string(), "missing_records": null,
            "reason": "Overflow", "connect_from_previous": false,
            "marker": "구독 내 전송 누락 — 이 경계에서 그래프 분리"}))?;
        self.span = span;
        Ok(())
    }
    /// Keep the terminal marker even when ordinary consumer history is full.
    pub fn end(&mut self, segment: u64, reason: EndReason) -> Result<(), Error> {
        self.live(segment)?;
        let reason = match reason {
            EndReason::StreamEnded => "StreamEnded".to_owned(),
            EndReason::Stopped => "Stopped".to_owned(),
            EndReason::Client(error) => format!("{error:?}"),
        };
        self.rows
            .push(json!({"kind": "subscription_ended", "segment": segment,
            "reason": reason, "connect_from_previous": false, "missing_records": null,
            "marker": "구독 종료 — 이후 데이터와 연결하지 않음"}));
        self.current = None;
        Ok(())
    }
    /// Immutable history for a renderer; no eviction may hide a boundary.
    pub fn rows(&self) -> &[Value] {
        &self.rows
    }
    /// Render the immutable receiving history with visible separator rows.
    /// Rendering happens after queue admission/ACK and never changes completion.
    pub fn render(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        for row in &self.rows {
            match row["kind"].as_str() {
                Some("record") => writeln!(
                    output,
                    "  segment={}/{} session={} producer={} seq={} revision={} mono_ns={} payload={}",
                    row["segment"],
                    row["span"],
                    row["receipt"]["session_id"].as_str().unwrap_or(""),
                    row["receipt"]["producer_id"],
                    row["receipt"]["accepted_seq"].as_str().unwrap_or(""),
                    row["revision"].as_str().unwrap_or(""),
                    row["mono_ns"].as_str().unwrap_or(""),
                    row["payload"]
                )?,
                Some("gap") => writeln!(
                    output,
                    "--- {} (segment={}, transfer={}..={}) ---",
                    row["marker"].as_str().unwrap_or(""),
                    row["segment"],
                    row["first_transfer"].as_str().unwrap_or(""),
                    row["last_transfer"].as_str().unwrap_or("")
                )?,
                Some("subscription_ended") => writeln!(
                    output,
                    "=== {} (segment={}, reason={}) ===",
                    row["marker"].as_str().unwrap_or(""),
                    row["segment"],
                    row["reason"].as_str().unwrap_or("")
                )?,
                _ => writeln!(
                    output,
                    "=== {} (segment={}) ===",
                    row["marker"].as_str().unwrap_or(""),
                    row["segment"]
                )?,
            }
        }
        Ok(())
    }
}

/// One bounded poll outcome; no branch reconnects or retries an ACK.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Consumer admitted the record and this connection confirmed the ACK.
    Acknowledged,
    /// Explicit subscription-local overflow boundary.
    Gap,
    /// No terminal conclusion; retry this exact subscription explicitly.
    Waiting,
    /// Terminal stream/cancellation/failure marker was admitted.
    Ended,
}
/// Consume one SDK event into the receiving application's bounded screen queue.
/// On Ended the caller must drop the stream; on error it must close its owners.
pub async fn receive(
    screen: &mut Screen,
    segment: u64,
    stream: &mut RemoteSubscription,
    cancel: &Cancellation,
) -> Result<Step, Error> {
    screen.live(segment)?;
    match stream.next(cancel).await {
        Ok(Some(RemoteEvent::Record(record))) => {
            match stream
                .handoff(
                    record,
                    |r, bytes| screen.record(segment, *r, bytes).is_ok(),
                    cancel,
                )
                .await
            {
                // A screen step reports a server-confirmed ACK, so flush the
                // queued ACK first (ADR-015).
                Ok(()) => match stream.flush(cancel).await {
                    Ok(()) => Ok(Step::Acknowledged),
                    Err(error) => {
                        screen.end(segment, EndReason::Client(error))?;
                        Ok(Step::Ended)
                    }
                },
                Err(error) => {
                    screen.end(segment, EndReason::Client(error))?;
                    Ok(Step::Ended)
                }
            }
        }
        Ok(Some(RemoteEvent::Gap { first, last })) => {
            if screen.gap(segment, first, last).is_err() {
                screen.end(segment, EndReason::Client(ClientError::ConsumerRejected))?;
                return Ok(Step::Ended);
            }
            Ok(Step::Gap)
        }
        Err(ClientError::Remote(dot_logger::adapters::remote::RemoteError::TimedOut)) => {
            if screen.wait(segment).is_err() {
                screen.end(segment, EndReason::Client(ClientError::ConsumerRejected))?;
                return Ok(Step::Ended);
            }
            Ok(Step::Waiting)
        }
        Ok(None) => {
            screen.end(segment, EndReason::StreamEnded)?;
            Ok(Step::Ended)
        }
        Err(error) => {
            screen.end(segment, EndReason::Client(error))?;
            Ok(Step::Ended)
        }
    }
}

/// Fixed synthetic loopback functional evidence, not continuous app integration.
pub struct TailReport {
    /// Bounded display history with visible discontinuities.
    pub screen: Screen,
    /// Actual confirmed consumer handoff acknowledgements.
    pub acknowledged: usize,
    /// Successful explicit subscriptions, never inferred from retries.
    pub subscriptions: usize,
    /// Shared requested bytes after actual runtime/server/receiver owner cleanup.
    pub external_after_close: usize,
}
