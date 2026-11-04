import { describe, expect, it } from "vitest";
import { IDLE_ACTIVITY, resolveSessionActivity } from "./session-activity";

describe("session activity", () => {
  it("is idle without a session", () => {
    expect(resolveSessionActivity({
      sessionId: null,
      statusType: "busy",
      trailingAssistantIncomplete: false,
      pendingPermissions: 0
    })).toBe(IDLE_ACTIVITY);
  });

  it("is idle when a permission is pending so the send button stays a send", () => {
    expect(resolveSessionActivity({
      sessionId: "s",
      statusType: "busy",
      trailingAssistantIncomplete: true,
      pendingPermissions: 1
    })).toEqual(IDLE_ACTIVITY);
  });

  it("mirrors the authoritative busy status", () => {
    expect(resolveSessionActivity({
      sessionId: "s",
      statusType: "busy",
      trailingAssistantIncomplete: false,
      pendingPermissions: 0
    })).toEqual({ phase: "busy", isWorking: true, isBusy: true, isCooldown: false });
  });

  it("mirrors retry status", () => {
    expect(resolveSessionActivity({
      sessionId: "s",
      statusType: "retry",
      trailingAssistantIncomplete: false,
      pendingPermissions: 0
    })).toEqual({ phase: "retry", isWorking: true, isBusy: false, isCooldown: false });
  });

  it("falls back to the trailing incomplete assistant while status settles", () => {
    expect(resolveSessionActivity({
      sessionId: "s",
      statusType: undefined,
      trailingAssistantIncomplete: true,
      pendingPermissions: 0
    })).toEqual({ phase: "busy", isWorking: true, isBusy: true, isCooldown: false });
  });

  it("honors authoritative idle despite a trailing incomplete assistant", () => {
    expect(resolveSessionActivity({
      sessionId: "s",
      statusType: "idle",
      trailingAssistantIncomplete: true,
      pendingPermissions: 0
    })).toBe(IDLE_ACTIVITY);
  });

  it("is idle when the trailing assistant completed", () => {
    expect(resolveSessionActivity({
      sessionId: "s",
      statusType: undefined,
      trailingAssistantIncomplete: false,
      pendingPermissions: 0
    })).toBe(IDLE_ACTIVITY);
  });
});
