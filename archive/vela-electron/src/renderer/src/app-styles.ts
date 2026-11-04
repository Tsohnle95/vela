import { useEffect } from "react";
import { useStore } from "./store";
import type { AppStylesheetUpdate } from "../../shared/app-styles";

let request = 0;

export function reloadAppStylesheet(update: AppStylesheetUpdate, onError: (message: string) => void): void {
  const next = new URL(update.url);
  next.searchParams.set("revision", String(update.revision));
  const links = [...document.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]')];
  if (links.some(link => link.href === next.href)) return;
  const previous = links.filter(link => {
    const url = new URL(link.href);
    url.search = "";
    return url.href === update.originalURL || url.href === update.url;
  });
  if (!previous.length) return;
  const token = ++request;
  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = next.href;
  link.onload = () => {
    if (token !== request) { link.remove(); return; }
    for (const old of previous) old.remove();
  };
  link.onerror = () => {
    link.remove();
    if (token === request) onError("Could not load updated app styles; the previous styles are still active.");
  };
  document.head.append(link);
}

export function useAppStyles(): void {
  const { notify } = useStore();
  useEffect(() => window.vela.onMessage(message => {
    if (message.kind !== "ui-command") return;
    if (message.command === "app-styles-error" && typeof message.data === "string") {
      notify(message.data, "error");
    } else if (message.command === "reload-app-styles") {
      const update = message.data as AppStylesheetUpdate | undefined;
      if (update && typeof update.url === "string" && typeof update.originalURL === "string" && typeof update.revision === "number") {
        reloadAppStylesheet(update, text => notify(text, "error"));
      }
    }
  }), [notify]);
}
