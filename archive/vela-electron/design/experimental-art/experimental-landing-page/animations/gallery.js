import { concepts, createStage, DURATION } from './motions.js?v=2';
const $=s=>document.querySelector(s);
const gallery=$('#gallery'),dialog=$('#inspector');
let saved;
try{saved=new Set(JSON.parse(localStorage.getItem('vela.motion-favorites')||'[]'));}catch{saved=new Set();}
let filter='all',playing=!matchMedia('(prefers-reduced-motion: reduce)').matches,speed=1,elapsed=0,last=performance.now();
let focus=null,focusElapsed=0,focusPlaying=true,launchMode=false,launchDone=false;
const cards=[];
const updateSaved=()=>{
  try{localStorage.setItem('vela.motion-favorites',JSON.stringify([...saved]));}catch{}
  $('#saved-count').textContent=saved.size;
  cards.forEach(c=>{const on=saved.has(c.info.id);c.save.setAttribute('aria-pressed',String(on));c.save.textContent=on?'◆':'◇';c.save.setAttribute('aria-label',`${on?'Unsave':'Save'} ${c.info.name}`);});
  if(focus){const on=saved.has(focus.info.id);$('#focus-save').setAttribute('aria-pressed',String(on));$('#focus-save').textContent=on?'Saved study ◆':'Save study ◇';}
};
const applyFilter=()=>{let count=0;cards.forEach(c=>{c.card.hidden=filter==='saved'?!saved.has(c.info.id):filter!=='all'&&c.info.family!==filter;if(!c.card.hidden)count++;});$('#count').textContent=`Showing ${count} of 25 motion studies`;$('#no-results').hidden=count>0;};
const observer=new IntersectionObserver(entries=>{entries.forEach(e=>{const c=cards.find(c=>c.card===e.target);if(c)c.visible=e.isIntersecting;});},{rootMargin:'100px'});
for(const info of concepts){
  const card=document.createElement('article');card.className='card';card.dataset.concept=info.id;
  card.innerHTML=`<button class="stage-button" aria-label="Inspect ${info.id}. ${info.name}"><div class="stage"></div><span class="inspect-label">Inspect ↗</span></button><div class="card-title"><span class="number">${info.id}</span><h2>${info.name}</h2><button class="bookmark" aria-pressed="false"></button></div><p>${info.description}</p><p class="rule">${info.rule}</p><div class="family">${info.family} / 3 seconds</div>`;
  gallery.append(card);
  const stage=createStage(card.querySelector('.stage'),info.id);
  const c={info,card,stage,visible:false,save:card.querySelector('.bookmark')};cards.push(c);observer.observe(card);
  card.querySelector('.stage-button').addEventListener('click',()=>open(info.id));
  c.save.addEventListener('click',()=>{saved.has(info.id)?saved.delete(info.id):saved.add(info.id);updateSaved();applyFilter();});
}
function focusUI(){
  $('#focus-play').textContent=focusPlaying?'Pause':'Play';$('#focus-play').setAttribute('aria-pressed',String(focusPlaying));
  $('#focus-stage').hidden=launchDone;$('#empty-workspace').hidden=!launchDone;
  $('#launch-state').textContent=launchDone?'Empty workspace · waiting for your choice':launchMode?'Playing the complete three-second launch':'Looping animation · one cycle is three seconds';
}
function open(id){
  focus?.stage.destroy();const info=concepts.find(c=>c.id===id)||concepts[0];
  const stage=createStage($('#focus-stage'),info.id);focus={info,stage};focusElapsed=0;focusPlaying=playing;launchMode=false;launchDone=false;
  $('#focus-number').textContent=info.id;$('#focus-title').textContent=info.name;$('#focus-family').textContent=info.family;$('#focus-description').textContent=info.description;$('#focus-rule').textContent=info.rule;
  updateSaved();focusUI();$('#scrub').value=0;$('#time').textContent='0.00 / 3.00 s';
  if(!dialog.open)dialog.showModal();const url=new URL(location.href);url.searchParams.set('concept',info.id);history.replaceState(null,'',url);
}
function close(){dialog.close();}
dialog.addEventListener('close',()=>{focus?.stage.destroy();focus=null;const url=new URL(location.href);url.searchParams.delete('concept');history.replaceState(null,'',url);});
$('#close').addEventListener('click',close);
$('#previous').addEventListener('click',()=>open(concepts[(Number(focus.info.id)+23)%25].id));
$('#next').addEventListener('click',()=>open(concepts[Number(focus.info.id)%25].id));
$('#focus-save').addEventListener('click',()=>{const id=focus.info.id;saved.has(id)?saved.delete(id):saved.add(id);updateSaved();applyFilter();});
$('#focus-play').addEventListener('click',()=>{if(launchDone){launchDone=false;launchMode=false;focusElapsed=0;}focusPlaying=!focusPlaying;focusUI();});
$('#focus-restart').addEventListener('click',()=>{focusElapsed=0;launchDone=false;focusPlaying=true;focusUI();});
$('#launch-preview').addEventListener('click',()=>{focusElapsed=0;launchMode=true;launchDone=false;focusPlaying=true;focusUI();});
$('#scrub').addEventListener('input',()=>{focusElapsed=Number($('#scrub').value);focusPlaying=false;launchMode=false;launchDone=false;focus.stage.draw(focusElapsed/DURATION);focusUI();$('#time').textContent=`${(focusElapsed/1000).toFixed(2)} / 3.00 s`;});
$('#mock-choose').addEventListener('click',()=>{$('#launch-state').textContent='Workspace selection happens in Vela. This is a visual preview only.';});
$('#play-all').addEventListener('click',()=>{playing=!playing;$('#play-all').textContent=playing?'Pause all':'Play all';$('#play-all').setAttribute('aria-pressed',String(playing));});
$('#restart-all').addEventListener('click',()=>{elapsed=0;cards.forEach(c=>c.stage.draw(0));});
$('#speed').addEventListener('change',()=>{speed=Number($('#speed').value);});
document.querySelectorAll('[data-filter]').forEach(button=>button.addEventListener('click',()=>{filter=button.dataset.filter;document.querySelectorAll('[data-filter]').forEach(b=>{b.classList.toggle('selected',b===button);b.setAttribute('aria-pressed',String(b===button));});applyFilter();}));
document.addEventListener('keydown',event=>{if(!dialog.open||event.target.matches('input,select,button'))return;if(event.key==='ArrowRight')$('#next').click();if(event.key==='ArrowLeft')$('#previous').click();if(event.key===' '){event.preventDefault();$('#focus-play').click();}});
function tick(now){const dt=now-last;last=now;if(!document.hidden){
  if(playing&&!dialog.open){elapsed+=dt*speed;cards.forEach(c=>{if(c.visible&&!c.card.hidden)c.stage.draw((elapsed%DURATION)/DURATION);});}
  if(focus&&focusPlaying&&!launchDone){focusElapsed+=dt*(launchMode?1:speed);if(launchMode&&focusElapsed>=DURATION){focusElapsed=DURATION;launchDone=true;focusPlaying=false;focus.stage.draw(1);focusUI();}else{focus.stage.draw((focusElapsed%DURATION)/DURATION);}const t=launchDone?DURATION:focusElapsed%DURATION;$('#scrub').value=t;$('#time').textContent=`${(t/1000).toFixed(2)} / 3.00 s`;}
}requestAnimationFrame(tick);}
updateSaved();applyFilter();$('#play-all').textContent=playing?'Pause all':'Play all';$('#play-all').setAttribute('aria-pressed',String(playing));
const initial=new URLSearchParams(location.search).get('concept');if(initial&&concepts.some(c=>c.id===initial))open(initial);
document.addEventListener('visibilitychange',()=>{last=performance.now();});
if(!playing)cards.forEach(c=>c.stage.draw(.65));
requestAnimationFrame(tick);
// Read-only inspection surface used by the browser verification page.
window.velaMotionReview={concepts,cards,get playing(){return playing;},get focus(){return focus;},get launchDone(){return launchDone;}};
