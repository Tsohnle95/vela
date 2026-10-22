import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, TranscriptItem } from "@shared/types";

const openFile = vi.fn(async () => {});
const reopenSession = vi.fn(async () => {});
const stageRevert = vi.fn(async () => {});

vi.mock("../store", () => ({
  useStore: () => ({
    sessions: [],
    models: [],
    openFile,
    reopenSession,
    stageRevert
  })
}));

import { DshTranscript, clearDshScrollMemoryForTests } from "./DshTranscript";

const session: SessionInfo = {
  id: "session-1",
  directory: "/workspace",
  workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 }
};

function assistant(parts: Extract<TranscriptItem, { kind: "assistant" }>["parts"]): Extract<TranscriptItem, { kind: "assistant" }> {
  return {
    kind: "assistant",
    id: "assistant-1",
    messageID: "message-1",
    parts,
    completed: true,
    createdAt: 1_000,
    completedAt: 4_000
  };
}

const transcript: TranscriptItem[] = [
  { kind: "user", id: "user-1", text: "Inspect the repo", createdAt: 0 },
  assistant([
    { kind: "reasoning", id: "reason-1", text: "I should inspect the files.\n\nThen summarize.", complete: true },
    {
      kind: "tool",
      id: "tool-1",
      tool: {
        id: "call-1",
        title: "bash",
        detail: "ls",
        status: "success",
        input: JSON.stringify({ command: "ls" }),
        inputValue: { command: "ls" },
        output: "src\npackage.json"
      }
    },
    { kind: "text", id: "text-1", text: "Here is the summary.", complete: true }
  ])
];

async function flush(): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

describe("DshTranscript", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    clearDshScrollMemoryForTests();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("folds completed process behind the DSH completed row and keeps the answer", async () => {
    await act(async () => root.render(
      <DshTranscript transcript={transcript} busy={false} lastAssistantId={null} session={session} />
    ));
    expect(container.textContent).toContain("Completed in 4s");
    expect(container.textContent).toContain("Here is the summary.");
    expect(container.textContent).not.toContain("I should inspect the files.");
    expect(container.textContent).not.toContain("src\npackage.json");
  });

  it("expands the process group, then the individual tool row", async () => {
    await act(async () => root.render(
      <DshTranscript transcript={transcript} busy={false} lastAssistantId={null} session={session} />
    ));
    const status = [...container.querySelectorAll<HTMLButtonElement>("button")]
      .find((button) => button.textContent?.includes("Completed in"));
    expect(status).toBeDefined();
    await act(async () => status!.click());

    const group = [...container.querySelectorAll<HTMLButtonElement>(".dsh-process-header")].at(0);
    expect(group?.textContent).toContain("Ran commands");
    await act(async () => group!.click());

    const toolTitle = container.textContent ?? "";
    expect(toolTitle).toContain("Shell");
    expect(toolTitle).toContain("ls");
    const toolRow = [...container.querySelectorAll<HTMLElement>(".dsh-tool-row")][0]!;
    await act(async () => toolRow.click());
    expect(container.textContent).toContain("src");
    expect(container.textContent).toContain("package.json");
  });

  it("renders reasoning collapsed with its DSH summary and expands on demand", async () => {
    await act(async () => root.render(
      <DshTranscript transcript={transcript} busy={false} lastAssistantId={null} session={session} />
    ));
    const status = [...container.querySelectorAll<HTMLButtonElement>("button")]
      .find((button) => button.textContent?.includes("Completed in"));
    await act(async () => status!.click());
    const group = container.querySelector<HTMLButtonElement>(".dsh-process-header")!;
    await act(async () => group.click());

    expect(container.textContent).toContain("Think");
    expect(container.textContent).toContain("I should inspect the files.");
    expect(container.textContent).not.toContain("Then summarize.");

    const think = [...container.querySelectorAll<HTMLElement>(".dsh-reasoning-row")][0]!;
    await act(async () => think.click());
    expect(container.textContent).toContain("Then summarize.");
  });

  it("follows the floor until the reader scrolls away, then restores on request", async () => {
    const longer: TranscriptItem[] = [
      ...transcript,
      { kind: "user", id: "user-2", text: "And the test?", createdAt: 5_000 },
      assistant([{ kind: "text", id: "text-2", text: "The regression test passes.", complete: true }])
    ];
    await act(async () => root.render(
      <DshTranscript transcript={transcript} busy={false} lastAssistantId={null} session={session} />
    ));
    const scroll = container.querySelector<HTMLDivElement>(".dsh-scroll")!;
    Object.defineProperties(scroll, {
      clientHeight: { configurable: true, value: 100 },
      scrollHeight: { configurable: true, value: 500, writable: true },
      scrollTop: { configurable: true, value: 0, writable: true }
    });
    await act(async () => {
      scroll.scrollTop = 500;
      scroll.dispatchEvent(new Event("scroll"));
    });

    await act(async () => {
      scroll.scrollTop = 120;
      scroll.dispatchEvent(new Event("scroll"));
    });
    await act(async () => root.render(
      <DshTranscript transcript={longer} busy={false} lastAssistantId={null} session={session} />
    ));
    expect(scroll.scrollTop).toBe(120);
    expect(container.querySelector(".dsh-to-bottom")).not.toBeNull();

    Object.defineProperty(scroll, "scrollHeight", { configurable: true, value: 700, writable: true });
    const toBottom = container.querySelector<HTMLButtonElement>(".dsh-to-bottom")!;
    await act(async () => toBottom.click());
    expect(scroll.scrollTop).toBe(700);
  });
});
