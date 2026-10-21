# Module: renderer

> **Document role:** canonical owner for renderer state ownership, renderer-side invariants, major actions, and component responsibilities. Keep presentation minutiae in source/tests unless they encode a non-obvious invariant.

`src/renderer/src/` — React 19 app. `store.tsx` exposes shared application state
and orchestrates its domain owners; components consume that facade and retain
their local interaction state. Monaco is configured in `monaco.ts`.

## Store (`store.tsx`)

`store.tsx` remains the public `useStore()` / `usePanel()` facade and the owner
of attributed backend-event dispatch, reconnect/startup scheduling and cross-domain
orchestration. Session attachment, focus and acquisition/close workflows belong to
`use-session-lifecycle.ts`; its authority owns visible panels, focus mirrors and
request/activation identities. Editor state and actions belong to
`use-workspace-editor.ts`; observed file changes, Explorer listings/visibility
and recovery records belong to `use-workspace-files.ts`. `store-contract.ts` owns its renderer-only contracts;
existing type imports from the facade remain supported. It depends only on shared
contracts and renderer domain types, so internal owners can use those types
without importing the orchestration module.

`use-saved-workspaces.ts` owns bookmark state, hydration, pending mutation replay,
revision protection and bookmark actions. Renderer startup migrates prior
product-prefixed local-storage keys before state initialization; this owner then
reads and writes the Vela key. It receives connection readiness and a stable
notification callback; main-process storage remains authoritative after initialization.
Related bookmark behavior belongs in this owner rather than session attachment
code. `panel-projection.ts` owns the pure panel view derivation
from session/workspace records and the authoritative chat state. It preserves
record references and queue ordering without owning subscriptions or caches;
generation filtering and React memoization remain in the facade.

`use-workspace-files.ts` owns workspace agent-file, tree, expansion, hidden-path
and recovery records, their last-render read references and directory refresh
debounce timers. Add listing, visibility, observed-change or recovery behavior
there. Its public actions retain the Store facade; narrow integration commands
receive attributed file updates, restore nested workspace visibility and apply
successful mutation results. It receives rendered focus, lazy current-focus and
panel queries, generation validation and notifications. `use-explorer-mutations.ts` owns mutation IPC and pending-name intent; Store owns
session attachment and backend attribution. Store
orders editor preparation, observed-file receipt, editor conflict application and
scheduled listing refresh. The editor reads its baseline before awaited file IO.
Directory application is synchronous after the original reads, without additional
subscriptions, caching or timer cleanup.

`use-explorer-mutations.ts` owns create/rename intent, last-render intent
references and Explorer mutation workflows: create, rename, delete, move, detach,
import and reveal. Add mutation sequencing and its failure handling there. Its
four host callbacks provide current focus, generation validation, notifications
and menu closure; structural editor/file ports expose only named domain commands.
The editor's rendered active-path query preserves deferred rename selection,
while visibility queries preserve their captured render and invocation-time
focus distinctions. No Node/filesystem SDK or private owner implementation is
imported. Store exposes its actions directly and retains session lifecycle and
backend routing. Focus clears mutation intent through the stable cancellation
command after menu closure; its lazy reference preserves startup initialization
and the existing subscription dependencies. Mutation state changes introduce no
new effects or resource lifetime policy.

`use-session-lifecycle.ts` composes its private authority at Store entry and its
workflows after Editor/Explorer initialization. Stable named queries replace shared
mutable panel/focus references; Store mirrors rendered state at the existing slot.
The owner preserves captured acquisition identities, replacement suppression and
dirty-save close ordering through narrow structural domain commands. Store keeps
replay/materialization transactions, startup/reconnect and all existing effects.
The editor focus bridge invokes the later workflow lazily after render; it adds
no resource lifetime or subscription policy.

Exposed state:

| Slice | Shape | Notes |
|---|---|---|
| `session` | `SessionInfo \| null` | the focused session (derived from `panels` + `activeSessionID`); null → an empty IDE whose Explorer offers an **Open Workspace** action after the launch splash |
| `panels` | `SessionInfo[]` | all attached/live sessions in panel order; each panel continues streaming even when normal coding mode renders only the focused coding session |
| `activeSessions` | `SessionInfo[]` | backend-owned open contexts, reconciled every second independently of visible panels; drives the complete **Open now** inventory |
| `savedWorkspaces` | `ProjectInfo[]` | Vela-owned workspace bookmarks stored in the main process under Electron user data with an atomic backup; startup imports the migrated local-storage value only when app storage is uninitialized, or OpenCode's validated project catalog on first use without that value; initialized app storage is authoritative. User additions/removals during hydration are replayed over that snapshot; a failed read never overwrites it with an empty value |
| `panelViews` | `Record<workspaceID, PanelView>` | per-panel scoped projection (`session`, `busy`, `transcript`, `todos`, `sessionUsage`, `models`, `currentModel`, `agents`, `currentAgent`) consumed through `usePanel(workspace)` |
| `activeSessionID` | `string \| null` | focused session id; the editor and terminal tray bind to the focused panel while the Explorer retains every open workspace tree and every panel keeps streaming |
| `connected` | `boolean` | from `health()` on mount |
| `busy` | `boolean` | active-session projection of the per-session busy map |
| `todos` | `TodoItem[]` | structured OpenCode todo state shown as persistent agent steps in the prompt dock |
| `transcript` | `TranscriptItem[]` | active-session projection of the per-session transcript map |
| `sessionUsage` | `SessionUsage \| null` | active-session projection of cumulative token usage/cost; hydrated from `session.get` whenever a panel is attached (reopen, add control, workspace swap, folder pick, startup restore) and kept fresh by `session.usage.updated` / `session.usage.recorded`; `session.compaction.ended` and manual `/compact` re-poll it (immediately plus a delayed backstop against slow backends) and snapshot the pre-compaction input total as the compaction baseline, so the context-window fill drops instead of sticking at its pre-compaction value; shown in the agent header popup |
| `providerUsage` | `ProviderUsageResult[]` | live-first per-provider plan/rate-limit snapshots fetched via `refreshProviderUsage()`; rendered under the session usage popup as remaining quota (`% left`) while warning tones derive from consumed quota |
| `providerUsageLoading` | `boolean` | true while `refreshProviderUsage()` is in flight (refetch happens each time the usage popup opens) |
| `tabs` | `Tab[]` | open editor tabs for the focused workspace (per-workspace record; each session restores its own tabs on focus) |
| `activePath` | `string \| null` | active tab path for the focused workspace |
| `singleFile` | `string \| null` | the active workspace's single-file path (set when a file was opened as the workspace via `selectFile`/`openFileWorkspace`); non-null with only one panel makes the sidebar render true single-file mode (file name, one file row, no folder tree) |
| `agentFiles` | `Map<path, AgentFileState>` | `{baseline, content, deleted}` for observed files still differing from their effective baseline; drives Changes and known/unknown diff state; per workspace |
| `tree` | `Record<relPath, TreeEntry[]>` | lazy-loaded explorer cache warmed by hover prefetch; per workspace |
| `expanded` | `Set<relPath>` | open tree directories; per workspace |
| `toasts` | `Toast[]` | transient notifications |
| `recoveryRecords` | `RecoveryRecord[]` | durable workspace artifacts for the focused workspace; unacknowledged records remain actionable across restart |
| `models` | `ModelOption[]` | for the composer model/strength picker |
| `availableModels` / `lastModel` | `ModelOption[]` / `ModelOption \| null` | last loaded model catalog and selection retained for the empty agent panel after its workspace closes; controls remain inert until a workspace is active |
| `currentModel` | `ModelOption \| null` | per workspace; seeded from the session selection (falling back to `modelDefault()`) when a panel activates so a new session never displays another session's model; live-updated by `session.model.selected` for the addressed session; includes selected `variant`; carries `limit.context` from the model catalog, which the usage popup uses to compute context-window fill |
| `agents` | `AgentOption[]` | for the composer agent picker |
| `currentAgent` | `AgentOption \| null` | per workspace; seeded from the session selection, falling back to the session's creation agent and then `build`; live-updated by `session.agent.selected` for the addressed session and optimistic `switchAgent` |
| `runtimes` | `RuntimeManifest[]` | runtime availability and capability manifests; `AgentPanel` uses the active manifest's `tui` flag to enable the embedded TUI option |
| `approvalMode` | `ApprovalMode` | `ask` shows permission cards; `approve` automatically replies `once` |
| `sessions` | `SessionSummary[]` | shared recent history for Welcome and the Sessions pane; loads after backend readiness and refreshes on reconnect. Welcome observes this inventory instead of issuing a one-shot startup fetch; older requests cannot overwrite newer results |
| `ctxMenu` | `{x, y, target} \| null` | explorer right-click menu position and target entry (`null` = empty area) |
| `pendingCreate` | `{parent, kind} \| null` | inline "new file/folder" name input target |
| `pendingRename` | `{path} \| null` | inline rename input target |

