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
  let decode: ReturnType<typeof deferred<void>>;
  beforeEach(() => {
    vi.useFakeTimers();
    frames = [];
    decode = deferred<void>();
    document.body.innerHTML = '<div id="launch-screen"><img id="launch-artwork" /><svg><rect id="launch-dawn" opacity="0" /></svg></div><div id="root"></div>';
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
    const launch = launchApplication(async () => ({ mountApp: vi.fn() }));
    await paint();
    await vi.advanceTimersByTimeAsync(3000);
    await launch;
  });

  it("sweeps the whole logo from transparent edges without popping in halfway through", () => {
    const page = new DOMParser().parseFromString(readFileSync("src/renderer/index.html", "utf8"), "text/html");
    const css = page.querySelector("style")!.textContent!;
    expect(css).toContain("launch-shimmer 2.5s linear .5s forwards");
    expect(css).toContain("0% { transform: translateX(87px); opacity: 0; }");
    expect(css).toContain("100% { transform: translateX(293px); opacity: 0; }");
    expect(css).toContain("8%, 92% { opacity: .65; }");
    expect(css).toContain("#launch-fold-motion, #launch-dawn { animation: none; }");
    const gradient = page.querySelector("#launch-dawn-gradient")!;
    expect(gradient.getAttribute("y1")).toBe(gradient.getAttribute("y2"));
    expect(87 + Number(gradient.getAttribute("x2"))).toBe(114);
    expect(293 + Number(gradient.getAttribute("x1"))).toBe(266);
    const stops = gradient.querySelectorAll("stop");
    expect(stops[0].getAttribute("stop-opacity")).toBe("0");
    expect(stops[stops.length - 1].getAttribute("stop-opacity")).toBe("0");
    expect(page.querySelector("#launch-dawn")!.getAttribute("opacity")).toBe("0");
  });

  it("uses the still logo for reduced motion while preserving the launch hold", async () => {
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: true })));
    const mountApp = vi.fn();
    const launch = launchApplication(async () => ({ mountApp }));
    await paint();
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
