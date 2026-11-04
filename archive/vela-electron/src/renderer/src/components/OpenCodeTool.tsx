import { useId, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { useStore } from "../store";
import type { ToolCallView, TranscriptItem, SessionInfo } from "@shared/types";
import { ExternalLink } from "./ExternalLink";
import { TextShimmer } from "./TextShimmer";
import { ToolStatusTitle } from "./ToolStatusTitle";
import { AnimatedCountList, type CountItem } from "./ToolCountSummary";
import { OpenCodeIcon } from "./OpenCodeIcon";
import { animate } from "motion";
import { OpenCodeFile } from "./OpenCodeFile";
import { OpenCodeFileIcon } from "./OpenCodeFileIcon";
import { OpenCodeSpecialTool, isDismissedOpenCodeQuestion, openCodeSpecialToolTitle } from "./OpenCodeSpecialTool";
import { resolveOpenCodeTaskAgent } from "../opencode-task-agent";
import stripAnsi from "strip-ansi";
import { writeClipboard } from "./TimelineProse";
import { fileName, contextDirectory, workspaceFilePath, titleCase, toolKey, toolInput, effectiveToolKey,
  contextToolArgs, contextToolSummary, taskDescriptor, taskChildID, parseSubagentTag, parseLegacyTaskText,
  subagentChildID, toolPresentation, formatDuration, progressText, editFileEntries, isEditCardTool,
  type EditFileEntry } from "./timeline-model";

const OUTPUT_LIMIT = 6000;

function useToolFileActivation(session: SessionInfo | null): (path: string) => void {
  const { openFile, focusSession } = useStore();
  return (path): void => {
    const target = workspaceFilePath(path, session);
    if (!target) return;
    if (session) focusSession?.(session.id);
    void openFile(target, session?.workspace);
  };
}

function ShellSubmessage({ text, animate: reveal }: { text: string; animate: boolean }): ReactNode {
  const width = useRef<HTMLSpanElement>(null);
  const value = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    if (!reveal) return;
    const animations: Array<ReturnType<typeof animate>> = [];
    const frame = requestAnimationFrame(() => {
      if (width.current) animations.push(animate(width.current, { width: "auto" }, { type: "spring", visualDuration: 0.25, bounce: 0 }));
      if (value.current) animations.push(animate(value.current, { opacity: 1, filter: "blur(0px)" }, { duration: 0.32, ease: [0.16, 1, 0.3, 1] }));
    });
    return () => { cancelAnimationFrame(frame); animations.forEach((animation) => animation.stop()); };
  }, []);
  return <span data-component="shell-submessage" dir="ltr"><span ref={width} data-slot="shell-submessage-width" style={reveal ? { width: "0px" } : undefined}>
    <span data-slot="basic-tool-tool-subtitle"><span ref={value} data-slot="shell-submessage-value" style={reveal ? { opacity: 0, filter: "blur(2px)" } : undefined}>{text}</span></span>
  </span></span>;
}

function DiffChanges({ file }: { file: Pick<EditFileEntry, "additions" | "deletions"> }): ReactNode {
  return <div data-component="diff-changes" data-variant="numbers">
    <span data-slot="diff-changes-additions">+{file.additions ?? 0}</span>
    <span data-slot="diff-changes-deletions">-{file.deletions ?? 0}</span>
  </div>;
}

