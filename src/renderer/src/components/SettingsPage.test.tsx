import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import QRCode from "qrcode";
import { ThemeProvider } from "../theme";
import { SettingsPage } from "./SettingsPage";
import { SettingsSidebar } from "./SettingsSidebar";
import type { MiraAppUpdateStatus } from "@shared/types";

vi.mock("qrcode", () => ({
  default: { toDataURL: vi.fn(async () => "data:image/png;base64,test-pairing-code") },
}));

type MockModel = { id: string; providerID: string; name: string };
type MockSession = { directory: string; workspace: { id: string; generation: number } };

const store = {
  session: null as MockSession | null,
  unsavedEditorFileCount: 0,
  unsavedEditorFiles: [] as Array<{ workspaceID: string; sessionID: string | null; workspace: { id: string; generation: number } | null; path: string; displayPath: string; standalone: boolean }>,
  focusSession: vi.fn(),
  openFile: vi.fn(async () => {}),
  openExternalPath: vi.fn(async () => null),
  runtimes: [],
  models: [] as MockModel[],
  currentModel: null as MockModel | null,
  switchModel: vi.fn(),
  providerUsage: [],
  refreshProviderUsage: vi.fn(),
  loadModels: vi.fn(),
  refreshRuntimes: vi.fn(async () => []),
  approvalMode: "ask",
  toggleApprovalMode: vi.fn(),
  followUpBehavior: "queue",
  setFollowUpBehavior: vi.fn()
};

vi.mock("../store", () => ({ useStore: () => store }));

const setAppearance = vi.fn().mockResolvedValue(undefined);
const mobileSetupStatus = vi.fn().mockResolvedValue({
  state: "ready",
  workspacePath: "/work/mira-mobile",
  port: 3011,
});
const mobilePairingQr = vi.fn().mockResolvedValue({
  connectionUrl: "orbit://connect?v=2&p=one-time",
  expiresAt: "2099-01-01T00:00:00.000Z",
  serverLabel: "Mira Desktop",
});
const checkAppUpdate = vi.fn();
const updateApp = vi.fn();
let isReleaseBuild = true;
let appVersion = "0.1.0";

