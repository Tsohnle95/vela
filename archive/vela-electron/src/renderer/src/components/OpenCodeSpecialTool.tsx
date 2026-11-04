// ToolRegistry / GenericTool port from OpenCode 03e67171ab2dc1e7f16e8cebfbc7f778f61b89f0 (MIT; styles/opencode/LICENSE).
import { useId, useState, type ReactNode } from "react";
import type { ToolCallView } from "@shared/types";
import { safeExternalUrl } from "@shared/url-policy";
import { OpenCodeIcon } from "./OpenCodeIcon";
import { TextShimmer } from "./TextShimmer";

function inputOf(tool: ToolCallView): Record<string, unknown> {
  const value = tool.inputValue ?? tool.input;
  if (value && typeof value === "object" && !Array.isArray(value)) return value as Record<string, unknown>;
  if (typeof value !== "string") return {};
  try {
    const parsed: unknown = JSON.parse(value);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed as Record<string, unknown> : {};
  } catch {
    return {};
  }
}

function string(value: unknown): string {
  return typeof value === "string" ? value : "";
}

export function openCodeSpecialToolTitle(tool: ToolCallView): string | undefined {
  switch (tool.title) {
    case "webfetch": return "Webfetch";
    case "websearch": {
      const provider = tool.metadata?.provider;
      return provider === "parallel" ? "Parallel Web Search" : provider === "exa" ? "Exa Web Search" : "Web Search";
    }
    case "skill": return string(inputOf(tool).name) || "Skill";
    case "question": return "Questions";
  }
}

export function isDismissedOpenCodeQuestion(tool: ToolCallView): boolean {
  return tool.title === "question" && tool.status === "failed" && (tool.output ?? "").replace("Error: ", "").includes("dismissed this question");
}

function BasicTool({ tool, title, subtitle, subtitleClass, args, titleClass, structured, hideDetails, defaultOpen = false, hasChildren = false, children }: {
  tool: ToolCallView;
  title?: string;
  subtitle?: string;
  subtitleClass?: string;
  args?: string[];
  titleClass?: string;
  structured?: ReactNode;
  hideDetails?: boolean;
  defaultOpen?: boolean;
  hasChildren?: boolean;
  children?: ReactNode;
}): ReactNode {
  const [open, setOpen] = useState(defaultOpen);
  const contentID = useId();
  const pending = tool.status === "running";
  return <div data-component="tool-part-wrapper" data-timeline-part-id={tool.id}>
    <div data-component="collapsible" data-variant="normal" className="tool-collapsible" data-expanded={open ? "" : undefined} data-closed={!open ? "" : undefined}>
      <button type="button" data-slot="collapsible-trigger" aria-expanded={open} aria-controls={hasChildren && !hideDetails ? contentID : undefined}
        data-expanded={open ? "" : undefined} data-closed={!open ? "" : undefined} data-hide-details={hideDetails ? "true" : undefined}
        onClick={() => { if (!pending) setOpen((value) => !value); }}>
        <div data-component="tool-trigger" data-hide-details={hideDetails ? "true" : undefined}>
          <div data-slot="basic-tool-tool-trigger-content"><div data-slot="basic-tool-tool-info">
            {structured ?? <div data-slot="basic-tool-tool-info-structured"><div data-slot="basic-tool-tool-info-main">
              <span data-slot="basic-tool-tool-title" className={titleClass}><TextShimmer text={title ?? ""} active={pending} /></span>
              {subtitle && <span data-slot="basic-tool-tool-subtitle" className={subtitleClass}>{subtitle}</span>}
              {args?.map((arg, index) => <span data-slot="basic-tool-tool-arg" key={index}>{arg}</span>)}
            </div></div>}
          </div></div>
          {hasChildren && !hideDetails && !pending && <div data-slot="collapsible-arrow"><span data-slot="collapsible-arrow-icon"><OpenCodeIcon name="chevron-down" /></span></div>}
        </div>
      </button>
      {open && hasChildren && !hideDetails && <div id={contentID} data-slot="collapsible-content" data-expanded="">{children}</div>}
    </div>
  </div>;
}

function ToolLink({ url, slot, className }: { url: string; slot: string; className?: string }): ReactNode {
  const href = safeExternalUrl(url);
  return <a data-slot={slot} className={className} href={href ?? undefined} target={href ? "_blank" : undefined} rel="noopener noreferrer"
    onClick={(event) => {
      event.stopPropagation();
      event.preventDefault();
      if (href) window.open(href, "_blank", "noopener,noreferrer");
    }}>{url}</a>;
}

