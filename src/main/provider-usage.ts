import { promises as fsp } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { readOAuthEntries, type OAuthEntry } from "./provider-credentials";
import type {
  ProviderUsageCredits,
  ProviderUsageError,
  ProviderUsageResult,
  ProviderUsageSnapshot,
  UsageWindow
} from "@shared/types";

const TIMEOUT_MS = 10_000;
const REAUTH_OPENAI = "OpenAI ChatGPT credentials expired. Send a prompt to refresh them or run: opencode auth login";
const REAUTH_CLAUDE = "Claude credentials expired. Send a prompt to refresh them or run: opencode auth login";

function expired(entry: OAuthEntry): boolean {
  return typeof entry.expires === "number" && entry.expires > 0 && Date.now() >= entry.expires;
}

function chatgptAccountId(entry: OAuthEntry): string | null {
  if (entry.accountId) return entry.accountId;
  const token = entry.access ?? "";
  const payload = token.split(".")[1];
  if (!payload) return null;
  try {
    const decoded = JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as Record<string, unknown>;
    const authClaim = decoded["https://api.openai.com/auth"];
    if (typeof authClaim === "string" && authClaim) return authClaim;
    const auth = asRecord(authClaim);
    const nested = auth?.chatgpt_account_id ?? auth?.account_id;
    if (typeof nested === "string" && nested) return nested;
    const account = decoded.account_id;
    return typeof account === "string" && account ? account : null;
  } catch {
    return null;
  }
}

function errorResult(
  provider: string,
  displayName: string,
  code: ProviderUsageError["code"],
  message: string,
  retryable = false,
  { status = "unauthenticated", snapshot = null }: Partial<Pick<ProviderUsageResult, "status" | "snapshot">> = {}
): ProviderUsageResult {
  return { provider, displayName, status, snapshot, error: { code, message, retryable } };
}

async function fetchJson(
  url: string,
  headers: Record<string, string>
): Promise<{ ok: boolean; status: number; body: unknown } | null> {
  const response = await fetch(url, { headers, signal: AbortSignal.timeout(TIMEOUT_MS) }).catch(() => null);
  if (!response) return null;
  const body = await response.json().catch(() => null);
  return { ok: response.ok, status: response.status, body };
}

function windowIdentity(minutes: number): { id: string; label: string } {
  const known: Array<[number, string, string]> = [
    [5 * 60, "5h", "5h"],
    [24 * 60, "daily", "Daily"],
    [7 * 24 * 60, "weekly", "Weekly"],
    [30 * 24 * 60, "monthly", "Monthly"]
  ];
  for (const [duration, id, label] of known) {
    if (Math.abs(minutes - duration) <= duration * 0.1) return { id, label };
  }
  if (minutes < 48 * 60) return { id: `w${minutes}`, label: `${Math.max(1, Math.round(minutes / 60))}h` };
  return { id: `w${minutes}`, label: `${Math.max(1, Math.round(minutes / (24 * 60)))}d` };
}

interface ChatgptWindow {
  used_percent?: unknown;
  limit_window_seconds?: unknown;
  reset_at?: unknown;
}

function chatgptWindow(raw: ChatgptWindow | null | undefined): UsageWindow | null {
  if (!raw) return null;
  const minutes = Math.round(Number(raw.limit_window_seconds) / 60);
  if (!Number.isFinite(minutes) || minutes <= 0) return null;
  const identity = windowIdentity(minutes);
  return {
    id: identity.id,
    label: identity.label,
    usedPercent: Number(raw.used_percent) || 0,
    windowMinutes: minutes,
    resetsAt: Number(raw.reset_at) > 0 ? Math.floor(Number(raw.reset_at)) : null
  };
}

