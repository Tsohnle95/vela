export const DURATION = 3000;
export const concepts = [
  ['celestial courier','Light','A single pearl follows the trail, carrying a narrow reflection across each cube.','Continuous travel; one bead, one reflected highlight.'],
  ['Relay stations','Rhythm','A satellite pauses at three stations. Each arrival wakes a different cube face.','Travel, dwell, transfer: three deliberate handoffs.'],
  ['Counterweights','Physics','Two unequal bodies counter-vela, pulling the cubes into a gentle balancing motion.','Opposed direction, unequal mass, shared equilibrium.'],
  ['Light loom','Surface','A fine luminous thread winds around the mark while silk-like highlights comb the cube faces.','A growing thread and three parallel surface sweeps.'],
  ['Slingshot','Physics','A fast probe dives behind the top cube and accelerates into a long violet wake.','A slow approach, sharp acceleration and long coast.'],
  ['Prism relay','Light','An amber beam meets a moving prism and separates into three colors across the cubes.','One incoming ray becomes three refracted rays.'],
  ['Magnetic capture','Physics','A metallic bead is caught in a field around the upper cube, circles it, then escapes.','Free vela changes into a local captured vela.'],
  ['Clockwork','Rhythm','A tiny hand advances through twelve celestial ticks as the cube edges count three beats.','Discrete mechanical steps rather than continuous glide.'],
  ['Ripple wake','Surface','A luminous drop sends elliptical ripples outward; soft caustics travel over the cubes.','Expanding wave fronts follow a single moving source.'],
  ['Solar sail','Objects','A small triangular sail banks along the trail, unfolding into a luminous foil.','A changing silhouette with a tangible banking motion.'],
  ['Meteor train','Particles','Seven meteors enter as a tight train, scatter around the far curve and vanish behind the cubes.','A staggered burst, with separation increasing over time.'],
  ['Cube gyroscope','Geometry','The three cubes gently precess around a shared center while a small gyroscope follows the vela.','The logo itself becomes the moving mechanism.'],
  ['Facet scanner','Surface','A celestial scanner stops over each cube, briefly revealing fine isometric lines on its faces.','A bracketed inspection pass with face-by-face scanning.'],
  ['Little eclipse','Light','A dark moon rolls along the trail. Its passing shadow dims the cubes before the light returns.','Light is removed and restored, rather than added.'],
  ['Firefly gathering','Particles','Loose fireflies gather onto the vela, travel together, then peel away into the dark.','Dispersed movement resolves into, then leaves, a shared path.'],
  ['Mercury loop','Objects','A silver droplet stretches, splits into two and joins again on the return arc.','A liquid body changes topology along the journey.'],
  ['Gentle assembly','Geometry','The cubes drift into place while a small light draws the final celestial sweep around them.','A three-second build from separated parts to a settled logo.'],
  ['Opening facets','Geometry','The top face of each cube lifts like a lid as a small light passes, then closes softly.','The cube surfaces articulate independently.'],
  ['Signal packets','Rhythm','Small square packets advance in short hops and send angular signals between the cubes.','Digital packets, stop-motion hops and straight-line responses.'],
  ['Aurora tether','Surface','A gauzy violet ribbon follows the vela, casting a broad, slowly shifting sheen across the mark.','A soft ribbon deforms continuously behind its leading light.'],
  ['Pendulum','Physics','A single weighted light swings back and forth along the visible arc; the upper cube rocks against it.','A reciprocal swing with a pause at each end.'],
  ['Stained light','Objects','A turning crystal travels the trail and projects rotating panes of colored light onto the cubes.','Faceted rotation and sharp polygonal projections.'],
  ['Gravity well','Physics','A small dark lens bends nearby light and draws each cube toward it as it passes.','Moving local distortion, light bending and attraction.'],
  ['Edge conductor','Light','A small sphere carries a charge around the trail, then passes it along the cube edges.','Travel transfers into a continuous edge-tracing sequence.'],
  ['Mirrored moons','Surface','A pearl travels around the mark while each cube reflects a smaller, reversed image of its journey.','The moving object is reflected inside three independent glass-like surfaces.'],
].map(([name,family,description,rule],i)=>({id:String(i+1).padStart(2,'0'),name,family,description,rule}));

