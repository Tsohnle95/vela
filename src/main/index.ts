import { app, BrowserWindow, dialog, ipcMain, nativeImage, nativeTheme, shell, type IpcMainInvokeEvent, type WebContents } from "electron";
import { autoUpdater } from "electron-updater";
import path from "node:path";
import fsp from "node:fs/promises";
import { existsSync } from "node:fs";
import { spawn } from "node:child_process";
import { pathToFileURL } from "node:url";
import { OpenShellBackend, SessionWorkspaceNotFoundError } from "./opencode";
import { attachBrowserGuards } from "./browser-guards";
import { createDisabledMiraAppUpdater, createMiraAppUpdater } from "./app-updater";
import { briefFilePath } from "./app-updater-window";
import { AppUpdateController } from "./app-update-controller";
import { TerminalManager } from "./terminal";
import { MobileServer } from "./mobile-server";
import { defaultViteDeps, findHtmlEntry, resolveViteCommand, viteServerKey, VitePreviewManager } from "./vite-server";
import { collectLaunchPaths, PendingOpenPaths } from "./open-paths";
import {
  applicationUrl,
  isAllowedMainFrameNavigation,
  isTrustedIpcSender,
  trustedApplicationLocation,
  type TrustedApplicationLocation
} from "./security";
import { safeExternalUrl } from "@shared/url-policy";
import { validateWithW3c } from "./w3c-validation";
import {
  DEFAULT_SESSION_SIZE,
  isWindowView,
  parseSessionBounds,
  serializeSessionBounds,
  type WindowSize,
  type WindowView
} from "./window-sizing";
import { resolveRunningAppSource } from "./source-resolver";
import { AppStylesController } from "./app-styles";
import { DevToolsController } from "./devtools-controller";
import { SavedWorkspaceStore } from "./saved-workspaces";
import { SassWatchManager } from "./sass-watch";
import { SassPreferences } from "./sass-preferences";
import type {
  ProjectInfo,
  ViteServerInfo,
  ViteToggleResult,
  WorkspaceIdentity
} from "@shared/types";
import {
  absoluteFilePath,
  confinedPath,
  fileContent,
  relativePath,
  workspaceId
} from "./workspace-security";
import {
  activationGeneration,
  directoryPath,
  optionalRuntimeId,
  sessionId,
  workspacePath
} from "./ipc-schema";
import { registerFilesIpc } from "./ipc/files";
import { registerAgentIpc } from "./ipc/agent";
import { registerTerminalIpc } from "./ipc/terminal";
import { applyExecPath } from "./exec-path";
import { RuntimeSessionIndex, previousRuntimeSessionPath } from "./runtimes/runtime-session-index";

declare const __MIRA_RELEASE_BUILD__: boolean;

app.setName("Mira");
const customUserDataPath = app.commandLine.getSwitchValue("user-data-dir");
const legacyUserDataPath = ["Orbit", "orbit"]
  .map((name) => path.join(app.getPath("appData"), name))
  .find((candidate) => existsSync(candidate));
if (!customUserDataPath && legacyUserDataPath) {
  app.setPath("userData", legacyUserDataPath);
}

// Must run before the first runtime probe or PTY spawn: GUI-launched builds do
// not inherit the user's shell PATH (see exec-path.ts).
applyExecPath();

const savedWorkspaceStore = new SavedWorkspaceStore(path.join(app.getPath("userData"), "saved-workspaces.json"));
const runtimeSessionIndex = new RuntimeSessionIndex(
  path.join(app.getPath("userData"), "runtime-sessions.json"),
  customUserDataPath ? undefined : previousRuntimeSessionPath()
);
const backend = new OpenShellBackend(undefined, undefined, runtimeSessionIndex, savedWorkspaceStore);
const terminals = new TerminalManager();
const mobileServer = new MobileServer({ cwd: app.getAppPath() });
let mobileServerStartup: Promise<void> = Promise.resolve();
const viteCommand = resolveViteCommand(app.getAppPath(), __dirname);
const viteServers = new VitePreviewManager(defaultViteDeps(viteCommand.command, viteCommand.prefix));
let win: BrowserWindow | null = null;
let unsavedEditorFileNames: string[] = [];
let editorSaveSequence = 0;
let pendingEditorSave: { id: string; finish: (saved: boolean) => void } | null = null;
let closeRequestPending = false;
let appQuitRequested = false;
let trustedLocation: TrustedApplicationLocation | null = null;
const sassWatchers = new SassWatchManager(new SassPreferences(path.join(app.getPath("userData"), "sass-watch.json")), (state) => {
  if (win && !win.isDestroyed() && !win.webContents.isDestroyed()) win.webContents.send("shell:message", { kind: "sass-watch", state });
});

