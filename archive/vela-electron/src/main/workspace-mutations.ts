import { constants, promises as fsp } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { shell } from "electron";
import type { FileWriteIdentity, ImportResult, WorkspaceIdentity } from "@shared/types";
import { type RecoveryStorage, RECOVERY_DIR, LEGACY_RECOVERY_DIR, PREVIOUS_RECOVERY_DIR } from "./recovery-storage";
import type { WorkspaceOperationCoordinator } from "./operation-coordinator";
import { SKIP_DIRS, type WorkspaceObservation, type WatchContext, type WatchSession } from "./workspace-observation";
import { confinedPath, fileContent, fileName, relativePath } from "./workspace-security";
import { movePathToTrash } from "./trash";

export type MutationPhase =
  | "save:temporary-ready"
  | "save:source-held"
  | "save:held-validated"
  | "save:target-installed"
  | "rename:source-inspected"
  | "rename:source-held"
  | "rename:target-installed";

export type MutationPhaseHandler = (phase: MutationPhase, source: string, target: string) => void | Promise<void>;

export interface MutationSession extends WatchSession {
  workspace: WorkspaceIdentity;
}

function assertWriteIdentity(workspace: WorkspaceIdentity, write: FileWriteIdentity): void {
  if (
    !write ||
    typeof write.id !== "string" ||
    write.id.length < 1 ||
    write.id.length > 128 ||
    write.workspaceID !== workspace.id ||
    !Number.isSafeInteger(write.revision) ||
    write.revision < 0 ||
    typeof write.expectedContent !== "string" ||
    typeof write.overwrite !== "boolean"
  ) throw new Error("invalid file write identity");
}

export class WorkspaceMutations<Session extends MutationSession> {
  constructor(
    private readonly mutations: WorkspaceOperationCoordinator,
    private readonly recovery: RecoveryStorage,
    private readonly observation: WorkspaceObservation<Session>,
    private readonly mutationPhase: MutationPhaseHandler,
    private readonly contextFor: (workspace: WorkspaceIdentity) => Session,
    private readonly currentWatch: (context: WatchContext) => boolean,
    private readonly emitRecoveryFor: (context: Session) => Promise<void>
  ) {}

  private workspaceRoot(expected: WorkspaceIdentity): string {
    return this.contextFor(expected).directory;
  }

  private relKey(abs: string, root: string): string {
    const rel = path.isAbsolute(abs) ? path.relative(root, abs) : abs;
    return rel.split(path.sep).join("/");
  }

  private async copyExternal(source: string, target: string, isDir: boolean): Promise<string[]> {
    const copied: string[] = [];
    let files = 0;
    let bytes = 0;
    const MAX_FILES = 10000;
    const MAX_BYTES = 2 * 1024 * 1024 * 1024;
    const copyFile = async (from: string, to: string): Promise<void> => {
      const stat = await fsp.stat(from);
      bytes += stat.size;
      if (bytes > MAX_BYTES) throw new Error("import is too large");
      await fsp.copyFile(from, to);
    };
    const walk = async (from: string, to: string): Promise<void> => {
      const stat = await fsp.lstat(from);
      if (stat.isSymbolicLink()) return;
      if (stat.isDirectory()) {
        await fsp.mkdir(to, { recursive: true });
        const entries = await fsp.readdir(from, { withFileTypes: true });
        for (const entry of entries) {
          if (entry.isDirectory() && SKIP_DIRS.has(entry.name)) continue;
          await walk(path.join(from, entry.name), path.join(to, entry.name));
        }
        return;
      }
      if (stat.isFile()) {
        files += 1;
        if (files > MAX_FILES) throw new Error("too many files to import");
        await copyFile(from, to);
        copied.push(to);
      }
    };
    if (isDir) {
      await walk(source, target);
    } else {
      await copyFile(source, target);
      copied.push(target);
    }
    return copied;
  }

