// Adapted from anomalyco/opencode at 03e67171ab2dc1e7f16e8cebfbc7f778f61b89f0 (MIT; see LICENSE).
export function checksum(content: string): string | undefined {
  if (!content) return undefined
  let hash = 0x811c9dc5
  for (let i = 0; i < content.length; i++) {
    hash ^= content.charCodeAt(i)
    hash = Math.imul(hash, 0x01000193)
  }
  return (hash >>> 0).toString(36)
}
