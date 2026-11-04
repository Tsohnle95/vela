const { readFileSync } = require("node:fs");
const path = require("node:path");

function validateReleaseTag(tag, version) {
  const expected = `v${version}`;
  if (tag !== expected) {
    throw new Error(`Release tag ${tag || "(missing)"} must match package version ${expected}.`);
  }
  return expected;
}

if (require.main === module) {
  try {
    const { version } = JSON.parse(readFileSync(path.join(__dirname, "..", "package.json"), "utf8"));
    console.log(`Publishing Vela ${validateReleaseTag(process.env.GITHUB_REF_NAME, version)}.`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

module.exports = { validateReleaseTag };