function ToolFileAccordion({ file, write = false, patch = false, session, activatePath }: {
  file: EditFileEntry; write?: boolean; patch?: boolean; session: SessionInfo | null; activatePath: () => void;
}): ReactNode {
  const [open, setOpen] = useState(!["delete", "deleted", "removed"].includes(file.status ?? ""));
  const triggerId = useId();
  const slash = file.file.lastIndexOf("/");
  const action = ["add", "added", "created"].includes(file.status ?? "") ? "Created"
    : ["delete", "deleted", "removed"].includes(file.status ?? "") ? "Deleted" : file.status === "move" ? "Moved" : null;
  return <div data-slot="accordion-item" data-type={file.status} data-expanded={open ? "" : undefined}>
    <h3 data-slot="accordion-header" data-component="sticky-accordion-header">
      <button type="button" id={triggerId} data-slot="accordion-trigger" aria-expanded={open} onClick={() => setOpen((value) => !value)}>
        <div data-slot="apply-patch-trigger-content">
          <div data-slot="apply-patch-file-info">
            <OpenCodeFileIcon path={file.file} />
            <div data-slot="apply-patch-file-name-container">
              {slash >= 0 && <span data-slot="apply-patch-directory">{"\u202a" + contextDirectory(file.file, session) + "\u202c"}</span>}
              <span data-slot="apply-patch-filename" onClick={(event) => { event.stopPropagation(); activatePath(); }}>{fileName(file.file)}</span>
            </div>
          </div>
          <div data-slot="apply-patch-trigger-actions">
            {!write && (patch && action ? <span data-slot="apply-patch-change" data-type={action === "Deleted" ? "removed" : action === "Created" ? "added" : "modified"}>{action}</span> : <DiffChanges file={file} />)}
            <OpenCodeIcon name="chevron-grabber-vertical" />
          </div>
        </div>
      </button>
    </h3>
    {open && <div role="region" aria-labelledby={triggerId} data-slot="accordion-content" data-expanded="">
      <div data-component={write ? "write-content" : patch ? "apply-patch-file-diff" : "edit-content"}>
        {write ? <OpenCodeFile mode="text" file={file.file} contents={file.content ?? file.after ?? ""} />
          : <OpenCodeFile mode="diff" file={file.file} patch={file.patch} before={file.before} after={file.after} />}
      </div>
    </div>}
  </div>;
}

function EditToolCard({ tool, session }: { tool: ToolCallView; session: SessionInfo | null }): ReactNode {
  const activatePath = useToolFileActivation(session);
  const [open, setOpen] = useState(false);
  const files = editFileEntries(tool);
  const name = toolKey(tool.title);
  const write = name === "write";
  const patch = name === "patch" || name === "apply_patch";
  const pending = tool.status === "running";
  const path = files[0]?.file ?? "";
  const inputPath = toolInput(tool).filePath;
  const triggerPath = !patch && (write || tool.metadata?.filediff) && typeof inputPath === "string" ? inputPath : path;
  const multi = files.length > 1;
  const diagnostics = tool.metadata?.diagnostics;
  const messages = diagnostics && typeof diagnostics === "object" && path
    ? (diagnostics as Record<string, unknown>)[path] : undefined;
  const errors = Array.isArray(messages) ? messages.filter((value) => value?.severity === 1).slice(0, 3) : [];
  return <div data-component={write ? "write-tool" : patch || multi ? "apply-patch-tool" : "edit-tool"} data-tool={name} data-status={tool.status} data-timeline-part-id={tool.id}>
    <div data-component="collapsible" className="tool-collapsible" data-expanded={open ? "" : undefined}>
      <button type="button" data-slot="collapsible-trigger" aria-expanded={open} onClick={() => { if (!pending) setOpen((value) => !value); }}>
        <div data-component="tool-trigger"><div data-slot="basic-tool-tool-trigger-content"><div data-slot="basic-tool-tool-info">
          {multi ? <div data-slot="basic-tool-tool-info-structured"><div data-slot="basic-tool-tool-info-main">
            <span data-slot="basic-tool-tool-title"><TextShimmer text="Patch" active={pending} /></span>
            <span data-slot="basic-tool-tool-subtitle">{files.length + " files"}</span>
          </div></div> : <div data-component={write ? "write-trigger" : "edit-trigger"}>
            <div data-slot="message-part-title-area">
              <div data-slot="message-part-title">
                <span data-slot="message-part-title-text"><TextShimmer text={write ? "Write" : patch ? "Patch" : "Edit"} active={pending} /></span>
                {!pending && <span data-slot="message-part-title-filename" onClick={(event) => { event.stopPropagation(); activatePath(path); }}>{fileName(triggerPath)}</span>}
              </div>
              {!pending && triggerPath.includes("/") && <div data-slot="message-part-path"><span data-slot="message-part-directory">{contextDirectory(triggerPath, session)}</span></div>}
            </div>
            <div data-slot="message-part-actions">{!pending && !write && (patch || files[0]?.additions !== undefined) && <DiffChanges file={files[0]} />}</div>
          </div>}
        </div></div>{!pending && <span data-slot="collapsible-arrow"><span data-slot="collapsible-arrow-icon"><OpenCodeIcon name="chevron-down" /></span></span>}</div>
      </button>
      {open && <div data-slot="collapsible-content" data-expanded="">
        {path && <div data-component="accordion" data-scope="apply-patch" style={{ "--sticky-accordion-offset": "calc(32px + var(--tool-content-gap))" } as CSSProperties}>
          {files.map((file) => <ToolFileAccordion key={file.file} file={file} write={write} patch={patch} session={session} activatePath={() => activatePath(file.file)} />)}
        </div>}
        {errors.length > 0 && <div data-component="diagnostics">{errors.map((value, index) => <div data-slot="diagnostic" key={index}>
          <span data-slot="diagnostic-label">Error</span><span data-slot="diagnostic-location">{"[" + (value.range.start.line + 1) + ":" + (value.range.start.character + 1) + "]"}</span><span data-slot="diagnostic-message">{value.message}</span>
        </div>)}</div>}
      </div>}
    </div>
  </div>;
}

