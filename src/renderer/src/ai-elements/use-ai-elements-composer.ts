import * as React from "react"
import type { SessionInfo } from "@shared/types"
import { usePanel, useStore } from "../store"
import { buildPromptFiles, useComposerCompletions } from "../components/composer-completions"
import { formatVariant, useComposerModels } from "../components/ComposerModelMenu"

type CompletionResult = ReturnType<typeof useComposerCompletions>

export interface AiElementsComposerProps {
  session: SessionInfo | null
  onDemoPrompt?: (text: string) => void
  hasConversation?: boolean
}
function useComposerState({ session, onDemoPrompt }: AiElementsComposerProps) {
  const store = useStore()
  const workspace = session?.workspace ?? null
  const view = usePanel(workspace)
  const demo = Boolean(onDemoPrompt)
  const { supportsAttachments, supportsCommands, supportsAgents, supportsModels } = composerCapabilities(store, session, demo)
  const models = session ? view.models ?? [] : store.availableModels ?? []
  const currentModel = session ? view.currentModel : store.lastModel ?? null
  const [text, setText] = React.useState("")
  const [files, setFiles] = React.useState<{ id: string; name: string; path: string }[]>([])
  const [mentions, setMentions] = React.useState<{ rel: string; path: string }[]>([])
  const [notice, setNotice] = React.useState("")
  const formRef = React.useRef<HTMLFormElement>(null)
  const inputRef = React.useRef<HTMLInputElement>(null)
  const textRef = React.useRef<HTMLTextAreaElement>(null)
  const modelState = useComposerModels(models, currentModel, store.switchModel, workspace, () => {})
  const providers = modelState.filteredGroups.map(([name, models]) => ({ name, models }))
  const model = currentModel ? `${currentModel.providerID}::${currentModel.id}` : ""
  const efforts = [ { id: "", name: "Auto" }, ...(currentModel?.variants ?? []).map((id) => ({ id, name: formatVariant(id) })) ]
  const effort = currentModel?.variant ?? ""
  const currentEffort = efforts.find((item) => item.id === effort) ?? efforts[0]
  const canSend = text.trim().length > 0 || files.length > 0
  const { completion, setCompletion, applyCompletion, updateCompletion } = useComposerCompletions({
    workspace: demo ? null : workspace, input: text, mentions, inputRef: textRef,
    setInput: setText, setMentions, runCommand: store.runCommand, setNotice,
    supportsCommands, supportsAttachments,
  })

  React.useEffect(() => {
    setFiles([])
    setNotice("")
  }, [workspace?.id, workspace?.generation])

  return { store, workspace, view, demo, supportsAttachments, supportsCommands, supportsAgents, supportsModels, models, currentModel, text, setText, files, setFiles, mentions, setMentions, notice, setNotice, formRef, inputRef, textRef, modelState, providers, model, efforts, effort, currentEffort, canSend, completion: completion as CompletionResult["completion"], setCompletion: setCompletion as CompletionResult["setCompletion"], applyCompletion, updateCompletion }
}

