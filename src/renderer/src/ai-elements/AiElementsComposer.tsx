"use client"

import * as React from "react"
import {
  ArrowUpIcon,
  ChevronDownIcon,
  FileTextIcon,
  PlusIcon,
  XIcon,
  SquareIcon,
} from "lucide-react"

import {
  Attachment,
  AttachmentAction,
  AttachmentActions,
  AttachmentContent,
  AttachmentMedia,
  AttachmentTitle,
} from "@/components/ui/attachment"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupTextarea,
} from "@/components/ui/input-group"

import { QueuedMessageChips } from "../components/QueuedMessageChips"
import { droppedFilePaths, isExternalFileDrag } from "../drop"
import { Button } from "@/components/ui/button"

import { composerKeyDown, useAiElementsComposer, type AiElementsComposerProps } from "./use-ai-elements-composer"

const suggestions = ["Explain this repository", "Find a bug and add a regression test", "Review the recent changes"]
type ComposerState = ReturnType<typeof useAiElementsComposer>

export function AiElementsComposer(props: AiElementsComposerProps) {
  const state = useAiElementsComposer(props)
  const { hasConversation } = props
  const { supportsAttachments, view, formRef, inputRef, textRef, text, setText, updateCompletion, addAttachmentPaths, send, onSubmit, onAttach } = state
  return (
    <div className="mx-auto w-full max-w-lg py-6" onDragOver={(event) => {
      if (supportsAttachments && isExternalFileDrag(event)) { event.preventDefault(); event.dataTransfer.dropEffect = "copy" }
    }} onDrop={(event) => {
      if (supportsAttachments && isExternalFileDrag(event)) { event.preventDefault(); addAttachmentPaths(droppedFilePaths(event)) }
    }}>
      {!(hasConversation ?? view.transcript.length > 0) && <div className="flex scroll-fade-x scrollbar-none gap-2 overflow-x-auto overscroll-x-contain">
        {suggestions.map((suggestion) => <Button key={suggestion} variant="outline" size="sm" className="shrink-0" onClick={() => send(suggestion)}>{suggestion}</Button>)}
      </div>}
      <form ref={formRef} onSubmit={onSubmit}>
        <input
          ref={inputRef}
          type="file"
          multiple
          className="sr-only"
          tabIndex={-1}
          onChange={onAttach}
        />
        <InputGroup>
          <ComposerAttachments state={state} />
          <InputGroupTextarea
            ref={textRef}
            aria-label="Message"
            value={text}
            onChange={(event) => { setText(event.target.value); updateCompletion(event.target.value, event.target.selectionStart) }}
            placeholder="Ask the agent to change your code..."
            onKeyDown={(event) => composerKeyDown(state, event)}
            className="p-3"
          />
          <ComposerToolbar state={state} />
        </InputGroup>
      </form>
      <ComposerFeedback state={state} />
    </div>
  )
}


function ComposerAttachments({ state }: { state: ComposerState }) {
  const { files, setFiles } = state
  return (<>
          {files.length > 0 && (
            <InputGroupAddon align="block-start" className="flex-wrap">
              {files.map((file) => (
                <Attachment key={file.id} size="sm">
                  <AttachmentMedia>
                    <FileTextIcon />
                  </AttachmentMedia>
                  <AttachmentContent>
                    <AttachmentTitle>{file.name}</AttachmentTitle>
                  </AttachmentContent>
                  <AttachmentActions>
                    <AttachmentAction
                      aria-label={`Remove ${file.name}`}
                      onClick={() =>
                        setFiles((current) =>
                          current.filter((item) => item.id !== file.id)
                        )
                      }
                    >
                      <XIcon />
                    </AttachmentAction>
                  </AttachmentActions>
                </Attachment>
              ))}
            </InputGroupAddon>
          )}

  </>)
}

