(() => {
  const concepts = window.sourceConcepts;
  const site = document.getElementById('site');
  const selector = document.getElementById('version');
  const dialog = document.getElementById('collection');
  const allButton = document.getElementById('show-all');
  selector.innerHTML = concepts.map((c, i) => `<option value="${i + 1}">${String(i + 1).padStart(2, '0')} — ${c.name}</option>`).join('');
  const requested = Number(new URLSearchParams(location.search).get('version'));
  let current = Number.isInteger(requested) && requested >= 1 && requested <= concepts.length ? requested : 1;
  const header = () => `<header class="site-header"><a class="wordmark" href="../index.html?design=24" aria-label="Vela — original Source design">Vela</a><span class="header-caption">A workspace for coding agents</span><nav aria-label="Product navigation"><a href="#workspace">The workspace</a><a href="https://github.com/Tsohnle95/vela" target="_blank" rel="noopener">GitHub ↗</a></nav></header>`;
  function fillCollection() {
    document.getElementById('collection-grid').innerHTML = concepts.map((c, i) => `<a class="collection-card" href="?version=${i + 1}" data-version="${i + 1}" ${current === i + 1 ? 'aria-current="page"' : ''}><small>${c.motif}</small><img class="collection-preview" src="previews/${String(i + 1).padStart(2, '0')}.jpg" loading="lazy" width="500" height="347" alt=""><h3>${c.name}</h3><p>${c.change}</p></a>`).join('');
  }
  function render(updateHistory = false, focusHeading = false) {
    const c = concepts[current - 1];
    document.body.className = `v${current}`;
    site.innerHTML = `${header()}<main id="main" tabindex="-1">${c.body()}<section id="workspace-details" class="design-note" aria-label="About this interpretation"><p class="label">SOURCE / ${String(current).padStart(2, '0')}<br>${c.name}</p><details><summary>Design notes — the relationship to #24</summary><p><strong>Reading the original.</strong> ${c.source}</p><p><strong>New direction.</strong> ${c.change}</p><p>Local design exploration. Product images show Vela; diagrams and code headings are illustrative.</p></details></section><footer class="site-footer"><a class="wordmark" href="../index.html?design=24">Vela</a><span>Your code. Your workspace.</span><a href="https://github.com/Tsohnle95/vela#setup" target="_blank" rel="noopener">Setup guide ↗</a></footer></main>`;
    selector.value = String(current);
    document.title = `Vela — ${String(current).padStart(2, '0')} / ${c.name}`;
    const url = new URL(location.href);
    url.searchParams.set('version', String(current));
    url.hash = '';
    if (updateHistory) history.pushState(null, '', url);
    else history.replaceState(null, '', url);
    fillCollection();
    window.scrollTo(0, 0);
    if (focusHeading) document.getElementById('main').focus({ preventScroll: true });
  }
  function go(value, focusHeading = false) {
    current = (value - 1 + concepts.length) % concepts.length + 1;
    render(true, focusHeading);
  }
  document.getElementById('previous').addEventListener('click', () => go(current - 1));
  document.getElementById('next').addEventListener('click', () => go(current + 1));
  selector.addEventListener('change', () => go(Number(selector.value)));
  allButton.addEventListener('click', () => { dialog.showModal(); allButton.setAttribute('aria-expanded', 'true'); });
  document.getElementById('close-collection').addEventListener('click', () => dialog.close());
  dialog.addEventListener('close', () => { allButton.setAttribute('aria-expanded', 'false'); allButton.focus(); });
  dialog.addEventListener('click', event => {
    const card = event.target.closest('[data-version]');
    if (!card || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    dialog.close();
    go(Number(card.dataset.version), true);
  });
  document.addEventListener('keydown', event => {
    if (dialog.open || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || /^(INPUT|SELECT|TEXTAREA|BUTTON|SUMMARY|A)$/.test(event.target.tagName) || event.target.isContentEditable) return;
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();
    go(current + (event.key === 'ArrowRight' ? 1 : -1));
  });
  site.addEventListener('click', event => {
    const task = event.target.closest('[data-task]');
    if (!task) return;
    const descriptions = {
      edit: 'Browse your repository and edit with Monaco, tabs, and autosave.',
      ask: 'Describe a change, attach relevant files, and follow OpenCode responses beside your code.',
      review: 'See observed workspace changes and inspect per-file diffs when starting contents are available.'
    };
    site.querySelectorAll('[data-task]').forEach(button => button.setAttribute('aria-pressed', String(button === task)));
    document.getElementById('task-detail').textContent = descriptions[task.dataset.task];
    site.querySelector('figcaption').textContent = descriptions[task.dataset.task];
  });
  window.addEventListener('popstate', () => {
    const value = Number(new URLSearchParams(location.search).get('version'));
    current = Number.isInteger(value) && value >= 1 && value <= concepts.length ? value : 1;
    render();
  });
  render();
})();
