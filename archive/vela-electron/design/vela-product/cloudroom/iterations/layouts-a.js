(() => {
  const { cta, shot, lead, story, details } = window.Iteration;
  const title = 'Your code.<br> Your agents.<br> <em>One home.</em>';
  const description = 'Keep your editor, the conversation, and the changes in one desktop workspace.';

  window.velaIterations.push(
    {
      id: 21, family: 1, name: 'Lower horizon',
      body: () => `<section class="horizon-hero">${lead(1)}<div class="horizon-image">${shot()}</div></section>${story()}<section class="horizon-product">${shot()}</section>${details()}`
    },
    {
      id: 22, family: 1, name: 'An open desk',
      body: () => `<section class="open-desk-hero">${lead(1)}${shot()}</section><section class="open-desk-story"><div><h2>A place for the whole session.</h2><p>Open your repository, describe the task, and work with OpenCode beside your files.</p></div><div><h2>Keep the next step close.</h2><p>Review observed changes, edit the result, and run your project in the integrated terminal.</p></div></section><section class="open-desk-product">${shot()}</section>${details()}`
    },
    {
      id: 23, family: 1, name: 'A broader view',
      body: () => `<section class="broader-hero">${lead(1, 'Your work.<br> Your agent.<br> <em>One place.</em>')}<div class="broader-image">${shot()}</div></section><section class="broader-story"><h2>Less moving around.<br> More moving forward.</h2><p>Vela brings the editor, OpenCode, terminal, and live file changes together. Keep following the task without losing your place.</p></section>${shot()}${details()}`
    },
    {
      id: 24, family: 1, name: 'A little room',
      body: () => `<section class="room-hero">${lead(1)}<div class="room-image">${shot()}</div></section><section class="room-story"><p>Start with the files.<br> Stay with the work.</p><div><h2>Your repository is the starting point.</h2><p>Browse the tree, open files in Monaco, and give OpenCode the context it needs. The conversation stays beside the code.</p></div></section>${shot()}${details()}`
    },
    {
      id: 25, family: 1, name: 'Quiet balance',
      body: () => `<section class="balance-hero">${lead(1)}${shot()}</section><section class="balance-story"><h2>Tools that belong together.</h2><p>A code editor you can work in. An agent you can follow. A terminal in the current workspace. Vela gives the session a place to happen.</p></section><section class="balance-product">${shot()}</section>${details()}`
    },
    {
      id: 26, family: 1, name: 'At the edge',
      body: () => `<section class="edge-hero">${lead(1)}<div class="edge-image">${shot()}</div></section>${story('The files. The conversation.<br> The same context.', 'Work with OpenCode in the same window as your editor. Follow the response, open a file, and review the changes as you go.')}<section class="edge-product">${shot()}</section>${details()}`
    },
    {
      id: 27, family: 1, name: 'Close company',
      body: () => `<section class="company-hero">${lead(1, 'Your code.<br> Your agent.<br> <em>Close company.</em>', 'Keep the conversation next to the files. Vela brings OpenCode into your desktop workspace.')}<div class="company-image">${shot()}</div></section><section class="company-story"><h2>Ask. Read. Edit.</h2><div><p>Describe what you want to change and attach the relevant files. Follow OpenCode’s streamed responses beside your code.</p><p>Review observed file changes and inspect per-file diffs when starting contents are available.</p></div></section>${shot()}${details()}`
    },
    {
      id: 28, family: 1, name: 'A clear opening',
      body: () => `<section class="opening-hero">${lead(1)}${shot()}</section><section class="opening-story"><div><h2>Start with your repository.</h2><p>Open the folder and get to work. Browse files, use Monaco, and keep an OpenCode session beside your editor.</p></div>${shot()}</section>${details()}<section class="opening-close"><h2>Make room for your next change.</h2>${cta()}</section>`
    },
    {
      id: 29, family: 1, name: 'The shared line',
      body: () => `<section class="sharedline-hero"><div class="sharedline-title"><h1>${title}</h1></div><div class="sharedline-image">${shot()}</div><div class="sharedline-copy"><p>${description}</p>${cta()}</div></section>${story('One workspace.<br> Every part of the work.', 'Give the task to OpenCode, keep editing, and follow what changes. Your files, conversation, and terminal remain within reach.')}<section class="sharedline-product">${shot()}</section>${details()}`
    },
    {
      id: 30, family: 1, name: 'The working day',
      body: () => `<section class="workingday-hero">${lead(1, 'Your files.<br> Your flow.<br> <em>Your workspace.</em>')}<div class="workingday-image">${shot()}</div></section><section class="workingday-story"><h2>Stay in the session.</h2><div><p>Your editor and your coding agent can share the same window. Work in Monaco while you follow OpenCode’s responses, reasoning, and tool calls.</p><p>Keep the integrated terminal close, then review observed changes before moving on.</p></div></section>${shot()}${details()}`
    }
  );
})();
