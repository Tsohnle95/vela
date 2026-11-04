import type {
  ModelOption,
  RuntimeManifest,
  SessionInfo,
  SessionSelection,
  SessionSummary,
  SessionTranscript
} from "@shared/types";

export const RUNTIME_PROTOCOL_VERSION = 1 as const;

export interface RuntimeSessionDraft {
  id: string;
  directory: string;
  title?: string;
  parentID?: string;
  agent?: string;
}

export interface RuntimeEventEnvelope {
  runtimeID: string;
  eventID: string;
  sessionID?: string;
  event: RuntimeEvent;
}

export type RuntimeEvent =
  | { type: "execution.started" }
  | { type: "execution.idle" }
  | { type: "execution.error"; code?: string; message: string }
  | { type: "stream.event"; eventType: string; data: Record<string, unknown>; created: number }
  | { type: "transcript.changed" }
  | { type: "todo.updated"; todos: unknown[] };

export interface RuntimeAdapter {
  readonly manifest: RuntimeManifest;
  connect(): Promise<boolean>;
  createSession(directory: string): Promise<RuntimeSessionDraft>;
  listSessions(): Promise<SessionSummary[]>;
  sessionInfo(sessionID: string): Promise<RuntimeSessionDraft>;
  sessionTranscript(sessionID: string): Promise<SessionTranscript>;
  prompt(sessionID: string, text: string): Promise<void>;
  interrupt(sessionID: string): Promise<void>;
  listModels(sessionID: string): Promise<ModelOption[]>;
  sessionSelection(sessionID: string): Promise<SessionSelection | null>;
  switchModel(sessionID: string, modelID: string, providerID: string, variant?: string): Promise<void>;
  subscribe(signal: AbortSignal): AsyncIterable<RuntimeEventEnvelope>;
  stop(): Promise<void>;
}

export function sessionInfoFromRuntime(
  draft: RuntimeSessionDraft,
  runtimeID: string,
  workspace: SessionInfo["workspace"]
): SessionInfo {
  return { ...draft, runtimeID, workspace };
}
