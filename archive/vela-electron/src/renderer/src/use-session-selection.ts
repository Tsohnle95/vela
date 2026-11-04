import type { SessionReadView } from "./store-contract";
import { useCallback, useRef, useState } from "react";
import type { AgentOption, ModelOption, WorkspaceIdentity } from "@shared/types";

interface SelectionHost {
  readFocusedSession: () => SessionReadView | null;
  panelFor: (workspace: WorkspaceIdentity) => SessionReadView | null;
  notify: (text: string, tone?: "info" | "error") => void;
}

type ModelSelection = { id?: string; providerID?: string; variant?: string };

export function useSessionSelection(host: SelectionHost) {
  const { readFocusedSession, panelFor, notify: toast } = host;
  const session = readFocusedSession();
  const [modelsByWorkspace, setModelsByWorkspace] = useState<Record<string, ModelOption[]>>({});
  const [currentModelByWorkspace, setCurrentModelByWorkspace] = useState<Record<string, ModelOption | null>>({});
  const [availableModels, setAvailableModels] = useState<ModelOption[]>([]);
  const [lastModel, setLastModel] = useState<ModelOption | null>(null);
  const [agentsByWorkspace, setAgentsByWorkspace] = useState<Record<string, AgentOption[]>>({});
  const [currentAgentByWorkspace, setCurrentAgentByWorkspace] = useState<Record<string, AgentOption | null>>({});
  const modelsByWorkspaceRef = useRef(modelsByWorkspace);
  modelsByWorkspaceRef.current = modelsByWorkspace;
  const agentsByWorkspaceRef = useRef(agentsByWorkspace);
  agentsByWorkspaceRef.current = agentsByWorkspace;
  const loadModels = useCallback(async (workspace?: WorkspaceIdentity) => {
    const target = workspace ?? readFocusedSession()?.workspace;
    if (!target) return;
    try {
      const [list, def, selection] = await Promise.all([
        window.vela.models(target),
        window.vela.modelDefault(target),
        window.vela.sessionSelection(target)
      ]);
      if (!panelFor(target)) return;
      setModelsByWorkspace((prev) => {
        const current = prev[target.id] ?? [];
        if (JSON.stringify(current) === JSON.stringify(list)) {
          return prev;
        }
        return { ...prev, [target.id]: list };
      });
      setAvailableModels(list);
      setCurrentModelByWorkspace((prev) => {
        const current = prev[target.id] ?? null;
        const pick = selection?.model ?? def ?? current;
        if (!pick) return prev;
        const match = list.find((m) => m.id === pick.id && m.providerID === pick.providerID);
        const next = match ? { ...match, ...(pick.variant ? { variant: pick.variant } : {}) } : pick;
        setLastModel(next);
        return prev[target.id] === next ? prev : { ...prev, [target.id]: next };
      });
    } catch (err) {
      if (panelFor(target)) toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [toast, panelFor]);

  const switchModel = useCallback(
    async (id: string, providerID: string, variant?: string, workspace?: WorkspaceIdentity) => {
      const target = workspace ?? readFocusedSession()?.workspace;
      if (!target) return;
      try {
        await window.vela.switchModel(target, id, providerID, variant);
        if (!panelFor(target)) return;
        const base = modelsByWorkspaceRef.current[target.id]?.find((m) => m.id === id && m.providerID === providerID);
        if (base) setCurrentModelByWorkspace((prev) => ({ ...prev, [target.id]: { ...base, ...(variant ? { variant } : {}) } }));
      } catch (err) {
        if (panelFor(target)) toast(err instanceof Error ? err.message : String(err), "error");
      }
    },
    [toast, panelFor]
  );

  const loadAgents = useCallback(async (workspace?: WorkspaceIdentity) => {
    const target = workspace ?? readFocusedSession()?.workspace;
    if (!target) return;
    try {
      const [list, selection] = await Promise.all([
        window.vela.agents(target),
        window.vela.sessionSelection(target)
      ]);
      if (!panelFor(target)) return;
      setAgentsByWorkspace((prev) => {
        const current = prev[target.id] ?? [];
        if (
          current.length === list.length &&
          current.every((a, i) => a.id === list[i].id && a.name === list[i].name && a.color === list[i].color)
        ) {
          return prev;
        }
        return { ...prev, [target.id]: list };
      });
      setCurrentAgentByWorkspace((prev) => {
        const panel = panelFor(target);
        const id = selection?.agent?.id ?? panel?.agent ?? "build";
        const next = list.find((agent) => agent.id === id) ?? selection?.agent ?? { id, name: id };
        return prev[target.id] === next ? prev : { ...prev, [target.id]: next };
      });
      const panel = panelFor(target);
      const id = selection?.agent?.id ?? panel?.agent ?? "build";
      if (!selection?.agent && list.some((agent) => agent.id === id) && panel?.agent !== id) {
        await window.vela.switchAgent(target, id);
      }
    } catch (err) {
      if (panelFor(target)) toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [toast, panelFor]);

  const switchAgent = useCallback(
    async (id: string, workspace?: WorkspaceIdentity) => {
      const target = workspace ?? readFocusedSession()?.workspace;
      if (!target) return;
      try {
        await window.vela.switchAgent(target, id);
        if (!panelFor(target)) return;
        const base = agentsByWorkspaceRef.current[target.id]?.find((agent) => agent.id === id);
        if (base) setCurrentAgentByWorkspace((prev) => ({ ...prev, [target.id]: base }));
      } catch (err) {
        if (panelFor(target)) toast(err instanceof Error ? err.message : String(err), "error");
      }
    },
    [toast, panelFor]
  );

  const applyModelSelection = useCallback((workspace: WorkspaceIdentity, model: ModelSelection | undefined): void => {
    if (model?.id && model.providerID) {
      const modelID = model.id;
      const providerID = model.providerID;
      const match = modelsByWorkspaceRef.current[workspace.id]?.find(
        (m) => m.id === modelID && m.providerID === providerID
      );
      setCurrentModelByWorkspace((prev) => ({
        ...prev,
        [workspace.id]: {
          ...(match ?? { id: modelID, providerID, name: modelID }),
          ...(model.variant ? { variant: model.variant } : {})
        }
      }));
    }
  }, []);
  const applyAgentSelection = useCallback((workspace: WorkspaceIdentity, agent: string | undefined): void => {
    if (agent) {
      setCurrentAgentByWorkspace((prev) => ({
        ...prev,
        [workspace.id]: agentsByWorkspaceRef.current[workspace.id]?.find((a) => a.id === agent) ?? { id: agent, name: agent }
      }));
    }
  }, []);
  const models = session ? modelsByWorkspace[session.workspace.id] ?? [] : [];
  const currentModel = session ? currentModelByWorkspace[session.workspace.id] ?? null : null;
  const agents = session ? agentsByWorkspace[session.workspace.id] ?? [] : [];
  const currentAgent = session ? currentAgentByWorkspace[session.workspace.id] ?? null : null;

  return {
    models, currentModel, agents, currentAgent, availableModels, lastModel,
    modelsByWorkspace: modelsByWorkspace as Readonly<Record<string, ModelOption[]>>,
    currentModelByWorkspace: currentModelByWorkspace as Readonly<Record<string, ModelOption | null>>,
    agentsByWorkspace: agentsByWorkspace as Readonly<Record<string, AgentOption[]>>,
    currentAgentByWorkspace: currentAgentByWorkspace as Readonly<Record<string, AgentOption | null>>,
    loadModels, switchModel, loadAgents, switchAgent,
    applyModelSelection, applyAgentSelection
  };
}
