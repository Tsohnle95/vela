import type { SessionUsage, TodoItem } from "./types";

export function normalizeTodos(value: unknown): TodoItem[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((raw, index) => {
    if (!raw || typeof raw !== "object") return [];
    const item = raw as Record<string, unknown>;
    const content = typeof item.content === "string" ? item.content : "";
    const rawStatus = typeof item.status === "string" ? item.status : "pending";
    const status = ["pending", "in_progress", "completed", "cancelled"].includes(rawStatus)
      ? rawStatus as TodoItem["status"]
      : "pending";
    if (!content) return [];
    return [{
      id: typeof item.id === "string" && item.id ? item.id : `todo-${index}`,
      content,
      status,
      ...(typeof item.priority === "string" ? { priority: item.priority } : {})
    }];
  });
}

export function sessionUsageValues(
  cost: unknown,
  tokens: Record<string, unknown>,
  cache: Record<string, unknown> | undefined
): SessionUsage {
  const num = (n: unknown): number => (typeof n === "number" && Number.isFinite(n) ? n : 0);
  return {
    cost: num(cost),
    tokens: {
      input: num(tokens.input),
      output: num(tokens.output),
      reasoning: num(tokens.reasoning),
      cache: {
        read: num(cache?.read),
        write: num(cache?.write)
      }
    }
  };
}