function chatgptWindows(
  primary: ChatgptWindow | null | undefined,
  secondary: ChatgptWindow | null | undefined
): UsageWindow[] {
  const first = chatgptWindow(primary);
  const second = chatgptWindow(secondary);
  const deduped = first && second && first.id === second.id ? { ...second, id: `${second.id}-2` } : second;
  return [first, deduped]
    .filter((window): window is UsageWindow => window !== null)
    .sort((a, b) => (a.windowMinutes ?? 0) - (b.windowMinutes ?? 0));
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" ? (value as Record<string, unknown>) : null;
}

async function fetchChatgpt(entry: OAuthEntry): Promise<ProviderUsageResult> {
  const displayName = "OpenAI ChatGPT";
  if (!entry.access) return errorResult("openai", displayName, "missing_oauth", `${displayName} is not authenticated.`);
  if (expired(entry)) return errorResult("openai", displayName, "reauth_required", REAUTH_OPENAI);

  const headers: Record<string, string> = {
    Authorization: `Bearer ${entry.access}`,
    Accept: "application/json"
  };
  const accountId = chatgptAccountId(entry);
  if (accountId) headers["ChatGPT-Account-Id"] = accountId;

  const response = await fetchJson("https://chatgpt.com/backend-api/wham/usage", headers);
  if (!response) return errorResult("openai", displayName, "fetch_failed", `${displayName} usage request failed (network)`, true);
  if (response.status === 401) return errorResult("openai", displayName, "reauth_required", REAUTH_OPENAI);
  if (!response.ok) return errorResult("openai", displayName, "fetch_failed", `${displayName} usage request failed (${response.status})`, true);

  const body = asRecord(response.body);
  if (!body) return errorResult("openai", displayName, "fetch_failed", `${displayName} usage request failed (empty response)`, true);

  const rateLimit = asRecord(body.rate_limit);
  const windows = [
    ...chatgptWindows(rateLimit ? rateLimit.primary_window as ChatgptWindow : null, rateLimit ? rateLimit.secondary_window as ChatgptWindow : null),
    ...chatgptAdditionalWindows(body.additional_rate_limits),
    ...chatgptSpendWindows(asRecord(body.spend_control))
  ];
  const credits = chatgptCredits(asRecord(body.credits)) ?? chatgptSpendCredits(asRecord(body.spend_control));
  const planType = typeof body.plan_type === "string" ? body.plan_type : null;

  return {
    provider: "openai",
    displayName,
    status: "ok",
    snapshot: { windows, credits, planType, updatedAt: Date.now() }
  };
}

function chatgptAdditionalWindows(limits: unknown): UsageWindow[] {
  if (!Array.isArray(limits)) return [];
  return limits.flatMap((limit) => {
    const record = asRecord(limit);
    if (!record) return [];
    const rateLimit = asRecord(record.rate_limit);
    if (!rateLimit) return [];
    const id = typeof record.metered_feature === "string" ? record.metered_feature.trim() : "";
    const label = typeof record.limit_name === "string" && record.limit_name.trim() ? record.limit_name.trim() : id;
    if (!id || !label) return [];
    return chatgptWindows(rateLimit.primary_window as ChatgptWindow, rateLimit.secondary_window as ChatgptWindow).map(
      (window) => ({ ...window, id: `${id}:${window.id}`, label: `${label} ${window.label}` })
    );
  });
}

function chatgptSpendWindows(spend: Record<string, unknown> | null): UsageWindow[] {
  const individual = asRecord(spend?.individual_limit);
  if (!individual) return [];
  const remainingPercent = Math.min(100, Math.max(0, Number(individual.remaining_percent) || 0));
  return [
    {
      id: "monthly-credit-limit",
      label: "Monthly Credit",
      usedPercent: 100 - remainingPercent,
      windowMinutes: null,
      resetsAt: Number(individual.reset_at) > 0 ? Math.floor(Number(individual.reset_at)) : null
    }
  ];
}

