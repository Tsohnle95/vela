import { memo, useEffect, useMemo, useState, type ReactNode } from "react";
import { TerminalIcon, CircleCheckIcon, CircleAlertIcon, LoaderCircleIcon, CopyIcon, CheckIcon, PencilIcon, RefreshCcwIcon } from "lucide-react";
import type { SessionInfo, SessionTranscript, ToolCallView, TranscriptItem } from "@shared/types";
import { useStore } from "../store";
import { Bubble, BubbleContent } from "@/components/ui/bubble";
import { MessageScrollerItem } from "@/components/ui/message-scroller";
import { Message, MessageContent, MessageFooter } from "@/components/ui/message";
import { Trace, TraceCollapsible, TraceCollapsibleContent, TraceCollapsibleTrigger, TraceIcon, TraceLine, TraceTitle } from "@/components/ui/trace";
import { Item, ItemMedia, ItemContent, ItemTitle, ItemDescription, ItemActions } from "@/components/ui/item";
import { Button } from "@/components/ui/button";
import { CodeBlock, CodeBlockContent } from "@/components/ui/code-block";
import { Marker, MarkerContent } from "@/components/ui/marker";
import { Markdown } from "./demos/lib/markdown";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { writeClipboard } from "../components/TimelineProse";
import { parseSubagentTag, parseLegacyTaskText, isInternalSystemReminder, taskChildID, taskDescriptor, toolInput, toolKey, workspaceFilePath, editFileEntries, isEditCardTool } from "../components/timeline-model";
import { ReasoningRow, Sources, StackTrace, TestResults, TodoQueue, FileTreeNode, PlanRow, TaskRow, AiElementsCheckpoint, Citation } from "./patterns";
import { fileTree, structuredToolOutput, structuredActivity, visibleAiTranscript, toolOutputText, structuredCitedText } from "./adapters";
export interface AiElementsTimelineProps {
    transcript: TranscriptItem[];
    busy: boolean;
    lastAssistantId: string | null;
    session?: SessionInfo | null;
    onRetry?: (messageID: string) => void;
    scrollerItems?: boolean;
}
type ToolProps = {tool: ToolCallView; streaming: boolean; session: SessionInfo | null; interrupted?: boolean};
type AssistantItem = Extract<TranscriptItem,{kind:"assistant"}>;
type SessionProps = {session: SessionInfo | null; busy: boolean};

export function AiElementsTool(props: ToolProps) {
    const {tool,session}=props;
    const {openFile}=useStore();
    const structured=structuredToolPresentation(tool,props.streaming,path => {
        const target=workspaceFilePath(path,session);
        if (target && session) void openFile(target,session.workspace);
    });
    if (structured) return structured;
    if (["task","subagent"].includes(toolKey(tool.title))) return <AiElementsDelegatedTool {...props}/>;
    return <AiElementsToolOutput {...props}/>;
}

function structuredToolPresentation(tool: ToolCallView, streaming: boolean, openFrame: (path:string) => void): ReactNode {
    const cited=structuredCitedText(tool);
    if (cited) return <CitedToolText value={cited}/>;
    const activity=structuredActivity(tool);
    if (activity.plan) return <PlanRow plan={activity.plan} streaming={streaming}/>;
    if (activity.task) return <TaskRow task={activity.task} streaming={streaming}/>;
    const rich=structuredToolOutput(tool);
    if (rich.suites.length) return <TestResults suites={rich.suites}/>;
    if (rich.stack) return <StackTrace stack={rich.stack} onOpenFrame={openFrame}/>;
    if (rich.sources.length) return <Sources sources={rich.sources}/>;
    return null;
}

function AiElementsDelegatedTool({tool,streaming,session,interrupted=false}:ToolProps) {
    const store=useStore();
    const childID=taskChildID(tool,store.sessions,session?.id,taskDescriptor(tool));
    const childView=Object.values(store.panelViews).find(view => view.session?.id === childID);
    return <AiElementsSubagent tool={tool} childID={childID === session?.id ? "" : childID} transcript={childView?.transcript} childSession={childView?.session} running={streaming && tool.status === "running"} interrupted={interrupted}/>;
}

