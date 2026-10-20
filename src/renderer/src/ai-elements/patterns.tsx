// Pattern compositions from ui.shadcn.com/docs/ai-elements/base; data bindings adapt Vela state.
import * as React from "react";
import { ArrowUpRightIcon, GlobeIcon, TerminalIcon, FileCodeIcon, LayersIcon, XIcon, CircleDashedIcon, CheckIcon, FlaskConicalIcon, ChevronRightIcon, FolderIcon, FileIcon, CircleCheckIcon, CircleDotIcon, CircleIcon, ListTodoIcon, ListChecksIcon, RouteIcon, LoaderCircleIcon, CircleAlertIcon } from "lucide-react";
import { Trace, TraceCollapsible, TraceCollapsibleContent, TraceCollapsibleTrigger, TraceIcon, TraceLine, TraceTitle, TraceDescription } from "@/components/ui/trace";
import { Button } from "@/components/ui/button";
import { CodeBlock, CodeBlockContent } from "@/components/ui/code-block";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";
import { HoverCard, HoverCardTrigger, HoverCardContent } from "@/components/ui/hover-card";
import { Marker, MarkerContent } from "@/components/ui/marker";
import { Progress } from "@/components/ui/progress";
import type { TodoItem, SessionUsage } from "@shared/types";
export type SourcePart = { url: string; title?: string; description?: string };
export type Test = { name: string; status: "passed" | "failed" | "skipped"; error?: string };
export type TestSuite = { name: string; tests: Test[] };
export type FileNode = { type: "file"; path: string; name: string } | { type: "folder"; path: string; name: string; children: FileNode[] };
type Todo = TodoItem;
type PlanPart = { data: { title: string; steps: { title: string; status: "pending" | "running" | "done" }[] } };
export function ReasoningRow({
  text,
  running,
  defaultOpen,
}: {
  text: string
  running: boolean
  defaultOpen?: boolean
}) {
  const [open, setOpen] = React.useState(defaultOpen)

  const line = (
    <TraceLine status={running ? "running" : undefined}>
      <TraceTitle>{running ? "Thinking" : "Thought"}</TraceTitle>
    </TraceLine>
  )

  if (text.length === 0) {
    return line
  }

  return (
    <TraceCollapsible open={open ?? running} onOpenChange={setOpen}>
      <TraceCollapsibleTrigger>{line}</TraceCollapsibleTrigger>
      <TraceCollapsibleContent className="whitespace-pre-wrap">
        {text}
      </TraceCollapsibleContent>
    </TraceCollapsible>
  )
}


export function Sources({ sources }: { sources: SourcePart[] }) {
  return (
    <Trace>
      <TraceCollapsible defaultOpen>
        <TraceCollapsibleTrigger>
          <TraceLine>
            <TraceIcon>
              <GlobeIcon />
            </TraceIcon>
            <TraceTitle>Used {sources.length} sources</TraceTitle>
          </TraceLine>
        </TraceCollapsibleTrigger>
        <TraceCollapsibleContent>
          <Trace>
            {sources.map((source) => (
              <a
                key={source.url}
                href={source.url}
                target="_blank"
                rel="noreferrer"
              >
                <TraceLine>
                  <TraceTitle>
                    {source.title ?? new URL(source.url).hostname}
                    <span className="sr-only"> (opens in a new tab)</span>
                  </TraceTitle>
                  <TraceIcon>
                    <ArrowUpRightIcon />
                  </TraceIcon>
                </TraceLine>
              </a>
            ))}
          </Trace>
        </TraceCollapsibleContent>
      </TraceCollapsible>
    </Trace>
  )
}


function parseFrame(line: string) {
  const match = line.match(/^at (.+) \((.+):(\d+):(\d+)\)$/)

  return {
    line,
    name: match?.[1],
    file: match?.[2],
    lineNumber: match?.[3],
    internal: line.includes("node_modules") || line.includes("node:"),
  }
}

