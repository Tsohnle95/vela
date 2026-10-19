# Module: shared types

> **Document role:** canonical owner for shared TypeScript contracts that cross main/preload/renderer boundaries.

`src/shared/types.ts` — the contract between main, preload, and renderer.
Imported everywhere as `@shared/types` (alias in both tsconfigs and
`electron.vite.config.ts`).

`src/shared/session-presentation.ts` owns the common todo normalization and
finite usage-field construction used by main replay and renderer state.
`normalizeTodos` retains source-index fallback ids, string priorities, and
pending status defaults. `sessionUsageValues` consumes an explicitly captured
cache snapshot; callers retain their envelope/token acceptance guards and read
order. It does not apply renderer validation policy to backend responses.
The separately established reopen usage constructor retains its distinct cache
read semantics in main. These are internal projections of existing contracts,
not new IPC payloads.

## Domain types

| Type | Shape | Used for |
|---|---|---|
| `RuntimeCapabilities` | `{ attachments, commands, models, agents, permissions, providerCredentials, sessionFork, sessionResume, steering, tui }` | Runtime-declared feature availability; `tui` controls whether the agent panel offers its embedded terminal interface |
| `OpenCodeSyncResult` | `{ version, previousVersion, cliUpdated }` | Result of a Settings sync/update action; reports the final CLI version and whether the CLI itself changed |
| `VelaAppUpdateStatus` | `{ state, currentVersion?, latestVersion?, message? }` | Published GitHub release status for a release build, or disabled status in local development/test builds |
| `VelaAppUpdateResult` | `{ ok, updated, currentVersion?, latestVersion?, message }` | Result of downloading a published Vela release for installation |
| `W3cDiagnostic` | `{ line, column, endLine, endColumn, message, severity, source }`; source is W3C HTML/CSS or local Monaco SCSS | Validation results passed to Monaco markers and the Problems view |
| `SassWatchState` | `{ workspace, directory, enabled, status, revision, error?, logs }` | app-owned Sass state for a captured workspace; status is off/starting/on/error; revision prevents stale UI replies |
| `SassWatchLog` | `{ time, level, message }` | bounded Sass Output entries; info/warning/error |
| `WorkspaceIdentity` | `{ id, generation }` | Opaque immutable UUID plus monotonic generation minted for one session context; capability calls must echo both; reused while a session stays open so file-updates and editor state keep routing to the same workspace |
| `SessionInfo` | `{ id, directory, workspace, parentID?, title?, agent? }` | An open OpenCode session (one per panel), immutable workspace identity, and parent/child navigation metadata |
| `SessionSummary` | `{ id, title, directory, updatedAt, parentID?, agent? }` | Recent-session graph used by Welcome and task/subagent links |
| `TreeEntry` | `{ path, type: "file" \| "directory" }` | Explorer tree nodes; `path` is `/`-relative, no trailing slash |
| `RecoveryRecord` | `{ id, artifact, originalPath, recoveryPath, createdAt, acknowledged, reason }` | Actionable durable save/rename artifact; acknowledgment does not remove bytes |
| `FileBaseline` | `{ kind: "known", content, exists? } \| { kind: "unknown" }` | First established pre-change state; existing-file baselines omit `exists`, while `exists: false` represents a path absent from Git or a newly created file; unknown never substitutes post-change bytes |
| `FileUpdate` | `{ workspace, sessionID, path, movedFrom?, baseline: FileBaseline, content: string \| null, deleted }` | Identity-bound observed workspace change payload; `movedFrom` is present only when main confirms that an opened file's filesystem identity moved to `path`; the renderer derives clean state from content and baseline existence |
| `ProjectInfo` | `{ directory, name }` | Saved workspace bookmark or validated OpenCode project on Welcome |
| `SavedWorkspaceSnapshot` | `{ workspaces, initialized }` | Durable saved workspace list plus a marker distinguishing first-run migration from a user-saved empty list |
| `ModelOption` | `{ id, providerID, name, variants?, variant?, limit? }` | Model picker options + current model/strength selection; `limit = { context }` is the model's total context-window size from the catalog, used for the usage popup's context fill |
| `AgentOption` | `{ id, name }` | Agent picker options + current selection |
| `CommandOption` | `{ name, description?, kind? }` | Slash commands and skills offered by `/` completion and run via `runCommand` (`kind: "command" \| "skill"`) |
| `ReferenceOption` | `{ name, path, rel, description? }` | `file.find` search results for `@` mentions; `path` is absolute, `rel` is relative to the session directory |
| `PromptFile` | `{ path, mention?: { start, end, text } }` | Files attached to a prompt; `mention` marks the `@rel` token span in the prompt text |
| `SessionSelection` | `{ model?: ModelOption, agent?: AgentOption }` | Restores the addressed session's model/agent picks |
| `ApprovalMode` | `"ask" \| "approve"` | Composer permission behavior; approve mode replies `once` automatically |
| `UserAttachment` | `{ name }` | Attachment chip rendered on a submitted user prompt |
| `TerminalData` | `{ id, data }` | PTY output chunk (`terminal-data` message) |
| `TerminalExit` | `{ id, exitCode }` | PTY exit (`terminal-exit` message) |
| `PermissionReply` | `"once" \| "always" \| "reject"` | Permission card buttons |
| `ProviderUsageResult` | `{ provider, displayName, status, snapshot, error? }` | Per-provider usage snapshot for the composer usage indicator (`fetchProviderUsage` in `src/main/provider-usage.ts`); `status` is `"ok" \| "stale" \| "unavailable" \| "unauthenticated" \| "unsupported"` |
| `ProviderIntegration` | `{ id, name, keyMethod, credentials, environment, oauth }` | Provider-neutral settings catalog from the active runtime; contains setup metadata and opaque credential labels but never secret values |
| `ProviderFormField` | `{ key, type, title?, description?, required?, placeholder?, options?, default? }` | Runtime-declared provider setup field rendered beside the write-only API key input |
| `ProviderCredentialAnswers` | `Record<string, string \| number \| boolean \| string[]>` | Bounded provider-specific setup values sent with a credential connection request |
| `PendingFormRequest` | `{ id, sessionID, title, fields }` | Interactive agent form; `sessionID` is the owning session or `"global"` for a request scoped to the panel's workspace location |
| `ReopenedSession` | `{ session, transcript, todos, usage: SessionUsage \| null }` | `openSessionById` result; `usage` is the cumulative `{ cost, tokens }` from `session.get`, also streamed live via `session.usage.updated` |
| `ExternalOpenResult` | `{ kind: "relative", rel, content } \| { kind: "standalone", path, content }` | `openExternal` result: relative resolves to an in-workspace file (open normally), standalone to an absolute writable file outside the workspace root |
| `ExternalKind` | `{ kind: "file" \| "directory" \| "missing" }` | `statExternal` result used to route a mixed file/folder drop (files → standalone tabs, folders → imports) |
| `OpenFileWorkspaceResult` | `{ session, path }` | `selectFile` / `openFileWorkspace` result: the opened single-file workspace session plus the workspace-relative file to open |
| `ImportResult` | `{ name, rel, imported, reason? }` | Per-source `importExternal` outcome; `imported: false` with a `reason` (not found, already in the workspace, already exists, size caps) |
| `SessionUsage` | `{ cost, tokens: { input, output, reasoning, cache: { read, write } } }` | Cumulative session token usage/cost shown in the agent header usage popup; the popup derives context-window fill from `tokens.input` vs the active model's `limit.context` |
| `ProviderUsageSnapshot` | `{ windows: UsageWindow[], credits, planType, updatedAt }` | Usage windows (`{ id, label, usedPercent, windowMinutes, resetsAt }`, where `usedPercent` is consumed quota and the UI displays its complementary `% left`) plus credits (`{ hasCredits, unlimited, balance, ... }`) or an `error: { code, message, retryable }` |

