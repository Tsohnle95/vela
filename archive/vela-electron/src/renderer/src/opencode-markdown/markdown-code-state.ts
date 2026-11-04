// Adapted from anomalyco/opencode at 03e67171ab2dc1e7f16e8cebfbc7f778f61b89f0 (MIT; see LICENSE).
import type { MarkdownToken } from "./markdown-worker-protocol"

export type RenderedCodeState = {
  language: string
  generation: number
  stableCount: number
  unstable: MarkdownToken[]
  raw: string
}

export function shouldResetCodeTokens(
  previous: RenderedCodeState | undefined,
  next: { language: string; generation: number; stableCount: number; raw: string },
) {
  return (
    !previous ||
    previous.language !== next.language ||
    previous.generation !== next.generation ||
    next.stableCount < previous.stableCount ||
    !next.raw.startsWith(previous.raw)
  )
}
