const icon = (name) => `<svg aria-hidden="true"><use href="#icon-${name}"></use></svg>`;

const treatments = [
  {
    name: "Whisper",
    titleSize: 11.75,
    titleWeight: 400,
    rowHeight: 28,
    meta: "none",
    active: "icon",
    search: "none",
    section: "titlecase",
    count: "none",
    action: "inline",
    rhythm: "row-tight",
    tone: "title-faint",
    indent: "indent-quiet",
    line: "tree-line-none"
  },
  {
    name: "File Rhythm",
    titleSize: 12.5,
    titleWeight: 500,
    rowHeight: 28,
    meta: "inline",
    active: "wash",
    search: "line",
    section: "upper",
    count: "plain",
    action: "head",
    rhythm: "row-tight",
    tone: "title-active-only",
    indent: "indent-quiet",
    line: "tree-line"
  },
  {
    name: "Sage Rail",
    titleSize: 12.25,
    titleWeight: 450,
    rowHeight: 32,
    meta: "right",
    active: "rail",
    search: "inset",
    section: "upper",
    count: "none",
    action: "toolbar",
    rhythm: "row-tight",
    tone: "title-active-only",
    indent: "indent-deep",
    line: "tree-line"
  },
  {
    name: "Soft Wash",
    titleSize: 12.5,
    titleWeight: 500,
    rowHeight: 34,
    meta: "stack",
    active: "wash",
    search: "inset",
    section: "titlecase",
    count: "pill",
    action: "toolbar",
    rhythm: "row-tight",
    tone: "title-active-only",
    indent: "indent-quiet",
    line: "tree-line"
  },
  {
    name: "Open Air",
    titleSize: 13,
    titleWeight: 400,
    rowHeight: 40,
    meta: "stack",
    active: "dot",
    search: "quiet",
    section: "titlecase",
    count: "none",
    action: "inline",
    rhythm: "row-open",
    tone: "title-active-only",
    indent: "indent-deep",
    line: "tree-line-none"
  },
  {
    name: "Quiet Index",
    titleSize: 12,
    titleWeight: 450,
    rowHeight: 30,
    meta: "right",
    active: "underline",
    search: "none",
    section: "quiet",
    count: "plain",
    action: "section",
    rhythm: "row-tight",
    tone: "title-faint",
    indent: "indent-quiet",
    line: "tree-line-none"
  },
  {
    name: "Cream Field",
    titleSize: 12.75,
    titleWeight: 500,
    rowHeight: 36,
    meta: "stack",
    active: "text",
    search: "inset",
    section: "titlecase",
    count: "pill",
    action: "toolbar",
    rhythm: "row-open",
    tone: "title-active-only",
    indent: "indent-deep",
    line: "tree-line"
  },
  {
    name: "Bare Line",
    titleSize: 12.25,
    titleWeight: 400,
    rowHeight: 30,
    meta: "inline",
    active: "underline",
    search: "line",
    section: "upper",
    count: "none",
    action: "bottom",
    rhythm: "row-tight",
    tone: "title-faint",
    indent: "indent-quiet",
    line: "tree-line-none"
  },
  {
    name: "Muted Focus",
    titleSize: 12.5,
    titleWeight: 525,
    rowHeight: 34,
    meta: "right",
    active: "icon",
    search: "quiet",
    section: "quiet",
    count: "plain",
    action: "head",
    rhythm: "row-tight",
    tone: "title-active-only",
    indent: "indent-deep",
    line: "tree-line"
  },
  {
    name: "Gentle Compact",
    titleSize: 12,
    titleWeight: 500,
    rowHeight: 28,
    meta: "none",
    active: "dot",
    search: "none",
    section: "upper",
    count: "pill",
    action: "section",
    rhythm: "row-tight",
    tone: "title-active-only",
    indent: "indent-quiet",
    line: "tree-line"
  }
];

const sessions = {
  panel: {
    title: "Refine sessions panel",
    meta: "vela · just now",
    time: "now",
    live: true,
    active: true
  },
  usage: {
    title: "Provider usage cleanup",
    meta: "vela · 18m ago",
    time: "18m",
    live: true
  },
  docs: {
    title: "Update runtime docs",
    meta: "vela · yesterday",
    time: "1d"
  },
  notes: {
    title: "Polish daily notes",
    meta: "atlas-notes · 2h ago",
    time: "2h"
  },
  search: {
    title: "Improve quick search",
    meta: "atlas-notes · Tuesday",
    time: "Tue"
  },
  landing: {
    title: "Soften landing layout",
    meta: "quiet-web · 4d ago",
    time: "4d"
  },
  tokens: {
    title: "Review color tokens",
    meta: "quiet-web · last week",
    time: "6d"
  },
  queue: {
    title: "Tune message queue",
    meta: "vela · 32m ago",
    time: "32m",
    live: true
  }
};

