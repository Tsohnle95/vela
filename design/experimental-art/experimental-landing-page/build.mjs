import { copyFile, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createHash } from 'node:crypto';

const root = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(root, 'concepts');
await mkdir(out, { recursive: true });
const V = '#b7a0ff', W = '#eeebff', D = '#090a17';
const esc = (s) => String(s).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('"', '&quot;');
const rect = (x,y,w,h,fill,extra='') => `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${fill}" ${extra}/>`;
const line = (x,y,x2,y2,color=V,opacity=.4,extra='') => `<line x1="${x}" y1="${y}" x2="${x2}" y2="${y2}" stroke="${color}" opacity="${opacity}" ${extra}/>`;
const circle = (x,y,r,fill='none',stroke=V,extra='') => `<circle cx="${x}" cy="${y}" r="${r}" fill="${fill}" stroke="${stroke}" ${extra}/>`;
const text = (s,x,y,size=16,color=W,extra='') => `<text x="${x}" y="${y}" fill="${color}" font-size="${size}" ${extra}>${esc(s)}</text>`;
const mono = (s,x,y,size=12,color=V,extra='') => text(s,x,y,size,color,`font-family="'SFMono-Regular',Consolas,monospace" letter-spacing="2" ${extra}`);
const title = (s,x,y,size=100,color=W,extra='') => text(s,x,y,size,color,`font-family="Arial,Helvetica,sans-serif" font-weight="700" ${extra.includes('letter-spacing=')?'':'letter-spacing="-5"'} ${extra}`);
const serif = (s,x,y,size=90,color=W,extra='') => text(s,x,y,size,color,`font-family="Georgia,serif" ${extra.includes('letter-spacing=')?'':'letter-spacing="-4"'} ${extra}`);
const pathEl = (d,fill='none',stroke=V,extra='') => `<path d="${d}" fill="${fill}" stroke="${stroke}" ${extra}/>`;
function rng(seed=42) { return () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296); }
function stars(seed=1,n=250,box=[0,0,1440,900],color=V) {
  const r=rng(seed); let s='';
  for(let i=0;i<n;i++){ const z=r()>.97?4:r()>.8?2:1; s+=rect(+(box[0]+r()*box[2]).toFixed(1),+(box[1]+r()*box[3]).toFixed(1),z,z,color,`opacity="${(.12+r()*.66).toFixed(2)}"`); } return s;
}
function star(x,y,size=30,color=W) {
  return `<g transform="translate(${x} ${y})">${pathEl(`M0 ${-size} L${size*.16} ${-size*.16} L${size} 0 L${size*.16} ${size*.16} L0 ${size} L${-size*.16} ${size*.16} L${-size} 0 L${-size*.16} ${-size*.16}Z`,color,'none')}${circle(0,0,size*.6,'url(#halo)','none')}</g>`;
}
function orbit(x,y,rx,ry,angle=0,color=V,extra='') {
  return `<ellipse cx="${x}" cy="${y}" rx="${rx}" ry="${ry}" fill="none" stroke="${color}" transform="rotate(${angle} ${x} ${y})" ${extra}/>`;
}
function planet(x,y,r=90,color=V){
  const ran=rng(x+y); let bits='';
  for(let i=0;i<450;i++){let a=ran()*Math.PI*2,b=Math.sqrt(ran())*r;bits+=rect(x+Math.cos(a)*b,y+Math.sin(a)*b,3+ran()*7,3+ran()*7,ran()>.55?W:'#30215d',`opacity="${ran()*.45}"`);}
  const id=`planet-${x}-${y}-${r}`;
  return `<defs><clipPath id="${id}">${circle(x,y,r,'white','none')}</clipPath></defs><g clip-path="url(#${id})">${circle(x,y,r,color,'none')}${circle(x+r*.5,y-r*.25,r*.95,'#080a18','none', 'opacity=".82"')}${bits}</g>`;
}
const letters={M:['10001','11011','10101','10101','10001','10001','10001'],I:['01110','01110','01110','01110','01110','01110','01110'],R:['11110','11011','11011','11110','11100','11010','11011'],A:['01110','11011','11011','11111','11011','11011','11011']};
function pixelWord(x,y,scale=20,color='url(#type)') {
  let s='';'MIRA'.split('').forEach((c,k)=>letters[c].forEach((row,j)=>[...row].forEach((v,i)=>{if(v==='1')s+=rect(x+3*scale+(k*6+i)*scale,y+j*scale,scale+.2,scale+.2,color);})));return s;
}
function grain(seed=3,n=1500,box=[0,0,1440,900],color=V){return stars(seed,n,box,color);}
function cross(x,y,s=10,color=V){return line(x-s,y,x+s,y,color,.8)+line(x,y-s,x,y+s,color,.8);}
const defs=`<defs>
 <radialGradient id="halo"><stop stop-color="#a283ff" stop-opacity=".75"/><stop offset="1" stop-color="#9171ff" stop-opacity="0"/></radialGradient>
 <radialGradient id="nebula"><stop stop-color="#54328f" stop-opacity=".5"/><stop offset="1" stop-color="#090a17" stop-opacity="0"/></radialGradient>
 <linearGradient id="type" x2=".2" y2="1"><stop stop-color="#a080f5"/><stop offset=".58" stop-color="#b9a5ff"/><stop offset="1" stop-color="#f4f0ff"/></linearGradient>
 <linearGradient id="fade" x2="0" y2="1"><stop stop-color="#9970ff"/><stop offset="1" stop-color="#100e24"/></linearGradient>
 <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse"><path d="M40 0H0V40" fill="none" stroke="#9b83d3" stroke-opacity=".14"/></pattern>
 <pattern id="scan" width="4" height="4" patternUnits="userSpaceOnUse"><path d="M0 0H4" stroke="#05050c" stroke-opacity=".4"/></pattern>
 </defs>`;