## TranscriptItem (agent panel feed)

Union discriminated on `kind`:

- `user` — `{ id, text }` prompt bubble
- `pending-input` — internal admitted user/synthetic input retained until
  promotion or cancellation; never rendered as a chat row
- `assistant` — `{ id, messageID, parts, completed, retry?, error? }`;
  `parts` is the ordered OpenCode content stream (`AssistantPartView[]`)
- `permission` — `{ id, requestID, action, resources, pending }`
- `selection` — retained agent/model switch metadata, filtered from the chat
  timeline
- `synthetic` / `system` / `skill` — non-assistant protocol messages with their supplied text;
  system prompt and `<system-reminder>` entries are retained but filtered from
  the visible timeline
- `shell` — a session shell command plus running/terminal state, output, and exit code
- `compaction` — running/completed/failed compaction with its streamed summary
- `status` — `{ id, text, tone: "info" \| "success" \| "error" }`
- `divider` — `{ id }` visual separator per execution

User, pending input, and assistant items retain optional `MessagePresentation`
metadata: agent, provider/model identity, creation/completion timestamps, parent
message identity, and interruption state. The renderer uses these server values
for the message footer; hydration and history merges preserve them. OpenCode V2
user messages inherit the responding assistant's agent and model, matching
upstream normalization. Missing timestamps remain absent instead of producing
an invented duration.

