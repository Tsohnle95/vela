export type RuntimeID = "opencode" | "deepseek" | (string & {});

const previousWorkspaceRecovery = String.fromCharCode(109, 105, 114, 97);
const previousSharedRecovery = String.fromCharCode(111, 112, 101, 110, 115, 104, 101, 108, 108);
const oldestWorkspaceRecovery = String.fromCharCode(111, 114, 98, 105, 116);

export const RECOVERY_DIR = ".vela-recovery";
export const PREVIOUS_RECOVERY_DIR = `.${previousWorkspaceRecovery}-recovery`;
export const LEGACY_RECOVERY_DIR = `.${previousSharedRecovery}-recovery`;
export const OLDEST_RECOVERY_DIR = `.${oldestWorkspaceRecovery}-recovery`;
export const HIDDEN_RECOVERY_DIRS = [RECOVERY_DIR, PREVIOUS_RECOVERY_DIR, LEGACY_RECOVERY_DIR, OLDEST_RECOVERY_DIR] as const;

export interface RuntimeCapabilities {
  attachments: boolean;
  commands: boolean;
  models: boolean;
  agents: boolean;
  permissions: boolean;
  providerCredentials: boolean;
  sessionFork: boolean;
  sessionResume: boolean;
  steering: boolean;
  tui: boolean;
}

export interface RuntimeManifest {
  protocolVersion: 1;
  id: RuntimeID;
  name: string;
  version: string | null;
  available: boolean;
  capabilities: RuntimeCapabilities;
}

export interface OpenCodeSyncResult {
  version: string;
  previousVersion: string;
  cliUpdated: boolean;
}

export type VelaAppUpdateStatus =
  | { state: "available"; currentVersion: string; latestVersion: string }
  | { state: "current"; currentVersion: string; latestVersion: string }
  | { state: "disabled"; currentVersion: string; message: string }
  | { state: "blocked"; message: string; currentVersion?: string };

export interface VelaAppUpdateResult {
  ok: boolean;
  updated: boolean;
  currentVersion?: string;
  latestVersion?: string;
  message: string;
}

export interface SessionInfo {
  id: string;
  runtimeID?: RuntimeID;
  directory: string;
  workspace: WorkspaceIdentity;
  parentID?: string;
  title?: string;
  agent?: string;
}

export interface WorkspaceIdentity {
  id: string;
  generation: number;
}

export interface VitePreview {
  url: string;
  port: number;
}

export interface ViteServerInfo {
  id: string;
  workspaceId: string;
  /** Absolute serve root. */
  directory: string;
  /** Entry relative to `directory`; an empty entry serves the folder's index.html. */
  entry: string;
  url: string;
  port: number;
}

export interface ViteToggleResult {
  running: boolean;
  server?: ViteServerInfo;
}

export interface SassWatchLog { time: number; level: "info" | "warning" | "error"; message: string }
export interface SassWatchState {
  workspace: WorkspaceIdentity;
  directory: string;
  enabled: boolean;
  status: "off" | "starting" | "on" | "error";
  revision: number;
  error?: string;
  logs: SassWatchLog[];
}

export interface MobileServerSetupStatus {
  state: "ready" | "starting" | "offline" | "setup-required";
  reason?: "companion-checkout-missing" | "node-runtime-missing" | "server-not-running";
  workspacePath: string;
  port: number;
}

export interface MobilePairingQr {
  /** Versioned, one-time Vela connection URI; never a reusable password/token. */
  connectionUrl: string;
  expiresAt: string;
  serverLabel: string;
}

export interface SessionSummary {
  id: string;
  runtimeID?: RuntimeID;
  title: string;
  directory: string;
  updatedAt: number;
  parentID?: string;
  agent?: string;
}

export interface ReopenedSession {
  session: SessionInfo;
  transcript: TranscriptItem[];
  todos: TodoItem[];
  usage: SessionUsage | null;
}

export type ExternalOpenResult =
  | { kind: "relative"; rel: string; content: string | null }
  | { kind: "standalone"; path: string; content: string | null };

export interface ExternalKind {
  kind: "file" | "directory" | "missing";
}

export interface OpenFileWorkspaceResult {
  session: SessionInfo;
  path: string;
}

export interface ImportResult {
  name: string;
  rel: string;
  imported: boolean;
  reason?: string;
}

export interface SessionTranscript {
  transcript: TranscriptItem[];
  todos: TodoItem[];
}

export interface SessionUsage {
  cost: number;
  tokens: {
    input: number;
    output: number;
    reasoning: number;
    cache: {
      read: number;
      write: number;
    };
  };
}

export type ProviderUsageStatus = "ok" | "stale" | "unavailable" | "unauthenticated" | "unsupported";

export interface UsageWindow {
  id: string;
  label: string;
  usedPercent: number;
  windowMinutes: number | null;
  resetsAt: number | null;
}

