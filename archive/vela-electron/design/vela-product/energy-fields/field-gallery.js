(() => {
  const systems = [
    { slug: 'cascade', name: 'Cascade', line: 'A full-page waterfall falls through the hero and pools at the workspace.', channel: 'waterfall', tone: '#9fe8d3', angle: '4deg' },
    { slug: 'rift', name: 'Rift', line: 'A vertical tear in the page leaks horizontal light across the interface.', channel: 'rift', tone: '#c9a1ff', angle: '0deg' },
    { slug: 'vortex', name: 'Vortex', line: 'A large spiral pulls energy down into the workspace.', channel: 'vortex', tone: '#8fb8ff', angle: '-18deg' },
    { slug: 'eclipse', name: 'Eclipse', line: 'A page-scale sun burns behind the hero and casts light onto the workspace.', channel: 'sun', tone: '#ffd36b', angle: '0deg' },
    { slug: 'curtain', name: 'Curtain', line: 'A full-width aurora curtain falls through the page.', channel: 'curtain', tone: '#9fe8d3', angle: '8deg' },
    { slug: 'lightning', name: 'Lightning', line: 'Branching bolts cross the page and strike the interface.', channel: 'storm', tone: '#bdeeff', angle: '-10deg' },
    { slug: 'tide', name: 'Tide', line: 'Horizontal waves roll across the entire viewport.', channel: 'tide', tone: '#8fb8ff', angle: '0deg' },
    { slug: 'portal', name: 'Portal', line: 'A large portal opens behind the workspace and feeds it with light.', channel: 'portal', tone: '#c9a1ff', angle: '0deg' },
    { slug: 'photon-rain', name: 'Photon Rain', line: 'A diagonal storm of photons crosses the page and collects at the workspace.', channel: 'photon storm', tone: '#8fb8ff', angle: '-24deg' },
    { slug: 'bloom', name: 'Bloom', line: 'A page-scale bloom opens from the logo and channels into the interface.', channel: 'bloom', tone: '#c9a1ff', angle: '14deg' },
  ];
  const grid = document.getElementById('energy-grid');
  grid.innerHTML = systems.map((system, index) => `
    <a class="eg-card" href="field.html?concept=${system.slug}" style="--tone-a:${system.tone};--angle:${system.angle}">
      <div class="eg-art">
        <span>${String(index + 1).padStart(2, '0')} · ${system.channel}</span>
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
