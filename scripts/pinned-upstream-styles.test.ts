// @vitest-environment node
import { describe, expect, it } from "vitest";
import { execFileSync, spawnSync } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { analyzeRepository, sourceCategory, validatePinnedStyles } from "./architecture-analysis.mjs";

const repository = path.resolve(__dirname, "..");
const pin = "src/renderer/src/styles/opencode/theme.scss";
const loadManifest = async () => JSON.parse(await readFile(path.join(repository, "scripts/pinned-upstream-styles.json"), "utf8"));

describe("frozen upstream stylesheet scope", () => {
  it("preserves exactly the baseline path and hash exclusions without exempting new code", async () => {
    const manifest = await loadManifest();
    const baseline = JSON.parse(await readFile(path.join(repository, "reports/architecture-optimization/baseline.json"), "utf8"));
    expect(baseline.production).toMatchObject({ files: 152, loc: 38_430 });
    expect(manifest.files).toEqual(Object.fromEntries(baseline.excludedAndAuxiliaryInventory
      .filter((file: any) => file.category === "pinned-upstream-styles")
      .map((file: any) => [file.path, file.sha256])));
    for (const file of Object.keys(manifest.files)) expect(sourceCategory(file)).toBe("pinned-upstream-styles");
    for (const file of ["new.scss", "new.css", "new.ts", "new.js", "theme.scss.ts", "nested/theme.scss"]) {
      expect(sourceCategory(`src/renderer/src/styles/opencode/${file}`)).toBe("production");
    }
    expect(sourceCategory("src/renderer/src/styles/opencode-other/theme.scss")).toBe("production");
  });

  it("fails closed on malformed provenance, paths and digests", async () => {
    const manifest = await loadManifest();
    expect(() => validatePinnedStyles(manifest)).not.toThrow();
    for (const invalid of [null, [], {}, { ...manifest, files: {} }, { ...manifest, files: [] },
      { ...manifest, provenance: " " }, { ...manifest, upstreamCommit: "unknown" },
      ...["src/renderer/src/styles/opencode/../theme.scss", "src/renderer/src/styles/opencode/theme.ts",
        "src//renderer/src/styles/opencode/theme.scss", "src/renderer/src/styles/opencode/theme.scss\n"]
        .map((file) => ({ ...manifest, files: { [file]: manifest.files[pin] } })),
      { ...manifest, files: { [pin]: "ABC" } }, { ...manifest, files: { [pin]: 123 } }]) {
      expect(() => validatePinnedStyles(invalid)).toThrow("Invalid frozen upstream style inventory");
    }
  });

  it("rejects changed pins and imported new handwritten styles or privileged scripts through the actual CLI", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "vela-pinned-styles-test-"));
    try {
      await mkdir(path.join(root, "scripts"));
      await mkdir(path.join(root, "docs"));
      await mkdir(path.dirname(path.join(root, pin)), { recursive: true });
      await symlink(path.join(repository, "node_modules"), path.join(root, "node_modules"), "junction");
      for (const file of ["architecture-analysis.mjs", "check-architecture.mjs", "pinned-upstream-styles.json", "pinned-upstream-ai-elements.json"]) {
        await copyFile(path.join(repository, "scripts", file), path.join(root, "scripts", file));
      }
      const aiPins = JSON.parse(await readFile(path.join(repository, "scripts/pinned-upstream-ai-elements.json"), "utf8"));
      for (const file of Object.keys(aiPins.files)) {
        await mkdir(path.dirname(path.join(root, file)), { recursive: true });
        await copyFile(path.join(repository, file), path.join(root, file));
      }
      await copyFile(path.join(repository, pin), path.join(root, pin));
      await writeFile(path.join(root, "package.json"), "{}");
      await writeFile(path.join(root, "docs/architecture.md"), "## Implementation ownership and fitness\n| Owner | Entry |\n| Renderer | `src/renderer/src/main.ts` |\n");
      await writeFile(path.join(root, "src/renderer/src/main.ts"), 'import "./styles/opencode/theme.scss";\nexport const app = true;\n');
      await writeFile(path.join(root, "scripts/architecture-baseline.json"), JSON.stringify({
        limits: { module: 800, codeLines: 100, cyclomatic: 20, nesting: 5 }, knownFiles: ["src/renderer/src/main.ts"],
        modules: {}, functions: {}, functionExceptions: {}, edgeExceptions: [], cycleExceptions: [],
        duplicates: { functions: {}, blocks: {} }, privateBoundaries: [], pureOwners: []
      }));
      execFileSync("git", ["init", "--quiet"], { cwd: root });
      execFileSync("git", ["add", "."], { cwd: root });
      execFileSync("git", ["-c", "user.name=Architecture Test", "-c", "user.email=architecture@example.invalid",
        "-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Pinned fixture"], { cwd: root });
      const check = () => spawnSync(process.execPath, ["scripts/check-architecture.mjs"], { cwd: root, encoding: "utf8" });
      expect(check().status).toBe(0);

      await writeFile(path.join(root, pin), await readFile(path.join(root, pin), "utf8") + "\n.vela-handwritten { color: red; }\n");
      const changed = check();
      expect(changed.status).toBe(1);
      expect(changed.stderr).toContain(`pinned-style-integrity: ${pin}`);
      const historical = await analyzeRepository(root, { ref: "HEAD", fast: true });
      expect(historical.pinnedStyleDrift).toEqual([]);
      expect(historical.production.files).toBe(1);
      await copyFile(path.join(repository, pin), path.join(root, pin));
      expect(check().status).toBe(0);

      const styles = "src/renderer/src/styles/opencode/new-handwritten.scss";
      const script = "src/renderer/src/styles/opencode/privileged.ts";
      await writeFile(path.join(root, styles), Array.from({ length: 801 }, (_, i) => `.new-${i} { color: red; }`).join("\n") + "\n");
      await writeFile(path.join(root, script), 'import fs from "node:fs";\nexport const read = fs.readFileSync;\n');
      await writeFile(path.join(root, "src/renderer/src/main.ts"), 'import "./styles/opencode/theme.scss";\nimport "./styles/opencode/new-handwritten.scss";\nimport { read } from "./styles/opencode/privileged";\nexport const app = read;\n');
      const rejected = check();
      expect(rejected.status).toBe(1);
      expect(rejected.stderr).toContain(`module-size: ${styles}`);
      expect(rejected.stderr).toContain(`ownership-route: ${styles}`);
      expect(rejected.stderr).toContain(`privileged-import: ${script}: node:fs`);
      const current = await analyzeRepository(root, { fast: true });
      expect(current.production.inventory.map((file: any) => file.path)).toEqual([
        "src/renderer/src/main.ts", script, styles
      ].sort());
      expect(current.production.loc).toBe(807);
      expect(current.pinnedStyleDrift).toEqual([]);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  }, 30_000);

});
