import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PendingFormRequest, SessionInfo, TranscriptItem } from "@shared/types";
import { ThemeProvider } from "../theme";

const terminalWrites = vi.hoisted(() => vi.fn());

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 80;
    rows = 24;
    options = { theme: {}, fontFamily: "", fontWeight: 400, fontSize: 12, lineHeight: 1.2 };
    loadAddon() {}
    open() {}
    onData() {
      return { dispose() {} };
    }
    write(data: string) {
      terminalWrites(data);
    }
    dispose() {}
  }
}));
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { fit() {} } }));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

let currentSession: SessionInfo;
let currentTranscript: TranscriptItem[] = [];
let currentPendingForms: PendingFormRequest[] = [];
const replyPermission = vi.fn(async () => {});
const agentTuiStart = vi.fn(async () => {});

vi.mock("../store", () => ({
  useStore: () => ({
    session: currentSession,
    sessions: [currentSession],
    approvalMode: "ask",
    toggleApprovalMode: vi.fn(),
    reopenSession: vi.fn(),
    providerUsage: [],
    providerUsageLoading: false,
    refreshProviderUsage: vi.fn(),
    selectPanelDirectory: vi.fn(),
    commitStagedRevert: vi.fn(),
    clearStagedRevert: vi.fn(),
    runtimes: [],
    selectFolder: vi.fn(),
    replyPermission
  }),
  usePanel: () => ({
    session: currentSession,
    busy: false,
    transcript: currentTranscript,
    todos: [],
    sessionUsage: null,
    compactionBaseline: null,
    currentModel: null,
    turnStartedAt: null,
    assistantStatus: null,
    models: [],
    agents: [],
    currentAgent: null,
    pendingForms: currentPendingForms,
    queuedMessages: [],
    stagedRevert: null
  })
}));

import { AgentPanel } from "./AgentPanel";

const pendingPermission: TranscriptItem = {
  kind: "permission",
  id: "perm-1",
  requestID: "req-1",
  action: "bash",
  resources: ["rm -rf build"],
  pending: true
};

const pendingQuestion: PendingFormRequest = {
  id: "form-1",
  sessionID: "one",
  title: "Which package manager?",
  fields: [{ key: "manager", title: "Package manager", type: "string", required: true }]
};

const dock = (kind: string): Element | null =>
  document.querySelector(`[data-component="dock-prompt"][data-kind="${kind}"]`);

describe("AgentPanel interactive prompt surface", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    vi.stubGlobal("ResizeObserver", class {
      observe() {}
      disconnect() {}
    });
    vi.stubGlobal("crypto", { randomUUID: () => "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa" });
    replyPermission.mockClear();
    agentTuiStart.mockClear();
    currentTranscript = [];
    currentPendingForms = [];
    window.vela = {
      onMessage: () => () => {},
      agentTuiStart,
      agentTuiInput: vi.fn(async () => {}),
      agentTuiResize: vi.fn(async () => {}),
      agentTuiStop: vi.fn(async () => {})
    } as unknown as typeof window.vela;
    currentSession = {
      id: "one",
      directory: "/workspace",
      workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 }
    };
    window.localStorage.clear();
    window.localStorage.setItem("vela.agent-panel-mode.one", "tui");
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  const renderPanel = async (): Promise<void> => {
    await act(async () => root.render(
      <ThemeProvider>
        <AgentPanel session={currentSession} />
      </ThemeProvider>
    ));
  };

  const switchToGui = async (): Promise<void> => {
    window.localStorage.setItem("vela.agent-panel-mode.one", "gui");
    await act(async () => root.unmount());
    container.remove();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await renderPanel();
  };

  it("leaves the embedded TUI to own its own prompts", async () => {
    currentTranscript = [pendingPermission];
    currentPendingForms = [pendingQuestion];
    await renderPanel();

    expect(container.querySelector(".agent-tui-host")).not.toBeNull();
    expect(dock("permission")).toBeNull();
    expect(dock("form")).toBeNull();
  });

  it("renders the approval dock again in the GUI view", async () => {
    currentTranscript = [pendingPermission];
    await switchToGui();

    const permission = dock("permission");
    expect(permission).not.toBeNull();
    expect(permission?.textContent).toContain("Permission required");
    expect(permission?.textContent).toContain("bash");
    expect(permission?.textContent).toContain("rm -rf build");
  });

  it("renders the question dock again in the GUI view", async () => {
    currentPendingForms = [pendingQuestion];
    await switchToGui();

    expect(dock("form")).not.toBeNull();
    expect(dock("form")?.textContent).toContain("Which package manager?");
  });

  it("replies to the approval from the GUI dock", async () => {
    currentTranscript = [pendingPermission];
    await switchToGui();

    const allow = [...document.querySelectorAll("button")].find((button) => button.textContent === "Allow once")!;
    await act(async () => allow.click());

    expect(replyPermission).toHaveBeenCalledWith("req-1", "once", "one");
  });
});
