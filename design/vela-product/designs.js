const designs = [
  ['Folded light', 'Your code.<br><em>Your workspace.</em>', 'Your editor, AI agents, terminal, and live diffs.<br>One desktop app. Powered by OpenCode.'],
  ['Terminal', 'Less switching.<br><em>More building.</em>', 'Edit files. Run OpenCode. Review the diff.<br>Keep the whole session in one window.'],
  ['Editorial', 'A place for<br><em>your code.</em>', 'A code editor, an agent, and a terminal.<br>Vela brings them together on your desktop.'],
  ['Blueprint', 'One workspace.<br><em>Every moving part.</em>', 'Your repository, editor, terminal, and OpenCode sessions.<br>Connected in one desktop app.'],
  ['Mint studio', 'Make room<br><em>to build.</em>', 'Keep your code and your agent in the same workspace.<br>Edit, ask, review, and run with Vela.'],
  ['Spotlight', 'Code with<br><em>a clearer view.</em>', 'OpenCode beside your editor.<br>Live diffs beside your decisions.'],
  ['Swiss', 'CODE. ASK.<br><em>REVIEW. RUN.</em>', 'One desktop workspace for your repository,<br>your terminal, and your coding agents.'],
  ['Two surfaces', 'Everything you need.<br><em>In one window.</em>', 'An editor for your code. A home for your agents.<br>A terminal to keep things moving.'],
  ['Lilac journal', 'Keep your code<br><em>in good company.</em>', 'Vela brings OpenCode into your workspace,<br>with your editor, terminal, and diffs close by.'],
  ['Monochrome', 'YOUR NEXT<br><em>WORKSPACE.</em>', 'Code editor. OpenCode agents. Integrated terminal.<br>Live diffs. Together in Vela.'],
  ['Horizon', 'Space to think.<br><em>Tools to build.</em>', 'Work with your code and OpenCode side by side.<br>Keep the editor, terminal, and changes in view.'],
  ['Dot matrix', 'Open folder.<br><em>Start building.</em>', 'Browse the repository. Ask your agent.<br>Read the diff. Run it in the terminal.'],
  ['Signal', 'Your agents.<br><em>In context.</em>', 'OpenCode sessions beside the files they work on.<br>One desktop app for the whole workflow.'],
  ['Terracotta', 'Good work<br><em>starts here.</em>', 'Your code, your terminal, and your agent.<br>Vela keeps the workspace together.'],
  ['Mint belt', 'From prompt<br><em>to working code.</em>', 'Ask OpenCode, follow the changes, and edit the result.<br>All inside the same desktop workspace.'],
  ['Prism', 'A new angle<br><em>on your workspace.</em>', 'Your editor and OpenCode, side by side.<br>With a terminal and live diffs built in.'],
  ['Ledger', 'The work.<br><em>All in view.</em>', 'Repository, code, agents, changes.<br>One considered desktop workspace.'],
  ['Desktop', 'Meet your<br><em>next workspace.</em>', 'A desktop app for coding with an AI agent.<br>Built around your files, terminal, and live diffs.'],
  ['Lemon', 'BUILD<br><em>RIGHT HERE.</em>', 'Keep your code, agents, and terminal together.<br>Vela is your desktop workspace for OpenCode.'],
  ['Ink', 'Code, with<br><em>room to think.</em>', 'An editor, OpenCode agents, and a terminal.<br>A single place to move the work forward.'],
  ['Dusk', 'Stay with<br><em>the work.</em>', 'Your files. Your agents. Your terminal.<br>Bring the session together in Vela.'],
  ['Film', 'See the code.<br><em>Follow the change.</em>', 'Work with OpenCode in the same window as your editor.<br>Review live file changes as you go.'],
  ['Evergreen', 'Your workspace.<br><em>Reassembled.</em>', 'Editor, agent chat, terminal, and diffs.<br>The tools you use, in one desktop app.'],
  ['Source', 'Build with<br><em>context.</em>', 'Keep OpenCode beside your repository.<br>Write, review, and run without leaving Vela.'],
  ['Quiet mark', 'One place.<br><em>Your whole workflow.</em>', 'Code with OpenCode, edit your files, and review changes.<br>Your desktop workspace, brought together.'],
];
const requested = Number(new URLSearchParams(location.search).get('design'));
let current = Number.isInteger(requested) && requested >= 1 && requested <= designs.length ? requested : 1;
function render() {
  const [name, title, description] = designs[current - 1];
  document.body.className = `design-${current}`;
  document.getElementById('headline').innerHTML = title;
  document.getElementById('description').innerHTML = description;
  document.title = `Vela — ${name}`;
  const url = new URL(location.href);
  url.searchParams.set('design', String(current));
  history.replaceState(null, '', url);
  window.scrollTo(0, 0);
}
document.addEventListener('keydown', (event) => {
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || /INPUT|TEXTAREA|SELECT/.test(event.target.tagName) || event.target.isContentEditable) return;
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
  event.preventDefault();
  current = (current - 1 + (event.key === 'ArrowRight' ? 1 : -1) + designs.length) % designs.length + 1;
  render();
});
render();
