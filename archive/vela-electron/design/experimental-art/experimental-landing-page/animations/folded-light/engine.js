const NS='http://www.w3.org/2000/svg';
export const DURATION=3000;
export const clamp=t=>Math.max(0,Math.min(1,t));
export const ease=t=>{t=clamp(t);return t*t*(3-2*t);};
export const segment=(p,a,b)=>clamp((p-a)/(b-a));
export const mix=(a,b,t)=>a+(b-a)*t;
export const shapeD='M114 178L156 79L192 145L228 79L266 178L229 194L192 145L153 194Z';
const wingD=['M114 178L156 79L192 145L153 194Z','M192 145L228 79L266 178L229 194Z','M156 79L165 161L192 145Z','M228 79L220 161L192 145Z'];
let serial=0;
export function createStage(host,study){
 const id='f'+(++serial),add=(parent,tag,attrs={})=>{const n=document.createElementNS(NS,tag);set(n,attrs);parent.append(n);return n;};
 const set=(node,attrs)=>{for(const [k,v]of Object.entries(attrs))node.setAttribute(k,String(v));};
 const svg=add(host,'svg',{viewBox:'0 0 380 320',role:'img','aria-label':`${study.id}. ${study.name}: ${study.description}`,'data-study':study.id});
 const defs=add(svg,'defs');
 const gradient=(name,stops,attrs={})=>{const g=add(defs,'linearGradient',{id:id+'-'+name,x1:0,y1:0,x2:1,y2:1,...attrs});stops.forEach(([offset,color,opacity=1])=>add(g,'stop',{offset,'stop-color':color,'stop-opacity':opacity}));return `url(#${id}-${name})`;};
 const mint=gradient('mint',[[0,'#e7fff7'],[.5,'#71cbb4'],[1,'#248b7d']],{gradientUnits:'userSpaceOnUse',x1:114,y1:79,x2:266,y2:194}),shade=gradient('shade',[[0,'#2e7e77'],[1,'#b4ead7']],{gradientUnits:'userSpaceOnUse',x1:156,y1:79,x2:192,y2:161});
 const blur=add(defs,'filter',{id:id+'-blur',x:'-100%',y:'-100%',width:'300%',height:'300%'});add(blur,'feGaussianBlur',{stdDeviation:14});
 const soft=add(defs,'filter',{id:id+'-soft',x:'-100%',y:'-100%',width:'300%',height:'300%'});add(soft,'feGaussianBlur',{stdDeviation:3});
 add(svg,'rect',{width:380,height:320,fill:'#edf5ee'});
 const under=add(svg,'g',{'data-layer':'under'});
 const shadow=add(svg,'ellipse',{cx:190,cy:209,rx:64,ry:8,fill:'#478978',opacity:.18,filter:`url(#${id}-blur)`});
 const mark=add(svg,'g',{'data-layer':'mark'}),parts=wingD.map((d,i)=>{const g=add(mark,'g',{'data-panel':i});add(g,'path',{d,fill:[mint,mint,shade,'#1c736c'][i]});return g;});
 const path=(parent,d,stroke='#3c8e7c',width=1,extra={})=>add(parent,'path',{d,fill:'none',stroke,'stroke-width':width,'stroke-linecap':'round','stroke-linejoin':'round',...extra});
 const rimD=['M114 178L156 79L192 145','M192 145L228 79'];
 const rims=rimD.map((d,i)=>path(parts[i],d,'#fff',1.3,{opacity:.9}));
 const over=add(svg,'g',{'data-layer':'over'});
 const label=add(svg,'text',{x:190,y:267,'font-family':'Arial,Helvetica,sans-serif','font-size':39,fill:'#254e46','font-weight':500,'letter-spacing':-2.3,'text-anchor':'middle'});label.textContent='Vela';
 const caption=add(svg,'text',{x:190,y:291,'font-family':'Arial,Helvetica,sans-serif','font-size':7,fill:'#6b9387','letter-spacing':1.8,'text-anchor':'middle'});caption.textContent='integrated development environment';
 const clip=add(defs,'clipPath',{id:id+'-clip'});path(clip,shapeD,'none',0,{fill:'#fff'});
 const circle=(parent,r,fill='#fff',extra={})=>add(parent,'circle',{r,fill,...extra});
 const poly=(parent,points,fill=mint,extra={})=>add(parent,'polygon',{points:points.map(p=>p.join(',')).join(' '),fill,...extra});
 const copy=(parent,extra={})=>{const g=add(parent,'g',extra);wingD.forEach((d,i)=>add(g,'path',{d,fill:[mint,mint,shade,'#1c736c'][i]}));return g;};
 const fx=add(over,'g'),backFx=add(under,'g');
 const clipped=()=>add(fx,'g',{'clip-path':`url(#${id}-clip)`});
 const context={svg,defs,id,add,set,path,circle,poly,gradient,copy,clipped,mark,parts,rims,shadow,label,caption,fx,backFx,mint,shade,wingD,shapeD,ease,clamp,segment,mix,soft:`url(#${id}-soft)`,blur:`url(#${id}-blur)`};
 const render=study.setup(context);
 const draw=progress=>{
  const p=clamp(progress),u=ease(segment(p,.04,.74)),e=Math.sin(Math.PI*segment(p,0,.86));
  set(mark,{opacity:1,transform:'',filter:'none'});parts.forEach((n,i)=>{set(n,{opacity:1,transform:''});set(n.firstChild,{d:wingD[i]});});
  rims.forEach((n,i)=>set(n,{d:rimD[i],opacity:.9}));set(shadow,{opacity:.18,rx:64,ry:8,cx:190,cy:209,transform:''});set(label,{opacity:1,transform:''});set(caption,{opacity:1});
  const extraOpacity=1-ease(segment(p,.76,.9));set(fx,{opacity:extraOpacity});set(backFx,{opacity:extraOpacity});
  render(p,{u,e,leave:extraOpacity});
  // All studies end on the exact source mark, so launch timing never crops an unfinished logo.
  if(p>=.9){set(mark,{opacity:1,transform:'',filter:'none'});parts.forEach((n,i)=>{set(n,{opacity:1,transform:''});set(n.firstChild,{d:wingD[i]});});set(label,{opacity:1,transform:''});set(caption,{opacity:1});rims.forEach((n,i)=>set(n,{d:rimD[i],opacity:.9}));set(shadow,{opacity:.18,rx:64,ry:8,cx:190,cy:209,transform:''});}
  svg.dataset.progress=String(p);
 };
 draw(0);return {svg,draw,destroy(){svg.remove();}};
}
