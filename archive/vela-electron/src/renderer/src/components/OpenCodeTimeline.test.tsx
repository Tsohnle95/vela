import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo, SessionSummary, TranscriptItem } from "@shared/types";
import { OpenCodeTimeline } from "./OpenCodeTimeline";
import { InProcessMarkdownWorker } from "../opencode-markdown/markdown-test-worker";

const storeState = vi.hoisted(() => ({
  agents: [] as { id: string; name: string }[],
  sessions: [] as SessionSummary[],
  session: null as SessionInfo | null,
  reopenSession: vi.fn(),
  openFile: vi.fn(),
  focusSession: vi.fn(),
  replyPermission: vi.fn(),
  stageRevert: vi.fn()
}));

vi.mock("../store", () => ({
  useStore: () => storeState
}));

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

beforeEach(() => vi.stubGlobal("Worker", InProcessMarkdownWorker));
afterEach(() => vi.unstubAllGlobals());

const assistant = (id: string): TranscriptItem => ({
  kind: "assistant",
  id,
  messageID: id,
  completed: true,
  parts: [{ kind: "text", id: `${id}:text`, text: id, complete: true }]
});

const reasoningAssistant = (complete: boolean): TranscriptItem => ({
  kind: "assistant",
  id: "assistant-reasoning",
  messageID: "assistant-reasoning",
  completed: complete,
  parts: [{ kind: "reasoning", id: "reasoning-1", text: "Inspecting the code", complete }]
});

const toolAssistant = (id: string, title: string, input: Record<string, unknown>, metadata?: Record<string, unknown>): TranscriptItem => ({
  kind: "assistant",
  id,
  messageID: id,
  completed: true,
  parts: [{
    kind: "tool",
    id: `${id}:tool`,
    tool: {
      id: `${id}:tool`,
      title,
      detail: "",
      status: "success",
      input: JSON.stringify(input),
      inputValue: input,
      ...(metadata ? { metadata } : {})
    }
  }]
});

const summary = (id: string, overrides: Partial<SessionSummary> = {}): SessionSummary => ({
  id,
  title: "Untitled",
  directory: "/repo",
  updatedAt: 100,
  ...overrides
});

const events: Array<[string, TranscriptItem, string]> = [
  ["shell", { kind: "shell", id: "event", shellID: "shell", command: "pwd", status: "exited", exit: 0 }, "ShellMessage"],
  ["compaction", { kind: "compaction", id: "event", status: "completed", reason: "auto", summary: "summary" }, "Compaction"],
  ["synthetic", { kind: "synthetic", id: "event", text: "visible synthetic" }, "SessionEvent"],
  ["skill", { kind: "skill", id: "event", skill: "review", name: "Review", text: "loaded" }, "SessionEvent"],
  ["status", { kind: "status", id: "event", text: "working", tone: "info" }, "StatusNote"],
  ["divider", { kind: "divider", id: "event" }, "TurnDivider"]
];

