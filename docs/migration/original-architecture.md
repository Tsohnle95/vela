# Original Vela architecture audit

Audited checkout: `/Users/tdawg/code-repositories/vela`, commit
`ba995c142c353722945b0c116694155409dfd8a8` (2026-10-23). The working tree
contained pre-existing user changes under `design/vela-product/styles.css` and
`mockup-design/`; they were not modified or committed. This document describes
the Electron implementation that Vela Native replaces.

## 1. Process topology

```
Electron main (src/main/index.ts)
  ├─ VelaBackend (src/main/opencode.ts) ── only process talking to OpenCode
  │    └─ OpenCode V2 shared service (`opencode serve --service`)
  ├─ TerminalManager (src/main/terminal.ts) ── node-pty children
  ├─ MobileServer (src/main/mobile-server.ts) ── loopback companion server
  ├─ file watcher / git baseline / recovery / Sass / Vite / updater
  └─ shell:* IPC + webContents.send
        │
Preload (src/preload/index.ts, contextIsolation)
        │ window.vela
        ▼
Renderer (React 19, Monaco, xterm.js, Zustand, 112 TSX components)
```

The renderer is sandboxed (`contextIsolation: true`, `nodeIntegration: false`,
`sandbox: true`), and main validates every IPC sender against the trusted
window/main frame. Markdown and tool links can open only credential-free
HTTPS URLs externally; same-frame navigation and popups are denied.

## 2. Backend connection and lifecycle

`VelaBackend.connect()` (`src/main/opencode.ts`):

1. Runs `opencode --version`; derives a V2 predicate from
   `MIN_SUPPORTED_SERVER_MAJOR = 2`.
2. Finds a matching registered service via `Service.discover()` (including the
   macOS desktop service file).
3. Otherwise calls `Service.ensure({ command: ["opencode", "serve", "--service"] })`
   with bounded cooldowns.
4. Constructs the typed client with `OpenCode.make({ baseUrl, headers })`;
   headers are HTTP basic auth produced by `Service.headers(endpoint)`.

An SSE loop (`runEventLoop`) feeds every open context. It reconnects with
exponential backoff and re-runs discovery/ensure after three consecutive stream
failures, so a dead shared daemon does not require an app restart. A clean
stream end or heartbeat timeout emits a synthetic `server.connected`; the
renderer re-materializes open sessions from authoritative message history.

## 3. Stream transport (verified behavior)

`src/main/stream-pipeline.ts` — the following is read from code, not inferred
from the README:

