# Architecture

> **Document role:** canonical owner for cross-process architecture and durable architectural invariants. Module docs may summarize or route here but should not redefine these invariants.

Vela is an Electron app built with **electron-vite** (three build
targets: `main`, `preload`, `renderer`). It is a GUI for the OpenCode
agent: you open a repository, send a prompt, and watch the agent stream
its work while live diffs of changed files appear in the editor.

## Process model

```
┌─────────────────────────────────────────────────────────────┐
│ Electron MAIN (src/main/index.ts)                           │
│  • creates the BrowserWindow                                │
│  • owns VelaBackend (src/main/opencode.ts)             │
│  • registers shell:* IPC handlers                           │
│  • is the ONLY process that talks to OpenCode               │
└──────────────┬───────────────────────────┬──────────────────┘
               │ ipcRenderer.invoke()      │ webContents.send()
               │ (renderer → main)         │ (main → renderer)
               ▼                           ▼
┌──────────────────────────┐   ┌─────────────────────────────┐
│ PRELOAD (src/preload)    │   │ RENDERER (React 19 + Monaco)│
│ contextBridge:           │   │  • store.tsx = all UI state │
│ window.vela API     │   │  • components render it     │
└──────────────────────────┘   └─────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ OpenCode V2 service (spawned via Service.ensure or existing)│
│  • SSE event stream (client.event.subscribe)                │
│  • REST: session/file/model/project/permission endpoints    │
└─────────────────────────────────────────────────────────────┘
```

## Backend connection

`VelaBackend.connect()` in `src/main/opencode.ts`:

1. Probes `opencode --version` and derives a V2 compatibility predicate
   (`MIN_SUPPORTED_SERVER_MAJOR = 2`): a registered service must report the
   same V2 major line, so V1 and legacy beta daemons are refused instead of
   being served with an incompatible protocol.
2. `Service.discover()` — finds an already-registered opencode service whose
   version satisfies the predicate.
3. Falls back to `Service.ensure({ command: ["opencode", "serve", "--service"] })`,
   which terminates a version-mismatched daemon, spawns the service, and waits
   for it to be ready.
4. Creates a typed client: `OpenCode.make({ baseUrl, headers })`.
5. The `runEventLoop()` SSE loop runs through the `createStreamPipeline`
   transport (`src/main/stream-pipeline.ts`): 33ms per-directory batched
   flushing with delta coalescing and snapshot barriers, a 30s heartbeat, and
   exponential reconnect backoff; `connect()` is retried every 2s until a
   client exists, and three consecutive stream failures release the client so
   the loop rediscovers or ensures a live service without an app restart.

## Runtime adapter boundary

The normalized adapter contract starts in
`src/main/runtimes/runtime-adapter.ts`. `RuntimeManifest` uses Vela
protocol version 1, a stable runtime id, runtime version, availability, and an
explicit capability bitmap. OpenCode V2 is the only runtime registered or
selectable by the app. The Settings page reports its capability manifest but
does not expose runtime choice.

The DeepSeek Harness implementation under `src/main/runtimes/deepseek/` is
retained as dormant source for possible later reactivation. The production
backend does not register its factory, probe `dsh`, or surface DeepSeek session
records in recents; attempts to open a DeepSeek runtime are rejected. Legacy
`runtimeID` fields remain optional in shared session types so older persisted
records can still be read without making DeepSeek an active runtime. New
sessions are created through OpenCode V2.

## Message flow

All backend→renderer message kinds are defined in
`src/shared/types.ts` (`BackendMessage`):

- `{ kind: "event", type, data }` — every OpenCode SSE event forwarded
  through the main-process transport pipeline (coalesced per directory and
  flushed in 33ms batches). The renderer dispatches on `type`. See
  `docs/events.md` for the full protocol map.
- `{ kind: "file-update", file: { workspace, sessionID, path, movedFrom?, baseline, content, deleted } }` —
  emitted by the generation-bound fs watcher (below).
- `{ kind: "session", session: { id, directory, workspace } }` — emitted when a
  session context activates (a new concurrent panel).
- `{ kind: "recovery", recovery: { workspace, records } }` — durable artifact
  inventory emitted on activation and transaction/acknowledgment changes.
- `{ kind: "terminal-data" | "terminal-exit", terminal }` — PTY output /
  exit from the terminal tray (`src/main/terminal.ts`).
