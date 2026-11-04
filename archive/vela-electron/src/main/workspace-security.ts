import { promises as fsp } from "node:fs";
import path from "node:path";
import type { WorkspaceIdentity } from "@shared/types";

export const MAX_WORKSPACE_PATH_LENGTH = 4096;
export const MAX_WORKSPACE_FILE_BYTES = 8 * 1024 * 1024;
export const MAX_TERMINAL_INPUT_BYTES = 1024 * 1024;
export const MAX_TERMINAL_COLUMNS = 1000;
export const MAX_TERMINAL_ROWS = 500;

export function workspaceId(value: unknown): string {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid workspace identity");
  const keys = Object.keys(value);
  const { id, generation } = value as { id?: unknown; generation?: unknown };
  if (
    keys.length !== 2 ||
    !keys.includes("id") ||
    !keys.includes("generation") ||
    typeof id !== "string" ||
    !/^[a-f0-9-]{36}$/.test(id) ||
    !Number.isSafeInteger(generation) ||
    Number(generation) < 1
  ) {
    throw new Error("invalid workspace identity");
  }
  return id;
}

export function assertWorkspace(expected: unknown, active: WorkspaceIdentity | null): WorkspaceIdentity {
  const id = workspaceId(expected);
  const generation = (expected as WorkspaceIdentity).generation;
  if (!active || id !== active.id || generation !== active.generation) throw new Error("stale workspace");
  return active;
}

export function relativePath(value: unknown, allowRoot = false): string {
  if (typeof value !== "string" || value.length > MAX_WORKSPACE_PATH_LENGTH || value.includes("\0")) {
    throw new Error("invalid workspace path");
  }
  if (value === "" && allowRoot) return value;
  if (!value || value.includes("\\") || path.posix.isAbsolute(value) || /^[a-zA-Z]:/.test(value)) {
    throw new Error("invalid workspace path");
  }
  const parts = value.split("/");
  if (parts.some((part) => !part || part === "." || part === "..")) throw new Error("invalid workspace path");
  return value;
}

export function fileName(value: unknown): string {
  if (typeof value !== "string" || !value || value.length > 255 || value === "." || value === ".." || /[\\/\0]/.test(value)) {
    throw new Error("invalid name");
  }
  return value;
}

export function fileContent(value: unknown): string {
  if (typeof value !== "string" || Buffer.byteLength(value) > MAX_WORKSPACE_FILE_BYTES) {
    throw new Error("invalid or oversized file content");
  }
  return value;
}

export function terminalId(value: unknown): string {
  if (typeof value !== "string" || !/^term-[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(value)) {
    throw new Error("invalid terminal id");
  }
  return value;
}

export function terminalInput(value: unknown): string {
  if (typeof value !== "string" || Buffer.byteLength(value) > MAX_TERMINAL_INPUT_BYTES) {
    throw new Error("invalid or oversized terminal input");
  }
  return value;
}

export function terminalDimensions(cols: unknown, rows: unknown): { cols: number; rows: number } {
  if (!Number.isInteger(cols) || !Number.isInteger(rows) || (cols as number) < 1 || (rows as number) < 1 ||
      (cols as number) > MAX_TERMINAL_COLUMNS || (rows as number) > MAX_TERMINAL_ROWS) {
    throw new Error("invalid terminal dimensions");
  }
  return { cols: cols as number, rows: rows as number };
}

export function absoluteFilePath(value: unknown): string {
  if (
    typeof value !== "string" ||
    value.length > MAX_WORKSPACE_PATH_LENGTH ||
    value.includes("\0") ||
    !path.isAbsolute(value)
  ) {
    throw new Error("invalid absolute file path");
  }
  return value;
}

export function absoluteFilePaths(value: unknown): string[] {
  if (!Array.isArray(value) || value.length === 0 || value.length > 2000) {
    throw new Error("invalid absolute file path list");
  }
  return value.map(absoluteFilePath);
}

export async function canonicalWorkspaceRoot(directory: string): Promise<string> {
  const root = await fsp.realpath(directory);
  const stat = await fsp.stat(root);
  if (!stat.isDirectory()) throw new Error("workspace is not a directory");
  return root;
}

export async function confinedPath(root: string, rel: string, allowRoot = false): Promise<string> {
  const clean = relativePath(rel, allowRoot);
  let current = root;
  for (const part of clean ? clean.split("/") : []) {
    current = path.join(current, part);
    try {
      const stat = await fsp.lstat(current);
      if (stat.isSymbolicLink()) throw new Error("workspace symlinks are not allowed");
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") break;
      throw error;
    }
  }
  return current;
}

export async function confinedAbsolutePath(root: string, value: unknown): Promise<string> {
  if (typeof value !== "string" || value.length > MAX_WORKSPACE_PATH_LENGTH || !path.isAbsolute(value) || value.includes("\0")) {
    throw new Error("invalid source path");
  }
  const rel = path.relative(root, value).split(path.sep).join("/");
  if (!rel || rel.startsWith("../") || rel === "..") throw new Error("source path is outside the application");
  return confinedPath(root, rel);
}
