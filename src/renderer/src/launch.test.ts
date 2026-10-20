import { readFileSync } from "node:fs";
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

  it("paints the local artwork before loading the IDE and holds it for three seconds regardless of shimmer duration", async () => {
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

  it("keeps the original moving folds and the new light in one group while the wordmark stays still", async () => {
    const html = readFileSync("src/renderer/index.html", "utf8");
    const page = new DOMParser().parseFromString(html, "text/html");
    const motion = page.querySelector("#launch-fold-motion")!;
    expect(motion.contains(page.querySelector("#launch-dawn"))).toBe(true);
    const source = new DOMParser().parseFromString(readFileSync("src/renderer/src/assets/folded-light.svg", "utf8"), "image/svg+xml");
    expect([...motion.children].filter((node) => node.localName === "path").map((node) => node.getAttribute("d")))
      .toEqual([...source.querySelectorAll("svg > g > path")].map((node) => node.getAttribute("d")));
    expect(motion.querySelector("text")).toBeNull();
    expect(page.querySelector("style")!.textContent).toContain("launch-float 4s ease-in-out infinite");
    expect(page.querySelector("style")!.textContent).toContain("prefers-reduced-motion: reduce");
    document.body.innerHTML = page.body.innerHTML;
    Object.defineProperty(document.querySelector("#launch-dawn"), "animate", { value: animate });
    const launch = launchApplication(async () => ({ mountApp: vi.fn() }));
    await paint();
    expect(animate).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(3000);
    await launch;
  });

  it("delays study 23 by half a second after visible paint and slows its full sweep by 40%", async () => {
    const launch = launchApplication(async () => ({ mountApp: vi.fn() }));
    expect(animate).not.toHaveBeenCalled();
    await paint();
    expect(animate).toHaveBeenCalledOnce();
    const [keyframes, options] = animate.mock.calls[0] as [Keyframe[], KeyframeAnimationOptions];
    expect(options).toEqual({ delay: 500, duration: 5000, fill: "forwards" });
    // The bright gradient band is already inside the folded surface when the delay ends.
    expect(keyframes[0]).toMatchObject({ offset: 0, transform: "translate(35px, 0)", opacity: 0.35 });
    expect(Number(keyframes[45].opacity)).toBeGreaterThan(0.5);
    expect(keyframes[90]).toMatchObject({ offset: 1, transform: "translate(335px, 0)", opacity: 0 });
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

  it("keeps the splash until the IDE is ready when module loading exceeds the animation hold", async () => {
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
