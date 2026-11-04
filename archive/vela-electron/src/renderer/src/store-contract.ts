import type {
  AgentFileState,
  AgentOption,
  ApprovalMode,
  FormAnswers,
  ModelOption,
  PendingFormField,
  PendingFormRequest,
  PermissionReply,
  ProjectInfo,
  PromptFile,
  ProviderUsageResult,
  RecoveryRecord,
  RuntimeManifest,
  SessionInfo,
  SessionRevertStage,
  SessionSummary,
  SessionUsage,
  Tab,
  TodoItem,
  TranscriptItem,
  TreeEntry,
  WorkspaceIdentity
} from "@shared/types";
import type { MessageStreamState } from "./streaming";
import type { SessionActivityResult } from "./session-activity";
import type { ActiveAssistantModel, FormingSummary, WorkingSummary } from "./assistant-status";
import type { FollowUpBehavior, QueuedMessage } from "./message-queue";

export type SessionReadView = Readonly<Omit<SessionInfo, "workspace">> & {
  readonly workspace: Readonly<WorkspaceIdentity>;
};

export type FormFieldReadView = Readonly<Omit<PendingFormField, "options">> & {
  readonly options?: readonly Readonly<NonNullable<PendingFormField["options"]>[number]>[];
};
export type FormRequestReadView = Readonly<Omit<PendingFormRequest, "fields">> & {
  readonly fields: readonly FormFieldReadView[];
};
export type SessionUsageReadView = Readonly<Omit<SessionUsage, "tokens">> & {
  readonly tokens: Readonly<Omit<SessionUsage["tokens"], "cache">> & {
    readonly cache: Readonly<SessionUsage["tokens"]["cache"]>;
  };
};
export type ProviderUsageReadView = Readonly<Omit<ProviderUsageResult, "snapshot" | "error">> & {
  readonly snapshot: (Readonly<Omit<NonNullable<ProviderUsageResult["snapshot"]>, "windows" | "credits">> & {
    readonly windows: readonly Readonly<NonNullable<ProviderUsageResult["snapshot"]>["windows"][number]>[];
    readonly credits: Readonly<NonNullable<NonNullable<ProviderUsageResult["snapshot"]>["credits"]>> | null;
  }) | null;
  readonly error?: Readonly<NonNullable<ProviderUsageResult["error"]>> | null;
};
export interface Toast {
  id: number;
  text: string;
  tone: "info" | "error";
}

export interface CtxMenuState {
  x: number;
  y: number;
  target: TreeEntry | null;
  workspaceSessionID?: string;
}

export interface CtxMenuApi {
  ctxMenu: CtxMenuState | null;
  openCtxMenu: (x: number, y: number, target: TreeEntry | null, workspaceSessionID?: string) => void;
  closeCtxMenu: () => void;
}

export interface PendingCreate {
  parent: string;
  kind: "file" | "dir";
}

export interface PanelView {
  session: SessionInfo | null;
  busy: boolean;
  transcript: TranscriptItem[];
  todos: TodoItem[];
  sessionUsage: SessionUsage | null;
  compactionBaseline: number | null;
  models: ModelOption[];
  currentModel: ModelOption | null;
  agents: AgentOption[];
  currentAgent: AgentOption | null;
  activity: SessionActivityResult;
  assistantStatus: WorkingSummary | null;
  forming: FormingSummary | null;
  activeModel: ActiveAssistantModel | null;
  streaming: MessageStreamState | null;
  turnStartedAt: number | null;
  queuedCount: number;
  queuedMessages: QueuedMessage[];
  pendingForms: PendingFormRequest[];
  stagedRevert: SessionRevertStage | null;
  restoredPrompt?: { text: string };
}

export interface UnsavedEditorFile {
  workspaceID: string;
  sessionID: string | null;
  workspace: WorkspaceIdentity | null;
  path: string;
  displayPath: string;
  standalone: boolean;
}

