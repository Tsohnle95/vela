import { useState } from "react";
import { aiElementDemos } from "../demos";
import "../ai-elements.css";
import "./reference-theme.css";
export default function Gallery({dark}:{dark:boolean}) {
  const [selected,setSelected] = useState("chat");
  const [replay,setReplay] = useState(0);
  const demo = aiElementDemos.find(value => value.id === selected)!;
  return <section className={`pattern-gallery ${dark ? "dark":""}`}><aside aria-label="AI Elements patterns">{aiElementDemos.map((value,index) => <button key={value.id} aria-current={selected === value.id ? "page":undefined} onClick={() => {setSelected(value.id);setReplay(0);}}><span>{String(index+1).padStart(2,"0")}</span>{value.title}</button>)}</aside><article><header><div><h2>{demo.title}</h2><p>Original upstream example, with its original script and interactions.</p></div><button onClick={() => setReplay(value => value+1)}>Remount example</button><a href={`https://ui.shadcn.com/docs/ai-elements/base/${selected}`} target="_blank" rel="noreferrer">View source ↗</a></header><div className="ai-elements pattern-preview bg-background text-foreground" data-slot="preview"><demo.Component key={`${selected}-${replay}`}/></div></article></section>;
}
