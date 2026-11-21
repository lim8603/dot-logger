# dot-logger CLI

`dot-logger` reads a dot-logger host over gRPC, or immutable recorded files
offline. Every command is a bounded task: it has a deadline, an output limit
and a record limit, and it never starts a daemon or a background worker.

```sh
cargo run -p dot-logger-cli -- --help
cargo run -p dot-logger-cli -- status --endpoint http://127.0.0.1:PORT --json
cargo run -p dot-logger-cli -- query --endpoint http://127.0.0.1:PORT --from 0 --to 1000000000 --json
```

Replace `PORT` with the port the host reports (for the default path,
`Logger::remote_addr()`); the server picks a free loopback port unless the
host configures one.

## Connecting

- The host must enable its server explicitly.
- Plain HTTP is accepted only for a literal loopback IP. Any other endpoint
  needs HTTPS, a CA certificate (`--ca <PEM>`) and a token. Use
  `--tls-domain NAME` to override the certificate name.
- The token is read from the `DOT_LOGGER_TOKEN` environment variable, or from
  the variable named by `--token-env NAME`. It is never accepted as an
  argument and never printed.
- After a failure the CLI does not reconnect or resend anything.

## Commands

| Command | Purpose |
|---|---|
| `status` | Session and revision, plus the host's `dot_logger.status.v1` metrics. A host without a metrics backend reports them as Unavailable |
| `query` | Records in `[from, to)` by monotonic, real or source time, filtered by channel and level, in bounded pages |
| `blob` | Read a range of a remote blob (`--snapshot --blob --offset --length`) |
| `replay` | Remote replay control (`--handle --action start/pause/resume/seek/stop/speed --generation --speed N/D`), or timed offline playback |
| `tail` | Subscribe (`--records --record-bytes --age-ms`). `--record-bytes` (default 8 MiB) and `--records` must fit the host's limits, or the request is `invalid_input`. Originals are opt-in with `--originals`. On overflow, report a Gap or stop with `--disconnect-on-overflow` |
| `completion` | Observe whether a record reached a boundary (`--session <32 hex> --producer --sequence --target --boundary`) |
| `loss` | Offline loss counters stored in the files: cumulative values, differences and unobserved ranges |
| `compare` | Compare two snapshots the host registered (`--before ID --after ID`) |
| `config validate` | Check an offline manifest's registrations, limits and memory reservations (it does not scan the files) |
| `config diff` | Report only whether two declarations differ (`--before FILE --after FILE`); values are not shown |
| `config apply` | Post a live change to the host (`--base-revision --patch FILE`); needs the Manage permission |
| `check plan`, `check run` | Preview, or run once, a check plan the host registered (`--plan ID`) |
| `recover` | Recover recorded files into new, independent files |
| `calibrate` | Measure an output file system once (writes up to `--write-limit`) and save a preset profile |

## Status, live changes and comparison

On the default path, `LoggerBuilder::remote` answers `status` and channel
level/sampling changes. Changes work with or without telemetry. Output routes
are fixed at start, so a route change is `invalid_input`, and a memory change
is `restart_required`. The default path cannot register comparison
snapshots, so `compare` has nothing to compare there.

On the advanced path the host creates a `telemetry::operations::OperationsHost`
and answers `GetStatus`, `ApplyConfig` and `Compare` commands with
`handle_command(command, telemetry, config_updates, now_ns)` on an ordinary
worker. The host also samples telemetry every second, registers snapshots
(`register_snapshot`) and lets producers apply changes
(`apply_pending_config()`); nothing does this automatically.

- `status` shows the latest sample and the 10/60/300-second windows, each as
  Observed, Stale or Unavailable, and lists snapshot IDs usable for
  `compare`. An unobserved value is never shown as zero.
- A `config apply` patch. Unknown fields are rejected; levels are trace,
  debug, info, warn and error; omitted lists are unchanged:

```json
{"schema":"dot_logger.config_patch.v1",
 "channels":[{"channel":7,"minimum_level":"warn","sample_every":2}],
 "routes":[{"channel":7,"targets":[10]}]}
```

| Result | outcome / exit | Meaning |
|---|---|---|
| Applied | `applied` / 0 | The posted revision. Each producer switches separately |
| Stale base | `revision_conflict` / 3 | Nothing posted. Read the revision with `status` and retry |
| Previous change draining | `revision_busy` / 3 | Nothing posted. Retry after idle producers poll and old records drain |
| Memory change | `restart_required` / 2 | Nothing posted. Restart the owner |
| Bad patch | `invalid_input` / 2 | Nothing posted |

