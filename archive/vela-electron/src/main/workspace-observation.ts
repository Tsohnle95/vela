import { execFile } from "node:child_process";
import { promises as fsp, watch, type FSWatcher } from "node:fs";
import path from "node:path";
import type { FileBaseline, FileWriteIdentity, WorkspaceIdentity } from "@shared/types";
import { HIDDEN_RECOVERY_DIRS } from "@shared/types";
import { knownBaseline, observedBaseline, preserveBaseline, unknownBaseline } from "./file-baseline";

export const SKIP_DIRS: ReadonlySet<string> = new Set([
  ".git", "node_modules", ".next", ".venv", "__pycache__", ".cache", ".turbo", ".nx",
  ".svn", ".hg", ".idea", ".vscode", ...HIDDEN_RECOVERY_DIRS, "dist", "out", "build", "target", "coverage", ".pytest_cache",
  ".opencode", ".claude", ".cursor", ".aider", ".windsurf", ".codeium", ".roo", ".gemini", ".kilocode", ".continue"
]);

const MAX_WATCHED_FILE_BYTES = 2 * 1024 * 1024;

export interface WatchContext {
  root: string;
  sessionID: string;
  workspace: WorkspaceIdentity;
  snapshots: Map<string, FileBaseline>;
  lastKnown: Map<string, string>;
  hasGit: boolean | null;
  gitRoot?: string | null;
  timers: Map<string, ReturnType<typeof setTimeout>>;
  identities?: Map<string, string>;
  deletedPaths?: Set<string>;
  recentMoves?: Map<string, number>;
  importing?: number;
  suppressedUntil?: Map<string, number>;
  /** Absolute paths Vela is writing; their filesystem events are not external changes. */
  mutating?: Set<string>;
}

export interface WatchSession {
  directory: string;
  watchContext: WatchContext;
  watcher: FSWatcher | null;
}

export class WorkspaceObservation<Session extends WatchSession> {
  constructor(
    private readonly currentWatch: (context: WatchContext) => boolean,
    private readonly currentContext: (context: Session) => boolean,
    private readonly emit: (message: unknown) => void
  ) {}

  private abs(rel: string, root: string): string {
    return path.isAbsolute(rel) ? rel : path.join(root, rel);
  }

  private relKey(abs: string, root: string): string {
    const rel = path.isAbsolute(abs) ? path.relative(root, abs) : abs;
    return rel.split(path.sep).join("/");
  }

  shouldSkip(abs: string, root: string): boolean {
    const rel = path.relative(root, abs);
    return rel.split(path.sep).some((segment) => SKIP_DIRS.has(segment));
  }

  private isGitMetadata(abs: string, root: string): boolean {
    const rel = path.relative(root, abs);
    return rel === ".git" || rel.startsWith(`.git${path.sep}`);
  }

  createContext(root: string, sessionID: string, workspace: WorkspaceIdentity): WatchContext {
    return {
      root, sessionID, workspace,
      snapshots: new Map(),
      lastKnown: new Map(),
      hasGit: null,
      gitRoot: null,
      timers: new Map()
    };
  }

  recordSnapshot(context: WatchContext, abs: string, content: string | null): void {
    if (this.currentWatch(context)) context.snapshots.set(abs, knownBaseline(content ?? "", content !== null));
  }

  recordReadIdentity(context: WatchContext, abs: string, stat: Awaited<ReturnType<typeof fsp.stat>> | null): void {
    const identity = stat?.isFile() ? this.fileIdentity(stat) : null;
    if (identity) (context.identities ??= new Map()).set(abs, identity);
  }

  beginWrite(context: WatchContext, abs: string): void {
    (context.mutating ??= new Set()).add(abs);
  }

  endWrite(context: WatchContext, abs: string): void {
    context.mutating?.delete(abs);
  }

  beginImport(context: Session): boolean {
    const restartWatcher = context.watcher !== null;
    if (restartWatcher) this.stopWatcher(context);
    context.watchContext.importing = (context.watchContext.importing ?? 0) + 1;
    return restartWatcher;
  }

  recordImported(context: WatchContext, abs: string, content: string): void {
    context.snapshots.set(abs, knownBaseline(content));
    context.lastKnown.set(abs, content);
  }

