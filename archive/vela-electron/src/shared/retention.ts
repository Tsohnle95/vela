import type { ToolContentView } from "./types";

export const MAX_INACTIVE_SESSION_RECORDS = 4;
export interface SessionTokenUsage {
  input?: number;
  output?: number;
  reasoning?: number;
  cache?: { read?: number; write?: number };
}

export function hasConversation(title: string | undefined, tokens: SessionTokenUsage | undefined): boolean {
  if (typeof title === "string" && title.trim()) return true;
  if (!tokens) return false;
  return Boolean(tokens.input || tokens.output || tokens.reasoning || tokens.cache?.read || tokens.cache?.write);
}

export function retainMatchingSessionRecords<T>(
  records: Record<string, T>,
  retained: Record<string, unknown>,
  activeSessionID?: string
): Record<string, T> {
  const entries = Object.entries(records).filter(([id]) => id === activeSessionID || id in retained);
  return entries.length === Object.keys(records).length ? records : Object.fromEntries(entries);
}

export function retainToolContent(content: ToolContentView[] | undefined): ToolContentView[] | undefined {
  const files = content?.filter((item) => item.type === "file");
  return files?.length ? files : undefined;
}

export function retainSessionRecord<T>(
  records: Record<string, T>,
  sessionID: string,
  value: T,
  protectedIDs?: string | ReadonlySet<string>
): Record<string, T> {
  const next = { ...records };
  delete next[sessionID];
  next[sessionID] = value;
  const protectedSet = protectedIDs instanceof Set
    ? protectedIDs
    : new Set(protectedIDs ? [protectedIDs] : []);
  const inactive = Object.keys(next).filter((id) => !protectedSet.has(id));
  for (const id of inactive.slice(0, -MAX_INACTIVE_SESSION_RECORDS)) delete next[id];
  return next;
}