const pendingOpenPaths = new PendingOpenPaths();

const isMiraReleaseBuild = app.isPackaged && __MIRA_RELEASE_BUILD__;
const miraAppUpdater = isMiraReleaseBuild
  ? createMiraAppUpdater(autoUpdater, app.getVersion())
  : createDisabledMiraAppUpdater(app.getVersion());
if (isMiraReleaseBuild) {
  autoUpdater.on("error", (error) => console.error("[mira] release updater error:", error.message));
}

function flushOpenPaths(): void {
  if (pendingOpenPaths.size === 0) return;
  if (!win || win.isDestroyed() || win.webContents.isDestroyed()) return;
  if (win.webContents.isLoading()) return;
  const paths = pendingOpenPaths.takeIfRendererReady();
  if (paths.length === 0) return;
  win.webContents.send("shell:message", { kind: "ui-command", command: "open-paths", data: paths });
}

const appIconPath = (() => {
  const fromApp = path.join(app.getAppPath(), "resources", "icon.png");
  const fromOut = path.join(__dirname, "../../resources/icon.png");
  return existsSync(fromApp) ? fromApp : fromOut;
})();
const appUpdate = new AppUpdateController(miraAppUpdater, isMiraReleaseBuild, appIconPath, () => win, () => unsavedEditorFileNames);
const devTools = new DevToolsController((file, title) => resolveRunningAppSource(__dirname, file, title));
const appStyles = new AppStylesController(path.resolve(__dirname, "../.."),
  data => { if (win && !win.isDestroyed()) win.webContents.send("shell:message", { kind: "ui-command", command: "reload-app-styles", data }); },
  data => { if (win && !win.isDestroyed()) win.webContents.send("shell:message", { kind: "ui-command", command: "app-styles-error", data }); }
);
function attachAppInspection(wc: WebContents): void {
  devTools.attach(wc);
  if (!process.env["ELECTRON_RENDERER_URL"]) wc.on("did-finish-load", () => { void appStyles.start().then(() => appStyles.publishCurrent()); });
}
const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const LANDING_WIDTH = 380;
const LANDING_HEIGHT = 320;

function windowBoundsPath(): string {
  return path.join(app.getPath("userData"), "window-bounds.json");
}

function loadSessionBounds(): WindowSize | null {
  return parseSessionBounds(readFileSyncSafe(windowBoundsPath()) || null);
}

let windowView: WindowView = "landing";

let saveBoundsTimer: NodeJS.Timeout | null = null;

function scheduleBoundsSave(win: BrowserWindow): void {
  if (windowView !== "session") return;
  if (saveBoundsTimer) clearTimeout(saveBoundsTimer);
  saveBoundsTimer = setTimeout(() => {
    saveBoundsTimer = null;
    try {
      if (win.isDestroyed() || win.isMinimized() || win.isMaximized() || win.isFullScreen()) return;
      const [width, height] = win.getSize();
      writeFileSyncSafe(windowBoundsPath(), serializeSessionBounds({ width, height }));
    } catch {
      // persistence is best-effort
    }
  }, 400);
}

function applyWindowView(nextView: WindowView): void {
  windowView = nextView;
  if (!win || win.isDestroyed()) return;
  const size =
    nextView === "landing"
      ? { width: LANDING_WIDTH, height: LANDING_HEIGHT }
      : loadSessionBounds() ?? DEFAULT_SESSION_SIZE;
  if (!win.isMaximized() && !win.isFullScreen()) {
    win.setSize(size.width, size.height);
    // setSize preserves the top-left corner, so a landing → session grow
    // would otherwise extend down-right and look like a bottom-right
    // teleport. Re-center so the saved session size appears centered.
    win.center();
  }
}

function readFileSyncSafe(file: string): string {
  try {
    return require("node:fs").readFileSync(file, "utf8");
  } catch {
    return "";
  }
}

function writeFileSyncSafe(file: string, content: string): void {
  require("node:fs").writeFileSync(file, content);
}

