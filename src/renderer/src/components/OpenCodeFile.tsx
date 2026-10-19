import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import {
  File as PierreFile,
  FileDiff,
  VirtualizedFile,
  Virtualizer,
  DEFAULT_VIRTUAL_FILE_METRICS,
  type FileDiffOptions
} from "@pierre/diffs";
import { createDefaultOptions, styleVariables } from "../opencode-file/default-options";
import { resolveFileDiff, type DiffSource } from "../opencode-file/diff-source";
import { getFileWorkerPool } from "../opencode-file/worker";
import { checksum } from "../opencode-markdown/checksum";
import "../opencode-file/file.css";

type SharedProps = {
  file: string;
  overflow?: "wrap" | "scroll";
  onRendered?: () => void;
  className?: string;
};

export type OpenCodeFileProps = SharedProps & (
  | { mode: "text"; contents: string }
  | ({ mode: "diff" } & Omit<DiffSource, "file">)
);

const VIRTUALIZE_BYTES = 500_000;
const codeMetrics = { ...DEFAULT_VIRTUAL_FILE_METRICS, lineHeight: 24, spacing: 0 };

function scrollParent(element: HTMLElement): HTMLElement | undefined {
  let parent = element.parentElement;
  while (parent) {
    const overflow = getComputedStyle(parent).overflowY;
    if (overflow === "auto" || overflow === "scroll") return parent;
    parent = parent.parentElement;
  }
}

function preservePosition(wrapper: HTMLElement, container: HTMLElement): () => void {
  const root = scrollParent(wrapper);
  const height = container.getBoundingClientRect().height;
  if (!root || !height) return () => {};
  const top = wrapper.getBoundingClientRect().top - root.getBoundingClientRect().top;
  const previous = container.style.minHeight;
  container.style.minHeight = `${Math.ceil(height)}px`;
  let done = false;
  return () => {
    if (done) return;
    done = true;
    container.style.minHeight = previous;
    const delta = wrapper.getBoundingClientRect().top - root.getBoundingClientRect().top - top;
    if (delta) root.scrollTop += delta;
  };
}

