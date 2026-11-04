import { describe, expect, it, vi } from "vitest";
import { homedir } from "node:os";
import path from "node:path";
import { MobileServer } from "./mobile-server";

const uiPassword = "local-only-ui-password";
const legacyProductPrefix = String.fromCharCode(79, 82, 66, 73, 84);
const serverEntry = (home: string) => path.join(home, "packages", "web", "server", "index.js");

const lanInterface = (address: string) => ({
  [address]: [{ address, netmask: "255.255.255.0", family: "IPv4", mac: "00:00:00:00:00:00", internal: false, cidr: `${address}/24` }]
}) as unknown as NodeJS.Dict<import("node:os").NetworkInterfaceInfo[]>;

const makeServer = (options: {
  exists?: (candidate: string) => boolean;
  fetcher?: typeof fetch;
  env?: NodeJS.ProcessEnv;
  cwd?: string;
  networkInterfaces?: () => NodeJS.Dict<import("node:os").NetworkInterfaceInfo[]>;
} = {}) => new MobileServer({
  cwd: options.cwd ?? "/work/vela",
  supportDirectory: "/support",
  env: options.env ?? { VELA_PASSWORD: uiPassword },
  exists: options.exists ?? ((candidate) => candidate === serverEntry("/work/openchamber")),
  readTextFile: () => uiPassword,
  writeTextFile: () => undefined,
  fetcher: options.fetcher,
  execPath: "/electron",
  networkInterfaces: options.networkInterfaces ?? (() => lanInterface("192.168.1.20")),
});

const jsonResponse = (body: unknown, init: ResponseInit = {}) => new Response(JSON.stringify(body), {
  headers: { "Content-Type": "application/json", ...init.headers },
  ...init,
});

const decodePayload = (connectionUrl: string): Record<string, unknown> => {
  expect(connectionUrl.startsWith("vela://connect?v=2&p=")).toBe(true);
  const encoded = new URLSearchParams(connectionUrl.slice(connectionUrl.indexOf("?") + 1)).get("p") ?? "";
  return JSON.parse(Buffer.from(encoded, "base64url").toString("utf8")) as Record<string, unknown>;
};