function createWindow(show = true): BrowserWindow {
  const packagedUrl = pathToFileURL(path.join(__dirname, "../renderer/index.html")).href;
  const rendererUrl = applicationUrl(app.isPackaged, process.env["ELECTRON_RENDERER_URL"], packagedUrl);
  const location = trustedApplicationLocation(rendererUrl);
  const newWin = new BrowserWindow({
    width: LANDING_WIDTH,
    height: LANDING_HEIGHT,
    minWidth: 200,
    minHeight: 200,
    title: "Mira",
    icon: appIconPath,
    transparent: true,
    backgroundColor: "#00000000",
    vibrancy: process.platform === "darwin" ? "under-window" : undefined,
    visualEffectState: process.platform === "darwin" ? "active" : undefined,
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "default",
    movable: true,
    resizable: true,
    fullscreenable: true,
    show: false,
    webPreferences: {
      preload: path.join(__dirname, "../preload/index.js"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      webviewTag: true,
      spellcheck: false,
      backgroundThrottling: false,
      additionalArguments: [
        ...(app.isPackaged ? ["--openshell-packaged"] : []),
        `--mira-app-version=${app.getVersion()}`,
        ...(isMiraReleaseBuild ? ["--mira-release-build"] : [])
      ]
    }
  });
  win = newWin;
  trustedLocation = location;
  devTools.cancel();
  // Boot centered at the landing size so the first paint never anchors
  // to a cascaded corner before the session view applies its saved size.
  newWin.center();
  const wc = newWin.webContents;

  wc.on("will-prevent-unload", (event) => {
    if (pendingEditorSave) return;
    const files = unsavedEditorFileNames.slice(0, 16).map(briefFilePath);
    const action = appQuitRequested ? "quitting" : closeRequestPending ? "closing" : "reloading";
    const choice = dialog.showMessageBoxSync(newWin, {
      type: "warning",
      title: "Unsaved Changes",
      message: `Do you want to save your changes before ${action} Mira?`,
      detail: `${files.map((file) => `• ${file}`).join("\n")}${unsavedEditorFileNames.length > files.length ? `\n• and ${unsavedEditorFileNames.length - files.length} more` : ""}`,
      buttons: ["Save All", "Don't Save", "Cancel"],
      defaultId: 0,
      cancelId: 2
    });
    if (choice === 1) {
      event.preventDefault();
      return;
    }
    const closing = closeRequestPending;
    const quitRequested = appQuitRequested;
    closeRequestPending = false;
    appQuitRequested = false;
    if (choice !== 0) return;
    const id = String(++editorSaveSequence);
    const saved = new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => pendingEditorSave?.id === id && pendingEditorSave.finish(false), 30_000);
      pendingEditorSave = { id, finish: (success) => {
        clearTimeout(timer);
        pendingEditorSave = null;
        resolve(success);
      } };
      wc.send("shell:save-editor-files-request", id);
    });
    void saved.then((success) => {
      if (newWin.isDestroyed()) return;
      if (!success) {
        closeRequestPending = false;
        appQuitRequested = false;
        void dialog.showMessageBox(newWin, {
          type: "error", title: "Files could not be saved",
          message: "Mira stayed open because some files could not be saved.",
          detail: "Your edits are still in the editor. Check the save error before trying again.", buttons: ["OK"]
        });
        return;
      }
      unsavedEditorFileNames = [];
      appQuitRequested = quitRequested;
      if (closing || quitRequested) newWin.close();
      else wc.reload();
    });
  });

  wc.on("console-message", (event) => {
    console.log(`[renderer:${event.level}] ${event.message} (${event.sourceId}:${event.lineNumber})`);
  });
  newWin.on("unresponsive", () => console.warn("[mira] renderer is unresponsive"));
  let lastRendererReload = 0;
  wc.on("render-process-gone", (_event, details) => {
    console.error("[mira] renderer process gone:", details.reason, details.exitCode);
    if (newWin.isDestroyed() || Date.now() - lastRendererReload < 10_000) return;
    lastRendererReload = Date.now();
    wc.reload();
  });

  newWin.on("closed", () => {
    closeRequestPending = false;
    if (win === newWin) {
      win = null;
      trustedLocation = null;
    }
    if (appQuitRequested) {
      appQuitRequested = false;
      app.quit();
    }
  });
  newWin.on("close", () => {
    closeRequestPending = true;
  });

  newWin.webContents.setWindowOpenHandler(({ url }) => {
    const external = safeExternalUrl(url);
    if (external) void shell.openExternal(external);
    return { action: "deny" };
  });
  wc.on("will-navigate", (event, url) => {
    if (!isAllowedMainFrameNavigation(url, true, location)) event.preventDefault();
  });
  wc.on("will-redirect", (event, url, _isInPlace, isMainFrame) => {
    if (!isAllowedMainFrameNavigation(url, isMainFrame, location)) event.preventDefault();
  });

  attachBrowserGuards(wc);

  attachAppInspection(wc);

  newWin.webContents.on("before-input-event", (event, input) => {
    const mod = process.platform === "darwin" ? input.meta : input.control;
    if (input.type === "keyDown" && mod && !input.alt && !input.shift && input.key.toLowerCase() === "w") {
      // Preserve the shortcut so it cannot close Mira; editor wrapping is always on.
      event.preventDefault();
      return;
    }
    devTools.handleInput(wc, event, input);
  });

  if (show) {
    let revealed = false;
    const reveal = (): void => {
      if (revealed || newWin.isDestroyed()) return;
      revealed = true;
      newWin.show();
    };
    newWin.once("ready-to-show", reveal);
    setTimeout(reveal, 5000);
  }
  newWin.on("resize", () => scheduleBoundsSave(newWin));
  newWin.webContents.on("did-start-navigation", (_event, _url, isInPlace, isMainFrame) => {
    if (isMainFrame && !isInPlace) pendingOpenPaths.setRendererReady(false);
  });
  newWin.webContents.on("did-finish-load", flushOpenPaths);
  void newWin.loadURL(rendererUrl);
  return newWin;
}

