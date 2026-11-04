export interface FailureInfo {
  code: string;
  message: string;
  details?: string;
}

function record(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : undefined;
}

function stringify(value: unknown): string | undefined {
  if (typeof value === "string" && value.trim()) return value;
  if (value == null) return undefined;
  try {
    const text = JSON.stringify(value);
    return text && text !== "{}" ? text : undefined;
  } catch {
    return String(value);
  }
}

function normalizeCode(value: string): string {
  return value.trim().replace(/[^A-Za-z0-9_.-]/g, "_").toUpperCase();
}

export function normalizeFailure(
  error: unknown,
  fallbackCode = "VELA_UNKNOWN_FAILURE",
  fallbackMessage = "Operation failed"
): FailureInfo {
  const source = record(error);
  const nested = record(source?.error) ?? record(source?.data);
  const values = { ...(nested ?? {}), ...(source ?? {}) };
  const rawMessage = typeof values.message === "string"
    ? values.message
    : typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : "";
  const marker = /^\[([A-Za-z][A-Za-z0-9_.-]*)\]\s*(.*)$/.exec(rawMessage.trim());
  const message = (marker?.[2] || rawMessage.trim() || fallbackMessage).trim();
  const rawCode = typeof values.code === "string"
    ? values.code
    : typeof values._tag === "string"
      ? values._tag
      : marker?.[1] ?? fallbackCode;
  const code = normalizeCode(rawCode) || normalizeCode(fallbackCode);
  const details = stringify(values.details);
  return { code, message, ...(details ? { details } : {}) };
}

export function formatFailure(
  error: unknown,
  fallbackCode = "VELA_UNKNOWN_FAILURE",
  fallbackMessage = "Operation failed"
): string {
  const failure = normalizeFailure(error, fallbackCode, fallbackMessage);
  return `[${failure.code}] ${failure.message}${failure.details ? ` — ${failure.details}` : ""}`;
}
