# dot-logger-client

Rust client for dot-logger: remote requests and subscriptions over gRPC, and
query and replay of recorded files. Without features it reads files offline
and provides cooperative cancellation; the `remote` feature adds the gRPC/TLS
client and receiver.

The client never starts a runtime or a background worker. Remote calls run on
a Tokio runtime the host owns.

## Remote requests

```rust
let config = RemoteConfig::new("http://127.0.0.1:PORT");
let mut client = RemoteClient::connect(config, &cancel).await?;
let reply = client.request(ManagementRequest::GetStatus, &cancel).await?;
```

`request` supports status, query, blob reads, replay control, completion,
configuration changes, checks and comparison. An operation the host does not
implement returns `Remote(Unsupported)`. Errors never contain socket, TLS or
authentication details or credentials.

- **Endpoints.** Plain HTTP is accepted only for a literal loopback IP. HTTPS
  needs an explicit CA certificate, and a non-loopback endpoint also needs a
  bearer token. `RemoteConfig` has no `Debug`. `checked_endpoint()` applies
  the same syntax check as `connect` and returns only `scheme://host:port`,
  so it is safe to show or log.
- **Limits.** Encoded requests and replies are capped at 263,168 bytes by
  default (2048 B – 16 MiB). Connecting, each call and each wait for the next
  record time out after 5 seconds by default (up to 1 hour). Decoder, Tokio,
  TLS and HTTP/2 buffers are extra host headroom; this is not a heap or RSS
  limit.
- **One connection.** A client opens one TCP connection and never reconnects
  or resends after a failure. `disconnect` or drop closes it; create a new
  `RemoteClient` for a new connection.
- **Unknown outcomes.** Cancelling or timing out a `request` ends the wait. It
  does not cancel a host command already posted or IO in progress. When a
  management command's outcome is unknown, query its status or completion.

A runnable query-then-replay example (start a host first; the example starts
no server):

```sh
cargo run -p dot-logger-client --features remote --example remote_query_replay -- http://127.0.0.1:PORT
```

It reads the token from `DOT_LOGGER_TOKEN`, the CA file from `DOT_LOGGER_CA`
and the TLS name from `DOT_LOGGER_TLS_DOMAIN`.

## Subscriptions and acknowledgements

`subscribe(&budget, filter, ReceiverConfig, &cancel)` returns a bounded
`RemoteSubscription`.

- `next` returns a fully verified record or a Gap. Reading a record does not
  acknowledge it. CRC, offset, lifetime, generation and connection identity
  are all checked.
- `handoff(record, consumer, &cancel)` calls your synchronous consumer. If it
  accepts the record, the acknowledgement is queued for the same connection
  and `handoff` returns without waiting for the server.
- Queued acknowledgements are sent as one RPC when no acknowledgement RPC is
  in flight, when 64 are queued, or when the oldest has waited 5 ms while
  `next` waits. At most 4 acknowledgement RPCs run at once.
- `flush(&cancel)` sends the queue and waits for every acknowledgement RPC.
  `Ok` means the server confirmed every record handed off so far.
- `acknowledged()` is the number of handed-off records the server has
  confirmed, including after a failure.

Failures:

- A consumer that rejects a record gets `ConsumerRejected`, and no
  acknowledgement is sent.
- Cancellation after a handoff, a failed acknowledgement RPC, or a stream
  that ends or fails with acknowledgements still queued makes the next
  `handoff`, `next` or `flush` return `AcknowledgementUnknown` and closes the
  stream. Every later `flush` returns it too.
- Nothing is resent on a new connection, and nothing already handed off is
  revoked.
- A handoff refused because the record expired or its generation was
  discarded returns `Remote(Gone)` without calling the consumer.
- Dropping the stream closes the subscription; dropping the client closes the
  connection. A `PendingRecord` kept elsewhere holds its reservation until it
  is dropped.

Waits and leases:

- `RemoteConfig::timeout` bounds connecting, unary calls, the subscribe
  handshake and each `next` wait. **When only the local wait expires, the
  subscription stays open**; call `next` again on the same subscription.
- A server error (including the lease expiring), cancellation or a
  connection error closes the stream; the next `next` returns `None`.
- `ReceiverConfig::ttl` is both the lifetime of incomplete or unaccepted
  records and the idle lease requested from the server (capped by the
  server). There is no heartbeat.
- **Records missed between two subscriptions are not reported as a Gap.**
  Show the end of one subscription and the start of the next as a
  discontinuity, not as one lossless stream.

