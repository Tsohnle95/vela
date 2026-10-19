import { useCallback, useRef, useState } from "react";
import { normalizeTodos, sessionUsageValues } from "@shared/session-presentation";
import { retainSessionRecord } from "@shared/retention";
import type { ProviderUsageResult, SessionInfo, SessionUsage, TodoItem, TranscriptItem } from "@shared/types";
import type { ProviderUsageReadView, SessionUsageReadView } from "./store-contract";

function todoToolName(value: string): boolean {
  return value.toLowerCase().replace(/[^a-z]/g, "") === "todowrite";
}

function normalizeSessionUsage(value: unknown): SessionUsage | null {
  if (!value || typeof value !== "object") return null;
  const data = value as Record<string, unknown>;
  const tokens = data.tokens as Record<string, unknown> | undefined;
  if (!tokens || typeof tokens !== "object") return null;
  const cache = tokens.cache as Record<string, unknown> | undefined;
  return sessionUsageValues(data.cost, tokens, cache);
}

function todoSnapshotFromTranscript(items: TranscriptItem[]): { key: string; todos: TodoItem[] } | null {
  for (let itemIndex = items.length - 1; itemIndex >= 0; itemIndex -= 1) {
    const item = items[itemIndex];
    if (item.kind !== "assistant") continue;
    for (let partIndex = item.parts.length - 1; partIndex >= 0; partIndex -= 1) {
      const part = item.parts[partIndex];
      if (part.kind !== "tool" || !todoToolName(part.tool.title)) continue;
      const metadataTodos = part.tool.metadata?.todos;
      if (Array.isArray(metadataTodos)) {
        return { key: `${part.id}:${JSON.stringify(metadataTodos)}`, todos: normalizeTodos(metadataTodos) };
      }
      try {
        const input = JSON.parse(part.tool.input ?? "{}") as { todos?: unknown };
        return { key: `${part.id}:${part.tool.input ?? ""}`, todos: normalizeTodos(input.todos) };
      } catch {
        return { key: `${part.id}:${part.tool.input ?? ""}`, todos: [] };
      }
    }
  }
  return null;
}

interface ProgressHost {
  panelForSession: (sessionID: string) => SessionInfo | null;
  protectedSessionIDs: () => Set<string>;
  notify: (text: string, tone?: "info" | "error") => void;
}