  endImport(context: Session, destAbs: string, restartWatcher: boolean): void {
    const watchContext = context.watchContext;
    const prefix = `${destAbs}${path.sep}`;
    for (const [abs, timer] of watchContext.timers) {
      if (abs === destAbs || abs.startsWith(prefix)) {
        clearTimeout(timer);
        watchContext.timers.delete(abs);
      }
    }
    watchContext.importing = Math.max(0, (watchContext.importing ?? 1) - 1);
    (watchContext.suppressedUntil ??= new Map()).set(destAbs, Date.now() + 10_000);
    if (restartWatcher && this.currentWatch(watchContext)) this.startWatcher(context);
  }

  recordSave(context: WatchContext, abs: string, content: string, before: string, write: FileWriteIdentity): void {
    context.snapshots.set(abs, preserveBaseline(context.snapshots.get(abs), knownBaseline(before)));
    context.lastKnown.set(abs, content);
    this.emitFileUpdate(context, abs, content, undefined, write);
  }

  recordCreate(context: WatchContext, abs: string): void {
    context.snapshots.set(abs, preserveBaseline(context.snapshots.get(abs), knownBaseline("", false)));
    context.lastKnown.set(abs, "");
    this.emitFileUpdate(context, abs, "");
  }

  recordDelete(context: WatchContext, abs: string, baseline: FileBaseline): void {
    context.snapshots.set(abs, baseline);
    context.lastKnown.delete(abs);
    this.emitFileUpdate(context, abs, null, baseline);
  }

  recordDetach(context: WatchContext, abs: string): void {
    context.snapshots.delete(abs);
    context.lastKnown.delete(abs);
    this.emitFileUpdate(context, abs, null, unknownBaseline);
  }

  recordRename(context: WatchContext, abs: string, target: string, captured: { baseline: FileBaseline; content: string | null }): void {
    context.snapshots.set(abs, captured.baseline);
    context.lastKnown.delete(abs);
    context.snapshots.set(target, captured.baseline);
    this.emitFileUpdate(context, abs, null, captured.baseline);
    if (!this.currentWatch(context)) return;
    if (captured.content !== null) context.lastKnown.set(target, captured.content);
    else context.lastKnown.delete(target);
    this.emitFileUpdate(context, target, captured.content, captured.baseline);
  }

  recordMove(context: WatchContext, abs: string, target: string, captured: { baseline: FileBaseline; content: string | null }): void {
    this.recordDelete(context, abs, captured.baseline);
    if (!this.currentWatch(context)) return;
    if (captured.content !== null) {
      context.snapshots.set(target, captured.baseline);
      context.lastKnown.set(target, captured.content);
      this.emitFileUpdate(context, target, captured.content, captured.baseline);
    }
  }

  async gitShow(context: WatchContext, rel: string): Promise<string | null> {
    if (context.hasGit === false) return null;
    if (context.hasGit === null) {
      let cur: string | null = context.root;
      let gitRoot: string | null = null;
      while (cur) {
        try {
          await fsp.access(path.join(cur, ".git"));
          gitRoot = cur;
          break;
        } catch {
          const parent = path.dirname(cur);
          if (parent === cur) break;
          cur = parent;
        }
      }
      if (!this.currentWatch(context)) return null;
      if (gitRoot) {
        context.hasGit = true;
        (context as { gitRoot: string | null }).gitRoot = gitRoot;
      } else {
        context.hasGit = false;
        (context as { gitRoot: string | null }).gitRoot = null;
        return null;
      }
    }
    const gitRoot = (context as { gitRoot: string | null }).gitRoot ?? context.root;
    const abs = path.isAbsolute(rel) ? rel : path.join(context.root, rel);
    const relGit = path.relative(gitRoot, abs).split(path.sep).join("/");
    if (!relGit || relGit.startsWith("..")) return null;
    return new Promise((resolve) => {
      execFile(
        "git",
        ["show", `HEAD:${relGit}`],
        { cwd: gitRoot, maxBuffer: 16 * 1024 * 1024, timeout: 10_000 },
        (err, stdout) => {
          resolve(err ? null : stdout);
        }
      );
    });
  }

