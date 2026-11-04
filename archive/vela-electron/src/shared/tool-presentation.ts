import type { ToolContentView } from "./types";

export function collectFilePaths(obj: unknown): string[] {
  const out: string[] = [];
  const walk = (o: unknown): void => {
    if (!o || typeof o !== "object") return;
    if (Array.isArray(o)) {
      o.forEach(walk);
      return;
    }
    for (const [k, v] of Object.entries(o)) {
      if ((k === "filePath" || k === "file_path" || k === "path") && typeof v === "string" && !v.startsWith("http")) {
        out.push(v);
      } else if (typeof v === "object") {
        walk(v);
      }
    }
  };
  walk(obj);
  return [...new Set(out)];
}

export function toolDetailText(input: unknown): string {
  if (!input || typeof input !== "object") return "";
  const o = input as Record<string, unknown>;
  if (typeof o.filePath === "string") return o.filePath;
  if (typeof o.file_path === "string") return o.file_path;
  if (typeof o.command === "string") return `$ ${o.command}`;
  return "";
}

export function toolContentViews(content: unknown): ToolContentView[] | undefined {
  if (!Array.isArray(content)) return undefined;
  const items = content.flatMap((raw): ToolContentView[] => {
    if (!raw || typeof raw !== "object") return [];
    const item = raw as Record<string, unknown>;
    if (item.type === "text") return [{ type: "text", text: String(item.text ?? "") }];
    if (item.type !== "file" || typeof item.uri !== "string" || typeof item.mime !== "string") return [];
    return [{
      type: "file",
      uri: item.uri,
      mime: item.mime,
      ...(typeof item.name === "string" ? { name: item.name } : {})
    }];
  });
  return items.length > 0 ? items : undefined;
}

export function record(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : undefined;
}

