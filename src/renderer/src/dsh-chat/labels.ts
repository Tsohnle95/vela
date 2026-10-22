import type {
  DiffBlockLabels,
  MarkdownLabels,
  TerminalBlockLabels
} from "@deepseek-ai/dsh-client-ui-primitives";

// DeepSeek Harness' Markdown chrome copy is memoized per locale. Keeping one
// module-level object preserves the streaming renderer's frozen blocks.
export const MARKDOWN_LABELS: MarkdownLabels = {
  code: {
    copyLabel: "Copy",
    copiedLabel: "Copied",
    toolbarLabels: {
      codeLabel: "Code",
      wrapLabel: "Wrap lines",
      unwrapLabel: "Unwrap lines"
    }
  },
  footnotes: "Footnotes"
};

export const TERMINAL_LABELS: TerminalBlockLabels = {
  commandLine: (line) => `Command line ${line}`,
  signal: (signal) => signal,
  exitCode: (exitCode) => `exit ${exitCode}`,
  noExitCode: "no exit code",
  running: "Running",
  failed: "Failed",
  done: "Completed",
  copy: "Copy",
  copied: "Copied",
  noOutput: "No output",
  collapseAria: "Collapse output",
  collapse: "Collapse",
  expandAria: (hidden) => `Expand ${hidden} hidden lines`,
  expand: (hidden) => `Show ${hidden} more lines`
};

export const DIFF_LABELS: DiffBlockLabels = {
  codeLabel: "Diff",
  wrapLabel: "Wrap lines",
  unwrapLabel: "Unwrap lines",
  copy: "Copy",
  copied: "Copied",
  collapseAria: "Collapse diff",
  expandAria: (hidden) => `Expand ${hidden} hidden lines`,
  collapse: "Collapse",
  expand: (hidden) => `Show ${hidden} more lines`
};