function Webfetch({ tool }: { tool: ToolCallView }): ReactNode {
  const pending = tool.status === "running";
  const url = string(inputOf(tool).url);
  return <BasicTool tool={tool} hideDetails structured={
    <div data-slot="basic-tool-tool-info-structured">
      <div data-slot="basic-tool-tool-info-main">
        <span data-slot="basic-tool-tool-title"><TextShimmer text="Webfetch" active={pending} /></span>
        {!pending && url && <ToolLink url={url} slot="basic-tool-tool-subtitle" className="clickable subagent-link" />}
      </div>
      {!pending && url && <div data-component="tool-action"><div data-component="icon" data-size="small">
        <svg data-slot="icon-svg" viewBox="0 0 20 20" fill="none" aria-hidden="true">
          <path d="M7.91675 2.9165H2.91675V17.0832H17.0834V12.0832M12.0834 2.9165H17.0834V7.9165M9.58342 10.4165L16.6667 3.33317" stroke="currentColor" strokeLinecap="square" />
        </svg>
      </div></div>}
    </div>
  } />;
}

function urls(text: string | undefined): string[] {
  if (!text) return [];
  const seen = new Set<string>();
  return [...text.matchAll(/https?:\/\/[^\s<>"'`)\]]+/g)]
    .map((item) => item[0].replace(/[),.;:!?]+$/g, ""))
    .filter((item) => {
      if (seen.has(item)) return false;
      seen.add(item);
      return true;
    });
}

function Websearch({ tool }: { tool: ToolCallView }): ReactNode {
  const links = urls(tool.output);
  return <BasicTool tool={tool} title={openCodeSpecialToolTitle(tool)} subtitle={string(inputOf(tool).query)} subtitleClass="exa-tool-query" hasChildren>
    {links.length > 0 && <div data-component="exa-tool-output"><div data-slot="exa-tool-links">
      {links.map((url) => <ToolLink key={url} url={url} slot="exa-tool-link" />)}
    </div></div>}
  </BasicTool>;
}

function Question({ tool }: { tool: ToolCallView }): ReactNode {
  const values = inputOf(tool).questions;
  const questions = Array.isArray(values) ? values.map((value) => value && typeof value === "object" ? string(value.question) : "") : [];
  const answers = Array.isArray(tool.metadata?.answers) ? tool.metadata.answers.map((value) => Array.isArray(value) ? value.filter((answer): answer is string => typeof answer === "string") : []) : [];
  const completed = answers.length > 0;
  const count = questions.length;
  const subtitle = count === 0 ? "" : completed ? `${count} answered` : `${count} ${count > 1 ? "questions" : "question"}`;
  return <BasicTool tool={tool} title="Questions" subtitle={subtitle} defaultOpen={completed} hasChildren>
    {completed && <div data-component="question-answers">{questions.map((question, index) =>
      <div data-slot="question-answer-item" key={index}>
        <div data-slot="question-text">{question}</div>
        <div data-slot="answer-text">{answers[index]?.join(", ") || "(no answer)"}</div>
      </div>
    )}</div>}
  </BasicTool>;
}

export function OpenCodeGenericTool({ tool }: { tool: ToolCallView }): ReactNode {
  const input = inputOf(tool);
  const keys = ["description", "query", "url", "filePath", "path", "pattern", "name"];
  const subtitle = keys.map((key) => input[key]).find((value): value is string => typeof value === "string" && value.length > 0);
  const args = Object.entries(input).filter(([key]) => !keys.includes(key)).flatMap(([key, value]) =>
    typeof value === "string" || typeof value === "number" || typeof value === "boolean" ? [`${key}=${value}`] : []
  ).slice(0, 3);
  return <BasicTool tool={tool} title={`Called \`${tool.title}\``} subtitle={subtitle} args={args} />;
}

/** Normal/error dispatch stays with the timeline; question dismissal is handled before its generic error card. */
export function OpenCodeSpecialTool({ tool }: { tool: ToolCallView }): ReactNode {
  switch (tool.title) {
    case "webfetch": return <Webfetch tool={tool} />;
    case "websearch": return <Websearch tool={tool} />;
    case "skill": return <BasicTool tool={tool} title={openCodeSpecialToolTitle(tool)} titleClass="capitalize agent-title" hideDetails />;
    case "question":
      if (tool.status === "running") return null;
      if (isDismissedOpenCodeQuestion(tool)) return <div data-component="tool-part-wrapper" data-timeline-part-id={tool.id}>
        <div style={{ width: "100%", display: "flex", justifyContent: "flex-end" }}>
          <span className="text-13-regular text-text-weak cursor-default">Questions dismissed</span>
        </div>
      </div>;
      return <Question tool={tool} />;
    default: return <OpenCodeGenericTool tool={tool} />;
  }
}
