(() => {
  const { EFFECTS, mount } = window.VelaLab;
  const canvas = document.getElementById('effect-canvas');
  const select = document.getElementById('effect-select');
  const previous = document.getElementById('previous');
  const next = document.getElementById('next');
  const index = document.getElementById('effect-index');
  const title = document.getElementById('effect-title');
  const line = document.getElementById('effect-line');
  const badge = document.getElementById('effect-badge');
  const toVela = document.getElementById('to-vela');
  const footerName = document.getElementById('effect-footer-name');

  const saved = EFFECTS.filter((effect) => effect.saved);
  const fresh = EFFECTS.filter((effect) => !effect.saved);
  const options = (list) => list
    .map((effect) => `<option value="${effect.slug}">${effect.name}</option>`)
    .join('');
  select.innerHTML = [
    `<optgroup label="Saved five">${options(saved)}</optgroup>`,
    `<optgroup label="Fifteen new">${options(fresh)}</optgroup>`,
  ].join('');

  const requested = new URLSearchParams(location.search).get('effect');
  let current = Math.max(0, EFFECTS.findIndex((effect) => effect.slug === requested));

  const render = (nextIndex, push = true) => {
    current = (nextIndex + EFFECTS.length) % EFFECTS.length;
    const effect = EFFECTS[current];
    const mounted = mount(canvas, effect.slug);
    document.body.classList.toggle('effect-failed', !mounted);
    document.body.dataset.layout = effect.layout;
    document.body.dataset.effect = effect.slug;
    document.title = `Vela — ${effect.name}`;
    select.value = effect.slug;
    index.textContent = `${String(current + 1).padStart(2, '0')} / ${String(EFFECTS.length).padStart(2, '0')}`;
    title.textContent = effect.name;
    line.textContent = effect.line;
    badge.hidden = !effect.saved;
    toVela.href = `vela.html?effect=${effect.slug}`;
    footerName.textContent = effect.name;
    const url = new URL(location.href);
    url.searchParams.set('effect', effect.slug);
    if (push) history.pushState(null, '', url);
    else history.replaceState(null, '', url);
  };

  previous.addEventListener('click', () => render(current - 1));
  next.addEventListener('click', () => render(current + 1));
  select.addEventListener('change', () => {
    const target = EFFECTS.findIndex((effect) => effect.slug === select.value);
    if (target >= 0) render(target);
  });
  window.addEventListener('keydown', (event) => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (/^(INPUT|SELECT|TEXTAREA|BUTTON|A|OPTION)$/.test(event.target.tagName)) return;
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      render(current - 1);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      render(current + 1);
    }
  });
  window.addEventListener('popstate', () => {
    const target = EFFECTS.findIndex((effect) => effect.slug === new URLSearchParams(location.search).get('effect'));
    if (target >= 0) render(target, false);
  });

  render(current, false);
})();
