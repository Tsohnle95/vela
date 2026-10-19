// @vitest-environment node
import { mkdtemp, mkdir, writeFile, readFile, rm, symlink, access } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, SassWatchState } from "@shared/types";
import { SassPreferences } from "./sass-preferences";
import { sassPlan } from "./sass-config";
import { SassWatchManager } from "./sass-watch";
import { compileSass } from "./sass-compile";
import type { SassCompiler } from "./sass-compiler";

let root: string;
let manager: SassWatchManager;
let preferences: SassPreferences;
const session = (directory: string): SessionInfo => ({ id: randomUUID(), directory, workspace: { id: randomUUID(), generation: 1 } });
const inline = (): SassCompiler => ({ compile: async (file, options) => compileSass(file, options), close: async () => {} });
async function file(relative: string, content: string): Promise<void> {
  const absolute = path.join(root, relative); await mkdir(path.dirname(absolute), { recursive: true }); await writeFile(absolute, content);
}
async function css(relative: string, contains: string): Promise<void> {
  await vi.waitFor(async () => expect(await readFile(path.join(root, relative), "utf8")).toContain(contains), { timeout: 5000 });
}
const exists = (file: string): Promise<boolean> => access(file).then(() => true, () => false);
beforeEach(async () => {
  root = await mkdtemp(path.join(tmpdir(), "vela-sass-"));
  preferences = new SassPreferences(path.join(root, "settings", "sass-watch.json"));
  manager = new SassWatchManager(preferences, () => {}, inline);
});
afterEach(async () => { await manager.stopAll(); await rm(root, { recursive: true, force: true }); });

