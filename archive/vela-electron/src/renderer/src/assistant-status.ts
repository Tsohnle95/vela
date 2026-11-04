import type { ChatMessageReadView, ChatPartReadView } from "./chat-store";
import type { SessionActivityResult, SessionActivityPhase } from "./session-activity";
import { isFullySyntheticMessage } from "./messages/synthetic";

export type AssistantActivity = "idle" | "streaming" | "tooling" | "cooldown" | "permission";

export type AssistantStreamPhase = "streaming" | "cooldown" | "completed";

export interface WorkingSummary {
  activity: AssistantActivity;
  hasWorkingContext: boolean;
  hasActiveTools: boolean;
  isWorking: boolean;
  isStreaming: boolean;
  isCooldown: boolean;
  lifecyclePhase: AssistantStreamPhase | null;
  statusText: string | null;
  isGenericStatus: boolean;
  isWaitingForPermission: boolean;
  canAbort: boolean;
  compactionDeadline: number | null;
  activePartType?: "text" | "tool" | "reasoning" | "editing";
  activeToolName?: string;
  wasAborted: boolean;
  abortActive: boolean;
  lastCompletionId: string | null;
  isComplete: boolean;
  retryInfo: { attempt?: number; next?: number } | null;
}

export interface FormingSummary {
  isActive: boolean;
  characterCount: number;
}

export interface AssistantStatusSnapshot {
  activeModel: ActiveAssistantModel | null;
  forming: FormingSummary;
  working: WorkingSummary;
}

export interface ActiveAssistantModel {
  providerId: string;
  modelId: string;
}

interface ActiveAssistantContext {
  assistantId: string | null;
  model: ActiveAssistantModel | null;
}

export interface SessionAbortFlag {
  timestamp: number;
  acknowledged: boolean;
}

const EMPTY_PARTS: ChatPartReadView[] = [];
const STATUS_SIGNATURE_SEPARATOR = "\u0000";
const EDITING_TOOLS = new Set(["edit", "write", "multiedit", "apply_patch"]);
const TOOL_STATUS_PHRASES: Record<string, string> = {
  read: "reading file",
  write: "writing file",
  edit: "editing file",
  multiedit: "editing files",
  apply_patch: "applying patch",
  bash: "running command",
  grep: "searching content",
  glob: "finding files",
  list: "listing directory",
  task: "delegating task",
  webfetch: "fetching URL",
  websearch: "searching web",
  codesearch: "web code search",
  todowrite: "updating todos",
  todoread: "reading todos",
  skill: "learning skill",
  question: "asking question",
  plan_enter: "switching to planning",
  plan_exit: "switching to building"
};

export type ParsedStatusResult = {
  activePartType: "text" | "tool" | "reasoning" | "editing" | undefined;
  activeToolName: string | undefined;
  statusText: string;
  isGenericStatus: boolean;
};

const getToolStatusPhrase = (toolName: string): string => {
  return TOOL_STATUS_PHRASES[toolName] ?? `using ${toolName}`;
};

function getPartTimeInfo(part: ChatPartReadView): { end?: number } | undefined {
  if (part.type === "text" || part.type === "reasoning") {
    const time = part.time as { end?: number } | undefined;
    return time && typeof time.end === "number" ? time : undefined;
  }
  const candidate = part.time as { end?: number } | undefined;
  return candidate && typeof candidate.end === "number" ? candidate : undefined;
}

function getToolDisplayName(part: ChatPartReadView): string {
  if (typeof part.tool === "string") {
    return part.tool;
  }
  return typeof part.name === "string" ? part.name : "tool";
}

function getLegacyTextContent(part: ChatPartReadView): string | undefined {
  if (typeof part.text === "string") {
    return part.text;
  }
  const candidate = part as Partial<{ content?: unknown; value?: unknown }>;
  if (typeof candidate.content === "string") {
    return candidate.content;
  }
  if (typeof candidate.value === "string") {
    return candidate.value;
  }
  return undefined;
}

