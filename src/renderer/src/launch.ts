const LAUNCH_DURATION_MS = 3000;

function startDawnTerminator(): void {
  const light = document.querySelector<SVGRectElement>("#launch-dawn");
  if (!light || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const segment = (p: number, start: number, end: number): number => Math.max(0, Math.min(1, (p - start) / (end - start)));
  const ease = (t: number): number => t * t * (3 - 2 * t);
  // Sample study 23's easing and light envelope; both the sweep and launch hold start after visible paint.
  const frames: Keyframe[] = Array.from({ length: 91 }, (_, index) => {
    const p = index / 90;
    const travel = ease(segment(p, 0.04, 0.74));
    const lightEnvelope = Math.sin(Math.PI * segment(p, 0, 0.86));
    const settle = 1 - ease(segment(p, 0.76, 0.9));
    return {
      offset: p,
      transform: `translate(${-100 + 300 * travel}px, 0)`,
      opacity: lightEnvelope * 0.7 * settle
    };
  });
  light.animate(frames, { duration: LAUNCH_DURATION_MS, fill: "forwards" });
}

async function visiblePaint(): Promise<void> {
  if (document.visibilityState === "hidden") {
    await new Promise<void>((resolve) => {
      const onVisible = (): void => {
        if (document.visibilityState === "hidden") return;
        document.removeEventListener("visibilitychange", onVisible);
        resolve();
      };
      document.addEventListener("visibilitychange", onVisible);
    });
  }
  await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  if (document.visibilityState === "hidden") await visiblePaint();
}

export async function launchApplication(load: () => Promise<{ mountApp: () => void }>): Promise<void> {
  const artwork = document.querySelector("#launch-artwork");
  if (artwork instanceof HTMLImageElement) await artwork.decode().catch(() => {});
  await visiblePaint();
  startDawnTerminator();
  performance.mark("vela-launch-artwork");
  const hold = new Promise<void>((resolve) => setTimeout(resolve, LAUNCH_DURATION_MS));
  // Load the IDE behind the already-painted splash, without delaying its first frame.
  const [application] = await Promise.all([load(), hold]);
  application.mountApp();
}
