import { randomUUID } from "node:crypto";
import { access, copyFile, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import type { ProjectInfo, SavedWorkspaceSnapshot } from "@shared/types";
import { existingWorkspace, locateWorkspace, within, workspaceLocation, workspacePath, type WorkspaceLocation, type WorkspaceProject } from "./workspace-location";

interface SavedWorkspaceData { workspaces: ProjectInfo[]; locations: WorkspaceLocation[] }

function isMissing(error: unknown): boolean {
  return Boolean(error && typeof error === "object" && "code" in error && error.code === "ENOENT");
}

function normalize(value: unknown): ProjectInfo[] {
  if (!Array.isArray(value)) throw new Error("Saved workspaces must be a list.");
  const seen = new Set<string>();
  return value.flatMap((item): ProjectInfo[] => {
    if (!item || typeof item !== "object" || Array.isArray(item)) return [];
    const workspace = item as { directory?: unknown; name?: unknown };
    if (typeof workspace.directory !== "string" || !workspace.directory.trim() || seen.has(workspace.directory)) return [];
    seen.add(workspace.directory);
    const fallbackName = workspace.directory.split(/[\\/]/).filter(Boolean).pop() ?? workspace.directory;
    return [{
      directory: workspace.directory,
      name: typeof workspace.name === "string" && workspace.name.trim() ? workspace.name : fallbackName
    }];
  });
}

function parse(raw: string): SavedWorkspaceData {
  const value: unknown = JSON.parse(raw);
  if (Array.isArray(value)) return { workspaces: normalize(value), locations: [] };
  if (!value || typeof value !== "object") throw new Error("Saved workspaces file has an invalid format.");
  const stored = value as { version?: unknown; workspaces?: unknown; locations?: unknown };
  if ((stored.version !== 1 && stored.version !== 2) || !Array.isArray(stored.workspaces)) {
    throw new Error("Saved workspaces file has an unsupported format.");
  }
  const locations: WorkspaceLocation[] = [];
  if (stored.version === 2) {
    if (!Array.isArray(stored.locations)) throw new Error("Saved workspace locations have an invalid format.");
    for (const entry of stored.locations as WorkspaceLocation[]) {
      if (!entry || !path.isAbsolute(entry.directory)) throw new Error("Invalid saved workspace location.");
      if (entry.movedTo !== undefined && !path.isAbsolute(entry.movedTo)) throw new Error("Invalid workspace relocation.");
      locations.push(entry);
    }
  }
  return { workspaces: normalize(stored.workspaces), locations };
}

export class SavedWorkspaceStore {
  private readonly backupPath: string;
  private writeQueue: Promise<void> = Promise.resolve();

  constructor(private readonly filePath: string) {
    this.backupPath = `${filePath}.bak`;
  }

  private async readData(): Promise<SavedWorkspaceData> {
    try {
      return parse(await readFile(this.filePath, "utf8"));
    } catch (primaryError) {
      try {
        return parse(await readFile(this.backupPath, "utf8"));
      } catch (backupError) {
        if (isMissing(primaryError) && isMissing(backupError)) return { workspaces: [], locations: [] };
        if (isMissing(backupError)) throw primaryError;
        throw new AggregateError([primaryError, backupError], "Could not read saved workspaces or their backup.");
      }
    }
  }

  async read(): Promise<ProjectInfo[]> {
    await this.writeQueue;
    return (await this.readData()).workspaces;
  }

  async resolve(directory: string, projects: WorkspaceProject[], projectID?: string): Promise<string | null> {
    await this.writeQueue;
    directory = await workspacePath(directory);
    const catalog = await Promise.all(projects.map(async (project) => ({
      ...project,
      ...(project.canonical ? { canonical: await workspacePath(project.canonical) } : {})
    })));
    const resolved = await locateWorkspace(directory, (await this.readData()).locations, catalog, projectID);
    if (!resolved) return null;
    if (resolved.previousRoot !== resolved.currentRoot) {
      await this.relocate(resolved.previousRoot, resolved.currentRoot);
    }
    return resolved.directory;
  }

  relocate(previousDirectory: string, directory: string): Promise<void> {
    return this.update(async (data) => {
      previousDirectory = await workspacePath(previousDirectory);
      const canonical = await existingWorkspace(directory);
      if (!canonical) throw new Error(`Workspace no longer exists: ${directory}`);
      if (previousDirectory === canonical) return;
      data.locations = data.locations.filter((entry) => entry.directory !== previousDirectory && entry.directory !== canonical);
      data.locations.push({ directory: previousDirectory, movedTo: canonical }, await workspaceLocation(canonical));
      const repaired = await Promise.all(data.workspaces.map(async (workspace) => {
        const original = await workspacePath(workspace.directory);
        if (!within(previousDirectory, original)) return workspace;
        const target = await existingWorkspace(path.join(canonical, path.relative(previousDirectory, original)));
        return target ? { ...workspace, directory: target } : workspace;
      }));
      data.workspaces = normalize(repaired);
    });
  }

  async load(): Promise<SavedWorkspaceSnapshot> {
    const workspaces = await this.read();
    const initialized = await Promise.all([this.filePath, this.backupPath].map(async (file) => {
      try {
        await access(file);
        return true;
      } catch (error) {
        if (isMissing(error)) return false;
        throw error;
      }
    }));
    return { workspaces, initialized: initialized.some(Boolean) };
  }

  save(value: unknown): Promise<ProjectInfo[]> {
    let workspaces = normalize(value);
    return this.update(async (data) => {
      workspaces = normalize(await Promise.all(workspaces.map(async (workspace) => {
        const directory = await workspacePath(workspace.directory);
        const resolved = await locateWorkspace(directory, data.locations, []);
        return { ...workspace, directory: resolved?.directory ?? directory };
      })));
      data.workspaces = workspaces;
      for (const workspace of workspaces) {
        const canonical = await existingWorkspace(workspace.directory);
        if (!canonical) continue;
        data.locations = data.locations.filter((entry) => entry.directory !== canonical);
        data.locations.push(await workspaceLocation(canonical));
      }
    }).then(() => workspaces);
  }

  private update(change: (data: SavedWorkspaceData) => Promise<void>): Promise<void> {
    const operation = this.writeQueue.then(async () => {
      const data = await this.readData();
      await change(data);
      await mkdir(path.dirname(this.filePath), { recursive: true });
      const temporaryPath = `${this.filePath}.${process.pid}.${randomUUID()}.tmp`;
      const serialized = `${JSON.stringify({ version: 2, ...data }, null, 2)}\n`;
      try {
        let previous: string | null = null;
        try {
          previous = await readFile(this.filePath, "utf8");
        } catch (error) {
          if (!isMissing(error)) throw error;
        }
        let previousIsValid = false;
        if (previous !== null) {
          try {
            parse(previous);
            previousIsValid = true;
          } catch {
            // Preserve an existing good backup when the primary file is corrupt.
          }
        }
        if (previousIsValid) {
          await copyFile(this.filePath, this.backupPath);
        } else {
          try {
            parse(await readFile(this.backupPath, "utf8"));
          } catch (error) {
            if (!isMissing(error)) throw new Error("Cannot replace saved workspaces without a readable current file or backup.");
            if (previous !== null) throw new Error("Cannot replace a corrupt saved workspaces file without a readable backup.");
          }
        }
        await writeFile(temporaryPath, serialized, { encoding: "utf8", mode: 0o600, flag: "wx" });
        await rename(temporaryPath, this.filePath);
      } catch (error) {
        await rm(temporaryPath, { force: true }).catch(() => {});
        throw error;
      }
    });
    this.writeQueue = operation.catch(() => {});
    return operation;
  }
}
