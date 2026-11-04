import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";

const require = createRequire(import.meta.url);
const { validateReleaseTag } = require("./check-release-tag.cjs") as {
  validateReleaseTag: (tag: string | undefined, version: string) => string;
};

describe("release tag validation", () => {
  it("accepts only a tag that matches the app package version", () => {
    expect(validateReleaseTag("v0.1.0", "0.1.0")).toBe("v0.1.0");
    expect(() => validateReleaseTag("v0.2.0", "0.1.0")).toThrow("must match package version v0.1.0");
    expect(() => validateReleaseTag(undefined, "0.1.0")).toThrow("Release tag (missing)");
  });
});
