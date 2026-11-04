import { promises as fsp } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import type { RuntimeID, SessionSummary } from "@shared/types";

export interface RuntimeSessionRecord extends SessionSummary {
  runtimeID: RuntimeID;
}

function defaultPath(): string {
  if (process.platform === "darwin") return path.join(homedir(), "Library", "Application Support", "Vela", "runtime-sessions.json");
  if (process.platform === "win32" && process.env.APPDATA) return path.join(process.env.APPDATA, "Vela", "runtime-sessions.json");
  return path.join(process.env.XDG_STATE_HOME ?? path.join(homedir(), ".local", "state"), "vela", "runtime-sessions.json");
}

export function previousRuntimeSessionPath(): string {
  if (process.platform === "darwin") return path.join(homedir(), "Library", "Application Support", "Vela", "runtime-sessions.json");
  if (process.platform === "win32" && process.env.APPDATA) return path.join(process.env.APPDATA, "Vela", "runtime-sessions.json");
  return path.join(process.env.XDG_STATE_HOME ?? path.join(homedir(), ".local", "state"), "vela", "runtime-sessions.json");
}

function valid(value: unknown): value is RuntimeSessionRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const item = value as Record<string, unknown>;
  return item.runtimeID === "deepseek" && typeof item.id === "string" && typeof item.title === "string" &&
    typeof item.directory === "string" && typeof item.updatedAt === "number" && Number.isFinite(item.updatedAt);
}

export class RuntimeSessionIndex {
  private loaded = false;
  private records = new Map<string, RuntimeSessionRecord>();
  private pending = Promise.resolve();

  constructor(private readonly file = defaultPath(), private readonly legacyFile?: string) {}

  async list(): Promise<RuntimeSessionRecord[]> {
    await this.load();
    return [...this.records.values()].sort((a, b) => b.updatedAt - a.updatedAt);
  }

  async get(sessionID: string): Promise<RuntimeSessionRecord | null> {
    await this.load();
    return this.records.get(sessionID) ?? null;
  }

  async put(record: RuntimeSessionRecord): Promise<void> {
    await this.load();
    this.records.set(record.id, record);
    await this.save();
  }

  async touch(sessionID: string, updatedAt = Date.now()): Promise<void> {
    await this.load();
    const record = this.records.get(sessionID);
    if (!record) return;
    this.records.set(sessionID, { ...record, updatedAt });
    await this.save();
  }

  private async load(): Promise<void> {
    if (this.loaded) return;
    this.loaded = true;
    const raw = await fsp.readFile(this.file, "utf8").catch(() =>
      this.legacyFile ? fsp.readFile(this.legacyFile, "utf8").catch(() => "[]") : "[]"
    );
    try {
      const parsed = JSON.parse(raw) as unknown;
      if (Array.isArray(parsed)) {
        for (const item of parsed) if (valid(item)) this.records.set(item.id, item);
      }
    } catch {
      this.records.clear();
    }
  }

  private save(): Promise<void> {
    this.pending = this.pending.then(async () => {
      await fsp.mkdir(path.dirname(this.file), { recursive: true, mode: 0o700 });
      const temporary = `${this.file}.${process.pid}.tmp`;
      await fsp.writeFile(temporary, JSON.stringify([...this.records.values()]), { encoding: "utf8", mode: 0o600 });
      await fsp.rename(temporary, this.file);
    });
    return this.pending;
  }
}
