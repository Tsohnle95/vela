const velaDetailedGroups = [
  {
    seed: "04-nocturne", group: "nocturne", terrain: "alpine", base: { sky: "#121e32", horizon: "#37536a", foreground: "#112635", glow: "#b6d1cf", ink: "#f4f4ed", accent: "#b7ddd0", light: false },
    variants: [
      ["observatory", "High Observatory", "starwatch / alpine valley", "An observatory dome and delicate star trails above textured peaks.", "Keep the project in your sights.", "Watch the whole session without losing the source beneath it.", "observatory", "#b5d6d4"],
      ["umbra", "Umbra", "eclipse / charcoal ridges", "A fine corona casts shadows across dark, layered stone.", "A little light on the hard parts.", "The repository stays visible while the agent works through the unknown.", "eclipse", "#e8e5cf"],
      ["beacon", "Beacon", "distant beacon / polar sky", "A small illuminated peak carries a signal through the night.", "Find the signal in the source.", "Keep the path between question, file, and answer in view.", "beacon", "#a1e2d4"],
      ["meteor", "Meteor Watch", "meteor shower / mountain pass", "Thin luminous trails cross a broad, craggy nightscape.", "Follow the work as it moves.", "See each change land in the project that started the session.", "meteor", "#e2b9ce"],
      ["blue-hour", "Blue Hour", "blue nebula / distant hills", "Dusty blue light and an almost invisible moon soften the peaks.", "The repo stays your north star.", "Give deep work room to breathe without losing your bearings.", "moon", "#b4cceb"]
    ]
  },
  {
    seed: "04-nocturne-tidal", group: "tidal", terrain: "water", base: { sky: "#15283b", horizon: "#52717b", foreground: "#153543", glow: "#d0dfd5", ink: "#f3f5ec", accent: "#b7d9d1", light: false },
    variants: [
      ["silver-current", "Silver Current", "moon path / dark sea", "A silver path of broken reflections travels toward the horizon.", "Keep the current connected.", "From the prompt to the diff, follow the work back to its source.", "moon", "#e5e5d7"],
      ["tidal-rings", "Tidal Rings", "ringed moon / midnight water", "A smaller ringed world hangs above textured waves.", "Come back to the code.", "Let the repository anchor every change made along the way.", "saturn", "#d5dcdb"],
      ["phosphor", "Phosphor", "bioluminescence / shore", "Luminous foam and tiny lights draw a line along the shore.", "Keep a light on the work.", "See the files under every response as the session moves forward.", "crescent", "#9ce0cf"],
      ["moonwake", "Moonwake", "lunar wake / layered surf", "Moonlight stretches into a wide wake across the dark water.", "Every change has a shoreline.", "Review the work exactly where it meets your project.", "moon", "#f1d9c9"],
      ["night-tide", "Night Tide", "eclipse / glassy bay", "A distant eclipse reflects in a glassy, almost still bay.", "Let the details surface.", "Read, ask, and review with the project always in the picture.", "eclipse", "#c7d5ea"]
    ]
  },
  {
    seed: "04-mist", group: "mist", terrain: "mist", base: { sky: "#ecf2ef", horizon: "#bbd0ca", foreground: "#557e78", glow: "#fffdf0", ink: "#294841", accent: "#477b70", light: true },
    variants: [
      ["moonveil", "Moonveil", "pale moon / forest fog", "Pine silhouettes dissolve into luminous, moonlit fog.", "Clarity starts in the project.", "See the next file, question, and change emerge in context.", "moon", "#f8faeb"],
      ["halo-cloud", "Halo Cloud", "celestial halo / silver cloud", "A fine celestial ring glows through a soft silver cloud bank.", "A clear view through the haze.", "Keep code and agent close enough to follow the whole thread.", "saturn", "#e3f3e9"],
      ["quiet-signal", "Quiet Signal", "star chart / misted hills", "Subtle star-chart lines float above rolling pale hills.", "Follow the quiet signals.", "The repository gives each tool and each change a place to belong.", "beacon", "#dcefe7"],
      ["cloudbreak", "Cloudbreak", "sun break / cold ridgeline", "Light breaks through a layered horizon of frosted stone.", "Let the next step appear.", "Move through the work without letting go of the code.", "crescent", "#fff3d4"],
      ["murmur", "Murmur", "faint aurora / conifers", "A barely visible aurora brushes a line of detailed conifers.", "A calmer way to find your place.", "Follow files and edits through a quieter workspace.", "aurora", "#c0e8d0"]
    ]
  },
  {
    seed: "04-mist-planetfall", group: "planetfall", terrain: "mist", base: { sky: "#dce8ec", horizon: "#a3beca", foreground: "#486b80", glow: "#edf8f2", ink: "#2e4a57", accent: "#507e8b", light: true },
    variants: [
      ["arrival", "Arrival", "distant world / blue valley", "A textured world rises over a deep blue valley of mist.", "Land where the work begins.", "Start with the repository and find the next step from there.", "planet", "#eaf0e2"],
      ["small-world", "Small World", "small ringed world / cloud sea", "A distant ringed world floats above overlapping cloud layers.", "Everything still starts here.", "Let the repo keep the bigger picture while the agent explores.", "saturn", "#e0e7e8"],
      ["first-light", "First Light", "celestial dawn / pale mountains", "A narrow planet edge catches the first light of day.", "See the project in a new light.", "Stay oriented from the first question to the final diff.", "horizon", "#f7e9ce"],
      ["far-shore", "Far Shore", "planet and coast / silver haze", "A distant coast and planet sit inside silver atmospheric haze.", "Keep the source within reach.", "Work outward without leaving the project behind.", "planet", "#cbdde0"],
      ["skybridge", "Skybridge", "arc of light / mist", "A celestial arc bridges a wide, pale mountain pass.", "Bring the work back together.", "The agent, editor, and changes meet in the same repository.", "beacon", "#e5f3ee"]
    ]
  },
  {
    seed: "10-dusk-crescent", group: "crescent", terrain: "city", base: { sky: "#292742", horizon: "#70607f", foreground: "#242841", glow: "#e9c8b3", ink: "#f8f2ec", accent: "#eccdbf", light: false },
    variants: [
      ["rooftop", "Rooftop Moon", "crescent / roof garden", "Lit windows and rooftop foliage sit beneath a slender moon.", "A desk worth coming back to.", "Your editor, agent, and terminal have room to share the night.", "crescent", "#e9d1ba"],
      ["violet-hour", "Violet Hour", "violet twilight / city lights", "Violet haze and a moonrise soften the working city.", "Stay with the whole loop.", "Make the next change without leaving your place in the project.", "moon", "#e1b9d1"],
      ["lunar-street", "Lunar Street", "moon path / evening street", "A reflective street runs between quiet rooftops and the sky.", "Keep the work close after dark.", "The project and its tools stay in one working space.", "crescent", "#d7d8d0"],
      ["celestial-block", "Celestial Block", "vela lines / city block", "Fine vela lines arc above blocks of softly glowing windows.", "Everything has a place to land.", "Move between tools without leaving the project behind.", "saturn", "#e7c2b4"],
      ["night-window", "Night Window", "moon through window / skyline", "A moonlit skyline is framed like a late-night studio view.", "Make room for focused work.", "Stay with your editor and agent through the session.", "moon", "#e7dfce"]
    ]
  },
  {
    seed: "10-dusk", group: "dusk", terrain: "city", base: { sky: "#21283f", horizon: "#5e637b", foreground: "#1c293d", glow: "#edaa85", ink: "#faf3ed", accent: "#eac0a9", light: false },
    variants: [
      ["after-hours", "After Hours", "starry dusk / rooftop lights", "Tiny city lights meet a richly textured evening sky.", "A good place to keep building.", "The everyday coding loop stays calm even when the day runs long.", "moon", "#f0c29d"],
      ["copper-sky", "Copper Sky", "copper planet / distant city", "Warm atmospheric dust and a small planet cross the skyline.", "A little more room to think.", "Keep code, conversation, and changes in one workspace.", "planet", "#f4b99b"],
      ["urban-aurora", "Urban Aurora", "aurora / city lights", "A restrained aurora drifts over streets and lit windows.", "Let the work move naturally.", "The workbench keeps your tools connected to the project.", "aurora", "#b7dcc9"],
      ["starline", "Starline", "star trail / quiet skyline", "A long star trail rises beyond layered, detailed rooftops.", "One desk for every next step.", "Move from asking to editing to reviewing without changing context.", "meteor", "#d8c4d9"],
      ["low-vela", "Low Vela", "small moon / city edge", "A low moon meets a layered city edge at the end of day.", "Stay with the work in front of you.", "All the everyday tools remain within reach.", "horizon", "#ecd6bf"]
    ]
  },
  {
    seed: "10-dusk-vela", group: "vela", terrain: "city", base: { sky: "#172036", horizon: "#53617e", foreground: "#152238", glow: "#edb5a1", ink: "#f8f1ed", accent: "#eac3b3", light: false },
    variants: [
      ["satellite", "Satellite Vela", "small ringed world / satellite", "A tiny satellite visibly travels through a smaller planet's rings.", "Keep the whole desk in vela.", "Your tools have a shared center: the project you're working on.", "saturn", "#efc2b1"],
      ["ice-rings", "Ice Rings", "icy rings / night city", "A cool blue ring system casts a subtle glow across the city.", "Find your working center.", "Keep code and agent together through every step.", "saturn", "#b6d3e2"],
      ["ember-rings", "Ember Rings", "copper rings / dark skyline", "Copper-colored rings and a warm orb hover beyond rooftops.", "Stay close to the work.", "The editor, session, and changes belong to one desk.", "saturn", "#f0b491"],
      ["twin-rings", "Twin Rings", "two satellites / celestial path", "Two small lights trace a fine path around a distant planet.", "Make space for collaboration.", "Follow the session without losing sight of the files.", "saturn", "#d1c8e4"],
      ["outer-ring", "Outer Ring", "distant rings / midnight skyline", "A small planet and wide outer ring sit quietly behind the city.", "Keep the work in motion.", "From the first file to the final review, stay in the same space.", "saturn", "#dfcabd"]
    ]
  }
];

