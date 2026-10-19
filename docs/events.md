# OpenCode Event Protocol

> **Document role:** canonical owner for the normalized runtime event protocol, renderer handling inventory, and event-specific semantics.

The main process subscribes to the OpenCode V2 SSE stream
(`client.event.subscribe()`) through a transport pipeline
(`src/main/stream-pipeline.ts`) and forwards **every** event to the renderer
as `{ kind: "event", type, data }` (`BackendMessage`). The renderer dispatch
lives in `src/renderer/src/store.tsx` (the `onMessage` effect).

Session events are routed by `data.sessionID` into a per-session transcript
and busy-state store, so every open panel streams independently: child and
subagent streams keep flowing while the user works in another panel, and
model/agent selections land on the panel that owns the session.

OpenCode is the only event producer registered by the production app. The
dormant DeepSeek adapter source still contains a mapping for its mux and host
WebSocket records into this vocabulary, but the backend does not register or
start it. Prompt submission never drives a history polling loop, so OpenCode's
pushed events are the live authority.

Failure records use the shared `{code, message, details?}` shape from
`src/shared/errors.ts`. Missing provider/runtime codes receive a stable `VELA_*`
code, and the renderer includes the code in the persistent timeline error and
toast. Prompt IPC rejection, runtime execution failure, stream failure, malformed
events, and event-delivery failure all produce a report instead of being silently
dropped.

Incoming events use the same scheduling strategy as OpenChamber: the main
process queues events per directory and flushes one batch per 33ms frame.
Deltas for the same part (`session.text.delta`, `session.reasoning.delta`,
`session.tool.input.delta`, `session.compaction.delta`, `message.part.delta`)
are concatenated while they share a coalescing key; an authoritative snapshot
(`session.text.ended`, `session.reasoning.ended`, `session.tool.input.ended`,
terminal tool events, `message.part.updated`) clears that part's delta key so
a delta arriving after the snapshot never merges into a pre-snapshot delta the
snapshot already covers. `session.idle` / `session.error` /
`session.created` / `session.deleted` clear the `session.status` coalescing
key. A 30s heartbeat aborts a silent stream and reconnects immediately without
emitting a user-facing error; stream failures retry with exponential backoff
(250ms base, ×2, 5s cap). A subscription counts as connected only once it
delivers an event, so an endpoint that never answers keeps accumulating
failures and reports one user-facing `global.error` per outage instead of one
per retry cycle. After three consecutive failed attempts the backend drops its
client and the next attempt re-runs `connect()`, which discovers or ensures a
live service, so a dead or replaced daemon recovers without an app restart. A
stream that delivered events emits a synthetic `server.connected` when it
reconnects so the renderer re-materializes open sessions. The pipeline also emits `server.connected`
whenever the SSE stream ends cleanly (`onStreamEnd`) or the heartbeat aborts a
silent stream, so a stream that goes quiet mid-response (server-side stall
after a large response) still triggers a full re-materialization that recovers
any terminal events (e.g. `session.execution.succeeded`) missed in the gap.

The renderer keeps an authoritative per-session chat store
(`src/renderer/src/chat-store.ts`): server messages and parts in binary-search
ordered maps keyed by message/part id, plus a session status map. Events
mutate the store; the visible transcript is a projection of it. Full part
snapshots replace streaming state with dedupe bookkeeping
(`__dedupeNextDeltaFields`) so a trailing delta already included in the
snapshot is not applied twice, and a finished tool card is never regressed by
a stale running snapshot. When the store detects an incomplete session
snapshot (an orphan delta, a delta for an unknown part, or a part whose
owning message is missing) it materializes that session over
`shell:session-transcript` and merges the authoritative history without
regressing longer live text.

## Events the renderer handles

