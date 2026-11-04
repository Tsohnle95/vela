import { app, BrowserWindow } from "electron";
import type { VelaAppUpdateResult } from "@shared/types";
import type { VelaAppUpdater } from "./app-updater";
import {
  briefFilePath,
  appUpdaterWindowUrl,
  setAppUpdaterWindowStatus,
  shouldBlockAppUpdaterReload,
  type AppUpdaterWindowStatus
} from "./app-updater-window";

export class AppUpdateController {
  private appUpdaterWindow: BrowserWindow | null = null;
  private appUpdaterWindowCanClose = false;
  private appUpdateRunning = false;

  constructor(
    private readonly velaAppUpdater: VelaAppUpdater,
    private readonly isVelaReleaseBuild: boolean,
    private readonly appIconPath: string,
    private readonly getWindow: () => BrowserWindow | null,
    private readonly getUnsavedFiles: () => readonly string[]
  ) {}

  private get window(): BrowserWindow | null {
    return this.getWindow();
  }
  private get unsavedEditorFileNames(): readonly string[] {
    return this.getUnsavedFiles();
  }

  async update(): Promise<VelaAppUpdateResult> {
    if (!this.isVelaReleaseBuild) {
      return {
        ok: false,
        updated: false,
        currentVersion: app.getVersion(),
        message: "GitHub release updates are disabled in local development and test builds."
      };
    }
    if (this.unsavedEditorFileNames.length > 0) {
      const files = this.unsavedEditorFileNames.slice(0, 8).map(briefFilePath).join(", ");
      const remaining = this.unsavedEditorFileNames.length > 8 ? ` and ${this.unsavedEditorFileNames.length - 8} more` : "";
      return {
        ok: false,
        updated: false,
        message: `Save unsaved files before updating Vela: ${files}${remaining}.`
      };
    }
    if (this.appUpdateRunning) return { ok: false, updated: false, message: "A Vela update is already running." };
    this.appUpdateRunning = true;

    if (this.appUpdaterWindow && !this.appUpdaterWindow.isDestroyed()) {
      this.appUpdaterWindowCanClose = true;
      this.appUpdaterWindow.close();
    }

    const popup = new BrowserWindow({
      width: 460,
      height: 270,
      minWidth: 460,
      minHeight: 270,
      maxWidth: 460,
      maxHeight: 270,
      title: "Updating Vela",
      icon: this.appIconPath,
      show: false,
      resizable: false,
      minimizable: false,
      maximizable: false,
      fullscreenable: false,
      autoHideMenuBar: true,
      backgroundColor: "#111214",
      titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "default",
      webPreferences: {
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: true,
        devTools: false
      }
    });
    this.appUpdaterWindow = popup;
    this.appUpdaterWindowCanClose = false;
    popup.on("close", (event) => {
      if (!this.appUpdaterWindowCanClose) event.preventDefault();
    });
    popup.on("closed", () => {
      if (this.appUpdaterWindow === popup) this.appUpdaterWindow = null;
    });
    popup.webContents.setWindowOpenHandler(() => ({ action: "deny" }));

    let lastStatus: AppUpdaterWindowStatus = {
      title: "Preparing update…",
      message: "Checking the latest published GitHub release.",
      tone: "progress"
    };
    popup.webContents.on("dom-ready", () => setAppUpdaterWindowStatus(popup, lastStatus));
    popup.webContents.on("before-input-event", (event, input) => {
      if (shouldBlockAppUpdaterReload(input)) event.preventDefault();
    });
    popup.webContents.on("did-navigate-in-page", (_event, url, _isInPlace, isMainFrame) => {
      if (isMainFrame && url.endsWith("#return-to-vela") && this.appUpdaterWindowCanClose) popup.close();
    });

    const report = (title: string, message: string, tone: "progress" | "success" | "error" = "progress"): void => {
      lastStatus = { title, message, tone };
      if (!popup.isDestroyed()) popup.setTitle(`${title} — ${message}`);
      setAppUpdaterWindowStatus(popup, lastStatus);
    };

    try {
      await popup.loadURL(appUpdaterWindowUrl());
      popup.show();
      popup.focus();
      if (this.window && !this.window.isDestroyed()) this.window.hide();
    } catch (error) {
      this.appUpdaterWindowCanClose = true;
      if (!popup.isDestroyed()) popup.destroy();
      if (this.appUpdaterWindow === popup) this.appUpdaterWindow = null;
      this.appUpdateRunning = false;
      const detail = error instanceof Error ? error.message : String(error);
      return { ok: false, updated: false, message: "Vela could not open its update window. " + detail };
    }

    try {
      const result = await this.velaAppUpdater.update((message) => report("Updating Vela", message));
      if (result.updated) {
        report("Update downloaded", `${result.message} Vela will close to install the release and reopen shortly.`, "success");
        setTimeout(() => {
          try {
            this.appUpdaterWindowCanClose = true;
            this.velaAppUpdater.installAndRestart();
          } catch (error) {
            const detail = error instanceof Error ? error.message : String(error);
            report("Update could not be installed", detail, "error");
            this.appUpdaterWindowCanClose = true;
            if (this.window && !this.window.isDestroyed()) {
              this.window.show();
              this.window.focus();
            }
          }
        }, 1_400);
      } else if (result.ok) {
        report("Already up to date", result.message, "success");
        this.appUpdaterWindowCanClose = true;
        if (this.window && !this.window.isDestroyed()) {
          this.window.show();
          this.window.focus();
        }
        setTimeout(() => {
          if (!popup.isDestroyed()) popup.close();
        }, 1_800);
      } else {
        report("Update could not be completed", result.message, "error");
        this.appUpdaterWindowCanClose = true;
        if (this.window && !this.window.isDestroyed()) {
          this.window.show();
          this.window.focus();
        }
      }
      return result;
    } finally {
      this.appUpdateRunning = false;
    }
  }
}
