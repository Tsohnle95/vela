import { describe, expect, it } from "vitest";
import { buildRateLimitNotice, type RateLimitAction } from "./chat-store";

type StatusNotice = { kind: "status"; id: string; text: string; tone: string };

function statusOf(action: RateLimitAction | null | undefined, sessionID: string): StatusNotice {
  const notice = buildRateLimitNotice(action, sessionID);
  expect(notice).not.toBeNull();
  return notice as StatusNotice;
}

describe("buildRateLimitNotice", () => {
  it("returns null when there is no action", () => {
    expect(buildRateLimitNotice(null, "s1")).toBeNull();
    expect(buildRateLimitNotice(undefined, "s1")).toBeNull();
  });

  it("returns null for a non rate-limit retry reason", () => {
    const action: RateLimitAction = { reason: "provider_error", message: "boom" };
    expect(buildRateLimitNotice(action, "s1")).toBeNull();
  });

  it("builds a free-tier notice with the upsell link appended", () => {
    const action: RateLimitAction = {
      reason: "free_tier_limit",
      message: "Subscribe to OpenCode Go for reliable access to open-source models.",
      link: "https://opencode.ai/go"
    };
    const notice = statusOf(action, "s1");
    expect(notice.kind).toBe("status");
    expect(notice.tone).toBe("error");
    expect(notice.id).toBe("s1-ratelimit-free_tier_limit");
    expect(notice.text).toContain("https://opencode.ai/go");
  });

  it("falls back to a default message when the action has none", () => {
    const action: RateLimitAction = { reason: "account_rate_limit", link: "https://opencode.ai/workspace/wk_1/go" };
    const notice = statusOf(action, "s1");
    expect(notice.text).toContain("OpenCode Go usage limit reached");
    expect(notice.text).toContain("https://opencode.ai/workspace/wk_1/go");
  });

  it("does not duplicate a link already present in the message", () => {
    const link = "https://opencode.ai/workspace/wk_1/go";
    const action: RateLimitAction = { reason: "account_rate_limit", message: `Usage limit reached. ${link}`, link };
    const notice = statusOf(action, "s1");
    expect(notice.text.indexOf(link)).toBe(notice.text.lastIndexOf(link));
  });

  it("scopes the notice id per session and reason", () => {
    const action: RateLimitAction = { reason: "free_tier_limit", message: "m", link: "https://opencode.ai/go" };
    expect(statusOf(action, "s1").id).toBe("s1-ratelimit-free_tier_limit");
    expect(statusOf(action, "s2").id).toBe("s2-ratelimit-free_tier_limit");
  });
});
