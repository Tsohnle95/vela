import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, TranscriptItem } from "@shared/types";
import { OpenCodeTimeline } from "./OpenCodeTimeline";

const storeState = vi.hoisted(() => ({
  agents: [] as { id: string; name: string }[],
  sessions: [] as unknown[],
  session: null as SessionInfo | null,
  reopenSession: vi.fn(),
  openFile: vi.fn(),
  focusSession: vi.fn(),
  replyPermission: vi.fn()
}));

vi.mock("../store", () => ({
  useStore: () => storeState
}));

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const PATCH = [
  "Index: src/renderer/src/styles/_foundation.scss",
  "===================================================================",
  "--- src/renderer/src/styles/_foundation.scss",
  "+++ src/renderer/src/styles/_foundation.scss",
  "@@ -3,4 +3,5 @@",
  "   --bg-panel: rgba(27, 25, 21, 0.82);",
  "-  --sidebar-surface-color: rgba(23, 23, 27, 0.72);",
  "+  --panel-surface-color: rgba(23, 23, 27, 0.72);",
  "+  --panel-aura-x: 50%;",
  " ",
  "   --text: #ece7dc;"
].join("\n") + "\n";

const editTool = (
  id: string,
  metadata: Record<string, unknown> | undefined,
  input: Record<string, unknown> = { path: "src/renderer/src/styles/_foundation.scss" }
): TranscriptItem => ({
  kind: "assistant",
  id,
  messageID: id,
  completed: true,
  parts: [{
    kind: "tool",
    id: `${id}:tool`,
    tool: {
      id: `${id}:tool`,
      title: "edit",
      detail: "",
      status: "success",
      input: JSON.stringify(input),
      inputValue: input,
      ...(metadata ? { metadata } : {})
    }
  }]
});

