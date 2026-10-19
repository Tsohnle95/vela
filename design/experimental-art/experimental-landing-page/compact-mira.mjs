// Compact studies of the supplied pixel star / orbit / MIRA reference.
const violet='#a384f2', pale='#e8e3ff', dark='#080a17';
const R=(x,y,w,h,c,opacity=1)=>`<rect x="${x.toFixed(2)}" y="${y.toFixed(2)}" width="${w}" height="${h}" fill="${c}" opacity="${opacity.toFixed(2)}"/>`;
const P=(d,c,s='none',extra='')=>`<path d="${d}" fill="${c}" stroke="${s}" ${extra}/>`;
const L=(x,y,u,v,c=violet,o=.2)=>`<path d="M${x} ${y}L${u} ${v}" fill="none" stroke="${c}" opacity="${o}" stroke-width=".6"/>`;
const T=(s,x,y,size=8,color=violet,extra='')=>`<text x="${x}" y="${y}" font-family="Menlo,Consolas,monospace" font-size="${size}" fill="${color}" ${extra}>${s}</text>`;
const G=(body,x,y,s=1)=>`<g transform="translate(${x} ${y}) scale(${s})">${body}</g>`;
const random=seed=>()=>((seed=(seed*1664525+1013904223)>>>0)/4294967296);

function sky(w,h,seed,density=1500,cloud=true){
  const r=random(seed);let out=R(0,0,w,h,dark);
  for(let i=0;i<density;i++){
    const s=r()>.98?1.8:.35+r()*.7;
    out+=R(r()*w,r()*h,s,s,r()>.88?pale:'#7155ab',.08+r()*.42);
  }
  if(cloud)for(let i=0;i<1800;i++){
    const x=r()*w,y=h*.11+x*.54+(r()+r()+r()-1.5)*h*.12;
    if(y>h*.33&&x>w*.2&&x<w*.8)continue;
    out+=R(x,y,.3+r(),.3+r(),r()>.65?'#c0a5ff':'#7357ba',.08+r()*.4);
  }
  return out;
}

function glint(x,y,size=14,color=pale){
  return G(P(`M0 ${-size}L2 -2L${size} 0L2 2L0 ${size}L-2 2L${-size} 0L-2 -2Z`,color),x,y);
}

function pixelStar(x,y,size=43,seed=9){
  const r=random(seed);let out='';
  for(let yy=-size;yy<=size;yy+=2)for(let xx=-size;xx<=size;xx+=2){
    const ax=Math.abs(xx),ay=Math.abs(yy),core=ax+ay<14;
    const cross=(ax<3&&ay<size)||(ay<3&&ax<size);
    const envelope=ax*ay<92&&ax+ay<size;
    const aura=ax+ay<size*.9&&r()>.63;
    if(core||cross||envelope||aura){
      out+=R(xx,yy,2,2,core||cross?pale:r()>.3?violet:'#6541b4',core?.98:cross?.85:.12+r()*.65);
    }
  }
  return G(out,x,y);
}

function arc(rx,ry,angle,seed,start=0,end=Math.PI*2,weight=4){
  const r=random(seed),a=angle*Math.PI/180;let out='';
  for(let i=0;i<1100;i++){
    const t=start+(end-start)*r(),spread=(r()+r()-1)*weight;
    const xx=(rx+spread)*Math.cos(t),yy=(ry+spread*.6)*Math.sin(t);
    const x=Math.round((xx*Math.cos(a)-yy*Math.sin(a))/2)*2;
    const y=Math.round((xx*Math.sin(a)+yy*Math.cos(a))/2)*2;
    const c=Math.cos(t+.9)>0?pale:violet;
    out+=R(x,y,r()>.8?3:1.5,r()>.8?3:1.5,c,.22+r()*.68);
  }
  return out;
}

function moon(x,y,rad=19,seed=1){
  const r=random(seed);let out='';
  for(let yy=-rad;yy<=rad;yy+=2)for(let xx=-rad;xx<=rad;xx+=2){
    if(xx*xx+yy*yy>rad*rad)continue;
    const c=xx>rad*.25?'#5840a4':r()>.65?pale:violet;
    out+=R(xx,yy,2,2,c,.65+r()*.35);
  }
  return G(out,x,y);
}

