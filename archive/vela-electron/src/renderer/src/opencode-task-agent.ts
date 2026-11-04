// OpenCode 03e67171, MIT; session-ui/message-part.tsx task agent colors.
const agentTones: Record<string, string> = {
  ask: "var(--icon-agent-ask-base)",
  build: "var(--icon-agent-build-base)",
  docs: "var(--icon-agent-docs-base)",
  plan: "var(--icon-agent-plan-base)",
}

const v2AgentTones: Record<string, string> = {
  build: "var(--v2-agent-build-solid)",
  explore: "var(--v2-agent-explore-solid)",
  plan: "var(--v2-agent-plan-solid)",
  review: "var(--v2-agent-review-solid)",
  writer: "var(--v2-agent-writer-solid)",
}

const agentThemeColors: Record<string, string> = {
  primary: "var(--text-interactive-base)",
  secondary: "var(--text-base)",
  accent: "var(--icon-info-base)",
  success: "var(--icon-success-base)",
  warning: "var(--icon-warning-base)",
  error: "var(--icon-critical-base)",
  info: "var(--icon-info-base)",
}

const v2AgentThemeColors: Record<string, string> = {
  primary: "var(--v2-text-text-accent)",
  secondary: "var(--v2-text-text-muted)",
  accent: "var(--v2-icon-icon-accent)",
  success: "var(--v2-state-fg-success)",
  warning: "var(--v2-state-fg-warning)",
  error: "var(--v2-state-fg-danger)",
  info: "var(--v2-state-fg-info)",
}

const agentPalette = [
  "var(--icon-agent-ask-base)",
  "var(--icon-agent-build-base)",
  "var(--icon-agent-docs-base)",
  "var(--icon-agent-plan-base)",
  "var(--syntax-info)",
  "var(--syntax-success)",
  "var(--syntax-warning)",
  "var(--syntax-property)",
  "var(--syntax-constant)",
  "var(--text-diff-add-base)",
  "var(--text-diff-delete-base)",
  "var(--icon-warning-base)",
]

function tone(name: string) {
  let hash = 0
  for (const char of name) hash = (hash * 31 + char.charCodeAt(0)) >>> 0
  return agentPalette[hash % agentPalette.length]
}

export function resolveOpenCodeTaskAgent(
  raw: unknown,
  list?: readonly { name: string; color?: string }[],
): { name?: string; color?: string; v2Color?: string } {
  if (typeof raw !== "string" || !raw) return {}
  const key = raw.toLowerCase()
  const item = list?.find((entry) => entry.name === raw || entry.name.toLowerCase() === key)
  const v2Tone = item?.color ? undefined : v2AgentTones[key]
  const color = agentColor(item?.color, agentThemeColors) ?? agentTones[key] ?? tone(key)
  const v2Color = agentColor(item?.color, v2AgentThemeColors) ?? v2Tone ?? color
  return {
    name: item?.name ?? `${raw[0]!.toUpperCase()}${raw.slice(1)}`,
    color,
    v2Color,
  }
}

function agentColor(value: string | undefined, themeColors: Record<string, string>) {
  if (!value) return
  return themeColors[value] ?? value
}