export interface ProviderUsageCredits {
  hasCredits: boolean;
  unlimited: boolean;
  balance: string | null;
  label?: string;
  total?: number | null;
  used?: number | null;
  remaining?: number | null;
  overagePermitted?: boolean;
}

export interface ProviderUsageSnapshot {
  windows: UsageWindow[];
  credits: ProviderUsageCredits | null;
  planType: string | null;
  updatedAt: number;
}

export interface ProviderUsageError {
  code: string;
  message: string;
  retryable: boolean;
}

export interface ProviderUsageResult {
  provider: string;
  displayName: string;
  status: ProviderUsageStatus;
  snapshot: ProviderUsageSnapshot | null;
  error?: ProviderUsageError | null;
}

export type ProviderCredentialValue = string | number | boolean | string[];

export interface ProviderFormOption {
  value: string;
  label: string;
  description?: string;
}

export interface ProviderFormCondition {
  key: string;
  op: "eq" | "neq";
  value: string | number | boolean;
}

export interface ProviderFormField {
  key: string;
  type: "string" | "number" | "integer" | "boolean" | "multiselect" | "external";
  title?: string;
  description?: string;
  required?: boolean;
  placeholder?: string;
  options?: ProviderFormOption[];
  default?: ProviderCredentialValue;
  when?: ProviderFormCondition[];
  minimum?: number;
  maximum?: number;
  minLength?: number;
  maxLength?: number;
  pattern?: string;
  minItems?: number;
  maxItems?: number;
  url?: string;
}

export interface ProviderIntegration {
  id: string;
  name: string;
  keyMethod: {
    label?: string;
    fields: ProviderFormField[];
  } | null;
  credentials: Array<{ id: string; label: string }>;
  environment: {
    names: string[];
    connected: string[];
  };
  oauth: Array<{ id: string; label: string }>;
}

export interface ProviderOAuthAttempt {
  attemptID: string;
  url: string;
  instructions: string;
  mode: "auto" | "code";
}

export type ProviderOAuthPoll =
  | { status: "pending" }
  | { status: "complete" }
  | { status: "failed"; message: string }
  | { status: "expired" };

export interface SessionRevertStage {
  messageID: string;
  partID?: string;
  snapshot?: string;
}

export interface McpServerOption {
  name: string;
  status: string;
}

export interface PluginOption {
  id: string;
  source: string;
  status: string;
}

export interface SkillOption {
  id: string;
  name: string;
  description?: string;
  slash?: boolean;
  location?: string;
}

export type ProviderCredentialAnswers = Record<string, ProviderCredentialValue>;

export interface TreeEntry {
  path: string;
  type: "file" | "directory";
}

export interface FileUpdate {
  workspace: WorkspaceIdentity;
  sessionID: string;
  path: string;
  movedFrom?: string;
  baseline: FileBaseline;
  content: string | null;
  deleted: boolean;
  write?: FileWriteIdentity;
}

export type FileBaseline =
  | { kind: "known"; content: string; exists?: boolean }
  | { kind: "unknown" };

export interface FileWriteIdentity {
  id: string;
  workspaceID: string;
  revision: number;
  expectedContent: string;
  overwrite: boolean;
}

export interface RecoveryRecord {
  id: string;
  artifact: "temporary" | "original" | "proposed" | "rename-source";
  originalPath: string;
  recoveryPath: string;
  createdAt: number;
  acknowledged: boolean;
  reason: "saved" | "renamed" | "save-failed" | "crash-recovered" | "rename-failed";
}

export interface RecoveryState {
  workspace: WorkspaceIdentity;
  records: RecoveryRecord[];
}

export interface ProjectInfo {
  directory: string;
  name: string;
}

export interface SavedWorkspaceSnapshot {
  workspaces: ProjectInfo[];
  initialized: boolean;
}

export interface ModelOption {
  id: string;
  providerID: string;
  name: string;
  variants?: string[];
  variant?: string;
  limit?: { context: number };
}

export interface AgentOption {
  id: string;
  name: string;
  color?: string;
}

export interface CommandOption {
  name: string;
  description?: string;
  kind?: "command" | "skill";
}

export interface ReferenceOption {
  name: string;
  path: string;
  rel: string;
  description?: string;
}

export interface W3cDiagnostic {
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
  message: string;
  severity: "error" | "warning";
  source: "w3c-html" | "w3c-css" | "monaco-scss";
}

export type PromptDelivery = "queue" | "steer";

export interface SessionInboxEntry {
  id: string;
  text: string;
  attachmentCount: number;
  createdAt: number;
  delivery?: "steer" | "queue";
}

export type FormAnswers = Record<string, string | number | boolean | string[]>;

