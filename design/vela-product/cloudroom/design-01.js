(() => {
  const repository = 'https://github.com/Tsohnle95/vela';
  const releases = `${repository}/releases`;
  const setup = `${repository}#setup`;
  const issues = `${repository}/issues`;
  const docs = `${repository}/blob/main/docs`;

  const icon = {
    arrow: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12h14"/><path d="m13 6 6 6-6 6"/></svg>',
    chevron: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>',
    download: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/></svg>',
    apple: '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M16.365 1.43c0 1.14-.493 2.27-1.177 3.08-.744.9-1.99 1.57-2.987 1.57-.12 0-.23-.02-.3-.03-.01-.06-.04-.22-.04-.39 0-1.15.572-2.27 1.206-2.98.804-.94 2.142-1.64 3.248-1.68.03.13.05.28.05.43zm4.565 15.71c-.03.07-.463 1.58-1.518 3.12-.945 1.34-1.94 2.71-3.43 2.71-1.517 0-1.9-.88-3.63-.88-1.698 0-2.302.91-3.67.91-1.377 0-2.332-1.26-3.428-2.8-1.287-1.82-2.323-4.63-2.323-7.28 0-4.28 2.797-6.55 5.552-6.55 1.448 0 2.675.95 3.6.95.865 0 2.222-1.01 3.902-1.01.613 0 2.886.06 4.374 2.19-.13.09-2.383 1.37-2.383 4.19 0 3.26 2.854 4.42 2.978 4.45z"/></svg>',
    windows: '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M3 5.1 10.4 4v7.4H3zM11.6 3.8 21 2.5v8.9h-9.4zM3 12.6h7.4V20L3 18.9zM11.6 12.6H21v8.9l-9.4-1.3z"/></svg>',
    terminal: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m4 6 5 6-5 6"/><path d="M12 18h8"/></svg>',
    code: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18-6-6 6-6"/><path d="m15 6 6 6-6 6"/></svg>',
    agent: '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 1.8 14.9 9l7.3 3-7.3 3-2.9 7.2-2.9-7.2L1.8 12l7.3-3Z"/></svg>',
    diff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 8h6M8 5v6"/><path d="M13 16h6"/><path d="M3 3v18M21 3v18"/></svg>',
    github: '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .7C5.7.7.8 5.6.8 11.9c0 5 3.2 9.3 7.6 10.8.6.1.8-.3.8-.6v-2.2c-3.1.7-3.8-1.3-3.8-1.3-.5-1.3-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.5-.3-5.1-1.2-5.1-5.6 0-1.2.4-2.2 1.2-3-.1-.3-.5-1.5.1-3 0 0 1-.3 3.3 1.2a11.4 11.4 0 0 1 6 0C17.7 6 18.8 6.3 18.8 6.3c.6 1.5.2 2.7.1 3 .8.8 1.2 1.8 1.2 3 0 4.4-2.6 5.3-5.1 5.6.4.4.8 1.1.8 2.1v3.2c0 .3.2.7.8.6a11.2 11.2 0 0 0 7.6-10.8C23.2 5.6 18.3.7 12 .7Z"/></svg>',
    folder: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/></svg>',
    chat: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 5h16v11H9l-5 4Z"/><path d="M8 9h8M8 12h5"/></svg>',
    lock: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5" y="10" width="14" height="10" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/></svg>',
    check: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m5 12 4 4L19 6"/></svg>',
  };

  const inline = name => `<span class="ob-inline-icon">${icon[name]}</span>`;
  const button = (href, label, kind = 'primary', size = '', suffix = '') =>
    `<a class="ob-btn ob-btn-${kind}${size ? ` ob-btn-${size}` : ''}" href="${href}" target="_blank" rel="noopener">${label}${suffix}</a>`;

  const honeycomb = () => {
    const art = new Map([
      ['0-2', ['{ }', 'lilac']], ['0-9', ['=>', 'mint']],
      ['1-1', ['git', 'mint']], ['1-6', ['++', 'sky']], ['1-12', ['@', 'peach']],
      ['2-4', ['ts', 'teal']], ['2-10', ['md', 'lilac']],
      ['3-0', ['01', 'sky']], ['3-7', ['[ ]', 'mint']], ['3-13', ['fn', 'peach']],
      ['4-3', ['&&', 'lilac']], ['4-9', ['::', 'teal']],
      ['5-1', ['#', 'peach']], ['5-5', ['V', 'mint']], ['5-11', ['=>', 'sky']],
      ['6-2', ['--', 'teal']], ['6-8', ['js', 'lilac']],
      ['7-0', ['++', 'mint']], ['7-6', ['@', 'peach']], ['7-12', ['fn', 'sky']],
      ['8-4', ['md', 'lilac']], ['8-10', ['[ ]', 'teal']],
      ['9-1', ['git', 'sky']], ['9-7', ['&&', 'mint']], ['9-13', ['::', 'peach']],
      ['10-3', ['01', 'peach']], ['10-9', ['=>', 'lilac']],
      ['11-0', ['js', 'teal']], ['11-5', ['#', 'sky']], ['11-11', ['--', 'mint']],
      ['12-2', ['{ }', 'mint']], ['12-8', ['++', 'peach']], ['12-12', ['@', 'lilac']],
    ]);
    const rows = [];
    for (let row = 0; row < 13; row += 1) {
      const cells = [];
      for (let column = 0; column < 14; column += 1) {
        const tile = art.get(`${row}-${column}`);
        cells.push(tile
          ? `<span class="ob-hex ob-hex-art ob-hex-${tile[1]}"><b>${tile[0]}</b></span>`
          : '<span class="ob-hex"></span>');
      }
      rows.push(`<div class="ob-hex-row">${cells.join('')}</div>`);
    }
    return rows.join('');
  };

  const faq = [
    ['What is Vela?', 'Vela is a desktop IDE for coding with an agent. It keeps the file tree, Monaco editor, OpenCode sessions, terminal and live workspace changes in one window.'],
    ['Do I need OpenCode?', 'Yes. Agent sessions run through OpenCode V2. Install it and make sure <code>opencode</code> is available on your PATH before starting a session.'],
    ['Which models can I use?', 'The models and providers configured in OpenCode. Choose the model and agent for a session from the agent panel, including local models when your OpenCode setup provides them.'],
    ['Where does my code go?', 'Vela reads and edits your workspace on your computer. It does not host your repository. When you send a prompt, the model provider you configured receives the context needed for that request.'],
    ['Does it run on Windows and Linux?', 'Yes, with less validation than macOS. The desktop app builds on all three platforms; the macOS experience is the best tested.'],
    ['Can I run more than one agent?', 'Agent mode tiles up to eight sessions in one workspace. Each panel keeps its own conversation and context.'],
    ['How do I review changes?', 'Vela watches the workspace and lists the files touched during the session. When it has a file’s starting contents it shows a real diff; otherwise the file is marked as observed.'],
    ['How do I start?', 'Install OpenCode V2, open Vela, and open a folder or file. The setup guide on GitHub covers running from source and packaging the desktop app.'],
  ];

  const defaultDescription = `A desktop IDE for coding with an agent. Keep
    <span class="ob-inline">${inline('code')} your editor</span>,
    <span class="ob-inline">${inline('agent')} OpenCode</span>,
    <span class="ob-inline">${inline('terminal')} the terminal</span> and
    <span class="ob-inline">${inline('diff')} live diffs</span>
    in one window. Use the models you already run.`;

  const build = (options = {}) => {
    const preview = options.shot === 'light'
      ? {
        file: '../assets/vela-light.png',
        width: 1619,
        height: 971,
        alt: 'Vela in a light theme, with the file tree, Monaco editor, OpenCode conversation and live changes in one window.',
      }
      : {
        file: '../assets/vela-dark.png',
        width: 1619,
        height: 971,
        alt: 'Vela showing the file tree, Monaco editor, OpenCode conversation and live changes in one window.',
      };
    const beamAttrs = options.beam === false ? '' : [
      options.beamOrigin ? `data-origin="${options.beamOrigin}"` : '',
      options.beamColor ? `data-color="${options.beamColor}"` : '',
      options.beamNative ? 'data-native="true"' : '',
      options.beamSettings ? `data-settings="${options.beamSettings}"` : '',
      options.beamPosition != null ? `data-position="${options.beamPosition}"` : '',
      options.beamDelay != null ? `data-delay="${options.beamDelay}"` : '',
      options.beamIntro != null ? `data-intro="${options.beamIntro}"` : '',
      options.beamRise != null ? `data-rise="${options.beamRise}"` : '',
      options.beamPace != null ? `data-pace="${options.beamPace}"` : '',
    ].filter(Boolean).join(' ');
    return `
    <header class="ob-header">
      <div class="ob-header-inner">
        <a class="ob-brand" href="#workspace" aria-label="Vela home">
          <img class="ob-brand-logo" src="../assets/icon.png" width="1024" height="1024" alt="">
          <span>Vela</span>
        </a>
        <nav class="ob-nav" aria-label="Primary navigation">
          <a href="#workspace">Workspace</a>
          <a href="#features">Features</a>
          <a href="#workflow">Workflow</a>
          <a href="#questions">Questions</a>
          <a href="${repository}" target="_blank" rel="noopener">GitHub</a>
        </nav>
        <div class="ob-header-actions">
          <a class="ob-btn ob-btn-primary ob-btn-sm" href="${releases}" target="_blank" rel="noopener">${icon.download} Download</a>
          <a class="ob-btn ob-btn-secondary ob-btn-sm" href="${repository}#setup" target="_blank" rel="noopener">Setup <span aria-hidden="true">→</span></a>
        </div>
      </div>
    </header>

    <main id="main" tabindex="-1">
      <section class="ob-hero" id="workspace">
        <div class="ob-hero-grid" aria-hidden="true"></div>
        ${options.beam === false ? '' : `<canvas class="ob-beam-canvas"${beamAttrs ? ` ${beamAttrs}` : ''} aria-hidden="true"></canvas>`}
        <div class="ob-hero-copy">
          <h1 class="ob-title">
            <span>Vela</span>
            <span class="ob-title-logo"><img src="../assets/icon.png" width="1024" height="1024" alt=""></span>
            <span>IDE</span>
          </h1>
          <p class="ob-description">${options.description || defaultDescription}</p>
          <div class="ob-actions">
            <div class="ob-download-split">
              <a class="ob-btn ob-btn-primary ob-btn-lg ob-download-main" href="${releases}" target="_blank" rel="noopener">${icon.apple} Download for macOS</a>
              <details class="ob-download-more">
                <summary aria-label="More download options">${icon.chevron}</summary>
                <div class="ob-menu">
                  <a href="${releases}" target="_blank" rel="noopener"><span>macOS</span><small>Download the latest build</small></a>
                  <a href="${setup}" target="_blank" rel="noopener"><span>Windows &amp; Linux</span><small>Build from source</small></a>
                  <a href="${repository}" target="_blank" rel="noopener"><span>Browse the source</span><small>GitHub</small></a>
                </div>
              </details>
            </div>
            <a class="ob-btn ob-btn-secondary ob-btn-lg ob-btn-glass" href="${setup}" target="_blank" rel="noopener">Setup guide ${icon.arrow}</a>
          </div>
        </div>

        <figure class="ob-preview">
          <img src="${preview.file}" width="${preview.width}" height="${preview.height}" alt="${preview.alt}">
          <figcaption class="ob-sr">Vela workspace preview</figcaption>
        </figure>
      </section>

      <section class="ob-section ob-network" id="network">
        <div class="ob-network-heading">
          <h2>Your editor and agent<br><span>work side by side.</span></h2>
          <p>Open a file, give OpenCode the task, and review the change without leaving the window. The terminal is already there.</p>
        </div>
        <div class="ob-network-stage">
          <svg class="ob-network-lines" viewBox="0 0 1000 620" preserveAspectRatio="none" aria-hidden="true">
            <path d="M500 322 L230 258"/>
            <path d="M500 322 L770 250"/>
            <path d="M500 322 L500 520"/>
          </svg>
          <div class="ob-net ob-net-editor">
            <span class="ob-net-icon">${icon.code}</span>
            <span class="ob-net-label">Editor</span>
          </div>
          <div class="ob-net-center">
            <span class="ob-net-icon ob-net-icon-lg"><img src="../assets/icon.png" width="1024" height="1024" alt=""></span>
            <span class="ob-net-label">Vela</span>
          </div>
          <div class="ob-net ob-net-diff">
            <span class="ob-net-icon">${icon.diff}</span>
            <span class="ob-net-label">Changes</span>
          </div>
          <div class="ob-net ob-net-terminal">
            <span class="ob-net-icon">${icon.terminal}</span>
            <span class="ob-net-label">Terminal</span>
          </div>
          <span class="ob-bubble ob-bubble-editor">@chatControllers.js</span>
          <span class="ob-bubble ob-bubble-diff">+ 12 &nbsp; − 4</span>
          <span class="ob-bubble ob-bubble-terminal">npm run dev</span>
        </div>
      </section>

      <section class="ob-section ob-features" id="features">
        <div class="ob-inner">
          <header class="ob-heading">
            <h2>What Vela does</h2>
            <p>One desktop app for the whole loop: read, ask, review and run. With the models you already use.</p>
          </header>
          <ul class="ob-bento">
            <li class="ob-card ob-card-workspace">
              <div class="ob-card-visual">
                <div class="ob-mini">
                  <div class="ob-mini-title"><i></i><i></i><i></i><span>chatControllers.js</span></div>
                  <div class="ob-mini-main">
                    <div class="ob-mini-side">
                      <div class="ob-mini-tree-head">Vela</div>
                      <div class="ob-mini-file ob-mini-file-active"><span>js</span> chatControllers.js</div>
                      <div class="ob-mini-file"><span>js</span> app.js</div>
                      <div class="ob-mini-file"><span>json</span> package.json</div>
                      <div class="ob-mini-file"><span>md</span> README.md</div>
                    </div>
                    <div class="ob-mini-editor">
                      <div class="ob-mini-tabs"><span>chatControllers.js</span></div>
                      <div class="ob-code">
                        <div><em>1</em><span><b class="kw">import</b> Groq <b class="kw">from</b> <i class="str">"groq-sdk"</i>;</span></div>
                        <div><em>2</em><span></span></div>
                        <div><em>3</em><span><b class="kw">const</b> groq = <b class="kw">new</b> Groq({ apiKey: process.env.<u class="fn">GROQ_API_KEY</u> });</span></div>
                        <div><em>4</em><span></span></div>
                        <div><em>5</em><span><b class="kw">export const</b> <u class="fn">handleChat</u> = <b class="kw">async</b> (req, res) =&gt; {</span></div>
                        <div><em>6</em><span>&nbsp;&nbsp;<b class="kw">const</b> stream = <b class="kw">await</b> groq.chat.completions.<u class="fn">create</u>({</span></div>
                        <div><em>7</em><span>&nbsp;&nbsp;&nbsp;&nbsp;model: <i class="str">"llama-3.3-70b-versatile"</i>,</span></div>
                        <div><em>8</em><span>&nbsp;&nbsp;&nbsp;&nbsp;stream: <b class="num">true</b>,</span></div>
                        <div><em>9</em><span>&nbsp;&nbsp;});</span></div>
                        <div><em>10</em><span>};</span></div>
                        <div><em>11</em><span></span></div>
                        <div><em>12</em><span><b class="kw">for await</b> (<b class="kw">const</b> chunk <b class="kw">of</b> stream) {</span></div>
                        <div><em>13</em><span>&nbsp;&nbsp;<b class="kw">const</b> content = chunk.choices[0]?.delta?.content;</span></div>
                        <div><em>14</em><span>&nbsp;&nbsp;<b class="kw">if</b> (content) res.<u class="fn">write</u>(content);</span></div>
                        <div><em>15</em><span>}</span></div>
                        <div><em>16</em><span>};</span></div>
                      </div>
                    </div>
                    <div class="ob-mini-chat">
                      <div class="ob-mini-chat-head">OpenCode</div>
                      <div class="ob-mini-bubble">Update the handler and keep the route.</div>
                      <div class="ob-mini-bubble ob-mini-bubble-agent">I’ll update the controller and run the tests.</div>
                      <div class="ob-mini-tool"><span>✎</span> Edit chatControllers.js</div>
                      <div class="ob-mini-compose">Ask about this repository…</div>
                    </div>
                  </div>
                </div>
              </div>
              <div class="ob-card-copy">
                <h3>The editor is the workspace.</h3>
                <p>Browse the repository, open tabs in Monaco, and keep OpenCode beside the code it changes.</p>
              </div>
            </li>

            <li class="ob-card ob-card-providers">
              <div class="ob-card-visual ob-providers-visual">
                <div class="ob-marquee ob-marquee-a" aria-hidden="true">
                  ${[1, 2].map(() => `<div class="ob-marquee-track">
                    <span class="ob-provider"><i class="ob-dot ob-dot-mint"></i>OpenCode</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-white"></i>OpenAI</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-peach"></i>Anthropic</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-sky"></i>Gemini</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-lilac"></i>OpenRouter</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-white"></i>Ollama</span>
                  </div>`).join('')}
                </div>
                <div class="ob-marquee ob-marquee-b" aria-hidden="true">
                  ${[1, 2].map(() => `<div class="ob-marquee-track">
                    <span class="ob-provider"><i class="ob-dot ob-dot-mint"></i>Claude</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-white"></i>GPT</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-sky"></i>Gemini</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-blue"></i>DeepSeek</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-lilac"></i>Llama</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-peach"></i>Qwen</span>
                  </div>`).join('')}
                </div>
                <div class="ob-marquee ob-marquee-c" aria-hidden="true">
                  ${[1, 2].map(() => `<div class="ob-marquee-track">
                    <span class="ob-provider"><i class="ob-dot ob-dot-white"></i>LM Studio</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-mint"></i>Ollama</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-sky"></i>OpenAI-compatible</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-lilac"></i>Local models</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-peach"></i>Mistral</span>
                    <span class="ob-provider"><i class="ob-dot ob-dot-blue"></i>Groq</span>
                  </div>`).join('')}
                </div>
              </div>
              <div class="ob-card-copy">
                <h3>Use the model you already run.</h3>
                <p>OpenCode connects the models and providers you already use. Pick one in the agent panel and keep working.</p>
              </div>
            </li>

            <li class="ob-card ob-card-local">
              <div class="ob-card-copy">
                <h3>Stored on your computer.</h3>
                <p>Your repository, sessions, terminal and diffs stay in the desktop app. Model requests go only to the provider you configure.</p>
              </div>
              <div class="ob-local-visual">
                <div class="ob-local-list">
                  <span>${icon.check} Repository</span>
                  <span>${icon.check} Sessions</span>
                  <span>${icon.check} Terminal</span>
                  <span>${icon.check} Changes</span>
                </div>
                <span class="ob-lock">${icon.lock}</span>
              </div>
            </li>

            <li class="ob-card ob-card-model">
              <div class="ob-card-visual ob-model-visual">
                <div class="ob-model">
                  <div class="ob-model-head"><span class="ob-model-avatar">${icon.agent}</span> OpenCode <span class="ob-model-select">Claude Sonnet ${icon.chevron}</span></div>
                  <div class="ob-model-msg">Refactor the controller.</div>
                  <div class="ob-model-msg ob-model-agent">Done. I changed 4 files.<span>Review diff →</span></div>
                </div>
              </div>
              <div class="ob-card-copy">
                <h3>Change the model. Keep the session.</h3>
                <p>Switch the model or agent inside the same OpenCode session. The conversation and its context stay with you.</p>
              </div>
            </li>

            <li class="ob-card ob-card-terminal">
              <div class="ob-card-visual ob-terminal-visual">
                <div class="ob-term">
                  <div class="ob-term-head"><i></i><i></i><i></i><span>zsh — vela</span></div>
                  <div class="ob-term-body">
                    <div><span class="ob-prompt">~</span> npm run dev</div>
                    <div class="ob-term-dim">electron-vite dev</div>
                    <div class="ob-term-ok">✓ ready</div>
                    <div><span class="ob-prompt">~</span> <span class="ob-caret"></span></div>
                  </div>
                </div>
              </div>
              <div class="ob-card-copy">
                <h3>A terminal that belongs here.</h3>
                <p>Run the project in the integrated terminal, or switch an agent panel to the OpenCode TUI.</p>
              </div>
            </li>

            <li class="ob-card ob-card-queue">
              <div class="ob-card-visual ob-queue-visual">
                <div class="ob-queue">
                  <div class="ob-queue-head">Waiting · OpenCode is working</div>
                  <div class="ob-queue-row"><span>Add tests for the new route</span><b>Steer</b></div>
                  <div class="ob-queue-row"><span>Update the migration notes</span><b>Steer</b></div>
                  <div class="ob-queue-input">Message OpenCode… <span>↑</span></div>
                </div>
              </div>
              <div class="ob-card-copy">
                <h3>Queue the next prompt.</h3>
                <p>Send more work while the agent is busy. Messages wait in a queue you can pause, resume or cancel.</p>
              </div>
            </li>
          </ul>
        </div>
      </section>

      <section class="ob-pricing" id="pricing">
        <p class="ob-eyebrow">What it costs</p>
        <p class="ob-price" aria-label="Zero dollars">$0</p>
        <p class="ob-price-note">Vela is free to run. No subscription, no account, no locked panels. Bring your own OpenCode setup and models.</p>
        <p class="ob-price-link"><a class="ob-inline-link" href="${repository}" target="_blank" rel="noopener">Read the source <span aria-hidden="true">→</span></a></p>
      </section>

      <section class="ob-section ob-flow" id="workflow">
        <div class="ob-inner ob-flow-layout">
          <div class="ob-flow-intro">
            <h2>One loop.<br>Four moves.</h2>
            <p class="ob-flow-lede">Open, ask, review, run. Vela keeps the whole loop in one window.</p>
            <p class="ob-flow-body">No app switching. No lost context. Your repository stays at the center.</p>
            <div class="ob-actions ob-actions-left">
              ${button(releases, `${icon.download} Download Vela`, 'primary')}
              ${button(repository, `Read the source ${icon.arrow}`, 'secondary')}
            </div>
            <ol class="ob-flow-progress">
              <li><span>01</span> Open</li>
              <li><span>02</span> Ask</li>
              <li><span>03</span> Review</li>
              <li><span>04</span> Run</li>
            </ol>
          </div>
          <div class="ob-flow-stack">
            <article class="ob-flow-card ob-card-lilac" style="--ob-card-index:0">
              <div class="ob-flow-card-top"><span class="ob-chip">Open</span><span class="ob-count">01 / 04</span></div>
              <span class="ob-flow-icon">${icon.folder}</span>
              <div class="ob-flow-copy"><h3>Your repository,<br>in reach.</h3><p>Open a folder, a file or a whole project. Browse the tree and edit with Monaco.</p></div>
            </article>
            <article class="ob-flow-card ob-card-mint" style="--ob-card-index:1">
              <div class="ob-flow-card-top"><span class="ob-chip">Ask</span><span class="ob-count">02 / 04</span></div>
              <span class="ob-flow-icon">${icon.chat}</span>
              <div class="ob-flow-copy"><h3>Give the task<br>a place.</h3><p>Attach files, reference paths with @, and follow streamed responses and tool calls.</p></div>
            </article>
            <article class="ob-flow-card ob-card-sand" style="--ob-card-index:2">
              <div class="ob-flow-card-top"><span class="ob-chip">Review</span><span class="ob-count">03 / 04</span></div>
              <span class="ob-flow-icon">${icon.diff}</span>
              <div class="ob-flow-copy"><h3>See the<br>actual change.</h3><p>Follow the files touched during the session and inspect per-file diffs when starting contents are available.</p></div>
            </article>
            <article class="ob-flow-card ob-card-sky" style="--ob-card-index:3">
              <div class="ob-flow-card-top"><span class="ob-chip">Run</span><span class="ob-count">04 / 04</span></div>
              <span class="ob-flow-icon">${icon.terminal}</span>
              <div class="ob-flow-copy"><h3>Keep the<br>loop moving.</h3><p>Run the project in the integrated terminal or switch a panel to the OpenCode TUI.</p></div>
            </article>
          </div>
        </div>
      </section>

      <section class="ob-section ob-faq" id="questions">
        <div class="ob-inner ob-faq-layout">
          <header class="ob-faq-intro">
            <h2>Questions</h2>
            <p>Short answers about setup, models, files and the editor.</p>
            <a class="ob-inline-link" href="${issues}" target="_blank" rel="noopener">Ask on GitHub <span aria-hidden="true">→</span></a>
          </header>
          <div class="ob-faq-list">
            ${faq.map(([question, answer]) => `<details class="ob-faq-item" name="ob-faq"><summary>${question}${icon.chevron}</summary><p>${answer}</p></details>`).join('')}
          </div>
        </div>
      </section>

      <section class="ob-download" id="download">
        <div class="ob-inner">
          <header class="ob-heading">
            <h2>Download Vela</h2>
            <p>Choose your platform. Vela is a desktop app; OpenCode V2 is required for agent sessions.</p>
          </header>
          <div class="ob-download-grid">
            <a class="ob-download-card ob-download-macos" href="${releases}" target="_blank" rel="noopener">
              <div class="ob-download-top"><span class="ob-platform-logo">${icon.apple}</span><span class="ob-status-pill">Available</span></div>
              <div class="ob-download-copy"><h3>macOS</h3><p>Best tested · OpenCode V2 required</p><span class="ob-download-action">Download for macOS ${icon.arrow}</span></div>
            </a>
            <a class="ob-download-card ob-download-windows" href="${setup}" target="_blank" rel="noopener">
              <div class="ob-download-top"><span class="ob-platform-logo">${icon.windows}</span><span class="ob-status-pill">Source</span></div>
              <div class="ob-download-copy"><h3>Windows</h3><p>Build from source · Less validation</p><span class="ob-download-action">Setup for Windows ${icon.arrow}</span></div>
            </a>
            <a class="ob-download-card ob-download-linux" href="${setup}" target="_blank" rel="noopener">
              <div class="ob-download-top"><span class="ob-platform-logo">${icon.terminal}</span><span class="ob-status-pill">Source</span></div>
              <div class="ob-download-copy"><h3>Linux</h3><p>Build from source · Less validation</p><span class="ob-download-action">Setup for Linux ${icon.arrow}</span></div>
            </a>
          </div>
          <p class="ob-download-pages">System requirements and install steps: <a href="${setup}" target="_blank" rel="noopener">macOS</a> <a href="${setup}" target="_blank" rel="noopener">Windows</a> <a href="${setup}" target="_blank" rel="noopener">Linux</a> · <a href="${repository}" target="_blank" rel="noopener">Source</a></p>
        </div>
      </section>

      <section class="ob-post">
        <div class="ob-post-comb" aria-hidden="true"><div class="ob-honeycomb">${honeycomb()}</div></div>
        <div class="ob-post-veil" aria-hidden="true"></div>
        <div class="ob-post-copy">
          <h2>Meet your new workspace.</h2>
          <p>Open a folder. Ask OpenCode for a change. Review the diff. Run it. The whole loop stays in one window.</p>
          <div class="ob-actions">
            ${button(releases, `${icon.download} Download Vela`, 'primary', 'lg')}
            ${button(repository, `Read the source ${icon.arrow}`, 'secondary', 'lg')}
          </div>
        </div>
      </section>
    </main>

    <footer class="ob-footer">
      <div class="ob-footer-inner">
        <div class="ob-footer-grid">
          <div class="ob-footer-brand">
            <a class="ob-footer-lockup" href="#workspace">
              <img src="../assets/icon.png" width="1024" height="1024" alt="">
              <span>Vela</span>
            </a>
            <p class="ob-footer-description">A desktop IDE for coding with an agent. Your editor, OpenCode, terminal and live diffs in one window.</p>
            <a class="ob-footer-cta" href="${releases}" target="_blank" rel="noopener">Download Vela ${icon.arrow}</a>
            <div class="ob-footer-socials">
              <a class="ob-footer-social" href="${repository}" target="_blank" rel="noopener" aria-label="GitHub">${icon.github}</a>
              <a class="ob-footer-social" href="${releases}" target="_blank" rel="noopener" aria-label="Releases">${icon.download}</a>
            </div>
          </div>
          <nav class="ob-footer-columns" aria-label="Footer navigation">
            <div class="ob-footer-column">
              <h2>Product</h2>
              <ul>
                <li><a href="#workspace">Workspace</a></li>
                <li><a href="#features">Features</a></li>
                <li><a href="#workflow">Workflow</a></li>
                <li><a href="#questions">Questions</a></li>
                <li><a href="#download">Download</a></li>
              </ul>
            </div>
            <div class="ob-footer-column">
              <h2>Build</h2>
              <ul>
                <li><a href="${repository}" target="_blank" rel="noopener">Source code</a></li>
                <li><a href="${releases}" target="_blank" rel="noopener">Releases</a></li>
                <li><a href="${setup}" target="_blank" rel="noopener">Setup guide</a></li>
                <li><a href="${docs}/architecture.md" target="_blank" rel="noopener">Architecture</a></li>
                <li><a href="${docs}/operations.md" target="_blank" rel="noopener">Operations</a></li>
              </ul>
            </div>
            <div class="ob-footer-column">
              <h2>Resources</h2>
              <ul>
                <li><a href="https://opencode.ai" target="_blank" rel="noopener">OpenCode</a></li>
                <li><a href="https://opencode.ai/docs" target="_blank" rel="noopener">OpenCode docs</a></li>
                <li><a href="https://microsoft.github.io/monaco-editor/" target="_blank" rel="noopener">Monaco editor</a></li>
                <li><a href="https://www.electronjs.org/" target="_blank" rel="noopener">Electron</a></li>
                <li><a href="https://react.dev/" target="_blank" rel="noopener">React</a></li>
              </ul>
            </div>
            <div class="ob-footer-column">
              <h2>Models</h2>
              <ul>
                <li><a href="https://opencode.ai/docs/providers/" target="_blank" rel="noopener">Providers</a></li>
                <li><a href="https://openai.com/" target="_blank" rel="noopener">OpenAI</a></li>
                <li><a href="https://www.anthropic.com/" target="_blank" rel="noopener">Anthropic</a></li>
                <li><a href="https://ollama.com/" target="_blank" rel="noopener">Ollama</a></li>
                <li><a href="https://lmstudio.ai/" target="_blank" rel="noopener">LM Studio</a></li>
              </ul>
            </div>
          </nav>
        </div>
        <div class="ob-footer-bottom">
          <p>© 2026 Vela</p>
          <div class="ob-footer-meta">
            <span><i class="ob-status-dot" aria-hidden="true"></i> Desktop IDE for coding agents</span>
            <span>Built with Electron, React and Monaco</span>
          </div>
        </div>
      </div>
    </footer>
  `;
  };

  window.velaMeet = { build };
  window.velaDesignOne = { name: 'Vela IDE', body: () => build() };
})();
