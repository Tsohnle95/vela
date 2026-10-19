import { useCallback, useEffect, useRef, type RefObject } from "react";
import type { BrowserTab } from "./use-browser-tabs";

export interface WebviewElement extends HTMLElement {
  getURL(): string;
  canGoBack(): boolean;
  canGoForward(): boolean;
  goBack(): void;
  goForward(): void;
  reload(): void;
  stop(): void;
}

export type BrowserPatch = Partial<Omit<BrowserTab, "id">>;
export interface BrowserGuest {
  hostRef: RefObject<HTMLDivElement | null>;
  navigate: (url: string) => void;
  goBack: () => void;
  goForward: () => void;
  reload: () => void;
  stop: () => void;
}

// Renderer validation gives a useful failure state; main remains the authority.
export function isBrowserTarget(value: string): boolean {
  try {
    const url = new URL(value);
    return !url.username && !url.password && (url.protocol === "http:" || url.protocol === "https:" || url.href === "about:blank");
  } catch {
    return false;
  }
}

function createBrowserView(url: string, onState: (patch: BrowserPatch) => void, onURL: (url: string) => void): WebviewElement {
  const view = document.createElement("webview") as WebviewElement;
  view.className = "browser-frame";
  view.setAttribute("partition", "vela-browser");
  let failed = false;
  const settle = (next?: string): void => {
    const current = next ?? view.getURL?.() ?? url;
    if (!failed) onURL(current);
    onState({ loading: false, ...(failed ? {} : { url: current }),
      canGoBack: view.canGoBack?.() ?? false, canGoForward: view.canGoForward?.() ?? false });
  };
  view.addEventListener("did-start-loading", () => {
    failed = false;
    onState({ loading: true, error: null });
  });
  view.addEventListener("did-stop-loading", () => settle());
  const onNavigate = (event: Event): void => {
    const detail = event as Event & { url?: string; isMainFrame?: boolean };
    if (detail.isMainFrame === false) return;
    failed = false;
    onState({ error: null });
    settle(detail.url);
  };
  view.addEventListener("did-navigate", onNavigate);
  view.addEventListener("did-navigate-in-page", onNavigate);
  view.addEventListener("page-title-updated", (event) => {
    onState({ title: (event as Event & { title?: string }).title ?? "" });
  });
  view.addEventListener("did-fail-load", (event) => {
    const detail = event as Event & { errorCode?: number; errorDescription?: string; isMainFrame?: boolean };
    if (detail.errorCode === -3 || detail.isMainFrame === false) return;
    failed = true;
    onState({ loading: false, error: detail.errorDescription ?? "This page did not load" });
  });
  view.setAttribute("src", url);
  return view;
}

// Empty tabs create no guest. The first navigation attaches a frame later, so
// the strip/placeholder can paint before Chromium performs guest startup.
export function useBrowserGuest(onState: (patch: BrowserPatch) => void, onURL: (url: string) => void, initialURL = ""): BrowserGuest {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<WebviewElement | null>(null);
  const pendingRef = useRef<string | null>(null);
  const frameRef = useRef<number | null>(null);
  const initialRef = useRef(initialURL);
  const requestedRef = useRef(initialURL);
  const stateRef = useRef(onState);
  const urlRef = useRef(onURL);
  stateRef.current = onState;
  urlRef.current = onURL;

  const navigate = useCallback((url: string): void => {
    if (!isBrowserTarget(url)) {
      stateRef.current({ loading: false, error: "Use a credential-free HTTP or HTTPS address." });
      return;
    }
    requestedRef.current = url;
    stateRef.current({ url, error: null, loading: true });
    urlRef.current(url);
    const view = viewRef.current;
    if (view) {
      view.setAttribute("src", url);
      return;
    }
    pendingRef.current = url;
    if (frameRef.current !== null) return;
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = null;
      const target = pendingRef.current;
      pendingRef.current = null;
      if (!target || !hostRef.current) return;
      const next = createBrowserView(target, (patch) => stateRef.current(patch), (current) => urlRef.current(current));
      viewRef.current = next;
      hostRef.current.append(next);
    });
  }, []);

  useEffect(() => {
    if (initialRef.current) navigate(initialRef.current);
    return () => {
      if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
      pendingRef.current = null;
      viewRef.current?.remove();
      viewRef.current = null;
    };
  }, [navigate]);

  const stop = (): void => {
    if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    frameRef.current = null;
    pendingRef.current = null;
    viewRef.current?.stop?.();
    stateRef.current({ loading: false });
  };
  return { hostRef, navigate, stop,
    goBack: () => viewRef.current?.goBack?.(), goForward: () => viewRef.current?.goForward?.(),
    reload: () => {
      if (viewRef.current) viewRef.current.reload?.();
      else if (requestedRef.current) navigate(requestedRef.current);
    } };
}