function toolRunState(tool: ToolCallView, streaming:boolean, interrupted:boolean) {
    const running=streaming && tool.status === "running";
    const failed=tool.status === "failed" || (!streaming && tool.status === "running" && !interrupted);
    const stopped=interrupted && tool.status === "running";
    return {running,failed,verb: running ? "Running" : failed ? "Failed to run" : stopped ? "Stopped" : "Ran"};
}

function AiElementsToolOutput({tool,streaming,session,interrupted=false}: ToolProps) {
    const [open,setOpen]=useState<boolean>();
    const {running,failed,verb}=toolRunState(tool,streaming,interrupted);
    const terminal=["bash","shell","terminal","runcommand"].includes(toolKey(tool.title));
    const input=toolInput(tool);
    const command=typeof input.command === "string" ? input.command : tool.detail || tool.title;
    const changedPaths=isEditCardTool(tool) && tool.status === "success" ? editFileEntries(tool).map(file => file.file).filter(Boolean) : [];
    const line=<TraceLine status={running ? "running" : failed ? "error" : undefined}><TraceIcon><TerminalIcon/></TraceIcon><TraceTitle>{verb} {command}</TraceTitle></TraceLine>;
    const output=toolOutputText(tool);
    if (!terminal && running && !output) return line;
    return <TraceCollapsible open={open ?? (terminal ? running && output.length > 0 : failed)} onOpenChange={setOpen}>
        <TraceCollapsibleTrigger>{line}</TraceCollapsibleTrigger>
        <TraceCollapsibleContent>
            {changedPaths.length > 0 && <AiElementsFileTree paths={changedPaths} session={session}/>}
            <CodeBlock size="sm"><CodeBlockContent wrap className="max-h-64">{output || tool.input || "No output"}</CodeBlockContent></CodeBlock>
        </TraceCollapsibleContent>
    </TraceCollapsible>;
}
export function AiElementsPermission({ item, session }: {
    item: Extract<TranscriptItem, {
        kind: "permission";
    }>;
    session: SessionInfo | null;
}) {
    const { replyPermission } = useStore();
    if (!item.pending)
        return <Trace><TraceLine><TraceTitle>{item.resolvedWith === "reject" ? "Denied" : "Approved"} {item.action}</TraceTitle></TraceLine></Trace>;
    return <Item variant="outline" size="sm"><ItemMedia variant="icon"><TerminalIcon /></ItemMedia><ItemContent><ItemTitle>{item.action}?</ItemTitle><ItemDescription>{item.resources.join(", ")}</ItemDescription></ItemContent><ItemActions><Button size="sm" variant="outline" disabled={!session} onClick={() => { if (session)
        void replyPermission(item.requestID, "reject", session.id); }}>Deny</Button><Button size="sm" disabled={!session} onClick={() => { if (session)
        void replyPermission(item.requestID, "once", session.id); }}>Approve</Button></ItemActions></Item>;
}
export function AiElementsFileTree({ paths, session }: {
    paths: string[];
    session: SessionInfo | null;
}) {
    const { openFile } = useStore();
    const [selected, setSelected] = useState("");
    const nodes = useMemo(() => fileTree(paths), [paths]);
    return <div className="flex flex-col">{nodes.map(node => <FileTreeNode key={node.path} node={node} selected={selected} onSelect={path => { setSelected(path); const target = workspaceFilePath(path, session); if (target)
        void openFile(target, session?.workspace); }}/>)}</div>;
}
export function AiElementsTranscript({transcript,busy,lastAssistantId,session=null,onRetry,scrollerItems=false}: AiElementsTimelineProps): ReactNode {
    return <>{transcript.map((item,index) => {
        if (hiddenEntry(item)) return null;
        const rendered=<AiElementsEntry key={item.id} item={item} next={transcript[index+1]} busy={busy} lastAssistantId={lastAssistantId} session={session} onRetry={onRetry}/>;
        return scrollerItems ? <MessageScrollerItem key={item.id} messageId={item.id} scrollAnchor={item.kind === "user"}>{rendered}</MessageScrollerItem> : rendered;
    })}</>;
}

function hiddenEntry(item: TranscriptItem):boolean {
    return item.kind === "system" || item.kind === "selection" || (item.kind === "synthetic" && isInternalSystemReminder(item));
}