const rowIcon = (item, kind = "history") => {
  if (kind === "dot" || item.live) return `<span class="panel-status-dot ${item.live ? "live" : ""}"></span>`;
  if (kind === "star") return `<span class="panel-row-icon">${icon("star")}</span>`;
  if (kind === "file") return `<span class="panel-row-icon">${icon("file")}</span>`;
  return `<span class="panel-row-icon">${icon("history")}</span>`;
};

const sessionRow = (item, options = {}) => {
  const kind = options.icon || "history";
  const active = item.active || options.active;
  const trailing = options.trailing === false
    ? ""
    : `<button class="panel-row-action" type="button" tabindex="-1" aria-label="Session actions">${icon("more")}</button>`;
  return `
    <div class="panel-row ${active ? "is-active" : ""}">
      ${rowIcon(item, kind)}
      <span class="panel-row-main">
        <span class="panel-row-title">${item.title}</span>
        <span class="panel-row-meta">${options.meta || item.meta}</span>
      </span>
      <span class="panel-row-time">${item.time}</span>
      ${trailing}
    </div>
  `;
};

const indexRow = (letter, item, path) => `
  <div class="panel-index-row ${item.active ? "is-active" : ""}">
    <span class="panel-index-letter">${letter}</span>
    <span class="panel-index-title">${item.title}</span>
    <span class="panel-index-meta">${path}</span>
    <span class="panel-row-time">${item.time}</span>
  </div>
`;

const projectRow = (name, count, open = false) => `
  <div class="panel-project-row ${open ? "open" : ""}">
    <span class="panel-chevron">${icon("chevron")}</span>
    <span class="panel-project-icon">${icon("folder")}</span>
    <span class="panel-project-name">${name}</span>
    <span class="panel-project-meta">${count}</span>
  </div>
`;

const section = (label, count, content, treatment, options = {}) => {
  const action = options.action && treatment.action === "section"
    ? `<button class="panel-section-action" type="button" tabindex="-1" aria-label="New session">${icon("plus")}</button>`
    : "";
  return `
    <section class="panel-section ${options.className || ""}">
      <div class="panel-section-head">
        <span>${label}</span>
        <span class="panel-count push">${count}</span>
        ${action}
      </div>
      <div class="panel-section-list">${content}</div>
    </section>
  `;
};

const renderMirrorTree = (treatment) => section(
  "Workspaces",
  "3",
  `
    <div class="panel-project">
      ${projectRow("vela", "3", true)}
      <div class="panel-tree-children">
        ${sessionRow(sessions.panel)}
        ${sessionRow(sessions.usage)}
        ${sessionRow(sessions.docs)}
      </div>
    </div>
    <div class="panel-project">
      ${projectRow("atlas-notes", "2", true)}
      <div class="panel-tree-children">
        ${sessionRow(sessions.notes)}
        ${sessionRow(sessions.search)}
      </div>
    </div>
    ${projectRow("quiet-web", "2")}
  `,
  treatment,
  { action: true }
);

const renderQuietRecents = (treatment) => `
  ${section(
    "Today",
    "4",
    sessionRow(sessions.panel) + sessionRow(sessions.usage) + sessionRow(sessions.queue) + sessionRow(sessions.notes),
    treatment,
    { action: true }
  )}
  ${section(
    "Earlier",
    "3",
    sessionRow(sessions.search) + sessionRow(sessions.landing) + sessionRow(sessions.docs),
    treatment
  )}
`;

const renderWorkspaceGroups = (treatment) => `
  ${section(
    "vela",
    "4",
    sessionRow(sessions.panel, { icon: "file" }) + sessionRow(sessions.usage, { icon: "file" }) + sessionRow(sessions.queue, { icon: "file" }) + sessionRow(sessions.docs, { icon: "file" }),
    treatment,
    { action: true }
  )}
  ${section(
    "atlas-notes",
    "2",
    sessionRow(sessions.notes, { icon: "file" }) + sessionRow(sessions.search, { icon: "file" }),
    treatment
  )}
`;

