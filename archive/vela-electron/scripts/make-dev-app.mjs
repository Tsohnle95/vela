import { existsSync, mkdirSync, rmSync, statSync } from "node:fs";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

if (process.platform !== "darwin") {
  console.log("dev app bundle is macOS-only; launching with the plain Electron binary");
  process.exit(0);
}

const src = path.join(root, "node_modules", "electron", "dist", "Electron.app");
const dst = path.join(root, "dev", "Vela.app");
const plist = path.join(dst, "Contents", "Info.plist");
const icns = path.join(root, "resources", "icon.icns");

if (!existsSync(src)) {
  console.error("electron dist not found — run npm install first");
  process.exit(1);
}

if (existsSync(dst) && statSync(plist).mtimeMs > statSync(path.join(src, "Contents", "Info.plist")).mtimeMs && existsSync(icns) && statSync(plist).mtimeMs > statSync(icns).mtimeMs && statSync(plist).mtimeMs > statSync(fileURLToPath(import.meta.url)).mtimeMs) {
  console.log("Vela.app is up to date");
  process.exit(0);
}

rmSync(dst, { recursive: true, force: true });
mkdirSync(path.dirname(dst), { recursive: true });
console.log("copying Electron.app -> dev/Vela.app …");
execFileSync("cp", ["-R", src, dst]);

rmSync(path.join(dst, "Contents", "_CodeSignature"), { recursive: true, force: true });

for (const [key, value] of [
  ["CFBundleName", "Vela"],
  ["CFBundleDisplayName", "Vela"],
  ["CFBundleIdentifier", "dev.vela.desktop"],
  ["CFBundleIconFile", "icon.icns"]
]) {
  execFileSync("plutil", ["-replace", key, "-string", value, plist]);
}

const docType = (name, ...contentTypes) =>
  `<dict><key>CFBundleTypeName</key><string>${name}</string>` +
  `<key>CFBundleTypeRole</key><string>Editor</string>` +
  `<key>LSHandlerRank</key><string>Alternate</string>` +
  `<key>LSItemContentTypes</key><array>${contentTypes.map((type) => `<string>${type}</string>`).join("")}</array></dict>`;
execFileSync("plutil", ["-replace", "CFBundleDocumentTypes", "-xml",
  `<array>${docType("Folder", "public.folder")}${docType("Plain text", "public.text")}${docType("Source code", "public.source-code")}</array>`,
  plist]);

execFileSync("cp", [icns, path.join(dst, "Contents", "Resources", "icon.icns")]);
execFileSync("codesign", ["--force", "--deep", "--sign", "-", dst]);
console.log("Vela.app ready");