function chatgptCredits(credits: Record<string, unknown> | null): ProviderUsageCredits | null {
  if (!credits || typeof credits.has_credits !== "boolean") return null;
  return {
    hasCredits: credits.has_credits,
    unlimited: credits.unlimited === true,
    balance: typeof credits.balance === "string" ? credits.balance : null,
    label: "Credits Balance"
  };
}

function chatgptSpendCredits(spend: Record<string, unknown> | null): ProviderUsageCredits | null {
  const individual = asRecord(spend?.individual_limit);
  if (!individual) return null;
  const total = parseCreditAmount(individual.limit);
  const used = parseCreditAmount(individual.used);
  if (total === null || used === null) return null;
  const remaining = Math.max(0, total - used);
  return {
    hasCredits: spend?.reached !== true && remaining > 0,
    unlimited: false,
    balance: String(remaining),
    label: "Monthly Credit Limit",
    total,
    used,
    remaining
  };
}

function parseCreditAmount(value: unknown): number | null {
  const normalized = typeof value === "string" ? value.trim() : "";
  if (!normalized) return null;
  const amount = Number(normalized);
  if (!Number.isFinite(amount) || amount < 0) return null;
  return Math.round(amount);
}

interface ClaudeWindow {
  utilization?: unknown;
  resets_at?: unknown;
}

interface ClaudeLimit {
  kind?: unknown;
  group?: unknown;
  percent?: unknown;
  resets_at?: unknown;
  scope?: { model?: { display_name?: unknown } } | null;
}

function parseResetsAt(value: unknown): number | null {
  if (typeof value !== "string" || !value) return null;
  const ms = new Date(value).getTime();
  if (Number.isNaN(ms)) return null;
  return Math.floor(ms / 1000);
}

function claudeWindow(id: string, label: string, raw: ClaudeWindow | null | undefined, windowMinutes: number | null): UsageWindow | null {
  if (!raw) return null;
  return {
    id,
    label,
    usedPercent: Number(raw.utilization) || 0,
    windowMinutes,
    resetsAt: parseResetsAt(raw.resets_at)
  };
}

function claudeLimitWindow(id: string, label: string, limit: ClaudeLimit | null, windowMinutes: number | null): UsageWindow | null {
  if (!limit || typeof limit.percent !== "number") return null;
  return {
    id,
    label,
    usedPercent: limit.percent,
    windowMinutes,
    resetsAt: parseResetsAt(limit.resets_at)
  };
}

function scopedWindowId(name: string, weekly: boolean): string {
  const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  return weekly ? `weekly-${slug}` : `scoped-${slug}`;
}

