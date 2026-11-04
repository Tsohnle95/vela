const params = new URLSearchParams(location.search);
const splash = document.querySelector('#splash');
const workspace = document.querySelector('#workspace');
const artwork = document.querySelector('#artwork');
const holdButton = document.querySelector('#hold');
const label = document.querySelector('#loading-label');
const notes = document.querySelector('#design-notes');
const notesToggle = document.querySelector('#notes-toggle');
let held = params.has('still');
let timer;
let generation = 0;
if (params.has('immersive')) document.body.classList.add('immersive');

const icon = name => `<svg viewBox="0 0 24 24" aria-hidden="true">${{
  folder:'<path d="M3 6h7l2 3h9v11H3z"/>',
  files:'<path d="M6 3h9l4 4v14H6z M3 6v15 M14 3v5h5"/>',
  chat:'<path d="M3 4h18v13H8l-5 4z M7 9h10 M7 13h7"/>',
  terminal:'<path d="m5 6 6 6-6 6 M13 18h7"/>',
  gear:'<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="2"/>'
}[name]}</svg>`;
workspace.innerHTML = `<header class="titlebar"><span class="traffic"><i></i><i></i><i></i></span><strong>✦ &nbsp; Vela</strong><span class="titlebar-right">⌘ &nbsp; ◧</span></header><div class="workarea"><aside class="rail" aria-label="Activity rail"><span class="active">${icon('files')}</span><span>${icon('chat')}</span><span>${icon('terminal')}</span><span class="bottom">${icon('gear')}</span></aside><aside class="explorer"><h2>WORKSPACES</h2><p class="empty-note">Your workspace will appear here.</p><button class="open-folder" data-choose>${icon('folder')}<span>Choose workspace</span></button><div class="workspace-choice" id="workspace-choice"></div></aside><div class="editor"><div class="empty-center"><div class="empty-star">✦</div><h1 id="empty-title">Space for your next idea.</h1><p id="empty-description">Choose a workspace to begin.<br>Your files and agents will follow.</p><button class="quiet-choose" data-choose>Choose workspace</button></div></div><aside class="agent"><div>Agent <span style="float:right;color:#5d5469">＋</span></div><div class="agent-space">Waiting for a workspace</div><div class="composer" aria-disabled="true">Open a workspace to start working.<small>Agent &nbsp; · &nbsp; No workspace selected</small></div></aside></div><footer class="statusbar"><span id="workspace-status">○ No workspace selected</span><span>Vela &nbsp; / &nbsp; Experimental preview</span></footer>`;

function paintHold() {
  holdButton.setAttribute('aria-pressed', String(held));
  holdButton.textContent = held ? 'Resume 3s launch' : 'Hold artwork';
  label.textContent = held ? 'Artwork held · resume to launch' : 'Opening Vela';
  for (const anchor of document.querySelectorAll('#previous, #next')) {
    const url = new URL(anchor.href);
    if (held) url.searchParams.set('still', '1');
    else url.searchParams.delete('still');
    if (params.has('immersive')) url.searchParams.set('immersive', '1');
    anchor.href = url.href;
  }
}
function enterWorkspace() {
  splash.hidden = true;
  workspace.hidden = false;
  document.body.dataset.phase = 'workspace';
  document.body.dataset.workspaceAt = String(performance.now());
}
async function launch() {
  clearTimeout(timer);
  const current = ++generation;
  delete document.body.dataset.artworkAt;
  delete document.body.dataset.workspaceAt;
  workspace.hidden = true;
  splash.hidden = false;
  workspace.classList.remove('workspace-opened');
  document.querySelector('#workspace-choice').textContent = '';
  document.querySelector('#workspace-status').textContent = '○ No workspace selected';
  document.querySelector('#empty-title').textContent = 'Space for your next idea.';
  document.querySelector('#empty-description').innerHTML = 'Choose a workspace to begin.<br>Your files and agents will follow.';
  picker.value = '';
  document.body.dataset.phase = 'artwork';
  paintHold();
  // Start the three seconds only after the local artwork is decoded and painted.
  await artwork.decode().catch(() => {});
  await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  if (generation !== current || held) return;
  document.body.dataset.artworkAt = String(performance.now());
  timer = setTimeout(enterWorkspace, 3000);
}
holdButton.addEventListener('click', () => {
  held = !held;
  void launch();
});
document.querySelector('#replay').addEventListener('click', () => {
  if (notes) notes.hidden = true;
  notesToggle?.setAttribute('aria-expanded', 'false');
  held = false;
  void launch();
});
notesToggle?.addEventListener('click', () => {
  notes.hidden = !notes.hidden;
  notesToggle.setAttribute('aria-expanded', String(!notes.hidden));
  if (!notes.hidden) { held = true; void launch(); }
});
const picker = document.querySelector('#folder-picker');
document.querySelectorAll('[data-choose]').forEach(button => button.addEventListener('click', () => picker.click()));
picker.addEventListener('change', () => {
  const file = picker.files[0];
  if (!file) return;
  const selectedFolder = file.webkitRelativePath.split('/')[0];
  document.querySelector('#workspace-choice').textContent = `▾ ${selectedFolder}`;
  document.querySelector('#workspace-status').textContent = `○ ${selectedFolder} · Preview only`;
  document.querySelector('#empty-title').textContent = selectedFolder;
  document.querySelector('#empty-description').textContent = 'Workspace selected in the prototype. Files stay on your device.';
  workspace.classList.add('workspace-opened');
});
void launch();
const destination = offset => document.querySelector(offset < 0 ? '#previous' : '#next').href;
document.addEventListener('keydown', event => {
  if (event.target instanceof HTMLButtonElement || event.target instanceof HTMLInputElement) return;
  if (event.key === 'ArrowRight') location.href = destination(1);
  if (event.key === 'ArrowLeft') location.href = destination(-1);
  if (event.key.toLowerCase() === 'r') { held = false; void launch(); }
  if (event.code === 'Space') { event.preventDefault(); held = !held; void launch(); }
  if (event.key === 'Escape') {
    if (notes) notes.hidden = true;
    notesToggle?.setAttribute('aria-expanded', 'false');
  }
});