const renderOpenHistory = (treatment) => `
  ${section(
    "Open now",
    "3",
    sessionRow(sessions.panel, { icon: "dot" }) + sessionRow(sessions.usage, { icon: "dot" }) + sessionRow(sessions.queue, { icon: "dot" }),
    treatment,
    { action: true }
  )}
  ${section(
    "History",
    "5",
    sessionRow(sessions.notes) + sessionRow(sessions.search) + sessionRow(sessions.landing) + sessionRow(sessions.docs),
    treatment
  )}
`;

const renderPinnedFirst = (treatment) => `
  ${section(
    "Pinned",
    "3",
    sessionRow(sessions.panel, { icon: "star" }) + sessionRow(sessions.notes, { icon: "star" }) + sessionRow(sessions.landing, { icon: "star" }),
    treatment,
    { action: true }
  )}
  ${section(
    "All sessions",
    "8",
    sessionRow(sessions.usage) + sessionRow(sessions.queue) + sessionRow(sessions.search) + sessionRow(sessions.docs),
    treatment
  )}
`;

const renderCompactIndex = (treatment) => section(
  "Session index",
  "8",
  `
    ${indexRow("I", sessions.search, "atlas-notes")}
    ${indexRow("P", sessions.notes, "atlas-notes")}
    ${indexRow("P", sessions.usage, "vela")}
    ${indexRow("R", sessions.panel, "vela")}
    ${indexRow("R", sessions.tokens, "quiet-web")}
    ${indexRow("S", sessions.landing, "quiet-web")}
    ${indexRow("T", sessions.queue, "vela")}
    ${indexRow("U", sessions.docs, "vela")}
  `,
  treatment,
  { action: true }
);

const renderTimeline = (treatment) => `
  ${section(
    "Today",
    "4",
    `<div class="panel-timeline">${sessionRow(sessions.panel)}${sessionRow(sessions.usage)}${sessionRow(sessions.queue)}${sessionRow(sessions.notes)}</div>`,
    treatment,
    { action: true }
  )}
  ${section(
    "This week",
    "3",
    `<div class="panel-timeline">${sessionRow(sessions.search)}${sessionRow(sessions.landing)}${sessionRow(sessions.docs)}</div>`,
    treatment
  )}
`;

const renderSearchLed = (treatment) => `
  <label class="panel-search force-search">
    ${icon("search")}
    <input type="text" value="panel" readonly tabindex="-1" aria-label="Search sessions" />
  </label>
  ${section(
    "3 matches",
    "",
    sessionRow(sessions.panel) + sessionRow(sessions.landing) + sessionRow(sessions.tokens),
    treatment,
    { action: true }
  )}
  ${section(
    "Recently opened",
    "3",
    sessionRow(sessions.usage) + sessionRow(sessions.notes) + sessionRow(sessions.queue),
    treatment
  )}
`;

const renderProjectSwitcher = (treatment) => section(
  "Projects",
  "3",
  `
    <div class="panel-project">
      ${projectRow("vela", "4", true)}
      <div class="panel-tree-children">
        ${sessionRow(sessions.panel, { trailing: false })}
        ${sessionRow(sessions.usage, { trailing: false })}
        ${sessionRow(sessions.queue, { trailing: false })}
        ${sessionRow(sessions.docs, { trailing: false })}
      </div>
    </div>
    ${projectRow("atlas-notes", "2")}
    ${projectRow("quiet-web", "2")}
  `,
  treatment,
  { action: true }
);

const renderUltraMinimal = (treatment) => `
  ${section(
    "Sessions",
    "8",
    sessionRow(sessions.panel, { trailing: false }) +
      sessionRow(sessions.usage, { trailing: false }) +
      sessionRow(sessions.queue, { trailing: false }) +
      sessionRow(sessions.notes, { trailing: false }) +
      sessionRow(sessions.search, { trailing: false }) +
      sessionRow(sessions.landing, { trailing: false }) +
      sessionRow(sessions.docs, { trailing: false }),
    treatment,
    { action: true }
  )}
`;