function ToolErrorCard({ tool, session }: { tool: ToolCallView; session: SessionInfo | null }): ReactNode {
  const activatePath = useToolFileActivation(session);
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const presentation = toolPresentation(tool);
  const errorTitles: Record<string, string> = { read: "Read", list: "List", glob: "Glob", grep: "Grep", task: "Task", webfetch: "Webfetch", bash: "Shell", shell: "Shell", patch: "Patch", apply_patch: "Patch", question: "Questions" };
  presentation.title = tool.title === "websearch" ? openCodeSpecialToolTitle(tool)! : errorTitles[tool.title] ?? tool.title;
  const failurePath = presentation.path;
  const cleaned = (tool.output ?? "").replace(/^Error:\s*/, "").trim();
  const prefix = `${effectiveToolKey(tool)} `;
  const tail = cleaned.startsWith(prefix) ? cleaned.slice(prefix.length) : cleaned;
  const segments = tail.split(": ");
  const head = segments[0]?.trim();
  // OpenCode capitalizes the error head: "bash: ..." -> "Bash".
  const subtitle = segments.length <= 1 || !head ? "Failed" : head[0].toUpperCase() + head.slice(1);
  const body = segments.length <= 1 ? cleaned : segments.slice(1).join(": ").trim() || cleaned;
  const activateSubtitle = failurePath
    ? (): void => {
        activatePath(failurePath);
      }
    : undefined;
  const copy = (): void => {
    if (!cleaned) return;
    void navigator.clipboard?.writeText(cleaned).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };
  return (
    <div data-component="tool-part-wrapper" data-tool={effectiveToolKey(tool)} data-variant="inline" data-status="failed" data-timeline-part-id={tool.id}>
      <div data-component="card" data-kind="tool-error-card" data-variant="error" data-open={open ? "true" : "false"}>
      <div data-component="collapsible" className="tool-collapsible" data-open={open ? "true" : "false"}>
        <button data-slot="collapsible-trigger" aria-expanded={open} onClick={() => setOpen((value) => !value)}>
          <div data-component="tool-trigger" data-clickable="true">
            <div data-slot="basic-tool-tool-trigger-content">
              <span data-slot="basic-tool-tool-indicator" data-component="tool-error-card-icon" style={{ strokeWidth: 1.5 }}><OpenCodeIcon name="circle-ban-sign" /></span>
              <div data-slot="basic-tool-tool-info">
                <div data-slot="basic-tool-tool-info-structured">
                  <div data-slot="basic-tool-tool-info-main">
                    <span data-slot="basic-tool-tool-title">{presentation.title}</span>
                    <span
                      data-slot="basic-tool-tool-subtitle"
                      className={activateSubtitle ? "clickable" : undefined}
                      onClick={activateSubtitle
                        ? (event) => {
                            event.stopPropagation();
                            activateSubtitle();
                          }
                        : undefined}
                    >
                      {subtitle}
                    </span>
                  </div>
                </div>
              </div>
            </div>
            <span data-slot="collapsible-arrow"><span data-slot="collapsible-arrow-icon"><OpenCodeIcon name="chevron-down" /></span></span>
          </div>
        </button>
        {open && (
          <div data-slot="collapsible-content">
            <div data-slot="tool-error-card-content">
              <div data-slot="tool-error-card-copy">
                <div data-component="tooltip-trigger" data-tooltip={copied ? "Copied" : "Copy error"}><button
                  data-component="icon-button" data-size="normal" data-variant="ghost" aria-label={copied ? "Copied" : "Copy error"}
                  data-slot="tool-error-card-copy-button" onMouseDown={(event) => event.preventDefault()}
                  onClick={(event) => {
                    event.stopPropagation();
                    copy();
                  }}
                >
                  <OpenCodeIcon name={copied ? "check" : "copy"} />
                </button></div>
              </div>
              <div data-slot="tool-error-card-description">{body}</div>
            </div>
          </div>
        )}
      </div>
      </div>
    </div>
  );
}

