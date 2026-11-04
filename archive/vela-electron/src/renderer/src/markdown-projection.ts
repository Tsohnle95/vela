export type Block = {
  raw: string;
  src: string;
  mode: "full" | "live" | "code";
  language?: string;
  complete?: boolean;
};

export type Projection = {
  text: string;
  blocks: Block[];
};

export function completedProjection(text: string): Projection {
  return { text, blocks: [{ raw: text, src: text, mode: "full" }] };
}

export function canReusePendingBlock(current: Pick<Block, "mode" | "raw"> | undefined, next: Block): boolean {
  if (!current || current.mode !== next.mode) return false;
  if (next.mode === "code" || next.mode === "live") return next.raw.startsWith(current.raw);
  return current.raw === next.raw;
}
