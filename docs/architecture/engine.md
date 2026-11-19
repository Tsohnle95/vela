# Application engine (`vela-engine`)

**Owns:** application orchestration and durable state away from the render
thread: engine commands, session store, warm-session lifecycle, OpenCode
connection/reconnect, persistence, filesystem access, workspace observation,
recoverable writes, Git status/diffs, changes review, terminal relay, Sass
watch/compile, preview servers, runtime management, provider OAuth/usage,
message queue, forms, permissions, and shutdown.

**Does not own:** GPUI views/focus/layout, the OpenCode wire protocol itself,
terminal escape interpretation, WebView hosting, or transcript block algorithms.
Those belong to `vela-ui`, `vela-opencode`, `vela-terminal`, `vela-browser`, and
`vela-core` respectively.

## Entry points

| Entry point | Responsibility |
|---|---|
| `Engine::spawn(EngineConfig, runtime)` | starts the engine command loop on Tokio |
| `Engine::subscribe()` | returns a broadcast receiver for `EngineUpdate` |
| `Engine` getters | pull session summaries, views, permissions, todos, usage, settings, connection, warm diagnostics |
| `Engine` action methods | enqueue protocol/file/terminal/settings commands |
| `Engine::shutdown_and_wait()` | cancel workers, kill PTYs, stop own child processes, wait for loop exit |
| `Store`, `SessionView` | state readable by the UI under short locks |

`Engine` is a cloneable handle over `Arc<Inner>`. `Inner` owns the settings
snapshot, store, broadcast sender, command sender, cancellation tokens,
observation pool, PTY map, Sass watches, forms, validation cancellations, and
preview process ownership.

## Command and update contract

The UI sends commands through an unbounded channel. Commands are small owned
values or request IDs; they must not carry render-thread borrows. State changes
are announced through `EngineUpdate`; full snapshots are pulled separately, but
some updates intentionally carry targeted payloads (terminal bytes, file
content, directory entries). Representative updates include:

- connection, sessions, session transcript, permissions, todos, selection;
- workspace files, directory/file/recovery/rename/create/remove/detach/import;
- Git status/diff and observed-change review;
- terminal data/exit and preview/runtime/Sass state;
- catalog, providers, OAuth, usage, integrations, commands, references;
- inbox/queue, forms, revert, validation, settings, and errors.

The UI may pull immutable snapshots after an update. Treat `EngineUpdate` as a
notification/invalidation signal plus any targeted payload it explicitly
contains, not as a guarantee that it carries every later change.

**Invariants:**

- No new or blocking network, disk, PTY, compiler, or process work on the GPUI
  thread. The UI persistence exceptions documented in [ui.md](ui.md) are debt.
- No store lock held across an await.
- Per-session transcript/draft/scroll state stays in that session's
  `SessionView`; a background session must not invalidate unrelated views.
- Stale async work must carry identity (session ID, request ID, document key,
  observation generation, or load generation) and must be rejected after
  deletion/replacement.
- A retired session ID is final for in-flight replay/inventory.

## Session and warm-cache lifecycle

1. `open_session`/`open_directory` resolves a summary and creates or promotes a
   `SessionView`.
2. History loads in a generation-scoped task; stale `Loaded` results are
   rejected.
3. Replay is projected by `vela-core::transcript`; live SSE deltas mutate the
   same transcript.
4. Stream events publish on a 40 ms cadence. The due tick precedes socket reads,
   so continuous input cannot starve publication. Normalized records persist
   before reduction. Reconciliation and shutdown also drain pending events.
5. Closing a session saves draft/scroll, moves it to the warm cache, and keeps
   background updates on the retained transcript instead of reopening UI.
6. The warm cache is bounded by session count and bytes. The separate live
   view limit reserves room for all eight desktop panels even with the default
   six-session warm setting. The focused view is never selected for eviction.
7. Deletion retires the ID, clears views/warm/permissions/todos/usage, and
   acknowledges success or a retryable error.

`SessionView::presentation` returns one immutable `SessionPresentation` with
blocks, phase, and prepared Markdown from the same revision. Preparation
serializes per session, parses outside the transcript and global store locks,
and publishes only if the captured revision still matches. Session updates
notify after publication; a lagged transcript subscriber reads the latest
snapshot and authoritative retirement state immediately. This does not recover
arbitrary lost events such as terminal output.

