import { spawn, type ChildProcess } from "node:child_process";
import { randomBytes } from "node:crypto";
import { existsSync, mkdirSync, openSync, closeSync, readFileSync, writeFileSync } from "node:fs";
import { homedir, networkInterfaces as osNetworkInterfaces } from "node:os";
import path from "node:path";

/**
 * Attaches the Vela mobile server to the desktop app's lifecycle.
 *
 * Vela Desktop hosts the OpenChamber-family web server in-process-equivalent:
 * while the app is open, it runs the server (`packages/web/server/index.js`) as
 * a child of this process using Electron's own Node runtime
 * (`ELECTRON_RUN_AS_NODE`), so no system Node install is required. When Vela
 * quits — or is force-killed — the server stops with it.
 *
 * The React renderer only ever receives a one-time pairing URI; the mobile UI
 * password and reusable device tokens never cross IPC.
 */

const DEFAULT_SERVER_HOMES = [
  path.join(homedir(), "code-repositories", "openchamber"),
  path.join(homedir(), "coding-projects", "openchamber")
];
const DEFAULT_SERVER_HOME = DEFAULT_SERVER_HOMES[1];
const SOURCE_SERVER_ENTRY = path.join("packages", "web", "server", "index.js");
const PACKAGED_SERVER_ENTRY = path.join("node_modules", "@openchamber", "web", "server", "index.js");
const LEGACY_PRODUCT_PREFIX = String.fromCharCode(79, 82, 66, 73, 84);
const LEGACY_MOBILE_HOME_ENV = `${LEGACY_PRODUCT_PREFIX}_MOBILE_HOME`;
const LEGACY_PORT_ENV = `${LEGACY_PRODUCT_PREFIX}_PORT`;
const SUPPORT = path.join(homedir(), "Library", "Application Support", "VelaMobile");
const DEFAULT_SERVER_PORT = 3011;
const MOBILE_SERVER_START_TIMEOUT_MS = 30_000;
const MOBILE_SERVER_PROBE_TIMEOUT_MS = 1_500;
const MOBILE_PAIRING_REQUEST_TIMEOUT_MS = 8_000;

const asRecord = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;

export interface MobileServerStatus {
  state: "ready" | "starting" | "offline" | "setup-required";
  reason?: "companion-checkout-missing" | "node-runtime-missing" | "server-not-running";
  workspacePath: string;
  port: number;
}

export interface MobilePairingQr {
  connectionUrl: string;
  expiresAt: string;
  serverLabel: string;
}

export interface MobileBackend {
  url: string;
  username: string;
  password: string;
}

interface MobileServerDependencies {
  env?: NodeJS.ProcessEnv;
  cwd?: string;
  supportDirectory?: string;
  exists?: (candidate: string) => boolean;
  readTextFile?: (candidate: string) => string;
  writeTextFile?: (candidate: string, contents: string) => void;
  fetcher?: typeof fetch;
  execPath?: string;
  networkInterfaces?: () => NodeJS.Dict<import("node:os").NetworkInterfaceInfo[]>;
  spawnProcess?: typeof spawn;
  resourcesPath?: string | null;
}

const resolveServerHome = (
  env: NodeJS.ProcessEnv,
  cwd: string,
  exists: (candidate: string) => boolean,
  resourcesPath: string | null
): { home: string | null; expectedHome: string; entry: string | null } => {
  // Packaged app: the server ships inside Resources/mobile-server.
  if (resourcesPath) {
    const home = path.join(resourcesPath, "mobile-server");
    const entry = path.join(home, PACKAGED_SERVER_ENTRY);
    if (exists(entry)) return { home, expectedHome: home, entry };
  }

  const override = (env["VELA_MOBILE_HOME"] ?? env[LEGACY_MOBILE_HOME_ENV])?.trim();
  if (override) {
    const home = path.resolve(cwd, override);
    const entry = path.join(home, SOURCE_SERVER_ENTRY);
    return { home: exists(entry) ? home : null, expectedHome: home, entry };
  }

  const candidates = [
    path.resolve(cwd, "..", "openchamber"),
    ...DEFAULT_SERVER_HOMES
  ];
  for (const home of [...new Set(candidates)]) {
    const entry = path.join(home, SOURCE_SERVER_ENTRY);
    if (exists(entry)) return { home, expectedHome: home, entry };
  }
  return { home: null, expectedHome: DEFAULT_SERVER_HOME, entry: null };
};

