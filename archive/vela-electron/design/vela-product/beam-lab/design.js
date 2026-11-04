(() => {
  const designs = window.BeamLab.DESIGNS;
  const params = new URLSearchParams(location.search);
  const index = Math.max(0, designs.findIndex((design) => design.slug === params.get('design')));
  const design = designs[index];
  const stage = document.getElementById('stage');
  const canvas = document.getElementById('beam-canvas');

  document.body.dataset.layout = design.layout;
  document.body.style.setProperty('--beam-tone-a', design.tone[0]);
  document.body.style.setProperty('--beam-tone-b', design.tone[1]);
  document.title = `Vela — ${design.name} beam`;
  stage.dataset.design = design.slug;

  document.getElementById('beam-index').textContent = `Beam lab · ${String(index + 1).padStart(2, '0')} / ${String(designs.length).padStart(2, '0')}`;
  document.getElementById('beam-name').textContent = design.name;
  document.getElementById('beam-line').textContent = design.line;
  document.getElementById('beam-note').textContent = design.note;
  document.getElementById('beam-bar-name').textContent = design.name;
  document.getElementById('surface-label').textContent = design.layout === 'palette' ? 'command' : design.layout === 'terminal' ? 'terminal' : design.layout === 'split' ? 'split workspace' : 'workspace';
  document.getElementById('beam-next').href = `design.html?design=${designs[(index + 1) % designs.length].slug}`;

  canvas.dataset.settings = design.settings;
  canvas.dataset.origin = design.origin;
  canvas.dataset.color = design.color;
  canvas.dataset.position = String(design.position);
  canvas.dataset.delay = String(design.delay);
  canvas.dataset.intro = String(design.intro);
  canvas.dataset.rise = String(design.rise);
  canvas.dataset.pace = String(design.pace);
  canvas.dataset.surface = 'preview';

  window.velaBeam.mount(stage);
})();
