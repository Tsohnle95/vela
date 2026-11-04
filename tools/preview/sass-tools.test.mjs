import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, readFile, rm, symlink } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { sassPlan } from "./sass-plan.mjs";
import { compileWorkspace } from "./sass-compile.mjs";

async function fixture(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "vela-sass-tools-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}
async function put(root, name, content) {
  await mkdir(path.dirname(path.join(root, name)), { recursive: true });
  await writeFile(path.join(root, name), content);
}

test("default planning skips dependencies/partials and compiles modern Dart Sass", async (t) => {
  const root = await fixture(t);
  await put(root, "nested/_ink.scss", "$ink: #123456;");
  await put(root, "nested/main.scss", '@use "sass:math"; @use "ink"; .a { width: math.div(10px, 2); color: ink.$ink; }');
  await put(root, ".styles/hidden.sass", ".hidden\n  color: red");
  for (const ignored of ["node_modules", "dist", ".vela-recovery", ".mira-recovery", ".openshell-recovery", ".orbit-recovery", "vendor"]) {
    await put(root, `${ignored}/ignored.scss`, "this is deliberately invalid");
  }
  const result = await compileWorkspace(root);
  assert.equal(result.convention, "CSS beside source");
  assert.equal(result.outputs.length, 2);
  const nested = result.outputs.find((output) => output.source.endsWith("main.scss"));
  assert.match(nested.css, /width: 5px/);
  assert.match(nested.css, /color: #123456/);
  assert.equal(nested.error, undefined);
});

test("explicit JSON5 mappings preserve compressed style, load paths and relative maps", async (t) => {
  const root = await fixture(t);
  await put(root, "include/_ink.scss", "$ink: #123456;");
  await put(root, "src/界.scss", '@use "ink"; .a { color: ink.$ink; }');
  await put(root, "vela-sass.json", `{
    // Workspace data is parsed, never evaluated.
    entries: {'src': 'public'}, style: 'compressed', loadPaths: ['include'], sourceMap: true,
  }`);
  const result = await compileWorkspace(root);
  assert.equal(result.convention, "vela-sass.json");
  const output = result.outputs[0];
  assert.equal(output.output, path.join(root, "public/界.css"));
  assert.match(output.css, /^\.a\{color:#123456\}/);
  assert.match(output.css, /sourceMappingURL=%E7%95%8C.css.map/);
  const map = JSON.parse(output.sourceMap);
  assert.ok(map.sources.includes("../src/%E7%95%8C.scss"));
  assert.ok(map.sources.every((source) => !source.startsWith("file:")));
});

test("Live Sass formats, include/exclude globs and replacement output directories", async (t) => {
  const root = await fixture(t);
  await put(root, "src/main.scss", ".main { color: red; }");
  await put(root, "src/skip.scss", ".skip { color: blue; }");
  await put(root, "other/unused.scss", ".unused {}");
  await put(root, ".vscode/settings.json", JSON.stringify({
    "liveSassCompile.settings.includeItems": ["src/**"],
    "liveSassCompile.settings.excludeList": ["src/skip.scss"],
    "liveSassCompile.settings.formats": [
      { format: "expanded", savePath: "/css" },
      { format: "compressed", extensionName: ".min.css", savePathReplacementPairs: { src: "min" } },
    ],
  }));
  const result = await compileWorkspace(root);
  assert.equal(result.convention, "Live Sass workspace settings");
  assert.deepEqual(result.outputs.map((output) => path.relative(root, output.output)), [path.join("css", "main.css"), path.join("min", "main.min.css")]);
  assert.match(result.outputs[1].css, /^\.main\{color:red\}/);
});

test("literal package Sass mappings never execute the surrounding shell script", async (t) => {
  const root = await fixture(t);
  await put(root, "src/main.scss", ".a { color: red; }");
  await put(root, "package.json", JSON.stringify({ scripts: { sass: "sass src:public --style=compressed --no-source-map && touch SHELL_MUST_NOT_RUN" } }));
  const result = await compileWorkspace(root);
  assert.equal(result.convention, "literal Sass package script mappings");
  assert.equal(result.outputs[0].output, path.join(root, "public/main.css"));
  assert.equal(result.outputs[0].css, ".a{color:red}\n");
  await assert.rejects(readFile(path.join(root, "SHELL_MUST_NOT_RUN")), { code: "ENOENT" });
});

test("invalid mappings, output collisions and oversized configuration fail before compiling", async (t) => {
  const root = await fixture(t);
  await put(root, "src/main.scss", ".a {}");
  await put(root, "vela-sass.json", "{entries:{src:'../escape'}}");
  await assert.rejects(sassPlan(root), /escapes the workspace/);
  await put(root, "vela-sass.json", "{entries:{src:'public','src/main.scss':'public/main.css'}}");
  await assert.rejects(sassPlan(root), /collision/);
  await put(root, "vela-sass.json", " ".repeat(1_048_577));
  await assert.rejects(sassPlan(root), /safely read/);
});

test("entry failures are separate from successful outputs and retain compiler warnings", async (t) => {
  const root = await fixture(t);
  await put(root, "good.scss", '@warn "visible warning"; .good { color: red; }');
  await put(root, "bad.scss", '@use "missing";');
  const result = await compileWorkspace(root);
  assert.equal(result.outputs.length, 2);
  assert.match(result.outputs.find((output) => output.source.endsWith("bad.scss")).error, /stylesheet to import/);
  const good = result.outputs.find((output) => output.source.endsWith("good.scss"));
  assert.deepEqual(good.warnings, ["visible warning"]);
  assert.match(good.css, /color: red/);
});

test("legacy explicit config names remain supported", async (t) => {
  const root = await fixture(t);
  await put(root, "main.scss", ".main { color: red; }");
  await put(root, "mira-sass.json", "{style:'compressed'}");
  assert.equal((await compileWorkspace(root)).convention, "mira-sass.json");
  await put(root, "vela-sass.json", "{style:'expanded'}");
  assert.equal((await compileWorkspace(root)).convention, "vela-sass.json");
});

test("source and configuration-directory symlinks are not followed", { skip: process.platform === "win32" }, async (t) => {
  const root = await fixture(t);
  const outside = await fixture(t);
  await put(outside, "outside.scss", ".outside {}");
  await symlink(path.join(outside, "outside.scss"), path.join(root, "link.scss"));
  assert.deepEqual((await sassPlan(root)).outputs, []);
  await put(outside, "settings.json", "{}");
  await symlink(outside, path.join(root, ".vscode"));
  await assert.rejects(sassPlan(root), /configuration directory/);
});