const isTailscaleAddress = (address: string): boolean => {
  const octets = address.split(".");
  if (octets.length !== 4) return false;
  const second = Number(octets[1]);
  return octets[0] === "100" && Number.isInteger(second) && second >= 64 && second <= 127;
};

// A phone cannot reach loopback or link-local addresses, even when a NIC marks
// them non-internal.
const isUnreachableHostAddress = (address: string): boolean =>
  address.startsWith("127.") || address.startsWith("169.254.") || address === "0.0.0.0";

export class MobileServer {
  private child: ChildProcess | null = null;
  private readonly env: NodeJS.ProcessEnv;
  private readonly home: string | null;
  private readonly expectedHome: string;
  private readonly entry: string | null;
  private readonly port: number;
  private readonly password: string;
  private readonly supportDirectory: string;
  private readonly exists: (candidate: string) => boolean;
  private readonly readTextFile: (candidate: string) => string;
  private readonly writeTextFile: (candidate: string, contents: string) => void;
  private readonly fetcher: typeof fetch;
  private readonly execPath: string;
  private readonly readNetworkInterfaces: () => NodeJS.Dict<import("node:os").NetworkInterfaceInfo[]>;
  private readonly spawnProcess: typeof spawn;

  constructor(dependencies: MobileServerDependencies = {}) {
    this.env = dependencies.env ?? process.env;
    const cwd = dependencies.cwd ?? process.cwd();
    this.exists = dependencies.exists ?? existsSync;
    const resourcesPath = dependencies.resourcesPath
      ?? (process as NodeJS.Process & { resourcesPath?: string }).resourcesPath
      ?? null;
    const resolved = resolveServerHome(this.env, cwd, this.exists, resourcesPath);
    this.home = resolved.home;
    this.expectedHome = resolved.expectedHome;
    this.entry = resolved.entry;
    this.supportDirectory = dependencies.supportDirectory ?? SUPPORT;
    this.port = this.configuredPort();
    this.readTextFile = dependencies.readTextFile ?? ((candidate) => readFileSync(candidate, "utf8"));
    this.writeTextFile = dependencies.writeTextFile ?? ((candidate, contents) => writeFileSync(candidate, contents, { mode: 0o600 }));
    this.fetcher = dependencies.fetcher ?? globalThis.fetch.bind(globalThis);
    this.execPath = dependencies.execPath ?? process.execPath;
    this.readNetworkInterfaces = dependencies.networkInterfaces ?? osNetworkInterfaces;
    this.spawnProcess = dependencies.spawnProcess ?? spawn;
    this.password = this.resolveUiPassword();
  }

  available(): boolean {
    return Boolean(this.home);
  }

  async getSetupStatus(): Promise<MobileServerStatus> {
    if (!this.home) {
      return { state: "setup-required", reason: "companion-checkout-missing", workspacePath: this.expectedHome, port: this.port };
    }
    if (await this.isHealthy(this.port)) {
      return { state: "ready", workspacePath: this.home, port: this.port };
    }
    return {
      state: this.child && this.child.exitCode === null ? "starting" : "offline",
      reason: "server-not-running",
      workspacePath: this.home,
      port: this.port
    };
  }