`AssistantPartView` is discriminated on `kind`: `text` and `reasoning`
parts carry `{ id, text, complete }`; `tool` parts carry
`{ id, tool: ToolCallView }`. Keeping these parts in event order lets the
panel render reasoning, tools, and responses exactly where OpenCode emitted
them instead of flattening each category into a separate block.

`ToolCallView`: `{ id, title, detail, status, input?, inputValue?, output?,
content?, progress?, startedAt?, duration?, paths?, metadata?, executed?,
providerState?, providerResultState? }` — `input` is the live argument buffer,
`inputValue` preserves parsed protocol input, `content` preserves text/file
blocks, and provider metadata is retained instead of flattened away.

## UI state

- `Tab` — `{ path, name, content, saved, baseline, deleted, dirty, stale,
  revision, conflict, binary, preview?, standalone? }`; `saved` is the last
  persisted content, `revision` increases on every edit, and `conflict`
  retains external content while normal saving is blocked. `preview: true`
  marks a transient preview tab (a single Explorer click; the next preview
  replaces it until it is opened again or edited). `standalone: true`
  marks a tab whose `path` is an absolute filesystem path outside the
  workspace root (a dragged/dropped or standalone file): its content is read
  and saved at that real path instead of through the workspace-relative read/write
  channels.
- `FileWriteIdentity` — `{ id, workspaceID, revision, expectedContent,
  overwrite }`; binds a write and its `file-update` echo to the exact revision
  and provides the normal-save disk precondition.
- `AgentFileState` — `{ baseline, content, deleted }`, per observed changed
  file; baseline survives editor saves.

## IPC envelope

`BackendMessage` is a discriminated union on `kind`:

- `"event"` / `"file-update"` / `"session"` / `"recovery"` — shared base
  (`BackendMessageBase`): `{ kind, type?, data?, file?, session? }` plus
  `{ kind: "ui-command", command }` (main→renderer requests, e.g.
  `open-source` with `{ path, line }` when a DevTools CSS source link is clicked — `path` is a
  canonical absolute application-source path opened through the renderer's
  external-file workflow). `reload-app-styles` carries `AppStylesheetUpdate`
  (`src/shared/app-styles.ts`) in `data`; `app-styles-error` carries a displayable
  compilation error string. App-style updates identify the original app link,
  replacement URL, and revision; they do not describe workspace compiler output.
  Recovery messages carry `{ workspace, records }` so renderer state rejects
  records emitted for a closed or replaced workspace.
- `{ kind: "terminal-data", terminal: TerminalData }`
- `{ kind: "terminal-exit", terminal: TerminalExit }`

This is the wire format for `shell:message` from main → renderer.

## Rules

- Keep IPC-shaped types here so main and renderer compile against one
  contract. Change a type here and both sides of the bridge update.
- Treat `WorkspaceIdentity` as an opaque activation capability. A new
  activation mints a fresh identity; re-activating an already-open session
  reuses its identity so renderer state survives the reopen.
- Generations are minted when main accepts the request. A replaced context
  invalidates its in-flight work; stale completions are discarded.
- Never import renderer or main code from this file — it must stay
  dependency-free (types only).
