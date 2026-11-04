// React lifecycle adapter for OpenCode's Markdown renderer. Upstream: 03e67171ab2dc1e7f16e8cebfbc7f778f61b89f0.
import { useId, useLayoutEffect, useRef, type HTMLAttributes } from "react";
import { completedProjection } from "../markdown-projection";
import type { Projection } from "../markdown-projection";
import {
  disposeMarkdownProjection,
  disposeStreamingCode,
  MarkdownWorkerDisposedError,
  MarkdownWorkerSupersededError,
  projectMarkdown
} from "../opencode-markdown/markdown-worker";
import {
  initialResult,
  pendingBlocks,
  pendingProjection,
  renderProjection,
  setCopyState,
  setupCodeCopy,
  updateBlock,
  type RenderedBlock,
  type RenderResult
} from "../opencode-markdown/render";

type OpenCodeMarkdownProps = Omit<HTMLAttributes<HTMLDivElement>, "children"> & {
  text: string;
  streaming?: boolean;
  cacheKey?: string;
};

export function OpenCodeMarkdown({ text, streaming = false, cacheKey, ...props }: OpenCodeMarkdownProps) {
  const owner = useId();
  const root = useRef<HTMLDivElement>(null);
  const state = useRef<{
    revision: number;
    streamed: boolean;
    projection?: Projection;
    result?: RenderResult;
    activeCodeKeys: Set<string>;
    completedCode: Map<string, Extract<RenderedBlock, { mode: "code" }>>;
  }>({ revision: 0, streamed: false, activeCodeKeys: new Set(), completedCode: new Map() });

  useLayoutEffect(() => {
    const container = root.current;
    if (!container) return;
    const current = state.current;
    const cleanup = setupCodeCopy(container, () => ({ copy: "Copy", copied: "Copied" }));
    return () => {
      current.revision++;
      cleanup();
      disposeMarkdownProjection(owner);
      current.activeCodeKeys.forEach(disposeStreamingCode);
      current.activeCodeKeys.clear();
      current.completedCode.clear();
    };
  }, [owner]);

  useLayoutEffect(() => {
    const container = root.current;
    if (!container) return;
    const current = state.current;
    const revision = ++current.revision;
    if (streaming) current.streamed = true;
    const labels = { copy: "Copy", copied: "Copied" };
    const apply = (projection: Projection) => {
      if (current.revision !== revision) return;
      const content = text ? pendingBlocks(current.result, projection, cacheKey, owner) : [];
      const keys = new Set(content.filter((block) => block.mode === "code").map((block) => block.key));
      current.activeCodeKeys.forEach((key) => {
        if (!keys.has(key)) disposeStreamingCode(key);
      });
      current.activeCodeKeys = keys;
      content.forEach((block, index) => updateBlock(container, index, block, labels));
      while (container.children.length > content.length) container.lastElementChild?.remove();
      container.querySelectorAll<HTMLElement>('[data-slot="markdown-copy-button"]').forEach((button) => {
        setCopyState(button, labels, button.dataset.copied === "true");
      });
    };
    const initialProjection = !streaming && !current.streamed
      ? completedProjection(text)
      : current.projection?.text ? current.projection : pendingProjection(text);
    current.result ??= initialResult(text, cacheKey, initialProjection, owner);
    apply(initialProjection);

    const render = async () => {
      const projection = !streaming && !current.streamed
        ? completedProjection(text)
        : await projectMarkdown(owner, text, streaming);
      if (current.revision !== revision) return;
      current.projection = projection;
      apply(projection);
      const result = await renderProjection({ text, key: cacheKey, projection }, owner, current.completedCode);
      if (current.revision !== revision) return;
      current.result = result;
      apply(projection);
    };
    void render().catch((error: unknown) => {
      if (error instanceof MarkdownWorkerDisposedError || error instanceof MarkdownWorkerSupersededError) return;
      // Keep the escaped pending text visible if the worker cannot be started.
      if (current.revision !== revision) return;
      current.result = initialResult(text, undefined, pendingProjection(text), owner);
      apply(pendingProjection(text));
    });
  }, [text, streaming, cacheKey, owner]);

  return <div {...props} data-component="markdown" dir="auto" ref={root} />;
}
