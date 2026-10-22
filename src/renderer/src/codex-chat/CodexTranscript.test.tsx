import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, TranscriptItem } from "@shared/types";

vi.mock("../store", () => ({
  useStore: () => ({
    sessions: [],
    models: [],
    openFile: vi.fn(async () => {})
  })
}));

import { CodexTranscript, clearCodexScrollMemoryForTests } from "./CodexTranscript";

const session: SessionInfo = {
  id: "session-1",
  directory: "/workspace",
  workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 }
};

function assistant(
  parts: Extract<TranscriptItem, { kind: "assistant" }>["parts"],
  completed = true
): Extract<TranscriptItem, { kind: "assistant" }> {
  return {
    kind: "assistant",
    id: "assistant-1",
    messageID: "message-1",
    parts,
    completed,
    createdAt: 1_000,
    completedAt: completed ? 4_000 : undefined
  };
}

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  clearCodexScrollMemoryForTests();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(transcript: TranscriptItem[], options: {
  busy?: boolean;
  lastAssistantId?: string | null;
} = {}): void {
  act(() => {
    root.render(
      <CodexTranscript
        transcript={transcript}
        busy={options.busy ?? false}
        lastAssistantId={options.lastAssistantId ?? null}
        session={session}
        showEmpty={false}
      />
    );
  });
}

describe("CodexTranscript", () => {
  it("renders a right-aligned prompt, a worked-for divider and the answer", () => {
    render([
      { kind: "user", id: "user-1", text: "Inspect the repo", createdAt: 0 },
      assistant([
        {
          kind: "tool",
          id: "tool-1",
          tool: {
            id: "call-1",
            title: "bash",
            detail: "ls",
            status: "success",
            input: "{}",
            inputValue: {},
            output: "src"
          }
        },
        { kind: "text", id: "text-1", text: "Here is the summary.", complete: true }
      ])
    ]);
    expect(container.querySelector(".codex-user-bubble")?.textContent).toBe("Inspect the repo");
    expect(container.querySelector(".codex-work-label")?.textContent).toBe("Worked for 4s");
    expect(container.querySelector(".codex-answer .codex-md")?.textContent).toContain("Here is the summary.");
    expect(container.querySelector(".dsh-scroll")).toBeNull();
  });

  it("keeps tool calls behind the activity divider until opened", () => {
    render([
      { kind: "user", id: "user-1", text: "Check it", createdAt: 0 },
      assistant([
        {
          kind: "tool",
          id: "tool-1",
          tool: {
            id: "call-1",
            title: "bash",
            detail: "ls",
            status: "success",
            input: "{}",
            inputValue: {},
            output: "src"
          }
        },
        { kind: "text", id: "text-1", text: "Done.", complete: true }
      ])
    ]);
    expect(container.querySelector(".codex-tool")).toBeNull();
    act(() => {
      container.querySelector<HTMLButtonElement>(".codex-work-head")?.click();
    });
    expect(container.querySelector(".codex-tool-title")?.textContent).toBe("bash");
    expect(container.querySelector(".codex-tool-detail")?.textContent).toBe("ls");
  });

  it("marks streamed text with fade segments", () => {
    render(
      [
        { kind: "user", id: "user-1", text: "Go", createdAt: 0 },
        assistant([{ kind: "text", id: "text-1", text: "Streaming words here", complete: false }], false)
      ],
      { busy: true, lastAssistantId: "assistant-1" }
    );
    const root = container.querySelector<HTMLElement>(".codex-md");
    expect(root?.hasAttribute("data-markdown-animated")).toBe(true);
    const segments = container.querySelectorAll(".codex-fade");
    expect(segments.length).toBeGreaterThan(1);
    expect(Array.from(segments).map((node) => node.textContent).join("")).toBe("Streaming words here");
  });

  it("renders markdown structure without streaming animation when complete", () => {
    render([
      { kind: "user", id: "user-1", text: "Go", createdAt: 0 },
      assistant([{ kind: "text", id: "text-1", text: "**Bold** and `code`", complete: true }])
    ]);
    expect(container.querySelector(".codex-md strong")?.textContent).toBe("Bold");
    expect(container.querySelector(".codex-md-codespan")?.textContent).toBe("code");
    expect(container.querySelectorAll(".codex-fade")).toHaveLength(0);
  });
});
