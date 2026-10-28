(() => {
  const { EFFECTS, mount } = window.VelaLab;
  const site = document.getElementById('site');
  const bar = document.getElementById('lab-bar');
  const select = document.getElementById('lab-select');
  const previous = document.getElementById('lab-previous');
  const next = document.getElementById('lab-next');
  const status = document.getElementById('lab-status');
  const pure = document.getElementById('lab-pure');
  const restore = document.getElementById('lab-restore');
  const params = new URLSearchParams(location.search);

  const saved = EFFECTS.filter((effect) => effect.saved);
  const fresh = EFFECTS.filter((effect) => !effect.saved);
  const options = (list) => list
    .map((effect) => `<option value="${effect.slug}">${effect.name}</option>`)
    .join('');
  select.innerHTML = [
    `<optgroup label="Saved five">${options(saved)}</optgroup>`,
    `<optgroup label="Fifteen new">${options(fresh)}</optgroup>`,
  ].join('');

  const requested = params.get('effect');
  let current = Math.max(0, EFFECTS.findIndex((effect) => effect.slug === requested));
  let clean = params.get('clean') === '1';

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

  const render = (nextIndex, push = true) => {
    current = (nextIndex + EFFECTS.length) % EFFECTS.length;
    const effect = EFFECTS[current];
    document.body.className = `d1 effect-page effect-${effect.slug}${clean ? ' lab-clean' : ''}`;
    document.body.dataset.effect = effect.slug;
    site.innerHTML = window.velaMeet.build({ effect: effect.slug, beam: false });
    mount(site.querySelector('.ob-effect-canvas'), effect.slug);
    document.title = `Vela — ${effect.name}`;
    select.value = effect.slug;
    status.hidden = !effect.saved;
    pure.href = `effects.html?effect=${effect.slug}`;
    const url = new URL(location.href);
    url.searchParams.set('effect', effect.slug);
    url.hash = '';
    if (push) history.pushState(null, '', url);
    else history.replaceState(null, '', url);
    setClean(clean, false);
    window.scrollTo(0, 0);
  };

  previous.addEventListener('click', () => render(current - 1));
  next.addEventListener('click', () => render(current + 1));
  select.addEventListener('change', () => {
    const target = EFFECTS.findIndex((effect) => effect.slug === select.value);
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
    const target = EFFECTS.findIndex((effect) => effect.slug === new URLSearchParams(location.search).get('effect'));
    clean = new URLSearchParams(location.search).get('clean') === '1';
    if (target >= 0) render(target, false);
  });

  render(current, false);
})();
