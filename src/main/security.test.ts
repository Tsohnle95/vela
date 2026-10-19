import { describe, expect, it } from "vitest";
import {
  applicationUrl,
  isAllowedMainFrameNavigation,
  isTrustedApplicationUrl,
  isTrustedIpcSender,
  trustedApplicationLocation
} from "./security";

describe("renderer navigation policy", () => {
  it("always selects the packaged file and ignores renderer environment overrides", () => {
    const packaged = "file:///Applications/Vela.app/Contents/Resources/app/out/renderer/index.html";
    expect(applicationUrl(true, "https://evil.test/app", packaged)).toBe(packaged);
    expect(trustedApplicationLocation(applicationUrl(true, "file:///tmp/evil.html", packaged))).toEqual({
      kind: "exact",
      value: packaged
    });
  });

  it("permits only local HTTP development renderer locations", () => {
    const packaged = "file:///app/index.html";
    expect(applicationUrl(false, "http://localhost:5173/", packaged)).toBe("http://localhost:5173/");
    expect(applicationUrl(false, "http://127.0.0.1:5173/app", packaged)).toBe("http://127.0.0.1:5173/app");
    expect(applicationUrl(false, undefined, packaged)).toBe(packaged);
    for (const value of ["https://localhost:5173/", "http://evil.test/", "http://localhost.evil.test/", "file:///tmp/app.html"]) {
      expect(() => applicationUrl(false, value, packaged)).toThrow("Unsupported development renderer URL");
    }
  });
  it("allows the exact packaged document but denies other main-frame navigations and redirects", () => {
    const trusted = trustedApplicationLocation("file:///Applications/Vela.app/Contents/Resources/app/out/renderer/index.html");

    expect(isAllowedMainFrameNavigation("file:///Applications/Vela.app/Contents/Resources/app/out/renderer/index.html", true, trusted)).toBe(true);
    expect(isAllowedMainFrameNavigation("file:///Applications/Vela.app/Contents/Resources/app/out/renderer/other.html", true, trusted)).toBe(false);
    expect(isAllowedMainFrameNavigation("https://example.com/redirect", true, trusted)).toBe(false);
    expect(isAllowedMainFrameNavigation("https://example.com/subframe", false, trusted)).toBe(true);
  });

  it("trusts the approved development origin without trusting lookalikes or credentials", () => {
    const trusted = trustedApplicationLocation("http://localhost:5173/");

    expect(isTrustedApplicationUrl("http://localhost:5173/src/main.tsx", trusted)).toBe(true);
    expect(isTrustedApplicationUrl("http://localhost:5174/", trusted)).toBe(false);
    expect(isTrustedApplicationUrl("http://localhost:5173.evil.test/", trusted)).toBe(false);
    expect(isTrustedApplicationUrl("http://user:secret@localhost:5173/", trusted)).toBe(false);
  });

  it("rejects unsupported application schemes", () => {
    expect(() => trustedApplicationLocation("javascript:alert(1)")).toThrow("Unsupported application URL");
  });
});

describe("IPC sender policy", () => {
  it("requires the owned WebContents, its main frame, and its trusted URL", () => {
    const trusted = trustedApplicationLocation("https://localhost:5173/");
    const mainFrame = { url: "https://localhost:5173/app" };
    const expected = { mainFrame };

    expect(isTrustedIpcSender({ sender: expected, senderFrame: mainFrame }, expected, trusted)).toBe(true);
    expect(isTrustedIpcSender({ sender: { mainFrame }, senderFrame: mainFrame }, expected, trusted)).toBe(false);
    expect(isTrustedIpcSender({ sender: expected, senderFrame: { url: mainFrame.url } }, expected, trusted)).toBe(false);
    expect(isTrustedIpcSender({ sender: expected, senderFrame: null }, expected, trusted)).toBe(false);
    expect(isTrustedIpcSender({ sender: expected, senderFrame: { url: "https://evil.test/" } }, expected, trusted)).toBe(false);
    expect(isTrustedIpcSender({ sender: expected, senderFrame: mainFrame }, null, trusted)).toBe(false);
  });
});
