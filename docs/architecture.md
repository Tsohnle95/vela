# Vela Native architecture

> **Status:** Stage 1 foundation plus the first Stage 2 vertical slice are in
> place. Parity work continues; this document separates what exists from what is
> designed but not yet built.

## 1. Goals and non-goals

Vela Native is a GPUI desktop application that preserves Vela's product
behavior while removing the Electron/React runtime from the presentation path.
The primary engineering targets are:

- warm session switching: p95 under 50 ms to the first visually correct frame
  on the reference machine;
- transcript cost proportional to visible rows and changed blocks, not to
  conversation length;
- bounded warm-session memory with immediate visual restoration;
- low idle CPU: no render loop when nothing is streaming or animating;
- scoped invalidation: a background session must not repaint the focused panel;
- native terminals, editor, git surfaces, and an embedded platform web view.

It is not a generic chat client and it is not a visual redesign. Product tokens
and workflows come from the original Vela interface
(`src/renderer/src/styles/_foundation.scss`).

## 2. Workspace layout

```
vela-native/
  apps/vela/              # binary: runtime, settings, window boot
  crates/vela-core/       # domain, events, markdown blocks, layout, warm cache
  crates/vela-opencode/   # native OpenCode V2 service + HTTP + SSE adapter
  crates/vela-engine/     # tokio orchestration, store, permissions, settings
  crates/vela-ui/         # GPUI window, sidebar, transcript, composer
  docs/                   # architecture, migration and performance records
```

Dependencies point inward: `vela-ui -> vela-engine -> vela-opencode ->
vela-core`. `vela-core` has no GPUI, network, or platform dependency.

### Data flow

```
OpenCode shared service
   │  HTTP + basic auth                   SSE /api/event
   ▼                                            │
vela-opencode Client ── requests ──┐            │ reconnecting, bounded channel
                                  ▼            ▼
                            vela-engine run loop
                            ├─ 16 ms batch + critical flush
                            ├─ normalize events (tolerant envelopes)
                            ├─ scoped transcript mutation
                            ├─ permissions / todos / usage
                            └─ broadcast EngineUpdate (no payload copies)
                                       │
                                       ▼
                            GPUI VelaApp (main thread)
                            ├─ reads Arc<SessionView>
                            ├─ block list + ListState splice
                            └─ paints visible rows only
```

The render thread never awaits HTTP, disk, PTY, or database work. It reads
short-lived locks over already-projected state and requests work through an
unbounded command channel.

## 3. Threading and state ownership

- **GPUI main thread** owns views, focus, layout, input, and the virtualized
  lists. `VelaApp` is a single GPUI entity.
- **Tokio runtime** (four workers) owns connection establishment, HTTP calls,
  SSE reads, transcript projection, and persistence.
- **Store ownership** is split by aggregate: `Store` holds the session index,
  per-session `Arc<SessionView>` handles, permissions, todos, usage, warm cache,
  and protocol-drift counters. Each `SessionView` has its own transcript lock,
  so a streaming background session does not lock unrelated views.
- **Updates** are notifications (`EngineUpdate::Session(id)`), not state
  snapshots. The UI pulls the latest projected blocks only when its active
  session changed, which coalesces bursts naturally.
- Locks are `std::sync::RwLock` and are never held across an await. Critical
  sections copy/clone only `Arc`-backed blocks or small vectors.

This avoids a single globally invalidated React-style store: a background
session mutates only its own transcript; the focused panel repaints only when
its revision changes.

## 4. Transcript projection

### Blocks, not messages

`vela-core::transcript` projects each message part into renderable blocks:

- user message -> one bubble row;
- assistant text -> top-level Markdown blocks (paragraph, heading, fence,
  list);
- reasoning -> one collapsed row;
- tool call -> one structured card row;
- shell / compaction / system / error -> notice rows.

Block IDs are stable: `{part_id}#{block_index}` for text, `{part_id}` for tool
and reasoning rows. When a part grows, the blocks before its last block are
reused if their byte ranges still match the new text; only the last block is
reparsed. Completed blocks therefore keep identity and measured height.

Blocks hold `Arc<str>` text. Cloning a block list for the UI is O(rows) pointer
copies, not O(characters) string copies.

### Incremental Markdown

`markdown::split_ranges` tracks code fences, blank lines, and ATX headings. It
never splits inside a fence and tolerates an unclosed fence while streaming.
The transcript's per-part cache stores byte ranges and verifies reused ranges
against the new text; a snapshot replacement (text no longer a prefix) falls
back to a full parse of that part only.

