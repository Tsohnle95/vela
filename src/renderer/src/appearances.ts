export type AppearanceId =
  | "codex"
  | "codex-dark"
  | "quiet-habitat"
  | "control-deck"
  | "papertrail"
  | "shell-first"
  | "swiss-grid"
  | "prism"
  | "workshop"
  | "blueprint"
  | "stillness"
  | "bloom"
  | "kitty"
  | "observatory"
  | "original";

export interface AppearanceColors {
  base: string;
  top: string;
  pane: string;
  editor: string;
  editorOpaque: string;
  agent: string;
  rail: string;
  tab: string;
  console: string;
  status: string;
  ink: string;
  muted: string;
  faint: string;
  line: string;
  lineStrong: string;
  lineSoft: string;
  selected: string;
  user: string;
  card: string;
  compose: string;
  composeShadow: string;
  accent: string;
  good: string;
  danger: string;
  icon: string;
  sendInk: string;
  glow: string;
  syntaxKeyword: string;
  syntaxFunction: string;
  syntaxString: string;
  syntaxComment: string;
  syntaxNumber: string;
  syntaxVariable?: string;
  syntaxAttribute?: string;
  // Optional Pierre-style token slots; the Codex profiles carry the reference
  // app's exact Pierre Light/Dark colors and Monaco falls back per token for
  // the other appearances.
  syntaxType?: string;
  syntaxConstant?: string;
  syntaxOperator?: string;
  syntaxPunctuation?: string;
  syntaxTag?: string;
  syntaxDecorator?: string;
  syntaxNamespace?: string;
  syntaxRegexp?: string;
}

export interface AppearanceProfile {
  id: AppearanceId;
  name: string;
  description: string;
  scheme: "dark" | "light";
  colors: AppearanceColors;
}