async function runTrustBoundarySmoke(): Promise<void> {
  const trustedWindow = createWindow(false);
  await new Promise<void>((resolve, reject) => {
    trustedWindow.webContents.once("did-finish-load", () => resolve());
    trustedWindow.webContents.once("did-fail-load", (_event, code, description) => reject(new Error(`${code}: ${description}`)));
  });
  const trustedUrl = trustedWindow.webContents.getURL();
  const trustedIpc = await trustedWindow.webContents.executeJavaScript("window.openshell.state().then(() => true)");
  await trustedWindow.webContents.executeJavaScript("location.href = 'data:text/html,untrusted'");
  await sleep(100);
  const navigationDenied = trustedWindow.webContents.getURL() === trustedUrl;
  const windowsBeforePopup = BrowserWindow.getAllWindows().length;
  await trustedWindow.webContents.executeJavaScript("window.open('custom-protocol://unsafe')");
  await sleep(100);
  const popupDenied = BrowserWindow.getAllWindows().length === windowsBeforePopup;

  const untrustedWindow = new BrowserWindow({
    show: false,
    webPreferences: {
      preload: path.join(__dirname, "../preload/index.js"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true
    }
  });
  await untrustedWindow.loadURL("data:text/html,<title>untrusted</title>");
  const untrustedIpcRejected = await untrustedWindow.webContents
    .executeJavaScript("window.openshell.state().then(() => false, () => true)");
  untrustedWindow.destroy();
  trustedWindow.destroy();
  const result = { trustedIpc, navigationDenied, popupDenied, untrustedIpcRejected };
  console.log(`[openshell-trust-smoke] ${JSON.stringify(result)}`);
  if (!Object.values(result).every(Boolean)) throw new Error(`trust smoke failed: ${JSON.stringify(result)}`);
}

async function installApplication(): Promise<{ ok: boolean; message: string }> {
  if (process.platform !== "darwin") return { ok: false, message: "Install app is macOS-only" };
  if (app.isPackaged) return { ok: false, message: "Mira is already running as a packaged app" };
  const script = path.join(app.getAppPath(), "scripts", "install-app.mjs");
  const run = await new Promise<{ code: number | null; stdout: string; stderr: string }>((resolve, reject) => {
    const child = spawn(process.execPath, [script], {
      env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" }
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk: Buffer) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk.toString();
    });
    child.on("error", reject);
    child.on("close", (code) => resolve({ code, stdout, stderr }));
  });
  for (const line of run.stdout.trim().split("\n").reverse()) {
    try {
      const parsed = JSON.parse(line) as { ok?: unknown; message?: unknown };
      if (typeof parsed.ok === "boolean" && typeof parsed.message === "string") {
        return { ok: parsed.ok, message: parsed.message };
      }
    } catch {
      continue;
    }
  }
  return { ok: false, message: run.stderr.trim() || `Install app failed (exit code ${run.code ?? "unknown"})` };
}

function handleTrusted<Args extends unknown[], Result>(
  channel: string,
  listener: (event: IpcMainInvokeEvent, ...args: Args) => Result
): void {
  ipcMain.handle(channel, (event, ...args) => {
    if (!trustedLocation || !isTrustedIpcSender(event, win?.webContents ?? null, trustedLocation)) {
      throw new Error("Rejected IPC from an untrusted sender");
    }
    return listener(event, ...args as Args);
  });
}

async function workspaceDirectoryExists(directory: string): Promise<boolean> {
  try {
    return (await fsp.stat(directory)).isDirectory();
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    if (code === "ENOENT" || code === "ENOTDIR") return false;
    throw error;
  }
}

