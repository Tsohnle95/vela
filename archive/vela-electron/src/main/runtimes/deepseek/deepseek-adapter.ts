import { spawn } from "node:child_process";
import path from "node:path";
import type { ModelOption, RuntimeManifest, SessionSelection, SessionSummary, SessionTranscript } from "@shared/types";
import type { RuntimeAdapter, RuntimeEventEnvelope, RuntimeSessionDraft } from "../runtime-adapter";
import { RUNTIME_PROTOCOL_VERSION } from "../runtime-adapter";
import { deepSeekRuntimeEvent, deepSeekTranscript } from "./deepseek-events";
import { DeepSeekRpcClient } from "./deepseek-rpc";
import { DeepSeekStreamProjector } from "./deepseek-stream";

interface DeepSeekSessionSummary {
  sessionId: string;
  updatedAt: number;
  blank: boolean;
  parentSessionId?: string;
  cwd?: string;
  agentPreset?: string;
  projections?: { values?: { title?: unknown } };
}

interface DeepSeekHistoryEntry {
  event: { type: string; seq: number; time?: number; data: Record<string, unknown> };
  view?: unknown;
}

interface DeepSeekModelGroup {
  id: string;
  models: Array<{
    id: string;
    name: string;
    reasoning?: { efforts?: Array<{ id: string }> };
  }>;
}

interface DeepSeekAdapterOptions {
  directory: string;
  baseUrl?: string;
  command?: string;
  startupTimeoutMs?: number;
  client?: DeepSeekRpcClient;
}

const STARTUP_URL = /^dsh web: (http:\/\/[^\s]+)/m;

export function deepSeekStartupUrl(output: string): string | null {
  return STARTUP_URL.exec(output)?.[1] ?? null;
}

function titleOf(summary: DeepSeekSessionSummary): string {
  const title = summary.projections?.values?.title;
  return typeof title === "string" && title.trim() ? title : path.basename(summary.cwd ?? "") || "DeepSeek session";
}

export class DeepSeekRuntimeAdapter implements RuntimeAdapter {
  readonly manifest: RuntimeManifest = {
    protocolVersion: RUNTIME_PROTOCOL_VERSION,
    id: "deepseek",
    name: "DeepSeek Harness",
    version: null,
    available: false,
    capabilities: {
      attachments: false,
      commands: false,
      models: true,
      agents: false,
      permissions: false,
      providerCredentials: false,
      sessionFork: false,
      sessionResume: true,
      steering: false,
      tui: false
    }
  };

  private client: DeepSeekRpcClient | null;
  private process: ReturnType<typeof spawn> | null = null;
  private stopped = false;
  private readonly streamProjector = new DeepSeekStreamProjector();

  constructor(private readonly options: DeepSeekAdapterOptions) {
    this.client = options.client ?? (options.baseUrl ? new DeepSeekRpcClient(options.baseUrl) : null);
  }

  async connect(): Promise<boolean> {
    this.stopped = false;
    try {
      if (!this.client) this.client = new DeepSeekRpcClient(await this.startService());
      const host = await this.client.call<{ version: string }>("host.describe", {});
      this.manifest.version = host.version;
      this.manifest.available = true;
      return true;
    } catch {
      this.manifest.available = false;
      return false;
    }
  }

