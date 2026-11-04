#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const pinned = pkg.dependencies["@opencode/client"];

const parse = (version) => {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(String(version ?? "").trim());
  return match ? match.slice(1).map(Number) : null;
};

const newer = (a, b) => {
  for (let index = 0; index < 3; index += 1) {
    if (a[index] !== b[index]) return a[index] > b[index];
  }
  return false;
};

let tags;
try {
  tags = JSON.parse(
    execFileSync("npm", ["view", "@opencode/client", "dist-tags", "--json"], {
      encoding: "utf8",
      timeout: 30_000
    })
  );
} catch (error) {
  console.error(`client:drift could not reach the npm registry: ${error.message}`);
  process.exit(2);
}

if (!tags.latest) {
  console.error("client:drift found no published `latest` tag for @opencode/client");
  process.exit(2);
}

const pinnedVersion = parse(pinned);
const latestVersion = parse(tags.latest);
console.log(`pinned  @opencode/client ${pinned}`);
console.log(`latest  @opencode/client ${tags.latest}`);

if (!pinnedVersion || !latestVersion) {
  console.error("\nCould not compare versions; review the pin and published tag by hand.");
  process.exit(1);
}

if (newer(latestVersion, pinnedVersion)) {
  console.error(
    "\nA newer stable @opencode/client release is published. Review the changelog, bump the exact pin, and re-run the check suite before adopting it."
  );
  process.exit(1);
}

if (newer(pinnedVersion, latestVersion)) {
  console.error(
    "\nThe pinned @opencode/client is newer than the published `latest` tag; the pin is not a reviewed public release."
  );
  process.exit(1);
}

console.log("\npinned client matches the newest published stable release.");
