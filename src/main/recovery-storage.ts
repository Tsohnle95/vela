import { promises as fsp } from "node:fs";
import path from "node:path";
import { createHash, randomUUID } from "node:crypto";
import { LEGACY_RECOVERY_DIR, OLDEST_RECOVERY_DIR, PREVIOUS_RECOVERY_DIR, RECOVERY_DIR, type RecoveryRecord } from "@shared/types";
import { confinedPath, relativePath } from "./workspace-security";

export { LEGACY_RECOVERY_DIR, PREVIOUS_RECOVERY_DIR, RECOVERY_DIR } from "@shared/types";

const RECOVERY_SETTLED_RETENTION_MS = 24 * 60 * 60 * 1000;
const RECOVERY_INTERRUPTED_RETENTION_MS = 7 * 24 * 60 * 60 * 1000;

export interface RecoveryTransaction {
  version: 1;
  id: string;
  operation: "save" | "rename";
  originalPath: string;
  targetPath?: string;
  createdAt: number;
  phase: "initialized" | "temporary-ready" | "source-held" | "held-validated" | "target-installed" | "failed" | "complete";
  acknowledged: string[];
  reason?: RecoveryRecord["reason"];
}

export class RecoveryStorage {
  private async recoveryRoot(root: string): Promise<string> {
    const rootStat = await fsp.stat(root).catch(() => null);
    const git = path.join(root, ".git");
    const gitStat = await fsp.lstat(git).catch(() => null);
    if (rootStat && gitStat?.isDirectory() && gitStat.dev === rootStat.dev) {
      return path.join(git, RECOVERY_DIR.slice(1));
    }
    const parent = path.dirname(root);
    const parentStat = await fsp.stat(parent).catch(() => null);
    if (rootStat && parentStat && parentStat.dev === rootStat.dev) {
      const key = createHash("sha256").update(root).digest("hex").slice(0, 16);
      return path.join(parent, `.vela-recovery-${key}`);
    }
    return path.join(root, RECOVERY_DIR);
  }

  private async recoveryLocations(root: string): Promise<string[]> {
    const current = await this.recoveryRoot(root);
    const currentName = path.basename(current);
    const previousNames = [PREVIOUS_RECOVERY_DIR, OLDEST_RECOVERY_DIR]
      .map((directory) => currentName.replace(RECOVERY_DIR.slice(1), directory.slice(1)));
    return [...new Set([
      current,
      ...previousNames.map((name) => path.join(path.dirname(current), name)),
      ...[PREVIOUS_RECOVERY_DIR, LEGACY_RECOVERY_DIR, OLDEST_RECOVERY_DIR].map((directory) => path.join(root, directory))
    ])];
  }

  private async checkedRecoveryRoot(root: string, create = false, location?: string): Promise<string> {
    const selected = location ?? await this.recoveryRoot(root);
    const parent = path.dirname(selected);
    const parentStat = await fsp.lstat(parent);
    if (!parentStat.isDirectory()) throw new Error("invalid recovery parent");
    const recoveryRoot = await confinedPath(parent, path.basename(selected));
    const stat = await fsp.lstat(recoveryRoot).catch((error) => {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw error;
    });
    if (!stat && create) {
      await fsp.mkdir(recoveryRoot, { mode: 0o700 });
      return confinedPath(parent, path.basename(selected));
    }
    if (stat && !stat.isDirectory()) throw new Error("invalid recovery directory");
    return recoveryRoot;
  }

  async syncDirectory(directory: string): Promise<void> {
    const handle = await fsp.open(directory, "r").catch((error) => {
      if (["EISDIR", "EINVAL", "EPERM", "ENOTSUP"].includes((error as NodeJS.ErrnoException).code ?? "")) return null;
      throw error;
    });
    if (!handle) return;
    try {
      await handle.sync().catch((error) => {
        if (!["EINVAL", "EPERM", "ENOTSUP"].includes((error as NodeJS.ErrnoException).code ?? "")) throw error;
      });
    } finally {
      await handle.close();
    }
  }

