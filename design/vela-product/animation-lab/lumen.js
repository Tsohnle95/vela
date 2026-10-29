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
  let sweepRaf = 0;

  const rectFor = (preview) => {
    const rect = preview.getBoundingClientRect();
    const height = Math.max(window.innerHeight, 1);
    return [
      (rect.left + rect.width / 2 - window.innerWidth / 2) / height,
      (height / 2 - (rect.top + rect.height / 2)) / height,
      (rect.width / 2) / height,
      (rect.height / 2) / height,
    ];
  };

  const updateHit = () => {
    const preview = site.querySelector('.ob-preview');
    const canvas = site.querySelector('.ob-effect-canvas');
    if (preview && canvas) window.LumenLab.setHit(canvas, rectFor(preview));
  };

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

  const lightNow = () => {
    clearTimeout(lightTimer);
    document.body.classList.add('is-lit');
  };

  const armLight = () => {
    clearTimeout(lightTimer);
    document.body.classList.remove('is-lit');
    lightTimer = setTimeout(lightNow, 1800);
  };

  const stopSweep = () => {
    cancelAnimationFrame(sweepRaf);
    sweepRaf = 0;
  };

  const startSweep = () => {
    stopSweep();
    const start = performance.now();
    const tick = () => {
      if (VARIANTS[current].slug !== 'searchlight') {
        sweepRaf = 0;
        return;
      }
      const fraction = 0.5 + 0.26 * Math.sin(((performance.now() - start) / 1000) * 0.35);
      document.body.style.setProperty('--lumen-x', `${(fraction * 100).toFixed(2)}%`);
      sweepRaf = requestAnimationFrame(tick);
    };
    tick();
  };

  const render = (nextIndex, push = true) => {
    current = (nextIndex + VARIANTS.length) % VARIANTS.length;
    const variant = VARIANTS[current];
    const style = document.body.style;
    style.setProperty('--lumen-angle', variant.angle);
    style.setProperty('--lumen-x', `${Math.round(variant.hit * 100)}%`);
    style.setProperty('--lumen-y', variant.y);
    style.setProperty('--lumen-brightness', variant.brightness);
    style.setProperty('--lumen-opacity', variant.opacity);
    style.setProperty('--lumen-hot', variant.toneB);
    style.setProperty('--lumen-warm', variant.toneA);
    document.body.className = `d1 effect-page lumen-page${clean ? ' lab-clean' : ''}`;
    document.body.dataset.cast = variant.slug;
    document.body.dataset.style = variant.style;
    site.innerHTML = window.velaMeet.build({ effect: variant.slug, beam: false });
    const preview = site.querySelector('.ob-preview');
    const canvas = site.querySelector('.ob-effect-canvas');
    mount(canvas, variant.slug, { delay: 0.5, hit: rectFor(preview), hitFrac: variant.hit, castOrigin: variant.castOrigin, onImpact: lightNow });
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
    if (variant.style === 'sweep') startSweep();
    else stopSweep();
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
  window.addEventListener('scroll', updateHit, { passive: true });
  window.addEventListener('resize', updateHit);
  window.addEventListener('popstate', () => {
    const target = VARIANTS.findIndex((variant) => variant.slug === new URLSearchParams(location.search).get('cast'));
    clean = new URLSearchParams(location.search).get('clean') === '1';
    if (target >= 0) render(target, false);
  });

  render(current, false);
  updateHit();
})();
