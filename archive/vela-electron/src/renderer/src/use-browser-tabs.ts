import { useCallback, useRef, useState } from "react";

// One browser tab per in-app web page. The address field keeps its own draft in
// the pane; this slice only tracks committed state so the strip and pane can
// render titles, loading and history without talking to the guest directly.
export interface BrowserTab {
  id: string;
  url: string;
  title: string;
  loading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  error: string | null;
}

export type BrowserTabActions = Pick<
  BrowserTabs,
  "openBrowserTab" | "closeBrowserTab" | "activateBrowserTab" | "updateBrowserTab"
>;

export interface BrowserTabs {
  browserTabs: BrowserTab[];
  activeBrowserTabID: string | null;
  openBrowserTab: (url?: string) => string;
  closeBrowserTab: (id: string) => void;
  activateBrowserTab: (id: string | null) => void;
  updateBrowserTab: (id: string, patch: Partial<Omit<BrowserTab, "id">>) => void;
}

export function browserTabLabel(tab: BrowserTab): string {
  if (tab.title.trim().length > 0) return tab.title;
  if (tab.url.length === 0) return "New tab";
  try {
    return new URL(tab.url).hostname.replace(/^www\./, "");
  } catch {
    return tab.url;
  }
}

function sameBrowserTab(left: BrowserTab, patch: Partial<Omit<BrowserTab, "id">>): boolean {
  return Object.entries(patch).every(([key, value]) => left[key as keyof BrowserTab] === value);
}

interface BrowserState {
  browserTabs: BrowserTab[];
  activeBrowserTabID: string | null;
}

export function useBrowserTabs(): BrowserTabs {
  const [state, setState] = useState<BrowserState>({ browserTabs: [], activeBrowserTabID: null });
  const seqRef = useRef(0);

  const openBrowserTab = useCallback((url = ""): string => {
    const id = `browser-${++seqRef.current}`;
    const tab: BrowserTab = { id, url, title: "", loading: url.length > 0,
      canGoBack: false, canGoForward: false, error: null };
    setState((current) => ({ browserTabs: [...current.browserTabs, tab], activeBrowserTabID: id }));
    return id;
  }, []);

  const closeBrowserTab = useCallback((id: string): void => {
    setState((current) => {
      const index = current.browserTabs.findIndex((tab) => tab.id === id);
      if (index < 0) return current;
      const next = current.browserTabs.filter((tab) => tab.id !== id);
      return { browserTabs: next, activeBrowserTabID: current.activeBrowserTabID !== id
        ? current.activeBrowserTabID : next[Math.min(index, next.length - 1)]?.id ?? null };
    });
  }, []);

  const activateBrowserTab = useCallback((id: string | null): void => {
    setState((current) => {
      if (id === current.activeBrowserTabID || (id !== null && !current.browserTabs.some((tab) => tab.id === id))) return current;
      return { ...current, activeBrowserTabID: id };
    });
  }, []);

  const updateBrowserTab = useCallback((id: string, patch: Partial<Omit<BrowserTab, "id">>): void => {
    setState((current) => {
      const index = current.browserTabs.findIndex((tab) => tab.id === id);
      if (index < 0 || sameBrowserTab(current.browserTabs[index]!, patch)) return current;
      const next = [...current.browserTabs];
      next[index] = { ...current.browserTabs[index]!, ...patch };
      return { ...current, browserTabs: next };
    });
  }, []);

  return { ...state, openBrowserTab, closeBrowserTab, activateBrowserTab, updateBrowserTab };
}
