import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Welcome } from "./Welcome";

const store = {
  selectFolder: vi.fn(),
  openPaths: vi.fn(),
  reopenSession: vi.fn(),
  openSession: vi.fn(),
  savedWorkspaces: [{ directory: "/workspace", name: "Workspace" }],
  saveWorkspace: vi.fn(),
  connected: true,
  sessions: [{ id: "closed", title: "Recent work", directory: "/missing", updatedAt: 1 }]
};

vi.mock("../store", () => ({ useStore: () => store }));

describe("Welcome saved workspaces", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    store.openSession.mockClear();
    store.savedWorkspaces = [{ directory: "/workspace", name: "Workspace" }];
    window.vela = {
      sessions: vi.fn(async () => [{ id: "closed", title: "Recent work", directory: "/missing", updatedAt: 1 }])
    } as unknown as typeof window.vela;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("opens a saved workspace from its row and expands history from its disclosure", async () => {
    await act(async () => {
      root.render(<Welcome />);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    await act(async () => container.querySelectorAll<HTMLButtonElement>(".sd-sec .sd-sh")[0].click());
    const recent = container.querySelector<HTMLButtonElement>(".num-list .rowlink")!;
    await act(async () => recent.click());
    expect(store.reopenSession).toHaveBeenCalledWith("closed", false, "/missing");

    await act(async () => container.querySelector<HTMLButtonElement>(".sd-sh-chevron")!.click());
    const workspace = container.querySelector<HTMLButtonElement>('[aria-label="Open workspace Workspace"]')!;
    await act(async () => workspace.click());
    expect(store.openSession).toHaveBeenCalledWith("/workspace");

    const disclosure = container.querySelector<HTMLButtonElement>('[aria-label="Expand Workspace sessions"]')!;
    await act(async () => disclosure.click());
    expect(disclosure.getAttribute("aria-expanded")).toBe("true");
    expect(container.textContent).toContain("No sessions yet.");
  });
  it("renders the shared history after delayed service readiness and subsequent updates", async () => {
    store.connected = false;
    store.sessions = [];
    await act(async () => root.render(<Welcome />));
    await act(async () => container.querySelectorAll<HTMLButtonElement>(".sd-sec .sd-sh")[0].click());
    expect(container.textContent).not.toContain("Recent work");
    store.connected = true;
    store.sessions = [{ id: "closed", title: "Recent work", directory: "/workspace", updatedAt: 1 }];
    await act(async () => root.render(<Welcome />));
    expect(container.textContent).toContain("Recent work");
    store.sessions = [{ id: "closed", title: "Updated title", directory: "/workspace", updatedAt: 2 }];
    await act(async () => root.render(<Welcome />));
    expect(container.textContent).toContain("Updated title");
    expect(window.vela.sessions).not.toHaveBeenCalled();
  });

});
