// Adapted from anomalyco/opencode at 03e67171ab2dc1e7f16e8cebfbc7f778f61b89f0 (MIT; see LICENSE).
/// <reference lib="webworker" />
import { createMarkdownWorkerHandler } from "./markdown-engine"
import type { MarkdownWorkerRequest } from "./markdown-worker-protocol"

const handle = createMarkdownWorkerHandler((response) => self.postMessage(response))
self.onmessage = (event: MessageEvent<MarkdownWorkerRequest>) => handle(event.data)
