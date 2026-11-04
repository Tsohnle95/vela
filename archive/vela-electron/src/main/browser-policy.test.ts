import type { WebPreferences } from "electron";
import { describe, expect, it } from "vitest";
import { BROWSER_PARTITION, hardenBrowserPreferences, isAllowedBrowserUrl } from "./browser-policy";

describe("browser guest policy", () => {
  it("permits credential-free web URLs and the exact blank page", () => {
    for (const url of ["http://localhost:5173/", "https://example.com/", "about:blank"]) {
      expect(isAllowedBrowserUrl(url)).toBe(true);
    }
    for (const url of ["", "example.com", "file:///tmp/secret", "javascript:alert(1)", "data:text/html,x", "vela://x", "https://user:password@example.com/", "about:blank?x", "about:blank#x"]) {
      expect(isAllowedBrowserUrl(url)).toBe(false);
    }
  });

  it("removes privilege and overrides every guest isolation setting", () => {
    const preferences: WebPreferences & { preloadURL?: string } = {
      preload: "/app/preload.js", preloadURL: "file:///app/preload.js",
      nodeIntegration: true, nodeIntegrationInSubFrames: true, nodeIntegrationInWorker: true,
      webviewTag: true, contextIsolation: false, sandbox: false, webSecurity: false,
      allowRunningInsecureContent: true, partition: "persist:default", session: {} as Electron.Session
    };
    expect(hardenBrowserPreferences(preferences, "https://example.com")).toBe(true);
    expect(preferences).toEqual({
      nodeIntegration: false, nodeIntegrationInSubFrames: false, nodeIntegrationInWorker: false,
      webviewTag: false, contextIsolation: true, sandbox: true, webSecurity: true,
      allowRunningInsecureContent: false, partition: BROWSER_PARTITION
    });
  });

  it("rejects nonweb attachment without modifying its preferences", () => {
    const preferences = { preload: "/app/preload.js" };
    expect(hardenBrowserPreferences(preferences, "file:///tmp/secret")).toBe(false);
    expect(preferences).toEqual({ preload: "/app/preload.js" });
  });
});
