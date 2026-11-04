import type { ChatPartReadView, ChatPartRecord } from "../chat-store";

const GITHUB_ISSUE_CONTEXT_PREFIX = "GitHub issue context (JSON)";
const GITHUB_PR_CONTEXT_PREFIX = "GitHub pull request context (JSON)";

export const isSyntheticPart = (part: ChatPartReadView | undefined): boolean => {
  if (!part || typeof part !== "object") {
    return false;
  }
  return Boolean((part as { synthetic?: boolean }).synthetic);
};

export const isFullySyntheticMessage = (parts: readonly ChatPartReadView[] | undefined): boolean => {
  if (!Array.isArray(parts) || parts.length === 0) {
    return false;
  }

  return parts.every((part) => isSyntheticPart(part));
};

export const filterSyntheticParts = (parts: ChatPartRecord[] | undefined): ChatPartRecord[] => {
  if (!Array.isArray(parts) || parts.length === 0) {
    return [];
  }

  const hasNonSynthetic = parts.some((part) => !isSyntheticPart(part));

  const shouldKeepSyntheticPart = (part: ChatPartRecord): boolean => {
    if (!isSyntheticPart(part) || part.type !== "text") {
      return false;
    }

    const text = part.text;
    if (typeof text !== "string") {
      return false;
    }

    const trimmed = text.trimStart();
    return trimmed.startsWith(GITHUB_ISSUE_CONTEXT_PREFIX) || trimmed.startsWith(GITHUB_PR_CONTEXT_PREFIX);
  };

  if (hasNonSynthetic) {
    const hasSynthetic = parts.some((part) => isSyntheticPart(part));
    if (!hasSynthetic) {
      return parts;
    }
    return parts.filter((part) => !isSyntheticPart(part) || shouldKeepSyntheticPart(part));
  }

  return parts;
};
