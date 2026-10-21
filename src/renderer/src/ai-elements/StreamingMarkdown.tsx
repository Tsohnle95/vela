import { Markdown } from "./demos/lib/markdown";
import { useIsCodeFenceIncomplete, type ExtraProps } from "streamdown";
import { CodeBlock, CodeBlockHeader, CodeBlockTitle, CodeBlockActions, CodeBlockCopyButton, CodeBlockContent, CodeBlockLine } from "@/components/ui/code-block";
import { cn } from "./class-names";
import { useWorkerCode } from "./use-worker-code";

function StreamingCode({node,className,children,...props}:React.ComponentProps<"code"> & ExtraProps) {
 if(!("data-block" in props)) return <code className={cn("rounded-sm bg-muted px-1 py-0.5 font-mono text-[0.9em]",className)} {...props}>{children}</code>;
 return <WorkerCodeBlock code={typeof children === "string" ? children : ""} language={/language-(\S+)/.exec(className??"")?.[1]??""}/>;
}
function WorkerCodeBlock({code:source,language}:{code:string;language:string}) {
 const code=source.endsWith("\n")?source.slice(0,-1):source;
 const result=useWorkerCode(code,language);
 const incomplete=useIsCodeFenceIncomplete();
 const previous=result?.code.split("\n")??[];
 return <CodeBlock><CodeBlockHeader><CodeBlockTitle>{language||"text"}</CodeBlockTitle><CodeBlockActions><CodeBlockCopyButton value={source} disabled={incomplete}/></CodeBlockActions></CodeBlockHeader><CodeBlockContent wrap lineNumbers>{code.split("\n").map((line,index)=>{
  const prefix=previous[index];
  const tokens=prefix!==undefined && line.startsWith(prefix)?result?.lines?.[index]:null;
  return <CodeBlockLine key={index}>{tokens?<>{tokens.map((token,n)=><span key={n} style={token.style} className="text-(--shiki-light) dark:text-(--shiki-dark)">{token.text}</span>)}{line.slice(prefix!.length)}</>:line}</CodeBlockLine>;
 })}</CodeBlockContent></CodeBlock>;
}
const components={code:StreamingCode};
export function StreamingMarkdown(props: React.ComponentProps<typeof Markdown>) {
 return <Markdown {...props} components={components}/>;
}