`use-session-inventory.ts` owns the backend-open and recent-history records,
their current refs, refresh request identities and local upsert/removal actions.
Store retains the polling/readiness lifecycle and invokes refreshes at the
existing scheduling points. Lifecycle callers use named inventory readers;
they do not mutate inventory state or refs. A superseded open-context refresh
still returns its fetched list to its caller while leaving the newer inventory
intact; a failed refresh returns an empty result without clearing known contexts.

## Authoritative chat store (`chat-store.ts`)

`use-session-chat-records.ts` owns renderer transcript records, their read
snapshots, raw chat-state lifetime and record/projection/replay commands. Its
only host capability identifies sessions protected from transcript eviction;
foreign streams remain accepted. Store retains runtime attribution, transport,
generation guards, materialization IO and interactive inventories. Execution
policy has the owner below. Reopened replay resets raw records and commits the flat transcript only
after streaming reconciliation; materialized replay merges into existing raw
records. Dropping raw records does not drop retained transcript history.

Borrowed raw-state, snapshot and status views expose readonly dictionaries,
arrays and known nested fields without additional copying or freezing them. Pure readers
accept those views; the reducer's mutable records and replacement rules remain
unchanged. Components retain the existing Store/PanelView presentation contract.

`chat-store.ts` remains the public entry point for chat reduction, records,
projection and hydration. It owns runtime-event dispatch and protocol adapters.
Its internal `chat-records.ts` owns ordered record mutations, delta overlap,
terminal-state preservation and record types; `chat-projection.ts` owns
assistant presentation, history hydration and reconciliation with auxiliary
transcript rows. These internal modules depend on shared contracts and renderer
chat primitives; components and lifecycle consumers use the `chat-store.ts`
interface rather than importing record mechanics directly. New event behavior
belongs in the reducer; record consistency belongs in the record owner;
presentation and replay reconciliation belong in the projection owner.