export function StackTrace({ stack, onOpenFrame }: { stack: string; onOpenFrame?: (path: string, line: number) => void }) {
  const [error, ...lines] = stack.split("\n").map((line) => line.trim())
  const frames = lines.map(parseFrame)
  const appFrames = frames.filter((frame) => !frame.internal)
  const internalFrames = frames.filter((frame) => frame.internal)

  return (
    <TraceCollapsible defaultOpen>
      <TraceCollapsibleTrigger>
        <TraceLine status="error">
          <TraceIcon>
            <TerminalIcon />
          </TraceIcon>
          <TraceTitle>Failed: {error}</TraceTitle>
        </TraceLine>
      </TraceCollapsibleTrigger>
      <TraceCollapsibleContent className="flex flex-col gap-2">
        <p className="font-mono text-xs text-destructive">{error}</p>
        <Trace>
          {appFrames.map((frame) => (
            <a key={frame.line} href={`#${frame.file}:${frame.lineNumber}`} onClick={event => { if (onOpenFrame && frame.file && frame.lineNumber) {event.preventDefault(); onOpenFrame(frame.file, Number(frame.lineNumber));} }}>
              <TraceLine>
                <TraceIcon>
                  <FileCodeIcon />
                </TraceIcon>
                <TraceTitle>
                  {frame.name} in {frame.file}:{frame.lineNumber}
                </TraceTitle>
              </TraceLine>
            </a>
          ))}
          <TraceCollapsible>
            <TraceCollapsibleTrigger>
              <TraceLine>
                <TraceIcon>
                  <LayersIcon />
                </TraceIcon>
                <TraceTitle>
                  {internalFrames.length} framework frames
                </TraceTitle>
              </TraceLine>
            </TraceCollapsibleTrigger>
            <TraceCollapsibleContent>
              <ol className="flex flex-col gap-1 overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">
                {internalFrames.map((frame) => (
                  <li key={frame.line} className="whitespace-nowrap">
                    {frame.line}
                  </li>
                ))}
              </ol>
            </TraceCollapsibleContent>
          </TraceCollapsible>
        </Trace>
      </TraceCollapsibleContent>
    </TraceCollapsible>
  )
}


function countTests(tests: Test[]) {
  return {
    total: tests.length,
    failed: tests.filter((test) => test.status === "failed").length,
    skipped: tests.filter((test) => test.status === "skipped").length,
  }
}

function formatCounts({
  total,
  failed,
  skipped,
}: ReturnType<typeof countTests>) {
  return [
    `${total} tests`,
    failed > 0 && `${failed} failed`,
    skipped > 0 && `${skipped} skipped`,
  ]
    .filter(Boolean)
    .join(", ")
}

export function TestRow({ test }: { test: Test }) {
  if (test.status === "failed") {
    return (
      <TraceCollapsible defaultOpen>
        <TraceCollapsibleTrigger>
          <TraceLine status="error">
            <TraceIcon>
              <XIcon />
            </TraceIcon>
            <TraceTitle>{test.name}</TraceTitle>
          </TraceLine>
        </TraceCollapsibleTrigger>
        <TraceCollapsibleContent>
          <CodeBlock size="sm">
            <CodeBlockContent wrap className="max-h-64">
              {test.error}
            </CodeBlockContent>
          </CodeBlock>
        </TraceCollapsibleContent>
      </TraceCollapsible>
    )
  }

  return (
    <TraceLine>
      <TraceIcon>
        {test.status === "skipped" ? <CircleDashedIcon /> : <CheckIcon />}
      </TraceIcon>
      <TraceTitle>
        {test.status === "skipped" ? `Skipped ${test.name}` : test.name}
      </TraceTitle>
    </TraceLine>
  )
}


export function FileTreeNode({
  node,
  selected,
  onSelect,
}: {
  node: FileNode
  selected: string
  onSelect: (path: string) => void
}) {
  switch (node.type) {
    case "folder":
      return (
        <Collapsible defaultOpen>
          <CollapsibleTrigger
            render={
              <Button
                variant="ghost"
                size="sm"
                className="w-full justify-start font-normal aria-expanded:bg-transparent aria-expanded:hover:bg-muted dark:aria-expanded:hover:bg-muted/50"
              />
            }
          >
            <ChevronRightIcon className="transition-transform group-data-panel-open/button:rotate-90" />
            <FolderIcon />
            {node.name}
          </CollapsibleTrigger>
          <CollapsibleContent>
            <div className="ml-4 flex flex-col">
              {node.children.map((child) => (
                <FileTreeNode
                  key={child.path}
                  node={child}
                  selected={selected}
                  onSelect={onSelect}
                />
              ))}
            </div>
          </CollapsibleContent>
        </Collapsible>
      )
    case "file":
      return (
        <Button
          variant="ghost"
          size="sm"
          className={`w-full justify-start font-normal ${
            selected === node.path ? "bg-accent text-accent-foreground" : ""
          }`}
          aria-current={selected === node.path ? "true" : undefined}
          onClick={() => onSelect(node.path)}
        >
          <FileIcon />
          {node.name}
        </Button>
      )
    default: {
      const unreachable: never = node
      return unreachable
    }
  }
}


