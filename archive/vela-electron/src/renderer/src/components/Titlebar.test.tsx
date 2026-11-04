import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const layoutStyles = readFileSync(resolve(process.cwd(), "src/renderer/src/styles/_layout.scss"), "utf8");

describe("Titlebar styles", () => {
  it("keeps the left-panel toggle clickable instead of treating it as a window drag handle", () => {
    expect(layoutStyles).toMatch(/\.titlebar-leading-actions\s+\.icon-btn\s*\{[^}]*-webkit-app-region:\s*no-drag/s);
  });
});
