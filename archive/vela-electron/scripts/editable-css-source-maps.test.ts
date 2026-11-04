// @vitest-environment node
import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { build } from "vite";
import { SourceMapConsumer } from "source-map-js";
import { editableCssSourceMaps, canonicalModuleId } from "./editable-css-source-maps";
import { resolveAppSource } from "../src/main/source-resolver";

const roots: string[] = [];
afterEach(async () => { await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true }))); });

describe("canonicalModuleId", () => {
  it("matches a Vite id against native realpath output on Windows", () => {
    expect(canonicalModuleId("D:/a/Vela/main.scss", true)).toBe(canonicalModuleId("D:\\a\\Vela\\main.scss", true));
    expect(canonicalModuleId("/d:/a/vela/MAIN.scss", true)).toBe(canonicalModuleId("D:\\a\\Vela\\main.scss", true));
  });

  it("keeps POSIX ids case-sensitive", () => {
    expect(canonicalModuleId("/Users/a/Main.scss", false)).toBe("/Users/a/Main.scss");
    expect(canonicalModuleId("/Users/a/Main.scss", false)).not.toBe(canonicalModuleId("/Users/a/main.scss", false));
  });
});

describe("editable compiled stylesheets", () => {
  it("maps extracted rules and declarations back to their owning Sass partial after concatenation and asset rewriting", async () => {
    const root = await realpath(await mkdtemp(path.join(tmpdir(), "vela-css-map-")));
    roots.push(root);
    await mkdir(path.join(root, "styles"));
    await writeFile(path.join(root, "index.html"), '<script type="module" src="/main.js"></script>');
    await writeFile(path.join(root, "main.js"), 'import "./vendor.css"; import "./styles/main.scss";');
    await writeFile(path.join(root, "vendor.css"), '.vendor { color: red; }');
    await writeFile(path.join(root, "pixel.svg"), '<svg xmlns="http://www.w3.org/2000/svg"/>');
    await writeFile(path.join(root, "styles/main.scss"), '@use "panel";\n.after { color: blue; }');
    const partial = path.join(root, "styles/_panel.scss");
    await writeFile(partial, '$color: purple;\n.panel {\n  color: $color;\n  background: url("../pixel.svg");\n  &:hover { opacity: .5; }\n}\n@media (width > 200px) {\n  .panel { display: flex; }\n}');
    const result = await build({
      configFile: false, root, logLevel: "silent", css: { devSourcemap: true },
      plugins: [editableCssSourceMaps(path.join(root, "styles/main.scss"))],
      build: { write: false, minify: false, cssMinify: false, assetsInlineLimit: 0 }
    });
    if (Array.isArray(result) || !("output" in result)) throw new Error("Expected one build output");
    const css = result.output.find(asset => asset.type === "asset" && asset.fileName.endsWith(".css"));
    const map = result.output.find(asset => asset.type === "asset" && asset.fileName.endsWith(".css.map"));
    if (css?.type !== "asset" || map?.type !== "asset") throw new Error("Missing CSS/map assets");
    const text = String(css.source);
    expect(text).toContain(".vendor");
    expect(text).not.toContain("vela-css-source:");
    expect(text).toContain(`sourceMappingURL=${path.basename(map.fileName)}`);
    const consumer = new SourceMapConsumer(JSON.parse(String(map.source)));
    for (const [needle, expectedLine] of [[".panel {", 2], ["color: purple", 3], ["background:", 4], [".panel:hover", 5], ["display: flex", 8]] as const) {
      const offset = text.indexOf(needle);
      expect(offset).toBeGreaterThan(-1);
      const prefix = text.slice(0, offset).split("\n");
      const original = consumer.originalPositionFor({ line: prefix.length, column: prefix.at(-1)!.length });
      const source = new URL(original.source, pathToFileURL(path.join(root, "dist", map.fileName))).href;
      expect(fileURLToPath(source)).toBe(partial);
      expect(original.line).toBe(expectedLine);
      await expect(resolveAppSource(root, source, `${source}:${original.line}`)).resolves.toBe(partial);
    }
    expect(await readFile(partial, "utf8")).toContain("$color: purple");
  });
});