function svg(body,bg=D){return `<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="900" viewBox="0 0 1440 900" role="img" aria-label="Mira launch artwork">${defs}${rect(0,0,1440,900,bg)}${body}</svg>`;}
const edge=(label='AN ENVIRONMENT FOR WHAT COMES NEXT',color=V)=>mono('MIRA / DESKTOP',52,55,11,color)+mono(label,1388,55,10,color,'text-anchor="end"')+line(52,77,1388,77,color,.2);
const foot=(s='YOUR NEXT WORKSPACE IS A MOMENT AWAY',color=V)=>mono(s,720,839,11,color,'text-anchor="middle"');
const concepts=[];
function add(name,family,description,body,bg=D){concepts.push({id:String(concepts.length+1).padStart(2,'0'),name,family,description,bg,art:svg(body,bg)});}

// 01–06: emblem, type, landscape, instrument, editorial, pixel environment.
add('First light','Emblem','A luminous orbital emblem suspended above a pixel wordmark.',
  stars(1,650)+circle(720,360,360,'url(#nebula)','none')+circle(720,350,233,'none',V,'opacity=".15" stroke-dasharray="2 8"')+cross(720,350,280)+orbit(720,350,230,100,-25,V,'stroke-width="3"')+orbit(720,350,230,100,25,W,'stroke-width="2"')+star(636,351,68)+planet(913,331,36)+pixelWord(430,570,20)+mono('INTEGRATED DEVELOPMENT ENVIRONMENT',720,760,12,V,'text-anchor="middle"')+foot());
add('Type in space','Typography','Oversized cropped lettering becomes the landscape; a tiny orbit gives it scale.',
  stars(2,140)+mono('A PLACE TO BUILD',70,110,14)+title('MIRA',-30,623,455,W,'font-stretch="condensed"')+orbit(1110,360,210,80,-46,V,'stroke-width="3"')+star(1090,390,44)+circle(1288,233,18,V,'none')+mono('CODE. EXPLORE. REPEAT.',80,745,16)+line(80,790,1360,790,V,.5)+mono('DESKTOP ENVIRONMENT',1360,830,11,V,'text-anchor="end"'));
add('Lunar rise','Landscape','A low lunar horizon and a small rising star create a cinematic opening.',
  stars(3,430)+mono('M I R A',720,150,23,W,'text-anchor="middle"')+circle(720,460,320,'url(#nebula)','none')+star(720,408,35)+orbit(720,408,160,45,-15,V,'opacity=".5"')+circle(720,1680,1100,'#1d1930',V,'stroke-width="2"')+Array.from({length:16},(_,i)=>orbit(720,980+i*40,850+i*15,200,0,V,'opacity=".12"')).join('')+grain(14,1800,[0,600,1440,300])+serif('Room for possibility.',720,777,44,W,'text-anchor="middle" letter-spacing="-1"'));
add('Acquisition','Instrument','A precise circular radar fills the right side while launch information stays on the left.',
  rect(0,0,1440,900,'url(#grid)')+edge('DEEP SPACE / SIGNAL ACQUISITION')+title('Mira',80,350,120)+mono('FIND YOUR NEXT SIGNAL.',87,410,13)+mono('OBSERVATORY  /  001',87,640,12)+line(87,670,370,670)+mono('STANDING BY',87,704,12)+[100,200,300].map(r=>circle(1030,450,r,'none',V,'opacity=".35"')).join('')+line(680,450,1380,450)+line(1030,100,1030,800)+pathEl('M1030 450 L1030 150 A300 300 0 0 1 1290 600Z','url(#nebula)','none')+Array.from({length:60},(_,i)=>{let a=i*Math.PI/30;return line(1030+Math.cos(a)*307,450+Math.sin(a)*307,1030+Math.cos(a)*315,450+Math.sin(a)*315,V,.7)}).join('')+star(1150,345,20)+circle(1150,345,45,'none',W,'stroke-dasharray="3 5"')+mono('01: ORIGIN',1070,730,11));
add('The field guide','Editorial','A cream book-jacket layout pairs elegant type with a tall astronomical plate.',
  rect(810,0,630,900,'#18132d')+stars(5,400,[830,20,590,860])+mono('MIRA',74,80,17,'#453167')+mono('A FIELD GUIDE TO',74,230,12,'#453167')+serif('Making',66,355,118,'#241a39')+serif('what’s',66,480,118,'#241a39')+serif('next.',66,605,118,'#241a39')+line(74,715,710,715,'#453167',.35)+mono('YOUR DEVELOPMENT ENVIRONMENT',74,756,12,'#453167')+orbit(1125,440,220,110,70,V,'stroke-width="2"')+orbit(1125,440,220,110,-25,V)+planet(1125,440,99)+star(1048,275,26)+mono('PLATE 01 / FIRST CONTACT',1125,827,11,V,'text-anchor="middle"'),'#f0eddf');
