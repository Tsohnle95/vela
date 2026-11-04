import { EventEmitter } from "node:events";
import type { Input, WebContents } from "electron";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DevToolsController } from "./devtools-controller";

function fixture() {
  let opened = false;
  let attached = false;
  const debuggerEvents = new EventEmitter();
  const devtools = Object.assign(new EventEmitter(), {
    isDestroyed: vi.fn(() => false),
    isLoading: vi.fn(() => false),
    executeJavaScript: vi.fn(async (_script: string) => JSON.stringify({ key: null, source: null }))
  });
  const debuggerApi = Object.assign(debuggerEvents, {
    isAttached: vi.fn(() => attached),
    attach: vi.fn(() => { attached = true; }),
    sendCommand: vi.fn(async (method: string, _params?: unknown) => method === "DOM.getBoxModel"
      ? { model: { content: [10, 20, 30, 20, 30, 40, 10, 40] } }
      : {})
  });
  const wc = Object.assign(new EventEmitter(), {
    debugger: debuggerApi,
    devToolsWebContents: devtools,
    isDevToolsOpened: vi.fn(() => opened),
    isDestroyed: vi.fn(() => false),
    openDevTools: vi.fn(() => { opened = true; }),
    closeDevTools: vi.fn(() => { opened = false; wc.emit("devtools-closed"); }),
    focus: vi.fn(),
    inspectElement: vi.fn(),
    send: vi.fn()
  });
  const resolve = vi.fn(async () => "/app/src/renderer/src/styles/main.scss" as string | null);
  const controller = new DevToolsController(resolve);
  const contents = wc as unknown as WebContents;
  controller.attach(contents);
  const event = { preventDefault: vi.fn() };
  const key = (value: string, options: Partial<Input> = {}) => {
    controller.handleInput(contents, event, {
      key: value, type: "keyDown", alt: false, shift: false,
      meta: false, control: false, isAutoRepeat: false, ...options
    } as Input);
  };
  const activate = () => key("C", { shift: true, ...(process.platform === "darwin" ? { meta: true } : { control: true }) });
  return { wc, devtools, debuggerApi, resolve, controller, event, key, activate };
}