type EntryProps = SessionProps & {item: TranscriptItem; next?: TranscriptItem; lastAssistantId:string|null; onRetry?: AiElementsTimelineProps["onRetry"]};
function AiElementsEntry({item,next,busy,lastAssistantId,session,onRetry}: EntryProps): ReactNode {
    switch (item.kind) {
        case "user":
        case "pending-input": return <AiElementsUserMessage item={item} busy={busy} session={session}/>;
        case "assistant": return <AiElementsAssistantMessage item={item} next={next} busy={busy} streaming={busy && (item.id === lastAssistantId || !item.completed)} session={session} onRetry={onRetry}/>;
        case "permission": return <AiElementsPermission item={item} session={session}/>;
        case "selection":
        case "system": return null;
        case "synthetic": return <AiElementsSyntheticMessage item={item} busy={busy} session={session}/>;
        case "shell": return <AiElementsShellMessage item={item} busy={busy} session={session}/>;
        case "divider": return <Marker variant="separator"><MarkerContent>Session compacted</MarkerContent></Marker>;
        case "compaction": return <AiElementsCompactionMessage item={item}/>;
        case "status": return <Message><MessageContent><Trace><TraceLine status={item.tone === "error" ? "error" : undefined}><TraceTitle>{item.text}</TraceTitle></TraceLine></Trace></MessageContent></Message>;
        case "skill": return <Message><MessageContent><Markdown>{item.text}</Markdown></MessageContent></Message>;
    }
}

function AiElementsUserMessage({item,session,busy}:SessionProps & {item:Extract<TranscriptItem,{kind:"user"|"pending-input"}>}) {
    const {stageRevert}=useStore();
    return <Message align="end"><MessageContent><Bubble align="end" variant="muted"><BubbleContent>{item.text}{item.attachments?.map((attachment,index) => <span key={index} className="block text-xs text-muted-foreground">{attachment.name}</span>)}</BubbleContent></Bubble><MessageFooter><MessageCopy text={item.text}/>{session && item.kind === "user" && <MessageAction label="Edit" disabled={busy} onClick={() => void stageRevert(session.workspace,item.id)}><PencilIcon/></MessageAction>}</MessageFooter></MessageContent></Message>;
}

function AiElementsAssistantMessage({item,next,streaming,busy,session,onRetry}: SessionProps & {item:AssistantItem;next?:TranscriptItem;streaming:boolean;onRetry?:AiElementsTimelineProps["onRetry"]}) {
    const {stageRevert}=useStore();
    return <Message><MessageContent className="gap-4"><AiElementsAssistantParts item={item} streaming={streaming} session={session}/>{item.retry && <Trace><TraceLine status="running"><TraceTitle>{item.retry.message} · attempt {item.retry.attempt}</TraceTitle></TraceLine></Trace>}{item.error && <Trace><TraceLine status="error"><TraceTitle>{item.error}</TraceTitle></TraceLine></Trace>}{item.completed && <AiElementsAssistantFooter item={item} busy={busy} onRetry={onRetry}/ >}{item.completed && session && next?.kind === "user" && <AiElementsCheckpoint label="Checkpoint" disabled={busy} onRestore={() => void stageRevert(session.workspace,next.id)}/>}</MessageContent></Message>;
}

function AiElementsAssistantFooter({item,busy,onRetry}: {item:AssistantItem;busy:boolean;onRetry?:AiElementsTimelineProps["onRetry"]}) {
    const text=item.parts.flatMap(part => part.kind === "text" ? [part.text] : []).join("\n\n");
    return <MessageFooter className="px-0"><MessageCopy text={text}/>{onRetry && <MessageAction label="Retry" disabled={busy} onClick={() => onRetry(item.messageID)}><RefreshCcwIcon/></MessageAction>}</MessageFooter>;
}

function AiElementsSyntheticMessage({item,busy,session}:SessionProps & {item:Extract<TranscriptItem,{kind:"synthetic"}>}) {
    if (isInternalSystemReminder(item)) return null;
    const ref=parseSubagentTag(item.text) ?? parseLegacyTaskText(item.text);
    if (!ref) return <Message><MessageContent><Markdown>{item.text}</Markdown></MessageContent></Message>;
    const status=["failed","error","cancelled"].includes(ref.state) ? "failed" : ["complete","completed","success"].includes(ref.state) ? "success" : "running";
    const tool:ToolCallView={id:item.id,title:"subagent",detail:ref.description,status,input:JSON.stringify({agent:ref.agent,description:ref.description}),metadata:{sessionID:ref.id}};
    return <Message><MessageContent><Trace><AiElementsTool tool={tool} streaming={busy} session={session}/></Trace></MessageContent></Message>;
}

