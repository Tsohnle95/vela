import { createMarkdownWorkerHandler } from "./markdown-engine";
import type { MarkdownWorkerRequest, MarkdownWorkerResponse } from "./markdown-worker-protocol";

// Exercise the production worker protocol and engine without a browser Worker thread.
export class InProcessMarkdownWorker {
  onmessage: ((event: MessageEvent<MarkdownWorkerResponse>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  onmessageerror: (() => void) | null = null;
  private readonly handle = createMarkdownWorkerHandler((response) => {
    queueMicrotask(() => this.onmessage?.({ data: response } as MessageEvent<MarkdownWorkerResponse>));
  });
  postMessage(request: MarkdownWorkerRequest) { this.handle(request); }
  terminate() {}
}
