import { readFile, mkdir, writeFile, rename, copyFile, rm } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";

export class SassPreferences {
  private loaded: Promise<Set<string>> | null = null;
  private queue: Promise<void> = Promise.resolve();
  constructor(private file: string) {}
  private read(): Promise<Set<string>> {
    return this.loaded ??= (async () => {
      let missing = 0;
      for (const file of [this.file, `${this.file}.bak`]) {
        try {
          const value = JSON.parse(await readFile(file, "utf8"));
          if (value.version !== 1 || !Array.isArray(value.enabled) || value.enabled.some((item: unknown) => typeof item !== "string" || !path.isAbsolute(item))) throw new Error("Invalid Sass watcher preferences");
          return new Set<string>(value.enabled);
        } catch (error) { if ((error as NodeJS.ErrnoException).code === "ENOENT") missing++; }
      }
      if (missing === 2) return new Set<string>();
      throw new Error("Could not read Sass watcher preferences or their backup");
    })();
  }
  async enabled(root: string): Promise<boolean> { await this.queue; return (await this.read()).has(root); }
  async set(root: string, enabled: boolean): Promise<void> {
    const writing = this.queue.then(async () => {
      const current = await this.read();
      const next = new Set(current);
      if (enabled) next.add(root); else next.delete(root);
      await mkdir(path.dirname(this.file), { recursive: true });
      const temp = `${this.file}.${randomUUID()}.tmp`;
      try {
        await writeFile(temp, JSON.stringify({ version: 1, enabled: [...next].sort() }) + "\n");
        await copyFile(this.file, `${this.file}.bak`).catch((error) => { if (error.code !== "ENOENT") throw error; });
        await rename(temp, this.file);
        this.loaded = Promise.resolve(next);
      } finally { await rm(temp, { force: true }).catch(() => {}); }
    });
    this.queue = writing.catch(() => {});
    return writing;
  }
}