const velaDetailedExplorations = velaDetailedGroups.flatMap(({ seed, group, terrain, base, variants }) => {
  const parent = velaExplorations.find((item) => item.id === seed);
  const colorShift = (hex, amount) => `#${[1, 3, 5].map((offset) => Math.max(0, Math.min(255, parseInt(hex.slice(offset, offset + 2), 16) + amount)).toString(16).padStart(2, "0")).join("")}`;
  const shifts = [-10, 5, -16, 12, -4];
  return variants.map(([slug, name, motif, note, title, lede, celestial, tint], index) => ({
    ...parent, ...base, id: `${seed}-detail-${slug}`, parent: seed, group, terrain,
    name, motif, note, title, lede, celestial, tint, detailIndex: index, atmosphere: true,
    sky: colorShift(base.sky, shifts[index]), horizon: colorShift(base.horizon, shifts[(index + 2) % 5]),
    foreground: colorShift(base.foreground, shifts[(index + 3) % 5]),
    form: ["grounded", "inset", "floating", "framed", "diagram"][index],
    message: `${name}: keep the next step connected to the work.`
  }));
});

const selectedCelestials = ["crescent", "moon", "moon", "planet", "crescent", "moon", "saturn"];
velaDetailedGroups.forEach(({ seed, terrain }, index) => {
  Object.assign(velaExplorations.find((item) => item.id === seed), {
    terrain, celestial: selectedCelestials[index], atmosphere: true, detailIndex: index
  });
});