| Event type | What the store does |
|---|---|
| `session.created` | Adds or reconciles the session graph entry, including `parentID`, agent, title, and directory so task calls can resolve child sessions |
| `session.renamed` | Updates the matching session title and timestamp |
| `session.deleted` | Removes the session from the graph |
| `session.inbox.enqueued` | Buffers user/synthetic inbox items as an internal `pending-input` keyed by `inboxID`; user entries also join the session's queued-chip list; it does not create a visible chat row |
| `session.inbox.delivered` | Materializes the buffered input as the canonical user/synthetic timeline entry, reconciles an optimistic local user message by text, and drops the entry from the queued-chip list |
| `session.inbox.cancelled` | Discards the buffered input by `inboxID` and drops the entry from the queued-chip list without removing a delivered chat message |
| `form.created` | Normalizes the incoming form and shows it as a dock card above the composer for the addressed session; a form main attributes to the panel's location (see below) is shown for the owning panels while retaining the form's true session id for replies |
| `form.replied` | Removes the answered form's dock card |
| `form.cancelled` | Removes the cancelled form's dock card |
| `session.execution.started` | Authoritatively marks both the chat session and composer busy; activity is shown by the agent header rather than a transcript status bubble |
| `session.execution.succeeded` | Authoritatively marks the chat session and composer idle, completes the active assistant, and clears retry state without adding transcript noise |
| `session.execution.failed` | Authoritatively marks the chat session non-busy with an error state, completes the active assistant, clears retry state, and adds an error status line |
| `session.execution.interrupted` | Authoritatively marks the chat session non-busy with an error state, completes the active assistant, clears retry state, and adds an error status line |
| `session.idle` | Sets `busy = false` and completes the active assistant |
| `session.error` | Records the structured failure on the active assistant, marks the chat session errored, sets `busy = false`, and adds a persistent error status line so a failed run cannot disappear or leave the composer stuck on the running/stop icon |
| `global.error` | Shows a structured background/transport failure in the error toast when no session can be safely attached |
| `session.status` | Mirrors OpenCode `busy` / `idle` / `retry` / `error`; an `error` status clears `busy` (and the chat store records a non-busy `error` status); retry details attach to the latest assistant. A retry carrying a `free_tier_limit` or `account_rate_limit` action appends an error `status` transcript item (`buildRateLimitNotice` in `src/renderer/src/chat-store.ts`) on the first attempt so the user sees the rate-limit reason and resolution link inline |
| `session.step.started` | Creates or reopens the addressed assistant message with its agent, model, and server start time in the authoritative chat store, marks the chat session busy, clears its retry/error state, and completes a different unfinished assistant |
| `session.step.ended` | Completes the addressed assistant message while preserving its start time and selection metadata; a terminal finish marks the chat session idle while `tool-calls` keeps the multi-step turn active |
| `session.step.failed` | Completes the assistant, records the structured failure, and marks the session errored so a failed step cannot remain busy |
| `session.text.started` | Adds one ordered text part for the message/ordinal |
| `session.text.delta` | Appends streamed text to that part (materializing the session if the message or part is unknown) |
| `session.text.ended` | Replaces the part with the authoritative final text, preserves its OpenCode phase, marks it complete, and dedupes trailing deltas; `commentary` projects into the turn's stable Think row while `final_answer` remains assistant body text |
| `session.reasoning.started` | Adds one ordered reasoning part behind a collapsed Thinking disclosure in event order |
| `session.reasoning.delta` | Appends native streamed reasoning to the active collapsed Think row without manufacturing intermediate characters when a runtime sends a one-shot summary |
| `session.reasoning.ended` | Replaces the part with authoritative final reasoning and keeps it available through the disclosure after completion |
| `session.tool.input.started` | Adds an inline tool part with its real name and begins the argument buffer |
| `session.tool.input.delta` | Appends to the live tool argument buffer |
| `session.tool.input.ended` | Replaces the argument buffer with authoritative input text |
| `session.tool.called` | Applies parsed input, detail, clickable paths, and start time without regressing a terminal card |
| `session.tool.progress` | Displays current tool progress metadata |
| `session.tool.success` | Marks a running tool successful, reads V2 content blocks or legacy output, and records duration |
| `session.tool.failed` | Marks a running tool failed, preserves content/error output, records duration, and auto-expands the card |
| `session.retry.scheduled` | Marks the session retrying and attaches attempt, structured error, and next-attempt time to the assistant |
| `session.synthetic` | Retains the synthetic/system message; internal `<system-reminder>` content is filtered from the visible timeline |
| `session.skill.activated` | Adds the activated skill name, id, and supplied text to the timeline |
| `session.shell.started` | Adds a running shell message with its command |
| `session.shell.ended` | Reconciles the shell by id with status, exit code, and captured output |
| `session.compaction.started` | Adds a live compaction entry and its reason/recent context |
| `session.compaction.delta` | Streams the compaction summary into the active compaction entry |
| `session.compaction.ended` | Finalizes the authoritative compaction summary |
| `session.compaction.failed` | Finalizes compaction with its structured error |
| `message.updated` | Upserts the authoritative assistant message (skipped when unchanged) with its completion/error state; terminal assistant finishes (`stop`, `length`, content filter, error, or unknown) authoritatively mark the chat session non-busy even if `session.idle` is delayed or absent |
| `message.removed` | Removes the assistant message and its parts from the authoritative store |
| `message.part.updated` | Authoritatively reconciles an ordered text, reasoning, or tool part; tool snapshots use `callID` to merge with their live row while retaining fields omitted by the snapshot, and missing owning messages request materialization |
| `message.part.delta` | Appends a text/input/output field delta; orphan and missing-part deltas trigger materialization |
| `message.part.removed` | Removes the part from the authoritative store |
| `server.connected` | Re-materializes every open session from the authoritative message history (emitted by the server and synthetically by main after a stream reconnect) |
| `global.disposed` | Same full re-materialization after the server reports its stream was disposed |
| `session.model.selected` | Retains internal model-switch metadata and updates `currentModel` for the addressed session's panel; it does not create a chat row |
| `session.agent.selected` | Retains internal agent-switch metadata and updates `currentAgent` for the addressed session's panel; it does not create a chat row |
| `agent.updated` | Refetches the agent catalog for the addressed session so the composer agent picker recovers from a lazy or empty first load |
| `catalog.updated` | Refetches the model catalog for the addressed session so the composer model picker stays current |
| `models-dev.refreshed` | Legacy model-catalog change event; handled identically to `catalog.updated` |
| `session.usage.updated` | Records cumulative session token usage (`tokens`) and cost (`cost`) for the addressed session; drives the token-usage popup in the agent header (context-window utilization is computed renderer-side from `tokens.input` vs the active model's `limit.context` delivered by `shell:models`) |
| `session.usage.recorded` | Same usage snapshot as `session.usage.updated` on the durable legacy stream; handled identically |
| `todo.updated` | Replaces the addressed session's todo list rendered in the dock above its composer; `todowrite` tool-part input/metadata is also consumed as a beta-protocol fallback |
| `permission.asked` | Appends a permission card (`action`, `resources`, pending=true); periodic reconciliation confirms pending requests and resolves cards whose requests vanish (for example, when answered in an attached TUI) |
| `permission.replied` | Marks `data.requestID` resolved, recording `resolvedWith` from `data.reply` |

## Events forwarded but NOT handled

The store has no case for these; they arrive on the wire and are dropped
by the switch statement. Revisit when adding features:

- `session.moved`, `session.forked`
- `session.inbox.delivery.changed`
- `session.compaction.admitted`, `session.revert.*`
- `filesystem.changed`, `reference.updated` (the main process handles the
  filesystem event separately; see below)
- `project.*`, `plugin.*`, `command.*`, `skill.*`, `mcp.*`, `vcs.*`,
  `websearch.*`, `pty.*`, `tui.*`, `config.*`

The main process accepts both `type` and the legacy SSE `event` field when it
forwards an event. The renderer accepts current `data` and legacy `properties`
envelopes, plus `id` and `callID` tool identifiers. `permission.v2.*` names are
adapted to `permission.*`. This lets the same reducer work with the installed
OpenCode client, the latest upstream protocol, and older opencode2 services.

## Main-process event handling

The transport pipeline in `src/main/stream-pipeline.ts` (see the scheduling
paragraph above) delivers per-directory batches into `deliverEvents` in
`src/main/opencode.ts`, which forwards each event and then runs
`handleServerEvent`. `deliverEvents` drops interactive prompts (`form.*`,
`permission.*`, `session.inbox.*`) for sessions Vela never opened: the
global daemon is shared with external `opencode` terminal sessions, and
forwarding those prompts would misattribute them to the focused panel
(`form.created` carries its session only inside `data.form`, so
`eventSessionID` also descends into `form`). Forms are the exception: when a
form's owner is not an open context (a delegated child, a location-global
form, or an external `opencode` TUI sharing the directory), main canonicalizes
the event location, attaches the open session ids for that location as
`velaSessionIDs`, and forwards it only when Vela owns that location. The
renderer routes the request to the panels open there while preserving the
form's true session id, so the prompt is answerable in the GUI and the reply
reaches the owning session. Forms from sessions outside every open workspace
are dropped. Permissions and inbox items keep the strict owner check.
Child/subagent transcript streams still flow; the renderer keeps them in
separate stored state.
`handleServerEvent` intercepts two types after forwarding:

