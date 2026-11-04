import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentOption, ModelOption, SessionInfo } from "@shared/types";
import type { VelaApi } from "../../preload";
import { useSessionSelection } from "./use-session-selection";

function rejectLifecycleQueries(host: Parameters<typeof useSessionSelection>[0]) {
  // @ts-expect-error Selection borrows readonly current focus.
  host.readFocusedSession()!.id = "other";
  // @ts-expect-error Selection cannot change workspace generation.
  host.panelFor(host.readFocusedSession()!.workspace)!.workspace.generation++;
}
void rejectLifecycleQueries;

const first: SessionInfo = { id: "first", directory: "/first", workspace: { id: "first-workspace", generation: 1 } };
const second: SessionInfo = { id: "second", directory: "/second", workspace: { id: "second-workspace", generation: 2 } };
const model: ModelOption = { id: "model", providerID: "provider", name: "Model", limit: { context: 10000 } };
const agent: AgentOption = { id: "build", name: "Build" };
let selection: ReturnType<typeof useSessionSelection>;
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("session selection ownership", () => {
  let root: Root;
  let container: HTMLDivElement;
  const sessionRef = { current: first as SessionInfo | null };
  let panels: SessionInfo[];
  const notify = vi.fn();
  const panelFor = (workspace: SessionInfo["workspace"]) => panels.find((panel) => panel.workspace.id === workspace.id && panel.workspace.generation === workspace.generation) ?? null;
  function Probe() {
    selection = useSessionSelection({ readFocusedSession: () => sessionRef.current, panelFor, notify });
    return null;
  }
  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    sessionRef.current = first;
    panels = [first, second];
    notify.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    window.vela = {
      models: async () => [model],
      modelDefault: async () => model,
      sessionSelection: async () => null,
      agents: async () => [agent],
      switchModel: async () => {},
      switchAgent: async () => {}
    } as unknown as VelaApi;
    await act(async () => root.render(<Probe />));
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("retains the global last-loaded catalog while background selections stay scoped", async () => {
    await act(async () => selection.loadModels(first.workspace));
    const background = { ...model, id: "background", name: "Background" };
    window.vela.models = async () => [background];
    window.vela.modelDefault = async () => background;
    await act(async () => selection.loadModels(second.workspace));
    expect(selection.models).toEqual([model]);
    expect(selection.currentModel?.id).toBe(model.id);
    expect(selection.availableModels).toEqual([background]);
    expect(selection.lastModel?.id).toBe(background.id);
    act(() => selection.applyModelSelection(second.workspace, { id: "background", providerID: "provider", variant: "high" }));
    expect(selection.currentModel?.id).toBe(model.id);
    expect(selection.currentModelByWorkspace[second.workspace.id]).toMatchObject({ ...background, variant: "high" });
  });

  it("keeps existing catalog comparator behavior and latest selection references", async () => {
    await act(async () => { await selection.loadModels(); await selection.loadAgents(); });
    const models = selection.models;
    const agents = selection.agents;
    const freshAgent = { ...agent, description: "new metadata" };
    const freshModels = [{ ...model }];
    window.vela.models = async () => freshModels;
    window.vela.agents = async () => [freshAgent];
    await act(async () => { await selection.loadModels(); await selection.loadAgents(); });
    expect(selection.models).toBe(models);
    expect(selection.availableModels).toBe(freshModels);
    expect(selection.agents).toBe(agents);
    expect(selection.currentAgent).toBe(freshAgent);
  });

  it("rejects late catalogs for a replaced generation without populating global fallback", async () => {
    const catalog = deferred<ModelOption[]>();
    window.vela.models = async () => catalog.promise;
    const loading = selection.loadModels(first.workspace);
    panels = [{ ...first, workspace: { ...first.workspace, generation: 3 } }, second];
    await act(async () => { catalog.resolve([model]); await loading; });
    expect(selection.modelsByWorkspace).toEqual({});
    expect(selection.availableModels).toEqual([]);
    expect(selection.lastModel).toBeNull();
    expect(notify).not.toHaveBeenCalled();
  });

  it("keeps an existing selection when a pending switch loses its workspace", async () => {
    window.vela.agents = async () => [agent, { id: "other", name: "Other" }];
    await act(async () => selection.loadAgents());
    const switching = deferred<void>();
    window.vela.switchAgent = async () => switching.promise;
    const task = selection.switchAgent("other", first.workspace);
    panels = [second];
    await act(async () => { switching.resolve(); await task; });
    expect(selection.currentAgent).toEqual(agent);
    expect(notify).not.toHaveBeenCalled();
  });

  it("uses synchronous focus refs and preserves failure and event fallback behavior", async () => {
    const models = vi.fn(async () => [model]);
    window.vela.models = models;
    sessionRef.current = second;
    await act(async () => selection.loadModels());
    expect(models).toHaveBeenCalledWith(second.workspace);
    act(() => selection.applyModelSelection(second.workspace, { id: "unknown", providerID: "provider" }));
    act(() => selection.applyAgentSelection(second.workspace, "unknown-agent"));
    expect(selection.currentModel).toEqual({ id: "unknown", providerID: "provider", name: "unknown" });
    expect(selection.currentAgent).toEqual({ id: "unknown-agent", name: "unknown-agent" });
    act(() => selection.applyModelSelection(second.workspace, { id: "missing-provider" }));
    expect(selection.currentModel?.id).toBe("unknown");
    window.vela.switchModel = async () => { throw new Error("switch failed"); };
    await act(async () => selection.switchModel(model.id, model.providerID));
    expect(notify).toHaveBeenCalledWith("switch failed", "error");
    expect(selection.currentModel?.id).toBe("unknown");
  });
});
