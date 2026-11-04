import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage } from "@shared/types";
import { StoreProvider, useCtxMenu, useStore, type CtxMenuApi } from "./store";

type Store = ReturnType<typeof useStore>;

let store: Store;
let storeRenders = 0;
let ctxMenuApi: CtxMenuApi;
let ctxMenuRenders = 0;
let messageHandler: ((message: BackendMessage) => void) | null;

function StoreProbe(): ReactNode {
  store = useStore();
  storeRenders += 1;
  return null;
}

function MenuProbe(): ReactNode {
  ctxMenuApi = useCtxMenu();
  ctxMenuRenders += 1;
  return null;
}

function api(overrides: Partial<VelaApi> = {}): VelaApi {
  return {
    platform: "darwin",
    onMessage: (handler) => {
      messageHandler = handler;
      return () => { messageHandler = null; };
    },
    health: async () => true,
    takePendingPaths: async () => [],
    state: async () => null,
    activeSessions: async () => [],
    models: async () => [],
    modelDefault: async () => null,
    sessionSelection: async () => null,
    agents: async () => [],
    sessions: async () => [],
    openSession: async () => ({ id: "session", directory: "/w", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } }),
    openSessionById: async (sessionID: string) => ({ session: { id: sessionID, directory: "/w", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } }, transcript: [], todos: [], usage: null }),
    closeSession: async () => {},
    readFile: async () => "",
    listDir: async () => [],
    ...overrides
  } as VelaApi;
}

describe("store context menu isolation", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    messageHandler = null;
    storeRenders = 0;
    ctxMenuRenders = 0;
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    window.vela = api();
    await act(async () =>
      root.render(
        <StoreProvider>
          <StoreProbe />
          <MenuProbe />
        </StoreProvider>
      )
    );
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("propagates ctxMenu immediately on open and close", () => {
    expect(ctxMenuApi.ctxMenu).toBeNull();
    act(() => ctxMenuApi.openCtxMenu(12, 34, { path: "a.txt", type: "file" }));
    expect(ctxMenuApi.ctxMenu).toEqual({ x: 12, y: 34, target: { path: "a.txt", type: "file" } });
    act(() => ctxMenuApi.closeCtxMenu());
    expect(ctxMenuApi.ctxMenu).toBeNull();
  });

  it("does not re-render the main store when the menu opens or closes", () => {
    const storeRendersBefore = storeRenders;
    const ctxMenuRendersBefore = ctxMenuRenders;
    act(() => ctxMenuApi.openCtxMenu(12, 34, { path: "a.txt", type: "file" }));
    act(() => ctxMenuApi.closeCtxMenu());
    expect(storeRenders).toBe(storeRendersBefore);
    expect(ctxMenuRenders).toBe(ctxMenuRendersBefore + 2);
  });
});