export const APPEARANCES: AppearanceProfile[] = [
  {
    id: "codex",
    name: "Daybreak",
    description: "Quiet white surfaces, hairline dividers and a blue signal.",
    scheme: "light",
    colors: {
      base: "#ffffff", top: "#f6f6f6", pane: "#fcfcfc", editor: "#ffffff", editorOpaque: "#ffffff", agent: "#ffffff", rail: "#f6f6f6", tab: "#f3f3f3", console: "#f8f8f8", status: "#ffffff",
      ink: "#0d0d0d", muted: "#5d5d5d", faint: "#8f8f8f", line: "rgba(0,0,0,.10)", lineStrong: "rgba(0,0,0,.15)", lineSoft: "rgba(0,0,0,.05)", selected: "#e8e8e8", user: "#f3f3f3", card: "#ffffff", compose: "#ffffff", composeShadow: "0 7px 22px rgba(0,0,0,.10), 0 1px 2px rgba(0,0,0,.05)",
      accent: "#3a83f7", good: "#3a843f", danger: "#d92d20", icon: "#8f8f8f", sendInk: "#ffffff", glow: "rgba(58,131,247,.05)", syntaxKeyword: "#d32a61", syntaxFunction: "#693acf", syntaxString: "#199f43", syntaxComment: "#737373", syntaxNumber: "#1ca1c7", syntaxVariable: "#d47628", syntaxAttribute: "#18a46c", syntaxType: "#a631be", syntaxConstant: "#d5a910", syntaxOperator: "#08c0ef", syntaxPunctuation: "#636363", syntaxTag: "#d5512f", syntaxDecorator: "#1a85d4", syntaxNamespace: "#d5901c", syntaxRegexp: "#17a5af"
    }
  },
  {
    id: "codex-dark",
    name: "Midnight",
    description: "Graphite surfaces with a night-blue signal.",
    scheme: "dark",
    colors: {
      base: "#181818", top: "#242424", pane: "#1d1d1d", editor: "#181818", editorOpaque: "#181818", agent: "#181818", rail: "#242424", tab: "#303030", console: "#1c1c1c", status: "#212121",
      ink: "#ffffff", muted: "#cdcdcd", faint: "#afafaf", line: "rgba(255,255,255,.15)", lineStrong: "rgba(255,255,255,.20)", lineSoft: "rgba(255,255,255,.05)", selected: "#303030", user: "#303030", card: "#2a2a2a", compose: "#363636", composeShadow: "0 7px 22px rgba(0,0,0,.35)",
      accent: "#63a8f8", good: "#83d197", danger: "#ff7b72", icon: "#afafaf", sendInk: "#0d0d0d", glow: "rgba(99,168,248,.06)", syntaxKeyword: "#ff678d", syntaxFunction: "#9d6afb", syntaxString: "#5ecc71", syntaxComment: "#737373", syntaxNumber: "#68cdf2", syntaxVariable: "#ffa359", syntaxAttribute: "#60d199", syntaxType: "#d568ea", syntaxConstant: "#ffd452", syntaxOperator: "#08c0ef", syntaxPunctuation: "#636363", syntaxTag: "#ff855e", syntaxDecorator: "#69b1ff", syntaxNamespace: "#ffab16", syntaxRegexp: "#64d1db"
    }
  },
  {
    id: "quiet-habitat",
    name: "Quiet Habitat",
    description: "Soft sage surfaces with a calm green signal.",
    scheme: "light",
    colors: {
      base: "#e9efe8", top: "#f2f6f0", pane: "#f0f4ee", editor: "#f7f9f5", editorOpaque: "#f7f9f5", agent: "#f0f4ef", rail: "#e4ebe3", tab: "#f0f4ee", console: "#eef3ed", status: "#dfe8df",
      ink: "#26332a", muted: "#66766a", faint: "#8a998d", line: "#dce5dc", lineStrong: "#bdcdbd", lineSoft: "rgba(87,126,92,.08)", selected: "#e1ece0", user: "#e1ebe1", card: "#f4f7f2", compose: "#fafcf8", composeShadow: "0 7px 18px rgba(44,74,49,.08)",
      accent: "#557d5b", good: "#537f59", danger: "#b65853", icon: "#718a73", sendInk: "#f7faf6", glow: "rgba(87,126,92,.16)", syntaxKeyword: "#8a5d92", syntaxFunction: "#316b72", syntaxString: "#99713e", syntaxComment: "#91a08e", syntaxNumber: "#b76b55"
    }
  },
  {
    id: "control-deck",
    name: "Control Deck",
    description: "Dense graphite panels with a warm instrument accent.",
    scheme: "dark",
    colors: {
      base: "#161b21", top: "#1e252d", pane: "#202830", editor: "#171d23", editorOpaque: "#171d23", agent: "#1d252d", rail: "#171d23", tab: "#1b2229", console: "#12171c", status: "#202830",
      ink: "#e8edf1", muted: "#a0acb5", faint: "#687681", line: "#303b45", lineStrong: "#53636e", lineSoft: "rgba(238,153,79,.075)", selected: "#263943", user: "#2a3740", card: "#222d35", compose: "#192129", composeShadow: "0 8px 24px rgba(0,0,0,.28)",
      accent: "#f0a15f", good: "#86c79c", danger: "#f07a70", icon: "#86a0b4", sendInk: "#21160e", glow: "rgba(240,161,95,.15)", syntaxKeyword: "#e6a6cd", syntaxFunction: "#80c8d5", syntaxString: "#dfc47c", syntaxComment: "#72847b", syntaxNumber: "#e89668"
    }
  },
  {
    id: "papertrail",
    name: "Papertrail",
    description: "A warm notebook palette with a restrained clay accent.",
    scheme: "light",
    colors: {
      base: "#e8dfce", top: "#f0e8d9", pane: "#eee6d8", editor: "#faf5e9", editorOpaque: "#faf5e9", agent: "#f2ebdd", rail: "#e6ddce", tab: "#f6f0e5", console: "#eee6d8", status: "#dfd5c4",
      ink: "#30291f", muted: "#696050", faint: "#9a8c77", line: "#d9cfbd", lineStrong: "#b7a88e", lineSoft: "rgba(174,82,57,.075)", selected: "#e8dec9", user: "#e8dfcf", card: "#f7f0e3", compose: "#fbf6eb", composeShadow: "0 5px 14px rgba(79,58,34,.08)",
      accent: "#a34d3b", good: "#667b50", danger: "#ad514a", icon: "#79735d", sendInk: "#fff9ed", glow: "rgba(163,77,59,.12)", syntaxKeyword: "#974f5f", syntaxFunction: "#486e75", syntaxString: "#aa6d35", syntaxComment: "#9b907e", syntaxNumber: "#8257a1"
    }
  },
  {
    id: "shell-first",
    name: "Shell First",
    description: "Deep forest panels with a bright terminal green.",
    scheme: "dark",
    colors: {
      base: "#090e0c", top: "#111815", pane: "#0f1713", editor: "#131d17", editorOpaque: "#131d17", agent: "#0d1511", rail: "#0b110e", tab: "#101913", console: "#080d0a", status: "#111a14",
      ink: "#ddf3df", muted: "#9db7a2", faint: "#597563", line: "#1d3627", lineStrong: "#326243", lineSoft: "rgba(135,227,142,.09)", selected: "#12251a", user: "#15281c", card: "#101d15", compose: "#0d1911", composeShadow: "0 0 24px rgba(82,217,108,.07)",
      accent: "#8bea88", good: "#a4f2a3", danger: "#f27f79", icon: "#70b682", sendInk: "#071109", glow: "rgba(139,234,136,.12)", syntaxKeyword: "#d0a0ec", syntaxFunction: "#91e1d2", syntaxString: "#f0d27e", syntaxComment: "#568468", syntaxNumber: "#fa9672"
    }
  },
  {
    id: "swiss-grid",
    name: "Swiss Grid",
    description: "Clear, precise paper with one vermilion signal.",
    scheme: "light",
    colors: {
      base: "#f2f1ed", top: "#f8f7f3", pane: "#f8f7f3", editor: "#fbfaf7", editorOpaque: "#fbfaf7", agent: "#f5f4ef", rail: "#ecebe6", tab: "#f2f1ed", console: "#e9e8e2", status: "#191b1b",
      ink: "#181b1a", muted: "#535957", faint: "#8e9591", line: "#d5d9d5", lineStrong: "#8a9690", lineSoft: "rgba(223,67,47,.07)", selected: "#f0e3de", user: "#ede8e3", card: "#fbfaf7", compose: "#fbfaf7", composeShadow: "none",
      accent: "#df472f", good: "#6f8e48", danger: "#b53632", icon: "#4c5e54", sendInk: "#ffffff", glow: "rgba(223,71,47,.08)", syntaxKeyword: "#bd3e53", syntaxFunction: "#315f73", syntaxString: "#886229", syntaxComment: "#8d948c", syntaxNumber: "#925094"
    }
  },
  {
    id: "prism",
    name: "Prism",
    description: "Atmospheric violet glass with soft, luminous edges.",
    scheme: "dark",
    colors: {
      base: "#151423", top: "#1d1b30", pane: "rgba(31,29,51,.95)", editor: "rgba(24,23,42,.97)", editorOpaque: "#18172a", agent: "rgba(30,27,51,.97)", rail: "#171628", tab: "#201e35", console: "rgba(15,15,28,.98)", status: "#1b1930",
      ink: "#f1efff", muted: "#b7b1d1", faint: "#797595", line: "#373451", lineStrong: "#625c91", lineSoft: "rgba(135,119,255,.10)", selected: "#302b52", user: "#343052", card: "#25223e", compose: "#201d38", composeShadow: "0 8px 27px rgba(0,0,0,.28),0 0 20px rgba(134,112,255,.1)",
      accent: "#b6a3ff", good: "#7fe3c2", danger: "#ff7c94", icon: "#a599eb", sendInk: "#201a3b", glow: "#302456", syntaxKeyword: "#f09ed0", syntaxFunction: "#7be3df", syntaxString: "#f3d28d", syntaxComment: "#7975a1", syntaxNumber: "#a8d987"
    }
  },
  {
    id: "workshop",
    name: "Workshop",
    description: "Tactile graphite with safety orange and olive details.",
    scheme: "dark",
    colors: {
      base: "#252a2a", top: "#343b3a", pane: "#2e3534", editor: "#202626", editorOpaque: "#202626", agent: "#303736", rail: "#252b2b", tab: "#303736", console: "#171c1c", status: "#d87c3c",
      ink: "#ece8dc", muted: "#b7b8a8", faint: "#858b7e", line: "#454e4b", lineStrong: "#737e72", lineSoft: "rgba(245,142,71,.09)", selected: "#3b4137", user: "#3b4039", card: "#292f2d", compose: "#272e2c", composeShadow: "inset 0 1px rgba(255,255,255,.035)",
      accent: "#f29a58", good: "#b4cc73", danger: "#ef8173", icon: "#c0b27d", sendInk: "#211811", glow: "rgba(242,154,88,.12)", syntaxKeyword: "#eaa69f", syntaxFunction: "#9dd5c0", syntaxString: "#efd284", syntaxComment: "#818c79", syntaxNumber: "#d5a9ed"
    }
  },
  {
    id: "blueprint",
    name: "Blueprint",
    description: "Technical navy surfaces traced with electric cyan.",
    scheme: "dark",
    colors: {
      base: "#0c1a27", top: "#122437", pane: "#102235", editor: "#0d2031", editorOpaque: "#0d2031", agent: "#102237", rail: "#0a1826", tab: "#11263a", console: "#091724", status: "#12314a",
      ink: "#e2f0f8", muted: "#abc2d2", faint: "#64849c", line: "#25445d", lineStrong: "#39718f", lineSoft: "rgba(78,205,226,.08)", selected: "#123147", user: "#15344a", card: "#10283b", compose: "#0c2133", composeShadow: "0 0 16px rgba(63,191,218,.08)",
      accent: "#65d0dd", good: "#9bd69a", danger: "#ff7b82", icon: "#6cb3cb", sendInk: "#08232d", glow: "rgba(101,208,221,.12)", syntaxKeyword: "#e29cde", syntaxFunction: "#78d7db", syntaxString: "#eecb82", syntaxComment: "#608ba1", syntaxNumber: "#8fbbfa"
    }
  },
  {
    id: "stillness",
    name: "Stillness",
    description: "Near-white canvas, quiet chrome and a single green thread.",
    scheme: "light",
    colors: {
      base: "#f4f5f2", top: "#fafbf9", pane: "#f7f8f5", editor: "#ffffff", editorOpaque: "#ffffff", agent: "#f8f9f6", rail: "#f4f5f2", tab: "#fbfcfa", console: "#f6f7f4", status: "#f0f2ee",
      ink: "#222824", muted: "#68736c", faint: "#9ba49d", line: "#e7eae5", lineStrong: "#cbd4cb", lineSoft: "rgba(71,133,83,.06)", selected: "#edf3ec", user: "#eef2ed", card: "#fafbf9", compose: "#ffffff", composeShadow: "0 6px 22px rgba(34,53,38,.06)",
      accent: "#597b61", good: "#527d57", danger: "#b4564f", icon: "#829488", sendInk: "#ffffff", glow: "rgba(89,123,97,.08)", syntaxKeyword: "#8f638d", syntaxFunction: "#3d7882", syntaxString: "#a17948", syntaxComment: "#a1aaa1", syntaxNumber: "#be7158"
    }
  },
  {
    id: "bloom",
    name: "Bloom",
    description: "Playful petal tones with soft, modular color accents.",
    scheme: "light",
    colors: {
      base: "#ececf4", top: "#f8f7fc", pane: "#f5f4fb", editor: "#fffefa", editorOpaque: "#fffefa", agent: "#f2effa", rail: "#e9e8f1", tab: "#f5f4fb", console: "#202339", status: "#deddec",
      ink: "#29293c", muted: "#6d6d84", faint: "#9998ae", line: "#dcdbe8", lineStrong: "#c1bfd6", lineSoft: "rgba(221,92,115,.07)", selected: "#eee7fa", user: "#e8e7f5", card: "#faf9fd", compose: "#ffffff", composeShadow: "0 7px 18px rgba(69,56,104,.09)",
      accent: "#c65370", good: "#579a7d", danger: "#ba465a", icon: "#6f7ca9", sendInk: "#ffffff", glow: "rgba(198,83,112,.12)", syntaxKeyword: "#ae568b", syntaxFunction: "#4c8194", syntaxString: "#a57737", syntaxComment: "#9a9aab", syntaxNumber: "#6c70bb"
    }
  },
  {
    id: "kitty",
    name: "Kitty Glass",
    description: "The original translucent dark glass with a cyan signal.",
    scheme: "dark",
    colors: {
      base: "#0e121c", top: "#0e121c00", pane: "rgba(14,18,28,.24)", editor: "rgba(2,2,4,.08)", editorOpaque: "#02020400", agent: "rgba(14,18,28,.44)", rail: "rgba(14,18,28,.24)", tab: "rgba(2,2,4,.16)", console: "rgba(2,2,4,0)", status: "rgba(14,18,28,.24)",
      ink: "#e7e7ee", muted: "#c0c3d0", faint: "#a0a4b4", line: "rgba(231,231,238,.08)", lineStrong: "rgba(231,231,238,.15)", lineSoft: "rgba(231,231,238,.05)", selected: "#343a55", user: "rgba(52,58,85,.52)", card: "rgba(24,31,48,.42)", compose: "rgba(14,18,28,.34)", composeShadow: "0 7px 24px rgba(0,0,0,.38)",
      accent: "#00a2ce", good: "#5bd69a", danger: "#ff4b67", icon: "#6fc3df", sendInk: "#020204", glow: "rgba(0,162,206,.10)", syntaxKeyword: "#ff8b85", syntaxFunction: "#e7e7ee", syntaxString: "#5bd69a", syntaxComment: "#7f8292", syntaxNumber: "#e0a85a"
    }
  },
  {
    id: "original",
    name: "Original Dark",
    description: "Vela's original warm charcoal and sage palette.",
    scheme: "dark",
    colors: {
      base: "#171412", top: "#171412", pane: "#262220", editor: "#262220", editorOpaque: "#262220", agent: "rgba(38,34,32,.92)", rail: "#171412", tab: "#201d1b", console: "#121317", status: "#171412",
      ink: "#e8e3dd", muted: "#a8a29e", faint: "#8f8880", line: "rgba(255,255,255,.05)", lineStrong: "rgba(255,255,255,.10)", lineSoft: "rgba(255,255,255,.04)", selected: "#37322e", user: "#37322e", card: "#2d2926", compose: "#262220", composeShadow: "0 4px 16px rgba(0,0,0,.28)",
      accent: "#9eb4a1", good: "#a9cbad", danger: "#e2988a", icon: "#8fbcd9", sendInk: "#172019", glow: "rgba(158,180,161,.06)", syntaxKeyword: "#e8875f", syntaxFunction: "#ead9c8", syntaxString: "#a8c69a", syntaxComment: "#78716c", syntaxNumber: "#e5b567"
    }
  },
  {
    id: "observatory",
    name: "Observatory",
    description: "Midnight violet, neon syntax and a pixel-art observatory beneath the stars.",
    scheme: "dark",
    colors: {
      base: "#050611", top: "#070814", pane: "#080a16", editor: "#060912", editorOpaque: "#060912", agent: "#050611", rail: "#090b1b", tab: "#090a1c", console: "#050611", status: "#070814",
      ink: "#e0dcff", muted: "#b5a9ef", faint: "#8279ad", line: "#29205c", lineStrong: "#554099", lineSoft: "#171333", selected: "#161040", user: "#171331", card: "#111025", compose: "linear-gradient(115deg, #101020ed, #25223eef)", composeShadow: "0 8px 26px #00000066, inset 0 1px #cec0ff12",
      accent: "#aa83ff", good: "#00dfb0", danger: "#ff5478", icon: "#b6a0ff", sendInk: "#160d38", glow: "#24104d", syntaxKeyword: "#f143ef", syntaxFunction: "#45e6e1", syntaxString: "#26dfcb", syntaxComment: "#71688e", syntaxNumber: "#26dfcb", syntaxVariable: "#45e6e1", syntaxAttribute: "#45e6e1", syntaxOperator: "#c9adff", syntaxPunctuation: "#ffd658", syntaxTag: "#f143ef", syntaxType: "#c9adff"
    }
  }
];

