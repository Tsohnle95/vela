import type { Event, Input, WebContents } from "electron";
import { InspectPickerState } from "./inspect-picker";

const DEVTOOLS_WATCHER = `
(() => {
  if (window.__velaWatchInstalled) return;
  window.__velaWatchInstalled = true;
  window.__velaKey = null;
  window.__velaOpenSource = null;
  window.addEventListener("keydown", (event) => {
    if (event.key === "F12" || event.key === "Escape") {
      window.__velaKey = event.key;
    }
  }, true);
  const STRICT = /^(.*\\.(?:css|scss|sass|less|styl|stylus|pcss)):(\\d+)\\s*$/i;
  const LOOSE = /([^\\s]*\\.(?:css|scss|sass|less|styl|stylus|pcss)):(\\d+)/i;
  function attr(el, name) {
    return el && el.getAttribute ? el.getAttribute(name) : null;
  }
  function candidate(el) {
    if (!el || el.nodeType !== 1) return null;
    const sources = [attr(el, "title"), attr(el, "href"), attr(el, "data-url"), attr(el, "data-source-url")];
    for (const value of sources) {
      if (!value) continue;
      const clean = value.replace(/[?#].*$/, "");
      let m = STRICT.exec(clean.trim());
      if (m) return { file: m[1], line: parseInt(m[2], 10), title: value };
      m = LOOSE.exec(clean);
      if (m) return { file: m[1], line: parseInt(m[2], 10), title: value };
    }
    return null;
  }
  window.addEventListener("click", (event) => {
    const path = event.composedPath ? event.composedPath() : [];
    let found = null;
    for (const el of path) {
      const c = candidate(el);
      if (c) { found = c; break; }
    }
    if (!found) return;
    event.preventDefault();
    event.stopPropagation();
    window.__velaOpenSource = JSON.stringify({ file: found.file, line: found.line, title: found.title });
  }, true);
})();
`;

