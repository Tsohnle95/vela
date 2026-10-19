import { describe, expect, it } from "vitest";
import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, writeFile, unlink, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import ts from "typescript";
import { analyzeRepository, countCodeLines, dependencyCycles, duplicateGroups, functionFingerprint, functionIdentity, functionMetrics, importSpecifiers, ipcAuthorityReferences, ownershipRoutes, sourceCategory } from "./architecture-analysis.mjs";

describe("architecture measurement", () => {
  it("counts physical token-bearing lines without counting comments or changing string semantics", () => {
    const source = `// only a comment
const url = "https://example.com/*text*/"; // trailing comment
/* several
 * comment lines */

const template = \`first
// retained template content

last\`;
const pattern = /https?:\\/\\//;
`;
    expect(countCodeLines(source, "example.ts")).toBe(5);
    expect(countCodeLines("const a = 1; const b = 2;\n", "example.ts")).toBe(1);
  });

  it("counts JSX and handwritten declarations as production code", () => {
    expect(countCodeLines("const node = <div>\n  text\n  {/* comment */}\n</div>;\n", "example.tsx")).toBe(4);
    expect(sourceCategory("src/renderer/src/global.d.ts")).toBe("production");
    expect(sourceCategory("src/renderer/src/opencode-file/default-options.ts")).toBe("production");
    expect(sourceCategory("src/renderer/src/styles/opencode/theme.scss")).toBe("pinned-upstream-styles");
    expect(sourceCategory("src/renderer/src/large-session-fixture.ts")).toBe("tests-and-fixtures");
    expect(sourceCategory("src/main/fixtures/events.ts")).toBe("tests-and-fixtures");
  });

  it("excludes JSDoc tag identifiers from production and function token lines", () => {
    const text = "/** docs\n * @param input value\n * @returns result\n */\nexport function work(input: number) { return input; }\n";
    expect(countCodeLines(text, "example.ts")).toBe(1);
    const source = ts.createSourceFile("example.ts", text, ts.ScriptTarget.Latest, true);
    expect(functionMetrics(source.statements[0], source).codeLines).toBe(1);
  });

  it("removes style comments while preserving quoted text and unquoted URL schemes", () => {
    const source = `// ignored
.image {
  background: url(https://example.com/image.png);
  content: "/*literal*/ // literal";
  /* ignored
     ignored */
}
`;
    expect(countCodeLines(source, "example.scss")).toBe(4);
    expect(countCodeLines(".image { content: '// literal'; } /* ignored */", "example.css")).toBe(1);
  });

  it("reports nesting and complexity per function without attributing nested closure branches to its parent", () => {
    const source = ts.createSourceFile("example.ts", `function outer() {
  if (ready && active) {
    for (const item of items) {
      if (item) return item;
    }
  }
  const nested = () => { if (other) return other; };
}`, ts.ScriptTarget.Latest, true);
    const declaration = source.statements[0] as ts.FunctionDeclaration;
    expect(functionMetrics(declaration, source)).toMatchObject({ codeLines: 8, nesting: 3, cyclomatic: 5 });
  });

  it("distinguishes cycles involving explicit type imports from value-import cycles", () => {
    const edges = [{ from: "a", to: "b", typeOnly: false }, { from: "b", to: "a", typeOnly: true }];
    expect(dependencyCycles(["a", "b"], edges, false)).toEqual([]);
    expect(dependencyCycles(["a", "b"], edges, true)).toEqual([["a", "b"]]);
    expect(dependencyCycles(["a", "b"], [...edges, { from: "b", to: "a", typeOnly: false }], false)).toEqual([["a", "b"]]);
  });

  it("discovers reexports, explicit type imports, dynamic imports and require edges", () => {
    const source = ts.createSourceFile("imports.ts", `import type { A } from "./a";
import { type B } from "./b";
export { value } from "./c";
export type { D } from "./d";
const dynamic = import("./e");
const required = require("./f");`, ts.ScriptTarget.Latest, true);
    expect(importSpecifiers(source)).toEqual([
      { specifier: "./a", typeOnly: true }, { specifier: "./b", typeOnly: true },
      { specifier: "./c", typeOnly: false }, { specifier: "./d", typeOnly: true },
      { specifier: "./e", typeOnly: false }, { specifier: "./f", typeOnly: false }
    ]);
  });

  it("recognizes specifier-level type reexports and mixed value exports", () => {
    const source = ts.createSourceFile("exports.ts", `export { type A, type B } from "./a";
export { type A, value } from "./b";`, ts.ScriptTarget.Latest, true);
    expect(importSpecifiers(source)).toEqual([{ specifier: "./a", typeOnly: true }, { specifier: "./b", typeOnly: false }]);
  });

  it("discovers constant template loaders, import options and import-equals dependencies", () => {
    const source = ts.createSourceFile("imports.ts", 'const a = import(`node:fs`); const b = require(`./private`); const c = import("./child", { with: { type: "json" } }); import runtime = require("./runtime");', ts.ScriptTarget.Latest, true);
    expect(importSpecifiers(source)).toEqual([
      { specifier: "node:fs", typeOnly: false }, { specifier: "./private", typeOnly: false },
      { specifier: "./child", typeOnly: false }, { specifier: "./runtime", typeOnly: false }
    ]);
    expect(ipcAuthorityReferences(ts.createSourceFile("src/main/ipc/files.ts", 'const e = require(`electron`);', ts.ScriptTarget.Latest, true))).toMatchObject([{ kind: "ipc-domain-electron-import" }]);
  });

  it("reports computed loaders instead of silently dropping them", () => {
    const source = ts.createSourceFile("imports.ts", 'const a = import(name);\nconst b = require(`./${name}`);', ts.ScriptTarget.Latest, true);
    expect(importSpecifiers(source)).toEqual([{ computed: "import(name)", line: 1 }, { computed: "require(`./${name}`)", line: 2 }]);
  });

  it("derives named routes from the existing canonical ownership table only", () => {
    const doc = 'Unrelated `src/main/unowned.ts`\n## Implementation ownership and fitness\n| Responsibility | Entry point and private owner | Dependency boundary |\n| Runtime | `src/main/opencode.ts`; private `src/main/runtimes/` | Do not own renderer state |\n## Other\n| Ignored | `src/main/ignored.ts` | |';
    expect(ownershipRoutes(doc)).toEqual([{ owner: "Runtime", path: "src/main/opencode.ts" }, { owner: "Runtime", path: "src/main/runtimes/" }]);
    expect(ownershipRoutes("no canonical table")).toEqual([]);
  });

  it("detects exact function-body clones with renamed declarations and changed comment trivia", () => {
    const fingerprint = (name: string, comment: string) => {
      const source = ts.createSourceFile("clone.ts", `function ${name}(value: number) { ${comment}
        return value > 10 ? value + 1 : value - 1;
      }`, ts.ScriptTarget.Latest, true);
      return functionFingerprint(source.statements[0] as ts.FunctionDeclaration, source);
    };
    const first = fingerprint("first", "// comment");
    const second = fingerprint("second", "/* other comment */");
    expect(second).toEqual(first);
    expect(duplicateGroups([{ ...first, file: "a.ts", line: 1, codeLines: 22, name: "first" },
      { ...second, file: "b.ts", line: 1, codeLines: 22, name: "second" }])).toHaveLength(1);
  });

  it("anchors IPC callbacks by channel so deletion or reordering cannot transfer their budgets", () => {
    const identities = (text: string) => {
      const source = ts.createSourceFile("ipc.ts", text, ts.ScriptTarget.Latest, true);
      return source.statements.map((statement) => functionIdentity(((statement as ts.ExpressionStatement).expression as ts.CallExpression).arguments[1], source));
    };
    const first = 'handleTrusted("shell:a", () => {});';
    const second = 'handleTrusted("shell:b", () => {});';
    expect(identities(first + second)).toEqual(["handleTrusted callback:shell:a", "handleTrusted callback:shell:b"]);
    expect(identities(second + first)).toEqual(["handleTrusted callback:shell:b", "handleTrusted callback:shell:a"]);
    expect(identities(second)).toEqual(["handleTrusted callback:shell:b"]);
  });

  it("anchors JSX callbacks by element and attribute across deletion and reordering", () => {
    const identities = (elements: string) => {
      const source = ts.createSourceFile("composer.tsx", `function Composer() { return <>${elements}</>; }`, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
      const values: string[] = [];
      const visit = (node: ts.Node) => { if (ts.isArrowFunction(node)) values.push(functionIdentity(node, source)); ts.forEachChild(node, visit); };
      visit(source);
      return values;
    };
    const first = '<input onKeyDown={() => {}} />';
    const second = '<textarea onKeyDown={() => {}} />';
    expect(identities(first + second)).toEqual(["Composer/input.onKeyDown callback", "Composer/textarea.onKeyDown callback"]);
    expect(identities(second + first)).toEqual(["Composer/textarea.onKeyDown callback", "Composer/input.onKeyDown callback"]);
    expect(identities(second)).toEqual(["Composer/textarea.onKeyDown callback"]);
  });

  it("detects raw Electron IPC access and permits only the exact domain registrar event type", () => {
    const analyze = (text: string, file = "src/main/ipc/registration.ts") => ipcAuthorityReferences(ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true)).map((reference: any) => reference.kind);
    expect(analyze('import type { IpcMainInvokeEvent } from "electron";')).toEqual([]);
    expect(analyze('import { type IpcMainInvokeEvent } from "electron";')).toEqual([]);
    expect(analyze('import { ipcMain as raw } from "electron";', "src/main/service.ts")).toEqual(["raw-ipc"]);
    expect(analyze('const raw = (await import("electron"))["ipcMain"];', "src/main/service.ts")).toEqual(["raw-ipc"]);
    expect(analyze('const electron = require("electron");')).toEqual(["ipc-domain-electron-import"]);
    expect(analyze('import electron = require("electron");')).toEqual(["ipc-domain-electron-import"]);
    expect(analyze('const raw = (await import(`electron`))[`ipcMain`];', "src/main/service.ts")).toEqual(["raw-ipc"]);
    expect(analyze('import type { BrowserWindow } from "electron";')).toEqual(["ipc-domain-electron-import"]);
    expect(analyze('import * as electron from "electron";')).toEqual(["ipc-domain-electron-import"]);
  });

  it("measures the current production tree and resolves imports into untracked excluded implementation", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "vela-architecture-test-"));
    try {
      await mkdir(path.join(root, "src/main"), { recursive: true });
      await mkdir(path.join(root, "scripts"));
      await writeFile(path.join(root, "package.json"), "{}");
      await writeFile(path.join(root, "src/main/deleted.ts"), "export const deleted = 1;\n");
      execFileSync("git", ["init", "--quiet"], { cwd: root });
      execFileSync("git", ["add", "."], { cwd: root });
      execFileSync("git", ["-c", "user.name=Architecture Test", "-c", "user.email=architecture@example.invalid", "-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Fixture"], { cwd: root });
      await unlink(path.join(root, "src/main/deleted.ts"));
      await writeFile(path.join(root, "src/main/new.ts"), 'import { hidden } from "../../scripts/hidden";\nexport const value = hidden;\n');
      await writeFile(path.join(root, "scripts/hidden.ts"), "export const hidden = 1;\n");
      const result = await analyzeRepository(root, { fast: true });
      expect(result.production.inventory.map((file: any) => file.path)).toEqual(["src/main/new.ts"]);
      expect(result.production.loc).toBe(2);
      expect(result.dependencies).toContainEqual({ from: "src/main/new.ts", to: "scripts/hidden.ts", typeOnly: false });
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
