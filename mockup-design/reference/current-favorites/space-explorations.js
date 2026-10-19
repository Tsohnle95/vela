const velaSpaceExplorations = [
  {
    id: "04-nocturne-eclipse", family: "04", lineage: "nocturne", name: "Eclipse", motif: "eclipsed moon / alpine night", note: "A luminous corona above layered midnight peaks.",
    title: "Find your way through the code.", lede: "The repository stays your point of reference while the agent follows the work into the dark.", section: "Every path has a source.", message: "The project remains your north star.",
    scene: "nocturne-eclipse", form: "inset", type: "serif", sky: "#0e1529", horizon: "#384b65", foreground: "#101b2b", glow: "#a5cae4", ink: "#f3f4ef", accent: "#bbd5dc", light: false
  },
  {
    id: "04-nocturne-constellation", family: "04", lineage: "nocturne", name: "Constellation", motif: "star map / quiet ridgeline", note: "Fine celestial lines connect a field of stars over the valley.",
    title: "Connect every part of the work.", lede: "Files, questions, and changes form one clear constellation around your repository.", section: "The connections are the point.", message: "Keep the whole project in view.",
    scene: "nocturne-constellation", form: "diagram", type: "mono", sky: "#101b32", horizon: "#38546a", foreground: "#101d2b", glow: "#80b8bd", ink: "#eef5ed", accent: "#addad0", light: false
  },
  {
    id: "04-nocturne-pulsar", family: "04", lineage: "nocturne", name: "Pulsar", motif: "light pulse / distant mountains", note: "A single distant signal cuts across a still night landscape.",
    title: "Stay close to the signal.", lede: "Vela keeps the repo in focus while the agent follows a question from file to answer.", section: "Less noise. More context.", message: "Follow the signal back to source.",
    scene: "nocturne-pulsar", form: "floating", type: "sans", sky: "#161528", horizon: "#43385f", foreground: "#141b30", glow: "#d8a9c4", ink: "#faf2f4", accent: "#edc3d0", light: false
  },
  {
    id: "04-nocturne-aurora", family: "04", lineage: "nocturne", name: "Polar Night", motif: "aurora / winter horizon", note: "A soft auroral ribbon drifts over a deep green horizon.",
    title: "Work with a wider horizon.", lede: "Open the project, keep its files in reach, and move through the session with confidence.", section: "Context from start to finish.", message: "A quiet horizon for the deep work.",
    scene: "nocturne-aurora", form: "framed", type: "serif", sky: "#0d2430", horizon: "#3e6d67", foreground: "#0f2c2c", glow: "#a3d8b1", ink: "#f0f7ec", accent: "#a9e1c4", light: false
  },
  {
    id: "04-nocturne-tidal", family: "04", lineage: "nocturne", name: "Tidal Moon", motif: "moonrise / dark water", note: "Moonlit reflection and still water ground the workspace.",
    title: "Return to what matters.", lede: "Whatever the agent changes, the path back to the project stays visible.", section: "The source is always nearby.", message: "The code stays above the tide.",
    scene: "nocturne-tidal", form: "grounded", type: "serif", sky: "#152137", horizon: "#456273", foreground: "#10232c", glow: "#d3d5c3", ink: "#f5f3eb", accent: "#bfd8cf", light: false
  },
  {
    id: "04-ridge-saturn", family: "04", lineage: "ridge", name: "Ringrise", motif: "ringed world / green ridge", note: "A ringed planet rises behind quiet, layered terrain.",
    title: "Start on solid ground.", lede: "Open the repo beneath your feet. Let everything else fall into place around it.", section: "One landscape. One project.", message: "The next horizon starts in your code.",
    scene: "ridge-saturn", form: "grounded", type: "serif", sky: "#203a41", horizon: "#6c8a73", foreground: "#1d3c32", glow: "#e1d6aa", ink: "#f8f4e9", accent: "#cce2b5", light: false
  },
  {
    id: "04-ridge-observatory", family: "04", lineage: "ridge", name: "Observatory", motif: "signal dome / sunset peaks", note: "A small observatory beneath a vast celestial sky.",
    title: "See the whole project clearly.", lede: "Keep the repository in view while you explore what the agent is working on.", section: "A vantage point for the work.", message: "Keep sight of where it all begins.",
    scene: "ridge-observatory", form: "inset", type: "sans", sky: "#e9d7c7", horizon: "#ac9d9d", foreground: "#3d4e50", glow: "#f9d9a6", ink: "#353f42", accent: "#496a6a", light: true
  },
  {
    id: "04-ridge-comet", family: "04", lineage: "ridge", name: "Comet Trail", motif: "comet / copper horizon", note: "A warm, fast-moving trail above grounded stone layers.",
    title: "Keep momentum in context.", lede: "Move quickly with an agent without losing track of the files and changes behind the work.", section: "The project holds the thread.", message: "Move forward without drifting away.",
    scene: "ridge-comet", form: "floating", type: "sans", sky: "#432f43", horizon: "#b17b73", foreground: "#463f49", glow: "#f4b785", ink: "#fff2e8", accent: "#ffd6b7", light: false
  },
  {
    id: "04-ridge-binary", family: "04", lineage: "ridge", name: "Twin Suns", motif: "binary suns / amber valley", note: "Two lights pass over an amber and sage mountain basin.",
    title: "Make space for the work.", lede: "One repository gives the editor and agent a common place to begin.", section: "A shared starting point.", message: "Two lights. One clear direction.",
    scene: "ridge-binary", form: "framed", type: "serif", sky: "#f1dfc0", horizon: "#cba975", foreground: "#57624f", glow: "#ffe2a0", ink: "#3d4b3c", accent: "#5b6f51", light: true
  },
  {
    id: "04-ridge-halo", family: "04", lineage: "ridge", name: "Halo Ridge", motif: "celestial halo / evergreen peaks", note: "A bright celestial arc curves around a mountain summit.",
    title: "Keep your bearings.", lede: "Every question can lead somewhere new. Vela keeps the source of the work close.", section: "Stay anchored in the project.", message: "Everything comes back around.",
    scene: "ridge-halo", form: "diagram", type: "sans", sky: "#dce5dc", horizon: "#9bb8a0", foreground: "#305148", glow: "#e9f3d0", ink: "#28453d", accent: "#487a61", light: true
  },
  {
    id: "04-mist-lunar", family: "04", lineage: "mist", name: "Lunar Fog", motif: "moon / fog bank", note: "A pale moon hangs above successive veils of mist.",
    title: "The next step comes into view.", lede: "Let the project reveal itself one file at a time, with the agent close by.", section: "Clearer with every step.", message: "A little light goes a long way.",
    scene: "mist-lunar", form: "inset", type: "serif", sky: "#e9edeb", horizon: "#bdcfd0", foreground: "#536f73", glow: "#f9fcf2", ink: "#2c4548", accent: "#4e7476", light: true
  },
  {
    id: "04-mist-transit", family: "04", lineage: "mist", name: "Transit", motif: "celestial transit / silver clouds", note: "A dark passing sphere gives the bright haze a center.",
    title: "Focus follows the project.", lede: "From the code in front of you to the diff at the end, each step stays connected.", section: "One clear point of focus.", message: "Move through the work together.",
    scene: "mist-transit", form: "framed", type: "sans", sky: "#eff0e8", horizon: "#bec7c6", foreground: "#5d7471", glow: "#fffde8", ink: "#304845", accent: "#567d72", light: true
  },
  {
    id: "04-mist-starfield", family: "04", lineage: "mist", name: "Starfield", motif: "faint stars / morning fog", note: "A barely-there constellation through translucent cloud.",
    title: "See the connections form.", lede: "Vela keeps files, questions, and changes visible while the work takes shape.", section: "A place for everything to connect.", message: "Even small signals make a pattern.",
    scene: "mist-starfield", form: "diagram", type: "mono", sky: "#e6eeec", horizon: "#b4cdca", foreground: "#4c7770", glow: "#f6f5e0", ink: "#294e49", accent: "#417b70", light: true
  },
  {
    id: "04-mist-planetfall", family: "04", lineage: "mist", name: "Planetfall", motif: "distant world / silver ridge", note: "A distant planet emerges through a silver-blue horizon.",
    title: "Land in the work you know.", lede: "Open a familiar repository and make the next change without losing your place.", section: "Your project is the landing point.", message: "A familiar place to touch down.",
    scene: "mist-planetfall", form: "floating", type: "serif", sky: "#dce5e9", horizon: "#9cb6c4", foreground: "#476278", glow: "#eef5ec", ink: "#2c4554", accent: "#527587", light: true
  },
  {
    id: "04-mist-afterglow", family: "04", lineage: "mist", name: "Afterglow", motif: "soft aurora / luminous fog", note: "A gentle aurora is almost hidden in morning mist.",
    title: "Keep the work in focus.", lede: "The editor, agent, and changed files stay within sight of the repository you opened.", section: "A calmer path to clarity.", message: "Follow the glow back to the source.",
    scene: "mist-afterglow", form: "grounded", type: "sans", sky: "#e7ebe5", horizon: "#b5cbbd", foreground: "#4a6862", glow: "#f6f1d9", ink: "#2e4840", accent: "#4f7e6c", light: true
  },
  {
    id: "10-dusk-vela", family: "10", lineage: "dusk", name: "Night Vela", motif: "planetary ring / city lights", note: "A celestial ring rises over a quiet working skyline.",
    title: "A home for every Vela.", lede: "Your tools revolve around the project, not the other way around.", section: "Everything in its place.", message: "A desk for the whole journey.",
    scene: "dusk-vela", form: "grounded", type: "serif", sky: "#151d35", horizon: "#4c5878", foreground: "#111a2c", glow: "#f0a88d", ink: "#f6f1ef", accent: "#edbca9", light: false
  },
  {
    id: "10-dusk-nebula", family: "10", lineage: "dusk", name: "Nebula Desk", motif: "nebula / city silhouette", note: "Colorful atmospheric light glows over a late-night desk.",
    title: "Stay in your element.", lede: "One workspace holds the code, the session, and the work still ahead.", section: "Your tools stay in reach.", message: "A little more room for the work.",
    scene: "dusk-nebula", form: "framed", type: "sans", sky: "#232040", horizon: "#76536e", foreground: "#1e2238", glow: "#df9cbd", ink: "#f8f0f4", accent: "#e4bdd3", light: false
  },
  {
    id: "10-dusk-satellite", family: "10", lineage: "dusk", name: "Satellite", motif: "satellite path / rooftops", note: "A small tracked signal moves above a still city.",
    title: "Keep the work connected.", lede: "Editor, terminal, and agent stay on the same wavelength as your project.", section: "One connected desk.", message: "Follow the thread across the sky.",
    scene: "dusk-satellite", form: "diagram", type: "mono", sky: "#111e33", horizon: "#36506a", foreground: "#111d2e", glow: "#8cc6c4", ink: "#eef6f4", accent: "#a4d9d2", light: false
  },
  {
    id: "10-dusk-crescent", family: "10", lineage: "dusk", name: "Crescent City", motif: "crescent / skyline", note: "A slim crescent and layered skyline make the desk feel alive.",
    title: "Work into the evening.", lede: "The project and its tools remain together as one session turns into the next.", section: "A good place to stay awhile.", message: "The next step is still within reach.",
    scene: "dusk-crescent", form: "inset", type: "serif", sky: "#282643", horizon: "#675a7d", foreground: "#1b2037", glow: "#ddbbad", ink: "#f5f1ec", accent: "#e4c9c2", light: false
  },
  {
    id: "10-dusk-horizon", family: "10", lineage: "dusk", name: "Event Horizon", motif: "solar horizon / night city", note: "A copper horizon glows behind a silhouetted workday.",
    title: "The day’s work, one place.", lede: "Keep the workbench calm as the code, agent, and changes move together.", section: "A steady rhythm for building.", message: "Stay for the whole arc.",
    scene: "dusk-horizon", form: "floating", type: "sans", sky: "#2b233b", horizon: "#a26a70", foreground: "#261e32", glow: "#f5b788", ink: "#fff2eb", accent: "#f3c6a9", light: false
  }
];

velaExplorations.push(...velaSpaceExplorations);
