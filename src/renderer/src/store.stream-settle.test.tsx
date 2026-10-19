import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VelaApi } from "../../preload";
import type { BackendMessage, SessionInfo, SessionTranscript, TranscriptItem } from "@shared/types";
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

const staleTail: TranscriptItem[] = [
  { kind: "user", id: "user-1", text: "hello" },
  { kind: "assistant", id: "asst-1", messageID: "msg_1", parts: [], completed: false }
];

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
    openSession: async (directory, generation) => info(directory, generation),
    openSessionById: async (sessionID: string) => ({
      session: { ...info("/reopened", 0), id: sessionID },
      transcript: staleTail,
      todos: []
    }),
    closeSession: async () => {},
    readFile: async () => "content",
    listDir: async () => [],
    listPermissions: async () => [],
    runtimes: async () => [],
    projects: async () => [],
    providerUsage: async () => null,
    ...overrides
  } as VelaApi;
}

describe("store stream settle", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.useFakeTimers();
    messageHandler = null;
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    window.vela = api();
    await act(async () => root.render(<StoreProvider><Probe /></StoreProvider>));
    await act(async () => store.addModelPanel("/one"));
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("projects canonical user time and the responding model through a live turn", async () => {
    const sessionID = store.activeSessionID!;
    const emit = (type: string, created: number, data: Record<string, unknown>) => {
      messageHandler!({ kind: "event", type, data: { id: `${type}-${created}`, created, data: { sessionID, ...data } } });
    };
    await act(async () => {
      emit("message.updated", 1000, {
        info: { id: "user-1", sessionID, role: "user", time: { created: 950 } }
      });
      emit("session.step.started", 1100, {
        assistantMessageID: "msg_2", agent: "build", model: { id: "m", providerID: "p" }, started: 1050
      });
    });
    expect(store.transcript.find((item) => item.id === "user-1")).toMatchObject({
      createdAt: 950, agent: "build", model: { id: "m", providerID: "p" }
    });
    expect(store.transcript.find((item) => item.id === "msg_2")).toMatchObject({
      parentID: "user-1", createdAt: 1100, agent: "build", model: { id: "m", providerID: "p" }, completed: false
    });
    await act(async () => { emit("session.step.ended", 4500, { assistantMessageID: "msg_2", finish: "stop" }); });
    expect(store.transcript.find((item) => item.id === "msg_2")).toMatchObject({
      parentID: "user-1", createdAt: 1100, completedAt: 4500, agent: "build", model: { id: "m", providerID: "p" }, completed: true
    });
  });

  it("marks a reopened stale turn busy and settles it after the quiet window", async () => {
    expect(store.busy).toBe(true);

    await act(async () => { vi.advanceTimersByTime(61_500); });

    expect(store.busy).toBe(false);
    expect(store.transcript.some((item) => item.kind === "assistant" && item.completed)).toBe(true);
  });

  it("restores busy when stream content arrives on a settled session", async () => {
    await act(async () => { vi.advanceTimersByTime(61_500); });
    expect(store.busy).toBe(false);

    const sessionID = store.activeSessionID!;
    const message: BackendMessage = {
      kind: "event",
      type: "message.updated",
      data: {
        id: "evt-1",
        created: Date.now(),
        data: { sessionID, info: { id: "msg_2", sessionID, role: "assistant", time: { created: Date.now() } } }
      }
    };
    await act(async () => { messageHandler!(message); });

    expect(store.busy).toBe(true);

    await act(async () => { vi.advanceTimersByTime(61_500); });

    expect(store.busy).toBe(false);
  });

  it("holds busy across slow-model gaps while execution events reset the quiet clock", async () => {
    await act(async () => { vi.advanceTimersByTime(61_500); });
    expect(store.busy).toBe(false);

    const sessionID = store.activeSessionID!;
    const executionStarted: BackendMessage = {
      kind: "event",
      type: "session.execution.started",
      data: { id: "exec-1", created: Date.now(), data: { sessionID } }
    };
    await act(async () => { messageHandler!(executionStarted); });
    expect(store.busy).toBe(true);

    // 50s of delta silence: still working (the old 15s window flipped to send here).
    await act(async () => { vi.advanceTimersByTime(50_000); });
    expect(store.busy).toBe(true);

    // A fresh authoritative start resets the clock again.
    const executionRestarted: BackendMessage = {
      kind: "event",
      type: "session.execution.started",
      data: { id: "exec-2", created: Date.now(), data: { sessionID } }
    };
    await act(async () => { messageHandler!(executionRestarted); });
    await act(async () => { vi.advanceTimersByTime(50_000); });
    expect(store.busy).toBe(true);

    // Genuine silence still settles the composer.
    await act(async () => { vi.advanceTimersByTime(61_500); });
    expect(store.busy).toBe(false);
  });

  it("does not restore busy when a completed message follows the idle event", async () => {
    const sessionID = store.activeSessionID!;
    const idle: BackendMessage = {
      kind: "event",
      type: "session.idle",
      data: {
        id: "idle-1",
        created: Date.now(),
        data: { sessionID }
      }
    };
    await act(async () => { messageHandler!(idle); });
    expect(store.busy).toBe(false);

    const completed: BackendMessage = {
      kind: "event",
      type: "message.updated",
      data: {
        id: "completed-1",
        created: Date.now(),
        data: {
          sessionID,
          info: {
            id: "msg_2",
            sessionID,
            role: "assistant",
            time: { created: Date.now(), completed: Date.now() },
            finish: "stop"
          }
        }
      }
    };
    await act(async () => { messageHandler!(completed); });

    expect(store.busy).toBe(false);
  });

  it("does not relatch busy when a late unfinished part follows idle", async () => {
    const sessionID = store.activeSessionID!;
    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "session.idle",
        data: { id: "idle-late", created: Date.now(), data: { sessionID } }
      });
    });
    expect(store.busy).toBe(false);

    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "message.part.updated",
        data: {
          id: "late-part",
          created: Date.now(),
          data: {
            sessionID,
            part: { id: "late-text", messageID: "msg_1", sessionID, type: "text", text: "done" }
          }
        }
      });
    });

    expect(store.busy).toBe(false);
  });

  it("returns idle from a final stop message even when no idle event arrives", async () => {
    const sessionID = store.activeSessionID!;
    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "session.status",
        data: { id: "busy-final", created: Date.now(), data: { sessionID, status: { type: "busy" } } }
      });
    });
    expect(store.busy).toBe(true);

    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "message.updated",
        data: {
          id: "stop-final",
          created: Date.now(),
          data: {
            sessionID,
            info: {
              id: "msg_final",
              sessionID,
              role: "assistant",
              time: { created: Date.now(), completed: Date.now() },
              finish: "stop"
            }
          }
        }
      });
    });

    expect(store.busy).toBe(false);
  });

  it("stops a retrying turn locally and ignores late retry events", async () => {
    const sessionID = store.activeSessionID!;
    window.vela.interrupt = vi.fn(async () => {});

    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "session.status",
        data: {
          id: "retrying",
          created: Date.now(),
          data: {
            sessionID,
            status: { type: "retry", attempt: 2, message: "Rate limit exceeded", next: Date.now() + 3_600_000 }
          }
        }
      });
    });
    expect(store.busy).toBe(true);
    expect(store.transcript).toContainEqual(expect.objectContaining({
      kind: "assistant",
      retry: expect.objectContaining({ attempt: 2 })
    }));

    await act(async () => { await store.stop(); });

    expect(window.vela.interrupt).toHaveBeenCalledTimes(1);
    expect(store.busy).toBe(false);
    expect(store.transcript).toContainEqual(expect.objectContaining({ kind: "assistant", completed: true }));
    expect(store.transcript).not.toContainEqual(expect.objectContaining({ kind: "assistant", retry: expect.anything() }));

    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "session.status",
        data: {
          id: "late-retry",
          created: Date.now(),
          data: { sessionID, status: { type: "retry", attempt: 3, message: "Still rate limited", next: Date.now() + 3_600_000 } }
        }
      });
    });

    expect(store.busy).toBe(false);
    expect(store.transcript).not.toContainEqual(expect.objectContaining({ kind: "assistant", retry: expect.anything() }));

    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "session.inbox.enqueued",
        data: {
          id: "queued-steer",
          created: Date.now(),
          data: {
            sessionID,
            inboxID: "queued-steer",
            delivery: "steer",
            item: { type: "user", payload: { text: "continue" }, delivery: "steer" }
          }
        }
      });
    });

    expect(store.panelViews[store.session!.workspace.id]?.queuedMessages).toContainEqual(expect.objectContaining({
      id: "queued-steer",
      content: "continue"
    }));
  });

  it.each([false, true])("preserves distinct visible/off-view close abort disposal (detached=%s)", async (detached) => {
    const sessionID = store.activeSessionID!;
    window.vela.interrupt = vi.fn(async () => {});
    await act(async () => store.stop());
    expect(store.panelViews[store.session!.workspace.id].assistantStatus?.wasAborted).toBe(true);
    if (detached) await act(async () => store.openSession("/two"));
    await act(async () => store.closePanel(sessionID));
    await act(async () => store.reopenSession(sessionID));
    expect(store.panelViews[store.session!.workspace.id].assistantStatus?.wasAborted).toBe(!detached);
  });

  it("keeps pushed execution state authoritative after prompt submission without polling history", async () => {
    await act(async () => { vi.advanceTimersByTime(61_500); });
    expect(store.busy).toBe(false);

    let finishPrompt: ((value: SessionTranscript) => void) | undefined;
    window.vela.prompt = vi.fn(() => new Promise<SessionTranscript>((resolve) => {
      finishPrompt = resolve;
    }));
    window.vela.sessionTranscript = vi.fn(async () => ({ transcript: [], todos: [] }));

    let submitted: Promise<void> | undefined;
    const submittedAt = Date.now();
    await act(async () => {
      submitted = store.sendPrompt("next task");
      await Promise.resolve();
    });
    expect(store.busy).toBe(true);
    expect(store.panelViews[store.session!.workspace.id]?.turnStartedAt).toBeGreaterThanOrEqual(submittedAt);

    await act(async () => {
      finishPrompt!({
        transcript: [
          { kind: "user", id: "remote-user", text: "next task" },
          {
            kind: "assistant",
            id: "remote-assistant",
            messageID: "remote-assistant",
            completed: true,
            parts: [{ kind: "text", id: "remote-text", text: "done", complete: true }]
          }
        ],
        todos: []
      });
      await submitted;
    });

    expect(store.busy).toBe(true);
    expect(window.vela.sessionTranscript).not.toHaveBeenCalled();

    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "session.execution.succeeded",
        data: { id: "execution-done", created: Date.now(), data: { sessionID: store.activeSessionID } }
      });
    });

    expect(store.busy).toBe(false);
  });

  it("retains prompt IPC failures in the transcript with their code", async () => {
    await act(async () => { vi.advanceTimersByTime(61_500); });
    window.vela.prompt = vi.fn(async () => {
      throw Object.assign(new Error("model unavailable"), { code: "MODEL_UNAVAILABLE" });
    });

    await act(async () => { await store.sendPrompt("this fails"); });

    expect(store.busy).toBe(false);
    expect(store.transcript).toContainEqual(expect.objectContaining({
      kind: "status",
      text: "[MODEL_UNAVAILABLE] model unavailable",
      tone: "error"
    }));
  });

  it("retains runtime session failures and settles the composer", async () => {
    const sessionID = store.activeSessionID!;
    await act(async () => {
      messageHandler!({
        kind: "event",
        type: "session.status",
        data: { id: "busy-before-error", created: Date.now(), data: { sessionID, status: { type: "busy" } } }
      });
      messageHandler!({
        kind: "event",
        type: "session.error",
        data: {
          id: "runtime-error",
          created: Date.now(),
          data: { sessionID, error: { code: "PROVIDER_AUTH", message: "Provider authentication failed" } }
        }
      });
    });

    expect(store.busy).toBe(false);
    expect(store.transcript).toContainEqual(expect.objectContaining({
      kind: "status",
      text: "[PROVIDER_AUTH] Provider authentication failed",
      tone: "error"
    }));
  });
});