Reconnect emits a synthetic `server.connected`; the engine re-discovers the
service, swaps in a fresh authenticated client/event stream, and rehydrates
sessions, permissions, and open views. While an authoritative history request
is in flight, live events continue publishing and the engine keeps a bounded
replay journal. On success it installs history and replays newer events in
arrival order, skipping prose already settled by history and matching
overlapping append-only text prefixes. OpenCode history and SSE have no shared
cursor, so overlapping replacements cannot be ordered by source revision.
On fetch failure or journal overflow the engine keeps the live transcript.
A live todo update takes precedence over todos recovered from the history.

## Persistence and migration

Normalized reducer inputs append to private native-profile
`stream-journal/<instance>/<session-hash>.jsonl` files on Tokio blocking workers.
They contain conversation/tool contents and are user data, never stderr logs.
Unix files/directories use 0600/0700 permissions; symlink files are refused.
An ordered OS file append is acknowledged before reduction. Full-cache file/
directory barriers run at normal shutdown rather than blocking every stream
frame; power loss can lose the unsynchronized tail. Reconciled history,
local prompts, generated notices and quiet settlement are recorded too. Failed
appends pause stream reduction/publication and report a restart-needed error.
Session deletion removes its journals across instances and ingestion metadata.

OpenCode remains authoritative at startup/reconnect; native logs do not replace
its history protocol. Explicit offline `stream_journal::replay_file` processes
one record at a time (32 MiB record limit); diagnostic `read_records` also caps
retained input at 256 MiB. Version, sequence and truncated-tail errors fail
explicitly. Logs persist until session deletion, so retained streamed sessions
consume growing disk space. They are not automatically imported or printed.

- Native profile: platform data directory + `vela-native`; override with
  `VELA_DATA_DIR`.
- `settings.json` is schema-versioned (v1), normalized on load, written through
  a temporary file + rename. Corrupt or future-version files fall back to
  defaults; loading never rewrites them.
- `window-bounds.json` stores only session width/height (v2 wrapper accepted),
  excludes position/maximized/fullscreen, and is written by the serialized
  engine loop.
- The legacy Electron profile is read-only input. Startup may import workspace
  bookmarks/locations, window size, Sass preferences, and renderer-localStorage
  pins once. Imports are idempotent and never overwrite native files.
- OpenCode owns conversations, credentials, and service registration. Vela does
  not copy them into its profile.

## Workspace files, observation, and recovery

`files.rs` is the workspace confinement boundary: session-relative
`/`-separated paths, canonicalized roots, traversal/absolute/backslash/NUL
rejection, symlink-target rejection, bounded reads/writes (8 MiB), and
blocking-worker I/O.

External single-file tabs are an explicit exception. `read_standalone` and
`write_standalone` accept absolute paths, enforce the same size/atomic-write
rules, and deliberately do **not** use workspace recovery because a recovery
store must not be created in an arbitrary external parent.

`observation.rs` owns one recursive watcher and one serialized read/write worker
per attached workspace:

- multiple sessions share the physical-root owner until the last detaches;
- notifications enqueue a bounded path set with a 200 ms debounce; overflow
  requests a non-starving rescan;
- watched-text limit is 2 MiB, while explicit editor reads remain 8 MiB;
- dependency/recovery directories and symlinks are skipped;
- own saves emit an exact-snapshot echo; a following external write remains an
  external update;
- observation generations reject detached workers; UI snapshot reads never hold
  a filesystem-I/O lock.

`recovery.rs` implements compare-and-write transactions with durable version-1
manifests, held originals, atomic installation, and interrupted-transaction
reconciliation. The UI begins document relocation/removal reservations in
`vela-core::document::Documents` before asking the engine to rename or remove a
path. Import sends its bounded engine command directly without a document
reservation. Detach and Trash never fall back to permanent deletion.

`changes.rs` owns context-scoped observed-change review. Clearing a review does
not edit a file, change a baseline, or clear sibling contexts.

## Git

Read-only status and unified diffs only. Commands run directly with `git -C`
(no shell), paths are validated as workspace-relative, and output is bounded.
Staging/commit/branch mutation is deliberately out of scope until it can reuse
Vela's recovery and document-ownership guarantees.

