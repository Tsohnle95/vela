import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { usePanel, useStore } from "../store";
import type { PromptFile, SessionInfo } from "@shared/types";
import { droppedFilePaths, isExternalFileDrag } from "../drop";
import { IconArrowUp, IconAdd, IconCheck, IconChevronDown, IconFile, IconGitBranch, IconImage, IconStop, IconTerminal } from "./icons";
import { QueuedMessageChips } from "./QueuedMessageChips";
import { buildPromptFiles, useComposerCompletions } from "./composer-completions";
import { ComposerModelMenu, formatVariant, useComposerModels } from "./ComposerModelMenu";
import { useMenuDismiss } from "./use-menu-dismiss";

type MenuKind = "model" | "agent" | null;

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif", "ico", "tiff", "tif"]);

function isImagePath(path: string): boolean {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  if (dot < 0) return false;
  return IMAGE_EXTENSIONS.has(name.slice(dot + 1).toLowerCase());
}

export function Composer({ session, controls, overlay }: { session?: SessionInfo | null; controls?: ReactNode; overlay?: ReactNode }): ReactNode {
  const store = useStore();
  const {
    switchModel,
    switchAgent,
    loadAgents,
    loadModels,
    sendPrompt,
    runCommand,
    stop
  } = store;
  const activeSession = session === undefined ? store.session : session;
  const runtime = (store.runtimes ?? []).find((item) => item.id === (activeSession?.runtimeID ?? "opencode"));
  const supportsAttachments = runtime?.capabilities.attachments ?? activeSession?.runtimeID !== "deepseek";
  const supportsCommands = runtime?.capabilities.commands ?? activeSession?.runtimeID !== "deepseek";
  const supportsAgents = runtime?.capabilities.agents ?? activeSession?.runtimeID !== "deepseek";
  const workspace = activeSession?.workspace ?? null;
  const view = usePanel(workspace);
  const { agents, currentAgent, busy, assistantStatus } = view;
  const models = activeSession ? view.models : store.availableModels ?? [];
  const currentModel = activeSession ? view.currentModel : store.lastModel ?? null;
  const [input, setInput] = useState("");
  const [files, setFiles] = useState<{ path: string; name: string }[]>([]);
  const [menu, setMenu] = useState<MenuKind>(null);
  const [notice, setNotice] = useState("");
  const modelState = useComposerModels(models, currentModel, switchModel, workspace, () => setMenu(null));
  const { modelView, setModelView, cycleFavorite, cycleStrength, reloadPreferences } = modelState;
  const [mentions, setMentions] = useState<{ rel: string; path: string }[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const attachmentInputRef = useRef<HTMLInputElement>(null);
  const composerRef = useRef<HTMLDivElement>(null);
  const canSend = input.trim().length > 0 || files.length > 0;
  const variantLabel = currentModel?.variant
    ? formatVariant(currentModel.variant)
    : (currentModel?.variants?.length ?? 0) > 0 ? "Auto" : "";

  useMenuDismiss(Boolean(menu), composerRef, useCallback(() => setMenu(null), []));

  const { completion, setCompletion, applyCompletion, updateCompletion } = useComposerCompletions({
    workspace, input, mentions, inputRef, setInput, setMentions, runCommand, setNotice, supportsCommands, supportsAttachments
  });
  useEffect(() => {
    setFiles([]);
    setPreviews({});
  }, [activeSession?.workspace.id, activeSession?.workspace.generation]);

  const send = (): void => {
    if (!canSend) return;
    const text = input.trim();
    const command = /^\/([^/\s]\S*)(?:\s+([\s\S]*))?$/.exec(text);
    const promptFiles: PromptFile[] = command && supportsCommands ? [] : [
      ...buildPromptFiles(text, mentions),
      ...files.map((file) => ({ path: file.path }))
    ];
    setInput("");
    setFiles([]);
    setPreviews({});
    setMentions([]);
    setCompletion(null);
    const el = inputRef.current;
    if (el) el.style.removeProperty("--composer-input-height");
    if (command && supportsCommands) {
      void runCommand(command[1], command[2] ?? "", workspace ?? undefined).catch((err) =>
        setNotice(err instanceof Error ? err.message : String(err))
      );
    } else {
      void sendPrompt(text, promptFiles, workspace ?? undefined);
    }
    el?.focus();
  };

  const loadPreview = async (path: string): Promise<void> => {
    if (!isImagePath(path)) return;
    try {
      const dataUrl = await window.vela.readImagePreview(path);
      if (!dataUrl) return;
      setPreviews((current) => ({ ...current, [path]: dataUrl }));
    } catch {
      return;
    }
  };

  const addAttachmentPaths = (paths: string[]): void => {
    if (!supportsAttachments) return;
    if (paths.length === 0) return;
    setFiles((current) => {
      const next = [...current];
      for (const filePath of paths) {
        if (!next.some((file) => file.path === filePath)) {
          next.push({ path: filePath, name: filePath.split(/[\\/]/).pop() ?? filePath });
        }
      }
      return next;
    });
    for (const filePath of paths) void loadPreview(filePath);
    inputRef.current?.focus();
  };

  const onPanelDragOver = (e: React.DragEvent): void => {
    if (!supportsAttachments || !isExternalFileDrag(e)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setDragOver(true);
  };

  const onPanelDragLeave = (e: React.DragEvent): void => {
    const dropTarget = composerRef.current?.closest(".agent-panel") ?? composerRef.current;
    if (dropTarget?.contains(e.relatedTarget as Node)) return;
    setDragOver(false);
  };

  const onPanelDrop = (e: React.DragEvent): void => {
    if (!supportsAttachments || !isExternalFileDrag(e)) return;
    e.preventDefault();
    setDragOver(false);
    addAttachmentPaths(droppedFilePaths(e));
  };

  useEffect(() => {
    const panel = composerRef.current?.closest(".agent-panel") ?? composerRef.current;
    if (!panel) return;
    const dragOver = (event: Event): void => onPanelDragOver(event as unknown as React.DragEvent);
    const dragLeave = (event: Event): void => onPanelDragLeave(event as unknown as React.DragEvent);
    const drop = (event: Event): void => onPanelDrop(event as unknown as React.DragEvent);
    panel.addEventListener("dragover", dragOver);
    panel.addEventListener("dragleave", dragLeave);
    panel.addEventListener("drop", drop);
    return () => {
      panel.removeEventListener("dragover", dragOver);
      panel.removeEventListener("dragleave", dragLeave);
      panel.removeEventListener("drop", drop);
    };
  }, [supportsAttachments]);

  const cycleAgent = (): void => {
    if (agents.length === 0) return;
    const index = agents.findIndex((agent) => agent.id === currentAgent?.id);
    void switchAgent(agents[(index + 1) % agents.length].id, workspace ?? undefined);
  };

  return (
    <div
      className="composer"
      ref={composerRef}
    >
      {files.length > 0 && (
        <div className="composer-attachments">
          {files.map((file) => (
            <span className="composer-attachment" key={file.path}>
              {previews[file.path] ? (
                <img className="composer-attachment-thumb" src={previews[file.path]} alt="" />
              ) : (
                <IconFile />
              )}
              <span>{file.name}</span>
              <button
                className="composer-attachment-remove"
                title={`Remove ${file.name}`}
                onClick={() => setFiles((current) => current.filter((item) => item.path !== file.path))}
              >
                ×
              </button>
            </span>
          ))}
        </div>
      )}
      <div className={`composer-body ${dragOver ? "drag-over" : ""}`}>
        {dragOver && (
          <div className="composer-drop-hint">
            <IconImage />
            Drop images to attach
          </div>
        )}
        <textarea
          ref={inputRef}
          className="composer-input"
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="off"
          rows={1}
          placeholder="Ask about this repository or describe a change…"
          value={input}
          onChange={(e) => {
            const value = e.target.value;
            setInput(value);
            e.target.style.setProperty("--composer-input-height", "0px");
            e.target.style.setProperty("--composer-input-height", `${e.target.scrollHeight}px`);
            const caret = e.target.selectionStart ?? value.length;
            updateCompletion(value, caret);
          }}
          onKeyDown={(e) => {
            if (e.key === "Tab" && e.ctrlKey && !e.metaKey && !e.shiftKey && !e.altKey) {
              e.preventDefault();
              e.stopPropagation();
              cycleStrength();
              return;
            }
            if (completion && completion.items.length > 0) {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setCompletion((c) => (c ? { ...c, selected: (c.selected + 1) % c.items.length } : c));
                return;
              }
              if (e.key === "ArrowUp") {
                e.preventDefault();
                setCompletion((c) => (c ? { ...c, selected: (c.selected - 1 + c.items.length) % c.items.length } : c));
                return;
              }
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                e.stopPropagation();
                applyCompletion();
                return;
              }
              if (e.key === "Escape") {
                e.preventDefault();
                setCompletion(null);
                return;
              }
            }
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
              return;
            }
            if (!e.shiftKey || e.ctrlKey || e.metaKey || e.altKey) return;
            const key = e.key.toLowerCase();
            if (key === "tab") {
              e.preventDefault();
              e.stopPropagation();
              cycleAgent();
            } else if (key === "p") {
              e.preventDefault();
              e.stopPropagation();
              cycleFavorite();
            }
          }}
        />
        <div className="composer-actions">
          <button
            className={`composer-send ${busy ? "stop" : ""}`}
            title={busy ? (assistantStatus?.statusText ?? "Stop the agent") : canSend ? "Send (Enter)" : "Type a prompt first"}
            disabled={!busy && !canSend}
            onClick={busy ? () => void stop(workspace ?? undefined) : send}
          >
            {busy ? <IconStop /> : <IconArrowUp />}
          </button>
        </div>
        {overlay}
        <div className="composer-chips">
          {supportsAttachments && (
            <>
              <input
                ref={attachmentInputRef}
                className="composer-file-input"
                type="file"
                multiple
                hidden
                onChange={(event) => {
                  addAttachmentPaths([...event.currentTarget.files ?? []]
                    .map((file) => window.vela.getPathForFile(file))
                    .filter(Boolean));
                  event.currentTarget.value = "";
                }}
              />
              <button className="composer-attach" title="Attach files" aria-label="Attach files" onClick={() => attachmentInputRef.current?.click()}>
                <IconAdd />
              </button>
            </>
          )}
          <button
            className={`composer-selector agent ${menu === "agent" ? "open" : ""}`}
            title="Change agent"
            onClick={() => {
              if (menu === "agent") {
                setMenu(null);
                return;
              }
              setMenu("agent");
              if (agents.length === 0) void loadAgents(workspace ?? undefined);
            }}
          >
            <IconGitBranch />
            <span>{currentAgent?.name ?? "Agent"}</span>
            <IconChevronDown />
          </button>
          <button
            className={`composer-selector model ${menu === "model" && modelView !== "strength" ? "open" : ""}`}
            title="Change model and response strength"
            onClick={() => {
              if (menu !== "model") {
                reloadPreferences();
              }
              setMenu(menu === "model" ? null : "model");
              if (menu !== "model") setModelView("list");
              if (menu !== "model" && models.length === 0) void loadModels(workspace ?? undefined);
            }}
          >
            <span>{currentModel?.name ?? "Model"}{variantLabel ? ` ${variantLabel}` : ""}</span>
            <IconChevronDown />
          </button>
          {currentModel?.variants && currentModel.variants.length > 0 && (
            <button
              className={`composer-selector strength ${menu === "model" && modelView === "strength" ? "open" : ""}`}
              title="Change response strength"
              onClick={() => {
                if (menu === "model" && modelView === "strength") {
                  setMenu(null);
                  return;
                }
                setMenu("model");
                setModelView("strength");
              }}
            >
              <span>{variantLabel}</span>
              <IconChevronDown />
            </button>
          )}
          {controls}
        </div>
      </div>

      {notice && <div className="composer-notice">{notice}</div>}

      {workspace && (
        <QueuedMessageChips
          workspace={workspace}
          onEditMessage={(content, attachments) => {
            setInput(content);
            const restored = attachments.map((file) => ({ path: file.path, name: file.path.split(/[\\/]/).pop() ?? file.path }));
            setFiles(restored);
            for (const file of restored) void loadPreview(file.path);
          }}
        />
      )}

      {completion && completion.items.length > 0 && (
        <div className="composer-completions">
          <div className="composer-completions-head">
            {completion.kind === "command" ? "Commands" : "Mention files"}
          </div>
          {completion.items.map((item, index) => (
            <button
              key={`${completion.kind}:${item.label}`}
              className={`composer-menu-item ${index === completion.selected ? "selected" : ""}`}
              onMouseDown={(e) => {
                e.preventDefault();
                applyCompletion(index);
              }}
            >
              {completion.kind === "command" ? <IconTerminal /> : <IconFile />}
              <span className="composer-completion-label">{item.label}</span>
              {item.detail && <span className="composer-completion-detail">{item.detail}</span>}
            </button>
          ))}
        </div>
      )}

      {menu && (
        <div className="composer-menu">
          {menu === "agent" ? (
            agents.length > 0 ? (
              agents.map((agent) => (
                <button
                  key={agent.id}
                  className={`composer-menu-item ${currentAgent?.id === agent.id ? "selected" : ""}`}
                  onClick={() => {
                    void switchAgent(agent.id, workspace ?? undefined);
                    setMenu(null);
                  }}
                >
                  <span className="composer-menu-check">{currentAgent?.id === agent.id ? <IconCheck /> : ""}</span>
                  {agent.name}
                </button>
              ))
            ) : (
              <div className="composer-menu-empty">No agents available.</div>
            )
          ) : (
            <ComposerModelMenu state={modelState} currentModel={currentModel} variantLabel={variantLabel} />
          )}
        </div>
      )}
    </div>
  );
}

