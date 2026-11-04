import { EventEmitter } from "node:events";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppUpdater } from "electron-updater";
import { createDisabledVelaAppUpdater, createVelaAppUpdater } from "./app-updater";

class FakeReleaseUpdater extends EventEmitter {
  autoDownload = true;
  autoInstallOnAppQuit = true;
  allowPrerelease = true;
  allowDowngrade = true;
  available = true;
  latestVersion = "0.2.0";
  checkError: Error | null = null;
  downloadError: Error | null = null;
  checkForUpdates = vi.fn(async () => {
    if (this.checkError) throw this.checkError;
    const updateInfo = { version: this.latestVersion };
    return {
      isUpdateAvailable: this.available,
      updateInfo,
      versionInfo: updateInfo
    };
  });
  downloadUpdate = vi.fn(async () => {
    if (this.downloadError) throw this.downloadError;
    this.emit("download-progress", { percent: 43.8, total: 100, transferred: 43, delta: 43, bytesPerSecond: 1024 });
    return ["/tmp/vela-release.zip"];
  });
  quitAndInstall = vi.fn();
}

function makeUpdater(overrides: {
  available?: boolean;
  latestVersion?: string;
  checkError?: Error;
  downloadError?: Error;
} = {}) {
  const client = new FakeReleaseUpdater();
  if (overrides.available !== undefined) client.available = overrides.available;
  if (overrides.latestVersion) client.latestVersion = overrides.latestVersion;
  if (overrides.checkError) client.checkError = overrides.checkError;
  if (overrides.downloadError) client.downloadError = overrides.downloadError;
  return {
    client,
    updater: createVelaAppUpdater(client as unknown as AppUpdater, "0.1.0")
  };
}

afterEach(() => vi.clearAllMocks());

describe("Vela app updater", () => {
  it("disables GitHub updater work in local development and test builds", async () => {
    const updater = createDisabledVelaAppUpdater("0.1.0");

    await expect(updater.check()).resolves.toEqual({
      state: "disabled",
      currentVersion: "0.1.0",
      message: "GitHub release updates are disabled in local development and test builds."
    });
    await expect(updater.update()).resolves.toMatchObject({ ok: false, updated: false, currentVersion: "0.1.0" });
    expect(() => updater.installAndRestart()).not.toThrow();
  });

  it("compares the installed version with the latest published GitHub release", async () => {
    const { client, updater } = makeUpdater();

    await expect(updater.check()).resolves.toEqual({
      state: "available",
      currentVersion: "0.1.0",
      latestVersion: "0.2.0"
    });
    expect(client.autoDownload).toBe(false);
    expect(client.autoInstallOnAppQuit).toBe(false);
    expect(client.allowPrerelease).toBe(false);
    expect(client.allowDowngrade).toBe(false);
    expect(client.checkForUpdates).toHaveBeenCalledOnce();
  });

  it("reports when the installed version is the latest published release", async () => {
    const { updater } = makeUpdater({ available: false, latestVersion: "0.1.0" });

    await expect(updater.check()).resolves.toEqual({
      state: "current",
      currentVersion: "0.1.0",
      latestVersion: "0.1.0"
    });
  });

  it("reports release feed failures without modifying the app", async () => {
    const { client, updater } = makeUpdater({ checkError: new Error("GitHub is unreachable") });

    await expect(updater.check()).resolves.toMatchObject({
      state: "blocked",
      currentVersion: "0.1.0",
      message: "Could not check GitHub Releases for Vela updates: GitHub is unreachable"
    });
    expect(client.downloadUpdate).not.toHaveBeenCalled();
    expect(client.quitAndInstall).not.toHaveBeenCalled();
  });

  it("downloads the published app, reports progress, then installs only when asked", async () => {
    const { client, updater } = makeUpdater();
    const progress: string[] = [];

    await expect(updater.update((message) => progress.push(message))).resolves.toEqual({
      ok: true,
      updated: true,
      currentVersion: "0.1.0",
      latestVersion: "0.2.0",
      message: "Vela 0.2.0 is downloaded and ready to install."
    });

    expect(client.checkForUpdates).toHaveBeenCalledOnce();
    expect(client.downloadUpdate).toHaveBeenCalledOnce();
    expect(client.listenerCount("download-progress")).toBe(0);
    expect(client.quitAndInstall).not.toHaveBeenCalled();
    expect(progress).toEqual([
      "Checking GitHub Releases for a Vela update…",
      "Downloading Vela 0.2.0 from GitHub Releases…",
      "Downloading Vela 0.2.0 from GitHub Releases (43%)…",
      "Release downloaded. Installing it and restarting Vela…"
    ]);

    updater.installAndRestart();
    expect(client.quitAndInstall).toHaveBeenCalledWith(false, true);
  });

  it("does not download or restart when the release is current", async () => {
    const { client, updater } = makeUpdater({ available: false, latestVersion: "0.1.0" });

    await expect(updater.update()).resolves.toEqual({
      ok: true,
      updated: false,
      currentVersion: "0.1.0",
      latestVersion: "0.1.0",
      message: "Vela 0.1.0 is the latest published release."
    });
    expect(client.downloadUpdate).not.toHaveBeenCalled();
    expect(client.quitAndInstall).not.toHaveBeenCalled();
  });

  it("keeps Vela open and reports a release download failure", async () => {
    const { client, updater } = makeUpdater({ downloadError: new Error("download interrupted") });

    await expect(updater.update()).resolves.toMatchObject({
      ok: false,
      updated: false,
      message: "Vela could not download the release update. download interrupted"
    });
    expect(client.listenerCount("download-progress")).toBe(0);
    expect(client.quitAndInstall).not.toHaveBeenCalled();
  });
});