A posted revision does not mean that files rotated or synchronized. On the
default path, a writer your code took and left idle must call
`Writer::refresh()` on its thread before a later change can post.

`compare` keeps every condition difference between the two snapshots. If the
conditions make them incomparable it reports `comparable:false` and no
numbers. CPU, memory and latency are Unavailable unless the host measured
them.

A query's snapshot is the handle for `replay`. Keep the confirmed generation
for control requests and use the new generation a seek returns. Snapshots,
blobs and cursors are bounded host capabilities, not file paths.

## Checks

A check needs a host that registered a dedicated path, input and budget with
`diagnostics::CheckHost` and handles commands on an ordinary worker. The CLI
only passes the registered ID.

```sh
# 7 is an example; use the ID the host registered.
cargo run -p dot-logger-cli -- check plan --endpoint http://127.0.0.1:PORT --plan 7 --json
cargo run -p dot-logger-cli -- check run --endpoint http://127.0.0.1:PORT --plan 7 --json
```

- `check plan` shows the budget, input and source without creating paths or
  probing the system.
- `check run` needs the Manage permission and consumes the ID; run it again
  only with a newly registered plan. If the host has no check backend the
  answer is Unsupported; it never falls back to running.
- The answer is `dot_logger.check.v1` JSON. Pass exits 0, Fail 4, Aborted or
  Inconclusive 3. A deadline or cancellation stops new load; the result comes
  after the footer, sync, close and cleanup finished. After a timeout the
  result keeps `completion_unconfirmed` and nothing is rerun.
- The host's message limit must be at least 5120 bytes; otherwise the run is
  refused with `limit` before any IO.
- Free space is probed with `statvfs` on Linux. The synthetic result is kept
  apart from the host's operating report and profile; `capacity` and
  `network_measurement` are Unavailable.

## Offline files

Copy [`examples/offline_session.json`](examples/offline_session.json) to
`.logger/session.json` in the host project and fill in the original session,
producer count, channel registration order, each source's revision, sequence
and path, and the budgets. Relative source paths are relative to the manifest.
If a channel has a schema descriptor, give its original value as
`descriptor: {"encoding": "...", "data_hex": "..."}`. The example values are
illustrations, not product or device defaults.

```sh
cargo run -p dot-logger-cli -- config validate --manifest .logger/session.json --json
cargo run -p dot-logger-cli -- query --manifest .logger/session.json --from 0 --to 1000000000 --json
cargo run -p dot-logger-cli -- replay --manifest .logger/session.json --speed 2/1 --json
cargo run -p dot-logger-cli -- recover --manifest .logger/session.json --output .logger/recovered.mcap --json
```

- The manifest is `schema_version: 2`; each `sources[]` entry needs its own
  `revision`. Version 1, a global `revision` and format selection fields are
  not supported.
- Each file's session, revision and sequence are checked against the
  manifest. Duplicate paths, a repeated target and sequence, and a repeated
  receipt are rejected. Outputs that reuse a sequence must each name
  `sources[].target`, which is checked against the file. Offline `status`
  lists each file's sequence and revision.
- Source files are opened read-only and their prefix must not change while in
  use. An active file needs an explicit `safe_offset` at a complete chunk.
- Offline cursors and blobs live as long as the process's snapshot. Results
  contain metadata and descriptors by default; add `--include-payload` for
  the bytes. Large reads use both the blob budgets and the output budget.
- Offline replay supports a start position and the real caller clock. For
  interactive control use the Rust replay API, or a remote replay. Ctrl-C
  cancels cooperatively and waits for the reader to finish, then returns the
  partial result.

### Recovery

- Each source is recovered independently, in manifest order, into a new
  file. One source writes `--output NEW_FILE`; several sources write
  `<sequence>.mcap` (or `<target>-<sequence>.mcap` when sequences repeat)
  under `--output NEW_DIRECTORY`. Existing paths are never overwritten.
- The recovered range is the whole immutable file, or a `safe_offset` equal to
  its length; recovering a partial active prefix is Unsupported.
- Each recovery row reports the sequence, revision and the recovered and
  excluded ranges. The excluded tail includes the replaced index and summary,
  so it is not a count of lost messages.
- `max_segments` limits the files, `max_source_bytes` each file, `read_bytes`
  the total source length, `max_records` the recovered user records and
  `limits.max_loss_reports` the recovered loss parts. Exceeding a budget is
  never reported as success.
