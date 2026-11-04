import { useMemo, useState } from "react";
import type { FormAnswers, PendingFormRequest, WorkspaceIdentity } from "@shared/types";
import { useStore } from "../store";

function isAnswered(field: PendingFormRequest["fields"][number], answer: FormAnswers[string] | undefined): boolean {
  if (field.type === "external") return true;
  if (answer === undefined || answer === "") return false;
  if (Array.isArray(answer)) return answer.length > 0;
  return true;
}

export function FormPrompt({ form, workspace }: { form: PendingFormRequest; workspace: WorkspaceIdentity }): React.ReactNode {
  const store = useStore();
  const [answers, setAnswers] = useState<FormAnswers>({});
  const [submitting, setSubmitting] = useState(false);

  const requiredKeys = useMemo(
    () => form.fields.filter((field) => field.required && field.type !== "external").map((field) => field.key),
    [form.fields]
  );
  const complete = requiredKeys.every((key) => isAnswered(form.fields.find((field) => field.key === key)!, answers[key]));

  const setValue = (key: string, value: FormAnswers[string]) =>
    setAnswers((current) => ({ ...current, [key]: value }));

  const toggleMulti = (key: string, value: string) =>
    setAnswers((current) => {
      const list = Array.isArray(current[key]) ? (current[key] as string[]) : [];
      return { ...current, [key]: list.includes(value) ? list.filter((item) => item !== value) : [...list, value] };
    });

  return (
    <div data-component="dock-prompt" data-kind="form">
      <div data-slot="permission-header">{form.title}</div>
      {form.fields.map((field) => (
        <div key={field.key} className="form-field" data-type={field.type}>
          {(field.title || field.description) && (
            <label>
              {field.title ?? field.key}
              {field.required && field.type !== "external" ? " *" : ""}
              {field.description && <small>{field.description}</small>}
            </label>
          )}
          {field.type === "string" && !field.options && (
            <input
              type="text"
              spellCheck={false}
              autoCorrect="off"
              autoCapitalize="off"
              placeholder={field.placeholder}
              value={(answers[field.key] as string) ?? ""}
              onChange={(event) => setValue(field.key, event.target.value)}
            />
          )}
          {field.type === "string" && field.options && (
            <select value={(answers[field.key] as string) ?? ""} onChange={(event) => setValue(field.key, event.target.value)}>
              <option value="" disabled>Choose…</option>
              {field.options.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </select>
          )}
          {(field.type === "number" || field.type === "integer") && (
            <input
              type="number"
              step={field.type === "integer" ? 1 : "any"}
              value={typeof answers[field.key] === "number" ? (answers[field.key] as number) : ""}
              onChange={(event) => setValue(field.key, event.target.value === "" ? "" : Number(event.target.value))}
            />
          )}
          {field.type === "boolean" && (
            <input
              type="checkbox"
              checked={answers[field.key] === true}
              onChange={(event) => setValue(field.key, event.target.checked)}
            />
          )}
          {field.type === "multiselect" && (
            <div className="form-field-options">
              {(field.options ?? []).map((option) => (
                <label key={option.value} className="form-field-check">
                  <input
                    type="checkbox"
                    checked={Array.isArray(answers[field.key]) && (answers[field.key] as string[]).includes(option.value)}
                    onChange={() => toggleMulti(field.key, option.value)}
                  />
                  {option.label}
                </label>
              ))}
            </div>
          )}
          {field.type === "external" && (
            <a href={field.url} target="_blank" rel="noreferrer">Open external step</a>
          )}
        </div>
      ))}
      <div data-slot="permission-actions">
        <button
          className="btn btn-primary"
          disabled={!complete || submitting}
          onClick={() => {
            setSubmitting(true);
            void store.submitForm(workspace, form.id, answers, form.sessionID).finally(() => setSubmitting(false));
          }}
        >
          Submit
        </button>
        <button className="btn btn-danger" onClick={() => store.dismissForm(workspace, form.id, form.sessionID)}>Dismiss</button>
      </div>
    </div>
  );
}
