import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RecoveryRecord } from "@shared/types";

const state = vi.hoisted(() => ({
  records: [] as RecoveryRecord[],
  open: vi.fn(async () => {}),
  acknowledge: vi.fn(async () => {})
}));

vi.mock("../store", () => ({
  useStore: () => ({
    recoveryRecords: state.records,
    openRecovery: state.open,
    acknowledgeRecovery: state.acknowledge
  })
}));

import { RecoveryNotice } from "./RecoveryNotice";

afterEach(() => {
  state.records = [];
  state.open.mockClear();
  state.acknowledge.mockClear();
});

describe("RecoveryNotice", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  const record = (acknowledged: boolean): RecoveryRecord => ({
    id: "1786533818724-e85066e0-7d22-4d91-b476-ba097731f371:original",
    artifact: "original",
    originalPath: "save.txt",
    recoveryPath: ".vela-recovery/transaction/original",
    createdAt: 1,
    acknowledged,
    reason: "save-failed"
  });

  it("keeps abnormal recovery actionable", () => {
    state.records = [record(false)];
    act(() => root.render(<RecoveryNotice />));

    const buttons = [...container.querySelectorAll("button")];
    act(() => buttons.find((button) => button.textContent === "Open")!.click());
    act(() => buttons.find((button) => button.textContent === "Acknowledge")!.click());

    expect(state.open).toHaveBeenCalledWith(state.records[0].id);
    expect(state.acknowledge).toHaveBeenCalledWith(state.records[0].id);
  });

  it("hides acknowledged successful history", () => {
    state.records = [{ ...record(true), reason: "saved" }];
    act(() => root.render(<RecoveryNotice />));

    expect(container.innerHTML).toBe("");
  });
});
