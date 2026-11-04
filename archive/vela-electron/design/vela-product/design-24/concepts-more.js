(() => {
  const { label, actions, shot, intro, features, note, steps } = window.sourceFragments;
  window.sourceConcepts.push(
    {
      name: 'Code poetry', motif: '11 / executable typography',
      source: 'Source turns a single word into syntax with its braces and monospaced accent.',
      change: 'An entire short, illustrative code declaration becomes the headline, with a numbered gutter and the screenshot nested into the return block.',
      body: () => `<section class="poetry">${label('A SMALL DECLARATION OF INTENT')}<div class="code-poem"><div><span class="line-number">01</span><span>function <b>build</b>() {</span></div><div><span class="line-number">02</span><h1>return <em>context;</em></h1></div><div class="poem-shot"><span class="line-number">03</span>${shot('Your code, your agent, and the space between.')}</div><div><span class="line-number">04</span><span>}</span></div></div><div class="poetry-bottom"><p class="lede">A desktop workspace for your editor, OpenCode, and terminal.</p>${actions()}</div></section>${features()}`
    },
    {
      name: 'Desktop mat', motif: '12 / an object on a surface',
      source: 'The original screenshot rests in a pale gray protective frame against a white grid.',
      change: 'The frame expands into a physical-looking sage desk mat with a paper title slip, inset app image, and a horizontal inventory beneath.',
      body: () => `<section class="desk-mat"><div class="desk-slip">${label('VELA / DESKTOP WORKSPACE')}${intro('A place for<br> <em>your next change.</em>')}</div>${shot('Your repository, at the center of the desk.')}<div class="desk-inventory"><span>01 / MONACO</span><span>02 / OPENCODE</span><span>03 / TERMINAL</span></div></section>${steps()}`
    },
    {
      name: 'The colophon', motif: '13 / publication cover',
      source: 'Source’s hierarchy places an oversized title above quiet technical metadata.',
      change: 'A publication cover inverts the relationship: huge Vela masthead, a thin bibliographic strip, an inset image, and a small serif thesis in the corner.',
      body: () => `<section class="colophon"><div class="colophon-meta"><span>A WORKSPACE FOR CODING AGENTS</span><span>MACOS / OPENCODE V2</span></div><h1>Vela<span>.</span></h1><div class="colophon-body">${shot('FIG. 01 / A shared context for code and conversation.')}<div class="colophon-thesis">${label('THE CONTEXT ISSUE')}<h2>Build with<br> <em>everything<br> in view.</em></h2><p>Editor, agent, terminal, and live diffs. Together on your desktop.</p>${actions()}</div></div></section>${features()}`
    },
    {
      name: 'Command palette', motif: '14 / task-first interface',
      source: 'Source borrows the language of code but presents conventional website navigation.',
      change: 'A functional command card organizes the landing page around three tasks; choosing one changes the product annotation and the visible detail panel.',
      body: () => `<section class="palette-layout"><div>${label('YOUR NEXT WORKSPACE')}${intro('What will you<br> <em>build next?</em>')}</div><div class="command-card"><div class="command-top"><span>vela / workspace</span><span>⌘</span></div><p class="command-question">Start with the work.</p><div class="task-options" role="group" aria-label="Explore workspace tasks"><button data-task="edit" aria-pressed="true"><span>01</span> Edit a file <b>↗</b></button><button data-task="ask" aria-pressed="false"><span>02</span> Work with an agent <b>↗</b></button><button data-task="review" aria-pressed="false"><span>03</span> Review a change <b>↗</b></button></div><p id="task-detail" aria-live="polite">Browse your repository and edit with Monaco, tabs, and autosave.</p></div></section><section class="palette-product">${shot('Choose a task above to explore the workspace.')}</section>${features()}`
    },
    {
      name: 'Context window', motif: '15 / an aperture in paper',
      source: 'Source uses the app image as a dark rectangular anchor in an otherwise very light page.',
      change: 'An oversized rounded aperture dominates a sage page, with a small centered title floating above and four quiet corner annotations framing the view.',
      body: () => `<section class="aperture"><div class="aperture-heading">${label('A CLEARER VIEW OF THE WORK')}<h1>Open up<br> <em>your context.</em></h1></div><div class="aperture-view"><span class="corner tl">REPOSITORY</span><span class="corner tr">CONVERSATION</span>${shot('Code and conversation, through the same window.')}<span class="corner bl">EDITOR</span><span class="corner br">CHANGES</span></div><div class="aperture-actions"><p class="lede">Your editor and OpenCode, side by side.</p>${actions()}</div></section>${features()}`
    },
    {
      name: 'Typesetter', motif: '16 / measured asymmetry',
      source: 'The original mixes proportional and monospaced type, allowing one green word to carry the whole identity.',
      change: 'An exaggerated left-aligned typographic staircase uses three scales and measured horizontal rules; a small product sits across the bottom like a printer’s proof.',
      body: () => `<section class="typesetter">${label('TYPE / TOOLS / CONTEXT')}<div class="type-line small-type">Your code.</div><div class="type-line middle-type">Your agent.</div><h1 class="type-line big-type">Your <em>context.</em></h1><div class="type-foot"><div><p class="lede">A considered workspace for your repository, OpenCode, and the changes between them.</p>${actions()}</div>${shot('VELA / DESKTOP PROOF')}</div></section>${steps()}`
    },
    {
      name: 'Session ledger', motif: '17 / structured rows',
      source: 'The small // labels and technical frame in Source suggest an orderly record of work.',
      change: 'A full-width ledger uses large rows and asymmetric columns, with a product image spanning the central record and a green total row for the call to action.',
      body: () => `<section class="ledger">${label('WORKSPACE LEDGER / VELA')}<div class="ledger-row ledger-heading"><span>00</span><h1>A session.<br> <em>All accounted for.</em></h1><p>Keep the whole workflow<br> in the same window.</p></div><div class="ledger-row"><span>01</span><h2>The workspace</h2><p>Repository + editor + OpenCode</p></div><div class="ledger-image">${shot('One desktop app. The working session, in view.')}</div><div class="ledger-row"><span>02</span><h2>The changes</h2><p>Observed files + per-file diffs</p></div><div class="ledger-total"><span>YOUR NEXT SESSION</span>${actions()}</div></section>`
    },
    {
      name: 'Sage pavilion', motif: '18 / architectural section',
      source: 'Source’s flat grid and detached product frame feel architectural, like a plan and an elevation.',
      change: 'Two broad green columns and a thin roof line form a pavilion around the screenshot; the title is a restrained architectural caption rather than an oversized central block.',
      body: () => `<section class="pavilion"><div class="pavilion-heading"><div>${label('A STRUCTURE FOR YOUR WORK')}<h1>Room to build.<br> <em>Space for context.</em></h1></div><p class="lede">Your repository is the foundation.<br> Vela brings the tools together.</p></div><div class="pavilion-structure"><div class="pillar"><span>EDITOR</span></div>${shot('Vela / a shared working environment')}<div class="pillar"><span>AGENT</span></div></div><div class="pavilion-base"><span>PLAN 01 / DESKTOP WORKSPACE</span>${actions()}</div></section>${features()}`
    },
    {
      name: 'Source notebook', motif: '19 / facing pages',
      source: 'The white technical canvas and monospaced accents make Source resemble a clean developer’s notebook.',
      change: 'A double-page spread pairs handwritten-style editorial hierarchy with a numbered product plate, followed by a full-width ruled note strip.',
      body: () => `<section class="notebook"><div class="notebook-left">${label('01 / THE IDEA')}<h1>Keep the code.<br> Keep the<br> <em>conversation.</em></h1><p class="lede">And keep them close. Vela is a desktop workspace for coding with OpenCode.</p>${actions()}<span class="page-number">01</span></div><div class="notebook-right">${label('02 / THE WORKSPACE')}${shot('A note beside the file it belongs to.')}<p class="notebook-note">Context is the connection<br> between the two.</p><span class="page-number">02</span></div></section><div class="notebook-rule">${features()}</div>`
    },
    {
      name: 'Focus rails', motif: '20 / a product with a sidebar',
      source: 'Source’s product image is informative but secondary to the central headline.',
      change: 'The composition becomes a product-first rail layout: a permanent narrow brand spine, huge workspace image, and a short bottom strip of copy and actions.',
      body: () => `<section class="focus-rails"><aside><span>VELA</span><span>EDITOR / AGENT / TERMINAL</span><a href="https://github.com/Tsohnle95/vela">SOURCE ↗</a></aside><div><div class="focus-heading">${label('A DESKTOP WORKSPACE FOR OPENCODE')}<h1>Stay <em>in context.</em></h1></div>${shot('Work across your code and your agent in one window.')}<div class="focus-bottom"><p>Your repository.<br> Your working session.<br> One place to follow it.</p>${actions()}</div></div></section>${steps()}`
    },
    {
      name: 'Assembly drawing', motif: '21 / separated layers',
      source: 'The braces in Source put a conceptual enclosure around the word “context.”',
      change: 'The enclosure is unpacked into three stacked planes: repository, tools, and conversation. An exploded paper diagram accompanies a split hero and a flat screenshot.',
      body: () => `<section class="assembly"><div>${label('THE PARTS, BROUGHT TOGETHER')}${intro('A workspace.<br> <em>Assembled.</em>')}</div><div class="assembly-diagram" aria-label="Repository, editor, and agent layers"><div><span>03</span> OPENCODE <b>conversation</b></div><div><span>02</span> MONACO <b>editor</b></div><div><span>01</span> REPOSITORY <b>context</b></div></div></section><div class="assembly-arrow">↓<span>TOGETHER IN VELA</span></div>${shot('The layers become one working environment.')}${features()}`
    },
    {
      name: 'Context index', motif: '22 / typographic directory',
      source: 'Source connects “context” with a repository but uses a conventional feature row to explain the product.',
      change: 'A directory of large linked words is the primary layout, each opening an actual feature explanation; the app image occupies the directory’s second column.',
      body: () => `<section class="context-index"><div class="index-title">${label('VELA / A DIRECTORY OF POSSIBILITIES')}<h1>Context<br> for <em>the work.</em></h1></div><div class="index-links"><details open><summary><span>01</span> Code <b>+</b></summary><p>Browse your repository, open tabs, and edit with Monaco and autosave.</p></details><details><summary><span>02</span> Conversation <b>+</b></summary><p>Ask OpenCode, attach workspace files, and follow streamed responses beside the editor.</p></details><details><summary><span>03</span> Changes <b>+</b></summary><p>Watch observed file changes and review per-file diffs when starting contents are available.</p></details><details><summary><span>04</span> Commands <b>+</b></summary><p>Run the project in the integrated terminal, in the current workspace.</p></details></div><div class="index-product">${shot('Code, conversation, changes, commands.')}${actions()}</div></section>`
    },
    {
      name: 'Contour', motif: '23 / a topography of context',
      source: 'The repeated grid in Source sets a very quiet environmental texture behind a sharply framed app.',
      change: 'Custom vector contour lines replace the rectangular grid; a low, left-aligned hero and a broad white product island create a landscape-like composition.',
      body: () => `<section class="contour"><svg class="contour-lines" viewBox="0 0 1200 650" preserveAspectRatio="none" aria-hidden="true"><g fill="none" stroke="currentColor">${Array.from({length:12},(_,i)=>`<path d="M ${-100+i*22} 650 C ${200+i*20} ${300-i*22}, ${500+i*40} ${700-i*35}, ${1000+i*27} -100"/>`).join('')}</g></svg><div>${label('THE TERRAIN OF YOUR NEXT CHANGE')}${intro('Know your<br> <em>working ground.</em>')}</div></section><div class="contour-island">${shot('Your repository. A familiar place to build.')}${features()}</div>`
    },
    {
      name: 'Syntax garden', motif: '24 / an organic technical field',
      source: 'Source’s muted green and braces are an unusual meeting of natural calm and code syntax.',
      change: 'Oversized bracket leaves grow around a small circular text clearing, while a grounded horizontal product plate gives the organic composition a technical base.',
      body: () => `<section class="garden"><div class="garden-glyph left" aria-hidden="true">{</div><div class="garden-center">${label('MAKE ROOM FOR THE WORK')}<h1>A little space.<br> <em>A lot of context.</em></h1><p class="lede">Keep your code and OpenCode in the same workspace.</p>${actions()}</div><div class="garden-glyph right" aria-hidden="true">}</div></section><div class="garden-ground">${shot('Rooted in your repository. Built for the whole session.')}<div class="garden-labels"><span>CODE</span><span>CONVERSATION</span><span>CHANGES</span></div></div>${steps()}`
    },
    {
      name: 'Dockside', motif: '25 / a desktop at the waterline',
      source: 'The original screenshot sits below the fold, introduced by a compact technical bar.',
      change: 'A small, intentionally low hero meets a large dark product dock. The download action sits on the waterline between the light copy and the dark app field.',
      body: () => `<section class="dockside"><div class="dock-copy">${label('VELA / READY FOR YOUR REPOSITORY')}<h1>Bring the work<br> <em>into view.</em></h1><p class="lede">Editor, OpenCode, terminal, and live diffs.<br> One desktop workspace.</p></div><div class="dock-waterline">${actions()}<span>YOUR DESKTOP. YOUR CONTEXT.</span></div><div class="dock-image">${shot('The whole session, docked in one place.')}</div></section>${features()}`
    },
    {
      name: 'Context card', motif: '26 / a single folded invitation',
      source: 'Source’s gray product border feels like a modest object placed on a very clean surface.',
      change: 'The entire proposition becomes one folded green invitation: a massive bracket seal on the left, a cream message on the right, and a full-width product reveal inside.',
      body: () => `<section class="invitation"><div class="invitation-cover"><div class="invitation-seal"><span>VELA</span><b aria-hidden="true">{v}</b><span>A DESKTOP WORKSPACE</span></div><div class="invitation-message">${label('AN INVITATION TO STAY WITH THE WORK')}${intro('Your next change.<br> <em>Starts here.</em>')}</div></div><div class="invitation-inside">${shot('Open your repository. Bring your context.')}</div></section>${features()}`
    },
    {
      name: 'Working proof', motif: '27 / registration and scale',
      source: 'Source’s faint grid gives the page precision without making it feel like a dashboard.',
      change: 'A printer’s registration sheet uses crosshairs, crop lines, a huge narrow title, and a deliberately offset image with a numbered proof caption.',
      body: () => `<section class="working-proof"><div class="proof-top"><span>VELA / WORKING PROOF</span><span>EDITOR + AGENT + TERMINAL</span></div><h1>CONTEXT<span>MATTERS.</span></h1><div class="proof-middle"><p class="lede">Keep the tools close to the code. Vela brings OpenCode into your desktop workspace.</p>${actions()}</div><div class="proof-image"><div class="proof-axis" aria-hidden="true">+</div>${shot('PLATE 01 / Your workspace, in view.')}<span class="proof-side">SOURCE / RECOMPOSED</span></div></section>${features()}`
    },
    {
      name: 'Conversation piece', motif: '28 / a dialogue across columns',
      source: 'The original says to keep OpenCode beside the repository, but the headline remains a single centered statement.',
      change: 'Two facing typographic voices enact that relationship: “your code” and “your agent” exchange short statements above a shared workspace image.',
      body: () => `<section class="conversation"><div class="conversation-title">${label('TWO SIDES OF THE SAME SESSION')}<h1>Better,<br> <em>in conversation.</em></h1></div><div class="conversation-voices"><div><span>YOUR CODE</span><p>Here’s the<br> context<span class="voice-dot">.</span></p><small>Your repository. Your editor.</small></div><div><span>YOUR AGENT</span><p>Let’s work<br> with it<span class="voice-dot">.</span></p><small>OpenCode. In the same window.</small></div></div>${shot('Keep the conversation beside the code it concerns.')}<div class="conversation-end"><p>Write. Ask. Review. Run.</p>${actions()}</div></section>`
    },
    {
      name: 'The workspace equation', motif: '29 / graphic identity system',
      source: 'Source’s braces make typography serve as a compact visual identity.',
      change: 'A large visual equation turns repository, tools, and agent into a graphic system; the headline follows the diagram, with a small app plate aligned to the equals sign.',
      body: () => `<section class="equation">${label('A SIMPLE WAY TO KEEP THE WORK TOGETHER')}<div class="equation-graphic" aria-label="Repository plus agent equals workspace"><div><b>{ }</b><span>YOUR CODE</span></div><i aria-hidden="true">+</i><div><b>↗</b><span>YOUR AGENT</span></div><i aria-hidden="true">=</i><div class="equation-result"><b>Vela</b><span>YOUR WORKSPACE</span></div></div><div class="equation-bottom"><div><h1>It adds up<br> to <em>context.</em></h1><p class="lede">Editor, OpenCode, terminal, and live diffs. Together on your desktop.</p>${actions()}</div>${shot('THE RESULT / A shared working context.')}</div></section>${steps()}`
    },
    {
      name: 'Quiet system', motif: '30 / radical reduction',
      source: 'Source works because it removes distractions: no icon in the header, no extra eyebrow, one idea and one app image.',
      change: 'Reduce it further: a tiny green square, a single-line statement, a large unframed image, and one vertical column of capabilities. No hero button pair or decorative grid.',
      body: () => `<section class="quiet"><div class="quiet-heading"><span class="quiet-square" aria-hidden="true"></span><h1>Code, <em>in context.</em></h1><p>Your editor and OpenCode.<br> One desktop workspace.</p></div>${shot('Vela. A place to stay with the work.')}<div class="quiet-bottom"><div><p>Made for the session,<br> from first prompt<br> to reviewed change.</p>${actions('Explore Vela for macOS')}</div>${features()}</div></section>`
    }
  );
})();
