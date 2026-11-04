import { chmodSync, existsSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export function configureNodePty(platform = process.platform, arch = process.arch, root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")) {
  if (platform !== "darwin") return null;
  const packageRoot = path.join(root, "node_modules", "node-pty");
  const candidates = [
    path.join(packageRoot, "prebuilds", `${platform}-${arch}`, "spawn-helper"),
    path.join(packageRoot, "build", "Release", "spawn-helper")
  ];
  const helper = candidates.find(existsSync);
  if (!helper) throw new Error(`node-pty spawn helper not found in ${packageRoot}`);
  const mode = statSync(helper).mode;
  chmodSync(helper, mode | 0o111);
  return helper;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) configureNodePty();