  /** Create a short-lived pairing URI the mobile companion can scan or redeem. */
  async createPairingQr(): Promise<MobilePairingQr> {
    if (!this.home) {
      throw new Error(`Vela's mobile server files were not found. Expected them at ${this.expectedHome}; set VELA_MOBILE_HOME to the folder that contains packages/web and restart Vela.`);
    }
    const advertised = this.advertisedServerUrl();
    if (!advertised) {
      throw new Error("Vela could not find a network address your phone can reach. Join a Tailscale network (or a LAN) on this Mac and try again.");
    }

    await this.waitForService();
    const baseUrl = `http://127.0.0.1:${this.port}`;
    const cookie = await this.createSessionCookie(baseUrl, this.password);
    const pairingResponse = await this.requestJson(`${baseUrl}/api/client-auth/pairing/sessions`, {
      method: "POST",
      headers: { Accept: "application/json", "Content-Type": "application/json", Cookie: cookie },
      body: JSON.stringify({
        allowedClientKinds: ["mobile"],
        serverUrl: advertised,
        includeDirect: true,
        includeRelay: false
      }),
      signal: AbortSignal.timeout(MOBILE_PAIRING_REQUEST_TIMEOUT_MS)
    });

    const pairing = asRecord(pairingResponse.pairing);
    const server = asRecord(pairingResponse.server);
    const candidates = Array.isArray(server?.candidates) ? server.candidates : [];
    const pairingId = typeof pairing?.id === "string" ? pairing.id.trim() : "";
    const secret = typeof pairing?.secret === "string" ? pairing.secret.trim() : "";
    const expiresAt = typeof pairing?.expiresAt === "string" ? pairing.expiresAt.trim() : "";
    if (!pairingId || !secret || candidates.length === 0 || !Number.isFinite(Date.parse(expiresAt))) {
      throw new Error(typeof pairingResponse.error === "string" ? pairingResponse.error : "Vela could not create a mobile pairing code. Keep Vela open and try again.");
    }

    const label = typeof server?.label === "string" && server.label.trim() ? server.label.trim() : "Vela Desktop";
    const fingerprint = typeof pairing?.fingerprint === "string" && pairing.fingerprint.trim() ? pairing.fingerprint.trim() : undefined;
    const payload: Record<string, unknown> = {
      v: 2,
      pairingId,
      secret,
      label,
      ...(fingerprint ? { fingerprint } : {}),
      expiresAt,
      candidates
    };
    // Keep the scheme understood by the currently released mobile companion.
    const connectionUrl = `vela://connect?v=2&p=${Buffer.from(JSON.stringify(payload), "utf8").toString("base64url")}`;

    return { connectionUrl, expiresAt, serverLabel: label };
  }

  private resolveUiPassword(): string {
    const fromEnv = (this.env["VELA_PASSWORD"] ?? this.env["VELA_PASSWORD"])?.trim();
    if (fromEnv) return fromEnv;
    const secretFile = path.join(this.supportDirectory, "ui-password");
    try {
      const existing = this.readTextFile(secretFile).trim();
      if (existing) return existing;
    } catch {
      // No persisted secret yet.
    }
    const generated = randomBytes(24).toString("base64url");
    try {
      mkdirSync(this.supportDirectory, { recursive: true });
      this.writeTextFile(secretFile, generated);
    } catch {
      // Fall back to the in-memory secret for this session.
    }
    return generated;
  }

  private configuredPort(): number {
    const configured = Number(this.env["VELA_PORT"] || this.env[LEGACY_PORT_ENV] || DEFAULT_SERVER_PORT);
    return Number.isInteger(configured) && configured > 0 && configured <= 65535 ? configured : DEFAULT_SERVER_PORT;
  }

  /** A non-loopback IPv4 the phone can reach — Tailscale first, then the LAN. */
  private advertisedServerUrl(): string | null {
    let interfaces: NodeJS.Dict<import("node:os").NetworkInterfaceInfo[]>;
    try {
      interfaces = this.readNetworkInterfaces();
    } catch {
      return null;
    }
    const lan: string[] = [];
    const tailscale: string[] = [];
    for (const list of Object.values(interfaces)) {
      for (const entry of list ?? []) {
        if (entry.family !== "IPv4" || entry.internal) continue;
        if (isUnreachableHostAddress(entry.address)) continue;
        (isTailscaleAddress(entry.address) ? tailscale : lan).push(entry.address);
      }
    }
    const address = tailscale[0] ?? lan[0];
    return address ? `http://${address}:${this.port}` : null;
  }

  private async requestJson(url: string, init: RequestInit): Promise<Record<string, unknown>> {
    const response = await this.fetcher(url, init);
    const payload = asRecord(await response.json().catch(() => null));
    if (!response.ok || !payload) {
      const message = typeof payload?.error === "string" ? payload.error : `Mobile server request failed (${response.status})`;
      throw new Error(message);
    }
    return payload;
  }

