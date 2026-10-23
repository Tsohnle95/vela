/* Each brief starts with a specific reading of Source (#24), then changes its
   composition. Shared product fragments keep facts and asset references stable. */
(() => {
  const repo = 'https://github.com/Tsohnle95/vela';
  const label = (text) => `<p class="label">${text}</p>`;
  const actions = (text = 'Download for macOS') => `<div class="actions"><a class="primary" href="${repo}/releases" target="_blank" rel="noopener">${text} <span aria-hidden="true">↗</span></a><a class="secondary" href="${repo}" target="_blank" rel="noopener">View source <span aria-hidden="true">↗</span></a></div><p class="platform">macOS · OpenCode V2</p>`;
  const shot = (caption = 'The editor. The agent. The same context.', cls = '') => `<figure id="workspace" class="product ${cls}"><div class="frame-label"><span>VELA / WORKSPACE</span><span>EDITOR + OPENCODE</span></div><img src="../assets/vela.png" width="1420" height="887" alt="Vela desktop: Neptune repository files, JavaScript editor, and an OpenCode agent conversation side by side."><figcaption>${caption}</figcaption></figure>`;
  const intro = (heading = 'Build with<br> <em>context.</em>', text = 'Keep OpenCode beside your repository. Write, review, and run without leaving Vela.') => `<div class="intro"><h1>${heading}</h1><p class="lede">${text}</p>${actions()}</div>`;
  const features = (cls = '') => `<section class="features ${cls}" aria-label="Workspace capabilities"><article><span>01 / EDIT</span><h2>Your files.<br> Your editor.</h2><p>Browse the repository and edit with Monaco, tabs, and autosave.</p></article><article><span>02 / ASK</span><h2>An agent.<br> In context.</h2><p>Work with OpenCode beside the files it changes.</p></article><article><span>03 / REVIEW</span><h2>Follow the work.<br> Review the diff.</h2><p>Watch workspace changes and inspect per-file diffs when starting contents are available.</p></article></section>`;
  const note = (text) => `<p class="annotation">${text}</p>`;
  const steps = () => `<ol class="steps"><li><span>01</span><div><h2>Open a repository.</h2><p>Your file tree, ready to explore.</p></div></li><li><span>02</span><div><h2>Work with OpenCode.</h2><p>Ask, attach files, and follow the session.</p></div></li><li><span>03</span><div><h2>Review. Edit. Run.</h2><p>Check the diff and use the integrated terminal.</p></div></li></ol>`;
  window.sourceConcepts = [
    {
      name: 'Margin notes', motif: '01 / annotated manuscript',
      source: 'Source’s comment-like feature numbers and fine grid become a working document with a wide annotation margin.',
      change: 'An off-center headline and numbered marginalia replace the centered hero; the screenshot sits on the document’s main column.',
      body: () => `<div class="manuscript"><aside class="margin">${label('SOURCE / 001')}<span class="margin-rule"></span><p>A workspace<br> for coding agents.</p><p class="margin-bottom">Write in the editor.<br> Think with your agent.</p></aside><div>${label('// A NOTE ON CONTEXT')}${intro('Good work starts<br> <em>in the margins.</em>')}${note('01 — Keep the conversation next to the code.')}${shot()}${features()}</div></div>`
    },
    {
      name: 'Open bracket', motif: '02 / architectural enclosure',
      source: 'The quiet braces around “context” are Source’s strongest typographic device.',
      change: 'The braces become a full-height architectural frame enclosing both the proposition and the real app, with the title set across a two-column plane.',
      body: () => `<section class="bracket-room">${label('A PLACE FOR THE WHOLE SESSION')}<div class="bracket-heading"><h1>Build with</h1><p>Code, conversation,<br> and changes.<br> Held together.</p></div><div class="bracket-enclosure"><div class="context-word">context.</div>${shot('One repository. A shared point of reference.')}</div><div class="bracket-bottom">${actions()}${note('Everything you need to follow the work.')}</div></section>${features()}`
    },
    {
      name: 'Field manual', motif: '03 / index and specimen',
      source: 'Source reads like a clean technical sheet: monospaced labels, restrained green, and a clearly identified product frame.',
      change: 'A vertical chapter index anchors a manual with a compact cover, a full-width specimen image, and large numbered instructions.',
      body: () => `<div class="manual"><aside><a href="#overview">00 / Overview</a><a href="#workspace">01 / Workspace</a><a href="#method">02 / Method</a><span>VELA<br> FIELD MANUAL<br> DESKTOP EDITION</span></aside><div><section id="overview" class="manual-cover">${label('FIELD MANUAL / VELA')}<h1>A workspace<br> with <em>context.</em></h1><div class="manual-summary"><p class="lede">An editor, an agent, and a terminal. The tools for following a change from prompt to review.</p>${actions()}</div></section><section id="workspace">${shot('FIG. 01 — The workspace, with an OpenCode session in view.')}</section><section id="method">${steps()}</section></div></div>`
    },
    {
      name: 'Split declaration', motif: '04 / two continuous planes',
      source: 'Source puts two typographic voices on the same centerline: plain-language sans and code-like mono.',
      change: 'Those voices occupy separate vertical planes: a white statement column and a sage product column, joined by a syntax annotation.',
      body: () => `<section class="declaration"><div class="declaration-copy">${label('YOUR CODE, IN GOOD COMPANY')}${intro('Your code.<br> <em>Its context.</em>')}<div class="declaration-end"><span>{ editor }</span><span>{ agent }</span><span>{ terminal }</span></div></div><div class="declaration-image">${label('THE OTHER HALF OF YOUR WORKFLOW')}${shot('Vela brings the session together.')}<p class="oversized-code">return<br> <em>workspace;</em></p></div></section>${features()}`
    },
    {
      name: 'Repository atlas', motif: '05 / connected cartography',
      source: 'The background grid suggests coordinates, while the product copy is about preserving the relationship between files and agents.',
      change: 'A branching map becomes the main composition: the repository is a central node, with editor, agent, and review stations connected around the screenshot.',
      body: () => `<section class="atlas-head">${label('MAP OF A WORKING SESSION')}${intro('Find your<br> <em>common ground.</em>')}</section><section class="atlas-map" aria-label="Workspace map"><div class="map-root">~/your-repository</div><div class="map-stations"><span>01 / EDITOR</span><span>02 / OPENCODE</span><span>03 / CHANGES</span></div>${shot('All paths lead back to your repository.')}<div class="map-coordinate">VELA / LOCAL WORKSPACE / DESKTOP</div></section>${features()}`
    },
    {
      name: 'Patch notes', motif: '06 / a visual diff',
      source: 'Source’s sober code references and muted green naturally connect to the app’s live file-change view.',
      change: 'A large illustrative diff carries the headline, with minus/plus gutters, alternating rows, and the app revealed beneath the added line.',
      body: () => `<section class="patch">${label('A CHANGE TO YOUR WORKSPACE')}<div class="diff-heading"><p class="removed"><span>−</span>Switch windows.</p><h1><span>+</span> Keep <em>context.</em></h1></div><div class="patch-description"><p class="lede">Keep your editor, OpenCode sessions, and changes in one desktop workspace.</p>${actions()}</div>${note('ILLUSTRATIVE WORKFLOW DIFF / NOT A LIVE FILE CHANGE')}${shot('See the code. Follow what changes.')}<div class="patch-summary"><span>+ Monaco editor</span><span>+ OpenCode sessions</span><span>+ Integrated terminal</span><span>+ Per-file diffs</span></div></section>${steps()}`
    },
    {
      name: 'Reading room', motif: '07 / typographic quiet',
      source: 'The most distinctive quality of Source is its restraint: white space, low-contrast rules, and a single green word.',
      change: 'A narrow serif reading column replaces the technical hero, while a broad unadorned app image interrupts the essay like a photographic plate.',
      body: () => `<section class="reading-room">${label('ON KEEPING THINGS TOGETHER')}<h1>A little more room<br> for <em>context.</em></h1><p class="reading-lead">A repository is more than a collection of files. It is the place where the work happens.</p><p>Vela keeps your editor, OpenCode agent, and terminal in that same place. Read the response. Open the file. Review the change.</p>${actions()}</section>${shot('An ordinary working session. Everything close at hand.')}<div class="reading-after"><h2>Stay with the work.</h2><p>Browse your files with Monaco, follow streamed OpenCode responses, and review the changes as you go.</p></div>`
    },
    {
      name: 'Workspace anatomy', motif: '08 / labeled product plate',
      source: 'Source frames its screenshot with an “editor / agent / terminal” label but leaves the relationships implicit.',
      change: 'The product becomes the first and largest element: an anatomical plate with labeled rails, followed by a compact bottom-aligned headline.',
      body: () => `<section class="anatomy">${label('ANATOMY OF A WORKSPACE')}<div class="anatomy-plate"><div class="anatomy-rail"><span>01 — YOUR REPOSITORY</span><span>02 — YOUR EDITOR</span><span>03 — YOUR AGENT</span></div>${shot('One window. Three connected parts.')}</div><div class="anatomy-bottom"><h1>Everything.<br> <em>In context.</em></h1><div><p class="lede">The files, the conversation, and the changes. Keep the whole working session in view.</p>${actions()}</div></div></section>${features()}`
    },
    {
      name: 'Grid specimen', motif: '09 / modular display cabinet',
      source: 'Source’s grid and monospaced micro-labels establish a precise technical rhythm.',
      change: 'That rhythm becomes a modular cabinet with unequal cells: a large headline, a syntax tile, a product tile, and a dedicated action rail.',
      body: () => `<section class="specimen"><div class="spec-title">${label('SPECIMEN 009 / DESKTOP WORKSPACE')}<h1>Build.<br> With<br> <em>context.</em></h1></div><div class="spec-symbol" aria-hidden="true">{<span>V</span>}</div><div class="spec-copy"><p>Editor.<br> Agent.<br> Terminal.</p><span>CONNECTED BY YOUR REPOSITORY.</span></div><div class="spec-product">${shot('THE WORKSPACE / IN USE')}</div><div class="spec-actions"><p>One desktop app.<br> Powered by OpenCode.</p>${actions()}</div></section>${features()}`
    },
    {
      name: 'Continuous line', motif: '10 / a guided vertical journey',
      source: 'Source numbers its features, hinting at a sequence from editing to working with an agent and reviewing changes.',
      change: 'A continuous green route runs down the page, passing an opening statement, the product, and large process stations instead of a hero-plus-cards stack.',
      body: () => `<div class="continuous"><section class="route-intro">${label('START HERE / YOUR REPOSITORY')}${intro('Follow the work.<br> <em>Keep the context.</em>')}</section><section class="route-product">${label('01 / YOUR WORKSPACE')}${shot('The session moves forward. Your files stay close.')}</section><section class="route-method">${label('02 / YOUR NEXT CHANGE')}${steps()}</section><div class="route-end">${label('03 / READY WHEN YOU ARE')}${actions()}</div></div>`
    },
  ];
  window.sourceFragments = { label, actions, shot, intro, features, note, steps };
})();
