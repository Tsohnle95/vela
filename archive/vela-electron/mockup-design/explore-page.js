const designId = new URLSearchParams(window.location.search).get("design");
const design = velaExplorations.find((item) => item.id === designId);
const app = document.querySelector("#app");
const sceneMessages = {
  "04-canopy": "Begin in the clearing. Keep the code in view.",
  "04-estuary": "Follow the current from question to change.",
  "04-terrace": "Each layer leads back to the source.",
  "04-topography": "The work has a place on the map.",
  "04-glasshouse": "Context is something worth keeping alive.",
  "04-nocturne": "Stay oriented, even in the deep work.",
  "04-sandstone": "Shape the next change from the code beneath it.",
  "04-mist": "See the next step come into focus.",
  "04-bloom": "Build on the project that is already growing.",
  "04-velaline": "Every path leads back to the repository.",
  "10-parchment": "Everything you need on the same page.",
  "10-grid": "The right tools, already lined up.",
  "10-lagoon": "Find a rhythm that keeps moving.",
  "10-copper": "Keep the tools warm and the work close.",
  "10-atlas": "A clear route through the day’s work.",
  "10-prism": "Different tools. One shared focus.",
  "10-dusk": "A good place to keep going.",
  "10-studio": "A room for every part of the process.",
  "10-midnight": "See the whole system without losing focus.",
  "10-daybreak": "Start fresh. Stay with the work."
};
const nightCityPageCopy = {
  "04-nocturne-tidal-city-rose-night": {
    workspaceLabel: "01 / WORKSPACE",
    workspaceHeading: "Open a folder. Work in its files.",
    workspaceDescription: "The selected folder becomes Vela’s workspace. Explorer lists its files, Monaco opens them, the integrated terminal uses that directory, and OpenCode V2 creates the session for it.",
    interludeLabel: "WORKSPACE / FILE WATCHING",
    interludeHeading: "Changes lists file edits Vela observes during the session.",
    interludeNote: "Edits are observed, not attributed.",
    reviewHeading: "Open a diff when Vela knows the starting content.",
    reviewDescription: "Changes lists workspace files Vela observed changing. When the original content is known, a file opens in Monaco Diff; otherwise it stays marked observed and Diff is unavailable.",
    closeHeading: "The editor, terminal, and agent session share this folder.",
    closeAction: "Back to workspace"
  },
  "04-nocturne-tidal-city-rose-night-workflow-rose-terminal": {
    workspaceLabel: "01 / INTEGRATED TERMINAL",
    workspaceHeading: "Run project commands from Vela’s terminal.",
    workspaceDescription: "The integrated terminal uses the active workspace directory. Explorer lists that folder’s files, Monaco opens them, and OpenCode V2 creates the session for it.",
    interludeLabel: "WORKSPACE / FILE WATCHING",
    interludeHeading: "Changes lists file edits Vela observes during the session.",
    interludeNote: "Edits are observed, not attributed.",
    reviewHeading: "Open a diff when Vela knows the starting content.",
    reviewDescription: "Changes lists workspace files Vela observed changing. When the original content is known, a file opens in Monaco Diff; otherwise it stays marked observed and Diff is unavailable.",
    closeHeading: "The editor, terminal, and agent session share this folder.",
    closeAction: "Back to workspace"
  }
};