  private startService(): Promise<string> {
    return new Promise((resolve, reject) => {
      const child = spawn(this.options.command ?? "dsh", ["web", "--host", "127.0.0.1", "--port", "0", "--no-open"], {
        cwd: this.options.directory,
        env: process.env,
        stdio: ["ignore", "pipe", "pipe"]
      });
      this.process = child;
      let output = "";
      let settled = false;
      const finish = (error?: Error, url?: string): void => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        if (error) {
          child.kill();
          if (this.process === child) this.process = null;
          reject(error);
        } else {
          resolve(url!);
        }
      };
      const consume = (chunk: Buffer): void => {
        output = (output + chunk.toString()).slice(-32_768);
        const url = deepSeekStartupUrl(output);
        if (url) finish(undefined, url);
      };
      child.stdout.on("data", consume);
      child.stderr.on("data", consume);
      child.once("error", (error) => finish(error));
      child.once("exit", (code) => {
        if (this.process === child) this.process = null;
        finish(new Error(`DeepSeek Harness exited before startup (${code ?? "signal"})`));
      });
      const timer = setTimeout(() => finish(new Error("DeepSeek Harness startup timed out")), this.options.startupTimeoutMs ?? 30_000);
    });
  }

  private rpc(): DeepSeekRpcClient {
    if (!this.client) throw new Error("DeepSeek Harness is not connected");
    return this.client;
  }

  async createSession(directory: string): Promise<RuntimeSessionDraft> {
    const value = await this.rpc().call<{ sessionId: string; agentPreset?: string }>("session.create", { cwd: directory });
    return { id: value.sessionId, directory, ...(value.agentPreset ? { agent: value.agentPreset } : {}) };
  }

  async listSessions(): Promise<SessionSummary[]> {
    const value = await this.rpc().call<{ items: DeepSeekSessionSummary[] }>("session.list", {});
    return value.items.filter((item) => !item.blank && item.cwd).map((item) => ({
      id: item.sessionId,
      runtimeID: "deepseek",
      title: titleOf(item),
      directory: item.cwd!,
      updatedAt: item.updatedAt,
      ...(item.parentSessionId ? { parentID: item.parentSessionId } : {}),
      ...(item.agentPreset ? { agent: item.agentPreset } : {})
    }));
  }

  async sessionInfo(sessionID: string): Promise<RuntimeSessionDraft> {
    const value = await this.rpc().call<{ items: DeepSeekSessionSummary[] }>("session.list", {});
    const item = value.items.find((candidate) => candidate.sessionId === sessionID);
    if (!item?.cwd) throw new Error("DeepSeek session not found");
    return {
      id: item.sessionId,
      directory: item.cwd,
      title: titleOf(item),
      ...(item.parentSessionId ? { parentID: item.parentSessionId } : {}),
      ...(item.agentPreset ? { agent: item.agentPreset } : {})
    };
  }

  async sessionTranscript(sessionID: string): Promise<SessionTranscript> {
    const entries: DeepSeekHistoryEntry[] = [];
    let beforeSeq: number | undefined;
    for (let page = 0; page < 100; page += 1) {
      const value = await this.rpc().call<{ events: DeepSeekHistoryEntry[]; hasMore: boolean }>("session.history", {
        sessionId: sessionID,
        maxMessages: 100,
        ...(beforeSeq === undefined ? {} : { beforeSeq })
      });
      entries.push(...value.events);
      if (!value.hasMore || value.events.length === 0) break;
      beforeSeq = Math.min(...value.events.map((entry) => entry.event.seq));
    }
    return deepSeekTranscript(entries);
  }

  async prompt(sessionID: string, text: string): Promise<void> {
    await this.rpc().call("session.prompt", {
      sessionId: sessionID,
      mode: "queue",
      content: [{ type: "text", text }]
    });
  }

  async interrupt(sessionID: string): Promise<void> {
    await this.rpc().call("session.cancel", { sessionId: sessionID });
  }

  async listModels(sessionID: string): Promise<ModelOption[]> {
    const value = await this.rpc().call<{
      current: { provider: string; model: string; reasoningEffort?: string };
      groups: DeepSeekModelGroup[];
    }>("session.models", { sessionId: sessionID });
    return value.groups.flatMap((group) => group.models.map((model) => ({
      id: model.id,
      providerID: group.id,
      name: model.name,
      variants: model.reasoning?.efforts?.map((effort) => effort.id) ?? [],
      ...(value.current.provider === group.id && value.current.model === model.id && value.current.reasoningEffort
        ? { variant: value.current.reasoningEffort }
        : {})
    })));
  }

  async sessionSelection(sessionID: string): Promise<SessionSelection | null> {
    const value = await this.rpc().call<{
      current: { provider: string; model: string; reasoningEffort?: string };
    }>("session.models", { sessionId: sessionID });
    return {
      model: {
        id: value.current.model,
        providerID: value.current.provider,
        name: value.current.model,
        ...(value.current.reasoningEffort ? { variant: value.current.reasoningEffort } : {})
      }
    };
  }

  async switchModel(sessionID: string, modelID: string, providerID: string, variant?: string): Promise<void> {
    await this.rpc().call("session.selectModel", {
      sessionId: sessionID,
      provider: providerID,
      model: modelID,
      ...(variant ? { reasoningEffort: variant } : {})
    });
  }

  async *subscribe(signal: AbortSignal): AsyncIterable<RuntimeEventEnvelope> {
    const queue: RuntimeEventEnvelope[] = [];
    const waiters: Array<() => void> = [];
    const seen = new Set<string>();
    let pumps = 2;
    let failure: unknown = null;
    const wake = (): void => waiters.splice(0).forEach((resolve) => resolve());
    const pump = async (stream: "mux" | "host"): Promise<void> => {
      let delay = 250;
      while (!signal.aborted && !this.stopped) {
        try {
          for await (const frame of this.rpc().events<Record<string, unknown>>(stream, signal)) {
            if (seen.has(frame.rpcId)) continue;
            seen.add(frame.rpcId);
            if (seen.size > 10_000) seen.delete(seen.values().next().value!);
            const sessionID = typeof frame.payload.sessionId === "string" ? frame.payload.sessionId : undefined;
            const native = frame.payload.type === "session/event" && sessionID && frame.payload.event && typeof frame.payload.event === "object"
              ? this.streamProjector.project(sessionID, frame.payload.event as DeepSeekHistoryEntry["event"], frame.payload.view)
              : [];
            if (frame.payload.type === "host/session-added" && sessionID) {
              native.push({
                type: "stream.event",
                eventType: "session.created",
                created: Date.now(),
                data: {
                  sessionID,
                  location: { directory: typeof frame.payload.cwd === "string" ? frame.payload.cwd : this.options.directory },
                  ...(typeof frame.payload.parentSessionId === "string" ? { parentID: frame.payload.parentSessionId } : {}),
                  ...(typeof frame.payload.agentPreset === "string" ? { agent: frame.payload.agentPreset } : {}),
                  ...(frame.payload.origin === "subagent" ? { origin: "subagent" } : {})
                }
              });
            } else if (frame.payload.type === "host/session-removed" && sessionID) {
              native.push({ type: "stream.event", eventType: "session.deleted", created: Date.now(), data: { sessionID } });
            } else if (frame.payload.type === "session/projection" && sessionID && frame.payload.key === "title" && typeof frame.payload.value === "string") {
              native.push({ type: "stream.event", eventType: "session.renamed", created: Date.now(), data: { sessionID, title: frame.payload.value } });
            }
            const lifecycle = deepSeekRuntimeEvent(frame.payload);
            const events = [...native, ...(lifecycle ? [lifecycle] : [])];
            if (events.length === 0) continue;
            events.forEach((event, index) => queue.push({
              runtimeID: "deepseek",
              eventID: `${frame.rpcId}:${index}`,
              ...(sessionID ? { sessionID } : {}),
              event
            }));
            wake();
          }
          delay = 250;
        } catch (error) {
          if (signal.aborted || this.stopped) break;
          failure = error;
          await new Promise((resolve) => setTimeout(resolve, delay));
          delay = Math.min(delay * 2, 5000);
        }
      }
      pumps -= 1;
      wake();
    };
    void pump("mux");
    void pump("host");
    while (!signal.aborted && !this.stopped && (pumps > 0 || queue.length > 0)) {
      if (queue.length > 0) {
        yield queue.shift()!;
        continue;
      }
      await new Promise<void>((resolve) => waiters.push(resolve));
    }
    if (failure && !signal.aborted && !this.stopped) throw failure;
  }

  async stop(): Promise<void> {
    this.stopped = true;
    this.manifest.available = false;
    const child = this.process;
    this.process = null;
    if (child && child.exitCode === null && child.signalCode === null) {
      await new Promise<void>((resolve) => {
        const timer = setTimeout(() => {
          child.kill("SIGKILL");
          resolve();
        }, 2000);
        child.once("exit", () => {
          clearTimeout(timer);
          resolve();
        });
        child.kill("SIGTERM");
      });
    }
  }
}
