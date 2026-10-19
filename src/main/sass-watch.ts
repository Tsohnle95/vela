import { realpath, lstat, mkdir, writeFile, rename, rm } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { fileURLToPath } from "node:url";
import { watch, type FSWatcher } from "chokidar";
import type { SessionInfo, WorkspaceIdentity, SassWatchState, SassWatchLog } from "@shared/types";
import { isSassSource, sassIgnored, sassPlan, insideRoot } from "./sass-config";
import { SassWorkerCompiler, type SassCompiler } from "./sass-compiler";
import { SassPreferences } from "./sass-preferences";

interface RecordState {
  root: string; enabled: boolean; active: boolean; status: SassWatchState["status"]; revision: number;
  error?: string; logs: SassWatchLog[]; ready: Promise<void>; watcher?: FSWatcher; compiler?: SassCompiler;
  timer?: ReturnType<typeof setTimeout>; running?: Promise<void>; dirty: boolean;
}
export class SassWatchManager {
  private records = new Map<string, RecordState>();
  private owners = new Map<string, { workspace: WorkspaceIdentity; root: string; sessionID: string }>();
  private closed = new Map<string, number>();
  private shuttingDown = false;
  private revision = 0;
  private operations = new Map<string, Promise<void>>();
  constructor(private preferences: SassPreferences, private publish: (state: SassWatchState) => void = () => {}, private compilerFactory: () => SassCompiler = () => new SassWorkerCompiler()) {}
  private withRoot<T>(root: string, action: () => Promise<T>): Promise<T> {
    const result = (this.operations.get(root) ?? Promise.resolve()).then(action);
    const tail = result.then(() => {}, () => {});
    this.operations.set(root, tail);
    void tail.then(() => { if (this.operations.get(root) === tail) this.operations.delete(root); });
    return result;
  }