Inline rendering currently paints the block text; inline emphasis/links and
syntax highlighting are planned. That is a presentation gap, not a parser
rewrite: the block boundaries and IDs do not change when inline styling lands.

### Layout cache

`LayoutCache` keeps measured heights keyed by block id plus a content
fingerprint. A changed block loses only its own height. Prefix offsets provide
O(log n) hit-testing and visible-range math. GPUI's `List` also measures rows;
`LayoutCache` is the deterministic, testable arithmetic used by benchmarks and
future custom scrolling.

## 5. Virtualized list behavior

`VelaApp` owns one `ListState` for the transcript and one for the session
sidebar. On updates it:

- appends with `splice(old..old, added)` rather than resetting the list;
- removes disappeared rows with `splice(new..old, 0)`;
- calls `remeasure()` only when the row count is unchanged and content changed;
- resets and restores a saved `ListOffset` when switching sessions.

Bottom alignment provides follow-tail behavior. A scroll handler records the
logical offset; drafts, offsets, and the follow flag are stored on
`SessionView` and survive warm eviction.

This means a 1,000-message transcript does not create 1,000 painted rows. The
list paints the visible window plus GPUI's bounded overdraw.

## 6. Warm-session navigation

`vela-core::warm::WarmCache` is an LRU bounded by both entry count (default 6)
and estimated retained bytes (default 96 MiB). An entry retains:

- the session summary;
- the projected transcript (including block caches);
- the unsent draft;
- scroll offset and follow-tail flag;
- model/agent selection.

The engine keeps up to `limit + 1` views open (the active one cannot be
evicted). Older views are moved to the warm cache. Reopening a warm session is
synchronous: the projection is promoted into a live view immediately, then
authoritative history is reconciled asynchronously. Background events for a
closed session update the retained warm transcript instead of silently
reopening a panel the user closed. Evicted sessions reload from OpenCode's
message history.

Navigation flow:

1. Click a session: save the outgoing draft and `ListOffset`.
2. Read `Arc<SessionView>` if present; otherwise promote a warm entry.
3. Reset the list to the retained blocks and restore the anchor.
4. The first frame paints retained content; the network is not on the path.

## 7. OpenCode V2 adapter

`vela-opencode` implements the V2 HTTP surface directly:

- `GET /api/session` (paged history, directory filter);
- `POST /api/session` with `location.directory`;
- `GET /api/session/{id}` and paged `GET /api/session/{id}/message`;
- `POST /api/session/{id}/prompt|interrupt`;
- `GET /api/event` (SSE);
- `GET /api/permission/request` and
  `POST /api/session/{id}/permission/{request}/reply`;
- `GET /api/model`, `GET /api/agent`, `GET /api/info`.

Service lifecycle mirrors Vela: discover the registered shared service
(`$XDG_STATE_HOME/opencode/service.json`, or
`VELA_OPENCODE_SERVICE_FILE`), probe `/api/info`, require major version 2, and
spawn `opencode serve --service` only when nothing compatible is registered.
Basic-auth credentials never leave the adapter.

SSE behavior:

- bounded channel (2,048) applies backpressure rather than growing memory;
- 30 s heartbeat; reconnect with 250 ms exponential backoff capped at 5 s;
- a synthetic `server.connected` after a reconnect triggers full
  re-materialization of open sessions;
- tolerant parsing accepts current `data`, legacy `properties`, and numbered
  event suffixes.

The engine flushes deltas on a 16 ms cadence and flushes critical events
(permissions, execution terminal states, idle, create/delete, reconnect)
immediately. This is intentionally finer than the original 33 ms queue because
it runs in-process without IPC serialization; it retains the same
coalescing/barrier principles.

## 7.1 Selection and settings

The model/agent catalog is loaded per active session through the nested
`location[directory]` query shape used by OpenCode. The engine owns the chosen
model/agent on the session view, sends `session/{id}/model` and
`session/{id}/agent`, and publishes a selection update. The UI renders native
catalog pickers in the composer and keeps the picker per session. Follow-ups
sent while a run is active use `delivery: "queue"`; optimistic rows are only
created for immediate prompts. Queued items are listed above the composer with
Steer and Cancel actions, refreshed from `session.inbox.*` events.