async function fetchClaude(entry: OAuthEntry): Promise<ProviderUsageResult> {
  const displayName = "Anthropic Claude";
  if (!entry.access) return errorResult("anthropic", displayName, "missing_oauth", `${displayName} is not authenticated.`);
  if (expired(entry)) return errorResult("anthropic", displayName, "reauth_required", REAUTH_CLAUDE);

  const response = await fetchJson("https://api.anthropic.com/api/oauth/usage", {
    Authorization: `Bearer ${entry.access}`,
    "anthropic-beta": "oauth-2025-04-20",
    Accept: "application/json"
  });
  if (!response) return errorResult("anthropic", displayName, "fetch_failed", `${displayName} usage request failed (network)`, true);
  if (response.status === 401) return errorResult("anthropic", displayName, "reauth_required", REAUTH_CLAUDE);
  if (!response.ok) return errorResult("anthropic", displayName, "fetch_failed", `${displayName} usage request failed (${response.status})`, true);

  const body = asRecord(response.body);
  if (!body) return errorResult("anthropic", displayName, "fetch_failed", `${displayName} usage request failed (empty response)`, true);

  const limits = (Array.isArray(body.limits) ? body.limits : []) as ClaudeLimit[];
  const unscoped = (kind: string): ClaudeLimit | null =>
    limits.find((limit) => limit.kind === kind && !limit.scope?.model?.display_name?.toString().trim()) ?? null;

  const session = claudeWindow("5h", "5h", body.five_hour as ClaudeWindow, 5 * 60) ??
    claudeLimitWindow("session", "Session", unscoped("session"), null);
  const weekly = claudeWindow("weekly", "Weekly", body.seven_day as ClaudeWindow, 7 * 24 * 60) ??
    claudeLimitWindow("weekly", "Weekly", unscoped("weekly_all"), 7 * 24 * 60);

  const scopedIds = new Set<string>();
  const scoped = limits.flatMap((limit) => {
    const name = limit.scope?.model?.display_name?.toString().trim();
    if (!name) return [];
    const isWeekly = limit.kind === "weekly_scoped" || limit.group === "weekly";
    const id = scopedWindowId(name, isWeekly);
    if (scopedIds.has(id)) return [];
    scopedIds.add(id);
    const window = claudeLimitWindow(id, isWeekly ? `${name} Weekly` : name, limit, isWeekly ? 7 * 24 * 60 : null);
    return window ? [window] : [];
  });

  const extra = asRecord(body.extra_usage);
  const credits: ProviderUsageCredits | null = (() => {
    if (!extra || extra.is_enabled === false) return null;
    const limit = typeof extra.monthly_limit === "number" ? extra.monthly_limit : null;
    const used = typeof extra.used_credits === "number" ? extra.used_credits : null;
    if (limit !== null && used !== null) {
      const remaining = Math.max(0, Math.round((limit - used) * 100) / 100);
      return { hasCredits: remaining > 0, unlimited: false, balance: String(remaining), label: "Usage Credits" };
    }
    const utilization = typeof extra.utilization === "number" ? extra.utilization : null;
    if (utilization !== null) {
      return { hasCredits: utilization < 100, unlimited: false, balance: null, label: "Usage Credits" };
    }
    return null;
  })();

  return {
    provider: "anthropic",
    displayName,
    status: "ok",
    snapshot: {
      windows: [session, weekly, ...scoped].filter((window): window is UsageWindow => window !== null),
      credits,
      planType: null,
      updatedAt: Date.now()
    }
  };
}

interface CopilotTokenMetadata {
  sku?: string;
  quotaLimit?: number;
  resetDate?: number;
}

function parseCopilotAccessToken(accessToken: string): CopilotTokenMetadata {
  const result: CopilotTokenMetadata = {};
  for (const part of accessToken.split(";")) {
    const eqIndex = part.indexOf("=");
    if (eqIndex === -1) continue;
    const key = part.slice(0, eqIndex);
    const value = part.slice(eqIndex + 1);
    if (key === "sku") result.sku = value;
    if (key === "cq") result.quotaLimit = parseTokenInteger(value);
    if (key === "rd") {
      const colon = value.indexOf(":");
      if (colon > 0) result.resetDate = parseTokenInteger(value.slice(0, colon));
    }
  }
  return result;
}

function parseTokenInteger(value: string): number | undefined {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed < 0) return undefined;
  return parsed;
}

function copilotSkuToPlan(sku: string | undefined): string | null {
  if (!sku) return null;
  const map: Record<string, string> = {
    free_limited_copilot: "free",
    copilot_for_individual: "pro",
    copilot_individual: "pro",
    copilot_business: "business",
    copilot_enterprise: "enterprise",
    copilot_for_business: "business"
  };
  if (map[sku]) return map[sku];
  const normalized = sku.toLowerCase();
  if (normalized.includes("free")) return "free";
  if (normalized.includes("individual") || normalized.includes("pro")) return "pro";
  if (normalized.includes("business")) return "business";
  if (normalized.includes("enterprise")) return "enterprise";
  return null;
}

function creditAmount(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  return Math.max(0, value);
}

function creditCount(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  return Math.max(0, Math.floor(value));
}

function clampPercent(value: number): number {
  if (Number.isNaN(value)) return 0;
  if (value < 0) return 0;
  if (value > 100) return 100;
  return value;
}

