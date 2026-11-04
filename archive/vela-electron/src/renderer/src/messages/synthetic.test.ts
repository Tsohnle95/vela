import { describe, expect, it } from "vitest";
import { filterSyntheticParts, isFullySyntheticMessage, isSyntheticPart } from "./synthetic";
import type { ChatPartRecord } from "../chat-store";

function part(id: string, synthetic = false, text = ""): ChatPartRecord {
  return { id, messageID: "msg_1", sessionID: "s", type: "text", text, ...(synthetic ? { synthetic: true } : {}) };
}

describe("synthetic parts", () => {
  it("detects synthetic parts", () => {
    expect(isSyntheticPart(part("p1", true))).toBe(true);
    expect(isSyntheticPart(part("p1"))).toBe(false);
    expect(isSyntheticPart(undefined)).toBe(false);
  });

  it("detects fully synthetic messages", () => {
    expect(isFullySyntheticMessage([part("p1", true), part("p2", true)])).toBe(true);
    expect(isFullySyntheticMessage([part("p1", true), part("p2")])).toBe(false);
    expect(isFullySyntheticMessage([])).toBe(false);
    expect(isFullySyntheticMessage(undefined)).toBe(false);
  });

  it("filters synthetic parts when non-synthetic parts exist", () => {
    const parts = [part("p1"), part("p2", true), part("p3")];
    expect(filterSyntheticParts(parts).map((item) => item.id)).toEqual(["p1", "p3"]);
    expect(filterSyntheticParts(parts.filter((item) => !isSyntheticPart(item))).length).toBe(2);
  });

  it("keeps GitHub context synthetic parts and all-synthetic messages", () => {
    const kept = part("ctx", true, "GitHub issue context (JSON)\n{...}");
    expect(filterSyntheticParts([part("p1"), kept]).map((item) => item.id)).toEqual(["p1", "ctx"]);
    expect(filterSyntheticParts([part("ctx", true, "other")]).map((item) => item.id)).toEqual(["ctx"]);
  });
});
