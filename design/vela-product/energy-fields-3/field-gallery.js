(() => {
  const systems = [
    { slug: 'dawn', name: 'Dawn', line: 'Warm light rises from the bottom edge, clear of all copy.', channel: 'horizon', tone: '#ffb15e', angle: '0deg' },
    { slug: 'halo', name: 'Halo', line: 'A soft breathing aura stays around the workspace window.', channel: 'window aura', tone: '#8fb8ff', angle: '0deg' },
    { slug: 'drift', name: 'Drift', line: 'Faint dust drifts through the page background.', channel: 'dust', tone: '#c9a1ff', angle: '8deg' },
    { slug: 'marquee', name: 'Marquee', line: 'Wide diagonal bands pass behind the page like blinds.', channel: 'light bands', tone: '#9fe8d3', angle: '-14deg' },
    { slug: 'edge', name: 'Edge', line: 'A current travels around the viewport border.', channel: 'perimeter', tone: '#d8fff3', angle: '0deg' },
    { slug: 'hairline', name: 'Hairline', line: 'Light moves along the real section dividers.', channel: 'structure', tone: '#8fb8ff', angle: '0deg' },
    { slug: 'aurora', name: 'Aurora', line: 'A low veil stays in the empty air above the hero.', channel: 'aurora', tone: '#9fe8d3', angle: '4deg' },
    { slug: 'bokeh', name: 'Bokeh', line: 'Large out-of-focus lights drift near the corners.', channel: 'bokeh', tone: '#f0a6ff', angle: '-8deg' },
    { slug: 'ember', name: 'Ember', line: 'Sparse warm embers lift from the bottom corners.', channel: 'embers', tone: '#ff7a5e', angle: '10deg' },
    { slug: 'signal', name: 'Signal', line: 'A faint band of light scans the page every few seconds.', channel: 'scan', tone: '#d8fff3', angle: '0deg' },
  ];
  const grid = document.getElementById('energy-grid');
  grid.innerHTML = systems.map((system, index) => `
    <a class="eg-card" href="field.html?concept=${system.slug}" style="--tone-a:${system.tone};--angle:${system.angle}">
      <div class="eg-art">
        <span>${String(index + 21).padStart(2, '0')} · ${system.channel}</span>
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
