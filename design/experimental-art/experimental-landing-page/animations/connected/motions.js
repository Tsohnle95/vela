import {studies} from './studies.js';
export const DURATION=3000;
export const concepts=studies.map((s,i)=>({...s,id:String(i+1).padStart(2,'0')}));
export function interaction(q,[x,y]){const dx=q.x-x,dy=q.y-y,distance=Math.hypot(dx,dy);const t=Math.max(0,1-distance/280);return {dx,dy,distance,strength:t*t*(3-2*t),angle:Math.atan2(dy,dx),nx:dx/Math.max(distance,1),ny:dy/Math.max(distance,1)};}
const NS='http://www.w3.org/2000/svg';
const wrap=t=>((t%1)+1)%1;
const tops=['M361 246L504 172L647 246L504 328Z','M213 519L355 442L497 519L355 598Z','M526 519L669 442L812 519L669 598Z'];
const faces=[tops[0]+'M361 246L504 328L647 246V401L504 484L361 401Z',tops[1]+'M213 519L355 598L497 519V679L355 760L213 679Z',tops[2]+'M526 519L669 598L812 519V679L669 760L526 679Z'];
const centers=[[504,328],[355,600],[669,600]];
const orbitD='M155 725A440 115 -30 1 1 917 285A440 115 -30 1 1 155 725Z';
let serial=0;
function el(tag,attrs={}){const n=document.createElementNS(NS,tag);for(const [key,v]of Object.entries(attrs))n.setAttribute(key,String(v));return n;}
function add(parent,tag,attrs){const n=el(tag,attrs);parent.append(n);return n;}
function set(node,attrs){for(const [key,value]of Object.entries(attrs))node.setAttribute(key,String(value));}
function path(parent,d,stroke='#d8caff',width=4,extra={}){return add(parent,'path',{d,fill:'none',stroke,'stroke-width':width,'stroke-linecap':'round','stroke-linejoin':'round',...extra});}
function circle(parent,r,fill='#f6efff',extra={}){return add(parent,'circle',{r,fill,...extra});}

export function createStage(host,id,{icon='../../mira-app-icon.png'}={}){
  const prefix=`c${++serial}`,info=concepts.find(c=>c.id===id)||concepts[0];
  const svg=add(host,'svg',{viewBox:'0 0 380 320',role:'img','aria-label':`${id}. ${info.name}: ${info.description}`});
  const defs=add(svg,'defs');
  const glow=add(defs,'filter',{id:`${prefix}-glow`,x:'-100%',y:'-100%',width:'300%',height:'300%'});
  add(glow,'feGaussianBlur',{stdDeviation:8});
  const logo=add(svg,'svg',{x:106,y:40,width:168,height:168,viewBox:'0 0 1024 1024',overflow:'visible'});
  const back=add(logo,'g',{'data-layer':'orbit'});
  const route=path(back,orbitD,'transparent',0);const routeLength=route.getTotalLength();
  const cubeGroups=[],faceEffects=[],edge=[],topGroups=[];
  for(let i=0;i<3;i++){
    const clip=add(defs,'clipPath',{id:`${prefix}-cube${i}`});add(clip,'path',{d:faces[i]});
    const topClip=add(defs,'clipPath',{id:`${prefix}-top${i}`});add(topClip,'path',{d:tops[i]});
    const g=add(logo,'g',{'data-cube':i});cubeGroups.push(g);
    add(g,'image',{href:icon,width:1024,height:1024,'clip-path':`url(#${prefix}-cube${i})`});
    const fx=add(g,'g',{'clip-path':`url(#${prefix}-cube${i})`});faceEffects.push(fx);
    edge.push(path(g,faces[i],'#e6d9ff',4,{opacity:0}));
    const lid=add(g,'g',{opacity:0});topGroups.push(lid);
    add(lid,'image',{href:icon,width:1024,height:1024,'clip-path':`url(#${prefix}-top${i})`});
  }
  // One visible vector loop shares its geometry with every orbiting object.
  path(back,orbitD,'#aa90ef',5,{'data-orbit-trail':'continuous',opacity:.8});
  const word=add(svg,'text',{x:190,y:267,'text-anchor':'middle',fill:'#f0ecff','font-family':'Arial,Helvetica,sans-serif','font-size':39,'font-weight':500,'letter-spacing':-2.3});word.textContent='Mira';
  const caption=add(svg,'text',{x:190,y:293,'text-anchor':'middle',fill:'#a59abb','font-family':'Arial,Helvetica,sans-serif','font-size':7,'letter-spacing':1.8});caption.textContent='integrated development environment';
  const at=t=>route.getPointAtLength(wrap(t)*routeLength);
  const glowDot=()=>{const g=add(back,'g',{'data-orbit-object':'pearl'});circle(g,28,'#f5edff',{filter:`url(#${prefix}-glow)`,opacity:.5});circle(g,14,'#f5edff');return {g,place(p){set(g,{transform:`translate(${p.x} ${p.y})`});}};};
  const dot=glowDot();const tail=path(back,orbitD,'#dfd0ff',6,{'stroke-dasharray':`75 ${routeLength-75}`});back.append(dot.g);
  const apply=info.setup({add,path,circle,set,defs,prefix,faceEffects,cubeGroups,edge,topGroups,centers,tops,faces});
  const draw=p=>{
    const q=at(wrap(p));dot.place(q);set(tail,{'stroke-dashoffset':75-wrap(p)*routeLength});
    centers.forEach(([x,y],i)=>{
      const state=interaction(q,[x,y]);
      cubeGroups[i].dataset.influence=state.strength.toFixed(4);
      apply({...state,i,x,y,q});
    });
    svg.dataset.progress=String(p);
  };
  draw(0);
  return {svg,draw,destroy(){svg.remove();}};
}
