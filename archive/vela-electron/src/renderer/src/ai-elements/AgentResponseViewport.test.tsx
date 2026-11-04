import { act } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import { AgentStreamStatus } from "./AgentResponseViewport";
beforeEach(() => vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true));
afterEach(() => vi.unstubAllGlobals());
vi.mock("./AiElementsSurface", () => ({ AiElementsSurface: ({ children }: { children: React.ReactNode }) => <div>{children}</div> }));
it("releases the activity row when a completed conversation becomes idle", () => {
  const host = document.createElement("div");
  const root = createRoot(host);
  try {
    act(() => root.render(<AgentStreamStatus legacy={false} visible busy={false} transcript={[]} />));
    expect(host.querySelector("[data-agent-stream-status]")).toBeNull();
    act(() => root.render(<AgentStreamStatus legacy={false} visible busy transcript={[]} />));
    expect(host.querySelector('[role="status"]')).not.toBeNull();
    act(() => root.render(<AgentStreamStatus legacy={false} visible busy={false} transcript={[]} activity={{ statusText: "waiting", isWaitingForPermission: true, retryInfo: null, activePartType: undefined }} />));
    expect(host.querySelector('[role="status"]')?.textContent).toContain("Waiting for approval");
    act(() => root.render(<AgentStreamStatus legacy={false} visible busy={false} transcript={[]} />));
    expect(host.innerHTML).toBe("");
  } finally { act(() => root.unmount()); }
});
