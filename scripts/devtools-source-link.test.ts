import { EventEmitter } from "node:events";
import type { WebContents } from "electron";
import { expect, it, vi } from "vitest";
import { DevToolsController } from "../src/main/devtools-controller";

function fixture() {
  const devtools = {
    isDestroyed: () => false, isLoading: () => false,
    executeJavaScript: vi.fn(async (_script: string) => "")
  };
  const wc = Object.assign(new EventEmitter(), {
    debugger: new EventEmitter(), devToolsWebContents: devtools,
    isDevToolsOpened: () => false, isDestroyed: () => false,
    closeDevTools: () => wc.emit("devtools-closed")
  });
  new DevToolsController(async () => null).attach(wc as unknown as WebContents);
  return { wc, devtools };
}

  it("captures a real source-link click through a DevTools shadow root", () => {
    const f = fixture();
    f.wc.emit("devtools-opened");
    const script = f.devtools.executeJavaScript.mock.calls[0][0];
    const frame = document.createElement("iframe");
    document.body.append(frame);
    try {
      const frontend = frame.contentWindow!;
      new Function("window", script)(frontend);
      const host = frontend.document.createElement("div");
      frontend.document.body.append(host);
      const shadow = host.attachShadow({ mode: "open" });
      const link = frontend.document.createElement("button");
      link.title = "file:///app/src/renderer/src/styles/_sidebar.scss:771";
      shadow.append(link);
      const click = new MouseEvent("click", { bubbles: true, composed: true, cancelable: true });
      link.dispatchEvent(click);
      expect(click.defaultPrevented).toBe(true);
      expect(JSON.parse((frontend as unknown as { __velaOpenSource: string }).__velaOpenSource)).toEqual({
        file: "file:///app/src/renderer/src/styles/_sidebar.scss", line: 771, title: link.title
      });
    } finally {
      frame.remove();
      f.wc.closeDevTools();
    }
  });