function TodoLine({ todo, streaming }: { todo: Todo; streaming: boolean }) {
  return (
    <TraceLine
      status={
        streaming && todo.status === "in_progress" ? "running" : undefined
      }
    >
      <TraceIcon>
        {todo.status === "completed" ? (
          <CircleCheckIcon />
        ) : todo.status === "in_progress" ? (
          <CircleDotIcon />
        ) : (
          <CircleIcon />
        )}
      </TraceIcon>
      <TraceTitle>{todo.content}</TraceTitle>
    </TraceLine>
  )
}

export function TodoQueue({
  todos,
  streaming,
}: {
  todos: Todo[]
  streaming: boolean
}) {
  const pending = todos.filter((todo) => todo.status !== "completed")
  const completed = todos.filter((todo) => todo.status === "completed")

  return (
    <Trace>
      {pending.length > 0 && (
        <TraceCollapsible defaultOpen>
          <TraceCollapsibleTrigger>
            <TraceLine>
              <TraceIcon>
                <ListTodoIcon />
              </TraceIcon>
              <TraceTitle>{pending.length} to do</TraceTitle>
            </TraceLine>
          </TraceCollapsibleTrigger>
          <TraceCollapsibleContent>
            <Trace>
              {pending.map((todo) => (
                <TodoLine
                  key={todo.content}
                  todo={todo}
                  streaming={streaming}
                />
              ))}
            </Trace>
          </TraceCollapsibleContent>
        </TraceCollapsible>
      )}
      {completed.length > 0 && (
        <TraceCollapsible defaultOpen>
          <TraceCollapsibleTrigger>
            <TraceLine>
              <TraceIcon>
                <ListChecksIcon />
              </TraceIcon>
              <TraceTitle>{completed.length} completed</TraceTitle>
            </TraceLine>
          </TraceCollapsibleTrigger>
          <TraceCollapsibleContent>
            <Trace>
              {completed.map((todo) => (
                <TodoLine
                  key={todo.content}
                  todo={todo}
                  streaming={streaming}
                />
              ))}
            </Trace>
          </TraceCollapsibleContent>
        </TraceCollapsible>
      )}
    </Trace>
  )
}


export function PlanRow({
  plan,
  streaming,
}: {
  plan: PlanPart["data"]
  streaming: boolean
}) {
  const done = plan.steps.filter((step) => step.status === "done").length
  const running = streaming && done < plan.steps.length

  return (
    <TraceCollapsible defaultOpen>
      <TraceCollapsibleTrigger>
        <TraceLine status={running ? "running" : undefined}>
          <TraceIcon>
            <RouteIcon />
          </TraceIcon>
          <TraceTitle>{plan.title}</TraceTitle>
          <TraceDescription>
            {done} of {plan.steps.length} done
          </TraceDescription>
        </TraceLine>
      </TraceCollapsibleTrigger>
      <TraceCollapsibleContent>
        <Trace>
          {plan.steps.map((step) => (
            <TraceLine
              key={step.title}
              status={
                step.status === "pending"
                  ? "pending"
                  : streaming && step.status === "running"
                    ? "running"
                    : undefined
              }
            >
              <TraceIcon>
                {step.status === "done" ? (
                  <CircleCheckIcon />
                ) : step.status === "running" ? (
                  <CircleDotIcon />
                ) : (
                  <CircleIcon />
                )}
              </TraceIcon>
              <TraceTitle>{step.title}</TraceTitle>
            </TraceLine>
          ))}
        </Trace>
      </TraceCollapsibleContent>
    </TraceCollapsible>
  )
}

function hostname(url: string) {
  return new URL(url).hostname.replace(/^www\./, "")
}

export function Citation({ source }: { source: {url: string; title: string; description: string} }) {
  const host = hostname(source.url)

  return (
    <HoverCard>
      <HoverCardTrigger
        delay={10}
        closeDelay={100}
        render={<Button variant="secondary" size="xs" />}
      >
        {host}
      </HoverCardTrigger>
      <HoverCardContent className="flex w-64 flex-col gap-1">
        <div className="font-medium">{source.title}</div>
        <p className="text-muted-foreground">{source.description}</p>
        <a
          href={source.url}
          target="_blank"
          rel="noreferrer"
          className="text-xs text-muted-foreground underline-offset-4 hover:underline"
        >
          {host}
          <span className="sr-only"> (opens in a new tab)</span>
        </a>
      </HoverCardContent>
    </HoverCard>
  )
}

function formatTokens(value: number) {
  if (value < 1000) {
    return String(value)
  }

  return `${(value / 1000).toFixed(1).replace(/\.0$/, "")}k`
}