`ClientReceiver` is the low-level receiver for hosts that attach their own
transport. Keep the `ConnectionId` from `begin_connection` for each real
connection and pass it to every push, generation update, end and
acknowledgement.

## Offline files

`open_snapshot(&budget, OfflineConfig, Vec<OfflineSource>, now_ns, &cancel)`
returns a `Snapshot<File>`. Use its page and blob API and `Replay::new` as
with any snapshot.

- `OfflineConfig` holds the original session, registrations and the read,
  index, page and source budgets. `OfflineSource` holds each file's revision,
  sequence and `safe_offset`.
- Several revisions of one session can be queried and replayed together.
  Repeated receipts and file identity mismatches are rejected.
- Files are opened read-only, each with its own reader. The host must keep
  the prefix unchanged and prevent the path from being replaced or deleted.
- An active file needs an explicit `safe_offset` at a complete chunk; nothing
  is guessed after corruption.

`Cancellation::as_atomic()` connects the same token to synchronous core IO.
It cannot force an OS read to stop or bound its time; reservations are
returned after the reader finishes.

## Showing results in your own UI

| Result | How to show it |
|---|---|
| Status | Session and revision. An empty report means the host has no metrics; otherwise show the `dot_logger.status.v1` states (Observed, Stale, Unavailable) as given, never as zeros |
| Applied, Conflict, Busy, RestartRequired | The posted revision, or the reason nothing was posted: read the revision again, retry after draining, or restart |
| Compare report | Show `comparable` and the differences first. An Unavailable value is not a decrease or an improvement |
| Page, Blob | Keep the snapshot, cursor and blob capabilities and the original record identity. They are not file paths |
| Replay transition | Apply the confirmed generation and state. After a seek, update the host and receiver gates together |
| `RemoteEvent::Record` | Accept the handoff only if your bounded queue took the record |
| `RemoteEvent::Gap` | The inclusive transfer range lost to overflow. Do not count it as received |
| Cancelled, TimedOut, AcknowledgementUnknown | Incomplete or unknown. It does not mean the host's IO ended |
| Completion | Distinguish Complete, Pending, Failed, Dropped, Unknown, Expired, NotRouted and Busy |

Rendering on screen happens after the acknowledgement and is not part of it.

## Examples

These examples are modeled on dot-probe, the first robot application that
uses dot-logger. They do not change that application; they show the same
integration points with synthetic or exported input.

| Example | Shows |
|---|---|
| `remote_query_replay` | Query, then replay control, against a running host |
| `dot_probe_bridge` | One exported application snapshot through capture, routing, gRPC and an acknowledged handoff |
| `dot_probe_check` | The same path as a registered, run-once check |
| `dot_probe_pipeline` | Synthetic input into files, a subscription and InfluxDB, before and after a configuration change |
| `dot_probe_tail` | A bounded screen history that marks waits, Gaps and new subscriptions |
| `lan_probe` | Throughput and acknowledgement latency over a LAN, compared with raw TCP |

### `dot_probe_bridge` and `dot_probe_check`

`dot_probe_bridge` reads one JSON snapshot that the application writes to
`PROBE_TICK_REPORT`, records it with `try_record_raw`, routes it to a
loopback gRPC server, receives it through a one-slot consumer queue and
observes the original receipt reaching `ReceiverAcknowledged`. Assembling or
reading a record never acknowledges it.

```sh
# Prints the plan without reading the path, binding a socket or adding load.
cargo run -p dot-logger-client --features remote --example dot_probe_bridge -- --plan /tmp/dot-probe-ticks.json
# Runs once on a report the application wrote.
cargo run -p dot-logger-client --features remote --example dot_probe_bridge -- --run /tmp/dot-probe-ticks.json
```

- Input up to 64 KiB, one slot each for raw, route and receive queues, a
  16 MiB logger budget, a current-thread runtime and a 2-second admission
  window. Cleanup and acknowledgement settlement continue after it (observed
  for up to 3 seconds).
- Pass means the receiver accepted the record and the server confirmed it.
  A refused queue, a closed connection or an unknown acknowledgement is
  Inconclusive; cancellation is Aborted. Exit codes: 0 for Pass or the plan,
  3 for not confirmed, 4 for IO, input or transport errors.
- Loopback is not presented as a LAN measurement. Capacity, network, RSS and
  freshness are Unavailable.

`dot_probe_check` registers the same path as a check with
`CheckHost::register_routed` and runs it once. The check backend
(`examples/support/dot_probe.rs`) is example code for this export format, not
library API; implement your own `RoutedCheckBackend` the same way.