## Terminal relay

The engine owns PTY child processes and routes input/resize/stop commands.
`terminal_relay.rs` preserves byte order while coalescing bursts:

- 16 ms window, 64-chunk cap, 64 KiB byte target;
- bounded 256-chunk PTY channel;
- final output is ordered before exit; a 500 ms post-exit deadline is not
  starved by continuous output;
- shutdown kills all PTYs.

Escape interpretation and selection live in `vela-terminal`/`vela-ui`; the
engine only relays bytes and lifecycle.

## Sass, previews, and runtime

- Sass watches are root-scoped: contexts sharing a canonical root share one
  worker/log; other roots run independently. Preferences survive restart.
- Coalesced filesystem events trigger source/partial/config inventory and the
  pinned bundled Dart Sass compiler in a cancellable child process. Rust plans
  outputs from reference conventions without executing package scripts, then
  validates and atomically installs CSS/source maps.
- Last-owner close cancels the worker; retired generations cannot write or
  publish.
- Preview servers are app-owned child processes with readiness checks,
  cancellation, and cleanup on workspace close/app shutdown.
- Runtime management discovers the installed CLI and exposes capabilities used
  to gate provider, permission, steering, attachment, command, and TUI
  controls. Fixture/double behavior is test-only.

## Providers, queue, forms, and permissions

- Provider OAuth is engine-owned. One cancellable lease owns an attempt,
  including late start replies; polling/code submission cannot retarget to
  another workspace.
- Provider usage reads the short-lived bridge snapshot, never OAuth tokens, and
  enforces the 15-minute age cap.
- The local queue is durable and scoped by workspace + session; unversioned
  legacy queues are quarantined and never replayed; runtime ACKs remove local
  retries.
- Forms own pending answers across periodic refreshes; session and
  location-global forms reply with the correct session/directory identity.
- Permissions are per session. Answering a prompt activates the owning panel;
  Approve mode may answer all open panels but not closed warm sessions.

## Failure, cancellation, and shutdown

| Failure | Expected behavior |
|---|---|
| OpenCode disconnect | connection state, backoff, re-discovery, rehydrate |
| Service moves port | discover new registration; replace client/stream |
| Session load races delete | generation/retired ID rejects stale result |
| Workspace detached | observation generation retires worker |
| Save conflict | pause saves, retain local text, expose reload/merge/overwrite |
| Recovery interrupted | reconcile held source/manifests on initialization |
| Preview/Sass child fails | scoped error update; no render-thread blocking |
| App shutdown | cancel token, stop own children, kill PTYs, wait, then Tokio shutdown |

## Tests

```sh
cargo test -p vela-engine
cargo test -p vela-engine --test engine_flow
```

`engine_flow.rs` is the main integration fixture: service discovery, replay,
SSE, prompts, permissions, forms, queue, provider catalog/OAuth, reconnect after
a port move, settings handoff, file/workspace flows, and shutdown. Inline tests
cover atomic settings, migration, recovery, observation, queue migration, file
confinement, Git bounds, preview lifecycle, provider usage, Sass planning, and
terminal coalescing.

## Cross-boundary change paths

| Change | Trace |
|---|---|
| Prompt/queue | UI composer/inbox → engine command → OpenCode client → SSE → core projection → `EngineUpdate` → transcript |
| File save/conflict | editor view → `Documents` → engine file command → recovery/observation → `FileSaved`/conflict → editor |
| Settings | UI settings → `apply_settings` → normalized snapshot + atomic save → `SettingsApplied` |
| Provider connect/OAuth | UI form → engine lease → OpenCode client → update → provider UI |
| Terminal | UI key/selection → engine → PTY → relay → emulator → UI |
| Workspace rename/remove | explorer UI → document reservation → engine filesystem transaction → `EntryRenamed`/`EntryRemoved` → documents/explorer |

## Neighboring owners

- Domain types and projection: [core.md](core.md)
- Protocol client/stream: [opencode.md](opencode.md)
- UI state and rendering: [ui.md](ui.md)
- Editor/document flows: [editor.md](editor.md)
- Operational setup and fixtures: [development.md](../development.md)