export function AiElementsContext({ usage: sessionUsage, limit }: {usage: SessionUsage; limit: number}) {
  const usage = { ...sessionUsage.tokens, limit };
  const used = usage.input + usage.output + usage.reasoning;
  const percent = Math.round((used / usage.limit) * 100);
  const rows = [{label: "Input", value: usage.input}, {label: "Output", value: usage.output}, {label: "Reasoning", value: usage.reasoning}];
  return (
    <div className="flex justify-center py-12">
      <HoverCard>
        <HoverCardTrigger
          delay={10}
          closeDelay={100}
          render={<Button variant="outline" size="sm" />}
        >
          {percent}% context
        </HoverCardTrigger>
        <HoverCardContent className="flex w-64 flex-col gap-3">
          <div className="flex items-center justify-between gap-3 text-sm">
            <span className="font-medium">Context</span>
            <span className="text-muted-foreground">
              {formatTokens(used)} / {formatTokens(usage.limit)}
            </span>
          </div>
          <Progress value={percent} aria-label="Context used" />
          <div className="flex flex-col gap-1 text-sm">
            {rows.map((row) => (
              <div key={row.label} className="flex justify-between gap-3">
                <span className="text-muted-foreground">{row.label}</span>
                <span>{formatTokens(row.value)}</span>
              </div>
            ))}
          </div>
        </HoverCardContent>
      </HoverCard>
    </div>
  )
}

export function AiElementsCheckpoint({ label, onRestore, disabled }: {label: string; onRestore?: () => void; disabled?: boolean}) {
  return <Marker variant="separator"><MarkerContent className="inline-flex items-center gap-2">{label}{onRestore && <Button size="xs" variant="outline" disabled={disabled} aria-label={`Restore to ${label}`} onClick={onRestore}>Restore</Button>}</MarkerContent></Marker>;
}
export function TestResults({ suites }: {suites: TestSuite[]}) {
 const summary = countTests(suites.flatMap(suite => suite.tests));
 return <TraceCollapsible defaultOpen={summary.failed > 0}><TraceCollapsibleTrigger><TraceLine status={summary.failed > 0 ? "error" : undefined}><TraceIcon><FlaskConicalIcon /></TraceIcon><TraceTitle>Ran {formatCounts(summary)}</TraceTitle></TraceLine></TraceCollapsibleTrigger><TraceCollapsibleContent keepMounted><Trace>{suites.map(suite => { const counts=countTests(suite.tests); return <TraceCollapsible key={suite.name} defaultOpen={counts.failed > 0}><TraceCollapsibleTrigger><TraceLine status={counts.failed > 0 ? "error" : undefined}><TraceIcon><FileCodeIcon /></TraceIcon><TraceTitle>{suite.name}, {formatCounts(counts)}</TraceTitle></TraceLine></TraceCollapsibleTrigger><TraceCollapsibleContent keepMounted><Trace>{suite.tests.map(test => <TestRow key={test.name} test={test} />)}</Trace></TraceCollapsibleContent></TraceCollapsible>;})}</Trace></TraceCollapsibleContent></TraceCollapsible>;
}

type Status = "running" | "error" | undefined;
export type TaskData = {title: string; steps: {title: string; status: "done" | "running" | "error"}[]};
type TaskPart = {data: TaskData};
function getStepStatus(
  status: TaskPart["data"]["steps"][number]["status"],
  streaming: boolean
): Status {
  if (status === "done") {
    return undefined
  }

  return status === "running" && streaming ? "running" : "error"
}

function StatusIcon({ status }: { status: Status }) {
  if (status === "running") {
    return <LoaderCircleIcon className="animate-spin" />
  }

  return status === "error" ? <CircleAlertIcon /> : <CircleCheckIcon />
}

export function TaskRow({
  task,
  streaming,
}: {
  task: TaskPart["data"]
  streaming: boolean
}) {
  const [open, setOpen] = React.useState<boolean>()
  const steps = task.steps.map((step) => ({
    title: step.title,
    status: getStepStatus(step.status, streaming),
  }))
  const status = steps.some((step) => step.status === "error")
    ? "error"
    : steps.some((step) => step.status === "running")
      ? "running"
      : undefined

  return (
    <TraceCollapsible
      open={open ?? status === "running"}
      onOpenChange={setOpen}
    >
      <TraceCollapsibleTrigger>
        <TraceLine status={status}>
          <TraceIcon>
            <StatusIcon status={status} />
          </TraceIcon>
          <TraceTitle>{task.title}</TraceTitle>
        </TraceLine>
      </TraceCollapsibleTrigger>
      <TraceCollapsibleContent keepMounted>
        <Trace>
          {steps.map((step) => (
            <TraceLine key={step.title} status={step.status}>
              <TraceTitle>{step.title}</TraceTitle>
            </TraceLine>
          ))}
        </Trace>
      </TraceCollapsibleContent>
    </TraceCollapsible>
  )
}