let hills='';for(let x=0;x<1440;x+=16){const h=100+Math.sin(x/120)*30+Math.cos(x/53)*16;hills+=rect(x,800-h,16,h+100,'#282044')+rect(x,800-h,16,6,'#5d467d');}
add('Night garden','Pixel world','A quiet pixel landscape, a distant moon, and a small illuminated workstation.',
  stars(6,340)+planet(1020,225,92)+pixelWord(150,210,18)+mono('A LITTLE SPACE TO MAKE THINGS.',152,403,12)+hills+rect(860,579,210,130,'#161525')+rect(850,570,230,15,'#6b527f')+rect(890,605,72,44,'#c2a9ff')+rect(899,614,54,26,'#655396')+rect(921,649,9,15,'#9e89c1')+rect(879,664,105,8,'#705e8b')+rect(975,629,35,35,'#342f4d')+star(636,170,15)+mono('MIRA / AFTER HOURS',72,842,11));

// 07–12: terminal, architecture, schematic, negative space, collectible, object.
add('Boot poetry','Terminal','An amber monochrome terminal treats startup as a short, carefully typeset poem.',
  rect(35,35,1370,830,'none','stroke="#bda36b" opacity=".35"')+mono('MIRA PERSONAL COMPUTER',85,100,18,'#dac08a')+mono('────────────────────────────────────────────────────────────',85,140,18,'#7e704d')+pixelWord(85,235,22,'#dac08a')+['> a space for thought','> a place for code','> a new beginning_'].map((s,i)=>mono(s,90,525+i*58,22,'#dac08a')).join('')+mono('DESKTOP ENVIRONMENT',85,794,12,'#b29b67')+mono('✦',1320,790,36,'#dac08a')+rect(0,0,1440,900,'url(#scan)'),'#12120f');
add('Observatory','Architecture','A vaulted window frames a dense star field in a quiet plum interior.',
  `<path d="M440 760V350 A280 280 0 0 1 1000 350V760Z" fill="#080a17" stroke="#766081" stroke-width="2"/>`+stars(8,400,[460,220,520,500])+planet(740,430,85)+star(637,315,26)+line(720,80,720,760,'#8a748f',.4)+line(440,480,1000,480,'#8a748f',.4)+rect(420,760,600,16,'#806486')+rect(400,776,640,15,'#4f3e5c')+serif('Mira',720,160,80,'#e1cde7','text-anchor="middle"')+mono('LET THE OUTSIDE IN.',720,848,12,'#c3a9ca','text-anchor="middle"'),'#241c2d');
add('Orbital mechanics','Schematic','An engineering drawing with offset ellipses, measured axes, and restrained labels.',
  rect(0,0,1440,900,'url(#grid)')+edge('FIGURE 01 / ORBITAL MECHANICS')+orbit(720,450,380,160,-23,'#a99acb','stroke-width="2"')+orbit(720,450,380,160,23,'#a99acb')+orbit(720,450,200,320,0,'#a99acb','stroke-dasharray="4 8"')+line(230,450,1210,450)+line(720,100,720,800)+star(720,450,44)+planet(1052,288,35)+[[-390,-230],[390,-230],[-390,230],[390,230]].map(([x,y])=>cross(720+x,450+y,10)).join('')+mono('A — CENTER OF POSSIBILITY',100,760,11)+mono('R = ∞',1110,720,14)+title('MIRA',80,200,60)+mono('EVERYTHING CONNECTS.',1360,830,11,V,'text-anchor="end"'));
add('Totality','Atmosphere','A monumental eclipsed disc; the wordmark is a tiny whisper beneath it.',
  circle(720,405,360,'url(#nebula)','none')+circle(720,405,228,'#bba1ff','none')+circle(723,399,227,'#070710','none')+circle(720,405,229,'none','#dccfff','stroke-width="1" opacity=".6"')+star(884,563,17)+stars(10,90)+mono('M I R A',720,746,22,W,'text-anchor="middle"')+mono('MAKE SPACE.',720,790,10,V,'text-anchor="middle"'),'#070710');
add('Special delivery','Collectible','A lilac postage stamp on paper, with a circular cancellation and tiny orbital engraving.',
  rect(490,140,460,590,'#b4a0d3')+Array.from({length:24},(_,i)=>circle(490,149+i*25,7,'#eee9dd','none')+circle(950,149+i*25,7,'#eee9dd','none')).join('')+Array.from({length:19},(_,i)=>circle(494+i*25,140,7,'#eee9dd','none')+circle(494+i*25,730,7,'#eee9dd','none')).join('')+rect(520,169,400,531,'none','stroke="#32213f"')+mono('MIRA',546,212,21,'#32213f')+mono('∞',880,213,27,'#32213f')+circle(720,425,130,'none','#4b365f','stroke-width="2"')+orbit(720,425,160,68,-30,'#4b365f','stroke-width="3"')+star(720,425,55,'#32213f')+mono('SPACE FOR IDEAS',720,641,15,'#32213f','text-anchor="middle"')+circle(968,665,106,'none','#50475a','stroke-width="2" opacity=".6"')+circle(968,665,94,'none','#50475a','opacity=".6"')+mono('DESKTOP',968,660,13,'#50475a','text-anchor="middle"')+mono('EXPRESS',968,688,13,'#50475a','text-anchor="middle"')+mono('SOMETHING GOOD IS ON ITS WAY.',80,830,12,'#5c5068'),'#eee9dd');
add('Long play','Object','A huge grooved vinyl disc sits half off-screen beside a modest record-sleeve title.',
  rect(0,0,510,900,'#c4b4dc')+title('Mira',65,205,96,'#251b38')+mono('SIDE A',70,270,13,'#493955')+mono('THE BEGINNING',70,302,13,'#493955')+line(70,685,440,685,'#493955',.4)+serif('Play something new.',70,749,25,'#493955','letter-spacing="-1"')+circle(1090,450,535,'#11101a','none')+Array.from({length:45},(_,i)=>circle(1090,450,150+i*8,'none','#7e6a99',`opacity="${i%4===0?.25:.1}"`)).join('')+circle(1090,450,143,'#b2a0cb','none')+orbit(1090,450,110,44,-25,'#241b34','stroke-width="3"')+star(1090,450,34,'#241b34')+circle(1090,450,7,'#11101a','none'),'#211a2e');

