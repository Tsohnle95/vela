import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { launchApplication } from "./launch";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("Vela launch handoff", () => {
  let frames: FrameRequestCallback[];
  let animate: ReturnType<typeof vi.fn>;
  let decode: ReturnType<typeof deferred<void>>;
  beforeEach(() => {
    vi.useFakeTimers();
    frames = [];
    decode = deferred<void>();
    document.body.innerHTML = '<div id="launch-screen"><img id="launch-artwork" /><svg><rect id="launch-dawn" opacity="0" /></svg></div><div id="root"></div>';
    animate = vi.fn();
    Object.defineProperty(document.querySelector("#launch-dawn"), "animate", { value: animate });
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: false })));
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

  it("starts study 23 after visible paint and settles its horizontal light sweep at handoff", async () => {
    const launch = launchApplication(async () => ({ mountApp: vi.fn() }));
    expect(animate).not.toHaveBeenCalled();
    await paint();
    expect(animate).toHaveBeenCalledOnce();
    const [keyframes, options] = animate.mock.calls[0] as [Keyframe[], KeyframeAnimationOptions];
    expect(options).toEqual({ duration: 3000, fill: "forwards" });
    expect(keyframes[0]).toMatchObject({ offset: 0, transform: "translate(-100px, 0)", opacity: 0 });
    expect(Number(keyframes[45].opacity)).toBeGreaterThan(0.5);
    expect(keyframes[90]).toMatchObject({ offset: 1, transform: "translate(200px, 0)", opacity: 0 });
    expect(keyframes.every((frame) => String(frame.transform).endsWith("px, 0)"))).toBe(true);
    await vi.advanceTimersByTimeAsync(3000);
    await launch;
  });

  it("uses the still logo for reduced motion while preserving the launch hold", async () => {
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: true })));
    const mountApp = vi.fn();
    const launch = launchApplication(async () => ({ mountApp }));
    await paint();
    expect(animate).not.toHaveBeenCalled();
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
    expect(animate).not.toHaveBeenCalled();
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
