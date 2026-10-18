import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import type { CommandOption, McpServerOption, MiraAppUpdateStatus, PluginOption, SkillOption } from "@shared/types";
import { useStore } from "../store";
import type { UnsavedEditorFile } from "../store";
import { useTheme } from "../theme";
import { AppearanceSettings } from "./AppearanceSettings";
import { MiraMark } from "./MiraMark";
import { ProviderSettings } from "./ProviderSettings";
import { ServerSettings } from "./ServerSettings";
import { MobileSetup } from "./MobileSetup";
import type { SettingsSection } from "./SettingsSidebar";


const sectionCopy: Record<SettingsSection, { title: string; description: string }> = {
  appearance: { title: "Appearance", description: "Choose how Mira looks and how code is presented." },
  plugins: { title: "Plugins", description: "Review commands and skills available in the current workspace." },
  providers: { title: "Providers", description: "Connect model services supported by OpenCode." },
  safety: { title: "Safety", description: "Set permission and follow-up defaults for agent behavior." },
  voice: { title: "Voice", description: "Configure voice input preferences and review availability." },
  model: { title: "Model", description: "Choose the model used by the current workspace." },
  servers: { title: "Servers", description: "See and stop the Vite preview servers running in this app." },
  mobile: { title: "Mobile Setup", description: "Prepare secure access to Mira from another device." },
  about: { title: "About", description: "Version and product information for this installation." }
};

function SettingRow({ title, detail, control }: { title: string; detail: string; control: ReactNode }): ReactNode {
  return (
    <div className="settings-list-row">
      <div><strong>{title}</strong><small>{detail}</small></div>
      {control}
    </div>
  );
}

function miraUpdateErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  if (/No handler registered for ['"]?shell:app-update(?:-check)?['"]?/.test(message)) {
    return "The updater is not available in this running Mira process. Quit Mira completely and reopen it, then check again.";
  }
  return message;
}

function SettingsHeader({ section, onClose }: { section: SettingsSection; onClose: () => void }): ReactNode {
  const copy = sectionCopy[section];
  return (
    <header className="settings-page-header">
      <div className="settings-page-brand"><MiraMark size={34} /></div>
      <div>
        <p className="settings-page-kicker">Mira preferences</p>
        <h1>{copy.title}</h1>
        <p>{copy.description}</p>
      </div>
      <button className="settings-close" onClick={onClose} aria-label="Back to workspace" title="Back to workspace">
        {section === "appearance" ? <i className="codicon codicon-close" aria-hidden="true" /> : "Back to workspace"}
      </button>
    </header>
  );
}

