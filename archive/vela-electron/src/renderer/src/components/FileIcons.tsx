import type { ReactNode } from "react";
import { TREE_BUILTIN_GLYPHS, treeGlyphName, treeIconColor } from "./file-glyphs";

const STROKE = { stroke: "currentColor", fill: "none", strokeWidth: 1.5, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };
const FINE = { stroke: "currentColor", fill: "none", strokeWidth: 1.3, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };

const SHEET = "M4.2 1.8h4.6l3.4 3.4v8.2a1.4 1.4 0 0 1-1.4 1.4H4.2a1.4 1.4 0 0 1-1.4-1.4V3.2a1.4 1.4 0 0 1 1.4-1.4z";
const FOLD = "M8.8 1.8v3.6h3.4";

export type FileGlyph = "generic" | "code" | "json" | "markdown" | "shell" | "text" | "config" | "media" | "binary";

const FILE_GLYPHS: Record<string, FileGlyph> = {
  ts: "code",
  tsx: "code",
  js: "code",
  jsx: "code",
  mjs: "code",
  cjs: "code",
  css: "code",
  scss: "code",
  less: "code",
  html: "code",
  xml: "code",
  py: "code",
  go: "code",
  rs: "code",
  rb: "code",
  java: "code",
  kt: "code",
  c: "code",
  h: "code",
  cpp: "code",
  cs: "code",
  php: "code",
  sql: "code",
  json: "json",
  jsonc: "json",
  md: "markdown",
  mdx: "markdown",
  sh: "shell",
  zsh: "shell",
  bash: "shell",
  command: "shell",
  yml: "config",
  yaml: "config",
  toml: "config",
  ini: "config",
  env: "text",
  conf: "text",
  txt: "text",
  log: "text",
  csv: "text",
  gitignore: "text",
  svg: "media",
  png: "media",
  jpg: "media",
  jpeg: "media",
  gif: "media",
  webp: "media",
  ico: "media",
  pdf: "media",
  bin: "binary",
  exe: "binary",
  dll: "binary",
  so: "binary",
  dylib: "binary"
};

function glyphFor(name: string): FileGlyph {
  if (name.includes(".")) {
    const ext = name.slice(name.lastIndexOf(".") + 1).toLowerCase();
    const hit = FILE_GLYPHS[ext];
    if (hit) return hit;
  }
  return FILE_GLYPHS[name.toLowerCase()] ?? "generic";
}

function Glyph({ children }: { children: ReactNode }): ReactNode {
  return <g {...FINE}>{children}</g>;
}

export function FolderIcon({ open }: { open?: boolean }): ReactNode {
  return (
    <svg className="si-icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
      {open ? (
        <>
          <path d="M2.2 6.4V4.4c0-.9.7-1.6 1.6-1.6h2.4c.47 0 .92.21 1.22.58l.88 1.02h4.28c.9 0 1.6.72 1.6 1.6v1" {...STROKE} />
          <path d="M2.1 12.1l1.6-3.7c.21-.49.69-.8 1.22-.8h8.06c.83 0 1.4.82 1.13 1.6l-1.03 2.95c-.19.55-.71.92-1.29.92H3.25c-.85 0-1.44-.84-1.15-1.64z" {...STROKE} />
        </>
      ) : (
        <path d="M2.2 4.4c0-.9.7-1.6 1.6-1.6h2.4c.47 0 .92.21 1.22.58l.88 1.02h4.28c.9 0 1.6.72 1.6 1.6v5.6c0 .88-.7 1.6-1.6 1.6H3.8c-.9 0-1.6-.72-1.6-1.6V4.4z" {...STROKE} />
      )}
    </svg>
  );
}

export function TreeFileGlyph({ name }: { name: string }): ReactNode {
  const glyphName = treeGlyphName(name);
  const glyph = TREE_BUILTIN_GLYPHS[glyphName] ?? TREE_BUILTIN_GLYPHS.default;
  return (
    <svg
      className="si-icon tree-file-icon"
      viewBox="0 0 16 16"
      width="16"
      height="16"
      aria-hidden="true"
      focusable="false"
      style={{ color: treeIconColor(glyphName) }}
      dangerouslySetInnerHTML={{ __html: glyph }}
    />
  );
}