export function createParsedStatus(parts: readonly ChatPartReadView[], _genericKey: string): ParsedStatusResult {
  let activePartType: ParsedStatusResult["activePartType"] = undefined;
  let activeToolName: string | undefined = undefined;

  if (!isFullySyntheticMessage(parts)) {
    for (let index = parts.length - 1; index >= 0; index -= 1) {
      const part = parts[index];
      if (!part) continue;

      switch (part.type) {
        case "reasoning": {
          const time = part.time ?? getPartTimeInfo(part);
          const stillRunning = !time || typeof (time as { end?: number }).end === "undefined";
          if (stillRunning && !activePartType) {
            activePartType = "reasoning";
          }
          break;
        }
        case "tool": {
          const toolStatus = part.state?.status;
          if ((toolStatus === "running" || toolStatus === "pending") && !activePartType) {
            const toolName = getToolDisplayName(part);
            if (EDITING_TOOLS.has(toolName)) {
              activePartType = "editing";
              activeToolName = toolName;
            } else {
              activePartType = "tool";
              activeToolName = toolName;
            }
          }
          break;
        }
        case "text": {
          const rawContent = getLegacyTextContent(part) ?? "";
          if (typeof rawContent === "string" && rawContent.trim().length > 0) {
            const time = getPartTimeInfo(part);
            const streamingPart = !time || typeof time.end === "undefined";
            if (streamingPart && !activePartType) {
              activePartType = "text";
            }
          }
          break;
        }
        default:
          break;
      }
    }
  }

  const isGenericStatus = activePartType === undefined;
  const statusText = (() => {
    if (activePartType === "editing") return activeToolName === "multiedit" ? getToolStatusPhrase(activeToolName) : "editing file";
    if (activePartType === "tool" && activeToolName) return getToolStatusPhrase(activeToolName);
    if (activePartType === "reasoning") return "thinking";
    if (activePartType === "text") return "composing";
    return "preparing response";
  })();

  return { activePartType, activeToolName, statusText, isGenericStatus };
}

export function encodeParsedStatus(status: ParsedStatusResult): string {
  return [
    status.activePartType ?? "",
    status.activeToolName ?? "",
    status.statusText,
    status.isGenericStatus ? "1" : "0"
  ].join(STATUS_SIGNATURE_SEPARATOR);
}

export function decodeParsedStatus(signature: string): ParsedStatusResult {
  const [activePartType, activeToolName, statusText = "working", isGenericStatus] = signature.split(STATUS_SIGNATURE_SEPARATOR);
  return {
    activePartType: activePartType === "text" || activePartType === "tool" || activePartType === "reasoning" || activePartType === "editing"
      ? activePartType
      : undefined,
    activeToolName: activeToolName || undefined,
    statusText,
    isGenericStatus: isGenericStatus === "1"
  };
}

export function getActiveAssistantContext(messages: readonly ChatMessageReadView[]): ActiveAssistantContext {
  let assistantId: string | null = null;
  let parentId: string | null = null;

  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message?.role !== "assistant") continue;

    const candidate = message as ChatMessageReadView & { parentID?: unknown };
    assistantId = message.id;
    parentId = typeof candidate.parentID === "string" && candidate.parentID.trim().length > 0
      ? candidate.parentID
      : null;
    break;
  }

  if (!assistantId || !parentId) {
    return { assistantId, model: null };
  }

  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message?.role !== "user" || message.id !== parentId) continue;

    const candidate = message as ChatMessageReadView & {
      model?: { providerID?: unknown; modelID?: unknown };
    };
    const providerId = typeof candidate.model?.providerID === "string"
      ? candidate.model.providerID.trim()
      : "";
    const modelId = typeof candidate.model?.modelID === "string"
      ? candidate.model.modelID.trim()
      : "";

    return {
      assistantId,
      model: providerId && modelId ? { providerId, modelId } : null
    };
  }

  return { assistantId, model: null };
}