describe("DevTools controller", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); });

  it("keeps F12 docking and ignores modifier, key-up and auto-repeat picker shortcuts", async () => {
    const f = fixture();
    f.key("F12", { alt: true });
    f.key("F12", { type: "keyUp" });
    expect(f.wc.openDevTools).not.toHaveBeenCalled();
    f.key("F12");
    expect(f.wc.openDevTools).toHaveBeenCalledWith({ mode: "bottom" });
    f.key("F12");
    expect(f.wc.closeDevTools).toHaveBeenCalledOnce();
    f.key("C", { shift: true, meta: true, control: true, isAutoRepeat: true });
    await vi.advanceTimersByTimeAsync(0);
    expect(f.debuggerApi.attach).not.toHaveBeenCalled();
  });

  it("grants one selection and awaits overlay disable before selecting the box center", async () => {
    const f = fixture();
    f.activate();
    await vi.advanceTimersByTimeAsync(0);
    expect(f.debuggerApi.attach).toHaveBeenCalledWith("1.3");
    f.debuggerApi.emit("message", {}, "Overlay.inspectNodeRequested", { backendNodeId: 12 });
    f.debuggerApi.emit("message", {}, "Overlay.inspectNodeRequested", { backendNodeId: 12 });
    await vi.advanceTimersByTimeAsync(0);
    expect(f.wc.inspectElement).toHaveBeenCalledExactlyOnceWith(20, 30);
    expect(f.debuggerApi.sendCommand.mock.calls.map(([method]) => method)).toEqual([
      "DOM.enable", "Overlay.enable", "Overlay.setInspectMode", "Overlay.setInspectMode", "DOM.getBoxModel"
    ]);
    expect(f.debuggerApi.sendCommand.mock.calls[3][1]).toMatchObject({ mode: "none" });
  });

  it("cancels asynchronous setup without rearming an old activation", async () => {
    const f = fixture();
    let release!: () => void;
    f.debuggerApi.sendCommand.mockImplementationOnce(() => new Promise((resolve) => {
      release = () => resolve({});
    }));
    f.activate();
    await vi.advanceTimersByTimeAsync(0);
    f.controller.cancel();
    release();
    await vi.advanceTimersByTimeAsync(0);
    expect(f.debuggerApi.sendCommand).toHaveBeenCalledExactlyOnceWith("DOM.enable");
    f.debuggerApi.emit("message", {}, "Overlay.inspectNodeRequested", { backendNodeId: 2 });
    expect(f.wc.inspectElement).not.toHaveBeenCalled();
  });

  it("keeps protocol cancellation one-way and clears overlay authority on detach", async () => {
    const f = fixture();
    f.activate();
    await vi.advanceTimersByTimeAsync(0);
    const count = f.debuggerApi.sendCommand.mock.calls.length;
    f.debuggerApi.emit("message", {}, "Overlay.inspectModeCanceled", {});
    f.key("Escape");
    expect(f.debuggerApi.sendCommand).toHaveBeenCalledTimes(count);
    f.debuggerApi.emit("detach");
    f.wc.emit("devtools-closed");
    expect(f.debuggerApi.sendCommand).toHaveBeenCalledTimes(count);
  });

  it("stops pending picker setup on Escape while DevTools is loading", async () => {
    const f = fixture();
    f.devtools.isLoading.mockReturnValue(true);
    f.activate();
    f.key("Escape");
    f.devtools.emit("dom-ready");
    await vi.advanceTimersByTimeAsync(0);
    expect(f.debuggerApi.attach).not.toHaveBeenCalled();
    expect(f.event.preventDefault).toHaveBeenCalledTimes(2);
  });

  it("polls CSS source requests through the confined resolver, and stops on frontend F12", async () => {
    const f = fixture();
    f.key("F12");
    f.devtools.executeJavaScript.mockImplementation(async (script) => script.includes("const k =")
      ? JSON.stringify({ key: "F12", source: JSON.stringify({ file: "main.scss", line: 12, title: "main.scss:12" }) })
      : "");
    f.wc.emit("devtools-opened");
    await vi.advanceTimersByTimeAsync(120);
    expect(f.resolve).toHaveBeenCalledExactlyOnceWith("main.scss", "main.scss:12");
    expect(f.wc.send).toHaveBeenCalledExactlyOnceWith("shell:message", {
      kind: "ui-command", command: "open-source", path: "/app/src/renderer/src/styles/main.scss", line: 12
    });
    expect(f.wc.closeDevTools).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(240);
    expect(f.resolve).toHaveBeenCalledOnce();
  });

  it("does not dispatch unresolved or malformed source payloads", async () => {
    const f = fixture();
    f.resolve.mockResolvedValue(null);
    vi.spyOn(console, "warn").mockImplementation(() => {});
    f.key("F12");
    const payloads = ["bad json", JSON.stringify({ source: "{}" }), JSON.stringify({ source: JSON.stringify({ file: "outside.scss", line: 1 }) })];
    f.devtools.executeJavaScript.mockImplementation(async (script) => script.includes("const k =") ? payloads.shift() ?? "null" : "");
    f.wc.emit("devtools-opened");
    await vi.advanceTimersByTimeAsync(360);
    expect(f.resolve).toHaveBeenCalledExactlyOnceWith("outside.scss", "");
    expect(f.wc.send).not.toHaveBeenCalled();
    f.wc.closeDevTools();
  });

  it("retries watcher injection and reinstalls after close/reopen", async () => {
    const f = fixture();
    f.devtools.executeJavaScript.mockRejectedValueOnce(new Error("frontend not ready"));
    f.wc.emit("devtools-opened");
    await vi.advanceTimersByTimeAsync(250);
    expect(f.devtools.executeJavaScript).toHaveBeenCalledTimes(2);
    f.key("F12");
    f.wc.emit("devtools-opened");
    await vi.advanceTimersByTimeAsync(120);
    expect(f.devtools.executeJavaScript.mock.calls.some(([script]) => script.includes("const k ="))).toBe(true);
    f.wc.closeDevTools();
  });
});