Model-response events (`session.step.*`, `session.text.*`,
`session.reasoning.*`, `session.tool.*`, `session.retry.scheduled`,
`message.*`, plus execution/status lifecycle events) mutate a per-session
authoritative store — server messages and parts kept in binary-search ordered
maps (`binary.ts`) plus a session status map — instead of appending to the
transcript directly. The transcript the UI reads is a projection
(`projectAssistantItems`) applied after each event batch; auxiliary items
(permissions, shells, selections, compaction) still reduce into the flat
transcript via `reduceChatStream` in `chat-stream.ts`. Full part snapshots
carry dedupe bookkeeping so a trailing delta already included in a snapshot
is not applied twice (exact suffix match only — deltas are otherwise
concatenated verbatim like OpenCode's own reducer), finished tool cards
cannot regress, and history hydration (`hydrateChatState`) never shrinks
longer live text. Canonical OpenCode parts render in ID order, including when
snapshots arrive out of order. V2 stream parts have synthetic IDs and render
in their event arrival order; the timeline groups consecutive activity
entries while keeping interleaved prose between them. Distinct parts and
assistant messages remain visible even when their content repeats. Text and
reasoning segments without an explicit `ordinal` resolve to the newest
same-type part, matching upstream's implicit ordinal.

Projection reuses unchanged assistant rows through a weak cache scoped to the
chat state. Both the message object and its parts-array identity must match:
part deltas replace the parts array without replacing the message, while
metadata and lifecycle changes replace the message. Record updates must retain
this replacement rule, and projection consumers must not mutate projected
assistant rows. Reconciliation clones user rows before backfilling the responding
agent/model and preserves auxiliary chronology and inferred parent ownership.
The focused unsaved-file inventory is memoized on workspace tabs and panels;
chat updates do not rescan editor buffers.

Prompt submission reconciles the returned canonical snapshot once without
polling, deleting, or rebuilding the live store. Pushed reasoning and text
deltas therefore remain visible between OpenCode steps, while
`session.execution.*` stays authoritative for the composer until the terminal
execution event.

When a delta arrives for an unknown message or part (an incomplete session
snapshot), the store materializes the session over `shell:session-transcript`
and merges the authoritative history. `server.connected` /
`global.disposed` re-materialize every open panel, which covers gaps after
stream reconnects.

## Streaming lifecycle (`streaming.ts`, `session-activity.ts`, `assistant-status.ts`)

`use-session-execution.ts` owns busy/abort/streaming records, activity clocks and
runtime execution/status transitions through a narrow structural record-command
interface. Its borrowed maps are readonly. Stop and new-prompt acknowledgement
update abort refs immediately; terminal-event acknowledgement updates queued
state, leaving the ref unchanged until render. Streaming refs change only on
commits. Replay seeding preserves explicit idle rather than treating it as
missing status. Root keeps attribution, generation guards, IO, queues/forms and
todo/usage effects; status handling retains both existing reduction passes.

The same module's `useSessionSettlement` owns the existing polling timer. Store
invokes it at the original lifecycle point, using stable member dependencies and
the existing notification callback. Busy pruning consumes captured render values;
closing an off-view context removes busy/abort records at its existing call site
without clearing streaming records or activity clocks.

Vela's synchronized streaming stack tracks which
message is streaming per session and its lifecycle phase
(`streaming` / `cooldown` / `completed`) with `startedAt` / `lastUpdateAt` /
`completedAt` timestamps; `lastUpdateAt` writes are throttled to a 1s
heartbeat for lifecycle timestamps; every changed text, reasoning, and tool
event is projected immediately into the visible per-session transcript. The
store reconciles streaming state incrementally after every chat event batch
(`updateChangedStreamingSessions` compares the pre-event snapshot against the
copy-on-write message/status maps), falls back to a full reconcile after
history hydration, and touches the heartbeat for part deltas.
`session-activity.ts` resolves the session phase (`idle` / `busy` / `retry`):
it mirrors the authoritative status, falls back to the trailing incomplete
assistant while status settles, and yields to a pending permission so the
send button stays a send. Three store-side guarantees keep that fallback from
latching. Authoritative end-of-turn signals (`session.idle`, `session.error`,
`session.execution.*`) materialize completion of the trailing assistant in
the chat store even if its completion marker was lost. Terminal message/part
snapshots cannot promote an already-idle session back to busy; prompt
submission clears the composer before inserting the optimistic user message
into both the transcript and authoritative chat store, then marks the session
busy, so the input does not briefly duplicate the new transcript row and the
elapsed clock starts from the current turn instead of an older prompt. Only
events that represent active work can restore busy after a quiet settle.
Terminal message finishes also clear busy directly, so the composer and turn status do not
depend on a later `session.idle` event. Finally, a 1s settle
watchdog finalizes any panel whose trailing assistant stays incomplete without
stream activity for 60s (or immediately once the runtime reported idle), which
is what flips the composer's stop button back to send after a missed end event
or a reopened session with a dead tail. Live stream content restores busy on a
settled session (unless a stop is in flight), so a watchdog false-positive
self-heals on the next delta.

While a turn is active, `AgentPanel` has one chronological activity surface.
Reasoning, tool calls, command output, progress, and final text update their own
stable inline nodes; there is no mirrored activity footer above the composer.
Before the runtime emits the first concrete item, the timeline shows one minimal
inline working placeholder and replaces it as soon as the real item arrives. A
resize observer follows timeline growth while the reader remains at the floor,
so streamed content stays at the live edge without a second surface changing the
viewport height.
`assistant-status.ts` derives the working summary
every panel exposes: the active model (from the trailing assistant's
`parentID` back to its user message), the active part (text → "composing",
reasoning → "thinking", running tool → its tool phrase, editing tools →
"editing file", otherwise "preparing response"), the fully-synthetic
message guard, the forming state, `canAbort`, unacknowledged abort flags set
by `stop()`, and retry info. Interactive questions use the form dock described
below rather than the status overlay. `PanelView.activity`,
`PanelView.assistantStatus`, `PanelView.forming`, `PanelView.activeModel`,
and `PanelView.streaming` carry these to components; the agent header shows
the live status beside a green working indicator and the composer's stop
button title uses the same status text.

## Message queue (`use-session-inbox.ts`, `message-queue.ts`, `messages/`)

Queued follow-ups are native: sending a prompt while the session is working
submits it immediately through `session.prompt` with `delivery` set from
`followUpBehavior` (`queue`, the default, holds it in the server's session
inbox; `steer` delivers it into the running turn without interrupting). The
server owns ordering, delivery, and crash safety, and reports the lifecycle
through `session.inbox.enqueued/delivered/cancelled/delivery.changed`. The
Inbox owner tracks user-type entries per session (`inboxBySession`), hydrated via
`shell:inbox-list` on attach, and merges them with a small persisted local
queue that only catches submissions that failed outright (max 20 messages,
50 targets; pre-v3 persisted queues migrate into a quarantine map).
`QueuedMessageChips` renders both sources above the composer with first-line
previews, attachment counts, edit-back-to-composer, send-now (native entries
steer, local entries prompt), and remove (native entries cancel server-side);
drag-reordering only exists between adjacent local fallback entries because
server inbox order is FIFO by submission. The former client-side auto-send
loop — idle-transition dispatch, abort window, retry backoff — was retired:
the server performs all of that by holding and delivering inbox entries.

The Inbox owner holds both the local queue and server-entry record, exposes named
commands and deep readonly borrowed views, and owns no effects. Local commits
write the immediate queue ref before state and persistence; server entries use
a last-render ref. Store retains envelope attribution, normal prompt lifecycle,
and delivered-input promotion before Inbox removal and auxiliary reduction.
The queue and saved-workspace lazy storage reads are independent and caught;
extraction reverses their incidental read order while both finish before hydration
effects, without changing keys, defaults, or writes.

`use-session-requests.ts` owns the form record, refresh/reply/cancel commands and
approval preference/ref, and uses the Chat owner's permission reconciliation and
resolution commands without keeping a second permission record. Store keeps
normalization, true-session versus location-panel attribution, event guards,
automatic permission transport, and existing polling/effect placement. The
uncaught approval storage initializer stays before other persisted records;
toggles update its ref inside the state updater before the uncaught storage write.
Permission lists capture panels before parallel requests, retain event-backed
rows for failed workspaces, and apply successful lists after panel closure. Form
refreshes also retain late success; failed form replies refresh then toast only
while the workspace remains attached. Manual permission reply failures toast
globally, and successful replies resolve the captured row after panel closure.
Forms are borrowed with readonly nested fields and options; the unused render
mirror ref was removed.

`use-session-progress.ts` owns todos, session usage, compaction baselines and
provider usage/loading. Store retains activation, replay, materialization,
event attribution, polling/timers and the three persistence/inference/todo
fallback effects in their original order with their original dependencies.
The baseline storage initializer remains after approval initialization; the
empty todo initializer moves to that owner without IO. Usage and baseline
refs remain render mirrors, while todo keys and provider refresh sequence update
immediately. Reopened usage is read inside the receipt updater, followed by
protected-session retention. Closing a panel removes only its baseline.
Borrowed progress views include readonly token/cache, todo and provider
snapshot/window/credit/error metadata; existing public presentation identity
and payloads remain compatible.

Actions: `openSession` / `selectFolder` (replace the current panels with one fresh
panel and automatically save its directory as a workspace), `openWorkspacePanel(dir)`
(add a saved or dropped workspace beside existing file roots), `saveWorkspace`
(bookmark a directory from the native folder picker), `addModelPanel(dir)`
(explicit model-mode addition to a directory and bookmark it),
`selectAddPanel` (model-mode addition from the native folder picker, so each
new panel can target a different project), `selectFile` / `openFileWorkspace`
(open a single file as a true single-file workspace: replace the panels with a
session on the file's parent folder, mark the sidebar single-file, and open the
file as the active tab), `openExternalPath(abs)` (resolve a dragged/dropped
path against the current session: in-workspace files open normally, outside
files open as editable standalone tabs saved back to their real path),
`importPaths(destDir, sources)` (copy external files/folders into the workspace
at `destDir`, seeding clean baselines so imports never show as changes),
`dropIntoExplorer(paths)` (imports dropped files into the current workspace root
and imports dropped folders there without changing the active panels),
`selectPanelDirectory(workspace)`
(swap one panel to a folder picked in the native dialog) and
`changePanelDirectory(workspace, dir)` (swap one panel to a directory without a
dialog), `reopenSession(id, silent)`
(focus a running panel, otherwise replace the selected panel or the current view when no panel is selected unless silent), `focusSession(id)`,
`closePanel(id)`, `loadSessions`, `sendPrompt(text, files, workspace?)`, `stop(workspace?)`,
`refreshProviderUsage`, `loadModels(workspace?)`, `switchModel(id, providerID, variant?, workspace?)`,
`loadAgents(workspace?)`, `switchAgent(id, workspace?)`, `toggleApprovalMode`,
`openFile(path, workspace?)`, `closeTab`, `setActive`, `editContent`, `saveTab`, `saveAllEditorFiles`,
`reloadTab`, `overwriteTab`, `mergeTab`, `toggleDir`, `ensureRootOpen`,
`replyPermission(requestID, reply, sessionID?)`,
`reconcilePermissions` (polls `shell:list-permissions` per open panel workspace on a 3s interval and on
`server.connected`; appends missing pending cards, resolves cards whose requests vanished — e.g. answered
in an attached TUI — while preserving event-backed cards when a list request fails),
`openCtxMenu`, `closeCtxMenu`, `startCreate(parent, kind)`, `startRename(path)`, `cancelPending`,
`commitName(name)`, `deleteEntry(path)`, `deleteEntries(paths)`,
`moveEntry(path, destDir)`, `moveEntries(paths, destDir)`,
`revealInFileManager(path)`,
`openRecovery(id)`, `acknowledgeRecovery(id)`. `closePanel` invokes
`shell:close-session` so main tears down the panel's backend context
(watcher, context map) while the opencode session stays reopenable. Each panel
can point at its own directory: `selectAddPanel` opens the native folder
picker and attaches the chosen session as a new panel, while
`selectPanelDirectory` / `changePanelDirectory` swap an existing panel to a
fresh session on another directory in place — the replaced session is detached
rather than torn down (it keeps its backend context and stays listed in
**Open now** until explicitly closed), the panel keeps its position and focus,
and the old session remains reopenable from recents. The optional workspace/session parameters let
a background panel's composer act on its own session while the focused session's editor keeps
its state; they default to the focused session. `commitName`/`deleteEntry`/
`moveEntry` call the `shell:fs-*` mutation channels, then re-list every expanded
ancestor dir of the touched path so the tree stays current (directories
emit no `file-update`), move/close matching tabs, and move `agentFiles`
entries on rename. `moveEntry` re-lists both the old parent and the
destination and drops `deleted` change entries instead of remapping them,
so a moved folder never reappears in Changes as deleted at its new path.

Filesystem and terminal calls carry the addressed panel's `session.workspace`,
the immutable identity for that activation. Main resolves each identity against
its open context map and rejects unknown or replaced identities. `open-source`
carries a canonical absolute app-source path. With an active session, the
renderer passes it through `openExternalPath`, producing a writable standalone
tab when the app source is outside that workspace without replacing any panels;
when the active workspace is the app repository, it opens as a normal relative
tab. With no active session, the containing directory is opened first.

Sessions activate concurrently when explicitly restored or added in model
mode. Replacing panels detaches the replaced sessions rather than closing
them: each keeps its backend context, chat state, and **Open now** entry until
it is reopened or explicitly closed (the row X), so opening a new session never
drops the previous one from the jump list. Workspace opening
otherwise replaces the displayed panels, while every async
continuation captures its workspace identity and mutates only that workspace's
records, so a slow operation from panel A can never populate panel B's editor
or tree. Focus has a monotonic version: a late activation completion never
steals focus from a newer user action. Startup never reopens sessions from a
previous process: a fresh launch shows the compact Folded light splash, then an empty IDE
until the user opens a workspace or file, and any stale
`vela.sessionLayout` record left by an older build is deleted on startup.
Only live backend contexts are re-attached at startup (which covers a
renderer reload while the main process is still running). They reopen
sequentially in backend order through the existing `openSessionById`
validation/canonicalization path, and the last one is focused only if the
user hasn't already acted, so deleted or moved workspaces, unavailable
runtimes, and individual reopen failures are skipped without blocking
launch. Past sessions stay reachable through recents and the **Open now**
inventory. A saved workspace's main row adds its file root beside existing open
workspaces; its disclosure control expands that workspace's session history.
Explicitly reopening a recent session can ask the user to locate a replacement folder when its saved location
has disappeared, then updates Vela's saved workspace bookmark. Silent startup
hydration never opens a folder picker. A renderer reload still restores live
backend contexts; editor tabs, changes, terminals,
permissions, queues, prompts, and other transient state are not persisted for
this purpose.
`file-update` is accepted only when both its session ID and full workspace
identity match an open panel. A destination update with `movedFrom` remaps the
open tab, active path, and pending-save ownership before applying the new disk
content. Uncorrelated deletion keeps the tab's last content and deleted/conflict
state so the user can recover or explicitly resolve it.

Key mechanisms:

- **Event dispatch** — the `onMessage` effect handles `session` /
  `file-update` / `event` messages. The event switch is documented in
  `docs/events.md`. Every session event is reduced under its own `sessionID`;
  side effects (model/agent selection, todos, permissions, busy) land on the
  panel that owns the addressed session, so background panels stream
  independently. `{kind:"session"}` messages upsert panels idempotently.
  Current V2 `data` and legacy `properties` envelopes are normalized before
  dispatch. OpenCode's V2 permission names are adapted to the common names.
- **OpenCode stream batching** — events queue for a 33ms frame and adjacent
  text, reasoning, and tool-input deltas are coalesced before React updates.
  Adjacent authoritative snapshots of the same legacy part collapse to the
  latest snapshot. A timer is used instead of animation frames so background
  windows continue draining the stream.
- **Session retention** — tool and shell output retains the complete text supplied
  by OpenCode, including after completion, replay, and reopening. Text tool
  content is discarded after it is projected into `output`;
  file content blocks remain available. Live reduction and replay hydration use
  the same policy. The active stream plus the four most recently updated
  inactive streams are retained in memory; usage records follow the same LRU
  policy. Open panels are exempt from eviction, so a background panel's
  transcript and usage can never be silently blanked by other sessions'
  streams. A panel whose record was evicted while closed (or is otherwise
  missing) is re-hydrated from OpenCode when it is focused or reopened, so
  closed sessions remain reopenable and are hydrated on demand.
- **Selection parity** — catalog refreshes reconcile against
  `sessionSelection()` before falling back to `modelDefault()`, so a newly
  created or reopened GPT/agent session cannot be mislabeled with the previous
  session's model in the composer.
  `use-session-selection.ts` owns workspace model/agent catalogs and selections,
  the global last-loaded catalog/selection fallback, catalog refs, load/switch
  actions and selection-event updates. Its host supplies the focused-session
  ref, workspace generation resolver and notification callback. Store keeps
  event attribution and refresh scheduling; callers use the facade or these
  domain methods rather than mutating selection records directly.
- **Catalog self-heal** — the store re-checks backend health every 2s until
  the first successful connect; a `connected` effect then re-runs
  `loadModels()` / `loadAgents()` once the backend client is up, so a boot or
  reconnect that first hit a silent empty catalog (no client yet) is retried
  and the composer agent/model menus never stay empty. `agent.updated` and
  `catalog.updated` events additionally refetch the agent/model catalogs when
  the service resolves them lazily after a session opened, and opening a
  picker menu whose list is still empty refetches that catalog and shows a
  "No agents available" placeholder instead of a blank box.
- **V2 session reducer** — `chat-stream.ts` buffers admitted input until its
  promoted event, retains non-chat agent/model switches as internal state, and
  folds synthetic/skill/shell/compaction messages, assistant lifecycle, and
  legacy `message.*` projections into ordered `TranscriptItem`s. Tool
  state retains parsed input, content blocks, metadata, execution state, and
  provider state. Durable end/snapshot events are authoritative; terminal
  tool states cannot regress when events arrive late.
- **Manual, revision-safe persistence** — editing updates the in-memory tab
  and marks it dirty only while its buffer differs from the saved content; undoing back to
  that content clears the orange indicator and unsaved-file guards. Typing
  stays in memory until Save or ⌘S/Ctrl+S; Vite sees only saved edits.
  `EditorPersistence`
  receives an immutable workspace/path/content/revision snapshot, serializes
  writes per workspace/file, and strongly identifies echoes. Save and ⌘S/Ctrl+S
  still save immediately. Dirty clears only for the matching current revision.
  `use-workspace-editor.ts` owns workspace tab/active-path/single-file records,
  unsaved-file projection, opening, save/conflict actions and editor reactions to
  filesystem changes. `openFile` pins a tab; `previewFile` opens a single-click
  transient preview that replaces the previous preview until the file is opened
  again or edited (Explorer single click vs. double click). Its host supplies
  focused/attached session access,
  generation validation, focus, captured agent baselines and notifications.
  Store retains session lifecycle; the Explorer mutation owner sequences IPC
  and forwards successful mutations through editor and file domain operations.
  The workspace-file owner manages observed changes and tree refresh.
  File updates prepare rename/echo classification before agent-file updates,
  then apply tab conflicts. Backend-subscription cleanup still cancels editor
  persistence at its original lifecycle boundary, including subscription reruns.
- **Lifecycle and conflicts** — reset, close, delete, rename, switch, and
  unmount invalidate expected echoes and in-flight completions. Closing a dirty
  tab saves it first; unresolved conflicts keep the tab open. Any dirty tab in
  any workspace blocks window unload until the save completes or the user
  confirms discarding it. The native quit dialog can save all dirty editor files
  across workspaces without navigating to their tabs; failed saves keep Vela open.
  External updates advance a per-path conflict generation, so completion of a
  write already in flight cannot clear the newer conflict.
  External updates preserve edits and pause saving until explicit Reload,
  Overwrite, or Keep editing to merge followed by Save merged content.
- **Change baselines** — tabs carry the first established known/unknown
  `baseline` from `agentFiles`, and saves do not replace it. File tabs are
  always the edit surface; the baseline feeds the rail's Changes view, where
  known content renders a diff and unknown content stays labeled `observed`
  without one. `stale`/`deleted` flags surface external changes.

- **V2 transcript replay** — reopened sessions accept OpenCode's flat
  `SessionMessageInfo[]` plus legacy `info`/`parts` projections and reconstruct
  the same user, selection, synthetic/system/skill/shell, assistant, and
  compaction items used by the live reducer. Selection and system prompt items
  remain available for state reconstruction but are filtered from the chat
  timeline. Synthetic `<system-reminder>` entries are also retained for
  replay but filtered from the chat. Renderer
  startup reopens the backend's active session silently so a reload hydrates
  persisted messages before new live events continue. `mergeChatHistory`
  reconciles replay with any global SSE events received during the request,
  preserving the live timeline's semantic interleaving, terminal tool states,
  and the longest streamed text/reasoning values.

- **Parent/child navigation** — `session.created` plus `session.list` maintain
  parent ids. Task cards use upstream's metadata-first, parent/title/agent
  fallback to open a child transcript, and child headers return to the parent.
  Timeline tool cards carry their panel's session so opening a file from a
  background panel focuses that panel first, and permission replies address
  the owning session.

- **AI Elements response presentation** — `AgentPanel.tsx` retains the original
  panel layout, header, empty-state artwork, scroll owner, approval/form/revert
  dock, todo dock and `Composer.tsx`. Only response content is rendered by
  `ai-elements/AiElementsTimeline.tsx`, which reads the existing panel transcript
  and execution state. Permission rows remain in the original dock. An empty
  response region has no layout footprint. Suggested prompts and the alternative
  input/context/todo controls in `AiElementsChat.tsx` are restricted to the
  isolated comparison examples; the application never mounts that full shell.
  `useAgentScroll` continues owning live-height following and reader scroll memory.
  Live AI Elements responses ease the viewport toward the growing bottom on
  animation frames; reader scrolling cancels following, and reduced motion uses
  immediate positioning. Initial history and the legacy renderer stay immediate.
  Production Markdown uses the upstream primitives with bounded word fades.
  Syntax highlighting runs in `ai-elements/highlight.worker.ts` with the existing
  latest-per-block queue, using the upstream GitHub themes and JavaScript engine
  under the existing CSP. New code remains visible while tokens are pending;
  matching highlighted prefixes are retained as more characters arrive. The
  original Markdown helper remains frozen for the upstream demo gallery.
  Production code fences and tool output wrap within the available response width,
  including unbroken tokens; horizontal scrolling is reserved for other widgets
  such as tables. `AiElementsActivity.tsx` displays the existing panel working
  summary, actual running tool details and elapsed time while busy, including
  gaps before the first token and between steps. Permission waits and retries
  have explicit labels, and the activity row clears on completion or stop. External anchors retain the shared HTTPS URL policy.
  `adapters.ts` accepts optional structured source, citation, suite, stack,
  plan and task metadata; it never infers those records from arbitrary prose.
  Such metadata is conditional on the runtime/tool supplying it. Permissions,
  file opening, child navigation and revert continue through the existing store
  with their explicit session/workspace identities. Production replies do not
  insert checkpoint or restore controls between turns; the original explicit
  user-message Edit action retains its existing revert confirmation flow.
  ThemeProvider mirrors the selected appearance scheme to the standard `dark`
  class so upstream widgets and portals receive the correct neutral theme.
  `AiElementsSurface.tsx` isolates response CSS in a ShadowRoot so shell Sass
  cannot override upstream typography, spacing or widgets. It owns the reference
  response padding and mirrors the selected color scheme. App response typography
  uses the same `--ui-font-*` tokens as the folder panels; upstream demo fonts
  remain unchanged. `StreamingMarkdown.tsx` enables bounded word fades for live
  deltas, while completed history renders immediately. Tool groups keep a stable
  component shape when later calls arrive, preserving open details and DOM state.
  Base UI portals use
  the same shadow boundary through `portal-container.tsx`; only fonts and
  Tailwind property registrations are document-wide. The original composer
  remains outside that boundary. Vendored
  source/provenance lives under `ai-elements/upstream/`; runnable original
  examples are under `ai-elements/demos/`. See the comparison procedure in
  [operations.md](operations.md). `?chat-renderer=old` selects the original
  implementation; `?chat-renderer=compare` displays both against the live panel
  view. Neither presentation becomes another transcript authority.
- **Legacy runtime-neutral transcript presentation** — `OpenCodeTimeline.tsx` owns
  chronological run/turn composition, auxiliary events and permission prompts.
  Its pure `timeline-model.ts` owns tool/path/edit metadata, task descriptors,
  delegation reconciliation, turn grouping and reasoning-heading projection;
  it depends only on shared contracts. `OpenCodeTool.tsx` owns tool dispatch and
  edit/error/delegation/context/shell/native presenters; its file activation
  owner maps tool paths to the addressed workspace before focusing and opening
  it. Main retains authoritative filesystem path confinement.
  `TimelineProse.tsx` owns prose pacing, clipboard/text controls, user rows,
  thinking transitions and shared row presentation. Feature changes enter through
  those owners; presentation does not acquire another transcript or runtime
  authority. Task display descriptions fall back from an empty description to a
  trimmed prompt, while dispatch signatures retain explicit empty descriptions.
  Tool dispatch preserves eager formatting failure behavior, branch order and
  its mounted controller state across failure/recovery. Dormant DeepSeek metadata rendering remains in
  source, but the production app only opens OpenCode sessions. The response layout
  follows OpenCode V2’s message timeline: a right-aligned user bubble, flat assistant
  parts, and a hover footer with copy plus the owning message’s agent/model and turn
  duration. Revert belongs to the user-message footer. The source styles under
  `styles/opencode/` are pinned to upstream revision `03e67171`; Vela’s theme colors
  and responsive code/table wrapping remain intentional overrides in `_opencode-chat.scss`.
  Markdown tables use fixed column layout and break long cell content; paths
  and prose break within the panel rather than increasing its intrinsic width.
  The ported semantic tokens follow the active appearance’s light/dark scheme.
  Reasoning parts are
  hidden (OpenCode's default `showReasoningSummaries: false`) and surface only as
  a single turn-level thinking row while the turn is busy. The latest reasoning
  heading replaces the previous one with OpenCode's 700 ms masked reveal and
  25 px travel; reduced-motion settings disable the transition. Reasoning and tool calls
  otherwise form one calm work log, with contiguous assistant messages flattened
  into one run before grouping. Context reads (read/list/glob/grep) always fold — as in
  OpenCode's `groupParts` — into a `ContextToolGroup`: its trigger shows a
  `ToolStatusTitle` (`Exploring` while pending, `Explored` when settled) plus an
  `AnimatedCountList` summary (`N reads, N searches, N lists`) and expands to
  compact inline rows. Those rows mirror OpenCode's `contextToolTrigger`: read shows
  the file basename plus offset/limit args, while list/glob/grep show the search
  directory with `pattern`/`include` as args. Every tool row is a single-line
  trigger using OpenCode's `basic-tool` metrics (`basic-tool-tool-info-main` is a
  baseline flex row; 14px title in text-base, 14px subtitle/args in text-muted), so
  a burst of exploration never fills the transcript; a running tool shimmers its
  title.
  Final prose remains visually separate from that log. Each assistant run keeps a
  stable keyed entry and chronological position. Short native deltas appear
  immediately; a text update more than 512 characters ahead of the visible text
  advances every 24 ms with OpenCode's chunk sizes and word-boundary snapping.
  Completing an assistant message flushes pending text immediately. Text
  rendering follows the owning message’s completion, even when a text part
  finished earlier. Copy appears only on the final text part of a settled turn.
  A one-shot reasoning summary is never made to resemble token streaming.
  `OpenCodeMarkdown.tsx` adapts the upstream Markdown lifecycle to React. The
  `opencode-markdown/` worker owns marked parsing, remend tail healing, KaTeX, and
  Shiki streaming tokens. Completed prose blocks retain their DOM via morphdom;
  code retains stable tokens while its unfinished suffix changes. Per-part cache
  keys distinguish healed live blocks from finalized blocks. Worker revisions
  prevent stale asynchronous results from replacing newer content; completed
  messages flush pending text. HTML is sanitized and external links retain Vela’s
  URL policy. Tool disclosures start collapsed like upstream; running titles
  shimmer without an additional status badge. Failed tools replace the inline row with OpenCode's
  `ToolErrorCard`: the capitalized error head becomes the subtitle, the cleaned
  error body expands on demand in the card's sans-serif body style, a flat 2px
  inset left accent bar marks the failure, and a copy affordance appears on hover. Bottom-follow uses a stable signature to follow
  new business rows and turn-state changes before paint, while a resize observer
  follows streamed height growth only while the reader remains at the floor.
  Programmatic positions are tracked separately
  so inertial or deliberate reader scrolling is not mistaken for stream movement.
  The turn-level thinking row is the only ambient progress affordance; no footer
  mirrors active reasoning or tools. Recursive
  dormant DeepSeek code-dispatch metadata stays nested beneath its root call;
  task calls use
  OpenCode's agent-colored delegation card and todo writes are hidden from the
  transcript in favor of the live prompt-dock checklist. Edit/write/patch parts
  use the upstream file accordions and `OpenCodeFile.tsx` / `opencode-file/`
  Pierre renderer: unified diff rows and gutters, syntax tokens, and partial/full
  patch semantics. Its Shiki worker uses the JavaScript regex engine so previews
  render under Vela's existing CSP, which disallows Wasm compilation.
  File metadata retains before/after contents and patch paths;
  write tools show the input file contents. Shell parts render `$ command` plus
  output in one scrollable pre. `OpenCodeSpecialTool.tsx` owns the upstream
  webfetch, search-link, skill, answered-question, and generic trigger layouts.
  Pending questions are hidden; generic tools show the label and up to three
  scalar arguments without a custom input/output body. Copy controls use the
  upstream tooltip timing and positioning. No non-error tool surface renders as
  a bordered card.
  Assistant prose carries no bubble; there is no typing-dot placeholder or
  stream cursor path.
- **Large-session fixture** — `large-session.performance.test.ts` deterministically
  reduces 2,400 events into 400 assistant messages and measures timeline-row
  derivation and retained output. After one warmup, median-of-five proxy budgets
  are 100 ms reducer/update time, 10 ms derived timeline time, at most 1,000
  estimated rows, and exactly one copy of each supplied tool result's output.
  A separate generous 5,000 ms budget measures React reconciliation and 800
  actual rows constructed in jsdom. It does not cover Chromium layout, paint,
  compositor work, or browser memory and is not a browser render budget.
- **Delegation consolidation benchmark** —
  `scripts/benchmark-timeline-consolidation.mjs` compares identical pure
  transcript workloads with absent, available and explicit child-session
  identities, plus ordinary assistant rows. `--ref <commit>` loads the same
  projection symbols from Git; both snapshots check output hashes and input
  immutability under the pinned Node version. It reports sample variability
  and excludes Store updates, React rendering, IPC, layout and paint.
- **Tree normalization** — `filterEntries` hides `HIDDEN_DIRS`; the main
  process `listDir` reads each directory directly from the filesystem
  (`fs.readdir`, with trailing slashes normalized away) rather than
  round-tripping through the opencode service, so the explorer refreshes
  instantly even while the service is busy streaming. Creating a file or
  folder also inserts the new entry into the tree and opens the new file tab
  optimistically in the renderer (`commitName`), with the authoritative
  `refreshTree`/`listDir` reconcile happening in the background.
  Previous product recovery directories, plus the current `.vela-recovery`
  directory, are hidden independently in main and renderer.
- **Explorer selection and moves** — ordinary clicks keep opening files and
  expanding folders. Command/Control-click toggles rows and Shift-click selects
  the visible range. Delete/Backspace and the context menu move selected items
  to recoverable Trash; when a selected folder contains another selected item,
  the nested selection is collapsed so that folder is operated on once. Dragging
  a selected row carries the selected top-level paths to a folder or the
  workspace root. Self drops, descendant drops, same-parent moves, and
  file-onto-file drops are rejected; invalid members of a multi-selection are
  skipped. Main still validates each `shell:fs-move` / `shell:fs-delete` call.
  Successful moves remap `tabs`, `activePath`, and `agentFiles` for that
  workspace.
  `FileSidebar.tsx` owns selection state, drag routing, root composition and
  sidebar navigation. Every open panel retains its own expandable root and
  cached tree in the same scrolling area. `workspaceFiles(workspace)` projects
  file state for each root; directory loading accepts an explicit workspace.
  Tree rows consume a root-local `ExplorerStoreContext` projection, and gestures
  focus the owning panel before invoking focus-based mutation actions. Internal
  moves across workspace roots are rejected because their paths are relative.
  Its internal `ExplorerTree.tsx` owns recursive directory
  and file rows, row actions, inline name input and the context-menu portal.
  Rows receive the sidebar's drag/selection callbacks and consume the existing
  Store/context-menu facade for editor and filesystem actions. Keep gesture
  routing in the sidebar and row/menu presentation in the tree owner; privileged
  operations remain in main behind the controlled bridge.
- **Single-entry drag-and-drop** — every explorer row is draggable; dropping
  onto a folder row moves the entry into it and dropping onto the empty tree
  area or workspace root moves it to the workspace root. The hovered
  destination gets a drop indicator.
- **External drag-and-drop** — OS file/folder drops are accepted with
  a file item/type in `DataTransfer` and routed through the main
  process, never interpreted as explorer moves. Dropping onto a folder row,
  including the current workspace root, **imports** the items into that folder
  (`importPaths` → `shell:fs-import`, then the tree refreshes the destination).
  Dropping files and folders into the remaining space below an expanded
  workspace imports them into that workspace root. With the root collapsed,
  that space opens dropped folders as additional workspace panels (files still
  import into the current workspace). Only the remaining space is highlighted,
  with a destination label; unrelated workspace rows stay unhighlighted.
  The current workspace is a collapsible root row; its entries can be
  hidden with **Remove from Workspace** without deleting their on-disk files; **Delete**
   remains the destructive filesystem action. Dropping files and folders into
   the **editor pane** opens them via `openPaths` (folders become panels, files
   become tabs); outside files save back to their absolute path via
   `shell:fs-write-standalone`. Dragging files into the empty IDE (no
  session yet) opens them via `openPaths`.
- **Split editor tabs** — each open tab can be dragged to the other editor
  group. The drop moves its group membership, activates it in the destination,
  and leaves one copy of the file tab.
- **Recovery notice** — unacknowledged records are shown persistently with
  Open and Acknowledge actions. Acknowledge updates manifest metadata and hides
  the record without deleting bytes. Files and directories both offer inline
  Rename; main applies their distinct mutation policies documented in
  `docs/architecture.md`.

## Components (`src/renderer/src/components/`)

This table is an ownership map, not a prose copy of component implementation.
For pixel-level layout, responsive behavior, and ordinary presentation details,
inspect the owning component/SCSS and tests. Keep details here only when they
express a cross-component invariant or non-obvious state contract.

| Component | File | Responsibility |
|---|---|---|
| `App` | `App.tsx` | Workspace shell and activity rail; panel geometry and focus routing; the rail hosts Sessions/Files views, Terminal, the Changes review view, and browser creation with Settings pinned at the bottom and stays visible in Agent Mode; Settings overlays the mounted workspace so editor group state survives |
| `Titlebar` | `Titlebar.tsx` | Left-panel toggle beside the traffic lights, a slot the focused editor group's tab strip portals into, and the titlebar actions (Agent Mode, add-agent panel, and the right-panel toggle in the corner); the leading block is sized so the strip starts exactly where the left app panel ends, and never narrower than its controls so a collapsed panel cannot clip the toggle; empty strip surface is part of the window drag region while tabs and controls set `no-drag`; both panel toggles are the same mirrored panel glyph pair (dashed while its panel is hidden) and flip with panel state without accent highlighting |
| Launch bootstrap | `launch.ts`, `mount-app.tsx`, `src/renderer/index.html` | Static Folded light art paints before the IDE modules load; three visible seconds precede mounting the IDE, which opens at workspace size without selecting a session |
| `FileSidebar` | `FileSidebar.tsx` | Sessions/Files navigation rendered for the rail-selected view (Files when a workspace opens), Vela header, Explorer, filesystem actions, open-workspace root removal, terminal context actions |
| `ChangesView` | `ChangesView.tsx`, `changes-view-model.ts` | Editor-area review surface activated by the rail: a directory tree of observed file changes beside stacked Pierre diffs with per-file status and +/- counts; omits paths hidden for the focused workspace (detached paths and nested-workspace roots, matching the Explorer), and defers both per-file diff statistics and the Pierre diff until a section nears the viewport (one shared observer) so a large change set never diffs every file on mount |
| `SettingsSidebar` | `SettingsSidebar.tsx` | Settings navigation |
| `SettingsPage` | `SettingsPage.tsx` | The independent Codex/Classic interface chooser and all appearance palettes, including the persisted Experimental art toggle, plugins, providers, safety, voice, default model and OpenCode sync, mobile, and About with a direct GitHub update action, separate status check, and clickable unsaved-file list |
| `ProviderSettings` | `ProviderSettings.tsx` | Runtime-neutral provider connection/status UI; never owns provider secrets |
| `SessionsPane` | `SessionsPane.tsx` | Vela header with session filter, Pinned/Projects/Recents inventory, saved workspaces, session open/close navigation |
| `EditorPane` | `EditorPane.tsx` | Monaco edit views (files always open in the edit surface; diff review lives in the rail's Changes view) with a Codex-style breadcrumb trail (text segments, chevron separators, ghost ancestors, current file emphasized) and unconditional line wrapping, manual save/conflict UI, editor validation entry points, and up to four groups with independent tab membership, selection, and drag transfer; every group's file tabs share the titlebar strip (group blocks in panel order, separated by a hairline) so a split panel never grows a second strip inside itself, and selecting a tab there focuses its group; each group's split/close controls sit at the right end of its breadcrumb row beside the editor status menu (empty groups keep that row so they can still be split); new tabs route to the focused group ; browser tabs share that strip (chips after the file tabs, with their `+` immediately after the last visible browser chip; file-only strips omit it and the rail opens the first browser) and while one is showing the editor keeps its mount behind a concealed wrapper |
| `BrowserPane` | `BrowserPane.tsx`, `use-browser-guest.ts`, `use-browser-tabs.ts` | Browser tab surface: a navigation row (back / forward / reload-or-stop, "Search or enter a URL" field that turns bare hosts into `https:` and everything else into a search) over a `<webview>` guest in the `vela-browser` partition. Guests are created lazily — a new tab is pure state with no guest at all — so opening one costs nothing, and a loading veil covers the frame until the page stops loading; `use-browser-guest` folds guest events into the browser slice |
| `BrowserTabsProvider` | `browser-tabs-context.tsx`, `browser-store-provider.tsx` | Browser tab state lives outside the app store, split into a *state* context (tab list + active tab, consumed by the strip and panes) and a stable *actions* context (consumed by the rail, the editor and the store). That split is a responsiveness contract, not a detail: when the slices lived in the app store, opening or closing a tab re-rendered every store consumer (measured 644ms with nine editors open; 2ms after the split)
| `EditorTabs` | `EditorTabs.tsx`, `editor-groups.ts` | Tab strip for the editor groups: tabs share the strip width (90px floor, 240px ceiling) and overflow past the floor moves into an "all open tabs" menu button; a preview tab (`preview: true`, italic) marks a single Explorer click as transient so the next preview replaces it, while reopening the file, editing it, or pinning it removes that marker; each tab resolves its owning group (drag source and selection target) through the strip's group lookup; owns the tab drag MIME and the editor-group identity helpers |
| `AgentPanel` | `AgentPanel.tsx` | Session-owned GUI/TUI surface, timeline, composer, model/agent controls, usage/status; its header keeps the turn timer, the back control for delegated sessions, and the panel options menu (context usage/session/mode/close) behind a horizontal-dots control; the empty GUI view shows optional station art above "Tell the agent what to work on." when Experimental art is enabled, centers the prompt without art when disabled, and clears both after the first user/assistant turn; Shift+Tab cycles agents, Shift+P cycles favorite models, and Ctrl+Tab cycles response strength without intercepting typed Shift+S |
| `Composer` | `Composer.tsx` | Authoritative local prompt draft, attachments, keyboard dispatch and agent controls; takes `controls` (extra control-row slot, used for the session usage ring) and `overlay` (popover anchored to the input); completion and model-menu behavior have the owners below; the agent input uses one rounded surface with a bottom toolbar (attachment and compact agent control on the left; model, available response strength, and send on the right); at minimum panel width the model and response strength remain visible while their labels can truncate |
| `AgentTui` | `AgentTui.tsx` | xterm view for the active runtime's PTY-backed TUI |
| `OpenCodeTimeline` | `OpenCodeTimeline.tsx` | Runtime-neutral chronological rendering of assistant text/tools/delegation with hidden reasoning |
| `OpenCodeTodoDock` | `OpenCodeTodoDock.tsx` | Structured todo/checklist state near the composer |
| `AgentTray` | `AgentTray.tsx` | Collapsed agent-panel affordance and session activity indication |
| `TerminalTray` | `TerminalTray.tsx` | Integrated terminal tabs backed by main-process `node-pty` |
| `ServerSettings` | `ServerSettings.tsx` | Settings inventory of active Vite preview servers with per-server and stop-all controls |

`ThemeProvider` owns the app-wide appearance state, including the persisted
Experimental art preference. `AppearanceSettings` changes the preference and
the agent panel consumes it immediately.

`AgentPanel.tsx` composes the session surface and owns GUI/TUI selection and
prompt-dock placement. `Composer.tsx` owns the sole input/files/mentions draft;
`composer-completions.ts` owns candidate acquisition, insertion and captured
workspace/sequence guards without keeping a second draft. `ComposerModelMenu.tsx`
owns model preference persistence, filtering, response strength and menu rows;
keyboard model cycling uses that same owner. `AgentUsage.tsx` owns the Context menu entry and usage popup, quota presentation,
refresh lifecycle and turn timer. Context lives in the panel options menu; its
popup is anchored below the header and is available in GUI and TUI modes. The
composer contains only its input and drafting controls.
`agent-scroll.ts` owns remount scroll memory, reader intent and content following;
`use-menu-dismiss.ts` owns the shared document mouse/Escape subscription.
Related changes belong in those owners; they use renderer state and the controlled
preload bridge and do not perform privileged filesystem or runtime work.

`App.tsx` retains mode/session slot memory and shell composition. Pure panel
geometry lives in `agent-panels.ts` without React or Store dependencies;
`layout/PanelColumn.tsx` owns column presentation, and
`layout/panel-interactions.ts` owns drag/tray previews and their final state
commits. Layout features enter through `App.tsx` and use those owners rather
than introducing another geometry or resize implementation.

The terminal tray's server button passes the active workspace-relative HTML
tab to Vite when available, so nested standalone sites get the correct document
root. Startup is considered successful only for a reachable non-error page;
HTTP failures and bounded Vite stderr are shown in the tray instead of the
generic startup notice. Each workspace/page target runs its own preview server:
clicking the button toggles the server for the current page, and right-clicking
lists every running server for the workspace with per-server Stop (plus Stop all
when several run). The Settings **Servers** tab lists every running server
across workspaces with Stop and Stop all, and main stops all preview servers
when the app quits.

Key cross-component invariants:

- Every open Explorer root's context menu removes the panel identified by that
  root; it never deletes files or removes a saved-workspace bookmark.
- Every panel acts on its own session/workspace identity; background panels keep
  streaming independently.
- Opening a file/tool link from a background panel must address/focus the owning
  workspace rather than whichever panel was previously focused.
- Agent GUI and TUI are alternate views of the same session, not separate
  sessions. Approvals and forms are interactive, so the active surface owns
  them: the embedded TUI renders its own prompts in the terminal, and the panel
  dock is GUI-only — rendering it above the TUI asks the same question twice
  (the question skill is the visible case). Pending requests stay in session
  state and reconcile every three seconds plus stream reconnect so a missed
  `form.created` event cannot leave the agent waiting without a GUI card. A
  form whose owner is not an open session is routed to the panels open at the
  event's location using main's `velaSessionIDs`, so a location-global form, a
  delegated child, or an external `opencode` TUI question surfaces in the GUI;
  the form keeps its true session id and replies carry it so the answer reaches
  the owning session. A temporary list failure preserves the existing card.
  Replies carry the form's session (or location) context.
  Pending requests remain in panel state regardless of active surface, so they
  appear in the dock when the panel returns to the GUI view.
- Timeline order comes from the authoritative chat/session state; components do
  not invent a second activity history.
- The empty-state station mark is decorative and GUI-only: it never owns
  pointer events, carries no text alternative, and is absent from the embedded
  TUI.
- Workspace-dependent actions remain inert when no workspace is active.

## Styles (`src/renderer/src/styles/`)

`_opencode-chat.scss` owns the ordered integration of the pinned upstream
partials and Vela's transcript overrides. Prompt, permission/form/revert,
todo and compact composer presentation lives in `_agent-prompt-dock.scss`;
the transcript owner includes its mixin at the existing cascade position.
`_agent.scss` owns the optional empty-state station mark and its prompt line.
Light transcript surfaces use `agent-station-light.png`; dark surfaces use
`agent-station.png`. Disabling Experimental art removes both marks, leaving the
prompt centered, and suppresses Observatory scene art in the agent panel.
Keep `main.scss` ordering explicit. Move related rules together rather than
creating per-selector partials; inherited palette overrides are intentional.

`app-styles.ts` subscribes to app-style updates from the root shell and swaps
only the application stylesheet link after the replacement loads successfully.
The store exposes its existing notification owner as `notify` so compilation
and loading failures use the shared toast surface.
Failed or superseded loads keep the last valid sheet. This changes no editor,
session, or document state and does not reload the renderer.

`main.scss` is the single renderer stylesheet entry and uses ordered Sass
partials so the cascade stays explicit. Vela's runtime-neutral chat tokens,
slots, typography, row geometry, and animations live in
`_opencode-chat.scss`; other component rules are owned by `_sidebar.scss`,
`_editor.scss`, `_agent.scss`, `_composer.scss`,
`_welcome.scss`, `_settings.scss`, `_sessions.scss`, and `_terminal.scss`; app-wide rules are separated into
`_foundation.scss`, `_layout.scss`, `_buttons.scss`, `_toasts.scss`, and
`_scrollbars.scss`. CSS source maps are preserved in development and compiled builds, so
DevTools links inspected rules back to the partial and source line rather
than a generated `<style>` block or bundled CSS location. See the
[main-process source-link flow](main.md) for resolution and build ownership. Runtime layout
measurements are passed as inline CSS custom-property values; their actual
presentational declarations remain in the owning SCSS partial.

Terminal input flows: keystrokes → `terminalInput(id, data)`; output
streams back via `onMessage` (`terminal-data`). The renderer measures the
visible terminal viewport and supplies its initial columns/rows to
`terminalStart`, so the shell's first prompt is laid out at xterm's width;
after startup, the xterm `fit` addon + `ResizeObserver` keep the PTY dimensions
in sync (`terminalResize`), and the
agent TUI re-sends its fitted size once `agentTuiStart` resolves — a resize
that arrives before the PTY is registered is dropped by the terminal manager,
which would leave the TUI drawing only the spawn-default rows.
Agent TUI input uses the same terminal message stream and ownership checks, but
starts the active runtime command through `agentTuiStart` in the panel's
workspace directory. OpenCode uses `opencode --session <session-id>`; the
dormant DeepSeek runtime is not available to panels.
Appearance and interface are separate axes. The selected *appearance* only
supplies colours (and the scheme); the selected *interface* (`vela.ui`,
`data-shell`) decides the chrome: `codex` is the new flat shell — icon rail, tab
strip in the top bar, docked agent panel, browser tabs — and `classic` is the
original card chrome. Every appearance works in both interfaces, and
`_codex.scss` is scoped to `[data-shell="codex"]` rather than to the Codex
palettes. `applyAppearance` layers the shell's tokens (flat workspace, composer
border, send colours, dark file-tree glyphs) over the palette and removes any
token the new combination no longer sets, so switching palettes or interfaces
cannot leave stale inline variables behind.
The Observatory appearance also supplies a decorative pixel-art observatory
backdrop and violet pixel wordmark. The backdrop covers the panel, frames the
observatory in narrow panels and reveals the galaxy in wide Agent Mode. It dims
behind conversations and is absent in the TUI; prompts, scrolling and menus
retain their existing owners. These decorations are scoped to the appearance
and work in both interfaces.
The selected appearance supplies the color palette for both embedded xterm
surfaces. Prism uses alpha-backed violet panels; Kitty Glass restores the
transparent glass surfaces and terminal backgrounds of the earlier app, and
Original Dark restores its warm charcoal palette. All appearances retain the
same workspace layout, with independent interface chrome. The window and native chrome
track each profile's native appearance: the renderer reports it through
`setAppearance` on boot and on every profile change so macOS `under-window`
vibrancy follows the palette's light or dark scheme.
The renderer generates and registers each validated terminal UUID before
invoking `terminalStart`, so startup output or exit can be attributed even when
it arrives before the invoke resolves. Only those pending IDs can buffer startup output. Buffers retain at most 64 chunks / 256 KiB for ten
seconds and are cleared on exit, close, registration, or workspace reset.
Closing the final tab commits an empty tab state before hiding the tray;
reopening shows that empty state and requires the explicit `+` action to start
a new process. A natural final exit leaves the empty tray visible.
Default terminal tab numbers follow the current left-to-right tab order, so
closing earlier tabs renumbers the remaining default tabs from one. Folder
terminals keep their folder names.
With shells open, the leftmost header tab selects the first shell rather than
acting as a separate panel selector. Selecting any shell tab returns from
Problems and focuses its input; `+` also returns to the terminal view.
Explorer context-menu requests add and focus a terminal whose cwd is the selected folder or a selected file's parent, and expand the normal bottom tray.
When renderer HMR is newer than the still-running main/preload process, a missing resolved-cwd response triggers one safely quoted `cd`/`Set-Location` command through the existing terminal input channel, so folder terminals work without restarting Vela.
The tray is toggled from the titlebar (⌥O) and drag-resized via the
`tray-divider`. Dragging the divider down to the bottom of the window
shrinks the tray to a 26px minimum and closes it only when the mouse is
released at that collapsed position.

## Monaco (`monaco.ts`)

- Workers wired for editor/json/css/html/ts (`?worker` imports).
- Codex profiles include Pierre-style syntax slots and language-specific token overrides; other profiles retain per-slot fallbacks. Comments use plain rather than italic text.
- Startup migrates old product-prefixed local-storage keys before persisted preferences initialize. The `vela.theme` color profile defaults to Codex when absent; saved palette IDs remain compatible. The independent `vela.ui` preference defaults to `codex`, with `classic` restoring card chrome. Shell selection publishes `data-shell`; palette selection supplies colors. Switching either removes obsolete inline tokens before applying the new combination.
- `vela-${appearanceId}` themes (diff insert/remove colors included) for all persisted color profiles. Saved `kitty` and `original` selections keep their IDs rather than mapping to Blueprint or Control Deck. Monaco
  parses theme palette colors with `Color.fromHex`, which silently maps any
  non-hex value to pure red — every palette color must be hex
  (`#RRGGBB` or `#RRGGBBAA`), never `rgba()`.
- Monaco's editor context menu is rendered outside its themed editor node, so
  `ThemeProvider` publishes its menu foreground/background colors on `:root`.
- Token rules come from each profile's syntax slots: comments (plain),
  keywords/operators, strings, numbers/constants/predefined, types/variables/functions,
  attributes, tags/meta, and JSON `string.key`/`string.value` scopes. The Codex
  profiles add optional `syntaxVariable`/`syntaxAttribute` slots; other profiles
  fall back to their function/number slots.
- Editor line wrapping is always enabled; there is no settings or toolbar toggle.
- `languageForPath()` — extension → Monaco language map (fallback
  `plaintext`).
- JavaScript uses Monaco's TypeScript worker with eager model synchronization,
  ES2020 and browser DOM libraries, `allowNonTsExtensions` for editor model
  URIs, syntax/semantic diagnostics, and explicit
  quick-suggestion/trigger-character settings in `EditorPane`. Typing after
  `document.` offers DOM API completions such as `querySelectorAll`. Suggestion
  and hover widgets use a Monaco-styled body-level overflow host so split-pane
  clipping cannot hide them. Visible suggestions retain a selected item for
  Arrow Up/Down navigation and Tab/Enter acceptance. JavaScript/TypeScript, HTML, and
  CSS use Monaco's language-service completions, including pseudo-selector and
  syntax details; local dirty editor buffers stay authoritative during typing
  so React state echoes cannot replace an active model between keystrokes.
- PHP offers common language keywords and built-in functions through a Monaco
  completion provider; Monaco also suggests words from the current document
  for PHP and other languages without bundled language services.
- HTML attribute `=` inserts a paired quote with the caret between them, and
  Monaco's language-defined quote pairing remains enabled elsewhere.
- `editor-navigation.ts` handles Command/Control + Arrow keys on the editor
  DOM node that received the event: Up/Down move to document start/end and
  Left/Right move to the literal start/end of the current line. Pane-local
  handling prevents split editors from routing a shortcut to another editor.
- CSS worker diagnostics stay enabled (the only language worker with a
  working `doValidation`), so CSS files lint inline exactly like VS Code.
  The HTML worker ships no `doValidation`, so HTML files have no built-in
  structural markers — which also matches VS Code.
- Emmet is registered through `emmet-monaco-es` for HTML and CSS/SCSS/LESS.
  Typing an abbreviation shows up as a snippet completion (as in VS Code),
  and `emmet.ts` makes expansion deterministic: pressing **Enter** (or Tab)
  at the end of a recognized abbreviation — including `!` at the top of an
  HTML file — replaces it with the expanded markup through Monaco's snippet
  controller, so the caret lands on the first tab stop (the `<title>` text
  for `!`) when the suggestion list is closed. The key handling lives in
  `emmet-keys.ts` (`wireEmmetKeys`, called from the `EditorPane` edit editor's
  `onMount`) and yields Tab/Enter to Monaco while a completion popup is open. It is
  skipped inside a `<script>` block or while a snippet session is active.
  `emmet.ts` mirrors the
  provider's noise checks (unknown tags, unresolved CSS values, unknown
  `.class` fragments) so Enter/Tab only expands genuine abbreviations.

