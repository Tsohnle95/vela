/// <reference lib="webworker" />
import { loadHighlighter } from "./upstream/lib/highlighter";
import { createLatestWorkerQueue } from "../opencode-markdown/markdown-worker-queue";
import type { HighlightRequest, HighlightResult } from "./highlight-protocol";

const queue = createLatestWorkerQueue<Extract<HighlightRequest,{type:"highlight"}>>({
 async run(request) {
  let lines: HighlightResult["lines"] = null;
  try {
   const highlighter = await loadHighlighter(request.language);
   lines = highlighter?.highlight(request.code).lines.map(line => line.map(token => ({text:token.props.children,style:token.props.style}))) ?? null;
  } catch { /* Unsupported or failed grammars retain readable plain text. */ }
  self.postMessage({key:request.key,id:request.id,code:request.code,language:request.language,lines} satisfies HighlightResult);
 },
 supersede() {},
 dispose() {},
});
self.onmessage = (event:MessageEvent<HighlightRequest>) => {
 if(event.data.type === "dispose") queue.dispose(event.data.key);
 else queue.highlight(event.data);
};
