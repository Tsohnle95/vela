import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { promises as fsp } from "node:fs";
import { fetchProviderUsage } from "./provider-usage";
import { readOAuthEntries, type OAuthEntry } from "./provider-credentials";
import type { ProviderUsageResult } from "@shared/types";

vi.mock("node:fs", () => {
  const promises = { readFile: vi.fn() };
  return { promises, default: { promises } };
});
vi.mock("./provider-credentials", () => ({ readOAuthEntries: vi.fn() }));

const credentials = vi.mocked(readOAuthEntries);
const files = vi.mocked(fsp.readFile);
const fallback = (provider: string): ProviderUsageResult => ({
  provider, displayName: provider, status: "stale", snapshot: { windows: [], credits: null, planType: "fixture", updatedAt: 1 }
});
const response = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status });

beforeEach(() => {
  vi.stubEnv("COMMANDCODE_API_KEY", "");
  vi.stubEnv("COMMAND_CODE_API_KEY", "");
  credentials.mockReset().mockResolvedValue({});
  files.mockReset().mockResolvedValue(JSON.stringify({ generatedAt: Date.now(), results: [] }));
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

describe("provider refresh ordering and failure semantics", () => {
  it("runs three providers sequentially and retains provider result order", async () => {
    vi.useFakeTimers();
    credentials.mockResolvedValue({
      openai: { type: "oauth", access: "fixture-openai" },
      anthropic: { type: "oauth", access: "fixture-claude" },
      "opencode-go": { type: "oauth", access: "fixture-go" }
    });
    const calls: string[] = [];
    const transport = vi.fn(async (url: string) => {
      const provider = url.includes("chatgpt") ? "openai" : url.includes("anthropic") ? "anthropic" : "opencode-go";
      calls.push(provider);
      await new Promise((resolve) => setTimeout(resolve, provider === "openai" ? 40 : provider === "anthropic" ? 30 : 20));
      return response(provider === "opencode-go" ? { usage: {} } : {});
    });
    vi.stubGlobal("fetch", transport);
    const result = fetchProviderUsage();
    await vi.advanceTimersByTimeAsync(0);
    expect(calls).toEqual(["openai"]);
    await vi.advanceTimersByTimeAsync(40);
    expect(calls).toEqual(["openai", "anthropic"]);
    await vi.advanceTimersByTimeAsync(30);
    expect(calls).toEqual(["openai", "anthropic", "opencode-go"]);
    await vi.advanceTimersByTimeAsync(20);
    expect((await result).map((entry) => [entry.provider, entry.status])).toEqual([
      ["openai", "ok"], ["anthropic", "ok"], ["opencode-go", "ok"]
    ]);
  });

  it("rejects malformed credentials before issuing any later provider requests", async () => {
    credentials.mockResolvedValue({
      openai: { type: "oauth", access: 42 } as unknown as OAuthEntry,
      anthropic: { type: "oauth", access: "fixture-claude" },
      "opencode-go": { type: "oauth", access: "fixture-go" }
    });
    const transport = vi.fn(async () => response({}));
    vi.stubGlobal("fetch", transport);
    await expect(fetchProviderUsage()).rejects.toMatchObject({ name: "TypeError", message: "token.split is not a function" });
    expect(transport).not.toHaveBeenCalled();
  });

  it("does not start later providers after an unexpected parsing failure, even with a snapshot", async () => {
    credentials.mockResolvedValue({ anthropic: { access: "fixture-claude" }, "opencode-go": { access: "fixture-go" } });
    files.mockResolvedValue(JSON.stringify({ generatedAt: Date.now(), results: [fallback("anthropic")] }));
    const transport = vi.fn(async (_url: string) => response({ limits: [null] }));
    vi.stubGlobal("fetch", transport);
    await expect(fetchProviderUsage()).rejects.toThrow();
    expect(transport).toHaveBeenCalledTimes(1);
    expect(transport.mock.calls[0][0]).toContain("anthropic");
  });

  it("continues after expected network failure and keeps the retryable result visible", async () => {
    credentials.mockResolvedValue({ openai: { access: "fixture-openai" }, "opencode-go": { access: "fixture-go" } });
    const transport = vi.fn(async (url: string) => {
      if (url.includes("chatgpt")) throw new Error("Fixture network failure");
      return response({ usage: {} });
    });
    vi.stubGlobal("fetch", transport);
    const results = await fetchProviderUsage();
    expect(results.map((entry) => entry.provider)).toEqual(["openai", "opencode-go"]);
    expect(results[0].error).toMatchObject({ code: "fetch_failed", retryable: true });
    expect(results[1].status).toBe("ok");
  });

  it("uses snapshots for expected authentication failures and missing credentials, appending unknown providers", async () => {
    credentials.mockResolvedValue({ openai: { access: "fixture-openai" } });
    const snapshots = [fallback("custom"), fallback("opencode-go"), fallback("openai")];
    files.mockResolvedValue(JSON.stringify({ generatedAt: Date.now(), results: snapshots }));
    const transport = vi.fn(async () => response({}, 401));
    vi.stubGlobal("fetch", transport);
    expect(await fetchProviderUsage()).toEqual([snapshots[2], snapshots[1], snapshots[0]]);
    expect(transport).toHaveBeenCalledTimes(1);
  });
});

describe("OpenAI account identity characterization", () => {
  const jwt = (payload: unknown): string => `header.${Buffer.from(JSON.stringify(payload)).toString("base64url")}.signature`;
  const cases: Array<{ label: string; entry: unknown; account?: string; missing?: boolean; error?: boolean }> = [
    { label: "null claim body", entry: { access: jwt(null) } },
    { label: "empty claim body", entry: { access: jwt({}) } },
    { label: "array claim body", entry: { access: jwt([]) } },
    { label: "boolean claim body", entry: { access: jwt(true) } },
    { label: "numeric claim body", entry: { access: jwt(42) } },
    { label: "string claim body", entry: { access: jwt("text") } },
    { label: "direct auth claim", entry: { access: jwt({ "https://api.openai.com/auth": "direct" }) }, account: "direct" },
    { label: "nested ChatGPT ID", entry: { access: jwt({ "https://api.openai.com/auth": { chatgpt_account_id: "nested" } }) }, account: "nested" },
    { label: "nested account ID", entry: { access: jwt({ "https://api.openai.com/auth": { account_id: "nested2" } }) }, account: "nested2" },
    { label: "empty nested ID uses root fallback", entry: { access: jwt({ "https://api.openai.com/auth": { chatgpt_account_id: "", account_id: "ignored" }, account_id: "outer" }) }, account: "outer" },
    { label: "empty direct claim uses root fallback", entry: { access: jwt({ "https://api.openai.com/auth": "", account_id: "outer" }) }, account: "outer" },
    { label: "array auth claim uses root fallback", entry: { access: jwt({ "https://api.openai.com/auth": [], account_id: "outer" }) }, account: "outer" },
    { label: "whitespace ID remains unchanged", entry: { access: jwt({ account_id: " " }) }, account: " " },
    { label: "numeric account ID is ignored", entry: { access: jwt({ account_id: 0 }) } },
    { label: "absent access", entry: {}, missing: true },
    { label: "empty access", entry: { access: "" }, missing: true },
    { label: "opaque access", entry: { access: "opaque" } },
    { label: "malformed payload", entry: { access: "header.!!!.signature" } },
    { label: "numeric access", entry: { access: 42 }, error: true },
    { label: "object access", entry: { access: {} }, error: true },
    { label: "null access", entry: { access: null }, missing: true },
    { label: "explicit ID precedes malformed access", entry: { access: 42, accountId: "override" }, account: "override" }
  ];

  it.each(cases)("preserves $label", async ({ entry, account, missing, error }) => {
    credentials.mockResolvedValue({ openai: entry as OAuthEntry });
    const transport = vi.fn(async (_url: string, _options?: RequestInit) => response({}));
    vi.stubGlobal("fetch", transport);
    if (error) {
      await expect(fetchProviderUsage()).rejects.toMatchObject({ name: "TypeError", message: "token.split is not a function" });
      expect(transport).not.toHaveBeenCalled();
      return;
    }
    const results = await fetchProviderUsage();
    if (missing) {
      expect(results[0].error?.code).toBe("missing_oauth");
      expect(transport).not.toHaveBeenCalled();
      return;
    }
    expect(results[0].status).toBe("ok");
    expect(transport).toHaveBeenCalledTimes(1);
    const headers = transport.mock.calls[0][1]!.headers as Record<string, string>;
    expect(headers["ChatGPT-Account-Id"]).toBe(account);
  });
});


describe("Copilot public usage characterization", () => {
  const access = "sku=copilot_for_individual;cq=10;rd=1700000000:0";
  const entry = { access, refresh: "fixture-github", expires: 1 };
  const tokenSnapshot = (updatedAt: number) => ({
    windows: [], planType: "pro", updatedAt,
    credits: { hasCredits: true, unlimited: false, balance: "10", label: "Premium Requests", total: null, used: null, remaining: 10 }
  });
  const failures = [
    { label: "missing refresh", refresh: "", code: "reauth_required", status: "unauthenticated", message: "Copilot usage requires the GitHub token from login. Run: opencode auth login", retryable: false },
    { label: "network rejection", network: true, code: "fetch_failed", status: "unavailable", message: "GitHub Copilot usage request failed (network)", retryable: true },
    { label: "401 before invalid body", http: 401, body: null, code: "reauth_required", status: "unauthenticated", message: "GitHub rejected the Copilot login token. Run: opencode auth login", retryable: false },
    { label: "403 before invalid body", http: 403, body: null, code: "fetch_failed", status: "unavailable", message: "GitHub Copilot usage request failed (403)", retryable: true },
    { label: "500", http: 500, code: "fetch_failed", status: "unavailable", message: "GitHub Copilot usage request failed (500)", retryable: true },
    { label: "null body", body: null, code: "fetch_failed", status: "unavailable", message: "GitHub Copilot usage request failed (empty response)", retryable: true },
    { label: "scalar body", body: 42, code: "fetch_failed", status: "unavailable", message: "GitHub Copilot usage request failed (empty response)", retryable: true },
    { label: "JSON rejection", jsonFailure: true, code: "fetch_failed", status: "unavailable", message: "GitHub Copilot usage request failed (empty response)", retryable: true }
  ];

  it.each(failures)("retains $label result, timing and subsequent-provider ordering", async (fixture) => {
    vi.useFakeTimers();
    vi.setSystemTime(1000);
    credentials.mockResolvedValue({ "github-copilot": { ...entry, refresh: "refresh" in fixture ? fixture.refresh : entry.refresh }, "opencode-go": { access: "fixture-go" } });
    const calls: string[] = [];
    const transport = vi.fn(async (url: string) => {
      calls.push(url);
      if (url.includes("opencode.ai")) return response({ usage: {} });
      vi.setSystemTime(2000);
      if ("network" in fixture) throw new Error("network fixture");
      const status = "http" in fixture ? fixture.http : 200;
      return { ok: status === 200, status, json: async () => {
        vi.setSystemTime(3000);
        if ("jsonFailure" in fixture) throw new Error("JSON fixture");
        return "body" in fixture ? fixture.body : {};
      } };
    });
    vi.stubGlobal("fetch", transport);
    const results = await fetchProviderUsage();
    const updatedAt = "refresh" in fixture ? 1000 : "network" in fixture ? 2000 : 3000;
    expect(results[0]).toEqual({ provider: "github-copilot", displayName: "GitHub Copilot", status: fixture.status, snapshot: tokenSnapshot(updatedAt), error: { code: fixture.code, message: fixture.message, retryable: fixture.retryable } });
    expect(results[1].provider).toBe("opencode-go");
    expect(results[1].status).toBe("ok");
    expect(calls.map((url) => url.includes("opencode.ai") ? "go" : "copilot")).toEqual("refresh" in fixture ? ["go"] : ["copilot", "go"]);
    if (!("refresh" in fixture)) {
      expect(transport.mock.calls[0][0]).toBe("https://api.github.com/copilot_internal/user");
    }
  });

  it.each([undefined, "", null])("keeps missing access %s unauthenticated without a fallback", async (access) => {
    credentials.mockResolvedValue({ "github-copilot": { access, refresh: "fixture" } as OAuthEntry });
    const transport = vi.fn();
    vi.stubGlobal("fetch", transport);
    expect(await fetchProviderUsage()).toEqual([{ provider: "github-copilot", displayName: "GitHub Copilot", status: "unauthenticated", snapshot: null, error: { code: "missing_oauth", message: "GitHub Copilot is not authenticated.", retryable: false } }]);
    expect(transport).not.toHaveBeenCalled();
  });

  it("keeps malformed access failure before any later requests", async () => {
    credentials.mockResolvedValue({ "github-copilot": { access: 42 } as unknown as OAuthEntry, "opencode-go": { access: "fixture-go" } });
    const transport = vi.fn();
    vi.stubGlobal("fetch", transport);
    await expect(fetchProviderUsage()).rejects.toMatchObject({ name: "TypeError", message: "accessToken.split is not a function or its return value is not iterable" });
    expect(transport).not.toHaveBeenCalled();
  });

  it("evaluates fallback time before malformed status coercion", async () => {
    credentials.mockResolvedValue({ "github-copilot": entry });
    const order: string[] = [];
    const clock = vi.spyOn(Date, "now").mockImplementation(() => { order.push("clock"); return 1000; });
    vi.stubGlobal("fetch", vi.fn(async () => ({ ok: false, status: { toString() { order.push("status"); throw new Error("status fixture"); } }, json: async () => { order.push("json"); return {}; } })));
    try {
      await expect(fetchProviderUsage()).rejects.toThrow("status fixture");
      expect(order.slice(-3)).toEqual(["json", "clock", "status"]);
    } finally { clock.mockRestore(); }
  });

  it("does not create a token snapshot when token metadata is absent", async () => {
    credentials.mockResolvedValue({ "github-copilot": { access: "opaque" } });
    const transport = vi.fn();
    vi.stubGlobal("fetch", transport);
    const [result] = await fetchProviderUsage();
    expect(result.snapshot).toBeNull();
    expect(result.error?.code).toBe("reauth_required");
    expect(transport).not.toHaveBeenCalled();
  });

  it("preserves GitHub headers and successful response precedence without mutating input", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1000);
    credentials.mockResolvedValue({ "github-copilot": entry });
    const body = { quota_reset_date: "2026-10-15T12:00:00Z", copilot_plan: "copilot_business", token_based_billing: true, quota_snapshots: {
      premium_models: { entitlement: 10.9, quota_remaining: 0, remaining: 9, percent_remaining: 25, overage_permitted: true },
      premium_interactions: { credits_used: 2.5, entitlement: 200, remaining: 100 }, chat: { credits_used: 8 }
    } };
    const prior = structuredClone(body);
    const transport = vi.fn(async (_url: string, _options?: RequestInit) => ({ ok: true, status: 200, json: async () => { vi.setSystemTime(3000); return body; } }));
    vi.stubGlobal("fetch", transport);
    expect((await fetchProviderUsage())[0]).toEqual({ provider: "github-copilot", displayName: "GitHub Copilot", status: "ok", snapshot: {
      windows: [{ id: "monthly", label: "Monthly", usedPercent: 75, windowMinutes: null, resetsAt: Date.parse(body.quota_reset_date) / 1000 }],
      credits: { hasCredits: false, unlimited: false, balance: null, label: "GitHub AI Credits", overagePermitted: true, total: null, used: 2.5, remaining: null }, planType: "business", updatedAt: 3000
    } });
    expect(body).toEqual(prior);
    expect(transport.mock.calls[0][1]!.headers).toEqual({ Authorization: "token fixture-github", Accept: "application/json", "Editor-Version": "vscode/1.96.2", "Editor-Plugin-Version": "copilot-chat/0.26.7", "User-Agent": "GitHubCopilotChat/0.26.7", "X-GitHub-Api-Version": "2026-03-10" });
    expect(transport.mock.calls[0][1]!.signal).toBeDefined();
  });

  const quotas = [
    { label: "fractional entitlement and negative remaining", premium: { entitlement: 10.9, remaining: -2.7 }, windows: [100], credits: { hasCredits: false, unlimited: false, balance: "0", label: "Premium Requests", overagePermitted: false, total: 10, used: 10, remaining: 0 } },
    { label: "nonfinite quota with unlimited and negative credits", premium: { entitlement: Infinity, remaining: NaN, unlimited: true, credits_used: -3 }, windows: [], credits: { hasCredits: true, unlimited: true, balance: null, label: "GitHub AI Credits", overagePermitted: false, total: null, used: 0, remaining: null } },
    { label: "null quota uses remaining", premium: { entitlement: 10, quota_remaining: null, remaining: 4 }, windows: [60], credits: { hasCredits: true, unlimited: false, balance: "4", label: "Premium Requests", overagePermitted: false, total: 10, used: 6, remaining: 4 } },
    { label: "percent clamps above usage range", premium: { entitlement: 10, remaining: 4, percent_remaining: -5 }, windows: [100], credits: { hasCredits: true, unlimited: false, balance: "4", label: "Premium Requests", overagePermitted: false, total: 10, used: 6, remaining: 4 } },
    { label: "percent NaN retains zero", premium: { percent_remaining: NaN }, windows: [0], credits: null },
    { label: "empty premium suppresses token quota", premium: {}, windows: [], credits: null }
  ];
  it.each(quotas)("retains $label", async ({ premium, windows, credits }) => {
    credentials.mockResolvedValue({ "github-copilot": entry });
    vi.stubGlobal("fetch", vi.fn(async () => ({ ok: true, status: 200, json: async () => ({ quota_snapshots: { premium_interactions: premium } }) })));
    const [result] = await fetchProviderUsage();
    expect(result.status).toBe("ok");
    expect(result.snapshot?.windows.map((window) => window.usedPercent)).toEqual(windows);
    if (windows.length) expect(result.snapshot?.windows[0].resetsAt).toBe(1700000000);
    expect(result.snapshot?.credits).toEqual(credits);
    expect(result.snapshot?.planType).toBe("pro");
  });

  it("accepts array body as an empty successful quota response", async () => {
    credentials.mockResolvedValue({ "github-copilot": entry });
    vi.stubGlobal("fetch", vi.fn(async () => response([])));
    const [result] = await fetchProviderUsage();
    expect(result.status).toBe("ok");
    expect(result.snapshot?.windows).toEqual([]);
    expect(result.snapshot?.credits).toEqual({ ...tokenSnapshot(0).credits, overagePermitted: false });
  });

  it("keeps unexpected quota errors before success timestamp and later providers", async () => {
    credentials.mockResolvedValue({ "github-copilot": entry, "opencode-go": { access: "fixture-go" } });
    const order: string[] = [];
    const clock = vi.spyOn(Date, "now").mockImplementation(() => { order.push("clock"); return 1000; });
    const body = { get quota_snapshots() { order.push("quota"); throw new Error("quota fixture"); } };
    const transport = vi.fn(async () => ({ ok: true, status: 200, json: async () => { order.push("json"); return body; } }));
    vi.stubGlobal("fetch", transport);
    try {
      await expect(fetchProviderUsage()).rejects.toThrow("quota fixture");
      expect(order.slice(-2)).toEqual(["json", "quota"]);
      expect(transport).toHaveBeenCalledTimes(1);
    } finally { clock.mockRestore(); }
  });

  it.each([false, true])("keeps disk fallback precedence for live success=%s", async (ok) => {
    credentials.mockResolvedValue({ "github-copilot": entry });
    const cached = fallback("github-copilot");
    files.mockResolvedValue(JSON.stringify({ generatedAt: Date.now(), results: [cached] }));
    vi.stubGlobal("fetch", vi.fn(async () => response({}, ok ? 200 : 401)));
    const [result] = await fetchProviderUsage();
    if (ok) { expect(result.status).toBe("ok"); expect(result.snapshot?.planType).toBe("pro"); }
    else expect(result).toEqual(cached);
  });

  it("preserves error defaults for every other provider", async () => {
    credentials.mockResolvedValue({ openai: {}, anthropic: {}, "opencode-go": {}, "command-code": {} });
    vi.stubGlobal("fetch", vi.fn());
    expect((await fetchProviderUsage()).map(({ status, snapshot, error }) => ({ status, snapshot, retryable: error?.retryable, code: error?.code }))).toEqual(Array.from({ length: 4 }, () => ({ status: "unauthenticated", snapshot: null, retryable: false, code: "missing_oauth" })));
  });
});
