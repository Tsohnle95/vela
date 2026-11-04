(() => {
  const { VARIANTS, snapshot } = window.LumenLab;
  const grid = document.getElementById('lumen-grid');

  grid.innerHTML = VARIANTS.map((variant, index) => `
    <article class="lab-card" data-cast="${variant.slug}" style="--tone-a:${variant.toneA};--tone-b:${variant.toneB};--card-angle:${variant.angle}">
      <a class="lab-thumb lumen-thumb" href="lumen.html?cast=${variant.slug}" aria-label="Open ${variant.name} in the Vela page">
        <img data-thumb="${variant.slug}" width="480" height="300" alt="${variant.name} light cast still" decoding="async">
        <span class="lumen-mini"><img src="../assets/vela-dark.png" width="1619" height="971" alt="" loading="lazy"></span>
        <span class="lab-number">${String(index + 1).padStart(2, '0')}</span>
        <span class="lab-open">Open the cast</span>
      </a>
      <div class="lab-card-body">
        <div class="lab-card-head">
          <h3>${variant.name}</h3>
          <span>0.5s cast</span>
        </div>
        <p class="lab-line">${variant.line}</p>
        <div class="lab-card-links">
          <a href="lumen.html?cast=${variant.slug}">Vela page</a>
          <a href="effects.html?effect=lumen">Original Lumen →</a>
        </div>
      </div>
    </article>`).join('');

  const nebulaGrid = document.getElementById('nebula-grid');
  nebulaGrid.innerHTML = `
    <article class="lab-card is-saved lumen-nebula-card">
      <a class="lab-thumb lumen-thumb" href="nebula.html" aria-label="Open the saved Nebula Vela page">
        <canvas class="lumen-nebula-live" data-live-effect="nebula" aria-hidden="true"></canvas>
        <span class="lab-number">REF</span>
        <span class="lab-pick">Saved reference</span>
        <span class="lab-open">Open Vela page</span>
      </a>
      <div class="lab-card-body">
        <div class="lab-card-head">
          <h3>Nebula</h3>
          <span>Saved reference</span>
        </div>
        <p class="lab-line">A field of quiet light, with its clouds and gas kept in motion.</p>
        <div class="lab-card-links">
          <a href="nebula.html">Vela page</a>
          <a href="nebula-pure.html">Pure animation →</a>
        </div>
      </div>
    </article>`;

  const live = document.querySelector('[data-live-effect]');
  if (live && window.VelaLab) {
    const mounted = window.VelaLab.mount(live, live.dataset.liveEffect);
    if (!mounted) live.closest('.lab-thumb')?.classList.add('is-fallback');
  }

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