/** The nonvirtual timeline branch of OpenCode's File component, hosted by React. */
export function OpenCodeFile(props: OpenCodeFileProps): ReactNode {
  const wrapper = useRef<HTMLDivElement>(null);
  const container = useRef<HTMLDivElement>(null);
  const instance = useRef<PierreFile | FileDiff | VirtualizedFile | null>(null);
  const mode = useRef<OpenCodeFileProps["mode"] | null>(null);
  const onRendered = useRef(props.onRendered);
  onRendered.current = props.onRendered;
  const [mobile, setMobile] = useState(() => window.matchMedia?.("(max-width: 640px)").matches ?? false);
  const before = props.mode === "diff" ? props.before : undefined;
  const after = props.mode === "diff" ? props.after : undefined;
  const patch = props.mode === "diff" ? props.patch : undefined;
  const contents = props.mode === "text" ? props.contents : undefined;
  const fileDiff = useMemo(() => {
    if (props.mode !== "diff") return;
    try {
      return resolveFileDiff({ file: props.file, patch, before, after });
    } catch {
      return resolveFileDiff({ file: props.file, before, after });
    }
  }, [props.mode, props.file, patch, before, after]);

  useLayoutEffect(() => {
    const query = window.matchMedia?.("(max-width: 640px)");
    if (!query) return;
    const update = (): void => setMobile(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);

  useLayoutEffect(() => {
    const host = container.current;
    const outer = wrapper.current;
    if (!host || !outer) return;
    const apply = (): void => {
      const viewer = host.querySelector<HTMLElement>("diffs-container");
      if (!viewer) return;
      // Vela's ThemeProvider owns colorScheme; upstream stores the same value in a data attribute.
      const scheme = document.documentElement.style.colorScheme || getComputedStyle(outer).colorScheme;
      if (scheme === "dark" || scheme === "light") viewer.dataset.colorScheme = scheme;
      else delete viewer.dataset.colorScheme;
    };
    const schemeObserver = new MutationObserver(apply);
    schemeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["class", "style", "data-theme", "data-color-scheme"] });
    const hostObserver = new MutationObserver(apply);
    hostObserver.observe(host, { childList: true });
    apply();
    return () => { schemeObserver.disconnect(); hostObserver.disconnect(); };
  }, []);

  useLayoutEffect(() => {
    const host = container.current;
    const outer = wrapper.current;
    if (!host || !outer) return;
    const finishPosition = props.mode === "diff" ? preservePosition(outer, host) : () => {};
    let cancelled = false;
    let frame = 0;
    let readyObserver: MutationObserver | undefined;
    let virtualizer: Virtualizer | undefined;
    const options = {
      ...createDefaultOptions("unified"),
      overflow: props.overflow ?? "wrap",
      ...(fileDiff?.isPartial ? { hunkSeparators: "simple" as const } : {}),
      ...(props.mode === "diff" && mobile ? { disableLineNumbers: true } : {})
    } satisfies FileDiffOptions<undefined>;
    if (fileDiff && Math.max(fileDiff.deletionLines.join("").length, fileDiff.additionLines.join("").length) > VIRTUALIZE_BYTES) {
      Object.assign(options, { lineDiffType: "none", maxLineDiffLength: 0, tokenizeMaxLineLength: 1 });
    }
    const pool = getFileWorkerPool();
    if (props.mode === "text" || mode.current !== props.mode) {
      instance.current?.cleanUp();
      instance.current = null;
      host.replaceChildren();
    }
    mode.current = props.mode;
    if (props.mode === "text") {
      if ((contents?.length ?? 0) > VIRTUALIZE_BYTES) {
        virtualizer = new Virtualizer();
        const root = scrollParent(outer) ?? document;
        virtualizer.setup(root, root instanceof Document ? undefined : outer);
        instance.current = new VirtualizedFile(options, virtualizer, codeMetrics, pool);
      } else {
        instance.current = new PierreFile(options, pool);
      }
      (instance.current as PierreFile).render({
        file: { name: props.file, contents: contents ?? "", cacheKey: checksum(contents ?? "") },
        lineAnnotations: [], containerWrapper: host
      });
    } else if (fileDiff) {
      const diff = instance.current as FileDiff | null;
      if (diff) diff.setOptions(options);
      else instance.current = new FileDiff(options, pool);
      (instance.current as FileDiff).render({ fileDiff, lineAnnotations: [], containerWrapper: host, forceRender: !!diff });
    }

    const notify = (): void => {
      if (cancelled) return;
      const root = host.querySelector("diffs-container")?.shadowRoot;
      if (!root) return;
      const count = props.mode === "text" && !virtualizer
        ? Math.max(1, (contents ?? "").split("\n").length - ((contents ?? "").endsWith("\n") ? 1 : 0))
        : 1;
      if (root.querySelectorAll("[data-line]").length < count) {
        readyObserver?.disconnect();
        readyObserver = new MutationObserver(notify);
        readyObserver.observe(root, { childList: true, subtree: true });
        return;
      }
      readyObserver?.disconnect();
      frame = requestAnimationFrame(() => {
        const finish = (): void => { if (!cancelled) { finishPosition(); onRendered.current?.(); } };
        if (props.mode === "diff") frame = requestAnimationFrame(finish);
        else finish();
      });
    };
    readyObserver = new MutationObserver(notify);
    readyObserver.observe(host, { childList: true, subtree: true });
    notify();
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
      readyObserver?.disconnect();
      finishPosition();
      if (virtualizer) { instance.current?.cleanUp(); instance.current = null; virtualizer.cleanUp(); }
    };
  }, [props.mode, props.file, props.overflow, fileDiff, contents, mobile]);

  useLayoutEffect(() => () => { instance.current?.cleanUp(); instance.current = null; }, []);

  return (
    <div ref={wrapper} data-component="file" data-mode={props.mode} dir="ltr" tabIndex={0}
      className={`opencode-file${props.className ? ` ${props.className}` : ""}`} style={styleVariables as CSSProperties}>
      <div ref={container} />
      <div style={{ pointerEvents: "none", position: "absolute", inset: 0, zIndex: 0 }} />
    </div>
  );
}
