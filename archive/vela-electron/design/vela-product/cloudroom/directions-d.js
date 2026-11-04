(() => {
  const { cta, shot, mark, copy, featureRow, repository } = window.VelaDesign;
  window.velaDirections.push(
    {
      id: 16,
      name: 'After hours',
      body: () => `<section class="after-hours"><div class="after-hours-title"><h1>Stay with<br> <em>the work.</em></h1><div><p>Your code and your agent belong in the same place. Vela brings your working session together.</p>${cta()}</div></div><div class="after-hours-picture">${shot()}<div class="after-hours-edge" aria-hidden="true"></div></div></section><section class="after-hours-story"><h2>From the first prompt<br> to the final diff.</h2><div><article><h3>Your repository is the starting point.</h3><p>Browse the files, open what you need, and edit with Monaco. Tabs and autosave keep the code close while you work.</p></article><article><h3>Your agent stays in the workspace.</h3><p>Describe a change to OpenCode, add the relevant files, and follow the response beside your editor.</p></article><article><h3>The changes stay in view.</h3><p>Follow the files touched during the session. Inspect a per-file diff when starting contents are available, edit the result, and run it in the terminal.</p></article></div></section><section class="after-hours-close"><h2>Make room<br> for your next change.</h2>${cta()}</section>`
    },
    {
      id: 17,
      name: 'Still point',
      body: () => `<section class="still-point"><div class="still-point-art" aria-hidden="true"><span class="still-point-ring"></span><div class="still-fold still-fold-left">${mark()}</div><div class="still-fold still-fold-right">${mark()}</div></div><div class="still-point-copy"><h1>A little space.<br> <em>A clearer view.</em></h1><p>An editor for your code. OpenCode beside it.<br> One workspace for the whole session.</p>${cta()}</div><div class="still-point-product">${shot()}</div></section><section class="still-point-story"><h2>Keep your tools<br> close to the code.</h2><div><p>Open a repository and get to work. Vela brings the file tree, Monaco editor, agent conversations, and integrated terminal into one desktop app.</p><p>Follow observed file changes as the session moves forward, then review per-file diffs when starting contents are available.</p><a class="still-point-link" href="${repository}#setup" target="_blank" rel="noopener">Set up your workspace <span aria-hidden="true">↗</span></a></div></section>${featureRow()}`
    },
    {
      id: 18,
      name: 'First look',
      body: () => `<section class="first-look"><div class="first-look-product">${shot()}</div><div class="first-look-copy"><h1>The whole session.<br> <em>Right here.</em></h1><div><p>Your editor, OpenCode conversations, and terminal. Keep the work together in Vela.</p>${cta()}</div></div></section><section class="first-look-features"><article><h2>Start with<br> the code.</h2><p>Browse your repository and open files in Monaco. Work across tabs and let autosave keep up with your edits.</p></article><article><h2>Give your agent<br> the context.</h2><p>Attach the relevant files, describe the change, and follow OpenCode responses beside the code you are working on.</p></article><article><h2>Follow it<br> through.</h2><p>Watch workspace changes, review per-file diffs when starting contents are available, and run your project in the integrated terminal.</p></article></section><section class="first-look-close"><div>${mark()}<h2>A better place<br> for your next session.</h2></div>${cta()}</section>`
    },
    {
      id: 19,
      name: 'Green room',
      body: () => `<section class="green-room"><div class="green-room-title"><h1>Your code.<br> Your agent.<br> <em>Your workspace.</em></h1>${cta()}</div><div class="green-room-product">${shot()}</div></section><section class="green-room-intro"><h2>Everything<br> <em>within reach.</em></h2><p>Vela brings your editor, OpenCode, terminal, and live file changes into one desktop workspace. Open a repository and stay with the work.</p></section><section class="green-room-features"><article><h3>Write in your editor.</h3><p>Monaco, tabs, and autosave, with your repository ready to browse.</p></article><article><h3>Work with OpenCode.</h3><p>Give your agent a task and follow the conversation in the same window.</p></article><article><h3>Review what changed.</h3><p>See observed file changes and per-file diffs when starting contents are available.</p></article><article><h3>Run it, right here.</h3><p>Use the integrated terminal in your current workspace.</p></article></section><section class="green-room-close"><p>Your next working session<br> has a place to call home.</p>${cta()}</section>`
    },
    {
      id: 20,
      name: 'On your desktop',
      body: () => `<section class="desktop-home"><div class="desktop-home-copy">${copy('Your coding agents.<br> Your own<br> <em>workspace.</em>', 'Code, ask, review, and run. Vela keeps OpenCode beside your editor, on your desktop.')}</div><div class="desktop-home-art"><div class="desktop-orbit" aria-hidden="true"></div><div class="desktop-floating-mark" aria-hidden="true">${mark()}</div><div class="desktop-machine">${shot()}<div class="desktop-keyboard" aria-hidden="true"><span></span></div><div class="desktop-machine-shadow" aria-hidden="true"></div></div></div></section><section class="desktop-home-workspace"><div><h2>Your code.<br> <em>In good company.</em></h2><p>Open a repository and keep the files, conversation, and changes in the same window. Vela gives OpenCode a place beside the code you are working on.</p></div>${shot()}</section>${featureRow()}<section class="desktop-home-close"><h2>Bring your repository.<br> We’ll make room.</h2><div>${cta()}<a href="${repository}#setup" target="_blank" rel="noopener">Read the setup guide <span aria-hidden="true">↗</span></a></div></section>`
    }
  );
})();
