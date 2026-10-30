(() => {
  const systems = [
    { slug: 'nebula-storm', name: 'Nebula Storm', line: 'Lightning moves through the galaxy clouds behind the page.', channel: 'nebula', tone: '#b45cff', angle: '-6deg', href: 'nebula-storm.html' },
    { slug: 'silk', name: 'Silk', line: 'Luminous fabric drifts across the page in deep folds.', channel: 'fabric', tone: '#c9a1ff', angle: '10deg' },
    { slug: 'gravity', name: 'Gravity', line: 'A gravity well bends a page-wide grid into the workspace.', channel: 'spacetime', tone: '#8fb8ff', angle: '0deg' },
    { slug: 'supernova', name: 'Supernova', line: 'A shockwave and debris sweep the page from behind the hero.', channel: 'shockwave', tone: '#ff7ab8', angle: '-16deg' },
    { slug: 'prism', name: 'Prism', line: 'A beam hits a prism and fans into spectra across the page.', channel: 'spectrum', tone: '#ffe08a', angle: '18deg' },
    { slug: 'helix', name: 'Helix', line: 'Two intertwined photon strands descend into the app.', channel: 'helix', tone: '#9fe8d3', angle: '-4deg' },
    { slug: 'magnetar', name: 'Magnetar', line: 'Dipole field loops throw particles across the page.', channel: 'magnetic', tone: '#b9a1ff', angle: '6deg' },
    { slug: 'geyser', name: 'Geyser', line: 'Geysers erupt from behind the workspace and rain back down.', channel: 'eruption', tone: '#6ee7b7', angle: '0deg' },
    { slug: 'flood', name: 'Flood', line: 'Light rises from the bottom and submerges the interface.', channel: 'flood', tone: '#6bd6ff', angle: '0deg' },
    { slug: 'constellation', name: 'Constellation', line: 'The real interface becomes a page-scale constellation.', channel: 'star map', tone: '#d8fff3', angle: '-10deg' },
  ];
  const grid = document.getElementById('energy-grid');
  grid.innerHTML = systems.map((system, index) => `
    <a class="eg-card" href="${system.href || `field.html?concept=${system.slug}`}" style="--tone-a:${system.tone};--angle:${system.angle}">
      <div class="eg-art">
        <span>${String(index + 11).padStart(2, '0')} · ${system.channel}</span>
        <div class="eg-shot"><img src="../assets/vela-dark.png" width="1619" height="971" alt="" loading="lazy"></div>
      </div>
      <div class="eg-body">
        <h3>${system.name}</h3>
        <p>${system.line}</p>
        <small>${system.channel}</small>
        <em>Open live system →</em>
      </div>
    </a>`).join('');
})();
