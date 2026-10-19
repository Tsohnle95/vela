// Independent physical, print and typographic studies. Shared helpers draw primitives only.
const esc = value => String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
const r = (x,y,w,h,fill,extra='') => `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${fill}" ${extra}/>`;
const p = (d,fill='none',stroke='none',extra='') => `<path d="${d}" fill="${fill}" stroke="${stroke}" ${extra}/>`;
const l = (x,y,x2,y2,stroke,width=1,extra='') => `<line x1="${x}" y1="${y}" x2="${x2}" y2="${y2}" stroke="${stroke}" stroke-width="${width}" ${extra}/>`;
const e = (x,y,rx,ry,fill,extra='') => `<ellipse cx="${x}" cy="${y}" rx="${rx}" ry="${ry}" fill="${fill}" ${extra}/>`;
const t = (value,x,y,size,fill,extra='') => `<text x="${x}" y="${y}" font-size="${size}" fill="${fill}" font-family="Arial,Helvetica,sans-serif" ${extra}>${esc(value)}</text>`;
const s = (value,x,y,size,fill,extra='') => `<text x="${x}" y="${y}" font-size="${size}" fill="${fill}" font-family="Georgia,serif" ${extra}>${esc(value)}</text>`;
const m = (value,x,y,size,fill,extra='') => `<text x="${x}" y="${y}" font-size="${size}" fill="${fill}" font-family="Menlo,Consolas,monospace" ${extra}>${esc(value)}</text>`;
const g = (body,transform,extra='') => `<g transform="${transform}" ${extra}>${body}</g>`;
const repeat = (n,fn) => Array.from({length:n},(_,i)=>fn(i)).join('');
function dust(seed,n,color,box=[0,0,1440,900],opacity=.15) {
  let state=seed;const random=()=>((state=(state*1664525+1013904223)>>>0)/4294967296);
  return repeat(n,()=>r((box[0]+random()*box[2]).toFixed(1),(box[1]+random()*box[3]).toFixed(1),random()>.9?2:1,1,color,`opacity="${opacity}"`));
}
const sources={cosmos:'ChatGPT Image Oct 13, 2026, 03_23_53 PM.png',pixels:'ChatGPT Image Oct 13, 2026, 03_24_00 PM.png',saturn:'ChatGPT Image Oct 14, 2026, 05_22_18 PM-4.png',city:'Untitled.png'};
const concepts=[];
function add(id,name,family,description,bg,tone,source,composition,silhouette,typography,material,nearest,body) {
  body=body.replaceAll(/id="([^\"]+)"/g,(_,value)=>`id="go${id}-${value}"`).replaceAll(/url\(#([^\)]+)\)/g,(_,value)=>`url(#go${id}-${value})`);
  concepts.push({id:String(id),name,family,description,bg,tone,source:source.map(key=>sources[key]),composition,silhouette,typography,material,nearest,art:`<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="900" viewBox="0 0 1440 900" role="img" aria-label="${esc(description)}">${r(0,0,1440,900,bg)}${body}</svg>`});
}

// 65. Offset ink impressions slice the word itself; no central pictogram.
{
  const print=p('M-90 700L630 34L900 48L1460 734L1305 900L746 266L107 900Z','#e25c43','none','opacity=".82"')+
    p('M148 0H457L1208 900H909Z','#453090','none','opacity=".9"')+
    p('M-20 470L1370 190L1515 384L-20 698Z','#41a4ac','none','opacity=".83"')+
    p('M456 0L752 0L944 900H786Z','#e1af3c','none','opacity=".76"');
  add(65,'Chromatic crossing','Ink collision','Scarlet, teal, ochre and violet ink planes collide across an interrupted black Vela heading.','#f5f1e4','light',['pixels'],
    'Flat full-bleed print with four crossing diagonals; word cuts from top-left to center-right through a white seam.','Long intersecting trapezoids, no bounded object.','Huge black sans letters are separated across the diagonal; two small printer notes at lower-left.','Translucent spot-color shapes, tiny ink pinholes and offset edge registrations.',
    '103 Misalignment: solid irregular overprinted planes rather than two square line grids; oblique word fragmentation, warm ink and a flat poster replace moire.',
    print+`<defs><clipPath id="upper">${p('M0 0H1440V252L0 560Z','#fff')}</clipPath><clipPath id="lower">${p('M0 603L1440 294V900H0Z','#fff')}</clipPath></defs>`+
    g(t('VELA',82,414,254,'#192125','font-weight="900" letter-spacing="-19"'),'translate(0 0)','clip-path="url(#upper)"')+
    g(t('VELA',102,445,254,'#192125','font-weight="900" letter-spacing="-19"'),'translate(0 0)','clip-path="url(#lower)"')+
    p('M0 560L1440 252V294L0 603Z','#f5f1e4')+m('LIGHT / INK / INTERSECTION',74,792,13,'#292a35','letter-spacing="2"')+
    l(75,813,347,813,'#292a35')+dust(65,1700,'#f6f1e4',[0,0,1440,900],.28)+m('Vela',1300,840,17,'#202131'));
}

// 66. The word is a physical depression in a nearly empty sheet.
{
  const letters=t('VELA',98,805,282,'#c2bba9','font-weight="700" letter-spacing="-18"');
  add(66,'Pressure of words','Blind impression','A giant blind-embossed Vela sits almost below the edge of warm ivory paper under a diagonal rake of light.','#e5dfcf','light',['cosmos'],
    'Extreme close-up of a matte sheet, five sixths quiet space; giant recessed lettering touches the lower edge.','A low horizontal debossed word with a long upper-right light wedge.','One low, weighty uppercase word; a tiny vertical printer mark at the far right.','Layered paper-colored letter edges simulate pressed stock; fine fiber grain and broad raking light.',
    '29 One small beginning: monumental tactile low relief, extreme edge crop, oblique light and warm paper replace a tiny centered flat word on dark space.',
    `<defs><linearGradient id="rake" x1="0" y1="1" x2="1" y2="0"><stop stop-color="#beb5a3" stop-opacity=".3"/><stop offset=".48" stop-color="#fffef2" stop-opacity="0"/><stop offset="1" stop-color="#fffdf2" stop-opacity=".9"/></linearGradient></defs>`+
    r(0,0,1440,900,'url(#rake)')+p('M0 664L1440 154V345L0 851Z','#fffdf1','none','opacity=".13"')+
    g(letters,'translate(-2 -3)')+g(letters.replaceAll('#c2bba9','#fff9e8'),'translate(2 3)')+
    t('VELA',98,805,282,'#ddd6c6','font-weight="700" letter-spacing="-18" stroke="#cfc6b3" stroke-width=".7"')+
    m('PRESS / RELEASE',1362,662,10,'#8b8374','transform="rotate(-90 1362 662)" letter-spacing="3"')+dust(66,3200,'#84745a',[0,0,1440,900],.13));
}

// 67. Every fold has its own plane and letter perspective.
{
  const shadow=p('M-70 614L226 443L493 594L739 417L996 549L1225 326L1500 421V555L1219 468L1013 684L738 553L482 735L224 576L-70 750Z','#40243f','none','opacity=".18"');
  const paper=p('M-80 448L225 281L225 551L-80 719Z','#f5e8c9')+p('M225 281L492 444V712L225 551Z','#cebca4')+
    p('M492 444L738 260V531L492 712Z','#f9eace')+p('M738 260L1006 395V664L738 531Z','#bdafa4')+
    p('M1006 395L1230 169V442L1006 664Z','#fff3d7')+p('M1230 169L1500 265V538L1230 442Z','#d9c5a6');
  add(67,'Unfold','Paper choreography','An accordion ribbon bends across the whole screen, with one oversized violet letter printed on each raised paper panel.','#927b8b','dark',['pixels'],
    'Side-on close view of an accordion ribbon entering and leaving both edges, rising from left to right.','Six alternating open zigzag paper planes.','Individual VELA letters occupy separate faces and skew with the folds; no detached heading.','Cream cut paper, shaded alternating facets, long soft-edged polygon shadow and sharp illuminated creases.',
    '81 Ticker thought: a single unperforated accordion with large isolated letters and open ends, versus many thin interwoven zigzags of repeated small text viewed from above.',
    shadow+paper+g(t('M',0,0,155,'#55334e','font-weight="700"'),'matrix(1 -.55 0 1 50 615)')+
    g(t('I',0,0,155,'#55334e','font-weight="700"'),'matrix(1 .61 0 1 278 497)')+
    g(t('R',0,0,155,'#55334e','font-weight="700"'),'matrix(1 -.74 0 1 539 610)')+
    g(t('A',0,0,155,'#55334e','font-weight="700"'),'matrix(1 .5 0 1 823 497)')+
    g(t('✦',0,0,155,'#55334e','font-weight="700"'),'matrix(.87 -.88 0 1 1075 502)')+
    l(225,281,225,551,'#fff6dd',2)+l(492,444,492,712,'#9e8982',2)+l(738,260,738,531,'#fff9e4',2)+l(1006,395,1006,664,'#907c78',2)+
    m('TAKE SHAPE',84,113,12,'#e8d7d9','letter-spacing="5"')+dust(67,850,'#fff2d7',[0,0,1440,900],.13));
}

// 68. Stencil letter columns form the entire image.
{
  add(68,'Hard line','Cut steel type','Three unequal black, violet and chalk columns carry gigantic cropped stencil letters divided by deliberate cutout bridges.','#d9d5c7','light',['pixels'],
    'Flat frontal composition with three unequal full-height columns, a heavy horizontal band interrupts all three near the bottom.','Tall compressed stencil blocks ending at hard screen edges.','MI stacked in the wide black column, R across a narrow violet column, one enormous A in chalk; type is the architecture.','Industrial stencil cutouts, abrasive edge marks and chalky overprint.',
    '02 Type in space: segmented multi-axis columns, punched stencil gaps, edge-to-edge material and stacked reading replace one horizontal word with celestial decoration.',
    r(0,0,657,900,'#171619')+r(657,0,339,900,'#8a64b1')+
    t('M',-76,438,590,'#d9d5c7','font-weight="900" letter-spacing="-15"')+t('I',206,929,570,'#d9d5c7','font-weight="900"')+
    t('R',622,434,455,'#171619','font-weight="900"')+t(' ',724,899,498,'#171619','font-weight="900"')+
    t('A',945,815,938,'#171619','font-weight="900"')+
    r(0,204,657,30,'#171619')+r(412,440,36,460,'#171619')+r(657,206,339,20,'#8a64b1')+
    r(1090,0,24,900,'#d9d5c7')+r(0,681,1440,39,'#d9d5c7')+m('VELA  /  CUT INTO THE DAY',46,707,15,'#171619','letter-spacing="4"')+
    repeat(70,i=>l((i*193)%1440,(i*139)%900,(i*193)%1440+12,(i*139)%900,'#bbb3b1',1,'opacity=".25"')));
}

// 69. Low mechanism, high untouched paper, one live ink ribbon.
{
  let keys='';for(let row=0;row<3;row++)for(let col=0;col<15;col++){const x=55+col*94+row*24,y=787+row*83;keys+=e(x+8,y+17,34,24,'#080b11')+e(x,y,34,24,'#46454b','stroke="#aaa397" stroke-width="3"')+m('QWERTYUIOPASDFG'[col],x,y+6,14,'#d4caba','text-anchor="middle"');}
  add(69,'Ribbon memory','Mechanical close-up','A cropped typewriter fills the very bottom; its violet ink ribbon passes across a nearly empty ivory sheet carrying the Vela name.','#ede5d2','light',['cosmos'],
    'Frontal macro of carriage at the bottom edge; white paper fills the upper two thirds, ribbon is a taut horizontal foreground line.','Low layered bank of round keys, two dark ribbon hubs, dense converging typebars.','A tiny monospaced Vela is typed onto the actual ink ribbon; a ghost ink impression rests on the page.','Mechanical steel strokes, rubber platen, worn circular key caps and dyed fabric ribbon.',
    '71 Departure board: irregular round mechanics cropped low with vast blank paper, versus a high stepped field of rectangular hinged character tiles on charcoal.',
    p('M0 69Q720 12 1440 69V690H0Z','#f4eedf')+l(0,669,1440,669,'#9b9487',3)+r(0,675,1440,225,'#29282e')+
    repeat(38,i=>l(170+i*30,838,605+i*6,650,i%3===0?'#b1a99e':'#656369',4))+
    e(211,691,126,55,'#12131a','stroke="#726b6d" stroke-width="8"')+e(1220,691,126,55,'#12131a','stroke="#726b6d" stroke-width="8"')+
    repeat(6,i=>e(211,691,32+i*15,14+i*6,'none',`stroke="#5c515d" stroke-width="2"`))+
    repeat(6,i=>e(1220,691,32+i*15,14+i*6,'none',`stroke="#5c515d" stroke-width="2"`))+
    r(202,613,1030,57,'#5c3569')+r(202,641,1030,29,'#251f30')+m('Vela',399,648,28,'#d9c8e0','letter-spacing="8"')+
    r(634,584,175,118,'#b3aaa0')+p('M651 584V659L674 682H769L792 659V584','#d2c8b6','#6a6562','stroke-width="4"')+
    r(684,615,74,42,'#2b2633')+l(721,583,721,670,'#4f4753',3)+keys+
    m('Vela',617,508,32,'#504454','opacity=".3" letter-spacing="4"')+dust(69,2000,'#7b705d',[0,0,1440,670],.12));
}

// 70. Counterweight relationships, sparse left typography, open descending right.
{
  add(70,'Balanced thought','Suspended geometry','A delicate black mobile holds an ochre triangle, scarlet bar and violet disc beside a tall narrow Vela heading.','#e9e4d8','light',['pixels'],
    'Looking slightly upward at a suspended asymmetric mobile in the top-right; typography climbs the far-left margin.','Sparse hanging angled rods with an off-balance triangle, thin rectangle and displaced disc.','Tall stacked M I R A down the left with a single small horizontal caption beneath.','Matte painted metal shapes, taut black string and fine offset paper shadows.',
    '101 Pendulum choir: branching rods and three different flat geometric weights, left vertical lettering and warm poster stock replace parallel hanging spheres and arc traces.',
    l(940,-30,940,167,'#3c3935',2)+l(676,245,1153,108,'#34312e',6)+l(681,249,681,429,'#3c3935',2)+
    l(1150,111,1150,341,'#3c3935',2)+p('M1051 348L1314 313L1208 539Z','#d7a23d')+
    l(520,492,893,400,'#373330',5)+l(681,428,681,451,'#373330',2)+l(520,491,520,708,'#373330',2)+
    l(895,399,895,584,'#373330',2)+r(452,711,230,24,'#ba4838','transform="rotate(9 452 711)"')+
    e(896,630,45,45,'#765795')+l(1098,353,1207,524,'#b78425',1)+
    ['M','I','R','A'].map((c,i)=>t(c,79,164+i*141,153,'#312d2e','font-weight="300"')).join('')+
    m('A LITTLE BALANCE.',333,813,12,'#6b615b','letter-spacing="2"')+dust(70,2400,'#7a705f',[0,0,1440,900],.1));
}

// 71. One broad stair of differently hinged split flaps, no fake operating data.
{
  let tiles='';const pos=[[80,129,232,335],[332,172,232,335],[584,221,232,335],[836,268,210,335],[1066,314,274,335]];
  'VELA'.split('').forEach((char,i)=>{const [x,y,w,h]=pos[i];tiles+=r(x+10,y+13,w,h,'#080a0d','rx="12"')+r(x,y,w,h,'#252a2f','rx="9" stroke="#41474e" stroke-width="2"')+r(x+6,y+7,w-12,h/2-8,'#353c42','rx="5"')+t(char,x+w/2,y+263,244,'#ece5cb','text-anchor="middle" font-weight="700"')+l(x,y+h/2,x+w,y+h/2,'#05090c',5)+r(x-6,y+h/2-16,20,32,'#9e9e90','rx="4"')+r(x+w-14,y+h/2-16,20,32,'#8d9289','rx="4"')+l(x+13,y+8,x+w-13,y+8,'#5a6466',2);});
  add(71,'Departure board','Split flap','Five split-flap letter tiles descend in an uneven physical staircase across a charcoal wall.','#171b20','dark',['pixels'],
    'Frontal close-up of five broad flaps stepping down from upper-left to lower-right, leaving an empty low-left wedge.','Stair-step row of chunky hinged rectangular tiles.','One cream letter per split flap, no secondary headline; a small static caption lives far below at right.','Painted steel, thick hinge barrels, inset tile shadows and horizontal split seams.',
    '68 Hard line: individual raised tile bodies on a diagonal stair with actual hinges, moderate letters and dark open corners replace flat floor-to-ceiling stencil columns.',
    repeat(17,i=>l(0,53+i*57,1440,53+i*57,'#232a31',1))+tiles+
    l(1054,772,1340,772,'#606b70')+m('VELA',1054,808,16,'#d9dece','letter-spacing="5"')+m('THE MOMENT BEFORE.',1054,835,11,'#79878b')+
    dust(71,1600,'#939b93',[0,0,1440,900],.08));
}

// 72. Optical verticality is punctured by the jagged silhouette itself.
{
  let stripes='';for(let x=362;x<1440;x+=11){const roof=295+Math.sin(x*.018)*52-Math.cos(x*.037)*31;stripes+=r(x,0,4,roof,'#2f2745')+r(x,704,4,196,'#2f2745');}
  const skyline=p('M375 703V577H426V491H447V397H458V489H487V580H521V421H543V305H559V228H569V301H582V442H613V557H651V359H674V284H690V146H702V283H714V361H738V570H781V465H803V381H818V266H831V381H845V466H868V610H907V518H926V447H947V324H963V192H976V324H993V448H1010V549H1042V617H1079V537H1100V478H1119V371H1133V478H1152V538H1180V589H1215V428H1235V359H1250V286H1262V359H1276V430H1300V571H1332V638H1390V704Z','#f2e9d4');
  add(72,'Between the stripes','Optical cutout','Dense black vertical lines stop at an ivory citadel silhouette whose slender spires become the only bright opening.','#f2e9d4','light',['city'],
    'Flat optical screen across the right three quarters; a quiet unprinted vertical gutter at left holds a small heading.','Negative jagged skyline punched out of a vertical striped field.','Small stacked left gutter text with a compact serif Vela near eye level; illustration bears the visual weight.','Lenticular black ink stripes and ivory knockouts, with tiny violet misregistration slivers.',
    '44 Roofline commons: abstract negative cutout, vertical optical line material, flat frontal framing and small gutter typography replace spatial terraced roofs and giant obscured letters.',
    stripes+skyline+repeat(98,i=>{const x=368+i*11;return r(x,704-(i%7)*7,2,(i%7)*7,'#8e78ad','opacity=".5"');})+
    s('Vela',71,381,65,'#302640')+l(76,419,264,419,'#302640')+m('BETWEEN',77,459,11,'#584d68','letter-spacing="4"')+m('THE LINES',77,479,11,'#584d68','letter-spacing="4"')+
    r(334,68,2,764,'#9f8eb0')+dust(72,1200,'#7b6c6a',[0,0,1440,900],.09));
}

// 73. Three irregularly placed exposures carried by a genuinely curling strip.
{
  let perf='';for(let i=0;i<32;i++){const u=i/31,x=82+1290*u,y=912-850*u+80*Math.sin(u*Math.PI*2);perf+=r(x,y,16,27,'#d7c6b7',`rx="3" transform="rotate(49 ${x} ${y})"`)+r(x+205,y+96,16,27,'#d7c6b7',`rx="3" transform="rotate(49 ${x+205} ${y+96})"`);}
  add(73,'Frames in motion','Celluloid','A black perforated film ribbon curls diagonally through warm paper, carrying three unequal violet cosmic exposures.','#d7c6b7','light',['cosmos','saturn'],
    'Oblique tabletop macro; film enters cropped lower-left, bows across the middle and exits top-right; title splits between open corners.','Long curling perforated S-band at steep diagonal.','Large Vela serif at top-left and tiny spaced caption in lower-right; frame numbers remain purely photographic.','Translucent brown celluloid edges, violet grain, dark contact-print frames and punched sprocket holes.',
    '15 Three bodies: one continuous bent photographic object with oblique exposures and punched edges, asymmetric corner type and warm table replace equal upright triptych panels.',
    `<defs><linearGradient id="film"><stop stop-color="#181520"/><stop offset=".4" stop-color="#54414f"/><stop offset=".7" stop-color="#191622"/><stop offset="1" stop-color="#3b293b"/></linearGradient></defs>`+
    p('M-117 936C116 882 297 727 482 560S748 505 878 312S1095 101 1340-135L1575 32C1350 232 1187 281 1103 431S862 632 696 749S416 961 149 1080Z','#493740','none','opacity=".2" transform="translate(20 24)"')+
    p('M-117 936C116 882 297 727 482 560S748 505 878 312S1095 101 1340-135L1575 32C1350 232 1187 281 1103 431S862 632 696 749S416 961 149 1080Z','url(#film)')+
    g(r(0,0,256,194,'#231a3e')+p('M-20 175Q80 90 280 55','#65468c')+dust(731,240,'#e2d4ff',[0,0,256,194],.7),'translate(334 671) rotate(-41)')+
    g(r(0,0,277,214,'#302347')+p('M-20 185L72 74L127 143L179 47L289 200Z','#8c6ca6')+dust(732,250,'#e6ddf1',[0,0,277,214],.65),'translate(696 484) rotate(-40)')+
    g(r(0,0,268,195,'#21182e')+p('M-20 149Q160 8 290 102L290 195H-20Z','#9f82b6')+dust(733,250,'#dfcced',[0,0,268,195],.75),'translate(1013 239) rotate(-45)')+
    perf+s('Vela',73,144,104,'#30263a')+m('LIGHT, HELD BRIEFLY.',1041,803,11,'#49374d','letter-spacing="2"')+
    l(1041,824,1357,824,'#49374d')+dust(73,900,'#5e4558',[0,0,1440,900],.1));
}

// 74. Material fills the screen; engraving is a quiet discovery.
{
  add(74,'Made of metal','Brushed alloy','A flush engraved Vela plate sits inside a vast brushed alloy surface crossed by one dramatic diagonal reflection.','#868895','light',['saturn'],
    'Extreme frontal material crop with the beveled nameplate low-right; a wide skew reflection traverses almost the entire surface.','Unbroken metal plane with one subtle recessed rectangular seam.','Tiny widely tracked engraved uppercase word on the plate, no poster headline.','Hundreds of directional brush strokes, bevel highlights, four countersunk screws and sharp silver reflected bands.',
    '66 Pressure of words: cold full-bleed directional alloy, inset hardware low-right and minute engraving replace warm matte stock and huge low blind letters.',
    `<defs><linearGradient id="metal" x1="0" y1="0" x2="1" y2=".5"><stop stop-color="#b8bcc5"/><stop offset=".26" stop-color="#717482"/><stop offset=".52" stop-color="#d2d3d5"/><stop offset=".7" stop-color="#8b8d99"/><stop offset="1" stop-color="#585c6a"/></linearGradient></defs>`+
    r(0,0,1440,900,'url(#metal)')+repeat(440,i=>l(0,i*2.1,1440,i*2.1-23,i%4===0?'#f1f0ee':'#3e4351',i%3===0?.8:.4,`opacity="${i%4===0?.22:.12}"`))+
    p('M-100 563L1175-40H1440V101L-50 826Z','#e8e7e4','none','opacity=".31"')+
    p('M-30 668L1396 0H1440V53L0 741Z','#fff7ed','none','opacity=".25"')+
    r(852,595,452,178,'none','rx="3" stroke="#414653" stroke-width="2"')+l(854,775,1304,775,'#e8e6e1',2)+l(1306,597,1306,773,'#c1c0c3')+
    t('M I R A',911,700,34,'#d3d0cd','font-weight="500"')+t('M I R A',910,699,34,'#505260','font-weight="500"')+
    [[876,620],[1280,620],[876,748],[1280,748]].map(([x,y])=>e(x+1,y+1,8,8,'#c3c3c5')+e(x,y,8,8,'#5a5f6a')+l(x-4,y-3,x+4,y+3,'#aaaab0',2)).join(''));
}

// 75. Rough diagonal tear interrupts the letter material, with stars only below.
{
  const rip='M485-20L511 47L494 89L535 133L524 164L583 216L569 243L620 285L604 321L655 365L638 400L702 445L690 470L739 511L727 552L787 594L772 628L828 666L815 708L875 747L860 778L923 830L910 862L965 920H643L619 867L573 843L586 807L528 765L543 723L484 692L495 654L442 612L454 571L398 529L411 496L357 452L371 419L317 375L330 344L278 305L290 268L242 229L254 196L208 151L220 122L170 71L181 42L144-20Z';
  add(75,'Through the tear','Paper rupture','An enormous ragged diagonal tear opens warm paper to a deep violet star field, splitting the black Vela lettering into two distant pieces.','#e6d8bd','light',['cosmos'],
    'Macro overhead sheet split by a wide descending diagonal fissure from upper-left to lower-right; two unequal paper banks hold separate type.','Ragged lightning-shaped opening with thick torn fibers.','MI sits high-right and RA low-left, both heavy cropped fragments; the missing line is the subject.','Layered torn-stock edges, violet ink depth, sparse pinprick light and warm rough fibers.',
    '34 Between worlds: flat ragged paper material, interrupted oversize type, overhead macro and violet stars replace a deep canyon with a bridge and spatial rock walls.',
    p(rip,'#bbaa94','none','transform="translate(12 10)"')+p(rip,'#f7eedb','none','transform="translate(-8 -4)"')+p(rip,'#171025')+
    `<defs><clipPath id="rip">${p(rip,'#fff')}</clipPath><linearGradient id="depth" x1="0" y1="0" x2="1" y2="0"><stop stop-color="#130d21"/><stop offset=".55" stop-color="#58417f"/><stop offset="1" stop-color="#100c19"/></linearGradient></defs>`+
    g(r(0,0,1440,900,'url(#depth)')+dust(75,1400,'#d6b9ff',[100,0,900,900],.8),'translate(0 0)','clip-path="url(#rip)"')+
    t('MI',799,297,270,'#2a2530','font-weight="800" letter-spacing="-18"')+t('RA',36,813,245,'#2a2530','font-weight="800" letter-spacing="-16"')+
    repeat(34,i=>l(180+i*22,68+i*24,198+i*22,66+i*24,'#fdf5e8',1.3))+
    m('THE SURFACE IS ONLY A BEGINNING.',962,740,11,'#6a5b60','transform="rotate(90 962 740)"')+dust(751,1900,'#796954',[0,0,1440,900],.14));
}

// 76. An irregular packaging cross, drawn flat except one lifted violet tab.
{
  const net='M589 112H837L884 166V286H1137L1202 339V548L1137 585H884V798L833 842H590L542 799V585H301L245 544V341L298 286H542V166Z';
  add(76,'Flat to form','Unmade object','An unfolded ivory carton net lies on an olive table, with one violet flap lifting out of the flat drawing.','#777963','dark',['pixels'],
    'Overhead tabletop view of a large asymmetrical cross slightly right of center, lifted top flap is the only depth cue.','Wide carton cross with notched tabs and one diagonal raised corner.','Restrained black Vela printed in the broad center panel, tiny crop notes at the left without a separate headline.','Cut card, dashed score lines, knife notches, flat olive ink and one violet folded plane.',
    '127 Paper computation: a single solid packaging net with broad blank panels, orthographic view, printed center and one lifted tab rather than a repeating cut lattice in perspective.',
    p(net,'#373b33','none','transform="translate(16 20)" opacity=".25"')+p(net,'#eee4ce','#d5c4a8','stroke-width="2"')+
    p('M589 112H837L884 166L837 221H589Z','#9c78b3')+p('M589 112L542 166L589 221Z','#684b7e')+
    l(542,286,884,286,'#9d9787',1,'stroke-dasharray="6 6"')+l(542,585,884,585,'#9d9787',1,'stroke-dasharray="6 6"')+
    l(542,286,542,585,'#9d9787',1,'stroke-dasharray="6 6"')+l(884,286,884,585,'#9d9787',1,'stroke-dasharray="6 6"')+
    l(301,316,301,553,'#aaa38f',1,'stroke-dasharray="3 5"')+l(1137,316,1137,553,'#aaa38f',1,'stroke-dasharray="3 5"')+
    s('Vela',595,471,82,'#333832')+m('SPACE TAKES SHAPE',598,505,10,'#6b6c59','letter-spacing="2"')+
    m('FOLD',941,449,10,'#6b6c59','transform="rotate(90 941 449)" letter-spacing="4"')+l(145,331,223,331,'#e4dfca')+
    m('A FORM',79,371,12,'#e4dfca')+m('IN WAITING.',79,391,12,'#e4dfca')+dust(76,2100,'#2c3529',[0,0,1440,900],.1));
}

// 77. Pleated fan radiates from an off-screen hinge, never a round emblem.
{
  const hinge=[1263,750];let fan='';const angles=[-173,-165,-157,-149,-141,-133,-125,-117,-109,-101,-93,-85,-77,-69,-61];
  for(let i=0;i<angles.length-1;i++){const a=angles[i]*Math.PI/180,b=angles[i+1]*Math.PI/180;const x1=hinge[0]+Math.cos(a)*920,y1=hinge[1]+Math.sin(a)*920,x2=hinge[0]+Math.cos(b)*920,y2=hinge[1]+Math.sin(b)*920;fan+=p(`M${hinge.join(' ')}L${x1} ${y1}L${x2} ${y2}Z`,['#f0dbb8','#d7b78d','#f7e2bb','#b29985'][i%4])+
    l(...hinge,x1,y1,i%2?'#7b5971':'#fff2d7',1.5);}
  add(77,'Open fan','Pleated light','A vast ochre paper fan spreads from a low-right hinge, its violet brush marks stretching across pleats above a small serif Vela.','#534351','dark',['pixels'],
    'Low oblique close view: fan dominates the upper-right and is cropped along the entire top edge; quiet dark triangle lower-left holds title.','Broad asymmetric serrated paper fan opening from the far-right bottom hinge.','Small ivory serif low-left, one understated spaced line below; no concentric labels.','Alternating warm paper facets, fine bamboo ribs and discontinuous violet brush strokes.',
    '10 Totality: edge-cropped angular fan facets, open wedge geometry, warm physical paper and low-left type replace a closed centered glowing disc.',
    fan+p('M452 394Q797 80 1243 82L1283 129Q844 157 487 451Z','#6f527f','none','opacity=".7"')+
    p('M647 518Q1025 208 1424 265L1440 304Q1086 272 685 565Z','#9e748b','none','opacity=".75"')+
    repeat(14,i=>{const a=angles[i]*Math.PI/180;return l(1263,750,1263+Math.cos(a)*912,750+Math.sin(a)*912,'#6d5a54',i%2?2:4,'opacity=".45"');})+
    p('M1258 736L1278 746L1317 907H1298Z','#372b35')+e(1266,749,8,8,'#d8b983')+
    s('Vela',79,714,84,'#f4e3c9')+m('OPEN TO POSSIBILITY',84,758,10,'#cfb7c3','letter-spacing="3"')+dust(77,1600,'#fff0d9',[0,0,1440,900],.1));
}

// 78. Seven differently colored flat pieces form one angular bird, not a flock.
{
  add(78,'Flight in pieces','Tangram print','A seven-piece violet and vermilion tangram bird stretches upward across an ivory page, with two spare geometric pieces on the bottom rule.','#eee8d5','light',['pixels'],
    'Bird occupies high-right in a rising diagonal; massive heavy word sits low-left; two spare shapes rest far below the bird.','Seven-piece angular crane with a long extended beak and a wide split wing.','Low-left black slab word, tight two-line caption, no type centered under the bird.','Solid silkscreen tangram polygons with dark offset paper shadows and tiny registration scars.',
    '122 Shared sky: one large seven-piece abstract bird, flat isolated polygon assembly and grounded spare pieces replace repeated interlocking silhouettes dissolving into a free bird.',
    p('M686 429L984 132L988 431Z','#544078')+p('M994 132L1292 434L996 434Z','#846392')+
    p('M996 444L1291 444L1144 592Z','#b14938')+p('M685 441L982 441L833 591Z','#b299ab')+
    p('M997 454L1140 598L995 740L850 597Z','#40364f')+p('M1149 590L1333 407L1287 589Z','#8d6d92')+
    p('M1339 402L1432 400L1338 497Z','#d38e51')+
    t('Vela',63,710,183,'#282633','font-weight="900" letter-spacing="-13"')+
    m('SEPARATE PIECES.',72,756,12,'#5b5160','letter-spacing="2"')+m('ONE NEW DIRECTION.',72,777,12,'#5b5160','letter-spacing="2"')+
    l(689,820,1364,820,'#6c6067',1)+p('M757 819H862L757 714Z','#8d6b91')+p('M1178 755L1241 819L1304 755L1241 692Z','#c19460')+
    dust(78,2300,'#786967',[0,0,1440,900],.1));
}

// 79. A huge book corner and textile ribbon create a quiet material landscape.
{
  add(79,'A place held','Book margin','A deep plum bookmark bends around the upper-left corner of an enormous blank ivory book page.','#b8a79c','light',['cosmos'],
    'Extreme overhead crop of an open book page entering from top-left and filling the screen; long bookmark descends near left edge.','A vertical notched textile ribbon with a folded upper hook and diagonal book corner.','Small vertical Vela printed along the ribbon; a widely spaced marginal sentence rests at far-right bottom.','Deckled layered page edges, cream rag paper, woven plum ribbon and a shallow cloth shadow.',
    '83 The spines: one near-blank open page with a long single ribbon, overhead macro and marginal text replace dense frontal multicolored book spines and a bright vertical gap.',
    p('M-40-40H1440V900H163L-40 711Z','#7f706c','none','opacity=".18" transform="translate(11 16)"')+
    repeat(9,i=>p(`M${-40+i*3} ${-40+i*3}H1440V${898-i*3}H${155+i*3}L${-40+i*3} ${708-i*2}Z`,i%2?'#e6dcc6':'#cfc2ac'))+
    p('M-40-40H1440V856H175L-40 680Z','#f2ecd9')+p('M-40-40H444L301 67L-40 171Z','#ddd1b9')+
    p('M124-30L298-30L281 110L276 713L215 669L149 713L149 110Z','#8a697a','none','opacity=".3" transform="translate(13 9)"')+
    p('M124-30L298-30L281 110L276 713L215 669L149 713L149 110Z','#5f3959')+
    p('M124-30H298L281 110L149 110Z','#9a718c')+l(155,114,155,690,'#a57b99',2)+l(272,114,272,690,'#38283b',2)+
    repeat(100,i=>l(160,120+i*5.2,269,120+i*5.2,'#b28c9e',.5,'opacity=".18"'))+
    s('Vela',236,257,38,'#d0b4c2','transform="rotate(90 236 257)"')+m('A PLACE FOR YOUR THOUGHTS',1292,793,10,'#7a6a63','text-anchor="end" letter-spacing="4"')+
    dust(79,2800,'#917b62',[0,0,1440,900],.1));
}

// 80. Orthogonal letter labyrinth is compact and left-heavy, serif lies on open field.
{
  const glyphs={V:['10001','10001','01010','01010','00100'],E:['11111','10000','11110','10000','11111'],L:['10000','10000','10000','10000','11111'],A:['01110','10001','11111','10001','10001']};
  let maze='';'VELA'.split('').forEach((char,k)=>{const ox=[87,380,87,380,673][k],oy=[96,96,390,390,390][k];for(let j=0;j<5;j++)for(let i=0;i<5;i++)if(glyphs[char][j][i]==='1'){maze+=r(ox+i*45,oy+j*45,38,38,'#3d3d34')+p(`M${ox+i*45+7} ${oy+j*45+7}h24v24h-12v-12`,'none','#dbd9c0','stroke-width="2"');}});
  add(80,'Letters to get lost in','Typographic labyrinth','Orthogonal maze strokes assemble four block letters in a compact offset cluster, while a wide italic Vela stretches through the clear right-hand field.','#e1dfc9','light',['city'],
    'Frontal flat print: block maze letters form a left-heavy 2-over-2 cluster; oversized serif begins in the open top-right and trails off the edge.','Unequal stepped mass of square letter labyrinths.','Small modular VELA reads through construction; an opposing long italic serif word supplies a different rhythm.','Dense olive ink walls with ivory hairline passages, rough print tooth and controlled square breaks.',
    '111 One continuous line: four visibly letter-shaped disconnected maze masses, modular wall blocks, olive paper and opposing wide serif replace one dense Hilbert square and escape route.',
    maze+s('Vela',773,294,171,'#5f5570','font-style="italic" letter-spacing="-10"')+p('M818 335H1352','none','#5f5570','stroke-width="1"')+
    m('THERE IS ALWAYS ANOTHER WAY.',784,374,10,'#59584c','letter-spacing="2"')+l(88,760,320,760,'#3d3d34',5)+
    m('ENTER ANYWHERE.',88,798,12,'#3d3d34','letter-spacing="3"')+dust(80,2300,'#746b54',[0,0,1440,900],.13));
}

// 81. Staggered paper strips weave rather than repeat the solid accordion of 67.
{
  let strips='';const sections=[[10,415,410,570,1],[380,567,749,455,-1],[704,459,1067,653,1],[1028,650,1480,462,-1]];
  for(let band=0;band<3;band++){const dy=band*112;for(let i=0;i<sections.length;i++){const [x,y,x2,y2,dir]=sections[i];const height=76;strips+=p(`M${x} ${y+dy}L${x2} ${y2+dy}V${y2+dy+height}L${x} ${y+dy+height}Z`,(i+band)%2?'#d9c7ab':'#f0dfbf','#b59a7d','stroke-width="1"');strips+=g(m('VELA  ·  THOUGHT IN MOTION  ·  VELA',0,43,16,'#593b50','letter-spacing="2"'),`matrix(1 ${(y2-y)/(x2-x)} 0 1 ${x+18} ${y+dy+8})`);}}
  add(81,'Ticker thought','Paper weave','Three long cream ticker strips zigzag in and out of one another across the lower screen, their plum type changing direction with each crease.','#9b6c72','dark',['pixels'],
    'Overhead view of three interwoven narrow zigzags across the lower two thirds, with a wide empty blush upper margin.','Repeated thin zigzag ribbons crossing at unequal points.','Small repeated monospaced Vela appears on the strips; a light uppercase heading is aligned high at right.','Perforated paper strip edges, alternating cream facets, ink type and cross-over shadows.',
    '67 Unfold: many narrow patterned strips viewed from above, dense weaving low on the page and a quiet high caption replace one broad side-on accordion with one huge letter per panel.',
    p('M0 446L375 616L735 500L1065 710L1440 548V900H0Z','#623d50','none','opacity=".19" transform="translate(18 18)"')+strips+
    repeat(96,i=>e(14+i*16,419+(i<24?i*5.5:i<48?230-i*2:i<70?i*4-155:650-i*2),2,2,'#9b6c72'))+
    t('M I R A',1002,135,32,'#f0dfc5','font-weight="300"')+l(1006,168,1350,168,'#d9bdb1',1)+
    m('ONE LINE BECOMES MANY.',1006,195,10,'#e9cfc2','letter-spacing="1"')+dust(81,1600,'#543340',[0,0,1440,900],.14));
}

// 82. Close friendly face is a hand-assembled mosaic, not a centered robot icon.
{
  let mosaic='';const rows=['00000111111111111111111','00011111111111111111111','00111112222222222221111','01111222222222222222111','11112223333222233332211','11112223333222233332211','11112223333222233332211','11112222222222222222211','11112222222222222222211','11112222244444444222211','11112222224444442222211','01111222222222222222111','00111112222222222221111','00011111111111111111111'];
  const colors=['none','#51416d','#aa89c0','#211e32','#f1c48c'];rows.forEach((row,j)=>[...row].forEach((c,i)=>{if(c!=='0')mosaic+=r(650+i*45,93+j*45,43,43,colors[+c],`rx="${c==='3'?3:0}"`);}));
  add(82,'Friendly machine','Mosaic companion','A giant cropped violet mosaic face smiles from the right edge while its spare antenna and cheek pixels float separately in the quiet left field.','#d4d2b8','light',['pixels'],
    'Very close frontal face hard-cropped at the right edge and low enough to feel oversized; spare components float in left negative space.','Asymmetrical stepped pixel face with square eyes and a warm angular smile.','Small stacked serif Or / bit at far-left, physically tiny against the huge face.','Uneven tile-like pixels, small grout gaps, two warm-color accents and matte ceramic colors.',
    '109 Small building blocks: a readable two-dimensional face with irregular stepped outline, ceramic tile color and tiny left serif rather than a dense three-dimensional cube losing blocks.',
    mosaic+r(842,0,44,91,'#51416d')+r(798,0,130,36,'#e6b46e')+
    r(370,255,44,44,'#51416d')+r(414,300,44,44,'#a789be')+r(280,559,44,44,'#f1c48c')+
    r(325,604,44,44,'#51416d')+r(414,649,44,44,'#a789be')+r(459,604,44,44,'#51416d')+
    s('Or',83,246,73,'#40364e')+s('bit',83,321,73,'#40364e')+m('A FAMILIAR FACE.',87,378,10,'#695f69','letter-spacing="2"')+
    l(1310,182,1398,182,'#cdb8db',3)+r(989,274,15,15,'#e9d8f2')+r(1304,274,15,15,'#e9d8f2')+
    dust(82,1400,'#615c52',[0,0,1440,900],.09));
}

// 83. Books touch every edge, the universe is only a sliver of air between them.
{
  const widths=[141,223,83,182,43,104,231,123,170,161];const cols=['#3d353b','#b7a58f','#66475e','#d2c2a4','#1b172b','#ad7769','#788176','#c4b294','#4c4653','#a18a7c'];let x=-46,spines='';
  widths.forEach((w,i)=>{const h=[960,870,990,930,1080,960,1010,875,980,1000][i],y=900-h;spines+=r(x+8,y+7,w,h,'#191521')+r(x,y,w-5,h,cols[i]);if(i!==4){spines+=l(x+9,y+8,x+9,900,'#eee0c4',1,'opacity=".25"')+l(x+w-13,y+10,x+w-13,900,'#241c29',4,'opacity=".4"')+m(i%2?'M I R A':'VELA',x+w/2,120,Math.min(w/5,31),i%2?'#4b3b47':'#e0c8ac',`transform="rotate(90 ${x+w/2} 120)" letter-spacing="5"`)+l(x+8,747,x+w-17,747,i%2?'#635449':'#bca589',2)+l(x+8,760,x+w-17,760,i%2?'#635449':'#bca589',2);}else{spines+=r(x,0,w-5,900,'#171226')+dust(834,125,'#cda9ff',[x,0,w-5,900],.9)+p(`M${x} 685L${x+w-5} 322V498L${x} 861Z`,'#776095','none','opacity=".5"');}x+=w;});
  add(83,'The spines','Shelf close-up','Unequal worn book spines fill the screen vertically; a single narrow gap between them exposes violet stars.','#3c313d','dark',['cosmos'],
    'Extreme straight-on shelf crop, books taller than screen and of wildly different widths; almost no neutral background.','Dense vertical material bars interrupted by one thin dark luminous slit.','Rotated stamped Vela words use several scales along different spines, with no detached heading.','Scuffed cloth, foil rules, discolored cream boards and cosmic pinpricks in the gap.',
    '72 Between the stripes: unequal solid textured book bodies and rotated spine lettering at macro scale replace regular optical hairlines, ivory skyline knockout and an empty left gutter.',
    spines+repeat(45,i=>l((i*193)%1440,(i*157)%900,(i*193)%1440+23,(i*157)%900+3,'#e4cfb5',1,'opacity=".19"')));
}

// 84. Interlaced letter textile flows from two opposite corners.
{
  let cloth='';for(let row=-2;row<17;row++)for(let col=-2;col<25;col++){const x=col*69+Math.sin(row*.63)*42,y=row*69+Math.sin(col*.29)*48;const color=(row+col)%2===0?'#4e3c58':'#dbbead';cloth+=p(`M${x} ${y}l68 ${Math.cos(col*.29)*13}v65l-68 ${-Math.cos(col*.29)*13}Z`,color)+t('VELA'[(col+row+30)%4],x+34,y+52,48,(row+col)%2===0?'#d6b69e':'#72526c','text-anchor="middle" font-weight="700"');}
  add(84,'Woven alphabet','Jacquard type','Warped plum and peach letter tiles interlace into a corner-to-corner textile field, interrupted by one smooth oval caption island.','#bc8f86','dark',['pixels'],
    'Overhead macro textile crosses diagonally between top-left and bottom-right; one pale smooth island cuts into the weave low-right.','Warped checker mass of interlaced letter squares with scalloped diagonal edges.','Repeated single letters become the weave; one unwarped italic Vela in a plain light island.','Jacquard-like alternating over-under fields, hairline warp threads and peach-plum woven ink.',
    '120 The weight of an idea: a letter-bearing diagonal fabric mass with an unwarped caption island and alternating block fills, versus a gravitationally sunken line grid and tiny upper title.',
    `<defs><clipPath id="cloth">${p('M-60-80H837Q735 162 1054 312T1530 901H639Q668 638 360 497T-60-80Z','#fff')}</clipPath></defs>`+
    g(cloth,'translate(0 0)','clip-path="url(#cloth)"')+repeat(140,i=>p(`M${i*13-150} -30Q${i*13-250} 400 ${i*13+140} 930`,'none','#eed0b4','stroke-width=".6" opacity=".21"'))+
    e(1034,674,189,92,'#ead9bf','transform="rotate(-12 1034 674)"')+s('Vela',1043,703,80,'#5b405c','text-anchor="middle" font-style="italic"')+
    dust(84,2400,'#382c3c',[0,0,1440,900],.13));
}

// 85. Original continuous neon handwriting, luminous tubing and exposed cable.
{
  const script='M270 505L352 301L371 471L472 294L462 488M530 467L568 367M580 331L585 320M610 470L643 374L626 448Q681 350 716 396M814 389C724 344 698 493 782 472C808 465 826 415 837 372L803 471Q827 488 855 455';
  let bricks='';for(let row=0;row<8;row++)for(let col=0;col<7;col++){const x=col*240-(row%2)*120;bricks+=r(x+3,row*122+4,231,113,row%3===0?'#26202c':'#211e27','rx="3" stroke="#15141c" stroke-width="3"');}
  add(85,'Afterglow sign','Hand-bent light','A freehand lavender neon Vela glows against dark brick, with an exposed black cable winding far into the right-hand emptiness.','#201c27','dark',['pixels'],
    'Slightly low frontal view of a neon word left of center on a full-bleed brick wall; cable snakes across the broad quiet right side.','Continuous loose handwritten light stroke and a long black wandering cable.','Original hand-drawn script is the main artwork; small lower-left sans label remains secondary.','Layered translucent luminous tubes, white-hot core, brick mortar and visible mounting clips.',
    '85 is nearest to 97 Liquid language: handmade thin luminous script suspended mid-wall, rigid brick texture and a trailing cable replace heavy pooled chrome letter masses along the floor.',
    bricks+p('M978 451C1096 465 1055 636 1223 608S1289 758 1463 742','none','#080b12','stroke-width="9"')+
    p('M978 451C1096 465 1055 636 1223 608S1289 758 1463 742','none','#37303c','stroke-width="2"')+
    [[321,323],[428,429],[554,423],[637,280],[729,422],[890,349]].map(([x,y])=>r(x-10,y-7,20,14,'#5d5663','rx="4"')+e(x,y,3,3,'#15131d')).join('')+
    p(script,'none','#9463d0','stroke-width="44" stroke-linecap="round" stroke-linejoin="round" opacity=".05"')+
    p(script,'none','#ad82e1','stroke-width="24" stroke-linecap="round" stroke-linejoin="round" opacity=".13"')+
    p(script,'none','#bb93ef','stroke-width="11" stroke-linecap="round" stroke-linejoin="round"')+
    p(script,'none','#f5ddff','stroke-width="4" stroke-linecap="round" stroke-linejoin="round"')+
    m('VELA',85,776,14,'#a79ab5','letter-spacing="5"')+l(85,800,263,800,'#64576e')+dust(85,1500,'#c3a2de',[0,0,1440,900],.07));
}

// 86. Casual ink sketch is contained on an oblique physical page.
{
  let notebook=r(0,0,651,695,'#282632','rx="7"')+r(23,12,614,669,'#e8dec5')+r(43,12,15,669,'#d6c9ae')+
    repeat(18,i=>l(68,102+i*29,612,102+i*29,'#a9a9a0',.6,'opacity=".45"'))+
    p('M636 559L527 679H636Z','#d3c3a5')+p('M636 559Q555 560 527 679L636 659Z','#f8ecd3')+
    repeat(12,i=>e(22,43+i*53,14,8,'none','stroke="#5e5454" stroke-width="6"')+e(22,43+i*53,14,8,'none','stroke="#bfb3a2" stroke-width="2"'))+
    s('Vela',56,79,45,'#3b3742')+m('NOTES ON THE UNMADE',505,47,8,'#716657','text-anchor="end"')+
    p('M166 331C202 245 387 211 480 287S422 458 294 437S139 385 166 331','none','#61536c','stroke-width="2.3"')+
    p('M139 393C235 347 408 326 502 349M255 202C286 270 318 403 350 496','none','#776279','stroke-width="1.3"')+
    e(393,316,42,42,'none','stroke="#61536c" stroke-width="2"')+repeat(11,i=>l(364+i*5,285,349+i*6,343,'#61536c',.8))+
    p('M227 259L239 280L260 284L242 298L239 320L227 300L204 296L221 281Z','none','#61536c','stroke-width="2"')+
    m('start here',126,461,14,'#61536c','transform="rotate(-9 126 461)"')+p('M210 453Q163 440 164 394','none','#61536c','stroke-width="1.2"')+
    m('leave room for a surprise.',144,559,15,'#61536c','transform="rotate(2 144 559)"');
  add(86,'Notes from Vela','Pocket sketchbook','A tilted spiral notebook carries a loose pencil celestial sketch and a lifted corner, resting on a dusty green field.','#6a786f','dark',['cosmos'],
    'Overhead notebook turned twenty degrees and cropped at the bottom; a small unbound caption sits high-right in open green table space.','Oblique rectangle with protruding spiral loops and a triangular lifted corner.','Vela masthead is printed directly on the page; casual monospaced handwritten notes stay within the drawing.','Cream ruled paper, steel spiral, rough graphite-like ink and a hard angular page shadow.',
    '18 Uncharted: an intimate single ruled notebook with spiral binding, casual mark-making, oblique crop and curled corner replaces a wide unfolded map with measured grids and coordinates.',
    g(r(10,17,654,696,'#233b31','rx="8" opacity=".25"'),'translate(310 -29) rotate(20 325 348)')+
    g(notebook,'translate(310 -29) rotate(20 325 348)')+m('ROOM TO WONDER.',1065,157,12,'#d7decb','letter-spacing="2"')+
    l(1065,185,1340,185,'#acb8a6')+dust(86,2300,'#283d32',[0,0,1440,900],.1));
}

// 87. The crescent consists only of words, with an intentionally open inner edge.
{
  const fragments=['wonder','make','begin','again','beyond','room','think','little','light','build','space','ideas'];let poem='';
  for(let row=0;row<35;row++){const y=63+row*23;const dy=(y-450)/420;const outer=Math.sqrt(Math.max(0,1-dy*dy))*475;const inner=Math.sqrt(Math.max(0,1-((y-415)/350)**2))*440;let from=600-outer,to=600-inner+172;if(Math.abs(y-415)>350)to=600+outer*.1;for(let x=from;x<to;x+=61){poem+=m(fragments[(row+Math.round(x/61)+30)%fragments.length],x,y,9+(row%3),'#aaa0bd',`opacity="${.45+(x-from)/Math.max(1,to-from)*.5}"`);}}
  add(87,'Word gravity','Concrete poetry','Tiny words gather into a large open crescent on the left, facing an enormous narrow vertical Vela at the opposite edge.','#26232d','dark',['cosmos'],
    'Flat typographic field: open crescent occupies the left two thirds, a thin vertical masthead hangs at the far-right edge.','A porous bowed crescent made solely of uneven text rows, no disc or planet.','Hundreds of tiny horizontal words oppose one very tall narrow rotated uppercase heading.','Typeset concrete poetry, varied ink density and light paper-like speckle on plum charcoal.',
    '25 Around the idea: an open irregular cloud of horizontal micro-words, no circular baseline or planet, off-center split-scale typography and dry print texture replace a closed radial word vela.',
    poem+t('VELA',1304,798,166,'#dad1de','font-weight="200" letter-spacing="10" transform="rotate(-90 1304 798)"')+
    m('THOUGHTS HAVE A WAY OF GATHERING.',729,748,11,'#93889f','letter-spacing="1"')+
    dust(87,1200,'#c1b3c8',[0,0,1440,900],.07));
}

// 88. A cyanotype made of three unequal leaves with extremely fine veins.
{
  let leaves='';const poses=[[393,608,-38,1.2],[917,512,24,1.8],[1170,790,70,.76]];
  poses.forEach(([x,y,angle,scale],k)=>{let leaf=p('M0 40C-147-36-148-262 0-390C150-249 156-61 0 40Z','#90b5d0','none','opacity=".12"')+p('M0 80V-385','none','#dceaf0','stroke-width="1.6"');for(let i=0;i<17;i++){const y=-10-i*21,w=120*Math.sin((i+1)/19*Math.PI);leaf+=p(`M0 ${y-17}Q${-w*.42} ${y-25} ${-w} ${y-98}M0 ${y-17}Q${w*.42} ${y-25} ${w} ${y-98}`,'none','#c1d9e5','stroke-width=".9"')+p(`M0 ${y-25}Q${-w*.25} ${y-65} ${-w*.78} ${y-117}M0 ${y-25}Q${w*.25} ${y-65} ${w*.78} ${y-117}`,'none','#a9ccdc','stroke-width=".45"');}leaves+=g(leaf,`translate(${x} ${y}) rotate(${angle}) scale(${scale})`);});
  add(88,'Blueprint botany','Sun print','Three unequal skeleton leaves overlap across deep blue cyanotype paper, their pale vein networks leaving the plant structures almost transparent.','#173b68','dark',['cosmos'],
    'Overhead sun-print specimen sheet; the largest leaf is clipped at the upper-right, a smaller leaf turns diagonally low-left and the title sits tightly in their lower gap.','Fine elongated leaf skeletons with branching veins, not enclosed plates.','Compact white Vela and two tiny botanical-print notes form one low central block.','Cyanotype blue mottling, ghost leaf silhouettes and dense pale vein-line impressions.',
    '51 Specimen bench: flat overlapping leaf imprints, no glassware or grounded workbench, close macro framing and tight low title replace a horizontal laboratory scene.',
    `<defs><radialGradient id="blue"><stop stop-color="#386187"/><stop offset="1" stop-color="#143663"/></radialGradient></defs>`+
    r(0,0,1440,900,'url(#blue)')+leaves+m('Vela',677,764,32,'#e2e9dd','letter-spacing="3"')+m('A STUDY IN LIGHT',679,791,9,'#c4d8d7','letter-spacing="2"')+
    l(679,807,898,807,'#c4d8d7',.7)+dust(88,2300,'#99bad4',[0,0,1440,900],.13)+dust(881,1500,'#061b43',[0,0,1440,900],.18));
}

// 89. Physical paper edges flow at macro scale; no generated contour diagram.
{
  const curls=[['M-110 215C78 214 291 153 346 303S359 618 544 573S712 300 901 238S1211 279 1510 74',34],['M-90 496C113 433 108 695 334 724S640 546 809 493S1069 476 1153 300S1296 82 1500 122',39],['M-80 129C72-6 277 41 245 187S54 388 178 473S419 414 426 272S558 49 731-53',31],['M646-59C799 61 715 267 587 280S404 440 535 488S778 374 899 421S954 695 1179 654S1398 411 1502 431',32]];
  let quills='';for(const [d,w] of curls){quills+=p(d,'none','#4f4944',`stroke-width="${w}" transform="translate(13 20)" opacity=".24"`)+p(d,'none','#c8bfa6',`stroke-width="${w}"`)+p(d,'none','#eee5cf',`stroke-width="${w-5}"`)+p(d,'none','#9b917e','stroke-width="1.5" transform="translate(-10 -5)"');}
  add(89,'Paper currents','Quilled macro','Broad cream paper strips curl into an open current, their deep shadows forming channels above a low debossed Vela strip.','#d9d3c1','light',['pixels'],
    'Very close overhead macro; continuous quilling curves enter and leave all four edges, with one nearly horizontal caption strip across the bottom.','Several thick open S-curls and spiraling loops at unequal scales.','Large widely spaced debossed title runs on a separate low cream band, no labels among the curls.','Raised cut paper edges, hard inner shadows, ivory top-edge highlights and a tactile quiet ground.',
    '26 Unknown terrain: a few wide raised physical paper curls with strong shadows and a low title band replace many thin flat nested topographic contours around one mass.',
    quills+p('M-30 791Q654 696 1480 762V934H-30Z','#b4ac99','none','transform="translate(0 10)"')+
    p('M-30 791Q654 696 1480 762V934H-30Z','#e9dfc7')+t('M I R A',370,861,87,'#a89d87','font-weight="300"')+
    t('M I R A',372,864,87,'#fff4d9','font-weight="300"')+t('M I R A',371,862,87,'#d6ccb5','font-weight="300"')+
    dust(89,2400,'#7f735f',[0,0,1440,900],.1));
}

// 90. The letter extrusion is the architecture, viewed from above at a hard angle.
{
  const word=t('VELA',0,0,321,'#e7e0c9','font-weight="900" letter-spacing="-25"');
  let sides='';for(let i=85;i>=1;i-=2)sides+=g(word.replaceAll('#e7e0c9',i%5?'#79766f':'#79727c'),`translate(${i*.78} ${i})`);
  add(90,'Built with letters','Typographic architecture','Huge extruded block letters rise from a pale ground in an overhead oblique view, their deep violet shadows crossing the cropped foreground.','#cec5b3','light',['city'],
    'High oblique camera looking down at a massive diagonal line of extruded letter buildings; nearest feet are cropped along the bottom edge.','Thick orthogonal word as a single staggered architectural mass.','The physical letters themselves say Vela; a very small upper note supplies scale.','Cream concrete top planes, layered gray extrusion sides, chiseled edges and long violet cast shadows.',
    '44 Roofline commons: the architecture is literally extruded letterforms, with no rooftop rooms, orthographic city detail or separate giant lettering behind the scene; material and framing are architectural macro.',
    p('M75 724L388 406L1268 505L1590 900H189Z','#605566','none','opacity=".37"')+
    g(sides+word,'matrix(1 -.28 .33 .79 65 691)')+
    m('M I R A',93,101,15,'#5b5261','letter-spacing="3"')+m('THOUGHT, GIVEN VOLUME.',93,132,10,'#6b6267','letter-spacing="2"')+
    repeat(19,i=>l(103+i*69,814-i*18,149+i*69,881-i*18,'#2f293e',1,'opacity=".13"'))+
    dust(90,2200,'#6d6258',[0,0,1440,900],.13));
}

// 91. A quiet encounter on a cropped checkerboard, no busy chess set.
{
  let floor='';for(let row=0;row<7;row++)for(let col=-2;col<10;col++){const x=col*211+row*37,y=188+row*122;floor+=p(`M${x} ${y}h211l37 122h-211Z`,(row+col)%2===0?'#d1c7b0':'#9f9a91');}
  const knight=p('M0 0L-66-10L-85-51L-63-96L-68-142L-26-181L1-223L34-209L65-133L94-80L65-54L19-73L-5-45L37 7L35 39H-50L-56 16Z','#624670','#352c43','stroke-width="3"')+
    p('M-18-177L5-191L23-163L0-144Z','#a184ad')+e(24,-129,5,5,'#e4ccbf')+p('M-57 40H49L66 63H-77Z','#503a60')+
    p('M-79 64H68V87H-79Z','#795a83')+l(-73,70,61,70,'#bfa0ba',2);
  const pawn=e(0,-107,35,35,'#ece1c9','stroke="#9c9788" stroke-width="2"')+p('M-18-74Q-8-12-46 14H46Q8-12 18-74Z','#e5dac3','#aaa08d','stroke-width="2"')+e(0,24,66,20,'#e8dcc1','stroke="#9b9381" stroke-width="2"')+e(0,20,56,13,'#f2e8d0');
  add(91,'The next move','Quiet strategy','One tall violet knight in the lower-left faces a small ivory pawn near the opposite corner of an otherwise empty checkerboard.','#bdb7a9','light',['pixels'],
    'Low oblique floor view with two opposed chess pieces in near-left and far-right corners; blank horizontal title sits between them near the horizon.','Angular horse profile balanced against a distant round-headed pawn.','Small dark serif Vela sits low and off-center at the board horizon, no notation or game state.','Matte carved pieces, pale checker floor and long soft polygon shadows.',
    '70 Balanced thought: grounded volumetric figurines in opposing depth corners with a low serif horizon, versus a suspended flat geometric mobile and tall left vertical type.',
    floor+p('M0 171H1440V203H0Z','#c8c0b0')+p('M91 803L575 900H739L198 749Z','#5d5063','none','opacity=".2"')+
    g(knight,'translate(186 733) scale(1.49)')+p('M1085 377L1340 464L1411 451L1150 357Z','#6a645f','none','opacity=".25"')+
    g(pawn,'translate(1122 372) scale(.69)')+s('Vela',590,154,53,'#48404e')+m('A LITTLE ROOM TO THINK.',591,180,9,'#6b6268','letter-spacing="1"')+
    dust(91,1600,'#6c6257',[0,0,1440,900],.1));
}

// 92. Faceted glass and pixel sand, vertically opposed words.
{
  let sand='';for(let i=0;i<360;i++){const x=665+(i*73%190),base=745-Math.max(0,1-Math.abs(x-760)/98)*145,y=base+(i*37%73);sand+=r(x,y,3+(i%3),3+(i%3),i%4?'#c299d4':'#efc8a5');}
  add(92,'Time in grains','Faceted glass','Two polygonal glass cones meet at a narrow waist, with pixel sand falling toward an uneven violet pile between opposing vertical Vela labels.','#191c28','dark',['pixels'],
    'Close frontal hourglass offset to the right; very narrow vertical words anchor opposite page sides at unequal heights.','Two clear triangular cones meeting tip-to-tip, resting between broad thin plates.','Vela rises vertically on the left in large thin sans; a smaller vertical phrase descends at right.','Faceted transparent glass, silver lip highlights and individual square sand grains.',
    '92 differs from 130 The long now: upright transparent double-cone object and falling grains, frontal camera and vertical opposing type replace a grounded wedge casting a diagonal sundial shadow.',
    `<defs><linearGradient id="glass" x1="0" x2="1"><stop stop-color="#b0bbd5" stop-opacity=".23"/><stop offset=".35" stop-color="#676f9a" stop-opacity=".03"/><stop offset=".8" stop-color="#aa88c9" stop-opacity=".12"/><stop offset="1" stop-color="#c3c5d2" stop-opacity=".25"/></linearGradient></defs>`+
    p('M528 137H982L778 441V477L982 801H528L739 477V441Z','url(#glass)','#8791ab','stroke-width="2"')+
    p('M562 158L751 441V476L563 776M950 157L767 441V476L950 776','none','#b4aac8','stroke-width="1" opacity=".7"')+
    p('M574 207H941L776 431H741Z','#9378a6','none','opacity=".8"')+
    repeat(68,i=>r(590+i*5,215+(i*13%48),3,3,'#d8b8df','opacity=".7"'))+
    r(751,443,5,209,'#ceb1d5','opacity=".45"')+repeat(21,i=>r(750+(i*7%11),456+i*9,4,4,i%3?'#cca9de':'#f5d5bf'))+sand+
    e(755,137,255,22,'#858393')+e(755,132,255,22,'#b4a9ba')+e(755,126,246,17,'#343449')+
    e(755,805,257,24,'#626172')+e(755,795,255,22,'#a79aae')+e(755,789,245,15,'#36364b')+
    t('VELA',237,747,138,'#c5becb','font-weight="200" letter-spacing="14" transform="rotate(-90 237 747)"')+
    m('A MOMENT TO BEGIN',1209,190,14,'#8f8ca4','transform="rotate(90 1209 190)" letter-spacing="6"')+dust(92,1300,'#adb1c4',[0,0,1440,900],.09));
}

// 93. The broken vessel is incomplete and heavily cropped, gold joins keep its story.
{
  const bowl=p('M271 449C399 162 1076 175 1349 486L1266 681C1121 900 518 1047 293 744Z','#ded7bb')+
    p('M271 449C400 211 1077 206 1349 486C1129 726 540 745 271 449Z','#7b7f70')+
    p('M320 454C456 266 1042 269 1280 486C1042 654 560 661 320 454Z','#b5b6a0')+
    p('M320 454C519 319 1091 312 1280 486C1073 330 560 364 320 454Z','#595e55','none','opacity=".3"');
  add(93,'Beautiful repair','Golden join','An incomplete pale ceramic vessel is shown at extreme close range, its large irregular fragments joined by bright gold seams.','#4f5e55','dark',['saturn'],
    'Extreme close oblique view into an incomplete bowl, its bottom and right side cropped away; spare fragment lies high-left and small title high-center.','Wide irregular open vessel rim with a missing bite and sharp shard outlines.','Small letterspaced white Vela above the bowl; gold seams carry the visual focus.','Matte speckled ceramic, dark interior glaze, uneven gold joins and three-dimensional shard shadows.',
    '98 Crystal fault: a recognizable opaque hollow ceramic vessel with gold repair joins, low oblique camera and incomplete rim replace translucent polygonal crystal wedges on an airy field.',
    bowl+p('M280 451L378 408L488 486L449 620L541 811L519 913M488 486L713 437L806 566L755 699L832 900M806 566L1021 511L1102 626L1243 692M1021 511L1126 356','none','#c5a05b','stroke-width="9" stroke-linejoin="round"')+
    p('M281 451L378 408L488 486L449 620L541 811L519 913M488 486L713 437L806 566L755 699L832 900M806 566L1021 511L1102 626L1243 692M1021 511L1126 356','none','#f5d58c','stroke-width="2"')+
    p('M1006 223L1153 253L1125 356L1016 327Z','#4f5e55')+p('M180 244L289 197L338 270L271 310Z','#212e29','none','opacity=".3" transform="translate(15 16)"')+
    p('M180 244L289 197L338 270L271 310Z','#ddd5b9')+p('M180 244L271 310L262 324L174 260Z','#a6aa93')+
    t('M I R A',524,119,28,'#e4e2ce','font-weight="300"')+m('ROOM FOR THE UNFINISHED.',525,148,10,'#aebfaf','letter-spacing="2"')+
    dust(93,2600,'#4d5146',[260,210,1100,690],.15));
}

// 94. An impossible triangular staircase is outlined by discrete physical steps.
{
  let steps='';for(let i=0;i<10;i++){const x=455+i*33,y=587-i*53;steps+=p(`M${x} ${y}l70-24l33 17l-70 24Z`,i===5?'#835c9c':'#e9d5b9')+p(`M${x} ${y}l70-24v25l-70 24Z`,i===5?'#604374':'#c3a691');}
  for(let i=0;i<10;i++){const x=785+i*34,y=59+i*53;steps+=p(`M${x} ${y}l71 25l-32 18l-71-25Z`,'#f4e1c1')+p(`M${x-32} ${y+18}l71 25v26l-71-25Z`,'#a59186');}
  for(let i=0;i<13;i++){const x=1177-i*53,y=631+i*2;steps+=p(`M${x} ${y}l-52 30l-66-2l52-30Z`,'#ddc5a7')+p(`M${x-118} ${y+28}l66 2v34l-66-2Z`,'#b59a85');}
  add(94,'Impossible step','Spatial paradox','An ivory Penrose staircase climbs around a triangular void on pale peach, with a single violet step and ultrathin Vela behind it.','#e8c6b8','light',['city'],
    'High oblique overhead framing of a closed triangular stair illusion slightly right of center; huge faint title extends behind it across the page.','Thick three-sided stair loop with a sharply triangular empty center.','Huge ultrathin uppercase word is partially occluded by the staircase; a tiny low-left phrase supplies quiet balance.','Cut stone-colored stair treads, warm side facets, one violet riser and angular cast shadows.',
    '64 Terraces of water: a closed impossible triangular loop with a central void, dry physical treads, overhead angle and giant pale type replace an open diagonal garden staircase with vertical lettering.',
    t('VELA',41,579,360,'#b9948f','font-weight="100" letter-spacing="-26" opacity=".47"')+
    p('M446 648L822 46L1234 706L573 782Z','#a27479','none','opacity=".18" transform="translate(43 44)"')+
    p('M450 588L791 40L1196 641L532 726Z','#bd9d8b')+p('M615 576L797 280L991 587Z','#e8c6b8')+steps+
    m('ONE STEP OPENS ANOTHER.',74,817,12,'#805c65','letter-spacing="2"')+dust(94,1900,'#9a6b68',[0,0,1440,900],.1));
}

// 95. Four different fold stages make a rising sequence, rather than four birds.
{
  const stage1=p('M112 695L217 626L323 695L218 767Z','#ece5d0','#a9978d','stroke-width="1"')+l(112,695,323,695,'#b9a598',1,'stroke-dasharray="4 5"')+l(217,626,218,767,'#b9a598',1,'stroke-dasharray="4 5"');
  const stage2=p('M400 612L518 432L638 611Z','#e6d7bc')+p('M400 612L518 432V612Z','#c2ad9c')+l(518,432,518,612,'#f9efdb',2);
  const stage3=p('M669 464L778 279L877 462L777 526Z','#d1b6bd')+p('M778 279L778 526L727 464Z','#936e89')+p('M778 279L827 466L877 462Z','#b697aa')+p('M778 526L805 416L877 462Z','#f0d6cf');
  const stage4=p('M911 254L1066 183L1239 271L1068 257Z','#ede4ce')+p('M1068 257L1019 63L1080 182L1086 245Z','#c8b2a8')+
    p('M1080 182L1196 134L1257 144L1199 155L1086 245Z','#ded0c1')+p('M1068 257L1136 366L1090 260Z','#9c7d91')+p('M911 254L1064 280L1068 257Z','#b5a394');
  add(95,'Crease study','Fold sequence','Four unequal origami stages rise from a flat diamond to an elongated crane, ending beside a tall vertical Vela at the right.','#867f6c','dark',['pixels'],
    'Orthographic overhead sequence ascends from bottom-left to top-right in four unequal sizes, finishing at a vertical title at the outer edge.','Four disconnected silhouettes: flat diamond, peaked triangle, folded kite and long-necked crane.','Tall narrow Vela runs down the right edge; tiny crease labels sit directly beside each paper stage.','Matte folded paper, alternating face tones, dashed crease marks and short offset shadows.',
    '78 Flight in pieces: a physical four-stage transformation and rising sequence of unequal folded volumes, narrow edge title and muted table replace one flat seven-piece tangram bird and low heavy word.',
    g(stage1,'translate(6 11)','opacity=".2"')+stage1+stage2+stage3+stage4+
    l(126,792,315,792,'#bab2a0')+m('CREASE',126,815,10,'#ded6bf','letter-spacing="3"')+
    m('LIFT',408,648,10,'#ded6bf','letter-spacing="3"')+m('TURN',684,565,10,'#ded6bf','letter-spacing="3"')+
    m('OPEN',1138,388,10,'#ded6bf','letter-spacing="3"')+t('VELA',1343,89,91,'#ece3cd','font-weight="300" letter-spacing="9" transform="rotate(90 1343 89)"')+
    dust(95,2200,'#393d30',[0,0,1440,900],.12));
}

// 96. A single orthogonal crossword, the letters are the masses.
{
  let cells='';const size=148,ox=97,oy=116;const horizontal='VELA',vertical='VELA';
  horizontal.split('').forEach((char,i)=>{cells+=r(ox+i*size,oy,size-3,size-3,i===0?'#3e3241':'#e7dcc2','stroke="#5b4854" stroke-width="2"')+t(char,ox+i*size+72,oy+111,111,i===0?'#eee4cd':'#3e3241','text-anchor="middle" font-weight="700"');});
  vertical.slice(1).split('').forEach((char,i)=>{cells+=r(ox,oy+(i+1)*size,size-3,size-3,'#e7dcc2','stroke="#5b4854" stroke-width="2"')+t(char,ox+72,oy+(i+1)*size+111,111,'#3e3241','text-anchor="middle" font-weight="700"');});
  add(96,'Crossing words','Letter crossword','Two giant Vela words share their opening M to form a strongly asymmetrical L across warm gridded paper.','#d7c9ae','light',['pixels'],
    'Flat desk-paper view: four large letter cells stretch across the high-left, while another four descend down the left edge; the open lower-right is almost empty.','Giant thick L made from seven square letter blocks.','Bold shared-letter crossword and a tiny low-right dot-matrix caption; no separate masthead.','Soft black block printing on warm graph paper, outlined cells and subtle rubbed ink.',
    '80 Letters to get lost in: seven clean typographic square cells form one large open L with a shared M; no maze passages, no stepped modular cluster and no opposing serif title.',
    repeat(37,i=>l(0,i*26,1440,i*26,'#a99b85',.5,'opacity=".23"'))+repeat(58,i=>l(i*26,0,i*26,900,'#a99b85',.5,'opacity=".23"'))+
    cells+m('WORDS FIND EACH OTHER.',853,742,12,'#6e5865','letter-spacing="3"')+m('VELA',853,776,20,'#3e3241','letter-spacing="6"')+
    repeat(75,i=>r(857+(i%25)*8,811+Math.floor(i/25)*8,i%3===0?3:2,2,'#7d6971'))+dust(96,2700,'#776653',[0,0,1440,900],.1));
}

// 97. The word is viscous chrome; reflections reach upward into empty space.
{
  const reflection=t('Vela',130,0,324,'url(#reflection)','font-weight="900" letter-spacing="-23" stroke="url(#reflection)" stroke-width="16"');
  add(97,'Liquid language','Chrome pool','A heavy chrome Vela pools against the bottom edge while impossibly long silver letter reflections rise into a deep quiet field.','#10151d','dark',['saturn'],
    'Low macro view across a dark reflecting plane: liquid word touches the bottom edge; long attenuated mirror forms rise far into the empty upper field.','Heavy soft-edged horizontal typographic puddle with long narrow upward reflections.','One large molten mixed-case Vela, no secondary slogan; elongated reflected letterforms dissolve above.','Alternating silver-black chrome bands, white specular contours, pooled meniscus and dark bluish reflections.',
    '85 Afterglow sign: heavy volumetric pooled chrome at the bottom with vertical reflections and large empty upper field replace thin hand-bent mid-wall neon script, bricks and a trailing cable.',
    `<defs><linearGradient id="chrome" x1="0" y1="0" x2="0" y2="1"><stop stop-color="#2d3c50"/><stop offset=".17" stop-color="#e0e2e3"/><stop offset=".32" stop-color="#5f6d7d"/><stop offset=".43" stop-color="#111b2a"/><stop offset=".51" stop-color="#f3eaf3"/><stop offset=".55" stop-color="#858694"/><stop offset=".68" stop-color="#202636"/><stop offset=".88" stop-color="#b3bdc9"/><stop offset="1" stop-color="#f7e3db"/></linearGradient><linearGradient id="reflection" x1="0" y1="0" x2="0" y2="1"><stop stop-color="#bec6d5" stop-opacity="0"/><stop offset=".65" stop-color="#9fadc0" stop-opacity=".13"/><stop offset="1" stop-color="#c3bdd5" stop-opacity=".4"/></linearGradient><linearGradient id="pool" x1="0" y1="0" x2="0" y2="1"><stop stop-color="#d5d3dc"/><stop offset=".15" stop-color="#131b29"/><stop offset=".6" stop-color="#3c4154"/><stop offset="1" stop-color="#a0a3b0"/></linearGradient></defs>`+
    g(reflection,'matrix(1 0 .12 1.64 84 654)')+
    p('M-60 863C87 827 90 789 203 813S426 777 575 818S798 787 923 830S1250 774 1484 848V951H-60Z','url(#pool)','#768093','stroke-width="2"')+
    t('Vela',130,861,324,'url(#chrome)','font-weight="900" letter-spacing="-23" stroke="url(#chrome)" stroke-width="17" stroke-linejoin="round"')+
    p('M146 862C384 913 502 869 687 888S1090 842 1331 882','none','#f2dfe6','stroke-width="2" opacity=".7"')+
    p('M163 711Q173 661 232 652M397 704Q429 675 478 684M744 674Q775 650 823 663','none','#fff0f1','stroke-width="4" stroke-linecap="round" opacity=".75"')+
    e(1200,830,31,9,'#7c8398')+e(1234,795,7,4,'#c1bdcb')+dust(97,1000,'#a5a9ba',[0,0,1440,900],.05));
}

export default concepts;
