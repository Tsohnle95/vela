import type { DragEvent } from "react";

export function isExternalFileDrag(e: DragEvent): boolean {
  const transfer = e.dataTransfer;
  if (!transfer) return false;
  if (Array.from((transfer.types ?? []) as ArrayLike<string>).some((type) => {
    const normalized = type.toLowerCase();
    return normalized === "files" || normalized === "text/uri-list" || normalized === "public.file-url";
  })) return true;
  if (Array.from(transfer.items ?? []).some((item) => item.kind === "file")) return true;
  return Boolean(transfer.files && transfer.files.length > 0);
}

export function droppedFilePaths(e: DragEvent): string[] {
  const files = Array.from(e.dataTransfer?.files ?? []);
  const paths: string[] = [];
  for (const file of files) {
    const viaBridge = window.vela?.getPathForFile?.(file);
    const path = viaBridge || (file as File & { path?: string }).path;
    if (path) paths.push(path);
  }
  if (paths.length > 0) return paths;
  const uriList = typeof e.dataTransfer?.getData === "function" ? e.dataTransfer.getData("text/uri-list") : "";
  return uriList
    .split(/\r?\n/)
    .filter((value) => value && !value.startsWith("#"))
    .map((value) => {
      try {
        const url = new URL(value);
        return url.protocol === "file:" ? decodeURIComponent(url.pathname) : "";
      } catch {
        return "";
      }
    })
    .filter((value) => value.length > 0);
}