// 13–18: hardware, constellation, panels, graphic system, speed, cartography.
add('Memory card','Hardware','A chunky pixel cartridge floating on a powder-lilac background.',
  rect(535,175,390,520,'#302841')+rect(552,155,356,40,'#302841')+rect(570,210,320,330,'#181624')+rect(590,230,280,290,'#9880bb')+stars(13,160,[590,230,280,290],'#34284b')+orbit(730,360,100,45,-30,'#302841','stroke-width="5"')+star(730,360,39,'#eee8fb')+mono('MIRA',730,482,27,'#302841','text-anchor="middle"')+rect(570,568,320,70,'#443952')+Array.from({length:9},(_,i)=>rect(589+i*33,593,17,75,'#c8b28f')).join('')+title('Insert possibility.',720,794,48,'#3e2d52','text-anchor="middle" letter-spacing="-2"')+mono('YOUR ENVIRONMENT IS COMING TO LIFE',720,841,10,'#5f4c74','text-anchor="middle"'),'#cfbfdf');
const nodes=[[210,350],[360,215],[526,300],[655,185],[805,338],[990,225],[1190,360],[1080,570],[856,635],[641,530],[434,660],[285,535]];
add('Connected dots','Constellation','A wide constellation connects isolated points into one open-ended system.',
  stars(14,350)+nodes.map(([x,y],i)=>line(x,y,...nodes[(i+1)%nodes.length],V,.35)).join('')+line(360,215,641,530)+line(526,300,856,635)+line(805,338,1080,570)+nodes.map(([x,y],i)=>star(x,y,i%3===0?18:8)).join('')+serif('Mira',720,475,105,W,'text-anchor="middle"')+mono('GOOD IDEAS FIND EACH OTHER.',720,789,13,V,'text-anchor="middle"')+edge('A CONSTELLATION OF POSSIBILITIES'));
add('Three bodies','Panels','Three tall art panels turn a star, an orbit, and a moon into a graphic sequence.',
  rect(70,150,410,550,'#221938')+rect(515,150,410,550,'#a995c7')+rect(960,150,410,550,'#171a31')+stars(15,250,[70,150,410,550])+star(275,425,110)+orbit(720,425,170,85,70,'#332342','stroke-width="3"')+orbit(720,425,170,85,-25,'#332342','stroke-width="3"')+star(720,425,30,'#332342')+planet(1165,425,145)+mono('01 / LIGHT',90,680,10)+mono('02 / MOTION',535,680,10,'#332342')+mono('03 / POSSIBILITY',980,680,10)+title('MIRA',70,104,62)+mono('A NEW PERSPECTIVE ON BUILDING.',1368,805,13,V,'text-anchor="end"'));
add('Soft geometry','Graphic','A bright Swiss composition with a huge vertical rule and flat orbital geometry.',
  line(80,105,1360,105,'#3f2e53',.5)+title('Mira.',72,307,190,'#3a254d')+rect(80,365,19,410,'#3a254d')+mono('A SPACE',132,420,17,'#3a254d')+mono('TO THINK.',132,450,17,'#3a254d')+mono('A PLACE',132,520,17,'#3a254d')+mono('TO BUILD.',132,550,17,'#3a254d')+circle(1060,470,240,'#40295b','none')+orbit(1060,470,345,105,-35,'#f0e6ff','stroke-width="10"')+star(1060,470,104,'#cbb2e9')+mono('DESKTOP / 001',80,832,12,'#3a254d'),'#cbb6e1');
let tail='';for(let i=0;i<110;i++){const r=rng(i+15);tail+=rect(260+i*8,730-i*4+r()*90,3+r()*9,3+r()*9,i>70?W:V,`opacity="${.08+i/160}"`);}
add('Departure','Motion','A diagonal pixel comet cuts through a restrained, left-aligned title.',
  stars(17,240)+title('See you',88,226,96)+title('out there.',88,323,96)+mono('M I R A',95,390,16)+tail+pathEl('M280 800Q720 490 1150 300','none',V,'stroke-width="2" opacity=".5"')+star(1150,300,54)+foot('A NEW TRAJECTORY.'));
add('Uncharted','Cartography','An unfolded celestial map with coordinates, contour lines, and a single marked destination.',
  rect(100,110,1240,660,'#e1d8c7')+Array.from({length:7},(_,i)=>line(100+i*206.6,110,100+i*206.6,770,'#695f72',.22)).join('')+Array.from({length:5},(_,i)=>line(100,110+i*165,1340,110+i*165,'#695f72',.22)).join('')+Array.from({length:13},(_,i)=>orbit(900,450,100+i*30,60+i*22,-30,'#776485','opacity=".35"')).join('')+stars(18,120,[130,140,1180,590],'#5d4a70')+star(900,450,30,'#614378')+cross(900,450,65,'#614378')+mono('YOU ARE HERE',965,433,11,'#614378')+serif('Mira',160,279,87,'#4f3d60')+mono('A MAP FOR THE UNMADE.',166,324,12,'#614378')+mono('N 00° 00′ / E ∞',160,710,12,'#614378')+mono('THERE IS MORE TO DISCOVER.',720,840,11,V,'text-anchor="middle"'),'#30283d');

