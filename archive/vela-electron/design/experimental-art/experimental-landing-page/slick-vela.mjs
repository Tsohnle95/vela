const designs=[];
const path=(d,fill='none',stroke='none',extra='')=>`<path d="${d}" fill="${fill}" stroke="${stroke}" ${extra}/>`;
const circle=(x,y,r,fill,extra='')=>`<circle cx="${x}" cy="${y}" r="${r}" fill="${fill}" ${extra}/>`;
const text=(s,x,y,size=28,color='#f5f3ff',extra='')=>`<text x="${x}" y="${y}" ${extra.includes('font-family=')?'':'font-family="Arial,Helvetica,sans-serif"'} font-size="${size}" fill="${color}" ${extra}>${s}</text>`;
const word=(x,y,size=45,color='#f5f3ff',extra='')=>text('Vela',x,y,size,color,`${extra.includes('font-weight=')?'':'font-weight="500"'} ${extra.includes('letter-spacing=')?'':'letter-spacing="-2.3"'} ${extra}`);
const label=(s,x,y,color='#85818f',extra='')=>text(s,x,y,7,color,`letter-spacing="1.8" ${extra}`);
const linear=(id,stops,extra='')=>`<linearGradient id="${id}" x1="0" y1="0" x2="1" y2="1" ${extra}>${stops.map(([offset,color])=>`<stop offset="${offset}" stop-color="${color}"/>`).join('')}</linearGradient>`;
const radial=(id,color)=>`<radialGradient id="${id}"><stop stop-color="${color}" stop-opacity=".65"/><stop offset="1" stop-color="${color}" stop-opacity="0"/></radialGradient>`;
const chrome=linear('chrome',[[0,'#51426e'],[.19,'#e4d4ff'],[.35,'#9c90ac'],[.43,'#fff'],[.49,'#3e3a49'],[.62,'#77717e'],[.8,'#e9ddff'],[1,'#6d577b']]);
const blur='<filter id="blur" x="-100%" y="-100%" width="300%" height="300%"><feGaussianBlur stdDeviation="14"/></filter>';
const motion=`<style>.float{animation:float 6s ease-in-out infinite;transform-box:fill-box;transform-origin:center}.bloom{animation:bloom 4s ease-in-out infinite}.draw{stroke-dasharray:650;animation:draw 3s ease-out both}@keyframes float{50%{transform:translateY(-5px) rotate(2deg)}}@keyframes bloom{50%{opacity:.55}}@keyframes draw{from{stroke-dashoffset:650}to{stroke-dashoffset:0}}@media(prefers-reduced-motion:reduce){.float,.bloom,.draw{animation:none}}</style>`;
function add(id,name,w,h,bg,tone,defs,body,description,material){
  designs.push({id:String(id),name,family:'Slick Vela',batch:'slick',domain:'Smooth vector studies',description,bg,tone,width:w,height:h,splashWidth:w,splashHeight:h,presentation:'compact',source:[],composition:description,silhouette:description,typography:'Smooth, optically spaced Vela lettering with restrained supporting text; no bitmap or pixel glyphs.',material,nearest:'The earlier compact pixel collection shares only the Vela name and compact launch behavior. This composition uses continuous curves and smooth surfaces.',art:`<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" role="img" aria-label="Vela ${name} loading artwork"><defs>${defs}</defs>${motion}<rect width="${w}" height="${h}" fill="${bg}"/>${body}</svg>`});
}

add(143,'Liquid signature',380,320,'#101014','dark',chrome+blur+radial('light','#8b6dd1'),
  circle(190,126,150,'url(#light)')+`<ellipse cx="190" cy="197" rx="64" ry="7" fill="#000" filter="url(#blur)"/>`+
  `<g class="float">${path('M124 166V108C124 73 153 72 165 107L190 164L215 107C228 72 256 73 256 108V166','none','url(#chrome)','stroke-width="24" stroke-linecap="round" stroke-linejoin="round"')}${path('M116 156V108C116 82 142 72 154 91','none','#fff','stroke-width="1" opacity=".55"')}</g>`+word(190,254,39,'#e9e4f1','text-anchor="middle"')+label('A SPACE TO MAKE',190,278,'#7f788c','text-anchor="middle"'),
  'A single sculpted liquid-metal M floats over a soft reflection, with a quiet centered Vela signature.','Continuous metallic strokes, sharp silver highlights and a diffused violet spotlight.');

