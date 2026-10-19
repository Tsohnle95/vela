// @vitest-environment node
import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  app: { getPath: () => "/tmp" },
  shell: { trashItem: vi.fn(), openPath: vi.fn() }
}));
vi.mock("@opencode/client", () => ({ OpenCode: { make: vi.fn() } }));
vi.mock("@opencode/client/service", () => ({ Service: {} }));

import { LatestGeneration } from "@shared/generation";
import type { BackendMessage, FileBaseline, WorkspaceIdentity } from "@shared/types";
import { VelaBackend, type SessionContext } from "./opencode";

const workspace: WorkspaceIdentity = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };

function fixture() {
  const backend = new VelaBackend();
  const directory = "/workspace";
  const client = {
    session: {
      form: {
        list: vi.fn(async () => [{
          id: "frm_session",
          sessionID: "session",
          title: "Session question",
          fields: [{ key: "answer", type: "string" }]
        }]),
        reply: vi.fn(async () => {}),
        cancel: vi.fn(async () => {})
      }
    },
    form: {
      list: vi.fn(async () => ({
        location: { directory },
        data: [{
          id: "frm_global",
          sessionID: "global",
          title: "Global question",
          fields: [{ key: "choice", type: "boolean" }]
        }]
      }))
    }
  };
  const context: SessionContext = {
    workspace,
    sessionID: "session",
    directory,
    sessionInfo: { id: "session", directory, workspace },
    watchContext: {
      root: directory,
      sessionID: "session",
      workspace,
      snapshots: new Map<string, FileBaseline>(),
      lastKnown: new Map<string, string>(),
      hasGit: false,
      timers: new Map()
    },
    watcher: null,
    activations: new LatestGeneration()
  };
  const state = backend as unknown as {
    client: typeof client;
    contexts: Map<string, SessionContext>;
    primary: string | null;
    eventLoop: { current: () => boolean };
  };
  state.client = client;
  state.contexts = new Map([[workspace.id, context]]);
  state.primary = workspace.id;
  state.eventLoop = { current: () => true };
  const messages: BackendMessage[] = [];
  backend.onMessage((message) => messages.push(message as BackendMessage));
  return { backend, client, messages };
}

function deliver(backend: VelaBackend, directory: string, events: unknown[]): Promise<void> {
  return (backend as unknown as {
    deliverEvents: (directory: string, events: unknown[], generation: number) => Promise<void>;
  }).deliverEvents(directory, events, 1);
}

function externalFormEvent(directory: string): unknown {
  return {
    id: "evt_ext",
    created: Date.now(),
    type: "form.created",
    location: { directory },
    data: {
      form: {
        id: "frm_external",
        sessionID: "external-session",
        title: "Questions",
        fields: [{ key: "q", type: "string" }]
      }
    }
  };
}

describe("backend forms", () => {
  it("merges session and location-global pending forms", async () => {
    const { backend, client } = fixture();

    await expect(backend.listForms(workspace)).resolves.toEqual([
      expect.objectContaining({ id: "frm_session", sessionID: "session" }),
      expect.objectContaining({ id: "frm_global", sessionID: "global" })
    ]);
    expect(client.session.form.list).toHaveBeenCalledWith({ sessionID: "session" });
    expect(client.form.list).toHaveBeenCalledWith({ location: { directory: "/workspace" } });
  });

  it("answers and cancels global forms with their location context", async () => {
    const { backend, client } = fixture();

    await backend.replyForm(workspace, "frm_global", { choice: true }, "global");
    await backend.cancelForm(workspace, "frm_global", "global");

    const requestOptions = { headers: { "x-opencode-directory": "%2Fworkspace" } };
    expect(client.session.form.reply).toHaveBeenCalledWith(
      { sessionID: "global", formID: "frm_global", answer: { choice: true } },
      requestOptions
    );
    expect(client.session.form.cancel).toHaveBeenCalledWith(
      { sessionID: "global", formID: "frm_global" },
      requestOptions
    );
  });

  it("rejects a form owned by an unrelated session", async () => {
    const { backend, client } = fixture();

    await expect(backend.replyForm(workspace, "frm_other", {}, "other-session"))
      .rejects.toThrow("form does not belong");
    expect(client.session.form.reply).not.toHaveBeenCalled();
  });

  it("merges location forms owned by other sessions at the workspace", async () => {
    const { backend, client } = fixture();
    client.form.list.mockResolvedValueOnce({
      location: { directory: "/workspace" },
      data: [{
        id: "frm_external",
        sessionID: "external-session",
        title: "TUI question",
        fields: [{ key: "q", type: "string" }]
      }]
    });

    await expect(backend.listForms(workspace)).resolves.toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: "frm_session", sessionID: "session" }),
        expect.objectContaining({ id: "frm_external", sessionID: "external-session" })
      ])
    );
  });

  it("replies to a form owned by another session at the same location", async () => {
    const { backend, client } = fixture();
    client.form.list.mockResolvedValueOnce({
      location: { directory: "/workspace" },
      data: [{ id: "frm_external", sessionID: "external-session", title: "TUI question", fields: [{ key: "q", type: "string" }] }]
    });

    await backend.replyForm(workspace, "frm_external", { q: "yes" }, "external-session");

    expect(client.session.form.reply).toHaveBeenCalledWith(
      { sessionID: "external-session", formID: "frm_external", answer: { q: "yes" } },
      undefined
    );
  });

  it("forwards external-session forms to the panels open at that location", async () => {
    const { backend, messages } = fixture();

    await deliver(backend, "/workspace", [externalFormEvent("/workspace")]);

    const forms = messages.filter((message) => message.kind === "event" && message.type === "form.created");
    expect(forms).toHaveLength(1);
    expect((forms[0] as { data: { velaSessionIDs?: string[] } }).data.velaSessionIDs).toEqual(["session"]);
  });

  it("drops forms from sessions outside every open workspace", async () => {
    const { backend, messages } = fixture();

    await deliver(backend, "/elsewhere", [externalFormEvent("/elsewhere")]);

    expect(messages.filter((message) => message.kind === "event" && message.type === "form.created")).toHaveLength(0);
  });
});
