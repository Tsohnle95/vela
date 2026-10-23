(() => {
  const { copy, shot, mark, cta, featureRow } = window.VelaDesign;

  window.velaDirections.push(
    {
      id: 11,
      name: 'In orbit',
      body: () => `
        <section class="c11-hero">
          ${copy('Your code.<br> Your agent.<br> <em>One orbit.</em>', 'Bring OpenCode into your workspace. Keep the conversation, your editor, and the changes together.')}
          <div class="c11-orbit" aria-hidden="true"><div class="c11-ring c11-ring-a"></div><div class="c11-ring c11-ring-b"></div><div class="c11-ring c11-ring-c"></div>${mark()}<span class="c11-orbit-point"></span></div>
        </section>
        <section class="c11-workspace" id="workspace"><div class="c11-introduction"><h2>The whole session.<br> In one place.</h2><p>Open a repository and get to work. Edit a file, ask OpenCode, and review the result without leaving the window.</p></div>${shot()}</section>
        ${featureRow()}
        <section class="c11-close"><h2>Keep building.<br> Keep your context.</h2>${cta()}</section>`
    },
    {
      id: 12,
      name: 'The wordmark',
      body: () => `
        <section class="c12-hero"><div class="c12-wordmark" aria-hidden="true">vela<span>.</span></div><div class="c12-opening"><h1>All your work.<br> <em>Right here.</em></h1><div><p>Code, conversation, terminal, and live diffs. A desktop workspace that brings OpenCode beside your files.</p>${cta()}</div></div></section>
        <section class="c12-product" id="workspace">${shot()}</section>
        <section class="c12-description"><h2>A familiar editor.<br> A different way<br> <em>to work.</em></h2><div><article><h3>Stay in the repository.</h3><p>Browse the file tree and edit with Monaco. Tabs and autosave keep the files you're working on close.</p></article><article><h3>Bring an agent along.</h3><p>Choose your OpenCode model and agent, add the relevant files, and follow the conversation beside your code.</p></article><article><h3>Read the result.</h3><p>Follow files touched during the session and review per-file diffs when starting contents are available.</p></article></div></section>`
    },
    {
      id: 13,
      name: 'Good company',
      body: () => `
        <section class="c13-hero"><h1>Your code.<br> In <em>good company.</em></h1><div class="c13-hero-bottom"><p>An editor for your files. OpenCode for the conversation. Vela keeps them in the same workspace.</p>${cta()}</div></section>
        <section class="c13-product" id="workspace">${shot()}</section>
        <section class="c13-reading"><h2>Work on the code.<br> Keep sight of<br> <em>the change.</em></h2><div><p>Open your repository, ask a question, and keep building. Your terminal and your agent stay beside the work.</p><p>Vela watches files touched during the session. Review the diff when starting contents are available, then edit the result yourself.</p><a class="c13-text-link" href="https://github.com/Tsohnle95/vela#setup" target="_blank" rel="noopener">Bring your repository ↗</a></div></section>
        <section class="c13-bottom"><article><h3>A real editor.</h3><p>Monaco, tabs, and autosave.</p></article><article><h3>A connected conversation.</h3><p>OpenCode sessions beside your files.</p></article><article><h3>A terminal at hand.</h3><p>Run commands in your workspace.</p></article></section>`
    },
    {
      id: 14,
      name: 'The working view',
      body: () => `
        <section class="c14-hero"><h1>One workspace.<br> <em>Every part of the work.</em></h1><p>Your editor and OpenCode, side by side.<br> With the terminal and live changes close at hand.</p>${cta()}</section>
        <section class="c14-workspace" id="workspace">${shot()}<fieldset class="c14-explore"><legend>Explore the workspace</legend><input type="radio" name="c14-view" id="c14-editor" checked><label for="c14-editor">Your editor</label><input type="radio" name="c14-view" id="c14-agent"><label for="c14-agent">Your agent</label><input type="radio" name="c14-view" id="c14-changes"><label for="c14-changes">Your changes</label><div class="c14-panels"><article class="c14-panel-editor"><h2>The code is still yours.</h2><p>Browse the repository and work in Monaco. Open tabs, edit files, and let autosave keep up while you work with an agent.</p></article><article class="c14-panel-agent"><h2>A conversation with context.</h2><p>Choose your model and agent, attach the relevant files, and follow OpenCode's streamed responses beside your editor.</p></article><article class="c14-panel-changes"><h2>Follow what changed.</h2><p>Vela watches files touched during the session. Open a per-file diff when starting contents are available, or see that a change was observed.</p></article></div></fieldset></section>
        <section class="c14-bottom"><article><h2>A terminal.<br> Where it belongs.</h2><p>Run commands in the current workspace. You can also switch an agent panel to the OpenCode TUI.</p></article><article><h2>Room for<br> more than one task.</h2><p>Tile up to eight agent sessions in one workspace, each with its own conversation and context.</p></article></section>`
    },
    {
      id: 15,
      name: 'A new angle',
      body: () => `
        <section class="c15-hero"><div class="c15-mark" aria-hidden="true">${mark()}</div><div class="c15-title">${copy('Make room<br> for <em>the work.</em>', 'Your code, your agent, and the space to bring them together. Meet Vela, a desktop workspace for OpenCode.')}</div><div class="c15-photo">${shot()}</div></section>
        <section class="c15-workflow" id="workspace"><h2>A clear path<br> from prompt<br> <em>to review.</em></h2><div><article><h3>Start with your repository.</h3><p>Open a folder and browse the files. Keep what you're working on in the editor.</p></article><article><h3>Work with OpenCode.</h3><p>Describe the change, attach the relevant files, and follow the agent beside your code.</p></article><article><h3>Review it. Run it.</h3><p>Check the changed files, edit the result, and use the integrated terminal to run your project.</p></article></div></section>
        <section class="c15-close"><h2>One window.<br> Your next change.</h2>${cta()}</section>`
    }
  );
})();
