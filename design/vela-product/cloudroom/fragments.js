(() => {
  const repository = 'https://github.com/Tsohnle95/vela';
  const assets = window.velaAssetBase || '../assets';
  const cta = (text = 'Download for macOS') => `<a class="button" href="${repository}/releases" target="_blank" rel="noopener">${text}<span aria-hidden="true">↗</span></a>`;
  const shot = (classes = '') => `<figure class="app-shot ${classes}"><img src="${assets}/vela.png" width="1420" height="887" alt="Vela showing a repository, JavaScript editor, and OpenCode conversation side by side."></figure>`;
  const icon = (classes = '') => `<img class="vela-icon ${classes}" src="${assets}/icon.png" width="1024" height="1024" alt="Vela">`;
  const mark = (classes = '') => `<svg class="vela-mark ${classes}" viewBox="0 0 200 160" aria-hidden="true"><path fill="#adedd9" d="M36 8 100 92 31 154 0 139Z"/><path fill="#62c4ad" d="M36 8 100 92 31 154 67 120Z"/><path fill="#b5f4dc" d="m164 8 36 131-31 15-69-62Z"/><path fill="#389d8e" d="m164 8 36 131-31 15-36-34Z"/><path fill="#18786f" d="m164 8-31 112-33-28Z"/></svg>`;
  const copy = (title, description = 'Your editor, OpenCode agents, terminal, and live diffs. Together in one desktop workspace.') => `<div class="hero-copy"><h1>${title}</h1><p>${description}</p>${cta()}</div>`;
  const featureRow = () => `<section class="feature-row"><article><h2>Code at home.</h2><p>Browse your repository and edit with Monaco, tabs, and autosave.</p></article><article><h2>Keep your agent close.</h2><p>Give OpenCode the task and follow the conversation beside your code.</p></article><article><h2>See what changed.</h2><p>Follow observed file changes and review per-file diffs when starting contents are available.</p></article></section>`;
  window.VelaDesign = { cta, shot, icon, mark, copy, featureRow, repository };
  window.velaDirections = [];
})();
