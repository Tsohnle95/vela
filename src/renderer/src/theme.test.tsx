import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider, useTheme } from "./theme";

// The appearance supplies colours; the shell decides the chrome. They are
// independent, and both must survive a remount.
describe("theme and UI shell", () => {
  let root: Root;
  let container: HTMLDivElement;
  let ctx: ReturnType<typeof useTheme>;

  function Probe(): null {
    ctx = useTheme();
    return null;
  }
  const render = async (): Promise<void> => act(async () => root.render(<ThemeProvider><Probe /></ThemeProvider>));

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.removeAttribute("data-shell");
    document.documentElement.removeAttribute("style");
    vi.unstubAllGlobals();
  });

  it("defaults to the Codex shell and records it on the document", async () => {
    await render();

    expect(ctx.uiShell).toBe("codex");
    expect(document.documentElement.dataset.shell).toBe("codex");
    expect(document.documentElement.dataset.theme).toBe("codex");
    expect(window.localStorage.getItem("mira.ui")).toBe("codex");
  });

  it("switches shells without touching the palette, and restores both", async () => {
    await render();
    await act(async () => { ctx.setTheme("prism"); });
    await act(async () => { ctx.setUiShell("classic"); });

    expect(document.documentElement.dataset.theme).toBe("prism");
    expect(document.documentElement.dataset.shell).toBe("classic");

    await act(async () => root.unmount());
    root = createRoot(container);
    await render();
    expect(ctx.theme).toBe("prism");
    expect(ctx.uiShell).toBe("classic");
  });

  it("layers the shell's tokens over any palette", async () => {
    await render();
    await act(async () => { ctx.setTheme("prism"); });

    // Codex shell: flat workspace, hairline composer border.
    expect(document.documentElement.style.getPropertyValue("--workspace-background")).toBe("#151423");
    expect(document.documentElement.style.getPropertyValue("--agent-composer-border")).not.toBe("");

    await act(async () => { ctx.setUiShell("classic"); });
    // Classic shell: the palette's own workspace wash, no shell override.
    expect(document.documentElement.style.getPropertyValue("--workspace-background")).toContain("radial-gradient");
  });

  it("gives dark palettes the dark file-tree glyphs in the Codex shell", async () => {
    await render();
    await act(async () => { ctx.setTheme("original"); });
    expect(document.documentElement.style.getPropertyValue("--trees-icon-gray")).toBe("#adadb1");

    await act(async () => { ctx.setTheme("papertrail"); });
    expect(document.documentElement.style.getPropertyValue("--trees-icon-gray")).toBe("");
  });

  it("persists Observatory and clears its glyph and composer overrides when switching", async () => {
    await render();
    await act(async () => { ctx.setTheme("observatory"); });
    expect(document.documentElement.dataset.theme).toBe("observatory");
    expect(document.documentElement.style.getPropertyValue("--editor-surface")).toBe("#060912");
    expect(document.documentElement.style.getPropertyValue("--trees-icon-indigo")).toBe("#aa83ff");
    expect(document.documentElement.style.getPropertyValue("--agent-composer-border")).toBe("#8d70df");
    await act(async () => { ctx.setUiShell("classic"); });
    expect(document.documentElement.style.getPropertyValue("--trees-icon-indigo")).toBe("#aa83ff");
    await act(async () => root.unmount());
    root = createRoot(container);
    await render();
    expect(ctx.theme).toBe("observatory");
    expect(ctx.uiShell).toBe("classic");
    await act(async () => { ctx.setTheme("papertrail"); });
    expect(document.documentElement.style.getPropertyValue("--trees-icon-indigo")).toBe("");
    expect(document.documentElement.style.getPropertyValue("--agent-composer-border")).not.toBe("#8d70df");
  });
});
