export type SessionActivityPhase = "idle" | "busy" | "retry";

export interface SessionActivityResult {
  phase: SessionActivityPhase;
  isWorking: boolean;
  isBusy: boolean;
  isCooldown: boolean;
}

export const IDLE_ACTIVITY: SessionActivityResult = {
  phase: "idle",
  isWorking: false,
  isBusy: false,
  isCooldown: false
};

export function resolveSessionActivity(input: {
  sessionId: string | null | undefined;
  statusType: SessionActivityPhase | undefined;
  trailingAssistantIncomplete: boolean;
  pendingPermissions: number;
  pendingQuestions?: number;
}): SessionActivityResult {
  const { sessionId, statusType, trailingAssistantIncomplete, pendingPermissions } = input;
  const pendingQuestions = input.pendingQuestions ?? 0;

  if (!sessionId) return IDLE_ACTIVITY;

  if (pendingPermissions > 0 || pendingQuestions > 0) return IDLE_ACTIVITY;

  const phase: SessionActivityPhase = statusType ?? "idle";

  const hasPendingAssistant = trailingAssistantIncomplete;
  const hasAuthoritativeStatus = statusType !== undefined;
  const statusWorking = hasAuthoritativeStatus && phase !== "idle";
  const isWorking = statusWorking || hasPendingAssistant;

  if (hasAuthoritativeStatus && !statusWorking) return IDLE_ACTIVITY;

  if (!isWorking) return IDLE_ACTIVITY;

  return {
    phase: statusWorking ? phase : "busy",
    isWorking: true,
    isBusy: phase === "busy" || (!statusWorking && hasPendingAssistant),
    isCooldown: false
  };
}
