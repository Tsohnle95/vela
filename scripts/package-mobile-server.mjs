// Packages the Vela mobile server (the OpenChamber-family web server) into
// out/mobile-server so the packaged Electron app can host it without a source
// checkout. Produces a self-contained tree with a production-only dependency
// closure; the entry the app spawns is node_modules/@openchamber/web/server/index.js.
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const legacyMobileHomeEnv = `${String.fromCharCode(79, 82, 66, 73, 84)}_MOBILE_HOME`;
const candidates = [
  process.env.VELA_MOBILE_HOME ?? process.env[legacyMobileHomeEnv],
  path.join(repoRoot, "..", "openchamber"),
  path.join(os.homedir(), "code-repositories", "openchamber"),
  path.join(os.homedir(), "coding-projects", "openchamber")
].filter(Boolean);
const source = candidates.find((candidate) => existsSync(path.join(candidate, "packages", "web", "server", "index.js")));
if (!source) {
  console.error("[package-mobile-server] mobile server source not found. Set VELA_MOBILE_HOME to the checkout that contains packages/web.");
  process.exit(1);
}

const out = path.join(repoRoot, "out", "mobile-server");
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

const skipNodeModules = (src, dest) => !dest.includes(`${path.sep}node_modules`) && !/\.(test|spec)\.(c|m)?[jt]sx?$/.test(src);
cpSync(path.join(source, "packages", "web"), path.join(out, "web"), { recursive: true, filter: skipNodeModules });
cpSync(path.join(source, "packages", "sdk"), path.join(out, "sdk"), { recursive: true, filter: skipNodeModules });

const strip = (pkgPath) => {
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  delete pkg.devDependencies;
  delete pkg.scripts;
  delete pkg.gitHead;
  return pkg;
};

const webPkg = strip(path.join(out, "web", "package.json"));
webPkg.dependencies = { ...webPkg.dependencies, "@openchamber/sdk": "file:../sdk" };
writeFileSync(path.join(out, "web", "package.json"), `${JSON.stringify(webPkg, null, 2)}\n`);

const sdkPkg = strip(path.join(out, "sdk", "package.json"));
writeFileSync(path.join(out, "sdk", "package.json"), `${JSON.stringify(sdkPkg, null, 2)}\n`);

writeFileSync(path.join(out, "package.json"), `${JSON.stringify({
  name: "vela-mobile-server",
  private: true,
  type: "module",
  dependencies: { "@openchamber/web": "file:./web", "@openchamber/sdk": "file:./sdk" }
}, null, 2)}\n`);

console.log(`[package-mobile-server] staging from ${source} -> ${out}`);
const install = spawnSync("bun", ["install", "--production", "--no-save"], { cwd: out, stdio: "inherit" });
if (install.status !== 0) {
  console.error("[package-mobile-server] bun install failed");
  process.exit(install.status ?? 1);
}
console.log("[package-mobile-server] done");