if (!design) {
  app.innerHTML = `<main class="missing-design"><h1>That exploration isn't here.</h1><a href="../index.html">See all Vela designs →</a></main>`;
} else {
  const isRepo = design.family === "04";
  const isAgentFocused = Boolean(design.productCopy);
  const isRoseTerminal = design.id === "04-nocturne-tidal-city-rose-night-workflow-rose-terminal";
  const nightCityCopy = nightCityPageCopy[design.id];
  if (isAgentFocused) document.body.classList.add("agent-focused");
  if (nightCityCopy) document.body.classList.add("night-city-narrative");
  if (isRoseTerminal) document.body.classList.add("vela-showcase");
  const colors = `--scene-ink:${design.ink};--scene-accent:${design.accent};--scene-paper:${design.sky};--scene-ground:${design.foreground}`;
  document.title = `Vela — ${design.name}`;
  document.body.classList.add(`explore-${design.family}`, `scene-${design.scene}`, `form-${design.form}`, `type-${design.type}`, design.light ? "scene-light" : "scene-dark");
  if (design.atmosphere) document.body.classList.add("has-atmosphere", `terrain-${design.terrain}`, `celestial-${design.celestial}`);
  if (design.layout) document.body.classList.add(`layout-${design.layout}`);
  document.body.style.cssText = colors;

  const projectDiagram = `<div class="project-map" aria-label="One repository connects the editor, agent, and change review">
    <div class="map-origin"><span class="map-icon">⌁</span><small>OPEN PROJECT</small><strong>~/code/vela</strong><span>main · local workspace</span></div>
    <div class="map-path" aria-hidden="true"><span></span><span></span><span></span></div>
    <div class="map-destinations"><div><span class="map-key">01 / READ</span><strong>Monaco editor</strong><small>App.tsx ↗</small></div><div><span class="map-key">02 / ASK</span><strong>Agent in context</strong><small>Working in this repo ↗</small></div><div><span class="map-key">03 / REVIEW</span><strong>Changed files</strong><small>3 changes ready ↗</small></div></div>
  </div>`;
  const ridgeTiles = `<div class="ridge-tiles" aria-label="The repository, editor, and change review stay together">
    <article class="ridge-tile"><div class="ridge-tile-bar"><span>01 / EXPLORE</span><span>⌁ &nbsp; main</span></div><div class="ridge-tile-files"><p>⌄ &nbsp; vela</p><p>⌄ &nbsp; src</p><p class="selected">TS &nbsp; App.tsx</p><p>TS &nbsp; workspace.ts</p><p>⌄ &nbsp; styles</p></div><div class="ridge-tile-foot"><strong>Begin with the repo.</strong><small>Your files are already here.</small></div></article>
    <article class="ridge-tile ridge-tile-code"><div class="ridge-tile-bar"><span>02 / EDIT</span><span>App.tsx &nbsp; ×</span></div><div class="ridge-tile-lines"><p><i>118</i> <b>export</b> function Workspace() {</p><p><i>119</i> &nbsp; const root = useRepo();</p><p class="selected"><i>120</i> &nbsp; return &lt;Agent root={root} /&gt;;</p><p><i>121</i> }</p></div><div class="ridge-tile-foot"><strong>Stay with the code.</strong><small>Editor and agent in the same view.</small></div></article>
    <article class="ridge-tile"><div class="ridge-tile-bar"><span>03 / REVIEW</span><span>3 files ↗</span></div><div class="ridge-tile-diff"><p>App.tsx <b>+12</b></p><p>workspace.ts <b>+7 −2</b></p><p>_layout.scss <b>+4</b></p><code>+ keep workspace context</code></div><div class="ridge-tile-foot"><strong>Know what changed.</strong><small>Follow each edit back to its file.</small></div></article>
  </div>`;
  const desktopDock = `<div class="desktop-dock" aria-label="The editor, agent, terminal, and change review share one desk">
    <div class="dock-pane"><span class="dock-glyph">⌘</span><span><small>READ + EDIT</small><strong>Monaco workbench</strong></span><i>01</i></div>
    <div class="dock-pane"><span class="dock-glyph">✳</span><span><small>ASK</small><strong>Agent beside your code</strong></span><i>02</i></div>
    <div class="dock-pane"><span class="dock-glyph">›_</span><span><small>RUN</small><strong>Integrated terminal</strong></span><i>03</i></div>
    <div class="dock-pane"><span class="dock-glyph">±</span><span><small>REVIEW</small><strong>Changes and diffs</strong></span><i>04</i></div>
  </div>`;
  const repoProof = `<div class="proof-visual proof-repo"><div class="proof-window-top"><span>VELA / FILES</span><span>main ↗</span></div><div class="proof-folders"><p>⌄ &nbsp; vela</p><p>⌄ &nbsp; src</p><p class="active">TS &nbsp; App.tsx</p><p>SCSS &nbsp; _layout.scss</p><p>⌄ &nbsp; mockup-design</p></div><div class="proof-focus"><span>App.tsx</span><code><b>const</b> repo = useWorkspace();<br /><b>return</b> &lt;AgentPanel repo={repo} /&gt;;</code></div><div class="proof-window-foot">The same project stays with every tool &nbsp; ↗</div></div>`;
  const deskProof = `<div class="proof-visual proof-desk"><div class="proof-window-top"><span>YOUR WORKSPACE / ACTIVE</span><span>⌘ K</span></div><div class="proof-screen"><div class="screen-editor"><span>App.tsx</span><code><b>export</b> function Workspace() {<br />&nbsp; <i>const</i> root = useRepo();<br />&nbsp; <b>return</b> &lt;Agent context={root} /&gt;;<br />}</code></div><div class="screen-agent"><span class="screen-status"></span> Agent<div>Following the code in your project…</div><small>✓ &nbsp; 2 files opened</small></div></div><div class="proof-terminal"><span>vela $</span> npm run check &nbsp; <strong>✓ ready</strong></div></div>`;
  const nightWorkspaceFacts = `<dl class="night-workspace-facts" aria-label="Tools in the active workspace">
    <div><dt>FILES</dt><dd>Explorer</dd></div>
    <div><dt>EDITOR</dt><dd>Monaco</dd></div>
    <div><dt>AGENT</dt><dd>OpenCode V2</dd></div>
    <div><dt>SHELL</dt><dd>Integrated terminal</dd></div>
  </dl>`;
  const nightChangeStates = `<dl class="night-change-states" aria-label="Changes list status meanings">
    <div><dt>Modified</dt><dd>A known baseline differs from the current file.</dd></div>
    <div><dt>Deleted</dt><dd>The file path is no longer in the workspace.</dd></div>
    <div><dt>Observed</dt><dd>Starting content is unknown; Diff is unavailable.</dd></div>
  </dl>`;
  const roseTerminalStory = `
    <section class="showcase-workflow" id="inside" aria-labelledby="workflow-title">
      <div class="showcase-heading">
        <p class="art-section-label">01 / ONE WORKSPACE, TWO WAYS TO BUILD</p>
        <h2 id="workflow-title">Stay in the code.<br /><em>Step back when you want to.</em></h2>
        <p>Vela brings the familiar coding desk and the agent conversation into one app. Open a repository, work directly in its files, or give an agent a task without losing sight of the project.</p>
      </div>
      <div class="showcase-workflow-grid">
        <article class="showcase-mode">
          <div class="showcase-mode-top"><span>01 / DIRECT WORK</span><span>EDITOR + TERMINAL</span></div>
          <h3>For the parts you want to do yourself.</h3>
          <p>Browse the file tree, edit in Monaco, run project commands in the integrated terminal, and inspect changed files in the diff view.</p>
          <div class="showcase-mini-editor" aria-label="Illustration of Vela's editor and integrated terminal">
            <div class="showcase-mini-bar"><span class="showcase-window-dots" aria-hidden="true">● ● ●</span><span>src / main / opencode.ts</span><span>×</span></div>
            <div class="showcase-code"><span>118</span><code><b>export async function</b> openWorkspace() {</code><span>119</span><code>&nbsp; <b>const</b> session = <i>await</i> connectRuntime();</code><span>120</span><code>&nbsp; <b>return</b> attachSession(session);</code></div>
            <div class="showcase-mini-terminal"><span>TERMINAL</span><code>vela $ npm run check <b>✓</b></code></div>
          </div>
        </article>
        <article class="showcase-mode">
          <div class="showcase-mode-top"><span>02 / AGENT WORK</span><span>CONVERSATION + TOOLS</span></div>
          <h3>For the work you want to direct.</h3>
          <p>Vela's agent panel is built in. Give it a task, follow progress and tool activity in a readable conversation, then inspect the files that changed. No editor plugin to assemble.</p>
          <div class="showcase-mini-agent" aria-label="Illustration of a Vela agent conversation">
            <div class="showcase-mini-bar"><span>◉ &nbsp; Agent</span><span>OpenCode V2</span></div>
            <div class="showcase-prompt">Trace how a repository session reaches the editor and agent panel.</div>
            <div class="showcase-agent-step"><span class="showcase-step-dot"></span><span><strong>Reading the project</strong><small>docs/architecture.md · src/main/opencode.ts</small></span></div>
            <div class="showcase-agent-step"><span class="showcase-step-dot"></span><span><strong>Following the handoff</strong><small>src/renderer/src/store.tsx</small></span></div>
            <div class="showcase-agent-composer">Ask about this repository or describe a change… <span>↑</span></div>
          </div>
        </article>
      </div>
      <p class="showcase-workflow-foot">YOUR REPOSITORY IS THE SHARED CONTEXT <span aria-hidden="true">↗</span> EXPLORER &nbsp;·&nbsp; EDITOR &nbsp;·&nbsp; TERMINAL &nbsp;·&nbsp; AGENTS &nbsp;·&nbsp; DIFFS</p>
    </section>
    <section class="showcase-fleet" id="agents" aria-labelledby="fleet-title">
      <div class="showcase-fleet-inner">
        <div class="showcase-fleet-copy">
          <p class="art-section-label">02 / AGENT MODE</p>
          <h2 id="fleet-title">A fleet of agents.<br /><em>One place to steer it.</em></h2>
          <p>Open up to eight agent panes at once. Keep sessions visible in a single organized view instead of spreading eight CLI windows across your desktop. Switch back to the focused coding workspace whenever you need to get into the files.</p>
          <div class="showcase-fleet-meta"><span>01—08</span><span>VISIBLE AGENT PANES</span></div>
        </div>
        <div class="showcase-fleet-window" role="img" aria-label="Illustration of eight agent sessions arranged in Vela Agent Mode">
          <div class="showcase-fleet-bar"><span><span class="showcase-window-dots" aria-hidden="true">● ● ●</span> VELA / AGENT MODE</span><span>8 PANELS <span aria-hidden="true">▦</span></span></div>
          <div class="showcase-fleet-grid">
            <div class="showcase-fleet-pane is-active"><span>01 <i>●</i></span><strong>Runtime bridge</strong><small>Following session events</small><b>Working</b></div>
            <div class="showcase-fleet-pane"><span>02 <i>●</i></span><strong>Editor state</strong><small>Reviewing file changes</small><b>Working</b></div>
            <div class="showcase-fleet-pane"><span>03 <i>●</i></span><strong>Terminal flow</strong><small>Checking command output</small><b>Working</b></div>
            <div class="showcase-fleet-pane"><span>04 <i>●</i></span><strong>Docs pass</strong><small>Ready for review</small><b>Ready</b></div>
            <div class="showcase-fleet-pane"><span>05 <i>●</i></span><strong>UI details</strong><small>Reading components</small><b>Working</b></div>
            <div class="showcase-fleet-pane"><span>06 <i>●</i></span><strong>Tests</strong><small>Ready for review</small><b>Ready</b></div>
            <div class="showcase-fleet-pane"><span>07 <i>●</i></span><strong>Refactor</strong><small>Following imports</small><b>Working</b></div>
            <div class="showcase-fleet-pane"><span>08 <i>●</i></span><strong>Release notes</strong><small>Waiting for input</small><b>Waiting</b></div>
          </div>
          <div class="showcase-fleet-bottom"><span>ONE REPOSITORY · MULTIPLE SESSIONS</span><span>+ Add panel</span></div>
        </div>
      </div>
    </section>
    <section class="showcase-runtimes" id="runtimes" aria-labelledby="runtimes-title">
      <div class="showcase-heading">
        <p class="art-section-label">03 / RUNTIME DIRECTION</p>
        <h2 id="runtimes-title">Your environment.<br /><em>Your choice of agent.</em></h2>
        <p>Vela is open source and is being built around a runtime adapter boundary. The aim is a home for the coding harnesses developers already use, with the editor and workspace experience staying familiar as runtime support grows.</p>
      </div>
      <div class="showcase-runtime-list">
        <div class="showcase-runtime-row current"><span class="showcase-runtime-index">NOW / 01</span><div><h3>OpenCode V2</h3><p>Connected today. Use its configured providers and models in Vela.</p></div><span class="showcase-runtime-status"><span class="showcase-step-dot"></span> AVAILABLE</span></div>
        <div class="showcase-runtime-row"><span class="showcase-runtime-index">NEXT / →</span><div><h3>More harnesses, same workspace.</h3><p>Codex, DeepSeek Harness, CommandCode, and Pi Agent are future integrations, not available in Vela yet.</p></div><span class="showcase-runtime-status">PLANNED</span></div>
      </div>
    </section>
    <section class="showcase-close" aria-labelledby="close-title">
      <div><p class="art-section-label">VELA / OPEN SOURCE AGENT DEVELOPMENT ENVIRONMENT</p><h2 id="close-title">Build in the code.<br />Work with your agents.</h2><p>An editor, terminal, agent workspace, and change review in one desktop app.</p></div>
      <div class="showcase-close-actions"><a class="art-button" href="https://github.com/Tsohnle95/Vela" target="_blank" rel="noopener noreferrer">View the source <span aria-hidden="true">↗</span></a><a class="showcase-back-link" href="#hero-title">Back to top ↑</a></div>
    </section>`;

  app.innerHTML = `
    <header class="explore-nav">
      <a class="explore-brand" href="../index.html"><span aria-hidden="true">◉</span> vela</a>
      <nav aria-label="Page navigation">${isRoseTerminal ? '<a href="#inside">Workspace</a><a href="#agents">Agent mode</a><a href="#runtimes">Runtimes</a>' : `<a href="#inside">${isRepo ? "The project" : "The workbench"}</a><a href="#review">The loop</a>`}</nav>
      <button class="theme-toggle" type="button" data-theme-toggle aria-label="Toggle dark mode" aria-pressed="false"><span class="theme-icon" aria-hidden="true">◐</span><span>Dark mode</span></button>
    </header>
    <main>
      <section class="art-hero" aria-labelledby="hero-title">
         ${design.atmosphere ? velaAtmosphere(design) : velaScene(design)}
        <div class="hero-light" aria-hidden="true"></div>
        <div class="art-hero-content">
          <div class="art-hero-copy">
            ${isAgentFocused ? "" : `<p class="art-kicker"><span class="live-dot"></span>${isRepo ? "YOUR REPOSITORY / YOUR WORKSPACE" : "THE DESKTOP WORKSPACE FOR CODING AGENTS"}</p>`}
            <h1 id="hero-title">${isAgentFocused ? "Vela" : design.title}</h1>
            ${isAgentFocused ? '<h2 class="art-hero-subtitle">Agent Development Environment</h2>' : `<p class="art-lede">${design.lede}</p>`}
            <div class="art-actions"><a class="art-button" href="#inside">${isAgentFocused ? "Explore Vela" : isRepo ? "Explore the workspace" : "Step inside"}<span aria-hidden="true">↓</span></a>${isAgentFocused ? "" : `<a class="art-secondary" href="#review">See the whole loop <span aria-hidden="true">→</span></a>`}</div>
            ${isAgentFocused ? "" : `<div class="art-facts" aria-label="Vela workspace features"><span>Repository first</span><span>Monaco editor</span><span>OpenCode V2</span></div>`}
          </div>
          <figure class="art-product">
            <img src="../assets/vela-workbench-reference.png" width="1375" height="779" alt="Vela desktop workbench showing the repository explorer, editor, terminal, and agent panel in one window." />
            ${isRoseTerminal ? "" : `<figcaption><span class="live-dot"></span>${isAgentFocused ? isRepo ? "Repository open · Agent ready" : "Agent workspace · Repository connected" : isRepo ? "Project open · Files in view" : "One desk · All your tools"}</figcaption>`}
          </figure>
        </div>
      </section>
      ${isRoseTerminal ? roseTerminalStory : `<section class="art-inside" id="inside">
        <div class="art-section-header"><p class="art-section-label">${nightCityCopy?.workspaceLabel ?? (isAgentFocused ? "01 / AGENT WORKSPACE" : isRepo ? "01 / THE SOURCE" : "01 / YOUR DESK")}</p><h2>${nightCityCopy?.workspaceHeading ?? design.section}</h2><p>${nightCityCopy?.workspaceDescription ?? (isAgentFocused ? "Open a repository, run a coding agent beside the editor and terminal, then inspect its file diffs in Vela." : isRepo ? "Vela starts with a real repository. Your editor, terminal, agent session, and review all stay attached to the folder you chose." : "The desktop workspace puts your code, conversation, terminal, and changes together. No tab shuffle between the important parts.")}</p></div>
        ${nightCityCopy ? nightWorkspaceFacts : design.lineage === "ridge" ? ridgeTiles : isRepo ? projectDiagram : desktopDock}
      </section>
      <section class="art-interlude" aria-label="${nightCityCopy ? "Workspace file change tracking" : isRepo ? "The repository stays at the center" : "A connected desktop workspace"}">
         ${design.atmosphere ? velaAtmosphere(design, "interlude") : velaScene(design, "interlude")}
         <div class="interlude-copy"><span class="interlude-mark" aria-hidden="true">◉</span><p>${nightCityCopy?.interludeLabel ?? (isAgentFocused ? "VELA / AGENT WORKFLOW" : isRepo ? "ONE REPOSITORY / MANY NEXT STEPS" : "ONE DESK / A BETTER RHYTHM")}</p><h2>${nightCityCopy?.interludeHeading ?? design.message ?? sceneMessages[design.id]}</h2></div>
         <div class="interlude-note"><span class="live-dot"></span>${nightCityCopy?.interludeNote ?? (isAgentFocused ? isRepo ? "~/code/vela · agent workspace" : "Vela · repository connected" : isRepo ? "~/code/vela · project open" : "Vela desktop · workspace active")}</div>
      </section>
      <section class="art-review" id="review">
        <div class="review-words"><p class="art-section-label">${nightCityCopy ? "02 / CHANGES & DIFFS" : isAgentFocused ? "02 / CHANGE REVIEW" : "02 / FROM START TO FINISH"}</p><h2>${nightCityCopy?.reviewHeading ?? (isAgentFocused ? "From agent task to reviewed diff." : isRepo ? "Follow the work back to its source." : "Stay for the whole coding loop.")}</h2><p>${nightCityCopy?.reviewDescription ?? (isAgentFocused ? "Run the agent in your codebase, then inspect the exact file changes before accepting them." : isRepo ? "Read a file, ask a question about it, then open the diff. The work stays legible because it never leaves its project behind." : "Bring a project into view, work alongside the agent, run a command, and review the changes before moving on.")}</p>${nightCityCopy ? "" : `<div class="review-sequence"><span>01 &nbsp; OPEN</span><span>02 &nbsp; ASK</span><span>03 &nbsp; REVIEW</span></div>`}</div>
        ${nightCityCopy ? nightChangeStates : isRepo ? repoProof : deskProof}
      </section>
      <section class="art-close"><div><p class="art-section-label">${nightCityCopy ? "VELA / WORKSPACE DETAILS" : isAgentFocused ? "VELA / AGENT DEVELOPMENT ENVIRONMENT" : isRepo ? "KEEP THE PROJECT CLOSE" : "WORK WHERE THE WORK IS"}</p><h2>${nightCityCopy?.closeHeading ?? (isAgentFocused ? "Code, run, review." : isRepo ? "A clear place to begin." : "Make yourself at home.")}</h2></div><a class="art-button" href="#inside">${nightCityCopy?.closeAction ?? (isAgentFocused ? "Open the workspace" : "Explore Vela")} <span aria-hidden="true">↑</span></a></section>`}
      </main>
      <footer class="art-footer"><a class="explore-brand" href="../index.html"><span aria-hidden="true">◉</span> vela</a><span>${isRoseTerminal ? "Open source · OpenCode V2 available now · More runtimes planned" : nightCityCopy ? "OpenCode V2 · Explorer · Monaco · integrated terminal" : isAgentFocused ? "Vela · repository-aware agent development environment." : isRepo ? "Built around your repository." : "A home for the coding loop."}</span></footer>`;
  if (design.atmosphere) {
    const hero = document.querySelector(".art-hero");
    const foreground = hero.querySelector(".atmosphere-foreground");
    if (design.terrain === "city") foreground.remove();
    else {
      foreground.remove();
      foreground.classList.add("hero-foreground");
      hero.append(foreground);
    }
    const updateDepth = () => {
      const progress = Math.max(0, Math.min(1, -hero.getBoundingClientRect().top / Math.max(hero.offsetHeight - innerHeight * .35, 1)));
      hero.style.setProperty("--scroll-depth", progress.toFixed(3));
    };
    if (!matchMedia("(prefers-reduced-motion: reduce)").matches) {
      let requested = false;
      addEventListener("scroll", () => {
        if (requested) return;
        requested = true;
        requestAnimationFrame(() => { updateDepth(); requested = false; });
      }, { passive: true });
      addEventListener("resize", updateDepth);
      updateDepth();
    }
  }
}