export function useAiElementsComposer(props: AiElementsComposerProps) {
  const state = useComposerState(props)
  const { store, workspace, supportsAttachments, supportsCommands, supportsModels, models, text, setText, files, setFiles, mentions, setMentions, setNotice, formRef, textRef, modelState, canSend, setCompletion } = state
  const { onDemoPrompt } = props
  function setModel(key: string) {
    if (!supportsModels) return
    const chosen = models.find((item) => `${item.providerID}::${item.id}` === key)
    if (chosen) modelState.chooseModel(chosen)
  }
  function setEffort(value: string) { if (supportsModels) modelState.chooseVariant(value || undefined) }
  function addAttachmentPaths(paths: string[]) {
    if (!supportsAttachments) return
    setFiles((current) => {
      const next = [...current]
      for (const path of paths) if (!next.some((file) => file.path === path)) next.push({ id: path, path, name: path.split(/[\\/]/).pop() ?? path })
      return next
    })
  }
  function send(value: string) {
    const prompt = value.trim()
    if (!prompt && !files.length) return
    setNotice("")
    if (onDemoPrompt) onDemoPrompt(prompt)
    else {
      const command = /^\/([^/\s]\S*)(?:\s+([\s\S]*))?$/.exec(prompt)
      const result = command && supportsCommands
        ? store.runCommand(command[1], command[2] ?? "", workspace ?? undefined)
        : store.sendPrompt(prompt, [...buildPromptFiles(prompt, mentions), ...files.map(({ path }) => ({ path }))], workspace ?? undefined)
      void result.catch((error) => setNotice(error instanceof Error ? error.message : String(error)))
    }
    setText("")
    setFiles([])
    setMentions([])
    setCompletion(null)
    textRef.current?.focus()
  }
  function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (canSend) send(text)
  }
  function onAttach(event: React.ChangeEvent<HTMLInputElement>) {
    const selected = Array.from(event.target.files ?? [])
    addAttachmentPaths(selected.map((file) => window.vela.getPathForFile(file)).filter(Boolean))
    event.target.value = ""
  }
  return { ...state, setModel, setEffort, addAttachmentPaths, send, onSubmit, onAttach }
}

type KeyboardState = ReturnType<typeof useAiElementsComposer>
export function composerKeyDown(state: KeyboardState, event: React.KeyboardEvent<HTMLTextAreaElement>) {
  const { formRef } = state
    if (event.nativeEvent.isComposing) return
    if (handleCompletionKey(state, event)) return
    if (handleModelShortcut(state, event)) return
    if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); formRef.current?.requestSubmit() }
  }

function handleCompletionKey(state: KeyboardState, event: React.KeyboardEvent<HTMLTextAreaElement>) {
  const { completion, setCompletion, applyCompletion } = state
    if (completion?.items.length) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault()
        const direction = event.key === "ArrowDown" ? 1 : -1
        setCompletion((current) => current ? { ...current, selected: (current.selected + direction + current.items.length) % current.items.length } : current)
        return true
      }
      if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); applyCompletion(); return true }
      if (event.key === "Escape") { event.preventDefault(); setCompletion(null); return true }
    }
  return false
}

function handleModelShortcut(state: KeyboardState, event: React.KeyboardEvent<HTMLTextAreaElement>) {
  if (event.metaKey || event.altKey) return false
  const { supportsModels, supportsAgents, modelState, view, store, workspace } = state
  const shiftOnly = event.shiftKey && !event.ctrlKey
  if (supportsModels && event.key === "Tab" && event.ctrlKey && !event.shiftKey) {
    event.preventDefault(); modelState.cycleStrength(); return true
  }
  if (event.key === "Tab" && shiftOnly && supportsAgents && view.agents.length > 0) {
    event.preventDefault()
    const index = view.agents.findIndex((agent) => agent.id === view.currentAgent?.id)
    void store.switchAgent(view.agents[(index + 1) % view.agents.length].id, workspace ?? undefined)
    return true
  }
  if (supportsModels && event.key.toLowerCase() === "p" && shiftOnly) {
    event.preventDefault(); modelState.cycleFavorite(); return true
  }
  return false
}

function composerCapabilities(store: ReturnType<typeof useStore>, session: SessionInfo | null, demo: boolean) {
  const runtime = (store.runtimes ?? []).find((item) => item.id === (session?.runtimeID ?? "opencode"))
  const supportsAttachments = !demo && (runtime?.capabilities.attachments ?? session?.runtimeID !== "deepseek")
  const supportsCommands = !demo && (runtime?.capabilities.commands ?? session?.runtimeID !== "deepseek")
  const supportsAgents = !demo && (runtime?.capabilities.agents ?? session?.runtimeID !== "deepseek")
  const supportsModels = !demo && (runtime?.capabilities.models ?? session?.runtimeID !== "deepseek")
  return { supportsAttachments, supportsCommands, supportsAgents, supportsModels }
}
