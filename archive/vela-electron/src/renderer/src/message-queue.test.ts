import { describe, expect, it } from "vitest";
import {
  addToQueue,
  clearAllQueues,
  clearQueue,
  createMessageQueueTarget,
  emptyMessageQueueState,
  getMessageQueueKey,
  getQueueForTarget,
  loadMessageQueueState,
  MESSAGE_QUEUE_STATE_VERSION,
  migrateMessageQueueState,
  normalizeFollowUpBehavior,
  popToInput,
  removeFromQueue,
  reorderQueue,
  type MessageQueueState
} from "./message-queue";

const target = { workspaceID: "w1", sessionID: "s1" };

function withOne(state: MessageQueueState): MessageQueueState {
  return addToQueue(state, target, { content: "hello" });
}

describe("message queue", () => {
  it("normalizes follow-up behavior", () => {
    expect(normalizeFollowUpBehavior("queue")).toBe("queue");
    expect(normalizeFollowUpBehavior("steer")).toBe("steer");
    expect(normalizeFollowUpBehavior("immediate")).toBe("steer");
    expect(normalizeFollowUpBehavior(undefined)).toBe("queue");
    expect(normalizeFollowUpBehavior("bogus", false)).toBe("steer");
  });

  it("adds, lists, removes, and pops messages", () => {
    let state = withOne(emptyMessageQueueState());
    state = addToQueue(state, target, { content: "second" });

    expect(getQueueForTarget(state, target).map((message) => message.content)).toEqual(["hello", "second"]);

    const popped = popToInput(state, target, getQueueForTarget(state, target)[0].id);
    expect(popped.message?.content).toBe("hello");
    expect(getQueueForTarget(popped.state, target).map((message) => message.content)).toEqual(["second"]);

    const remaining = getQueueForTarget(popped.state, target);
    state = removeFromQueue(popped.state, target, remaining[0].id);
    expect(getQueueForTarget(state, target)).toEqual([]);
  });

  it("reorders the queue", () => {
    let state = withOne(emptyMessageQueueState());
    state = addToQueue(state, target, { content: "second" });
    const queue = getQueueForTarget(state, target);

    const reordered = reorderQueue(state, target, queue[1].id, queue[0].id);
    expect(getQueueForTarget(reordered, target).map((message) => message.content)).toEqual(["second", "hello"]);
  });

  it("caps queues at 20 entries and 50 targets", () => {
    let state = emptyMessageQueueState();
    for (let index = 0; index < 25; index += 1) {
      state = addToQueue(state, target, { content: `m${index}` });
    }
    expect(getQueueForTarget(state, target)).toHaveLength(20);
    expect(getQueueForTarget(state, target)[19].content).toBe("m24");

    for (let index = 0; index < 55; index += 1) {
      state = addToQueue(state, { workspaceID: `w${index}`, sessionID: "s" }, { content: "x" });
    }
    expect(Object.keys(state.queuedMessages).length).toBeLessThanOrEqual(50);
  });

  it("clears queues and all queues", () => {
    let state = withOne(emptyMessageQueueState());
    state = addToQueue(state, target, { content: "second" });
    state = clearQueue(state, { workspaceID: "w2", sessionID: "s2" });
    expect(getQueueForTarget(state, target)).toHaveLength(2);

    expect(getQueueForTarget(clearQueue(state, target), target)).toEqual([]);
    expect(clearAllQueues(state).queuedMessages).toEqual({});
  });

  it("creates targets only with both ids and round-trips keys", () => {
    expect(createMessageQueueTarget("s1", null)).toBeNull();
    expect(getMessageQueueKey(target)).toBe("w1\ns1");
  });

  it("migrates pre-native queues into quarantine", () => {
    const migrated = migrateMessageQueueState({
      queuedMessages: { "w1\ns1": [{ id: "m1", content: "legacy", createdAt: 1 }] },
      followUpBehavior: "steer"
    }, MESSAGE_QUEUE_STATE_VERSION - 1);

    expect(migrated.queuedMessages).toEqual({});
    expect(migrated.quarantinedLegacyMessages?.["w1\ns1"]).toHaveLength(1);
    expect(migrated.followUpBehavior).toBe("steer");
  });

  it("keeps current queues across a current-version load", () => {
    window.localStorage.setItem("messageQueue", JSON.stringify({
      queuedMessages: { "w1\ns1": [{ id: "m1", content: "hello", createdAt: 1 }] },
      followUpBehavior: "queue"
    }));

    const state = loadMessageQueueState();

    expect(state.queuedMessages["w1\ns1"]).toHaveLength(1);
    expect(state.followUpBehavior).toBe("queue");
    expect(state.quarantinedLegacyMessages).toEqual({});
    window.localStorage.clear();
  });
});