Vela Native has two render-only shells over the same state and engine commands.
**Codex** is the default: a 50px activity rail, 44px titlebar, one flush content
card, no top/left panel borders or gutters, a hairline-free sidebar/editor seam,
and a docked 325px agent panel. **Classic** is the original floating-card layout
with 10px gutters, rounded bordered panels, and the same components. The
Appearance page persists the choice as `"shell"` in `settings.json`; both
directions were verified in the running app. New surfaces must be validated in
Codex first and remain usable in Classic.

Sass watch belongs to the native engine. Contexts sharing a canonical root
share one worker and log; other roots run independently. Versioned root
preferences survive restart. Coalesced native filesystem events trigger source/partial/configuration
inventory on the blocking pool and the pinned bundled Dart Sass
compiler in a cancellable child process. The compiler helpers plan outputs
using the reference config conventions, without executing package scripts.
Rust validates and atomically installs CSS and relative source maps, then
publishes scoped logs to Sass Output. Last-owner close cancels the worker;
retired generations cannot write or publish. Full GUI/status/log-layout and
packaged-platform acceptance are tracked separately in the parity matrix.

The Appearance page also reports warm budget/residency, connection endpoint,
protocol drift, active workspace, and profile location. Other preferences
remain read-only and require a safe live-update/reload path.

## 8. Persistence

- Vela Native owns only its own profile:
  `~/Library/Application Support/vela-native/settings.json` on macOS.
- Writes are atomic (temp file + rename) and schema-versioned. Corrupt or
  future-version files are read as defaults and are never overwritten as a side
  effect of loading.
- OpenCode owns conversations, credentials, and service registration. Vela
  Native does not duplicate the transcript database.
- Migration reads the original profile's `saved-workspaces.json` and
  `window-bounds.json` and merges bookmarks into the native profile. It never
  writes to the Electron profile or OpenCode's database; the import is
  idempotent and verified by hash in the running app.

## 9. Security boundaries

- The engine talks only to the loopback OpenCode endpoint registered by
  OpenCode, with the service's basic credentials.
- No local HTTP server is exposed by Vela Native today. If a background engine
  or mobile endpoint is added, it must use authenticated loopback IPC and a
  documented lifecycle.
- GPUI has no web-content bridge. The embedded browser is a separate platform
  view with a nonpersistent profile and no access to engine commands;
  privileged bridge calls, if introduced for agent automation, require explicit
  per-origin authorization.
- Untrusted Markdown currently renders as text, so no HTML/script injection
  path exists.

## 10. Planned native surfaces

### Terminal

A native terminal dock is implemented with `alacritty_terminal` (grid/escape
state) and `portable-pty` (process). The engine owns the PTY, forwards bounded
output chunks, accepts input/resize/stop commands, and kills terminals on
shutdown. The UI owns the emulator and paints ANSI-colored runs plus a cursor;
PageUp/PageDown scroll history, and the alternate screen is preserved.

Current scope is one dock terminal using the active session's directory. Per-
workspace terminal trays, tabs, mouse selection/copy, wheel scrolling, and the
OpenCode TUI are follow-up work; the parity matrix names each gap. Tests cover
PTY→emulator output, ANSI SGR, alternate screen, resize, and engine command
routing.

### Editor and explorer

`vela-core::document::Documents` owns buffers by workspace/path independently
of session navigation. Preview tabs become pinned on text edits, including when
an edit is later undone. Selection moves alone do not pin them. Asynchronous
reads are scoped to request/document identity. Closed dirty views do not destroy
their buffers; explicit tab closing first saves the exact snapshot.

`vela-core::text::TextBuffer` handles UTF-8 boundaries, platform UTF-16 ranges,
grapheme navigation, selection, composition and bounded delta undo/redo. Native
`CodeEditor` views use platform input and a virtualized list of wrapped logical
lines. They own focus, geometry and scroll state. A rendered regression covers
large-file document-end navigation and input viewport bounds.

`vela-engine::files` confines workspace I/O and uses the original 8 MiB text
limit. Recoverable compare-and-write transactions serialize through a workspace
coordinator, hold the source, validate its expected content and install with an
exclusive hard link. Durable version-1 manifests retain an independent proposal
and the held original. Interrupted source holds are reconciled on initialization.
Conflicts pause ordinary saves; reload, continued manual merging and explicit
overwrite have native controls and preserve local text/recovery versions.