- `{ kind: "ui-command", command }` — main-process requests to the
  renderer (`open-source` with `{ path, line }` when a CSS rule's source link
  is clicked in DevTools — `path` is a canonical app-root-confined absolute
  path opened at that line as a standalone tab when outside the active workspace).

Renderer→main is synchronous invoke over `shell:*` channels; the full
table is in `docs/main.md`.

Prompt failures are normalized through `src/shared/errors.ts`. Submission
rejections preserve native codes when available or receive a stable `VELA_*`
code; runtime, stream, malformed-event, and event-handler failures are forwarded
as structured session/global events so the renderer can retain both the message
and code.

## Renderer trust boundary

The application window is explicitly sandboxed with context isolation and no
Node integration. Its privileged preload bridge is protected in main: every
IPC invoke must come from the active window's main frame while it is at the
exact packaged application document or an approved loopback HTTP development
origin. Packaged startup ignores `ELECTRON_RENDERER_URL` and always loads the
bundled file.
Unexpected main-frame navigation and redirects are canceled, and all popup
creation is denied. Markdown and tool attachment anchors prevent same-frame
navigation and request an external popup instead. Main opens only absolute,
credential-free `https:` URLs through the operating system; local files,
custom schemes, malformed targets, and insecure HTTP targets are rejected.

## Session lifecycle

1. User picks a folder (renderer → `shell:select-folder` / `shell:open-session`).
   Main accepts the renderer generation before a native dialog opens, so a
   later user action wins even if an earlier dialog resolves later.
2. `openSession(directory)` creates the session via
   `client.session.create({ location: { directory } })` — opencode's own
   defaults pick the model and agent; canonicalizes the workspace root,
   assigns a fresh immutable workspace identity, and starts the context's
   fs watcher.
3. Emits `{ kind: "session" }`; the renderer replaces the displayed panels
   with the selected session and focuses it. Reopening an already-open session
   reuses its context (stable workspace identity, no re-emit) and the renderer
   just focuses the panel. Model mode's explicit `+` action is the additive
   path.
4. Prompts go through `client.session.prompt({ sessionID, text, files? })`;
   interrupt through `client.session.interrupt`.
5. The backend owns any number of concurrent session contexts at once, each
   with its own watcher, snapshots, and recovery state. Every `shell:*`
   invoke addresses one workspace identity, resolved against the context map.
   One global SSE loop feeds all sessions; events route by session id and
   `filesystem.changed` fans out to every context on the reported directory
   (the reported path is realpath-canonicalized before matching, so
   symlinked or case-differing roots reach the right context).
6. Closing a panel invokes `shell:close-session`: main stops that context's
   fs watcher and removes it from the context map (the opencode session
   itself stays alive so recents can reopen it), and reopening the session
   later activates a fresh context with a fresh workspace identity. A workspace
   replacement detaches a panel without closing its context while that session
   is busy; the renderer reconciles `activeSessions()` every second so the
   background run remains in **Open now** and can be reopened or explicitly
   closed.
7. `window-all-closed` quits on every platform. `before-quit` aborts the active
   SDK SSE subscription, stops every context watcher, and tears down terminals.
   A renderer reload does not close the backend process; the renderer reattaches
   `activeSessions()` through the existing session-by-id path and focuses the
   last valid context only if the user has not acted. A full process restart
   has no live backend contexts: after the launch splash, the IDE waits for an
   explicit workspace or file open. Stale `vela.sessionLayout` restart hints are
   removed rather than replayed. Missing, moved or runtime-unavailable live
   entries are skipped independently; transient editor, terminal, permission,
   queue, prompt and capability state is not recreated.
   Dock `activate` can re-create a window only while the process is still alive
   (for example after programmatic window destruction), not after the
   last-window quit path.

## Diffs and baselines (how the diff view works)

The Changes list represents workspace file changes observed during the active
session, regardless of whether they came from a tool, shell, editor, formatter,
user, or another process. Main preserves the first baseline per path until Git
metadata changes, then refreshes the effective Git baseline:

- **Tool snapshot**: when `session.tool.called` arrives, every file path
  found in the tool input (keys `filePath`/`file_path`/`path`) is read and
  stored as that file's baseline *before* the tool executes.
- **git fallback**: files first observed via `fs.watch` use
  `git show HEAD:<rel>` in a Git workspace; untracked Git paths use a known
  empty baseline with `exists: false`. A Git HEAD result, including empty
  content, represents an existing file.