function SessionProgressIndicator(): ReactNode {
  const dots = Array.from({ length: 25 }, (_, index) => ({
    index,
    x: 1.5 + (index % 5) * 3,
    y: 1.5 + Math.floor(index / 5) * 3
  }));
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" data-component="session-progress-indicator-v2" aria-hidden="true">
      {dots.map((dot) => <rect data-dot={dot.index} x={dot.x} y={dot.y} width="2" height="2" key={dot.index} />)}
    </svg>
  );
}

function SubagentIcon(): ReactNode {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M4.5 5C4.5 4.72386 4.72386 4.5 5 4.5H11C11.2761 4.5 11.5 4.72386 11.5 5V11C11.5 11.2761 11.2761 11.5 11 11.5H5C4.72386 11.5 4.5 11.2761 4.5 11V5Z" fill="currentColor" />
      <path d="M13.5 2C13.7761 2 14 2.22386 14 2.5V13.5C14 13.7761 13.7761 14 13.5 14H2.5C2.22386 14 2 13.7761 2 13.5V2.5C2 2.22386 2.22386 2 2.5 2H13.5ZM3 13H13V3H3V13Z" fill="currentColor" />
    </svg>
  );
}

function DelegatedAgentCard({
  id,
  title,
  agent,
  description,
  state,
  tone,
  childID,
  onOpen
}: {
  id: string;
  title: string;
  agent: string;
  description: string;
  state: string;
  tone: "working" | "complete" | "failed";
  childID: string;
  onOpen: () => void;
}): ReactNode {
  const { agents } = useStore();
  const appearance = resolveOpenCodeTaskAgent(agent, agents);
  const agentTitle = appearance.name ?? "Agent";
  return (
    <div data-component="collapsible" className="tool-collapsible">
      <button data-slot="collapsible-trigger" data-hide-details="true" disabled={!childID}
        aria-label={childID ? `Open delegated agent session: ${title}` : `Delegated agent: ${title}`} onClick={onOpen}>
        <div data-component="tool-trigger" data-hide-details="true" data-clickable={childID ? "true" : undefined}>
          <div data-slot="basic-tool-tool-trigger-content"><div data-slot="basic-tool-tool-info">
            <div data-component="task-tool-card" data-state={state} data-timeline-part-id={id} style={{ "--task-agent-color": appearance.v2Color, "--task-agent-legacy-color": appearance.color } as CSSProperties}>
              <div data-component="task-tool-surface">
                <div data-slot="basic-tool-tool-info-structured"><div data-slot="basic-tool-tool-info-main">
                  <span data-component={tone === "working" ? "task-tool-spinner" : "task-tool-icon"}>
                    {tone === "working" ? <SessionProgressIndicator /> : <SubagentIcon />}
                  </span>
                  <span data-component="task-tool-title">{agentTitle}</span>
                  {description && <span data-slot="basic-tool-tool-subtitle">{description}</span>}
                </div></div>
              </div>
              {childID && <span data-component="task-tool-action"><OpenCodeIcon name="square-arrow-top-right" /></span>}
            </div>
          </div></div>
        </div>
      </button>
    </div>
  );
}

function TaskTool({ tool, session }: { tool: ToolCallView; session: SessionInfo | null }): ReactNode {
  const { agents, sessions, reopenSession } = useStore();
  const request = taskDescriptor(tool);
  const requested = request.agent;
  const configured = agents.find((agent) =>
    agent.id.toLowerCase() === requested.toLowerCase() || agent.name.toLowerCase() === requested.toLowerCase()
  );
  const description = request.description;
  const childSession = taskChildID(tool, sessions, session?.id, request);
  const resolved = sessions.find((candidate) => candidate.id === childSession);
  const agentName = configured?.name ?? requested;
  const title = resolved?.title.trim()
    ? resolved.title
    : agentName
      ? titleCase(agentName)
      : "Subagent";
  const detail = description || childSession;
  const subtitle = tool.metadata?.background === true && detail ? `${detail} (background)` : detail;
  const running = tool.status === "running";
  return <DelegatedAgentCard
    id={tool.id}
    title={title}
    agent={agentName}
    description={subtitle}
    state={tool.status}
    tone={running ? "working" : tool.status === "failed" ? "failed" : "complete"}
    childID={childSession}
    onOpen={() => childSession && void reopenSession(childSession)}
  />;
}