function AiElementsShellMessage({item,busy,session}: SessionProps & {item:Extract<TranscriptItem,{kind:"shell"}>}) {
    const succeeded=item.status === "exited" && (!item.exit || item.exit === 0);
    const tool:ToolCallView={id:item.shellID,title:"shell",detail:item.command,inputValue:{command:item.command},status:item.status === "running" ? "running" : succeeded ? "success" : "failed",output:item.output};
    return <Message><MessageContent><Trace><AiElementsTool session={session} streaming={busy} tool={tool}/></Trace></MessageContent></Message>;
}

function AiElementsCompactionMessage({item}: {item:Extract<TranscriptItem,{kind:"compaction"}>}) {
    const status=item.status === "running" ? "running" : item.status === "failed" ? "error" : undefined;
    const label=item.status === "running" ? "Compacting session" : item.status === "failed" ? "Compaction failed" : "Session compacted";
    return <Message><MessageContent className="gap-4"><Trace><TraceLine status={status}><TraceTitle>{label}</TraceTitle></TraceLine></Trace><Markdown isAnimating={item.status === "running"}>{item.summary}</Markdown>{item.error && <p className="text-destructive">{item.error}</p>}</MessageContent></Message>;
}
export const AiElementsTimeline = memo(function AiElementsTimeline(props: AiElementsTimelineProps) {
    const store = useStore();
    const session = props.session === undefined ? store.session : props.session;
    const transcript = useMemo(() => visibleAiTranscript(props.transcript, store.sessions, session?.id), [props.transcript, store.sessions, session?.id]);
    return <div className="flex w-full flex-col gap-4 py-6 text-sm" data-ai-elements="timeline"><AiElementsTranscript {...props} transcript={transcript} session={session}/></div>;
});
export { TodoQueue as AiElementsTodoQueue };
function MessageAction({ label, children, ...props }: React.ComponentProps<typeof Button> & {
    label: string;
}) {
    return <Tooltip><TooltipTrigger render={<Button variant="ghost" size="icon-sm" aria-label={label} {...props}/>}>{children}</TooltipTrigger><TooltipContent><p>{label}</p></TooltipContent></Tooltip>;
}
function MessageCopy({ text }: {
    text: string;
}) {
    const [copied, setCopied] = useState(false);
    const { notify } = useStore();
    useEffect(() => { if (!copied)
        return; const timeout = setTimeout(() => setCopied(false), 2000); return () => clearTimeout(timeout); }, [copied]);
    return <MessageAction label={copied ? "Copied" : "Copy"} onClick={() => { void writeClipboard(text).then(success => { if (success)
        setCopied(true);
    else
        notify("Could not copy the message", "error"); }).catch(error => notify(String(error), "error")); }}>{copied ? <CheckIcon /> : <CopyIcon />}</MessageAction>;
}
function AiElementsSubagent({ tool, childID, transcript, childSession, running, interrupted }: {
    tool: ToolCallView;
    childID: string;
    transcript?: TranscriptItem[];
    childSession?: SessionInfo | null;
    running: boolean;
    interrupted: boolean;
}) {
    const {reopenSession} = useStore();
    const navigation = childID ? <Button size="xs" variant="outline" aria-label="Open agent" onClick={() => void reopenSession(childID)}>Open agent</Button> : null;
    const [snapshot, setSnapshot] = useState<{
        id: string;
        value: SessionTranscript;
    } | null>(null);
    useEffect(() => {
        if (!childID || transcript)
            return;
        let disposed = false;
        let timer: ReturnType<typeof setTimeout> | undefined;
        const read = async () => {
            try {
                const value = await window.vela.sessionTranscript(childID);
                if (!disposed)
                    setSnapshot({ id: childID, value });
            }
            catch { /* A pending child may not be queryable until its session is announced. */ }
            if (!disposed && running)
                timer = setTimeout(() => void read(), 1000);
        };
        void read();
        return () => { disposed = true; if (timer)
            clearTimeout(timer); };
    }, [childID, transcript, running]);
    const items = transcript ?? (snapshot?.id === childID ? snapshot.value.transcript : undefined);
    const request = taskDescriptor(tool);
    const done = tool.status === "success";
    const line = <TraceLine status={done || interrupted ? undefined : running ? "running" : "error"}><TraceIcon>{done ? <CircleCheckIcon /> : running ? <LoaderCircleIcon className="animate-spin"/> : <CircleAlertIcon />}</TraceIcon><TraceTitle>{done ? "Ran" : interrupted ? "Stopped" : running ? "Running" : "Failed to run"} the {request.agent || "delegated"} subagent</TraceTitle></TraceLine>;
    if (!items?.length && !tool.output)
        return <div className="flex items-center gap-2">{line}{navigation}</div>;
    return <TraceCollapsible defaultOpen><TraceCollapsibleTrigger>{line}</TraceCollapsibleTrigger><TraceCollapsibleContent keepMounted className="flex flex-col gap-3">{navigation}{items?.length ? <AiElementsTranscript transcript={items} busy={running} lastAssistantId={items.filter(item => item.kind === "assistant").at(-1)?.id ?? null} session={childSession ?? null}/> : <Markdown>{tool.output}</Markdown>}</TraceCollapsibleContent></TraceCollapsible>;
}