export interface Store {
  session: SessionInfo | null;
  connected: boolean;
  runtimes: RuntimeManifest[];
  refreshRuntimes: () => Promise<RuntimeManifest[]>;
  busy: boolean;
  todos: TodoItem[];
  transcript: TranscriptItem[];
  sessionUsage: SessionUsage | null;
  providerUsage: ProviderUsageResult[];
  providerUsageLoading: boolean;
  tabs: Tab[];
  unsavedEditorFileCount: number;
  unsavedEditorFiles: UnsavedEditorFile[];
  activePath: string | null;
  singleFile: string | null;
  agentFiles: Map<string, AgentFileState>;
  workspaceFiles: (workspace: WorkspaceIdentity) => Pick<Store, "tree" | "expanded" | "hiddenPaths" | "agentFiles">;
  tree: Record<string, TreeEntry[]>;
  expanded: Set<string>;
  hiddenPaths: Set<string>;
  toasts: Toast[];
  notify: (text: string, tone?: "info" | "error") => void;
  recoveryRecords: RecoveryRecord[];
  models: ModelOption[];
  availableModels: ModelOption[];
  lastModel: ModelOption | null;
  currentModel: ModelOption | null;
  agents: AgentOption[];
  currentAgent: AgentOption | null;
  approvalMode: ApprovalMode;
  followUpBehavior: FollowUpBehavior;
  setFollowUpBehavior: (behavior: FollowUpBehavior) => void;
  sessions: SessionSummary[];
  savedWorkspaces: ProjectInfo[];
  saveWorkspace: () => Promise<void>;
  removeWorkspace: (directory: string) => void;
  activeSessions: SessionInfo[];
  panels: SessionInfo[];
  workspaceOnlyPanelIDs: Set<string>;
  panelViews: Record<string, PanelView>;
  activeSessionID: string | null;
  focusSession: (sessionID: string) => void;
  closePanel: (sessionID: string, preserveBusy?: boolean) => void;
  openSession: (dir: string) => Promise<SessionInfo | null>;
  addModelPanel: (dir: string) => Promise<void>;
  openWorkspacePanel: (dir: string) => Promise<void>;
  selectAddPanel: () => Promise<SessionInfo | null>;
  selectFolder: () => Promise<void>;
  selectFile: () => Promise<void>;
  openFileWorkspace: (file: string) => Promise<SessionInfo | null>;
  openExternalPath: (absolutePath: string, workspace?: WorkspaceIdentity) => Promise<string | null>;
  importPaths: (destDir: string, sources: string[]) => Promise<void>;
  dropIntoExplorer: (paths: string[]) => Promise<void>;
  openPaths: (paths: string[]) => Promise<void>;
  dismissChange: (path: string) => void;
  dismissChanges: () => void;
  selectPanelDirectory: (workspace: WorkspaceIdentity) => Promise<void>;
  changePanelDirectory: (workspace: WorkspaceIdentity, dir: string) => Promise<void>;
  reopenSession: (sessionID: string, silent?: boolean, directoryHint?: string) => Promise<SessionInfo | null>;
  deleteSession: (sessionID: string) => Promise<void>;
  loadSessions: () => Promise<void>;
  sendPrompt: (text: string, files?: PromptFile[], workspace?: WorkspaceIdentity) => Promise<void>;
  runCommand: (name: string, args?: string, workspace?: WorkspaceIdentity) => Promise<void>;
  stop: (workspace?: WorkspaceIdentity) => Promise<void>;
  refreshProviderUsage: () => Promise<void>;
  loadModels: (workspace?: WorkspaceIdentity) => Promise<void>;
  switchModel: (id: string, providerID: string, variant?: string, workspace?: WorkspaceIdentity) => Promise<void>;
  loadAgents: (workspace?: WorkspaceIdentity) => Promise<void>;
  switchAgent: (id: string, workspace?: WorkspaceIdentity) => Promise<void>;
  toggleApprovalMode: () => void;
  openFile: (path: string, workspace?: WorkspaceIdentity) => Promise<void>;
  previewFile: (path: string, workspace?: WorkspaceIdentity) => Promise<void>;
  closeTab: (path: string) => void;
  setActive: (path: string) => void;
  editContent: (path: string, content: string) => void;
  saveTab: (path: string) => Promise<void>;
  saveAllEditorFiles: () => Promise<boolean>;
  reloadTab: (path: string) => void;
  overwriteTab: (path: string) => Promise<void>;
  mergeTab: (path: string) => void;
  toggleDir: (path: string, workspace?: WorkspaceIdentity) => Promise<void>;
  prefetchDir: (path: string, workspace?: WorkspaceIdentity) => void;
  ensureRootOpen: (workspace?: WorkspaceIdentity) => Promise<void>;
  revealInFileManager: (path: string) => Promise<void>;
  replyPermission: (requestID: string, reply: PermissionReply, sessionID?: string) => Promise<void>;
  removeQueuedMessage: (workspace: WorkspaceIdentity, messageID: string) => void;
  popQueuedMessage: (workspace: WorkspaceIdentity, messageID: string) => QueuedMessage | null;
  sendQueuedNow: (workspace: WorkspaceIdentity, messageID: string) => Promise<void>;
  submitForm: (workspace: WorkspaceIdentity, formID: string, answers: FormAnswers, formSessionID?: string) => Promise<void>;
  dismissForm: (workspace: WorkspaceIdentity, formID: string, formSessionID?: string) => void;
  stageRevert: (workspace: WorkspaceIdentity, messageID: string, edit?: boolean) => Promise<void>;
  clearRestoredPrompt: (workspace: WorkspaceIdentity) => void;
  commitStagedRevert: (workspace: WorkspaceIdentity) => Promise<void>;
  clearStagedRevert: (workspace: WorkspaceIdentity) => Promise<void>;
  reorderQueuedMessage: (workspace: WorkspaceIdentity, fromID: string, toID: string) => void;
  pendingCreate: PendingCreate | null;
  pendingRename: { path: string } | null;
  startCreate: (parent: string, kind: "file" | "dir") => void;
  startRename: (path: string) => void;
  cancelPending: () => void;
  commitName: (name: string) => Promise<void>;
  deleteEntry: (path: string, workspace?: WorkspaceIdentity) => Promise<void>;
  deleteEntries: (paths: string[]) => Promise<void>;
  removeFromWorkspace: (path: string) => void;
  moveEntry: (path: string, destDir: string, workspace?: WorkspaceIdentity) => Promise<void>;
  moveEntries: (paths: string[], destDir: string) => Promise<void>;
  openRecovery: (id: string) => Promise<void>;
  acknowledgeRecovery: (id: string) => Promise<void>;
}
