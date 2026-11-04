import type { IpcMainInvokeEvent } from "electron";

export type TrustedIpcRegistrar = <Args extends unknown[], Result>(
  channel: string,
  listener: (event: IpcMainInvokeEvent, ...args: Args) => Result
) => void;