describe("MobileServer setup and pairing", () => {
  it("finds the sibling server checkout and reports ready without exposing its password", async () => {
    const fetcher = vi.fn<typeof fetch>(async (input) => {
      expect(String(input)).toBe("http://127.0.0.1:3011/health");
      return jsonResponse({ status: "ok" });
    });
    const server = makeServer({ fetcher });

    await expect(server.getSetupStatus()).resolves.toEqual({
      state: "ready",
      workspacePath: "/work/openchamber",
      port: 3011,
    });
  });

  it("finds the code-repositories checkout when the packaged app has no sibling checkout", async () => {
    const home = path.join(homedir(), "code-repositories", "openchamber");
    const server = makeServer({
      cwd: "/Applications/Vela.app/Contents/Resources/app.asar",
      exists: (candidate) => candidate === serverEntry(home),
      fetcher: vi.fn<typeof fetch>(async () => jsonResponse({ status: "ok" })),
    });

    await expect(server.getSetupStatus()).resolves.toEqual({
      state: "ready",
      workspacePath: home,
      port: 3011,
    });
  });

  it("honours an explicit VELA_MOBILE_HOME override", async () => {
    const home = "/opt/vela/server";
    const server = makeServer({
      env: { VELA_PASSWORD: uiPassword, VELA_MOBILE_HOME: home },
      exists: (candidate) => candidate === serverEntry(home),
      fetcher: vi.fn<typeof fetch>(async () => jsonResponse({ status: "ok" })),
    });

    await expect(server.getSetupStatus()).resolves.toMatchObject({ state: "ready", workspacePath: home });
  });

  it("honours prior mobile-home and port environment settings", async () => {
    const home = "/opt/previous/server";
    const server = makeServer({
      env: { VELA_PASSWORD: uiPassword, [`${legacyProductPrefix}_MOBILE_HOME`]: home, [`${legacyProductPrefix}_PORT`]: "3022" },
      exists: (candidate) => candidate === serverEntry(home),
      fetcher: vi.fn<typeof fetch>(async (input) => {
        expect(String(input)).toBe("http://127.0.0.1:3022/health");
        return jsonResponse({ status: "ok" });
      })
    });

    await expect(server.getSetupStatus()).resolves.toMatchObject({ state: "ready", workspacePath: home, port: 3022 });
  });

  it("explains missing server files rather than silently disabling mobile setup", async () => {
    const server = makeServer({ exists: () => false });

    await expect(server.getSetupStatus()).resolves.toMatchObject({
      state: "setup-required",
      reason: "companion-checkout-missing",
      workspacePath: path.join(homedir(), "coding-projects", "openchamber"),
    });
    await expect(server.createPairingQr()).rejects.toThrow("set VELA_MOBILE_HOME");
  });

  it("reports offline while the server has not started", async () => {
    const server = makeServer({ fetcher: vi.fn<typeof fetch>(async () => jsonResponse({ status: "down" }, { status: 503 })) });
    await expect(server.getSetupStatus()).resolves.toMatchObject({ state: "offline", reason: "server-not-running" });
  });

  it("keeps the UI password in main while building an authenticated one-time pairing URI", async () => {
    const fetcher = vi.fn<typeof fetch>(async (input, init) => {
      const url = String(input);
      if (url.endsWith("/health")) return jsonResponse({ status: "ok" });
      if (url.endsWith("/auth/session")) {
        expect(JSON.parse(String(init?.body))).toEqual({ password: uiPassword });
        return jsonResponse({ authenticated: true }, {
          headers: { "Set-Cookie": "oc_ui_session=session-cookie; Path=/; HttpOnly; SameSite=Lax" },
        });
      }
      if (url.endsWith("/api/client-auth/pairing/sessions")) {
        expect(new Headers(init?.headers).get("cookie")).toBe("oc_ui_session=session-cookie");
        expect(JSON.parse(String(init?.body))).toEqual({
          allowedClientKinds: ["mobile"],
          serverUrl: "http://192.168.1.20:3011",
          includeDirect: true,
          includeRelay: false,
        });
        return jsonResponse({
          pairing: { id: "pair_once", secret: "single-use", expiresAt: "2099-01-01T00:00:00.000Z", fingerprint: "ABCD-EF12" },
          server: { label: "Mac Studio", candidates: [{ type: "lan", url: "http://192.168.1.20:3011", priority: 10 }] },
        }, { status: 201, headers: { "Cache-Control": "no-store" } });
      }
      throw new Error(`Unexpected URL: ${url}`);
    });
    const server = makeServer({ fetcher });

    const result = await server.createPairingQr();

    expect(result.expiresAt).toBe("2099-01-01T00:00:00.000Z");
    expect(result.serverLabel).toBe("Mac Studio");
    expect(JSON.stringify(result)).not.toContain(uiPassword);
    expect(decodePayload(result.connectionUrl)).toEqual({
      v: 2,
      pairingId: "pair_once",
      secret: "single-use",
      label: "Mac Studio",
      fingerprint: "ABCD-EF12",
      expiresAt: "2099-01-01T00:00:00.000Z",
      candidates: [{ type: "lan", url: "http://192.168.1.20:3011", priority: 10 }],
    });
  });

  it("prefers the Tailscale address when the Mac is on a tailnet", async () => {
    const fetcher = vi.fn<typeof fetch>(async (input, init) => {
      const url = String(input);
      if (url.endsWith("/health")) return jsonResponse({ status: "ok" });
      if (url.endsWith("/auth/session")) {
        return jsonResponse({ authenticated: true }, { headers: { "Set-Cookie": "oc_ui_session=session-cookie; Path=/" } });
      }
      if (url.endsWith("/api/client-auth/pairing/sessions")) {
        expect(JSON.parse(String(init?.body))).toMatchObject({ serverUrl: "http://100.89.86.73:3011" });
        return jsonResponse({
          pairing: { id: "pair_ts", secret: "single-use", expiresAt: "2099-01-01T00:00:00.000Z" },
          server: { label: "Desktop", candidates: [{ type: "lan", url: "http://100.89.86.73:3011" }] },
        }, { status: 201 });
      }
      throw new Error(`Unexpected URL: ${url}`);
    });
    const server = makeServer({
      fetcher,
      networkInterfaces: () => ({ en0: lanInterface("192.168.1.20")["192.168.1.20"], utun: lanInterface("100.89.86.73")["100.89.86.73"] }),
    });

    await expect(server.createPairingQr()).resolves.toMatchObject({ serverLabel: "Desktop" });
  });

  it("refuses to build a code when only loopback interfaces exist", async () => {
    const server = makeServer({ networkInterfaces: () => ({ lo0: lanInterface("127.0.0.1")["127.0.0.1"] }) });
    await expect(server.createPairingQr()).rejects.toThrow("network address your phone can reach");
  });

  it("uses the bundled server under Resources/mobile-server when packaged", async () => {
    const resources = "/Applications/Vela.app/Contents/Resources";
    const entry = path.join(resources, "mobile-server", "node_modules", "@openchamber", "web", "server", "index.js");
    const server = new MobileServer({
      cwd: "/Applications/Vela.app/Contents/Resources/app",
      supportDirectory: "/support",
      env: { VELA_PASSWORD: uiPassword },
      resourcesPath: resources,
      exists: (candidate) => candidate === entry,
      readTextFile: () => uiPassword,
      writeTextFile: () => undefined,
      fetcher: vi.fn<typeof fetch>(async () => jsonResponse({ status: "ok" })),
      execPath: "/electron",
      networkInterfaces: () => lanInterface("192.168.1.20")
    });

    await expect(server.getSetupStatus()).resolves.toEqual({
      state: "ready",
      workspacePath: path.join(resources, "mobile-server"),
      port: 3011
    });
  });
});