  async writeRecoveryTransaction(directory: string, transaction: RecoveryTransaction): Promise<void> {
    const temporary = path.join(directory, `manifest-${randomUUID()}.tmp`);
    const handle = await fsp.open(temporary, "wx", 0o600);
    try {
      await handle.writeFile(JSON.stringify(transaction, null, 2), "utf8");
      await handle.sync();
    } finally {
      await handle.close();
    }
    await fsp.rename(temporary, path.join(directory, "manifest.json"));
    await this.syncDirectory(directory);
  }

  async createRecoveryTransaction(
    root: string,
    operation: RecoveryTransaction["operation"],
    originalPath: string,
    targetPath?: string
  ): Promise<{ directory: string; transaction: RecoveryTransaction }> {
    const id = `${Date.now()}-${randomUUID()}`;
    const recoveryRoot = await this.checkedRecoveryRoot(root, true);
    const directory = path.join(recoveryRoot, id);
    await fsp.mkdir(directory);
    const transaction: RecoveryTransaction = {
      version: 1,
      id,
      operation,
      originalPath,
      ...(targetPath ? { targetPath } : {}),
      createdAt: Date.now(),
      phase: "initialized",
      acknowledged: []
    };
    await this.writeRecoveryTransaction(directory, transaction);
    await this.syncDirectory(recoveryRoot);
    return { directory, transaction };
  }

  private validRecoveryTransaction(value: unknown, id: string): value is RecoveryTransaction {
    if (!value || typeof value !== "object" || Array.isArray(value)) return false;
    const transaction = value as Partial<RecoveryTransaction>;
    const phases = ["initialized", "temporary-ready", "source-held", "held-validated", "target-installed", "failed", "complete"];
    const artifacts = transaction.operation === "save"
      ? ["temporary", "original", "proposed"]
      : ["rename-source"];
    try {
      relativePath(transaction.originalPath);
      if (transaction.operation === "rename") relativePath(transaction.targetPath);
    } catch {
      return false;
    }
    return transaction.version === 1 &&
      transaction.id === id &&
      /^\d{13}-[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id) &&
      (transaction.operation === "save" || transaction.operation === "rename") &&
      typeof transaction.createdAt === "number" && Number.isSafeInteger(transaction.createdAt) && transaction.createdAt > 0 &&
      typeof transaction.phase === "string" && phases.includes(transaction.phase) &&
      Array.isArray(transaction.acknowledged) &&
      transaction.acknowledged.every((artifact) => typeof artifact === "string" && artifacts.includes(artifact));
  }

  async readRecoveryTransactions(root: string): Promise<Array<{
    directory: string;
    transaction: RecoveryTransaction;
  }>> {
    const locations = await this.recoveryLocations(root);
    const transactions = await Promise.all(locations.map(async (location) => {
      const recoveryRoot = await this.checkedRecoveryRoot(root, false, location);
      const entries = await fsp.readdir(recoveryRoot, { withFileTypes: true }).catch(() => []);
      return Promise.all(entries.filter((entry) => entry.isDirectory()).map(async (entry) => {
        if (!/^\d{13}-[0-9a-f-]{36}$/.test(entry.name)) return null;
        try {
          const directory = await confinedPath(recoveryRoot, entry.name);
          const manifest = await confinedPath(directory, "manifest.json");
          if (!(await fsp.lstat(manifest)).isFile()) return null;
          const transaction = JSON.parse(await fsp.readFile(manifest, "utf8")) as unknown;
          if (!this.validRecoveryTransaction(transaction, entry.name)) return null;
          return { directory, transaction };
        } catch {
          return null;
        }
      }));
    }));
    return transactions.flat().filter((value): value is NonNullable<typeof value> => value !== null);
  }

  async recoveryRecords(root: string): Promise<RecoveryRecord[]> {
    const transactions = await this.readRecoveryTransactions(root);
    const records: RecoveryRecord[] = [];
    for (const { directory, transaction } of transactions) {
      const artifacts = transaction.operation === "save"
        ? [["temporary", "temporary"] as const, ["original", "original"] as const, ["proposed", "proposed"] as const]
        : [["rename-source", "source"] as const];
      for (const [artifact, name] of artifacts) {
        const file = await confinedPath(directory, name);
        if (!(await fsp.lstat(file).catch(() => null))?.isFile()) continue;
        records.push({
          id: `${transaction.id}:${artifact}`,
          artifact,
          originalPath: transaction.originalPath,
          recoveryPath: file,
          createdAt: transaction.createdAt,
          acknowledged: transaction.acknowledged.includes(artifact),
          reason: transaction.reason ?? (transaction.phase === "complete" ? "saved" : "save-failed")
        });
      }
    }
    return records.sort((a, b) => b.createdAt - a.createdAt || a.id.localeCompare(b.id));
  }

