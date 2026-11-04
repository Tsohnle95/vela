import fsp from "node:fs/promises";
import path from "node:path";

const SKIP_DIRS = new Set([
  "node_modules",
  ".git",
  "out",
  "dist",
  "build",
  ".next",
  ".turbo",
  "coverage",
  ".venv",
  "venv",
  "__pycache__",
  ".opencode",
  ".claude",
  ".cursor",
  ".aider",
  ".windsurf",
  ".codeium",
  ".roo",
  ".gemini",
  ".kilocode",
  ".continue"
]);

async function findFileByBasename(root: string, basename: string, maxDepth = 7): Promise<string | null> {
  const matches: string[] = [];
  const walk = async (dir: string, depth: number): Promise<void> => {
    if (depth > maxDepth || matches.length >= 5) return;
    let entries;
    try {
      entries = await fsp.readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (matches.length >= 5) return;
      if (entry.isDirectory()) {
        if (SKIP_DIRS.has(entry.name) || entry.name.startsWith(".")) continue;
        await walk(path.join(dir, entry.name), depth + 1);
      } else if (entry.isFile() && entry.name === basename) {
        matches.push(path.join(dir, entry.name));
      }
    }
  };
  await walk(root, 0);
  matches.sort((a, b) => a.length - b.length);
  return matches[0] ?? null;
}

function stripFragment(target: string): string {
  return target.replace(/[?#].*$/, "");
}

function sourceTarget(title: string): string | null {
  const target = title.trim();
  return target ? target.replace(/:(\d+)\s*$/, "") : null;
}

function toAbsolute(root: string, target: string): string | null {
  if (target.startsWith("file://")) {
    let file = target.slice("file://".length);
    try {
      file = decodeURIComponent(file);
    } catch {
      return null;
    }
    return path.isAbsolute(file) ? file : path.join(root, file);
  }
  if (/^https?:\/\//i.test(target)) {
    try {
      const url = new URL(target);
      const decoded = decodeURIComponent(url.pathname);
      return path.isAbsolute(decoded) ? decoded : path.join(root, decoded.replace(/^\/+/, ""));
    } catch {
      return null;
    }
  }
  return path.isAbsolute(target) ? target : path.join(root, target.replace(/^\/+/, ""));
}

function isInsideRoot(root: string, target: string): boolean {
  const rel = path.relative(root, target);
  return Boolean(rel) && rel !== ".." && !rel.startsWith(`..${path.sep}`) && !path.isAbsolute(rel);
}

function isUnderSkippedDir(root: string, target: string): boolean {
  return path.relative(root, target).split(path.sep).some((segment) => SKIP_DIRS.has(segment));
}

async function canonicalFile(root: string, target: string): Promise<string | null> {
  try {
    const file = await fsp.realpath(target);
    if (!isInsideRoot(root, file) || isUnderSkippedDir(root, file)) return null;
    return (await fsp.stat(file)).isFile() ? file : null;
  } catch {
    return null;
  }
}

async function resolveInRoot(root: string, file: string, title: string): Promise<string | null> {
  const candidates: string[] = [];
  if (file) candidates.push(stripFragment(file));
  const titleTarget = sourceTarget(title);
  if (titleTarget) candidates.push(stripFragment(titleTarget));
  for (const candidate of candidates) {
    const absolute = toAbsolute(root, candidate);
    const resolved = absolute ? await canonicalFile(root, absolute) : null;
    if (resolved) return resolved;
  }
  for (const candidate of candidates) {
    if (!/^https?:\/\//i.test(candidate)) continue;
    const mapped = stripFragment(candidate.replace(/^[a-z]+:\/\/[^/]+/i, ""));
    if (!mapped) continue;
    const resolved = await canonicalFile(root, path.join(root, mapped.replace(/^\/+/, "")));
    if (resolved) return resolved;
  }
  const basenames = new Set<string>();
  for (const source of [file, titleTarget].filter((value): value is string => Boolean(value))) {
    const basename = stripFragment(source).split(/[\\/]/).pop();
    if (basename) basenames.add(basename);
  }
  for (const basename of basenames) {
    const found = await findFileByBasename(root, basename);
    if (!found) continue;
    const resolved = await canonicalFile(root, found);
    if (resolved) return resolved;
  }
  return null;
}

export async function resolveAppSource(appRoot: string, file: string, title: string): Promise<string | null> {
  try {
    const root = await fsp.realpath(appRoot);
    if (!(await fsp.stat(root)).isDirectory()) return null;
    return resolveInRoot(root, file, title);
  } catch {
    return null;
  }
}

// Installed live launchers keep Electron's appPath at the wrapper while loading
// out/main from the checkout. The executing main module owns the source root.
export function resolveRunningAppSource(mainDirectory: string, file: string, title: string): Promise<string | null> {
  return resolveAppSource(path.resolve(mainDirectory, "../.."), file, title);
}
