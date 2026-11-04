import { execFileSync } from "node:child_process";
import { mkdirSync, renameSync, rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const iconPath = path.join(root, "resources", "icon.png");
const setDir = path.join(root, "resources", "icon.iconset");
const icnsPath = path.join(root, "resources", "icon.icns");
const pendingIcnsPath = path.join(root, "resources", "icon.pending.icns");

const SIZES = [
  ["icon_16x16.png", 16],
  ["icon_16x16@2x.png", 32],
  ["icon_32x32.png", 32],
  ["icon_32x32@2x.png", 64],
  ["icon_128x128.png", 128],
  ["icon_128x128@2x.png", 256],
  ["icon_256x256.png", 256],
  ["icon_256x256@2x.png", 512],
  ["icon_512x512.png", 512],
  ["icon_512x512@2x.png", 1024]
];

rmSync(setDir, { recursive: true, force: true });
mkdirSync(setDir, { recursive: true });
try {
  for (const [name, size] of SIZES) {
    execFileSync("/usr/bin/sips", ["-z", String(size), String(size), iconPath, "--out", path.join(setDir, name)], { stdio: "ignore" });
  }
  execFileSync("/usr/bin/iconutil", ["-c", "icns", setDir, "-o", pendingIcnsPath]);
  renameSync(pendingIcnsPath, icnsPath);
  console.error("[icons] wrote resources/icon.icns from resources/icon.png");
} finally {
  rmSync(setDir, { recursive: true, force: true });
  rmSync(pendingIcnsPath, { force: true });
}
