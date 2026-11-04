import type { TranscriptItem } from "@shared/types";
import { applyChatEvent, projectAssistantItems, type ChatDirectoryState } from "./chat-store";
import type { ChatStreamEvent } from "./chat-stream";

export const LARGE_SESSION_TURNS = 400;
export const LARGE_SESSION_OUTPUT_CHARS = 32 * 1024;
const TEXT = "The benchmark response is deterministic and intentionally short.";
const OUTPUT = "x".repeat(LARGE_SESSION_OUTPUT_CHARS);

function event(id: string, type: string, data: Record<string, unknown>): ChatStreamEvent {
  return { id, type, created: 1_000, data };
}

export function largeSessionEvents(turns = LARGE_SESSION_TURNS): ChatStreamEvent[] {
  return Array.from({ length: turns }, (_, index) => {
    const messageID = `assistant-${index}`;
    const callID = `tool-${index}`;
    return [
      event(`step-${index}`, "session.step.started", { sessionID: "session", assistantMessageID: messageID }),
      event(`text-${index}`, "session.text.ended", { sessionID: "session", assistantMessageID: messageID, ordinal: 0, text: TEXT }),
      event(`tool-start-${index}`, "session.tool.input.started", { sessionID: "session", assistantMessageID: messageID, callID, name: "bash" }),
      event(`tool-call-${index}`, "session.tool.called", { sessionID: "session", assistantMessageID: messageID, callID, input: { command: `printf ${index}` } }),
      event(`tool-end-${index}`, "session.tool.success", { sessionID: "session", assistantMessageID: messageID, callID, content: [{ type: "text", text: OUTPUT }] }),
      event(`step-end-${index}`, "session.step.ended", { sessionID: "session", assistantMessageID: messageID })
    ];
  }).flat();
}

export function reduceLargeSession(turns = LARGE_SESSION_TURNS): TranscriptItem[] {
  const state: ChatDirectoryState = { message: {}, part: {}, session_status: {} };
  for (const streamEvent of largeSessionEvents(turns)) {
    applyChatEvent(state, "session", streamEvent);
  }
  return projectAssistantItems(state, "session");
}

export function retainedOutputChars(items: TranscriptItem[]): number {
  let count = 0;
  for (const item of items) {
    if (item.kind === "shell") count += item.output?.length ?? 0;
    if (item.kind !== "assistant") continue;
    for (const part of item.parts) {
      if (part.kind !== "tool") continue;
      count += part.tool.output?.length ?? 0;
      for (const content of part.tool.content ?? []) {
        if (content.type === "text") count += content.text.length;
      }
    }
  }
  return count;
}

export function timelineDomSizeProxy(items: TranscriptItem[]): number {
  let rows = 0;
  for (const item of items) {
    if (item.kind === "user" || item.kind === "shell" || item.kind === "compaction" || item.kind === "status" || item.kind === "divider") rows += 1;
    if (item.kind === "synthetic" || item.kind === "skill") rows += 1;
    if (item.kind === "assistant") rows += item.parts.filter((part) => part.kind === "tool" || part.text.trim()).length;
  }
  return rows;
}