  async importExternal(
    workspace: WorkspaceIdentity,
    destDir: string,
    sources: string[]
  ): Promise<ImportResult[]> {
    return this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const context = this.contextFor(workspace);
      const watchContext = context.watchContext;
      if (!this.currentWatch(watchContext)) throw new Error("stale workspace");
      const cleanDest = relativePath(destDir, true);
      const destAbs = await confinedPath(root, cleanDest, true);
      const restartWatcher = this.observation.beginImport(context);
      try {
        await fsp.mkdir(destAbs, { recursive: true });
        await confinedPath(root, cleanDest, true);
        this.contextFor(workspace);
        const results: ImportResult[] = [];
        for (const source of sources) {
        const real = await fsp.realpath(source).catch(() => null);
        const name = path.basename(source);
        const rel = cleanDest ? `${cleanDest}/${name}` : name;
        if (!real) {
          results.push({ name, rel, imported: false, reason: "not found" });
          continue;
        }
        const relToRoot = path.relative(root, real);
        const underRoot = relToRoot !== "" && !relToRoot.startsWith("..") && !path.isAbsolute(relToRoot);
        const target = path.join(destAbs, name);
        if (underRoot) {
          results.push({ name, rel, imported: false, reason: "already in the workspace" });
          continue;
        }
        const occupied = await fsp.lstat(target).then(() => true).catch(() => false);
        if (occupied) {
          results.push({ name, rel, imported: false, reason: "already exists" });
          continue;
        }
        try {
          const stat = await fsp.stat(real);
          const copied = await this.copyExternal(real, target, stat.isDirectory());
          for (const abs of copied) {
            const content = await fsp.readFile(abs, "utf8").catch(() => null);
            if (content === null) continue;
            this.observation.recordImported(watchContext, abs, content);
          }
          results.push({ name, rel, imported: true });
        } catch (error) {
          await fsp.rm(target, { recursive: true, force: true }).catch(() => {});
          results.push({ name, rel, imported: false, reason: error instanceof Error ? error.message : String(error) });
        }
        }
        return results;
      } finally {
        this.observation.endImport(context, destAbs, restartWatcher);
      }
    });
  }

  async writeFile(
    workspace: WorkspaceIdentity,
    rel: string,
    content: string,
    write: FileWriteIdentity
  ): Promise<void> {
    const cleanContent = fileContent(content);
    assertWriteIdentity(workspace, write);
    const expectedContent = fileContent(write.expectedContent);
    await this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const context = this.contextFor(workspace);
      const watchContext = context.watchContext;
      if (!this.currentWatch(watchContext)) throw new Error("stale workspace");
      const cleanRel = relativePath(rel);
      const abs = await confinedPath(root, cleanRel);
      await fsp.mkdir(path.dirname(abs), { recursive: true });
      await confinedPath(root, cleanRel);
      this.contextFor(workspace);
      const recovery = await this.recovery.createRecoveryTransaction(root, "save", cleanRel);
      const temporary = path.join(recovery.directory, "temporary");
      const holding = path.join(recovery.directory, "original");
      const proposed = path.join(recovery.directory, "proposed");
      let sourceHeld = false;
      let targetInstalled = false;
      // Vela's own write must not be observed as an external change: block
      // filesystem events for this path until the save (or its failure) settles.
      this.observation.beginWrite(watchContext, abs);
      try {
        const temporaryHandle = await fsp.open(temporary, "wx", 0o600);
        try {
          await temporaryHandle.writeFile(cleanContent, "utf8");
          await temporaryHandle.sync();
        } finally {
          await temporaryHandle.close();
        }
        await fsp.copyFile(temporary, proposed, constants.COPYFILE_EXCL);
        const proposedHandle = await fsp.open(proposed, "r");
        try {
          await proposedHandle.sync();
        } finally {
          await proposedHandle.close();
        }
        const existing = await fsp.stat(abs).catch(() => null);
        if (existing) await fsp.chmod(temporary, existing.mode);
        recovery.transaction.phase = "temporary-ready";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
        await this.recovery.syncDirectory(recovery.directory);
        await this.mutationPhase("save:temporary-ready", abs, temporary);
        await confinedPath(root, cleanRel);
        this.contextFor(workspace);
        await fsp.rename(abs, holding);
        await this.recovery.syncDirectory(path.dirname(abs));
        await this.recovery.syncDirectory(recovery.directory);
        sourceHeld = true;
        recovery.transaction.phase = "source-held";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
        await this.mutationPhase("save:source-held", holding, abs);
        const heldContent = await fsp.readFile(holding, "utf8");
        if (!write.overwrite && heldContent !== expectedContent) {
          throw new Error("file changed on disk");
        }
        recovery.transaction.phase = "held-validated";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
        await this.mutationPhase("save:held-validated", holding, abs);
        await fsp.link(temporary, abs);
        await this.recovery.syncDirectory(path.dirname(abs));
        targetInstalled = true;
        recovery.transaction.phase = "target-installed";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
        await this.mutationPhase("save:target-installed", temporary, abs);
        if (await fsp.readFile(abs, "utf8") !== cleanContent) {
          throw new Error("file changed during save");
        }
        recovery.transaction.phase = "complete";
        recovery.transaction.reason = "saved";
        recovery.transaction.acknowledged = ["temporary", "original", "proposed"];
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
      } catch (error) {
        if (sourceHeld && !targetInstalled) {
          try {
            await fsp.link(holding, abs);
          } catch {}
        }
        recovery.transaction.phase = "failed";
        recovery.transaction.reason = "save-failed";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction).catch(() => {});
        const current = await fsp.readFile(abs, "utf8").catch(() => null);
        this.observation.endWrite(watchContext, abs);
        this.observation.emitFileUpdate(watchContext, abs, current);
        await this.emitRecoveryFor(context);
        const detail = error instanceof Error ? error.message : String(error);
        throw new Error(`${detail}; recovery artifacts preserved in ${this.relKey(recovery.directory, root)}`);
      }
      this.contextFor(workspace);
      this.observation.recordSave(watchContext, abs, cleanContent, expectedContent, write);
      this.observation.endWrite(watchContext, abs);
      await this.emitRecoveryFor(context);
    });
  }

  async createFile(workspace: WorkspaceIdentity, rel: string): Promise<void> {
    await this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const context = this.contextFor(workspace);
      const watchContext = context.watchContext;
      if (!this.currentWatch(watchContext)) throw new Error("stale workspace");
      const clean = relativePath(rel);
      const abs = await confinedPath(root, clean);
      await fsp.mkdir(path.dirname(abs), { recursive: true });
      await confinedPath(root, clean);
      this.contextFor(workspace);
      await fsp.writeFile(abs, "", { flag: "wx" });
      this.contextFor(workspace);
      this.observation.recordCreate(watchContext, abs);
    });
  }

  async createDir(workspace: WorkspaceIdentity, rel: string): Promise<void> {
    await this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const abs = await confinedPath(root, relativePath(rel));
      this.contextFor(workspace);
      await fsp.mkdir(abs, { recursive: false });
    });
  }

  async deletePath(workspace: WorkspaceIdentity, rel: string): Promise<void> {
    await this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const context = this.contextFor(workspace);
      const watchContext = context.watchContext;
      if (!this.currentWatch(watchContext)) throw new Error("stale workspace");
      const abs = await confinedPath(root, relativePath(rel));
      const captured = await this.observation.captureDirectMutation(watchContext, abs);
      this.contextFor(workspace);
      await movePathToTrash(abs, (target) => shell.trashItem(target));
      this.contextFor(workspace);
      this.observation.recordDelete(watchContext, abs, captured.baseline);
    });
  }

  async detachPath(workspace: WorkspaceIdentity, rel: string): Promise<void> {
    await this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const context = this.contextFor(workspace);
      const watchContext = context.watchContext;
      if (!this.currentWatch(watchContext)) throw new Error("stale workspace");
      const abs = await confinedPath(root, relativePath(rel));
      const detachedRoot = path.join(homedir(), ".vela-detached");
      await fsp.mkdir(detachedRoot, { recursive: true });
      const target = path.join(detachedRoot, `${randomUUID()}-${path.basename(abs)}`);
      await fsp.rename(abs, target);
      this.contextFor(workspace);
      this.observation.recordDetach(watchContext, abs);
    });
  }

  async renamePath(workspace: WorkspaceIdentity, rel: string, newName: string): Promise<void> {
    await this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const context = this.contextFor(workspace);
      const watchContext = context.watchContext;
      if (!this.currentWatch(watchContext)) throw new Error("stale workspace");
      const abs = await confinedPath(root, relativePath(rel));
      const parent = this.relKey(path.dirname(abs), root);
      const target = await confinedPath(root, parent ? `${parent}/${fileName(newName)}` : fileName(newName));
      if (target === abs) return;
      this.contextFor(workspace);
      const source = await fsp.lstat(abs);
      if (source.isDirectory()) {
        const occupied = await fsp.lstat(target).catch((error: NodeJS.ErrnoException) => {
          if (error.code === "ENOENT") return null;
          throw error;
        });
        if (occupied) throw new Error(`destination already exists: ${path.basename(target)}`);
        this.contextFor(workspace);
        try {
          await fsp.rename(abs, target);
        } catch (error) {
          const code = (error as NodeJS.ErrnoException).code;
          if (code?.includes("EEXIST") || code === "ENOTEMPTY") {
            throw new Error(`destination already exists: ${path.basename(target)}`);
          }
          throw error;
        }
        this.contextFor(workspace);
        return;
      }
      const captured = await this.observation.captureDirectMutation(watchContext, abs);
      await this.mutationPhase("rename:source-inspected", abs, target);
      const recovery = await this.recovery.createRecoveryTransaction(
        root,
        "rename",
        this.relKey(abs, root),
        this.relKey(target, root)
      );
      const holding = path.join(recovery.directory, "source");
      let installed = false;
      try {
        await fsp.rename(abs, holding);
        await this.recovery.syncDirectory(path.dirname(abs));
        await this.recovery.syncDirectory(recovery.directory);
        recovery.transaction.phase = "source-held";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
        await this.mutationPhase("rename:source-held", holding, abs);
        await fsp.link(holding, target);
        await this.recovery.syncDirectory(path.dirname(target));
        installed = true;
        recovery.transaction.phase = "target-installed";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
        await this.mutationPhase("rename:target-installed", holding, target);
        recovery.transaction.phase = "complete";
        recovery.transaction.reason = "renamed";
        recovery.transaction.acknowledged = ["rename-source"];
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction);
      } catch (error) {
        if (!installed) {
          try {
            await fsp.link(holding, abs);
            await this.recovery.syncDirectory(path.dirname(abs));
          } catch {}
        }
        recovery.transaction.phase = "failed";
        recovery.transaction.reason = "rename-failed";
        await this.recovery.writeRecoveryTransaction(recovery.directory, recovery.transaction).catch(() => {});
        await this.emitRecoveryFor(context);
        if ((error as NodeJS.ErrnoException).code?.includes("EEXIST")) {
          throw new Error(`destination already exists: ${path.basename(target)}`);
        }
        throw error;
      }
      this.contextFor(workspace);
      this.observation.recordRename(watchContext, abs, target, captured);
    });
  }

  async movePath(workspace: WorkspaceIdentity, rel: string, newParent: string): Promise<void> {
    await this.mutations.run(workspace, async () => {
      const root = this.workspaceRoot(workspace);
      const context = this.contextFor(workspace);
      const watchContext = context.watchContext;
      if (!this.currentWatch(watchContext)) throw new Error("stale workspace");
      const clean = relativePath(rel);
      if ([RECOVERY_DIR, LEGACY_RECOVERY_DIR, PREVIOUS_RECOVERY_DIR].some((dir) => clean === dir || clean.startsWith(`${dir}/`))) {
        throw new Error("cannot move the recovery directory");
      }
      const parentRel = relativePath(newParent, true);
      if ([RECOVERY_DIR, LEGACY_RECOVERY_DIR, PREVIOUS_RECOVERY_DIR].some((dir) => parentRel === dir || parentRel.startsWith(`${dir}/`))) {
        throw new Error("cannot move into the recovery directory");
      }
      if (parentRel === clean || parentRel.startsWith(`${clean}/`)) {
        throw new Error("cannot move a folder into itself");
      }
      const abs = await confinedPath(root, clean);
      const parentDir = await confinedPath(root, parentRel, true);
      const parentStat = await fsp.lstat(parentDir).catch(() => null);
      if (!parentStat?.isDirectory()) throw new Error("destination folder does not exist");
      const name = path.basename(abs);
      const target = await confinedPath(root, parentRel ? `${parentRel}/${name}` : name);
      if (target === abs) throw new Error("entry is already in that folder");
      const occupied = await fsp.lstat(target).catch(() => null);
      if (occupied) throw new Error(`destination already exists: ${name}`);
      const captured = await this.observation.captureDirectMutation(watchContext, abs);
      this.contextFor(workspace);
      try {
        await fsp.rename(abs, target);
      } catch (error) {
        const code = (error as NodeJS.ErrnoException).code;
        if (code === "EXDEV") throw new Error("cannot move across filesystems");
        if (code?.includes("EEXIST")) throw new Error(`destination already exists: ${name}`);
        throw error;
      }
      this.contextFor(workspace);
      this.observation.recordMove(watchContext, abs, target, captured);
    });
  }

}
