import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendMessage, SessionInfo, SassWatchState, WorkspaceIdentity } from "@shared/types";
import { EditorStatusMenu } from "./EditorStatusMenu";

const first: SessionInfo = { id: "first", directory: "/one", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
const second: SessionInfo = { id: "second", directory: "/two", workspace: { id: "22222222-2222-4222-8222-222222222222", generation: 2 } };
let selected = first;
vi.mock("../store", () => ({ useStore: () => ({ session: selected, tabs: [], activePath: null }) }));
vi.mock("../monaco", () => ({ monaco: { editor: { getModelMarkers: vi.fn() }, MarkerSeverity: {} } }));
vi.mock("../reveal", () => ({ getEditor: vi.fn() }));
vi.mock("../w3c-validation", () => ({ applyW3cMarkers: vi.fn(), clearW3cMarkers: vi.fn() }));
const snapshot = (session: SessionInfo, status: SassWatchState["status"] = "off", revision = 1): SassWatchState => ({
  workspace: session.workspace, directory: session.directory, enabled: status !== "off", status, revision, logs: []
});

describe("footer Sass control", () => {
  let container: HTMLDivElement;
  let root: Root;
  const listeners = new Set<(message: BackendMessage) => void>();
  const emit = (state: SassWatchState): void => { for (const listener of listeners) listener({ kind: "sass-watch", state }); };
  const button = (): HTMLButtonElement => container.querySelector(".sass-watch-btn")!;
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    selected = first; listeners.clear();
    window.vela = {
      onMessage: (handler: (message: BackendMessage) => void) => { listeners.add(handler); return () => { listeners.delete(handler); }; },
      sassWatchState: vi.fn(async () => snapshot(selected)),
      setSassWatch: vi.fn(async (workspace: WorkspaceIdentity, enabled: boolean) => ({ ...snapshot(selected, enabled ? "starting" : "off", 2), workspace }))
    } as unknown as typeof window.vela;
    container = document.createElement("div"); document.body.append(container); root = createRoot(container);
  });

  const renderMenu = async (node: ReactNode = <EditorStatusMenu />): Promise<void> => {
    await act(async () => root.render(node));
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".status-menu-button")!.click();
    });
  };

  afterEach(async () => { await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals(); });

  it("shows off, starting, on and error and toggles the selected immutable workspace", async () => {
    await renderMenu();
    expect(button().textContent).toBe("Watch Sass");
    expect(button().getAttribute("aria-pressed")).toBe("false");
    await act(async () => button().click());
    expect(window.vela.setSassWatch).toHaveBeenCalledWith(first.workspace, true);
    expect(button().textContent).toBe("Starting Sass…");
    act(() => emit(snapshot(first, "on", 3)));
    expect(button().textContent).toBe("Sass: On");
    act(() => emit({ ...snapshot(first, "error", 4), error: "Undefined variable in main.scss" }));
    expect(button().textContent).toBe("Sass: Error");
    expect(button().title).toContain("Undefined variable");
    expect(button().getAttribute("aria-label")).toContain("/one: error");
    await act(async () => button().click());
    expect(window.vela.setSassWatch).toHaveBeenLastCalledWith(first.workspace, false);
  });

  it("shows each selected workspace's state and ignores a previous workspace's late events", async () => {
    await renderMenu();
    act(() => emit(snapshot(first, "on", 3)));
    await act(async () => { selected = second; root.render(<EditorStatusMenu />); });
    expect(button().textContent).toBe("Watch Sass");
    act(() => emit({ ...snapshot(first, "error", 99), error: "old workspace error" }));
    expect(button().textContent).toBe("Watch Sass");
    expect(button().title).not.toContain("old workspace");
    await act(async () => button().click());
    expect(window.vela.setSassWatch).toHaveBeenLastCalledWith(second.workspace, true);
    act(() => emit(snapshot(second, "on", 3)));
    expect(button().textContent).toBe("Sass: On");
    act(() => emit(snapshot(second, "starting", 2)));
    expect(button().textContent).toBe("Sass: On");
  });
});
