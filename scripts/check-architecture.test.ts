import { describe, expect, it } from "vitest";
import ts from "typescript";
import { functionIdentity } from "./architecture-analysis.mjs";
import { architectureViolations, ratchetConfiguration } from "./check-architecture.mjs";

const configuration = () => ({
  limits: { module: 800, codeLines: 100, cyclomatic: 20, nesting: 5 }, knownFiles: [], modules: {}, functions: {}, functionExceptions: {},
  edgeExceptions: [], cycleExceptions: [], duplicates: { functions: {}, blocks: {} },
  privateBoundaries: [], pureOwners: []
});
const report = () => ({ production: { files: 0, inventory: [] }, functions: [], dependencies: [], forbiddenEdges: [], externalImports: [], duplication: { functions: [], blocks: [] } });
const checked = (value: any, config = configuration()) => architectureViolations({ ...value,
  ownershipRoutes: value.ownershipRoutes ?? value.production.inventory.map((file: any) => ({ owner: "Fixture owner", path: file.path }))
}, config).map((entry: any) => entry.rule);

describe("architecture fitness checks", () => {
  it("allows frozen AI Elements dependencies but fails tampered or missing pinned content", () => {
    const value = report() as any;
    value.production.inventory.push({ path: "src/renderer/src/ai-elements/adapter.ts", loc: 1 });
    value.dependencies.push({ from: "src/renderer/src/ai-elements/adapter.ts", to: "src/renderer/src/components/ui/trace.tsx", typeOnly: false });
    expect(checked(value)).toEqual([]);
    value.pinnedAiElementDrift = [{ file: "src/renderer/src/components/ui/trace.tsx", expected: "original", actual: "missing" }];
    expect(checked(value)).toEqual(["pinned-ai-elements-integrity"]);
    value.pinnedAiElementDrift[0].actual = "changed";
    expect(checked(value)).toEqual(["pinned-ai-elements-integrity"]);
    value.dependencies[0].to = "src/renderer/src/components/ui/unpinned.tsx";
    expect(checked(value)).toContain("production-scope");
  });

  it("fails new oversized modules and functions, nesting and complexity", () => {
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/new.ts", loc: 801 });
    value.functions.push({ file: "src/main/new.ts", identity: "run#1", codeLines: 101, cyclomatic: 21, nesting: 6 });
    expect(checked(value)).toEqual(["module-size", "function-codeLines", "function-cyclomatic", "function-nesting"]);
  });

  it("does not transfer an old function budget to renamed or moved functions", () => {
    const config = configuration() as any;
    config.functions["src/main/old.ts:run#1"] = { codeLines: 200, cyclomatic: 30, nesting: 6, reason: "Existing frozen debt" };
    const value = report() as any;
    value.functions.push({ file: "src/main/old.ts", identity: "renamed#1", codeLines: 150, cyclomatic: 1, nesting: 0 },
      { file: "src/main/new.ts", identity: "run#1", codeLines: 150, cyclomatic: 1, nesting: 0 });
    expect(checked(value, config)).toEqual(["function-codeLines", "function-codeLines"]);
  });

  it("removes deleted function allowances and only ratchets limits downward", () => {
    const config = configuration() as any;
    config.functions["src/main/old.ts:run#1"] = { codeLines: 200, cyclomatic: 30, nesting: 6, reason: "Existing frozen debt" };
    config.modules["src/main/old.ts"] = { loc: 900, reason: "Existing debt" };
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/old.ts", loc: 850 });
    value.ownershipRoutes = [{ owner: "Fixture owner", path: "src/main/old.ts" }];
    const tightened = ratchetConfiguration(value, config);
    expect(tightened.functions).toEqual({});
    expect(tightened.modules["src/main/old.ts"].loc).toBe(850);
    value.production.inventory[0].loc = 901;
    expect(() => ratchetConfiguration(value, config)).toThrow("Cannot ratchet");
  });

  it("keeps function identities stable when comments or preceding lines move", () => {
    const identity = (text: string) => {
      const source = ts.createSourceFile("x.ts", text, ts.ScriptTarget.Latest, true);
      return functionIdentity(source.statements.find(ts.isFunctionDeclaration)!, source);
    };
    expect(identity("function work() { return 1; }")).toBe(identity("// new comment\n\nfunction work() { return 1; }"));
    expect(identity("function changed() { return 1; }")).not.toBe(identity("function work() { return 1; }"));
  });

  it("rejects forbidden layers, private implementation imports and privileged renderer dependencies", () => {
    const config = configuration();
    config.privateBoundaries.push({ targetPrefix: "src/main/runtimes/deepseek/", owner: "dormant runtime", allowedFrom: ["src/main/runtimes/deepseek/"] });
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/runtimes/deepseek/private.ts", loc: 1 });
    const edge = { from: "src/renderer/src/panel.ts", to: "src/main/runtimes/deepseek/private.ts", typeOnly: false };
    value.dependencies.push(edge); value.forbiddenEdges.push(edge);
    value.externalImports.push({ file: "src/renderer/src/panel.ts", specifier: "node:fs", typeOnly: false },
      { file: "src/main/other.ts", specifier: "@opencode/client", typeOnly: true });
    expect(checked(value, config)).toEqual(["layer-direction", "private-owner", "privileged-import", "sdk-owner"]);
  });

  it("allows only the narrow declaration bridge exception", () => {
    const config = configuration() as any;
    config.edgeExceptions.push({ from: "src/renderer/src/global.d.ts", to: "src/preload/index.ts", typeOnly: true, reason: "Bridge declaration" });
    const value = report() as any;
    value.production.inventory.push({ path: "src/preload/index.ts", loc: 1 });
    const edge = { from: "src/renderer/src/global.d.ts", to: "src/preload/index.ts", typeOnly: true };
    value.dependencies.push(edge); value.forbiddenEdges.push(edge);
    expect(checked(value, config)).toEqual([]);
    value.dependencies[0] = { ...edge, typeOnly: false }; value.forbiddenEdges[0] = value.dependencies[0];
    expect(checked(value, config)).toEqual(["layer-direction"]);
  });

  it("fails value and type-only cycles and substantially duplicated exact bodies", () => {
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/a.ts", loc: 10 }, { path: "src/main/b.ts", loc: 10 });
    value.dependencies.push({ from: "src/main/a.ts", to: "src/main/b.ts", typeOnly: false }, { from: "src/main/b.ts", to: "src/main/a.ts", typeOnly: true });
    value.duplication.functions.push({ hash: "clone", codeLines: 30, instances: [{ file: "src/main/a.ts" }, { file: "src/main/b.ts" }] });
    expect(checked(value)).toEqual(["dependency-cycle", "new-duplication"]);
    value.dependencies[1].typeOnly = false;
    expect(checked(value)).toEqual(["dependency-cycle", "dependency-cycle", "new-duplication"]);
  });

  it("rejects dependency leakage from pure replay and generic ownership-free names", () => {
    const config = configuration();
    config.pureOwners.push({ file: "src/main/session-replay.ts", allowedLocal: ["src/shared/"], allowedExternal: [] });
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/helpers.ts", loc: 10 }, { path: "src/main/opencode.ts", loc: 10 }, { path: "src/main/session-replay.ts", loc: 10 });
    value.dependencies.push({ from: "src/main/session-replay.ts", to: "src/main/opencode.ts", typeOnly: true });
    value.externalImports.push({ file: "src/main/session-replay.ts", specifier: "electron", typeOnly: false });
    expect(checked(value, config)).toEqual(["ownership-name", "pure-owner", "pure-owner"]);
  });

  it("rejects excluded implementation imports rather than permitting metric-scope leaks", () => {
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/service.ts", loc: 10 });
    value.dependencies.push({ from: "src/main/service.ts", to: "src/main/fixtures/implementation.ts", typeOnly: false },
      { from: "src/main/service.ts", to: "scripts/implementation.ts", typeOnly: false },
      { from: "src/main/service.ts", to: "src/renderer/src/styles/opencode/theme.scss", typeOnly: false });
    expect(checked(value)).toEqual(["production-scope", "production-scope"]);
  });

  it("does not grant private-file access through a filename prefix or generic folder", () => {
    const config = configuration() as any;
    config.privateBoundaries.push({ targetPrefix: "src/main/private.ts", owner: "service", allowedFrom: ["src/main/public.ts"] });
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/utils/new.ts", loc: 10 }, { path: "src/main/private.ts", loc: 10 });
    value.dependencies.push({ from: "src/main/public.ts.extra.ts", to: "src/main/private.ts", typeOnly: false });
    expect(checked(value, config)).toEqual(["ownership-name", "private-owner"]);
  });

  it("cannot retain an old cycle exception when edges inside its component change", () => {
    const config = configuration() as any;
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/a.ts", loc: 10 }, { path: "src/main/b.ts", loc: 10 });
    value.dependencies.push({ from: "src/main/a.ts", to: "src/main/b.ts", typeOnly: false }, { from: "src/main/b.ts", to: "src/main/a.ts", typeOnly: false });
    const key = "src/main/a.ts>src/main/b.ts:value|src/main/b.ts>src/main/a.ts:value";
    config.cycleExceptions.push({ mode: "valueImports", key, reason: "Reviewed cycle" }, { mode: "includingTypeOnly", key, reason: "Reviewed cycle" });
    expect(checked(value, config)).toEqual([]);
    value.dependencies.push({ from: "src/main/a.ts", to: "src/main/b.ts", typeOnly: true });
    expect(checked(value, config)).toEqual(["dependency-cycle"]);
  });

  it("ratchets removed duplication and cycles without blessing new clones", () => {
    const config = configuration() as any;
    config.duplicates.functions.clone = 3;
    config.cycleExceptions.push({ mode: "valueImports", key: "gone", reason: "Existing cycle" });
    const value = report() as any;
    value.duplication.functions.push({ hash: "clone", codeLines: 30, instances: [{ file: "a" }, { file: "b" }] });
    const tightened = ratchetConfiguration(value, config);
    expect(tightened.duplicates.functions.clone).toBe(2);
    expect(tightened.cycleExceptions).toEqual([]);
    value.duplication.functions[0].instances.push({ file: "c" });
    expect(() => ratchetConfiguration(value, tightened)).toThrow("Cannot ratchet");
  });

  it("keeps raw IPC authority in the central sender validation owner", () => {
    const value = report() as any;
    value.ipcReferences = [{ file: "src/main/index.ts", line: 1, kind: "raw-ipc" },
      { file: "src/main/service.ts", line: 1, kind: "raw-ipc" }, { file: "src/main/ipc/agent.ts", line: 1, kind: "ipc-domain-electron-import" }];
    expect(checked(value)).toEqual(["ipc-authority", "ipc-authority"]);
  });

  it("fails undocumented debt and dependency exceptions", () => {
    const config = configuration() as any;
    config.modules["src/main/old.ts"] = { loc: 900 };
    config.functionExceptions["src/main/old.ts:run#1"] = { codeLines: 200 };
    config.edgeExceptions.push({ from: "src/main/a.ts", to: "src/main/b.ts", typeOnly: true });
    expect(checked(report(), config)).toEqual(["exception-rationale", "exception-rationale", "exception-rationale"]);
  });

  it("does not permit React DOM or store imports in a React-only interaction owner", () => {
    const config = configuration() as any;
    config.pureOwners.push({ file: "src/renderer/src/layout/panel-interactions.ts", allowedLocal: [], allowedExternal: ["react"] });
    const value = report() as any;
    value.production.inventory.push({ path: "src/renderer/src/store.tsx", loc: 10 }, { path: "src/renderer/src/layout/panel-interactions.ts", loc: 10 });
    value.dependencies.push({ from: "src/renderer/src/layout/panel-interactions.ts", to: "src/renderer/src/store.tsx", typeOnly: false });
    value.externalImports.push({ file: "src/renderer/src/layout/panel-interactions.ts", specifier: "react", typeOnly: false },
      { file: "src/renderer/src/layout/panel-interactions.ts", specifier: "react-dom", typeOnly: false });
    expect(checked(value, config)).toEqual(["pure-owner", "pure-owner"]);
  });

  it("fails closed when limits or required manifest sections are invalid", () => {
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/new.ts", loc: 10_000 });
    value.functions.push({ file: "src/main/new.ts", identity: "run#1", codeLines: 1000, cyclomatic: 1000, nesting: 100 });
    for (const invalid of [{}, { module: "mistyped", codeLines: NaN, cyclomatic: Infinity, nesting: -1 }]) {
      expect(checked(value, { ...configuration(), limits: invalid } as any)).toContain("configuration-schema");
    }
    for (const section of ["modules", "functions", "functionExceptions", "knownFiles", "privateBoundaries", "pureOwners", "duplicates"]) {
      const config = configuration() as any;
      delete config[section];
      expect(checked(value, config)).toContain("configuration-schema");
    }
  });

  it("rejects malformed allowance paths, budgets and boundary lists", () => {
    const config = configuration() as any;
    config.modules["src/main/../hidden.ts"] = { loc: "900", reason: "invalid" };
    config.functions["src/main/a.ts:run#1"] = { cyclomatic: "unlimited", origin: 42, reason: "invalid" };
    config.privateBoundaries.push({ owner: "invalid", targetPrefix: "src/main/private", allowedFrom: ["src/main/public"] });
    config.pureOwners.push({ file: "src/main/a.ts", prefix: "src/main/", allowedLocal: [], allowedExternal: [] });
    expect(checked(report(), config).every((rule: string) => rule === "configuration-schema")).toBe(true);
    expect(checked(report(), config)).toHaveLength(6);
    expect(() => ratchetConfiguration(report(), config)).toThrow("Cannot ratchet");
  });

  it("requires new modules to have a named canonical owner before ratcheting", () => {
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/unowned/new-service.ts", loc: 10 });
    value.ownershipRoutes = [];
    expect(checked(value)).toEqual(["ownership-route"]);
    expect(() => ratchetConfiguration(value, configuration())).toThrow("Cannot ratchet");
    value.ownershipRoutes.push({ owner: "Session service", path: "src/main/unowned/new-service.ts" });
    expect(checked(value)).toEqual([]);
    expect(ratchetConfiguration(value, configuration()).knownFiles).toEqual(["src/main/unowned/new-service.ts"]);
  });

  it("resolves directory ownership without granting sibling-prefix access", () => {
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/runtime/child.ts", loc: 10 }, { path: "src/main/runtime-other/child.ts", loc: 10 });
    value.ownershipRoutes = [{ owner: "Runtime", path: "src/main/runtime/" }];
    expect(checked(value)).toEqual(["ownership-route"]);
    value.ownershipRoutes[0].owner = "";
    expect(checked(value)).toEqual(["ownership-route", "ownership-route", "ownership-route"]);
    value.ownershipRoutes = [{ owner: "Gone", path: "src/main/gone.ts" }];
    expect(checked(value)).toEqual(["ownership-route", "ownership-route", "ownership-route"]);
  });

  it("fails nonliteral loaders that hide dependency authority or cycles", () => {
    const value = report() as any;
    value.computedImports = [{ file: "src/renderer/src/panel.ts", line: 1, computed: "import(name)" }];
    expect(checked(value)).toEqual(["computed-import"]);
    expect(() => ratchetConfiguration(value, configuration())).toThrow("Cannot ratchet");
  });

  it("rejects mismatched or misspelled owner selectors instead of disabling rules", () => {
    for (const selector of [{ file: "src/renderer/src/" }, { prefix: "src/renderer/src/selection.ts" }]) {
      const config = configuration() as any;
      config.pureOwners.push({ ...selector, allowedLocal: [], allowedExternal: [] });
      expect(checked(report(), config)).toEqual(["configuration-schema"]);
    }
    const config = configuration() as any;
    config.pureOwners.push({ file: "src/renderer/src/seletion.ts", allowedLocal: [], allowedExternal: [] });
    config.privateBoundaries.push({ owner: "Gone", targetPrefix: "src/main/gone/", allowedFrom: ["src/main/index.ts"] });
    const value = report() as any;
    value.production.inventory.push({ path: "src/renderer/src/selection.ts", loc: 10 });
    expect(checked(value, config)).toEqual(["owner-target", "owner-target"]);
    expect(() => ratchetConfiguration(value, config)).toThrow("Cannot ratchet");
  });

  it("requires normalized production paths in manifest selectors", () => {
    for (const file of ["src//main/a.ts", "src/main/../a.ts", "src/main/a.ts\n", "src/main\\a.ts", "src/main/a b.ts"]) {
      const config = configuration() as any;
      config.knownFiles.push(file);
      expect(checked(report(), config)).toEqual(["configuration-schema"]);
    }
  });

  it("preserves canonical routes for active owners after their files enter the baseline", () => {
    const config = configuration() as any;
    config.knownFiles.push("src/main/owner.ts", "src/main/index.ts");
    config.pureOwners.push({ file: "src/main/owner.ts", allowedLocal: [], allowedExternal: [] });
    config.privateBoundaries.push({ owner: "Service", targetPrefix: "src/main/owner.ts", allowedFrom: ["src/main/index.ts"] });
    const value = report() as any;
    value.production.inventory.push({ path: "src/main/owner.ts", loc: 10 }, { path: "src/main/index.ts", loc: 10 });
    value.ownershipRoutes = [];
    expect(checked(value, config)).toEqual(["ownership-route"]);
    value.ownershipRoutes.push({ owner: "Composition", path: "src/main/index.ts" });
    expect(checked(value, config)).toEqual(["ownership-route", "ownership-route"]);
    value.ownershipRoutes.push({ owner: "Service", path: "src/main/owner.ts" });
    expect(checked(value, config)).toEqual([]);
  });
});