```sh
cargo run -p dot-logger-client --features remote --example dot_probe_check -- --plan /tmp/ticks.json
cargo run -p dot-logger-client --features remote --example dot_probe_check -- --run /tmp/ticks.json
cargo test -p dot-logger-cli --test routed_check_host
```

- The host calls it from an ordinary worker; called inside a Tokio runtime it
  returns an error.
- One record up to 64 KiB, target 10, a single worker, `LoopbackOnly` and
  `ReceiverAcknowledged`, admission up to 2 seconds. The backend checks for
  24 MiB (the 16 MiB logger plus input, JSON, queue and runtime headroom); the
  example allows 32 MiB. These are not product defaults or RSS limits.
- No files are written. Network bytes, capacity, LAN, RSS and freshness are
  Unavailable.
- Registered plans are also reachable through `dot-logger check plan` and
  `check run --plan ID`. Exit codes: 0 for the plan or Pass, 3 for Aborted or
  Inconclusive, 4 for Fail or input errors.

### `dot_probe_pipeline`

Synthetic application-shaped input through the public API, before and after a
configuration change.

```sh
# Prints the plan without database variables, files or sockets.
cargo run --locked -p dot-logger-client --features integration-example --example dot_probe_pipeline -- --plan
# Runs against a disposable InfluxDB container and cleans up.
python3 scripts/check-influxdb.py --pipeline-example --output /tmp/dot-logger-pipeline-evidence
```

To run against an existing database, set `DOT_LOGGER_TEST_INFLUX_URL`,
`DOT_LOGGER_TEST_INFLUX_ORG`, `DOT_LOGGER_TEST_INFLUX_BUCKET` and
`DOT_LOGGER_TEST_INFLUX_TOKEN`, and pass `--run <new-exclusive-measurement>`.
The measurement must not overlap existing data; database data is never
deleted and the token is never printed.

| | Revision 1 | Revision 2 |
|---|---|---|
| Input | 10 each of logs, I64 values and 64-byte raw records | The same input |
| Policy | Accept all | Skip Info logs, keep every 2nd Normal value, keep raw |
| Capture | 30 accepted, 0 skipped | 15 accepted, 15 skipped on purpose |
| File and subscription | 30 each | 15 each |
| Database | 20 logs and values | Route turned off, 0 assigned |

- Receipts, metadata and payloads are compared in an independent MCAP read
  and through real TCP/gRPC with a one-slot consumer queue. Database values
  are checked by query; raw records are not stored there.
- The change goes through `validate_config`, `apply_config` and producer
  acknowledgement; each revision gets its own MCAP writer.
- Measurement windows use a virtual session clock (0–10 s and 11–21 s), so
  the output is marked `SyntheticFunctionalExample`. It is not a
  performance measurement; CPU, RSS, capacity and real-time cost are not
  claimed.
- Exit codes: 0 when every boundary, independent read, query and cleanup
  matched, 3 when the database or subscription result fell short, 4 for
  input, IO or verification errors.

### `dot_probe_tail`

Builds a bounded screen history from synthetic records over loopback gRPC.

```sh
cargo run --locked -p dot-logger-client --features remote --example dot_probe_tail -- --plan
cargo run --locked -p dot-logger-client --features remote --example dot_probe_tail -- --run
cargo test --locked -p dot-logger-client --features remote --test tail_screen
```

| Observation | What the history shows |
|---|---|
| Local or server `TimedOut` | `wait_timed_out` and continuity `Unconfirmed`; wait again on the same subscription |
| `None`, cancellation, other errors | A `subscription_ended` separator with the observed cause |
| New subscription | A new `segment` and a `subscription_started` separator, with `connect_from_previous=false` and an unknown (`null`) missing count |
| Overflow Gap | That subscription's inclusive transfer range, as a new `span`; not converted into a capture loss count |
| History full | Refuses the handoff without acknowledging, keeps the separators, then ends the stream |
| Unknown acknowledgement after a handoff | Keeps the received rows and ends with `AcknowledgementUnknown` |

A renderer must keep `segment`/`span` and the original session, producer,
channel, schema and revision apart, and must not join lines across a new
subscription. The fixed run covers two records after a wait timeout, one on
a new connection after the lease ended, a Gap from a full slot, and explicit
cancellation. Exit codes: 0 when the fixed profile and cleanup matched, 4 for
input, IO or verification errors.
