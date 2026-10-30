(() => {
  const site = document.getElementById('site');
  site.innerHTML = window.velaMeet.build({ effect: 'nebulaStorm', beam: false });
  const canvas = site.querySelector('.ob-effect-canvas');
  window.VelaLab.mount(canvas, 'nebulaStorm');
  document.title = 'Vela — Nebula Storm';
})();