- **unknown fallback**: first-observed non-git changes have an explicit unknown
  baseline because the watcher only has post-change bytes.
- **Vela mutations**: saves and creates establish a known baseline only
  when none exists. Delete and rename preserve the established baseline.
- **Live watching**: one recursive `fs.watch(directory, { recursive: true })`
  per open context captures the activation root/session/identity and
  workspace-scoped maps, then feeds every change through a 200ms debounce into
  `onFsChanged`, which compares against `lastKnown`, assigns a baseline if
  missing, and emits identity-bound `file-update` with `{baseline, content}`.
  Reading an editor file also records its device/inode identity. If a later
  watcher event finds that identity at exactly one new path and confirms the
  old path is gone, main emits `movedFrom` on the destination update and
  suppresses the duplicate late deletion. Equal content is never treated as
  proof of a move; uncorrelated deletes keep the normal recoverable deleted-file
  behavior.
  Git metadata events use the same debounce to refresh tracked snapshots;
  files equal to their known baseline, and deleted paths with `exists: false`,
  leave Changes, while files still differing remain listed.
  Await boundaries and emissions re-check that the context is still
  registered.

The renderer merges updates into tabs and removes clean file updates from
Changes. A known baseline renders a diff in the Changes view; an unknown
baseline stays in Changes as `observed` without one.

This is observation, not attribution. Structured tool paths can be captured
before execution, but arbitrary shell command strings are not parsed for paths,
and Git `HEAD` does not identify an author. Skipped directories (`.git`,
dependencies, caches, IDE metadata, and build outputs), unreadable/binary files,
watcher coalescing, pre-activation changes, and missed first notifications can
prevent observation or recovery of pre-change content.

### Native revert and VCS queries (v2 contract)

The server now owns undo checkpoints: `session.revert.stage` stages a rollback
at a message (optionally including file changes), `commit` applies it, and
`clear` discards the stage; Vela surfaces this as a hover "Revert from here"
action on completed assistant turns plus a staged-undo dock card. The v2
contract also exposes `vcs.get`, `vcs.status`, and `vcs.diff` per location.
These are candidates to replace the local `git show HEAD:` baseline heuristic
because they understand staged/untracked/merge state, but adopting them means
re-running the watcher-phase tests against a new source; keep that as its own
change rather than mixing it into feature work.

## Editing and saves

The renderer is fully editable. Each edit increments its tab revision and
autosave captures the exact workspace, path, content, expected disk content,
and revision after a 900ms debounce; ⌘S saves immediately. Saves serialize
per workspace/file in the renderer and all filesystem mutations serialize per
workspace in main; identified echoes clear only matching state.
Normal writes require the disk to still equal the last saved content. External
updates preserve local edits and pause saving until explicit reload, overwrite,
or keep-editing then save-merged resolution. Lifecycle changes invalidate
timers and stale completions; external updates advance a conflict generation so
an already-started completion cannot clear a newer conflict. Writes create a
transaction under `.git/vela-recovery` for ordinary Git repositories, or a
workspace-specific hidden sibling directory on the same filesystem. A non-Git
workspace at a filesystem mount point uses workspace-local `.vela-recovery`
to preserve same-filesystem atomic operations. Vela then copies proposed bytes
into a second durable artifact, move the current target inode into the
transaction, validate the held bytes, and install the temporary inode with a
no-replace hard link. The original pathname is briefly unavailable between the
hold and install. A concurrent recreation is never overwritten. Neither success
nor rollback unlinks the held original inode, so later writes through an
already-open descriptor remain visible in the recovery artifact. Proposed bytes
remain durable. Phase metadata is atomically replaced and fsynced; activation
restores the held original only for interrupted `source-held` or
`held-validated` transactions and only when the canonical path is missing,
never over an existing path. Completed, failed, and acknowledged history never
replays. Successful transactions are acknowledged automatically while their
bytes remain retained; abnormal transactions remain visible until acknowledged.
Acknowledge persists metadata only and never deletes bytes. Activation runs a
best-effort retention purge that removes settled transactions (`complete`,
`failed`, acknowledged) older than 24 hours and interrupted ones
(`source-held`, `held-validated`) older than 7 days; fresh transactions are
never purged. This protocol requires recovery and target names to share a
filesystem. Writes go through Node `fs` in the main process
(`shell:fs-write`); the OpenCode API has no write
endpoint — the server sees the change via its own file watching. The
explorer also supports create/rename/delete through `shell:fs-create-*`,
`shell:fs-rename`, `shell:fs-delete` (delete moves to Trash). File rename uses
same-filesystem no-replace hard-link/unlink semantics. Directory rename follows
the directory-move policy: Vela rejects an already occupied destination and
then performs one atomic `fs.rename`, avoiding recursive copy/delete while
accepting the portable API's destination race limitation. File rename moves the
source into a durable hold before linking the no-replace destination. Rollback
only links back into an absent source and never
unlinks the hold, preserving ambiguity when another process recreates the
source. These operations run through the same watcher so baselines and the tree
stay consistent.