async function fetchCopilot(entry: OAuthEntry): Promise<ProviderUsageResult> {
  const displayName = "GitHub Copilot";
  const token = parseCopilotAccessToken(entry.access ?? "");
  const planType = copilotSkuToPlan(token.sku);

  const snapshotFromToken = (): ProviderUsageSnapshot | null => {
    const credits =
      token.quotaLimit !== null && token.quotaLimit !== undefined
        ? {
            hasCredits: token.quotaLimit > 0,
            unlimited: false,
            balance: String(token.quotaLimit),
            label: "Premium Requests",
            total: null,
            used: null,
            remaining: token.quotaLimit
          }
        : null;
    if (!credits && !planType) return null;
    return { windows: [], credits, planType, updatedAt: Date.now() };
  };

  if (!entry.access) return errorResult("github-copilot", displayName, "missing_oauth", `${displayName} is not authenticated.`);
  if (!entry.refresh) {
    return errorResult(
      "github-copilot", displayName, "reauth_required",
      "Copilot usage requires the GitHub token from login. Run: opencode auth login", false, { snapshot: snapshotFromToken() }
    );
  }

  const response = await fetchJson("https://api.github.com/copilot_internal/user", {
    Authorization: `token ${entry.refresh}`,
    Accept: "application/json",
    "Editor-Version": "vscode/1.96.2",
    "Editor-Plugin-Version": "copilot-chat/0.26.7",
    "User-Agent": "GitHubCopilotChat/0.26.7",
    "X-GitHub-Api-Version": "2026-03-10"
  });
  if (!response) {
    return errorResult(
      "github-copilot", displayName, "fetch_failed",
      `${displayName} usage request failed (network)`, true, { status: "unavailable", snapshot: snapshotFromToken() }
    );
  }
  if (response.status === 401) {
    return errorResult(
      "github-copilot", displayName, "reauth_required",
      "GitHub rejected the Copilot login token. Run: opencode auth login", false, { snapshot: snapshotFromToken() }
    );
  }
  if (!response.ok) {
    const snapshot = snapshotFromToken();
    return errorResult(
      "github-copilot", displayName, "fetch_failed",
      `${displayName} usage request failed (${response.status})`, true, { status: "unavailable", snapshot }
    );
  }

  const body = asRecord(response.body);
  if (!body) {
    return errorResult(
      "github-copilot", displayName, "fetch_failed",
      `${displayName} usage request failed (empty response)`, true, { status: "unavailable", snapshot: snapshotFromToken() }
    );
  }

  return {
    provider: "github-copilot",
    displayName,
    status: "ok",
    snapshot: { ...copilotQuota(body, token, planType), updatedAt: Date.now() }
  };
}

