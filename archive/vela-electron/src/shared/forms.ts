import type { PendingFormField, PendingFormRequest } from "./types";

export function normalizePendingFormField(raw: Record<string, unknown>): PendingFormField | null {
  const key = typeof raw.key === "string" ? raw.key : "";
  const type = raw.type;
  if (!key || typeof type !== "string") return null;
  const base = {
    key,
    ...(typeof raw.title === "string" ? { title: raw.title } : {}),
    ...(typeof raw.description === "string" ? { description: raw.description } : {}),
    ...(raw.required === true ? { required: true } : {})
  };
  if (type === "external") {
    return { ...base, type: "external", url: typeof raw.url === "string" ? raw.url : "" };
  }
  const options = Array.isArray(raw.options)
    ? raw.options.flatMap((option) => {
        if (!option || typeof option !== "object") return [];
        const record = option as Record<string, unknown>;
        return typeof record.value === "string" && typeof record.label === "string"
          ? [{ value: record.value, label: record.label }]
          : [];
      })
    : undefined;
  if (type === "multiselect") return { ...base, type: "multiselect", options: options ?? [] };
  if (type === "boolean") return { ...base, type: "boolean" };
  if (type === "number") return { ...base, type: "number" };
  if (type === "integer") return { ...base, type: "integer" };
  return {
    ...base,
    type: "string",
    ...(typeof raw.placeholder === "string" ? { placeholder: raw.placeholder } : {}),
    ...(options && options.length > 0 ? { options } : {})
  };
}

export function normalizePendingForm(raw: Record<string, unknown>): PendingFormRequest | null {
  const id = typeof raw.id === "string" ? raw.id : "";
  const sessionID = typeof raw.sessionID === "string" ? raw.sessionID : "";
  if (!id || !sessionID) return null;
  const fields = Array.isArray(raw.fields)
    ? raw.fields.flatMap((field) => {
        if (!field || typeof field !== "object") return [];
        const normalized = normalizePendingFormField(field as Record<string, unknown>);
        return normalized ? [normalized] : [];
      })
    : [];
  return {
    id,
    sessionID,
    title: typeof raw.title === "string" && raw.title ? raw.title : "Agent request",
    fields
  };
}
