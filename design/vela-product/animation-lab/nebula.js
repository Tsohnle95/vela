(() => {
  const site = document.getElementById('site');
  site.innerHTML = window.velaMeet.build({ effect: 'nebula', beam: false });
  const canvas = site.querySelector('.ob-effect-canvas');
  window.VelaLab.mount(canvas, 'nebula');
  document.title = 'Vela — Nebula';
})();
