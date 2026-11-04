import type { PromptFile } from "@shared/types";

export type FollowUpBehavior = "steer" | "queue";

export const DEFAULT_FOLLOW_UP_BEHAVIOR: FollowUpBehavior = "queue";

export const isFollowUpBehavior = (value: unknown): value is FollowUpBehavior =>
  value === "steer" || value === "queue";

export const normalizeFollowUpBehavior = (
  value: unknown,
  legacyQueueModeEnabled?: boolean | null
): FollowUpBehavior => {
  if (value === "immediate") {
    return "steer";
  }

  if (isFollowUpBehavior(value)) {
    return value;
  }

  if (legacyQueueModeEnabled === false) {
    return "steer";
  }

  if (legacyQueueModeEnabled === true) {
    return "queue";
  }

  return DEFAULT_FOLLOW_UP_BEHAVIOR;
};

export interface QueuedMessage {
  id: string;
  content: string;
  attachments?: PromptFile[];
  attachmentCount?: number;
  createdAt: number;
  sendConfig?: {
    providerID: string;
    modelID: string;
    agent?: string;
    variant?: string;
  };
}

export type MessageQueueTarget = {
  workspaceID: string;
  sessionID: string;
};

const MAX_QUEUE_TARGETS = 50;
const MAX_MESSAGES_PER_QUEUE = 20;

export const createMessageQueueTarget = (
  sessionID: string,
  workspaceID: string | null | undefined
): MessageQueueTarget | null => {
  if (!workspaceID || !sessionID) return null;
  return { workspaceID, sessionID };
};

export const getMessageQueueKey = (target: MessageQueueTarget): string =>
  `${target.workspaceID}\n${target.sessionID}`;

export const parseMessageQueueKey = (key: string): MessageQueueTarget | null => {
  const [workspaceID, sessionID] = key.split("\n");
  return createMessageQueueTarget(sessionID, workspaceID);
};

export interface MessageQueueState {
  queuedMessages: Record<string, QueuedMessage[]>;
  quarantinedLegacyMessages: Record<string, QueuedMessage[]>;
  followUpBehavior: FollowUpBehavior;
}

export type QueuedPromptFileReadView = Readonly<Omit<PromptFile, "mention">> & {
  readonly mention?: Readonly<NonNullable<PromptFile["mention"]>>;
};

export type QueuedMessageReadView = Readonly<Omit<QueuedMessage, "attachments" | "sendConfig">> & {
  readonly attachments?: readonly QueuedPromptFileReadView[];
  readonly sendConfig?: Readonly<NonNullable<QueuedMessage["sendConfig"]>>;
};

export interface MessageQueueReadView {
  readonly queuedMessages: Readonly<Record<string, readonly QueuedMessageReadView[]>>;
  readonly quarantinedLegacyMessages: Readonly<Record<string, readonly QueuedMessageReadView[]>>;
  readonly followUpBehavior: FollowUpBehavior;
}

export function emptyMessageQueueState(): MessageQueueState {
  return {
    queuedMessages: {},
    quarantinedLegacyMessages: {},
    followUpBehavior: DEFAULT_FOLLOW_UP_BEHAVIOR
  };
}