- Events are queued **per directory** (the event's top-level location), not in
  one global queue.
- A queue flushes after **33 ms** (`FLUSH_FRAME_MS`), computed against the last
  flush time. Under backpressure the flush interval rises to **200 ms** for
  **10 seconds** (`BACKPRESSURE_FLUSH_FRAME_MS`, `BACKPRESSURE_MODE_MS`).
- Deltas for the same key are concatenated in place before flushing:
  `session.text.delta*`, `session.reasoning.delta*`,
  `session.tool.input.delta*`, `session.compaction.delta*`,
  `message.part.delta*`.
- Authoritative snapshots act as barriers: `message.part.updated`,
  `session.text.ended`, `session.reasoning.ended`,
  `session.tool.input.ended`, tool success/failure clear the coalescing key so
  a late delta cannot merge across the snapshot.
- `session.idle`, `session.error`, `session.created`, `session.deleted` clear
  the `session.status` key (and session-update keys for create/delete).
- The consumer yields after **8 ms** of continuous iteration to keep the main
  process responsive.
- A **30 s** heartbeat aborts a silent stream; a heartbeat/stream-end triggers
  immediate reconnect, while ordinary failures back off 250 ms × 2 up to 5 s.
- Invalid events raise `VELA_STREAM_INVALID_EVENT`; failures are normalized to
  structured errors.

This batching is deliberate and effective at reducing IPC/render pressure. The
native design keeps the same policy shape but can apply events directly in the
engine without an IPC hop or JSON envelope per event.

## 4. Authoritative chat state and projection

`src/renderer/src/chat-store.ts` + `chat-records.ts` + `chat-projection.ts`:

- Per-session message and part maps keyed by ID; binary-search ordered maps for
  server-ordered records.
- Full snapshots replace streamed state with dedupe bookkeeping
  (`__dedupeNextDeltaFields`); terminal tool cards never regress.
- Text/reasoning parts without IDs use `{messageID}:{type}:{ordinal}`, matching
  live deltas and replay.
- `message.part.updated` reconciles whole parts; tool snapshots merge by
  `callID` and retain fields omitted by the snapshot.
- `projectAssistantItems` caches the last projection per message and only
  re-projects when the parts array identity changes.
- `chat-projection.ts` orders parts (canonical `prt_` order vs. stream `seq`),
  merges duplicate tool parts, and attaches agent/model metadata to turns.
- `timeline-model.ts` filters internal system reminders, consolidates subagent
  tools, drops permission/pending-input/selection/system rows from the visible
  transcript, and builds user-rooted turns.
- `VelaTurn` splits process rows (reasoning and tool calls) behind a "Worked
  for" divider; final text is rendered last. `VelaMarkdown` uses a paced
  reveal while streaming (24 ms cadence, immediate catch-up above 512 chars).

`large-session.performance.test.tsx` establishes the existing regression floor:
400 transcript items / 800 DOM rows must reduce in under 100 ms (median) and
derive under 10 ms in jsdom; this excludes Chromium layout and paint.

## 5. Renderer surfaces (verified inventory)

- `App.tsx` / `store.tsx`: workspace, explorer, editor tabs, agent panels,
  terminal tray, browser pane, settings, welcome.
- `AgentPanel.tsx`: one agent session, model/agent pickers, composer, transcript,
  usage, TUI switch, permission/form cards.
- `SessionsPane.tsx` / `Welcome.tsx`: recent sessions and workspace entry.
- `FileSidebar.tsx` / `ExplorerTree.tsx`: file tree, create/rename/move/delete,
  drag/drop import, context menu, file icons.
- `EditorPane.tsx`, `EditorTabs.tsx`, `monaco.ts`: Monaco, tabs, manual Save/⌘S,
  external modification, language detection, Emmet, split view.
- `ChangesView.tsx` / `@pierre/diffs`: observed file changes and real diffs with
  baselines.
- `TerminalTray.tsx` / xterm.js: per-workspace terminals, fit/resize, ANSI,
  scrollback, copy/paste, process exit.
- `BrowserPane.tsx` / `browser-store-provider.tsx`: Electron `<webview>` tabs,
  address bar, localhost previews, nonpersistent partition.
- `SettingsPage.tsx`, `AppearanceSettings.tsx`, `ProviderSettings.tsx`,
  `ServerSettings.tsx`, `MobileSetup.tsx`: preferences, palettes, provider
  credentials, OpenCode sync, mobile pairing.
- `OpenCodeTimeline.tsx`, `OpenCodeTool.tsx`, `OpenCodeFile.tsx`,
  `OpenCodeTodoDock.tsx`, `OpenCodeSpecialTool.tsx`: tool cards, file links,
  todo dock, special tools.
- `VelaTranscript` / `VelaTurn` / `VelaToolRow` / `VelaReasoning`: the current
  chat experience.

Base CSS is in `src/renderer/src/styles/` (20 Sass modules); the established
palette is defined in `_foundation.scss` (`#171412` shell, `#262220` panels,
`#9eb4a1` accent). The native theme reuses these product tokens.

## 6. Terminals

`src/main/terminal.ts` owns a `TerminalManager` over `node-pty`. Terminals are
owned by a workspace identity, addressed as `term-N`, capped at 1 MiB input and
1000×500 dimensions, and forward `{kind:"terminal-data"}` / `terminal-exit`.
Shutdown is bounded (3 s per child, 10 s total). The renderer uses xterm.js and
a fit addon. Agent panels can launch the OpenCode TUI as an embedded terminal.

## 7. Embedded browser

Main installs `webview` attachment guards before the renderer loads
(`browser-guards.ts`, `browser-policy.ts`). Guests:

- admit credential-free HTTP(S) and exact `about:blank`;
- strip preload/session overrides, Node integration and nested guests;
- force sandbox, context isolation, web security;
- use the nonpersistent `vela-browser` partition and deny permissions/devices;
- deny popups and external navigation; only safe HTTPS links reach the OS browser;
- receive no `window.vela` bridge.

The UI is `BrowserPane.tsx` with tabs/address bar in React; the page itself is
Electron's Chromium guest. State is in `browser-store-provider.tsx` and
`use-browser-tabs.ts`.

## 8. Persistence and user data

- Chromium profile under Electron `userData`; window bounds in
  `window-bounds.json`; bookmarks + relocation aliases in the saved-workspace
  store with a `.bak` fallback.
- Vela does not own OpenCode conversations; OpenCode owns its SQLite DB,
  credentials and service registration. Main reads provider credentials from
  `auth.json` and `opencode.db` only to query provider usage; tokens never cross
  IPC.
- Recovery artifacts live in the repository or a workspace-specific hidden
  sibling, with fsynced manifests and bounded retention (24 h settled, 7 d
  interrupted).
- Mobile pairing: `MobileServer` launches an OpenChamber-family companion
  server, keeps its password in main, and returns only a one-time pairing URI.

## 9. Performance-sensitive behavior and hypotheses

Verified mechanisms (not bottlenecks by assumption):

1. **33 ms event batching** with delta coalescing and snapshot barriers reduces
   React update rate.
2. **Per-message projection cache** avoids re-projecting unchanged messages.
3. **Paced streaming reveal** avoids per-token full Markdown renders.
4. **Monaco** edits files outside the transcript.
5. **Terminal output is batched** to the renderer by xterm.js.

Architectural risks (require measurement before optimization):

- Every visible turn is a React tree; each tool/message can become multiple DOM
  nodes. There is no row virtualization in the current transcript.
- Session switching remounts panels and re-projects the session; warm state is
  not a first-class cache.
- The renderer owns all projections while main owns all I/O; each event crosses
  IPC and is JSON-decoded again.
- Markdown is parsed per streaming text part; long assistant messages can
  reparse from the start on each paced update.
- xterm.js, Monaco, webviews, DevTools and React share one renderer process, so
  a heavy surface can compete with chat input.

No CPU or memory profile of the original app was available in this environment;
`docs/performance/vela-baseline.md` records that limitation instead of
presenting guesses as measurements.

## 10. Audit gaps

- Exact Chromium/Electron rendering cost under long sessions is unmeasured.
- Mobile companion contents are not vendored in this checkout
  (`Resources/mobile-server`), so the server protocol was audited only through
  `mobile-server.ts` and setup UI.
- Window vibrancy and appearance behavior on macOS 26/27 were not profiled.
- The original repository has no root `LICENSE`; reuse of source/assets beyond
  product tokens was therefore avoided. See `THIRD_PARTY_NOTICES.md`.