const ribbon='M106 146C102 88 174 70 222 91C277 115 264 176 215 189C170 201 132 171 150 139C166 110 202 113 231 140';
add(144,'Glass loop',380,320,'#111522','dark',linear('glass',[[0,'#bff8ff'],[.4,'#6963df'],[.7,'#d9beff'],[1,'#3d43ae']])+radial('aura','#3d55be')+blur,
  circle(190,144,149,'url(#aura)')+`<g class="float">${path(ribbon,'none','#647bff','stroke-width="28" opacity=".22"')}${path(ribbon,'none','url(#glass)','stroke-width="20" stroke-linecap="round" opacity=".82"')}${path(ribbon,'none','#effdff','stroke-width=".7" opacity=".8" transform="translate(-4 -7)"')}</g>`+word(190,257,38,'#edf4ff','text-anchor="middle"')+label('OPEN A NEW POSSIBILITY',190,281,'#7c89ae','text-anchor="middle"'),
  'A translucent ribbon folds through itself above a small, precise Vela wordmark.','Smooth translucent glass tubing with cyan edges, violet refraction and restrained glow.');

let iris='';for(let i=0;i<36;i++){const a=i*10;iris+=`<ellipse cx="190" cy="130" rx="61" ry="17" fill="none" stroke="url(#spectrum)" stroke-width="1.2" opacity=".7" transform="rotate(${a} 190 130)"/>`;}
add(145,'Spectral aperture',380,320,'#080b10','dark',linear('spectrum',[[0,'#a4f7e5'],[.35,'#557ce8'],[.65,'#d18eff'],[1,'#f4c4ae']])+radial('aura','#263b69'),
  circle(190,130,135,'url(#aura)')+`<g class="float">${iris}${circle(190,130,25,'#080b10','stroke="#c2d8fa" stroke-width=".6"')}</g>`+word(190,251,41,'#f1f5ff','text-anchor="middle"')+label('ROOM FOR WHAT COMES NEXT',190,277,'#68778c','text-anchor="middle"'),
  'Thirty-six hairline ellipses form an iridescent aperture with a perfectly open center.','Precision continuous curves and overlapping spectral light; no texture or particle field.');

add(146,'Modernist',400,300,'#f3f3ef','light',linear('cobalt',[[0,'#1343e8'],[1,'#6b8eff']]),
  label('VELA / DESKTOP',28,35,'#4a4d54')+path('M28 52H372','none','#d3d5d4','stroke-width=".7"')+
  `<g class="float">${path('M313 79L347 113L313 147L279 113Z','url(#cobalt)')}${path('M313 92V134M292 113H334','none','#fff','stroke-width="1" opacity=".7"')}</g>`+
  word(23,216,103,'#13171e','font-weight="600" letter-spacing="-7"')+label('IDEAS INTO MOTION',30,252,'#585e68')+text('01 — ∞',372,252,9,'#585e68','text-anchor="end"'),
  'Oversized black Vela lettering anchors an ivory canvas, balanced by a small cobalt diamond in the upper right.','Crisp editorial typography, flat ivory and one glossy blue geometric accent.');

add(147,'Obsidian dawn',360,340,'#070708','dark',radial('halo','#a692cf')+linear('rim',[[0,'#ddd6ed'],[.35,'#72617c'],[.65,'#141016'],[1,'#d6bddc']])+linear('stone',[[0,'#27252d'],[.55,'#0e0e11'],[1,'#020204']]),
  `<g class="bloom">${circle(180,142,113,'url(#halo)')}</g>`+circle(180,142,70,'url(#rim)')+circle(181,144,68.5,'url(#stone)')+
  path('M120 120A65 65 0 0 1 224 94','none','#eee1ff','stroke-width=".6" opacity=".7"')+word(180,270,36,'#e4dfe8','text-anchor="middle"')+label('BEGIN AGAIN',180,294,'#6e6876','text-anchor="middle"'),
  'A dark polished disc catches one fine crescent of light, with generous space around a small Vela signature.','Obsidian, edge light and soft atmosphere with smooth tonal transitions.');

