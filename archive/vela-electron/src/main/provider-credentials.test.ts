import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { promises as fsp } from "node:fs";
import path from "node:path";
import { readOAuthEntries, entriesFromDbRows } from "./provider-credentials";
import { sqliteQuery } from "./sqlite";

vi.mock("node:fs", () => {
  const promises = { readFile: vi.fn() };
  return { promises, default: { promises } };
});
vi.mock("node:os", () => {
  const homedir = () => "/fixture-home";
  return { homedir, default: { homedir } };
});
vi.mock("./sqlite", () => ({ sqliteQuery: vi.fn() }));

const readFile = vi.mocked(fsp.readFile);
const query = vi.mocked(sqliteQuery);
const dataRoot = path.join("/fixture-data", "opencode");
const configRoot = path.join("/fixture-home", ".config", "opencode");
const roots = [dataRoot, configRoot, ...(process.platform === "darwin"
  ? [path.join("/fixture-home", "Library", "Application Support", "ai.opencode.desktop", "opencode")] : [])];

beforeEach(() => {
  vi.stubEnv("XDG_DATA_HOME", "/fixture-data");
  readFile.mockReset().mockRejectedValue(new Error("Missing fixture"));
  query.mockReset().mockResolvedValue([]);
});
afterEach(() => vi.unstubAllEnvs());

describe("provider credential discovery", () => {
  it("tries invalid and empty JSON before selecting the first nonempty OAuth map", async () => {
    readFile.mockImplementation(async (file) => {
      if (String(file) === path.join(dataRoot, "auth.json")) return "{broken";
      return JSON.stringify({ openai: { type: "oauth", access: "fixture-json" }, excluded: { type: "key", key: "fixture-key" } });
    });
    expect(await readOAuthEntries()).toEqual({ openai: { type: "oauth", access: "fixture-json" } });
    expect(readFile.mock.calls.map(([file]) => String(file))).toEqual(roots.slice(0, 2).map((root) => path.join(root, "auth.json")));
    readFile.mockClear().mockResolvedValue("{}");
    expect(await readOAuthEntries()).toEqual({});
    expect(readFile.mock.calls.map(([file]) => String(file))).toEqual(roots.map((root) => path.join(root, "auth.json")));
  });

  it("keeps account/credential query ordering, later database precedence and database-over-JSON", async () => {
    readFile.mockResolvedValue(JSON.stringify({ openai: { type: "oauth", access: "json" }, anthropic: { type: "oauth", access: "json-only" } }));
    const calls: string[] = [];
    query.mockImplementation(async (db, sql) => {
      const index = roots.findIndex((root) => path.join(root, "opencode.db") === db);
      const account = sql.includes("FROM account");
      calls.push(`${index}:${account ? "account" : "credential"}`);
      if (account) return [{ url: "https://chatgpt.com", access_token: `account-${index}` }];
      expect(sql).toContain("WHERE active IS NULL OR active = 1;");
      return [{ integration_id: "openai", value: JSON.stringify({ type: "key", key: `credential-${index}` }) }];
    });
    const entries = await readOAuthEntries();
    expect(entries.openai.access).toBe(`credential-${roots.length - 1}`);
    expect(entries.anthropic.access).toBe("json-only");
    expect(calls).toEqual(roots.flatMap((_root, index) => [`${index}:account`, `${index}:credential`]));
  });

  it("preserves malformed JSON OAuth values rather than silently changing provider failures", async () => {
    readFile.mockResolvedValue(JSON.stringify({ openai: { type: "oauth", access: 42 } }));
    expect(await readOAuthEntries()).toEqual({ openai: { type: "oauth", access: 42 } });
  });

  it("preserves typed credentials, plugin aliases and legacy plain-string fallback", () => {
    expect(entriesFromDbRows([{ url: "https://chatgpt.com", access_token: "account" }], [
      { integration_id: "openai", value: JSON.stringify({ type: "oauth", access: "typed", refresh: "refresh", expires: 0 }) },
      { integration_id: "commandcode", value: "legacy" },
      { integration_id: "opencode-go", value: JSON.stringify({ type: "key", key: "go" }) },
      { integration_id: "unknown", value: "ignored" }
    ])).toEqual({
      openai: { type: "oauth", access: "typed", refresh: "refresh", expires: 0 },
      "command-code": { type: "oauth", access: "legacy", refresh: undefined, expires: undefined },
      "opencode-go": { type: "oauth", access: "go", refresh: undefined, expires: undefined }
    });
  });
});
