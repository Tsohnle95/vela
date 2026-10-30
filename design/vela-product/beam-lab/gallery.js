(() => {
  const grid = document.getElementById('beam-grid');
  grid.innerHTML = window.BeamLab.DESIGNS.map((design, index) => `
    <a class="leaf-card" href="design.html?design=${design.slug}" style="--tone-a:${design.tone[0]};--tone-b:${design.tone[1]}">
      <div class="beam-mini" data-layout="${design.layout}">
        <span class="beam-mini-beam"></span>
        <span class="beam-mini-surface">
          <img src="../assets/vela-dark.png" width="1619" height="971" alt="" loading="lazy">
        </span>
      </div>
      <div class="leaf-card-body">
        <div class="leaf-card-head">
          <h3>${design.name}</h3>
          <span>${String(index + 1).padStart(2, '0')} / ${String(window.BeamLab.DESIGNS.length).padStart(2, '0')}</span>
        </div>
        <p class="leaf-card-line">${design.line}</p>
        <p class="leaf-card-note">${design.note}</p>
        <div class="leaf-card-foot">
          <span>${design.layout}</span>
          <span>Open study →</span>
        </div>
      </div>
    </a>`).join('');
})();
