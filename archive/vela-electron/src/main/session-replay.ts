import { normalizeTodos } from "@shared/session-presentation";
import type { AssistantPartView, TodoItem, ToolCallView, TranscriptItem } from "@shared/types";
import { messagePresentation } from "@shared/message-presentation";
import { retainToolContent } from "@shared/retention";
import { formatFailure } from "@shared/errors";
import { collectFilePaths, record, toolContentViews, toolDetailText } from "@shared/tool-presentation";

function toolInputText(input: unknown): string {
  if (input == null) return "";
  try {
    return JSON.stringify(input, null, 2);
  } catch {
    return String(input);
  }
}

function toolTitleText(input: unknown, name: string): string {
  if (name) return name;
  if (!input || typeof input !== "object") return "tool";
  const o = input as Record<string, unknown>;
  if (typeof o.tool === "string" && o.tool) return o.tool;
  if ("command" in o) return "bash";
  if ("filePath" in o || "file_path" in o) return "file";
  if ("query" in o) return "search";
  if ("url" in o) return "web";
  if ("prompt" in o) return "prompt";
  return "tool";
}

function toolContentText(content: unknown[] | undefined): string {
  if (!Array.isArray(content)) return "";
  return content
    .filter((c) => (c as { type?: string })?.type === "text")
    .map((c) => String((c as { text?: unknown }).text ?? ""))
    .join("\n");
}

function replayToolCard(part: Record<string, unknown>): ToolCallView {
  const state = (part.state ?? {}) as Record<string, unknown>;
  const input = state.input;
  const status = ["completed", "success"].includes(String(state.status))
    ? "success"
    : ["error", "failed", "aborted", "timeout", "cancelled"].includes(String(state.status))
      ? "failed"
      : "running";
  const time = (part.time ?? state.time ?? {}) as Record<string, number | undefined>;
  const ran = time.ran ?? time.created ?? Date.now();
  const completed = time.completed;
  return {
    id: String(part.callID ?? part.id),
    title: toolTitleText(input, String(part.name ?? part.tool ?? "")),
    detail: toolDetailText(input),
    status,
    input: toolInputText(input),
    output:
      toolContentText(state.content as unknown[] | undefined) ||
      (state.error !== undefined
        ? formatFailure(state.error, "VELA_TOOL_FAILED", "Tool failed")
        : String(state.output ?? "")),
    startedAt: time.created ?? Date.now(),
    duration: completed ? Math.max(0, completed - ran) : undefined,
    paths: collectFilePaths(input),
    metadata: record(state.metadata),
    inputValue: input,
    content: retainToolContent(toolContentViews(state.content)),
    ...(typeof part.executed === "boolean" ? { executed: part.executed } : {}),
    providerState: record(part.providerState),
    providerResultState: record(part.providerResultState)
  };
}

export function replayTodos(messages: unknown[]): TodoItem[] {
  let result: TodoItem[] = [];
  for (const raw of messages) {
    const msg = raw as Record<string, unknown>;
    const info = (msg.info ?? msg) as Record<string, unknown>;
    const parts = Array.isArray(msg.parts)
      ? msg.parts as Record<string, unknown>[]
      : Array.isArray(info.content)
        ? info.content as Record<string, unknown>[]
        : [];
    for (const part of parts) {
      const name = String(part.tool ?? part.name).toLowerCase().replace(/[^a-z]/g, "");
      if (part.type !== "tool" || name !== "todowrite") continue;
      const state = part.state && typeof part.state === "object"
        ? part.state as Record<string, unknown>
        : {};
      const metadata = state.metadata && typeof state.metadata === "object"
        ? state.metadata as Record<string, unknown>
        : {};
      const input = state.input && typeof state.input === "object"
        ? state.input as Record<string, unknown>
        : {};
      const next = normalizeTodos(metadata.todos ?? input.todos);
      if (next.length > 0) result = next;
    }
  }
  return result;
}

