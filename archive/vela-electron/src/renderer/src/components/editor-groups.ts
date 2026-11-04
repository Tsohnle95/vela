export type EditorGroupID = "primary" | "secondary" | "tertiary" | "quaternary";

export const EDITOR_GROUP_IDS: EditorGroupID[] = ["primary", "secondary", "tertiary", "quaternary"];

export function isEditorGroupID(value: unknown): value is EditorGroupID {
  return EDITOR_GROUP_IDS.includes(value as EditorGroupID);
}
