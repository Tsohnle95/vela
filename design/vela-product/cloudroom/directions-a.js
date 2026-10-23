(() => {
  const { cta, shot, mark, copy, featureRow } = window.VelaDesign;
  const faq = () => `<section class="questions"><h2>A few good questions.</h2><div><details><summary>What is Vela?</summary><p>A desktop workspace that brings a Monaco editor, OpenCode sessions, an integrated terminal, and live file changes into one window.</p></details><details><summary>What do I need to run it?</summary><p>Have OpenCode V2 available on your PATH before starting an agent session. The source setup guide is available on GitHub.</p></details><details><summary>Can I follow more than one task?</summary><p>Agent mode tiles up to eight sessions. Each panel keeps its own conversation and context.</p></details></div></section>`;
  window.velaDirections.push(
    {
      id: 1, name: 'Home for the work',
      body: () => `<section class="home-hero">${copy('Your code.<br> Your agents.<br> <em>One home.</em>', 'Keep the editor, the conversation, and the changes in one desktop workspace.')}<div class="home-art"><div class="home-orbit orbit-one"></div><div class="home-orbit orbit-two"></div>${mark()}<div class="home-screen">${shot()}</div></div></section><section class="home-product"><h2>Stay with the work.</h2><p>Open a repository. Give OpenCode a task. Follow the changes beside your code.</p>${shot()}</section>${featureRow()}${faq()}`
    },
    {
      id: 2, name: 'Everything in view',
      body: () => `<section class="wide-hero">${copy('Code with an agent.<br> <em>Stay in your element.</em>', 'Your editor, OpenCode, and terminal. Close to the files you’re working on.')}${shot('wide-screen')}</section><section class="wide-story"><h2>Less moving around.<br> More moving forward.</h2><div><p>Keep the conversation next to your code. Open a file, ask for a change, and review the result without moving between apps.</p><a class="text-link" href="https://github.com/Tsohnle95/vela#setup" target="_blank" rel="noopener">See how to get started ↗</a></div></section>${featureRow()}${faq()}`
    },
    {
      id: 3, name: 'Close to the code',
      body: () => `<section class="close-hero"><div class="close-title"><h1>Keep your<br> <em>code close.</em></h1><div><p>Give your coding agent a place beside the files it works on.</p>${cta()}</div></div><div class="close-stage">${shot()}<div class="close-mark">${mark()}</div></div></section><section class="close-story"><h2>Your repository.<br> Your starting point.</h2><p>Browse your files, open them in Monaco, and work with OpenCode in the same window. The working session stays together.</p></section><div class="close-features">${featureRow()}</div>${faq()}`
    },
    {
      id: 4, name: 'A place to build',
      body: () => `<section class="place-hero"><div>${mark()}<h1>A place<br> to <em>build.</em></h1></div><div class="place-copy"><p>Your code, your agent, your next change. Vela brings them together on your desktop.</p>${cta()}<a class="text-link" href="https://github.com/Tsohnle95/vela" target="_blank" rel="noopener">Explore the source ↗</a></div></section><section class="place-product">${shot()}<div class="place-product-copy"><h2>Open the folder.<br> Start the conversation.</h2><p>Give OpenCode the relevant files, follow its responses, and keep editing while you work.</p></div></section>${featureRow()}${faq()}`
    },
    {
      id: 5, name: 'One working space',
      body: () => `<section class="space-hero">${copy('Make space<br> for your<br> <em>next idea.</em>', 'Code with OpenCode in a workspace built around your repository.')}<div class="space-art" aria-hidden="true">${Array.from({length:6},(_,i)=>`<div class="fold-orbit fold-${i}">${mark()}</div>`).join('')}</div></section><section class="space-product"><div><h2>The idea is yours.<br> The workspace is Vela.</h2><p>Editor, agent, terminal, and live file changes. Everything close enough to keep the work moving.</p></div>${shot()}</section><section class="space-story"><h2>A conversation.<br> A working result.</h2><div><p>Follow OpenCode’s responses and tool calls alongside your files. Review observed changes and inspect per-file diffs when starting contents are available.</p><p>Run the project from the integrated terminal, then keep building.</p></div></section>${faq()}`
    }
  );
})();