// 19–24: depth, sculpture, sound, exploded geometry, halftone, woven motion.
let tunnel='';for(let i=0;i<17;i++){const s=Math.pow(i/16,2);tunnel+=rect(720-690*s,430-410*s,1380*s,820*s,'none',`stroke="${V}" opacity="${.1+s*.5}"`);}[[30,20],[1410,20],[1410,840],[30,840]].forEach(([x,y])=>{tunnel+=line(720,430,x,y,V,.45);});
add('Event horizon','Perspective','A wireframe rectangular tunnel pulls the eye toward a single distant light.',
  tunnel+star(720,430,18)+mono('M I R A',720,110,22,W,'text-anchor="middle"')+mono('GO A LITTLE FURTHER.',720,803,12,V,'text-anchor="middle"'),'#100d20');
add('The monolith','Sculpture','A tall slab of violet light stands on a silent plane; the title stays low and small.',
  stars(20,180,[0,0,1440,650])+rect(0,650,1440,250,'#15121e')+pathEl('M630 210L810 155L810 650L630 710Z','url(#fade)','none')+pathEl('M810 155L854 188L854 625L810 650Z','#3a285d','none')+pathEl('M630 710L810 650L1130 780L790 830Z','#21192e','none')+line(630,210,810,155,W,.7)+star(810,155,17)+serif('Mira',100,786,70)+mono('BUILD SOMETHING THAT LASTS.',1340,826,11,V,'text-anchor="end"'));
let bars='';for(let i=0;i<79;i++){const h=40+Math.pow(Math.sin(i*.13),2)*180+Math.sin(i*.7)*28;bars+=rect(90+i*16,480-h,7,h*2,i%5===0?W:V,`opacity="${.25+Math.sin(i*.07)**2*.6}"`);}
add('Resonance','Signal','A full-width violet waveform with a tiny centered title and generous silence.',
  mono('M I R A',720,182,22,W,'text-anchor="middle"')+bars+line(80,480,1360,480,V,.25)+mono('TUNE INTO YOUR NEXT IDEA.',720,800,12,V,'text-anchor="middle"')+mono('SIGNAL / 001',80,844,10)+mono('∞ Hz',1360,844,10,V,'text-anchor="end"'),'#0e0c19');
const tile=(x,y,w,h,c)=>pathEl(`M${x} ${y} l${w} ${-w*.45} l${h} ${h*.45} l${-w} ${w*.45}Z`,c,V,'stroke-width="1"');
add('Layer by layer','Spatial','Exploded translucent isometric planes surround a central star.',
  edge('THE SPACE BETWEEN IDEA AND REALITY')+title('Mira',90,290,100)+mono('MAKE IT REAL.',95,343,13)+tile(700,620,320,280,'#1c1532')+tile(700,480,320,280,'#382557')+tile(700,340,320,280,'#674894')+tile(700,200,320,280,'#a080cc')+line(700,200,700,620,V,.35,'stroke-dasharray="4 7"')+line(1020,56,1020,476,V,.35,'stroke-dasharray="4 7"')+star(980,370,55)+mono('01 / AN OPEN FOUNDATION',95,790,11));
let dots='';for(let y=-180;y<=180;y+=10)for(let x=-180;x<=180;x+=10){if(x*x+y*y<180*180){let r=1.2+((x+180)/360)*3.5;dots+=circle(1040+x,450+y,r,'#352840','none');}}
add('Moon bulletin','Print','A bold newspaper masthead and a large halftone moon on warm lavender paper.',
  mono('THE DESKTOP EDITION',60,66,13,'#352840')+mono('NO. 01',1380,66,13,'#352840','text-anchor="end"')+line(60,95,1380,95,'#352840',.8)+title('MIRA',52,310,251,'#352840')+line(60,345,1380,345,'#352840',.8)+serif('A bright idea',60,501,64,'#352840')+serif('starts here.',60,573,64,'#352840')+dots+orbit(1040,450,255,70,-32,'#352840','stroke-width="2"')+line(60,756,1380,756,'#352840',.8)+mono('AN ENVIRONMENT FOR MAKING THINGS.',60,806,13,'#352840'),'#d5c7dd');
let knot='';for(let i=0;i<24;i++){knot+=orbit(720,420,230,84,i*7.5,i%3===0?W:V,'opacity=".5" stroke-width="1.3"');}
add('Continuum','Generative','A woven torus made of intersecting orbital ribbons becomes an intricate central object.',
  stars(24,160)+knot+circle(720,420,340,'url(#nebula)','none')+serif('Mira',720,770,66,W,'text-anchor="middle"')+mono('FOLLOW THE THREAD.',720,818,11,V,'text-anchor="middle"'));

// 25–30: kinetic letters, contours, industrial scene, code, restraint, atlas.
let radial='';'M I R A · M I R A · '.split('').forEach((c,i,a)=>{let t=i/a.length*Math.PI*2;radial+=title(c,720+Math.sin(t)*245,460-Math.cos(t)*245,70,W,`text-anchor="middle" transform="rotate(${t*180/Math.PI} ${720+Math.sin(t)*245} ${460-Math.cos(t)*245})"`);});
add('Around the idea','Letterform','The wordmark becomes a circular typographic orbit around a small violet planet.',
  radial+planet(720,450,100)+orbit(720,450,370,370,0,V,'stroke-dasharray="2 9" opacity=".4"')+star(350,450,20)+mono('EVERY IDEA HAS ITS OWN GRAVITY.',720,845,11,V,'text-anchor="middle"'),'#121020');
