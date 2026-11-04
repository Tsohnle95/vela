import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BrowserTabsProvider, useBrowserTabActions, useBrowserTabState } from "./browser-tabs-context";

// The split exists for responsiveness: opening or closing a tab must not
// re-render the shell. This guards the contract that made it so.
describe("browser tab context split", () => {
  let root: Root;
  let container: HTMLDivElement;
  let stateRenders = 0;
  let actionRenders = 0;
  let actions: ReturnType<typeof useBrowserTabActions>;
  let state: ReturnType<typeof useBrowserTabState>;

  function StateProbe(): null {
    state = useBrowserTabState();
    stateRenders += 1;
    return null;
  }

  function ActionProbe(): null {
    actions = useBrowserTabActions();
    actionRenders += 1;
    return null;
  }

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    stateRenders = 0;
    actionRenders = 0;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  it("re-renders tab state consumers but leaves action consumers alone", async () => {
    await act(async () => root.render(
      <BrowserTabsProvider>
        <StateProbe />
        <ActionProbe />
      </BrowserTabsProvider>
    ));
    expect(stateRenders).toBe(1);
    expect(actionRenders).toBe(1);

    await act(async () => { actions.openBrowserTab(); });
    expect(state.browserTabs).toHaveLength(1);
    expect(stateRenders).toBe(2);
    expect(actionRenders).toBe(1);

    await act(async () => { actions.closeBrowserTab(state.browserTabs[0]!.id); });
    expect(state.browserTabs).toHaveLength(0);
    expect(stateRenders).toBe(3);
    expect(actionRenders).toBe(1);
  });

  it("keeps the action object identical across state changes", async () => {
    await act(async () => root.render(
      <BrowserTabsProvider>
        <StateProbe />
        <ActionProbe />
      </BrowserTabsProvider>
    ));
    const first = actions;
    await act(async () => { actions.openBrowserTab("https://example.com"); });
    expect(actions).toBe(first);
  });
});
