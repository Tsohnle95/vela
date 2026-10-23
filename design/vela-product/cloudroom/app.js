(() => {
  const directions = window.velaDirections.sort((a, b) => a.id - b.id);
  const { cta, repository } = window.VelaDesign;
  const site = document.getElementById('site');
  const select = document.getElementById('design');
  const gallery = document.getElementById('gallery');
  const controls = document.querySelector('.design-controls');
  const restore = document.getElementById('restore-controls');
  const params = new URLSearchParams(location.search);
  const requested = Number(params.get('design'));
  let current = Number.isInteger(requested) && requested >= 1 && requested <= directions.length ? requested : 1;
  let clean = params.get('clean') === '1';
  select.innerHTML = directions.map(d => `<option value="${d.id}">${String(d.id).padStart(2, '0')} — ${d.name}</option>`).join('');
  const header = () => `<header class="site-header"><a class="brand" href="#main" aria-label="Vela home"><img src="../assets/icon.png" width="40" height="40" alt=""><span>Vela</span></a><nav aria-label="Main navigation"><a href="#workspace">Workspace</a><a href="${repository}#setup" target="_blank" rel="noopener">Setup</a><a href="${repository}" target="_blank" rel="noopener">GitHub ↗</a></nav>${cta('Get Vela')}</header>`;
  const footer = () => `<footer class="site-footer"><a class="brand" href="#main">Vela</a><nav aria-label="Footer navigation"><a href="${repository}" target="_blank" rel="noopener">GitHub ↗</a><a href="${repository}#setup" target="_blank" rel="noopener">Setup guide ↗</a></nav></footer>`;
  function setPresentation(hidden, writeUrl = true) {
    clean = hidden;
    controls.hidden = hidden;
    restore.hidden = !hidden;
    if (writeUrl) {
      const url = new URL(location.href);
      if (hidden) url.searchParams.set('clean', '1'); else url.searchParams.delete('clean');
      history.replaceState(null, '', url);
    }
  }
  function fillGallery() {
    document.getElementById('gallery-grid').innerHTML = directions.map(d => `<a class="gallery-card" href="?design=${d.id}" data-design="${d.id}" ${current === d.id ? 'aria-current="page"' : ''}><img src="previews/${String(d.id).padStart(2, '0')}.jpg" width="500" height="347" loading="lazy" alt=""><h3>${d.name}</h3></a>`).join('');
  }
  function render(push = false, focus = false) {
    const direction = directions.find(d => d.id === current);
    document.body.className = `d${current}`;
    site.innerHTML = `${header()}<main id="main" tabindex="-1">${direction.body()}</main>${footer()}`;
    if (!site.querySelector('#workspace')) site.querySelector('.app-shot')?.setAttribute('id', 'workspace');
    select.value = String(current);
    document.title = `Vela — ${direction.name}`;
    const url = new URL(location.href);
    url.searchParams.set('design', String(current));
    url.hash = '';
    if (push) history.pushState(null, '', url); else history.replaceState(null, '', url);
    fillGallery();
    setPresentation(clean, false);
    window.scrollTo(0, 0);
    if (focus) document.getElementById('main').focus({ preventScroll: true });
  }
  function go(id, focus = false) {
    current = (id - 1 + directions.length) % directions.length + 1;
    render(true, focus);
  }
  document.getElementById('previous').addEventListener('click', () => go(current - 1));
  document.getElementById('next').addEventListener('click', () => go(current + 1));
  select.addEventListener('change', () => go(Number(select.value)));
  document.getElementById('hide-controls').addEventListener('click', () => setPresentation(true));
  restore.addEventListener('click', () => setPresentation(false));
  document.getElementById('browse').addEventListener('click', () => gallery.showModal());
  document.getElementById('close-gallery').addEventListener('click', () => gallery.close());
  gallery.addEventListener('close', () => document.getElementById('browse').focus());
  gallery.addEventListener('click', event => {
    const link = event.target.closest('[data-design]');
    if (!link || event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
    event.preventDefault();
    gallery.close();
    go(Number(link.dataset.design), true);
  });
  document.addEventListener('keydown', event => {
    if (gallery.open || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || /^(INPUT|SELECT|TEXTAREA|BUTTON|A|SUMMARY)$/.test(event.target.tagName) || event.target.isContentEditable) return;
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();
    go(current + (event.key === 'ArrowRight' ? 1 : -1));
  });
  window.addEventListener('popstate', () => {
    const value = Number(new URLSearchParams(location.search).get('design'));
    current = Number.isInteger(value) && value >= 1 && value <= directions.length ? value : 1;
    clean = new URLSearchParams(location.search).get('clean') === '1';
    render();
  });
  render();
})();
