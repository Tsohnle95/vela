(() => {
  const { EFFECTS, snapshot } = window.VelaLab;
  const savedGrid = document.getElementById('saved-grid');
  const newGrid = document.getElementById('new-grid');

  const card = (effect, index) => `
    <article class="lab-card${effect.saved ? ' is-saved' : ''}" data-effect="${effect.slug}" style="--tone-a:${effect.tone[0]};--tone-b:${effect.tone[1]}">
      <a class="lab-thumb" href="vela.html?effect=${effect.slug}" aria-label="Open ${effect.name} in the Vela page">
        <img data-thumb="${effect.slug}" width="480" height="300" alt="${effect.name} animation still" decoding="async">
        <span class="lab-number">${String(index + 1).padStart(2, '0')}</span>
        ${effect.saved ? '<span class="lab-pick">Saved pick</span>' : ''}
        <span class="lab-open">Open Vela page</span>
      </a>
      <div class="lab-card-body">
        <div class="lab-card-head">
          <h3>${effect.name}</h3>
          <span>${effect.saved ? 'Saved' : 'New'}</span>
        </div>
        <p class="lab-line">${effect.line}</p>
        <div class="lab-card-links">
          <a href="vela.html?effect=${effect.slug}">Vela page</a>
          <a href="effects.html?effect=${effect.slug}">Pure animation →</a>
        </div>
      </div>
    </article>`;

  savedGrid.innerHTML = EFFECTS
    .map((effect, index) => ({ effect, index }))
    .filter(({ effect }) => effect.saved)
    .map(({ effect, index }) => card(effect, index))
    .join('');
  newGrid.innerHTML = EFFECTS
    .map((effect, index) => ({ effect, index }))
    .filter(({ effect }) => !effect.saved)
    .map(({ effect, index }) => card(effect, index))
    .join('');

  const thumbs = Array.from(document.querySelectorAll('[data-thumb]'));
  const paint = (img) => {
    const url = snapshot(img.dataset.thumb);
    if (!url) {
      img.closest('.lab-thumb').classList.add('is-fallback');
      return;
    }
    img.addEventListener('load', () => img.closest('.lab-thumb').classList.add('is-ready'), { once: true });
    img.src = url;
  };

  const queue = () => {
    const img = thumbs.shift();
    if (!img) return;
    paint(img);
    requestAnimationFrame(queue);
  };
  requestAnimationFrame(queue);
})();