Drag-and-drop moves (`shell:fs-move`) deliberately deviate from the
hold/link recovery dance: a directory cannot be moved as a held single
inode the way a file can, and recursive copy-into-recovery would invite
concurrent-mutation races. `movePath` instead performs one atomic
`fs.rename` after re-checking the destination (never replacing an
existing path), so a failed move leaves both the source and destination
untouched and no recovery transaction is recorded. Cross-filesystem
`EXDEV` failures are rejected as errors rather than falling back to a
copy. The backend emits a tracked deletion at the source and, for files,
an addition carrying the captured baseline at the target; files inside a
moved directory surface through the fs watcher, and the renderer
re-lists both the old parent and the destination. The source/target
confinement rules are identical to the other mutations, and prior product
recovery directories are rejected as both source and destination.

Every workspace filesystem call carries the expected workspace UUID and main
rejects stale generations. Paths are bounded strict relative paths and no
existing symlink component may be traversed, including an intermediate parent
of a new target. This assumes stable topology during the operation; Node
pathname APIs cannot fully prevent an external symlink swap after validation.
Absolute reads are not part of the workspace API. DevTools CSS navigation
resolves only app-root-confined sources in main, then uses the same explicit
external-file capability as a user-opened standalone file.

New recovery storage is normally outside the project file tree. Existing
`.vela-recovery` transactions remain readable, move to the new storage on
workspace activation when safe, and are excluded from
watching, Explorer, and application file references. The recovery root, transaction directories, artifacts, and
canonical recovery parents must contain no symlink component. Transaction ids
and manifests are validated, and Open resolves a known artifact id in main
rather than accepting a renderer path. Recovery reconciliation is activation-
generation guarded before filesystem mutation.

## Models, agents, and composer controls

Each session panel's header has two pickers. Models come from
`client.model.list()` (filtered to `{ id, providerID, name, variants }`) and
are grouped by provider in the composer menu. The current model is seeded
from `client.model.default()` per panel and updated live by
`session.model.selected` for the addressed session.
Switching calls `client.session.switchModel({ sessionID, model })`, including
`model.variant` when a model exposes response-strength variants.
Provider settings are adapter-backed rather than OpenCode-specific UI. The
current adapter filters `integration.list` down to Vela's supported provider
set (`opencode-go`, `command-code`/`commandcode`, `openai`) and maps the
survivors into secret-free shared types; write-only keys go through
`integration.connect.key`, and credential changes refresh the workspace model
catalog. Connect and OAuth flows reject integrations outside that set.
Agents come from `client.agent.list()`; the selection is updated live by
`session.agent.selected` and switched via
`client.session.switchAgent({ sessionID, agent })`. Both choices are
session-scoped only: a new session starts on opencode's configured defaults
(`default_agent` / default model), and Vela never writes preferences of
its own. The composer
also opens a native multi-file picker; the main process converts selected
files to `file://` URIs for the prompt API. Its approval toggle is local UI
state: approve mode automatically replies `once` to each permission request.
Voice input uses the Chromium Speech Recognition API when the Electron build
provides it.

## Terminal tray