export function appearanceForId(id: AppearanceId): AppearanceProfile {
  return APPEARANCES.find((appearance) => appearance.id === id) ?? APPEARANCES.find((appearance) => appearance.id === "prism")!;
}

export function terminalThemeForAppearance(id: AppearanceId) {
  if (id === "kitty") return {
    background: "rgba(2, 2, 4, 0)", foreground: "#f4f4fa", cursor: "#00a2ce", cursorAccent: "#020204", selectionBackground: "#2e4d78",
    black: "#020204", red: "#ff4b67", green: "#5bd69a", yellow: "#e0a85a", blue: "#00a2ce", magenta: "#c99ff2", cyan: "#6fc3df", white: "#f4f4fa",
    brightBlack: "#a6a9b8", brightRed: "#ff8b85", brightGreen: "#82e8b4", brightYellow: "#f0c780", brightBlue: "#25b8dd", brightMagenta: "#dcb8ff", brightCyan: "#8fd8ef", brightWhite: "#ffffff"
  };
  if (id === "original") return {
    background: "#121317", foreground: "#e8eaef", cursor: "#d97757", cursorAccent: "#0d0e11", selectionBackground: "#2e4d78",
    black: "#17181d", red: "#f16d6b", green: "#4cc38a", yellow: "#e0af68", blue: "#d97757", magenta: "#c99ff2", cyan: "#6fc3df", white: "#e8eaef",
    brightBlack: "#626b78", brightRed: "#ff8b85", brightGreen: "#6fd8a8", brightYellow: "#eec27f", brightBlue: "#e68a68", brightMagenta: "#dcb8ff", brightCyan: "#8fd8ef", brightWhite: "#ffffff"
  };
  const { colors, scheme } = appearanceForId(id);
  const black = scheme === "light" ? colors.ink : colors.base;
  return {
    background: colors.console,
    foreground: colors.ink,
    cursor: colors.accent,
    cursorAccent: colors.base,
    selectionBackground: colors.selected,
    black,
    red: colors.danger,
    green: colors.good,
    yellow: colors.syntaxString,
    blue: colors.syntaxFunction,
    magenta: colors.syntaxKeyword,
    cyan: colors.icon,
    white: colors.ink,
    brightBlack: colors.faint,
    brightRed: colors.danger,
    brightGreen: colors.good,
    brightYellow: colors.syntaxString,
    brightBlue: colors.accent,
    brightMagenta: colors.syntaxKeyword,
    brightCyan: colors.icon,
    brightWhite: colors.ink
  };
}

export function agentTerminalThemeForAppearance(id: AppearanceId) {
  const { colors } = appearanceForId(id);
  return {
    ...terminalThemeForAppearance(id),
    background: id === "kitty" ? "rgba(2, 2, 4, 0)" : colors.agent,
    foreground: colors.ink,
    cursor: colors.accent,
    cursorAccent: colors.base,
    selectionBackground: colors.selected,
    cyan: colors.accent,
    brightCyan: colors.accent
  };
}