let contours='';for(let i=0;i<30;i++){const r=120+i*9;let pts=[];for(let j=0;j<=120;j++){let a=j/120*Math.PI*2;let rad=r+Math.sin(a*5+i*.2)*18+Math.cos(a*3+i*.13)*30;pts.push(`${900+Math.cos(a)*rad},${450+Math.sin(a)*rad*.87}`);}contours+=pathEl('M'+pts.join('L')+'Z','none',V,`opacity="${.15+i/60}"`);}
add('Unknown terrain','Topography','Fluid contour lines form a terrain of possibility beside spare typography.',
  contours+rect(70,340,400,230,D)+serif('Mira',70,440,107)+mono('FIND YOUR OWN WAY.',76,503,13)+cross(900,450,30)+mono('00° / ORIGIN',944,430,11)+edge('TERRAIN STUDY / 001')+mono('BEYOND THE FAMILIAR.',80,827,11));
let gantry='';for(let i=0;i<7;i++){gantry+=line(950,180+i*80,1170,260+i*80,'#77658f',.65)+line(1170,180+i*80,950,260+i*80,'#77658f',.65);}
add('Launch structure','Industrial','A pixel launch tower holds a star-shaped payload above a violet exhaust field.',
  stars(27,350)+rect(950,150,18,575,'#665379')+rect(1152,150,18,575,'#665379')+gantry+rect(850,320,310,14,'#8e76a6')+rect(790,424,360,14,'#8e76a6')+star(804,323,68)+rect(790,390,28,240,'url(#fade)')+rect(0,725,1440,175,'#191625')+grain(27,1200,[560,565,400,160])+pixelWord(90,250,19)+mono('THE NEXT THING STARTS HERE.',90,465,12)+mono('MIRA / LAUNCH FACILITY',90,830,11));
let rain='';const rr=rng(28);for(let i=0;i<38;i++){const x=20+i*38,y=rr()*-400;for(let j=0;j<30;j++){rain+=mono(rr()>.5?'1':'0',x,y+j*30,13,j>26?W:V,`opacity="${.08+(j/30)*.5}"`);}}
add('Source of light','Code','Columns of binary dissolve into a calm central clearing and a compact pixel mark.',
  rain+`<ellipse cx="720" cy="450" rx="440" ry="250" fill="${D}"/>`+star(720,365,45)+pixelWord(502.5,465,15)+mono('THOUGHT INTO FORM.',720,650,12,V,'text-anchor="middle"'));
add('One small beginning','Minimal','A single bright pixel, a restrained wordmark, and almost nothing else.',
  rect(712,352,16,16,'#bba5eb')+serif('mira',720,482,62,'#ddd3ec','text-anchor="middle" letter-spacing="-2"')+mono('IT STARTS WITH A LITTLE SPACE.',720,826,10,'#9687a7','text-anchor="middle"'),'#15111c');
let atlas='';for(let y=0;y<3;y++)for(let x=0;x<5;x++){const cx=220+x*250,cy=265+y*205;atlas+=rect(cx-110,cy-83,220,166,'none','stroke="#aa93c9" opacity=".25"')+stars(x+y*5+30,42,[cx-100,cy-73,200,146])+mono(`${String(y*5+x+1).padStart(2,'0')}`,cx-97,cy+67,9);if((x+y)%3===0)atlas+=star(cx,cy,20);else if((x+y)%3===1)atlas+=orbit(cx,cy,62,30,-25,V)+planet(cx+35,cy-24,12);else atlas+=planet(cx,cy,38);}
add('The star atlas','Archive','Fifteen little celestial plates form a structured atlas beneath a quiet editorial heading.',
  serif('Mira',106,113,72)+mono('AN ATLAS OF WHAT COULD BE.',1330,102,12,V,'text-anchor="end"')+line(110,143,1330,143,V,.3)+atlas+mono('A NEW CHAPTER IS ABOUT TO OPEN.',720,853,11,V,'text-anchor="middle"'),'#14111f');

