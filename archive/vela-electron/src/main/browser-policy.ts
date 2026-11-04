import type { WebPreferences } from "electron";

export const BROWSER_PARTITION = "vela-browser";

export function isAllowedBrowserUrl(value: string): boolean {
  try {
    const url = new URL(value);
    if (url.username || url.password) return false;
    return url.protocol === "http:" || url.protocol === "https:" || url.href === "about:blank";
  } catch {
    return false;
  }
}

export function hardenBrowserPreferences(preferences: WebPreferences, src: string): boolean {
  if (!isAllowedBrowserUrl(src)) return false;
  delete preferences.preload;
  // Older Electron versions and forged attachment preferences may carry this alias.
  delete (preferences as WebPreferences & { preloadURL?: string }).preloadURL;
  preferences.nodeIntegration = false;
  preferences.nodeIntegrationInSubFrames = false;
  preferences.nodeIntegrationInWorker = false;
  preferences.webviewTag = false;
  preferences.contextIsolation = true;
  preferences.sandbox = true;
  preferences.webSecurity = true;
  preferences.allowRunningInsecureContent = false;
  preferences.partition = BROWSER_PARTITION;
  delete preferences.session;
  return true;
}
