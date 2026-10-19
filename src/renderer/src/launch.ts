const LAUNCH_DURATION_MS = 3000;

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
  const artwork = document.querySelector<HTMLImageElement>("#launch-artwork");
  if (artwork) await artwork.decode().catch(() => {});
  await visiblePaint();
  performance.mark("vela-launch-artwork");
  const hold = new Promise<void>((resolve) => setTimeout(resolve, LAUNCH_DURATION_MS));
  // Load the IDE behind the already-painted splash, without delaying its first frame.
  const [application] = await Promise.all([load(), hold]);
  application.mountApp();
}