const NS='http://www.w3.org/2000/svg';
const TAU=Math.PI*2;
const clamp=(v,a=0,b=1)=>Math.max(a,Math.min(b,v));
const mix=(a,b,t)=>a+(b-a)*t;
const ease=t=>{t=clamp(t);return t*t*(3-2*t);};
const pulse=(p,at,width=.16)=>Math.max(0,1-Math.abs(p-at)/width);
const wrap=t=>((t%1)+1)%1;
const tops=['M361 246L504 172L647 246L504 328Z','M213 519L355 442L497 519L355 598Z','M526 519L669 442L812 519L669 598Z'];
const faces=[tops[0]+'M361 246L504 328L647 246V401L504 484L361 401Z',tops[1]+'M213 519L355 598L497 519V679L355 760L213 679Z',tops[2]+'M526 519L669 598L812 519V679L669 760L526 679Z'];
const centers=[[504,328],[355,600],[669,600]];
const velaD='M155 725A440 115 -30 1 1 917 285A440 115 -30 1 1 155 725Z';
const topVelaD='M360 332C360 250 647 250 647 332C647 414 360 414 360 332Z';
let serial=0;
function el(tag,attrs={}){const n=document.createElementNS(NS,tag);for(const [key,v]of Object.entries(attrs))n.setAttribute(key,String(v));return n;}
function add(parent,tag,attrs){const n=el(tag,attrs);parent.append(n);return n;}
function set(node,attrs){for(const [key,value]of Object.entries(attrs))node.setAttribute(key,String(value));}
function path(parent,d,stroke='#d8caff',width=4,extra={}){return add(parent,'path',{d,fill:'none',stroke,'stroke-width':width,'stroke-linecap':'round','stroke-linejoin':'round',...extra});}
function circle(parent,r,fill='#f6efff',extra={}){return add(parent,'circle',{r,fill,...extra});}
function line(parent,stroke='#dfd3ff',width=3){return add(parent,'line',{stroke,'stroke-width':width,'stroke-linecap':'round'});}

