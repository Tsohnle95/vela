import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ViteServerInfo } from "@shared/types";
import { ServerSettings } from "./ServerSettings";

function server(entry: string, directory = "/repo/docs"): ViteServerInfo {
  return {
    id: `${directory}:${entry}`,
    workspaceId: "11111111-1111-4111-8111-111111111111",
    directory,
    entry,
    url: `http://127.0.0.1:5200/${entry}`,
    port: 5200
  };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

describe("ServerSettings", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("lists running servers and stops a single one", async () => {
    const running = [server("a.html"), server("", "/repo")];
    const viteStop = vi.fn(async (id: string) => {
      running.splice(running.findIndex((item) => item.id === id), 1);
    });
    window.vela = {
      viteServers: vi.fn(async () => [...running]),
      viteStop,
      viteStopAll: vi.fn(async () => {})
    } as unknown as typeof window.vela;

    await act(async () => root.render(<ServerSettings />));
    await act(async () => { await flush(); });

    expect(container.textContent).toContain("2 servers running");
    expect(container.textContent).toContain("docs/a.html");
    expect(container.textContent).toContain("repo/index.html");
    expect(container.textContent).toContain("http://127.0.0.1:5200/a.html");

    const stop = container.querySelector<HTMLButtonElement>(".settings-action-button")!;
    await act(async () => {
      stop.click();
      await flush();
    });
    expect(viteStop).toHaveBeenCalledWith(server("a.html").id);
  });

  it("stops every server at once", async () => {
    const viteStopAll = vi.fn(async () => {});
    window.vela = {
      viteServers: vi.fn(async () => [server("a.html"), server("b.html")]),
      viteStop: vi.fn(async () => {}),
      viteStopAll
    } as unknown as typeof window.vela;

    await act(async () => root.render(<ServerSettings />));
    await act(async () => { await flush(); });

    const stopAll = [...container.querySelectorAll<HTMLButtonElement>("button")]
      .find((button) => button.textContent === "Stop all servers")!;
    await act(async () => {
      stopAll.click();
      await flush();
    });
    expect(viteStopAll).toHaveBeenCalledOnce();
  });

  it("shows an empty state when nothing is running", async () => {
    window.vela = {
      viteServers: vi.fn(async () => []),
      viteStop: vi.fn(async () => {}),
      viteStopAll: vi.fn(async () => {})
    } as unknown as typeof window.vela;

    await act(async () => root.render(<ServerSettings />));
    await act(async () => { await flush(); });

    expect(container.textContent).toContain("No servers running");
    expect(container.textContent).toContain("Start a preview from the server button");
  });
});