  startWatcher(context: Session): void {
    try {
      context.watcher = watch(context.directory, { recursive: true }, (event, filename) => {
        if (!this.currentContext(context)) return;
        if (!filename || typeof filename !== "string") return;
        const abs = this.abs(filename, context.directory);
        if (this.isGitMetadata(abs, context.directory)) {
          this.scheduleGitRefresh(context.watchContext);
          return;
        }
        if (this.shouldSkip(abs, context.directory)) return;
        this.scheduleWatch(context.watchContext, abs, event);
      });
      context.watcher.on("error", (err) => console.error("[vela] watcher error:", err));
      console.log("[vela] watching", context.directory);
    } catch (err) {
      console.error("[vela] fs.watch unavailable, live updates disabled:", err);
    }
  }

  stopWatcher(context: Session): void {
    for (const t of context.watchContext.timers.values()) clearTimeout(t);
    context.watchContext.timers.clear();
    context.watcher?.close();
    context.watcher = null;
  }

  private blocked(context: WatchContext, abs: string): boolean {
    if ((context.importing ?? 0) > 0) return true;
    if (context.mutating?.has(abs)) return true;
    const now = Date.now();
    const suppressedUntil = context.suppressedUntil ?? (context.suppressedUntil = new Map());
    for (const [prefix, until] of suppressedUntil) {
      if (until <= now) suppressedUntil.delete(prefix);
      else if (abs === prefix || abs.startsWith(`${prefix}${path.sep}`)) return true;
    }
    return false;
  }

  private scheduleWatch(context: WatchContext, abs: string, event: string): void {
    if (this.blocked(context, abs)) return;
    const existing = context.timers.get(abs);
    if (existing) clearTimeout(existing);
    context.timers.set(
      abs,
      setTimeout(() => {
        context.timers.delete(abs);
        if (this.currentWatch(context)) void this.onFsChanged(context, abs, event);
      }, 200)
    );
  }

  private scheduleGitRefresh(context: WatchContext): void {
    const key = path.join(context.root, ".git");
    const existing = context.timers.get(key);
    if (existing) clearTimeout(existing);
    context.timers.set(key, setTimeout(() => {
      context.timers.delete(key);
      if (this.currentWatch(context)) void this.refreshGitBaselines(context);
    }, 200));
  }

  async refreshGitBaselines(context: WatchContext): Promise<void> {
    for (const [abs] of context.snapshots) {
      if (!this.currentWatch(context)) return;
      const rel = this.relKey(abs, context.root);
      const git = await this.gitShow(context, rel);
      if (!this.currentWatch(context)) return;
      const baseline = observedBaseline(true, git);
      let content: string | null = null;
      try {
        content = await fsp.readFile(abs, "utf8");
      } catch {
      }
      context.snapshots.set(abs, baseline);
      if (content === null) context.lastKnown.delete(abs);
      else context.lastKnown.set(abs, content);
      this.emitFileUpdate(context, abs, content, baseline);
    }
  }

