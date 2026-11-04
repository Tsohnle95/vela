export type WindowView = "landing" | "session";

export interface WindowSize {
  width: number;
  height: number;
}

export const MIN_WINDOW_SIZE = 200;
export const DEFAULT_SESSION_SIZE: WindowSize = { width: 1280, height: 800 };

export function isWindowSize(value: unknown): value is WindowSize {
  if (!value || typeof value !== "object") return false;
  const size = value as Record<string, unknown>;
  return (
    typeof size.width === "number" &&
    Number.isFinite(size.width) &&
    size.width >= MIN_WINDOW_SIZE &&
    typeof size.height === "number" &&
    Number.isFinite(size.height) &&
    size.height >= MIN_WINDOW_SIZE
  );
}

export function parseSessionBounds(raw: string | null | undefined): WindowSize | null {
  if (!raw) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!parsed || typeof parsed !== "object") return null;
  if (isWindowSize(parsed)) {
    const { width, height } = parsed as WindowSize;
    return { width, height };
  }
  const record = parsed as Record<string, unknown>;
  if (record.version === 2 && isWindowSize(record.session)) return record.session;
  return null;
}

export function serializeSessionBounds(size: WindowSize): string {
  return JSON.stringify({ version: 2, session: size });
}

export function isWindowView(value: unknown): value is WindowView {
  return value === "landing" || value === "session";
}
