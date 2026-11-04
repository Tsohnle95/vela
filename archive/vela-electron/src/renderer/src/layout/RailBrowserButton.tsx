import type { ReactNode } from "react";

// The rail's browser action: opens a browser tab in the editor strip. It lives
// here rather than in App to keep the shell module inside its size budget.
export function RailBrowserButton({ onOpen }: { onOpen: () => void }): ReactNode {
  return (
    <button
      className="activity-tool"
      data-panel-action="new-browser-tab"
      title="Open a browser tab"
      aria-label="Open a browser tab"
      onClick={onOpen}
    >
      <i className="codicon codicon-globe" aria-hidden="true" />
    </button>
  );
}
