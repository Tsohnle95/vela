import type { BrowserWindow } from "electron";
import { EventEmitter } from "node:events";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaAppUpdater } from "./app-updater";
import { AppUpdateController } from "./app-update-controller";

const state = vi.hoisted(() => ({ windows: [] as Array<{
  emit(name: string, ...args: unknown[]): boolean;
  options: Record<string, unknown>;
  webContents: EventEmitter & { executeJavaScript: ReturnType<typeof vi.fn>; setWindowOpenHandler: ReturnType<typeof vi.fn> };
  loadURL: ReturnType<typeof vi.fn>;
  close: ReturnType<typeof vi.fn>;
  destroy: ReturnType<typeof vi.fn>;
}>, loadFailure: false }));

vi.mock("electron", async () => {
  const { EventEmitter } = await import("node:events");
  return {
    app: { getVersion: () => "1.2.3" },
    BrowserWindow: class extends EventEmitter {
      options: Record<string, unknown>;
      webContents = Object.assign(new EventEmitter(), {
        isDestroyed: vi.fn(() => false),
        executeJavaScript: vi.fn(async () => undefined),
        setWindowOpenHandler: vi.fn()
      });
      isDestroyed = vi.fn(() => false);
      close = vi.fn();
      destroy = vi.fn();
      show = vi.fn();
      focus = vi.fn();
      setTitle = vi.fn();
      loadURL = vi.fn(async () => { if (state.loadFailure) throw new Error("load failed"); });
      constructor(options: Record<string, unknown>) {
        super(); this.options = options; state.windows.push(this);
      }
    }
  };
});

function fixture(enabled = true) {
  let window = { isDestroyed: () => false, show: vi.fn(), focus: vi.fn(), hide: vi.fn() };
  const unsaved = vi.fn(() => [] as string[]);
  const updater = {
    check: vi.fn(),
    update: vi.fn<VelaAppUpdater["update"]>(async () => ({ ok: true, updated: false, message: "Current" })),
    installAndRestart: vi.fn()
  };
  const controller = new AppUpdateController(updater, enabled, "/icon.png", () => window as unknown as BrowserWindow, unsaved);
  return { controller, updater, unsaved, get window() { return window; }, replaceWindow() {
    window = { isDestroyed: () => false, show: vi.fn(), focus: vi.fn(), hide: vi.fn() };
  } };
}

describe("release update window lifecycle", () => {
  beforeEach(() => { vi.useFakeTimers(); state.windows.length = 0; state.loadFailure = false; });
  afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); });

  it("disables local builds before reading unsaved state or creating a window", async () => {
    const f = fixture(false);
    expect(await f.controller.update()).toMatchObject({ ok: false, updated: false, currentVersion: "1.2.3", message: expect.stringContaining("disabled") });
    expect(f.unsaved).not.toHaveBeenCalled();
    expect(f.updater.update).not.toHaveBeenCalled();
    expect(state.windows).toHaveLength(0);
  });

  it("preserves the eight-file unsaved guard and bounded path display", async () => {
    const f = fixture();
    f.unsaved.mockReturnValue(["a".repeat(400), ...Array.from({ length: 8 }, (_, i) => `${i}.ts`)]);
    const result = await f.controller.update();
    expect(result.message).toBe(`Save unsaved files before updating Vela: …${"a".repeat(383)}, 0.ts, 1.ts, 2.ts, 3.ts, 4.ts, 5.ts, 6.ts and 1 more.`);
    expect(state.windows).toHaveLength(0);
  });

  it("blocks competing updates and popup close/reload until the update finishes", async () => {
    const f = fixture();
    let finish!: (value: { ok: boolean; updated: boolean; message: string }) => void;
    f.updater.update.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    const pending = f.controller.update();
    await vi.advanceTimersByTimeAsync(0);
    expect(await f.controller.update()).toMatchObject({ ok: false, message: "A Vela update is already running." });
    const popup = state.windows[0];
    expect(popup.options.webPreferences).toEqual({ contextIsolation: true, nodeIntegration: false, sandbox: true, devTools: false });
    const event = { preventDefault: vi.fn() };
    popup.emit("close", event);
    popup.webContents.emit("before-input-event", event, { type: "keyDown", key: "F5" });
    expect(event.preventDefault).toHaveBeenCalledTimes(2);
    expect(popup.webContents.setWindowOpenHandler.mock.calls[0][0]()).toEqual({ action: "deny" });
    finish({ ok: false, updated: false, message: "Download failed" });
    await pending;
    popup.emit("close", event);
    expect(event.preventDefault).toHaveBeenCalledTimes(2);
    expect(f.window.show).toHaveBeenCalledOnce();
    expect(f.window.focus).toHaveBeenCalledOnce();
  });

  it("cleans up load failure and permits the next attempt without hiding main", async () => {
    const f = fixture();
    state.loadFailure = true;
    expect(await f.controller.update()).toEqual({ ok: false, updated: false, message: "Vela could not open its update window. load failed" });
    expect(state.windows[0].destroy).toHaveBeenCalledOnce();
    expect(f.window.hide).not.toHaveBeenCalled();
    expect(f.updater.update).not.toHaveBeenCalled();
    state.loadFailure = false;
    expect(await f.controller.update()).toMatchObject({ ok: true });
    expect(state.windows).toHaveLength(2);
  });

  it("restores main and closes an up-to-date popup only after 1800ms", async () => {
    const f = fixture();
    expect(await f.controller.update()).toEqual({ ok: true, updated: false, message: "Current" });
    expect(f.window.hide).toHaveBeenCalledOnce();
    expect(f.window.show).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(1799);
    expect(state.windows[0].close).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(state.windows[0].close).toHaveBeenCalledOnce();
  });

  it("preserves unexpected rejection and releases the running guard for retry", async () => {
    const f = fixture();
    f.updater.update.mockRejectedValueOnce(new Error("unexpected adapter failure"));
    await expect(f.controller.update()).rejects.toThrow("unexpected adapter failure");
    expect(f.window.hide).toHaveBeenCalledOnce();
    expect(f.window.show).not.toHaveBeenCalled();
    const event = { preventDefault: vi.fn() };
    state.windows[0].emit("close", event);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    await expect(f.controller.update()).resolves.toMatchObject({ ok: true });
    expect(state.windows[0].close).toHaveBeenCalledOnce();
    expect(state.windows).toHaveLength(2);
  });

  it("keeps the install delay and restores the current main window on install failure", async () => {
    const f = fixture();
    f.updater.update.mockResolvedValue({ ok: true, updated: true, message: "Downloaded" });
    f.updater.installAndRestart.mockImplementation(() => { throw new Error("install failed"); });
    await f.controller.update();
    const oldWindow = f.window;
    f.replaceWindow();
    await vi.advanceTimersByTimeAsync(1399);
    expect(f.updater.installAndRestart).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(f.updater.installAndRestart).toHaveBeenCalledOnce();
    expect(f.window.show).toHaveBeenCalledOnce();
    expect(oldWindow.show).not.toHaveBeenCalled();
    expect(state.windows[0].webContents.executeJavaScript.mock.calls.at(-1)?.[0]).toContain("install failed");
    const event = { preventDefault: vi.fn() };
    state.windows[0].webContents.emit("did-navigate-in-page", {}, "data:text/html#return-to-vela", true, 42, 1);
    expect(state.windows[0].close).toHaveBeenCalledOnce();
    state.windows[0].emit("close", event);
    expect(event.preventDefault).not.toHaveBeenCalled();
  });
});
