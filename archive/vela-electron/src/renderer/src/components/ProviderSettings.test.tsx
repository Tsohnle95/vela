import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProviderIntegration } from "@shared/types";
import type { VelaApi } from "../../../preload";
import { ProviderSettings } from "./ProviderSettings";

const workspace = { id: "11111111-1111-4111-8111-111111111111", generation: 1 };
const openai: ProviderIntegration = {
  id: "openai",
  name: "OpenAI",
  keyMethod: { fields: [] },
  credentials: [],
  environment: { names: ["OPENAI_API_KEY"], connected: [] },
  oauth: [{ id: "oauth", label: "ChatGPT Pro/Plus" }]
};
const opencodeGo: ProviderIntegration = {
  id: "opencode-go",
  name: "OpenCode Go",
  keyMethod: {
    label: "API key",
    fields: [{ key: "org", type: "string", title: "Organization", required: true }]
  },
  credentials: [],
  environment: { names: ["OPENCODE_API_KEY"], connected: [] },
  oauth: []
};

function type(input: HTMLInputElement, value: string): void {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("ProviderSettings", () => {
  let container: HTMLDivElement;
  let root: Root;
  let previousApi: VelaApi;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    previousApi = window.vela;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    window.vela = previousApi;
    container.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("connects a provider key with provider-specific fields and never redisplays the secret", async () => {
    const connected = { ...opencodeGo, credentials: [{ id: "credential-1", label: "work" }] };
    const providerIntegrations = vi.fn()
      .mockResolvedValueOnce([openai, opencodeGo])
      .mockResolvedValueOnce([openai, connected]);
    const connectProviderKey = vi.fn().mockResolvedValue(undefined);
    const refreshModels = vi.fn().mockResolvedValue(undefined);
    window.vela = { ...previousApi, providerIntegrations, connectProviderKey };

    await act(async () => root.render(<ProviderSettings workspace={workspace} usage={[]} refreshModels={refreshModels} />));
    await act(async () => {});
    const card = [...container.querySelectorAll<HTMLElement>(".provider-card")].find((item) => item.textContent?.includes("OpenCode Go"))!;
    act(() => [...card.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent === "Add key")!.click());
    const inputs = card.querySelectorAll<HTMLInputElement>("input");
    act(() => {
      type(inputs[0], "go-secret");
      type(inputs[1], "work");
      type(inputs[2], "my-org");
    });
    await act(async () => card.querySelector<HTMLFormElement>("form")!.requestSubmit());

    expect(connectProviderKey).toHaveBeenCalledWith(workspace, "opencode-go", "go-secret", "work", { org: "my-org" });
    expect(refreshModels).toHaveBeenCalled();
    expect(container.textContent).toContain("Connected");
    expect(container.textContent).not.toContain("go-secret");
  });

  it("renders every runtime-reported provider with no catalog browsing and removes opaque credentials", async () => {
    const connected = { ...openai, credentials: [{ id: "credential-1", label: "default" }] };
    const providerIntegrations = vi.fn().mockResolvedValue([connected, opencodeGo]);
    const removeProviderCredential = vi.fn().mockResolvedValue(undefined);
    window.vela = { ...previousApi, providerIntegrations, removeProviderCredential };
    vi.spyOn(window, "confirm").mockReturnValue(true);

    await act(async () => root.render(<ProviderSettings workspace={workspace} usage={[]} refreshModels={async () => {}} />));
    await act(async () => {});
    expect(container.textContent).toContain("OpenAI");
    expect(container.textContent).toContain("OpenCode Go");
    expect(container.querySelector("[aria-label='Search providers']")).toBeNull();
    expect(container.textContent).not.toContain("Browse all");

    await act(async () => [...container.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent === "Remove")!.click());
    expect(removeProviderCredential).toHaveBeenCalledWith(workspace, "credential-1");
  });

  it("clears an unsaved secret when the connection form is cancelled", async () => {
    window.vela = { ...previousApi, providerIntegrations: vi.fn().mockResolvedValue([openai]) };

    await act(async () => root.render(<ProviderSettings workspace={workspace} usage={[]} refreshModels={async () => {}} />));
    await act(async () => {});
    act(() => [...container.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent === "Add key")!.click());
    const secret = container.querySelector<HTMLInputElement>("input[type='password']")!;
    act(() => type(secret, "do-not-retain"));
    act(() => [...container.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent === "Cancel")!.click());
    act(() => [...container.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent === "Add key")!.click());

    expect(container.querySelector<HTMLInputElement>("input[type='password']")?.value).toBe("");
    expect(container.textContent).not.toContain("do-not-retain");
  });
});