function emblem(x,y,s=1,seed=1,style='classic'){
  let out='';
  if(style==='upright')out+=arc(82,108,-12,seed,0,Math.PI*2,4)+arc(101,52,28,seed+3);
  else if(style==='crescent')out+=arc(118,58,-24,seed,.1,Math.PI*1.8,6)+arc(91,42,24,seed+2,.1,Math.PI*1.7,3);
  else if(style==='wide')out+=arc(130,44,-8,seed,0,Math.PI*2,4)+arc(102,39,18,seed+1,0,Math.PI*2,3);
  else out+=arc(110,55,-17,seed,0,Math.PI*2,5)+arc(91,45,24,seed+1,0,Math.PI*2,4);
  out+=pixelStar(-49,-3,42,seed+5)+moon(80,14,19,seed+7);
  out+=glint(-7,-63,4)+R(28,58,3,3,pale,.8)+R(-94,-39,3,3,violet,.8);
  return G(out,x,y,s);
}

const glyphs={
  M:['10001','11011','10101','10101','10001','10001','10001'],
  I:['01110','01110','01110','01110','01110','01110','01110'],
  R:['11110','11011','11011','11110','11100','11010','11011'],
  A:['01110','11011','11011','11111','11011','11011','11011'],
};
function word(cx,y,scale=12,seed=1,flat=false){
  const width=23*scale,x=cx-width/2,r=random(seed);let mask='',dust='';
  [...'MIRA'].forEach((letter,k)=>glyphs[letter].forEach((row,j)=>[...row].forEach((bit,i)=>{if(bit==='1')mask+=R(x+(k*6+i)*scale,y+j*scale,scale+.1,scale+.1,'white');})));
  for(let i=0;i<1900;i++)dust+=R(x+r()*width,y+r()*7*scale,.4+r()*1.1,.4+r()*1.1,r()>.55?pale:'#6042a3',.1+r()*.5);
  const id=`type-${seed}`;
  return `<defs><clipPath id="${id}">${mask}</clipPath><linearGradient id="${id}-ink" x1="0" y1="0" x2="0" y2="1"><stop stop-color="${flat?pale:'#a27bef'}"/><stop offset=".56" stop-color="#bba3ff"/><stop offset="1" stop-color="#eeeaff"/></linearGradient></defs><g clip-path="url(#${id})">${R(x,y,width,7*scale,`url(#${id}-ink)`)}${dust}</g>`;
}
const subtitle=(cx,y,s='INTEGRATED DEVELOPMENT ENVIRONMENT')=>T(s,cx,y,6.7,'#c0b7db','text-anchor="middle" letter-spacing="1.1"');
const guides=(cx,cy,r=164)=>`<circle cx="${cx}" cy="${cy}" r="${r}" fill="none" stroke="#7d61b7" stroke-width=".5" stroke-dasharray="1 5" opacity=".3"/>`+L(cx-r-15,cy,cx+r+15,cy)+L(cx,cy-r-15,cx,cy+r+15);
const source=['ChatGPT Image Oct 13, 2026, 03_23_53 PM.png'];
const variants=[];
function add(id,name,w,h,splashWidth,splashHeight,body,description){
  variants.push({id:String(id),name,family:'Compact pixel Mira',description,bg:dark,tone:'dark',batch:'compact',domain:'Reference studies',source,width:w,height:h,splashWidth,splashHeight,presentation:'compact',composition:description,silhouette:'Small native-size splash with a pixel star, orbital trails and a MIRA wordmark.',typography:'Reference-inspired chunky pixel MIRA lettering, kept readable at the stated splash size.',material:'Original violet and white pixel geometry with square dither, granular trails and star-field dust.',nearest:'The supplied Mira reference; this is a deliberately related loading-screen variation rather than a new landing-page composition.',art:`<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" role="img" aria-label="Mira compact loading artwork">${body}</svg>`});
}

add(131,'Reference spark',480,480,360,360,
  sky(480,480,131)+guides(240,205,151)+emblem(240,204,1,131)+word(240,315,12,131)+subtitle(240,419)+glint(74,90,7)+glint(401,102,6)+glint(399,391,8),
  'The closest study: a bright pixel star and two granular orbital loops above a large violet-to-white MIRA wordmark.');

add(132,'Quiet core',480,480,320,320,
  sky(480,480,132,330,false)+emblem(240,196,.85,132)+word(240,315,11,132)+subtitle(240,412,'A LITTLE SPACE TO BUILD')+glint(388,94,6)+glint(96,366,5),
  'The same star-and-orbit identity with a quieter sky, more breathing room and a slightly smaller mark.');

add(133,'Violet wake',480,480,360,360,
  sky(480,480,133,2700)+G(arc(188,43,-32,133,0,Math.PI*1.35,11),235,176)+emblem(228,204,.91,133,'crescent')+word(240,328,12,133)+subtitle(240,429)+glint(391,69,10),
  'A broad violet dust wake crosses behind an open orbital arc, with the pixel MIRA wordmark held low.');

add(134,'Moon passage',480,480,350,350,
  sky(480,480,134,1500)+guides(236,205,137)+emblem(225,219,.91,134,'upright')+moon(342,134,34,134)+word(240,338,11.5,134)+subtitle(240,433,'YOUR WORKSPACE IS A MOMENT AWAY')+glint(104,106,8),
  'A larger violet moon passes above a more upright orbit; the bright star stays left of the compact wordmark.');

add(135,'Side by side',600,400,420,280,
  sky(600,400,135,1300)+emblem(155,182,.86,135,'wide')+word(422,155,11,135)+subtitle(423,258,'INTEGRATED DEVELOPMENT')+subtitle(423,274,'ENVIRONMENT')+L(286,96,286,288)+glint(549,63,6),
  'A small horizontal splash pairs the orbital symbol on the left with a pixel MIRA wordmark on the right.');

add(136,'Orbital seal',480,480,340,340,
  sky(480,480,136,700,false)+guides(240,231,174)+`<circle cx="240" cy="231" r="179" fill="none" stroke="#9273d0" stroke-width="1" opacity=".35"/>`+emblem(240,205,.83,136)+word(240,306,10,136)+subtitle(240,397,'MIRA / DESKTOP')+glint(240,52,8),
  'The reference becomes a restrained celestial seal, with a fine circular perimeter around the mark and name.');

add(137,'Twin glimmer',540,450,360,300,
  sky(540,450,137,1400)+G(arc(142,51,-14,137,0,Math.PI*2,5),270,182)+pixelStar(214,175,39,137)+pixelStar(337,174,23,139)+moon(380,203,17,137)+word(270,292,12,137)+subtitle(270,397,'A LITTLE LIGHT. A NEW BEGINNING.'),
  'Two unequal pixel stars share one wide orbital trail above the familiar gradient MIRA lettering.');

add(138,'Midnight coordinates',480,480,360,360,
  sky(480,480,138,700,false)+L(42,223,438,223)+L(240,36,240,432)+guides(240,222,155)+emblem(240,212,.9,138)+word(240,329,11,138,true)+subtitle(240,428)+[[-150,-120],[150,-120],[-150,110],[150,110]].map(([x,y])=>G(P('M-4 0H4M0-4V4','none',violet,'stroke-width=".7"'),240+x,223+y)).join(''),
  'A darker coordinate study with small registration crosses, a precise orbital mark and pale pixel type.');

add(139,'Stardust wordmark',540,450,360,300,
  sky(540,450,139,2000)+emblem(270,153,.66,139)+word(270,259,15,139)+subtitle(270,390)+glint(79,295,7)+glint(460,269,6),
  'The granular MIRA lettering is the hero, with a smaller star-and-orbit symbol floating close above it.');

add(140,'Close approach',420,480,300,343,
  sky(420,480,140,1700)+guides(210,201,144)+emblem(210,201,.89,140,'upright')+word(210,336,11,140)+subtitle(210,433,'INTEGRATED DEVELOPMENT ENVIRONMENT')+glint(58,88,7),
  'A narrow portrait splash holds the upright orbital trail, bright star and MIRA in a closely stacked arrangement.');

add(141,'Silver orbit',570,450,380,300,
  sky(570,450,141,1300)+G(arc(182,58,-22,141,0,Math.PI*1.7,2.4),310,172)+emblem(303,173,.83,141,'crescent')+word(255,293,12,141,true)+subtitle(255,403)+glint(466,95,10)+moon(102,156,13,141),
  'A long broken silver trail reaches toward the top-right; the pale pixel wordmark sits slightly left of center.');

add(142,'A small beginning',480,480,300,300,
  sky(480,480,142,220,false)+emblem(240,208,.64,142)+word(240,313,9.5,142)+subtitle(240,405,'MIRA / YOUR SPACE TO BUILD')+glint(368,125,4),
  'The most compact and quiet study: a small luminous orbital mark, pixel MIRA and a sparse midnight star field.');

export default variants;
