import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { launchApplication } from "./launch";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("Vela launch handoff", () => {
  let frames: FrameRequestCallback[];
  let decode: ReturnType<typeof deferred<void>>;
  beforeEach(() => {
    vi.useFakeTimers();
    frames = [];
    decode = deferred<void>();
    document.body.innerHTML = '<div id="launch-screen"><img id="launch-artwork" /></div><div id="root"></div>';
    Object.defineProperty(document.querySelector("img"), "decode", { value: () => decode.promise });
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => frames.push(callback));
    vi.stubGlobal("performance", { mark: vi.fn() });
  });
  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });
  async function frame() {
    const pending = frames.splice(0);
    for (const callback of pending) callback(0);
    await vi.advanceTimersByTimeAsync(0);
  }
  async function paint() {
    decode.resolve();
    await vi.advanceTimersByTimeAsync(0);
    await frame();
    await frame();
  }

  it("paints the local artwork before loading the IDE and holds it for three seconds", async () => {
    const mountApp = vi.fn();
    const load = vi.fn(async () => ({ mountApp }));
    const launch = launchApplication(load);
    await vi.advanceTimersByTimeAsync(5000);
    expect(load).not.toHaveBeenCalled();
    decode.resolve();
    await vi.advanceTimersByTimeAsync(0);
    await frame();
    expect(load).not.toHaveBeenCalled();
    await frame();
    expect(load).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(2999);
    expect(mountApp).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    await launch;
    expect(mountApp).toHaveBeenCalledOnce();
  });

  it("keeps the splash until the IDE is ready when module loading exceeds three seconds", async () => {
    const application = deferred<{ mountApp: () => void }>();
    const mountApp = vi.fn();
    const launch = launchApplication(() => application.promise);
    await paint();
    await vi.advanceTimersByTimeAsync(3100);
    expect(mountApp).not.toHaveBeenCalled();
    expect(document.querySelector("#launch-screen")).not.toBeNull();
    application.resolve({ mountApp });
    await launch;
    expect(mountApp).toHaveBeenCalledOnce();
  });

  it("starts the hold only after the app becomes visible", async () => {
    const visibility = vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    const mountApp = vi.fn();
    const load = vi.fn(async () => ({ mountApp }));
    const launch = launchApplication(load);
    decode.resolve();
    await vi.advanceTimersByTimeAsync(5000);
    expect(load).not.toHaveBeenCalled();
    visibility.mockReturnValue("visible");
    document.dispatchEvent(new Event("visibilitychange"));
    await vi.advanceTimersByTimeAsync(0);
    await frame();
    await frame();
    await vi.advanceTimersByTimeAsync(2999);
    expect(mountApp).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    await launch;
    expect(mountApp).toHaveBeenCalledOnce();
  });
});
