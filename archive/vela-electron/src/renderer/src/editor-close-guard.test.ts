import { describe, expect, it } from "vitest";
import { protectEditorUnload } from "./editor-close-guard";

describe("protectEditorUnload", () => {
  it("blocks window unload while editor files are dirty", () => {
    const event = new Event("beforeunload", { cancelable: true }) as BeforeUnloadEvent;

    protectEditorUnload(event, 2);

    expect(event.defaultPrevented).toBe(true);
  });

  it("allows window unload when every editor file is saved", () => {
    const event = new Event("beforeunload", { cancelable: true }) as BeforeUnloadEvent;

    protectEditorUnload(event, 0);

    expect(event.defaultPrevented).toBe(false);
  });
});
