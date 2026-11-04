import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, PendingFormRequest, SessionInfo } from "@shared/types";
import { StoreProvider, useStore } from "./store";

type Store = ReturnType<typeof useStore>;

let store: Store;
let messageHandler: ((message: BackendMessage) => void) | null;

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

const globalForm = (): PendingFormRequest => ({
  id: "frm_global",
  sessionID: "global",
  title: "Choose a direction",
  fields: [{ key: "choice", type: "string", required: true }]
});

function api(formsList: VelaApi["formsList"], formReply = vi.fn(async () => {})): VelaApi {
  return {
    platform: "darwin",
    onMessage: (handler: (message: BackendMessage) => void) => {
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
    openSession: async (directory: string, generation: number) => info(directory, generation),
    closeSession: async () => {},
    readFile: async () => "content",
    listDir: async () => [],
    listPermissions: async () => [],
    formsList,
    formReply,
    formCancel: async () => {}
  } as unknown as VelaApi;
}

function globalFormEvent(ownerSessionID: string): BackendMessage {
  return {
    kind: "event",
    type: "form.created",
    data: {
      id: "evt_form",
      type: "form.created",
      created: Date.now(),
      velaSessionIDs: [ownerSessionID],
      data: { form: globalForm() }
    }
  };
}

describe("renderer form prompts", () => {
  let container: HTMLDivElement;
  let root: Root;

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
    vi.restoreAllMocks();
  });

  it("loads location-global questions into the GUI panel inventory", async () => {
    window.vela = api(async () => [globalForm()]);
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => {
      await store.openSession("/one");
      await Promise.resolve();
    });

    const panel = store.panels[0];
    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([globalForm()]);
  });

  it("routes a live global form event to its location owner and replies as global", async () => {
    const formReply = vi.fn(async () => {});
    window.vela = api(async () => [], formReply);
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    const panel = store.panels[0];

    await act(async () => messageHandler!(globalFormEvent(panel.id)));
    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([globalForm()]);

    await act(async () => store.submitForm(panel.workspace, "frm_global", { choice: "A" }, "global"));
    expect(formReply).toHaveBeenCalledWith(panel.workspace, "frm_global", { choice: "A" }, "global");
  });

  it("surfaces an external session form routed by location and replies to that session", async () => {
    const formReply = vi.fn(async () => {});
    window.vela = api(async () => [], formReply);
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    const panel = store.panels[0];
    const external: PendingFormRequest = {
      id: "frm_external",
      sessionID: "external-session",
      title: "TUI question",
      fields: [{ key: "q", type: "string", required: true }]
    };

    await act(async () => messageHandler!({
      kind: "event",
      type: "form.created",
      data: {
        id: "evt_external",
        type: "form.created",
        created: Date.now(),
        velaSessionIDs: [panel.id],
        data: { form: external }
      }
    }));
    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([external]);

    await act(async () => store.submitForm(panel.workspace, "frm_external", { q: "A" }, "external-session"));
    expect(formReply).toHaveBeenCalledWith(panel.workspace, "frm_external", { q: "A" }, "external-session");
  });

  it("keeps an event-backed question visible when reconciliation temporarily fails", async () => {
    const formsList = vi.fn<VelaApi["formsList"]>(async () => []);
    window.vela = api(formsList);
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.openSession("/one"));
    const panel = store.panels[0];
    await act(async () => messageHandler!(globalFormEvent(panel.id)));

    formsList.mockRejectedValueOnce(new Error("form endpoint unavailable"));
    await act(async () => {
      messageHandler!({ kind: "event", type: "server.connected", data: {} });
      await Promise.resolve();
    });

    expect(store.panelViews[panel.workspace.id].pendingForms).toEqual([globalForm()]);
  });
});