  async onFsChanged(context: WatchContext, abs: string, event: string): Promise<void> {
    if (!this.currentWatch(context)) return;
    if (this.blocked(context, abs)) return;
    if (this.isGitMetadata(abs, context.root)) {
      await this.refreshGitBaselines(context);
      return;
    }
    if (this.shouldSkip(abs, context.root)) return;
    let stat: Awaited<ReturnType<typeof fsp.stat>> | null = null;
    try {
      stat = await fsp.stat(abs);
    } catch {
      /* missing */
    }
    if (!this.currentWatch(context)) return;
    if (!stat || !stat.isFile()) {
      const movedUntil = context.recentMoves?.get(abs);
      if (movedUntil !== undefined) {
        if (movedUntil > Date.now()) return;
        context.recentMoves?.delete(abs);
      }
      if (event === "unlink" || context.lastKnown.has(abs) || context.snapshots.has(abs)) {
        let baseline = context.snapshots.get(abs);
        if (!baseline) {
          const lastKnown = context.lastKnown.get(abs);
          if (lastKnown !== undefined) {
            baseline = knownBaseline(lastKnown);
          } else {
            const git = await this.gitShow(context, this.relKey(abs, context.root));
            if (!this.currentWatch(context)) return;
            baseline = observedBaseline(context.hasGit === true, git);
          }
        }
        if (!this.currentWatch(context)) return;
        context.snapshots.set(abs, baseline);
        context.lastKnown.delete(abs);
        (context.deletedPaths ??= new Set()).add(abs);
        this.emitFileUpdate(context, abs, null, baseline);
      }
      return;
    }
    if (stat.size > MAX_WATCHED_FILE_BYTES) return;
    let content: string;
    try {
      content = await fsp.readFile(abs, "utf8");
    } catch {
      return;
    }
    if (!this.currentWatch(context)) return;
    const identity = this.fileIdentity(stat);
    const movedFrom = identity ? await this.missingIdentitySource(context, abs, identity) : null;
    if (!this.currentWatch(context)) return;
    if (movedFrom && identity) {
      const oldBaseline = await this.baselineForObservedPath(context, movedFrom);
      if (!this.currentWatch(context)) return;
      const targetBaseline = await this.baselineForObservedPath(context, abs);
      if (!this.currentWatch(context)) return;
      if (!context.deletedPaths?.has(movedFrom)) {
        context.snapshots.set(movedFrom, oldBaseline);
        context.lastKnown.delete(movedFrom);
        this.emitFileUpdate(context, movedFrom, null, oldBaseline);
      }
      context.snapshots.set(abs, targetBaseline);
      context.lastKnown.set(abs, content);
      context.identities?.delete(movedFrom);
      context.identities?.set(abs, identity);
      context.deletedPaths?.delete(movedFrom);
      context.deletedPaths?.delete(abs);
      (context.recentMoves ??= new Map()).set(movedFrom, Date.now() + 2_000);
      this.emitFileUpdate(context, abs, content, targetBaseline, undefined, this.relKey(movedFrom, context.root));
      return;
    }
    if (identity) (context.identities ??= new Map()).set(abs, identity);
    context.deletedPaths?.delete(abs);
    if (content === context.lastKnown.get(abs)) return;
    context.lastKnown.set(abs, content);
    if (!context.snapshots.has(abs)) {
      const git = await this.gitShow(context, this.relKey(abs, context.root));
      if (!this.currentWatch(context)) return;
      const baseline = observedBaseline(context.hasGit === true, git);
      context.snapshots.set(abs, baseline);
    }
    this.emitFileUpdate(context, abs, content);
  }

  emitFileUpdate(
    context: WatchContext,
    abs: string,
    content: string | null,
    baselineOverride?: FileBaseline,
    write?: FileWriteIdentity,
    movedFrom?: string
  ): void {
    const baseline =
      baselineOverride !== undefined
        ? baselineOverride
        : (context.snapshots.get(abs) ?? unknownBaseline);
    if (!this.currentWatch(context)) return;
    this.emit({
      kind: "file-update",
      file: {
        workspace: context.workspace,
        sessionID: context.sessionID,
        path: this.relKey(abs, context.root),
        ...(movedFrom ? { movedFrom } : {}),
        baseline,
        content,
        deleted: content === null,
        ...(write ? { write } : {})
      }
    });
  }

  private fileIdentity(stat: Awaited<ReturnType<typeof fsp.stat>>): string | null {
    if (!Number.isFinite(stat.dev) || !Number.isFinite(stat.ino) || stat.ino <= 0) return null;
    return `${stat.dev}:${stat.ino}`;
  }

  private async missingIdentitySource(context: WatchContext, abs: string, identity: string): Promise<string | null> {
    const candidates = [...(context.identities ?? [])]
      .filter(([candidate, value]) => candidate !== abs && value === identity)
      .map(([candidate]) => candidate);
    const missing: string[] = [];
    for (const candidate of candidates) {
      const stat = await fsp.stat(candidate).catch(() => null);
      if (!this.currentWatch(context)) return null;
      if (!stat) missing.push(candidate);
    }
    return missing.length === 1 ? missing[0] : null;
  }

  private async baselineForObservedPath(context: WatchContext, abs: string): Promise<FileBaseline> {
    const established = context.snapshots.get(abs);
    if (established) return established;
    const git = await this.gitShow(context, this.relKey(abs, context.root));
    return observedBaseline(context.hasGit === true, git);
  }

  async captureDirectMutation(context: WatchContext, abs: string): Promise<{
    baseline: FileBaseline;
    content: string | null;
  }> {
    const established = context.snapshots.get(abs);
    try {
      const content = await fsp.readFile(abs, "utf8");
      return { baseline: established ?? knownBaseline(content), content };
    } catch {
      return { baseline: established ?? unknownBaseline, content: null };
    }
  }

}
