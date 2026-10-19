import { useCallback, useRef, useState } from "react";
import type { SessionInfo, SessionSummary } from "@shared/types";

export function useSessionInventory(toast: (text: string, tone?: "info" | "error") => void) {
  const [activeSessions, setActiveSessions] = useState<SessionInfo[]>([]);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const activeSessionsRef = useRef(activeSessions);
  activeSessionsRef.current = activeSessions;
  const sessionsRef = useRef(sessions);
  sessionsRef.current = sessions;
  const activeRefreshSequenceRef = useRef(0);
  const refreshActiveSessions = useCallback(async (): Promise<SessionInfo[]> => {
    const sequence = ++activeRefreshSequenceRef.current;
    try {
      const list = await window.vela.activeSessions();
      if (sequence === activeRefreshSequenceRef.current) {
        activeSessionsRef.current = list;
        setActiveSessions(list);
      }
      return list;
    } catch {
      return [];
    }
  }, []);

  const addActiveSession = useCallback((info: SessionInfo): void => {
    setActiveSessions((current) => {
      const index = current.findIndex((session) => session.id === info.id);
      if (index === -1) {
        const next = [...current, info];
        activeSessionsRef.current = next;
        return next;
      }
      if (current[index] === info) return current;
      const next = current.map((session, sessionIndex) => sessionIndex === index ? info : session);
      activeSessionsRef.current = next;
      return next;
    });
  }, []);

  const removeActiveSession = useCallback((sessionID: string): void => {
    setActiveSessions((current) => {
      const next = current.filter((session) => session.id !== sessionID);
      activeSessionsRef.current = next;
      return next;
    });
  }, []);

  const sessionsRequestRef = useRef(0);
  const loadSessions = useCallback(async () => {
    const request = ++sessionsRequestRef.current;
    try {
      const summaries = await window.vela.sessions();
      if (request === sessionsRequestRef.current) setSessions(summaries);
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), "error");
    }
  }, [toast]);

  const activeSessionFor = useCallback((sessionID: string): SessionInfo | undefined =>
    activeSessionsRef.current.find((session) => session.id === sessionID), []);
  const summaryFor = useCallback((sessionID: string): SessionSummary | undefined =>
    sessionsRef.current.find((session) => session.id === sessionID), []);
  const upsertSummary = useCallback((summary: SessionSummary): void => {
    setSessions((current) => {
      const index = current.findIndex((item) => item.id === summary.id);
      if (index === -1) return [summary, ...current];
      return current.map((item, itemIndex) => itemIndex === index ? summary : item);
    });
  }, []);
  const renameSummary = useCallback((sessionID: string, title: string, updatedAt: number): void => {
    setSessions((current) => current.map((item) =>
      item.id === sessionID
        ? { ...item, title, updatedAt }
        : item
    ));
  }, []);
  const removeSummary = useCallback((sessionID: string): void => {
    setSessions((current) => current.filter((item) => item.id !== sessionID));
  }, []);

  return {
    activeSessions, sessions, refreshActiveSessions, addActiveSession,
    removeActiveSession, loadSessions, activeSessionFor, summaryFor,
    upsertSummary, renameSummary, removeSummary
  };
}