## Editor status menu and W3C validation

- The status bar's right side reports the focused Monaco editor's line and
  column, indentation width, and UTF-8 encoding. Cursor updates from an
  unfocused split group do not replace the focused editor's status.

- The W3C checkers never run automatically. The **Validate** control lives in
  the editor's status menu (the "⋯" button at the right end of the breadcrumb
  row); it is disabled with an explanation unless an
  HTML, CSS, or SCSS tab is active. HTML and CSS use W3C; SCSS uses diagnostics
  from Monaco's Sass language service because the W3C CSS Validator cannot
  parse preprocessor syntax.
- For HTML and CSS, `EditorStatusMenu` sends the active tab's content through
  `window.vela.validateW3c` and applies returned diagnostics as `w3c`-owner
  Monaco markers. For SCSS, it reads Sass diagnostics from Monaco's marker
  service. Both paths show error and warning counts (or a failure state) beside
  the button. The results also open
  the terminal area's **Problems** view with file context, severity, message,
  and line/column; clicking a diagnostic opens and focuses the file at the
  reported line and column after its Monaco editor has mounted. After a result,
  the status control toggles error markers with **Show errors** and **Hide errors**.
- CSS validation posts the stylesheet as a form body to the W3C CSS Validator's
  SOAP 1.2 endpoint and parses its structured errors and warnings. This avoids
  unsupported output formats and long source code in a URL.