velaExplorations.push(...velaDetailedExplorations);

const violetHourPalette = {
  sky: "#2e2c47", horizon: "#7c6c8b", foreground: "#20243d", glow: "#e9c8b3",
  ink: "#f8f2ec", accent: "#eccdbf", tint: "#e1b9d1", light: false
};
const violetHourSeed = velaDetailedExplorations.find((item) => item.id === "10-dusk-crescent-detail-violet-hour");
Object.assign(violetHourSeed, { layout: "centered", terrain: "city" });

const nocturneTidalSeed = velaExplorations.find((item) => item.id === "04-nocturne-tidal");
Object.assign(nocturneTidalSeed, violetHourPalette, { terrain: "city", celestial: "moon", atmosphere: true });

const velaFocusedGroups = [
  {
    seed: violetHourSeed.id, group: "violet-center", title: "Violet Hour · centered over the city.", terrain: "city", layout: "centered",
    variants: [
      ["lilac-afterglow", "Lilac Afterglow", "lilac moon / distant towers", "A lilac moon settles over a deep, many-layered skyline.", "Stay for the whole loop.", "Make the next change without leaving your place in the project.", "moon", { sky: "#292741", horizon: "#716983", foreground: "#20233d", glow: "#e6d3ea", tint: "#d6c4e8", accent: "#ddc9e8" }],
      ["plum-vela", "Plum Vela", "ringed plum world / city lights", "A small ringed world floats above a plum-lit city.", "Let every tool find its place.", "Code, agent, and review move together around the project.", "saturn", { sky: "#30243d", horizon: "#765774", foreground: "#251e38", glow: "#edb9c9", tint: "#dab2df", accent: "#ecc5d6" }],
      ["amethyst-window", "Amethyst Window", "crescent / violet windows", "A crescent glow catches on windows across the dark skyline.", "Keep your focus in vela.", "Move between the editor and the agent without losing context.", "crescent", { sky: "#29263f", horizon: "#6c648a", foreground: "#20243d", glow: "#ded0ed", tint: "#cbbbe8", accent: "#d8c9eb" }],
      ["rose-nebula", "Rose Nebula", "eclipsed rose / city haze", "A rose-colored corona diffuses across distant buildings.", "A little more room to think.", "The whole coding loop stays calm and connected in one workspace.", "eclipse", { sky: "#34243e", horizon: "#805a76", foreground: "#281f39", glow: "#f0b8c8", tint: "#e6a9cf", accent: "#edc2d3" }],
      ["lavender-rise", "Lavender Rise", "pale planet / lavender skyline", "A pale lavender world rises behind the city's outer towers.", "See the work from one place.", "Follow the session from the first file to the final review.", "planet", { sky: "#28243e", horizon: "#6a6182", foreground: "#20233d", glow: "#e3c7df", tint: "#d4bce4", accent: "#ddc9e5" }]
    ]
  },
  {
    seed: nocturneTidalSeed.id, group: "tidal-violet-city", title: "Nocturne Tidal · violet light, distant towers.", terrain: "city",
    variants: [
      ["violet-quay", "Violet Quay", "violet moon / layered waterfront city", "The old tidal calm meets a deep-violet city at night.", "The project stays your shoreline.", "Keep the source in view as the agent moves through the work.", "moon", { sky: "#24213b", horizon: "#625875", foreground: "#1d223a", glow: "#e7c3cf", tint: "#d9b4d6", accent: "#e5c4d3" }],
      ["amethyst-harbor", "Amethyst Harbor", "ringed world / harbor lights", "A small amethyst world hangs above a glimmering city harbor.", "Keep the signal above the tide.", "Questions and changes both lead back to the same repository.", "saturn", { sky: "#29243f", horizon: "#746381", foreground: "#20233d", glow: "#edc4d2", tint: "#cdb4e6", accent: "#d7c3ec" }],
      ["orchid-night", "Orchid Night", "orchid crescent / close-set towers", "An orchid crescent softens the edges of a close-set skyline.", "Stay close to what matters.", "Let every next step land in the project you already know.", "crescent", { sky: "#35253e", horizon: "#805d78", foreground: "#281f3c", glow: "#f0b8c8", tint: "#dda9cd", accent: "#ecc3d1" }],
      ["violet-spires", "Violet Spires", "eclipse / distant spires", "A fine eclipse halo silhouettes a row of distant towers.", "Bring every path back to source.", "Keep code, conversation, and the resulting diff connected.", "eclipse", { sky: "#252540", horizon: "#625f88", foreground: "#1c233c", glow: "#d9d6ed", tint: "#c9c3ef", accent: "#d8d3ef" }],
      ["rose-night", "Rose Night", "rose planet / night windows", "A softly lit rose world hangs above violet window lights.", "Keep the details within reach.", "The repository holds the thread from question through review.", "planet", { sky: "#30253f", horizon: "#785d7c", foreground: "#241f3a", glow: "#efc1b8", tint: "#e5b3d0", accent: "#eac5d1" }]
    ]
  }
];