  async reconcileRecovery(root: string, current: () => boolean = () => true): Promise<void> {
    const transactions = await this.readRecoveryTransactions(root);
    for (const item of transactions) {
      if (!current()) throw new Error("activation superseded");
      const { directory, transaction } = item;
      if (transaction.phase === "complete" || transaction.phase === "failed") continue;
      if (transaction.phase !== "source-held" && transaction.phase !== "held-validated") continue;
      const original = await confinedPath(directory, transaction.operation === "rename" ? "source" : "original");
      if (!(await fsp.lstat(original).catch(() => null))?.isFile()) continue;
      const canonical = await confinedPath(root, transaction.originalPath);
      if (transaction.operation === "rename" && transaction.targetPath) {
        const target = await confinedPath(root, transaction.targetPath);
        const [heldStat, targetStat] = await Promise.all([
          fsp.stat(original),
          fsp.stat(target).catch(() => null)
        ]);
        if (targetStat && heldStat.dev === targetStat.dev && heldStat.ino === targetStat.ino) {
          if (!current()) throw new Error("activation superseded");
          transaction.phase = "complete";
          transaction.reason = "renamed";
          transaction.acknowledged = ["rename-source"];
          await this.writeRecoveryTransaction(directory, transaction);
          continue;
        }
      }
      if (await fsp.lstat(canonical).catch(() => null)) continue;
      if (!current()) throw new Error("activation superseded");
      const parentRel = path.posix.dirname(transaction.originalPath);
      const parent = parentRel === "." ? root : await confinedPath(root, parentRel);
      await fsp.mkdir(parent, { recursive: true });
      await confinedPath(root, transaction.originalPath);
      if (!current()) throw new Error("activation superseded");
      try {
        await fsp.link(original, canonical);
        await this.syncDirectory(parent);
        transaction.phase = "complete";
        transaction.reason = "crash-recovered";
        await this.writeRecoveryTransaction(directory, transaction);
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
      }
    }
  }

  async migrateLegacyRecovery(root: string, current: () => boolean): Promise<void> {
    const destination = await this.checkedRecoveryRoot(root, true);
    const locations = await this.recoveryLocations(root);
    const currentRoot = await this.recoveryRoot(root);
    const legacyLocations = locations.filter((location) => location !== currentRoot);
    const transactions = await this.readRecoveryTransactions(root);
    for (const legacyRoot of legacyLocations) {
      const legacy = transactions.filter(({ directory }) => path.dirname(directory) === legacyRoot);
      for (const { directory, transaction } of legacy) {
        if (!current()) return;
        const target = await confinedPath(destination, transaction.id);
        if (await fsp.lstat(target).catch(() => null)) continue;
        await fsp.rename(directory, target).catch(() => {});
      }
      await this.syncDirectory(legacyRoot).catch(() => {});
      await fsp.rmdir(legacyRoot).catch(() => {});
    }
    await this.syncDirectory(destination).catch(() => {});
  }

  async purgeExpiredRecovery(root: string, current: () => boolean): Promise<void> {
    const transactions = await this.readRecoveryTransactions(root).catch(() => []);
    const now = Date.now();
    for (const { directory, transaction } of transactions) {
      if (!current()) return;
      const settled = transaction.phase === "complete" || transaction.phase === "failed" || transaction.acknowledged.length > 0;
      const interrupted = transaction.phase === "source-held" || transaction.phase === "held-validated";
      if (!settled && !interrupted) continue;
      const retention = settled ? RECOVERY_SETTLED_RETENTION_MS : RECOVERY_INTERRUPTED_RETENTION_MS;
      if (now - transaction.createdAt < retention) continue;
      await fsp.rm(directory, { recursive: true, force: true }).catch(() => {});
    }
    for (const location of await this.recoveryLocations(root)) {
      await this.syncDirectory(location).catch(() => {});
    }
  }

}
