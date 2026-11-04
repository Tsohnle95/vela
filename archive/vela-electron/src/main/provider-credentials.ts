import { promises as fsp } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { sqliteQuery } from "./sqlite";

export interface OAuthEntry {
  type?: string;
  access?: string;
  refresh?: string;
  expires?: number;
  accountId?: string;
  enterpriseUrl?: string;
}

interface DbAccountRow {
  url?: unknown;
  access_token?: unknown;
  refresh_token?: unknown;
  token_expiry?: unknown;
}

interface DbCredentialRow {
  integration_id?: unknown;
  value?: unknown;
}

interface CredentialValue {
  type?: string;
  key?: string;
  access?: string;
  refresh?: string;
  expires?: number;
}

function parseCredentialValue(raw: string): Pick<OAuthEntry, "access" | "refresh" | "expires"> | null {
  let value: CredentialValue;
  try {
    value = JSON.parse(raw) as CredentialValue;
  } catch {
    return null;
  }
  if (!value || typeof value !== "object") return null;
  if (value.type === "key" && typeof value.key === "string" && value.key) {
    return { access: value.key };
  }
  if (value.type === "oauth") {
    return {
      access: typeof value.access === "string" ? value.access : undefined,
      refresh: typeof value.refresh === "string" ? value.refresh : undefined,
      expires: typeof value.expires === "number" ? value.expires : undefined
    };
  }
  return null;
}

function credentialFileCandidates(filename: string): string[] {
  const data = process.env.XDG_DATA_HOME ?? path.join(homedir(), ".local", "share");
  const candidates = [
    path.join(data, "opencode", filename),
    path.join(homedir(), ".config", "opencode", filename)
  ];
  if (process.platform === "darwin") {
    candidates.push(
      path.join(homedir(), "Library", "Application Support", "ai.opencode.desktop", "opencode", filename)
    );
  }
  return candidates;
}

async function readOAuthEntriesFromJson(): Promise<Record<string, OAuthEntry>> {
  for (const file of credentialFileCandidates("auth.json")) {
    try {
      const raw = JSON.parse(await fsp.readFile(file, "utf8")) as Record<string, unknown>;
      const out: Record<string, OAuthEntry> = {};
      for (const [key, value] of Object.entries(raw)) {
        if (value && typeof value === "object" && (value as OAuthEntry).type === "oauth") {
          out[key] = value as OAuthEntry;
        }
      }
      if (Object.keys(out).length > 0) return out;
    } catch {
      /* try the next location */
    }
  }
  return {};
}

function providerKeyForAccountUrl(url: string | undefined): string | null {
  const normalized = (url ?? "").toLowerCase();
  if (normalized.includes("chatgpt") || normalized.includes("openai")) return "openai";
  if (normalized.includes("anthropic") || normalized.includes("claude")) return "anthropic";
  if (normalized.includes("github")) return "github-copilot";
  return null;
}

function providerKeyForCredentialId(id: string): string | null {
  if (id === "openai") return "openai";
  if (id === "opencode-go") return "opencode-go";
  // Command Code ships as a community plugin; plugin releases have registered
  // both `commandcode` and `command-code` as the auth provider id.
  if (id === "command-code" || id === "commandcode") return "command-code";
  return null;
}

export function entriesFromDbRows(accounts: DbAccountRow[], credentials: DbCredentialRow[]): Record<string, OAuthEntry> {
  const out: Record<string, OAuthEntry> = {};
  for (const row of accounts) {
    const key = providerKeyForAccountUrl(row.url != null ? String(row.url) : undefined);
    if (!key) continue;
    const access = row.access_token != null ? String(row.access_token) : undefined;
    if (!access) continue;
    out[key] = {
      type: "oauth",
      access,
      refresh: row.refresh_token != null ? String(row.refresh_token) : undefined,
      expires: typeof row.token_expiry === "number" ? row.token_expiry : undefined
    };
  }
  for (const row of credentials) {
    const key = providerKeyForCredentialId(String(row.integration_id ?? ""));
    if (!key) continue;
    const raw = row.value != null ? String(row.value) : undefined;
    if (!raw) continue;
    const parsed = parseCredentialValue(raw) ?? { access: raw };
    if (!parsed.access) continue;
    out[key] = { type: "oauth", access: parsed.access, refresh: parsed.refresh, expires: parsed.expires };
  }
  return out;
}

async function readOAuthEntriesFromDb(): Promise<Record<string, OAuthEntry>> {
  const out: Record<string, OAuthEntry> = {};
  for (const db of credentialFileCandidates("opencode.db")) {
    const accounts = await sqliteQuery(db, "SELECT url, access_token, refresh_token, token_expiry FROM account;");
    const credentials = await sqliteQuery(db, "SELECT integration_id, value FROM credential WHERE active IS NULL OR active = 1;");
    Object.assign(out, entriesFromDbRows(accounts as DbAccountRow[], credentials as DbCredentialRow[]));
  }
  return out;
}

export async function readOAuthEntries(): Promise<Record<string, OAuthEntry>> {
  const json = await readOAuthEntriesFromJson();
  const db = await readOAuthEntriesFromDb();
  return { ...json, ...db };
}