add(148,'Folded light',380,320,'#edf5ee','light',linear('mint',[[0,'#e7fff7'],[.5,'#71cbb4'],[1,'#248b7d']])+linear('shade',[[0,'#2e7e77'],[1,'#b4ead7']])+blur,
  `<ellipse cx="190" cy="209" rx="64" ry="8" fill="#478978" opacity=".18" filter="url(#blur)"/><g class="float">`+
  path('M114 178L156 79L192 145L228 79L266 178L229 194L192 145L153 194Z','url(#mint)')+
  path('M156 79L165 161L192 145Z','url(#shade)')+path('M228 79L220 161L192 145Z','#1c736c')+
  path('M114 178L156 79L192 145L228 79','none','#fff','stroke-width="1.3" opacity=".9"')+`</g>`+
  word(190,267,39,'#254e46','text-anchor="middle"')+label('integrated development environment',190,291,'#6b9387','text-anchor="middle"'),
  'Two folded mint planes meet as a sculptural M, floating above an airy pale-green surface.','Satin planes, clean folds, soft shadows and mint-to-emerald shading.');

add(149,'Blue lens',400,300,'#101b3d','dark',radial('lens','#86c6ff')+linear('blue',[[0,'#172b60'],[.5,'#4176ce'],[1,'#8cd3ff']])+blur,
  `<ellipse cx="299" cy="149" rx="118" ry="121" fill="url(#blue)"/>`+
  `<ellipse cx="289" cy="141" rx="74" ry="103" fill="#122347" stroke="#a2d6ff" stroke-width="1.2" transform="rotate(22 289 141)"/>`+
  `<ellipse cx="277" cy="126" rx="59" ry="89" fill="url(#lens)" opacity=".45" transform="rotate(22 277 126)"/>`+
  `<ellipse class="bloom" cx="300" cy="90" rx="16" ry="58" fill="#b8e9ff" opacity=".2" filter="url(#blur)" transform="rotate(25 300 90)"/>`+
  label('VELA DESKTOP',29,39,'#99b1da')+word(26,178,62,'#edf7ff')+label('A CLEARER SPACE',30,214,'#93add5'),
  'A huge optical lens clips against the right edge while the Vela name stays cleanly left-aligned.','Polished blue glass, elliptical refraction and a broad smooth highlight.');

let silk='';for(let i=0;i<22;i++){const y=75+i*4;silk+=path(`M-30 ${y+45}C90 ${y-90} 192 ${y+133} 425 ${y-25}`,'none','url(#silk)',`stroke-width="2.4" opacity="${.35+i*.025}"`);}
add(150,'Silk current',400,300,'#210e1b','dark',linear('silk',[[0,'#6c244d'],[.3,'#f094af'],[.5,'#ffdad5'],[.72,'#bd497d'],[1,'#4f1c4b']]),
  `<g class="float">${silk}</g>`+word(200,237,46,'#f7e6eb','text-anchor="middle"')+label('FROM THOUGHT TO FORM',200,266,'#ab7e95','text-anchor="middle"'),
  'A sweeping field of continuous rose-satin curves crosses the canvas above a centered Vela wordmark.','Fine silk-like lines, warm specular bands and a dark mulberry ground.');

