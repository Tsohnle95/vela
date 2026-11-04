import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { SidebarBrand } from "./SidebarBrand";
import { Titlebar } from "./Titlebar";

it("places the Observatory wordmark at the sidebar brand instead of the titlebar", () => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  const container = document.createElement("div");
  const root = createRoot(container);
  const noop = (): void => {};
  act(() => root.render(<><SidebarBrand /><Titlebar sidebarOpen agentMode={false} agentPanelOpen onToggleSidebar={noop} onToggleAgentMode={noop} onToggleAgentPanel={noop} onAddAgentPanel={noop} canAddAgentPanel /></>));
  expect(container.querySelector(".sidebar-brand .observatory-wordmark")?.getAttribute("alt")).toBe("Vela");
  expect(container.querySelector(".titlebar .observatory-wordmark")).toBeNull();
  act(() => root.unmount());
  vi.unstubAllGlobals();
});
