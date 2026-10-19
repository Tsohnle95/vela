import { promises as fs } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { Plugin } from "@opencode/plugin"

type UsageWindow = {
  id: string
  label: string
  usedPercent: number
  windowMinutes: number | null
  resetsAt: number | null
}

type UsageResult = {
  provider: string
  displayName: string
  status: "ok" | "unavailable"
  snapshot: { windows: UsageWindow[]; credits: null; planType: string | null; updatedAt: number } | null
  error: { code: string; message: string; retryable: boolean } | null
}

const REFRESH_INTERVAL_MS = 5 * 60 * 1000

function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" ? value as Record<string, unknown> : null
}

function accountID(token: string): string | null {
  try {
    const payload = JSON.parse(Buffer.from(token.split(".")[1], "base64url").toString("utf8")) as Record<string, unknown>
    const auth = record(payload["https://api.openai.com/auth"])
    const value = auth?.chatgpt_account_id ?? auth?.account_id ?? payload.account_id
    return typeof value === "string" && value ? value : null
  } catch {
    return null
  }
}

function usageWindow(raw: unknown): UsageWindow | null {
  const value = record(raw)
  const seconds = Number(value?.limit_window_seconds)
  const used = Number(value?.used_percent)
  if (!Number.isFinite(seconds) || seconds <= 0 || !Number.isFinite(used)) return null
  const minutes = Math.round(seconds / 60)
  const weekly = Math.abs(minutes - 7 * 24 * 60) <= 7 * 24 * 60 * 0.1
  const fiveHour = Math.abs(minutes - 5 * 60) <= 5 * 60 * 0.1
  return {
    id: fiveHour ? "5h" : weekly ? "weekly" : `w${minutes}`,
    label: fiveHour ? "5h" : weekly ? "Weekly" : `${Math.max(1, Math.round(minutes / 60))}h`,
    usedPercent: Math.min(100, Math.max(0, used)),
    windowMinutes: minutes,
    resetsAt: Number(value?.reset_at) > 0 ? Math.floor(Number(value?.reset_at)) : null
  }
}

function snapshotPath(): string {
  const override = process.env.VELA_USAGE_SNAPSHOT ?? process.env.VELA_USAGE_SNAPSHOT
  if (override) return override
  const data = process.env.XDG_DATA_HOME ?? path.join(homedir(), ".local", "share")
  return path.join(data, "opencode", "vela-usage.json")
}

async function resolvedToken(ctx: Plugin.Context, integrationID: string): Promise<string | null> {
  try {
    const connection = await ctx.integration.connection.active(integrationID)
    if (!connection) return null
    const credential = await ctx.integration.connection.resolve(connection)
    return credential && typeof credential.access === "string" ? credential.access : typeof credential?.key === "string" ? credential.key : null
  } catch {
    return null
  }
}

async function chatgptUsage(token: string): Promise<UsageResult> {
  const base: UsageResult = { provider: "openai", displayName: "OpenAI ChatGPT", status: "ok", snapshot: null, error: null }
  try {
    const account = accountID(token)
    const response = await fetch("https://chatgpt.com/backend-api/wham/usage", {
      headers: {
        authorization: `Bearer ${token}`,
        ...(account ? { "chatgpt-account-id": account } : {}),
        accept: "application/json"
      },
      signal: AbortSignal.timeout(10_000)
    })
    if (!response.ok) return { ...base, status: "unavailable", error: { code: "http", message: `HTTP ${response.status}`, retryable: response.status >= 500 } }
    const raw = await response.json() as Record<string, unknown>
    const rateLimit = record(raw.rate_limit)
    const windows = [usageWindow(rateLimit?.primary_window), usageWindow(rateLimit?.secondary_window)]
      .filter((window): window is UsageWindow => window !== null)
      .sort((a, b) => (a.windowMinutes ?? 0) - (b.windowMinutes ?? 0))
    return { ...base, snapshot: { windows, credits: null, planType: typeof raw.plan_type === "string" ? raw.plan_type : null, updatedAt: Date.now() } }
  } catch (cause) {
    return { ...base, status: "unavailable", error: { code: "network", message: cause instanceof Error ? cause.message : String(cause), retryable: true } }
  }
}

async function claudeUsage(token: string): Promise<UsageResult> {
  const base: UsageResult = { provider: "anthropic", displayName: "Anthropic Claude", status: "ok", snapshot: null, error: null }
  try {
    const response = await fetch("https://api.anthropic.com/api/oauth/usage", {
      headers: { authorization: `Bearer ${token}`, "anthropic-beta": "oauth-2025-04-20", accept: "application/json" },
      signal: AbortSignal.timeout(10_000)
    })
    if (!response.ok) return { ...base, status: "unavailable", error: { code: "http", message: `HTTP ${response.status}`, retryable: response.status >= 500 } }
    const raw = await response.json() as Record<string, unknown>
    const windows: UsageWindow[] = []
    const read = (key: string, id: string, label: string, minutes: number) => {
      const value = record(raw[key])
      const used = Number(value?.utilization)
      if (!Number.isFinite(used)) return
      const resetMs = typeof value?.resets_at === "string" ? new Date(value.resets_at).getTime() : Number.NaN
      windows.push({
        id,
        label,
        usedPercent: Math.min(100, Math.max(0, used)),
        windowMinutes: minutes,
        resetsAt: Number.isFinite(resetMs) ? Math.floor(resetMs / 1000) : null
      })
    }
    read("five_hour", "5h", "5h", 5 * 60)
    read("seven_day", "weekly", "Weekly", 24 * 60 * 7)
    return { ...base, snapshot: { windows, credits: null, planType: null, updatedAt: Date.now() } }
  } catch (cause) {
    return { ...base, status: "unavailable", error: { code: "network", message: cause instanceof Error ? cause.message : String(cause), retryable: true } }
  }
}

export default Plugin.define({
  id: "vela-usage",
  async setup(ctx) {
    let running = false
    const collect = async (): Promise<void> => {
      if (running) return
      running = true
      try {
        const results: UsageResult[] = []
        const chatgpt = await resolvedToken(ctx, "openai")
        if (chatgpt) results.push(await chatgptUsage(chatgpt))
        const anthropic = await resolvedToken(ctx, "anthropic")
        if (anthropic) results.push(await claudeUsage(anthropic))
        if (results.length > 0) {
          await fs.mkdir(path.dirname(snapshotPath()), { recursive: true })
          await fs.writeFile(snapshotPath(), JSON.stringify({ generatedAt: Date.now(), results }))
        }
      } catch {
      } finally {
        running = false
      }
    }
    void collect()
    const timer = setInterval(() => void collect(), REFRESH_INTERVAL_MS)
    return () => clearInterval(timer)
  }
})