add(151,'Quiet luxury',380,320,'#eae7df','light',linear('gold',[[0,'#76624a'],[.25,'#d1bda0'],[.5,'#7a684e'],[.8,'#a89070'],[1,'#eadcc3']]),
  label('THE VELA ENVIRONMENT',190,36,'#827668','text-anchor="middle"')+
  `<g class="draw" fill="none" stroke="url(#gold)" stroke-width="1.2">`+Array.from({length:7},(_,i)=>`<rect x="${143+i*4}" y="${77+i*4}" width="${94-i*8}" height="${94-i*8}" rx="${45-i*4}" transform="rotate(-26 190 124)"/>`).join('')+`</g>`+
  text('Vela',190,249,61,'#3b3834','font-family="Georgia,serif" font-style="italic" text-anchor="middle" letter-spacing="-3"')+label('SPACE TO DO YOUR BEST WORK',190,278,'#8b7c68','text-anchor="middle"'),
  'Concentric champagne-gold contours sit above an elegant italic wordmark on warm stone.','Fine metallic linework and a smooth paper-colored surface; an understated editorial direction.');

const knot='M144 126C99 61 232 51 220 116C208 181 126 185 142 124C158 63 283 105 242 156C201 207 130 193 168 146C206 99 257 175 208 180';
add(152,'One continuous thought',380,320,'#101414','dark',linear('thread',[[0,'#c4f7dc'],[.4,'#5bcfa9'],[.7,'#e2ffed'],[1,'#308d78']])+blur,
  `<g class="float">${path(knot,'none','#2a8f71','stroke-width="12" opacity=".12" filter="url(#blur)"')}${path(knot,'none','url(#thread)','stroke-width="5" stroke-linecap="round"')}${path(knot,'none','#e6fff4','stroke-width=".5" transform="translate(-1 -1)" opacity=".6"')}</g>`+
  word(190,257,40,'#e4f1e8','text-anchor="middle"')+label('FOLLOW THE IDEA',190,283,'#6c9185','text-anchor="middle"'),
  'One luminous green filament loops into an asymmetric knot, giving a quiet sense of thought becoming form.','An unbroken rounded filament with smooth green highlights against deep charcoal.');

add(153,'Soft instrument',400,300,'#e9eaf0','light',linear('shell',[[0,'#fff'],[.4,'#e1e1e9'],[1,'#a9adbf']])+linear('core',[[0,'#8872e3'],[.5,'#b7c4fc'],[1,'#7563bd']])+blur,
  `<ellipse cx="200" cy="179" rx="79" ry="11" fill="#777e9e" opacity=".23" filter="url(#blur)"/><g class="float"><rect x="111" y="87" width="178" height="83" rx="41.5" fill="url(#shell)" stroke="#fff" stroke-width=".8"/><rect x="124" y="100" width="152" height="57" rx="28.5" fill="#a5a8b8"/><rect x="126" y="102" width="148" height="53" rx="26.5" fill="url(#core)"/>`+
  path('M200 111C202 123 205 126 222 128C205 130 202 133 200 146C198 133 195 130 178 128C195 126 198 123 200 111Z','none','#fff','stroke-width="1" opacity=".9"')+`</g>`+
  word(200,244,38,'#393b50','text-anchor="middle"')+label('READY FOR POSSIBILITY',200,272,'#89899d','text-anchor="middle"'),
  'A tactile white capsule houses a violet illuminated face with a fine four-point light.','Soft ceramic housing, a recessed glass center and gently diffused physical shadows.');

let topo='';for(let i=0;i<32;i++){const y=64+i*3.2;topo+=path(`M48 ${y}C108 ${y+40} 136 ${y-51} 195 ${y-12}S286 ${y+54} 351 ${y-12}`,'none','url(#topo)',`stroke-width=".7" opacity="${.28+i*.018}"`);}
add(154,'Signal surface',400,300,'#101118','dark',linear('topo',[[0,'#3853a6'],[.4,'#a499f8'],[.65,'#e6d9ff'],[1,'#5a6d9b']]),
  label('VELA',29,35,'#a69cbd')+label('DESKTOP ENVIRONMENT',371,35,'#6e6b80','text-anchor="end"')+`<g class="float">${topo}</g>`+
  word(28,251,52,'#f2edff')+path('M249 245H371','none','#4d485d','stroke-width="1"')+path('M249 245H289','none','#c3b2ef','stroke-width="1.5" class="draw"'),
  'A fine violet contour surface floats above a left-aligned Vela signature and a restrained launch indicator.','Smooth topology lines, continuous cubic curves and measured violet light on graphite.');

export default designs;