export function SubagentLink({ item, session }: { item: Extract<TranscriptItem, { kind: "synthetic" }>; session: SessionInfo | null }): ReactNode {
  const { agents, sessions, reopenSession } = useStore();
  const ref = parseSubagentTag(item.text) ?? parseLegacyTaskText(item.text);
  const childID = ref ? subagentChildID(ref, sessions, session?.id) : "";
  const resolved = sessions.find((candidate) => candidate.id === childID);
  const configured = ref?.agent
    ? agents.find((agent) =>
        agent.id.toLowerCase() === ref.agent.toLowerCase() || agent.name.toLowerCase() === ref.agent.toLowerCase()
      )
    : undefined;
  const agentName = resolved?.agent ?? configured?.name ?? ref?.agent ?? "";
  const state = ref?.state ?? "";
  const failed = state === "error" || state === "cancelled";
  const running = !state || state === "running";
  const title = resolved?.title.trim()
    ? resolved.title
    : ref?.id
      ? ref.description || "Subagent"
      : agentName
        ? titleCase(agentName)
        : "Subagent";
  const detail = ref?.description
    ? (ref.description.length > 100 ? `${ref.description.slice(0, 100)}…` : ref.description)
    : "";
  return <DelegatedAgentCard
    id={item.id}
    title={title}
    agent={agentName}
    description={detail}
    state={state || "running"}
    tone={failed ? "failed" : running ? "working" : "complete"}
    childID={childID}
    onOpen={() => childID && void reopenSession(childID)}
  />;
}

