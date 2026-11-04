import { describe, expect, it } from "vitest";
import { appUpdaterWindowDocument, appUpdaterWindowUrl, shouldBlockAppUpdaterReload } from "./app-updater-window";

describe("Vela updater window", () => {
  it("renders a self-contained, sandbox-friendly progress window", () => {
    const document = appUpdaterWindowDocument();

    expect(document).toContain("Content-Security-Policy");
    expect(document).toContain("default-src 'none'");
    expect(document).toContain('role="status"');
    expect(document).toContain("Vela will close while the downloaded release is installed.");
    expect(document).toContain("window.setVelaUpdaterStatus");
    expect(document).toContain("location.hash = 'return-to-vela'");
    expect(document).toContain("Return to Vela to close this window and retry from Settings.");
    expect(appUpdaterWindowUrl()).toMatch(/^data:text\/html;charset=utf-8,/);
  });

  it.each([
    { type: "keyDown", key: "r", meta: true },
    { type: "keyDown", key: "r", control: true },
    { type: "keyDown", key: "F5" }
  ])("blocks the updater window reload shortcut %#", (input) => {
    expect(shouldBlockAppUpdaterReload(input)).toBe(true);
  });

  it("allows regular input and unrelated shortcuts", () => {
    expect(shouldBlockAppUpdaterReload({ type: "char", key: "r", meta: true })).toBe(false);
    expect(shouldBlockAppUpdaterReload({ type: "keyDown", key: "w", meta: true })).toBe(false);
  });
});