- `session.tool.called` → `snapshotInputs(context, input)` snapshots structured
  file paths for the session context addressed by the event's `sessionID`
  before execution when possible. Shell command strings are not parsed
  as paths.
- `filesystem.changed` → `onFsChanged(file)` for every open session context
  whose directory matches the event's top-level `location.directory`; a shared
  directory fans the event out to each of its contexts. Global events from
  directories without an open session are ignored.

## Tool-card data flow (trace)

```
input.started {id/callID, name} ─► ordered tool part + empty input
input.delta   {id/callID, delta}─► tool.input += delta
input.ended   {id/callID, text} ─► authoritative tool.input
called        {id/callID, input}─► detail + paths + parsed input
progress      {id/callID, data} ─► live progress metadata
success/failed{id/callID, ...}  ─► terminal status + output + duration
```

Every update addresses the assistant message plus the tool call. A missing
part is created so a late subscription can recover, while a success/failed
part is terminal and cannot regress when an older running event arrives.

Tool parts retain the protocol's parsed input, content blocks, metadata,
`executed` flag, provider state, and provider-result state. Text content drives
the expandable output and file content becomes an attachment link. Edit and
patch parts additionally carry `metadata.files[]` (`file`, `patch`, `status`,
`additions`, `deletions`); when present the timeline renders them as a dedicated
diff card instead of a generic tool trigger (see `EditToolCard` in
`OpenCodeTimeline.tsx`).

## Parent and child sessions

The global SSE subscription includes every child session. The renderer stores
those streams separately rather than mixing them into the parent transcript.
Task cards resolve a child from `metadata.sessionId`/`sessionID`, then fall back
to the newest session whose `parentID`, title/description, and agent match the
task input. Opening the card replays and activates that child; its header links
back to the parent. Permission replies carry the owning session id so a child
request is never sent to the parent session.

## App-owned Sass notifications

`kind: "sass-watch"` messages carry `SassWatchState` for an immutable workspace
identity. Footer and terminal-tray hooks consume them; the chat reducer ignores
them. These are local app notifications, not OpenCode runtime events. See
[the Sass service](sass.md) for ownership and lifecycle.

`ui-command` notifications `reload-app-styles` and `app-styles-error` are
consumed by the root app-style hook rather than the chat reducer. They refresh
Vela's own compiled stylesheet or report compilation failure; see the
[application style owner](main.md). They are independent of workspace Sass state.