- W3C markers are cleared whenever the file content changes or the tab closes,
  so stale line numbers never linger while editing.
- About shows the running app version and build flavor. Local development/test
  builds have no GitHub updater controls and make no release-check requests.
  Release builds compare their version with the latest published GitHub
  release. **Update Vela** downloads that packaged release, shows progress in
  the standalone updater window, then closes Vela while `electron-updater`
  installs and relaunches the new bundle. **Check for updates** only checks the
  release version. Pushing a commit does not publish an app update; a matching
  version tag must create a GitHub release. The updater never modifies a source
  checkout or user workspace. Its popup retains progress after reload attempts
  and provides a Return to Vela action after errors. Unsaved editor files are
  listed with buttons to open each one and block both update and app close. A tab's dirty marker is
  derived from whether its buffer differs from its saved content, so undoing to
  the saved text clears the marker.
- The main process calls the Nu Html Checker for HTML and the W3C CSS
  Validator for CSS, then returns diagnostics. Network failures leave the
  editor unchanged; Vue and Svelte files are not sent to the validators.

## Entry

`src/renderer/index.html` owns the initial inline Folded light splash and critical
styles. Its geometry and mint palette mirror `src/renderer/src/assets/folded-light.svg`,
with the Vela wordmark and small “integrated development environment” caption.
The original gentle four-second float carries the folded planes and the clipped
Dawn Terminator light sweep (motion study 23) in one SVG group, keeping the light
aligned with the moving logo. Critical inline CSS starts the light sweep with a
half-second delay, independently of JavaScript loading. A narrow light band enters
from outside the left edge, fades in smoothly, crosses the full mark at constant
speed, and fades out beyond the right edge. Both gradient ends are transparent
so SVG gradient padding cannot flash or darken the entire logo. Its 2.5-second
pass fits inside the unchanged three-second launch hold. Reduced-motion preferences disable both motions. The wordmark and ground
shadow remain still, and inline artwork paints before the launch module runs.
The local `design/experimental-art/experimental-landing-page/launch-comparison.html`
preserves the earlier Vela Folded light and app-icon comparison frames. The 380 × 320 native landing window
appears before React, Monaco or backend readiness. `launch.ts` waits for a
visible paint, loads the IDE modules in parallel with a three-second
hold, and hands off only when both finish. `mount-app.tsx` removes the static
splash when React commits. `App.tsx` always mounts the IDE, requesting workspace
window sizing even when no session is active; the explorer retains workspace
selection. Live-context recovery on renderer reload and explicit Finder/Dock
open requests keep their existing store ownership.

