import { contextBridge, ipcRenderer, webUtils } from "electron";
import type {
  AgentOption,
  BackendMessage,
  CommandOption,
  ExternalKind,
  ExternalOpenResult,
  FileWriteIdentity,
  ImportResult,
  ModelOption,
  OpenFileWorkspaceResult,
  PendingPermissionRequest,
  PermissionReply,
  ProjectInfo,
  PromptDelivery,
  PromptFile,
  SessionInboxEntry,
  SessionUsage,
  FormAnswers,
  PendingFormRequest,
  ProviderCredentialAnswers,
  ProviderIntegration,
  ProviderOAuthAttempt,
  ProviderOAuthPoll,
  McpServerOption,
  MobilePairingQr,
  MobileServerSetupStatus,
  PluginOption,
  SessionRevertStage,
  SkillOption,
  ProviderUsageResult,
  RecoveryRecord,
  ReferenceOption,
  ReopenedSession,
  RuntimeID,
  RuntimeManifest,
  OpenCodeSyncResult,
  VelaAppUpdateResult,
  VelaAppUpdateStatus,
  SessionInfo,
  SessionSelection,
  SessionSummary,
  SessionTranscript,
  SavedWorkspaceSnapshot,
  ViteServerInfo,
  ViteToggleResult,
  SassWatchState,
  W3cDiagnostic,
  WorkspaceIdentity
} from "@shared/types";

const appVersionArgument = process.argv.find((argument) => argument.startsWith("--vela-app-version="));

