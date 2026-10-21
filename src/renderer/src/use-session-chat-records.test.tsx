import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TranscriptItem } from "@shared/types";
import { useSessionChatRecords } from "./use-session-chat-records";
import type { ChatStreamEvent } from "./chat-stream";

type Owner = ReturnType<typeof useSessionChatRecords>;
let records: Owner;
const user: Extract<TranscriptItem, { kind: "user" }> = { kind: "user", id: "user", text: "hello", createdAt: 10 };
const assistant: Extract<TranscriptItem, { kind: "assistant" }> = {
  kind: "assistant", id: "assistant", messageID: "assistant", completed: false,
  parts: [{ kind: "text", id: "text", text: "hello", complete: false }]
};
function event(type: string, data: Record<string, unknown>): ChatStreamEvent {
  return { id: type, type, created: 1, data };
}

// These type errors protect borrowed views without mutating their runtime values.
function rejectMutation(owner: Owner): void {
  const view = owner.rawChatReadMap.get("session")!;
  // @ts-expect-error owner transcript entries cannot be replaced
  owner.transcriptsBySession.session = [];
  // @ts-expect-error borrowed maps expose no setter
  owner.rawChatReadMap.set("session", view);
  // @ts-expect-error borrowed directories cannot be replaced
  view.message = {};
  // @ts-expect-error borrowed message arrays cannot grow
  view.message.session.push(view.message.session[0]);
  // @ts-expect-error borrowed timestamps cannot change
  view.message.session[0].time.created = 2;
  // @ts-expect-error borrowed retry metadata cannot change
  view.message.session[0].retry!.attempt = 2;
  // @ts-expect-error borrowed part arrays cannot grow
  view.part.assistant.push(view.part.assistant[0]);
  // @ts-expect-error borrowed part text cannot change
  view.part.assistant[0].text = "changed";
  // @ts-expect-error borrowed part state cannot change
  view.part.assistant[0].state!.status = "running";
  // @ts-expect-error borrowed part timestamps cannot change
  view.part.assistant[0].time!.created = 2;
  // @ts-expect-error borrowed status dictionaries cannot change
  view.session_status.session = { type: "idle" };
  // @ts-expect-error borrowed snapshots retain readonly arrays
  owner.snapshotSession("session").message.session.push(view.message.session[0]);
  // @ts-expect-error borrowed status records cannot change
  owner.recordStatusFor("session")!.type = "idle";
}
void rejectMutation;