export function resolveAssistantStatus(input: {
  assistantId: string | null;
  model: ActiveAssistantModel | null;
  activity: SessionActivityResult;
  parts: readonly ChatPartReadView[];
  pendingPermissions: number;
  pendingQuestions?: number;
  abortFlag: SessionAbortFlag | null;
  retryInfo: { attempt?: number; next?: number } | null;
}): AssistantStatusSnapshot | null {
  const { assistantId, model, activity, parts, pendingPermissions, abortFlag, retryInfo } = input;
  const pendingQuestions = input.pendingQuestions ?? 0;
  if (!assistantId) return null;

  const parsedStatus = createParsedStatus(parts ?? EMPTY_PARTS, `${assistantId}`);
  const abortState = { wasAborted: Boolean(abortFlag && !abortFlag.acknowledged), abortActive: Boolean(abortFlag && !abortFlag.acknowledged) };

  const isWorking = activity.isWorking;
  const isStreaming = activity.phase === "busy";
  const isCooldown = false;
  const isRetry = activity.phase === "retry";

  const baseWorking: WorkingSummary = abortState.wasAborted
    ? {
        activity: "idle",
        hasWorkingContext: false,
        hasActiveTools: false,
        isWorking: false,
        isStreaming: false,
        isCooldown: false,
        lifecyclePhase: null,
        statusText: null,
        isGenericStatus: true,
        isWaitingForPermission: false,
        canAbort: false,
        compactionDeadline: null,
        activePartType: undefined,
        activeToolName: undefined,
        wasAborted: true,
        abortActive: abortState.abortActive,
        lastCompletionId: null,
        isComplete: false,
        retryInfo: null
      }
    : (() => {
        let activityKind: AssistantActivity = "idle";
        if (isWorking) {
          if (parsedStatus.activePartType === "tool" || parsedStatus.activePartType === "editing") {
            activityKind = "tooling";
          } else {
            activityKind = isCooldown ? "cooldown" : "streaming";
          }
        }
        return {
          activity: activityKind,
          hasWorkingContext: isWorking,
          hasActiveTools: parsedStatus.activePartType === "tool" || parsedStatus.activePartType === "editing",
          isWorking,
          isStreaming,
          isCooldown,
          lifecyclePhase: isStreaming ? "streaming" : isCooldown ? "cooldown" : null,
          statusText: isWorking ? parsedStatus.statusText : null,
          isGenericStatus: isWorking ? parsedStatus.isGenericStatus : true,
          isWaitingForPermission: false,
          canAbort: isWorking,
          compactionDeadline: null,
          activePartType: isWorking ? parsedStatus.activePartType : undefined,
          activeToolName: isWorking ? parsedStatus.activeToolName : undefined,
          wasAborted: false,
          abortActive: false,
          lastCompletionId: null,
          isComplete: false,
          retryInfo: isRetry ? { attempt: retryInfo?.attempt, next: retryInfo?.next } : null
        };
      })();

  const hasPendingPermission = pendingPermissions > 0;
  const hasPendingQuestion = pendingQuestions > 0;

  let working = baseWorking;
  if (hasPendingPermission || hasPendingQuestion) {
    if (hasPendingQuestion) {
      working = {
        ...baseWorking,
        statusText: null,
        isWorking: false,
        hasWorkingContext: false,
        hasActiveTools: false,
        canAbort: false,
        activePartType: undefined,
        activeToolName: undefined,
        retryInfo: null
      };
    } else {
      working = {
        ...baseWorking,
        statusText: "waiting for permission",
        isWaitingForPermission: true,
        canAbort: false,
        retryInfo: null
      };
    }
  }

  const forming: FormingSummary = {
    isActive: isWorking && parsedStatus.activePartType === "text",
    characterCount: 0
  };

  return {
    activeModel: model,
    forming,
    working
  };
}

export type { SessionActivityPhase };
