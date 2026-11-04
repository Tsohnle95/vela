import type { AppUpdater } from "electron-updater";
import type { VelaAppUpdateResult, VelaAppUpdateStatus } from "@shared/types";

export interface VelaAppUpdater {
  check(): Promise<VelaAppUpdateStatus>;
  update(onProgress?: (message: string) => void): Promise<VelaAppUpdateResult>;
  installAndRestart(): void;
}

export function createDisabledVelaAppUpdater(currentVersion: string): VelaAppUpdater {
  const message = "GitHub release updates are disabled in local development and test builds.";
  return {
    async check() {
      return { state: "disabled", currentVersion, message };
    },
    async update() {
      return { ok: false, updated: false, currentVersion, message };
    },
    installAndRestart() {}
  };
}

export function createVelaAppUpdater(updater: AppUpdater, currentVersion: string): VelaAppUpdater {
  updater.autoDownload = false;
  updater.autoInstallOnAppQuit = false;
  updater.allowPrerelease = false;
  updater.allowDowngrade = false;

  let updateInProgress = false;

  async function check(): Promise<VelaAppUpdateStatus> {
    try {
      const result = await updater.checkForUpdates();
      if (!result) {
        return {
          state: "blocked",
          currentVersion,
          message: "Vela could not check GitHub Releases because this app has no release update feed."
        };
      }
      const latestVersion = result.updateInfo.version;
      return result.isUpdateAvailable
        ? { state: "available", currentVersion, latestVersion }
        : { state: "current", currentVersion, latestVersion };
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      return {
        state: "blocked",
        currentVersion,
        message: `Could not check GitHub Releases for Vela updates: ${detail}`
      };
    }
  }

  async function update(onProgress?: (message: string) => void): Promise<VelaAppUpdateResult> {
    if (updateInProgress) {
      return { ok: false, updated: false, currentVersion, message: "A Vela update is already running." };
    }
    updateInProgress = true;
    try {
      onProgress?.("Checking GitHub Releases for a Vela update…");
      const status = await check();
      if (status.state === "blocked" || status.state === "disabled") {
        return { ok: false, updated: false, currentVersion, message: status.message };
      }
      if (status.state === "current") {
        return {
          ok: true,
          updated: false,
          currentVersion,
          latestVersion: status.latestVersion,
          message: `Vela ${currentVersion} is the latest published release.`
        };
      }

      onProgress?.(`Downloading Vela ${status.latestVersion} from GitHub Releases…`);
      const reportDownloadProgress = (progress: { percent: number }): void => {
        const percent = Number.isFinite(progress.percent)
          ? Math.max(0, Math.min(100, Math.floor(progress.percent)))
          : null;
        onProgress?.(percent === null
          ? `Downloading Vela ${status.latestVersion} from GitHub Releases…`
          : `Downloading Vela ${status.latestVersion} from GitHub Releases (${percent}%)…`);
      };
      updater.on("download-progress", reportDownloadProgress);
      try {
        await updater.downloadUpdate();
      } finally {
        updater.removeListener("download-progress", reportDownloadProgress);
      }

      onProgress?.("Release downloaded. Installing it and restarting Vela…");
      return {
        ok: true,
        updated: true,
        currentVersion,
        latestVersion: status.latestVersion,
        message: `Vela ${status.latestVersion} is downloaded and ready to install.`
      };
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      return {
        ok: false,
        updated: false,
        currentVersion,
        message: `Vela could not download the release update. ${detail}`
      };
    } finally {
      updateInProgress = false;
    }
  }

  return {
    check,
    update,
    installAndRestart() {
      updater.quitAndInstall(false, true);
    }
  };
}