const families = [
  {
    id: "mirror-tree",
    name: "Mirror Tree",
    label: "File-tree mirror",
    description: "Workspace roots with nested session rows",
    render: renderMirrorTree
  },
  {
    id: "quiet-recents",
    name: "Quiet Recents",
    label: "Recency sections",
    description: "A chronological stream with restrained headings",
    render: renderQuietRecents
  },
  {
    id: "workspace-groups",
    name: "Workspace Groups",
    label: "Workspace sections",
    description: "Flat lists grouped by repository",
    render: renderWorkspaceGroups
  },
  {
    id: "open-history",
    name: "Open + History",
    label: "State first",
    description: "Live work separated from session history",
    render: renderOpenHistory
  },
  {
    id: "pinned-first",
    name: "Pinned First",
    label: "Favorites first",
    description: "Pinned work before the complete list",
    render: renderPinnedFirst
  },
  {
    id: "compact-index",
    name: "Compact Index",
    label: "Single-line index",
    description: "File-list density with inline project context",
    render: renderCompactIndex
  },
  {
    id: "gentle-timeline",
    name: "Gentle Timeline",
    label: "Time-based tree",
    description: "A tree-line timeline using existing indentation",
    render: renderTimeline
  },
  {
    id: "search-led",
    name: "Search Led",
    label: "Filter first",
    description: "Search results lead, recents remain secondary",
    ownsSearch: true,
    render: renderSearchLed
  },
  {
    id: "project-switcher",
    name: "Project Switcher",
    label: "Project roots",
    description: "Projects behave like file-panel workspace roots",
    render: renderProjectSwitcher
  },
  {
    id: "ultra-minimal",
    name: "Ultra Minimal",
    label: "Bare session list",
    description: "One quiet list with almost no supporting chrome",
    ownsSearch: true,
    render: renderUltraMinimal
  }
];

const renderTop = (treatment) => `
  <div class="panel-top">
    <span class="panel-top-actions">
      <button class="panel-icon-button" type="button" tabindex="-1" aria-label="Collapse sidebar">${icon("expand")}</button>
      <button class="panel-icon-button" type="button" tabindex="-1" aria-label="Switch folder">${icon("folder")}</button>
    </span>
    ${treatment.action === "head" ? `<span class="panel-top-actions"><button class="panel-icon-button" type="button" tabindex="-1" aria-label="New session">${icon("plus")}</button></span>` : ""}
  </div>
`;

const renderTabs = () => `
  <div class="panel-tabs">
    <button class="panel-tab active" type="button" tabindex="-1">Sessions</button>
    <button class="panel-tab" type="button" tabindex="-1">Files</button>
  </div>
`;

const renderToolbar = (treatment) => {
  if (treatment.action === "toolbar") {
    return `
      <div class="panel-toolbar">
        <button class="panel-new-button" type="button" tabindex="-1">${icon("plus")}<span>New session</span></button>
        <button class="panel-small-button" type="button" tabindex="-1" aria-label="Open file">${icon("file")}</button>
        <button class="panel-small-button" type="button" tabindex="-1">Plugins</button>
      </div>
    `;
  }
  if (treatment.action === "inline") {
    return `<button class="panel-inline-new" type="button" tabindex="-1">${icon("plus")}<span>New session</span></button>`;
  }
  return "";
};

const renderSearch = (treatment) => `
  <label class="panel-search">
    ${icon("search")}
    <input type="text" placeholder="Search sessions…" readonly tabindex="-1" aria-label="Search sessions" />
  </label>
`;

const treatmentClasses = (treatment) => [
  `meta-${treatment.meta}`,
  `active-${treatment.active}`,
  `search-${treatment.search}`,
  `section-${treatment.section}`,
  `count-${treatment.count}`,
  treatment.rhythm,
  treatment.tone,
  treatment.indent,
  treatment.line,
  treatment.action === "section" ? "section-actions-mode" : ""
].filter(Boolean).join(" ");

const renderPanel = (family, treatment) => `
  <div
    class="mock-panel ${treatmentClasses(treatment)}"
    style="--row-height: ${treatment.rowHeight}px; --title-size: ${treatment.titleSize}px; --title-weight: ${treatment.titleWeight};"
  >
    ${renderTop(treatment)}
    ${renderTabs()}
    <div class="panel-body">
      ${renderToolbar(treatment)}
      ${family.ownsSearch ? "" : renderSearch(treatment)}
      <div class="panel-scroll">${family.render(treatment)}</div>
      ${treatment.action === "bottom" ? `<button class="panel-inline-new push-bottom" type="button" tabindex="-1">${icon("plus")}<span>New session</span></button>` : ""}
    </div>
  </div>
`;

const activeLabel = {
  icon: "sage icon",
  wash: "sage wash",
  rail: "sage rail",
  dot: "sage dot",
  underline: "sage line",
  text: "sage text"
};