function copilotQuota(
  body: Record<string, unknown>,
  token: CopilotTokenMetadata,
  planType: string | null
): Omit<ProviderUsageSnapshot, "updatedAt"> {
  const snapshots = asRecord(body.quota_snapshots) ?? {};
  const premium = asRecord(snapshots.premium_models) ?? asRecord(snapshots.premium_interactions);
  const tokenBasedBilling = body.token_based_billing === true || premium?.token_based_billing === true;
  const resetAt = parseResetsAt(body.quota_reset_date) ?? token.resetDate ?? null;
  const resolvedPlanType = copilotSkuToPlan(typeof body.copilot_plan === "string" ? body.copilot_plan : undefined) ?? planType;
  const creditsUsed =
    [snapshots.premium_interactions, snapshots.premium_models, snapshots.chat, snapshots.completions]
      .map((snapshot) => creditAmount(asRecord(snapshot)?.credits_used))
      .find((value) => value !== null) ?? null;

  const usageCounts = (() => {
    if (!premium) return null;
    const total = creditCount(premium.entitlement);
    const remaining = creditCount(premium.quota_remaining ?? premium.remaining);
    if (total === null || remaining === null) return null;
    return { total, remaining, used: Math.max(0, total - remaining) };
  })();

  const primary: UsageWindow | null = (() => {
    if (!premium) return null;
    let usedPercent: number | null = null;
    if (typeof premium.percent_remaining === "number") usedPercent = 100 - premium.percent_remaining;
    else if (usageCounts && usageCounts.total > 0) usedPercent = (usageCounts.used / usageCounts.total) * 100;
    if (usedPercent === null) return null;
    return {
      id: "monthly",
      label: "Monthly",
      usedPercent: clampPercent(usedPercent),
      windowMinutes: null,
      resetsAt: resetAt
    };
  })();

  const quota = usageCounts
    ? usageCounts.remaining
    : premium
      ? creditCount(premium.quota_remaining ?? premium.remaining)
      : token.quotaLimit ?? null;
  const unlimited = premium?.unlimited === true;
  const credits =
    quota !== null || unlimited || creditsUsed !== null
      ? {
          hasCredits: unlimited || (quota ?? 0) > 0,
          unlimited,
          balance: creditsUsed !== null || quota === null ? null : String(quota),
          label: tokenBasedBilling || creditsUsed !== null ? "GitHub AI Credits" : "Premium Requests",
          overagePermitted: premium?.overage_permitted === true,
          total: creditsUsed === null ? (usageCounts?.total ?? null) : null,
          used: creditsUsed ?? usageCounts?.used ?? null,
          remaining: creditsUsed === null ? (usageCounts?.remaining ?? quota) : null
        }
      : null;

  return {
    windows: primary ? [primary] : [],
    credits,
    planType: resolvedPlanType
  };
}

async function fetchOpencodeGo(entry: OAuthEntry): Promise<ProviderUsageResult> {
  const displayName = "OpenCode Go";
  if (!entry.access) {
    return errorResult("opencode-go", displayName, "missing_oauth", `${displayName} is not authenticated. Run: opencode auth login`);
  }
  const response = await fetchJson("https://opencode.ai/zen/go/v1/usage", {
    Authorization: `Bearer ${entry.access}`,
    Accept: "application/json"
  });
  if (!response) return errorResult("opencode-go", displayName, "fetch_failed", `${displayName} usage request failed (network)`, true);
  if (response.status === 401 || response.status === 403) {
    return errorResult("opencode-go", displayName, "reauth_required", `${displayName} subscription could not be verified.`);
  }
  if (!response.ok) return errorResult("opencode-go", displayName, "fetch_failed", `${displayName} usage request failed (${response.status})`, true);

  const usage = asRecord(asRecord(response.body)?.usage);
  if (!usage) return errorResult("opencode-go", displayName, "fetch_failed", `${displayName} usage request failed (empty response)`, true);
  const windows = [
    goUsageWindow("rolling", "Rolling", usage.rolling),
    goUsageWindow("weekly", "Weekly", usage.weekly),
    goUsageWindow("monthly", "Monthly", usage.monthly)
  ].filter((window): window is UsageWindow => window !== null);
  return {
    provider: "opencode-go",
    displayName,
    status: "ok",
    snapshot: {
      windows,
      credits: null,
      planType: "go",
      updatedAt: Date.now()
    }
  };
}

function goUsageWindow(id: string, label: string, raw: unknown): UsageWindow | null {
  const record = asRecord(raw);
  if (!record) return null;
  const percent = Number(record.percent ?? record.usagePercent);
  if (!Number.isFinite(percent)) return null;
  const resetsAt = typeof record.resetsAt === "string" ? parseResetsAt(record.resetsAt) : Number(record.resetsAt);
  return { id, label, usedPercent: Math.min(100, Math.max(0, percent)), windowMinutes: null, resetsAt: Number.isFinite(resetsAt) ? resetsAt : null };
}

