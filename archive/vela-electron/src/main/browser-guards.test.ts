import { EventEmitter } from "node:events";
import type { Session, WebContents, WebPreferences } from "electron";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { attachBrowserGuards } from "./browser-guards";
import { BROWSER_PARTITION } from "./browser-policy";

const mocks = vi.hoisted(() => ({
  fromPartition: vi.fn(), openExternal: vi.fn(), request: vi.fn(), check: vi.fn(), device: vi.fn()
}));
vi.mock("electron", () => ({ session: { fromPartition: mocks.fromPartition }, shell: { openExternal: mocks.openExternal } }));

function contents(): EventEmitter & { setWindowOpenHandler: ReturnType<typeof vi.fn> } {
  return Object.assign(new EventEmitter(), { setWindowOpenHandler: vi.fn() });
}

describe("browser guest guards", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.fromPartition.mockReturnValue({
      setPermissionRequestHandler: mocks.request,
      setPermissionCheckHandler: mocks.check,
      setDevicePermissionHandler: mocks.device
    });
    mocks.openExternal.mockResolvedValue(undefined);
  });

  it("denies permission checks, requests and devices before attachment", () => {
    const host = contents();
    attachBrowserGuards(host as unknown as WebContents);
    expect(mocks.fromPartition).toHaveBeenCalledWith(BROWSER_PARTITION);
    const callback = vi.fn();
    const request = mocks.request.mock.calls[0]![0] as Parameters<Session["setPermissionRequestHandler"]>[0];
    request!(host as unknown as WebContents, "media", callback, {} as Electron.MediaAccessPermissionRequest);
    expect(callback).toHaveBeenCalledWith(false);
    const check = mocks.check.mock.calls[0]![0] as Parameters<Session["setPermissionCheckHandler"]>[0];
    expect(check!(null, "usb", "https://example.com", {} as Electron.PermissionCheckHandlerHandlerDetails)).toBe(false);
    const device = mocks.device.mock.calls[0]![0] as Parameters<Session["setDevicePermissionHandler"]>[0];
    expect(device!({} as Electron.DevicePermissionHandlerHandlerDetails)).toBe(false);
  });

  it("blocks forbidden attachment and strips privilege before permitted attachment", () => {
    const host = contents();
    attachBrowserGuards(host as unknown as WebContents);
    const event = { preventDefault: vi.fn() };
    host.emit("will-attach-webview", event, {}, { src: "file:///tmp/secret" });
    expect(event.preventDefault).toHaveBeenCalledOnce();
    event.preventDefault.mockClear();
    const preferences: WebPreferences = { preload: "/app/preload.js", nodeIntegration: true };
    const params = { src: "https://example.com", partition: "persist:default" };
    host.emit("will-attach-webview", event, preferences, params);
    expect(params.partition).toBe(BROWSER_PARTITION);
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(preferences).toMatchObject({ sandbox: true, nodeIntegration: false, partition: BROWSER_PARTITION });
    expect(preferences.preload).toBeUndefined();
  });

  it("blocks nonweb navigation and redirects including child frames", () => {
    const host = contents();
    const guest = contents();
    attachBrowserGuards(host as unknown as WebContents);
    host.emit("did-attach-webview", {}, guest);
    for (const name of ["will-navigate", "will-redirect"]) {
      const event = { preventDefault: vi.fn() };
      guest.emit(name, event, "file:///tmp/secret", false, true, 1, 2);
      expect(event.preventDefault).toHaveBeenCalledOnce();
      event.preventDefault.mockClear();
      guest.emit(name, event, "http://localhost:5173", false, true, 1, 2);
      expect(event.preventDefault).not.toHaveBeenCalled();
    }
    const frameEvent = { url: "javascript:alert(1)", isMainFrame: false, preventDefault: vi.fn() };
    guest.emit("will-frame-navigate", frameEvent);
    expect(frameEvent.preventDefault).toHaveBeenCalledOnce();
  });

  it("denies every popup and opens only credential-free HTTPS externally", () => {
    const host = contents();
    const guest = contents();
    attachBrowserGuards(host as unknown as WebContents);
    host.emit("did-attach-webview", {}, guest);
    const popup = guest.setWindowOpenHandler.mock.calls[0]![0] as Parameters<WebContents["setWindowOpenHandler"]>[0];
    for (const url of ["https://example.com/", "http://localhost:5173", "file:///tmp/secret", "https://user:pass@example.com", "vela://x"]) {
      expect(popup({ url } as Electron.HandlerDetails)).toEqual({ action: "deny" });
    }
    expect(mocks.openExternal).toHaveBeenCalledExactlyOnceWith("https://example.com/");
  });
});