export function FileIcon({ name, isDir, open }: { name: string; isDir: boolean; open?: boolean }): ReactNode {
  if (isDir) return <FolderIcon open={open} />;
  const glyph = glyphFor(name);
  const inner: ReactNode = {
    generic: <path d="M5.6 7.8h4.8M5.6 9.4h3.4" />,
    code: (
      <>
        <path d="M6.7 7.3L5 9l1.7 1.7" />
        <path d="M9.3 7.3L11 9l-1.7 1.7" />
        <path d="M8.5 6.9l-1 4.2" />
      </>
    ),
    json: (
      <>
        <path d="M7.1 6.7h-.6a.85.85 0 0 0-.85.85v.75c0 .55-.4 1-.95 1.05.55.05.95.5.95 1.05v.75c0 .47.38.85.85.85h.6" />
        <path d="M8.9 6.7h.6c.47 0 .85.38.85.85v.75c0 .55.4 1 .95 1.05-.55.05-.95.5-.95 1.05v.75a.85.85 0 0 1-.85.85h-.6" />
      </>
    ),
    markdown: <path d="M4.9 10.6V7.3l1.65 2 1.65-2v3.3" />,
    shell: (
      <>
        <path d="M5.5 7.4l1.7 1.7-1.7 1.7" />
        <path d="M8.6 10.8h2.3" />
      </>
    ),
    text: <path d="M5.5 7.5h5M5.5 9h3.5M5.5 10.5h5" />,
    config: (
      <>
        <path d="M4.8 8h6.4M4.8 10.6h6.4" />
        <circle cx="7.3" cy="8" r="1.05" />
        <circle cx="9.1" cy="10.6" r="1.05" />
      </>
    ),
    media: (
      <>
        <circle cx="6.5" cy="7.5" r="0.8" />
        <path d="M4.8 11.5l2.1-2.6 1.7 2.1 1.35-1.55 1.45 2.05" />
      </>
    ),
    binary: (
      <>
        <circle cx="6.2" cy="7.6" r="0.75" fill="currentColor" stroke="none" />
        <circle cx="9.3" cy="7.6" r="0.75" fill="currentColor" stroke="none" />
        <circle cx="6.2" cy="10.4" r="0.75" fill="currentColor" stroke="none" />
        <circle cx="9.3" cy="10.4" r="0.75" fill="currentColor" stroke="none" />
      </>
    )
  }[glyph];
  return (
    <svg className="si-icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
      <path d={SHEET} {...STROKE} />
      <path d={FOLD} {...FINE} />
      <Glyph>{inner}</Glyph>
    </svg>
  );
}

export function ChevronIcon({ open }: { open?: boolean }): ReactNode {
  return (
    <svg
      className={`si-chevron ${open ? "open" : ""}`}
      viewBox="0 0 10 10"
      width="10"
      height="10"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M2.4 3.6 l2.6 2.6 -2.6 2.6" {...STROKE} strokeWidth={1.6} />
    </svg>
  );
}

export function PlusIcon(): ReactNode {
  return (
    <svg className="si-mini" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" focusable="false">
      <path d="M8 3.6v8.8M3.6 8h8.8" {...STROKE} />
    </svg>
  );
}

export function FolderPlusIcon(): ReactNode {
  return (
    <svg className="si-mini" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" focusable="false">
      <path d="M2.2 4.4c0-.9.7-1.6 1.6-1.6h2.4c.47 0 .92.21 1.22.58l.88 1.02h4.28c.9 0 1.6.72 1.6 1.6v5.6c0 .88-.7 1.6-1.6 1.6H3.8c-.9 0-1.6-.72-1.6-1.6V4.4z" {...STROKE} />
      <path d="M5.7 8.9h4.6M8 6.6v4.6" {...STROKE} />
    </svg>
  );
}

export function FilePlusIcon(): ReactNode {
  return (
    <svg className="si-mini" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" focusable="false">
      <path d={SHEET} {...STROKE} />
      <path d={FOLD} {...FINE} />
      <path d="M8 9.2v3.4M6.3 10.9h3.4" {...FINE} />
    </svg>
  );
}

export function EllipsisIcon(): ReactNode {
  return (
    <svg className="si-mini" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" focusable="false">
      <circle cx="3.8" cy="8" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="8" cy="8" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="12.2" cy="8" r="1.1" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function PencilIcon(): ReactNode {
  return (
    <svg className="si-mini" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" focusable="false">
      <path d="M3.2 12.8l.7-2.9 6.7-6.7a1.53 1.53 0 0 1 2.2 2.2l-6.7 6.7-2.9.7z" {...STROKE} />
      <path d="M10 4.8l1.2 1.2" {...FINE} />
    </svg>
  );
}

export function TrashIcon(): ReactNode {
  return (
    <svg className="si-mini" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" focusable="false">
      <path d="M4.4 5h7.2M6.4 5V3.9c0-.5.4-.9.9-.9h1.4c.5 0 .9.4.9.9V5" {...STROKE} />
      <path d="M5.4 5l.5 6.6a1.3 1.3 0 0 0 1.3 1.2h1.6a1.3 1.3 0 0 0 1.3-1.2L10.6 5" {...STROKE} />
      <path d="M7 7.2v3.4M9 7.2v3.4" {...FINE} />
    </svg>
  );
}
