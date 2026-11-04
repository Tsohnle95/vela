import { describe, expect, it } from "vitest";
import { InspectPickerState } from "./inspect-picker";

describe("InspectPickerState", () => {
  it("allows only one node claim per picker activation", () => {
    const picker = new InspectPickerState();
    picker.begin();

    expect(picker.claim()).toBe(true);
    expect(picker.claim()).toBe(false);
    expect(picker.active).toBe(false);
  });

  it("invalidates an asynchronous activation when canceled", () => {
    const picker = new InspectPickerState();
    const token = picker.begin();
    picker.cancel();

    expect(picker.isCurrent(token)).toBe(false);
    expect(picker.active).toBe(false);
  });

  it("keeps a newer activation current when an older one finishes", () => {
    const picker = new InspectPickerState();
    const first = picker.begin();
    const second = picker.begin();

    expect(picker.isCurrent(first)).toBe(false);
    expect(picker.isCurrent(second)).toBe(true);
  });
});
