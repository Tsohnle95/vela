import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { SessionInfo } from "@shared/types";
import { useSessionLifecycle, useSessionLifecycleAuthority } from "./use-session-lifecycle";

function rejectAuthorityMutation(authority: ReturnType<typeof useSessionLifecycleAuthority>) {
  // @ts-expect-error Panel arrays are borrowed views.
  authority.panels.push(authority.session!);
  // @ts-expect-error Workspace-only membership belongs to lifecycle authority.
  authority.workspaceOnlyPanelIDs.add("other");
  // @ts-expect-error Rendered focus cannot be assigned by consumers.
  authority.activeSessionID = "other";
  // @ts-expect-error Named panel queries expose borrowed arrays.
  authority.readPanels().pop();
  // @ts-expect-error Borrowed panel metadata cannot change.
  authority.readPanels()[0].directory = "/other";
  // @ts-expect-error Borrowed focus metadata cannot change.
  authority.readFocusedSession()!.id = "other";
  // @ts-expect-error Borrowed workspace generations cannot change.
  authority.panelForSession("session")!.workspace.generation++;
  // @ts-expect-error Raw setters stay behind the module-private symbol.
  authority.setPanels([]);
  // @ts-expect-error Raw refs stay behind the module-private symbol.
  authority.sessionRef.current = null;
  const hiddenKey = Symbol() as Extract<keyof typeof authority, symbol>;
  // @ts-expect-error Reflecting the private brand never exposes its authority value.
  authority[hiddenKey].panelsRef.current = [];
  // @ts-expect-error Acquisition identities cannot be modified by consumers.
  authority.activationSeqRef.current++;
}
void rejectAuthorityMutation;

const panel: SessionInfo = { id: "session", directory: "/workspace", workspace: { id: "11111111-1111-4111-8111-111111111111", generation: 1 } };
let root: Root;
let container: HTMLDivElement;
beforeEach(() => { vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true); container = document.createElement("div"); document.body.append(container); root = createRoot(container); });
afterEach(() => { act(() => root.unmount()); container.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
function mount() {
  let authority!: ReturnType<typeof useSessionLifecycleAuthority>;
  let workflows!: ReturnType<typeof useSessionLifecycle>;
  const read = vi.fn(() => undefined);
  const command = vi.fn(() => {});
  const asyncCommand = vi.fn(async () => {});
  const hasTranscript = vi.fn(() => true);
  function Probe() {
    authority = useSessionLifecycleAuthority();
    authority.mirrorRenderedState(authority.panels, authority.session);
    workflows = useSessionLifecycle(authority,
      { addActiveSession: command, removeActiveSession: command, activeSessionFor: read, summaryFor: read, loadSessions: asyncCommand },
      { rememberWorkspace: command, replaceSavedWorkspace: command },
      { hideNestedWorkspace: command, restoreNestedWorkspaceVisibility: command, loadRecovery: asyncCommand },
      { dirtyTabsFor: () => [], saveCapturedSnapshot: async () => true, setSingleFileFor: command, openFile: asyncCommand },
      { refreshInbox: asyncCommand, refreshForms: asyncCommand, loadModels: asyncCommand, loadAgents: asyncCommand },
      { hasTranscript, hydrateTranscript: asyncCommand, applyReopenedSnapshot: command },
      { busyFor: () => false, dropClosedActivity: command },
      { notify: command, closeMenu: command, cancelPending: command, forgetClosedSessionRecords: command }
    );
    return null;
  }
  act(() => root.render(<Probe />));
  return { authority: () => authority, workflows: () => workflows, command, read, asyncCommand, hasTranscript };
}
it("initializes both composition stages without invoking domain ports", () => {
  const owner = mount();
  expect(owner.command).not.toHaveBeenCalled();
  expect(owner.read).not.toHaveBeenCalled();
  expect(owner.asyncCommand).not.toHaveBeenCalled();
  expect(owner.hasTranscript).not.toHaveBeenCalled();
  expect(owner.authority().panels).toEqual([]);
  expect(owner.authority().readFocusedSession()).toBeNull();
});
it("publishes attachment and focus mirrors synchronously while retaining query and workflow identities", () => {
  const owner = mount();
  const firstAuthority = owner.authority();
  const firstWorkflows = owner.workflows();
  act(() => {
    firstWorkflows.attachPanel(panel);
    expect(firstAuthority.readPanels()).toEqual([panel]);
    expect(firstAuthority.panelFor(panel.workspace)).toBe(panel);
    firstWorkflows.focusSession(panel.id);
    expect(firstAuthority.readFocusedSession()).toBe(panel);
    expect(firstAuthority.hasUserActivated()).toBe(true);
  });
  expect(owner.authority().panels[0]).toBe(panel);
  expect(owner.authority().session).toBe(panel);
  expect(owner.authority().readPanels).toBe(firstAuthority.readPanels);
  expect(owner.authority().readFocusedSession).toBe(firstAuthority.readFocusedSession);
  expect(owner.authority().nextRequest).toBe(firstAuthority.nextRequest);
  expect(owner.workflows().focusSession).toBe(firstWorkflows.focusSession);
  expect(owner.workflows().attachPanel).toBe(firstWorkflows.attachPanel);
  expect(firstAuthority.nextRequest()).toBe(1);
  expect(owner.authority().nextRequest()).toBe(2);
});