The bottom tray (`src/main/terminal.ts` + `TerminalTray.tsx`) and the optional
agent TUI panel (`AgentTui.tsx`) run real PTYs via `node-pty`. Main resolves the workspace identity and supplies the
addressed workspace's canonical directory as cwd rather than accepting one
from the renderer. The tray belongs to the focused panel: switching focus
boots a fresh terminal for that workspace and stops the previous panel's
PTYs; `stopAll()` runs only at quit.
It spawns the selected shell in that directory and forwards PTY
output to the renderer as `terminal-data` messages; keystrokes go back
via `shell:terminal-input`. Resizes are handled with the xterm `fit`
addon + `shell:terminal-resize`. The tray is toggled from the titlebar
(⌥O), its height is drag-resizable, and dragging the divider to the
window bottom closes it on mouse release rather than mid-drag.
Terminal ids, ownership, input size, and bounded positive dimensions are
validated before operations reach `node-pty`.
The selected shell is the user's normal interactive shell, not a login shell.
The locked Node-API-based `node-pty` is exercised under Electron on macOS,
Linux, and Windows CI.

An agent panel can switch from GUI to TUI from its header mode pill. The main
process resolves the active session's runtime command and starts it with the
session directory as cwd; OpenCode launches `opencode --session <session-id>`.
The renderer keeps the TUI inside the panel with xterm.js and reuses the
terminal data, resize, ownership, and cleanup paths. Only OpenCode is currently
available to panels; the dormant DeepSeek adapter has no TUI path. The Prism
appearance uses violet alpha surfaces with the same embedded terminal; it does
not open an external terminal window. The renderer reports the active color
profile's native appearance to the main process on boot and on profile changes,
so macOS `under-window` vibrancy and native chrome use the matching light or
dark material.

## Permissions

When OpenCode needs approval it emits `permission.asked`. The renderer
shows a card with the action and resources and three buttons; the reply
(`once` | `always` | `reject`) goes through
`client.permission.reply({ sessionID, requestID, decision })`.

## Key constraints

- The renderer never touches Node APIs (contextIsolation + no
  nodeIntegration); everything goes through `window.vela`.
- `SKIP_DIRS` (main) and `HIDDEN_DIRS` (renderer) filter tree noise
  (`node_modules`, `.git`, build dirs). `listDir` reads directories
  directly with `fs.readdir` (no opencode round trip), so explorer
  refreshes never depend on service latency.
- `MAX_EDITABLE_BYTES` (4 MiB) and a NUL-byte check keep binary/huge
  files out of the editor.

## Implementation ownership and fitness

These implementation boundaries preserve the existing process model and public
contracts. Related features enter through the owning facade; extracted private
modules own cohesive transformations or storage, rather than a second source
of application state.

