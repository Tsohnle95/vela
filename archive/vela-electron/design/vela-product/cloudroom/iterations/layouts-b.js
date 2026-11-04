(() => {
  const { cta, shot, lead, story, details, repository } = window.Iteration;
  window.velaIterations.push(
    {
      id: 31, family: 1, name: 'The left bank',
      body: () => `<section class="i31-hero">${lead(1, 'Your code.<br> Your agents.<br> <em>Right here.</em>')}<div class="i31-product">${shot()}</div></section><section class="i31-follow"><h2>A whole session.<br> One place to follow it.</h2><div><p>Open a repository and give OpenCode a task. Keep editing in Monaco while the conversation stays beside your files.</p><a class="text-link" href="${repository}#setup" target="_blank" rel="noopener">Make yourself at home ↗</a></div></section><div class="i31-details">${details()}</div>`
    },
    {
      id: 32, family: 1, name: 'A shared desk',
      body: () => `<section class="i32-hero">${lead(1, 'The code.<br> The agent.<br> <em>One desk.</em>', 'An editor for your files. OpenCode for the conversation. A terminal to keep things moving.')}<div class="i32-product">${shot()}</div></section><section class="i32-story"><div><h2>Keep going.</h2><p>Open a file, ask for a change, and review the result in the same desktop workspace.</p></div><div><h2>Stay close.</h2><p>Attach the relevant files to your OpenCode session and follow the responses beside your editor.</p></div></section><div class="i32-details">${details()}</div>`
    },
    {
      id: 33, family: 1, name: 'The working spread',
      body: () => `<section class="i33-hero">${lead(1, 'Stay with<br> your code.<br> <em>Bring an agent.</em>', 'Give OpenCode the context. Keep your editor and the conversation in one place.')}<div class="i33-product">${shot()}</div></section><section class="i33-essay"><h2>The repository<br> comes first.</h2><p>Browse your files, open them in Monaco, and work with OpenCode in the same window. The workspace stays familiar, from the first prompt to the reviewed change.</p></section><section class="i33-workflow"><article><h3>Open the folder.</h3><p>Your file tree and editor are ready for the work.</p></article><article><h3>Start a conversation.</h3><p>Describe the task and attach the files that matter.</p></article><article><h3>Follow the result.</h3><p>See observed changes, edit the result, and run it in the terminal.</p></article></section>`
    },
    {
      id: 34, family: 1, name: 'The upper room',
      body: () => `<section class="i34-hero">${lead(1, 'More room<br> for <em>the work.</em>', 'Your repository, your OpenCode sessions, and the changes between them. All in one desktop workspace.')}<div class="i34-product">${shot()}</div></section><section class="i34-statement"><h2>Work on the code.<br> Keep sight of the change.</h2><p>Vela watches files touched during the session. Review a per-file diff when starting contents are available, then keep editing.</p></section><div class="i34-details">${details()}</div>`
    },
    {
      id: 35, family: 1, name: 'A familiar corner',
      body: () => `<section class="i35-hero">${lead(1, 'Your files.<br> Your thinking.<br> <em>Together.</em>', 'Work with OpenCode beside the files it changes. Keep the session together in Vela.')}<div class="i35-product">${shot()}</div></section><section class="i35-bridge"><h2>From a question<br> to a working change.</h2><div><p>Start with the repository. Give your agent the task, follow the conversation, and edit the result yourself.</p>${cta()}</div></section><div class="i35-details">${details()}</div>`
    },
    {
      id: 36, family: 2, name: 'The open view',
      body: () => `<section class="i36-hero">${lead(2, 'Keep the code.<br> <em>Open up the view.</em>', 'Your editor, OpenCode, and terminal. Together in the workspace you’re building in.')}${shot()}</section><div class="i36-story">${story('Less switching.<br> More staying with it.', 'Read the response. Open the file. Review the change. Vela keeps the conversation and the code close enough to work across both.')}</div>${details()}`
    },
    {
      id: 37, family: 2, name: 'The quiet stage',
      body: () => `<section class="i37-hero">${lead(2, 'A place for your code.<br> <em>And what comes next.</em>', 'Build with OpenCode in a desktop workspace that keeps the files, the conversation, and the changes together.')}${shot()}</section><section class="i37-follow"><h2>A conversation.<br> A working result.</h2><div><p>Choose your model and agent, attach the relevant files, and follow the session beside your editor.</p><p>Run your project from the integrated terminal. Keep building from the result.</p></div></section><div class="i37-details">${details()}</div>`
    },
    {
      id: 38, family: 2, name: 'The broad canvas',
      body: () => `<section class="i38-hero">${lead(2, 'Code with an agent.<br> <em>Keep your bearings.</em>', 'Your files are the starting point. Vela brings OpenCode, the editor, and your terminal into the same view.')}${shot()}</section><section class="i38-description"><h2>All the tools.<br> One working context.</h2><p>Browse the repository and work in Monaco. Keep an OpenCode session beside your code, follow the observed changes, and use the terminal without leaving the app.</p></section><div class="i38-details">${details()}</div>`
    },
    {
      id: 39, family: 2, name: 'The clear center',
      body: () => `<section class="i39-hero">${lead(2, 'Your whole workflow.<br> <em>Within reach.</em>', 'An editor for your code. A home for your OpenCode sessions. A terminal to keep moving.')}${shot()}</section><section class="i39-workflow"><h2>Pick up the work.<br> Keep it together.</h2><div><article><h3>Open your repository.</h3><p>Browse the file tree and edit with Monaco, tabs, and autosave.</p></article><article><h3>Bring your agent in.</h3><p>Ask OpenCode, attach files, and follow the conversation beside the editor.</p></article><article><h3>Review the result.</h3><p>Follow observed file changes and inspect per-file diffs when starting contents are available.</p></article></div></section>`
    },
    {
      id: 40, family: 2, name: 'The uninterrupted view',
      body: () => `<section class="i40-hero">${lead(2, 'Stay in the flow.<br> <em>Stay with the work.</em>', 'Code, conversation, and changes. Together in a desktop workspace for OpenCode.')}${shot()}</section><section class="i40-follow"><div><h2>Make a change.<br> See where it goes.</h2><p>Follow OpenCode’s responses beside your files. Review the changes, edit the result, and run your project in the terminal.</p></div><div class="i40-download"><h3>Your next working space.</h3>${cta()}</div></section><div class="i40-details">${details()}</div>`
    }
  );
})();
