import { useCallback, useEffect, useRef, useState } from "react";
import type { SessionInfo, SassWatchState } from "@shared/types";
import { sameWorkspace } from "@shared/generation";

export function useSassWatch(session: SessionInfo | null | undefined) {
  const [state, setState] = useState<SassWatchState | null>(null);
  const [pendingID, setPendingID] = useState<string | null>(null);
  const selection = useRef(session);
  selection.current = session;
  const available = typeof window.vela?.sassWatchState === "function" && typeof window.vela?.setSassWatch === "function";
  const apply = useCallback((next: SassWatchState): void => {
    if (!selection.current || !sameWorkspace(selection.current.workspace, next.workspace)) return;
    setState((current) => !current || !sameWorkspace(current.workspace, next.workspace) || next.revision >= current.revision ? next : current);
  }, []);
  const fail = useCallback((captured: SessionInfo, error: unknown): void => {
    if (!selection.current || !sameWorkspace(selection.current.workspace, captured.workspace)) return;
    setState((current) => ({
      ...(current && sameWorkspace(current.workspace, captured.workspace) ? current : { workspace: captured.workspace, directory: captured.directory, enabled: false, revision: 0, logs: [] }),
      status: "error", error: String(error)
    }));
  }, []);
  useEffect(() => {
    if (!session || !available) return;
    let alive = true;
    const captured = session;
    const off = window.vela.onMessage((message) => { if (alive && message.kind === "sass-watch") apply(message.state); });
    void window.vela.sassWatchState(captured.workspace).then((next) => { if (alive) apply(next); }, (error) => { if (alive) fail(captured, error); });
    return () => { alive = false; off(); };
  }, [session?.workspace.id, session?.workspace.generation, available, apply, fail]);
  const current = session && state && sameWorkspace(state.workspace, session.workspace) ? state : null;
  const toggle = async (): Promise<void> => {
    if (!session || !available) return;
    const captured = session;
    const key = `${captured.workspace.id}:${captured.workspace.generation}`;
    setPendingID(key);
    try { apply(await window.vela.setSassWatch(captured.workspace, !current?.enabled)); }
    catch (error) { fail(captured, error); }
    finally { setPendingID((pending) => pending === key ? null : pending); }
  };
  return { state: current, toggle, pending: Boolean(session && pendingID === `${session.workspace.id}:${session.workspace.generation}`), available };
}
