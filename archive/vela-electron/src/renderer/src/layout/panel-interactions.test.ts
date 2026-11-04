import { describe, expect, it } from "vitest";
import { titlebarLeadingWidth } from "./panel-interactions";

describe("titlebar leading block width", () => {
  it("keeps the titlebar controls whole when the workspace starts to their left", () => {
    expect(titlebarLeadingWidth(-36, 26)).toBe(26);
    expect(titlebarLeadingWidth(0, 26)).toBe(26);
  });

  it("back-aligns the tab strip with the workspace edge when there is room", () => {
    expect(titlebarLeadingWidth(194.4, 26)).toBe(194);
  });
});
