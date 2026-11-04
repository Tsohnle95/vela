import { describe, expect, it, vi } from "vitest";
import { migrateUserDataDirectory } from "./user-data-migration";

describe("user data folder rebrand", () => {
  it("moves the first existing product data folder to the new location", () => {
    const exists = vi.fn((path: string) => path === "/support/current");
    const rename = vi.fn();

    expect(migrateUserDataDirectory("/support/vela", ["/support/current", "/support/other"], false, exists, rename)).toBeNull();
    expect(rename).toHaveBeenCalledWith("/support/current", "/support/vela");
  });

  it("keeps a custom or already created app data folder untouched", () => {
    const rename = vi.fn();

    expect(migrateUserDataDirectory("/support/vela", ["/support/current"], true, () => true, rename)).toBeNull();
    expect(migrateUserDataDirectory("/support/vela", ["/support/current"], false, (path) => path === "/support/vela", rename)).toBeNull();
    expect(rename).not.toHaveBeenCalled();
  });

  it("falls back to the prior folder when a move fails", () => {
    const previous = "/support/current";
    expect(migrateUserDataDirectory("/support/vela", [previous], false, (path) => path === previous, () => { throw new Error("move failed"); }))
      .toBe(previous);
  });
});