export interface PendingFormField {
  key: string;
  title?: string;
  description?: string;
  required?: boolean;
  type: "string" | "number" | "integer" | "boolean" | "multiselect" | "external";
  placeholder?: string;
  url?: string;
  options?: { value: string; label: string }[];
}

export interface PendingFormRequest {
  id: string;
  sessionID: string;
  title: string;
  fields: PendingFormField[];
}

export interface PromptFile {
  path: string;
  mention?: { start: number; end: number; text: string };
}

export type TodoStatus = "pending" | "in_progress" | "completed" | "cancelled";

export interface TodoItem {
  id: string;
  content: string;
  status: TodoStatus;
  priority?: string;
}

export interface SessionSelection {
  model?: ModelOption;
  agent?: AgentOption;
}

export type ApprovalMode = "ask" | "approve";

export interface UserAttachment {
  name: string;
}

export type PermissionReply = "once" | "always" | "reject";

export interface PendingPermissionRequest {
  id: string;
  sessionID: string;
  action: string;
  resources: string[];
}

export interface TerminalData {
  id: string;
  data: string;
}

export interface TerminalExit {
  id: string;
  exitCode: number | null;
}

export interface ToolCallView {
  id: string;
  title: string;
  detail: string;
  status: "running" | "success" | "failed";
  input?: string;
  output?: string;
  progress?: string;
  startedAt?: number;
  duration?: number;
  paths?: string[];
  metadata?: Record<string, unknown>;
  inputValue?: unknown;
  content?: ToolContentView[];
  executed?: boolean;
  providerState?: Record<string, unknown>;
  providerResultState?: Record<string, unknown>;
}

export type ToolContentView =
  | { type: "text"; text: string }
  | { type: "file"; uri: string; mime: string; name?: string };

export type AssistantPartView =
  | { kind: "text"; id: string; text: string; complete: boolean }
  | { kind: "reasoning"; id: string; text: string; complete: boolean }
  | { kind: "tool"; id: string; tool: ToolCallView };

export interface MessagePresentation {
  agent?: string;
  model?: { id: string; providerID: string };
  createdAt?: number;
  completedAt?: number;
  parentID?: string;
  interrupted?: boolean;
}

export type TranscriptItem =
  | ({ kind: "user"; id: string; text: string; attachments?: UserAttachment[] } & MessagePresentation)
  | ({
      kind: "pending-input";
      id: string;
      inputType: "user" | "synthetic";
      text: string;
      attachments?: UserAttachment[];
      description?: string;
    } & MessagePresentation)
  | ({
      kind: "assistant";
      id: string;
      messageID: string;
      parts: AssistantPartView[];
      completed: boolean;
      retry?: { attempt: number; message: string; next?: number };
      error?: string;
    } & MessagePresentation)
  | {
      kind: "permission";
      id: string;
      requestID: string;
      action: string;
      resources: string[];
      pending: boolean;
      resolvedWith?: PermissionReply;
    }
  | {
      kind: "selection";
      id: string;
      selection: "agent" | "model";
      title: string;
      detail?: string;
    }
  | { kind: "synthetic"; id: string; text: string; description?: string }
  | { kind: "system"; id: string; text: string }
  | { kind: "skill"; id: string; skill: string; name: string; text: string }
  | {
      kind: "shell";
      id: string;
      shellID: string;
      command: string;
      status: "running" | "exited" | "timeout" | "killed";
      output?: string;
      exit?: number | string;
    }
  | {
      kind: "compaction";
      id: string;
      status: "running" | "completed" | "failed";
      reason: "auto" | "manual";
      summary: string;
      recent?: string;
      error?: string;
    }
  | { kind: "status"; id: string; text: string; tone: "info" | "success" | "error" }
  | { kind: "divider"; id: string };

export interface Tab {
  path: string;
  name: string;
  content: string;
  saved: string;
  baseline: FileBaseline | null;
  deleted: boolean;
  dirty: boolean;
  stale: boolean;
  revision: number;
  conflict: FileConflict | null;
  binary: boolean;
  /** Transient VS Code-style preview tab: replaced by the next preview open. */
  preview?: boolean;
  standalone?: boolean;
}

export interface FileConflict {
  content: string | null;
  deleted: boolean;
  resolution: "pending" | "merge";
}

export interface AgentFileState {
  baseline: FileBaseline;
  content: string | null;
  deleted: boolean;
}

export interface BackendMessageBase {
  kind: "event" | "file-update" | "session" | "ui-command" | "recovery";
  type?: string;
  data?: unknown;
  file?: FileUpdate;
  session?: SessionInfo;
  command?: string;
  path?: string;
  line?: number;
  recovery?: RecoveryState;
}

export type BackendMessage =
  | BackendMessageBase
  | { kind: "terminal-data"; terminal: TerminalData }
  | { kind: "terminal-exit"; terminal: TerminalExit }
  | { kind: "sass-watch"; state: SassWatchState };