export function ToolPart({ tool, session }: { tool: ToolCallView; session: SessionInfo | null }): ReactNode {
  const activatePath = useToolFileActivation(session);
  const [open, setOpen] = useState(false);
  const presentation = toolPresentation(tool);
  const output = tool.output ?? "";
  const input = tool.inputValue === undefined
    ? tool.input ?? ""
    : typeof tool.inputValue === "string"
      ? tool.inputValue
      : JSON.stringify(tool.inputValue, null, 2);
  const files = tool.content?.filter((item) => item.type === "file") ?? [];
  const native = tool.metadata?.deepseek;
  const nativeRecord = native && typeof native === "object" && !Array.isArray(native) ? native as Record<string, unknown> : null;
  const subCalls = Array.isArray(nativeRecord?.subCalls) ? nativeRecord.subCalls.flatMap((item): ToolCallView[] => {
    if (!item || typeof item !== "object" || Array.isArray(item)) return [];
    const value = item as Record<string, unknown>;
    if (typeof value.id !== "string" || typeof value.title !== "string") return [];
    const status = value.status === "success" || value.status === "failed" ? value.status : "running";
    return [{
      id: value.id,
      title: value.title,
      detail: typeof value.detail === "string" ? value.detail : "",
      status,
      ...(typeof value.input === "string" ? { input: value.input } : {}),
      ...(value.inputValue !== undefined ? { inputValue: value.inputValue } : {}),
      ...(typeof value.output === "string" ? { output: value.output } : {}),
      ...(typeof value.startedAt === "number" ? { startedAt: value.startedAt } : {})
    }];
  }) : [];
  const expandable = input.length > 0 || output.length > 0 || files.length > 0 || subCalls.length > 0;
  const displayInput = input.length > OUTPUT_LIMIT ? `${input.slice(0, OUTPUT_LIMIT)}\n… (truncated)` : input;
  const displayOutput = output.length > OUTPUT_LIMIT ? `${output.slice(0, OUTPUT_LIMIT)}\n… (truncated)` : output;
  const shellInput = toolInput(tool);
  const shell = ["bash", "shell"].includes(effectiveToolKey(tool));
  const shellCommand = typeof shellInput.command === "string" ? shellInput.command : "";
  const shellOutput = stripAnsi(output || (typeof tool.metadata?.output === "string" ? tool.metadata.output : "")).replace(/\r\n?/g, "\n");
  const shellText = `$ ${shellCommand || (typeof tool.metadata?.command === "string" ? tool.metadata.command : "")}${shellOutput ? `\n\n${shellOutput}` : ""}`;
  const sawPending = useRef(tool.status === "running").current;
  const [copied, setCopied] = useState(false);
  const copyShell = (): void => {
    if (!shellText.trim()) return;
    void writeClipboard(shellText).then((success) => {
      if (!success) return;
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };
  const activateSubtitle = (): void => {
    if (!presentation.path) return;
    activatePath(presentation.path);
  };

  if (toolKey(tool.title) === "todowrite") return null;
  if (isDismissedOpenCodeQuestion(tool)) return <OpenCodeSpecialTool tool={tool} />;
  if (tool.status === "failed") return <ToolErrorCard tool={tool} session={session} />;
  if (toolKey(tool.title) === "task" || toolKey(tool.title) === "subagent") return <TaskTool tool={tool} session={session} />;
  if (isEditCardTool(tool)) return <EditToolCard tool={tool} session={session} />;
  if (!shell && !nativeRecord) return <OpenCodeSpecialTool tool={tool} />;

  return (
    <div data-component="tool-part-wrapper" data-tool={effectiveToolKey(tool)} data-variant="inline" data-status={tool.status} data-timeline-part-id={tool.id}>
      <div data-component="collapsible" className="tool-collapsible" data-expanded={open ? "true" : undefined}>
        <button
          data-slot="collapsible-trigger"
          aria-expanded={open}
          disabled={!expandable}
          onClick={() => expandable && setOpen((value) => !value)}
        >
          <div data-component="tool-trigger" data-clickable={expandable ? "true" : undefined}>
            <div data-slot="basic-tool-tool-trigger-content">
              <div data-slot="basic-tool-tool-info">
                <div data-slot="basic-tool-tool-info-structured">
                  <div data-slot="basic-tool-tool-info-main">
                    <span data-slot="basic-tool-tool-title">
                      <TextShimmer text={presentation.title} active={tool.status === "running"} />
                    </span>
                    {shell ? !open && shellCommand && <ShellSubmessage text={shellCommand} animate={sawPending} /> : presentation.subtitle && (
                      <span
                        data-slot="basic-tool-tool-subtitle"
                        className={presentation.path ? "clickable" : undefined}
                        title={presentation.subtitle}
                        onClick={(event) => {
                          if (!presentation.path) return;
                          event.stopPropagation();
                          activateSubtitle();
                        }}
                      >
                        {presentation.subtitle}
                      </span>
                    )}
                  </div>
                </div>
              </div>
            </div>
            {expandable && <span data-slot="collapsible-arrow"><span data-slot="collapsible-arrow-icon"><OpenCodeIcon name="chevron-down" /></span></span>}
          </div>
        </button>
        {tool.progress && tool.status === "running" && (
          <div data-slot="tool-progress"><TextShimmer text={progressText(tool.progress)} /></div>
        )}
        {expandable && open && (
          <div data-slot="collapsible-content">
            {shell && (input || output) ? (
              <div data-component="bash-output" dir="ltr">
                <div data-slot="bash-copy">
                  <div data-component="tooltip-v2-trigger" data-tooltip={copied ? "Copied" : "Copy"}><button
                    data-component="icon-button-v2" data-size="normal" data-variant="ghost-muted" aria-label={copied ? "Copied" : "Copy"} onMouseDown={(event) => event.preventDefault()}
                    data-slot="bash-copy-button"
                    onClick={(event) => {
                      event.stopPropagation();
                      copyShell();
                    }}
                  >
                    <OpenCodeIcon name={copied ? "check" : "outline-copy"} />
                  </button></div>
                </div>
                <div data-slot="bash-scroll" data-scrollable="" tabIndex={0} role="region" aria-label="Scrollable content"><pre data-slot="bash-pre"><code>{shellText}</code></pre></div>
              </div>
            ) : (input || output) ? (
              <div data-component="tool-io">
                {input && (
                  <div data-component="tool-io-section">
                    <span data-slot="tool-io-label">IN</span>
                    <pre data-slot="tool-io-text">{displayInput}</pre>
                  </div>
                )}
                {input && output && <span data-slot="tool-io-divider" />}
                {output && (
                  <div data-component="tool-io-section">
                    <span data-slot="tool-io-label">OUT</span>
                    <pre data-slot="tool-io-text">{displayOutput}</pre>
                  </div>
                )}
              </div>
            ) : null}
            {files.length > 0 && (
              <div data-component="tool-files">
                {files.map((file) => (
                  <ExternalLink href={file.uri} key={`${file.uri}:${file.name ?? ""}`}>
                    <span className="codicon codicon-file" />
                    <span>{file.name ?? file.uri}</span>
                    <span data-slot="tool-file-mime">{file.mime}</span>
                  </ExternalLink>
                ))}
              </div>
            )}
            {subCalls.length > 0 && (
              <div data-component="nested-tool-calls">
                {subCalls.map((subCall) => <ToolPart tool={subCall} session={session} key={subCall.id} />)}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function ContextToolRow({ tool, session }: { tool: ToolCallView; session: SessionInfo | null }): ReactNode {
  const activatePath = useToolFileActivation(session);
  const name = effectiveToolKey(tool);
  const input = toolInput(tool);
  const base = toolPresentation(tool);
  // OpenCode's `contextToolTrigger`: list/glob/grep subtitle is the search directory; the
  // pattern/include live in `contextToolArgs` so they are never shown twice.
  const presentation =
    name === "list" || name === "glob" || name === "grep"
      ? { title: base.title, subtitle: contextDirectory(typeof input.path === "string" ? input.path : undefined, session), path: base.path }
      : base;
  const args = contextToolArgs(tool);
  const activateSubtitle = presentation.path
    ? (): void => {
        activatePath(presentation.path!);
      }
    : undefined;
  return (
    <div data-slot="context-tool-group-item">
      <div data-component="tool-trigger">
        <div data-slot="basic-tool-tool-trigger-content">
          <div data-slot="basic-tool-tool-info">
            <div data-slot="basic-tool-tool-info-structured">
              <div data-slot="basic-tool-tool-info-main">
                <span data-slot="basic-tool-tool-title">
                  <TextShimmer text={presentation.title} active={tool.status === "running"} />
                </span>
                {presentation.subtitle && (
                  <span
                    data-slot="basic-tool-tool-subtitle"
                    className={activateSubtitle ? "clickable" : undefined}
                    title={presentation.subtitle}
                    onClick={activateSubtitle
                      ? (event) => {
                          event.stopPropagation();
                          activateSubtitle();
                        }
                      : undefined}
                  >
                    {presentation.subtitle}
                  </span>
                )}
                {args.map((arg) => <span data-slot="basic-tool-tool-arg" key={arg}>{arg}</span>)}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export function ContextToolGroup({ tools, session, working = false }: { tools: ToolCallView[]; session: SessionInfo | null; working?: boolean }): ReactNode {
  const [open, setOpen] = useState(false);
  const summary = contextToolSummary(tools);
  const pending = working || tools.some((tool) => tool.status === "running");
  const items: CountItem[] = [
    { key: "read", count: summary.read },
    { key: "search", count: summary.search },
    { key: "list", count: summary.list }
  ];
  return (
    <div
      data-component="context-tool-group"
      className="tool-collapsible"
      data-timeline-part-ids={tools.map((tool) => tool.id).join(",")}
      data-expanded={open ? "true" : undefined}
    >
      <button data-slot="collapsible-trigger" aria-expanded={open} onClick={() => setOpen((value) => !value)}>
        <div data-component="context-tool-group-trigger">
          <span data-slot="context-tool-group-title">
            <span data-slot="context-tool-group-label">
              <ToolStatusTitle active={pending} activeText="Exploring" doneText="Explored" split={false} />
            </span>
            <span data-slot="context-tool-group-summary">
              <AnimatedCountList items={items} fallback="" />
            </span>
          </span>
          <span data-slot="collapsible-arrow"><span data-slot="collapsible-arrow-icon"><OpenCodeIcon name="chevron-down" /></span></span>
        </div>
      </button>
      {open && (
        <div data-slot="collapsible-content">
          <div data-component="context-tool-group-list">
            {tools.map((tool) => <ContextToolRow tool={tool} session={session} key={tool.id} />)}
          </div>
        </div>
      )}
    </div>
  );
}

