import { createContext, useContext, useMemo, type ReactNode } from "react";
import { useBrowserTabs, type BrowserTab, type BrowserTabActions } from "./use-browser-tabs";

// Browser tabs live in their own context, split in two: tab *state* changes on
// every open/close/navigation, so it must only reach the strip and the panes;
// the *actions* are stable and are what the app shell (rail, editor file
// activation, store wiring) consumes. Keeping both out of the app-wide store is
// what makes opening and closing a tab instant — a change there used to
// re-render every store consumer in the app.
interface BrowserTabState {
  browserTabs: BrowserTab[];
  activeBrowserTabID: string | null;
}

const BrowserTabStateContext = createContext<BrowserTabState | null>(null);
const BrowserTabActionsContext = createContext<BrowserTabActions | null>(null);

export function BrowserTabsProvider({ children }: { children: ReactNode }): ReactNode {
  const tabs = useBrowserTabs();
  const { openBrowserTab, closeBrowserTab, activateBrowserTab, updateBrowserTab } = tabs;
  const actions = useMemo<BrowserTabActions>(
    () => ({ openBrowserTab, closeBrowserTab, activateBrowserTab, updateBrowserTab }),
    [openBrowserTab, closeBrowserTab, activateBrowserTab, updateBrowserTab]
  );
  const state = useMemo<BrowserTabState>(
    () => ({ browserTabs: tabs.browserTabs, activeBrowserTabID: tabs.activeBrowserTabID }),
    [tabs.browserTabs, tabs.activeBrowserTabID]
  );
  return (
    <BrowserTabActionsContext.Provider value={actions}>
      <BrowserTabStateContext.Provider value={state}>{children}</BrowserTabStateContext.Provider>
    </BrowserTabActionsContext.Provider>
  );
}

export function useBrowserTabState(): BrowserTabState {
  const value = useContext(BrowserTabStateContext);
  if (!value) throw new Error("useBrowserTabState requires BrowserTabsProvider");
  return value;
}

export function useBrowserTabActions(): BrowserTabActions {
  const value = useContext(BrowserTabActionsContext);
  if (!value) throw new Error("useBrowserTabActions requires BrowserTabsProvider");
  return value;
}
