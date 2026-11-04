import { session, shell, type WebContents } from "electron";
import { safeExternalUrl } from "@shared/url-policy";
import { BROWSER_PARTITION, hardenBrowserPreferences, isAllowedBrowserUrl } from "./browser-policy";

export function attachBrowserGuards(contents: WebContents): void {
  // Configure the dedicated session before any guest can attach or request a device.
  const browserSession = session.fromPartition(BROWSER_PARTITION);
  browserSession.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
  browserSession.setPermissionCheckHandler(() => false);
  browserSession.setDevicePermissionHandler(() => false);

  contents.on("will-attach-webview", (event, preferences, params) => {
    if (!hardenBrowserPreferences(preferences, params.src)) {
      event.preventDefault();
      return;
    }
    params.partition = BROWSER_PARTITION;
  });
  contents.on("did-attach-webview", (_event, guest) => {
    guest.setWindowOpenHandler(({ url }) => {
      const external = safeExternalUrl(url);
      if (external) void shell.openExternal(external);
      return { action: "deny" };
    });
    guest.on("will-navigate", (event, url) => {
      if (!isAllowedBrowserUrl(url)) event.preventDefault();
    });
    // will-frame-navigate also protects frame navigation; redirects retain the
    // existing Electron positional URL signature used by the application guard.
    guest.on("will-frame-navigate", (event) => {
      if (!isAllowedBrowserUrl(event.url)) event.preventDefault();
    });
    guest.on("will-redirect", (event, url) => {
      if (!isAllowedBrowserUrl(url)) event.preventDefault();
    });
  });
}