export function addToQueue(
  state: MessageQueueState,
  target: MessageQueueTarget,
  message: Omit<QueuedMessage, "id" | "createdAt">
): MessageQueueState {
  const key = getMessageQueueKey(target);
  const id = `queued-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
  const queuedMessage: QueuedMessage = {
    id,
    content: message.content,
    attachments: message.attachments,
    createdAt: Date.now(),
    sendConfig: message.sendConfig
  };
  const currentQueue = state.queuedMessages[key] ?? [];
  const queuedMessages = {
    ...state.queuedMessages,
    [key]: [...currentQueue, queuedMessage].slice(-MAX_MESSAGES_PER_QUEUE)
  };
  const keys = Object.keys(queuedMessages);
  if (keys.length > MAX_QUEUE_TARGETS) {
    keys.sort((left, right) => (
      (queuedMessages[left]?.[0]?.createdAt ?? 0) - (queuedMessages[right]?.[0]?.createdAt ?? 0)
    ));
    for (const staleKey of keys.slice(0, keys.length - MAX_QUEUE_TARGETS)) delete queuedMessages[staleKey];
  }
  return { ...state, queuedMessages };
}

export function removeFromQueue(state: MessageQueueState, target: MessageQueueTarget, messageId: string): MessageQueueState {
  const key = getMessageQueueKey(target);
  const currentQueue = state.queuedMessages[key] ?? [];
  const newQueue = currentQueue.filter((message) => message.id !== messageId);

  if (newQueue.length === 0) {
    const { [key]: _removed, ...rest } = state.queuedMessages;
    void _removed;
    return { ...state, queuedMessages: rest };
  }

  return {
    ...state,
    queuedMessages: {
      ...state.queuedMessages,
      [key]: newQueue
    }
  };
}

export function reorderQueue(state: MessageQueueState, target: MessageQueueTarget, fromId: string, toId: string): MessageQueueState {
  if (fromId === toId) return state;
  const key = getMessageQueueKey(target);
  const currentQueue = state.queuedMessages[key];
  if (!currentQueue) return state;
  const fromIndex = currentQueue.findIndex((message) => message.id === fromId);
  const toIndex = currentQueue.findIndex((message) => message.id === toId);
  if (fromIndex === -1 || toIndex === -1) return state;

  const newQueue = currentQueue.slice();
  const [moved] = newQueue.splice(fromIndex, 1);
  newQueue.splice(toIndex, 0, moved);

  return {
    ...state,
    queuedMessages: {
      ...state.queuedMessages,
      [key]: newQueue
    }
  };
}

export function popToInput(state: MessageQueueState, target: MessageQueueTarget, messageId: string): { state: MessageQueueState; message: QueuedMessage | null } {
  const key = getMessageQueueKey(target);
  const currentQueue = state.queuedMessages[key] ?? [];
  const message = currentQueue.find((item) => item.id === messageId);

  if (!message) {
    return { state, message: null };
  }

  return { state: removeFromQueue(state, target, messageId), message };
}

export function clearQueue(state: MessageQueueState, target: MessageQueueTarget): MessageQueueState {
  const key = getMessageQueueKey(target);
  if (!(key in state.queuedMessages)) return state;
  const { [key]: _removed, ...rest } = state.queuedMessages;
  void _removed;
  return { ...state, queuedMessages: rest };
}

export function clearAllQueues(state: MessageQueueState): MessageQueueState {
  return { ...state, queuedMessages: {} };
}

export function getQueueForTarget(state: MessageQueueState, target: MessageQueueTarget): QueuedMessage[];
export function getQueueForTarget(state: MessageQueueReadView, target: MessageQueueTarget): readonly QueuedMessageReadView[];
export function getQueueForTarget(state: MessageQueueState | MessageQueueReadView, target: MessageQueueTarget): QueuedMessage[] | readonly QueuedMessageReadView[] {
  return state.queuedMessages[getMessageQueueKey(target)] ?? [];
}

type PersistedMessageQueueState = {
  queuedMessages?: Record<string, QueuedMessage[]>;
  quarantinedLegacyMessages?: Record<string, QueuedMessage[]>;
  followUpBehavior?: FollowUpBehavior;
  queueModeEnabled?: boolean;
};

export const MESSAGE_QUEUE_STATE_VERSION = 3;

export const migrateMessageQueueState = (persistedState: unknown, version: number): Partial<MessageQueueState> => {
  const state = (persistedState ?? {}) as PersistedMessageQueueState;
  const legacyQueues = version < MESSAGE_QUEUE_STATE_VERSION ? (state.queuedMessages ?? {}) : {};
  return {
    queuedMessages: version < MESSAGE_QUEUE_STATE_VERSION ? {} : (state.queuedMessages ?? {}),
    quarantinedLegacyMessages: {
      ...(state.quarantinedLegacyMessages ?? {}),
      ...legacyQueues
    },
    followUpBehavior: normalizeFollowUpBehavior(state.followUpBehavior, state.queueModeEnabled ?? null)
  };
};

export function persistMessageQueueState(state: MessageQueueState): void {
  try {
    window.localStorage.setItem("messageQueue", JSON.stringify({
      queuedMessages: state.queuedMessages,
      quarantinedLegacyMessages: state.quarantinedLegacyMessages,
      followUpBehavior: state.followUpBehavior
    }));
  } catch {
  }
}

export function loadMessageQueueState(): MessageQueueState {
  try {
    const raw = window.localStorage.getItem("messageQueue");
    if (!raw) return emptyMessageQueueState();
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const migrated = migrateMessageQueueState(parsed, MESSAGE_QUEUE_STATE_VERSION);
    const queuedMessages: Record<string, QueuedMessage[]> = {};
    if (migrated.queuedMessages && typeof migrated.queuedMessages === "object") {
      for (const [key, value] of Object.entries(migrated.queuedMessages)) {
        if (Array.isArray(value)) {
          queuedMessages[key] = value.filter((message): message is QueuedMessage =>
            Boolean(message && typeof message === "object" && typeof message.id === "string" && typeof message.content === "string")
          );
        }
      }
    }
    const quarantinedLegacyMessages: Record<string, QueuedMessage[]> = {};
    if (migrated.quarantinedLegacyMessages && typeof migrated.quarantinedLegacyMessages === "object") {
      for (const [key, value] of Object.entries(migrated.quarantinedLegacyMessages)) {
        if (Array.isArray(value)) {
          quarantinedLegacyMessages[key] = value.filter((message): message is QueuedMessage =>
            Boolean(message && typeof message === "object" && typeof message.id === "string" && typeof message.content === "string")
          );
        }
      }
    }
    return {
      queuedMessages,
      quarantinedLegacyMessages,
      followUpBehavior: migrated.followUpBehavior ?? DEFAULT_FOLLOW_UP_BEHAVIOR
    };
  } catch {
    return emptyMessageQueueState();
  }
}