const baselinePath=path.join(root,'expansion/original-art-baseline.json');
if(process.argv.includes('--update-brand-baseline')){
  await writeFile(baselinePath,JSON.stringify(Object.fromEntries(concepts.map(c=>[c.id,createHash('sha256').update(c.art).digest('hex').slice(0,10)])),null,2)+'\n');
}
const baseline=JSON.parse(await readFile(baselinePath,'utf8'));
for(const c of concepts){
  const actual=createHash('sha256').update(c.art).digest('hex').slice(0,10);
  if(actual!==baseline[c.id])throw new Error(`Original artwork ${c.id} changed`);
  c.batch='original';
  c.domain='Original 30';
  c.tone=['05','11','13','16','23'].includes(c.id)?'light':'dark';
}
const reservations=JSON.parse(await readFile(path.join(root,'expansion/registry.json'),'utf8'));
const additions=(await Promise.all(['environments','graphic-objects','systems'].map(async name=>(await import(`./expansion/${name}.mjs`)).default))).flat();
if(additions.length!==100)throw new Error(`Expected 100 additions; found ${additions.length}`);
const seen=new Set();
for(const c of additions){
  const reservation=reservations.find(row=>row.id===c.id);
  if(!reservation||seen.has(c.id)||c.name!==reservation.name)throw new Error(`Unexpected or duplicate concept: ${c.id}/${c.name}`);
  seen.add(c.id);
  for(const field of ['art','bg','family','description','composition','silhouette','typography','material','nearest']){
    if(typeof c[field]!=='string'||!c[field].trim())throw new Error(`Concept ${c.id} missing ${field}`);
  }
  if(!Array.isArray(c.source)||!c.source.length)throw new Error(`Concept ${c.id} missing source references`);
  for(const source of c.source){
    if(path.basename(source)!==source)throw new Error(`Use a source filename in concept ${c.id}`);
    await stat(path.join(root,'..',source));
  }
  c.batch='new';c.domain=reservation.domain;
  concepts.push(c);
}
const compact=(await import('./compact-mira.mjs')).default;
if(compact.length!==12)throw new Error('Expected twelve compact Mira variations');
concepts.push(...compact);
const slick=(await import('./slick-mira.mjs')).default;
if(slick.length!==12)throw new Error('Expected twelve slick Mira variations');
concepts.push(...slick);
concepts.sort((a,b)=>Number(a.id)-Number(b.id));
const count=concepts.length;
const assetVersions=Object.fromEntries(await Promise.all(['preview.css','preview.js','gallery.css','gallery.js'].map(async file=>[file,createHash('sha256').update(await readFile(path.join(root,file))).digest('hex').slice(0,10)])));
const asset=file=>`${file}?v=${assetVersions[file]}`;
const legacySlugs=JSON.parse(await readFile(path.join(root,'expansion/legacy-slugs.json'),'utf8'));
const getSlug=c=>legacySlugs[c.id]??`${c.id}-${c.name.toLowerCase().replaceAll(/[^a-z0-9]+/g,'-').replace(/-$/,'')}`;
for(const [index,c] of concepts.entries()){
  const slug=getSlug(c);
  c.slug=slug;
  c.version=createHash('sha256').update(c.art).digest('hex').slice(0,10);
  await writeFile(path.join(out,`${slug}.svg`),c.art);
  await writeFile(path.join(out,`${slug}.html`),`<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${c.id} — ${esc(c.name)} / Mira</title><link rel="stylesheet" href="../${asset('preview.css')}"></head>
<body style="--scene-bg:${c.bg};--splash-width:${c.splashWidth??360}px;--splash-ratio:${c.splashWidth/c.splashHeight||1}" data-concept="${c.id}" data-presentation="${c.presentation??'full'}">
<nav class="review-bar" aria-label="Concept review"><a href="../index.html">← All ${count}</a><span>${c.id} / ${esc(c.name)}${c.presentation==='compact'?` <small>${c.splashWidth} × ${c.splashHeight} px</small>`:''}</span><div><button id="notes-toggle" aria-expanded="false" aria-controls="design-notes">Design notes</button><button id="hold" aria-pressed="false">Hold artwork</button><button id="replay">Replay 3s launch</button><a href="./${getSlug(concepts[(index+count-1)%count])}.html" id="previous" aria-label="Previous concept">←</a><a href="./${getSlug(concepts[(index+1)%count])}.html" id="next" aria-label="Next concept">→</a></div></nav>
<aside id="design-notes" class="design-notes" hidden><h2>${esc(c.id)} / ${esc(c.name)}</h2><p>${esc(c.description)}</p>${['composition','silhouette','typography','material','nearest'].filter(key=>c[key]).map(key=>`<h3>${key==='nearest'?'Nearest design and differences':key}</h3><p>${esc(c[key])}</p>`).join('')}${c.source?.length===0?'<h3>Design direction</h3><p>Original smooth vector composition for Mira.</p>':'<h3>Inspiration</h3>'}${(c.source??['ChatGPT Image Oct 13, 2026, 03_23_53 PM.png']).map(source=>`<p class="source-name">${esc(source)}</p>`).join('')}<a href="../expansion/reference-sheet.png" target="_blank" rel="noreferrer">Open source contact sheet ↗</a></aside>
<main class="app-window"><div class="splash ${c.presentation==='compact'?'compact':''}" id="splash"><div class="${c.presentation==='compact'?'compact-window':'art-window'}"><img id="artwork" src="./${slug}.svg?v=${c.version}" alt="${esc(c.description)}" fetchpriority="high"><div class="loading" aria-live="polite"><span class="loading-dot"></span><span id="loading-label">Opening Mira</span></div></div></div><section class="workspace" id="workspace" hidden aria-label="Empty Mira workspace"></section></main>
<input id="folder-picker" type="file" webkitdirectory multiple hidden><script src="../${asset('preview.js')}" type="module"></script></body></html>`);
}
const manifest=concepts.map(({art,...c})=>c);
await copyFile(path.join(root,'../ChatGPT Image Oct 13, 2026, 03_23_53 PM.png'),path.join(root,'inspiration.png'));
await writeFile(path.join(root,'concepts.json'),JSON.stringify(manifest,null,2)+'\n');
await writeFile(path.join(root,'index.html'),`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Mira / Compact loading studies</title><link rel="stylesheet" href="${asset('gallery.css')}"></head><body>
<header><a class="brand" href="index.html"><span>✦</span> MIRA <small>EXPERIMENTAL ART / 001—${count}</small></a><div class="header-links"><a class="reference" href="animations/index.html">Motion studies · 25 ↗</a><a class="reference" href="launch-comparison.html">Launch comparison ↗</a><a class="reference" href="process.html">Agent briefs & process ↗</a><a class="reference" href="contact-sheet.html">Art sheet ↗</a><a class="reference" href="expansion/reference-sheet.png" target="_blank" rel="noreferrer">Source artwork ↗</a></div></header>
<main><section class="intro"><div class="eyebrow">THE MOMENT BEFORE THE WORK</div><h1>Mira, before the work.</h1><p>Twelve slick new directions. Smooth materials, clean type, subtle motion.<br>Compact splash windows, then an empty workspace. All earlier concepts are here too.</p><div class="flow"><span><b>01</b> Artwork appears</span><i>→</i><span><b>02</b> Three seconds</span><i>→</i><span><b>03</b> Empty workspace</span></div></section>
<nav class="filters" aria-label="Browse concepts"><div class="group-filters"><button data-group="slick" aria-pressed="true">Slick Mira <span>12</span></button><button data-group="compact" aria-pressed="false">Pixel Mira <span>12</span></button><button data-group="all" aria-pressed="false">All <span>${count}</span></button><button data-group="original" aria-pressed="false">Original 30</button><button data-group="new" aria-pressed="false">New 100</button></div><label class="search"><span class="sr-only">Search concepts by number or name</span><input id="search" type="search" placeholder="Find a number, name, or direction…" autocomplete="off"></label><label class="tone-filter"><span class="sr-only">Color treatment</span><select id="tone"><option value="all">All treatments</option><option value="dark">Dark space</option><option value="light">Light & paper</option></select></label></nav><div class="gallery-meta"><span id="result-count" aria-live="polite">Showing ${count} concepts</span><span>Click a design for its 3-second launch · Design notes explain each direction</span></div>
<div class="gallery">${concepts.map(c=>`<a class="card" data-tone="${c.tone??'dark'}" data-group="${c.batch}" data-search="${esc(`${c.id} ${c.name} ${c.family} ${c.domain}`.toLowerCase())}" href="concepts/${c.slug}.html"><div class="thumbnail ${c.presentation==='compact'?'compact-thumbnail':''}"><img src="concepts/${c.slug}.svg?v=${c.version}" alt="${esc(c.description)}" loading="lazy" width="${c.width??1440}" height="${c.height??900}"><span class="play">▶ Preview launch</span>${c.presentation==='compact'?`<span class="new-tag">${c.splashWidth} × ${c.splashHeight} PX</span>`:c.batch==='new'?'<span class="new-tag">NEW</span>':''}</div><div class="card-title"><span class="number">${c.id}</span><h2>${esc(c.name)}</h2><span class="family">${esc(c.family)}</span><span class="arrow">↗</span></div><p>${esc(c.description)}</p></a>`).join('')}</div><p id="no-results" hidden>No matching concepts. Try another number or name.</p>
<footer><span>MIRA / LANDING EXPLORATIONS</span><span>12 slick studies · 12 pixel studies · 130 earlier concepts</span><a href="#">Back to top ↑</a></footer></main><script src="${asset('gallery.js')}" type="module"></script></body></html>`);
const prompt=await readFile(path.join(root,'expansion/AGENT-PROMPT.md'),'utf8');
await writeFile(path.join(root,'process.html'),`<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Mira / Agent design briefs</title><link rel="stylesheet" href="${asset('gallery.css')}"><body><main class="process"><a href="index.html">← All ${count} designs</a><h1>One hundred.<br>Independently composed.</h1><p class="process-intro">Three agents received non-overlapping subject reservations. The shared instructions prohibit template swaps and require differences in composition, silhouette, framing, typography, material, and narrative. The original 30 remain part of the comparison set.</p><p class="process-intro">Archived instructions for the earlier 100 concepts are preserved below. The app is named Mira; the gallery and artwork now use that name.</p><h2>The archived shared prompt</h2><pre>${esc(prompt)}</pre><h2>Reserved design directions</h2><div class="brief-list">${reservations.map(row=>`<article><span>${row.id} / ${esc(row.domain)}</span><h3>${esc(row.name)}</h3><p>${esc(row.brief)}</p></article>`).join('')}</div><p><a href="expansion/reference-sheet.png">View the source-art contact sheet ↗</a></p></main></body></html>`);
await writeFile(path.join(root,'contact-sheet.html'),`<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Mira / Art contact sheet</title><link rel="stylesheet" href="${asset('gallery.css')}"><body><main class="contact-page"><header><a class="brand" href="index.html">← MIRA / ART SHEET</a><a class="reference" href="index.html">Detailed gallery ↗</a></header><nav class="filters"><div class="group-filters"><button data-group="slick" aria-pressed="true">Slick 12</button><button data-group="compact" aria-pressed="false">Pixel 12</button><button data-group="all" aria-pressed="false">All ${count}</button><button data-group="original" aria-pressed="false">Original 30</button><button data-group="new" aria-pressed="false">New 100</button></div><label class="search"><span class="sr-only">Search concepts</span><input id="search" type="search" placeholder="Number or name…"></label><label class="tone-filter"><span class="sr-only">Color treatment</span><select id="tone"><option value="all">All treatments</option><option value="dark">Dark</option><option value="light">Light</option></select></label></nav><p id="result-count" aria-live="polite"></p><div class="gallery">${concepts.map(c=>`<a class="card" data-tone="${c.tone}" data-group="${c.batch}" data-search="${esc(`${c.id} ${c.name} ${c.family} ${c.domain}`.toLowerCase())}" href="concepts/${c.slug}.html?still=1"><div class="thumbnail ${c.presentation==='compact'?'compact-thumbnail':''}"><img src="concepts/${c.slug}.svg?v=${c.version}" loading="lazy" alt="${esc(c.description)}" width="${c.width??1440}" height="${c.height??900}"></div><div class="card-title"><span class="number">${c.id}</span><h2>${esc(c.name)}</h2></div></a>`).join('')}</div><p id="no-results" hidden>No matching concepts.</p><footer><a href="index.html">Back to gallery</a><a href="#">Back to top ↑</a></footer></main><script src="${asset('gallery.js')}" type="module"></script></body></html>`);
console.log(`Built ${concepts.length} unique artworks and working launch previews.`);