- If a later file fails, earlier outputs and the partial output stay for
  inspection; nothing is retried, merged, overwritten or deleted.
- Outputs are file-synced with a rebuilt index, but the parent directory is
  not synced (`namespace_synced:false`). Recovery does not claim that records
  were written or synced by the original process.

### Loss counters

`loss --manifest .logger/session.json --json` prints the loss counters stored
in `.logger/loss` as separate `loss` rows.

- Set a positive part limit in `limits.max_loss_reports` and size
  `external_bytes` for it. Without it, ordinary queries skip the counters, and
  a loss query is `limit`.
- Files recorded without loss collection are listed under
  `unavailable_sources`; that is not proof of zero loss.
- `--from/--to` select overlapping observation windows on the counter
  clock (`observation_clock:counter_epoch_monotonic_ns`), not record times or
  the time of a loss. `--channel` filters a product channel; `--basis`
  supports `mono` only.
- Values are cumulative per session, epoch and original target; output
  replicas are not added up. A missing part, an overflow or a decreasing
  value makes the difference `null`. `tail_confirmed:false` means the tail
  after the last sample is unconfirmed.
- Rows with a zero count and no overflow are omitted. `summary.records` is 0
  for `loss`; `loss_rows` and `report_parts` count the rows.

## Output

`--json` prints NDJSON: one object per line with `schema_version:1`,
`command`, `kind` and `data`. Data rows such as `record`, `gap` or `status`
end with a `summary` or `error` row.

- 128-bit IDs are 32 hex digits. Lossless u64/i128 times, sequences,
  revisions and handles are strings.
- `data_hex` appears only when payload output was requested.
- In `summary` and `error`, `records` is the number of `record` rows this
  command printed and `acknowledged` the number of those the server
  confirmed. Commands that print no records report 0.
- Payloads and credentials are never included in error messages.

| Exit code | Meaning |
|---|---|
| 0 | The requested bounded task or observation completed |
| 2 | Input, configuration, permission or unsupported |
| 3 | Partial result, cancellation, deadline, output or read limit, gone or expired, unknown acknowledgement |
| 4 | IO, transport or invalid response |

`gone_or_expired`, `permission_denied`, `incompatible_protocol`,
`revision_conflict`, `revision_busy` and `restart_required` are reported apart
from ordinary transport errors.

## Limits

| Limit | Default | Range |
|---|---|---|
| Whole-command deadline (`--timeout-ms`) | 5 s | up to 1 hour |
| Output (`--max-output-bytes`) | 16 MiB | 1024 B – 256 MiB |
| Records (`--limit`) | 1000 | up to 1,000,000 |
| Encoded request/reply size (`--message-bytes`) | 263,168 B | 2048 B – 16 MiB |

- Data output always leaves room for the final summary.
- `tail --limit N` hands N records to stdout, waits for their
  acknowledgements and exits normally. A record whose stdout write failed is
  never acknowledged. If an acknowledgement cannot be confirmed after the
  handoff, the result is unknown; nothing is resent to guess success.
- A query or replay that reaches a limit before finishing its selection is a
  partial result. A local replay that ends exactly at the limit is complete.
- `tail` checks its whole input and reservation before connecting and stays
  within `--timeout-ms` in total. `--age-ms` sets both the record lifetime and
  the server idle lease. Records missed between two subscriptions are not
  reported as a Gap; do not treat separate subscriptions as one continuous
  stream.
- A handoff refused because the record expired or its generation was
  discarded is `gone_or_expired` (exit 3) with no acknowledgement. Corrupt
  input is `invalid_data` (exit 4) and an oversized message `limit` (exit 3).

## Cancellation and unknown outcomes

Ctrl-C or a deadline does not cancel host commands or IO already posted, and
never turns an unfinished task into success. A cancelled or timed-out remote
command, and a `gone` answer to `config apply`, `check run` or `replay`, add
`posted_host_io: completion_unconfirmed`; check `status` or `completion`
afterwards. The three refused changes above post nothing and carry no such
note. A status without metrics never gets invented zeros or a pass, and an
unknown status or compare report schema is `invalid_data` (exit 4).

## File format

Files are native MCAP with the profile `dot-logger/standard/2`. They hold the
user payload and schema directly, so standard MCAP tools can decode them;
there is no separate export step. The `.logger/meta` channel keeps the
original receipts, clocks and configuration, and `.logger/loss` the loss
counters. Earlier development files (`dot-logger/standard/1`) are not
supported as input.
