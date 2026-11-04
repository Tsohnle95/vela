import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { SourceMapConsumer } from "source-map-js";
import { AppStylesController } from "./app-styles";
import { compileSass } from "./sass-compile";
import { APP_STYLES_START, APP_STYLES_END, type AppStylesheetUpdate } from "../shared/app-styles";

let root: string;
let controller: AppStylesController | undefined;
afterEach(async () => { await controller?.stop(); controller = undefined; if (root) await rm(root, { recursive: true, force: true }); });

async function fixture() {
  root = await realpath(await mkdtemp(path.join(tmpdir(), "vela-app-styles-")));
  const styles = path.join(root, "src/renderer/src/styles");
  const assets = path.join(root, "out/renderer/assets");
  await mkdir(styles, { recursive: true });
  await mkdir(assets, { recursive: true });
  await writeFile(path.join(styles, "main.scss"), '@use "panel";');
  const partial = path.join(styles, "_panel.scss");
  await writeFile(partial, '.panel { padding: 12px; background: url("../assets/font file.woff2"); }');
  await writeFile(path.join(assets, "index-fixture.css"), `.vendor { background: url("./vendor.png"); }\n${APP_STYLES_START}\n.panel { padding: 1px; margin: 7px; }\n${APP_STYLES_END}\n/*# sourceMappingURL=index-fixture.css.map */`);
  const updates: AppStylesheetUpdate[] = [];
  const errors: string[] = [];
  const compiler = { compile: async (source: string, options: Parameters<typeof compileSass>[1]) => compileSass(source, options), close: async () => {} };
  controller = new AppStylesController(root, update => updates.push(update), error => errors.push(error), () => compiler);
  await controller.start();
  return { partial, assets, updates, errors };
}

describe("compiled app stylesheet saves", () => {
  it("recompiles saved partials, removes stale declarations, preserves vendor assets and maps rules back to source", async () => {
    const f = await fixture();
    await expect.poll(() => f.updates.length).toBe(1);
    const output = fileURLToPath(f.updates[0].url);
    let css = await readFile(output, "utf8");
    expect(css).toContain("padding: 12px");
    expect(css).not.toContain("margin: 7px");
    expect(css).toContain('url("./vendor.png")');
    expect(css).toContain(pathToFileURL(path.join(root, "src/renderer/src/assets/font file.woff2")).href);
    await writeFile(f.partial, '.panel { padding: 39px; }');
    await expect.poll(() => f.updates.length).toBe(2);
    css = await readFile(output, "utf8");
    expect(css).toContain("padding: 39px");
    expect(css).not.toContain("padding: 12px");
    const map = new SourceMapConsumer(JSON.parse(await readFile(`${output}.map`, "utf8")));
    const prefix = css.slice(0, css.indexOf(".panel")).split("\n");
    const source = map.originalPositionFor({ line: prefix.length, column: prefix.at(-1)!.length });
    expect(fileURLToPath(new URL(source.source, pathToFileURL(`${output}.map`)))).toBe(f.partial);
    expect(source.line).toBe(1);
    expect(f.errors).toEqual([]);
  });

  it("preserves the last valid CSS on syntax errors, recovers, and stops publishing after close", async () => {
    const f = await fixture();
    await expect.poll(() => f.updates.length).toBe(1);
    const output = fileURLToPath(f.updates[0].url);
    const valid = await readFile(output, "utf8");
    await writeFile(f.partial, '.panel { padding: ; invalid');
    await expect.poll(() => f.errors.length).toBe(1);
    expect(await readFile(output, "utf8")).toBe(valid);
    expect(f.updates).toHaveLength(1);
    await writeFile(f.partial, '.panel { padding: 20px; }');
    await expect.poll(() => f.updates.length).toBe(2);
    await controller!.stop();
    await writeFile(f.partial, '.panel { padding: 50px; }');
    controller!.publishCurrent();
    expect(f.updates).toHaveLength(2);
  });

  it("does not publish an in-flight compilation after shutdown", async () => {
    const f = await fixture();
    await expect.poll(() => f.updates.length).toBe(1);
    await controller!.stop();
    let release: (() => void) | undefined;
    controller = new AppStylesController(root, update => f.updates.push(update), error => f.errors.push(error), () => ({
      compile: async (source, options) => {
        const result = compileSass(source, options);
        return new Promise(resolve => { release = () => resolve(result); });
      },
      close: async () => { release?.(); }
    }));
    await controller.start();
    await expect.poll(() => typeof release).toBe("function");
    await controller.stop();
    expect(f.updates).toHaveLength(1);
    expect(f.errors).toEqual([]);
  });

  it("does nothing for an installed package without application sources", async () => {
    root = await mkdtemp(path.join(tmpdir(), "vela-app-styles-package-"));
    const updates: unknown[] = [];
    const errors: string[] = [];
    controller = new AppStylesController(root, update => updates.push(update), error => errors.push(error));
    await controller.start();
    expect(updates).toEqual([]);
    expect(errors).toEqual([]);
  });
});