describe("edit tool diff cards", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    const report = console.error;
    vi.spyOn(console, "error").mockImplementation((...args: unknown[]) => {
      if (String(args[0]).includes("Could not parse CSS stylesheet")) return;
      report(...args);
    });
    vi.stubGlobal("Worker", undefined);
    vi.stubGlobal("ResizeObserver", class { observe(): void {} unobserve(): void {} disconnect(): void {} });
    Object.defineProperty(CSSStyleSheet.prototype, "replaceSync", { configurable: true, value: vi.fn() });
    storeState.agents = [];
    storeState.sessions = [];
    storeState.session = null;
    storeState.reopenSession.mockReset();
    storeState.openFile.mockReset();
    storeState.focusSession.mockReset();
    storeState.replyPermission.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("renders the full path and addition/deletion stats from metadata.files", () => {
    act(() => root.render(
      <OpenCodeTimeline
        transcript={[editTool("assistant-1", {
          files: [{ file: "src/renderer/src/styles/_foundation.scss", patch: PATCH, status: "modified", additions: 2, deletions: 1 }]
        })]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const card = container.querySelector("[data-component='edit-tool']");
    expect(card).not.toBeNull();
    expect(card?.querySelector("[data-slot='message-part-title-filename']")?.textContent).toBe("_foundation.scss");
    expect(card?.querySelector("[data-slot='message-part-directory']")?.textContent).toBe("src/renderer/src/styles/");
    expect(card?.querySelector("[data-slot='diff-changes-additions']")?.textContent).toBe("+2");
    expect(card?.querySelector("[data-slot='diff-changes-deletions']")?.textContent).toBe("-1");
  });

  it.each([
    ["/repo/src/app.ts", "/src/"],
    ["src/app.ts", "src/"]
  ])("uses the input directory and the metadata accordion path %s", (file, directory) => {
    const session = { id: "session-1", directory: "/repo", workspace: { id: "workspace-1", generation: 1 } } as SessionInfo;
    act(() => root.render(<OpenCodeTimeline
      transcript={[editTool("assistant-1", {
        filediff: { file, before: "old", after: "new", additions: 1, deletions: 1 }
      }, { filePath: "/repo/src/app.ts" })]}
      busy={false} lastAssistantId={null} session={session}
    />));
    expect(container.querySelector("[data-slot='message-part-directory']")?.textContent).toBe("/src/");
    act(() => (container.querySelector("[data-slot='collapsible-trigger']") as HTMLButtonElement).click());
    expect(container.querySelector("[data-slot='apply-patch-directory']")?.textContent).toBe(`\u202a${directory}\u202c`);
  });

  it("expands into the real unified file diff without patch headers", async () => {
    act(() => root.render(
      <OpenCodeTimeline
        transcript={[editTool("assistant-1", {
          files: [{ file: "src/app.ts", patch: PATCH, status: "modified", additions: 2, deletions: 1 }]
        })]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const trigger = container.querySelector("[data-slot='collapsible-trigger']") as HTMLButtonElement | null;
    expect(trigger?.disabled).toBe(false);
    expect(document.querySelector("[data-component='patch-diff']")).toBeNull();

    act(() => trigger?.click());

    const shadow = container.querySelector("diffs-container")?.shadowRoot;
    await vi.waitFor(() => expect(shadow?.querySelectorAll('[data-line][data-line-type="change-addition"]')).toHaveLength(2));
    expect(shadow?.querySelectorAll('[data-line][data-line-type="change-deletion"]')).toHaveLength(1);
    expect(shadow?.textContent).not.toContain("Index:");
    expect(shadow?.querySelector('[data-line][data-line-type="change-deletion"]')?.textContent).toContain("--sidebar-surface-color");
  });

  it("opens the edited file in the editor pane from the path subtitle", () => {
    storeState.session = { id: "session-1", directory: "/repo", workspace: { id: "workspace-1", generation: 1 } };
    act(() => root.render(
      <OpenCodeTimeline
        transcript={[editTool("assistant-1", {
          files: [{ file: "src/app.ts", patch: PATCH, additions: 1, deletions: 0 }]
        })]}
        busy={false}
        lastAssistantId={null}
        session={storeState.session}
      />
    ));

    const subtitle = container.querySelector("[data-slot='message-part-title-filename']") as HTMLElement | null;
    act(() => subtitle?.click());
    expect(storeState.focusSession).toHaveBeenCalledWith("session-1");
    expect(storeState.openFile).toHaveBeenCalledWith("src/app.ts", storeState.session.workspace);
  });

  it("renders one section per file when a part edits several files", () => {
    act(() => root.render(
      <OpenCodeTimeline
        transcript={[editTool("assistant-1", {
          files: [
            { file: "src/a.ts", patch: PATCH, status: "modified", additions: 2, deletions: 1 },
            { file: "src/b.ts", patch: PATCH, status: "added", additions: 2, deletions: 0 }
          ]
        })]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const card = container.querySelector("[data-component='apply-patch-tool']");
    expect(card?.textContent).toContain("2 files");
    expect(card?.querySelector("[data-component='diff-changes']")).toBeNull();

    const trigger = container.querySelector("[data-slot='collapsible-trigger']") as HTMLButtonElement | null;
    act(() => trigger?.click());

    expect(container.querySelectorAll("[data-slot='accordion-item']")).toHaveLength(2);
    expect([...container.querySelectorAll("[data-slot='apply-patch-filename']")].map((node) => node.textContent)).toEqual(["a.ts", "b.ts"]);
  });

  it("keeps the edit trigger when file metadata has not arrived", () => {
    act(() => root.render(
      <OpenCodeTimeline transcript={[editTool("assistant-1", undefined)]} busy={false} lastAssistantId={null} />
    ));

    expect(container.querySelector("[data-component='edit-tool']")).not.toBeNull();
    expect(container.querySelector("[data-component='diff-changes']")).toBeNull();
    expect(container.querySelector("[data-component='patch-diff']")).toBeNull();
  });
});
