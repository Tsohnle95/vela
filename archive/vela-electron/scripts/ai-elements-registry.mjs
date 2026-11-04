export function installAiElementRegistrySource(name, source) {
  let content = source
  const icons = new Set()
  content = content.replace(/<IconPlaceholder\s+([\s\S]*?)\/>/g, (_, props) => {
    const icon = props.match(/lucide="([^"]+)"/)?.[1]
    if (!icon) throw new Error(`Missing Lucide icon in ${name}`)
    icons.add(icon)
    const extra = props.replace(/(?:lucide|tabler|hugeicons|phosphor|remixicon)="[^"]+"\s*/g, '').trim()
    return `<${icon}${extra ? ` ${extra}` : ''} />`
  })
  content = content.replace(/import \{ IconPlaceholder \} from "@\/app\/\(create\)\/components\/icon-placeholder"\n/g, '')
  if (icons.size) {
    const iconImport = `import { ${[...icons].join(', ')} } from "lucide-react"\n`
    content = content.startsWith('"use client"\n')
      ? content.replace('"use client"\n', `"use client"\n\n${iconImport}`)
      : iconImport + content
  }
  content = content.replaceAll('from "cn"', 'from "@/ai-elements/class-names"')
    .replaceAll('@/registry/base-rhea/ui/', '@/components/ui/')
    .replaceAll('@/registry/base-rhea/hooks/', '@/ai-elements/upstream/hooks/')
    .replaceAll('@/registry/base-rhea/lib/', '@/ai-elements/upstream/lib/')
  if (['dropdown-menu', 'hover-card', 'tooltip'].includes(name)) {
    content = content.replace('className="isolate z-50', 'className="ai-elements-portal isolate z-50')
  }
  if (name === 'sheet') {
    content = content.replace(/(<SheetPrimitive.Popup[\s\S]*?className=\{cn\()/, '$1\n          "ai-elements-portal",')
  }

  const portals = {
    tooltip: ["TooltipPrimitive", "TooltipContent"],
    "hover-card": ["PreviewCardPrimitive", "HoverCardContent"],
    "dropdown-menu": ["MenuPrimitive", "DropdownMenuContent"],
    sheet: ["SheetPrimitive", "SheetPortal"],
  }
  if (portals[name]) {
    const [primitive, component] = portals[name]
    content = content.replace('import { cn }', 'import { useAiElementsPortalContainer } from "@/ai-elements/portal-container"\nimport { cn }')
    if (name === "sheet" || name === "dropdown-menu") {
      content = content.replace(`  return <${primitive}.Portal data-slot=`, `  const container = useAiElementsPortalContainer()\n  return <${primitive}.Portal container={container} data-slot=`)
    }
    if (name !== "sheet") {
      const position = content.indexOf("  return (", content.indexOf("function " + component + "("))
      content = content.slice(0, position) + "  const container = useAiElementsPortalContainer()\n" + content.slice(position)
      content = content.replace(`<${primitive}.Portal>`, `<${primitive}.Portal container={container}>`)
        .replace(`<${primitive}.Portal data-slot="hover-card-portal">`, `<${primitive}.Portal container={container} data-slot="hover-card-portal">`)
    }
  }
  return content
}
