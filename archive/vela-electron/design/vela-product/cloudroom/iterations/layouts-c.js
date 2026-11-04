(() => {
  const { cta, shot, lead, story, details, repository } = window.Iteration;
  const setup = () => `<a class="iteration-link" href="${repository}#setup" target="_blank" rel="noopener">Read the setup guide <span aria-hidden="true">↗</span></a>`;
  window.velaIterations.push(
    {
      id: 41, family: 2, name: 'A centered workspace',
      body: () => `<section class="centered-workspace">${lead(2, 'Your code. Your agent.<br> <em>Your working space.</em>', 'Keep OpenCode beside the files you’re working on. Edit, ask, review, and run in Vela.')}${shot()}</section>${details()}<section class="centered-workspace-close"><h2>Bring your repository.<br> Make yourself at home.</h2><div>${cta()}${setup()}</div></section>`
    },
    {
      id: 42, family: 2, name: 'The wider view',
      body: () => `<section class="wider-view">${lead(2, 'The whole session.<br> <em>A clearer view.</em>', 'Your editor, the conversation, and the changes. Together in one desktop workspace.')}${shot()}</section><section class="wider-view-story"><h2>Less switching.<br> More building.</h2><div><p>Browse your repository and work in Monaco. Give OpenCode the relevant files, follow the response, and keep editing without leaving the workspace.</p>${setup()}</div></section><div class="wider-view-details">${details()}</div><section class="wider-view-close"><p>Stay close to the code.</p>${cta()}</section>`
    },
    {
      id: 43, family: 2, name: 'Close to the work',
      body: () => `<section class="close-work"><div class="hero-copy"><h1>Work with an agent.<br> <em>Stay close to the code.</em></h1>${cta()}</div>${shot()}<p class="close-work-description">Vela keeps your editor, OpenCode, and terminal in the same workspace as the files you’re working on.</p></section><section class="close-work-story"><h2>Your repository is<br> the common ground.</h2><p>Open a folder, browse the files, and start a conversation with OpenCode. Follow its responses beside your editor and review observed changes as the session moves forward.</p></section>${details()}<section class="close-work-close">${setup()}${cta()}</section>`
    },
    {
      id: 44, family: 2, name: 'One clear window',
      body: () => `<section class="clear-window"><div class="hero-copy"><h1>A clearer view.<br> <em>Of your next change.</em></h1><div class="clear-window-intro"><p>Code with OpenCode in a workspace built around your repository.</p>${cta()}</div></div>${shot()}</section><section class="clear-window-story"><div><h2>Everything close<br> enough to keep moving.</h2>${setup()}</div><div><article><h3>Keep the conversation beside the code.</h3><p>Give OpenCode the files it needs and follow streamed responses in the same window as your editor.</p></article><article><h3>Take the change through to review.</h3><p>Follow observed file changes and inspect per-file diffs when starting contents are available. Run the result in the integrated terminal.</p></article></div></section>`
    },
    {
      id: 45, family: 2, name: 'Room to follow through',
      body: () => `<section class="follow-through">${lead(2, 'Make space for<br> <em>your next change.</em>', 'An editor, an agent, and a terminal. A single place to follow the work forward.')}${shot()}</section><section class="follow-through-story"><h2>The folder is yours.<br> The workspace is Vela.</h2><p>Browse your repository with Monaco, work with OpenCode beside your files, and keep your terminal in the same window.</p></section><div class="follow-through-details">${details()}</div><section class="follow-through-close"><h2>Ready for your next session.</h2>${cta()}</section>`
    },
    {
      id: 46, family: 2, name: 'The working session',
      body: () => `<section class="working-session"><div class="hero-copy"><h1>Your code, in context.<br> <em>Your session, together.</em></h1><p>OpenCode and your editor. In the same window.</p>${cta()}</div>${shot()}</section><section class="working-session-intro"><p>Open a repository. Describe the change.<br> Stay with the work.</p><div><h2>A familiar editor.<br> An agent alongside it.</h2><p>Browse your files and edit with Monaco, tabs, and autosave. Attach the relevant files to an OpenCode session and follow its responses beside the code.</p>${setup()}</div></section><div class="working-session-review"><h2>See what the session changed.</h2><p>Vela follows observed workspace changes. Review a per-file diff when starting contents are available, edit the result, and run your project from the integrated terminal.</p></div>`
    },
    {
      id: 47, family: 2, name: 'Everything comes together',
      body: () => `<section class="comes-together"><div class="hero-copy"><h1>The work is yours.<br> <em>Keep it together.</em></h1><p>Your editor, OpenCode, and terminal. One workspace for your next working session.</p></div>${shot()}<div class="comes-together-actions">${cta()}${setup()}</div></section>${story('From the first prompt<br> to the reviewed change.', 'Open your repository and work with OpenCode beside your files. Follow observed changes and inspect per-file diffs when starting contents are available.')}<div class="comes-together-details">${details()}</div>`
    },
    {
      id: 48, family: 2, name: 'The shared workspace',
      body: () => `<section class="shared-workspace"><div class="hero-copy"><h1>Your files. Your agent.<br> <em>A shared workspace.</em></h1><p>Work with OpenCode where the code already is.</p>${cta()}</div>${shot()}</section><section class="shared-workspace-story"><article><h2>An editor you<br> can stay in.</h2><p>Open files with Monaco, move between tabs, and let autosave keep up while you work.</p></article><article><h2>An agent you<br> can work beside.</h2><p>Give OpenCode the relevant files and follow the conversation without losing the view of your repository.</p></article></section><section class="shared-workspace-review"><div><h2>Follow the changes.<br> Review the result.</h2><p>See observed file changes and review per-file diffs when starting contents are available. Then run it in the integrated terminal.</p></div>${setup()}</section>`
    },
    {
      id: 49, family: 2, name: 'A place for the session',
      body: () => `<section class="place-for-session">${lead(2, 'Code with your agent.<br> <em>Build in your workspace.</em>', 'A desktop app for the files, conversations, and changes that make up your working session.')}${shot()}</section><section class="place-for-session-story"><div><h2>The session stays<br> with the repository.</h2><p>Browse the file tree and edit with Monaco. Work with OpenCode in the same window and use the terminal in your current workspace.</p></div><div><h2>The changes stay<br> within view.</h2><p>Follow observed workspace changes and review per-file diffs when starting contents are available.</p></div></section><section class="place-for-session-close"><h2>A place to move the work forward.</h2><div>${cta()}${setup()}</div></section>`
    },
    {
      id: 50, family: 2, name: 'Stay in your element',
      body: () => `<section class="your-element">${lead(2)}${shot()}</section><section class="your-element-story"><h2>Your code, your conversation,<br> and the space between.</h2><p>Open a repository and work with OpenCode beside your files. Keep editing in Monaco, follow the changes, and run the project in the integrated terminal.</p></section><div class="your-element-details">${details()}</div><section class="your-element-close"><h2>Make room for the work.</h2><p>Bring your repository. Connect OpenCode. Start your next session in Vela.</p>${cta()}</section>`
    }
  );
})();
