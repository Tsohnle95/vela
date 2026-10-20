import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'

const root = path.resolve(import.meta.dirname, '..')
const upstream = path.join(root, 'src/renderer/src/ai-elements/upstream')
const patterns = ['chain-of-thought', 'chat', 'checkpoint', 'confirmation', 'context', 'file-tree', 'inline-citation', 'message', 'model-selector', 'plan', 'prompt-input', 'queue', 'reasoning', 'sources', 'stack-trace', 'subagent', 'suggestion', 'task', 'terminal', 'test-results', 'tool']
const primitives = ['attachment', 'badge', 'bubble', 'button', 'code-block', 'collapsible', 'dropdown-menu', 'empty', 'hover-card', 'input-group', 'item', 'marker', 'message', 'message-scroller', 'progress', 'sheet', 'tooltip', 'trace']
const cache = process.argv[2]
const sources = []

async function source(url, cacheName) {
  const text = cache
    ? await readFile(path.join(cache, cacheName), 'utf8')
    : await fetch(url).then(async (response) => {
        if (!response.ok) throw new Error(`${response.status}: ${url}`)
        return response.text()
      })
  sources.push({ url, sha256: createHash('sha256').update(text).digest('hex') })
  return text
}

async function save(file, content) {
  await mkdir(path.dirname(file), { recursive: true })
  await writeFile(file, content)
}

await save(path.join(upstream, 'catalog.md'), await source('https://ui.shadcn.com/docs/ai-elements.md', 'vela-ai-elements.md'))
await save(path.join(upstream, 'LICENSE.md'), await source('https://raw.githubusercontent.com/shadcn-ui/ui/main/LICENSE.md', 'vela-ai-license.md'))
await save(path.join(upstream, 'shimmer.md'), await source('https://ui.shadcn.com/docs/utils/shimmer.md', 'vela-ai-shimmer.md'))
await save(path.join(upstream, 'registry/style.json'), await source('https://ui.shadcn.com/r/styles/base-rhea/style.json', 'vela-ai-style.json'))
await save(path.join(upstream, 'styles/globals.css.txt'), await source('https://raw.githubusercontent.com/shadcn-ui/ui/main/apps/v4/app/globals.css', 'vela-shadcn-globals.css'))
await save(path.join(upstream, 'styles/fonts.ts.txt'), await source('https://raw.githubusercontent.com/shadcn-ui/ui/main/apps/v4/lib/fonts.ts', 'vela-shadcn-fonts.ts'))
await save(path.join(upstream, 'helpers/markdown.tsx.txt'), await source('https://raw.githubusercontent.com/shadcn-ui/ui/main/apps/v4/components/markdown.tsx', 'vela-markdown.tsx'))
await save(path.join(upstream, 'helpers/message-animations.ts.txt'), await source('https://raw.githubusercontent.com/shadcn-ui/ui/main/apps/v4/lib/message-animations.ts', 'vela-message-animations.ts'))
for (const name of patterns) {
  const text = await source(`https://ui.shadcn.com/docs/ai-elements/base/${name}.md`, `vela-ai-${name}.md`)
  await save(path.join(upstream, 'patterns', `${name}.md`), text)
}

const visited = new Set()
async function registry(name) {
  if (visited.has(name)) return
  visited.add(name)
  const text = await source(`https://ui.shadcn.com/r/styles/base-rhea/${name}.json`, `vela-ai-ui-${name}.json`)
  const entry = JSON.parse(text)
  await save(path.join(upstream, 'registry', `${name}.json`), text)
  for (const file of entry.files) {
    let content = file.content
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
    const relative = file.path.replace('registry/base-rhea/', '')
    const destination = relative.startsWith('ui/')
      ? path.join(root, 'src/renderer/src/components', relative)
      : path.join(upstream, relative)
    await save(destination, content)
  }
  for (const dependency of entry.registryDependencies ?? []) await registry(dependency)
}
for (const name of primitives) await registry(name)
await save(path.join(upstream, 'manifest.json'), JSON.stringify({
  upstream: 'https://ui.shadcn.com/docs/ai-elements',
  license: 'MIT',
  style: 'base-rhea',
  transforms: ['Resolve registry imports to local files', 'Resolve cn to local clsx/tailwind-merge helper', 'Select Lucide icons from IconPlaceholder, as the shadcn CLI does', 'Scope portal theme inheritance to the AI Elements surface'],
  sources,
}, null, 2) + '\n')