const designs = families.flatMap((family, familyIndex) => treatments.map((treatment, treatmentIndex) => {
  const number = familyIndex * treatments.length + treatmentIndex + 1;
  return {
    id: String(number).padStart(3, "0"),
    family,
    treatment,
    name: `${family.name} · ${treatment.name}`
  };
}));

const gallery = document.getElementById("design-gallery");
const searchInput = document.getElementById("design-search");
const familyFilter = document.getElementById("family-filter");
const resultsCount = document.getElementById("results-count");
const selectionCount = document.getElementById("selection-count");
const selectionTray = document.getElementById("selection-tray");
const selectionItems = document.getElementById("selection-items");
const selectionClear = document.getElementById("selection-clear");
const themeToggle = document.getElementById("theme-toggle");
const selected = [];

families.forEach((family) => {
  const option = document.createElement("option");
  option.value = family.id;
  option.textContent = `${family.name} · 10`;
  familyFilter.append(option);
});

gallery.innerHTML = designs.map((design) => `
  <article
    class="design-card"
    data-id="${design.id}"
    data-family="${design.family.id}"
    data-search="${design.id} ${design.name.toLowerCase()} ${design.family.description.toLowerCase()} ${design.treatment.titleSize} ${design.treatment.titleWeight} ${design.treatment.rowHeight} ${design.treatment.active} ${design.treatment.meta}"
  >
    <div class="design-card-head">
      <span class="design-number">${design.id}</span>
      <span class="design-title-wrap">
        <span class="design-title">${design.name}</span>
        <span class="design-family">${design.family.label}</span>
      </span>
      <button class="design-select" type="button" aria-pressed="false">Select</button>
    </div>
    ${renderPanel(design.family, design.treatment)}
    <div class="design-spec">${design.treatment.titleSize}px / ${design.treatment.titleWeight} · ${design.treatment.rowHeight}px rows · ${activeLabel[design.treatment.active]} · ${design.treatment.meta} meta</div>
  </article>
`).join("");

const updateSelection = () => {
  const cards = gallery.querySelectorAll(".design-card");
  cards.forEach((card) => {
    const isSelected = selected.includes(card.dataset.id);
    card.classList.toggle("selected", isSelected);
    const button = card.querySelector(".design-select");
    button.setAttribute("aria-pressed", String(isSelected));
    button.textContent = isSelected ? "Selected" : "Select";
  });
  selectionCount.textContent = selected.length === 0
    ? "Nothing selected"
    : `${selected.length} selected${selected.length === 4 ? " · shortlist full" : ""}`;
  selectionTray.classList.toggle("visible", selected.length > 0);
  selectionItems.innerHTML = selected.map((id) => {
    const design = designs.find((item) => item.id === id);
    return `<span class="selection-item"><strong>${design.id}</strong><span>${design.name}</span></span>`;
  }).join("");
};

const applyFilters = () => {
  const query = searchInput.value.trim().toLowerCase();
  const family = familyFilter.value;
  let visible = 0;
  gallery.querySelectorAll(".design-card").forEach((card) => {
    const matchesQuery = query === "" || card.dataset.search.includes(query);
    const matchesFamily = family === "all" || card.dataset.family === family;
    const show = matchesQuery && matchesFamily;
    card.hidden = !show;
    if (show) visible += 1;
  });
  const empty = gallery.querySelector(".no-results");
  if (empty) empty.remove();
  if (visible === 0) {
    gallery.insertAdjacentHTML("beforeend", `<div class="no-results">No directions match that filter.</div>`);
  }
  resultsCount.textContent = `${visible} direction${visible === 1 ? "" : "s"}`;
};

gallery.addEventListener("click", (event) => {
  const button = event.target.closest(".design-select");
  if (!button) return;
  const card = button.closest(".design-card");
  const id = card.dataset.id;
  const index = selected.indexOf(id);
  if (index >= 0) selected.splice(index, 1);
  else {
    if (selected.length === 4) selected.shift();
    selected.push(id);
  }
  updateSelection();
});

searchInput.addEventListener("input", applyFilters);
familyFilter.addEventListener("change", applyFilters);
selectionClear.addEventListener("click", () => {
  selected.splice(0, selected.length);
  updateSelection();
});
themeToggle.addEventListener("click", () => {
  const dark = document.documentElement.dataset.theme === "dark";
  document.documentElement.dataset.theme = dark ? "paper" : "dark";
  themeToggle.textContent = dark ? "Dark preview" : "Paper preview";
});

updateSelection();
applyFilters();
