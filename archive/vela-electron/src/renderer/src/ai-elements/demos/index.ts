import { ChainOfThoughtDemo } from "./chain-of-thought"
import { ChatDemo } from "./chat"
import { CheckpointDemo } from "./checkpoint"
import { ConfirmationDemo } from "./confirmation"
import { ContextDemo } from "./context"
import { FileTreeDemo } from "./file-tree"
import { InlineCitationDemo } from "./inline-citation"
import { MessagePattern } from "./message"
import { ModelSelectorDemo } from "./model-selector"
import { PlanDemo } from "./plan"
import { PromptInputDemo } from "./prompt-input"
import { QueueDemo } from "./queue"
import { ReasoningDemo } from "./reasoning"
import { SourcesDemo } from "./sources"
import { StackTraceDemo } from "./stack-trace"
import { SubagentDemo } from "./subagent"
import { SuggestionDemo } from "./suggestion"
import { TaskDemo } from "./task"
import { TerminalDemo } from "./terminal"
import { TestResultsDemo } from "./test-results"
import { ToolDemo } from "./tool"

export const aiElementDemos = [
  { id: "chain-of-thought", title: "Chain Of Thought", Component: ChainOfThoughtDemo },
  { id: "chat", title: "Chat", Component: ChatDemo },
  { id: "checkpoint", title: "Checkpoint", Component: CheckpointDemo },
  { id: "confirmation", title: "Confirmation", Component: ConfirmationDemo },
  { id: "context", title: "Context", Component: ContextDemo },
  { id: "file-tree", title: "File Tree", Component: FileTreeDemo },
  { id: "inline-citation", title: "Inline Citation", Component: InlineCitationDemo },
  { id: "message", title: "Message", Component: MessagePattern },
  { id: "model-selector", title: "Model Selector", Component: ModelSelectorDemo },
  { id: "plan", title: "Plan", Component: PlanDemo },
  { id: "prompt-input", title: "Prompt Input", Component: PromptInputDemo },
  { id: "queue", title: "Queue", Component: QueueDemo },
  { id: "reasoning", title: "Reasoning", Component: ReasoningDemo },
  { id: "sources", title: "Sources", Component: SourcesDemo },
  { id: "stack-trace", title: "Stack Trace", Component: StackTraceDemo },
  { id: "subagent", title: "Subagent", Component: SubagentDemo },
  { id: "suggestion", title: "Suggestion", Component: SuggestionDemo },
  { id: "task", title: "Task", Component: TaskDemo },
  { id: "terminal", title: "Terminal", Component: TerminalDemo },
  { id: "test-results", title: "Test Results", Component: TestResultsDemo },
  { id: "tool", title: "Tool", Component: ToolDemo },
] as const
