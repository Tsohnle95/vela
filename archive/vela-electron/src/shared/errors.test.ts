import { describe, expect, it } from "vitest";
import { formatFailure, normalizeFailure } from "./errors";

describe("failure reporting", () => {
  it("preserves native codes and details", () => {
    expect(normalizeFailure({ code: "session-not-found", message: "missing", details: { sessionID: "s" } })).toEqual({
      code: "SESSION-NOT-FOUND",
      message: "missing",
      details: '{"sessionID":"s"}'
    });
  });

  it("adds a stable code when an error has only a message", () => {
    expect(formatFailure(new Error("provider unavailable"), "VELA_PROMPT_FAILED", "Prompt failed"))
      .toBe("[VELA_PROMPT_FAILED] provider unavailable");
  });

  it("recovers a code after IPC serializes an error into its message", () => {
    expect(normalizeFailure("[DEEPSEEK_TURN_FAILED] Provider failed", "VELA_PROMPT_FAILED")).toEqual({
      code: "DEEPSEEK_TURN_FAILED",
      message: "Provider failed"
    });
  });
});
