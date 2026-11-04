import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { canonicalWorkspaceRoot } from "./workspace-security";

export interface WorkspaceLocation {
  directory: string;
  device?: number;
  inode?: number;
  movedTo?: string;
}

export interface WorkspaceProject {
  id?: string;
  canonical?: string;
}

export function within(root: string, directory: string): boolean {
  const rel = path.relative(root, directory);
  return rel === "" || (!rel.startsWith(`..${path.sep}`) && rel !== ".." && !path.isAbsolute(rel));
}

export async function existingWorkspace(directory: string): Promise<string | null> {
  try {
    if (!(await stat(directory)).isDirectory()) return null;
    return await canonicalWorkspaceRoot(directory);
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    if (code === "ENOENT" || code === "ENOTDIR") return null;
    throw error;
  }
}

export async function workspacePath(directory: string): Promise<string> {
  let parent = path.resolve(directory);
  while (true) {
    const canonical = await existingWorkspace(parent);
    if (canonical) return path.join(canonical, path.relative(parent, path.resolve(directory)));
    const next = path.dirname(parent);
    if (next === parent) return path.resolve(directory);
    parent = next;
  }
}

export async function workspaceLocation(directory: string): Promise<WorkspaceLocation> {
  const info = await stat(directory);
  return { directory, device: info.dev, inode: info.ino };
}

async function nearbyDirectories(directory: string): Promise<string[]> {
  let parent = path.dirname(directory);
  const candidates = new Set<string>();
  // Only inspect nearby ancestors, never recursively crawl the home or disk.
  for (let level = 0; level < 4 && path.dirname(parent) !== parent; level += 1, parent = path.dirname(parent)) {
    if (!(await existingWorkspace(parent))) continue;
    const entries = await readdir(parent, { withFileTypes: true });
    if (entries.length > 500) continue;
    for (const entry of entries) if (entry.isDirectory()) candidates.add(path.join(parent, entry.name));
  }
  return [...candidates];
}

export async function locateWorkspace(
  directory: string,
  locations: WorkspaceLocation[],
  projects: WorkspaceProject[],
  projectID?: string
): Promise<{ directory: string; previousRoot: string; currentRoot: string } | null> {
  const direct = await existingWorkspace(directory);
  if (direct) return { directory: direct, previousRoot: directory, currentRoot: direct };
  const aliases = locations.filter((entry) => entry.movedTo && within(entry.directory, directory))
    .sort((a, b) => b.directory.length - a.directory.length);
  for (const alias of aliases) {
    let target = alias.movedTo!;
    const visited = new Set([alias.directory]);
    while (!visited.has(target)) {
      visited.add(target);
      const next = locations.find((entry) => entry.directory === target && entry.movedTo);
      if (!next) break;
      target = next.movedTo!;
    }
    const resolved = await existingWorkspace(path.join(target, path.relative(alias.directory, directory)));
    if (resolved) return { directory: resolved, previousRoot: alias.directory, currentRoot: target };
  }

  const recorded = locations.find((entry) => entry.directory === directory);
  const project = projects.filter((entry) => entry.canonical && (!projectID || entry.id === projectID))
    .filter((entry) => within(entry.canonical!, directory) && (projectID || entry.canonical === directory))
    .sort((a, b) => b.canonical!.length - a.canonical!.length)[0];
  let oldRoot = project?.canonical ?? directory;
  let expectedProjectID = project?.id ?? projectID;
  const known = projectID && projectID !== "global" ? projects.find((entry) => entry.id === projectID) : undefined;
  if (known?.canonical && !within(known.canonical, directory)) {
    // The catalog may already point to a moved project. Recover the old root
    // from the same project ID, preserving nested workspace paths.
    let ancestor = directory;
    while (path.dirname(ancestor) !== ancestor && path.basename(ancestor) !== path.basename(known.canonical)) {
      ancestor = path.dirname(ancestor);
    }
    if (path.basename(ancestor) === path.basename(known.canonical)) {
      const resolved = await existingWorkspace(path.join(known.canonical, path.relative(ancestor, directory)));
      if (resolved) return { directory: resolved, previousRoot: ancestor, currentRoot: known.canonical };
      oldRoot = ancestor;
    }
  }
  if (expectedProjectID === "global") expectedProjectID = undefined;
  if (!expectedProjectID && !recorded?.inode) return null;
  const candidates = await nearbyDirectories(oldRoot);
  const matches: string[] = [];
  for (const candidate of candidates) {
    let matchesIdentity = false;
    if (recorded?.inode && oldRoot === directory) {
      const info = await stat(candidate);
      matchesIdentity = info.dev === recorded.device && info.ino === recorded.inode;
    }
    if (!(recorded?.inode && oldRoot === directory) && expectedProjectID && path.basename(candidate) === path.basename(oldRoot)) {
      const identity = await readFile(path.join(candidate, ".git", "opencode"), "utf8").catch(() => null);
      matchesIdentity = identity?.trim() === expectedProjectID;
    }
    if (matchesIdentity) matches.push(candidate);
  }
  if (matches.length !== 1) return null;
  const resolved = await existingWorkspace(path.join(matches[0]!, path.relative(oldRoot, directory)));
  return resolved ? { directory: resolved, previousRoot: oldRoot, currentRoot: matches[0]! } : null;
}
