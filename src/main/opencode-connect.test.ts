// @vitest-environment node
import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  discover: vi.fn(),
  ensure: vi.fn(),
  openCodeMake: vi.fn()
}));

vi.mock("electron", () => ({ shell: { trashItem: vi.fn(), openPath: vi.fn() } }));
vi.mock("@opencode/client", () => ({ OpenCode: { make: hoisted.openCodeMake } }));
vi.mock("@opencode/client/service", () => ({
  Service: { discover: hoisted.discover, ensure: hoisted.ensure, headers: () => undefined }
}));

import packageJson from "../../package.json";
import { VelaBackend, serverVersionPredicate } from "./opencode";

describe("serverVersionPredicate", () => {
  it("accepts the pinned V2 SDK's service version", () => {
    const sdkVersion = packageJson.dependencies["@opencode/client"];
    expect(serverVersionPredicate(`opencode v${sdkVersion}`)(sdkVersion)).toBe(true);
  });

  it("accepts compatible V2 services and rejects V1 and legacy beta versions", () => {
    const accept = serverVersionPredicate("opencode v2.0.15");
    expect(accept("2.0.15")).toBe(true);
    expect(accept("2.0.16")).toBe(true);
    expect(accept("2.1.0")).toBe(true);
    expect(accept("1.18.32")).toBe(false);
    expect(accept("0.0.0-beta-19242")).toBe(false);
    expect(accept("not-a-version")).toBe(false);
  });

  it("rejects incompatible services when the installed CLI version is unknown", () => {
    const accept = serverVersionPredicate(null);
    expect(accept("2.0.15")).toBe(true);
    expect(accept("1.18.32")).toBe(false);
  });
});

describe("VelaBackend.connect", () => {
  const stubInstalledVersion = (backend: VelaBackend, version: string | null): void => {
    (backend as unknown as { installedServerVersion: () => Promise<string | null> }).installedServerVersion =
      async () => version;
  };

  beforeEach(() => {
    hoisted.discover.mockReset();
    hoisted.ensure.mockReset();
    hoisted.openCodeMake.mockReset();
    hoisted.openCodeMake.mockReturnValue({ session: { list: vi.fn(async () => []) } });
  });

  it("reuses a running V2 service", async () => {
    const endpoint = { url: "http://127.0.0.1:1", auth: undefined };
    hoisted.discover.mockResolvedValue(endpoint);
    const backend = new VelaBackend();
    stubInstalledVersion(backend, "opencode v2.0.15");

    await expect(backend.connect()).resolves.toBe(true);

    const options = hoisted.discover.mock.calls[0][0] as { version: (value: string) => boolean };
    expect(options.version("2.0.15")).toBe(true);
    expect(options.version("2.1.0")).toBe(true);
    expect(options.version("1.18.32")).toBe(false);
    expect(hoisted.ensure).not.toHaveBeenCalled();
  });

  it("replaces an incompatible legacy daemon by ensuring the V2 service", async () => {
    const legacyVersion = "0.0.0-beta-19242";
    hoisted.discover.mockImplementation(async (options: { version: (value: string) => boolean }) =>
      options.version(legacyVersion) ? { url: "http://127.0.0.1:1", auth: undefined } : undefined
    );
    hoisted.ensure.mockResolvedValue({ url: "http://127.0.0.1:2", auth: undefined });
    const backend = new VelaBackend();
    stubInstalledVersion(backend, "opencode v2.0.15");

    await expect(backend.connect()).resolves.toBe(true);

    expect(hoisted.ensure).toHaveBeenCalledTimes(1);
    const options = hoisted.ensure.mock.calls[0][0] as {
      command: string[];
      version: (value: string) => boolean;
    };
    expect(options.command).toEqual(["opencode", "serve", "--service"]);
    expect(options.version("2.0.15")).toBe(true);
    expect(options.version(legacyVersion)).toBe(false);
  });
});
