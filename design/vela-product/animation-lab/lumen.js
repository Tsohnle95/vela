(() => {
  const { VARIANTS, mount } = window.LumenLab;
  const site = document.getElementById('site');
  const bar = document.getElementById('lumen-bar');
  const select = document.getElementById('lumen-select');
  const previous = document.getElementById('lumen-previous');
  const next = document.getElementById('lumen-next');
  const meta = document.getElementById('lumen-meta');
  const restore = document.getElementById('lumen-restore');
  const params = new URLSearchParams(location.search);

  select.innerHTML = VARIANTS.map((variant, index) =>
    `<option value="${variant.slug}">${String(index + 1).padStart(2, '0')} — ${variant.name}</option>`).join('');

  const requested = params.get('cast');
  let current = Math.max(0, VARIANTS.findIndex((variant) => variant.slug === requested));
  let clean = params.get('clean') === '1';
  let lightTimer = 0;

  const setClean = (hidden, writeUrl = true) => {
    clean = hidden;
    bar.hidden = hidden;
    restore.hidden = !hidden;
    document.body.classList.toggle('lab-clean', hidden);
    if (!writeUrl) return;
    const url = new URL(location.href);
    if (hidden) url.searchParams.set('clean', '1');
    else url.searchParams.delete('clean');
    history.replaceState(null, '', url);
  };

  const armLight = () => {
    clearTimeout(lightTimer);
    document.body.classList.remove('is-lit');
    lightTimer = setTimeout(() => document.body.classList.add('is-lit'), 500);
  };

  const render = (nextIndex, push = true) => {
    current = (nextIndex + VARIANTS.length) % VARIANTS.length;
    const variant = VARIANTS[current];
    const style = document.body.style;
    style.setProperty('--lumen-angle', variant.angle);
    style.setProperty('--lumen-x', variant.x);
    style.setProperty('--lumen-y', variant.y);
    style.setProperty('--lumen-brightness', variant.brightness);
    style.setProperty('--lumen-opacity', variant.opacity);
    style.setProperty('--lumen-hot', variant.toneB);
    style.setProperty('--lumen-warm', variant.toneA);
    document.body.className = `d1 effect-page lumen-page${clean ? ' lab-clean' : ''}`;
    document.body.dataset.cast = variant.slug;
    site.innerHTML = window.velaMeet.build({ effect: variant.slug, beam: false });
    mount(site.querySelector('.ob-effect-canvas'), variant.slug, { delay: 0.5 });
    document.title = `Vela — ${variant.name} light`;
    select.value = variant.slug;
    meta.textContent = `${String(current + 1).padStart(2, '0')} / ${String(VARIANTS.length).padStart(2, '0')}`;
    const url = new URL(location.href);
    url.searchParams.set('cast', variant.slug);
    url.hash = '';
    if (push) history.pushState(null, '', url);
    else history.replaceState(null, '', url);
    setClean(clean, false);
    window.scrollTo(0, 0);
    armLight();
  };

  previous.addEventListener('click', () => render(current - 1));
  next.addEventListener('click', () => render(current + 1));
  select.addEventListener('change', () => {
    const target = VARIANTS.findIndex((variant) => variant.slug === select.value);
    if (target >= 0) render(target);
  });
  restore.addEventListener('click', () => setClean(false));
  document.addEventListener('keydown', (event) => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key.toLowerCase() === 'h' && !/^(INPUT|SELECT|TEXTAREA)$/.test(event.target.tagName)) {
      event.preventDefault();
      setClean(!clean);
      return;
    }
    if (event.shiftKey || /^(INPUT|SELECT|TEXTAREA|BUTTON|A|SUMMARY|OPTION)$/.test(event.target.tagName)) return;
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      render(current - 1);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      render(current + 1);
    }
  });
  window.addEventListener('popstate', () => {
    const target = VARIANTS.findIndex((variant) => variant.slug === new URLSearchParams(location.search).get('cast'));
    clean = new URLSearchParams(location.search).get('clean') === '1';
    if (target >= 0) render(target, false);
  });

  render(current, false);
})();
