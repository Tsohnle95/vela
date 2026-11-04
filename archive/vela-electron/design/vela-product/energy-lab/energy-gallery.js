(() => {
  const concepts = [
    { slug: 'circuit-rail', name: 'Circuit Rail', line: 'Energy routes through the real navigation, actions and sections.', channel: 'page structure', tone: '#9fe8d3', angle: '-18deg' },
    { slug: 'sidebar-conduit', name: 'Sidebar Conduit', line: 'A vertical channel feeds the file rows in the workspace.', channel: 'file tree', tone: '#6ee7b7', angle: '0deg' },
    { slug: 'editor-scan', name: 'Editor Sweep', line: 'A scanline crosses the code and collects at the active tab.', channel: 'editor', tone: '#6bd6ff', angle: '0deg' },
    { slug: 'agent-thread', name: 'Agent Thread', line: 'Photons climb the conversation from prompt to messages.', channel: 'agent panel', tone: '#f0a6ff', angle: '4deg' },
    { slug: 'diff-runoff', name: 'Diff Runoff', line: 'Energy leaves the changes rail and crosses changed-file rows.', channel: 'diff rail', tone: '#ffd36b', angle: '12deg' },
    { slug: 'terminal-pour', name: 'Terminal Pour', line: 'A falling feed pours into the prompt and splashes upward.', channel: 'terminal', tone: '#6ee7b7', angle: '-8deg' },
    { slug: 'window-halo', name: 'Window Halo', line: 'The app window itself becomes a continuous perimeter channel.', channel: 'window frame', tone: '#8fb8ff', angle: '0deg' },
    { slug: 'prompt-channel', name: 'Prompt Channel', line: 'Energy enters the prompt and branches into suggestions.', channel: 'prompt box', tone: '#bdeeff', angle: '8deg' },
    { slug: 'tab-bridge', name: 'Tab Bridge', line: 'An arc of energy connects the editor tab to the agent tab.', channel: 'tab bar', tone: '#ffd36b', angle: '0deg' },
    { slug: 'logo-fountain', name: 'Logo Fountain', line: 'Light erupts from the real logo and falls over the hero.', channel: 'logo', tone: '#b9a1ff', angle: '-14deg' },
  ];
  const grid = document.getElementById('energy-grid');
  grid.innerHTML = concepts.map((concept, index) => `
    <a class="eg-card" href="energy.html?concept=${concept.slug}" style="--tone-a:${concept.tone};--angle:${concept.angle}">
      <div class="eg-art">
        <span>${String(index + 1).padStart(2, '0')} · ${concept.channel}</span>
        <div class="eg-shot"><img src="../assets/vela-dark.png" width="1619" height="971" alt="" loading="lazy"></div>
      </div>
      <div class="eg-body">
        <h3>${concept.name}</h3>
        <p>${concept.line}</p>
        <small>${concept.channel}</small>
        <em>Open live study →</em>
      </div>
    </a>`).join('');
})();
