// Read-only execution of the reference's planning/compiler utilities against
// disposable fixtures. No renderer, project scripts or real workspaces run.
import { build } from "../tools/preview/node_modules/esbuild/lib/main.js";
import { sassPlan } from "../tools/preview/sass-plan.mjs";
import { compileWorkspace } from "../tools/preview/sass-compile.mjs";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { randomUUID } from "node:crypto";
import { fileURLToPath, pathToFileURL } from "node:url";

const checkout = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const reference = path.resolve(process.argv[2] ?? path.join(checkout, "../vela"));
const utility = path.join(checkout, "tools/preview", `.reference-sass-${randomUUID()}.mjs`);
const fixtures = await mkdtemp(path.join(os.tmpdir(), "vela-sass-reference-"));
let plans = 0;
let outputs = 0;
try {
  const result = await build({
    stdin: { contents: `export {sassPlan} from ${JSON.stringify(path.join(reference, "src/main/sass-config.ts"))}; export {compileSass} from ${JSON.stringify(path.join(reference, "src/main/sass-compile.ts"))};`, resolveDir: checkout },
    bundle: true, platform: "node", format: "esm", target: "node22", write: false,
    external: ["sass", "json5"], alias: { "@shared/types": path.join(reference, "src/shared/types.ts") },
  });
  await writeFile(utility, result.outputFiles[0].contents, { flag: "wx" });
  const source = await import(pathToFileURL(utility).href);
  const cases = [
    { name: "default", files: { "nested/main.scss": '@use "ink"; @warn "kept warning"; .a { color: ink.$ink; }', "nested/_ink.scss": "$ink: #123456;", ".styles/hidden.sass": ".hidden\n  color: blue", "dist/ignored.scss": "invalid", ".orbit-recovery/ignored.scss": "invalid" } },
    { name: "explicit", files: { "src/界.scss": '@use "sass:math"; .a { width: math.div(10px, 2); }', "vela-sass.json": "{entries:{src:'public'},style:'compressed',sourceMap:true,}" } },
    { name: "live", files: { "src/main.scss": ".main { color: red; }", "src/skip.scss": ".skip {}", ".vscode/settings.json": JSON.stringify({ "liveSassCompile.settings.excludeList": ["src/skip.scss"], "liveSassCompile.settings.formats": [{ format: "expanded", savePath: "/css" }, { format: "compressed", extensionName: ".min.css", savePath: "~/compiled" }], "liveSassCompile.settings.generateMap": true }) } },
    { name: "package", files: { "src/main.scss": ".main { color: red; }", "package.json": JSON.stringify({ scripts: { styles: "sass src:public --style=compressed --no-source-map && this-must-never-execute" } }) } },
    { name: "legacy-mira", files: { "main.scss": ".main {}", "mira-sass.json": "{style:'compressed'}" } },
    { name: "legacy-orbit", files: { "main.scss": ".main {}", "orbit-sass.json": "{style:'expanded'}" } },
    { name: "escape", files: { "main.scss": ".main {}", "vela-sass.json": "{entries:{'main.scss':'../escape.css'}}" } },
    { name: "collision", files: { "src/main.scss": ".main {}", "vela-sass.json": "{entries:{src:'public','src/main.scss':'public/main.css'}}" } },
    { name: "compile-error", files: { "main.scss": '@use "missing";' } },
  ];
  for (const fixture of cases) {
    const root = path.join(fixtures, fixture.name);
    for (const [name, content] of Object.entries(fixture.files)) {
      const file = path.join(root, name);
      await mkdir(path.dirname(file), { recursive: true });
      await writeFile(file, content);
    }
    const capture = async (plan) => { try { return await plan(root); } catch (error) { return { error: String(error) }; } };
    const expected = await capture(source.sassPlan);
    const actual = await capture(sassPlan);
    assert.deepEqual(actual, expected, `${fixture.name}: output plan`);
    plans++;
    if (expected.error) continue;
    const compiled = await compileWorkspace(root);
    assert.equal(compiled.outputs.length, expected.outputs.length);
    for (let index = 0; index < expected.outputs.length; index++) {
      const output = expected.outputs[index];
      const actual = compiled.outputs[index];
      let result;
      try { result = source.compileSass(output.source, output.options); }
      catch (error) { assert.equal(actual.error, String(error), `${fixture.name}: compiler error`); outputs++; continue; }
      let css = result.css;
      let sourceMap;
      if (result.sourceMap) {
        const map = JSON.parse(result.sourceMap);
        map.sources = map.sources.map((source) => source.startsWith("file:") ? encodeURI(path.relative(path.dirname(output.output), fileURLToPath(source)).split(path.sep).join("/")) : source);
        sourceMap = JSON.stringify(map);
        css += `\n/*# sourceMappingURL=${encodeURIComponent(path.basename(output.output))}.map */\n`;
      } else { css += "\n"; }
      assert.equal(actual.css, css, `${fixture.name}: CSS bytes`);
      assert.equal(actual.sourceMap, sourceMap, `${fixture.name}: map bytes`);
      assert.deepEqual(actual.warnings, result.warnings, `${fixture.name}: warnings`);
      outputs++;
    }
  }
  process.stdout.write(JSON.stringify({ plans, outputs, matching: true, reference, compiler: "Dart Sass 1.102.0" }) + "\n");
} finally {
  await rm(utility, { force: true });
  await rm(fixtures, { recursive: true, force: true });
}
