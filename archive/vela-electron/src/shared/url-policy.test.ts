import { describe, expect, it } from "vitest";
import { safeExternalUrl } from "./url-policy";

describe("external URL policy", () => {
  it("allows only credential-free HTTPS URLs", () => {
    expect(safeExternalUrl("https://example.com/docs?q=1#intro")).toBe("https://example.com/docs?q=1#intro");
    expect(safeExternalUrl("https://user:secret@example.com/")).toBeNull();
    expect(safeExternalUrl("http://example.com/")).toBeNull();
    expect(safeExternalUrl("file:///tmp/report.txt")).toBeNull();
    expect(safeExternalUrl("javascript:alert(1)")).toBeNull();
    expect(safeExternalUrl("vscode://file/tmp/report.txt")).toBeNull();
    expect(safeExternalUrl("not a url")).toBeNull();
  });
});
