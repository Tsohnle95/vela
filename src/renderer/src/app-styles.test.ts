import { afterEach, expect, it, vi } from "vitest";
import { reloadAppStylesheet } from "./app-styles";

const originalURL = "file:///vela/out/renderer/assets/index-built.css";
const url = "file:///vela/out/renderer/assets/vela-live.css";
afterEach(() => document.head.querySelectorAll("link").forEach(link => link.remove()));
function original() {
  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = originalURL;
  document.head.append(link);
  return link;
}
it("replaces the stylesheet only after successful load without touching the document", () => {
  const old = original();
  const report = vi.fn();
  reloadAppStylesheet({ originalURL, url, revision: 1 }, report);
  const next = document.head.querySelector<HTMLLinkElement>(`link[href*="revision=1"]`)!;
  expect(old.isConnected).toBe(true);
  next.dispatchEvent(new Event("load"));
  expect(old.isConnected).toBe(false);
  expect(next.isConnected).toBe(true);
  expect(report).not.toHaveBeenCalled();
});
it("retains the last valid stylesheet on load errors and ignores superseded loads", () => {
  const old = original();
  const report = vi.fn();
  reloadAppStylesheet({ originalURL, url, revision: 2 }, report);
  const stale = document.head.querySelector<HTMLLinkElement>(`link[href*="revision=2"]`)!;
  reloadAppStylesheet({ originalURL, url, revision: 3 }, report);
  const next = document.head.querySelector<HTMLLinkElement>(`link[href*="revision=3"]`)!;
  stale.dispatchEvent(new Event("load"));
  expect(stale.isConnected).toBe(false);
  expect(old.isConnected).toBe(true);
  next.dispatchEvent(new Event("error"));
  expect(next.isConnected).toBe(false);
  expect(old.isConnected).toBe(true);
  expect(report).toHaveBeenCalledOnce();
});
