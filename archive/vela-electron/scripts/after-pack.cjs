// Copies the packaged mobile server (out/mobile-server) into the built app's
// Resources. electron-builder's file matcher excludes node_modules from
// extraResources, so the full tree is copied here after packing instead.
const { cpSync, existsSync } = require("node:fs");
const path = require("node:path");

module.exports = async function afterPack(context) {
  const source = path.join(context.packager.projectDir, "out", "mobile-server");
  if (!existsSync(source)) {
    console.warn("[after-pack] out/mobile-server missing; skipping mobile server copy");
    return;
  }
  const appName = context.packager.appInfo.productFilename;
  const dest = path.join(context.appOutDir, `${appName}.app`, "Contents", "Resources", "mobile-server");
  cpSync(source, dest, { recursive: true });
  console.log(`[after-pack] copied mobile server -> ${dest}`);
};