`main.tsx` starts the launch coordinator and dynamically imports `mount-app.tsx`.
`App` owns the theme/browser providers, with `BrowserStoreProvider` mounting the
renderer store. `global.d.ts` types `window.vela` from the preload API.

Agent Mode is a temporary panel view. Normal coding mode renders only the
focused coding session. Entering Agent Mode seeds its panel set with that one
session; additional active sessions are loaded explicitly from the agent
panel's three-dots menu, up to eight visible panels. Three or more panels use
a two-row grid that expands from two through four columns. Exiting Agent Mode resets the temporary set to the
currently focused session, while all other active sessions remain available in
the menu and continue running in the backend. Selecting a hidden session in
normal mode switches the single coding workspace; selecting one in Agent Mode
attaches it to the visible panel set and focuses it.

## Workspace Sass controls

`use-sass-watch.ts` projects the currently selected immutable workspace's local
Sass state; previous-workspace events and stale revisions cannot replace it.
`EditorStatusMenu` exposes the Watch Sass toggle and `TerminalTray` adds
Sass Output alongside Terminal and Problems. The watcher compiles only disk
contents after explicit saves; its state does not affect editor persistence.
See [Sass configuration and lifecycle](sass.md).

### Integrated browser and editor ownership

Browser state/actions contexts are separate from the application Store. A stable
`onFileActivate` capability enters the Editor owner through `BrowserStoreProvider`;
successful foreground file activation exits browser mode, while background,
failed and stale requests do not. Guests are created lazily, including tabs
opened with an initial URL; main-frame failures survive stop-loading and
subframe failures do not replace the page error. Browser and file tabs share
an accessible overflow menu, including at narrow widths. Inactive guests and
editors remain mounted until closed. Split-group status menus receive their
own tab identity; validation never borrows another group's focused file.

Directory hover prefetch belongs to WorkspaceFiles, not Store. It deduplicates
workspace/generation/path requests, retains current cache results, never expands
a directory and ignores closed, stale or unmounted responses. Normal expansion
remains the owner of visible loading/error behavior.
