import { useEffect, useRef, useState, type Dispatch, type RefObject, type SetStateAction } from "react";
import type { PromptFile, WorkspaceIdentity } from "@shared/types";
import { sameWorkspace } from "@shared/generation";

interface TriggerMatch {
  kind: "command" | "mention";
  start: number;
  query: string;
}

interface CompletionItem {
  label: string;
  detail?: string;
  insert: string;
  path?: string;
}

interface CompletionState {
  kind: "command" | "mention";
  start: number;
  query: string;
  items: CompletionItem[];
  selected: number;
}

function detectTrigger(value: string, caret: number, spans: { start: number; end: number }[]): TriggerMatch | null {
  const before = value.slice(0, caret);
  const command = /(^|\n)(\/)(\S*)$/.exec(before);
  if (command) return { kind: "command", start: command.index + command[1].length, query: command[3] };
  const mention = /(^|\s)(@)(\S*)$/.exec(before);
  if (!mention) return null;
  const start = mention.index + mention[1].length;
  if (spans.some((span) => start < span.end && caret > span.start)) return null;
  return { kind: "mention", start, query: mention[3] };
}

function mentionSpans(value: string, list: { rel: string }[]): { start: number; end: number }[] {
  const spans: { start: number; end: number }[] = [];
  for (const mention of list) {
    const token = `@${mention.rel}`;
    const idx = value.indexOf(token);
    if (idx >= 0) spans.push({ start: idx, end: idx + token.length });
  }
  return spans;
}

function filterCompletionItems(items: CompletionItem[], query: string): CompletionItem[] {
  if (!query) return items;
  const q = query.toLowerCase();
  return items.filter((item) => item.label.toLowerCase().includes(q));
}

export function buildPromptFiles(text: string, list: { rel: string; path: string }[]): PromptFile[] {
  const files: PromptFile[] = [];
  for (const mention of list) {
    const token = `@${mention.rel}`;
    const idx = text.indexOf(token);
    if (idx >= 0) files.push({ path: mention.path, mention: { start: idx, end: idx + token.length, text: token } });
  }
  return files;
}

interface CompletionOptions {
  workspace: WorkspaceIdentity | null;
  input: string;
  mentions: { rel: string; path: string }[];
  inputRef: RefObject<HTMLTextAreaElement | null>;
  setInput: Dispatch<SetStateAction<string>>;
  setMentions: Dispatch<SetStateAction<{ rel: string; path: string }[]>>;
  runCommand: (name: string, args?: string, workspace?: WorkspaceIdentity) => Promise<void>;
  setNotice: Dispatch<SetStateAction<string>>;
  supportsCommands: boolean;
  supportsAttachments: boolean;
}

function useCompletionCandidates(workspace: WorkspaceIdentity | null,
  setMentions: CompletionOptions["setMentions"]) {
  const [completion, setCompletion] = useState<CompletionState | null>(null);
  const candidatesRef = useRef<{ kind: "command" | "mention"; items: CompletionItem[] } | null>(null);
  const fetchSeqRef = useRef(0);
  const mentionTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const workspaceRef = useRef<WorkspaceIdentity | null>(workspace);
  workspaceRef.current = workspace;
  useEffect(
    () => () => {
      if (mentionTimerRef.current) clearTimeout(mentionTimerRef.current);
    },
    []
  );

  useEffect(() => {
    fetchSeqRef.current += 1;
    candidatesRef.current = null;
    setCompletion(null);
    setMentions([]);
    if (mentionTimerRef.current) clearTimeout(mentionTimerRef.current);
  }, [workspace?.id, workspace?.generation]);

  const openCandidates = async (kind: "command" | "mention", query: string, start: number): Promise<void> => {
    const seq = ++fetchSeqRef.current;
    const workspace = workspaceRef.current;
    if (!workspace) return;
    if (kind === "command") {
      try {
        const raw = await window.vela.commands(workspace);
        if (fetchSeqRef.current !== seq || !sameWorkspace(workspace, workspaceRef.current)) return;
        const items: CompletionItem[] = raw.map((c) => ({
          label: c.name,
          detail: c.description,
          insert: c.name
        }));
        candidatesRef.current = { kind, items };
        setCompletion({ kind, start, query, items: filterCompletionItems(items, query), selected: 0 });
      } catch {
        if (!sameWorkspace(workspace, workspaceRef.current)) return;
        candidatesRef.current = null;
        setCompletion(null);
      }
      return;
    }
    if (mentionTimerRef.current) clearTimeout(mentionTimerRef.current);
    mentionTimerRef.current = setTimeout(() => {
      void (async () => {
        try {
          const raw = await window.vela.references(workspace, query);
          if (fetchSeqRef.current !== seq || !sameWorkspace(workspace, workspaceRef.current)) return;
          const items: CompletionItem[] = raw.map((r) => ({
            label: r.rel,
            detail: r.description ?? r.name,
            insert: r.rel,
            path: r.path
          }));
          setCompletion({ kind, start, query, items: filterCompletionItems(items, query), selected: 0 });
        } catch {
          if (!sameWorkspace(workspace, workspaceRef.current)) return;
          setCompletion(null);
        }
      })();
    }, query ? 200 : 0);
  };

  return { completion, setCompletion, candidatesRef, openCandidates };
}

export function useComposerCompletions({ workspace, input, mentions, inputRef, setInput, setMentions,
  runCommand, setNotice, supportsCommands, supportsAttachments }: CompletionOptions) {
  const { completion, setCompletion, candidatesRef, openCandidates } = useCompletionCandidates(workspace, setMentions);

  const applyCompletion = (index?: number): void => {
    const c = completion;
    if (!c || c.items.length === 0) return;
    const item = c.items[Math.min(index ?? c.selected, c.items.length - 1)];
    const spanEnd = c.start + 1 + c.query.length;
    const rest = input.slice(spanEnd);
    if (c.kind === "command") {
      setInput("");
      setCompletion(null);
      void runCommand(item.insert, rest.trim(), workspace ?? undefined).catch((err) =>
        setNotice(err instanceof Error ? err.message : String(err))
      );
      return;
    }
    if (!item.path) return;
    const filePath = item.path;
    const prefix = input.slice(0, c.start);
    const next = `${prefix}@${item.insert}${rest}`;
    setInput(next);
    setMentions((prev) => [
      ...prev.filter((mnt) => mnt.rel !== item.insert),
      { rel: item.insert, path: filePath }
    ]);
    setCompletion(null);
    const el = inputRef.current;
    if (el) {
      el.focus();
      const pos = prefix.length + 1 + item.insert.length;
      el.setSelectionRange(pos, pos);
    }
  };

  const updateCompletion = (value: string, caret: number): void => {
    const detected = detectTrigger(value, caret, mentionSpans(value, mentions));
    const trigger = detected && ((detected.kind === "command" && supportsCommands) || (detected.kind === "mention" && supportsAttachments))
      ? detected
      : null;
    if (!trigger) {
      setCompletion(null);
      return;
    }
    if (candidatesRef.current?.kind === trigger.kind) {
      setCompletion({
        kind: trigger.kind,
        start: trigger.start,
        query: trigger.query,
        items: filterCompletionItems(candidatesRef.current.items, trigger.query),
        selected: 0
      });
    } else {
      void openCandidates(trigger.kind, trigger.query, trigger.start);
    }
  };
  return { completion, setCompletion, applyCompletion, updateCompletion };
}
