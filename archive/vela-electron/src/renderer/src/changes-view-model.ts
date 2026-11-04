import { diffLines } from "diff";
import type { AgentFileState } from "@shared/types";

// Presentation model for the Changes review view: turns the workspace's
// observed file changes into an ordered file list, per-file +/- statistics and
// a collapsible directory tree. It owns no state and performs no IO.

export interface ChangeFile {
  path: string;
  name: string;
  directory: string;
  state: AgentFileState;
}

export interface ChangeStats {
  additions: number;
  deletions: number;
}

export interface ChangeTreeRow {
  kind: "directory" | "file";
  key: string;
  depth: number;
  label: string;
  /** File path for file rows, directory prefix for directory rows. */
  path: string;
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^\/+/, "");
}

export function changeFilePathParts(path: string): string[] {
  return normalizePath(path).split("/").filter(Boolean);
}

export function changeList(agentFiles: Map<string, AgentFileState>): ChangeFile[] {
  const files: ChangeFile[] = [];
  for (const [path, state] of agentFiles) {
    const parts = changeFilePathParts(path);
    files.push({
      path,
      name: parts.at(-1) ?? path,
      directory: parts.slice(0, -1).join("/"),
      state
    });
  }
  return files.sort((left, right) => left.path.localeCompare(right.path));
}

/** True when a change path is itself hidden or lives under a hidden workspace path. */
export function isHiddenChangePath(path: string, hidden: ReadonlySet<string>): boolean {
  if (hidden.size === 0) return false;
  let prefix = normalizePath(path);
  while (prefix) {
    if (hidden.has(prefix)) return true;
    const slash = prefix.lastIndexOf("/");
    if (slash < 0) break;
    prefix = prefix.slice(0, slash);
  }
  return false;
}

/** Line counts for a file's observed change; null when no pre-change baseline exists. */
export function changeDiffStats(state: AgentFileState): ChangeStats | null {
  if (state.baseline.kind !== "known") return null;
  const before = state.baseline.content;
  const after = state.deleted ? "" : state.content ?? "";
  let additions = 0;
  let deletions = 0;
  for (const part of diffLines(before, after)) {
    if (part.added) additions += part.count ?? 0;
    else if (part.removed) deletions += part.count ?? 0;
  }
  return { additions, deletions };
}

interface DirectoryNode {
  name: string;
  path: string;
  dirs: Map<string, DirectoryNode>;
  files: ChangeFile[];
}

function createDirectory(name: string, path: string): DirectoryNode {
  return { name, path, dirs: new Map(), files: [] };
}

function insertFile(root: DirectoryNode, file: ChangeFile): void {
  const directories = changeFilePathParts(file.path).slice(0, -1);
  let node = root;
  let prefix = "";
  for (const segment of directories) {
    prefix = prefix ? `${prefix}/${segment}` : segment;
    let child = node.dirs.get(segment);
    if (!child) {
      child = createDirectory(segment, prefix);
      node.dirs.set(segment, child);
    }
    node = child;
  }
  node.files.push(file);
}

// Collapse chains of single-child folders into one "a/b/c" row, matching the
// compact directory labels used by the reference changes view.
function collapseDirectory(node: DirectoryNode): { node: DirectoryNode; label: string } {
  let current = node;
  let label = node.name;
  while (current.files.length === 0 && current.dirs.size === 1) {
    const [child] = current.dirs.values();
    current = child;
    label = `${label}/${child.name}`;
  }
  return { node: current, label };
}

export function buildChangeTreeRows(files: ChangeFile[], collapsed: ReadonlySet<string>): ChangeTreeRow[] {
  const root = createDirectory("", "");
  for (const file of files) insertFile(root, file);
  const rows: ChangeTreeRow[] = [];
  const walk = (directory: DirectoryNode, depth: number): void => {
    const directories = [...directory.dirs.values()].sort((left, right) => left.name.localeCompare(right.name));
    for (const child of directories) {
      const { node, label } = collapseDirectory(child);
      rows.push({ kind: "directory", key: `dir:${node.path}`, depth, label, path: node.path });
      if (!collapsed.has(node.path)) walk(node, depth + 1);
    }
    const entries = [...directory.files].sort((left, right) => left.name.localeCompare(right.name));
    for (const file of entries) {
      rows.push({ kind: "file", key: `file:${file.path}`, depth, label: file.name, path: file.path });
    }
  };
  walk(root, 0);
  return rows;
}