const api = {
  platform: process.platform,
  isPackaged: process.argv.includes("--vela-packaged"),
  isReleaseBuild: process.argv.includes("--vela-release-build"),
  appVersion: appVersionArgument?.slice("--vela-app-version=".length) ?? "unknown",
  onMessage: (cb: (msg: BackendMessage) => void): (() => void) => {
    const listener = (_e: unknown, msg: BackendMessage): void => cb(msg);
    ipcRenderer.on("shell:message", listener);
    return () => ipcRenderer.removeListener("shell:message", listener);
  },
  selectFolder: (generation: number, runtimeID?: RuntimeID): Promise<SessionInfo | null> => ipcRenderer.invoke("shell:select-folder", generation, runtimeID),
  selectDirectory: (): Promise<string | null> => ipcRenderer.invoke("shell:select-directory"),
  selectFile: (generation: number, runtimeID?: RuntimeID): Promise<OpenFileWorkspaceResult | null> => ipcRenderer.invoke("shell:select-file", generation, runtimeID),
  openFileWorkspace: (file: string, generation: number, runtimeID?: RuntimeID): Promise<OpenFileWorkspaceResult> =>
    ipcRenderer.invoke("shell:open-file", file, generation, runtimeID),
  openExternal: (workspace: WorkspaceIdentity, file: string): Promise<ExternalOpenResult> =>
    ipcRenderer.invoke("shell:open-external", workspace, file),
  externalKind: (file: string): Promise<ExternalKind> =>
    ipcRenderer.invoke("shell:stat-external", file),
  getPathForFile: (file: File): string => webUtils.getPathForFile(file),
  writeStandalone: (file: string, content: string, expectedContent: string, overwrite: boolean): Promise<void> =>
    ipcRenderer.invoke("shell:fs-write-standalone", file, content, expectedContent, overwrite),
  importExternal: (workspace: WorkspaceIdentity, destDir: string, sources: string[]): Promise<ImportResult[]> =>
    ipcRenderer.invoke("shell:fs-import", workspace, destDir, sources),
  openSession: (dir: string, generation: number, runtimeID?: RuntimeID): Promise<SessionInfo> => ipcRenderer.invoke("shell:open-session", dir, generation, runtimeID),
  runtimes: (): Promise<RuntimeManifest[]> => ipcRenderer.invoke("shell:runtimes"),
  syncOpenCode: (): Promise<OpenCodeSyncResult> => ipcRenderer.invoke("shell:sync-opencode"),
  updateOpenCode: (): Promise<OpenCodeSyncResult> => ipcRenderer.invoke("shell:update-opencode"),
  checkAppUpdate: (): Promise<VelaAppUpdateStatus> => ipcRenderer.invoke("shell:app-update-check"),
  updateApp: (): Promise<VelaAppUpdateResult> => ipcRenderer.invoke("shell:app-update"),
  reportUnsavedEditorFiles: (paths: string[]): Promise<void> =>
    ipcRenderer.invoke("shell:unsaved-editor-files", paths),
  onSaveAllEditorFiles: (save: () => Promise<boolean>): (() => void) => {
    const listener = (_event: unknown, requestID: string): void => {
      void Promise.resolve().then(save).catch(() => false).then((saved) =>
        ipcRenderer.invoke("shell:save-editor-files-result", requestID, saved)
      ).catch(() => {});
    };
    ipcRenderer.on("shell:save-editor-files-request", listener);
    return () => ipcRenderer.removeListener("shell:save-editor-files-request", listener);
  },
  sessions: (): Promise<SessionSummary[]> => ipcRenderer.invoke("shell:sessions"),
  activeSessions: (): Promise<SessionInfo[]> => ipcRenderer.invoke("shell:active-sessions"),
  closeSession: (workspace: WorkspaceIdentity): Promise<void> => ipcRenderer.invoke("shell:close-session", workspace),
  openSessionById: (sessionID: string, generation: number, runtimeID?: RuntimeID, allowWorkspaceRelink = false): Promise<ReopenedSession> =>
    ipcRenderer.invoke("shell:open-session-id", sessionID, generation, runtimeID, allowWorkspaceRelink),
  deleteSession: (sessionID: string): Promise<void> =>
    ipcRenderer.invoke("shell:delete-session", sessionID),
  sessionTranscript: (sessionID: string): Promise<SessionTranscript> =>
    ipcRenderer.invoke("shell:session-transcript", sessionID),
  sessionUsage: (sessionID: string): Promise<SessionUsage | null> =>
    ipcRenderer.invoke("shell:session-usage", sessionID),
  prompt: (workspace: WorkspaceIdentity, text: string, files: PromptFile[] = [], delivery?: PromptDelivery): Promise<SessionTranscript> =>
    ipcRenderer.invoke("shell:prompt", workspace, text, files, delivery),
  inboxList: (workspace: WorkspaceIdentity): Promise<SessionInboxEntry[]> =>
    ipcRenderer.invoke("shell:inbox-list", workspace),
  inboxCancel: (workspace: WorkspaceIdentity, inboxID: string): Promise<void> =>
    ipcRenderer.invoke("shell:inbox-cancel", workspace, inboxID),
  inboxSteer: (workspace: WorkspaceIdentity, inboxID: string): Promise<void> =>
    ipcRenderer.invoke("shell:inbox-steer", workspace, inboxID),
  formsList: (workspace: WorkspaceIdentity): Promise<PendingFormRequest[]> =>
    ipcRenderer.invoke("shell:forms-list", workspace),
  formReply: (workspace: WorkspaceIdentity, formID: string, answers: FormAnswers, formSessionID?: string): Promise<void> =>
    ipcRenderer.invoke("shell:form-reply", workspace, formID, answers, formSessionID),
  formCancel: (workspace: WorkspaceIdentity, formID: string, formSessionID?: string): Promise<void> =>
    ipcRenderer.invoke("shell:form-cancel", workspace, formID, formSessionID),
  commands: (workspace: WorkspaceIdentity): Promise<CommandOption[]> => ipcRenderer.invoke("shell:commands", workspace),
  runCommand: (workspace: WorkspaceIdentity, name: string, args: string = ""): Promise<void> =>
    ipcRenderer.invoke("shell:run-command", workspace, name, args),
  references: (workspace: WorkspaceIdentity, query: string): Promise<ReferenceOption[]> =>
    ipcRenderer.invoke("shell:find-files", workspace, query),
  selectFiles: (): Promise<string[]> => ipcRenderer.invoke("shell:select-files"),
  selectImages: (): Promise<string[]> => ipcRenderer.invoke("shell:select-images"),
  readImagePreview: (file: string): Promise<string | null> => ipcRenderer.invoke("shell:read-image-preview", file),
  interrupt: (workspace: WorkspaceIdentity): Promise<void> => ipcRenderer.invoke("shell:interrupt", workspace),
  listDir: (workspace: WorkspaceIdentity, rel: string): Promise<{ path: string; type: "file" | "directory" }[]> =>
    ipcRenderer.invoke("shell:fs-list", workspace, rel),
  revealInFileManager: (workspace: WorkspaceIdentity, rel: string): Promise<void> =>
    ipcRenderer.invoke("shell:fs-reveal", workspace, rel),
  readFile: (workspace: WorkspaceIdentity, rel: string): Promise<string | null> =>
    ipcRenderer.invoke("shell:fs-read", workspace, rel),
  writeFile: (
    workspace: WorkspaceIdentity,
    rel: string,
    content: string,
    write: FileWriteIdentity
  ): Promise<void> => ipcRenderer.invoke("shell:fs-write", workspace, rel, content, write),
  createFile: (workspace: WorkspaceIdentity, rel: string): Promise<void> =>
    ipcRenderer.invoke("shell:fs-create-file", workspace, rel),
  createDir: (workspace: WorkspaceIdentity, rel: string): Promise<void> =>
    ipcRenderer.invoke("shell:fs-create-dir", workspace, rel),
  deletePath: (workspace: WorkspaceIdentity, rel: string): Promise<void> =>
    ipcRenderer.invoke("shell:fs-delete", workspace, rel),
  detachPath: (workspace: WorkspaceIdentity, rel: string): Promise<void> =>
    ipcRenderer.invoke("shell:fs-detach", workspace, rel),
  renamePath: (workspace: WorkspaceIdentity, rel: string, newName: string): Promise<void> =>
    ipcRenderer.invoke("shell:fs-rename", workspace, rel, newName),
  movePath: (workspace: WorkspaceIdentity, rel: string, newParent: string): Promise<void> =>
    ipcRenderer.invoke("shell:fs-move", workspace, rel, newParent),
  recoveryRecords: (workspace: WorkspaceIdentity): Promise<RecoveryRecord[]> =>
    ipcRenderer.invoke("shell:recovery-list", workspace),
  openRecovery: (workspace: WorkspaceIdentity, id: string): Promise<void> =>
    ipcRenderer.invoke("shell:recovery-open", workspace, id),
  acknowledgeRecovery: (workspace: WorkspaceIdentity, id: string): Promise<void> =>
    ipcRenderer.invoke("shell:recovery-acknowledge", workspace, id),
  projects: (): Promise<ProjectInfo[]> => ipcRenderer.invoke("shell:projects"),
  savedWorkspaces: (): Promise<SavedWorkspaceSnapshot> => ipcRenderer.invoke("shell:saved-workspaces"),
  saveSavedWorkspaces: (workspaces: ProjectInfo[]): Promise<ProjectInfo[]> =>
    ipcRenderer.invoke("shell:saved-workspaces-save", workspaces),
  models: (workspace: WorkspaceIdentity): Promise<ModelOption[]> => ipcRenderer.invoke("shell:models", workspace),
  modelDefault: (workspace: WorkspaceIdentity): Promise<ModelOption | null> => ipcRenderer.invoke("shell:model-default", workspace),
  switchModel: (workspace: WorkspaceIdentity, id: string, providerID: string, variant?: string): Promise<void> =>
    ipcRenderer.invoke("shell:switch-model", workspace, id, providerID, variant),
  agents: (workspace: WorkspaceIdentity): Promise<AgentOption[]> => ipcRenderer.invoke("shell:agents", workspace),
  switchAgent: (workspace: WorkspaceIdentity, id: string): Promise<void> => ipcRenderer.invoke("shell:switch-agent", workspace, id),
  terminalStart: (workspace: WorkspaceIdentity, id: string, directory = "", cols = 100, rows = 24): Promise<string | void> =>
    ipcRenderer.invoke("shell:terminal-start", workspace, id, directory, cols, rows),
  terminalInput: (workspace: WorkspaceIdentity, id: string, data: string): Promise<void> =>
    ipcRenderer.invoke("shell:terminal-input", workspace, id, data),
  terminalResize: (workspace: WorkspaceIdentity, id: string, cols: number, rows: number): Promise<void> =>
    ipcRenderer.invoke("shell:terminal-resize", workspace, id, cols, rows),
  terminalStop: (workspace: WorkspaceIdentity, id: string): Promise<void> =>
    ipcRenderer.invoke("shell:terminal-stop", workspace, id),
  agentTuiStart: (workspace: WorkspaceIdentity, id: string): Promise<void> =>
    ipcRenderer.invoke("shell:agent-tui-start", workspace, id),
  agentTuiInput: (workspace: WorkspaceIdentity, id: string, data: string): Promise<void> =>
    ipcRenderer.invoke("shell:terminal-input", workspace, id, data),
  agentTuiResize: (workspace: WorkspaceIdentity, id: string, cols: number, rows: number): Promise<void> =>
    ipcRenderer.invoke("shell:terminal-resize", workspace, id, cols, rows),
  agentTuiStop: (workspace: WorkspaceIdentity, id: string): Promise<void> =>
    ipcRenderer.invoke("shell:terminal-stop", workspace, id),
  permissionReply: (workspace: WorkspaceIdentity, requestID: string, reply: PermissionReply, sessionID: string): Promise<void> =>
    ipcRenderer.invoke("shell:permission-reply", workspace, requestID, reply, sessionID),
  listPermissions: (workspace: WorkspaceIdentity): Promise<PendingPermissionRequest[]> =>
    ipcRenderer.invoke("shell:list-permissions", workspace),
  state: (): Promise<SessionInfo | null> => ipcRenderer.invoke("shell:state"),
  sessionSelection: (workspace: WorkspaceIdentity): Promise<SessionSelection | null> =>
    ipcRenderer.invoke("shell:session-selection", workspace),
  providerUsage: (): Promise<ProviderUsageResult[]> => ipcRenderer.invoke("shell:provider-usage"),
  providerIntegrations: (workspace: WorkspaceIdentity): Promise<ProviderIntegration[]> =>
    ipcRenderer.invoke("shell:provider-integrations", workspace),
  connectProviderKey: (workspace: WorkspaceIdentity, integrationID: string, key: string, label: string, answers: ProviderCredentialAnswers): Promise<void> =>
    ipcRenderer.invoke("shell:provider-key-connect", workspace, integrationID, key, label, answers),
  providerOauthStart: (workspace: WorkspaceIdentity, integrationID: string, methodID: string): Promise<ProviderOAuthAttempt> =>
    ipcRenderer.invoke("shell:provider-oauth-start", workspace, integrationID, methodID),
  providerOauthPoll: (workspace: WorkspaceIdentity, integrationID: string, attemptID: string): Promise<ProviderOAuthPoll> =>
    ipcRenderer.invoke("shell:provider-oauth-poll", workspace, integrationID, attemptID),
  providerOauthComplete: (workspace: WorkspaceIdentity, integrationID: string, attemptID: string, code?: string): Promise<void> =>
    ipcRenderer.invoke("shell:provider-oauth-complete", workspace, integrationID, attemptID, code),
  providerOauthCancel: (workspace: WorkspaceIdentity, integrationID: string, attemptID: string): Promise<void> =>
    ipcRenderer.invoke("shell:provider-oauth-cancel", workspace, integrationID, attemptID),
  mcpList: (workspace: WorkspaceIdentity): Promise<McpServerOption[]> =>
    ipcRenderer.invoke("shell:mcp-list", workspace),
  pluginsList: (workspace: WorkspaceIdentity): Promise<PluginOption[]> =>
    ipcRenderer.invoke("shell:plugins-list", workspace),
  skillsList: (workspace: WorkspaceIdentity): Promise<SkillOption[]> =>
    ipcRenderer.invoke("shell:skills-list", workspace),
  revertStage: (workspace: WorkspaceIdentity, messageID: string, files: boolean): Promise<SessionRevertStage> =>
    ipcRenderer.invoke("shell:session-revert-stage", workspace, messageID, files),
  revertCommit: (workspace: WorkspaceIdentity): Promise<void> =>
    ipcRenderer.invoke("shell:session-revert-commit", workspace),
  revertClear: (workspace: WorkspaceIdentity): Promise<void> =>
    ipcRenderer.invoke("shell:session-revert-clear", workspace),
  removeProviderCredential: (workspace: WorkspaceIdentity, credentialID: string): Promise<void> =>
    ipcRenderer.invoke("shell:provider-credential-remove", workspace, credentialID),
  health: (): Promise<boolean> => ipcRenderer.invoke("shell:health"),
  mobileSetupStatus: (): Promise<MobileServerSetupStatus> => ipcRenderer.invoke("shell:mobile-setup-status"),
  mobilePairingQr: (): Promise<MobilePairingQr> => ipcRenderer.invoke("shell:mobile-pairing-qr"),
  installApp: (): Promise<{ ok: boolean; message: string }> => ipcRenderer.invoke("shell:install-app"),
  windowView: (view: "landing" | "session"): Promise<void> => ipcRenderer.invoke("shell:window-view", view),
  setAppearance: (appearance: "dark" | "light"): Promise<void> => ipcRenderer.invoke("shell:set-appearance", appearance),
  validateW3c: (path: string, content: string): Promise<W3cDiagnostic[]> =>
    ipcRenderer.invoke("shell:validate-w3c", path, content),
  viteToggle: (workspace: WorkspaceIdentity, entryPath?: string): Promise<ViteToggleResult> =>
    ipcRenderer.invoke("shell:vite-toggle", workspace, entryPath),
  sassWatchState: (workspace: WorkspaceIdentity): Promise<SassWatchState> => ipcRenderer.invoke("shell:sass-watch-state", workspace),
  setSassWatch: (workspace: WorkspaceIdentity, enabled: boolean): Promise<SassWatchState> => ipcRenderer.invoke("shell:sass-watch-set", workspace, enabled),
  viteServers: (): Promise<ViteServerInfo[]> =>
    ipcRenderer.invoke("shell:vite-servers"),
  viteStop: (serverID: string): Promise<void> =>
    ipcRenderer.invoke("shell:vite-stop", serverID),
  viteStopAll: (): Promise<void> =>
    ipcRenderer.invoke("shell:vite-stop-all"),
  takePendingPaths: (): Promise<string[]> =>
    ipcRenderer.invoke("shell:take-pending-paths")
};

export type VelaApi = typeof api;

contextBridge.exposeInMainWorld("vela", api);