Remaining work includes multi-cursor and per-split view selection ownership,
syntax/language services, completion/Emmet, search/folding, unsaved
journaling, full window-close protection, captured agent-input baselines, a
virtualized explorer and its mutation/import operations. The current directory
listing still has a 1,000-entry cap that must be removed with the explorer work.
Recovery management/retention and legacy recovery locations also remain gaps.

`vela-engine::observation` owns one native recursive watcher per attached
workspace and a serialized read/write worker. Multiple sessions share that
owner until the last detaches. Notifications enqueue a bounded set of paths
with the reference's200ms debounce; overflow requests a non-starving rescan.
Workers retain file identities, correlate unique external renames across
batches, skip dependency/recovery directories and symlinks, and follow the
reference's2MiB watched-text limit. Explicit reads retain the8MiB editor limit.
Reads/writes and snapshot classification share one owner; own saves emit an
identified exact-snapshot echo. A following different external write remains
an external update. The recovery transaction supplies the actual held original
for a save baseline, including overwrite and deleted-file recreation.

Clean document updates reload without becoming dirty. Dirty updates retain
text and pause saves behind the existing conflict controls. External renames
move buffers, tabs, selection and undo together, without replacing an open
destination. Delayed save acknowledgements cannot replace a newer external
revision. Observation generations reject detached workers; UI snapshot reads
never hold a filesystem-I/O lock. Git baselines use bounded local object reads;
outside Git, unknown observations remain explicitly unknown. Agent-input
capture and complete baseline-aware Changes presentation remain unfinished.

### Browser

Each tab retains a separate WKWebView hosted through Wry. GPUI draws the native
chrome and editable address field; only the web page uses a webview. URL, title,
loading state and navigation availability come from the native host. Hiding a
tab preserves its input and history.

On macOS, tab configurations share an application-owned nonpersistent WebKit
data store. Cookie sharing and clearing after a full restart were verified with
local fixtures. Cocoa responder checks, standard edit selectors and native undo
routing prevent the native address/composer fields from intercepting guest input.
Navigation is restricted to credential-free HTTP(S) and `about:blank`; no Node,
preload or page-to-engine bridge exists. OS external-link handoff, the complete
permission/download/shortcut matrix and cross-platform validation remain open.

### Multi-panel layout

The engine supports concurrent session views, but the native UI is still a
single panel. Tiling up to eight panels, drag/swap, per-panel scroll/draft/focus
and related source behavior remain required migration work.

## 11. GPUI fork decision

Pinned dependency:
`https://github.com/zeronsh/zui` rev
`361ff20b919fdfb80199ee6d9aed725300cb7d54`.

The fork is an extracted standalone GPUI workspace from
`wingleeio/zed@e2ddcc6` (upstream `zed-industries/zed@f14fea9` plus the comet
patch line). Features relevant to Vela Native:

- `EdgeFade` primitives for scroll-edge fades;
- bounded backdrop blur and frosted surfaces, including macOS 26 fixes;
- GPU memory bounding, idle release of blur/path intermediates, image atlas
  eviction;
- paint-time rounded clipping and transparent-window alpha fixes;
- wgpu frosted-glass rasterization for non-Metal platforms.

Tradeoff: it is a fork of a fork with no shared Git history with upstream.
Upgrades require porting patches manually; we pin an exact SHA and record it in
`Cargo.lock`. Upstream GPUI remains usable as a fallback if the fork becomes
unmaintained, at the cost of the blur/edge-fade primitives. Licensing is
Apache-2.0 for GPUI with the `path` crate under GPL-3.0-or-later in-tree; Vela
Native does not use `path`. Attribution is recorded in
`THIRD_PARTY_NOTICES.md`.

## 12. Platform status

| Platform | UI | OpenCode | Terminal | Browser | Status |
|---|---|---|---|---|---|
| macOS 27 arm64 | GPUI window builds | native adapter | planned | planned (WKWebView/Wry) | primary target |
| Windows | compiles from same crates (unverified) | adapter is portable | planned | WebView2 | not validated |
| Linux | compiles from same crates (unverified) | adapter is portable | planned | WebKitGTK | not validated |

No cross-platform parity is claimed until builds and tests are run there.

## 13. Build profiles and instrumentation

- `cargo build --release` for measurements; debug builds distort UI timing.
- `tracing` spans exist for the adapter/engine; the UI exposes warm count and
  retained bytes in the sidebar footer.
- Planned: frame timing counters, navigation signposts, allocation counters,
  and a repeatable harness in `scripts/`.
