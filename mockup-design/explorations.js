const velaExplorations = [
  {
    id: "04-canopy", family: "04", name: "Canopy", motif: "botanical canopy", note: "A sunlit clearing with foliage framing the project.",
    title: "Let the project lead the way.", lede: "Start in the repository you know. Vela gives the files, the agent, and every change a place to meet.",
    scene: "canopy", form: "grounded", type: "serif", section: "The source stays in sight.",
    sky: "#e6e6c6", horizon: "#a6bd91", foreground: "#233e32", glow: "#f7dc8b", ink: "#20382c", accent: "#436f51", light: true
  },
  {
    id: "04-estuary", family: "04", name: "Estuary", motif: "layered water", note: "An open tidal landscape and a workbench on the horizon.",
    title: "Everything flows from the repo.", lede: "The folder you opened anchors the editor, terminal, agent, and the files that change along the way.",
    scene: "estuary", form: "floating", type: "sans", section: "Follow the current back to code.",
    sky: "#d4e2da", horizon: "#85aaa6", foreground: "#174a52", glow: "#f2e2b2", ink: "#183f44", accent: "#267a79", light: true
  },
  {
    id: "04-terrace", family: "04", name: "Terrace", motif: "terraced earth", note: "Contour-cut terrain in ochre, moss, and cream.",
    title: "A better view of the work.", lede: "Open a project and keep your attention on its code. The rest of your workspace takes its shape from there.",
    scene: "terrace", form: "inset", type: "serif", section: "One folder. Every next step.",
    sky: "#f2e4cb", horizon: "#bb9675", foreground: "#544e39", glow: "#ffe5a5", ink: "#473f31", accent: "#8c6643", light: true
  },
  {
    id: "04-topography", family: "04", name: "Topography", motif: "contour lines", note: "A cartographic field with the repository as its center.",
    title: "Know where the work lives.", lede: "From the first file to the final diff, keep the whole development loop mapped to your project.",
    scene: "topography", form: "diagram", type: "mono", section: "A clear route through the project.",
    sky: "#dce6d8", horizon: "#9db9a4", foreground: "#386354", glow: "#f4f2d4", ink: "#29483b", accent: "#397758", light: true
  },
  {
    id: "04-glasshouse", family: "04", name: "Glasshouse", motif: "architectural glass", note: "A luminous greenhouse of panes and soft shadows.",
    title: "Give your project room to grow.", lede: "The workbench grows around the repo in front of you: edit, ask, run, and inspect without losing its roots.",
    scene: "glasshouse", form: "framed", type: "sans", section: "A workspace rooted in the source.",
    sky: "#e0eadc", horizon: "#9bc6ad", foreground: "#285641", glow: "#f8efba", ink: "#224433", accent: "#3a8061", light: true
  },
  {
    id: "04-nocturne", family: "04", name: "Nocturne", motif: "night mountains", note: "An ink-dark valley with a quiet halo of light.",
    title: "The repo is your north star.", lede: "Keep every question and every change tethered to the project. Work calmly, even when the code gets complex.",
    scene: "nocturne", form: "floating", type: "serif", section: "Context, even after dark.",
    sky: "#131e30", horizon: "#34515a", foreground: "#101e27", glow: "#c7d5ae", ink: "#f4f4e8", accent: "#b7d6ba", light: false
  },
  {
    id: "04-sandstone", family: "04", name: "Sandstone", motif: "sculpted canyon", note: "Warm carved strata with a clear desktop horizon.",
    title: "Find the shape of the project.", lede: "Read the code that is already there. Let the agent help, then see the exact file changes in context.",
    scene: "sandstone", form: "grounded", type: "serif", section: "A working path through the layers.",
    sky: "#f1d7b8", horizon: "#c38165", foreground: "#633c3c", glow: "#ffedc5", ink: "#503634", accent: "#a35745", light: true
  },
  {
    id: "04-mist", family: "04", name: "Mist", motif: "fog and treeline", note: "Pale atmospheric layers let the workspace emerge.",
    title: "Clarity starts at the source.", lede: "Find the file, follow the thought, review the change. The project remains visible through every handoff.",
    scene: "mist", form: "inset", type: "sans", section: "Bring the next step into focus.",
    sky: "#eff2e8", horizon: "#b8c6bc", foreground: "#4a655c", glow: "#ffffff", ink: "#263c38", accent: "#50736a", light: true
  },
  {
    id: "04-bloom", family: "04", name: "Bloom", motif: "abstract wildflowers", note: "A graphic meadow of oversized forms and soft grain.",
    title: "Build from what is already here.", lede: "Bring your own repository. Vela keeps its files and the agent's work close enough to understand together.",
    scene: "bloom", form: "framed", type: "serif", section: "The project is the starting point.",
    sky: "#f3e6df", horizon: "#d9b6b3", foreground: "#6d6852", glow: "#fff1bd", ink: "#493d42", accent: "#8d5f62", light: true
  },
  {
    id: "04-velaline", family: "04", name: "Velaline", motif: "celestial navigation", note: "A celestial map with code as the fixed center.",
    title: "Keep the source in your vela.", lede: "Files, agent sessions, commands, and changes stay connected to the same repository, from start to review.",
    scene: "velaline", form: "diagram", type: "mono", section: "Everything returns to the repo.",
    sky: "#171d33", horizon: "#424977", foreground: "#16182c", glow: "#edbd9b", ink: "#f4eee8", accent: "#dab6a7", light: false
  },
  {
    id: "10-parchment", family: "10", name: "Parchment", motif: "warm paper", note: "A calm desktop desk laid over tactile paper and print.",
    title: "A good day starts here.", lede: "Your editor, repository, terminal, and agent finally have a desk to share.",
    scene: "parchment", form: "inset", type: "serif", section: "Made for the everyday loop.",
    sky: "#f6efe0", horizon: "#d7c6ad", foreground: "#706a5a", glow: "#fff9d5", ink: "#38382e", accent: "#6a7456", light: true
  },
  {
    id: "10-grid", family: "10", name: "Grid", motif: "graph paper", note: "Precision lines and an app that sits on its own canvas.",
    title: "The whole workflow, one surface.", lede: "Open the project, ask your agent, run commands, and inspect the results in a single working space.",
    scene: "grid", form: "diagram", type: "mono", section: "A practical place for every tool.",
    sky: "#edf1ee", horizon: "#becfc6", foreground: "#344f4a", glow: "#f7f8e6", ink: "#273b39", accent: "#397b6a", light: true
  },
  {
    id: "10-lagoon", family: "10", name: "Lagoon", motif: "aquatic light", note: "A bright pool of color behind the everyday desk.",
    title: "Settle into your flow.", lede: "Keep the code in front, the agent nearby, and the change review only a step away.",
    scene: "lagoon", form: "floating", type: "sans", section: "Everything within reach.",
    sky: "#d5f2e6", horizon: "#75c5b8", foreground: "#145f64", glow: "#fff7c6", ink: "#164a4d", accent: "#157a78", light: true
  },
  {
    id: "10-copper", family: "10", name: "Copper", motif: "metal and ember", note: "Brushed warmth, deep plum, and a lit-up workbench.",
    title: "The desk is yours.", lede: "Bring the repo you have and the tools you use. Stay close to the work from the first edit to the final diff.",
    scene: "copper", form: "framed", type: "serif", section: "A familiar working rhythm.",
    sky: "#442e3d", horizon: "#a76261", foreground: "#271d2e", glow: "#f2ba8c", ink: "#fff3e8", accent: "#f3c6ac", light: false
  },
  {
    id: "10-atlas", family: "10", name: "Atlas", motif: "folded map", note: "Paper folds lead from project to editor to agent.",
    title: "Everything has its place.", lede: "One desktop home for the code in front of you, the commands you run, and the agent helping you along.",
    scene: "atlas", form: "grounded", type: "sans", section: "Your whole route on one desk.",
    sky: "#e8ece4", horizon: "#b4c4ae", foreground: "#536756", glow: "#f6e7bb", ink: "#34423a", accent: "#5c7556", light: true
  },
  {
    id: "10-prism", family: "10", name: "Prism", motif: "refracted color", note: "Translucent color panes wrap a focused desktop app.",
    title: "One space. Many ways to work.", lede: "Move between editing, asking, running, and reviewing without scattering your attention.",
    scene: "prism", form: "framed", type: "sans", section: "The tools work better together.",
    sky: "#e7e8f2", horizon: "#b6b9d5", foreground: "#55577c", glow: "#f6cbd4", ink: "#34364e", accent: "#6c6599", light: true
  },
  {
    id: "10-dusk", family: "10", name: "Dusk", motif: "after-hours skyline", note: "A quiet evening skyline behind an active workspace.",
    title: "Make room for the work.", lede: "A calm desktop environment keeps your project, tools, and agent together through every session.",
    scene: "dusk", form: "floating", type: "serif", section: "A better place to stay awhile.",
    sky: "#20283f", horizon: "#5b5b78", foreground: "#191c34", glow: "#eea878", ink: "#f5f2f1", accent: "#eebda8", light: false
  },
  {
    id: "10-studio", family: "10", name: "Studio", motif: "architectural arches", note: "A softly lit studio with a workbench at its center.",
    title: "Come in and get to work.", lede: "Open your repository and find the editor, agent, and terminal already waiting in the same room.",
    scene: "studio", form: "inset", type: "serif", section: "A desk built around your project.",
    sky: "#f4e7dd", horizon: "#d7ab9d", foreground: "#876c68", glow: "#fff0cf", ink: "#473b3a", accent: "#996759", light: true
  },
  {
    id: "10-midnight", family: "10", name: "Midnight", motif: "constellation grid", note: "Quiet technical stars, crisp lines, and an illuminated app.",
    title: "Keep your tools in vela.", lede: "When the work gets complex, keep the project, editor, terminal, and agent in the same field of view.",
    scene: "midnight", form: "diagram", type: "mono", section: "Stay oriented in the work.",
    sky: "#111a2b", horizon: "#303e57", foreground: "#0b1522", glow: "#77c5c4", ink: "#eaf6f4", accent: "#95d4c8", light: false
  },
  {
    id: "10-daybreak", family: "10", name: "Daybreak", motif: "soft morning hills", note: "Quiet gradients and a wide-open, welcoming desk.",
    title: "Start here. Keep going.", lede: "One desktop workspace for the day ahead: your code, your agent, and the changes you want to review.",
    scene: "daybreak", form: "grounded", type: "sans", section: "The day's work, in one place.",
    sky: "#f8efd7", horizon: "#dbbf99", foreground: "#64766b", glow: "#fff0a7", ink: "#3e4a3d", accent: "#657d59", light: true
  }
];
