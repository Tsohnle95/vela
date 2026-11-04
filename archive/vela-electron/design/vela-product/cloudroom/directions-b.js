(() => {
  const { cta, shot, mark, copy, featureRow } = window.VelaDesign;

  window.velaDirections.push(
    {
      id: 6,
      name: 'The big picture',
      body: () => `
        <section class="cinema-intro">
          <h1>The whole session.<br><em>In one window.</em></h1>
          <div><p>Your editor, your agent, and the changes between them. Vela keeps the work together.</p>${cta()}</div>
        </section>
        <div class="cinema-screen">${shot()}</div>
        <section class="cinema-after">
          <div>${mark()}<h2>Stay with<br>the work.</h2></div>
          <div><p>Open your repository. Work with OpenCode beside your files. Review the changes without losing the thread.</p><a class="text-link" href="https://github.com/Tsohnle95/vela#setup" target="_blank" rel="noopener">Get your workspace ready <span aria-hidden="true">↗</span></a></div>
        </section>
        ${featureRow()}`
    },
    {
      id: 7,
      name: 'Folded space',
      body: () => `
        <section class="folded-studio">
          ${copy('Make room.<br><em>Make something.</em>', 'Your code and OpenCode, side by side. A desktop workspace with room for the whole session.')}
          <div class="folded-object" aria-hidden="true"><div class="folded-floor"></div>${mark()}<div class="folded-shadow"></div></div>
        </section>
        <section class="folded-workspace">
          <div><h2>A familiar editor.<br><em>A different way to work.</em></h2><p>Browse files, write code with Monaco, and ask OpenCode to work beside you. Your repository stays at the center.</p></div>
          ${shot()}
        </section>
        <section class="folded-features">
          <article><h2>Ask in context.</h2><p>Attach the files that matter. Follow streamed responses, reasoning, and tool calls as the agent works.</p></article>
          <article><h2>Keep your hands on the code.</h2><p>Edit the result in Monaco, review per-file diffs, and run your project in the integrated terminal.</p></article>
        </section>`
    },
    {
      id: 8,
      name: 'Code, together',
      body: () => `
        <section class="together-title"><h1>Code.<br><em>Together.</em></h1><div>${mark()}<p>A home for your code.<br>A place for your agent.</p></div></section>
        <section class="together-workspace">
          <div class="together-copy"><p>Vela brings OpenCode into the same workspace as your editor. Read the response, open the file, and follow the change.</p>${cta()}</div>
          ${shot()}
        </section>
        <section class="together-story"><h2>From a thought<br>to a working change.</h2><div><article><h3>Give the task a place.</h3><p>Open a repository and bring its files into the conversation with OpenCode.</p></article><article><h3>Follow it through.</h3><p>Watch workspace changes, check per-file diffs, and edit what comes next.</p></article><article><h3>Run the result.</h3><p>The integrated terminal keeps your commands in the current workspace.</p></article></div></section>`
    },
    {
      id: 9,
      name: 'The shared workspace',
      body: () => `
        <section class="shared-workspace">
          <div class="shared-statement"><h1>Your code.<br>Your agent.<br><em>Your space.</em></h1><p>A desktop workspace for the way you build with OpenCode.</p>${cta()}</div>
          <div class="shared-product">${shot()}<div class="shared-under"><p>Write it.<br>Ask about it.<br>Keep it moving.</p>${mark()}</div></div>
        </section>
        <section class="shared-edit"><h2>Close to the code.<br><em>From start to finish.</em></h2><p>Browse the repository. Work with your agent. Review observed changes and inspect per-file diffs when starting contents are available. Everything stays in the same desktop workspace.</p></section>
        ${featureRow()}`
    },
    {
      id: 10,
      name: 'In plain view',
      body: () => `
        <section class="plain-view">
          <div class="plain-product">${shot()}</div>
          <div class="plain-title"><h1>Keep the work<br><em>in plain view.</em></h1><div><p>Code, conversation, and changes. One desktop app for working with OpenCode.</p>${cta()}</div></div>
        </section>
        <section class="plain-editor"><div>${mark()}<h2>Your editor.<br>Your decisions.</h2></div><p>Vela brings the agent into your workspace. You can open the files, edit with Monaco, and review the changes as the session unfolds.</p></section>
        <section class="plain-capabilities"><article><h3>More than one conversation.</h3><p>Tile up to eight agent sessions. Each panel keeps its own conversation and context.</p></article><article><h3>A terminal that belongs here.</h3><p>Run commands in the current workspace or switch an agent panel to the OpenCode TUI.</p></article><article><h3>The files are always close.</h3><p>Browse the repository tree, open tabs, and keep working with autosave.</p></article></section>`
    }
  );
})();