describe("SettingsPage", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    window.localStorage.clear();
    store.unsavedEditorFileCount = 0;
    store.unsavedEditorFiles = [];
    store.focusSession.mockClear();
    store.openFile.mockClear();
    store.openExternalPath.mockClear();
    isReleaseBuild = true;
    appVersion = "0.1.0";
    delete document.documentElement.dataset.theme;
    window.openshell = { setAppearance, mobileSetupStatus, mobilePairingQr, checkAppUpdate, updateApp, isReleaseBuild, appVersion } as unknown as typeof window.openshell;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    window.localStorage.clear();
    document.documentElement.removeAttribute("data-shell");
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  it("picks the interface independently of the palette", () => {
    act(() => root.render(<ThemeProvider><SettingsPage section="appearance" onClose={() => {}} /></ThemeProvider>));

    const choices = [...container.querySelectorAll<HTMLButtonElement>(".ui-choice-card")];
    expect(choices.map((card) => card.textContent)).toEqual([
      expect.stringContaining("Codex interface"),
      expect.stringContaining("Classic interface")
    ]);
    expect(choices.map((card) => card.getAttribute("aria-checked"))).toEqual(["true", "false"]);
    expect(document.documentElement.dataset.shell).toBe("codex");

    act(() => choices[1]!.click());

    expect(document.documentElement.dataset.shell).toBe("classic");
    expect(window.localStorage.getItem("mira.ui")).toBe("classic");
    const after = [...container.querySelectorAll<HTMLButtonElement>(".ui-choice-card")];
    expect(after.map((card) => card.getAttribute("aria-checked"))).toEqual(["false", "true"]);
    // The palette is untouched by an interface switch.
    expect(document.documentElement.dataset.theme).toBe("codex");
  });

  it("defaults to Codex, persists color-profile selection, restores it, and tracks native appearance", () => {
    act(() => root.render(<ThemeProvider><SettingsPage section="appearance" onClose={() => {}} /></ThemeProvider>));

    const cards = [...container.querySelectorAll<HTMLButtonElement>(".theme-card")];
    expect(cards.map((card) => card.textContent)).toEqual([
      expect.stringContaining("Codex"),
      expect.stringContaining("Codex Dark"),
      expect.stringContaining("Quiet Habitat"),
      expect.stringContaining("Control Deck"),
      expect.stringContaining("Papertrail"),
      expect.stringContaining("Shell First"),
      expect.stringContaining("Swiss Grid"),
      expect.stringContaining("Prism"),
      expect.stringContaining("Workshop"),
      expect.stringContaining("Blueprint"),
      expect.stringContaining("Stillness"),
      expect.stringContaining("Bloom"),
      expect.stringContaining("Kitty Glass"),
      expect.stringContaining("Original Dark"),
      expect.stringContaining("Observatory")
    ]);
    expect(document.documentElement.dataset.theme).toBe("codex");
    expect(window.localStorage.getItem("mira.theme")).toBe("codex");
    expect(setAppearance).toHaveBeenLastCalledWith("light");

    act(() => cards[4].click());
    expect(document.documentElement.dataset.theme).toBe("papertrail");
    expect(cards[4].getAttribute("aria-checked")).toBe("true");
    expect(setAppearance).toHaveBeenLastCalledWith("light");

    act(() => root.unmount());
    container.remove();
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<ThemeProvider><SettingsPage section="appearance" onClose={() => {}} /></ThemeProvider>));
    expect(document.documentElement.dataset.theme).toBe("papertrail");
    expect(setAppearance).toHaveBeenLastCalledWith("light");

    const restored = [...container.querySelectorAll<HTMLButtonElement>(".theme-card")];
    act(() => restored[7].click());
    expect(document.documentElement.dataset.theme).toBe("prism");
    expect(window.localStorage.getItem("mira.theme")).toBe("prism");
    expect(setAppearance).toHaveBeenLastCalledWith("dark");

    act(() => restored[2].click());
    expect(document.documentElement.dataset.theme).toBe("quiet-habitat");
    expect(window.localStorage.getItem("mira.theme")).toBe("quiet-habitat");
    expect(setAppearance).toHaveBeenLastCalledWith("light");
  });

  it.each(["kitty", "original"] as const)("restores the saved %s appearance without remapping it", (saved) => {
    window.localStorage.setItem("mira.theme", saved);
    act(() => root.render(<ThemeProvider><SettingsPage section="appearance" onClose={() => {}} /></ThemeProvider>));

    expect(document.documentElement.dataset.theme).toBe(saved);
    expect(container.querySelector<HTMLButtonElement>(`.theme-card[aria-checked="true"]`)?.textContent)
      .toContain(saved === "kitty" ? "Kitty Glass" : "Original Dark");
    expect(setAppearance).toHaveBeenLastCalledWith("dark");
    // The Codex shell paints one flat workspace for every palette …
    expect(document.documentElement.style.getPropertyValue("--workspace-background"))
      .toBe(saved === "kitty" ? "#0e121c" : "#171412");

    // … while the palette's own treatment (Kitty's glass) belongs to the
    // classic shell.
    act(() => container.querySelectorAll<HTMLButtonElement>(".ui-choice-card")[1]!.click());
    expect(document.documentElement.dataset.shell).toBe("classic");
    expect(document.documentElement.style.getPropertyValue("--workspace-background"))
      .toBe(saved === "kitty" ? "transparent" : "#171412");
  });

  it("restores Kitty Glass transparency and clears it when choosing another profile", () => {
    act(() => root.render(<ThemeProvider><SettingsPage section="appearance" onClose={() => {}} /></ThemeProvider>));
    const cards = [...container.querySelectorAll<HTMLButtonElement>(".theme-card")];
    act(() => cards[12].click());

    expect(document.documentElement.dataset.theme).toBe("kitty");
    expect(window.localStorage.getItem("mira.theme")).toBe("kitty");
    expect(document.documentElement.style.getPropertyValue("--bg")).toBe("transparent");
    expect(document.documentElement.style.getPropertyValue("--panel-surface-image")).toContain("radial-gradient");

    act(() => cards[7].click());
    expect(document.documentElement.dataset.theme).toBe("prism");
    expect(document.documentElement.style.getPropertyValue("--panel-surface-image")).toBe("none");
  });

  it("keeps editor wrapping enabled without a wrap control", () => {
    act(() => root.render(<ThemeProvider><SettingsPage section="appearance" onClose={() => {}} /></ThemeProvider>));
    expect(container.textContent).not.toContain("Word wrap");
    expect(container.querySelector('[role="switch"]')).toBeNull();
  });

  it("provides dedicated settings navigation with About as the final tab", () => {
    const onSectionChange = vi.fn();
    act(() => root.render(<SettingsSidebar section="appearance" onSectionChange={onSectionChange} />));

    const labels = [...container.querySelectorAll<HTMLButtonElement>(".settings-nav-item")].map((button) => button.textContent);
    expect(labels).toEqual(["Appearance", "Plugins", "Providers", "Safety", "Voice", "Model", "Servers", "Mobile Setup", "About"]);
    expect(container.querySelector<HTMLButtonElement>(".settings-nav-item:last-child")?.textContent).toBe("About");

    act(() => container.querySelectorAll<HTMLButtonElement>(".settings-nav-item")[5].click());
    expect(onSectionChange).toHaveBeenCalledWith("model");
  });

  it("checks GitHub Releases on About and starts the popup updater from Update Mira", async () => {
    const confirm = vi.fn(() => true);
    vi.stubGlobal("confirm", confirm);
    checkAppUpdate.mockResolvedValue({
      state: "available",
      currentVersion: "0.1.0",
      latestVersion: "0.2.0"
    });
    updateApp.mockResolvedValue({
      ok: true,
      updated: true,
      currentVersion: "0.1.0",
      latestVersion: "0.2.0",
      message: "Mira 0.2.0 is downloaded and ready to install."
    });
    await act(async () => root.render(<ThemeProvider><SettingsPage section="about" onClose={() => {}} /></ThemeProvider>));
    expect(checkAppUpdate).toHaveBeenCalledOnce();
    expect(container.textContent).toContain("Mira 0.1.0 is installed · release 0.2.0 is available");
    expect(container.textContent).toContain("GitHub commits become app updates after a release is published.");
    const updateButton = [...container.querySelectorAll<HTMLButtonElement>(".settings-action-button")]
      .find((button) => button.textContent === "Update Mira")!;
    expect(updateButton).toBeTruthy();
    expect([...container.querySelectorAll<HTMLButtonElement>(".settings-action-button")]
      .some((button) => button.textContent === "Check for updates")).toBe(true);

    await act(async () => {
      updateButton.click();
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(confirm).not.toHaveBeenCalled();
    expect(updateApp).toHaveBeenCalledOnce();
    expect(container.textContent).toContain("Mira 0.2.0 is downloaded and ready to install.");
    expect([...container.querySelectorAll<HTMLButtonElement>(".settings-action-button")]
      .some((button) => button.textContent === "Check for updates")).toBe(true);
  });

  it("starts an update while the initial status check is pending and ignores its stale result", async () => {
    let resolveCheck!: (status: MiraAppUpdateStatus) => void;
    const pendingCheck = new Promise<MiraAppUpdateStatus>((resolve) => { resolveCheck = resolve; });
    checkAppUpdate.mockReturnValueOnce(pendingCheck);
    updateApp.mockResolvedValue({
      ok: true,
      updated: true,
      currentVersion: "0.1.0",
      latestVersion: "0.2.0",
      message: "Mira 0.2.0 is downloaded and ready to install."
    });

    await act(async () => root.render(<ThemeProvider><SettingsPage section="about" onClose={() => {}} /></ThemeProvider>));
    const updateButton = [...container.querySelectorAll<HTMLButtonElement>(".settings-action-button")]
      .find((button) => button.textContent === "Update Mira")!;
    expect(updateButton.disabled).toBe(false);

    await act(async () => {
      updateButton.click();
      await Promise.resolve();
    });
    expect(updateApp).toHaveBeenCalledOnce();
    expect(container.textContent).toContain("Mira 0.2.0 is downloaded and ready to install.");

    await act(async () => resolveCheck({
      state: "available",
      currentVersion: "0.1.0",
      latestVersion: "0.3.0"
    }));
    expect(container.textContent).not.toContain("0.3.0 is available");
    expect(container.textContent).toContain("Mira 0.2.0 is downloaded and ready to install.");
  });

  it("keeps local development builds away from the GitHub release updater", async () => {
    isReleaseBuild = false;
    window.openshell = { ...window.openshell, isReleaseBuild } as typeof window.openshell;

    await act(async () => root.render(<ThemeProvider><SettingsPage section="about" onClose={() => {}} /></ThemeProvider>));

    expect(checkAppUpdate).not.toHaveBeenCalled();
    expect(updateApp).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Version 0.1.0 · Development/test build");
    expect(container.textContent).toContain("GitHub app updates are disabled here.");
    expect(container.textContent).toContain("Updater disabled");
    expect(container.textContent).not.toContain("Check for updates");
    expect(container.textContent).not.toContain("Update Mira");
  });

  it("requires unsaved editor files to be saved before an update can relaunch Mira", async () => {
    store.unsavedEditorFileCount = 2;
    store.unsavedEditorFiles = [
      { workspaceID: "workspace-1", sessionID: "session-1", workspace: { id: "workspace-1", generation: 1 }, path: "src/first.ts", displayPath: "/repo/src/first.ts", standalone: false },
      { workspaceID: "workspace-1", sessionID: "session-1", workspace: { id: "workspace-1", generation: 1 }, path: "src/second.ts", displayPath: "/repo/src/second.ts", standalone: false }
    ];
    checkAppUpdate.mockResolvedValue({
      state: "current",
      currentVersion: "0.1.0",
      latestVersion: "0.1.0"
    });
    await act(async () => root.render(<ThemeProvider><SettingsPage section="about" onClose={() => {}} /></ThemeProvider>));
    const updateButton = [...container.querySelectorAll<HTMLButtonElement>(".settings-action-button")]
      .find((button) => button.textContent === "Update Mira")!;

    await act(async () => updateButton.click());

    expect(updateApp).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Save 2 unsaved files before Mira installs the release and relaunches.");
    expect(container.textContent).toContain("/repo/src/first.ts");
    expect(container.textContent).toContain("/repo/src/second.ts");
  });

  it("opens the selected unsaved file in its workspace", async () => {
    store.unsavedEditorFileCount = 1;
    const workspace = { id: "workspace-1", generation: 1 };
    store.unsavedEditorFiles = [{
      workspaceID: workspace.id,
      sessionID: "session-1",
      workspace,
      path: "src/first.ts",
      displayPath: "/repo/src/first.ts",
      standalone: false
    }];
    const onClose = vi.fn();
    await act(async () => root.render(<ThemeProvider><SettingsPage section="about" onClose={onClose} /></ThemeProvider>));

    const open = container.querySelector<HTMLButtonElement>(".settings-unsaved-file")!;
    await act(async () => open.click());

    expect(store.focusSession).toHaveBeenCalledWith("session-1");
    expect(store.openFile).toHaveBeenCalledWith("src/first.ts", workspace);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("explains that Mira must be reopened when the running main process has no updater handler", async () => {
    checkAppUpdate.mockRejectedValueOnce(new Error(
      "Error invoking remote method 'shell:app-update-check': Error: No handler registered for shell:app-update-check"
    ));

    await act(async () => root.render(<ThemeProvider><SettingsPage section="about" onClose={() => {}} /></ThemeProvider>));

    expect(container.textContent).toContain("Quit Mira completely and reopen it, then check again.");
    expect(container.textContent).not.toContain("No handler registered");
  });

  it("keeps default model settings and removes runtime selection", async () => {
    const workspace = { id: "workspace-1", generation: 1 };
    store.session = { directory: "/repo", workspace };
    store.models = [{ id: "model-1", providerID: "provider-1", name: "Model One" }];
    store.currentModel = store.models[0];

    await act(async () => root.render(<ThemeProvider><SettingsPage section="model" onClose={() => {}} /></ThemeProvider>));

    expect(container.textContent).toContain("Default model");
    expect(container.textContent).not.toContain("Agent runtime");
    expect(store.loadModels).toHaveBeenCalledWith(workspace);
    expect(container.querySelector<HTMLSelectElement>(".settings-list-row select")?.value).toBe("provider-1:model-1");

    store.session = null;
    store.models = [];
    store.currentModel = null;
  });

  it("shows the desktop-to-phone pairing walkthrough in Mobile Setup", async () => {
    await act(async () => {
      root.render(<ThemeProvider><SettingsPage section="mobile" onClose={() => {}} /></ThemeProvider>);
    });

    expect(container.textContent).toContain("Continue your workspace on your phone.");
    expect(container.textContent).toContain("Prepare this Mac");
    expect(container.textContent).toContain("Open the mobile companion app");
    expect(container.textContent).toContain("Scan the code from this page");
    expect(container.querySelector<HTMLButtonElement>(".mobile-setup-generate")?.disabled).toBe(false);
    expect(container.textContent).toContain("Generate pairing QR");
    expect(mobileSetupStatus).toHaveBeenCalledTimes(1);
  });

  it("generates the pairing QR from the one-time URI returned by main", async () => {
    await act(async () => {
      root.render(<ThemeProvider><SettingsPage section="mobile" onClose={() => {}} /></ThemeProvider>);
    });
    const generate = container.querySelector<HTMLButtonElement>(".mobile-setup-generate");
    expect(generate?.disabled).toBe(false);

    await act(async () => {
      generate?.click();
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    const qr = container.querySelector<HTMLImageElement>(".mobile-setup-qr-result img");
    expect(mobilePairingQr).toHaveBeenCalledTimes(1);
    expect(QRCode.toDataURL).toHaveBeenCalledWith("orbit://connect?v=2&p=one-time", {
      width: 480,
      margin: 2,
      errorCorrectionLevel: "L",
    });
    expect(qr?.src).toBe("data:image/png;base64,test-pairing-code");
    expect(qr?.alt).toContain("One-time mobile companion pairing QR");
    expect(container.textContent).toContain("One use · expires in 10 minutes");
  });
});