export function replayTranscript(messages: unknown[]): TranscriptItem[] {
  const out: TranscriptItem[] = [];
  let agent: string | undefined;
  let model: { id: string; providerID: string } | undefined;
  let parentID: string | undefined;
  for (const raw of messages) {
    const msg = raw as Record<string, unknown>;
    const info = (msg.info ?? msg) as Record<string, unknown>;
    const parts = Array.isArray(msg.parts) ? (msg.parts as Record<string, unknown>[]) : [];
    const type = info.type ?? info.role;
    if (type === "user") {
      parentID = String(info.id);
      const text = String(
        info.text ?? parts.filter((part) => part.type === "text").map((part) => part.text ?? "").join("\n")
      );
      const files = Array.isArray(info.files) ? info.files as Record<string, unknown>[] : [];
      const attachments = files.flatMap((file): { name: string }[] =>
        typeof file.name === "string" && file.name ? [{ name: file.name }] : []
      );
      if (text.trim()) {
        out.push({
          kind: "user",
          id: String(info.id),
          text,
          ...(agent !== undefined ? { agent } : {}),
          ...(model ? { model } : {}),
          ...messagePresentation(info),
          ...(attachments.length > 0 ? { attachments } : {})
        });
      }
      continue;
    }
    if (type === "agent-switched") {
      agent = typeof info.agent === "string" ? info.agent : undefined;
      out.push({
        kind: "selection",
        id: String(info.id),
        selection: "agent",
        title: "Agent switched",
        detail: String(info.agent ?? "")
      });
      continue;
    }
    if (type === "model-switched") {
      model = messagePresentation(info).model;
      const id = model?.id ?? "";
      const provider = model?.providerID ?? "";
      out.push({
        kind: "selection",
        id: String(info.id),
        selection: "model",
        title: "Model switched",
        detail: [provider, id].filter(Boolean).join(" / ")
      });
      continue;
    }
    if (type === "synthetic") {
      if (typeof info.description === "string" && info.description.trim()) parentID = String(info.id);
      out.push({
        kind: "synthetic",
        id: String(info.id),
        text: String(info.text ?? ""),
        ...(typeof info.description === "string" ? { description: info.description } : {})
      });
      continue;
    }
    if (type === "system") {
      out.push({ kind: "system", id: String(info.id), text: String(info.text ?? "") });
      continue;
    }
    if (type === "skill") {
      out.push({
        kind: "skill",
        id: String(info.id),
        skill: String(info.skill ?? ""),
        name: String(info.name ?? info.skill ?? "Skill"),
        text: String(info.text ?? "")
      });
      continue;
    }
    if (type === "shell") {
      parentID = undefined;
      const output = record(info.output);
      const status = ["running", "exited", "timeout", "killed"].includes(String(info.status))
        ? String(info.status) as "running" | "exited" | "timeout" | "killed"
        : "running";
      out.push({
        kind: "shell",
        id: String(info.id),
        shellID: String(info.shellID ?? info.id),
        command: String(info.command ?? ""),
        status,
        ...(typeof output?.output === "string" ? { output: output.output } : {}),
        ...(["number", "string"].includes(typeof info.exit) ? { exit: info.exit as number | string } : {})
      });
      continue;
    }
    if (type === "assistant") {
      const presentation = messagePresentation(info);
      agent = presentation.agent ?? agent;
      model = presentation.model ?? model;
      const parent = out.findLast((item) => item.id === (presentation.parentID ?? parentID));
      if (parent?.kind === "user") {
        if (presentation.agent !== undefined) parent.agent = presentation.agent;
        if (presentation.model) parent.model = presentation.model;
      }
      const content = parts.length > 0
        ? parts
        : Array.isArray(info.content) ? (info.content as Record<string, unknown>[]) : [];
      const completed = Boolean(
        (info.time as Record<string, unknown> | undefined)?.completed ?? info.finish ?? info.error
      );
      // V2 stream ordinals count each part type independently, not positions in content.
      const ordinals = { text: 0, reasoning: 0 };
      const assistantParts = content.flatMap((part, index): AssistantPartView[] => {
        const ordinal = part.type === "text" || part.type === "reasoning" ? ordinals[part.type]++ : index;
        const id = String(part.id ?? `${String(info.id)}:${String(part.type)}:${ordinal}`);
        if (part.type === "text" || part.type === "reasoning") {
          const time = (part.time ?? {}) as Record<string, unknown>;
          return [{
            kind: part.type,
            id,
            text: String(part.text ?? ""),
            complete: Boolean(time.end ?? time.completed ?? completed)
          }];
        }
        if (part.type === "tool") return [{ kind: "tool", id, tool: replayToolCard(part) }];
        return [];
      });
      if (assistantParts.length > 0 || !completed || info.retry || info.error) {
        out.push({
          kind: "assistant",
          id: String(info.id),
          messageID: String(info.id),
          parts: assistantParts,
          completed,
          ...(parentID ? { parentID } : {}),
          ...presentation,
          ...(info.retry && typeof info.retry === "object"
            ? {
                retry: {
                  attempt: Number((info.retry as Record<string, unknown>).attempt ?? 1),
                  message: formatFailure(
                    (info.retry as Record<string, unknown>).error,
                    "VELA_RETRY_SCHEDULED",
                    "Retrying"
                  ),
                  next: Number((info.retry as Record<string, unknown>).at ?? 0) || undefined
                }
              }
            : {}),
          ...(info.error ? { error: formatFailure(info.error, "VELA_ASSISTANT_FAILED", "Assistant failed") } : {})
        });
      }
      continue;
    }
    if (type === "compaction") {
      const status = ["running", "completed", "failed"].includes(String(info.status))
        ? String(info.status) as "running" | "completed" | "failed"
        : "completed";
      out.push({
        kind: "compaction",
        id: String(info.id),
        status,
        reason: info.reason === "manual" ? "manual" : "auto",
        summary: String(info.summary ?? ""),
        ...(typeof info.recent === "string" ? { recent: info.recent } : {}),
        ...(info.error ? { error: formatFailure(info.error, "VELA_COMPACTION_FAILED", "Compaction failed") } : {})
      });
    }
  }
  return out;
}