export function useSessionProgress(host: ProgressHost) {
  const { panelForSession, protectedSessionIDs, notify: toast } = host;
  const [todosByWorkspace, setTodosByWorkspace] = useState<Record<string, TodoItem[]>>({});
  const [usageBySession, setUsageBySession] = useState<Record<string, SessionUsage>>({});
  const [compactionBaselineBySession, setCompactionBaselineBySession] = useState<Record<string, number>>(() => {
    try {
      const raw = window.localStorage.getItem("compactionBaseline");
      return raw ? (JSON.parse(raw) as Record<string, number>) : {};
    } catch {
      return {};
    }
  });
  const [providerUsage, setProviderUsage] = useState<ProviderUsageResult[]>([]);
  const [providerUsageLoading, setProviderUsageLoading] = useState(false);
  const usageBySessionRef = useRef(usageBySession);
  usageBySessionRef.current = usageBySession;
  const compactionBaselineBySessionRef = useRef(compactionBaselineBySession);
  compactionBaselineBySessionRef.current = compactionBaselineBySession;
  const todoKeysRef = useRef<Record<string, string>>({});
  const providerUsageSeqRef = useRef(0);

  const setTodosFor = useCallback((workspaceID: string, items: TodoItem[]) => {
    setTodosByWorkspace((current) =>
      current[workspaceID] === items ? current : { ...current, [workspaceID]: items });
  }, []);

  const recordUsage = useCallback((sessionID: string, readUsage: () => SessionUsage): void => {
    setUsageBySession((current) => retainSessionRecord(current, sessionID, readUsage(), protectedSessionIDs()));
  }, [protectedSessionIDs]);

  const refreshSessionUsage = useCallback(async (sessionID: string): Promise<void> => {
    if (!panelForSession(sessionID)) return;
    try {
      const usage = await (window.vela.sessionUsage
        ? window.vela.sessionUsage(sessionID)
        : Promise.resolve(null));
      if (!panelForSession(sessionID) || !usage) return;
      recordUsage(sessionID, () => usage);
    } catch {
    }
  }, [panelForSession, recordUsage]);

  const refreshProviderUsage = useCallback(async () => {
    const sequence = ++providerUsageSeqRef.current;
    setProviderUsageLoading(true);
    try {
      const next = await window.vela.providerUsage();
      if (sequence === providerUsageSeqRef.current) setProviderUsage(next);
    } catch (err) {
      if (sequence === providerUsageSeqRef.current) toast(err instanceof Error ? err.message : String(err), "error");
    } finally {
      if (sequence === providerUsageSeqRef.current) setProviderUsageLoading(false);
    }
  }, [toast]);

  const persistRenderedBaseline = useCallback((compactionBaselineBySession: Readonly<Record<string, number>>): void => {
    try {
      window.localStorage.setItem("compactionBaseline", JSON.stringify(compactionBaselineBySession));
    } catch {}
  }, []);

  const inferRenderedBaselines = useCallback((
    panels: readonly SessionInfo[], transcriptsBySession: Readonly<Record<string, TranscriptItem[]>>,
    usageBySession: Readonly<Record<string, SessionUsageReadView>>, compactionBaselineBySession: Readonly<Record<string, number>>
  ): void => {
    let changed = false;
    const next: Record<string, number> = { ...compactionBaselineBySession };
    for (const panel of panels) {
      const transcript = transcriptsBySession[panel.id];
      const usage = usageBySession[panel.id];
      if (!transcript || !usage) continue;
      if (compactionBaselineBySession[panel.id] != null) continue;
      if (!transcript.some((item) => item.kind === "compaction")) continue;
      if (usage.tokens.input <= 10000) continue;
      next[panel.id] = Math.max(0, usage.tokens.input - 10000);
      changed = true;
    }
    if (changed) setCompactionBaselineBySession(next);
  }, []);

  const inferMaterializedBaseline = useCallback((sessionID: string, transcript: TranscriptItem[], usage: SessionUsage): void => {
    const hasCompaction = transcript.some((item) => item.kind === "compaction");
    const existingBaseline = compactionBaselineBySessionRef.current[sessionID];
    if (hasCompaction && (existingBaseline == null) && usage.tokens.input > 10000) {
      const estimatedWindow = 10000;
      setCompactionBaselineBySession((prev) => ({
        ...prev,
        [sessionID]: Math.max(0, usage.tokens.input - estimatedWindow)
      }));
    }
  }, []);

  const snapshotCompactionBaseline = useCallback((targetSessionID: string): void => {
    const currentInput = usageBySessionRef.current[targetSessionID]?.tokens.input ?? 0;
    if (currentInput > 0) {
      setCompactionBaselineBySession((prev) => ({ ...prev, [targetSessionID]: currentInput }));
    }
  }, []);

  const removeClosedBaseline = useCallback((sessionID: string): void => {
    setCompactionBaselineBySession((current) => {
      if (!(sessionID in current)) return current;
      const { [sessionID]: _dropped, ...rest } = current;
      void _dropped;
      return rest;
    });
  }, []);

  const applyBusyTodoFallback = useCallback((
    panels: readonly SessionInfo[], busyBySession: Readonly<Record<string, boolean>>,
    transcriptsBySession: Readonly<Record<string, TranscriptItem[]>>
  ): void => {
    for (const panel of panels) {
      if (!busyBySession[panel.id]) continue;
      const snapshot = todoSnapshotFromTranscript(transcriptsBySession[panel.id] ?? []);
      if (!snapshot || snapshot.key === todoKeysRef.current[panel.workspace.id]) continue;
      todoKeysRef.current[panel.workspace.id] = snapshot.key;
      setTodosFor(panel.workspace.id, snapshot.todos);
    }
  }, [setTodosFor]);

  return {
    todosByWorkspace: todosByWorkspace as Readonly<Record<string, readonly Readonly<TodoItem>[]>>,
    usageBySession: usageBySession as Readonly<Record<string, SessionUsageReadView>>,
    compactionBaselineBySession: compactionBaselineBySession as Readonly<Record<string, number>>,
    providerUsage: providerUsage as readonly ProviderUsageReadView[], providerUsageLoading,
    setTodosFor, recordUsage, refreshSessionUsage, refreshProviderUsage, normalizeSessionUsage,
    persistRenderedBaseline, inferRenderedBaselines, inferMaterializedBaseline,
    snapshotCompactionBaseline, removeClosedBaseline, applyBusyTodoFallback
  };
}
