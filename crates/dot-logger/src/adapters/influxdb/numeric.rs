//! Fixed-template Flux queries; only this numeric/receipt CSV schema is accepted.
use super::*;
use crate::capture::CaptureReceipt;

/// Explicit half-open real-time range for one configured numeric channel.
#[derive(Clone, Copy, Debug)]
pub struct NumericQuery {
    /// Fixed startup channel. Event channels are not numeric queries.
    pub channel: u32,
    /// Inclusive Unix nanoseconds, without clock conversion.
    pub from_ns: i64,
    /// Exclusive Unix nanoseconds.
    pub to_ns: i64,
    /// Positive complete-result row limit, at most 1024.
    pub max_rows: usize,
    /// Positive response bytes allowance, 256 bytes through 1 MiB.
    pub max_response_bytes: usize,
}
/// Preserved numeric field type; no float conversion of integer identities.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumericValue {
    /// Signed integer field.
    Integer(i64),
    /// Finite floating point field.
    Float(f64),
    /// Boolean field.
    Boolean(bool),
}
/// One verified row in the writer's fixed session/device/channel scope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericPoint {
    /// Original capture receipt stored as typed fields.
    pub receipt: CaptureReceipt,
    /// Exact stored Unix nanoseconds.
    pub real_ns: i64,
    /// Original numeric value type.
    pub value: NumericValue,
}
/// Immutable source identity retained even for an empty range or dropped writer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumericScope {
    /// Original capture session.
    pub session_id: u128,
    /// Fixed measurement, without endpoint or credentials.
    pub measurement: String,
    /// Fixed device tag.
    pub device: String,
    /// Queried product channel.
    pub channel: u32,
}
/// A complete bounded range result. Failure never returns an apparent prefix.
/// Its shared external reservation lasts until this actual owner is dropped.
pub struct NumericPage {
    scope: NumericScope,
    query: NumericQuery,
    points: Vec<NumericPoint>,
    _reservation: Arc<Reservation>,
}
impl NumericPage {
    /// Original fixed source identity, including for empty results.
    pub fn scope(&self) -> &NumericScope {
        &self.scope
    }
    /// Exact requested half-open range and limits.
    pub fn query(&self) -> NumericQuery {
        self.query
    }
    /// Borrow the complete, strictly time-ordered result; no detached allocation.
    pub fn points(&self) -> &[NumericPoint] {
        &self.points
    }
}
/// Redacted query failure, independent of an earlier write's API acknowledgment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryError {
    /// Unknown/event channel, invalid range/limits or unsupported Flux string.
    InvalidInput,
    /// Shared memory, response bytes or complete-result row limit exceeded.
    Limit,
    /// Caller cancelled its local wait; remote execution rollback is not inferred.
    Cancelled,
    /// Per-request or whole-query deadline expired.
    TimedOut,
    /// HTTP status other than 200, without request or server error text.
    Http(u16),
    /// Network failure, without credentials or endpoint diagnostics.
    Transport,
    /// Invalid/late-error CSV, type/scope mismatch or unordered/foreign row.
    Corrupt,
}
impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InfluxDB numeric query: {self:?}")
    }
}
impl std::error::Error for QueryError {}
impl Writer {
    /// Query only this writer's fixed session/device/channel/field and time range.
    /// No arbitrary Flux, server mutation, retries or automatic runtime/worker.
    /// Server row limit is global (grouped first) and requests one extra row so
    /// truncation is an explicit Limit error, never a complete-looking prefix.
    /// Client limits bound application result work, not remote query engine RAM.
    pub async fn query_numeric(
        &mut self,
        query: NumericQuery,
        cancel: &AtomicBool,
    ) -> Result<NumericPage, QueryError> {
        if cancel.load(Ordering::Acquire) {
            return Err(QueryError::Cancelled);
        }
        let deadline = Instant::now() + self.config.ttl;
        let kind = self
            .series
            .iter()
            .find(|s| s.channel.id == query.channel)
            .map(|s| s.channel.kind)
            .ok_or(QueryError::InvalidInput)?;
        let field = match kind {
            Kind::Integer => "integer",
            Kind::Float => "float",
            Kind::Boolean => "boolean",
            Kind::Event => return Err(QueryError::InvalidInput),
        };
        if query.from_ns <= i64::MIN + 1
            || query.to_ns == i64::MAX
            || query.from_ns >= query.to_ns
            || !(1..=1024).contains(&query.max_rows)
            || !(256..=1024 * 1024).contains(&query.max_response_bytes)
        {
            return Err(QueryError::InvalidInput);
        }
        let bytes = query
            .max_rows
            .checked_mul(std::mem::size_of::<NumericPoint>())
            .and_then(|n| n.checked_add(query.max_response_bytes))
            .and_then(|n| n.checked_add(65536))
            .ok_or(QueryError::Limit)?;
        let reservation = Arc::new(
            self.reservation
                .reserve_extra(bytes)
                .map_err(|_| QueryError::Limit)?,
        );
        let mut script = Limited {
            text: String::with_capacity(16384),
            limit: 16384,
        };
        // Flux interpolates ${...} inside string literals. Reject that syntax in
        // this intentionally restricted API instead of emitting an unsafe quote.
        let bucket = flux_string(&self.config.bucket)?;
        let measurement = flux_string(&self.config.measurement)?;
        let device = flux_string(&self.config.device)?;
        write!(script, "from(bucket: {bucket}) \
            |> range(start: time(v: {from}), stop: time(v: {to})) \
            |> filter(fn: (r) => r._measurement == {measurement} and r.device == {device} \
                and r.channel == \"{channel}\" and (r._field == \"{field}\" \
                or r._field == \"session\" or r._field == \"producer\" or r._field == \"sequence\")) \
            |> pivot(rowKey: [\"_time\"], columnKey: [\"_field\"], valueColumn: \"_value\") \
            |> filter(fn: (r) => r.session == \"{session:032x}\") \
            |> group(columns: []) |> sort(columns: [\"_time\"]) |> limit(n: {limit}) \
            |> map(fn: (r) => ({{real_ns: int(v: r._time), value: r.{field}, \
                session: r.session, producer: r.producer, sequence: r.sequence}}))",
            from=query.from_ns, to=query.to_ns, channel=query.channel,
            session=self.config.session_id, limit=query.max_rows+1).map_err(|_|QueryError::Limit)?;
        let request_json = serde_json::json!({"type":"flux", "query":script.text,
            "dialect":{"annotations":["datatype","group","default"],"header":true,
            "dateTimeFormat":"RFC3339Nano"}});
        let text = serde_json::to_string(&request_json).map_err(|_| QueryError::InvalidInput)?;
        if text.capacity() > 32768 {
            return Err(QueryError::Limit);
        }
        let body = bytes::Bytes::from_owner(BodyOwner(Arc::new(BodyData {
            text,
            _reservation: Arc::clone(&reservation),
        })));
        let mut url = self.url.clone();
        url.set_path("/api/v2/query");
        url.set_query(None);
        url.query_pairs_mut()
            .append_pair("org", &self.config.organization);
        let request = self
            .client
            .post(url)
            .header(AUTHORIZATION, self.authorization.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "text/csv")
            .body(body)
            .send();
        let mut response = bounded(request, cancel, deadline).await?;
        if response.status().as_u16() != 200 {
            return Err(QueryError::Http(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|n| n > query.max_response_bytes as u64)
        {
            return Err(QueryError::Limit);
        }
        let mut buffer = Vec::with_capacity(query.max_response_bytes);
        while let Some(chunk) = bounded(response.chunk(), cancel, deadline).await? {
            if chunk.len() > query.max_response_bytes - buffer.len() {
                return Err(QueryError::Limit);
            }
            buffer.extend_from_slice(&chunk);
        }
        if cancel.load(Ordering::Acquire) {
            return Err(QueryError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(QueryError::TimedOut);
        }
        let points = parse(&buffer, query, kind, self.config.session_id)?;
        if cancel.load(Ordering::Acquire) {
            return Err(QueryError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(QueryError::TimedOut);
        }
        Ok(NumericPage {
            scope: NumericScope {
                session_id: self.config.session_id,
                measurement: self.config.measurement.clone(),
                device: self.config.device.clone(),
                channel: query.channel,
            },
            query,
            points,
            _reservation: reservation,
        })
    }
}
async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T, reqwest::Error>>,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<T, QueryError> {
    match cooperative(tokio::time::timeout_at(deadline, future), cancel).await {
        None => Err(QueryError::Cancelled),
        Some(Err(_)) => Err(QueryError::TimedOut),
        Some(Ok(Err(error))) => Err(if error.is_timeout() {
            QueryError::TimedOut
        } else {
            QueryError::Transport
        }),
        Some(Ok(Ok(value))) => Ok(value),
    }
}
fn flux_string(text: &str) -> Result<String, QueryError> {
    if text.contains("${") {
        return Err(QueryError::InvalidInput);
    }
    Ok(format!(
        "\"{}\"",
        text.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}
// The query map emits only fixed header names, numeric cells and a 32-byte hex
// session. We intentionally reject quoted/arbitrary CSV instead of implementing
// a permissive general CSV or exposing partially decoded server error records.
fn parse(
    bytes: &[u8],
    query: NumericQuery,
    kind: Kind,
    session: u128,
) -> Result<Vec<NumericPoint>, QueryError> {
    let text = std::str::from_utf8(bytes).map_err(|_| QueryError::Corrupt)?;
    let mut types: Option<[&str; 8]> = None;
    let mut header: Option<[&str; 8]> = None;
    let mut points: Vec<NumericPoint> = Vec::with_capacity(query.max_rows);
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() {
            continue;
        }
        if line.contains('"') {
            return Err(QueryError::Corrupt);
        }
        let mut cells = [""; 8];
        let mut split = line.split(',');
        for cell in &mut cells {
            *cell = split.next().ok_or(QueryError::Corrupt)?;
        }
        if split.next().is_some() {
            return Err(QueryError::Corrupt);
        }
        if cells[0] == "#datatype" {
            if types.is_some() && header.is_none() {
                return Err(QueryError::Corrupt);
            }
            types = Some(cells);
            header = None;
            continue;
        }
        if matches!(cells[0], "#group" | "#default") {
            if types.is_none() || header.is_some() {
                return Err(QueryError::Corrupt);
            }
            continue;
        }
        if header.is_none() {
            let expected = [
                ("", "#datatype"),
                ("result", "string"),
                ("table", "long"),
                ("producer", "unsignedLong"),
                ("real_ns", "long"),
                ("sequence", "unsignedLong"),
                ("session", "string"),
                (
                    "value",
                    match kind {
                        Kind::Integer => "long",
                        Kind::Float => "double",
                        Kind::Boolean => "boolean",
                        _ => return Err(QueryError::Corrupt),
                    },
                ),
            ];
            let types = types.as_ref().ok_or(QueryError::Corrupt)?;
            for (name, ty) in expected {
                let index = cells
                    .iter()
                    .position(|s| *s == name)
                    .ok_or(QueryError::Corrupt)?;
                if cells.iter().filter(|s| **s == name).count() != 1 || types[index] != ty {
                    return Err(QueryError::Corrupt);
                }
            }
            header = Some(cells);
            continue;
        }
        let header = header.as_ref().ok_or(QueryError::Corrupt)?;
        if !cells[0].is_empty() {
            return Err(QueryError::Corrupt);
        }
        let cell = |key: &str| -> Result<&str, QueryError> {
            let index = header
                .iter()
                .position(|s| *s == key)
                .ok_or(QueryError::Corrupt)?;
            Ok(cells[index])
        };
        if !matches!(cell("result")?, "" | "_result") || cell("table")?.parse::<u64>().is_err() {
            return Err(QueryError::Corrupt);
        }
        let real_ns = cell("real_ns")?
            .parse::<i64>()
            .map_err(|_| QueryError::Corrupt)?;
        let session_text = cell("session")?;
        if session_text.len() != 32
            || !session_text.bytes().all(|b| b.is_ascii_hexdigit())
            || u128::from_str_radix(session_text, 16).map_err(|_| QueryError::Corrupt)? != session
        {
            return Err(QueryError::Corrupt);
        }
        let producer_id = cell("producer")?
            .parse::<u32>()
            .map_err(|_| QueryError::Corrupt)?;
        let accepted_seq = cell("sequence")?
            .parse::<u64>()
            .map_err(|_| QueryError::Corrupt)?;
        if accepted_seq == 0
            || real_ns < query.from_ns
            || real_ns >= query.to_ns
            || points.last().is_some_and(|p| real_ns <= p.real_ns)
        {
            return Err(QueryError::Corrupt);
        }
        let value = match kind {
            Kind::Integer => {
                NumericValue::Integer(cell("value")?.parse().map_err(|_| QueryError::Corrupt)?)
            }
            Kind::Float => {
                let value: f64 = cell("value")?.parse().map_err(|_| QueryError::Corrupt)?;
                if !value.is_finite() {
                    return Err(QueryError::Corrupt);
                }
                NumericValue::Float(value)
            }
            Kind::Boolean => {
                NumericValue::Boolean(cell("value")?.parse().map_err(|_| QueryError::Corrupt)?)
            }
            _ => return Err(QueryError::Corrupt),
        };
        if points.len() == query.max_rows {
            return Err(QueryError::Limit);
        }
        points.push(NumericPoint {
            receipt: CaptureReceipt {
                session_id: session,
                producer_id,
                accepted_seq,
            },
            real_ns,
            value,
        });
    }
    if types.is_some() && header.is_none() {
        return Err(QueryError::Corrupt);
    }
    Ok(points)
}