const velaFocusedExplorations = velaFocusedGroups.flatMap(({ seed, group, terrain, layout, variants }) => {
  const parent = velaExplorations.find((item) => item.id === seed);
  return variants.map(([slug, name, motif, note, title, lede, celestial, colors], index) => ({
    ...parent, ...colors, id: `${seed}-city-${slug}`, parent: seed, group, terrain, layout,
    name, motif, note, title, lede, celestial, atmosphere: true, detailIndex: index,
    form: ["grounded", "inset", "floating", "framed", "diagram"][index],
    message: `${name}: the city moves, the project stays in view.`
  }));
});

velaExplorations.push(...velaFocusedExplorations);

const roseNightSeed = velaFocusedExplorations.find((item) => item.id === "04-nocturne-tidal-city-rose-night");
Object.assign(violetHourSeed, {
  productCopy: true,
  note: "A centered Vela agent workspace over a violet city skyline.",
  title: "Code and agents in one workspace.",
  lede: "Vela brings your repository, editor, agent session, terminal, and file diffs into one desktop environment.",
  section: "Run agents against the open repository.",
  message: "Open a repository. Run an agent. Review its diffs."
});
Object.assign(roseNightSeed, {
  productCopy: true,
  note: "A repository-aware agent workspace framed by a rose-lit skyline.",
  title: "Run agents in repository context.",
  lede: "Open a codebase, launch an agent task, and inspect every changed file in Vela.",
  section: "Agent tasks tied to their source files.",
  message: "Run the task in context. Inspect the changed files."
});