describe("OpenCodeTimeline chronology", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    storeState.agents = [];
    storeState.sessions = [];
    storeState.session = null;
    storeState.reopenSession.mockReset();
    storeState.openFile.mockReset();
    storeState.focusSession.mockReset();
    storeState.replyPermission.mockReset();
    storeState.stageRevert.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  it("hides reasoning parts from the transcript like OpenCode's default", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[reasoningAssistant(true)]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.querySelector("[data-component='reasoning-part']")).toBeNull();
    expect(container.querySelector("[data-timeline-row='AssistantMessage']")).toBeNull();
    expect(container.querySelector("[data-timeline-row='AssistantActivity']")).toBeNull();
    expect(container.querySelector("[data-slot='session-turn-thinking']")).toBeNull();
  });

  it("shows one turn-level thinking row with the reasoning heading while busy", async () => {
    const live = reasoningAssistant(false) as Extract<TranscriptItem, { kind: "assistant" }>;
    live.parts = [{ kind: "reasoning", id: "reasoning-1", text: "**Inspecting the repository**", complete: false }];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[live]}
        busy
        lastAssistantId="assistant-reasoning"
      />
    ));

    expect(container.querySelector("[data-slot='session-turn-thinking']")?.getAttribute("role")).toBe("status");
    expect(container.querySelector("[data-slot='session-turn-thinking'] [data-component='text-shimmer']")?.getAttribute("aria-label")).toBe("Thinking");
    expect(container.querySelector("[data-slot='session-turn-thinking-heading']")?.textContent).toBe("Inspecting the repository");
    expect(container.querySelector("[data-component='reasoning-part']")).toBeNull();
  });

  it("derives the thinking heading from a markdown heading like OpenCode", async () => {
    const live = reasoningAssistant(false) as Extract<TranscriptItem, { kind: "assistant" }>;
    live.parts = [
      { kind: "reasoning", id: "r1", text: "Scratch work without a heading", complete: true },
      { kind: "reasoning", id: "r2", text: "# Planning the fix\n\nMore detail", complete: false }
    ];
    await act(async () => root.render(
      <OpenCodeTimeline transcript={[live]} busy lastAssistantId="assistant-reasoning" />
    ));

    expect(container.querySelector("[data-slot='session-turn-thinking-heading']")?.textContent).toBe("Planning the fix");
  });

  it("uses the latest reasoning heading in an active turn", async () => {
    const live = reasoningAssistant(false) as Extract<TranscriptItem, { kind: "assistant" }>;
    live.parts = [
      { kind: "reasoning", id: "r1", text: "# Inspecting", complete: true },
      { kind: "reasoning", id: "r2", text: "# Implementing", complete: false }
    ];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId={live.id} />));

    expect(container.querySelector("[data-slot='session-turn-thinking-heading']")?.textContent).toBe("Implementing");
  });

  it("reveals a replacement reasoning heading with OpenCode's entering and leaving tracks", async () => {
    const live = reasoningAssistant(false) as Extract<TranscriptItem, { kind: "assistant" }>;
    live.parts = [{ kind: "reasoning", id: "r1", text: "# Inspecting", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId={live.id} />));

    live.parts = [{ kind: "reasoning", id: "r1", text: "# Implementing", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[{ ...live }]} busy lastAssistantId={live.id} />));

    const reveal = container.querySelector("[data-slot='session-turn-thinking-heading']");
    expect(reveal?.getAttribute("data-component")).toBe("text-reveal");
    expect(reveal?.getAttribute("aria-label")).toBe("Implementing");
    expect(reveal?.querySelector("[data-slot='text-reveal-entering']")?.textContent).toBe("Implementing");
    expect(reveal?.querySelector("[data-slot='text-reveal-leaving']")?.textContent).toBe("Inspecting");
  });

  it("hides the thinking row when the active turn errored", async () => {
    const failed = reasoningAssistant(false) as Extract<TranscriptItem, { kind: "assistant" }>;
    failed.error = "Error: boom";
    await act(async () => root.render(
      <OpenCodeTimeline transcript={[failed]} busy lastAssistantId="assistant-reasoning" />
    ));

    expect(container.querySelector("[data-slot='session-turn-thinking']")).toBeNull();
    expect(container.querySelector("[data-timeline-row='Error']")).not.toBeNull();
  });

  it("opens absolute read paths relative to the tool session workspace", async () => {
    const session: SessionInfo = {
      id: "session-read",
      directory: "/repo",
      workspace: { id: "workspace-read", generation: 4 }
    };
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[toolAssistant("read", "read", { filePath: "/repo/docs/README.md" })]}
        busy={false}
        lastAssistantId={null}
        session={session}
      />
    ));

    const subtitle = (() => {
      act(() => container.querySelector<HTMLButtonElement>("[data-component='context-tool-group'] [data-slot='collapsible-trigger']")!.click());
      return container.querySelector("[data-slot='basic-tool-tool-subtitle']") as HTMLElement | null;
    })();
    act(() => subtitle?.click());

    expect(storeState.focusSession).toHaveBeenCalledWith("session-read");
    expect(storeState.openFile).toHaveBeenCalledWith("docs/README.md", session.workspace);
  });

  it("renders one thinking row while the first stream item is pending", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline transcript={[]} busy lastAssistantId={null} />
    ));

    const thinking = container.querySelector("[data-slot='session-turn-thinking']");
    expect(thinking?.getAttribute("role")).toBe("status");
    expect(thinking?.querySelector("[data-component='text-shimmer']")?.getAttribute("aria-label")).toBe("Thinking");
    expect(container.querySelector("[data-component='live-activity-dock']")).toBeNull();
  });

  it("keeps the thinking row beside the live stream item without duplicating it", async () => {
    const live = assistant("assistant-live") as Extract<TranscriptItem, { kind: "assistant" }>;
    live.completed = false;
    live.parts = [{ kind: "text", id: "text-live", text: "Streaming answer", complete: false }];

    await act(async () => root.render(
      <OpenCodeTimeline transcript={[live]} busy lastAssistantId={live.id} />
    ));

    expect(container.querySelector("[data-timeline-row='AssistantMessage']")?.textContent).toContain("Streaming answer");
    expect(container.querySelectorAll("[data-timeline-row='AssistantWorking']")).toHaveLength(1);
    expect(container.querySelector("[data-component='live-activity-dock']")).toBeNull();
  });

  it("folds a context run across contiguous assistant messages", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[toolAssistant("a1", "read", { filePath: "a.ts" }), toolAssistant("a2", "read", { filePath: "b.ts" })]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.querySelectorAll("[data-component='context-tool-group']")).toHaveLength(1);
    expect([...container.querySelectorAll("[data-timeline-row]")].map((row) => row.getAttribute("data-timeline-row")))
      .toEqual(["AssistantActivity"]);

    act(() => container.querySelector<HTMLButtonElement>("[data-component='context-tool-group'] [data-slot='collapsible-trigger']")!.click());
    expect([...container.querySelectorAll("[data-slot='basic-tool-tool-subtitle']")].map((node) => node.textContent))
      .toEqual(["a.ts", "b.ts"]);
  });

  it("keeps an assistant run as a stable keyed node while its parts change", async () => {
    const first: TranscriptItem = toolAssistant("tool-a", "bash", { command: "pwd" });
    const second: TranscriptItem = {
      kind: "assistant",
      id: "assistant-text",
      messageID: "assistant-text",
      completed: false,
      parts: [{ kind: "text", id: "text-1", text: "Before", complete: false }]
    };
    await act(async () => root.render(<OpenCodeTimeline transcript={[first, second]} busy lastAssistantId="assistant-text" />));

    const row = container.querySelector("[data-timeline-row='AssistantMessage']");
    expect(row?.textContent).toContain("Before");

    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[first, { ...second, parts: [{ kind: "text" as const, id: "text-1", text: "After", complete: false }] }]}
        busy
        lastAssistantId="assistant-text"
      />
    ));

    const updated = container.querySelector("[data-timeline-row='AssistantMessage']");
    expect(updated).toBe(row);
    expect(updated?.textContent).toContain("After");
  });

  it("renders the thinking row once at the end of the active run", async () => {
    const first = toolAssistant("a1", "bash", { command: "pwd" });
    const second = assistant("a2") as Extract<TranscriptItem, { kind: "assistant" }>;
    second.completed = false;
    await act(async () => root.render(
      <OpenCodeTimeline transcript={[first, second]} busy lastAssistantId="a2" />
    ));

    const rows = [...container.querySelectorAll("[data-timeline-row]")].map((row) => row.getAttribute("data-timeline-row"));
    expect(rows.at(-1)).toBe("AssistantWorking");
    expect(rows.filter((row) => row === "AssistantWorking")).toHaveLength(1);
  });

  it("renders streamed text parts while hiding the reasoning that produced them", async () => {
    const live = reasoningAssistant(false) as Extract<TranscriptItem, { kind: "assistant" }>;
    live.parts = [
      { kind: "reasoning", id: "reasoning-1", text: "Inspecting", complete: false },
      { kind: "text", id: "text-1", text: "Visible update", complete: false }
    ];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId="assistant-reasoning" />));

    expect(container.querySelector("[data-component='reasoning-part']")).toBeNull();
    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe("Visible update");
    expect(container.querySelector("[data-slot='session-turn-thinking']")).not.toBeNull();
  });

  it("paces large text updates using OpenCode's live text progression and flushes on completion", async () => {
    vi.useFakeTimers();
    const live = assistant("assistant-live") as Extract<TranscriptItem, { kind: "assistant" }>;
    live.completed = false;
    live.parts = [{ kind: "text", id: "text-live", text: "Start", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId="assistant-live" />));

    const update = `Start ${"streamed content ".repeat(80)}`;
    live.parts = [{ kind: "text", id: "text-live", text: update, complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[{ ...live }]} busy lastAssistantId="assistant-live" />));

    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe("Start");
    await act(async () => vi.advanceTimersByTime(24));
    const first = container.querySelector("[data-component='markdown']")?.textContent?.trim() ?? "";
    expect(first.length).toBeGreaterThan("Start".length);
    expect(first.length).toBeLessThan(update.trim().length);
    expect(update.startsWith(first)).toBe(true);

    await act(async () => vi.advanceTimersByTime(240));
    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe(update.trim());

    const larger = `${update} ${"more words ".repeat(80)}`;
    live.parts = [{ kind: "text", id: "text-live", text: larger, complete: true }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[{ ...live }]} busy lastAssistantId="assistant-live" />));
    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe(update.trim());
    live.completed = true;
    await act(async () => root.render(<OpenCodeTimeline transcript={[{ ...live }]} busy={false} lastAssistantId={null} />));
    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe(larger.trim());
  });

  it("keeps a completed text part live until its assistant message completes", async () => {
    const live = assistant("assistant-live") as Extract<TranscriptItem, { kind: "assistant" }>;
    live.completed = false;
    live.parts = [{ kind: "text", id: "text-live", text: "An unfinished **answer", complete: true }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId="assistant-live" />));

    await vi.waitFor(() => expect(container.querySelector("[data-component='markdown'] strong")?.textContent).toBe("answer"));
    expect(container.querySelector("[data-slot='text-part-copy-wrapper']")).toBeNull();

    live.completed = true;
    await act(async () => root.render(<OpenCodeTimeline transcript={[{ ...live }]} busy={false} lastAssistantId={null} />));
    await vi.waitFor(() => expect(container.querySelector("[data-component='markdown'] strong")).toBeNull());
    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe("An unfinished **answer");
    expect(container.querySelector("[data-slot='text-part-copy-wrapper']")).not.toBeNull();
  });

  it("shows short deltas and non-prefix replacements immediately", async () => {
    vi.useFakeTimers();
    const live = assistant("assistant-live") as Extract<TranscriptItem, { kind: "assistant" }>;
    live.completed = false;
    live.parts = [{ kind: "text", id: "text-live", text: "Start", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId={live.id} />));

    live.parts = [{ kind: "text", id: "text-live", text: "Start of an answer", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[{ ...live }]} busy lastAssistantId={live.id} />));
    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe("Start of an answer");

    live.parts = [{ kind: "text", id: "text-live", text: "A replacement answer", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[{ ...live }]} busy lastAssistantId={live.id} />));
    expect(container.querySelector("[data-component='markdown']")?.textContent?.trim()).toBe("A replacement answer");
    expect(vi.getTimerCount()).toBe(0);
  });

  it("heals incomplete markdown in the streaming tail like OpenCode", async () => {
    const live = assistant("assistant-heal") as Extract<TranscriptItem, { kind: "assistant" }>;
    live.completed = false;
    live.parts = [{ kind: "text", id: "text-heal", text: "hello **world", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId="assistant-heal" />));

    await vi.waitFor(() => expect(container.querySelector("[data-component='markdown'] strong")?.textContent).toBe("world"));
  });

  it("freezes completed markdown blocks and keeps only the tail live", async () => {
    const live = assistant("assistant-blocks") as Extract<TranscriptItem, { kind: "assistant" }>;
    live.completed = false;
    live.parts = [
      { kind: "text", id: "text-blocks", text: "# Plan\n\nFinished paragraph.\n\n- live item", complete: false }
    ];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId="assistant-blocks" />));

    await vi.waitFor(() => expect(container.querySelectorAll("[data-component='markdown'] [data-markdown-block]")).toHaveLength(3));
    const blocks = container.querySelectorAll("[data-component='markdown'] [data-markdown-block]");
    expect(blocks).toHaveLength(3);
    expect(blocks[0]?.querySelector("h1")?.textContent).toBe("Plan");
  });

  it("renders an unfinished code fence as a code block while streaming", async () => {
    const live = assistant("assistant-code") as Extract<TranscriptItem, { kind: "assistant" }>;
    live.completed = false;
    live.parts = [{ kind: "text", id: "text-code", text: "before\n\n```ts\nconst x = 1", complete: false }];
    await act(async () => root.render(<OpenCodeTimeline transcript={[live]} busy lastAssistantId="assistant-code" />));

    await vi.waitFor(() => expect(container.querySelector("[data-component='markdown-code'] code")?.textContent).toBe("const x = 1"));
    const code = container.querySelector(
      "[data-component='markdown'] [data-markdown-block][data-markdown-complete='false'] code"
    );
    expect(code?.className).toBe("language-ts");
    expect(code?.textContent).toContain("const x = 1");
  });

  it("shows generic scalar arguments in upstream order without a diagnostic subtitle", async () => {
    const generic = toolAssistant("tool", "tool", { limit: 40, content: "private" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const transcript: TranscriptItem[] = [{
      ...generic,
      parts: [{
        kind: "tool",
        id: "tool:part",
        tool: { id: "tool:part", title: "tool", detail: "a very long diagnostic detail", status: "success", input: JSON.stringify({ limit: 40, content: "private" }) }
      }]
    }];
    await act(async () => root.render(<OpenCodeTimeline transcript={transcript} busy={false} lastAssistantId={null} />));

    expect(container.querySelector("[data-slot='basic-tool-tool-title'] [data-component='text-shimmer']")?.getAttribute("aria-label")).toBe("Called `tool`");
    expect([...container.querySelectorAll("[data-slot='basic-tool-tool-arg']")].map((node) => node.textContent)).toEqual(["limit=40", "content=private"]);
    expect(container.querySelector("[data-slot='basic-tool-tool-subtitle']")).toBeNull();
  });

  it("keeps generic tools as a trigger without a custom I/O disclosure", async () => {
    const generic = toolAssistant("tool", "custom_tool", { query: "stream events" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    generic.parts[0] = generic.parts[0].kind === "tool"
      ? { ...generic.parts[0], tool: { ...generic.parts[0].tool, output: "event payload" } }
      : generic.parts[0];
    await act(async () => root.render(<OpenCodeTimeline transcript={[generic]} busy={false} lastAssistantId={null} />));

    act(() => container.querySelector<HTMLButtonElement>("[data-slot='collapsible-trigger']")!.click());
    expect(container.querySelector("[data-component='tool-io']")).toBeNull();
    expect(container.querySelector("[data-slot='basic-tool-tool-subtitle']")?.textContent).toBe("stream events");
    expect(container.textContent).not.toContain("event payload");
  });

  it("keeps live command output collapsed until opened and shimmers the running tool title", async () => {
    const running = toolAssistant("tool", "bash", { command: "npm test" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    running.completed = false;
    const part = running.parts[0];
    if (part.kind === "tool") {
      part.tool.status = "running";
      part.tool.output = "RUN  v3.2.7";
    }

    await act(async () => root.render(<OpenCodeTimeline transcript={[running]} busy lastAssistantId="tool" />));

    expect(container.querySelector("[data-slot='basic-tool-tool-title'] [data-component='text-shimmer']")?.getAttribute("data-active")).toBe("true");
    expect(container.querySelector("[data-slot='collapsible-content']")).toBeNull();
    await act(async () => container.querySelector<HTMLButtonElement>("[data-slot='collapsible-trigger']")!.click());
    expect(container.querySelector("[data-slot='collapsible-content']")).not.toBeNull();
    expect(container.querySelector("[data-component='bash-output']")).not.toBeNull();
    expect(container.querySelector("[data-slot='bash-pre']")?.textContent).toBe("$ npm test\n\nRUN  v3.2.7");
  });

  it("renders a failed call as an expandable error card", async () => {
    const failed = toolAssistant("tool", "tool", { path: "/repo/source b", limit: 2000 }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const part = failed.parts[0];
    if (part.kind === "tool") {
      part.tool.status = "failed";
      part.tool.output = "File not found: /repo/source b";
    }

    await act(async () => root.render(<OpenCodeTimeline transcript={[failed]} busy={false} lastAssistantId={null} />));

    expect(container.querySelector("[data-slot='basic-tool-tool-title']")?.textContent).toBe("tool");
    expect(container.querySelector("[data-kind='tool-error-card']")).not.toBeNull();
    expect(container.querySelector("[data-slot='basic-tool-tool-subtitle']")?.textContent).toBe("File not found");
    expect(container.querySelector("[data-slot='collapsible-content']")).toBeNull();

    act(() => container.querySelector<HTMLButtonElement>("[data-slot='collapsible-trigger']")!.click());
    expect(container.querySelector("[data-slot='tool-error-card-description']")?.textContent).toBe("/repo/source b");
  });

  it("renders the full shell payload with OpenCode's terminal sequence cleanup", async () => {
    const item = toolAssistant("shell", "bash", { command: "echo output" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const output = "\u001b[31mred\u001b[0m\r\n\u001b]8;;https://example.com\u0007link\u001b]8;;\u0007\n" + "x".repeat(7000);
    if (item.parts[0].kind === "tool") item.parts[0].tool.output = output;
    await act(async () => root.render(<OpenCodeTimeline transcript={[item]} busy={false} lastAssistantId={null} />));
    await act(async () => container.querySelector<HTMLButtonElement>("[data-slot='collapsible-trigger']")!.click());
    expect(container.querySelector("[data-slot='bash-pre']")?.textContent).toBe("$ echo output\n\nred\nlink\n" + "x".repeat(7000));
  });

  it("capitalizes a lowercase error head like OpenCode", async () => {
    const failed = toolAssistant("tool", "bash", { command: "false" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const part = failed.parts[0];
    if (part.kind === "tool") {
      part.tool.status = "failed";
      part.tool.output = "Error: bash: command failed with exit code 1: boom";
    }

    await act(async () => root.render(<OpenCodeTimeline transcript={[failed]} busy={false} lastAssistantId={null} />));

    expect(container.querySelector("[data-slot='basic-tool-tool-subtitle']")?.textContent).toBe("Bash");
  });

  it.each(events)("keeps an interleaved %s event between assistant runs", async (_name, event, row) => {
    await act(async () => root.render(
      <OpenCodeTimeline transcript={[assistant("before"), event, assistant("after")]} busy={false} lastAssistantId={null} />
    ));

    expect([...container.querySelectorAll("[data-timeline-row]")].map((node) => node.getAttribute("data-timeline-row")))
      .toEqual(["AssistantMessage", row, "AssistantMessage"]);
  });

  it("groups only contiguous assistant messages", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[assistant("one"), assistant("two"), events[4][1], assistant("three"), assistant("four")]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const rowText = (node: Element): string | null => {
      const clone = node.cloneNode(true) as Element;
      clone.querySelectorAll("[data-slot='assistant-message-head']").forEach((head) => head.remove());
      return clone.textContent?.trim() ?? null;
    };

    expect([...container.querySelectorAll("[data-timeline-row]")].map(rowText))
      .toEqual(["one", "two", "working", "three", "four"]);
  });

  it("renders flat prose around activity without an extra assistant avatar or name head", async () => {
    const message: TranscriptItem = {
      kind: "assistant",
      id: "a1",
      messageID: "a1",
      completed: true,
      parts: [
        { kind: "text", id: "t1", text: "intro", complete: true },
        { kind: "tool", id: "tool1", tool: { id: "tool1", title: "read", detail: "", status: "success", input: "{}", inputValue: {} } },
        { kind: "text", id: "t2", text: "outro", complete: true }
      ]
    };
    await act(async () => root.render(<OpenCodeTimeline transcript={[message]} busy={false} lastAssistantId={null} />));

    const heads = [...container.querySelectorAll("[data-slot='assistant-message-head']")];
    expect(heads).toHaveLength(0);
    expect(container.querySelector("[data-slot='assistant-avatar']")).toBeNull();
    expect(container.querySelectorAll("[data-component='text-part']")).toHaveLength(2);
  });

  it("renders status notes as quiet inline markers, not boxes", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[{ kind: "status", id: "event", text: "Interrupted", tone: "info" }]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const note = container.querySelector("[data-component='session-note']");
    expect(note?.getAttribute("data-tone")).toBe("info");
    expect(note?.querySelector(".codicon-info")).toBeTruthy();
    expect(note?.querySelector("[data-slot='session-note-text']")?.textContent).toBe("Interrupted");
  });

  it("groups contiguous exploration under an OpenCode status title with counts", async () => {
    const read = toolAssistant("read", "read", { filePath: "/repo/src/main.ts" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const grep = toolAssistant("grep", "grep", { pattern: "stream", path: "/repo/src" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const transcript: TranscriptItem[] = [{ ...read, parts: [...read.parts, ...grep.parts] }];
    await act(async () => root.render(<OpenCodeTimeline transcript={transcript} busy={false} lastAssistantId={null} />));

    expect(container.querySelector("[data-component='tool-status-title']")?.getAttribute("aria-label")).toBe("Explored");
    const summary = container.querySelector("[data-slot='context-tool-group-summary']");
    const activeCounts = [...(summary?.querySelectorAll("[data-slot='tool-count-summary-item'][data-active='true'] [data-component='animated-number']") ?? [])]
      .map((node) => node.getAttribute("aria-label"));
    expect(activeCounts).toEqual(["1", "1"]);
    const activeLabels = [...(summary?.querySelectorAll("[data-slot='tool-count-summary-item'][data-active='true']") ?? [])]
      .map((node) => node.textContent?.replace(/[0-9]/g, "").trim());
    expect(activeLabels).toEqual(["read", "search"]);
    expect(container.querySelector("[data-component='context-tool-group-list']")).toBeNull();

    act(() => container.querySelector<HTMLButtonElement>("[data-component='context-tool-group'] [data-slot='collapsible-trigger']")!.click());

    const titles = [...container.querySelectorAll("[data-component='context-tool-group-list'] [data-slot='basic-tool-tool-title']")]
      .map((node) => node.querySelector("[data-component='text-shimmer']")?.getAttribute("aria-label"));
    expect(titles).toEqual(["Read", "Grep"]);

    const subtitles = [...container.querySelectorAll("[data-component='context-tool-group-list'] [data-slot='basic-tool-tool-subtitle']")]
      .map((node) => node.textContent);
    // read shows the file basename; list/glob/grep show the search directory (OpenCode's `getDirectory`).
    expect(subtitles).toEqual(["main.ts", "/repo/"]);
    const args = [...container.querySelectorAll("[data-component='context-tool-group-list'] [data-slot='basic-tool-tool-arg']")]
      .map((node) => node.textContent);
    // The pattern lives only in the args, never duplicated into the subtitle.
    expect(args).toEqual(["pattern=stream"]);
  });

  it("renders every tool as a compact inline trigger inside context groups", async () => {
    const read = toolAssistant("read", "read", { filePath: "/repo/src/main.ts" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const bash = toolAssistant("bash", "bash", { command: "npm test" }) as Extract<TranscriptItem, { kind: "assistant" }>;
    const transcript: TranscriptItem[] = [{ ...read, parts: [...read.parts, ...bash.parts] }];
    await act(async () => root.render(<OpenCodeTimeline transcript={transcript} busy={false} lastAssistantId={null} />));

    const wrappers = [...container.querySelectorAll("[data-component='tool-part-wrapper']")];
    expect(wrappers).toHaveLength(1);
    expect(wrappers[0]?.getAttribute("data-tool")).toBe("bash");
    expect(wrappers[0]?.getAttribute("data-variant")).toBe("inline");
    expect(container.querySelector("[data-component='context-tool-group']")).not.toBeNull();
  });

  it("retains the raw generic tool name when its arguments contain a path", async () => {
    const generic = toolAssistant("tool", "tool", { path: "/repo/source b", limit: 2000 }) as Extract<TranscriptItem, { kind: "assistant" }>;
    await act(async () => root.render(<OpenCodeTimeline transcript={[generic]} busy={false} lastAssistantId={null} />));

    expect(container.querySelector("[data-slot='basic-tool-tool-title'] [data-component='text-shimmer']")?.getAttribute("aria-label")).toBe("Called `tool`");
    expect(container.querySelector("[data-slot='basic-tool-tool-subtitle']")?.textContent).toBe("/repo/source b");
  });
});

describe("completed assistant layout", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    storeState.agents = [];
    storeState.sessions = [];
    storeState.session = null;
    storeState.reopenSession.mockReset();
    storeState.openFile.mockReset();
    storeState.focusSession.mockReset();
    storeState.replyPermission.mockReset();
    storeState.stageRevert.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("shows the response copy action on only the last text part of a turn", async () => {
    const transcript: TranscriptItem[] = [
      { kind: "assistant", id: "a1", messageID: "a1", completed: true,
        parts: [{ kind: "text", id: "p1", text: "First", complete: true }] },
      { kind: "assistant", id: "a2", messageID: "a2", completed: true,
        parts: [{ kind: "text", id: "p2", text: "Second", complete: true }] }
    ];
    await act(async () => root.render(<OpenCodeTimeline transcript={transcript} busy={false} lastAssistantId={null} />));

    expect(container.querySelectorAll("[data-slot='text-part-copy-wrapper']")).toHaveLength(1);
    expect(container.querySelector("[data-slot='text-part-copy-wrapper']")?.closest("[data-timeline-part-id]")?.getAttribute("data-timeline-part-id"))
      .toBe("p2");
  });

  it("renders each assistant message in its own stream phase during a multi-step turn", async () => {
    const transcript: TranscriptItem[] = [
      { kind: "assistant", id: "a1", messageID: "a1", completed: true,
        parts: [{ kind: "text", id: "p1", text: "First **step", complete: true }] },
      { kind: "assistant", id: "a2", messageID: "a2", completed: false,
        parts: [{ kind: "text", id: "p2", text: "Second **step", complete: false }] }
    ];
    await act(async () => root.render(<OpenCodeTimeline transcript={transcript} busy lastAssistantId="a2" />));

    const textRows = container.querySelectorAll("[data-component='text-part']");
    await vi.waitFor(() => expect(textRows[1]?.querySelector("strong")?.textContent).toBe("step"));
    expect(textRows[0]?.querySelector("strong")).toBeNull();
    expect(textRows[0]?.textContent?.trim()).toBe("First **step");
    expect(container.querySelector("[data-slot='text-part-copy-wrapper']")).toBeNull();
  });

  it("renders one row per group with no phantom action rows between assistant messages", async () => {
    const session = { id: "session-1", workspace: "/repo", directory: "/repo" } as unknown as SessionInfo;
    const transcript: TranscriptItem[] = [
      {
        kind: "assistant",
        id: "a1",
        messageID: "a1",
        completed: true,
        parts: [
          { kind: "reasoning", id: "r1", text: "thinking", complete: true },
          { kind: "tool", id: "t1", tool: { id: "t1", title: "read", detail: "README.md", status: "success", input: "{}", inputValue: {} } }
        ]
      },
      {
        kind: "assistant",
        id: "a2",
        messageID: "a2",
        completed: true,
        parts: [{ kind: "text", id: "p1", text: "Answer", complete: true }]
      }
    ];
    await act(async () => root.render(<OpenCodeTimeline transcript={transcript} busy={false} lastAssistantId={null} session={session} />));

    const rows = Array.from(container.querySelectorAll("[data-timeline-row]")).map((row) => row.getAttribute("data-timeline-row"));
    expect(rows).toEqual(["AssistantActivity", "AssistantMessage"]);

    const actions = container.querySelector("[data-component='turn-actions']");
    expect(actions).toBeNull();
    const copy = container.querySelector("[data-slot='text-part-copy-button']");
    expect(copy).not.toBeNull();
    const messageRow = container.querySelector("[data-timeline-row='AssistantMessage']");
    expect(copy?.closest("[data-slot='text-part-copy-wrapper']")).toBe(messageRow?.querySelector("[data-slot='text-part-copy-wrapper']"));
  });

  it("places response copy directly below the answer and revert beside the user message", async () => {
    const session: SessionInfo = {
      id: "session-1", directory: "/repo", workspace: { id: "workspace-1", generation: 1 }
    };
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    await act(async () => root.render(
      <OpenCodeTimeline transcript={[
        { kind: "user", id: "user-1", text: "Prompt" },
        { kind: "assistant", id: "assistant-1", messageID: "assistant-1", completed: true,
          parts: [{ kind: "text", id: "text-1", text: "Answer", complete: true }] }
      ]} busy={false} lastAssistantId={null} session={session} />
    ));
    expect(container.querySelector("[data-component='response-options']")).toBeNull();
    const copy = container.querySelector<HTMLButtonElement>("[data-slot='text-part-copy-button']");
    expect(copy).not.toBeNull();
    await act(async () => copy?.click());
    expect(writeText).toHaveBeenCalledWith("Answer");
    const revert = container.querySelector<HTMLButtonElement>("[data-slot='user-message-copy-wrapper'] [aria-label='Revert message']");
    expect(revert).not.toBeNull();
    act(() => revert?.click());
    expect(storeState.stageRevert).toHaveBeenCalledWith(session.workspace, "user-1");
  });
});

describe("subagent dispatch links", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    storeState.agents = [{ id: "build", name: "Build" }];
    storeState.sessions = [];
    storeState.session = { id: "session-parent", directory: "/repo", workspace: { id: "workspace-1", generation: 1 } };
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
  });

  it("renders a task tool card that opens the resolved child session", async () => {
    storeState.sessions = [summary("session-child", {
      title: "Run the tests",
      agent: "build",
      parentID: "session-parent",
      updatedAt: 200
    })];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[toolAssistant("assistant-1", "subagent", { agent: "build", description: "Run the tests", prompt: "run them" })]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const card = container.querySelector("[data-component='task-tool-card']");
    expect(card).not.toBeNull();
    expect(card?.textContent).toContain("Run the tests");
    expect(card?.querySelector("[data-component='task-tool-title']")?.textContent).toBe("Build");

    const surface = card?.closest<HTMLButtonElement>("[data-slot='collapsible-trigger']");
    expect(surface?.disabled).toBe(false);
    act(() => surface?.click());
    expect(storeState.reopenSession).toHaveBeenCalledWith("session-child");
  });

  it("resolves the task tool fallback by description and agent when metadata is absent", async () => {
    storeState.sessions = [
      summary("session-other", { title: "Review docs", agent: "plan", parentID: "session-parent", updatedAt: 300 }),
      summary("session-child", { title: "Run the tests", agent: "build", parentID: "session-parent", updatedAt: 200 })
    ];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[toolAssistant("assistant-1", "task", { subagent_type: "build", description: "Run the tests" })]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const surface = container.querySelector("[data-component='task-tool-card']")?.closest<HTMLButtonElement>("[data-slot='collapsible-trigger']");
    expect(surface?.disabled).toBe(false);
    act(() => surface?.click());
    expect(storeState.reopenSession).toHaveBeenCalledWith("session-child");
  });

  it("disables the task tool card when no child session resolves", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[toolAssistant("assistant-1", "subagent", { agent: "build", description: "Run the tests" })]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const surface = container.querySelector("[data-component='task-tool-card']")?.closest<HTMLButtonElement>("[data-slot='collapsible-trigger']");
    expect(surface?.disabled).toBe(true);
  });

  it("turns a subagent synthetic tag into a clickable link to the child session", async () => {
    storeState.sessions = [summary("session-child", {
      title: "Run the tests",
      agent: "build",
      parentID: "session-parent",
      updatedAt: 200
    })];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[{
          kind: "synthetic",
          id: "synthetic-1",
          text: '<subagent id="session-child" state="completed" description="Run the tests">\n</subagent>',
          description: "Run the tests"
        }]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.textContent).not.toContain("<subagent");
    const card = container.querySelector("[data-component='task-tool-card']");
    expect(card).not.toBeNull();
    expect(card?.getAttribute("data-state")).toBe("completed");
    expect(card?.textContent).toContain("Run the tests");
    expect(card?.querySelector("[data-component='task-tool-title']")?.textContent).toBe("Build");

    const surface = card?.closest<HTMLButtonElement>("[data-slot='collapsible-trigger']");
    expect(surface?.disabled).toBe(false);
    act(() => surface?.click());
    expect(storeState.reopenSession).toHaveBeenCalledWith("session-child");
  });

  it("opens the child by id from a subagent synthetic tag even when the sessions list is stale", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[{
          kind: "synthetic",
          id: "synthetic-1",
          text: '<subagent id="session-child" state="running" description="Run the tests">\n</subagent>',
          description: "Run the tests"
        }]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const surface = container.querySelector("[data-component='task-tool-card']")?.closest<HTMLButtonElement>("[data-slot='collapsible-trigger']");
    expect(surface?.disabled).toBe(false);
    act(() => surface?.click());
    expect(storeState.reopenSession).toHaveBeenCalledWith("session-child");
  });

  it("collapses tool and synthetic records for one child into one descriptive card", async () => {
    storeState.sessions = [summary("session-child", {
      title: "Inspect the renderer",
      agent: "build",
      parentID: "session-parent"
    })];
    const synthetic: TranscriptItem = {
      kind: "synthetic",
      id: "dispatch-child",
      text: '<subagent id="session-child" agent="build" description="Inspect the renderer" state="completed" />'
    };
    const dispatch = toolAssistant(
      "assistant-1",
      "subagent",
      { agent: "build", description: "Inspect the renderer" },
      { sessionID: "session-child" }
    ) as Extract<TranscriptItem, { kind: "assistant" }>;
    const dispatchPart = dispatch.parts[0];
    if (dispatchPart?.kind === "tool") dispatchPart.tool.status = "running";
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[
          dispatch,
          synthetic
        ]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.querySelectorAll("[data-component='task-tool-card']")).toHaveLength(1);
    expect(container.querySelector("[data-component='task-tool-title']")?.textContent).toBe("Build");
    expect(container.querySelector("[data-slot='basic-tool-tool-subtitle']")?.textContent).toBe("Inspect the renderer");
    expect(container.querySelector("[data-slot='task-tool-status-label']")).toBeNull();
    expect(container.querySelector("[data-component='task-tool-spinner']")).toBeNull();
    expect(container.querySelector("[data-component='task-tool-card']")?.closest("[data-slot='collapsible-trigger']")?.getAttribute("aria-label"))
      .toBe("Open delegated agent session: Inspect the renderer");
  });

  it("consolidates repeated tool snapshots for one child into one stable card", async () => {
    storeState.sessions = [summary("session-child", {
      title: "Inspect the renderer",
      agent: "build",
      parentID: "session-parent"
    })];
    const first = toolAssistant("assistant-1", "subagent", { agent: "build", description: "Inspect the renderer" }, { sessionID: "session-child" });
    await act(async () => root.render(
      <OpenCodeTimeline transcript={[first]} busy lastAssistantId="assistant-1" />
    ));
    const initialCard = container.querySelector("[data-component='task-tool-card']");

    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[
          first,
          toolAssistant("assistant-2", "subagent", { agent: "build", description: "Inspect the renderer" }, { sessionID: "session-child" }),
          toolAssistant("assistant-3", "task", { agent: "build", description: "Inspect the renderer" }, { sessionID: "session-child" })
        ]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.querySelectorAll("[data-component='task-tool-card']")).toHaveLength(1);
    expect(container.querySelector("[data-component='task-tool-card']")).toBe(initialCard);
    expect(container.querySelector("[data-component='task-tool-title']")?.textContent).toBe("Build");
    expect(container.querySelector("[data-slot='basic-tool-tool-subtitle']")?.textContent).toBe("Inspect the renderer");
  });

  it("keeps one dispatch card while the child session graph is still arriving", async () => {
    storeState.sessions = [];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[
          toolAssistant("assistant-1", "subagent", { agent: "build", description: "Inspect the renderer" }),
          {
            kind: "synthetic",
            id: "dispatch-child",
            text: '<subagent id="session-child" agent="build" description="Inspect the renderer" state="running" />'
          }
        ]}
        busy
        lastAssistantId="assistant-1"
      />
    ));

    expect(container.querySelectorAll("[data-component='task-tool-card']")).toHaveLength(1);
    expect(container.querySelector("[data-component='task-tool-card']")).not.toBeNull();
  });

  it("keeps deliberate matching dispatches from separate user turns", async () => {
    storeState.sessions = [];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[
          { kind: "user", id: "user-1", text: "Inspect it" },
          toolAssistant("assistant-1", "subagent", { agent: "build", description: "Inspect the renderer" }),
          { kind: "user", id: "user-2", text: "Inspect it again" },
          toolAssistant("assistant-2", "subagent", { agent: "build", description: "Inspect the renderer" })
        ]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.querySelectorAll("[data-component='task-tool-card']")).toHaveLength(2);
  });

  it("keeps cards for distinct child sessions separate", async () => {
    storeState.sessions = [
      summary("session-child-a", { title: "First task", parentID: "session-parent" }),
      summary("session-child-b", { title: "Second task", parentID: "session-parent" })
    ];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[
          toolAssistant("assistant-1", "subagent", { description: "First task" }, { sessionID: "session-child-a" }),
          { kind: "synthetic", id: "dispatch-b", text: '<subagent id="session-child-b" description="Second task" state="running" />' }
        ]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.querySelectorAll("[data-component='task-tool-card']")).toHaveLength(2);
  });

  it("resolves legacy agent=/prompt= synthetic input to the matching child session", async () => {
    storeState.sessions = [summary("session-child", {
      title: "Run the tests",
      agent: "build",
      parentID: "session-parent",
      updatedAt: 200
    })];
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[{
          kind: "synthetic",
          id: "synthetic-1",
          text: "agent=build\nprompt=Run the test suite and report failures"
        }]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    const card = container.querySelector("[data-component='task-tool-card']");
    expect(card).not.toBeNull();
    expect(card?.querySelector("[data-component='task-tool-title']")?.textContent).toBe("Build");
    expect(card?.closest("button")?.getAttribute("aria-label")).toBe("Open delegated agent session: Run the tests");
    const surface = card?.closest<HTMLButtonElement>("[data-slot='collapsible-trigger']");
    act(() => surface?.click());
    expect(storeState.reopenSession).toHaveBeenCalledWith("session-child");
  });

  it("leaves synthetic text alone when it is not a subagent dispatch", async () => {
    await act(async () => root.render(
      <OpenCodeTimeline
        transcript={[{ kind: "synthetic", id: "synthetic-1", text: "plain system note", description: "Note" }]}
        busy={false}
        lastAssistantId={null}
      />
    ));

    expect(container.querySelector("[data-component='task-tool-card']")).toBeNull();
    expect(container.querySelector("[data-component='session-message']")).not.toBeNull();
  });
});