async function chooseWorkspaceDirectory(missingDirectory: string): Promise<string | null> {
  let defaultPath = path.dirname(path.resolve(missingDirectory));
  while (!(await workspaceDirectoryExists(defaultPath))) {
    const parent = path.dirname(defaultPath);
    if (parent === defaultPath) {
      defaultPath = app.getPath("home");
      break;
    }
    defaultPath = parent;
  }
  const result = await (win && !win.isDestroyed()
    ? dialog.showOpenDialog(win, {
        title: "Locate workspace folder",
        message: `Mira could not find this workspace. Choose its current folder to continue.\n\n${missingDirectory}`,
        buttonLabel: "Use this folder",
        defaultPath,
        properties: ["openDirectory"]
      })
    : dialog.showOpenDialog({
        title: "Locate workspace folder",
        message: `Mira could not find this workspace. Choose its current folder to continue.\n\n${missingDirectory}`,
        buttonLabel: "Use this folder",
        defaultPath,
        properties: ["openDirectory"]
      }));
  return result.canceled ? null : result.filePaths[0] ?? null;
}

function viteEntry(fileName: string): string {
  return fileName.toLowerCase() === "index.html" ? "" : fileName;
}

/**
 * Resolve which folder and page a Vite toggle should serve for a workspace:
 * an explicit confined HTML entry when the renderer supplies one, otherwise the
 * workspace root, or the shallowest HTML page when the root has no index.html.
 */
async function resolveViteTarget(workspace: WorkspaceIdentity, requestedEntry?: string): Promise<{ directory: string; entry: string }> {
  const directory = await backend.workspaceDirectory(workspace);
  if (requestedEntry !== undefined) {
    const absolute = await confinedPath(directory, relativePath(requestedEntry));
    const stat = await fsp.stat(absolute).catch(() => null);
    if (!stat?.isFile() || !/\.html?$/i.test(absolute)) throw new Error("the Vite preview target must be an existing HTML file");
    return { directory: path.dirname(absolute), entry: viteEntry(path.basename(absolute)) };
  }
  const rootIndex = await fsp.stat(path.join(directory, "index.html")).catch(() => null);
  if (!rootIndex?.isFile()) {
    const found = await findHtmlEntry(directory);
    if (found) return { directory: path.dirname(found), entry: viteEntry(path.basename(found)) };
  }
  return { directory, entry: "" };
}

