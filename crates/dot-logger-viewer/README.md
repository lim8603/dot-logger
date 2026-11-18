# dot-logger-viewer

The viewer mirrors live gRPC subscriptions to the read-only Foxglove WebSocket
v1 protocol. Run it on the operator PC with Foxglove or Lichtblick. The
WebSocket listener binds only to loopback and prints its URL at startup.

```sh
cargo run --locked -p dot-logger-viewer --example live_host -- ./viewer-demo
# In another terminal, replace PORT with the port printed by the demo.
cargo run --locked -p dot-logger-viewer -- \
    --upstream http://127.0.0.1:PORT --catalog ./viewer-demo/viewer-catalog.json
```

Open the printed WebSocket URL as a **Foxglove WebSocket** connection. Select
`robot/pose` in Raw Messages. The demo publishes `x=51`, `y=-7` for one minute. Choose a new
output directory for each demo run.

For a robot over the LAN, use its HTTPS endpoint, pass the trusted PEM with
`--ca`, and set `DOT_LOGGER_TOKEN` to an upstream read credential. Use
`--tls-domain` if a certificate name override is required. The browser never
receives the credential. Browser Origins are restricted to loopback HTTP(S).
Use `--allow-origin` to grant a trusted web frontend's exact Origin or set
`--allow-origin file://` for a desktop app that sends that Origin. Native
clients that omit the Origin can connect from loopback.

## Catalog

```json
{
  "version": 1,
  "channels": [
    {
      "id": 1,
      "channel_id": 1,
      "schema_id": "000000000000000000000000504f5345",
      "schema_version": 1,
      "topic": "robot/pose",
      "encoding": "json",
      "schema_name": "Pose",
      "schema": "{\"type\":\"object\"}"
    }
  ]
}
```

Use the host registration's `channel_id`, 32 hexadecimal digits for its
`schema_id`, and `schema_version`. The advertisement `id`, native channel
ID and topic each have to be unique. Use a JSON Schema string (or an empty
string) for JSON, or a base64-encoded `FileDescriptorSet` with imports and the
fully qualified message name for protobuf. Set `originals: true` in a channel
entry to subscribe to raw originals. The catalog is loaded at startup and has
a 1 MiB limit and a maximum of 128 channels.

The viewer forwards the original encoded bytes and the record's Unix
nanoseconds. If the upstream schema ID or version changes, the stale advertisement
is withdrawn. Update the catalog and assign a new advertisement ID when changing
a topic or schema, then restart the viewer.

## Live limits and loss

- Clients interested in the same native channel share one upstream subscription.
  A channel stops consuming upstream resources after its last subscription closes.
- Each WebSocket has independently bounded frame and byte queues and a two-second
  write deadline. Tune limits with `--help`. The counts, bytes, age, and number of
  streams requested upstream must fit the robot's configured remote limits.
- The viewer acknowledges a record once at least one downstream queue accepts it.
  The **Upstream Gap** status counts reported upstream transfer gaps.
  **Viewer Gap** counts records dropped by this socket's own queue.
  **Unknown** marks an unobserved range after an upstream termination or an
  unconfirmed handoff. The viewer retries while a subscription remains active.
## Opening MCAP recordings

Open a closed native `dot-logger/standard/2` recording directly in the viewer app.
New dot-logger files place registrations before message chunks for compatibility
with Lichtblick's indexed playback.

Some earlier `standard/2` files include a registration-only chunk. If Lichtblick
reports a playback range starting at 1970 and fails to preload messages, use the
existing CLI recovery command to write a separate compatible copy:

```sh
cargo run --locked -p dot-logger-cli -- recover \
    --manifest .logger/session.json --output .logger/compatible.mcap --json
```

For a manifest with multiple sources, supply a new output directory. Open the
recovered file in Lichtblick. The command reads the source files read-only and
reports each verified prefix. Remote Query/Replay remains under TASK-031 S3.