interface CommandCodeLimit {
  used?: unknown;
  cap?: unknown;
  resetAt?: unknown;
}

function commandCodeLimitWindow(id: string, label: string, raw: CommandCodeLimit | null, windowMinutes: number): UsageWindow | null {
  if (!raw) return null;
  const used = Number(raw.used);
  const cap = Number(raw.cap);
  if (!Number.isFinite(used) || !Number.isFinite(cap) || cap <= 0) return null;
  const resetRaw = Number(raw.resetAt);
  const resetsAt = Number.isFinite(resetRaw) && resetRaw > 0
    ? (resetRaw >= 1_000_000_000_000 ? Math.floor(resetRaw / 1000) : Math.floor(resetRaw))
    : null;
  return {
    id,
    label,
    usedPercent: clampPercent((used / cap) * 100),
    windowMinutes,
    resetsAt
  };
}

interface CommandCodeCredits {
  monthlyCredits?: unknown;
  purchasedCredits?: unknown;
  freeCredits?: unknown;
}

function commandCodeCredits(raw: CommandCodeCredits | null): ProviderUsageCredits | null {
  if (!raw) return null;
  const monthly = creditCount(raw.monthlyCredits);
  const purchased = creditCount(raw.purchasedCredits);
  const free = creditCount(raw.freeCredits);
  const total = [monthly, purchased, free].filter((value): value is number => value !== null).reduce((sum, value) => sum + value, 0);
  if (total <= 0) return null;
  return {
    hasCredits: true,
    unlimited: false,
    balance: String(total),
    label: "Command Code Credits",
    total,
    used: null,
    remaining: total
  };
}

async function fetchCommandCode(entry: OAuthEntry): Promise<ProviderUsageResult> {
  const displayName = "Command Code";
  if (!entry.access) {
    return errorResult("command-code", displayName, "missing_oauth", `${displayName} is not authenticated. Set COMMANDCODE_API_KEY or run: opencode auth login`);
  }

  const headers: Record<string, string> = {
    Authorization: `Bearer ${entry.access}`,
    Accept: "application/json"
  };
  const identity = await fetchJson("https://api.commandcode.ai/alpha/whoami", headers);
  if (!identity) return errorResult("command-code", displayName, "fetch_failed", `${displayName} usage request failed (network)`, true);
  if (identity.status === 401 || identity.status === 403) {
    return errorResult("command-code", displayName, "reauth_required", `${displayName} authentication failed. Run: opencode auth login`);
  }
  if (!identity.ok) return errorResult("command-code", displayName, "fetch_failed", `${displayName} usage request failed (${identity.status})`, true);

  const whoami = asRecord(identity.body);
  const org = asRecord(whoami?.org);
  const orgId = typeof org?.id === "string" ? org.id.trim() : "";
  const creditsPath = orgId
    ? `/alpha/billing/credits?orgId=${encodeURIComponent(orgId)}`
    : "/alpha/billing/credits";
  const response = await fetchJson(`https://api.commandcode.ai${creditsPath}`, headers);
  if (!response) return errorResult("command-code", displayName, "fetch_failed", `${displayName} usage request failed (network)`, true);
  if (response.status === 401 || response.status === 403) {
    return errorResult("command-code", displayName, "reauth_required", `${displayName} authentication failed. Run: opencode auth login`);
  }
  if (!response.ok) return errorResult("command-code", displayName, "fetch_failed", `${displayName} usage request failed (${response.status})`, true);

  const body = asRecord(response.body);
  if (!body) return errorResult("command-code", displayName, "fetch_failed", `${displayName} usage request failed (empty response)`, true);

  const limits = asRecord(body.windowLimits);
  const fiveHour = commandCodeLimitWindow("5h", "5h", (limits?.fiveHour ?? null) as CommandCodeLimit | null, 5 * 60);
  const weekly = commandCodeLimitWindow("weekly", "Weekly", (limits?.weekly ?? null) as CommandCodeLimit | null, 7 * 24 * 60);
  const credits = commandCodeCredits(asRecord(body.credits) as CommandCodeCredits | null);
  const windows = [fiveHour, weekly].filter((window): window is UsageWindow => window !== null);
  if (windows.length === 0 && !credits) {
    return errorResult("command-code", displayName, "fetch_failed", `${displayName} usage data could not be parsed`, true);
  }
  return {
    provider: "command-code",
    displayName,
    status: "ok",
    snapshot: { windows, credits, planType: null, updatedAt: Date.now() }
  };
}