export function createStage(host,id,{icon='../vela-app-icon.png'}={}){
  const prefix=`m${++serial}`,index=Number(id)-1,info=concepts[index];
  const svg=add(host,'svg',{viewBox:'0 0 380 320',role:'img','aria-label':`${id}. ${info.name}: ${info.description}`});
  const defs=add(svg,'defs');
  const glow=add(defs,'filter',{id:`${prefix}-glow`,x:'-100%',y:'-100%',width:'300%',height:'300%'});
  add(glow,'feGaussianBlur',{stdDeviation:8});
  const gloss=add(defs,'linearGradient',{id:`${prefix}-gloss`,x1:0,y1:0,x2:1,y2:.4});
  for(const [offset,opacity]of [[0,0],[.38,0],[.49,.8],[.55,.08],[.7,0],[1,0]])add(gloss,'stop',{offset,'stop-color':'#fff','stop-opacity':opacity});
  const spectrum=add(defs,'linearGradient',{id:`${prefix}-spectrum`,x1:0,y1:0,x2:1,y2:1});
  for(const [offset,color]of [[0,'#7effdf'],[.33,'#c9bcff'],[.65,'#f8c49d'],[1,'#a994ff']])add(spectrum,'stop',{offset,'stop-color':color});
  const logo=add(svg,'svg',{x:106,y:40,width:168,height:168,viewBox:'0 0 1024 1024',overflow:'visible'});
  const back=add(logo,'g');
  const route=path(back,velaD,'transparent',0);const routeLength=route.getTotalLength();
  const localRoute=path(back,topVelaD,'transparent',0),localLength=localRoute.getTotalLength();
  const cubeGroups=[],shine=[],faceEffects=[],edge=[],topGroups=[];
  for(let i=0;i<3;i++){
    const clip=add(defs,'clipPath',{id:`${prefix}-cube${i}`});add(clip,'path',{d:faces[i]});
    const topClip=add(defs,'clipPath',{id:`${prefix}-top${i}`});add(topClip,'path',{d:tops[i]});
    const g=add(logo,'g');cubeGroups.push(g);
    add(g,'image',{href:icon,width:1024,height:1024,'clip-path':`url(#${prefix}-cube${i})`});
    const fx=add(g,'g',{'clip-path':`url(#${prefix}-cube${i})`});faceEffects.push(fx);
    shine.push(add(fx,'rect',{x:-500,y:80,width:700,height:850,fill:`url(#${prefix}-gloss)`,opacity:0}));
    edge.push(path(g,faces[i],'#e6d9ff',4,{opacity:0}));
    const lid=add(g,'g',{opacity:0});topGroups.push(lid);
    add(lid,'image',{href:icon,width:1024,height:1024,'clip-path':`url(#${prefix}-top${i})`});
  }
  const front=add(logo,'g');
  // One visible vector loop shares its geometry with every rotating object.
  path(back,velaD,'#aa90ef',5,{'data-vela-trail':'continuous',opacity:.8});
  const word=add(svg,'text',{x:190,y:267,'text-anchor':'middle',fill:'#f0ecff','font-family':'Arial,Helvetica,sans-serif','font-size':39,'font-weight':500,'letter-spacing':-2.3});word.textContent='Vela';
  const caption=add(svg,'text',{x:190,y:293,'text-anchor':'middle',fill:'#a59abb','font-family':'Arial,Helvetica,sans-serif','font-size':7,'letter-spacing':1.8});caption.textContent='integrated development environment';
  const at=(t,local=false)=>{const r=local?localRoute:route,l=local?localLength:routeLength;return r.getPointAtLength(wrap(t)*l);};
  const point=(node,p)=>set(node,{cx:p.x,cy:p.y});
  const travel=(node,t,local=false)=>point(node,at(t,local));
  const tangent=t=>{const a=at(t),b=at(t+.002);return Math.atan2(b.y-a.y,b.x-a.x)*180/Math.PI;};
  const glowDot=(parent=back,r=14,color='#f5edff')=>{const g=add(parent,'g'),halo=circle(g,r*2,color,{filter:`url(#${prefix}-glow)`,opacity:.5}),core=circle(g,r,color);return {g,halo,core,move(t,local=false){const p=at(t,local);set(g,{transform:`translate(${p.x} ${p.y})`});},place(p){set(g,{transform:`translate(${p.x} ${p.y})`});}};};
  const sweep=(i,p,opacity=.7)=>set(shine[i],{x:-750+1900*p,opacity});
  const edgePulse=(i,v)=>set(edge[i],{opacity:v});
  const attrs=(n,a)=>set(n,a);
  let render;
  if(index===0){
    const dot=glowDot();const tail=path(back,velaD,'#dfd0ff',6,{'stroke-dasharray':`75 ${routeLength-75}`});back.append(dot.g);
    render=p=>{dot.move(p);attrs(tail,{'stroke-dashoffset':75-p*routeLength});for(let i=0;i<3;i++)sweep(i,wrap(p-i*.17),.55);};
  }else if(index===1){
    const dot=glowDot(back,19,'#d7fff1');const rings=centers.map(([x,y])=>circle(back,44,'none',{cx:x,cy:y,stroke:'#b5f1de','stroke-width':3,opacity:0}));
    render=p=>{const step=Math.floor(p*3),phase=p*3-step,t=(step+ease(clamp(phase/.48)))/3;dot.move(t);centers.forEach((_,i)=>{const v=i===[0,2,1][step]?pulse(phase,.7,.35):0;attrs(rings[i],{opacity:v,r:38+v*23});edgePulse(i,v);sweep(i,phase,v*.7);});};
  }else if(index===2){
    const a=glowDot(back,22,'#ccbaff'),b=glowDot(back,10,'#e0fff7');
    render=p=>{a.move(p);b.move(.5-p);centers.forEach(([x,y],i)=>{const wave=Math.sin(TAU*p+i*2);attrs(cubeGroups[i],{transform:`translate(${wave*8} ${Math.cos(TAU*p+i*2)*9})`});sweep(i,wrap(p+i/3),.3);});};
  }else if(index===3){
    const thread=path(back,velaD,'#e8dcff',4,{'stroke-dasharray':routeLength});const dot=glowDot(back,8);
    render=p=>{attrs(thread,{'stroke-dashoffset':routeLength*(1-p),opacity:1-ease((p-.87)/.13)});dot.move(p);for(let i=0;i<3;i++)sweep(i,wrap(p*1.1+i*.13),.8);};
  }else if(index===4){
    const comet=glowDot(back,17);const tails=Array.from({length:16},(_,i)=>circle(back,Math.max(2,13-i*.65),'#a58bff',{opacity:(1-i/16)*.6}));
    render=p=>{const t=wrap(p<.35?.28*(p/.35):.28+.72*ease((p-.35)/.65));comet.move(t);tails.forEach((n,i)=>{point(n,at(t-(i+1)*.009));attrs(n,{opacity:(1-i/16)*pulse(p,.59,.4)*.75});});for(let i=0;i<3;i++)edgePulse(i,pulse(p,.44+i*.09,.09));};
  }else if(index===5){
    const prism=add(back,'path',{d:'M0 -30L25 17L-25 17Z',fill:'#f1dca7'});const incoming=line(back,'#f7d6a5',5);const beams=['#a2ffe7','#c7a7ff','#ffbbaa'].map((c,i)=>line(faceEffects[i],c,38));
    render=p=>{const q=at(p),rot=tangent(p);attrs(prism,{transform:`translate(${q.x} ${q.y}) rotate(${rot})`});attrs(incoming,{x1:q.x-160,y1:q.y-75,x2:q.x,y2:q.y,opacity:.7});beams.forEach((n,i)=>{attrs(n,{x1:q.x,y1:q.y,x2:centers[i][0]+150*Math.sin(p*TAU+i),y2:centers[i][1]+120,opacity:.55});});};
  }else if(index===6){
    const bead=glowDot(back,17,'#e4e8f8');const field=add(front,'ellipse',{cx:504,cy:332,rx:165,ry:88,fill:'none',stroke:'#baa4ff','stroke-width':3,'stroke-dasharray':'6 17'});
    render=p=>{const capture=p>.25&&p<.74;bead.move(capture?(p-.25)/.49:p<.25?p*.8:.2+(p-.74)*3.1,capture);attrs(field,{opacity:capture?.7:0,transform:`rotate(${p*180} 504 332)`});attrs(cubeGroups[0],{transform:`translate(0 ${capture?Math.sin(p*TAU*3)*-10:0})`});edgePulse(0,capture?.45:0);};
  }else if(index===7){
    const dot=glowDot(back,12,'#d8e3ff');const ticks=Array.from({length:12},(_,i)=>{const q=at(i/12);return circle(back,7,'#ab9cc9',{cx:q.x,cy:q.y});});
    render=p=>{const step=Math.floor(p*12);dot.move(step/12);ticks.forEach((n,i)=>attrs(n,{fill:i<=step?'#ece1ff':'#51446c'}));for(let i=0;i<3;i++){edgePulse(i,step%3===i?.7:.08);sweep(i,(step%4)/3,.35);}};
  }else if(index===8){
    const drop=glowDot(back,14,'#acfff0');const ripples=Array.from({length:4},()=>add(back,'ellipse',{fill:'none',stroke:'#b9caff','stroke-width':3}));
    const caustics=faceEffects.map(g=>add(g,'ellipse',{cx:504,cy:400,rx:150,ry:35,fill:'none',stroke:'#e5eeff','stroke-width':18}));
    render=p=>{drop.move(p);ripples.forEach((n,i)=>{const u=wrap(p+i/4),q=at(p-u*.22);attrs(n,{cx:q.x,cy:q.y,rx:20+u*150,ry:10+u*65,opacity:(1-u)*.55,transform:`rotate(-28 ${q.x} ${q.y})`});});caustics.forEach((n,i)=>attrs(n,{cx:centers[i][0],cy:centers[i][1]+Math.sin(p*TAU+i)*90,rx:140+Math.cos(p*TAU+i)*50,opacity:.3}));};
  }else if(index===9){
    const sail=add(back,'g');add(sail,'path',{d:'M-28 -28L49 0L-28 30L-7 0Z',fill:`url(#${prefix}-spectrum)`,stroke:'#f1eaff','stroke-width':2});path(sail,'M-28 -28L-7 0L-28 30M-7 0H49','#eae2ff',2);
    render=p=>{const q=at(p),bank=Math.sin(p*TAU)*.35;attrs(sail,{transform:`translate(${q.x} ${q.y}) rotate(${tangent(p)}) skewX(${bank*30}) scale(${.7+Math.sin(p*Math.PI)*.5} 1)`});for(let i=0;i<3;i++)sweep(i,wrap(p+i*.25),.35);};
  }else if(index===10){
    const meteors=Array.from({length:7},(_,i)=>{const g=add(back,'g');path(g,'M-85 0H0',['#d2b8ff','#b1d9ff','#e4dcff'][i%3],6,{opacity:.5});circle(g,9,'#f3eaff');return g;});
    render=p=>{meteors.forEach((n,i)=>{const u=clamp((p-i*.065)/.57),t=wrap(u*.95+.12),q=at(t);attrs(n,{transform:`translate(${q.x} ${q.y}) rotate(${tangent(t)})`,opacity:Math.sin(Math.PI*u)});});for(let i=0;i<3;i++)edgePulse(i,pulse(p,.25+i*.17,.14)*.7);};
  }else if(index===11){
    const gyro=add(back,'g');add(gyro,'ellipse',{rx:30,ry:12,fill:'none',stroke:'#d5c1ff','stroke-width':4});add(gyro,'ellipse',{rx:12,ry:30,fill:'none',stroke:'#c2fff0','stroke-width':3});circle(gyro,5);
    render=p=>{const a=Math.sin(TAU*p)*6;cubeGroups.forEach((g,i)=>attrs(g,{transform:`rotate(${a*(i===0?1:-.65)} ${centers[i][0]} ${centers[i][1]}) translate(0 ${Math.sin(TAU*p+i)*7})`}));const q=at(p);attrs(gyro,{transform:`translate(${q.x} ${q.y}) rotate(${p*360})`});};
  }else if(index===12){
    const bracket=path(front,'M-45 -25V-45H-25M25 -45H45V-25M45 25V45H25M-25 45H-45V25','#a7f3ff',4);const grids=faceEffects.map(g=>{const group=add(g,'g');for(let i=0;i<9;i++)path(group,`M160 ${150+i*80}L890 ${550+i*80}M${100+i*85} 100V900`,'#b6ebff',2);return group;});
    render=p=>{const step=Math.floor(p*3),u=wrap(p*3),q=at((step+ease(clamp(u/.5)))/3);attrs(bracket,{transform:`translate(${q.x} ${q.y}) rotate(30)`,opacity:.8});grids.forEach((n,i)=>attrs(n,{opacity:i===step?pulse(u,.65,.35)*.55:0}));};
  }else if(index===13){
    const moon=add(back,'g');circle(moon,27,'#090a1b',{stroke:'#e2d6ff','stroke-width':2});circle(moon,42,'none',{stroke:'#9e8cd0','stroke-width':2,opacity:.3});
    const shades=faceEffects.map(g=>add(g,'rect',{x:0,y:0,width:1024,height:1024,fill:'#050616'}));
    render=p=>{const q=at(p);attrs(moon,{transform:`translate(${q.x} ${q.y})`});shades.forEach((n,i)=>{const d=Math.hypot(q.x-centers[i][0],q.y-centers[i][1]);attrs(n,{opacity:clamp(1-d/380)*.8});});};
  }else if(index===14){
    const flies=Array.from({length:9},(_,i)=>glowDot(back,5+i%3*2,['#d7ffdf','#cfc0ff','#efffed'][i%3]));
    render=p=>{const gather=ease(p/.28)*(1-ease((p-.73)/.27));flies.forEach((n,i)=>{const q=at(p+i*.025),a=i*TAU/9+p*4;const loose={x:512+Math.cos(a)*400,y:470+Math.sin(a*1.3)*330};n.place({x:mix(loose.x,q.x,gather),y:mix(loose.y,q.y,gather)});attrs(n.g,{opacity:.55+.45*Math.sin(p*12+i)**2});});for(let i=0;i<3;i++)edgePulse(i,.1+.2*gather);};
  }else if(index===15){
    const blobs=[0,1].map(()=>add(back,'ellipse',{fill:'#e9e7ff',stroke:'#a798d9','stroke-width':2}));
    const liquid=faceEffects.map(g=>path(g,'M160 450Q510 300 900 500','none',0));
    render=p=>{const split=Math.sin(Math.PI*p)**2;blobs.forEach((n,i)=>{const t=p+(i?1:-1)*.035*split,q=at(t);attrs(n,{cx:q.x,cy:q.y,rx:28-7*split,ry:12+split*9,transform:`rotate(${tangent(t)} ${q.x} ${q.y})`});});liquid.forEach((n,i)=>attrs(n,{d:`M120 ${centers[i][1]+50}Q${380+Math.sin(p*TAU)*140} ${centers[i][1]-230} 920 ${centers[i][1]+80}L920 950H100Z`,fill:'#d7d5ff',opacity:.13+.1*Math.sin(p*TAU+i)**2}));};
  }else if(index===16){
    const dot=glowDot(back,12);const arc=path(back,velaD,'#ddd0ff',4,{'stroke-dasharray':routeLength});
    render=p=>{cubeGroups.forEach((g,i)=>{const t=ease((p-i*.1)/.48),dx=[0,-130,130][i],dy=[-190,100,100][i];attrs(g,{transform:`translate(${dx*(1-t)} ${dy*(1-t)})`,opacity:.15+.85*t});});const u=clamp((p-.38)/.5);dot.move(u);attrs(dot.g,{opacity:Math.sin(Math.PI*u)});attrs(arc,{'stroke-dashoffset':routeLength*(1-u),opacity:p<.93?.7:.7*(1-ease((p-.93)/.07))});};
  }else if(index===17){
    const dot=glowDot(back,10,'#d7fff3');const openings=faceEffects.map((g,i)=>add(g,'path',{d:tops[i],fill:'#08091a'}));
    render=p=>{dot.move(p);topGroups.forEach((g,i)=>{const lift=pulse(p,.2+i*.27,.18);attrs(openings[i],{opacity:lift*.85});attrs(g,{opacity:lift>0?1:0,transform:`translate(0 ${-55*lift})`});edgePulse(i,lift*.5);});};
  }else if(index===18){
    const packets=Array.from({length:4},()=>add(back,'rect',{x:-10,y:-10,width:20,height:20,rx:3,fill:'#c6efff'}));const links=centers.map((c,i)=>path(front,`M${c[0]} ${c[1]}V${c[1]-55}H${centers[(i+1)%3][0]}V${centers[(i+1)%3][1]}`,'#c5adff',4));
    render=p=>{const t=Math.floor(p*18)/18;packets.forEach((n,i)=>{const q=at(t-i*.075);attrs(n,{transform:`translate(${q.x} ${q.y}) rotate(45)`});});links.forEach((n,i)=>attrs(n,{opacity:Math.floor(p*9)%3===i?.65:.03,'stroke-dasharray':'12 10','stroke-dashoffset':-p*100}));};
  }else if(index===19){
    const ribbon=path(back,'','url(#'+prefix+'-spectrum)',22,{opacity:.45});const dot=glowDot(back,11);const veils=faceEffects.map(g=>add(g,'rect',{x:0,y:0,width:1024,height:1024,fill:`url(#${prefix}-spectrum)`}));
    render=p=>{dot.move(p);let d='';for(let j=0;j<45;j++){const q=at(p-j*.0035);const offset=Math.sin(j*.25-p*TAU)*j*.75;d+=`${j?'L':'M'}${q.x} ${q.y+offset}`;}attrs(ribbon,{d});veils.forEach((n,i)=>attrs(n,{opacity:.14+.13*Math.sin(p*TAU+i),transform:`rotate(${p*35} 512 512)`}));};
  }else if(index===20){
    const dot=glowDot(back,23,'#ffe9d6');const string=path(back,'','#d1b8ff',2,{opacity:.6});
    render=p=>{const swing=.35+.3*(.5-.5*Math.cos(p*TAU)),q=at(swing);dot.place(q);attrs(string,{d:`M504 170Q${q.x-25} ${q.y-110} ${q.x} ${q.y}`});attrs(cubeGroups[0],{transform:`rotate(${Math.sin(p*TAU)*-5} 504 328)`});sweep(0,(Math.sin(p*TAU)+1)/2,.45);};
  }else if(index===21){
    const crystal=add(back,'g');add(crystal,'path',{d:'M0 -35L25 0L0 35L-25 0Z',fill:'#e6ddff',stroke:'#fff','stroke-width':2});path(crystal,'M0 -35V35M-25 0H25','#a49ac9',2);const panes=faceEffects.map((g,i)=>add(g,'path',{fill:['#8ce8e4','#edb5ff','#ffce9f'][i]}));
    render=p=>{const q=at(p);attrs(crystal,{transform:`translate(${q.x} ${q.y}) rotate(${p*540})`});panes.forEach((n,i)=>{const [x,y]=centers[i],a=p*TAU+i;const points=Array.from({length:3},(_,j)=>`${x+Math.cos(a+j*TAU/3)*240} ${y+Math.sin(a+j*TAU/3)*200}`);attrs(n,{d:`M${points.join('L')}Z`,opacity:.32});});};
  }else if(index===22){
    const lens=add(back,'g');circle(lens,27,'#070714',{stroke:'#f6dbff','stroke-width':3});add(lens,'ellipse',{rx:55,ry:22,fill:'none',stroke:'#d3b2ff','stroke-width':4,transform:'rotate(-30)'});const arcs=Array.from({length:3},()=>path(back,'','#a9b9ff',2));
    render=p=>{const q=at(p);attrs(lens,{transform:`translate(${q.x} ${q.y})`});arcs.forEach((n,i)=>attrs(n,{d:`M${q.x-90-i*15} ${q.y-65}Q${q.x+130} ${q.y} ${q.x-90-i*15} ${q.y+65}`,opacity:.5-i*.12}));cubeGroups.forEach((g,i)=>{const dx=q.x-centers[i][0],dy=q.y-centers[i][1],d=Math.hypot(dx,dy),force=clamp(1-d/440)*.07;attrs(g,{transform:`translate(${dx*force} ${dy*force})`});sweep(i,wrap(p+i/3),.25);});};
  }else if(index===23){
    const dot=glowDot(back,14,'#e6d6ff'),charge=glowDot(front,10,'#fff');const lengths=edge.map(n=>n.getTotalLength());
    render=p=>{dot.move(clamp(p/.28));attrs(dot.g,{opacity:p<.28?1:0});attrs(charge.g,{opacity:p>=.28?1:0});const step=clamp((p-.28)/.72)*3;edge.forEach((n,i)=>{const u=clamp(step-i);attrs(n,{'stroke-dasharray':lengths[i],'stroke-dashoffset':lengths[i]*(1-u),opacity:u>0?.7:0});});const i=Math.min(2,Math.floor(step)),u=wrap(step);charge.place(edge[i].getPointAtLength(u*lengths[i]));};
  }else{
    const dot=glowDot(back,21,'#f6eaff');const reflections=faceEffects.map(g=>{const group=add(g,'g');add(group,'ellipse',{rx:31,ry:14,fill:'#e7d5ff',opacity:.8});add(group,'ellipse',{rx:65,ry:29,fill:'none',stroke:'#d0bcff','stroke-width':3,opacity:.4});return group;});
    render=p=>{dot.move(p);const q=at(p);reflections.forEach((n,i)=>{const [x,y]=centers[i];attrs(n,{transform:`translate(${x-(q.x-512)*.22} ${y+(q.y-512)*.16}) rotate(${i===0?-30:30})`});sweep(i,wrap(1-p+i*.12),.22);});};
  }
  const draw=p=>{render(clamp(p,0,.999999));svg.dataset.progress=String(p);};
  draw(0);
  return {svg,draw,destroy(){svg.remove();}};
}