const INSPECT_HIGHLIGHT = {
  showInfo: true,
  showStyles: true,
  contentColor: { r: 111, g: 168, b: 220, a: 0.66 },
  paddingColor: { r: 147, g: 196, b: 125, a: 0.55 },
  borderColor: { r: 255, g: 229, b: 153, a: 0.66 },
  marginColor: { r: 246, g: 178, b: 107, a: 0.66 }
} as const;

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export class DevToolsController {
  private readonly inspectPicker = new InspectPickerState();
  private overlayEnabled = false;
  private devToolsKeyPolling = false;

  constructor(private readonly resolveSource: (file: string, title: string) => Promise<string | null>) {}

  cancel(): void {
    this.inspectPicker.cancel();
  }

  attach(wc: WebContents): void {
    wc.debugger.on("message", (_e, method, params) => {
      if (method === "Overlay.inspectNodeRequested") {
        const backendNodeId = (params as { backendNodeId?: number }).backendNodeId;
        if (typeof backendNodeId === "number" && this.inspectPicker.claim()) {
          void this.disableInspectMode(wc)
            .then(() => this.selectPickedNode(wc, backendNodeId))
            .catch((err) => console.error("selectPickedNode:", err));
        }
      } else if (method === "Overlay.inspectModeCanceled") {
        this.inspectPicker.cancel();
      }
    });
    wc.debugger.on("detach", () => {
      this.inspectPicker.cancel();
      this.overlayEnabled = false;
    });
    wc.on("devtools-closed", () => {
      this.devToolsKeyPolling = false;
      this.stopInspectPicker(wc);
    });

    const installDevToolsWatcher = (attempt = 0): void => {
      const dtc = wc.devToolsWebContents;
      if (!dtc || dtc.isDestroyed()) return;
      const run = (): void => {
        void dtc.executeJavaScript(DEVTOOLS_WATCHER).catch(() => {
          if (attempt < 5) setTimeout(() => installDevToolsWatcher(attempt + 1), 250);
        });
      };
      if (dtc.isLoading()) dtc.once("dom-ready", run);
      else run();
    };

    wc.on("devtools-opened", () => {
      installDevToolsWatcher();
      if (!this.devToolsKeyPolling) {
        this.devToolsKeyPolling = true;
        void this.pollDevTools(wc);
      }
    });
  }

  handleInput(wc: WebContents, event: Pick<Event, "preventDefault">, input: Input): void {
    const mod = process.platform === "darwin" ? input.meta : input.control;
    if (input.type === "keyDown" && !mod && !input.alt && !input.shift && input.key === "F12") {
      event.preventDefault();
      if (wc.isDevToolsOpened()) {
        wc.closeDevTools();
      } else {
        wc.openDevTools({ mode: "bottom" });
      }
      return;
    }
    if (input.type === "keyDown" && !mod && !input.alt && !input.shift && input.key === "Escape" && this.inspectPicker.active) {
      event.preventDefault();
      this.stopInspectPicker(wc);
      return;
    }
    if (input.type === "keyDown" && !input.isAutoRepeat && mod && input.shift && !input.alt && input.key.toLowerCase() === "c") {
      event.preventDefault();
      if (this.inspectPicker.active) {
        this.stopInspectPicker(wc);
      } else {
        void this.startInspectPicker(wc);
      }
    }
  }

  private async pollDevTools(wc: WebContents): Promise<void> {
    while (this.devToolsKeyPolling) {
      await sleep(120);
      const dtc = wc.devToolsWebContents;
      if (!wc.isDevToolsOpened() || wc.isDestroyed() || !dtc || dtc.isDestroyed()) break;
      void dtc.executeJavaScript(DEVTOOLS_WATCHER).catch(() => {});
      const payload = await dtc
        .executeJavaScript(`(() => {
          const k = window.__velaKey;
          window.__velaKey = null;
          const s = window.__velaOpenSource;
          window.__velaOpenSource = null;
          return JSON.stringify({ key: k, source: s });
        })()`)
        .catch(() => null);
      if (typeof payload !== "string") continue;
      let parsed: { key?: string | null; source?: string | null };
      try {
        parsed = JSON.parse(payload);
      } catch {
        continue;
      }
      if (parsed.key === "F12") {
        wc.closeDevTools();
      } else if (parsed.key === "Escape" && this.inspectPicker.active) {
        this.stopInspectPicker(wc);
      }
      if (typeof parsed.source === "string") {
        try {
          const src = JSON.parse(parsed.source) as { file?: string; line?: number; title?: string };
          if (typeof src.file === "string" && typeof src.line === "number") {
            const resolved = await this.resolveSource(src.file, src.title ?? "");
            if (resolved) {
              wc.send("shell:message", {
                kind: "ui-command",
                command: "open-source",
                path: resolved,
                line: src.line
              });
            } else {
              console.warn("open-source: no local file for", src.file, src.title ?? "");
            }
          }
        } catch (err) {
          console.error("open-source:", err);
        }
      }
    }
    this.devToolsKeyPolling = false;
  }

  private async ensureDevToolsOpen(wc: WebContents): Promise<void> {
    if (!wc.isDevToolsOpened()) wc.openDevTools({ mode: "bottom" });
    for (let i = 0; i < 50 && !wc.devToolsWebContents; i++) await sleep(100);
    const dtc = wc.devToolsWebContents;
    if (dtc && dtc.isLoading()) {
      await new Promise<void>((resolve) => dtc.once("dom-ready", () => resolve()));
    }
  }

  private async startInspectPicker(wc: WebContents): Promise<void> {
    const token = this.inspectPicker.begin();
    await this.ensureDevToolsOpen(wc);
    if (!this.inspectPicker.isCurrent(token)) return;
    wc.focus();
    try {
      if (!wc.debugger.isAttached()) wc.debugger.attach("1.3");
      await wc.debugger.sendCommand("DOM.enable");
      if (!this.inspectPicker.isCurrent(token)) return;
      await wc.debugger.sendCommand("Overlay.enable");
      if (!this.inspectPicker.isCurrent(token)) return;
      this.overlayEnabled = true;
      await wc.debugger.sendCommand("Overlay.setInspectMode", {
        mode: "searchForNode",
        highlightConfig: INSPECT_HIGHLIGHT
      });
      if (!this.inspectPicker.isCurrent(token)) await this.disableInspectMode(wc);
    } catch (err) {
      console.error("inspect picker failed:", err);
      if (this.inspectPicker.isCurrent(token)) this.inspectPicker.cancel();
    }
  }

  private async selectPickedNode(wc: WebContents, backendNodeId: number): Promise<void> {
    try {
      const { model } = await wc.debugger.sendCommand("DOM.getBoxModel", { backendNodeId });
      const q = model.content;
      const x = Math.round((Math.min(q[0], q[2], q[4], q[6]) + Math.max(q[0], q[2], q[4], q[6])) / 2);
      const y = Math.round((Math.min(q[1], q[3], q[5], q[7]) + Math.max(q[1], q[3], q[5], q[7])) / 2);
      wc.inspectElement(x, y);
    } catch (err) {
      console.error("selectPickedNode:", err);
    }
  }

  private async disableInspectMode(wc: WebContents): Promise<void> {
    if (!this.overlayEnabled || !wc.debugger.isAttached()) return;
    await wc.debugger.sendCommand("Overlay.setInspectMode", { mode: "none", highlightConfig: INSPECT_HIGHLIGHT });
  }

  private stopInspectPicker(wc: WebContents): void {
    this.inspectPicker.cancel();
    void this.disableInspectMode(wc).catch((err) => console.error("stopInspectPicker:", err));
  }
}