function CitedToolText({value}: {value: NonNullable<ReturnType<typeof structuredCitedText>>}) {
 let start=0;
 return <p className="whitespace-pre-wrap">{value.citations.map((citation,index) => {
  const text=value.text.slice(start,citation.end);
  start=citation.end;
  return <span key={`${citation.end}:${index}`}>{text} <Citation source={citation.source}/></span>;
 })}{value.text.slice(start)}</p>;
}

function AiElementsChainOfThought({tools, streaming, answering, interrupted, session}: {tools: ToolCallView[]; streaming: boolean; answering: boolean; interrupted?: boolean; session: SessionInfo | null}) {
 const [open,setOpen]=useState<boolean>();
 const running=streaming && !answering && !interrupted;
 const failed=tools.filter(tool => tool.status === "failed").length;
 const status=running ? "running" : failed > 0 ? "error" : undefined;
 return <Trace><TraceCollapsible open={open ?? running} onOpenChange={setOpen}><TraceCollapsibleTrigger><TraceLine status={status}><TraceIcon>{status === "running" ? <LoaderCircleIcon className="animate-spin"/> : status === "error" ? <CircleAlertIcon/> : <CircleCheckIcon/>}</TraceIcon><TraceTitle>{running ? "Researching" : `Researched in ${tools.length} steps${failed > 0 ? `, ${failed} failed` : ""}`}</TraceTitle></TraceLine></TraceCollapsibleTrigger><TraceCollapsibleContent keepMounted><Trace>{tools.map(tool => <AiElementsTool key={tool.id} tool={tool} streaming={streaming} interrupted={interrupted} session={session}/>)}</Trace></TraceCollapsibleContent></TraceCollapsible></Trace>;
}

function AiElementsAssistantParts({item, streaming, session}: {item: Extract<TranscriptItem,{kind:"assistant"}>; streaming: boolean; session: SessionInfo | null}) {
 const groups: Array<{kind:"tools";id:string;tools:ToolCallView[]}|{kind:"part";part:Exclude<typeof item.parts[number],{kind:"tool"}>}> = [];
 for (const part of item.parts) {
  if (part.kind !== "tool") groups.push({kind:"part",part});
  else {
   const last=groups.at(-1);
   if (last?.kind === "tools") last.tools.push(part.tool);
   else groups.push({kind:"tools",id:part.id,tools:[part.tool]});
  }
 }
 return <>{groups.map((group,index) => group.kind === "tools" ? group.tools.length > 1 ? <AiElementsChainOfThought key={group.id} tools={group.tools} streaming={streaming} answering={groups.slice(index+1).some(next => next.kind === "part" && next.part.kind === "text" && Boolean(next.part.text))} interrupted={item.interrupted} session={session}/> : <Trace key={group.id}><AiElementsTool tool={group.tools[0]} streaming={streaming} interrupted={item.interrupted} session={session}/></Trace> : group.part.kind === "text" ? group.part.text && <Markdown key={group.part.id} isAnimating={streaming && !item.completed}>{group.part.text}</Markdown> : <Trace key={group.part.id}><ReasoningRow text={group.part.text} running={streaming && !group.part.complete && !item.completed}/></Trace>)}</>;
}
