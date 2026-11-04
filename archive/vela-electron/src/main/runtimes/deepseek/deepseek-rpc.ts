import { randomUUID } from "node:crypto";

const MAX_RESPONSE_BYTES = 16 * 1024 * 1024;
const DEFAULT_TIMEOUT_MS = 15_000;

interface RpcSuccess<T> {
  ok: true;
  value: T;
}

interface RpcFailure {
  ok: false;
  error: {
    code: string;
    message: string;
    details: unknown;
  };
}

interface ServerResponse<T> {
  type: "server-response";
  rpcId: string;
  result: RpcSuccess<T> | RpcFailure;
}

interface ServerRequest<T> {
  type: "server-request";
  rpcId: string;
  method: string;
  payload: T;
}

export class DeepSeekRpcError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly details: unknown
  ) {
    super(message);
    this.name = "DeepSeekRpcError";
  }
}

function loopback(hostname: string): boolean {
  if (hostname === "localhost" || hostname === "[::1]" || hostname === "::1") return true;
  const octets = hostname.split(".");
  return octets.length === 4 && octets[0] === "127" && octets.every((part) => /^\d{1,3}$/.test(part) && Number(part) <= 255);
}

export function deepSeekBaseUrl(value: string): URL {
  const url = new URL(value);
  if (url.protocol !== "http:" || !loopback(url.hostname) || url.username || url.password || url.search || url.hash) {
    throw new Error("DeepSeek Harness URL must be an unauthenticated loopback HTTP URL");
  }
  url.pathname = url.pathname.replace(/\/$/, "");
  return url;
}

async function boundedText(response: Response): Promise<string> {
  const declared = Number(response.headers.get("content-length"));
  if (Number.isFinite(declared) && declared > MAX_RESPONSE_BYTES) throw new Error("DeepSeek Harness response is too large");
  if (!response.body) return "";
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let size = 0;
  let text = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > MAX_RESPONSE_BYTES) {
      await reader.cancel();
      throw new Error("DeepSeek Harness response is too large");
    }
    text += decoder.decode(value, { stream: true });
  }
  return text + decoder.decode();
}

function object(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function parseResponse<T>(value: unknown, rpcId: string): ServerResponse<T> {
  if (!object(value) || value.type !== "server-response" || value.rpcId !== rpcId || !object(value.result)) {
    throw new Error("DeepSeek Harness returned an invalid or mismatched RPC response");
  }
  if (value.result.ok === true && "value" in value.result) return value as unknown as ServerResponse<T>;
  const error = value.result.error;
  if (value.result.ok !== false || !object(error) || typeof error.code !== "string" || typeof error.message !== "string") {
    throw new Error("DeepSeek Harness returned an invalid RPC result");
  }
  return value as unknown as ServerResponse<T>;
}

export class DeepSeekRpcClient {
  readonly baseUrl: URL;

  constructor(
    baseUrl: string,
    private readonly fetcher: typeof fetch = fetch,
    private readonly webSocketFactory: (url: string) => WebSocket = (url) => new WebSocket(url)
  ) {
    this.baseUrl = deepSeekBaseUrl(baseUrl);
  }

  async call<T>(method: string, payload: Record<string, unknown>, signal?: AbortSignal): Promise<T> {
    if (!/^[a-z][a-z0-9-]*\.[A-Za-z][A-Za-z0-9]*$/.test(method)) throw new Error("Invalid DeepSeek Harness RPC method");
    const rpcId = randomUUID();
    const timeout = AbortSignal.timeout(DEFAULT_TIMEOUT_MS);
    const combined = signal ? AbortSignal.any([signal, timeout]) : timeout;
    const response = await this.fetcher(new URL(`/api/${method}`, this.baseUrl), {
      method: "POST",
      redirect: "manual",
      signal: combined,
      headers: {
        accept: "application/json",
        "content-type": "application/json",
        host: this.baseUrl.host
      },
      body: JSON.stringify({ type: "client-request", rpcId, method, payload })
    });
    if (response.status >= 300 && response.status < 400) throw new Error("DeepSeek Harness refused an RPC redirect");
    if (!response.ok) throw new Error(`DeepSeek Harness RPC failed with HTTP ${response.status}`);
    const contentType = response.headers.get("content-type")?.split(";", 1)[0].trim().toLowerCase();
    if (contentType !== "application/json") throw new Error("DeepSeek Harness RPC returned non-JSON content");
    const text = await boundedText(response);
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      throw new Error("DeepSeek Harness RPC returned malformed JSON");
    }
    const envelope = parseResponse<T>(parsed, rpcId);
    if (!envelope.result.ok) throw new DeepSeekRpcError(envelope.result.error.code, envelope.result.error.message, envelope.result.error.details);
    return envelope.result.value;
  }

  async *events<T>(stream: "mux" | "host", signal: AbortSignal): AsyncIterable<ServerRequest<T>> {
    const url = new URL(`/api/events.${stream}`, this.baseUrl);
    url.protocol = "ws:";
    const socket = this.webSocketFactory(url.toString());
    const queue: ServerRequest<T>[] = [];
    const waiters: Array<() => void> = [];
    let closed = false;
    let failure: Error | null = null;
    const wake = (): void => waiters.splice(0).forEach((resolve) => resolve());
    const onMessage = (message: MessageEvent): void => {
      void (async () => {
        const text = typeof message.data === "string"
          ? message.data
          : message.data instanceof ArrayBuffer
            ? new TextDecoder().decode(message.data)
            : message.data instanceof Blob
              ? await message.data.text()
              : "";
        if (new TextEncoder().encode(text).byteLength > MAX_RESPONSE_BYTES) {
          failure = new Error("DeepSeek Harness event frame is too large");
          socket.close();
          wake();
          return;
        }
        let parsed: unknown;
        try {
          parsed = JSON.parse(text);
        } catch {
          failure = new Error("DeepSeek Harness event stream returned malformed JSON");
          socket.close();
          wake();
          return;
        }
        if (!object(parsed) || parsed.type !== "server-request" || typeof parsed.rpcId !== "string" || typeof parsed.method !== "string" || !("payload" in parsed)) {
          failure = new Error("DeepSeek Harness event stream returned an invalid frame");
          socket.close();
          wake();
          return;
        }
        queue.push(parsed as unknown as ServerRequest<T>);
        wake();
      })();
    };
    const onError = (): void => {
      failure ??= new Error("DeepSeek Harness event WebSocket failed");
      wake();
    };
    const onClose = (): void => {
      closed = true;
      wake();
    };
    const onAbort = (): void => socket.close();
    socket.addEventListener("message", onMessage);
    socket.addEventListener("error", onError);
    socket.addEventListener("close", onClose);
    signal.addEventListener("abort", onAbort, { once: true });
    try {
      while (!signal.aborted && (!closed || queue.length > 0)) {
        if (queue.length > 0) {
          yield queue.shift()!;
          continue;
        }
        if (failure) throw failure;
        await new Promise<void>((resolve) => waiters.push(resolve));
      }
      if (failure && !signal.aborted) throw failure;
    } finally {
      signal.removeEventListener("abort", onAbort);
      socket.removeEventListener("message", onMessage);
      socket.removeEventListener("error", onError);
      socket.removeEventListener("close", onClose);
      if (!closed) socket.close();
    }
  }
}
