import type { MessagePresentation } from "./types";

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

export function messagePresentation(input: unknown): MessagePresentation {
  const value = record(input);
  const model = record(value.model);
  const id = model.id ?? model.modelID ?? value.modelID;
  const providerID = model.providerID ?? value.providerID;
  const time = record(value.time);
  const createdAt = time.created ?? value.createdAt;
  const completedAt = time.completed ?? value.completedAt;
  const error = record(value.error);
  const errorType = typeof error.type === "string" ? error.type.toLowerCase() : "";
  const interrupted = value.interrupted === true || error.name === "MessageAbortedError" ||
    errorType.includes("abort") || errorType.includes("interrupt");
  return {
    ...(typeof value.agent === "string" ? { agent: value.agent } : {}),
    ...(typeof id === "string" && typeof providerID === "string" ? { model: { id, providerID } } : {}),
    ...(typeof createdAt === "number" ? { createdAt } : {}),
    ...(typeof completedAt === "number" ? { completedAt } : {}),
    ...(typeof value.parentID === "string" ? { parentID: value.parentID } : {}),
    ...(interrupted ? { interrupted: true } : {})
  };
}
