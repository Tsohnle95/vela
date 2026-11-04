# Walkthrough: how the pieces connect

> **Document role:** end-to-end routing aid. Use it to follow a flow across
> processes, then consult the canonical module/architecture doc for durable
> invariants. If this walkthrough conflicts with a canonical owner, fix the
> walkthrough rather than creating a second truth.

This is the moving-picture map of Vela. It intentionally stays concise: its
job is to route an unfamiliar agent to the correct connection points and source,
not to restate every invariant already owned by `architecture.md`, `main.md`,
`renderer.md`, `preload.md`, and `events.md`.

## Connection map

```text
RENDERER (React / Monaco)
  store + components
       │ window.vela.* / onMessage
       ▼
PRELOAD
  src/preload/index.ts
       │ shell:* invoke / shell:message
       ▼
MAIN
  src/main/index.ts
  src/main/opencode.ts
  src/main/terminal.ts
       │                     │
       ▼                     ▼
OpenCode / runtimes      filesystem / PTY
```

Five connection points cover most debugging:

| # | Connection | Follow |
|---|---|---|
| 1 | renderer → main calls | component/store action → `window.vela.*` → preload wrapper → centralized trusted registrar → domain IPC handler / backend method |
| 2 | main → renderer messages | backend/terminal emitter → `webContents.send("shell:message")` → preload `onMessage` → renderer store |
| 3 | main → runtime request | backend/runtime manager → selected adapter / OpenCode client |
| 4 | runtime → main stream | OpenCode SSE or runtime-native downlink → normalization → shared event vocabulary |
| 5 | main ↔ disk / PTY | workspace identity → confined filesystem or `TerminalManager` operation |

Canonical details: `architecture.md`, `main.md`, `preload.md`, `shared.md`.

## Boot

Route:

```text
app.whenReady()
→ backend.start()
→ register message forwarders
→ registerIpc()
→ createWindow()
→ backend.connect()
```

`backend.start()` owns the reconnecting stream lifecycle. `createWindow()` uses
context isolation, no Node integration, sandboxing, and the preload bridge.
`backend.connect()` discovers or ensures the OpenCode service and creates the
typed client. Renderer startup probes health and reconciles `activeSessions()`
so open backend contexts can be restored.

Read next:

- main startup / trust details: `main.md`
- reconnect/transport behavior: `architecture.md` + `events.md`
- renderer restoration: `renderer.md`

## Opening a repository or file

Folder flow:

```text
Welcome / Sessions action
→ selectFolder() or openSession(dir)
→ preload shell:* wrapper
→ VelaBackend.openSession()
→ runtime session create
→ activateSession()
→ { kind: "session" }
→ renderer mounts/focuses panel
```

Each active panel receives an immutable workspace identity. That identity is
carried by later filesystem/terminal calls so stale or replaced workspaces are
rejected in main.

A single-file workspace opens a runtime session on the file's parent directory,
then opens the selected file in that session. External files outside the active
workspace use the explicit standalone-file path rather than bypassing main.

Read next: `architecture.md#session-lifecycle`, `main.md`, `preload.md`.

## Sending a prompt and watching work

Request path:

```text
Composer / store sendPrompt()
→ window.vela.prompt(...)
→ backend.prompt()
→ active runtime adapter
```

Response path is stream-driven rather than a second request/response path:

```text
runtime event
→ main transport / normalization
→ { kind: "event", type, data }
→ renderer store
→ authoritative chat store
→ visible transcript projection
```

Busy/idle/retry/error state is session-scoped; model/agent selection events also
land on the session that owns them. Child/subagent sessions continue streaming
independently.

Read next: `events.md` for event semantics; `renderer.md` for chat/stream state.

## File changes and Diff

Observed workspace changes enter through tool pre-snapshots, Git fallback, Vela
mutations, and filesystem watching. Main owns the baseline and emits an
identity-bound `file-update`; renderer merges that into the Changes list and any
open tab.

```text
source/tool/editor/disk change
→ main baseline + watcher pipeline
→ file-update { baseline, content, deleted }
→ renderer agentFiles + tab state
→ Changes / Diff
```

Known baselines enable Diff. Unknown non-Git first observations remain visible
as observed changes without pretending the pre-change bytes are known.

Canonical invariants: `architecture.md#diffs-and-baselines-how-the-diff-view-works`.
Renderer behavior: `renderer.md`.

## Editing and saving

Renderer edits are revisioned and debounced. Saves cross the preload bridge to
main, where workspace identity/path confinement and recovery semantics are
applied. External disk changes do not silently overwrite dirty editor state;
conflict resolution remains explicit.

```text
Monaco edit
→ renderer persistence snapshot
→ shell:fs-write
→ main validation / recovery transaction
→ filesystem
→ observed file-update echo
→ renderer reconciliation
```

Create/delete/rename/move use the corresponding `shell:fs-*` surfaces. Do not
infer mutation semantics from this walkthrough; use the canonical architecture
and main-process docs when changing them.

## Permissions

Runtime permission requests normalize into the shared event vocabulary and are
rendered as session-owned permission state. Replies travel renderer → preload →
main → active runtime. Auto-approve is renderer-local policy; main/runtime still
owns the actual permission operation.

Read next: `events.md`, `renderer.md`, `main.md`.

## Runtimes, models, and agents

Runtime selection enters at workspace/session opening and becomes part of the
session's runtime identity. Runtime-owned operations (prompt/history/model/etc.)
route through the retained adapter; core workspace editing/watching remains in
Vela.

Model/agent catalogs and selections are session-scoped. UI controls depend on
the active runtime capability manifest; unsupported capabilities should be
hidden rather than allowed to fail later.

Canonical boundary: `architecture.md#runtime-adapter-boundary`.
Implementation ownership: `main.md` + `renderer.md`.

## Terminal and embedded agent TUI

Normal terminals and runtime TUIs use `node-pty` through main. Renderer owns the
xterm views; main resolves the addressed workspace identity and supplies the
confined cwd/command.

```text
renderer terminal/TUI action
→ preload
→ TerminalManager / runtime TUI resolution
→ node-pty
→ terminal-data / terminal-exit
→ owning renderer panel
```

Read next: terminal sections in `main.md`, `renderer.md`, and platform checks in
`operations.md`.

## Session history, reopen, and subagents

Recent sessions come from the backend session inventory. Reopen hydrates the
session transcript into the same semantic shapes used by the live stream.
Already-open sessions reuse/focus their active context; closed sessions activate
a new workspace context.

Parent/child metadata makes delegated-agent cards navigable. Child sessions are
real sessions with their own streams, not nested transient text maintained only
by the parent UI.

Read next: `main.md` session methods, `renderer.md` parent/child navigation.

## Window lifecycle and DevTools

`window-all-closed` quits on every platform. `before-quit` tears down the stream,
watchers, and terminals. Renderer reload is different from application quit:
while main remains alive, active backend contexts can be restored into the new
renderer.

Main also owns the DevTools integration and CSS-source navigation. Source links
are resolved only against Vela's canonical application root and then opened
through the normal external-file workflow; they never search whichever user
workspace happens to be active.

Canonical owner: `main.md` (window/trust/DevTools details).

## One rule behind it all

The renderer has no direct Node capability. Privileged work crosses
`window.vela` into trusted main-process handlers; runtime-native details are
normalized before they cross into renderer state.

When adding a cross-process feature, trace the complete path rather than editing
only the UI surface:

```text
shared contract (if needed)
→ main/backend/runtime implementation
→ shell:* IPC
→ preload wrapper
→ renderer action/state/component
→ tests/fixtures
→ canonical docs
```
