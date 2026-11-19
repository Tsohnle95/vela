# OpenCode adapter (`vela-opencode`)

**Owns:** discovery and lifecycle of the shared OpenCode V2 service, the
authenticated HTTP client (normally loopback), the reconnecting SSE transport,
and typed parsing for sessions, messages, permissions, models, agents, commands,
forms, inventory, providers, and usage.

**Does not own:** application state, warm caches, transcript projection, file
I/O, UI rendering, or credential persistence. It depends on `vela-core` for
shared domain types, not on `vela-engine` or `vela-ui`.

## Service discovery and lifecycle

- Registration defaults to `$XDG_STATE_HOME/opencode/service.json`, falling
  back to `~/.local/state/opencode/service.json`. `VELA_OPENCODE_SERVICE_FILE`
  overrides the path; `VELA_OPENCODE_BINARY` overrides the executable.
- `discover_at` reads the registration, probes `GET /api/info`, and requires
  the live service to report V2. A missing registration version is accepted;
  when the registration supplies a version or PID, those values must match the
  probe. A stale or incompatible registration is rejected.
- The registration URL and password are trusted local state. The adapter does
  not independently require the URL to be loopback, so do not point a service
  file at an untrusted host.
- `ensure_at_with_program` discovers first; only when no compatible service is
  registered does it spawn `opencode serve --service` under the managed
  `XDG_STATE_HOME` and wait for registration.
- The service is shared with the CLI and is not owned by a Vela window. Vela
  must not kill or rewrite a service it does not own.
- `managed_profile` rejects custom registration locations it cannot safely
  manage; it never copies or rewrites a registration just to make a connection
  succeed.

Basic-auth credentials are derived from the registration and attached by
`Client`. They stay at this boundary and must not appear in logs, errors,
settings, or UI state.

## Client contract

`Client` normalizes the endpoint to a base URL with a trailing slash and builds
paths by segment, so IDs are URL-encoded rather than string-concatenated. It
uses a short connect timeout and a 120-second request timeout. Non-success
responses become `OpencodeError::Status` with a bounded body preview.

The client covers these protocol families:

| Family | Representative operations |
|---|---|
| Service/session | info, list/create/get/delete session, prompt, interrupt, compact, command |
| History | paged messages, replay ordering, continuation cursors |
| Permissions | list pending, reply allow/always/deny |
| Inbox/queue | list, steer, cancel |
| Models/agents | catalog, selected/default model, switch model/agent, session usage |
| Integrations | provider catalog, key connect/remove, OAuth start/poll/complete/cancel, MCP/plugins/skills |
| Forms | list session/location forms, reply, cancel |
| Discovery | file search, command catalog, skill activation |
| Revert | stage, commit, clear |

Response envelopes are intentionally tolerant: current `data` envelopes, raw
arrays, and legacy field spellings are accepted where the runtime has emitted
them. Unknown fields are ignored; malformed required fields become a protocol
error rather than a panic. `load_messages` follows continuation pages up to a
hard cap of 200 pages of 200 messages. If a continuation cursor remains after
the cap, the current implementation returns the partial history; the cap is a
known limitation, not a completeness guarantee.

When adding a protocol call, keep request construction, response parsing, error
mapping, and a parser/fixture test together. Do not leak raw protocol `Value`s
into the engine unless the engine explicitly owns normalization (for example,
replay payloads projected by `vela-core`).

## SSE transport

`spawn_event_stream` returns a bounded `mpsc::Receiver<Value>` with capacity
2,048. Backpressure reaches the socket instead of growing memory without bound.

- Heartbeat timeout: 30 seconds.
- Reconnect: 250 ms exponential backoff, capped at 5 seconds.
- On a reconnect after the first connection, a synthetic
  `server.connected` is emitted so the engine re-materializes open sessions.
- When an opened stream fails, a synthetic `server.disconnected` is emitted.
- Frames are decoded as complete UTF-8 lines, never transport chunks. CR, LF,
  and CRLF may straddle chunks; comments and non-`data` fields are ignored;
  multiple `data` lines are joined; `[DONE]` is skipped. An explicit frame `id:`
  is retained as reserved `_velaEventId` metadata for normalization. Inherited
  last-event IDs are not assigned to later id-less deltas. Payload message/tool
  IDs are never used as token event identities.
- Invalid JSON frames are logged and skipped rather than terminating the
  stream.

Event-name tolerance and normalization belong to `vela-core::event`; this crate
only guarantees transport framing and JSON delivery.

## Failure behavior

| Failure | Owner behavior |
|---|---|
| Missing/invalid registration | discovery returns `None`; engine may attempt `ensure` |
| Service starts but never registers | bounded wait returns a service error |
| Version < 2 or PID mismatch | treated as incompatible; do not attach |
| HTTP non-success | typed status error with bounded body |
| SSE disconnect | synthetic disconnect, backoff, reconnect, synthetic connect |
| Malformed SSE frame | skip frame; keep the stream alive |
| Cancellation | token stops the reader and closes the channel |

Do not add an unbounded retry loop, a second service process, or a silent
fallback to a real provider in tests. Provider/runtime fixtures are the
supported test path; see [development.md](../development.md).

## Tests

```sh
cargo test -p vela-opencode
```

Inline tests cover service version gating, registration parsing, client URL and
response parsing, SSE framing/UTF-8 splitting, provider catalogs, form/command
parsing, and inventory parsing. The engine integration fixture in
`crates/vela-engine/tests/engine_flow.rs` exercises discovery, replay, prompt,
permissions, forms, queue, provider, reconnect, and shutdown paths end to end.

## Neighboring owners

- Event normalization and transcript projection: [core.md](core.md)
- Runtime state, reconnect, and command routing: [engine.md](engine.md)
- Provider/settings surfaces: [ui.md](ui.md)
