import { readFile, readdir, realpath, writeFile, rename } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { watch, type FSWatcher } from "chokidar";
import postcss from "postcss";
import valueParser from "postcss-value-parser";
import { SourceMapConsumer } from "source-map-js";
import { APP_STYLES_START, APP_STYLES_END, type AppStylesheetUpdate } from "../shared/app-styles";
import { SassWorkerCompiler, type SassCompiler } from "./sass-compiler";
import type { SassCompileResult } from "./sass-compile";

function compiledStyles(result: SassCompileResult, entry: string, output: string, before: string, after: string, revision: number) {
  if (!result.sourceMap) throw new Error("App stylesheet compilation returned no source map");
  const consumer = new SourceMapConsumer(JSON.parse(result.sourceMap));
  const styles = postcss.parse(result.css, { from: entry, map: { prev: result.sourceMap } });
  styles.walkAtRules("charset", rule => { rule.remove(); });
  styles.walkDecls(decl => {
    const start = decl.source!.start!;
    const original = consumer.originalPositionFor({ line: start.line, column: start.column - 1 });
    const owner = original.source ? new URL(original.source, pathToFileURL(entry)) : pathToFileURL(entry);
    const value = valueParser(decl.value);
    value.walk(node => {
      if (node.type !== "function" || node.value.toLowerCase() !== "url") return;
      const argument = node.nodes[0];
      if (!argument || (argument.type !== "string" && argument.type !== "word")) return false;
      if (/^(?:[a-z][\w+.-]*:|\/|#)/i.test(argument.value)) return false;
      argument.value = new URL(argument.value, owner).href;
      return false;
    });
    decl.value = value.toString();
  });
  const combined = postcss.parse(before, { from: output });
  combined.append(styles.nodes);
  combined.append(postcss.parse(after, { from: output }).nodes);
  return combined.toResult({ to: output, map: { inline: false, annotation: `${path.basename(output)}.map?revision=${revision}`, sourcesContent: false } });
}

export class AppStylesController {
  private watcher?: FSWatcher;
  private compiler?: SassCompiler;
  private starting?: Promise<void>;
  private running?: Promise<void>;
  private ready?: () => void;
  private timer?: ReturnType<typeof setTimeout>;
  private dirty = false;
  private closed = false;
  private revision = 0;
  private current?: AppStylesheetUpdate;
  private entry = "";
  private output = "";
  private originalURL = "";
  private before = "";
  private after = "";

  constructor(
    private readonly root: string,
    private readonly publish: (update: AppStylesheetUpdate) => void,
    private readonly reportError: (message: string) => void,
    private readonly compilerFactory: () => SassCompiler = () => new SassWorkerCompiler()
  ) {}

  start(): Promise<void> {
    if (this.closed) return Promise.resolve();
    return this.starting ??= this.initialize().catch(error => this.error(error));
  }

  private async initialize(): Promise<void> {
    const source = path.join(this.root, "src/renderer/src/styles/main.scss");
    try { this.entry = await realpath(source); } catch { return; }
    const assets = path.join(this.root, "out/renderer/assets");
    for (const name of await readdir(assets)) {
      if (!name.endsWith(".css") || name === "vela-live.css") continue;
      const file = path.join(assets, name);
      const css = await readFile(file, "utf8");
      const start = css.indexOf(APP_STYLES_START);
      const end = css.indexOf(APP_STYLES_END);
      if (start < 0 || end <= start) continue;
      this.before = css.slice(0, start);
      this.after = css.slice(end + APP_STYLES_END.length).replace(/\/\*# sourceMappingURL=.*?\*\//g, "");
      this.originalURL = pathToFileURL(file).href;
      break;
    }
    if (!this.originalURL) throw new Error("Rebuild Vela to enable live app stylesheet saves");
    if (this.closed) return;
    this.output = path.join(assets, "vela-live.css");
    this.compiler = this.compilerFactory();
    this.watcher = watch(path.dirname(this.entry), { ignoreInitial: true, followSymlinks: false, usePolling: true, interval: 200 });
    this.watcher.on("all", (_event, file) => { if (/\.(scss|sass|css)$/i.test(file)) this.schedule(); });
    this.watcher.on("error", error => this.error(error));
    await new Promise<void>(resolve => { this.ready = resolve; this.watcher!.once("ready", resolve); });
    this.schedule();
  }

  publishCurrent(): void { if (!this.closed && this.current) this.publish(this.current); }

  private schedule(): void {
    if (this.closed) return;
    this.dirty = true;
    clearTimeout(this.timer);
    this.timer = setTimeout(() => { void this.drain(); }, 100);
  }

  private async drain(): Promise<void> {
    if (this.running || this.closed) return;
    this.running = (async () => {
      while (this.dirty && !this.closed) {
        this.dirty = false;
        try {
          const result = await this.compiler!.compile(this.entry, { style: "expanded", sourceMap: true, loadPaths: [this.root] });
          if (this.closed || this.dirty) continue;
          const revision = ++this.revision;
          const compiled = compiledStyles(result, this.entry, this.output, this.before, this.after, revision);
          const map = `${this.output}.map`;
          await writeFile(`${map}.tmp`, compiled.map!.toString());
          await writeFile(`${this.output}.tmp`, compiled.css);
          if (this.closed || this.dirty) continue;
          await rename(`${map}.tmp`, map);
          await rename(`${this.output}.tmp`, this.output);
          if (this.closed || this.dirty) continue;
          this.current = { originalURL: this.originalURL, url: pathToFileURL(this.output).href, revision };
          this.publishCurrent();
        } catch (error) {
          if (!this.closed && !this.dirty) this.error(error);
          await this.compiler?.close();
          if (!this.closed) this.compiler = this.compilerFactory();
        }
      }
    })().finally(() => { this.running = undefined; });
    await this.running;
  }

  private error(error: unknown): void {
    if (!this.closed) this.reportError(`App styles: ${error instanceof Error ? error.message : String(error)}`);
  }

  async stop(): Promise<void> {
    this.closed = true;
    clearTimeout(this.timer);
    this.ready?.();
    await this.watcher?.close();
    await this.compiler?.close();
    await this.starting;
    await this.running;
  }
}