export function SettingsPage({ section, onClose }: { section: SettingsSection; onClose: () => void }): ReactNode {
  const { theme } = useTheme();
  const {
    session,
    unsavedEditorFileCount,
    unsavedEditorFiles,
    focusSession,
    openFile,
    openExternalPath,
    runtimes,
    models,
    currentModel,
    switchModel,
    loadModels,
    refreshRuntimes,
    providerUsage,
    refreshProviderUsage,
    approvalMode,
    toggleApprovalMode,
    followUpBehavior,
    setFollowUpBehavior
  } = useStore();
  const [commands, setCommands] = useState<CommandOption[]>([]);
  const [mcpServers, setMcpServers] = useState<McpServerOption[]>([]);
  const [plugins, setPlugins] = useState<PluginOption[]>([]);
  const [skills, setSkills] = useState<SkillOption[]>([]);
  const [openCodeAction, setOpenCodeAction] = useState<"sync" | "update" | null>(null);
  const [openCodeFeedback, setOpenCodeFeedback] = useState("");
  const [miraUpdateStatus, setMiraUpdateStatus] = useState<MiraAppUpdateStatus | null>(null);
  const [miraUpdateAction, setMiraUpdateAction] = useState<"check" | "update" | null>(null);
  const [miraUpdateFeedback, setMiraUpdateFeedback] = useState("");
  const miraUpdateOperationRef = useRef(0);
  const runtime = (runtimes ?? []).find((item) => item.id === (session?.runtimeID ?? "opencode"));
  const openCodeRuntime = (runtimes ?? []).find((item) => item.id === "opencode");

  const runOpenCodeAction = async (action: "sync" | "update"): Promise<void> => {
    if (openCodeAction) return;
    const intent = action === "update"
      ? "OpenCode will update its global CLI using its own updater. Mira will then restart the shared OpenCode service to match."
      : "Mira will restart the shared OpenCode service so it matches the installed CLI.";
    if (!window.confirm(`${intent}\n\nActive agent runs in Mira or other OpenCode apps may be interrupted. Continue?`)) return;
    setOpenCodeAction(action);
    setOpenCodeFeedback("");
    try {
      const result = action === "update"
        ? await window.openshell.updateOpenCode()
        : await window.openshell.syncOpenCode();
      await refreshRuntimes().catch(() => []);
      setOpenCodeFeedback(action === "update"
        ? result.cliUpdated
          ? `Updated OpenCode from ${result.previousVersion} to ${result.version}. Mira is synced.`
          : `OpenCode ${result.version} is already current. Mira is synced.`
        : `Mira is synced to OpenCode ${result.version}.`);
    } catch (error) {
      setOpenCodeFeedback(error instanceof Error ? error.message : String(error));
    } finally {
      setOpenCodeAction(null);
    }
  };

  const checkMiraUpdate = async (): Promise<void> => {
    if (!window.openshell.isReleaseBuild) return;
    if (miraUpdateAction) return;
    const operation = ++miraUpdateOperationRef.current;
    setMiraUpdateAction("check");
    setMiraUpdateFeedback("");
    try {
      const status = await window.openshell.checkAppUpdate();
      if (miraUpdateOperationRef.current === operation) setMiraUpdateStatus(status);
    } catch (error) {
      if (miraUpdateOperationRef.current === operation) setMiraUpdateFeedback(miraUpdateErrorMessage(error));
    } finally {
      if (miraUpdateOperationRef.current === operation) setMiraUpdateAction(null);
    }
  };

  const updateMira = async (): Promise<void> => {
    if (!window.openshell.isReleaseBuild) return;
    if (miraUpdateAction === "update") return;
    if (unsavedEditorFileCount > 0) {
      setMiraUpdateFeedback(`Save ${unsavedEditorFileCount} unsaved file${unsavedEditorFileCount === 1 ? "" : "s"} before Mira installs the release and relaunches. Select a file below to return to it.`);
      return;
    }
    const operation = ++miraUpdateOperationRef.current;
    setMiraUpdateAction("update");
    setMiraUpdateFeedback("");
    try {
      const result = await window.openshell.updateApp();
      if (miraUpdateOperationRef.current !== operation) return;
      setMiraUpdateFeedback(result.message);
      if (result.updated && result.latestVersion) {
        setMiraUpdateStatus({
          state: "current",
          currentVersion: result.latestVersion,
          latestVersion: result.latestVersion
        });
      } else {
        const status = await window.openshell.checkAppUpdate();
        if (miraUpdateOperationRef.current === operation) setMiraUpdateStatus(status);
      }
    } catch (error) {
      if (miraUpdateOperationRef.current === operation) setMiraUpdateFeedback(miraUpdateErrorMessage(error));
    } finally {
      if (miraUpdateOperationRef.current === operation) setMiraUpdateAction(null);
    }
  };

  const returnToUnsavedFile = async (file: UnsavedEditorFile): Promise<void> => {
    if (!file.sessionID || !file.workspace) return;
    focusSession(file.sessionID);
    if (file.standalone) {
      const openedPath = await openExternalPath(file.path, file.workspace);
      if (!openedPath) return;
    } else {
      await openFile(file.path, file.workspace);
    }
    onClose();
  };

  useEffect(() => {
    if (section !== "about" || !window.openshell.isReleaseBuild) return;
    let cancelled = false;
    const operation = ++miraUpdateOperationRef.current;
    setMiraUpdateAction("check");
    setMiraUpdateFeedback("");
    void window.openshell.checkAppUpdate().then((status) => {
      if (!cancelled && miraUpdateOperationRef.current === operation) setMiraUpdateStatus(status);
    }).catch((error: unknown) => {
      if (!cancelled && miraUpdateOperationRef.current === operation) setMiraUpdateFeedback(miraUpdateErrorMessage(error));
    }).finally(() => {
      if (!cancelled && miraUpdateOperationRef.current === operation) setMiraUpdateAction(null);
    });
    return () => {
      cancelled = true;
      if (miraUpdateOperationRef.current === operation) miraUpdateOperationRef.current++;
    };
  }, [section]);

  useEffect(() => {
    if (section !== "model") return;
    void refreshRuntimes().catch(() => {});
    if (session) void loadModels(session.workspace);
  }, [section, session?.workspace, refreshRuntimes, loadModels]);

  useEffect(() => {
    if (section !== "plugins") return;
    if (!session) {
      setCommands([]);
      setMcpServers([]);
      setPlugins([]);
      setSkills([]);
      return;
    }
    void window.openshell.commands(session.workspace).then(setCommands).catch(() => setCommands([]));
    void window.openshell.mcpList(session.workspace).then(setMcpServers).catch(() => setMcpServers([]));
    void window.openshell.pluginsList(session.workspace).then(setPlugins).catch(() => setPlugins([]));
    void window.openshell.skillsList(session.workspace).then(setSkills).catch(() => setSkills([]));
  }, [section, session]);

  useEffect(() => {
    if (section === "providers" && session) void refreshProviderUsage();
  }, [section, session?.id, refreshProviderUsage]);

  return (
    <main className="settings-page" data-settings-section={section}>
      <SettingsHeader section={section} onClose={onClose} />

      {section === "appearance" && <AppearanceSettings />}

      {section === "plugins" && <section className="settings-section">
        <div className="settings-list">
          {commands.length === 0 ? <div className="settings-empty">No workspace commands or skills were reported.</div> : commands.map((command) => (
            <SettingRow key={`${command.kind}:${command.name}`} title={command.name} detail={command.description ?? "No description provided."} control={<span className="settings-badge">{command.kind ?? "command"}</span>} />
          ))}
        </div>
        <h2 className="settings-group-title">Skills</h2>
        <div className="settings-list">
          {skills.length === 0 ? <div className="settings-empty">No skills are available in this workspace.</div> : skills.map((skill) => (
            <SettingRow key={skill.id} title={skill.name} detail={skill.description ?? skill.location ?? "No description provided."} control={<span className="settings-badge">{skill.slash ? "slash" : "skill"}</span>} />
          ))}
        </div>
        <h2 className="settings-group-title">Plugins</h2>
        <div className="settings-list">
          {plugins.length === 0 ? <div className="settings-empty">No plugins are active in this workspace.</div> : plugins.map((plugin) => (
            <SettingRow key={plugin.id} title={plugin.id} detail={`Source: ${plugin.source}`} control={<span className="settings-badge">{plugin.status}</span>} />
          ))}
        </div>
        <h2 className="settings-group-title">MCP servers</h2>
        <div className="settings-list">
          {mcpServers.length === 0 ? <div className="settings-empty">No MCP servers are configured for this workspace.</div> : mcpServers.map((server) => (
            <SettingRow key={server.name} title={server.name} detail="Managed by the OpenCode runtime." control={<span className={`settings-badge ${server.status === "connected" ? "available" : ""}`}>{server.status}</span>} />
          ))}
        </div>
      </section>}

      {section === "providers" && <section className="settings-section">
        {runtime && !runtime.capabilities.providerCredentials
          ? <div className="settings-callout"><strong>Managed by {runtime.name}</strong><p>This runtime does not expose provider credential editing through Mira. Configure credentials in the runtime, then refresh its model list here.</p></div>
          : <ProviderSettings workspace={session?.workspace ?? null} usage={providerUsage} refreshModels={() => loadModels(session?.workspace)} />}
      </section>}

      {section === "safety" && <section className="settings-section">
        <div className="settings-list">
          {(!runtime || runtime.capabilities.permissions) && <SettingRow
            title="Tool permissions"
            detail="Choose whether agent tool actions need confirmation."
            control={<span className="settings-segmented"><button className={approvalMode !== "approve" ? "on" : ""} onClick={() => approvalMode === "approve" && toggleApprovalMode()}>Ask</button><button className={approvalMode === "approve" ? "on" : ""} onClick={() => approvalMode !== "approve" && toggleApprovalMode()}>Approve</button></span>}
          />}
          {(!runtime || runtime.capabilities.steering) && <SettingRow
            title="Follow-up behavior"
            detail="Queue new prompts or use them to steer active work."
            control={<span className="settings-segmented"><button className={followUpBehavior !== "steer" ? "on" : ""} onClick={() => setFollowUpBehavior("queue")}>Queue</button><button className={followUpBehavior === "steer" ? "on" : ""} onClick={() => setFollowUpBehavior("steer")}>Steer</button></span>}
          />}
        </div>
      </section>}

      {section === "voice" && <section className="settings-section">
        <div className="settings-list">
          <SettingRow title="Voice input" detail="Voice transcription is not included in this build. Controls will appear here when a voice service is available." control={<span className="settings-badge">Unavailable</span>} />
        </div>
      </section>}

      {section === "model" && <section className="settings-section">
        <div className="settings-list">
          <SettingRow
            title="Default model"
            detail={session ? `Used for new prompts in ${session.directory}.` : "Open a workspace to choose its default model."}
            control={<select className="settings-select" value={currentModel ? `${currentModel.providerID}:${currentModel.id}` : ""} disabled={!session || models.length === 0} onChange={(event) => {
              const model = models.find((option) => `${option.providerID}:${option.id}` === event.target.value);
              if (model) void switchModel(model.id, model.providerID, model.variant);
            }}><option value="">{models.length === 0 ? "No models available" : "Select a model"}</option>{models.map((model) => <option key={`${model.providerID}:${model.id}`} value={`${model.providerID}:${model.id}`}>{model.name} · {model.providerID}</option>)}</select>}
          />
        </div>
        <h2 className="settings-group-title">OpenCode</h2>
        <div className="settings-callout"><strong>Mira uses the global OpenCode CLI.</strong><p>Sync and update restart the shared OpenCode service. Active runs in Mira or other OpenCode apps may be interrupted.</p></div>
        <div className="settings-list">
          <SettingRow
            title="Installed CLI"
            detail="The version resolved from Mira's PATH."
            control={<span className="settings-badge">{openCodeRuntime?.version ?? "Not found"}</span>}
          />
          <SettingRow
            title="Sync installed version"
            detail="Restart the shared service to use the CLI version already installed."
            control={<button className="settings-action-button" disabled={!openCodeRuntime?.version || openCodeAction !== null} onClick={() => void runOpenCodeAction("sync")}>{openCodeAction === "sync" ? "Syncing…" : "Sync"}</button>}
          />
          <SettingRow
            title="Update to latest"
            detail="Run OpenCode's updater for its detected install method, then sync the service."
            control={<button className="settings-action-button" disabled={!openCodeRuntime?.version || openCodeAction !== null} onClick={() => void runOpenCodeAction("update")}>{openCodeAction === "update" ? "Updating…" : "Update"}</button>}
          />
        </div>
        {openCodeFeedback && <p className="settings-action-feedback" role="status" aria-live="polite">{openCodeFeedback}</p>}
      </section>}

      {section === "servers" && <ServerSettings />}

      {section === "mobile" && <MobileSetup />}

      {section === "about" && <section className="settings-section">
        <div className="settings-about"><MiraMark size={72} /><div><h2>Mira</h2><p>Version {window.openshell.appVersion} · {window.openshell.isReleaseBuild ? "Release build" : "Development/test build"}</p><small>A native desktop cockpit for coding agents.</small></div></div>
        <h2 className="settings-group-title">App updates</h2>
        <div className="settings-list">
          <SettingRow
            title={window.openshell.isReleaseBuild ? "GitHub Releases" : "Development build"}
            detail={window.openshell.isReleaseBuild
              ? "Checks published Mira releases only. GitHub commits become app updates after a release is published. Update downloads and installs that release, then restarts Mira."
              : "GitHub app updates are disabled here. Develop and test this local checkout; source edits do not update the installed app through the release updater."}
            control={window.openshell.isReleaseBuild
              ? <div className="settings-update-actions">
                <button
                  className="settings-action-button primary"
                  disabled={miraUpdateAction === "update"}
                  onClick={() => void updateMira()}
                >{miraUpdateAction === "update" ? "Downloading…" : "Update Mira"}</button>
                <button
                  className="settings-action-button"
                  disabled={miraUpdateAction !== null}
                  onClick={() => void checkMiraUpdate()}
                >{miraUpdateAction === "check" ? "Checking…" : "Check for updates"}</button>
              </div>
              : <small>Updater disabled</small>}
          />
        </div>
        {window.openshell.isReleaseBuild && (miraUpdateFeedback || miraUpdateStatus) && <p className="settings-action-feedback" role="status" aria-live="polite">
          {miraUpdateFeedback || (miraUpdateStatus?.state === "available"
            ? `Mira ${miraUpdateStatus.currentVersion} is installed · release ${miraUpdateStatus.latestVersion} is available. Click Update Mira to download and install it, then Mira will restart.`
            : miraUpdateStatus?.state === "current"
              ? `Mira ${miraUpdateStatus.currentVersion} is up to date with the latest published release (${miraUpdateStatus.latestVersion}). New commits are available after a release is published.`
              : miraUpdateStatus?.message)}
        </p>}
        {unsavedEditorFileCount > 0 && <div className="settings-unsaved-files" role="alert">
          <strong>Save these files before closing or updating Mira.</strong>
          <ul>
            {unsavedEditorFiles.map((file) => <li key={`${file.workspaceID}:${file.path}`}>
              <button
                type="button"
                className="settings-unsaved-file"
                disabled={!file.sessionID || !file.workspace}
                title={file.sessionID ? `Open ${file.displayPath}` : "This workspace is closed"}
                onClick={() => void returnToUnsavedFile(file)}
              >
                <span>{file.displayPath}</span>
                <strong>{file.sessionID ? "Open" : "Workspace closed"}</strong>
              </button>
            </li>)}
          </ul>
        </div>}
      </section>}
    </main>
  );
}
