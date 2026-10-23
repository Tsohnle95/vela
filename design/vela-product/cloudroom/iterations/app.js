(() => {
  const all = window.velaIterations;
  const references = all.filter(d => d.reference).sort((a, b) => a.id - b.id);
  const first = all.filter(d => !d.reference && d.family === 1).sort((a, b) => a.id - b.id);
  const second = all.filter(d => !d.reference && d.family === 2).sort((a, b) => a.id - b.id);
  const designs = [...references, ...first, ...second];
  const palettes = window.velaPalettes;
  const storageKey = 'vela-release-iteration-palettes-v1';
  let choices = {};
  try {
    const stored = JSON.parse(localStorage.getItem(storageKey) || '{}');
    if (stored && typeof stored === 'object' && !Array.isArray(stored)) choices = stored;
  } catch { /* File previews can run with browser storage disabled. */ }
  const validPalette = id => palettes.some(p => p.id === id);
  const requested = new URLSearchParams(location.search);
  let current = Number(requested.get('design'));
  if (!designs.some(d => d.id === current)) current = 21;
  let clean = requested.get('clean') === '1';
  if (validPalette(requested.get('palette'))) choices[current] = requested.get('palette');
  const site = document.getElementById('site');
  const selector = document.getElementById('design');
  const colorSelector = document.getElementById('palette');
  const dialog = document.getElementById('gallery');
  const { repository, cta } = window.Iteration;
  const indexLabel = d => d.reference ? `Reference #${d.id}` : `${d.family}.${String(d.family === 1 ? d.id - 20 : d.id - 35).padStart(2, '0')}`;
  const groups = [['References', references], ['From #1 · 15 iterations', first], ['From #2 · 15 iterations', second]];
  selector.innerHTML = groups.map(([name, items]) => `<optgroup label="${name}">${items.map(d => `<option value="${d.id}">${indexLabel(d)} — ${d.name}</option>`).join('')}</optgroup>`).join('');
  colorSelector.innerHTML = [...new Set(palettes.map(p => p.group))].map(group => `<optgroup label="${group}">${palettes.filter(p => p.group === group).map(p => `<option value="${p.id}">${p.name}</option>`).join('')}</optgroup>`).join('');
  function urlFor(id, palette = choices[id] || 'mint') {
    const url = new URL(location.href);
    url.searchParams.set('design', String(id));
    url.searchParams.set('palette', validPalette(palette) ? palette : 'mint');
    url.hash = '';
    return url;
  }
  function save() {
    try { localStorage.setItem(storageKey, JSON.stringify(choices)); } catch { /* Session choices still work without storage. */ }
  }
  function applyPalette(id) {
    const palette = palettes.find(p => p.id === id) || palettes[0];
    const tokens = { '--bg': palette.bg, '--fg': palette.fg, '--accent': palette.accent, '--muted': palette.muted, '--line': palette.line, '--surface': palette.surface, '--button-text': palette.button, '--shadow': palette.shadow };
    for (const [token, value] of Object.entries(tokens)) document.body.style.setProperty(token, value);
    document.body.dataset.palette = palette.id;
    colorSelector.value = palette.id;
  }
  function gallery() {
    document.getElementById('gallery-groups').innerHTML = groups.map(([name, items]) => `<section class="gallery-family"><h3>${name}</h3><div class="gallery-family-grid">${items.map(d => `<a class="gallery-card" href="${urlFor(d.id)}" data-design="${d.id}" ${d.id === current ? 'aria-current="page"' : ''}><img src="previews/${String(d.id).padStart(2, '0')}.jpg" width="500" height="347" loading="lazy" alt=""><h3>${indexLabel(d)} — ${d.name}</h3></a>`).join('')}</div></section>`).join('');
  }
  function render(push = false, focus = false) {
    const design = designs.find(d => d.id === current);
    document.body.className = `iteration-page d${design.id} family-${design.family}`;
    site.innerHTML = `<header class="site-header"><a class="brand" href="#main" aria-label="Vela home"><img src="../../assets/icon.png" width="40" height="40" alt=""><span>Vela</span></a><nav aria-label="Main navigation"><a href="#workspace">Workspace</a><a href="${repository}#setup" target="_blank" rel="noopener">Setup</a><a href="${repository}" target="_blank" rel="noopener">GitHub ↗</a></nav>${cta('Get Vela')}</header><main id="main" tabindex="-1">${design.body()}</main><footer class="site-footer"><a class="brand" href="#main">Vela</a><nav aria-label="Footer navigation"><a href="${repository}" target="_blank" rel="noopener">GitHub ↗</a><a href="${repository}#setup" target="_blank" rel="noopener">Setup guide ↗</a></nav></footer>`;
    if (!site.querySelector('#workspace')) site.querySelector('.app-shot')?.setAttribute('id', 'workspace');
    selector.value = String(current);
    applyPalette(choices[current]);
    const url = urlFor(current, colorSelector.value);
    if (push) history.pushState(null, '', url); else history.replaceState(null, '', url);
    document.title = `Vela — ${design.reference ? 'Reference ' + current : design.name}`;
    gallery();
    setControls(clean, false);
    window.scrollTo(0, 0);
    if (focus) document.getElementById('main').focus({ preventScroll: true });
  }
  function go(id, focus = false) { if (!designs.some(d => d.id === id)) return; current = id; render(true, focus); }
  function advance(delta) { const index = designs.findIndex(d => d.id === current); go(designs[(index + delta + designs.length) % designs.length].id); }
  function setControls(hidden, update = true) {
    clean = hidden;
    document.querySelector('.iteration-controls').hidden = hidden;
    document.getElementById('restore-controls').hidden = !hidden;
    if (update) {
      const url = new URL(location.href);
      if (hidden) url.searchParams.set('clean', '1'); else url.searchParams.delete('clean');
      history.replaceState(null, '', url);
    }
  }
  selector.addEventListener('change', () => go(Number(selector.value)));
  colorSelector.addEventListener('change', () => {
    choices[current] = colorSelector.value;
    applyPalette(colorSelector.value);
    save();
    history.pushState(null, '', urlFor(current, colorSelector.value));
    gallery();
  });
  document.getElementById('previous').addEventListener('click', () => advance(-1));
  document.getElementById('next').addEventListener('click', () => advance(1));
  document.getElementById('browse').addEventListener('click', () => dialog.showModal());
  document.getElementById('close-gallery').addEventListener('click', () => dialog.close());
  dialog.addEventListener('close', () => document.getElementById('browse').focus());
  dialog.addEventListener('click', event => {
    const link = event.target.closest('[data-design]');
    if (!link || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    event.preventDefault(); dialog.close(); go(Number(link.dataset.design), true);
  });
  document.getElementById('hide-controls').addEventListener('click', () => setControls(true));
  document.getElementById('restore-controls').addEventListener('click', () => setControls(false));
  document.addEventListener('keydown', event => {
    if (dialog.open || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || /^(INPUT|SELECT|TEXTAREA|BUTTON|SUMMARY|A)$/.test(event.target.tagName) || event.target.isContentEditable) return;
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault(); advance(event.key === 'ArrowRight' ? 1 : -1);
  });
  window.addEventListener('popstate', () => {
    const params = new URLSearchParams(location.search);
    const id = Number(params.get('design'));
    current = designs.some(d => d.id === id) ? id : 21;
    if (validPalette(params.get('palette'))) choices[current] = params.get('palette');
    clean = params.get('clean') === '1'; render();
  });
  render();
})();