function registerIpc(): void {
  registerFilesIpc(handleTrusted, backend);
  registerAgentIpc(handleTrusted, backend);
  registerTerminalIpc(handleTrusted, backend, terminals);
  handleTrusted("shell:select-folder", async (e, requestGeneration: number, requestedRuntimeID?: unknown) => {
    const generation = backend.beginActivation(activationGeneration(requestGeneration));
    const parent = BrowserWindow.fromWebContents(e.sender);
    const result = await dialog.showOpenDialog(parent ?? win!, {
      title: "Open a repository folder",
      properties: ["openDirectory"]
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    return backend.openSession(result.filePaths[0], generation, optionalRuntimeId(requestedRuntimeID));
  });

  handleTrusted("shell:select-directory", async (e) => {
    const parent = BrowserWindow.fromWebContents(e.sender);
    const result = await dialog.showOpenDialog(parent ?? win!, {
      title: "Save a workspace in Mira",
      properties: ["openDirectory"]
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    return fsp.realpath(directoryPath(result.filePaths[0]));
  });

  handleTrusted("shell:select-file", async (e, requestGeneration: number, requestedRuntimeID?: unknown) => {
    const generation = backend.beginActivation(activationGeneration(requestGeneration));
    const parent = BrowserWindow.fromWebContents(e.sender);
    const result = await dialog.showOpenDialog(parent ?? win!, {
      title: "Open a file",
      properties: ["openFile"]
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    return backend.openFileWorkspace(result.filePaths[0], generation, optionalRuntimeId(requestedRuntimeID));
  });

  handleTrusted("shell:open-file", async (_e, file: string, requestGeneration: number, requestedRuntimeID?: unknown) =>
    backend.openFileWorkspace(absoluteFilePath(file), backend.beginActivation(activationGeneration(requestGeneration)), optionalRuntimeId(requestedRuntimeID)));

  handleTrusted("shell:open-session", async (_e, dir: string, requestGeneration: number, requestedRuntimeID?: unknown) => {
    const requestedDirectory = directoryPath(dir);
    const generation = backend.beginActivation(activationGeneration(requestGeneration));
    const runtimeID = optionalRuntimeId(requestedRuntimeID);
    const directory = await backend.resolveWorkspaceDirectory(requestedDirectory)
      ?? await chooseWorkspaceDirectory(requestedDirectory);
    if (!directory) throw new Error(`Opening the workspace was canceled. Its folder is missing: ${requestedDirectory}`);
    const session = await backend.openSession(directory, generation, runtimeID);
    if (requestedDirectory !== session.directory) await savedWorkspaceStore.relocate(requestedDirectory, session.directory);
    return session;
  });

  handleTrusted("shell:sessions", async () => backend.listSessions());

  handleTrusted("shell:active-sessions", async () => backend.activeSessions());

  handleTrusted("shell:close-session", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    viteServers.stopWorkspace(workspace.id);
    await sassWatchers.unregister(workspace);
    return backend.closeSession(workspace);
  });

  handleTrusted("shell:open-session-id", async (
    _e,
    sessionID: string,
    requestGeneration: number,
    requestedRuntimeID?: unknown,
    allowWorkspaceRelink?: unknown
  ) => {
    const id = sessionId(sessionID);
    const generation = backend.beginActivation(activationGeneration(requestGeneration));
    const runtimeID = optionalRuntimeId(requestedRuntimeID);
    try {
      return await backend.openSessionById(id, generation, runtimeID);
    } catch (error) {
      if (allowWorkspaceRelink !== true || !(error instanceof SessionWorkspaceNotFoundError)) throw error;
      const directory = await chooseWorkspaceDirectory(error.directory);
      if (!directory) throw new Error(`Reopening the session was canceled. Its workspace is missing: ${error.directory}`);
      return backend.openSessionByIdInDirectory(id, directory, generation, runtimeID);
    }
  });

  handleTrusted("shell:delete-session", async (_e, sessionID: string) => {
    sessionId(sessionID);
    await sassWatchers.stopSession(sessionID);
    return backend.deleteSession(sessionID);
  });

  const registerSassWorkspace = async (workspace: WorkspaceIdentity): Promise<void> => {
    workspaceId(workspace);
    const session = (await backend.activeSessions()).find((entry) => entry.workspace.id === workspace.id && entry.workspace.generation === workspace.generation);
    if (!session) throw new Error("stale workspace");
    try { sassWatchers.state(workspace); return; } catch { /* Register only new identities; a missing folder must not prevent stopping an existing watcher. */ }
    await sassWatchers.register(session);
  };
  handleTrusted("shell:sass-watch-state", async (_e, workspace: WorkspaceIdentity) => {
    await registerSassWorkspace(workspace);
    return sassWatchers.state(workspace);
  });
  handleTrusted("shell:sass-watch-set", async (_e, workspace: WorkspaceIdentity, enabled: unknown) => {
    if (typeof enabled !== "boolean") throw new Error("Sass watching must be on or off");
    await registerSassWorkspace(workspace);
    return sassWatchers.setEnabled(workspace, enabled);
  });

  handleTrusted("shell:runtimes", async () => backend.runtimeManifests());

  handleTrusted("shell:sync-opencode", async () => backend.syncOpenCode());

  handleTrusted("shell:update-opencode", async () => backend.updateOpenCode());

  handleTrusted("shell:app-update-check", async () => miraAppUpdater.check());

  handleTrusted("shell:unsaved-editor-files", async (_e, paths: unknown) => {
    unsavedEditorFileNames = Array.isArray(paths)
      ? [...new Set(paths.slice(0, 256).filter((value): value is string =>
        typeof value === "string" && value.length > 0 && value.length <= 2_048 && !/[\u0000-\u001f\u007f]/.test(value)
      ))].slice(0, 256)
      : [];
  });

  handleTrusted("shell:save-editor-files-result", async (_e, requestID: unknown, saved: unknown) => {
    if (typeof requestID === "string" && pendingEditorSave?.id === requestID) pendingEditorSave.finish(saved === true);
  });

  handleTrusted("shell:app-update", async () => appUpdate.update());

  handleTrusted("shell:select-files", async (e) => {
    const parent = BrowserWindow.fromWebContents(e.sender);
    const result = await dialog.showOpenDialog(parent ?? win!, {
      title: "Attach files",
      properties: ["openFile", "multiSelections"]
    });
    return result.canceled ? [] : result.filePaths;
  });

  handleTrusted("shell:select-images", async (e) => {
    const parent = BrowserWindow.fromWebContents(e.sender);
    const result = await dialog.showOpenDialog(parent ?? win!, {
      title: "Upload images",
      properties: ["openFile", "multiSelections"],
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif", "ico", "tiff", "tif"] }]
    });
    return result.canceled ? [] : result.filePaths;
  });

  handleTrusted("shell:read-image-preview", async (_e, file: string) => {
    try {
      const image = nativeImage.createFromPath(absoluteFilePath(file));
      if (image.isEmpty()) return null;
      const { width } = image.getSize();
      return (width > 128 ? image.resize({ width: 128 }) : image).toDataURL();
    } catch {
      return null;
    }
  });

  handleTrusted("shell:projects", async () => backend.listProjects());

  handleTrusted("shell:saved-workspaces", async () => {
    const stored = await savedWorkspaceStore.load();
    // History carries project IDs for legacy paths whose catalog already moved.
    if (stored.workspaces.some((workspace) => !existsSync(workspace.directory))) await backend.listSessions().catch(() => []);
    for (const workspace of stored.workspaces) await backend.resolveWorkspaceDirectory(workspace.directory).catch(() => null);
    return savedWorkspaceStore.load();
  });

  handleTrusted("shell:saved-workspaces-save", async (_e, workspaces: ProjectInfo[]) => savedWorkspaceStore.save(workspaces));

  handleTrusted("shell:mobile-setup-status", async () => {
    await mobileServerStartup;
    return mobileServer.getSetupStatus();
  });
  handleTrusted("shell:mobile-pairing-qr", async () => {
    await mobileServerStartup;
    return mobileServer.createPairingQr();
  });

  handleTrusted("shell:health", async () => { appStyles.publishCurrent(); return backend.connect().catch(() => false); });

  handleTrusted("shell:window-view", (_e, view: unknown) => {
    if (!isWindowView(view)) return;
    applyWindowView(view);
  });

  // The native appearance tracks the active theme — only the Paper profile is
  // light — so window vibrancy and native chrome never follow a mismatched
  // system appearance.
  handleTrusted("shell:set-appearance", (_e, appearance: unknown) => {
    nativeTheme.themeSource = appearance === "light" ? "light" : "dark";
  });

  handleTrusted("shell:install-app", async () => installApplication());

  handleTrusted("shell:validate-w3c", async (_e, filePath: string, content: string) => {
    const path = directoryPath(filePath);
    const source = fileContent(content);
    return validateWithW3c(path, source);
  });

  handleTrusted("shell:vite-toggle", async (_e, workspace: WorkspaceIdentity, requestedEntry?: string): Promise<ViteToggleResult> => {
    workspaceId(workspace);
    const target = await resolveViteTarget(workspace, requestedEntry);
    const id = viteServerKey(workspace.id, target.directory, target.entry);
    if (viteServers.running(id)) {
      viteServers.stop(id);
      return { running: false };
    }
    const preview = await viteServers.start(id, workspace.id, target.directory, target.entry);
    void shell.openExternal(preview.url);
    return {
      running: true,
      server: { id, workspaceId: workspace.id, directory: target.directory, entry: target.entry, url: preview.url, port: preview.port }
    };
  });

  handleTrusted("shell:vite-servers", (): ViteServerInfo[] => viteServers.list());

  handleTrusted("shell:vite-stop", (_e, serverID: unknown) => {
    if (typeof serverID !== "string" || !serverID) throw new Error("invalid Vite server id");
    viteServers.stop(serverID);
  });

  handleTrusted("shell:vite-stop-all", async () => {
    await viteServers.stopAll();
  });

  handleTrusted("shell:take-pending-paths", () => {
    // The renderer invokes this only after installing its message listener.
    pendingOpenPaths.setRendererReady(true);
    return pendingOpenPaths.take();
  });
}

if (!app.requestSingleInstanceLock()) {
  console.log("[mira] another instance is already running — exiting; the running instance will open or focus its window");
  app.quit();
} else {
  process.on("uncaughtException", (err) => {
    console.error("uncaughtException:", err);
  });
  process.on("unhandledRejection", (reason) => {
    console.error("unhandledRejection:", reason);
  });

  app.on("second-instance", (_event, argv) => {
    pendingOpenPaths.push(collectLaunchPaths(argv.slice(1), existsSync, process.execPath));
    if (!app.isReady()) return;
    if (!win || win.isDestroyed()) {
      createWindow();
    } else {
      if (win.isMinimized()) win.restore();
      win.focus();
    }
    flushOpenPaths();
  });

  app.on("open-file", (event, path) => {
    event.preventDefault();
    pendingOpenPaths.push([path]);
    if (!app.isReady()) return;
    if (!win || win.isDestroyed()) {
      createWindow();
    } else {
      if (win.isMinimized()) win.restore();
      win.focus();
    }
    flushOpenPaths();
  });

  app.whenReady().then(() => {
    // Window vibrancy and native chrome follow the native appearance, so pin
    // the dark default before any window exists; the renderer reports the
    // persisted theme through `shell:set-appearance` right after boot.
    nativeTheme.themeSource = "dark";
    if (process.platform === "darwin") {
      try {
        app.dock?.setIcon(appIconPath);
      } catch {
        // icon is cosmetic
      }
    }
    const trustSmoke = process.env["OPENSHELL_TRUST_SMOKE"] === "1";
    if (!trustSmoke) backend.start();
    const pendingFileUpdates = new Map<string, unknown>();
    let fileUpdateFlush: ReturnType<typeof setTimeout> | null = null;
    const flushFileUpdates = (): void => {
      fileUpdateFlush = null;
      const updates = [...pendingFileUpdates.values()];
      pendingFileUpdates.clear();
      for (const update of updates) {
        if (win && !win.isDestroyed() && !win.webContents.isDestroyed()) win.webContents.send("shell:message", update);
      }
    };
    const fwd = (msg: unknown): void => {
      if (msg && typeof msg === "object") {
        const message = msg as { kind?: string; type?: string; session?: import("@shared/types").SessionInfo; data?: { sessionID?: string; id?: string } };
        if (message.kind === "session" && message.session) void sassWatchers.register(message.session).catch((error) => console.warn("[sass]", String(error)));
        if (message.kind === "event" && message.type === "session.deleted") void sassWatchers.stopSession(message.data?.sessionID ?? message.data?.id ?? "");
      }
      if (!win || win.isDestroyed() || win.webContents.isDestroyed()) return;
      if (typeof msg === "object" && msg !== null && (msg as { kind?: string }).kind === "file-update") {
        const file = (msg as { file?: { workspace?: { id?: string }; path?: string } }).file;
        const key = `${file?.workspace?.id ?? ""}:${file?.path ?? ""}`;
        pendingFileUpdates.set(key, msg);
        if (!fileUpdateFlush) fileUpdateFlush = setTimeout(flushFileUpdates, 16);
        return;
      }
      win.webContents.send("shell:message", msg);
    };
    backend.onMessage(fwd);
    terminals.onMessage(fwd);
    registerIpc();
    if (trustSmoke) {
      void runTrustBoundarySmoke().then(
        () => app.quit(),
        (error) => {
          console.error("[openshell-trust-smoke]", error);
          process.exitCode = 1;
          app.quit();
        }
      );
      return;
    }
    createWindow();
    if (app.isPackaged) pendingOpenPaths.push(collectLaunchPaths(process.argv.slice(1), existsSync, process.execPath));
    mobileServerStartup = backend.connect()
      .then((connected) => {
        // Share the desktop's OpenCode daemon (and therefore its sessions) with
        // the mobile server when available.
        mobileServer.start(connected ? backend.mobileEndpoint() ?? undefined : undefined);
      })
      .catch(() => {
        mobileServer.start();
      });

    app.on("activate", () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        createWindow();
      }
    });
  });
}

app.on("window-all-closed", () => {
  // macOS would normally keep the app alive with no window; Mira must not
  // linger invisibly in the background (macOS then files it as a background
  // task), so closing the last window quits on every platform. The backend,
  // terminals, Vite servers and mobile server are stopped in `before-quit`.
  app.quit();
});

let quitting = false;
let shutdownStarted = false;

const QUIT_TIMEOUT_MS = 10_000;

app.on("before-quit", (event) => {
  if (quitting) return;
  event.preventDefault();
  if (win && !win.isDestroyed()) {
    appQuitRequested = true;
    if (!closeRequestPending) {
      closeRequestPending = true;
      win.close();
    }
    return;
  }
  if (shutdownStarted) return;
  shutdownStarted = true;
  void (async () => {
    // Bounded shutdown: terminals must reap their children before Node
    // teardown (a late pty exit callback aborts the process), but a wedged
    // child must never block quit forever.
    const shutdown = (async () => {
      await appStyles.stop();
      await sassWatchers.stopAll();
      await backend.stop();
      await terminals.stopAll();
      await viteServers.stopAll();
      await mobileServer.stop();
    })().catch(() => {});
    await Promise.race([
      shutdown,
      new Promise<void>((resolve) => setTimeout(resolve, QUIT_TIMEOUT_MS))
    ]);
    quitting = true;
    app.quit();
  })();
});