| Responsibility | Entry point and private owner | Dependency boundary |
|---|---|---|
| AI Elements transcript and comparison | `src/renderer/src/ai-elements/` | Renderer-only presentation over the existing chat projection. Upstream primitives and demos are frozen by a SHA-256 inventory; authored adapters retain normal architecture checks. The comparison uses isolated documents and deterministic event replay, with no runtime credentials or filesystem writes. |
| Browser tab state and presentation | `src/renderer/src/browser-tabs-context.tsx`, state in `src/renderer/src/use-browser-tabs.ts`, integration `src/renderer/src/browser-store-provider.tsx`, pane `src/renderer/src/components/BrowserPane.tsx`, guest lifecycle `src/renderer/src/use-browser-guest.ts`, skin `src/renderer/src/styles/_browser.scss` | Stable actions enter shell/Editor composition; changing URL/title/loading reaches only tab and guest consumers. Renderer never configures Electron session authority. |
| Editor strip and status | `src/renderer/src/components/EditorPane.tsx`; private `src/renderer/src/components/EditorTabs.tsx`, `src/renderer/src/components/editor-groups.ts`, `src/renderer/src/components/EditorStatusMenu.tsx`, `src/renderer/src/editor-activity.ts`, skin `src/renderer/src/styles/_status-menu.scss` | Editor owns split presentation and resource cleanup. Group-local tab identity drives metadata/validation; file content/persistence remains the existing Editor state owner. |
| Changes review view | `src/renderer/src/components/ChangesView.tsx`, model `src/renderer/src/changes-view-model.ts`, skin `src/renderer/src/styles/_changes.scss` | Renderer-only review surface for observed workspace changes; reads the file-observation owner through the store and renders Pierre diffs. Owns no persistence, filesystem or runtime authority. |
| Launch presentation and IDE handoff | `src/renderer/src/main.tsx`, coordinator `src/renderer/src/launch.ts`, React entry `src/renderer/src/mount-app.tsx` | Static local artwork paints before IDE loading. The renderer owns the three-second visible-paint hold and handoff; main owns compact landing and workspace window dimensions. No session is selected by launch presentation. |
| Window chrome and agent empty state | `src/renderer/src/App.tsx`, private titlebar `src/renderer/src/components/Titlebar.tsx`, sidebar wordmark `src/renderer/src/components/SidebarBrand.tsx`, multi-agent input skin `src/renderer/src/styles/_compact-agent.scss`; AgentPanel composes the empty-state station artwork in the renderer assets directory | Presentation and user gestures only; no session, runtime or privileged authority. The empty-state artwork is decorative and never receives pointer events. |
| File tree glyph palette | `src/renderer/src/components/FileIcons.tsx`, private mapping `src/renderer/src/components/file-glyphs.ts` | Pure filename-to-glyph/color projection; tree loading and filesystem authority remain separate. |
| Embedded browser isolation | `src/main/browser-guards.ts`, policy in `src/main/browser-policy.ts` | Main attaches guest guards before renderer load; external content has no application preload, Node or IPC authority. Dedicated nonpersistent session denies permission checks, requests and devices. |
| Appearance and interface selection | `src/renderer/src/theme.tsx`, palettes in `src/renderer/src/appearances.ts`, private settings presenter `src/renderer/src/components/AppearanceSettings.tsx`, shell skin `src/renderer/src/styles/_codex.scss`, decorative appearance skin `src/renderer/src/styles/_observatory.scss` | Palette selects colors, syntax and optional decorative artwork; independent shell preference selects chrome. Settings never owns session or runtime state. |
| Electron lifecycle, windows, trusted IPC | `src/main/index.ts`; domain registrations in `src/main/ipc/` | Main composition retains sender validation and raw `ipcMain` authority. Domain registrars receive only the trusted registration interface, and own no runtime/session state. |
| Application data folder migration | `src/main/user-data-migration.ts`, called by `src/main/index.ts` | Moves the first available prior product data directory to Vela's data location before stores initialize; a failed move keeps the prior directory active, and explicit custom paths are untouched. |
| Runtime/session orchestration and service traffic | `src/main/opencode.ts`; adapter contracts and retained implementations in `src/main/runtimes/` | Main remains the only OpenCode SDK owner. Dormant DeepSeek implementation is private to its retained runtime directory. |
| History projection | `src/main/session-replay.ts` | Pure normalized replay depends on shared contracts/presentation, not SDK or privileged effects. |
| Recovery artifacts and transactions | `src/main/recovery-storage.ts` | Captured workspace root/current predicate come from backend orchestration; storage uses Node filesystem operations and workspace security, without owning sessions or events. |
| Workspace observation | `src/main/workspace-observation.ts`, private to the backend | Owns watch maps, debounce/import suppression, Git baselines and inode/file emissions; accepts session/watch authority predicates. SDK and session identity stay backend-owned; confined mutation choreography belongs to `workspace-mutations.ts`, and artifact storage to `recovery-storage.ts`. |
| Workspace mutation workflows | `src/main/workspace-mutations.ts`, private to the backend | Owns confined serialized import/save/create/delete/detach/rename/move and transaction ordering. Uses the existing coordinator, observer and recovery storage with backend authority predicates; no SDK or session registry. |
| Release update window | `src/main/app-update-controller.ts`, private to main composition | Owns popup lifecycle and update/unsaved gates; receives the existing updater and live window/file readers. Shared popup presentation stays in `app-updater-window.ts`. |
| Session inventory | `src/renderer/src/use-session-inventory.ts`, private to Store | Owns open-context and recent-history records, request identities and domain writers. Store owns polling/readiness scheduling and session generations; consumers use named readers rather than mutable refs. |
| Session model and agent selection | `src/renderer/src/use-session-selection.ts`, private to Store | Owns catalogs, selection records and load/switch actions; only focused-session access, generation validation and notification enter. Effects and event subscription remain in Store. |
| Workspace bookmarks | `src/renderer/src/use-saved-workspaces.ts`, private to Store | Owns bookmark records, hydration revisions and persistence. Store supplies readiness and notification; session activation remains in Store. |
| Panel state projection | `src/renderer/src/panel-projection.ts`, private to Store | Derives presentation from existing records; owns no mutable state, effects or runtime calls. |
| Workspace file observations and tree | `src/renderer/src/use-workspace-files.ts`, private to Store | Owns file, tree, expansion, visibility and recovery records with directory loading, generation-aware hover prefetch and existing debounce timers. Editor samples a readonly single-record query before its file read. Store retains attributed file classification/application ordering, session generations; Explorer disk workflows belong to `use-explorer-mutations.ts`. |
| Renderer storage migration | `src/renderer/src/storage-migration.ts`, called by `src/renderer/src/main.tsx` | Moves legacy product-prefixed local-storage keys to the Vela prefix before renderer owners initialize, preserving existing values when both versions exist. |
| Explorer disk workflows | `src/renderer/src/use-explorer-mutations.ts`, private to Store | Owns create/rename intents and disk mutation actions through the controlled preload bridge. Named file/editor capabilities retain their distinct remap and capture policies; Store retains session identity and generations. |
| Workspace editor | `src/renderer/src/use-workspace-editor.ts`, private to Store | Owns editor records, persistence and actions; receives six existing typed host capabilities plus an optional foreground-file activation notification. WorkspaceFiles supplies borrowed file observations; Store keeps session generations; Explorer disk workflows use named editor commands and a render-captured active-path query. |
| Live app stylesheet saves | `src/main/app-styles.ts`, renderer swap/hook in `src/renderer/src/app-styles.ts`, markers and update shape in `src/shared/app-styles.ts` | The application source root owns a serialized Sass worker and watcher independently of user workspaces. Compiled vendor CSS is retained; failed compilation or loading preserves the last valid sheet. The renderer swaps only app-owned links without reloading state. |
| Window DevTools | `src/main/devtools-controller.ts`, private to main composition | Owns picker, debugger events and frontend polling; receives the application-confined source resolver. Window trust and IPC remain in main. |
| Explorer presentation | `src/renderer/src/components/ExplorerTree.tsx`, private to FileSidebar | Recursive rows, inputs and menus receive the existing gesture contracts. Sidebar owns selection, drop routing and workspace state. |
| Session chat records | `src/renderer/src/use-session-chat-records.ts`, private to Store | Owns raw records, transcript projection, replay commands and permission/status rows. Exposes readonly borrowed records and named mutations. Store retains event attribution, generations and materialization IO; execution state belongs to the session execution owner. |
| Visible session lifecycle | `src/renderer/src/use-session-lifecycle.ts`, private to Store; query views in `src/renderer/src/store-contract.ts` | Owns panels, focus and acquisition/close identity across an early authority initializer and later workflow composition in the same module. Borrows named inventory, bookmark, file/editor, catalog, replay and execution operations. Root retains startup scheduling, materialization, hydration and attributed event order. Private state is opaque; consumers receive readonly queries. |
| Session progress | `src/renderer/src/use-session-progress.ts`, private to Store; borrowed UI views in `src/renderer/src/store-contract.ts` | Owns workspace todos, session/provider usage, compaction baselines and their private timing refs. Store retains event attribution, materialization/replay order, three effect slots and refresh timers. Permitted imports are shared presentation/retention/contracts and React. |
| Forms and approval requests | `src/renderer/src/use-session-requests.ts`, private to Store; borrowed UI views in `src/renderer/src/store-contract.ts` | Owns form records, approval preference and request commands. Existing chat commands remain the permission-row authority. Store retains event normalization, location and session attribution, automatic reply dispatch and polling slots. |
| Session prompt undo/edit | `src/renderer/src/use-session-revert.ts`, private to Store | Owns runtime revert staging/commit/clear, duplicate-action locks, authoritative transcript refresh and one-shot restored composer drafts. Borrows identity, chat, execution and todo operations through named commands. |
| Session inbox and local queue | `src/renderer/src/use-session-inbox.ts`, private to Store; pure queue policy in `src/renderer/src/message-queue.ts` | Owns local queue persistence and immediate ref authority, server inbox render mirrors and queue commands. Borrows render-captured selection metadata and structural transcript append commands. Store retains normal prompt lifecycle, polling and event attribution/order. |
| Session execution | `src/renderer/src/use-session-execution.ts`, private to Store | Owns busy/abort/stream/activity records and named execution commands; settlement scheduling stays at its existing Store effect position. Uses the chat facade through narrow structural operations, never its private state owner. Store retains runtime attribution, generations, IO and user notifications. |
| Renderer state and chat projection | `src/renderer/src/store.tsx`, `src/renderer/src/chat-store.ts`; private `src/renderer/src/chat-records.ts` and `src/renderer/src/chat-projection.ts` | The chat facade owns authoritative record reduction and projection; consumers do not import its private record/projection owners. Pure projection depends on renderer stream/record transformations and shared contracts. |
| Transcript presentation models | `src/renderer/src/components/OpenCodeTimeline.tsx`; private `src/renderer/src/components/timeline-model.ts` | Pure turn, tool and delegation transformations depend on existing chat/shared contracts; timeline presenters own markup and disclosure interaction. |
| Codex chat transcript | `src/renderer/src/codex-chat/` | Renderer-only default transcript over the existing chat projection. Renders the Codex column and worked-for divider, reveals streamed Markdown with the ported per-segment fade timeline, owns compact tool/reasoning rows, and owns its floor-aware scrollport. No persistence, runtime, filesystem, or privileged authority. |
| DSH chat transcript | `src/renderer/src/dsh-chat/` | Renderer-only opt-in (`?chat-renderer=dsh`) transcript over the existing chat projection. Uses the published DeepSeek Harness Markdown primitive for frozen-block streaming, owns turn process folding/tool/reasoning rows, and owns its floor-aware scrollport. No persistence, runtime, filesystem, or privileged authority. |
| Provider credential discovery | `src/main/provider-usage.ts`; private `src/main/provider-credentials.ts` | Credential file discovery belongs to main. Provider-specific authentication, request ordering and quota semantics stay in the usage facade. |
| Composer control presentation | `src/renderer/src/components/ComposerToolbar.tsx` | Stateless attachment, agent, model, strength and send buttons; Composer retains draft, dispatch and menu state. |
| Agent prompt styling | `src/renderer/src/styles/_agent-prompt-dock.scss` | Owns composer layout and integration selectors, included through the renderer stylesheet entry point. Pinned upstream styles keep their separate retention policy. |
| Panel geometry and layout interaction | `src/renderer/src/agent-panels.ts`; App composes private `src/renderer/src/layout/` | Geometry imports only shared session contracts; drag/tray interaction imports only React, not store or backend owners. Layout presenters stay private to App composition. |
| Controlled cross-process bridge | `src/preload/index.ts` | Preload depends on shared contracts, never main/renderer implementation. Renderer declarations may derive its interface through an explicit type-only exception. |
| Shared contracts and normalization | `src/shared/` | Shared imports stay within shared; no Node, Electron, renderer, preload, or main implementation dependencies. |

