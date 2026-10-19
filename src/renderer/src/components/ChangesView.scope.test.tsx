import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../../preload";
import type { BackendMessage, FileUpdate, SessionInfo } from "@shared/types";
import { StoreProvider, useStore } from "../store";
import { ChangesView } from "./ChangesView";

vi.mock("./OpenCodeFile", () => ({
  OpenCodeFile: ({ file }: { file: string }) => <div data-testid="diff" data-file={file} />
}));

type Store = ReturnType<typeof useStore>;

let store: Store;
let messageHandler: ((message: BackendMessage) => void) | null = null;

function Probe(): ReactNode {
  store = useStore();
  return null;
}

function info(directory: string, generation: number): SessionInfo {
  return {
    id: `session-${generation}`,
    directory,
    workspace: { id: `${generation}1111111-1111-4111-8111-111111111111`, generation }
  };
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
    projects: async () => [],
    savedWorkspaces: async () => ({ workspaces: [], initialized: true }),
    saveSavedWorkspaces: async () => {},
    models: async () => [],
    modelDefault: async () => null,
    sessionSelection: async () => null,
    agents: async () => [],
    sessions: async () => [],
    openSession: async (directory: string, generation: number) => info(directory, generation),
    openSessionById: async (sessionID: string) => ({
      session: { ...info("/reopened", 0), id: sessionID },
      transcript: [],
      todos: [],
      usage: null
    }),
    closeSession: async () => {},
    readFile: async () => "content",
    listDir: async () => [],
    ...overrides
  } as VelaApi;
}

function update(session: SessionInfo, path: string, content: string | null): FileUpdate {
  return {
    workspace: session.workspace,
    sessionID: session.id,
    path,
    baseline: { kind: "known", content: "before\n" },
    content,
    deleted: content === null
  };
}

async function dispatch(file: FileUpdate): Promise<void> {
  await act(async () => messageHandler!({ kind: "file-update", file }));
}

function changePaths(): string[] {
  return [...document.querySelectorAll<HTMLElement>(".change-section")]
    .map((section) => section.getAttribute("data-change-path") ?? "");
}

let container: HTMLDivElement;
let root: Root;

describe("ChangesView workspace scoping", () => {
  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    messageHandler = null;
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  async function render(): Promise<void> {
    window.vela = api();
    await act(async () => root.render(
      <StoreProvider><Probe /><ChangesView onClose={() => {}} /></StoreProvider>
    ));
  }

  it("shows only the focused workspace's changes when several are open", async () => {
    await render();

    await act(async () => store.openSession("/alpha"));
    const alpha = store.panels[0];
    await act(async () => store.addModelPanel("/beta"));
    const beta = store.panels.find((panel) => panel.directory === "/beta")!;
    expect(store.panels).toHaveLength(2);

    await dispatch(update(alpha, "src/alpha-file.ts", "changed\n"));
    await dispatch(update(beta, "src/beta-file.ts", "changed\n"));

    await act(async () => store.focusSession(beta.id));
    expect(changePaths()).toEqual(["src/beta-file.ts"]);

    await act(async () => store.focusSession(alpha.id));
    expect(changePaths()).toEqual(["src/alpha-file.ts"]);
  });

  it("excludes paths hidden because a nested workspace owns them", async () => {
    await render();

    await act(async () => store.openSession("/root"));
    const parent = store.panels[0];
    // Opening the nested folder as its own workspace hides it inside the parent.
    await act(async () => store.addModelPanel("/root/child"));
    await act(async () => store.focusSession(parent.id));
    expect(store.hiddenPaths.has("child")).toBe(true);

    await dispatch(update(parent, "top.ts", "changed\n"));
    await dispatch(update(parent, "child/nested-file.ts", "changed\n"));

    expect(changePaths()).toEqual(["top.ts"]);
  });
});