  async register(session: SessionInfo): Promise<SassWatchState> {
    if (this.shuttingDown || (this.closed.get(session.workspace.id) ?? 0) >= session.workspace.generation) throw new Error("Sass workspace is closed");
    const root = await realpath(session.directory);
    if (!(await lstat(root)).isDirectory()) throw new Error("Sass workspace is not a directory");
    if (this.shuttingDown || (this.closed.get(session.workspace.id) ?? 0) >= session.workspace.generation) throw new Error("Sass workspace is closed");
    const previous = this.owners.get(session.workspace.id);
    if (previous && (previous.root !== root || previous.workspace.generation !== session.workspace.generation)) await this.unregister(previous.workspace);
    return this.withRoot(root, async () => {
      if (this.shuttingDown || (this.closed.get(session.workspace.id) ?? 0) >= session.workspace.generation) throw new Error("Sass workspace is closed");
      this.owners.set(session.workspace.id, { workspace: session.workspace, root, sessionID: session.id });
      let record = this.records.get(root);
      if (!record) {
        record = { root, enabled: false, active: false, status: "off", revision: ++this.revision, logs: [], ready: Promise.resolve(), dirty: false };
        this.records.set(root, record);
        const captured = record;
        record.ready = this.preferences.enabled(root).then((enabled) => {
          if (enabled && !this.shuttingDown && (this.closed.get(session.workspace.id) ?? 0) < session.workspace.generation && this.records.get(root) === captured && this.hasOwners(root)) this.start(captured);
        }).catch((error) => this.error(captured, error));
      }
      await record.ready;
      if ((this.closed.get(session.workspace.id) ?? 0) >= session.workspace.generation) throw new Error("Sass workspace is closed");
      this.assertOwner(session.workspace);
      return this.snapshot(record, session.workspace);
    });
  }
  private hasOwners(root: string): boolean { return [...this.owners.values()].some((owner) => owner.root === root); }
  private assertOwner(workspace: WorkspaceIdentity): RecordState {
    const owner = this.owners.get(workspace.id);
    if (!owner || owner.workspace.generation !== workspace.generation) throw new Error("Sass workspace is closed");
    return this.records.get(owner.root)!;
  }
  private snapshot(record: RecordState, workspace: WorkspaceIdentity): SassWatchState {
    return { workspace, directory: record.root, enabled: record.enabled, status: record.status, revision: record.revision, ...(record.error ? { error: record.error } : {}), logs: [...record.logs] };
  }
  private emit(record: RecordState): void {
    record.revision = ++this.revision;
    for (const owner of this.owners.values()) if (owner.root === record.root) this.publish(this.snapshot(record, owner.workspace));
  }
  private log(record: RecordState, level: SassWatchLog["level"], message: string): void {
    record.logs = [...record.logs.slice(-99), { time: Date.now(), level, message: message.slice(0, 8_192) }];
  }
  private error(record: RecordState, error: unknown): void {
    record.status = "error"; record.error = String(error);
    this.log(record, "error", record.error);
    this.emit(record);
  }
  state(workspace: WorkspaceIdentity): SassWatchState { return this.snapshot(this.assertOwner(workspace), workspace); }
  async setEnabled(workspace: WorkspaceIdentity, enabled: boolean): Promise<SassWatchState> {
    const record = this.assertOwner(workspace);
    return this.withRoot(record.root, async () => {
      await record.ready;
      this.assertOwner(workspace);
      if (!enabled) await this.stop(record);
      try {
        await this.preferences.set(record.root, enabled);
        if (this.records.get(record.root) !== record || !this.hasOwners(record.root) || this.shuttingDown) return this.snapshot(record, workspace);
        if (enabled && !record.active) this.start(record);
      } catch (error) { this.error(record, error); }
      return this.snapshot(record, workspace);
    });
  }
  private start(record: RecordState): void {
    record.enabled = true; record.active = true; record.status = "starting"; record.error = undefined;
    this.log(record, "info", `Starting bundled Dart Sass in ${record.root}`);
    this.emit(record);
    try {
      record.compiler = this.compilerFactory();
      record.watcher = watch(record.root, {
        ignoreInitial: true, followSymlinks: false,
        ignored: (file, stat) => sassIgnored(record.root, file) || Boolean(stat?.isFile() && !this.relevant(record.root, file)),
        awaitWriteFinish: { stabilityThreshold: 100, pollInterval: 25 }
      });
      record.watcher.on("ready", () => { if (record.active) this.schedule(record); });
      record.watcher.on("all", (event, file) => {
        if (record.active && (event === "addDir" || event === "unlinkDir" || this.relevant(record.root, file))) this.schedule(record);
      });
      record.watcher.on("error", (error) => { if (record.active) this.error(record, error); });
    } catch (error) { record.active = false; this.error(record, error); }
  }
  private relevant(root: string, file: string): boolean {
    return isSassSource(file) || ["vela-sass.json", "vela-sass.json", "package.json", path.join(".vscode", "settings.json")].includes(path.relative(root, file));
  }
  private schedule(record: RecordState): void {
    clearTimeout(record.timer);
    record.timer = setTimeout(() => { record.timer = undefined; void this.run(record); }, 150);
  }
  private async run(record: RecordState): Promise<void> {
    if (!record.active) return;
    if (record.running) { record.dirty = true; return; }
    record.running = (async () => {
      do {
        record.dirty = false;
        const failures: string[] = [];
        try {
          const rootStat = await lstat(record.root);
          if (!rootStat.isDirectory() || rootStat.isSymbolicLink() || await realpath(record.root) !== record.root) throw new Error("Sass workspace root changed; reopen it before compiling");
          const plan = await sassPlan(record.root);
          if (!record.active) return;
          this.log(record, "info", `${plan.convention}: compiling ${plan.outputs.length} CSS output${plan.outputs.length === 1 ? "" : "s"}`);
          for (const output of plan.outputs) {
            if (!record.active) return;
            try {
              const result = await record.compiler!.compile(output.source, output.options);
              if (!record.active) return;
              for (const warning of result.warnings) this.log(record, "warning", warning);
              if (result.sourceMap) {
                const map = JSON.parse(result.sourceMap);
                map.sources = map.sources.map((source: string) => source.startsWith("file:") ? encodeURI(path.relative(path.dirname(output.output), fileURLToPath(source)).split(path.sep).join("/")) : source);
                await this.write(record, `${output.output}.map`, JSON.stringify(map));
              }
              await this.write(record, output.output, result.css + (result.sourceMap ? `\n/*# sourceMappingURL=${encodeURIComponent(path.basename(output.output))}.map */\n` : "\n"));
              if (!record.active) return;
              this.log(record, "info", `${path.relative(record.root, output.source)} → ${path.relative(record.root, output.output)}`);
            } catch (error) {
              if (!record.active) return;
              failures.push(String(error));
              this.log(record, "error", `${path.relative(record.root, output.source)}: ${String(error)}`);
            }
          }
        } catch (error) { if (!record.active) return; failures.push(String(error)); this.log(record, "error", String(error)); }
        if (!record.active) return;
        record.status = failures.length ? "error" : "on"; record.error = failures[0]; this.emit(record);
      } while (record.active && record.dirty);
    })().finally(() => { record.running = undefined; });
    await record.running;
  }
  private async write(record: RecordState, file: string, content: string): Promise<void> {
    if (!insideRoot(record.root, file)) throw new Error("Sass output escapes the workspace");
    const rootStat = await lstat(record.root);
    if (!rootStat.isDirectory() || rootStat.isSymbolicLink() || await realpath(record.root) !== record.root) throw new Error("Sass workspace root changed; reopen it before compiling");
    let directory = record.root;
    for (const segment of path.relative(record.root, path.dirname(file)).split(path.sep).filter(Boolean)) {
      directory = path.join(directory, segment);
      await mkdir(directory, { recursive: false }).catch((error) => { if (error.code !== "EEXIST") throw error; });
      if (!(await lstat(directory)).isDirectory() || (await lstat(directory)).isSymbolicLink()) throw new Error("Sass output directory is not a real workspace directory");
    }
    const existing = await lstat(file).catch((error) => { if (error.code === "ENOENT") return null; throw error; });
    if (existing && (!existing.isFile() || existing.isSymbolicLink())) throw new Error("Sass output is not a regular file");
    if (!record.active) return;
    const temp = path.join(directory, `.vela-sass-${randomUUID()}.tmp`);
    try {
      await writeFile(temp, content, { flag: "wx" });
      if (record.active) await rename(temp, file);
    } finally { await rm(temp, { force: true }).catch(() => {}); }
  }
  private async stop(record: RecordState): Promise<void> {
    record.active = false; record.enabled = false;
    clearTimeout(record.timer); record.timer = undefined;
    await Promise.all([record.watcher?.close(), record.compiler?.close()]);
    await record.running;
    record.watcher = undefined; record.compiler = undefined; record.status = "off"; record.error = undefined;
    this.log(record, "info", "Sass watching stopped");
    this.emit(record);
  }
  async unregister(workspace: WorkspaceIdentity): Promise<void> {
    this.closed.set(workspace.id, Math.max(this.closed.get(workspace.id) ?? 0, workspace.generation));
    const owner = this.owners.get(workspace.id);
    if (!owner || owner.workspace.generation !== workspace.generation) return;
    return this.withRoot(owner.root, async () => {
      if (this.owners.get(workspace.id)?.workspace.generation !== workspace.generation) return;
      this.owners.delete(workspace.id);
      const record = this.records.get(owner.root);
      if (record && !this.hasOwners(owner.root)) { await this.stop(record); this.records.delete(owner.root); }
    });
  }
  async stopSession(sessionID: string): Promise<void> {
    await Promise.all([...this.owners.values()].filter((owner) => owner.sessionID === sessionID).map((owner) => this.unregister(owner.workspace)));
  }
  async stopAll(): Promise<void> {
    this.shuttingDown = true;
    await Promise.all([...this.operations.values()]);
    await Promise.all([...this.records.values()].map((record) => this.stop(record)));
    this.records.clear(); this.owners.clear();
  }
}