Architecture fitness is executable through the canonical gate. Module/function
budgets ratchet existing debt downward while enforcing defaults immediately for
new owners. Both runtime and type-only imports remain acyclic; shared markdown
projection types have one owner in `markdown-projection.ts`. Substantial exact-token
clones, new cycles, process direction violations, private-owner leaks, excluded
implementation imports, and privileged authority leaks fail validation.
New modules must resolve to a named file or directory route in this ownership
table before their paths can enter the ratchet. Computed dependency loaders and
invalid guard manifests fail closed; literal dependency forms share the same
authority and cycle checks. Retained upstream styles have a frozen path/hash
inventory; changed pinned content and new unowned production in that directory
cannot inherit its exclusion.
The cohesive inline-code language table is retained as baseline size debt;
compressing data literals or moving them outside the measured scope would not
improve responsibility ownership. See [operations](operations.md#architecture-fitness-and-measurement)
for commands, measurement semantics, and exception review.

The reviewed `opencode.ts` size exception retains OpenCode SDK/service/session
authority under the protected single-SDK-owner policy. Watcher, recovery engine
and transaction workflows live in their private owners. Remaining read and
standalone-file capabilities have distinct confinement/failure semantics; their
methods remain individually constrained. The integration ceiling ratchets down;
new growth requires reassessing helper and capability ownership rather than
raising the baseline automatically.

The remaining `store.tsx` size exception covers the public React context facade
and cross-domain runtime event/IO coordinator. Its private owners retain session,
chat, execution, files/editor, explorer, inbox, requests, progress and catalog
authority. Materialization, hydration, prompts, reconnect and the single attributed
event dispatcher retain their shared ordering here; an opaque EventsHub would
merely relocate that coupling. Each callback remains individually constrained
and the composition ceiling ratchets downward. Lifecycle workflows have one
precisely inherited reopen-complexity exception for generation, silent restoration,
placement, abandonment, replay and finally-count behavior; its budget cannot grow.