describe("session chat record authority", () => {
  let root: Root;
  let container: HTMLDivElement;
  let attached: Set<string>;
  const protectedIDs = vi.fn(() => attached);
  function Probe() {
    records = useSessionChatRecords(protectedIDs);
    return null;
  }
  beforeEach(async () => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    protectedIDs.mockClear();
    attached = new Set();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<Probe />));
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("does not invoke protection during render and uses current protection before commit", () => {
    expect(protectedIDs).not.toHaveBeenCalled();
    attached.add("panel");
    act(() => {
      records.appendTranscriptItem("panel", user);
      expect(records.hasTranscript("panel")).toBe(false);
    });
    for (let index = 0; index < 10; index++) act(() => records.appendTranscriptItem(`foreign-${index}`, user));
    expect(records.hasTranscript("panel")).toBe(true);
    expect(Object.keys(records.transcriptsBySession)).toHaveLength(5);
    expect(records.hasTranscript("foreign-9")).toBe(true);
  });

  it("preserves borrowed identities and keeps replay preparation separate from transcript commit", () => {
    act(() => {
      records.mergeMaterializedRecords("session", [user, assistant], false);
      records.projectSession("session");
    });
    const original = records.rawChatReadMap.get("session")!;
    const snapshot = records.snapshotSession("session");
    expect(snapshot.message).not.toBe(original.message);
    expect(snapshot.message.session).toBe(original.message.session);
    expect(snapshot.message.session[0].time).toBe(original.message.session[0].time);
    const rows = records.transcriptsBySession.session;
    records.mergeMaterializedRecords("session", [user, assistant], false);
    expect(records.rawChatReadMap.get("session")).toBe(original);
    const merged = records.prepareReopenedReplay("session", [user, assistant], true);
    expect(records.rawChatReadMap.get("session")).not.toBe(original);
    expect(records.transcriptsBySession.session).toBe(rows);
    act(() => records.commitReplayTranscript("session", merged));
    expect(records.transcriptsBySession.session).toBe(merged);
    expect(records.rawChatReadMap.get("session")!.message.session.find((message) => message.role === "assistant")?.time.completed).toBeDefined();
    records.forgetRecords("session");
    expect(records.rawChatReadMap.has("session")).toBe(false);
    expect(records.hasTranscript("session")).toBe(true);
  });

  it("replaces both transcript and raw records after an authoritative runtime revert", () => {
    act(() => {
      records.appendTranscriptItem("session", user);
      records.appendTranscriptItem("session", assistant);
      records.mergeMaterializedRecords("session", [user, assistant], false);
    });
    const canonical = records.prepareReopenedReplay("session", [], false, true);
    act(() => records.commitReplayTranscript("session", canonical));
    act(() => records.projectSession("session"));
    expect(records.transcriptFor("session")).toEqual([]);
    expect(records.rawChatReadMap.get("session")!.message.session ?? []).toEqual([]);
    expect(records.rawChatReadMap.get("session")!.part[assistant.id]).toBeUndefined();
  });

  it("keeps queued row insertion distinct from prompt and delivered input records", () => {
    act(() => records.appendTranscriptItem("queued", user));
    expect(records.rawChatReadMap.has("queued")).toBe(false);
    act(() => records.appendPromptUser("prompt", user));
    expect(records.rawChatReadMap.get("prompt")!.message.prompt[0].id).toBe(user.id);
    act(() => records.appendTranscriptItem("queued", { kind: "pending-input", id: "inbox", inputType: "user", text: "next" }));
    records.promoteDeliveredInput("queued", "inbox");
    expect(records.rawChatReadMap.get("queued")!.message.queued[0].id).toBe("inbox");
    expect(records.transcriptFor("queued")).toHaveLength(2);
  });

  it("preserves status presence and primitive identities through repeated reduction", () => {
    expect(records.recordStatusFor("session")).toBeUndefined();
    const message = event("session.status", { sessionID: "session", status: { type: "busy" } });
    expect(records.reduceSessionEvent("session", message)).toBe(true);
    const status = records.recordStatusFor("session");
    expect(status).toBe(records.rawChatReadMap.get("session")!.session_status.session);
    expect(records.reduceSessionEvent("session", message)).toBe(false);
    expect(records.recordStatusFor("session")).toBe(status);
  });

  it("does not evaluate duplicate permission fields and retains reply/poll semantics", () => {
    act(() => records.addPermissionFromEvent("session", "request", { action: "read", resources: ["file"] }));
    const data = {} as ChatStreamEvent["data"];
    Object.defineProperty(data, "action", { get() { throw new Error("duplicate action evaluated"); } });
    Object.defineProperty(data, "resources", { get() { throw new Error("duplicate resources evaluated"); } });
    act(() => records.addPermissionFromEvent("session", "request", data));
    const stringify = vi.fn(() => { throw new Error("duplicate action coerced"); });
    const coercionData = { action: { toString: stringify } } as ChatStreamEvent["data"];
    Object.defineProperty(coercionData, "resources", { get() { throw new Error("duplicate resources evaluated"); } });
    act(() => records.addPermissionFromEvent("session", "request", coercionData));
    expect(stringify).not.toHaveBeenCalled();
    expect(records.transcriptFor("session")).toHaveLength(1);
    act(() => records.resolvePermission("session", "request", "once"));
    act(() => records.reconcilePermissionRows("session", []));
    expect(records.transcriptFor("session")[0]).toMatchObject({ pending: false, resolvedWith: "once" });
  });

  it("keeps duplicate-preserving append, unique IDs and tail replacement distinct", () => {
    const first: TranscriptItem = { kind: "status", id: "first", text: "first", tone: "info" };
    const second: TranscriptItem = { kind: "status", id: "second", text: "second", tone: "info" };
    act(() => { records.appendTranscriptItem("session", first); records.appendTranscriptItem("session", second); });
    act(() => records.appendUniqueTranscriptItem("session", first));
    expect(records.transcriptFor("session")).toEqual([first, second]);
    act(() => records.replaceTranscriptItemAtTail("session", first));
    expect(records.transcriptFor("session")).toEqual([second, first]);
    act(() => records.appendTranscriptItem("session", first));
    expect(records.transcriptFor("session")).toEqual([second, first, first]);
  });

  it("deduplicates rate-limit notices by status kind without suppressing same-ID user rows", () => {
    const notice: TranscriptItem = { kind: "status", id: user.id, text: "Rate limit", tone: "info" };
    act(() => records.appendTranscriptItem("session", user));
    act(() => records.appendUniqueTranscriptItem("session", notice));
    expect(records.transcriptFor("session")).toEqual([user]);
    act(() => records.appendRateLimitNotice("session", notice));
    expect(records.transcriptFor("session")).toEqual([user, notice]);
    act(() => records.appendRateLimitNotice("session", notice));
    expect(records.transcriptFor("session")).toEqual([user, notice]);
    act(() => records.appendUniqueTranscriptItem("session", notice));
    expect(records.transcriptFor("session")).toEqual([user, notice]);
  });
});
