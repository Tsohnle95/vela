# Module: preload bridge

> **Document role:** canonical owner for the renderer-to-main preload bridge contract (`window.vela`).

`src/preload/index.ts` — a thin `contextBridge` layer that exposes the
renderer's only gateway to the main process: `window.vela`.

The API object is typed as `VelaApi` (`typeof api`) and is the type
source for `src/renderer/src/global.d.ts`. If you add an IPC channel in
the main process, add the wrapper here and it flows to the renderer
automatically.

## Contract

| Member | Type |
|---|---|
| `platform()` | `string` — `process.platform` (not an invoke; the renderer uses it for the darwin titlebar inset) |
| `isPackaged()` | `boolean` — true when main added the `--vela-packaged` flag; the renderer hides install affordances in packaged builds |
| `isReleaseBuild` | `boolean` — true only when the release pipeline compiled the published-app updater into this build; local builds never run GitHub release checks |
| `appVersion` | `string` — version reported by the running Electron app, passed to the trusted preload as a startup argument |
| `onMessage(cb)` | `(msg: BackendMessage) => void`, returns unsubscribe |
| `selectFolder(generation, runtimeID?)` | `Promise<SessionInfo \| null>` — native dialog; opens through OpenCode, and the caller decides mounting (replace panels, add a model panel, or swap an existing panel's directory) |
| `selectDirectory()` | `Promise<string \| null>` — native directory dialog that returns a canonical folder path without creating a runtime session; used only to save a Vela workspace bookmark |
| `selectFile(generation, runtimeID?)` | `Promise<OpenFileWorkspaceResult \| null>` — native single-file dialog; opens the file's parent folder through OpenCode and reports which file to open |
| `openFileWorkspace(file, generation, runtimeID?)` | `Promise<OpenFileWorkspaceResult>` — opens an absolute path as a single-file workspace backed by OpenCode (parent folder session + the file to open), no dialog |
| `openSession(dir, generation, runtimeID?)` | `Promise<SessionInfo>` — creates an OpenCode session; non-OpenCode runtime ids are rejected |
| `runtimes()` | `Promise<RuntimeManifest[]>` — OpenCode's version and normalized capability manifest; DeepSeek is currently dormant |
| `syncOpenCode()` | `Promise<OpenCodeSyncResult>` — syncs the attached shared service to the exact version resolved from Vela's PATH |
| `updateOpenCode()` | `Promise<OpenCodeSyncResult>` — runs OpenCode's own updater, then syncs the attached service |
| `checkAppUpdate()` | `Promise<VelaAppUpdateStatus>` — release builds check the latest published Vela release; local builds return disabled without network access |
| `updateApp()` | `Promise<VelaAppUpdateResult>` — downloads the published app update in release builds and starts the popup; main enforces the unsaved-file guard; local builds return disabled |
| `reportUnsavedEditorFiles(paths)` | `Promise<void>` — reports display paths for currently dirty editor tabs so main can block close/update and identify the affected files |
| `sassWatchState(workspace)` | `Promise<SassWatchState>` — restores and reads an opened workspace's Sass watcher state and logs |
| `setSassWatch(workspace, enabled)` | `Promise<SassWatchState>` — starts or stops local disk compilation for the captured workspace identity |
| `onSaveAllEditorFiles(save)` | `(save: () → Promise<boolean>) → unsubscribe` — handles main's Save All request and returns the save result through the trusted bridge |
| `sessions()` | `Promise<SessionSummary[]>` — recent session list |
| `activeSessions()` | `Promise<SessionInfo[]>` — currently open backend sessions in activation order; the last element is the most recently activated (used for startup restore) |
| `closeSession(workspace)` | `Promise<void>` — tears down the backend context when a panel closes; the opencode session remains reopenable |
| `openSessionById(sessionID, generation, runtimeID?, allowWorkspaceRelink?)` | `Promise<ReopenedSession>` (session + replayed transcript + cumulative `usage`); idempotent for already-open OpenCode sessions. Set `allowWorkspaceRelink` only for an explicit user action to let Vela ask for the replacement folder when the previous one is missing; startup hydration leaves it false. Legacy DeepSeek sessions cannot be reopened by the dormant adapter |
| `deleteSession(sessionID)` | `Promise<void>` — permanently destroys the OpenCode session server-side (`client.session.remove`) and closes its panel |
| `sessionTranscript(sessionID)` | `Promise<{transcript, todos}>` — authoritative message replay used to materialize incomplete stream snapshots |
| `sessionUsage(sessionID)` | `Promise<SessionUsage \| null>` — normalized `cost`/`tokens` for the addressed session; used after `session.compaction` to refresh the context-window display |
| `prompt(workspace, text, files?, delivery?)` | Sends the prompt; `delivery` is `"queue"` or `"steer"` for follow-ups while busy |
| `inboxList(workspace)` | `Promise<SessionInboxEntry[]>` — queued entries for the panel's session |
| `inboxCancel(workspace, inboxID)` | `Promise<void>` — cancels a queued entry |
| `inboxSteer(workspace, inboxID)` | `Promise<void>` — delivers a queued entry immediately |
| `formsList(workspace)` | `Promise<PendingFormRequest[]>` — pending session and location-global agent forms for the panel |
| `formReply(workspace, formID, answers, formSessionID?)` | `Promise<void>` — replies through the form's session or location-global route |
| `formCancel(workspace, formID, formSessionID?)` | `Promise<void>` — cancels through the form's session or location-global route |
| `providerOauthStart(workspace, integrationID, methodID)` | `Promise<ProviderOAuthAttempt>` — begins the OAuth flow |
| `providerOauthPoll(workspace, integrationID, attemptID)` | `Promise<ProviderOAuthPoll>` — pending/complete/failed/expired |
| `providerOauthComplete(workspace, integrationID, attemptID, code?)` | `Promise<void>` — confirms an attempt (code mode) |
| `providerOauthCancel(workspace, integrationID, attemptID)` | `Promise<void>` — aborts an attempt |
| `mcpList(workspace)` | `Promise<McpServerOption[]>` — configured MCP servers with status |
| `pluginsList(workspace)` | `Promise<PluginOption[]>` — active plugins with source |
| `skillsList(workspace)` | `Promise<SkillOption[]>` — workspace skills |
| `revertStage(workspace, messageID, files)` | `Promise<SessionRevertStage>` — stages a revert of the panel's session to a message |
| `revertCommit(workspace)` | `Promise<void>` — applies the staged revert |
| `revertClear(workspace)` | `Promise<void>` — discards the staged revert |
| `commands(workspace)` | `Promise<CommandOption[]>` — slash commands + skills for the session directory (`kind` distinguishes them) |
| `runCommand(workspace, name, args?)` | `Promise<void>` — runs a slash command or skill in the addressed session |
| `references(workspace, query)` | `Promise<ReferenceOption[]>` — `file.find` search results for @-mentions, `rel` paths relative to the session directory |
| `selectFiles()` | `Promise<string[]>` |
| `selectImages()` | `Promise<string[]>` — image-filtered multi-file dialog |
| `readImagePreview(file)` | `Promise<string \| null>` — data-URL thumbnail for a local image path |
| `interrupt(workspace)` | `Promise<void>` |
| `listDir(workspace, rel)` | `Promise<TreeEntry[]>` |
| `revealInFileManager(workspace, rel)` | `Promise<void>` — validates and reveals a file or folder in the operating system's native file manager; missing items fail without changing workspace or editor state |
| `readFile(workspace, rel)` | `Promise<string \| null>` — workspace-relative only |
| `openExternal(workspace, file)` | `Promise<ExternalOpenResult>` — resolves an absolute path against the workspace: `{kind:"relative", rel, content}` when the file lives under the workspace root (open it normally) or `{kind:"standalone", path, content}` when outside it (open as a standalone tab) |
| `externalKind(file)` | `Promise<ExternalKind>` — probes an absolute path as `file`/`directory`/`missing` so explorer drops can route files to standalone tabs and folders to imports |
| `getPathForFile(file)` | `string` — synchronous (not an invoke): `webUtils.getPathForFile` is the Electron 32+ replacement for the removed `File.path`, exposed so renderer drop handlers can recover absolute paths from dropped OS files |
| `writeStandalone(file, content, expectedContent, overwrite)` | `Promise<void>` — atomic standalone-file write (conflict-checked unless `overwrite`) for external tabs outside the workspace root |
| `importExternal(workspace, destDir, sources)` | `Promise<ImportResult[]>` — copies external files/folders into the workspace at `destDir`, seeding clean baselines so imports never show as changes |
| `writeFile(workspace, rel, content, write)` | `Promise<void>` |
| `createFile(workspace, rel)` | `Promise<void>` — creates an empty file, erroring if it exists |
| `createDir(workspace, rel)` | `Promise<void>` — creates a folder, erroring if it exists |
| `deletePath(workspace, rel)` | `Promise<void>` — moves to Trash and preserves failures without permanent deletion |
| `detachPath(workspace, rel)` | `Promise<void>` — moves an entry outside the workspace while preserving its contents |
| `renamePath(workspace, rel, newName)` | `Promise<void>` — renames within the same folder |
| `movePath(workspace, rel, newParent)` | `Promise<void>` — moves a file or folder into another folder; empty `newParent` is the workspace root |
| `recoveryRecords(workspace)` | `Promise<RecoveryRecord[]>` — persistent durable artifacts for the addressed workspace |
| `openRecovery(workspace, id)` | `Promise<void>` — opens a validated artifact selected by record id |
| `acknowledgeRecovery(workspace, id)` | `Promise<void>` — persists acknowledgment without deleting bytes |
| `projects()` | `Promise<ProjectInfo[]>` |
| `savedWorkspaces()` | `Promise<SavedWorkspaceSnapshot>` — durable bookmarks plus an initialized marker; the renderer migrates legacy storage/catalog data only on first use |
| `saveSavedWorkspaces(workspaces)` | `Promise<ProjectInfo[]>` — writes normalized bookmarks atomically and retains a last-known-good backup |
| `models(workspace)` | `Promise<ModelOption[]>` |
| `modelDefault(workspace)` | `Promise<ModelOption \| null>` |
| `switchModel(workspace, id, providerID, variant?)` | `Promise<void>` |
| `agents(workspace)` | `Promise<AgentOption[]>` |
| `switchAgent(workspace, id)` | `Promise<void>` |
| `terminalStart(workspace, id, directory?, cols?, rows?)` | `Promise<string \| void>`; `id` is a renderer-generated `term-<UUID>` registered before invoke; `cols`/`rows` seed the PTY at xterm's measured viewport before the first shell prompt, and new main processes return the validated cwd while `void` remains accepted so a hot-reloaded renderer can move a terminal opened by an older main process into the requested directory |
| `terminalInput(workspace, id, data)` | `Promise<void>` |
| `terminalResize(workspace, id, cols, rows)` | `Promise<void>` |
| `terminalStop(workspace, id)` | `Promise<void>` |
| `agentTuiStart(workspace, id)` | `Promise<void>`; starts the active runtime's TUI in the addressed workspace |
| `agentTuiInput(workspace, id, data)` | `Promise<void>`; forwards xterm input through the terminal ownership checks |
| `agentTuiResize(workspace, id, cols, rows)` | `Promise<void>`; forwards xterm dimensions through the terminal validation |
| `agentTuiStop(workspace, id)` | `Promise<void>`; stops the panel's TUI PTY |
| `permissionReply(workspace, requestID, reply, sessionID)` | `Promise<void>` |
| `listPermissions(workspace)` | `Promise<PendingPermissionRequest[]>` — pending permission requests across the service's sessions |
| `state()` | `Promise<SessionInfo \| null>` — the most recently activated session |
| `sessionSelection(workspace)` | `Promise<SessionSelection \| null>` |
| `providerUsage()` | `Promise<ProviderUsageResult[]>` |
| `providerIntegrations(workspace)` | `Promise<ProviderIntegration[]>` — supported-provider catalog (OpenCode Go, Command Code, OpenAI) and secret-free OpenCode connection state |
| `connectProviderKey(workspace, integrationID, key, label, answers)` | `Promise<void>` — sends a write-only key and provider-specific form answers; the key is never returned to the renderer |
| `removeProviderCredential(workspace, credentialID)` | `Promise<void>` — removes one stored credential by opaque id |
| `mobileSetupStatus()` | `Promise<MobileServerSetupStatus>` — reports the local companion server state without returning its UI password |
| `mobilePairingQr()` | `Promise<MobilePairingQr>` — returns only a short-lived one-time Vela pairing URI for rendering in Desktop Mobile Setup |
| `health()` | `Promise<boolean>` |
| `windowView(view)` | `Promise<void>` — asks the main process to switch between the `"landing"` and `"session"` window profiles |
| `setAppearance(appearance)` | `Promise<void>` — reports the selected color profile's native appearance (`"dark"` or `"light"`) so window vibrancy and native chrome follow its color scheme |
| `installApp()` | `Promise<{ok: boolean, message: string}>` — macOS-only: builds the packaged app and installs it to `/Applications`; `ok` false with a message on failure |
| `validateW3c(path, content)` | `Promise<W3cDiagnostic[]>` — validates HTML/CSS source through the W3C services |
| `viteToggle(workspace, entryPath?)` | `Promise<ViteToggleResult>` — starts the loopback Vite dev server for the resolved page (a confined active HTML file's directory, or the workspace root, falling back to the shallowest HTML page) and opens it, or stops the server already running for that same target |
| `viteServers()` | `Promise<ViteServerInfo[]>` — lists every running Vite dev server across workspaces (id, workspace, directory, entry, URL, port) |
| `viteStop(serverID)` | `Promise<void>` — stops one Vite dev server by its opaque id |
| `viteStopAll()` | `Promise<void>` — stops every running Vite dev server |
| `takePendingPaths()` | `Promise<string[]>` — drains OS-dropped or launch paths queued before the renderer was ready |

All are `ipcRenderer.invoke` wrappers over the `shell:*` channels
documented in `docs/main.md`. `onMessage` subscribes to
`ipcRenderer.on("shell:message")` and removes the listener on
unsubscribe.

Security posture: `contextIsolation: true`, `nodeIntegration: false`, and
`sandbox: true`. The renderer has no direct Node or Electron access. Main
accepts bridge invokes only from the active window's trusted main frame and
trusted application URL; a document that navigates elsewhere or an untrusted
subframe cannot use the exposed API. Workspace filesystem and terminal calls
also require a live `WorkspaceIdentity` matching an open backend session
context; stale or unknown identities are rejected in main. Main validates
bounded runtime schemas for activation and session IDs, prompts and
attachments, runtime ids, command/search payloads, model/agent selection, permission
replies, provider keys and form answers, filesystem writes, and terminal arguments. The standalone-file channels
(`openExternal`, `writeStandalone`) are the one bridge surface that accepts
arbitrary absolute paths; main bounds them (length/NUL/size caps) and `realpath`s
them to a regular file before reading or wiring an atomic write, so they behave
like the workspace channels but without a recovery store (standalone files always
live outside the watched workspace root).