function ComposerModelPicker({ state }: { state: ComposerState }) {
  const { models, supportsModels, store, workspace, currentModel, demo, model, setModel, providers } = state
  return (<>
            <DropdownMenu onOpenChange={(open) => { if (open && models.length === 0 && supportsModels) void store.loadModels(workspace ?? undefined) }}>
              <DropdownMenuTrigger
                render={
                  <InputGroupButton
                    size="sm"
                    variant="outline"
                    disabled={!supportsModels}
                    aria-label={`Model: ${currentModel?.name ?? "Unavailable"}`}
                    className="ml-auto"
                  />
                }
              >
                {demo ? "Replay" : currentModel?.name ?? "Model"}
                <ChevronDownIcon />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-48">
                <DropdownMenuRadioGroup value={model} onValueChange={setModel}>
                  {providers.map((provider, index) => (
                    <React.Fragment key={provider.name}>
                      {index > 0 && <DropdownMenuSeparator />}
                      <DropdownMenuGroup>
                        <DropdownMenuLabel>{provider.name}</DropdownMenuLabel>
                        {provider.models.map((item) => (
                          <DropdownMenuRadioItem
                            key={`${item.providerID}::${item.id}`}
                            value={`${item.providerID}::${item.id}`}
                            closeOnClick
                          >
                            {item.name}
                          </DropdownMenuRadioItem>
                        ))}
                      </DropdownMenuGroup>
                    </React.Fragment>
                  ))}
                </DropdownMenuRadioGroup>
              </DropdownMenuContent>
            </DropdownMenu>

  </>)
}

function ComposerEffortPicker({ state }: { state: ComposerState }) {
  const { supportsModels, currentModel, currentEffort, effort, setEffort, efforts } = state
  return (<>
            {supportsModels && (currentModel?.variants?.length ?? 0) > 0 && <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <InputGroupButton
                    size="sm"
                    variant="outline"
                    aria-label={`Effort: ${currentEffort.name}`}
                  />
                }
              >
                <span className="text-muted-foreground">Effort</span>
                {currentEffort.name}
                <ChevronDownIcon />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-36">
                <DropdownMenuGroup>
                  <DropdownMenuLabel>Effort</DropdownMenuLabel>
                  <DropdownMenuRadioGroup
                    value={effort}
                    onValueChange={setEffort}
                  >
                    {efforts.map((item) => (
                      <DropdownMenuRadioItem
                        key={item.id}
                        value={item.id}
                        closeOnClick
                      >
                        {item.name}
                      </DropdownMenuRadioItem>
                    ))}
                  </DropdownMenuRadioGroup>
                </DropdownMenuGroup>
              </DropdownMenuContent>
            </DropdownMenu>}

  </>)
}

function ComposerToolbar({ state }: { state: ComposerState }) {
  const { supportsAttachments, inputRef, demo, view, store, workspace, canSend } = state
  return (<>
          <InputGroupAddon align="block-end">
            <InputGroupButton
              size="icon-sm"
              variant="outline"
              className="rounded-full"
              aria-label="Attach files"
              disabled={!supportsAttachments}
              onClick={() => inputRef.current?.click()}
            >
              <PlusIcon />
            </InputGroupButton>
            <ComposerModelPicker state={state} />
            <ComposerEffortPicker state={state} />
            {!demo && view.busy && <InputGroupButton type="button" size="icon-sm" variant="outline" className="rounded-full" aria-label="Stop" onClick={() => void store.stop(workspace ?? undefined)}><SquareIcon /></InputGroupButton>}
            <InputGroupButton
              type="submit"
              size="icon-sm"
              variant="default"
              className="rounded-full"
              disabled={!canSend}
              aria-label={!demo && view.busy ? "Queue message" : "Send"}
            >
              <ArrowUpIcon />
            </InputGroupButton>
          </InputGroupAddon>
  </>)
}

function ComposerFeedback({ state }: { state: ComposerState }) {
  const { notice, completion, applyCompletion, demo, workspace, setText, setFiles } = state
  return (<>
      {notice && <p role="alert" className="text-sm text-destructive">{notice}</p>}
      {completion && completion.items.length > 0 && <div role="listbox" aria-label={completion.kind === "command" ? "Commands" : "Mention files"} className="flex flex-col gap-1">
        {completion.items.map((item, index) => <Button key={item.label} role="option" aria-selected={index === completion.selected} variant={index === completion.selected ? "secondary" : "ghost"} size="sm" onMouseDown={(event) => { event.preventDefault(); applyCompletion(index) }}>{item.label}</Button>)}
      </div>}
      {!demo && workspace && <QueuedMessageChips workspace={workspace} onEditMessage={(content, attachments) => {
        setText(content)
        setFiles(attachments.map(({ path }) => ({ id: path, path, name: path.split(/[\\/]/).pop() ?? path })))
      }} />}
  </>)
}
