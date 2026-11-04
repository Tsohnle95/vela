import type { FileWriteIdentity, PermissionReply, PromptFile, ProviderCredentialAnswers, RuntimeID, WorkspaceIdentity } from "@shared/types";
import { fileContent, relativePath, workspaceId } from "./workspace-security";

export const IPC_LIMITS = {
  identifier: 512,
  directory: 4096,
  prompt: 1024 * 1024,
  promptFiles: 100,
  command: 256,
  commandArgs: 1024 * 1024,
  query: 4096,
  mentionText: 4096,
  providerKey: 64 * 1024,
  providerLabel: 256,
  providerAnswers: 32,
  providerAnswer: 4096
} as const;

export function boundedString(value: unknown, name: string, max: number, allowEmpty = false): string {
  if (typeof value !== "string" || (!allowEmpty && value.length === 0) || value.length > max || value.includes("\0")) {
    throw new Error(`invalid ${name}`);
  }
  return value;
}

export function activationGeneration(value: unknown): number {
  if (!Number.isSafeInteger(value) || (value as number) < 1) throw new Error("invalid activation generation");
  return value as number;
}

export function directoryPath(value: unknown): string {
  return boundedString(value, "directory", IPC_LIMITS.directory);
}

export function sessionId(value: unknown): string {
  return boundedString(value, "session id", IPC_LIMITS.identifier);
}

export function runtimeId(value: unknown): RuntimeID {
  if (value !== "opencode" && value !== "deepseek") throw new Error("invalid runtime id");
  return value;
}

export function optionalRuntimeId(value: unknown): RuntimeID | undefined {
  return value === undefined ? undefined : runtimeId(value);
}

export function promptPayload(workspace: unknown, text: unknown, files: unknown): {
  workspace: WorkspaceIdentity;
  text: string;
  files: PromptFile[];
} {
  workspaceId(workspace);
  const cleanText = boundedString(text, "prompt text", IPC_LIMITS.prompt, true);
  if (!Array.isArray(files) || files.length > IPC_LIMITS.promptFiles) throw new Error("invalid prompt files");
  const cleanFiles = files.map((value): PromptFile => {
    if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid prompt file");
    const keys = Object.keys(value);
    const file = value as { path?: unknown; mention?: unknown };
    if (keys.some((key) => key !== "path" && key !== "mention")) throw new Error("invalid prompt file");
    const path = boundedString(file.path, "prompt file path", IPC_LIMITS.directory);
    if (file.mention === undefined) return { path };
    if (!file.mention || typeof file.mention !== "object" || Array.isArray(file.mention)) throw new Error("invalid prompt mention");
    const mention = file.mention as { start?: unknown; end?: unknown; text?: unknown };
    if (Object.keys(mention).some((key) => !["start", "end", "text"].includes(key)) ||
        !Number.isSafeInteger(mention.start) || !Number.isSafeInteger(mention.end) ||
        (mention.start as number) < 0 || (mention.end as number) < (mention.start as number) ||
        (mention.end as number) > cleanText.length) throw new Error("invalid prompt mention");
    return {
      path,
      mention: {
        start: mention.start as number,
        end: mention.end as number,
        text: boundedString(mention.text, "prompt mention text", IPC_LIMITS.mentionText)
      }
    };
  });
  return { workspace: workspace as WorkspaceIdentity, text: cleanText, files: cleanFiles };
}

export function commandPayload(name: unknown, args: unknown): { name: string; args: string } {
  return {
    name: boundedString(name, "command", IPC_LIMITS.command),
    args: boundedString(args, "command arguments", IPC_LIMITS.commandArgs, true)
  };
}

export function queryText(value: unknown): string {
  return boundedString(value, "query", IPC_LIMITS.query, true);
}

export function selectionId(value: unknown, name: string): string {
  return boundedString(value, name, IPC_LIMITS.identifier);
}

export function optionalSelectionId(value: unknown, name: string): string | undefined {
  return value === undefined ? undefined : selectionId(value, name);
}

export function providerCredentialPayload(integrationID: unknown, key: unknown, label: unknown, answers: unknown): {
  integrationID: string;
  key: string;
  label?: string;
  answers: ProviderCredentialAnswers;
} {
  if (!answers || typeof answers !== "object" || Array.isArray(answers)) throw new Error("invalid provider answers");
  const entries = Object.entries(answers);
  if (entries.length > IPC_LIMITS.providerAnswers) throw new Error("invalid provider answers");
  const cleanAnswers: ProviderCredentialAnswers = {};
  for (const [answerKey, value] of entries) {
    const cleanKey = selectionId(answerKey, "provider answer key");
    if (typeof value === "string") {
      cleanAnswers[cleanKey] = boundedString(value, "provider answer", IPC_LIMITS.providerAnswer, true);
    } else if (typeof value === "number" && Number.isFinite(value)) {
      cleanAnswers[cleanKey] = value;
    } else if (typeof value === "boolean") {
      cleanAnswers[cleanKey] = value;
    } else if (Array.isArray(value) && value.length <= IPC_LIMITS.providerAnswers) {
      cleanAnswers[cleanKey] = value.map((item) => boundedString(item, "provider answer", IPC_LIMITS.providerAnswer, true));
    } else {
      throw new Error("invalid provider answer");
    }
  }
  return {
    integrationID: selectionId(integrationID, "provider integration id"),
    key: boundedString(key, "provider key", IPC_LIMITS.providerKey),
    ...(label === undefined || label === "" ? {} : { label: boundedString(label, "provider label", IPC_LIMITS.providerLabel) }),
    answers: cleanAnswers
  };
}

export function permissionPayload(requestID: unknown, reply: unknown, sessionID: unknown): {
  requestID: string;
  reply: PermissionReply;
  sessionID: string;
} {
  if (reply !== "once" && reply !== "always" && reply !== "reject") throw new Error("invalid permission reply");
  return {
    requestID: selectionId(requestID, "permission request id"),
    reply,
    sessionID: sessionId(sessionID)
  };
}

export function fileWriteIdentity(value: unknown, workspace: WorkspaceIdentity): FileWriteIdentity {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid file write identity");
  const write = value as Partial<FileWriteIdentity>;
  if (Object.keys(write).some((key) => !["id", "workspaceID", "revision", "expectedContent", "overwrite"].includes(key)) ||
      selectionId(write.id, "file write id") !== write.id || write.workspaceID !== workspace.id ||
      !Number.isSafeInteger(write.revision) || (write.revision as number) < 0 || typeof write.overwrite !== "boolean") {
    throw new Error("invalid file write identity");
  }
  return { ...write, expectedContent: fileContent(write.expectedContent) } as FileWriteIdentity;
}

export function workspacePath(workspace: unknown, rel: unknown, allowRoot = false): {
  workspace: WorkspaceIdentity;
  rel: string;
} {
  workspaceId(workspace);
  return { workspace: workspace as WorkspaceIdentity, rel: relativePath(rel, allowRoot) };
}

export function movePayload(workspace: unknown, rel: unknown, newParent: unknown): {
  workspace: WorkspaceIdentity;
  rel: string;
  newParent: string;
} {
  workspaceId(workspace);
  return {
    workspace: workspace as WorkspaceIdentity,
    rel: relativePath(rel),
    newParent: relativePath(newParent, true)
  };
}