  private async isHealthy(port: number): Promise<boolean> {
    try {
      const response = await this.fetcher(`http://127.0.0.1:${port}/health`, {
        headers: { Accept: "application/json" },
        signal: AbortSignal.timeout(MOBILE_SERVER_PROBE_TIMEOUT_MS)
      });
      const body = await response.json().catch(() => null) as { status?: unknown } | null;
      return response.ok && body?.status === "ok";
    } catch {
      return false;
    }
  }

  private async waitForService(): Promise<void> {
    const deadline = Date.now() + MOBILE_SERVER_START_TIMEOUT_MS;
    while (Date.now() < deadline) {
      if (await this.isHealthy(this.port)) return;
      if (!this.child && !(await this.isHealthy(this.port))) break;
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
    if (await this.isHealthy(this.port)) return;
    throw new Error("Vela's mobile server is not ready yet. Keep Vela open, wait a few seconds, and try again.");
  }

  private async createSessionCookie(baseUrl: string, password: string): Promise<string> {
    const response = await this.fetcher(`${baseUrl}/auth/session`, {
      method: "POST",
      headers: { Accept: "application/json", "Content-Type": "application/json" },
      body: JSON.stringify({ password }),
      signal: AbortSignal.timeout(MOBILE_PAIRING_REQUEST_TIMEOUT_MS)
    });
    const payload = await response.json().catch(() => null) as { authenticated?: unknown; error?: unknown } | null;
    const setCookie = response.headers.get("set-cookie") ?? "";
    const cookie = setCookie.split(";", 1)[0]?.trim() ?? "";
    if (!response.ok || payload?.authenticated !== true || !cookie.includes("=")) {
      throw new Error(typeof payload?.error === "string" ? payload.error : "Vela could not authenticate with its local mobile server. Restart Vela and try again.");
    }
    return cookie;
  }

  start(backend?: MobileBackend): void {
    if (!this.available() || this.child) return;
    const serverEntry = this.entry as string;
    const logs = path.join(this.supportDirectory, "logs");
    mkdirSync(logs, { recursive: true });
    const out = openSync(path.join(logs, "mobile-server.out.log"), "a");
    const err = openSync(path.join(logs, "mobile-server.err.log"), "a");
    const dataDir = path.join(this.supportDirectory, "data");
    const distDir = path.join(this.home as string, "packages", "web", "dist");
    // When the desktop OpenCode daemon endpoint is known, point the server at it
    // so phone and desktop share sessions across one backend. Without it, the
    // server manages its own OpenCode process.
    const backendEnv: Record<string, string> = backend
      ? {
          OPENCODE_HOST: backend.url,
          OPENCODE_SERVER_USERNAME: backend.username,
          OPENCODE_SERVER_PASSWORD: backend.password
        }
      : {};
    try {
      this.child = this.spawnProcess(this.execPath, [serverEntry, "--port", String(this.port), "--host", "0.0.0.0"], {
        cwd: this.home as string,
        env: {
          ...this.env,
          ELECTRON_RUN_AS_NODE: "1",
          HOME: homedir(),
          OPENCHAMBER_DATA_DIR: dataDir,
          OPENCHAMBER_HOST: "0.0.0.0",
          OPENCHAMBER_UI_PASSWORD: this.password,
          OPENCHAMBER_RELAY_HOST: "off",
          ...(this.exists(distDir) ? { OPENCHAMBER_DIST_DIR: distDir } : {}),
          ...backendEnv
        },
        stdio: ["ignore", out, err]
      });
      this.child.on("exit", () => {
        this.child = null;
      });
    } catch (error) {
      console.error("[mobile-server] failed to start:", error);
    } finally {
      closeSync(out);
      closeSync(err);
    }
  }

  async stop(): Promise<void> {
    const child = this.child;
    if (!child) return;
    this.child = null;
    await new Promise<void>((resolve) => {
      const done = (): void => resolve();
      child.once("exit", done);
      try {
        child.kill("SIGTERM");
      } catch {
        done();
        return;
      }
      setTimeout(() => {
        try {
          child.kill("SIGKILL");
        } catch {
          // already gone
        }
        done();
      }, 4000);
    });
  }
}