describe("workspace Sass watching", () => {
  it("writes scss and indented sass beside their sources and never outputs partials or excluded directories", async () => {
    await file("one/css/main.scss", "@use './tone'; body { color: tone.$color; }");
    await file("one/css/_tone.scss", "$color: red;");
    await file("one/nested/indented.sass", "h2\n  color: blue\n");
    for (const dir of ["node_modules", ".git", "dist", "build"]) await file(`one/${dir}/ignored.scss`, ".ignored { color: red; }");
    const owner = session(path.join(root, "one"));
    expect((await manager.register(owner)).status).toBe("off");
    expect((await manager.setEnabled(owner.workspace, true)).status).toBe("starting");
    await css("one/css/main.css", "color: red");
    await css("one/nested/indented.css", "color: blue");
    expect(await exists(path.join(root, "one/css/_tone.css"))).toBe(false);
    for (const dir of ["node_modules", ".git", "dist", "build"]) expect(await exists(path.join(root, `one/${dir}/ignored.css`))).toBe(false);
    await vi.waitFor(() => expect(manager.state(owner.workspace).status).toBe("on"));
  });

  it("recompiles entrypoints when an imported partial changes", async () => {
    await file("one/main.scss", "@use './tone'; body { color: tone.$color; }");
    await file("one/_tone.scss", "$color: red;");
    const owner = session(path.join(root, "one")); await manager.register(owner); await manager.setEnabled(owner.workspace, true);
    await css("one/main.css", "color: red");
    await file("one/_tone.scss", "$color: green;");
    await css("one/main.css", "color: green");
  });

  it("isolates roots, stops on close, and restores only the enabled root on reopen", async () => {
    await file("one/main.scss", "body { color: red; }"); await file("two/main.scss", "body { color: blue; }");
    const one = session(path.join(root, "one")), two = session(path.join(root, "two"));
    await manager.register(one); await manager.register(two); await manager.setEnabled(one.workspace, true);
    await css("one/main.css", "red");
    expect(manager.state(two.workspace).status).toBe("off"); expect(await exists(path.join(root, "two/main.css"))).toBe(false);
    await manager.setEnabled(two.workspace, true); await css("two/main.css", "blue");
    await manager.unregister(one.workspace); await file("one/main.scss", "body { color: green; }");
    await file("two/main.scss", "body { color: yellow; }"); await css("two/main.css", "yellow");
    expect(await readFile(path.join(root, "one/main.css"), "utf8")).toContain("red");
    const reopened = session(one.directory); await manager.register(reopened); await css("one/main.css", "green");
    await manager.setEnabled(reopened.workspace, false); await manager.unregister(reopened.workspace);
    expect((await manager.register(session(one.directory))).enabled).toBe(false);
  });

  it("retains the enabled preference across manager restart without writing an off preference during shutdown", async () => {
    await file("one/main.scss", "body { color: red; }"); const owner = session(path.join(root, "one"));
    await manager.register(owner); await manager.setEnabled(owner.workspace, true); await css("one/main.css", "red");
    await manager.stopAll();
    manager = new SassWatchManager(new SassPreferences(path.join(root, "settings", "sass-watch.json")), () => {}, inline);
    const reopened = session(owner.directory); expect((await manager.register(reopened)).enabled).toBe(true);
    await css("one/main.css", "red");
  });

  it("serializes rapid on/off requests and ignores late compiler results", async () => {
    await file("one/main.scss", "body { color: red; }"); const owner = session(path.join(root, "one"));
    let rejectPending: ((error: Error) => void) | undefined;
    const started = vi.fn();
    await manager.stopAll();
    manager = new SassWatchManager(preferences, () => {}, () => ({
      compile: () => { started(); return new Promise((_resolve, reject) => { rejectPending = reject; }); },
      close: async () => { rejectPending?.(new Error("stopped")); }
    }));
    await manager.register(owner);
    await Promise.all([manager.setEnabled(owner.workspace, true), manager.setEnabled(owner.workspace, false)]);
    expect(manager.state(owner.workspace).status).toBe("off");
    await manager.setEnabled(owner.workspace, true); await vi.waitFor(() => expect(started).toHaveBeenCalled());
    await manager.setEnabled(owner.workspace, false);
    expect(await exists(path.join(root, "one/main.css"))).toBe(false);
    expect(manager.state(owner.workspace).enabled).toBe(false);
  });

  it("shares one compiler for two agent contexts of the same root and stops it after the last closes", async () => {
    await file("one/main.scss", "body { color: red; }");
    const factory = vi.fn(inline); const updates: SassWatchState[] = [];
    await manager.stopAll(); manager = new SassWatchManager(preferences, (state) => updates.push(state), factory);
    const one = session(path.join(root, "one")), same = session(one.directory);
    await manager.register(one); await manager.register(same); await manager.setEnabled(one.workspace, true); await css("one/main.css", "red");
    expect(factory).toHaveBeenCalledTimes(1);
    expect(updates.some((update) => update.workspace.id === same.workspace.id && update.enabled)).toBe(true);
    await manager.unregister(one.workspace); expect(manager.state(same.workspace).enabled).toBe(true);
    await manager.unregister(same.workspace);
    expect(() => manager.state(same.workspace)).toThrow("closed");
  });

  it("surfaces compiler errors, keeps existing CSS, and recovers after a saved correction", async () => {
    await file("one/main.scss", "body { color: red; }"); const owner = session(path.join(root, "one"));
    await manager.register(owner); await manager.setEnabled(owner.workspace, true); await css("one/main.css", "red");
    await file("one/main.scss", "body { color: $undefined; }");
    await vi.waitFor(() => expect(manager.state(owner.workspace).status).toBe("error"));
    expect(manager.state(owner.workspace).logs.some((log) => log.level === "error" && log.message.includes("Undefined variable"))).toBe(true);
    expect(await readFile(path.join(root, "one/main.css"), "utf8")).toContain("red");
    await file("one/main.scss", "body { color: green; }"); await css("one/main.css", "green");
    await vi.waitFor(() => expect(manager.state(owner.workspace).status).toBe("on"));
  });

  it("respects explicit directory output mapping, compressed style and source maps", async () => {
    await file("one/sass/nested/main.scss", "body { color: red; }");
    await file("one/vela-sass.json", JSON.stringify({ entries: { sass: "public/css" }, style: "compressed", sourceMap: true }));
    const owner = session(path.join(root, "one")); await manager.register(owner); await manager.setEnabled(owner.workspace, true);
    await css("one/public/css/nested/main.css", "body{color:red}");
    expect(await exists(path.join(root, "one/public/css/nested/main.css.map"))).toBe(true);
    expect(await exists(path.join(root, "one/sass/nested/main.css"))).toBe(false);
  });

  it("continues to read previous branded Sass configuration files", async () => {
    const previousName = `${String.fromCharCode(109, 105, 114, 97)}-sass.json`;
    await file("one/sass/nested/main.scss", "body { color: red; }");
    await file(path.join("one", previousName), JSON.stringify({ entries: { sass: "public/css" }, style: "compressed" }));
    await expect(sassPlan(path.join(root, "one"))).resolves.toMatchObject({ convention: previousName });
  });

  it("detects Live Sass workspace settings with comments and trailing commas", async () => {
    await file("one/sass/main.scss", "body { color: red; }");
    await file("one/.vscode/settings.json", '{ // Local output convention\n"liveSassCompile.settings.formats": [{"format":"compressed","extensionName":".min.css","savePath":"/css",}],}');
    const owner = session(path.join(root, "one")); await manager.register(owner); await manager.setEnabled(owner.workspace, true);
    await css("one/css/main.min.css", "body{color:red}");
  });

  it("detects literal Sass CLI mappings without executing package scripts", async () => {
    await file("one/sass/main.scss", "body { color: red; }");
    await file("one/package.json", JSON.stringify({ scripts: { sass: "sass sass:public/css --style=compressed; touch must-not-run" } }));
    const owner = session(path.join(root, "one")); await manager.register(owner); await manager.setEnabled(owner.workspace, true);
    await css("one/public/css/main.css", "body{color:red}");
    expect(await exists(path.join(root, "one/must-not-run"))).toBe(false);
  });

  it("rejects escaping configuration and symlink output targets", async () => {
    await file("one/main.scss", "body { color: red; }"); await file("outside.css", "preserved");
    await symlink(path.join(root, "outside.css"), path.join(root, "one/main.css"));
    const owner = session(path.join(root, "one")); await manager.register(owner); await manager.setEnabled(owner.workspace, true);
    await vi.waitFor(() => expect(manager.state(owner.workspace).status).toBe("error"));
    expect(await readFile(path.join(root, "outside.css"), "utf8")).toBe("preserved");
    await file("one/vela-sass.json", JSON.stringify({ entries: { "main.scss": "../outside.css" } }));
    await vi.waitFor(() => expect(manager.state(owner.workspace).logs.some((log) => log.message.includes("escapes"))).toBe(true));
    expect(await readFile(path.join(root, "outside.css"), "utf8")).toBe("preserved");
  });

  it("detects quoted one-to-one CLI output and local load paths", async () => {
    await file("one/src/main.scss", "@use 'colors'; body { color: colors.$tone; }");
    await file("one/include/_colors.scss", "$tone: blue;");
    await file("one/package.json", JSON.stringify({ scripts: { sass: 'sass --load-path=include "src/main.scss" "public/site styles.css" --style compressed --no-source-map' } }));
    const owner = session(path.join(root, "one")); await manager.register(owner); await manager.setEnabled(owner.workspace, true);
    await css("one/public/site styles.css", "body{color:blue}");
    expect(await exists(path.join(root, "one/public/site styles.css.map"))).toBe(false);
  });

  it("does not restore a watcher after its workspace closes during preference loading", async () => {
    await file("one/main.scss", "body { color: red; }"); const owner = session(path.join(root, "one"));
    let finishRestore: ((enabled: boolean) => void) | undefined;
    vi.spyOn(preferences, "enabled").mockImplementationOnce(() => new Promise((resolve) => { finishRestore = resolve; }));
    const factory = vi.fn(inline); await manager.stopAll(); manager = new SassWatchManager(preferences, () => {}, factory);
    const registration = manager.register(owner).catch((error) => error);
    await vi.waitFor(() => expect(finishRestore).toBeDefined());
    const closing = manager.unregister(owner.workspace); finishRestore!(true); await closing;
    expect(await registration).toBeInstanceOf(Error);
    expect(factory).not.toHaveBeenCalled();
    expect(await exists(path.join(root, "one/main.css"))).toBe(false);
  });

  it("can stop an existing watcher after its directory disappears", async () => {
    await file("one/main.scss", "body { color: red; }"); const owner = session(path.join(root, "one"));
    await manager.register(owner); await manager.setEnabled(owner.workspace, true); await css("one/main.css", "red");
    await rm(owner.directory, { recursive: true, force: true });
    const stopped = await manager.setEnabled(owner.workspace, false);
    expect(stopped.enabled).toBe(false); expect(stopped.status).toBe("off");
  });
});