const velaCityWorkflowGroups = [
  {
    seed: violetHourSeed.id, group: "violet-agent-city", title: "Violet Hour · Vela agent workspace.", terrain: "city", layout: "centered",
    variants: [
      { slug: "agent-console", name: "Agent Console", motif: "editor / agent / terminal", note: "Run an OpenCode V2 agent beside the editor and integrated terminal.", title: "Run coding agents beside your code.", lede: "Vela connects the open repository to an OpenCode V2 agent session, editor, terminal, and file diffs.", section: "Run an agent task in repository context.", message: "Open a repository. Run an agent. Review its diffs.", celestial: "moon", colors: { sky: "#2a2943", horizon: "#716783", foreground: "#20243d", glow: "#e7cbd9", tint: "#d8c0e0", accent: "#e6c9dc" } },
      { slug: "context-window", name: "Context Window", motif: "repository context / violet skyline", note: "Keep the agent's file context and resulting edits visible together.", title: "Keep agent work tied to the repository.", lede: "See which project files inform the task, then inspect the changes from the same workspace.", section: "Repository context before and after each task.", message: "See the files the agent reads and changes.", celestial: "crescent", colors: { sky: "#28263f", horizon: "#6e6283", foreground: "#1f233c", glow: "#dfc9e5", tint: "#ccbee0", accent: "#dccce8" } },
      { slug: "diff-review", name: "Diff Review", motif: "file diffs / city lights", note: "Review the agent's file-by-file changes in repository context.", title: "Review every agent file change.", lede: "Vela keeps the agent session beside its file-by-file edits, so you can inspect the patch in its repository context.", section: "Keep agent edits attached to their source files.", message: "Review each changed file alongside the agent session.", celestial: "eclipse", colors: { sky: "#30253e", horizon: "#785874", foreground: "#241f3a", glow: "#efc0d0", tint: "#e3add1", accent: "#eec6d8" } },
      { slug: "task-trace", name: "Task Trace", motif: "agent task / changed files", note: "Follow a task from its prompt through the files it updates.", title: "Follow agent edits from prompt to file.", lede: "Run a task in Vela, track its repository activity, and review the resulting diff.", section: "See where each task changes the code.", message: "Follow the task from prompt to patch.", celestial: "planet", colors: { sky: "#292741", horizon: "#716582", foreground: "#20243e", glow: "#e5c7de", tint: "#d3b5df", accent: "#e5c9e3" } },
      { slug: "workspace-view", name: "Workspace View", motif: "one codebase / connected tools", note: "Use editor, agent, terminal, and diff review around one project.", title: "Use editor, terminal, and agent together.", lede: "Vela connects daily development tools to the repository you opened.", section: "One project shared by every tool.", message: "Editor, agent, terminal, and diffs share context.", celestial: "aurora", colors: { sky: "#2c2742", horizon: "#75637f", foreground: "#22233d", glow: "#e8c2d9", tint: "#d7b9dc", accent: "#ebc8de" } },
      { slug: "build-session", name: "Build Session", motif: "agent task / review workflow", note: "Start an agent task, follow its work, and inspect its file diffs.", title: "Run a task and review its file diffs.", lede: "Keep the codebase open while Vela's agent works, then inspect each edit before continuing.", section: "From agent run to reviewed changes.", message: "Run the task. Check the patch. Keep control.", celestial: "moon", colors: { sky: "#272640", horizon: "#6a6387", foreground: "#1f233b", glow: "#dfc9df", tint: "#d0c0e6", accent: "#e3cfe8" } }
    ]
  },
  {
    seed: roseNightSeed.id, group: "rose-agent-city", title: "Nocturne Tidal · agent work in context.", terrain: "city",
    variants: [
      { slug: "rose-terminal", name: "Rose Terminal", motif: "agent run / integrated terminal", note: "Launch an agent task and run project commands from Vela.", title: "Run an agent task in the open repository.", lede: "Vela keeps the codebase, agent session, terminal output, and changed files in one desktop workspace.", section: "Run agents without losing project context.", message: "Keep agent runs attached to the open repo.", celestial: "planet", colors: { sky: "#30243e", horizon: "#795b79", foreground: "#241e39", glow: "#efc1c0", tint: "#e3b1d0", accent: "#ecc9d2" } },
      { slug: "change-trace", name: "Change Trace", motif: "changed files / source context", note: "Trace each generated edit back to the file the agent changed.", title: "Trace every agent edit to its source file.", lede: "Vela shows repository changes beside the agent session that produced them.", section: "Connect each edit to its source.", message: "See which files the agent changed and why.", celestial: "crescent", colors: { sky: "#2e243e", horizon: "#765a7b", foreground: "#231f3a", glow: "#ecc5d1", tint: "#dcaed1", accent: "#e9c8d8" } },
      { slug: "session-review", name: "Session Review", motif: "agent session / patch review", note: "Inspect the resulting patch before accepting agent changes.", title: "Review generated changes in context.", lede: "Keep the agent conversation and repository diffs connected while you check its work.", section: "Review the agent's patch in Vela.", message: "Check the diff before accepting agent edits.", celestial: "eclipse", colors: { sky: "#30263f", horizon: "#7a5d7a", foreground: "#241f3b", glow: "#f0c5c4", tint: "#e3bad8", accent: "#efccd8" } },
      { slug: "project-control", name: "Project Control", motif: "editor / agent / one codebase", note: "Keep the editor and coding agent connected to one repository.", title: "Keep the editor and agent on one codebase.", lede: "Vela supplies repository context to the agent and keeps edits visible in the development environment.", section: "Your codebase stays the shared context.", message: "The agent works against the project you opened.", celestial: "moon", colors: { sky: "#29243d", horizon: "#715b7b", foreground: "#211f39", glow: "#e9c3ca", tint: "#dcb6d8", accent: "#e9cbd8" } },
      { slug: "diff-queue", name: "Diff Review", motif: "file changes / review", note: "Inspect the agent's changed files alongside the session that produced them.", title: "Review each changed file in Vela.", lede: "The workspace keeps file diffs connected to the project and agent session that produced them.", section: "Inspect agent edits beside their source files.", message: "Review the patch in the context of the open repository.", celestial: "beacon", colors: { sky: "#32243f", horizon: "#7c5b78", foreground: "#261f3a", glow: "#efc0c4", tint: "#e5afd1", accent: "#edc9d6" } },
      { slug: "night-build", name: "Night Build", motif: "repository task / reviewed patch", note: "Move from repository task to reviewed patch in one environment.", title: "From agent task to reviewed patch.", lede: "Run the agent in your codebase, track its edits, and review the final diffs in Vela.", section: "A direct path from task to diff.", message: "Run the task. Inspect the patch. Continue coding.", celestial: "planet", colors: { sky: "#2b233d", horizon: "#755876", foreground: "#211e38", glow: "#ebc0c8", tint: "#ddb5d5", accent: "#e9c9da" } }
    ]
  }
];

const velaCityWorkflowExplorations = velaCityWorkflowGroups.flatMap(({ seed, group, terrain, layout, variants }) => {
  const parent = velaExplorations.find((item) => item.id === seed);
  return variants.map((variant, index) => ({
    ...parent, ...variant.colors,
    id: `${seed}-workflow-${variant.slug}`, parent: seed, group, terrain,
    layout: layout ?? parent.layout, productCopy: true, atmosphere: true,
    name: variant.name, motif: variant.motif, note: variant.note,
    title: variant.title, lede: variant.lede, section: variant.section,
    message: variant.message, celestial: variant.celestial, detailIndex: index,
    form: ["grounded", "inset", "floating", "framed", "diagram", "grounded"][index]
  }));
});

velaExplorations.push(...velaCityWorkflowExplorations);
