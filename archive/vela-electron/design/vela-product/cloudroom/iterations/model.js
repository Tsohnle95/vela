(() => {
  const { cta, shot, copy, featureRow, repository } = window.VelaDesign;
  window.velaIterations = window.velaDirections.filter(d => d.id === 1 || d.id === 2).map(d => ({ ...d, family: d.id, reference: true }));
  const heading = (family, title) => title || (family === 1 ? 'Your code.<br> Your agents.<br> <em>One home.</em>' : 'Code with an agent.<br> <em>Stay in your element.</em>');
  const lead = (family, title, description) => copy(heading(family, title), description || (family === 1 ? 'Keep your editor, the conversation, and the changes in one desktop workspace.' : 'Your editor, OpenCode, and terminal. Close to the files you’re working on.'));
  const story = (title = 'Stay with the work.', description = 'Open a repository. Work with OpenCode beside your files. Review observed changes and inspect per-file diffs when starting contents are available.') => `<section class="iteration-story"><h2>${title}</h2><p>${description}</p></section>`;
  const details = () => `<section class="iteration-details"><article><h2>Your editor.</h2><p>Browse the repository and edit with Monaco, tabs, and autosave.</p></article><article><h2>Your agent.</h2><p>Give OpenCode the relevant files and follow its responses beside your code.</p></article><article><h2>Your next change.</h2><p>Follow observed file changes, review the diff when starting contents are available, and run your project in the terminal.</p></article></section>`;
  window.Iteration = { cta, shot, copy, featureRow, repository, lead, story, details };
})();
