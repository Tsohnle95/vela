import { describe, expect, it, vi } from "vitest";
import { collectLaunchPaths, PendingOpenPaths } from "./open-paths";

describe("PendingOpenPaths", () => {
  it("retains native requests until the renderer has subscribed", () => {
    const pending = new PendingOpenPaths();
    pending.push(["/dropped folder"]);
    expect(pending.takeIfRendererReady()).toEqual([]);
    expect(pending.size).toBe(1);

    pending.setRendererReady(true);
    expect(pending.take()).toEqual(["/dropped folder"]);
    pending.push(["/another folder"]);
    expect(pending.takeIfRendererReady()).toEqual(["/another folder"]);
    expect(pending.takeIfRendererReady()).toEqual([]);
  });

  it("queues drops during a renderer reload until it subscribes again", () => {
    const pending = new PendingOpenPaths();
    pending.setRendererReady(true);
    pending.setRendererReady(false);
    pending.push(["/during-reload"]);
    expect(pending.takeIfRendererReady()).toEqual([]);

    pending.setRendererReady(true);
    expect(pending.take()).toEqual(["/during-reload"]);
    expect(pending.size).toBe(0);
  });

  it("takes queued paths once and clears the queue", () => {
    const pending = new PendingOpenPaths();
    pending.push(["/repo/a", "/repo/b"]);
    expect(pending.size).toBe(2);
    expect(pending.take()).toEqual(["/repo/a", "/repo/b"]);
    expect(pending.size).toBe(0);
    expect(pending.take()).toEqual([]);
  });

  it("ignores blanks and duplicates and caps the queue", () => {
    const pending = new PendingOpenPaths();
    pending.push(["  ", "/repo/a", "/repo/a", "/repo/b"]);
    expect(pending.take()).toEqual(["/repo/a", "/repo/b"]);
    pending.push(Array.from({ length: 12 }, (_, index) => `/repo/${index}`));
    expect(pending.take()).toHaveLength(10);
  });
});

describe("collectLaunchPaths", () => {
  it("keeps existing paths and drops flags", () => {
    const exists = vi.fn((path: string) => path === "/repo/a" || path === "/file.txt");
    expect(collectLaunchPaths(["/repo/a", "--new-window", "-n", "+42", "--", "missing", "/file.txt"], exists)).toEqual([
      "/repo/a",
      "/file.txt"
    ]);
  });

  it("treats exists failures as missing", () => {
    const exists = vi.fn(() => {
      throw new Error("denied");
    });
    expect(collectLaunchPaths(["/repo/a"], exists)).toEqual([]);
  });

  it("ignores relative launch arguments", () => {
    const exists = vi.fn(() => true);
    expect(collectLaunchPaths([".", "/repo/a"], exists)).toEqual(["/repo/a"]);
  });

  it("ignores the app executable in secondary-instance arguments", () => {
    const exists = vi.fn(() => true);
    expect(collectLaunchPaths(["/Applications/Vela.app/Contents/MacOS/Vela", "/repo/a"], exists, "/Applications/Vela.app/Contents/MacOS/Vela"))
      .toEqual(["/repo/a"]);
  });

  it("keeps launch paths after the executable argument is removed", () => {
    const exists = vi.fn(() => true);
    const argv = ["/different/Vela.app/Contents/MacOS/Electron", "/repo/a"];
    expect(collectLaunchPaths(argv.slice(1), exists, "/Applications/Vela.app/Contents/MacOS/Vela")).toEqual(["/repo/a"]);
  });
});