interface ProviderSpec {
  fetch: (entry: OAuthEntry) => Promise<ProviderUsageResult>;
  envKeys?: string[];
}

const PROVIDERS: Record<string, ProviderSpec> = {
  openai: { fetch: fetchChatgpt },
  anthropic: { fetch: fetchClaude },
  "github-copilot": { fetch: fetchCopilot },
  "opencode-go": { fetch: fetchOpencodeGo },
  "command-code": { fetch: fetchCommandCode, envKeys: ["COMMANDCODE_API_KEY", "COMMAND_CODE_API_KEY"] }
};

function envEntryFor(spec: ProviderSpec): OAuthEntry | null {
  for (const name of spec.envKeys ?? []) {
    const value = process.env[name];
    if (value) return { type: "oauth", access: value };
  }
  return null;
}

const SNAPSHOT_MAX_AGE_MS = 15 * 60 * 1000;

export function velaUsageSnapshotCandidates(): string[] {
  const override = process.env.VELA_USAGE_SNAPSHOT ?? process.env.VELA_USAGE_SNAPSHOT;
  if (override) return [override];
  const data = process.env.XDG_DATA_HOME ?? path.join(homedir(), ".local", "share");
  return [
    path.join(data, "opencode", "vela-usage.json"),
    path.join(data, "opencode", "vela-usage.json"),
    path.join(homedir(), "Library", "Application Support", "ai.opencode.desktop", "opencode", "vela-usage.json")
  ];
}

export async function readVelaUsageSnapshot(
  candidates: string[] = velaUsageSnapshotCandidates(),
  now = Date.now()
): Promise<ProviderUsageResult[]> {
  for (const candidate of candidates) {
    let raw: string;
    try {
      raw = await fsp.readFile(candidate, "utf8");
    } catch {
      continue;
    }
    try {
      const parsed = JSON.parse(raw) as { generatedAt?: unknown; results?: unknown };
      const generatedAt = Number(parsed.generatedAt);
      if (!Number.isFinite(generatedAt) || now - generatedAt > SNAPSHOT_MAX_AGE_MS) continue;
      const results = Array.isArray(parsed.results) ? parsed.results : [];
      const usable = results.filter((entry): entry is ProviderUsageResult =>
        Boolean(entry && typeof entry === "object" && typeof (entry as ProviderUsageResult).provider === "string")
      );
      if (usable.length > 0) return usable;
    } catch {
      continue;
    }
  }
  return [];
}

export async function fetchProviderUsage(): Promise<ProviderUsageResult[]> {
  const snapshotted = await readVelaUsageSnapshot();
  const snapshotByProvider = new Map(snapshotted.map((result) => [result.provider, result]));
  const auth = await readOAuthEntries();
  const results: ProviderUsageResult[] = [];
  for (const [provider, spec] of Object.entries(PROVIDERS)) {
    const entry = auth[provider] ?? envEntryFor(spec);
    if (!entry) {
      const fallback = snapshotByProvider.get(provider);
      if (fallback) results.push(fallback);
      snapshotByProvider.delete(provider);
      continue;
    }
    const live = await spec.fetch(entry);
    results.push(live.status === "ok" ? live : snapshotByProvider.get(provider) ?? live);
    snapshotByProvider.delete(provider);
  }
  results.push(...snapshotByProvider.values());
  return results;
}
