use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use vela_core::document::ExternalChange;
use vela_core::event::{AgentEvent, PermissionRequest, TodoItem, parse_event};
use vela_core::session::SessionSummary;
use vela_core::transcript::Transcript;
use vela_core::warm::WarmEntry;
use vela_opencode::client::PermissionReply;
use vela_opencode::{AgentInfo, Client, Endpoint, FormRequest, InboxItem, ModelInfo, PromptFile};

use crate::persist::Settings;
use crate::store::{SessionView, Store};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Ready,
    Disconnected,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SassLogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SassLine {
    pub time_ms: u64,
    pub level: SassLogLevel,
    pub message: String,
}
pub(crate) fn sass_log_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

/// Deliberately has no Debug implementation: this request owns a credential.
pub struct ProviderKeyConnection {
    pub request_id: u64,
    pub session_id: String,
    pub directory: String,
    pub integration_id: String,
    pub key: String,
    pub label: String,
    pub answers: serde_json::Map<String, Value>,
}

pub struct ProviderCredentialRemoval {
    pub request_id: u64,
    pub session_id: String,
    pub directory: String,
    pub integration_id: String,
    pub credential_id: String,
}

#[derive(Clone, Debug)]
pub enum EngineUpdate {
    WorkspaceFiles {
        directory: String,
        generation: u64,
        changes: Arc<Vec<crate::observation::ObservedFile>>,
        tree_changed: bool,
        git_changed: bool,
        error: Option<String>,
    },
    Previews,
    PreviewFinished {
        request_id: u64,
        session_id: Option<String>,
        result: Result<Option<crate::preview::PreviewServer>, String>,
    },
    RuntimeFinished {
        request_id: u64,
        result: Result<crate::runtime::RuntimeResult, String>,
    },
    RuntimeInfo {
        request_id: u64,
        cli_version: Option<String>,
        capabilities: crate::runtime::RuntimeCapabilities,
    },
    SettingsApplied {
        error: Option<String>,
    },
    Connection,
    Sessions,
    WorkspaceOpened {
        directory: String,
        result: Result<String, String>,
    },
    Session(String),
    SessionRemoved(String),
    SessionDeleteFinished {
        request_id: u64,
        session_id: String,
        error: Option<String>,
    },
    Permissions,
    PermissionAsked(PermissionRequest),
    Todo(String),
    TerminalData {
        id: String,
        data: Arc<Vec<u8>>,
    },
    TerminalExit {
        id: String,
        code: Option<i32>,
    },
    Directory {
        request_id: u64,
        rel: String,
        entries: Vec<crate::files::FileEntry>,
        error: Option<String>,
    },
    EntryResolved {
        request_id: u64,
        path: Option<std::path::PathBuf>,
        error: Option<String>,
    },
    EntryRemoved {
        request: crate::files::RemoveEntryRequest,
        error: Option<String>,
    },
    EntriesImported {
        request: crate::imports::Request,
        outcomes: Vec<crate::imports::Outcome>,
        error: Option<String>,
    },
    EntryDetached {
        request: crate::detach::Request,
        path: Option<std::path::PathBuf>,
        error: Option<String>,
    },
    EntryRenamed {
        request: crate::files::RenameEntryRequest,
        new_rel: Option<String>,
        snapshots: Vec<crate::files::RenameSnapshot>,
        error: Option<String>,
    },
    EntryCreated {
        request: crate::files::CreateEntryRequest,
        rel: Option<String>,
        error: Option<String>,
    },
    FileContent {
        request_id: u64,
        rel: String,
        content: Option<String>,
        deleted: bool,
        error: Option<String>,
    },
    FileSaved {
        request_id: u64,
        rel: String,
        error: Option<String>,
        conflict: Option<ExternalChange>,
    },
    Recovery {
        session_id: String,
        records: Vec<crate::recovery::RecoveryRecord>,
    },
    RecoveryFailed {
        session_id: String,
        error: String,
    },
    GitStatus {
        request_id: u64,
        entries: Vec<crate::git::GitEntry>,
        error: Option<String>,
    },
    GitDiff {
        request_id: u64,
        rel: String,
        content: Option<String>,
        full: Option<String>,
        error: Option<String>,
    },
    Catalog {
        session_id: String,
        models: Vec<ModelInfo>,
        agents: Vec<AgentInfo>,
        default_model: Option<ModelInfo>,
        error: Option<String>,
    },
    Integrations {
        request_id: u64,
        session_id: String,
        mcp_servers: Vec<vela_opencode::inventory::McpServerInfo>,
        plugins: Vec<vela_opencode::inventory::PluginInfo>,
        skills: Vec<vela_opencode::inventory::SkillInfo>,
        error: Option<String>,
    },
    ProviderUsage {
        request_id: u64,
        results: Vec<crate::provider_usage::ProviderUsageResult>,
    },
    Providers {
        request_id: u64,
        session_id: String,
        providers: Vec<vela_opencode::providers::ProviderIntegration>,
        error: Option<String>,
    },
    ProviderConnected {
        request_id: u64,
        session_id: String,
        integration_id: String,
        error: Option<String>,
    },
    ProviderCredentialRemoved {
        request_id: u64,
        session_id: String,
        integration_id: String,
        credential_id: String,
        error: Option<String>,
    },
    ProviderOAuth {
        flow_id: u64,
        session_id: String,
        integration_id: String,
        state: crate::ProviderOAuthState,
    },
    Selection {
        session_id: String,
        model: Option<ModelInfo>,
        agent: Option<String>,
    },
    Inbox {
        session_id: String,
        items: Vec<InboxItem>,
        error: Option<String>,
    },
    Forms {
        session_id: String,
        forms: Vec<FormRequest>,
        error: Option<String>,
    },
    FormFinished {
        request_id: u64,
        session_id: String,
        form_id: String,
        error: Option<String>,
    },
    LocalQueue(String),
    SessionSettled {
        session_id: String,
        stalled: bool,
    },
    RevertFinished {
        request_id: u64,
        session_id: String,
        error: Option<String>,
    },
    RevertStaged {
        session_id: String,
    },
    SlashCommandFinished {
        request_id: u64,
        session_id: String,
        error: Option<String>,
    },
    CommandCatalog {
        session_id: String,
        commands: Vec<vela_opencode::CommandOption>,
        error: Option<String>,
    },
    References {
        request_id: u64,
        session_id: String,
        files: Vec<String>,
        error: Option<String>,
    },
    Validation {
        request_id: u64,
        session_id: String,
        path: String,
        diagnostics: Vec<crate::validation::Diagnostic>,
        error: Option<String>,
    },
    Sass {
        session_id: String,
        state: crate::SassWatchState,
    },
    Error(String),
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub warm_sessions: usize,
    pub warm_bytes: usize,
    pub data_dir: Option<PathBuf>,
    pub service_file: Option<PathBuf>,
    pub opencode_binary: Option<PathBuf>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            warm_sessions: 6,
            warm_bytes: 96 * 1024 * 1024,
            data_dir: None,
            service_file: None,
            opencode_binary: None,
        }
    }
}

struct Inner {
    settings: RwLock<Settings>,
    message_queues: std::sync::Mutex<crate::message_queue::MessageQueues>,
    store: RwLock<Store>,
    updates: broadcast::Sender<EngineUpdate>,
    commands: mpsc::UnboundedSender<Command>,
    cancel: CancellationToken,
    stopped: CancellationToken,
    observations: crate::observation::ObservationPool,
    #[cfg(unix)]
    preview_processes: crate::preview::ProcessGroups,
    config: EngineConfig,
    terminals:
        std::sync::Mutex<std::collections::HashMap<String, Arc<vela_terminal::pty::PtySession>>>,
    sass_watches: std::sync::Mutex<crate::sass_state::Watches>,
    session_loads: std::sync::Mutex<HashMap<String, uuid::Uuid>>,
    forms: std::sync::Mutex<HashMap<String, Vec<FormRequest>>>,
    validation_cancels: std::sync::Mutex<HashMap<u64, CancellationToken>>,
}

#[derive(Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

#[derive(Clone, Debug)]
pub struct TerminalCommand {
    pub id: String,
    pub cwd: String,
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

enum Command {
    RetainEditorWorkspace {
        session_id: String,
        directory: String,
    },
    ReleaseEditorWorkspace(String),
    WriteDocument(vela_core::document::FileRequest),
    ReadDocument {
        request_id: u64,
        key: vela_core::document::DocumentKey,
    },
    LoadRecovery {
        session_id: String,
    },
    AcknowledgeRecovery {
        session_id: String,
        record_id: String,
    },
    TogglePreview {
        request_id: u64,
        session_id: String,
        entry: Option<String>,
    },
    StopPreview {
        request_id: u64,
        server_id: Option<String>,
    },
    ManageRuntime {
        request_id: u64,
        action: crate::runtime::RuntimeAction,
    },
    RuntimeFinished {
        request_id: u64,
        result: Result<crate::runtime::RuntimeSync, String>,
    },
    ReconnectFinished {
        attempt: u64,
        result: Result<(Client, Endpoint, PathBuf), String>,
    },
    LoadRuntimeInfo(u64),
    ApplySettings(Settings),
    SaveWindowSize(crate::window_state::WindowSize),
    OpenDirectory(String),
    OpenFileWorkspace {
        file: String,
        reply: tokio::sync::oneshot::Sender<Result<crate::FileWorkspace, String>>,
    },
    OpenById(String),
    RefreshSessions,
    DeleteSession {
        request_id: u64,
        session_id: String,
    },
    SessionDeletionFinished {
        request_id: u64,
        session_id: String,
        error: Option<String>,
    },
    Loaded {
        session_id: String,
        generation: uuid::Uuid,
        result: Result<(SessionSummary, Value), String>,
    },
    Prompt {
        session_id: String,
        text: String,
        files: Vec<PromptFile>,
        delivery: Option<String>,
        retry: Option<(String, String)>,
    },
    RemoveQueued {
        directory: String,
        session_id: String,
        id: String,
    },
    ReorderQueued {
        directory: String,
        session_id: String,
        from: String,
        to: String,
    },
    RetryQueued {
        directory: String,
        session_id: String,
        id: String,
    },
    Interrupt(String),
    RevertSession {
        request_id: u64,
        session_id: String,
        message_id: String,
    },
    CommitRevert {
        request_id: u64,
        session_id: String,
    },
    ClearRevert {
        request_id: u64,
        session_id: String,
    },
    Slash {
        request_id: u64,
        session_id: String,
        name: String,
        args: String,
    },
    RefreshCommands {
        session_id: String,
    },
    FindReferences {
        request_id: u64,
        session_id: String,
        query: String,
    },
    Validate {
        request_id: u64,
        session_id: String,
        path: String,
        content: String,
    },
    ValidateScss {
        request_id: u64,
        session_id: String,
        path: String,
        content: String,
    },
    CancelValidation {
        request_id: u64,
    },
    ReplyPermission {
        session_id: String,
        request_id: String,
        reply: PermissionReply,
    },
    RefreshPermissions,
    Close(String),
    SetDraft {
        session_id: String,
        draft: String,
    },
    SetScroll {
        session_id: String,
        top: f32,
        stick: bool,
    },
    StartTerminal {
        id: String,
        cwd: String,
        cols: u16,
        rows: u16,
    },
    TerminalExec(TerminalCommand),
    TerminalInput {
        id: String,
        bytes: Vec<u8>,
    },
    TerminalResize {
        id: String,
        cols: u16,
        rows: u16,
    },
    StopTerminal(String),
    ListDir {
        request_id: u64,
        session_id: String,
        rel: String,
    },
    CreateEntry(crate::files::CreateEntryRequest),
    ImportExternal(crate::imports::Request),
    DetachEntry(crate::detach::Request),
    ResolveEntry {
        request_id: u64,
        session: String,
        workspace: String,
        rel: String,
        directory: bool,
    },
    RemoveEntry(crate::files::RemoveEntryRequest),
    RenameEntry(crate::files::RenameEntryRequest),
    ReadFile {
        request_id: u64,
        session_id: String,
        rel: String,
    },
    WriteFile {
        request_id: u64,
        session_id: String,
        rel: String,
        content: String,
        expected: String,
        overwrite: bool,
    },
    GitStatus {
        request_id: u64,
        session_id: String,
    },
    GitDiff {
        request_id: u64,
        session_id: String,
        rel: String,
    },
    LoadCatalog {
        session_id: String,
    },
    LoadIntegrations {
        request_id: u64,
        session_id: String,
    },
    LoadProviderUsage {
        request_id: u64,
    },
    LoadProviders {
        request_id: u64,
        session_id: String,
    },
    ConnectProviderKey(ProviderKeyConnection),
    RemoveProviderCredential(ProviderCredentialRemoval),
    StartProviderOAuth(crate::ProviderOAuthRequest),
    ProviderOAuthCode {
        flow_id: u64,
        code: String,
    },
    CancelProviderOAuth(u64),
    ProviderOAuthFinished(u64),
    SwitchModel {
        session_id: String,
        model: ModelInfo,
    },
    SwitchAgent {
        session_id: String,
        agent_id: String,
    },
    RefreshInbox {
        session_id: String,
    },
    RefreshForms {
        session_id: String,
    },
    ReplyForm {
        request_id: u64,
        session_id: String,
        form_session_id: String,
        form_id: String,
        answers: serde_json::Map<String, Value>,
    },
    CancelForm {
        request_id: u64,
        session_id: String,
        form_session_id: String,
        form_id: String,
    },
    CancelInbox {
        session_id: String,
        inbox_id: String,
    },
    SteerInbox {
        session_id: String,
        inbox_id: String,
    },
    SassWatch {
        session_id: String,
        enabled: bool,
        reply: Option<tokio::sync::oneshot::Sender<Result<crate::SassWatchState, String>>>,
    },
    Shutdown,
}

impl Engine {
    pub fn spawn(config: EngineConfig, runtime: &tokio::runtime::Handle) -> Self {
        let (updates, _) = broadcast::channel(1_024);
        let (commands, command_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let observations = crate::observation::ObservationPool::new(updates.clone());
        let sass_watches =
            std::sync::Mutex::new(crate::sass_state::Watches::load(config.data_dir.as_deref()));
        let settings = Settings::load(config.data_dir.as_deref());
        let inner = Arc::new(Inner {
            settings: RwLock::new(settings),
            message_queues: std::sync::Mutex::new(crate::message_queue::MessageQueues::load(
                config.data_dir.as_deref(),
            )),
            store: RwLock::new(Store::new(config.warm_sessions, config.warm_bytes)),
            updates,
            commands: commands.clone(),
            cancel: cancel.clone(),
            stopped: CancellationToken::new(),
            observations,
            #[cfg(unix)]
            preview_processes: crate::preview::ProcessGroups::default(),
            config,
            terminals: std::sync::Mutex::new(std::collections::HashMap::new()),
            sass_watches,
            session_loads: std::sync::Mutex::new(HashMap::new()),
            forms: std::sync::Mutex::new(HashMap::new()),
            validation_cancels: std::sync::Mutex::new(HashMap::new()),
        });
        let engine = Self {
            inner: inner.clone(),
        };
        runtime.spawn(async move {
            let _stopped = inner.stopped.clone().drop_guard();
            run(inner, command_rx).await;
        });
        engine
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EngineUpdate> {
        self.inner.updates.subscribe()
    }

    pub fn sessions(&self) -> Vec<SessionSummary> {
        self.read(|store| store.sessions_sorted())
    }

    pub fn connection(&self) -> ConnectionState {
        self.read(|store| store.connection)
    }

    pub fn view(&self, id: &str) -> Option<Arc<SessionView>> {
        self.read(|store| store.view(id))
    }

    pub fn active_view(&self) -> Option<Arc<SessionView>> {
        self.read(|store| store.active_view())
    }

    pub fn active_session(&self) -> Option<String> {
        self.read(|store| store.active_session.clone())
    }

    pub fn permissions(&self, session_id: &str) -> Vec<PermissionRequest> {
        self.read(|store| {
            store
                .permissions
                .get(session_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn all_permissions(&self) -> Vec<PermissionRequest> {
        self.read(|store| store.permissions.values().flatten().cloned().collect())
    }

    pub fn todos(&self, session_id: &str) -> Vec<TodoItem> {
        self.read(|store| store.todos.get(session_id).cloned().unwrap_or_default())
    }

    pub fn usage(&self, session_id: &str) -> Option<vela_core::session::SessionUsage> {
        self.read(|store| store.usage.get(session_id).copied())
    }

    pub fn endpoint(&self) -> Option<String> {
        self.read(|store| store.endpoint.clone())
    }

    pub fn last_error(&self) -> Option<String> {
        self.read(|store| store.last_error.clone())
    }

    pub fn open_views(&self) -> usize {
        self.read(|store| store.views.len())
    }

    pub fn open_session_ids(&self) -> Vec<String> {
        self.read(|store| store.views.keys().cloned().collect())
    }

    pub fn warm_len(&self) -> usize {
        self.read(|store| store.warm.len())
    }

    pub fn retained_bytes(&self) -> usize {
        self.read(|store| store.warm.retained_bytes())
    }

    pub fn protocol_drift(&self) -> Vec<(String, u64)> {
        self.read(|store| {
            let mut drift: Vec<_> = store
                .protocol_drift
                .iter()
                .map(|(event, count)| (event.clone(), *count))
                .collect();
            drift.sort();
            drift
        })
    }

    pub fn settings(&self) -> Settings {
        self.inner
            .settings
            .read()
            .expect("settings lock")
            .clone()
    }

    pub fn warm_limits(&self) -> (usize, usize) {
        self.read(|store| (store.warm.limit(), store.warm.byte_budget()))
    }

    /// Persist preferences and hand the normalized snapshot to the engine so
    /// warm-cache limits and `settings()` agree with the Settings UI immediately
    /// instead of only after the next launch.
    pub fn apply_settings(&self, settings: Settings) {
        self.send(Command::ApplySettings(settings));
    }

    pub fn data_dir(&self) -> Option<PathBuf> {
        self.inner.config.data_dir.clone()
    }

    pub fn save_window_size(&self, size: crate::window_state::WindowSize) {
        self.send(Command::SaveWindowSize(size));
    }

    pub fn load_runtime_info(&self, request_id: u64) {
        self.send(Command::LoadRuntimeInfo(request_id));
    }

    pub fn manage_runtime(&self, request_id: u64, action: crate::runtime::RuntimeAction) {
        self.send(Command::ManageRuntime { request_id, action });
    }

    pub fn preview_servers(&self) -> Vec<crate::preview::PreviewServer> {
        self.read(|store| store.preview_servers.clone())
    }

    pub fn toggle_preview(&self, request_id: u64, session_id: String, entry: Option<String>) {
        self.send(Command::TogglePreview {
            request_id,
            session_id,
            entry,
        });
    }

    pub fn stop_preview(&self, request_id: u64, server_id: Option<String>) {
        self.send(Command::StopPreview {
            request_id,
            server_id,
        });
    }

    pub fn open_directory(&self, directory: impl Into<String>) {
        self.send(Command::OpenDirectory(directory.into()));
    }
    /// Create the selected file's conversation without changing active context.
    /// The caller activates the returned session only if its request is current.
    pub async fn open_file_workspace(
        &self,
        file: impl Into<String>,
    ) -> Result<crate::FileWorkspace, String> {
        let (reply, receiver) = tokio::sync::oneshot::channel();
        self.send(Command::OpenFileWorkspace {
            file: file.into(),
            reply,
        });
        receiver
            .await
            .map_err(|_| "Vela stopped before opening the file workspace".to_owned())?
    }

    pub fn open_session(&self, session_id: impl Into<String>) {
        self.send(Command::OpenById(session_id.into()));
    }

    pub fn refresh_sessions(&self) {
        self.send(Command::RefreshSessions);
    }

    pub fn prompt(&self, session_id: impl Into<String>, text: impl Into<String>) {
        self.send(Command::Prompt {
            session_id: session_id.into(),
            text: text.into(),
            files: Vec::new(),
            delivery: None,
            retry: None,
        });
    }

    /// Queue a follow-up while the current run is still active.
    pub fn queue_prompt(&self, session_id: impl Into<String>, text: impl Into<String>) {
        self.send(Command::Prompt {
            session_id: session_id.into(),
            text: text.into(),
            files: Vec::new(),
            delivery: Some("queue".into()),
            retry: None,
        });
    }

    /// Send the complete staged prompt in one request, including file-only prompts.
    pub fn prompt_with_files(
        &self,
        session_id: impl Into<String>,
        text: impl Into<String>,
        files: Vec<PromptFile>,
        queued: bool,
    ) {
        self.prompt_with_behavior(
            session_id,
            text,
            files,
            queued.then_some(crate::message_queue::FollowUpBehavior::Queue),
        );
    }

    pub fn prompt_with_behavior(
        &self,
        session_id: impl Into<String>,
        text: impl Into<String>,
        files: Vec<PromptFile>,
        delivery: Option<crate::message_queue::FollowUpBehavior>,
    ) {
        self.send(Command::Prompt {
            session_id: session_id.into(),
            text: text.into(),
            files,
            delivery: delivery.map(|behavior| behavior.as_str().into()),
            retry: None,
        });
    }

    pub fn local_queue(
        &self,
        directory: &str,
        session_id: &str,
    ) -> Vec<crate::message_queue::QueuedPrompt> {
        self.inner
            .message_queues
            .lock()
            .expect("message queue lock")
            .list(directory, session_id)
    }

    pub fn remove_queued(&self, directory: String, session_id: String, id: String) {
        self.send(Command::RemoveQueued {
            directory,
            session_id,
            id,
        });
    }

    pub fn reorder_queued(&self, directory: String, session_id: String, from: String, to: String) {
        self.send(Command::ReorderQueued {
            directory,
            session_id,
            from,
            to,
        });
    }

    pub fn retry_queued(&self, directory: String, session_id: String, id: String) {
        self.send(Command::RetryQueued {
            directory,
            session_id,
            id,
        });
    }

    pub fn interrupt(&self, session_id: impl Into<String>) {
        self.send(Command::Interrupt(session_id.into()));
    }

    pub fn revert_session(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        message_id: impl Into<String>,
    ) {
        self.send(Command::RevertSession {
            request_id,
            session_id: session_id.into(),
            message_id: message_id.into(),
        });
    }

    pub fn commit_revert(&self, request_id: u64, session_id: impl Into<String>) {
        self.send(Command::CommitRevert {
            request_id,
            session_id: session_id.into(),
        });
    }

    pub fn clear_revert(&self, request_id: u64, session_id: impl Into<String>) {
        self.send(Command::ClearRevert {
            request_id,
            session_id: session_id.into(),
        });
    }

    pub fn run_slash_command(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        name: impl Into<String>,
        args: impl Into<String>,
    ) {
        self.send(Command::Slash {
            request_id,
            session_id: session_id.into(),
            name: name.into(),
            args: args.into(),
        });
    }

    pub fn refresh_commands(&self, session_id: impl Into<String>) {
        self.send(Command::RefreshCommands {
            session_id: session_id.into(),
        });
    }

    pub fn find_references(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        query: impl Into<String>,
    ) {
        self.send(Command::FindReferences {
            request_id,
            session_id: session_id.into(),
            query: query.into(),
        });
    }

    pub fn validate_document(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        path: impl Into<String>,
        content: impl Into<String>,
    ) {
        self.send(Command::Validate {
            request_id,
            session_id: session_id.into(),
            path: path.into(),
            content: content.into(),
        });
    }

    pub fn validate_scss(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        path: impl Into<String>,
        content: impl Into<String>,
    ) {
        self.send(Command::ValidateScss {
            request_id,
            session_id: session_id.into(),
            path: path.into(),
            content: content.into(),
        });
    }

    pub fn cancel_validation(&self, request_id: u64) {
        self.send(Command::CancelValidation { request_id });
    }

    pub fn reply_permission(
        &self,
        session_id: impl Into<String>,
        request_id: impl Into<String>,
        reply: PermissionReply,
    ) {
        self.send(Command::ReplyPermission {
            session_id: session_id.into(),
            request_id: request_id.into(),
            reply,
        });
    }

    pub fn refresh_permissions(&self) {
        self.send(Command::RefreshPermissions);
    }

    pub fn delete_session(&self, request_id: u64, session_id: impl Into<String>) {
        self.send(Command::DeleteSession {
            request_id,
            session_id: session_id.into(),
        });
    }

    pub fn close_session(&self, session_id: impl Into<String>) {
        self.send(Command::Close(session_id.into()));
    }

    pub fn set_draft(&self, session_id: impl Into<String>, draft: impl Into<String>) {
        self.send(Command::SetDraft {
            session_id: session_id.into(),
            draft: draft.into(),
        });
    }

    pub fn set_scroll(&self, session_id: impl Into<String>, top: f32, stick_to_bottom: bool) {
        self.send(Command::SetScroll {
            session_id: session_id.into(),
            top,
            stick: stick_to_bottom,
        });
    }

    pub fn list_dir(&self, request_id: u64, session_id: impl Into<String>, rel: impl Into<String>) {
        self.send(Command::ListDir {
            request_id,
            session_id: session_id.into(),
            rel: rel.into(),
        });
    }

    pub fn resolve_entry(&self, request_id: u64, session: String, workspace: String, rel: String) {
        self.send(Command::ResolveEntry {
            request_id,
            session,
            workspace,
            rel,
            directory: false,
        });
    }
    pub fn resolve_terminal_directory(
        &self,
        request_id: u64,
        session: String,
        workspace: String,
        rel: String,
    ) {
        self.send(Command::ResolveEntry {
            request_id,
            session,
            workspace,
            rel,
            directory: true,
        });
    }
    pub fn remove_entry(&self, request: crate::files::RemoveEntryRequest) {
        self.send(Command::RemoveEntry(request));
    }
    pub fn rename_entry(&self, request: crate::files::RenameEntryRequest) {
        self.send(Command::RenameEntry(request));
    }

    pub fn create_entry(&self, request: crate::files::CreateEntryRequest) {
        self.send(Command::CreateEntry(request));
    }

    pub fn read_file(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        rel: impl Into<String>,
    ) {
        self.send(Command::ReadFile {
            request_id,
            session_id: session_id.into(),
            rel: rel.into(),
        });
    }

    pub fn write_file(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        rel: impl Into<String>,
        content: impl Into<String>,
        expected: impl Into<String>,
        overwrite: bool,
    ) {
        self.send(Command::WriteFile {
            request_id,
            session_id: session_id.into(),
            rel: rel.into(),
            content: content.into(),
            expected: expected.into(),
            overwrite,
        });
    }

    pub fn retain_editor_workspace(&self, session_id: String, directory: String) {
        self.send(Command::RetainEditorWorkspace {
            session_id,
            directory,
        });
    }

    pub fn release_editor_workspace(&self, directory: String) {
        self.send(Command::ReleaseEditorWorkspace(directory));
    }

    pub fn write_document(&self, request: vela_core::document::FileRequest) {
        self.send(Command::WriteDocument(request));
    }

    pub fn read_document(&self, request_id: u64, key: vela_core::document::DocumentKey) {
        self.send(Command::ReadDocument { request_id, key });
    }
    pub fn load_recovery(&self, session_id: impl Into<String>) {
        self.send(Command::LoadRecovery {
            session_id: session_id.into(),
        });
    }
    pub fn acknowledge_recovery(
        &self,
        session_id: impl Into<String>,
        record_id: impl Into<String>,
    ) {
        self.send(Command::AcknowledgeRecovery {
            session_id: session_id.into(),
            record_id: record_id.into(),
        });
    }

    pub fn git_status(&self, request_id: u64, session_id: impl Into<String>) {
        self.send(Command::GitStatus {
            request_id,
            session_id: session_id.into(),
        });
    }

    pub fn git_diff(&self, request_id: u64, session_id: impl Into<String>, rel: impl Into<String>) {
        self.send(Command::GitDiff {
            request_id,
            session_id: session_id.into(),
            rel: rel.into(),
        });
    }

    pub fn start_terminal(
        &self,
        id: impl Into<String>,
        cwd: impl Into<String>,
        cols: u16,
        rows: u16,
    ) {
        self.send(Command::StartTerminal {
            id: id.into(),
            cwd: cwd.into(),
            cols,
            rows,
        });
    }

    pub fn start_terminal_command(&self, command: TerminalCommand) {
        self.send(Command::TerminalExec(command));
    }

    pub fn terminal_input(&self, id: impl Into<String>, bytes: Vec<u8>) {
        self.send(Command::TerminalInput {
            id: id.into(),
            bytes,
        });
    }

    pub fn terminal_resize(&self, id: impl Into<String>, cols: u16, rows: u16) {
        self.send(Command::TerminalResize {
            id: id.into(),
            cols,
            rows,
        });
    }

    pub fn stop_terminal(&self, id: impl Into<String>) {
        self.send(Command::StopTerminal(id.into()));
    }

    pub fn has_terminal(&self, id: &str) -> bool {
        self.inner
            .terminals
            .lock()
            .map(|terminals| terminals.contains_key(id))
            .unwrap_or(false)
    }

    pub fn import_external(&self, request: crate::imports::Request) {
        self.send(Command::ImportExternal(request));
    }
    pub fn detach_entry(&self, request: crate::detach::Request) {
        self.send(Command::DetachEntry(request));
    }

    pub fn load_catalog(&self, session_id: impl Into<String>) {
        self.send(Command::LoadCatalog {
            session_id: session_id.into(),
        });
    }

    pub fn load_provider_usage(&self, request_id: u64) {
        self.send(Command::LoadProviderUsage { request_id });
    }

    pub fn load_integrations(&self, request_id: u64, session_id: impl Into<String>) {
        self.send(Command::LoadIntegrations {
            request_id,
            session_id: session_id.into(),
        });
    }

    pub fn load_providers(&self, request_id: u64, session_id: impl Into<String>) {
        self.send(Command::LoadProviders {
            request_id,
            session_id: session_id.into(),
        });
    }

    pub fn connect_provider_key(&self, request: ProviderKeyConnection) {
        self.send(Command::ConnectProviderKey(request));
    }

    pub fn remove_provider_credential(&self, request: ProviderCredentialRemoval) {
        self.send(Command::RemoveProviderCredential(request));
    }
    pub fn start_provider_oauth(&self, request: crate::ProviderOAuthRequest) {
        self.send(Command::StartProviderOAuth(request));
    }
    pub fn complete_provider_oauth(&self, flow_id: u64, code: String) {
        self.send(Command::ProviderOAuthCode { flow_id, code });
    }
    pub fn cancel_provider_oauth(&self, flow_id: u64) {
        self.send(Command::CancelProviderOAuth(flow_id));
    }

    pub fn switch_model(&self, session_id: impl Into<String>, model: ModelInfo) {
        self.send(Command::SwitchModel {
            session_id: session_id.into(),
            model,
        });
    }

    pub fn switch_agent(&self, session_id: impl Into<String>, agent_id: impl Into<String>) {
        self.send(Command::SwitchAgent {
            session_id: session_id.into(),
            agent_id: agent_id.into(),
        });
    }

    pub fn refresh_inbox(&self, session_id: impl Into<String>) {
        self.send(Command::RefreshInbox {
            session_id: session_id.into(),
        });
    }

    pub fn cancel_inbox(&self, session_id: impl Into<String>, inbox_id: impl Into<String>) {
        self.send(Command::CancelInbox {
            session_id: session_id.into(),
            inbox_id: inbox_id.into(),
        });
    }

    pub fn steer_inbox(&self, session_id: impl Into<String>, inbox_id: impl Into<String>) {
        self.send(Command::SteerInbox {
            session_id: session_id.into(),
            inbox_id: inbox_id.into(),
        });
    }

    pub fn refresh_forms(&self, session_id: impl Into<String>) {
        self.send(Command::RefreshForms {
            session_id: session_id.into(),
        });
    }

    pub fn reply_form(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        form_session_id: impl Into<String>,
        form_id: impl Into<String>,
        answers: serde_json::Map<String, Value>,
    ) {
        self.send(Command::ReplyForm {
            request_id,
            session_id: session_id.into(),
            form_session_id: form_session_id.into(),
            form_id: form_id.into(),
            answers,
        });
    }

    pub fn cancel_form(
        &self,
        request_id: u64,
        session_id: impl Into<String>,
        form_session_id: impl Into<String>,
        form_id: impl Into<String>,
    ) {
        self.send(Command::CancelForm {
            request_id,
            session_id: session_id.into(),
            form_session_id: form_session_id.into(),
            form_id: form_id.into(),
        });
    }

    pub fn forms(&self, session_id: &str) -> Vec<FormRequest> {
        self.inner
            .forms
            .lock()
            .expect("forms lock")
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_sass_watch(&self, session_id: impl Into<String>, enabled: bool) {
        self.send(Command::SassWatch {
            session_id: session_id.into(),
            enabled,
            reply: None,
        });
    }

    pub async fn update_sass_watch(&self, session_id: String, enabled: bool) -> Result<crate::SassWatchState, String> {
        let (reply, receiver) = tokio::sync::oneshot::channel();
        self.send(Command::SassWatch { session_id, enabled, reply: Some(reply) });
        receiver.await.map_err(|_| "Vela stopped before updating Sass watching".to_owned())?
    }

    pub fn sass_watch_enabled(&self, session_id: &str) -> bool {
        self.inner
            .sass_watches
            .lock()
            .expect("sass watches lock")
            .enabled(session_id)
    }
    pub fn sass_watch_lines(&self, session_id: &str) -> Vec<SassLine> {
        self.inner
            .sass_watches
            .lock()
            .expect("sass watches lock")
            .lines(session_id)
    }
    pub fn sass_watch_state(&self, session_id: &str) -> crate::SassWatchState {
        self.inner
            .sass_watches
            .lock()
            .expect("sass watches lock")
            .state(session_id)
    }

    pub fn selection(&self, session_id: &str) -> (Option<ModelInfo>, Option<String>) {
        self.read(|store| {
            store
                .views
                .get(session_id)
                .map(|view| view.selection())
                .unwrap_or((None, None))
        })
    }

    pub fn shutdown(&self) {
        self.inner.cancel.cancel();
        self.inner.observations.close();
        kill_all_terminals(&self.inner);
        #[cfg(unix)]
        self.inner.preview_processes.close();
        self.send(Command::Shutdown);
    }

    pub async fn shutdown_and_wait(&self) {
        self.shutdown();
        self.inner.stopped.cancelled().await;
    }

    pub fn observation_is_current(&self, directory: &str, generation: u64) -> bool {
        self.inner.observations.is_current(directory, generation)
    }

    pub fn observed_files(&self, directory: &str) -> Arc<Vec<crate::observation::ObservedFile>> {
        self.inner.observations.snapshot(directory)
    }

    fn send(&self, command: Command) {
        let _ = self.inner.commands.send(command);
    }

    fn read<R>(&self, read: impl FnOnce(&Store) -> R) -> R {
        let store = self.inner.store.read().expect("store lock");
        read(&store)
    }
}

async fn run(inner: Arc<Inner>, mut commands: mpsc::UnboundedReceiver<Command>) {
    let changed = inner.clone();
    let previews = crate::preview::PreviewManager::new(
        crate::preview::PreviewProgram::discover(),
        inner.cancel.child_token(),
        move |servers| {
            changed.store.write().expect("store lock").preview_servers = servers;
            changed.emit(EngineUpdate::Previews);
        },
    );
    #[cfg(unix)]
    let previews = previews.with_processes(inner.preview_processes.clone());
    let program = inner
        .config
        .opencode_binary
        .clone()
        .unwrap_or_else(|| PathBuf::from(crate::runtime::executable()));
    let (connect_tx, mut connect_rx) = mpsc::channel(1);
    let connect_cancel = inner.cancel.child_token();
    {
        let cancel = connect_cancel.clone();
        let service_file = inner.config.service_file.clone();
        tokio::spawn(connect_loop(
            connect_tx,
            cancel,
            service_file,
            program.clone(),
        ));
    }

    let mut client: Option<Client> = None;
    let mut service_file = inner
        .config
        .service_file
        .clone()
        .unwrap_or_else(vela_opencode::service::service_file);
    let mut runtime_operation = None;
    let mut session_deletions = HashMap::new();
    let mut connecting = true;
    let mut reconnecting = false;
    let mut reconnect_attempt = 0u64;
    let mut event_cancel = inner.cancel.child_token();
    let mut oauth_flows: HashMap<u64, crate::provider_oauth::Lease> = HashMap::new();
    let mut events: Option<mpsc::Receiver<Value>> = None;
    let mut pending: Vec<Value> = Vec::new();
    let mut flush = tokio::time::interval(Duration::from_millis(16));
    flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut settlement = tokio::time::interval(Duration::from_secs(1));
    settlement.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut stream_activity: HashMap<String, Instant> = HashMap::new();

    loop {
        tokio::select! {
            biased;
            _ = inner.cancel.cancelled() => break,
            command = commands.recv() => {
                let Some(command) = command else { break };
                match command {
                    Command::RetainEditorWorkspace {session_id,directory} => {
                        if inner.read_view(&session_id).is_some_and(|view|view.summary().directory==directory) {
                            if let Err(error)=inner.observations.retain_editor(&session_id,&directory) {set_error(&inner,&error);}
                        } else {set_error(&inner,"The workspace changed before opening this editor");}
                    }
                    Command::ReleaseEditorWorkspace(directory) => inner.observations.release_editor(&directory),
                    Command::WriteDocument(request) => {
                        let request_id=request.id;let rel=request.key.rel.clone();let directory=request.key.directory.clone();
                        if std::path::Path::new(&rel).is_absolute() {
                            let content=request.text.clone();let expected=request.expected.clone();let overwrite=request.overwrite;
                            let inner=inner.clone();let path=rel.clone();
                            tokio::spawn(async move {
                                let result=tokio::task::spawn_blocking(move||crate::files::write_standalone(&path,&content,&expected,overwrite)).await;
                                let result=match result {Ok(result)=>result,Err(error)=>return inner.emit(EngineUpdate::FileSaved{request_id,rel,error:Some(error.to_string()),conflict:None})};
                                let conflict=match &result {
                                    Err(crate::files::FileError::Changed(_))=>crate::files::read_standalone(&rel).ok().map(|content|ExternalChange{content:Some(content),..Default::default()}),
                                    _=>None,
                                };
                                inner.emit(EngineUpdate::FileSaved{request_id,rel,error:result.err().map(|error|error.to_string()),conflict});
                            });
                        } else if let Err(error)=inner.observations.submit_editor(&directory,move |workspace| {
                            write_file_update(workspace,request.id,request.key.rel,request.text,request.expected,request.overwrite)
                        }) {
                            inner.emit(EngineUpdate::FileSaved {request_id,rel,error:Some(error),conflict:None});
                        }
                    }
                    Command::ReadDocument {request_id,key} => {
                        let rel=key.rel.clone();
                        if std::path::Path::new(&rel).is_absolute() {
                            let inner=inner.clone();let path=rel.clone();
                            tokio::spawn(async move {
                                let result=tokio::task::spawn_blocking(move||crate::files::read_standalone(&path)).await;
                                let deleted=matches!(&result,Ok(Err(crate::files::FileError::Io(error))) if error.kind()==std::io::ErrorKind::NotFound);
                                let result=result.map_err(|error|error.to_string()).and_then(|result|result.map_err(|error|error.to_string()));
                                inner.emit(EngineUpdate::FileContent {
                                    request_id,
                                    rel,
                                    content:result.as_ref().ok().cloned(),
                                    deleted,
                                    error:if deleted{None}else{result.err()},
                                });
                            });
                        } else if let Err(error)=inner.observations.submit_editor(&key.directory,move |workspace| {
                            read_file_update(workspace, request_id, key.rel)
                        }) {
                            inner.emit(EngineUpdate::FileContent {request_id,rel,content:None,deleted:false,error:Some(error)});
                        }
                    }
                    Command::LoadRecovery { session_id } => {
                        spawn_load_recovery(&inner, session_id);
                    }
                    Command::AcknowledgeRecovery { session_id, record_id } => {
                        let Some(root) = session_directory(&inner, &session_id) else {
                            inner.emit(EngineUpdate::RecoveryFailed { session_id, error: "This session has no workspace directory".into() });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let result = tokio::task::spawn_blocking(move || crate::recovery::acknowledge(&root, &record_id))
                                .await
                                .map_err(|error| error.to_string())
                                .and_then(|result| result.map_err(|error| error.to_string()));
                            match result {
                                Ok(()) => spawn_load_recovery(&inner, session_id),
                                Err(error) => inner.emit(EngineUpdate::RecoveryFailed { session_id, error }),
                            }
                        });
                    }
                    Command::TogglePreview { request_id, session_id, entry } => {
                        let Some(root) = inner.read_view(&session_id).map(|view|PathBuf::from(view.summary().directory)) else {
                            inner.emit(EngineUpdate::PreviewFinished { request_id, session_id:Some(session_id), result:Err("Open a workspace to start a Vite preview".into()) });
                            continue;
                        };
                        // Register the scope before launching the worker, so a
                        // queued close also cancels startup that has not run yet.
                        let scope=previews.workspace_scope(&session_id).await;
                        let previews=previews.clone();let inner=inner.clone();
                        tokio::spawn(async move {
                            let result=previews.toggle_in_scope(session_id.clone(),root,entry,scope).await;
                            inner.emit(EngineUpdate::PreviewFinished { request_id,session_id:Some(session_id),result });
                        });
                    }
                    Command::StopPreview { request_id, server_id } => {
                        let previews=previews.clone();let inner=inner.clone();
                        tokio::spawn(async move {
                            if let Some(id)=server_id { previews.stop(&id).await; } else { previews.stop_all().await; }
                            inner.emit(EngineUpdate::PreviewFinished { request_id,session_id:None,result:Ok(None) });
                        });
                    }
                    Command::ManageRuntime { request_id, action } => {
                        if runtime_operation.is_some() {
                            inner.emit(EngineUpdate::RuntimeFinished { request_id, result: Err("An OpenCode update or sync is already in progress.".into()) });
                            continue;
                        }
                        runtime_operation = Some(request_id);
                        let inner = inner.clone();
                        let program = program.clone();
                        let file = service_file.clone();
                        tokio::spawn(async move {
                            let result = tokio::select! {
                                _ = inner.cancel.cancelled() => return,
                                result = crate::runtime::manage(&program, action, file) => result,
                            };
                            let _ = inner.commands.send(Command::RuntimeFinished { request_id, result });
                        });
                    }
                    Command::RuntimeFinished { request_id, result } => {
                        if runtime_operation != Some(request_id) { continue; }
                        runtime_operation = None;
                        reconnecting = false;
                        reconnect_attempt = reconnect_attempt.wrapping_add(1);
                        let result = result.and_then(|synced| {
                            let connected = Client::new(synced.endpoint.clone()).map_err(|error| error.to_string())?;
                            connect_cancel.cancel();
                            connecting = false;
                            event_cancel.cancel();
                            event_cancel = inner.cancel.child_token();
                            pending.clear();
                            for lease in oauth_flows.values() { lease.cancel.cancel(); }
                            oauth_flows.clear();
                            events = Some(vela_opencode::spawn_event_stream(connected.clone(), event_cancel.clone()));
                            client = Some(connected.clone());
                            service_file = synced.service_file;
                            connected_to(&inner, connected, &synced.endpoint);
                            Ok(crate::runtime::RuntimeResult {
                                version: synced.version, previous_version: synced.previous_version, cli_updated: synced.cli_updated,
                            })
                        });
                        inner.emit(EngineUpdate::RuntimeFinished { request_id, result });
                    }
                    Command::ReconnectFinished { attempt, result } => {
                        if attempt != reconnect_attempt { continue; }
                        reconnecting = false;
                        match result {
                            Ok((connected, endpoint, file)) => {
                                info!(url = %endpoint.url, "reconnected to OpenCode");
                                event_cancel.cancel();
                                event_cancel = inner.cancel.child_token();
                                pending.clear();
                                events = Some(vela_opencode::spawn_event_stream(
                                    connected.clone(),
                                    event_cancel.clone(),
                                ));
                                client = Some(connected.clone());
                                service_file = file;
                                connected_to(&inner, connected, &endpoint);
                            }
                            Err(error) => {
                                let mut store = inner.store.write().expect("store lock");
                                if store.connection != ConnectionState::Ready {
                                    store.connection = ConnectionState::Error;
                                    store.last_error = Some(error);
                                    drop(store);
                                    inner.emit(EngineUpdate::Connection);
                                }
                            }
                        }
                    }
                    Command::LoadRuntimeInfo(request_id) => {
                        let inner = inner.clone();
                        let program = program.clone();
                        tokio::spawn(async move {
                            let cli_version = crate::runtime::installed_version_at(&program).await;
                            let tui = cli_version
                                .as_deref()
                                .and_then(vela_opencode::service::normalized_version)
                                .is_some_and(|version| version.split('.').next() == Some("2"));
                            inner.emit(EngineUpdate::RuntimeInfo {
                                request_id,
                                cli_version,
                                capabilities: crate::runtime::RuntimeCapabilities::opencode(tui),
                            });
                        });
                    }
                    Command::ApplySettings(settings) => {
                        let settings = settings.normalized();
                        *inner.settings.write().expect("settings lock") = settings.clone();
                        {
                            let mut store = inner.store.write().expect("store lock");
                            store
                                .warm
                                .set_limits(settings.warm_sessions, settings.warm_bytes);
                        }
                        let directory = inner.config.data_dir.clone();
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let error = match tokio::task::spawn_blocking(move || {
                                settings.save(directory.as_deref())
                            })
                            .await
                            {
                                Ok(Ok(_)) => None,
                                Ok(Err(error)) => Some(error.to_string()),
                                Err(error) => Some(error.to_string()),
                            };
                            inner.emit(EngineUpdate::SettingsApplied { error });
                        });
                    }
                    Command::SaveWindowSize(size) => {
                        if let Some(directory) = inner.config.data_dir.clone() {
                            match tokio::task::spawn_blocking(move || size.save(&directory)).await {
                                Ok(Ok(())) => {},
                                Ok(Err(error)) => tracing::warn!(%error, "could not save session window dimensions"),
                                Err(error) => tracing::warn!(%error, "session window persistence task failed"),
                            }
                        }
                    }
                    Command::StartProviderOAuth(request) => {
                        let flow_id = request.flow_id;
                        if oauth_flows.contains_key(&flow_id) { continue; }
                        let workspace_matches = inner.read_view(&request.session_id).is_some_and(|view| view.summary().directory == request.directory);
                        if let Some(active) = client.clone().filter(|_| workspace_matches) {
                            let commands = inner.commands.clone();
                            let lease = crate::provider_oauth::spawn(active, request, inner.cancel.child_token(), inner.updates.clone(), move || {
                                let _ = commands.send(Command::ProviderOAuthFinished(flow_id));
                            });
                            oauth_flows.insert(flow_id, lease);
                        } else {
                            inner.emit(EngineUpdate::ProviderOAuth { flow_id, session_id: request.session_id, integration_id: request.integration_id,
                                state: crate::ProviderOAuthState::Failed(if workspace_matches { "OpenCode is not connected" } else { "The workspace changed before starting provider authorization." }.into()) });
                        }
                    }
                    Command::ProviderOAuthCode { flow_id, code } => {
                        if let Some(lease) = oauth_flows.get(&flow_id) { let _ = lease.code.send(code); }
                    }
                    Command::CancelProviderOAuth(flow_id) => {
                        if let Some(lease) = oauth_flows.get(&flow_id) { lease.cancel.cancel(); }
                    }
                    Command::ProviderOAuthFinished(flow_id) => { oauth_flows.remove(&flow_id); }
                    Command::SassWatch { session_id, enabled, reply } => {
                        let result = update_sass_watch(&inner, &session_id, enabled);
                        if let Some(reply) = reply {
                            let result = result.map(|()| inner.sass_watches.lock().expect("sass watches lock").state(&session_id));
                            let _ = reply.send(result);
                        }
                    }
                    Command::Shutdown => {
                        kill_all_terminals(&inner);
                        break;
                    }
                    Command::RefreshInbox { session_id } => {
                        if let Some(active) = client.clone() {
                            spawn_inbox_refresh(&inner, active, session_id);
                        }
                    }
                    Command::RefreshForms { session_id } => {
                        if let Some(active) = client.clone() {
                            spawn_forms_refresh(&inner, active, session_id);
                        }
                    }
                    Command::RefreshCommands { session_id } => {
                        let Some(active) = client.clone() else {
                            inner.emit(EngineUpdate::CommandCatalog { session_id, commands: Vec::new(), error: Some("OpenCode is not connected.".into()) });
                            continue;
                        };
                        let Some(directory) = session_directory(&inner, &session_id).and_then(|path| path.to_str().map(str::to_owned)) else {
                            inner.emit(EngineUpdate::CommandCatalog { session_id, commands: Vec::new(), error: Some("This session has no workspace directory".into()) });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            match active.command_catalog(&directory).await {
                                Ok(commands) => inner.emit(EngineUpdate::CommandCatalog { session_id, commands, error: None }),
                                Err(error) => inner.emit(EngineUpdate::CommandCatalog { session_id, commands: Vec::new(), error: Some(error.to_string()) }),
                            }
                        });
                    }
                    Command::FindReferences { request_id, session_id, query } => {
                        let Some(active) = client.clone() else {
                            inner.emit(EngineUpdate::References { request_id, session_id, files: Vec::new(), error: Some("OpenCode is not connected.".into()) });
                            continue;
                        };
                        let Some(directory) = session_directory(&inner, &session_id).and_then(|path| path.to_str().map(str::to_owned)) else {
                            inner.emit(EngineUpdate::References { request_id, session_id, files: Vec::new(), error: Some("This session has no workspace directory".into()) });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            match active.find_files(&directory, &query).await {
                                Ok(files) => inner.emit(EngineUpdate::References { request_id, session_id, files, error: None }),
                                Err(error) => inner.emit(EngineUpdate::References { request_id, session_id, files: Vec::new(), error: Some(error.to_string()) }),
                            }
                        });
                    }
                    Command::Validate { request_id, session_id, path, content } => {
                        let cancel = CancellationToken::new();
                        inner
                            .validation_cancels
                            .lock()
                            .expect("validation cancels lock")
                            .insert(request_id, cancel.clone());
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let result =
                                crate::validation::validate(&path, &content, &cancel).await;
                            inner
                                .validation_cancels
                                .lock()
                                .expect("validation cancels lock")
                                .remove(&request_id);
                            match result {
                                Ok(diagnostics) => inner.emit(EngineUpdate::Validation { request_id, session_id, path, diagnostics, error: None }),
                                Err(error) => inner.emit(EngineUpdate::Validation { request_id, session_id, path, diagnostics: Vec::new(), error: Some(error) }),
                            }
                        });
                    }
                    Command::ValidateScss { request_id, session_id, path, content } => {
                        let Some(root) = session_directory(&inner, &session_id) else {
                            inner.emit(EngineUpdate::Validation {
                                request_id,
                                session_id,
                                path,
                                diagnostics: Vec::new(),
                                error: Some("This session has no workspace directory".into()),
                            });
                            continue;
                        };
                        let cancel = CancellationToken::new();
                        inner
                            .validation_cancels
                            .lock()
                            .expect("validation cancels lock")
                            .insert(request_id, cancel.clone());
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let source = if std::path::Path::new(&path).is_absolute() {
                                std::path::PathBuf::from(&path)
                            } else {
                                root.join(&path)
                            };
                            let result = match crate::sass_compiler::Program::discover() {
                                Ok(program) => {
                                    program
                                        .validate_with_cancel(&root, &source, &content, &cancel)
                                        .await
                                }
                                Err(error) => Err(error),
                            };
                            inner
                                .validation_cancels
                                .lock()
                                .expect("validation cancels lock")
                                .remove(&request_id);
                            match result {
                                Ok(diagnostics) => inner.emit(EngineUpdate::Validation {
                                    request_id,
                                    session_id,
                                    path,
                                    diagnostics,
                                    error: None,
                                }),
                                Err(error) => inner.emit(EngineUpdate::Validation {
                                    request_id,
                                    session_id,
                                    path,
                                    diagnostics: Vec::new(),
                                    error: Some(error),
                                }),
                            }
                        });
                    }
                    Command::CancelValidation { request_id } => {
                        if let Some(cancel) = inner
                            .validation_cancels
                            .lock()
                            .expect("validation cancels lock")
                            .remove(&request_id)
                        {
                            cancel.cancel();
                        }
                    }
                    Command::ReplyForm { request_id, session_id, form_session_id, form_id, answers } => {
                        let known = inner.forms.lock().expect("forms lock").get(&session_id).is_some_and(|forms| forms.iter().any(|form| form.id == form_id && form.session_id == form_session_id));
                        if !known {
                            inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some("form does not belong to the active workspace session".into()) });
                            continue;
                        }
                        let Some(directory) = session_directory(&inner, &session_id).and_then(|path| path.to_str().map(str::to_owned)) else {
                            inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some("This session has no workspace directory".into()) });
                            continue;
                        };
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            tokio::spawn(async move {
                                let header = (form_session_id != session_id).then_some(directory.as_str());
                                let result = active.reply_form(&form_session_id, &form_id, answers, header).await;
                                match result {
                                    Ok(()) => {
                                        inner.emit(EngineUpdate::FormFinished { request_id, session_id: session_id.clone(), form_id, error: None });
                                        spawn_forms_refresh(&inner, active, session_id);
                                    }
                                    Err(error) => inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some(error.to_string()) }),
                                }
                            });
                        } else {
                            inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some("OpenCode is not connected.".into()) });
                        }
                    }
                    Command::CancelForm { request_id, session_id, form_session_id, form_id } => {
                        let known = inner.forms.lock().expect("forms lock").get(&session_id).is_some_and(|forms| forms.iter().any(|form| form.id == form_id && form.session_id == form_session_id));
                        if !known {
                            inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some("form does not belong to the active workspace session".into()) });
                            continue;
                        }
                        let Some(directory) = session_directory(&inner, &session_id).and_then(|path| path.to_str().map(str::to_owned)) else {
                            inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some("This session has no workspace directory".into()) });
                            continue;
                        };
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            tokio::spawn(async move {
                                let header = (form_session_id != session_id).then_some(directory.as_str());
                                let result = active.cancel_form(&form_session_id, &form_id, header).await;
                                match result {
                                    Ok(()) => {
                                        inner.emit(EngineUpdate::FormFinished { request_id, session_id: session_id.clone(), form_id, error: None });
                                        spawn_forms_refresh(&inner, active, session_id);
                                    }
                                    Err(error) => inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some(error.to_string()) }),
                                }
                            });
                        } else {
                            inner.emit(EngineUpdate::FormFinished { request_id, session_id, form_id, error: Some("OpenCode is not connected.".into()) });
                        }
                    }
                    Command::CancelInbox { session_id, inbox_id } => {
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            tokio::spawn(async move {
                                match active.cancel_inbox(&session_id, &inbox_id).await {
                                    Ok(()) => spawn_inbox_refresh(&inner, active, session_id),
                                    Err(error) => set_error(&inner, &error.to_string()),
                                }
                            });
                        }
                    }
                    Command::SteerInbox { session_id, inbox_id } => {
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            tokio::spawn(async move {
                                match active.steer_inbox(&session_id, &inbox_id).await {
                                    Ok(()) => spawn_inbox_refresh(&inner, active, session_id),
                                    Err(error) => set_error(&inner, &error.to_string()),
                                }
                            });
                        }
                    }
                    Command::LoadCatalog { session_id } => {
                        if let Some(active) = client.clone() {
                            spawn_catalog(&inner, active, session_id);
                        }
                    }
                    Command::LoadProviderUsage { request_id } => {
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let results = tokio::task::spawn_blocking(
                                crate::provider_usage::read_default_provider_usage_snapshot,
                            )
                            .await
                            .unwrap_or_default();
                            inner.emit(EngineUpdate::ProviderUsage {
                                request_id,
                                results,
                            });
                        });
                    }
                    Command::LoadIntegrations {
                        request_id,
                        session_id,
                    } => {
                        let Some(directory) =
                            session_directory(&inner, &session_id).and_then(|path| {
                                path.to_str().map(str::to_owned)
                            })
                        else {
                            inner.emit(EngineUpdate::Integrations {
                                request_id,
                                session_id,
                                mcp_servers: Vec::new(),
                                plugins: Vec::new(),
                                skills: Vec::new(),
                                error: Some(
                                    "This session has no workspace directory".into(),
                                ),
                            });
                            continue;
                        };
                        let Some(active) = client.clone() else {
                            inner.emit(EngineUpdate::Integrations {
                                request_id,
                                session_id,
                                mcp_servers: Vec::new(),
                                plugins: Vec::new(),
                                skills: Vec::new(),
                                error: Some("OpenCode is not connected.".into()),
                            });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let result = async {
                                let mcp_servers = active
                                    .list_mcp_servers(&directory)
                                    .await
                                    .map_err(|error| error.to_string())?;
                                let plugins = active
                                    .list_plugins(&directory)
                                    .await
                                    .map_err(|error| error.to_string())?;
                                let skills = active
                                    .list_skill_inventory(&directory)
                                    .await
                                    .map_err(|error| error.to_string())?;
                                Ok::<_, String>((mcp_servers, plugins, skills))
                            }
                            .await;
                            let (mcp_servers, plugins, skills, error) = match result {
                                Ok((mcp_servers, plugins, skills)) => {
                                    (mcp_servers, plugins, skills, None)
                                }
                                Err(error) => (
                                    Vec::new(),
                                    Vec::new(),
                                    Vec::new(),
                                    Some(error),
                                ),
                            };
                            inner.emit(EngineUpdate::Integrations {
                                request_id,
                                session_id,
                                mcp_servers,
                                plugins,
                                skills,
                                error,
                            });
                        });
                    }
                    Command::ConnectProviderKey(request) => {
                        let inner = inner.clone();
                        let active = client.clone();
                        let workspace_matches = inner.read_view(&request.session_id).is_some_and(|view| view.summary().directory == request.directory);
                        tokio::spawn(async move {
                            let error = match (active, workspace_matches) {
                                (Some(active), true) => active.connect_provider_key(&request.directory, &request.integration_id, request.key, request.label, request.answers).await.err().map(|e| e.to_string()),
                                (None, _) => Some("OpenCode is not connected".into()),
                                (_, false) => Some("The workspace changed before connecting the provider.".into()),
                            };
                            inner.emit(EngineUpdate::ProviderConnected { request_id: request.request_id, session_id: request.session_id, integration_id: request.integration_id, error });
                        });
                    }
                    Command::RemoveProviderCredential(request) => {
                        let inner = inner.clone();
                        let active = client.clone();
                        let workspace_matches = inner.read_view(&request.session_id).is_some_and(|view| view.summary().directory == request.directory);
                        tokio::spawn(async move {
                            let error = match (active, workspace_matches) {
                                (Some(active), true) => active.remove_provider_credential(&request.directory, &request.integration_id, &request.credential_id).await.err().map(|e| e.to_string()),
                                (None, _) => Some("OpenCode is not connected".into()),
                                (_, false) => Some("The workspace changed before removing the provider credential.".into()),
                            };
                            inner.emit(EngineUpdate::ProviderCredentialRemoved { request_id: request.request_id, session_id: request.session_id, integration_id: request.integration_id, credential_id: request.credential_id, error });
                        });
                    }
                    Command::LoadProviders { request_id, session_id } => {
                        let inner = inner.clone();
                        let active = client.clone();
                        let directory = inner.read_view(&session_id).map(|view| view.summary().directory).filter(|d| !d.is_empty());
                        tokio::spawn(async move {
                            let result = match (active, directory) {
                                (Some(active), Some(directory)) => active.list_provider_integrations(&directory).await.map_err(|e| e.to_string()),
                                (None, _) => Err("OpenCode is not connected".to_owned()),
                                (_, None) => Err("Open a workspace to connect model providers.".to_owned()),
                            };
                            let (providers, error) = match result { Ok(providers) => (providers, None), Err(error) => (Vec::new(), Some(error)) };
                            inner.emit(EngineUpdate::Providers { request_id, session_id, providers, error });
                        });
                    }
                    Command::SwitchModel { session_id, model } => {
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            let chosen = model.clone();
                            tokio::spawn(async move {
                                match active.switch_model(&session_id, &chosen).await {
                                    Ok(()) => {
                                        if let Some(view) = inner.read_view(&session_id) {
                                            view.set_selection(Some(chosen.clone()), None);
                                        }
                                        inner.emit(EngineUpdate::Selection {
                                            session_id,
                                            model: Some(chosen),
                                            agent: None,
                                        });
                                    }
                                    Err(error) => set_error(&inner, &error.to_string()),
                                }
                            });
                        }
                    }
                    Command::SwitchAgent { session_id, agent_id } => {
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            let chosen = agent_id.clone();
                            tokio::spawn(async move {
                                match active.switch_agent(&session_id, &chosen).await {
                                    Ok(()) => {
                                        if let Some(view) = inner.read_view(&session_id) {
                                            view.set_agent(chosen.clone());
                                        }
                                        inner.emit(EngineUpdate::Selection {
                                            session_id,
                                            model: None,
                                            agent: Some(chosen),
                                        });
                                    }
                                    Err(error) => set_error(&inner, &error.to_string()),
                                }
                            });
                        }
                    }
                    Command::StartTerminal { id, cwd, cols, rows } => {
                        start_terminal(&inner, id, cwd, cols, rows);
                    }
                    Command::TerminalExec(command) => {
                        start_terminal_command(&inner, command);
                    }
                    Command::TerminalInput { id, bytes } => {
                        if bytes.len()>vela_terminal::input::MAX_INPUT_BYTES {
                            set_error(&inner,"invalid or oversized terminal input");continue;
                        }
                        let session = inner
                            .terminals
                            .lock()
                            .ok()
                            .and_then(|terminals| terminals.get(&id).cloned());
                        if let Some(session) = session
                            && let Err(error) = session.write(&bytes)
                        {
                            set_error(&inner, &format!("Terminal input failed: {error}"));
                        }
                    }
                    Command::TerminalResize { id, cols, rows } => {
                        let session = inner
                            .terminals
                            .lock()
                            .ok()
                            .and_then(|terminals| terminals.get(&id).cloned());
                        if let Some(session) = session {
                            let _ = session.resize(cols, rows);
                        }
                    }
                    Command::StopTerminal(id) => {
                        if let Ok(mut terminals) = inner.terminals.lock()
                            && let Some(session) = terminals.remove(&id)
                        {
                            session.kill();
                        }
                    }
                    Command::ListDir { request_id, session_id, rel } => {
                        spawn_file_job(&inner, session_id, move |root| {
                            let result = crate::files::list_dir(&root, &rel);
                            match result {
                                Ok(entries) => EngineUpdate::Directory {
                                    request_id,
                                    rel,
                                    entries,
                                    error: None,
                                },
                                Err(error) => EngineUpdate::Directory {
                                    request_id,
                                    rel,
                                    entries: Vec::new(),
                                    error: Some(error.to_string()),
                                },
                            }
                        });
                    }
                    Command::DetachEntry(request) => {
                        let valid_owner=inner.read_view(&request.session).is_some_and(|view|view.summary().directory==request.workspace);
                        if !valid_owner {inner.emit(EngineUpdate::EntryDetached {request,path:None,error:Some("The workspace changed before detaching this entry.".into())});continue;}
                        let job_request=request.clone();
                        let result=inner.observations.submit(&request.session,&request.workspace,move|workspace| {
                            let result=workspace.detach_entry(&job_request.rel);
                            EngineUpdate::EntryDetached {request:job_request,path:result.as_ref().ok().cloned(),error:result.err().map(|error|error.to_string())}
                        });
                        if let Err(error)=result {inner.emit(EngineUpdate::EntryDetached {request,path:None,error:Some(error)});}
                    }
                    Command::ImportExternal(request) => {
                        let valid_owner=inner.read_view(&request.session).is_some_and(|view|view.summary().directory==request.workspace);
                        if !valid_owner {inner.emit(EngineUpdate::EntriesImported{request,outcomes:Vec::new(),error:Some("The workspace changed before importing these entries.".into())});continue;}
                        let job_request=request.clone();
                        let result=inner.observations.submit(&request.session,&request.workspace,move|workspace| {
                            let result=workspace.import_external(&job_request.parent,&job_request.sources);
                            EngineUpdate::EntriesImported {request:job_request,outcomes:result.as_ref().ok().cloned().unwrap_or_default(),error:result.err().map(|error|error.to_string())}
                        });
                        if let Err(error)=result {inner.emit(EngineUpdate::EntriesImported {request,outcomes:Vec::new(),error:Some(error)});}
                    }
                    Command::ResolveEntry {request_id,session,workspace,rel,directory} => {
                        let valid_owner=inner.read_view(&session).is_some_and(|view|view.summary().directory==workspace);
                        if !valid_owner {inner.emit(EngineUpdate::EntryResolved {request_id,path:None,error:Some("The workspace changed before revealing this entry.".into())});continue;}
                        let result=inner.observations.submit(&session,&workspace,move|workspace| {
                            let result=crate::files::resolve_entry(workspace.root(),&rel).and_then(|path| {
                                if directory && !path.is_dir() {Err(crate::files::FileError::Invalid("terminal path is not a directory".into()))}else{Ok(path)}
                            });
                            EngineUpdate::EntryResolved {request_id,path:result.as_ref().ok().cloned(),error:result.err().map(|error|error.to_string())}
                        });
                        if let Err(error)=result {inner.emit(EngineUpdate::EntryResolved {request_id,path:None,error:Some(error)});}
                    }
                    Command::RemoveEntry(request) => {
                        let valid_owner = inner.read_view(&request.session).is_some_and(|view| view.summary().directory == request.workspace);
                        if !valid_owner {
                            inner.emit(EngineUpdate::EntryRemoved {request,error:Some("The workspace changed before removing this entry.".into())}); continue;
                        }
                        let job_request=request.clone();
                        let result=inner.observations.submit(&request.session,&request.workspace,move |workspace| {
                            let error=workspace.remove_entry(&job_request.rel).err().map(|error|error.to_string());
                            EngineUpdate::EntryRemoved {request:job_request,error}
                        });
                        if let Err(error)=result {inner.emit(EngineUpdate::EntryRemoved {request,error:Some(error)});}
                    }
                    Command::RenameEntry(request) => {
                        let valid_owner = inner.read_view(&request.session).is_some_and(|view| view.summary().directory == request.workspace);
                        if !valid_owner {
                            inner.emit(EngineUpdate::EntryRenamed { request, new_rel: None, snapshots: Vec::new(), error: Some("The workspace changed before renaming this entry.".into()) });
                            continue;
                        }
                        let job_request = request.clone();
                        let result = inner.observations.submit(&request.session, &request.workspace, move |workspace| {
                            match workspace.relocate_entry(&job_request.rel, &job_request.name, job_request.target_parent.as_deref()) {
                                Ok((new_rel,snapshots))=>EngineUpdate::EntryRenamed {request:job_request,new_rel:Some(new_rel),snapshots,error:None},
                                Err(error)=>EngineUpdate::EntryRenamed {request:job_request,new_rel:None,snapshots:Vec::new(),error:Some(error.to_string())},
                            }
                        });
                        if let Err(error) = result { inner.emit(EngineUpdate::EntryRenamed { request, new_rel: None, snapshots: Vec::new(), error: Some(error) }); }
                    }
                    Command::CreateEntry(request) => {
                        let valid_owner = inner.read_view(&request.session).is_some_and(|view| view.summary().directory == request.workspace);
                        if !valid_owner {
                            inner.emit(EngineUpdate::EntryCreated { request, rel: None, error: Some("The workspace changed before creating this entry.".into()) });
                            continue;
                        }
                        let job_request = request.clone();
                        let result = inner.observations.submit(&request.session, &request.workspace, move |workspace| {
                            let result = crate::files::create_entry(workspace.root(), &job_request.parent, &job_request.name, job_request.is_dir);
                            EngineUpdate::EntryCreated { request: job_request, rel: result.as_ref().ok().cloned(), error: result.err().map(|error| error.to_string()) }
                        });
                        if let Err(error) = result { inner.emit(EngineUpdate::EntryCreated { request, rel: None, error: Some(error) }); }
                    }
                    Command::ReadFile { request_id, session_id, rel } => {
                        spawn_workspace_job(&inner, session_id, move |workspace| {
                            read_file_update(workspace, request_id, rel)
                        });
                    }
                    Command::WriteFile { request_id, session_id, rel, content, expected, overwrite } => {
                        spawn_workspace_job(&inner, session_id, move |workspace| {
                            write_file_update(workspace,request_id,rel,content,expected,overwrite)
                        });
                    }
                    Command::GitStatus { request_id, session_id } => {
                        spawn_file_job(&inner, session_id, move |root| {
                            let result = crate::git::status(&root, "");
                            match result {
                                Ok(entries) => EngineUpdate::GitStatus {
                                    request_id,
                                    entries,
                                    error: None,
                                },
                                Err(error) => EngineUpdate::GitStatus {
                                    request_id,
                                    entries: Vec::new(),
                                    error: Some(error.to_string()),
                                },
                            }
                        });
                    }
                    Command::GitDiff { request_id, session_id, rel } => {
                        spawn_file_job(&inner, session_id, move |root| {
                            let result = crate::git::diff(&root, &rel);
                            let full = std::fs::read_to_string(root.join(&rel)).ok();
                            EngineUpdate::GitDiff {
                                request_id,
                                rel,
                                content: result.as_ref().ok().cloned(),
                                full,
                                error: result.err().map(|error| error.to_string()),
                            }
                        });
                    }
                    Command::OpenDirectory(directory) => {
                        tracing::debug!(%directory, "open directory command");
                        if let Some(active) = client.clone() {
                            spawn_open_directory(&inner, active, directory);
                        } else {
                            set_error(&inner, "OpenCode is not connected yet");
                        }
                    }
                    Command::OpenFileWorkspace { file, reply } => {
                        if let Some(active) = client.clone() {
                            spawn_open_file_workspace(&inner, active, file, reply);
                        } else { let _ = reply.send(Err("OpenCode is not connected yet".into())); }
                    }
                    Command::OpenById(id) => {
                        if inner.read(|store| store.retired_sessions.contains(&id)) { continue; }
                        // Warm navigation: a retained view is promoted and shown
                        // immediately. Network reconciliation happens in the
                        // background and never gates the first frame.
                        if let Some(view) = inner.read_view(&id) {
                            view.touch();
                            {
                                let mut store = inner.store.write().expect("store lock");
                                store.active_session = Some(id.clone());
                            }
                            inner.emit(EngineUpdate::Session(id.clone()));
                        } else {
                            let promoted = {
                                let mut store = inner.store.write().expect("store lock");
                                store.warm.remove(&id).map(|entry| {
                                    let view = std::sync::Arc::new(SessionView::from_warm(entry));
                                    store.insert_view(view.clone());
                                    view
                                })
                            };
                            if promoted.is_none() {
                                let summary = inner.read(|store| {
                                    store.sessions.iter().find(|item| item.id == id).cloned()
                                });
                                if let Some(summary) = summary {
                                    let view = std::sync::Arc::new(SessionView::new(summary));
                                    let mut store = inner.store.write().expect("store lock");
                                    store.insert_view(view);
                                }
                            }
                            inner.emit(EngineUpdate::Session(id.clone()));
                            if let Some(active) = client.clone() {
                                spawn_open(&inner, active, id.clone());
                            } else {
                                set_error(&inner, "OpenCode is not connected yet");
                            }
                        }
                        if let Some(view) = inner.read_view(&id) {
                            register_sass_context(&inner, &view.summary());
                        }
                        evict_views(&inner);
                    }
                    Command::DeleteSession { request_id, session_id } => {
                        if session_deletions.contains_key(&session_id) {
                            inner.emit(EngineUpdate::SessionDeleteFinished { request_id, session_id, error: Some("Session deletion is already in progress.".into()) });
                            continue;
                        }
                        session_deletions.insert(session_id.clone(), request_id);
                        let inner = inner.clone();
                        let active = client.clone();
                        tokio::spawn(async move {
                            let error = if inner.read(|store| store.retired_sessions.contains(&session_id)) { None }
                            else if let Some(active) = active { active.delete_session(&session_id).await.err().map(|error| error.to_string()) }
                            else { Some("OpenCode is not connected.".into()) };
                            let _ = inner.commands.send(Command::SessionDeletionFinished { request_id, session_id, error });
                        });
                    }
                    Command::SessionDeletionFinished { request_id, session_id, error } => {
                        if session_deletions.get(&session_id) != Some(&request_id) { continue; }
                        session_deletions.remove(&session_id);
                        if error.is_none() {
                            retire_session(&inner, &session_id);
                            stream_activity.remove(&session_id);
                        }
                        inner.emit(EngineUpdate::SessionDeleteFinished { request_id, session_id, error });
                    }
                    Command::RefreshSessions => {
                        if let Some(active) = client.clone() {
                            spawn_refresh_sessions(&inner, active);
                        }
                    }
                    Command::Loaded { session_id, generation, result } => {
                        if inner.read(|store| store.retired_sessions.contains(&session_id)) { continue; }
                        let current = {
                            let mut loads = inner.session_loads.lock().expect("session loads lock");
                            let current = loads.get(&session_id) == Some(&generation);
                            if current { loads.remove(&session_id); }
                            current
                        };
                        if !current { continue; }
                        match result {
                            Ok((summary, messages)) => {
                                {
                                    let mut store = inner.store.write().expect("store lock");
                                    store.last_error = None;
                                }
                                let view = ensure_view(&inner, summary);
                                let hydrated = Transcript::from_messages_json(&messages);
                                let count = hydrated.messages().len();
                                stream_activity.remove(&session_id);
                                view.update_transcript(|transcript| {
                                    transcript.hydrate(hydrated.messages().to_vec());
                                });
                                tracing::debug!(session = %session_id, messages = count, "hydrated session");
                                evict_views(&inner);
                                inner.emit(EngineUpdate::Session(session_id));
                            }
                            Err(error) => {
                                set_error(&inner, &error);
                                if let Some(view) = inner.read_view(&session_id) {
                                    view.update_transcript(|transcript| {
                                        transcript.apply(&AgentEvent::Notice {
                                            session_id: session_id.clone(),
                                            text: format!("Could not load conversation: {error}"),
                                            error: true,
                                            at: now(),
                                        });
                                    });
                                }
                                inner.emit(EngineUpdate::Session(session_id));
                            }
                        }
                    }
                    Command::RemoveQueued { directory, session_id, id } => {
                        let result = inner.message_queues.lock().expect("message queue lock")
                            .remove(&directory, &session_id, &id);
                        if let Err(error) = result { set_error(&inner, &format!("Could not save queued messages: {error}")); }
                        inner.emit(EngineUpdate::LocalQueue(session_id));
                    }
                    Command::ReorderQueued { directory, session_id, from, to } => {
                        let result = inner.message_queues.lock().expect("message queue lock")
                            .reorder(&directory, &session_id, &from, &to);
                        if let Err(error) = result { set_error(&inner, &format!("Could not save queued messages: {error}")); }
                        inner.emit(EngineUpdate::LocalQueue(session_id));
                    }
                    Command::RetryQueued { directory, session_id, id } => {
                        if inner.read(|store| store.retired_sessions.contains(&session_id)) { continue; }
                        let item = inner.message_queues.lock().expect("message queue lock")
                            .claim(&directory, &session_id, &id);
                        if let Some(item) = item {
                            let _ = inner.commands.send(Command::Prompt {
                                session_id: session_id.clone(), text: item.text, files: item.files,
                                delivery: None, retry: Some((directory, id)),
                            });
                            inner.emit(EngineUpdate::LocalQueue(session_id));
                        }
                    }
                    Command::Prompt { session_id, text, files, delivery, retry } => {
                        if inner.read(|store| store.retired_sessions.contains(&session_id)) { continue; }
                        let directory = retry.as_ref().map(|(directory, _)| directory.clone())
                            .or_else(|| inner.read_view(&session_id).map(|view| view.summary().directory));
                        // Vela appends the submitted prompt before awaiting
                        // either immediate or queued delivery. Canonical replay
                        // reconciles this optimistic user row by text.
                        if let Some(view) = inner.read_view(&session_id) {
                            let local_id = format!("local:{}", uuid::Uuid::new_v4());
                            let attachments = files
                                .iter()
                                .filter_map(|file| {
                                    file.name.clone().or_else(|| {
                                        std::path::Path::new(&file.path)
                                            .file_name()
                                            .map(|name| name.to_string_lossy().into_owned())
                                    })
                                })
                                .collect::<Vec<_>>();
                            view.update_transcript(|transcript| {
                                transcript.upsert_local_user(
                                    local_id,
                                    text.clone(),
                                    now(),
                                    attachments,
                                );
                            });
                            inner.emit(EngineUpdate::Session(session_id.clone()));
                        }
                        {
                            let active = client.clone();
                            let inner = inner.clone();
                            let session_id = session_id.clone();
                            tokio::spawn(async move {
                                let result = match &active {
                                    Some(active) => active.prompt(&session_id, &text, &files, delivery.as_deref())
                                        .await.map_err(|error| error.to_string()),
                                    None => Err("OpenCode is not connected yet".into()),
                                };
                                match result {
                                    Ok(_) => {
                                        if let Some((directory, id)) = &retry {
                                            let saved = inner.message_queues.lock().expect("message queue lock")
                                                .remove(directory, &session_id, id);
                                            if let Err(error) = saved { set_error(&inner, &format!("Could not save queued messages: {error}")); }
                                            inner.emit(EngineUpdate::LocalQueue(session_id.clone()));
                                        }
                                        if delivery.is_some() && let Some(active) = active {
                                            spawn_inbox_refresh(&inner, active, session_id);
                                        }
                                    }
                                    Err(error) => {
                                        if let Some((directory, id)) = &retry {
                                            let saved = inner.message_queues.lock().expect("message queue lock")
                                                .retry_failed(directory, &session_id, id, now());
                                            if let Err(error) = saved { set_error(&inner, &format!("Could not save queued messages: {error}")); }
                                            inner.emit(EngineUpdate::LocalQueue(session_id.clone()));
                                        } else if delivery.as_deref() == Some("queue") && let Some(directory) = directory {
                                            let saved = inner.message_queues.lock().expect("message queue lock")
                                                .enqueue(&directory, &session_id, text.clone(), files, now());
                                            if let Err(error) = saved { set_error(&inner, &format!("Queued locally in memory, but could not save: {error}")); }
                                            inner.emit(EngineUpdate::LocalQueue(session_id.clone()));
                                        }
                                        if let Some(view) = inner.read_view(&session_id) {
                                            view.update_transcript(|transcript| {
                                                transcript.apply(&AgentEvent::Notice {
                                                    session_id: session_id.clone(),
                                                    text: format!("Prompt failed: {error}"),
                                                    error: true,
                                                    at: now(),
                                                });
                                            });
                                        }
                                        inner.emit(EngineUpdate::Session(session_id.clone()));
                                        set_error(&inner, &error);
                                    }
                                }
                            });
                        }
                    }
                    Command::Interrupt(session_id) => {
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            tokio::spawn(async move {
                                if let Err(error) = active.interrupt(&session_id).await {
                                    set_error(&inner, &error.to_string());
                                }
                            });
                        }
                    }
                    Command::RevertSession { request_id, session_id, message_id } => {
                        let Some(active) = client.clone() else {
                            inner.emit(EngineUpdate::RevertFinished { request_id, session_id, error: Some("OpenCode is not connected.".into()) });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let mut staged = false;
                            let result = async {
                                active.stage_revert(&session_id, &message_id, true).await.map_err(|error| error.to_string())?;
                                staged = true;
                                active.commit_revert(&session_id).await.map_err(|error| error.to_string())
                            }.await;
                            match result {
                                Ok(()) => {
                                    inner.emit(EngineUpdate::RevertFinished { request_id, session_id: session_id.clone(), error: None });
                                    spawn_open(&inner, active, session_id);
                                }
                                Err(error) => {
                                    if staged {
                                        inner.emit(EngineUpdate::RevertStaged { session_id: session_id.clone() });
                                    }
                                    inner.emit(EngineUpdate::RevertFinished { request_id, session_id, error: Some(error) });
                                }
                            }
                        });
                    }
                    Command::CommitRevert { request_id, session_id } => {
                        let Some(active) = client.clone() else {
                            inner.emit(EngineUpdate::RevertFinished { request_id, session_id, error: Some("OpenCode is not connected.".into()) });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            match active.commit_revert(&session_id).await {
                                Ok(()) => {
                                    inner.emit(EngineUpdate::RevertFinished { request_id, session_id: session_id.clone(), error: None });
                                    spawn_open(&inner, active, session_id);
                                }
                                Err(error) => inner.emit(EngineUpdate::RevertFinished { request_id, session_id, error: Some(error.to_string()) }),
                            }
                        });
                    }
                    Command::ClearRevert { request_id, session_id } => {
                        let Some(active) = client.clone() else {
                            inner.emit(EngineUpdate::RevertFinished { request_id, session_id, error: Some("OpenCode is not connected.".into()) });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let error = active.clear_revert(&session_id).await.err().map(|error| error.to_string());
                            inner.emit(EngineUpdate::RevertFinished { request_id, session_id, error });
                        });
                    }
                    Command::Slash { request_id, session_id, name, args } => {
                        let Some(active) = client.clone() else {
                            inner.emit(EngineUpdate::SlashCommandFinished { request_id, session_id, error: Some("OpenCode is not connected.".into()) });
                            continue;
                        };
                        let Some(directory) = session_directory(&inner, &session_id).and_then(|path| path.to_str().map(str::to_owned)) else {
                            inner.emit(EngineUpdate::SlashCommandFinished { request_id, session_id, error: Some("This session has no workspace directory".into()) });
                            continue;
                        };
                        let inner = inner.clone();
                        tokio::spawn(async move {
                            let result = if matches!(name.as_str(), "compact" | "compress") {
                                active.compact(&session_id).await.map_err(|error| error.to_string())
                            } else {
                                let skills = active.list_skills(&directory).await.ok();
                                let skill = skills.and_then(|value| {
                                    value
                                        .get("data")
                                        .and_then(Value::as_array)
                                        .or_else(|| value.as_array())
                                        .and_then(|rows| {
                                            rows.iter().find_map(|row| {
                                                let id = row.get("id").and_then(Value::as_str);
                                                let skill_name = row.get("name").and_then(Value::as_str);
                                                (skill_name == Some(name.as_str()) || id == Some(name.as_str()))
                                                    .then(|| id.unwrap_or(name.as_str()).to_owned())
                                            })
                                        })
                                });
                                if let Some(skill) = skill {
                                    active.activate_skill(&session_id, &skill).await.map_err(|error| error.to_string())
                                } else {
                                    active.run_command(&session_id, &name, &args).await.map_err(|error| error.to_string())
                                }
                            };
                            inner.emit(EngineUpdate::SlashCommandFinished { request_id, session_id, error: result.err() });
                        });
                    }
                    Command::ReplyPermission { session_id, request_id, reply } => {
                        if let Some(active) = client.clone() {
                            let inner = inner.clone();
                            tokio::spawn(async move {
                                if let Err(error) = active
                                    .reply_permission(&session_id, &request_id, reply)
                                    .await
                                {
                                    set_error(&inner, &error.to_string());
                                }
                            });
                        }
                    }
                    Command::RefreshPermissions => {
                        if let Some(active) = client.clone() {
                            spawn_refresh_permissions(&inner, active);
                        }
                    }
                    Command::Close(session_id) => {
                        inner.session_loads.lock().expect("session loads lock").remove(&session_id);
                        inner.sass_watches.lock().expect("sass watches lock").unregister(&session_id);
                        stream_activity.remove(&session_id);
                        previews.stop_workspace(&session_id).await;
                        for lease in oauth_flows.values().filter(|lease| lease.session_id == session_id) { lease.cancel.cancel(); }
                        close_to_warm(&inner, &session_id);
                        inner.emit(EngineUpdate::Sessions);
                    }
                    Command::SetDraft { session_id, draft } => {
                        if let Some(view) = inner.read_view(&session_id) {
                            view.set_draft(draft);
                        }
                    }
                    Command::SetScroll { session_id, top, stick } => {
                        if let Some(view) = inner.read_view(&session_id) {
                            view.set_scroll(top, stick);
                        }
                    }
                }
            }
            connection = connect_rx.recv(), if connecting => {
                connecting = false;
                if let Some((connected, endpoint)) = connection {
                    info!(url = %endpoint.url, "connected to OpenCode");
                    reconnecting = false;
                    reconnect_attempt = reconnect_attempt.wrapping_add(1);
                    events = Some(vela_opencode::spawn_event_stream(connected.clone(), event_cancel.clone()));
                    client = Some(connected.clone());
                    connected_to(&inner, connected, &endpoint);
                } else {
                    let mut store = inner.store.write().expect("store lock");
                    store.connection = ConnectionState::Error;
                    store.last_error = Some("Could not connect to OpenCode V2".into());
                    drop(store);
                    inner.emit(EngineUpdate::Connection);
                }
            }
            raw = next_event(&mut events) => {
                if let Some(raw) = raw {
                    if raw
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| kind == "server.disconnected")
                    {
                        if !reconnecting {
                            reconnecting = true;
                            reconnect_attempt = reconnect_attempt.wrapping_add(1);
                            let attempt = reconnect_attempt;
                            {
                                let mut store = inner.store.write().expect("store lock");
                                store.connection = ConnectionState::Connecting;
                                store.last_error = None;
                                drop(store);
                            }
                            inner.emit(EngineUpdate::Connection);
                            spawn_reconnect(inner.clone(), attempt, service_file.clone());
                        }
                        continue;
                    }
                    let critical = is_critical(&raw);
                    pending.push(raw);
                    if critical {
                        flush_pending(&inner, &mut pending, &mut client, &mut stream_activity);
                    }
                } else {
                    events = None;
                }
            }
            _ = flush.tick(), if !pending.is_empty() => {
                flush_pending(&inner, &mut pending, &mut client, &mut stream_activity);
            }
            _ = settlement.tick() => {
                if let Some(id) = inner.read(|store| store.active_session.clone())
                    && let Some(view) = inner.read_view(&id)
                {
                    let last = stream_activity.entry(id.clone()).or_insert_with(Instant::now);
                    let result = view.update_transcript(|transcript| transcript.settle_quiet(
                        last.elapsed().as_millis().min(u64::MAX as u128) as u64, now()));
                    if let Some(stalled) = result {
                        inner.emit(EngineUpdate::SessionSettled { session_id: id, stalled });
                    }
                }
            }
        }
    }
    kill_all_terminals(&inner);
    previews.stop_all().await;
    info!("engine stopped");
}

fn connected_to(inner: &Arc<Inner>, client: Client, endpoint: &vela_opencode::Endpoint) {
    {
        let mut store = inner.store.write().expect("store lock");
        store.connection = ConnectionState::Ready;
        store.last_error = None;
        store.endpoint = Some(endpoint.url.clone());
    }
    inner.emit(EngineUpdate::Connection);
    spawn_refresh_sessions(inner, client.clone());
    spawn_refresh_permissions(inner, client.clone());
    for id in inner.read(|store| store.views.keys().cloned().collect::<Vec<_>>()) {
        spawn_open(inner, client.clone(), id);
    }
}

async fn next_event(events: &mut Option<mpsc::Receiver<Value>>) -> Option<Value> {
    match events {
        Some(receiver) => receiver.recv().await,
        None => std::future::pending().await,
    }
}

fn flush_pending(
    inner: &Arc<Inner>,
    pending: &mut Vec<Value>,
    client: &mut Option<Client>,
    stream_activity: &mut HashMap<String, Instant>,
) {
    let batch = std::mem::take(pending);
    let mut touched = HashSet::new();
    let mut sessions_changed = false;
    let mut permissions_changed = false;
    let mut rematerialize = false;
    let mut inbox_refresh = HashSet::new();
    for raw in batch {
        if raw
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.starts_with("session.inbox."))
            && let Some(session_id) = raw.pointer("/data/sessionID").and_then(Value::as_str)
        {
            inbox_refresh.insert(session_id.to_string());
        }
        let Some(event) = parse_event(&raw) else {
            continue;
        };
        if event
            .session_id()
            .is_some_and(|id| inner.read(|store| store.retired_sessions.contains(id)))
        {
            continue;
        }
        if let AgentEvent::ToolCalled {
            session_id, input, ..
        } = &event
            && let Some(root) = session_directory(inner, session_id)
        {
            let input = serde_json::from_str::<Value>(input).unwrap_or(Value::Null);
            let paths = tool_input_file_paths(&input);
            if !paths.is_empty() {
                inner.observations.capture_baselines(
                    session_id,
                    &root.to_string_lossy(),
                    paths,
                );
            }
        }
        match &event {
            AgentEvent::ServerConnected => rematerialize = true,
            AgentEvent::SessionCreated { session } => {
                let mut store = inner.store.write().expect("store lock");
                store.upsert_summary(session.clone());
                sessions_changed = true;
            }
            AgentEvent::SessionRenamed {
                id,
                title,
                updated_at,
            } => {
                let mut store = inner.store.write().expect("store lock");
                if let Some(summary) = store.sessions.iter_mut().find(|item| &item.id == id) {
                    summary.title = title.clone();
                    summary.updated_at = *updated_at;
                }
                sessions_changed = true;
            }
            AgentEvent::SessionDeleted { id } => {
                retire_session(inner, id);
                stream_activity.remove(id);
                touched.remove(id);
                continue;
            }
            AgentEvent::PermissionAsked { request } => {
                let mut store = inner.store.write().expect("store lock");
                let entries = store
                    .permissions
                    .entry(request.session_id.clone())
                    .or_default();
                let new_request = !entries.iter().any(|item| item.id == request.id);
                entries.retain(|item| item.id != request.id);
                entries.push(request.clone());
                drop(store);
                if new_request {
                    inner.emit(EngineUpdate::PermissionAsked(request.clone()));
                }
                permissions_changed = true;
            }
            AgentEvent::PermissionReplied {
                session_id,
                request_id,
                ..
            } => {
                let mut store = inner.store.write().expect("store lock");
                if let Some(entries) = store.permissions.get_mut(session_id) {
                    entries.retain(|item| &item.id != request_id);
                }
                permissions_changed = true;
            }
            AgentEvent::TodoUpdated { session_id, todos } => {
                let mut store = inner.store.write().expect("store lock");
                store.todos.insert(session_id.clone(), todos.clone());
                drop(store);
                inner.emit(EngineUpdate::Todo(session_id.clone()));
            }
            AgentEvent::UsageUpdated { session_id, usage } => {
                let mut store = inner.store.write().expect("store lock");
                store.usage.insert(session_id.clone(), *usage);
            }
            AgentEvent::Unknown { event_type } => {
                let mut store = inner.store.write().expect("store lock");
                *store.protocol_drift.entry(event_type.clone()).or_default() += 1;
            }
            _ => {}
        }
        if let Some(session_id) = event.session_id()
            && !matches!(
                event,
                AgentEvent::PermissionAsked { .. } | AgentEvent::PermissionReplied { .. }
            )
        {
            let summary = inner.read(|store| {
                store
                    .sessions
                    .iter()
                    .find(|item| item.id == session_id)
                    .cloned()
            });
            let view = match inner.read_view(session_id) {
                Some(view) => Some(view),
                None => {
                    // A closed session lives in the warm cache. Background
                    // events update that retained transcript instead of
                    // silently reopening a panel the user closed.
                    let mut store = inner.store.write().expect("store lock");
                    if let Some(entry) = store.warm.get(session_id) {
                        entry.transcript.apply(&event);
                        None
                    } else if store.active_session.as_deref() == Some(session_id) {
                        let summary = summary.unwrap_or_else(|| SessionSummary {
                            id: session_id.to_string(),
                            title: "Background session".into(),
                            directory: String::new(),
                            updated_at: 0,
                            parent_id: None,
                            agent: None,
                        });
                        let view = Arc::new(SessionView::new(summary));
                        store.views.insert(session_id.to_string(), view.clone());
                        store.upsert_summary(view.summary());
                        Some(view)
                    } else {
                        // Subagent/background sessions are not displayed; their
                        // authoritative history remains in OpenCode.
                        None
                    }
                }
            };
            if let Some(view) = view {
                let changed = view.update_transcript(|transcript| transcript.apply(&event));
                if changed {
                    stream_activity.insert(session_id.to_owned(), Instant::now());
                }
                touched.insert(session_id.to_string());
                view.touch();
            }
        }
    }
    evict_views(inner);
    for id in touched {
        inner.emit(EngineUpdate::Session(id));
    }
    if sessions_changed {
        inner.emit(EngineUpdate::Sessions);
    }
    if permissions_changed {
        inner.emit(EngineUpdate::Permissions);
    }
    if rematerialize && let Some(client) = client.clone() {
        {
            let mut store = inner.store.write().expect("store lock");
            if store.connection != ConnectionState::Ready {
                store.connection = ConnectionState::Ready;
                store.last_error = None;
                drop(store);
                inner.emit(EngineUpdate::Connection);
            }
        }
        let ids = inner.read(|store| store.views.keys().cloned().collect::<Vec<_>>());
        for id in ids {
            spawn_open(inner, client.clone(), id);
        }
    }
}

impl Inner {
    fn read<R>(&self, read: impl FnOnce(&Store) -> R) -> R {
        let store = self.store.read().expect("store lock");
        read(&store)
    }

    fn read_view(&self, id: &str) -> Option<Arc<SessionView>> {
        self.store
            .read()
            .expect("store lock")
            .views
            .get(id)
            .cloned()
    }

    fn emit(&self, update: EngineUpdate) {
        let _ = self.updates.send(update);
    }
}

fn update_sass_watch(inner: &Arc<Inner>, session_id: &str, enabled: bool) -> Result<(), String> {
    if !enabled {
        let mut watches = inner.sass_watches.lock().expect("sass watches lock");
        let updates = watches.disable(session_id);
        if updates.is_empty() { inner.emit(watches.snapshot(session_id)); }
        for update in updates { inner.emit(update); }
        if watches.registered(session_id) && let Err(error) = watches.set_preference(session_id, false) {
            for update in watches.fail(session_id, error.clone()) { inner.emit(update); }
            return Err(error);
        }
        return Ok(());
    }
    let directory = inner.read_view(session_id).map(|view| view.summary().directory)
        .ok_or_else(|| "Open a session to watch Sass files".to_owned());
    let root = match directory.and_then(|directory| sass_root(&directory)) {
        Ok(root) => root,
        Err(error) => {
            emit_sass_failure(inner, session_id, error.clone());
            return Err(error);
        }
    };
    let worker = {
        let mut watches = inner.sass_watches.lock().expect("sass watches lock");
        watches.register(session_id, root);
        if let Err(error) = watches.set_preference(session_id, true) {
            for update in watches.fail(session_id, error.clone()) { inner.emit(update); }
            return Err(error);
        }
        let worker = watches.start(session_id, &inner.cancel);
        if let Some(worker) = &worker {
            let lines = watches.lines(session_id);
            for update in watches.publish(worker, lines) { inner.emit(update); }
        } else { inner.emit(watches.snapshot(session_id)); }
        worker
    };
    if let Some(worker) = worker { spawn_sass_watch(inner.clone(), worker); }
    Ok(())
}

fn register_sass_context(inner: &Arc<Inner>, summary: &SessionSummary) {
    if let Ok(root) = sass_root(&summary.directory) {
        let mut watches = inner.sass_watches.lock().expect("sass watches lock");
        let fresh = watches.register(&summary.id, root);
        let worker = match if fresh {
            watches.configured(&summary.id)
        } else {
            Ok(false)
        } {
            Ok(true) if fresh => watches.start(&summary.id, &inner.cancel),
            Ok(true) => None,
            Ok(false) => None,
            Err(error) => {
                for update in watches.fail(&summary.id, error) {
                    inner.emit(update);
                }
                return;
            }
        };
        inner.emit(watches.snapshot(&summary.id));
        drop(watches);
        if let Some(worker) = worker {
            spawn_sass_watch(inner.clone(), worker);
        }
    }
}

fn ensure_view(inner: &Arc<Inner>, summary: SessionSummary) -> Arc<SessionView> {
    register_sass_context(inner, &summary);

    if !summary.directory.is_empty()
        && let Err(error) = inner.observations.attach(&summary.id, &summary.directory)
        && !inner.cancel.is_cancelled()
    {
        tracing::warn!(%error, "could not attach workspace observation");
    }
    if let Some(view) = inner.read_view(&summary.id) {
        view.set_summary(summary);
        view.touch();
        return view;
    }
    let warmed = {
        inner
            .store
            .write()
            .expect("store lock")
            .warm
            .remove(&summary.id)
    };
    if let Some(entry) = warmed {
        let view = Arc::new(SessionView::from_warm(entry));
        view.set_summary(summary.clone());
        inner
            .store
            .write()
            .expect("store lock")
            .insert_view(view.clone());
        return view;
    }
    let view = Arc::new(SessionView::new(summary.clone()));
    let mut store = inner.store.write().expect("store lock");
    store.insert_view(view.clone());
    store.upsert_summary(summary);
    view
}

fn close_to_warm(inner: &Arc<Inner>, id: &str) {
    inner
        .session_loads
        .lock()
        .expect("session loads lock")
        .remove(id);
    inner.observations.detach(id);
    let mut store = inner.store.write().expect("store lock");
    let Some(view) = store.views.remove(id) else {
        return;
    };
    let transcript = view.transcript_clone();
    let entry = WarmEntry::new(view.summary(), transcript)
        .with_draft(view.draft())
        .with_scroll(view.scroll());
    store.warm.insert(entry);
    if store.active_session.as_deref() == Some(id) {
        store.active_session = None;
    }
}

fn retire_session(inner: &Arc<Inner>, id: &str) {
    {
        let mut store = inner.store.write().expect("store lock");
        if store.retired_sessions.contains(id) {
            return;
        }
        store.retire_session(id);
    }
    inner.observations.detach(id);
    inner.forms.lock().expect("forms lock").remove(id);
    inner.emit(EngineUpdate::SessionRemoved(id.to_owned()));
    inner.emit(EngineUpdate::Sessions);
    inner.emit(EngineUpdate::Permissions);
    // Run the actor's preview/OAuth close cleanup without retaining deleted
    // conversation state in the warm cache.
    let _ = inner.commands.send(Command::Close(id.to_owned()));
}

fn evict_views(inner: &Arc<Inner>) {
    let limit = inner.config.warm_sessions;
    loop {
        let victim = {
            let store = inner.store.read().expect("store lock");
            if store.views.len() <= limit.saturating_add(1) {
                None
            } else {
                store
                    .views
                    .iter()
                    .filter(|(id, _)| store.active_session.as_deref() != Some(id.as_str()))
                    .min_by_key(|(_, view)| view.last_used())
                    .map(|(id, _)| id.clone())
            }
        };
        let Some(id) = victim else { break };
        close_to_warm(inner, &id);
    }
}

fn spawn_open(inner: &Arc<Inner>, client: Client, session_id: String) {
    let generation = uuid::Uuid::new_v4();
    inner
        .session_loads
        .lock()
        .expect("session loads lock")
        .insert(session_id.clone(), generation);
    if let Some(view) = inner.read_view(&session_id) {
        view.touch();
    }
    let inner = inner.clone();
    tokio::spawn(async move {
        let result = async {
            let summary = client
                .get_session(&session_id)
                .await
                .map_err(|e| e.to_string())?;
            let messages = client
                .load_messages(&session_id)
                .await
                .map_err(|e| e.to_string())?;
            Ok((summary, messages))
        }
        .await;
        let _ = inner.commands.send(Command::Loaded {
            session_id,
            generation,
            result,
        });
    });
}

fn spawn_open_directory(inner: &Arc<Inner>, client: Client, directory: String) {
    let inner = inner.clone();
    tokio::spawn(async move {
        match client.create_session(&directory).await {
            Ok(summary) => {
                tracing::debug!(session = %summary.id, "created session");
                let session_id = summary.id.clone();
                let generation = uuid::Uuid::new_v4();
                inner
                    .session_loads
                    .lock()
                    .expect("session loads lock")
                    .insert(session_id.clone(), generation);
                ensure_view(&inner, summary);
                inner.emit(EngineUpdate::WorkspaceOpened {
                    directory: directory.clone(),
                    result: Ok(session_id.clone()),
                });
                inner.emit(EngineUpdate::Sessions);
                let result = async {
                    let messages = client
                        .load_messages(&session_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    let summary = client
                        .get_session(&session_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok((summary, messages))
                }
                .await;
                let _ = inner.commands.send(Command::Loaded {
                    session_id,
                    generation,
                    result,
                });
            }
            Err(error) => {
                tracing::warn!(%error, "create session failed");
                set_error(&inner, &format!("Could not open workspace: {error}"));
                inner.emit(EngineUpdate::WorkspaceOpened {
                    directory,
                    result: Err(error.to_string()),
                });
            }
        }
    });
}

/// File-path fields used by tool inputs, matching Source's recursive
/// `collectFilePaths`: `filePath`, `file_path` and `path`, with HTTP values
/// skipped. Order is retained and exact duplicates are removed.
fn tool_input_file_paths(input: &Value) -> Vec<String> {
    fn walk(value: &Value, paths: &mut Vec<String>, seen: &mut HashSet<String>) {
        match value {
            Value::Array(items) => {
                for item in items {
                    walk(item, paths, seen);
                }
            }
            Value::Object(fields) => {
                for (key, value) in fields {
                    if matches!(key.as_str(), "filePath" | "file_path" | "path")
                        && let Some(path) = value.as_str()
                    {
                        if !path.starts_with("http") && seen.insert(path.to_owned()) {
                            paths.push(path.to_owned());
                        }
                        continue;
                    }
                    walk(value, paths, seen);
                }
            }
            _ => {}
        }
    }
    let mut paths = Vec::new();
    walk(input, &mut paths, &mut HashSet::new());
    paths
}

fn session_directory(inner: &Arc<Inner>, session_id: &str) -> Option<PathBuf> {
    inner
        .read_view(session_id)
        .map(|view| PathBuf::from(view.summary().directory))
        .or_else(|| {
            inner.read(|store| {
                store
                    .sessions
                    .iter()
                    .find(|session| session.id == session_id)
                    .map(|session| PathBuf::from(&session.directory))
            })
        })
        .filter(|root| !root.as_os_str().is_empty())
}

fn spawn_load_recovery(inner: &Arc<Inner>, session_id: String) {
    let Some(root) = session_directory(inner, &session_id) else {
        inner.emit(EngineUpdate::RecoveryFailed {
            session_id,
            error: "This session has no workspace directory".into(),
        });
        return;
    };
    let inner = inner.clone();
    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || crate::recovery::records(&root))
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result.map_err(|error| error.to_string()));
        match result {
            Ok(records) => inner.emit(EngineUpdate::Recovery { session_id, records }),
            Err(error) => inner.emit(EngineUpdate::RecoveryFailed { session_id, error }),
        }
    });
}

fn spawn_open_file_workspace(
    inner: &Arc<Inner>,
    client: Client,
    file: String,
    reply: tokio::sync::oneshot::Sender<Result<crate::FileWorkspace, String>>,
) {
    let inner = inner.clone();
    tokio::spawn(async move {
        if reply.is_closed() || inner.cancel.is_cancelled() {
            return;
        }
        let target =
            match tokio::task::spawn_blocking(move || crate::file_workspace::resolve(&file))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result)
            {
                Ok(target) => target,
                Err(error) => {
                    let _ = reply.send(Err(error));
                    return;
                }
            };
        if reply.is_closed() || inner.cancel.is_cancelled() {
            return;
        }
        match client.create_session(&target.directory).await {
            Ok(mut summary) => {
                summary.directory = target.directory.clone();
                let session_id = summary.id.clone();
                // Keep this as an inventory entry until the frontend accepts
                // the captured request. No observer, Sass, terminal or active
                // view can be stolen by an obsolete file-dialog completion.
                inner
                    .store
                    .write()
                    .expect("store lock")
                    .upsert_summary(summary);
                inner.emit(EngineUpdate::Sessions);
                let _ = reply.send(Ok(crate::FileWorkspace {
                    session_id,
                    directory: target.directory,
                    path: target.path,
                }));
            }
            Err(error) => {
                let _ = reply.send(Err(error.to_string()));
            }
        }
    });
}

fn spawn_refresh_sessions(inner: &Arc<Inner>, client: Client) {
    let inner = inner.clone();
    tokio::spawn(async move {
        match client.list_sessions(None).await {
            Ok(mut sessions) => {
                sessions.sort_by_key(|left| std::cmp::Reverse(left.updated_at));
                {
                    let mut store = inner.store.write().expect("store lock");
                    sessions.retain(|session| !store.retired_sessions.contains(&session.id));
                    store.sessions = sessions;
                }
                inner.emit(EngineUpdate::Sessions);
            }
            Err(error) => set_error(&inner, &format!("Could not list sessions: {error}")),
        }
    });
}

fn spawn_refresh_permissions(inner: &Arc<Inner>, client: Client) {
    let inner = inner.clone();
    tokio::spawn(async move {
        match client.list_permissions(None).await {
            Ok(permissions) => {
                let mut grouped: std::collections::HashMap<String, Vec<PermissionRequest>> =
                    std::collections::HashMap::new();
                for request in permissions {
                    grouped
                        .entry(request.session_id.clone())
                        .or_default()
                        .push(request);
                }
                {
                    let mut store = inner.store.write().expect("store lock");
                    grouped.retain(|id, _| !store.retired_sessions.contains(id));
                    store.permissions = grouped;
                }
                inner.emit(EngineUpdate::Permissions);
            }
            Err(error) => debug!(%error, "permission refresh failed"),
        }
    });
}

/// Keep re-resolving the shared registration until a healthy service returns.
/// A stopped daemon may be restarted much later; the engine stays connected to
/// the window and rehydrates when the new endpoint appears instead of giving up
/// after one timeout.
fn spawn_reconnect(inner: Arc<Inner>, attempt: u64, service_file: PathBuf) {
    tokio::spawn(async move {
        loop {
            if inner.cancel.is_cancelled() {
                return;
            }
            if let Some(endpoint) = vela_opencode::service::discover_at(&service_file).await {
                let result = Client::new(endpoint.clone())
                    .map(|client| (client, endpoint, service_file.clone()))
                    .map_err(|error| format!("Could not reconnect to OpenCode: {error}"));
                let _ = inner
                    .commands
                    .send(Command::ReconnectFinished { attempt, result });
                return;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

async fn connect_loop(
    tx: mpsc::Sender<(Client, vela_opencode::Endpoint)>,
    cancel: CancellationToken,
    service_file: Option<PathBuf>,
    program: PathBuf,
) {
    loop {
        if cancel.is_cancelled() {
            return;
        }
        let connecting = async {
            let file = service_file
                .clone()
                .unwrap_or_else(vela_opencode::service::service_file);
            vela_opencode::service::connect_at_with_program(&file, &program).await
        };
        let result = tokio::select! {
            _ = cancel.cancelled() => return,
            result = tokio::time::timeout(Duration::from_secs(70), connecting) => result,
        };
        match result {
            Ok(Ok(connection)) => {
                if tx.send(connection).await.is_ok() {
                    return;
                }
                return;
            }
            Ok(Err(error)) => warn!(%error, "OpenCode connection failed; retrying"),
            Err(_) => warn!("OpenCode connection timed out; retrying"),
        }
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
        }
    }
}

fn spawn_inbox_refresh(inner: &Arc<Inner>, client: Client, session_id: String) {
    let inner = inner.clone();
    tokio::spawn(async move {
        match client.list_inbox(&session_id).await {
            Ok(items) => inner.emit(EngineUpdate::Inbox {
                session_id,
                items,
                error: None,
            }),
            Err(error) => inner.emit(EngineUpdate::Inbox {
                session_id,
                items: Vec::new(),
                error: Some(error.to_string()),
            }),
        }
    });
}

fn spawn_forms_refresh(inner: &Arc<Inner>, client: Client, session_id: String) {
    let directory = session_directory(inner, &session_id).and_then(|path| path.to_str().map(str::to_owned));
    let inner = inner.clone();
    tokio::spawn(async move {
        let session_forms = client.list_forms(&session_id).await;
        let location_forms = match directory.as_deref() {
            Some(directory) => client.list_location_forms(directory).await,
            None => Ok(Vec::new()),
        };
        let mut forms = Vec::new();
        let mut error = None;
        match session_forms {
            Ok(rows) => forms = rows,
            Err(failure) => error = Some(failure.to_string()),
        }
        match location_forms {
            Ok(rows) => {
                for form in rows {
                    if !forms.iter().any(|existing| existing.id == form.id) {
                        forms.push(form);
                    }
                }
            }
            Err(failure) => {
                if error.is_none() {
                    error = Some(failure.to_string());
                }
            }
        }
        if error.is_some() && forms.is_empty() {
            let cached = inner
                .forms
                .lock()
                .expect("forms lock")
                .get(&session_id)
                .cloned()
                .unwrap_or_default();
            inner.emit(EngineUpdate::Forms {
                session_id,
                forms: cached,
                error,
            });
            return;
        }
        inner
            .forms
            .lock()
            .expect("forms lock")
            .insert(session_id.clone(), forms.clone());
        inner.emit(EngineUpdate::Forms {
            session_id,
            forms,
            error,
        });
    });
}

fn sass_root(directory: &str) -> Result<String, String> {
    let root = std::fs::canonicalize(directory).map_err(|error| error.to_string())?;
    if !root.is_dir() {
        return Err("Sass workspace is not a directory".into());
    }
    root.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "Sass workspace path is not valid Unicode".into())
}

fn spawn_sass_watch(inner: Arc<Inner>, worker: crate::sass_state::Lease) {
    tokio::spawn(async move {
        let program = crate::sass_compiler::Program::discover();
        let mut seen = None;
        let Some(mut lines) = inner
            .sass_watches
            .lock()
            .expect("sass watches lock")
            .worker_lines(&worker)
        else {
            return;
        };
        let root = PathBuf::from(&worker.root);
        let mut changes =
            match tokio::task::spawn_blocking(move || crate::sass_watch::Changes::start(&root))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result)
            {
                Ok(changes) => changes,
                Err(error) => {
                    lines.push(SassLine {
                        time_ms: crate::engine::sass_log_time(),
                        level: crate::SassLogLevel::Error,
                        message: error.clone(),
                    });
                    fail_sass_worker(&inner, &worker, &mut lines, error);
                    return;
                }
            };
        tokio::select! {
            _ = worker.cancel.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_millis(150)) => {},
        }
        loop {
            tokio::select! {
                _ = worker.cancel.cancelled() => break,
                _ = sass_pass(&inner, &worker, &program, &mut seen, &mut lines) => {},
            }
            loop {
                match changes.next(&worker.cancel).await {
                    None => {
                        if !worker.cancel.is_cancelled() {
                            fail_sass_worker(
                                &inner,
                                &worker,
                                &mut lines,
                                "Sass file watcher stopped".into(),
                            );
                        }
                        return;
                    }
                    Some(Ok(())) => break,
                    Some(Err(error)) => {
                        lines.push(SassLine {
                            time_ms: crate::engine::sass_log_time(),
                            level: crate::SassLogLevel::Error,
                            message: error.clone(),
                        });
                        finish_sass_lines(&inner, &worker, &mut lines, Some(error));
                    }
                }
            }
        }
    });
}
async fn sass_pass(
    inner: &Arc<Inner>,
    worker: &crate::sass_state::Lease,
    program: &Result<crate::sass_compiler::Program, String>,
    seen: &mut Option<crate::sass_sources::Inventory>,
    lines: &mut Vec<SassLine>,
) {
    let root = PathBuf::from(&worker.root);
    let scan_root = root.clone();
    let scanned = tokio::task::spawn_blocking(move || crate::sass_sources::scan(&scan_root)).await;
    let inventory = match scanned
        .map_err(|error| error.to_string())
        .and_then(|result| result)
    {
        Ok(inventory) => inventory,
        Err(error) => {
            if !lines
                .last()
                .is_some_and(|line| line.level == crate::SassLogLevel::Error && line.message == error)
            {
                lines.push(SassLine {
                    time_ms: crate::engine::sass_log_time(),
                    level: crate::SassLogLevel::Error,
                    message: error.clone(),
                });
                finish_sass_lines(inner, worker, lines, Some(error));
            }
            *seen = None;
            return;
        }
    };
    if seen.as_ref() == Some(&inventory) {
        return;
    }
    tracing::debug!(root = %worker.root, entries = inventory.entries.len(), "compiling Sass inventory");
    let result = match program {
        Ok(program) => program.compile(&root, &worker.cancel).await,
        Err(error) => Err(error.clone()),
    };
    match result {
        Ok(batch) => {
            let pass_inner = inner.clone();
            let pass_worker = worker.clone();
            let pass_lines = lines.clone();
            match tokio::task::spawn_blocking(move || {
                install_sass_batch(&pass_inner, &pass_worker, batch, pass_lines)
            })
            .await
            {
                Ok(next) => *lines = next,
                Err(error) => {
                    let error = format!("Sass output worker failed: {error}");
                    lines.push(SassLine {
                        time_ms: crate::engine::sass_log_time(),
                        level: crate::SassLogLevel::Error,
                        message: error.clone(),
                    });
                    finish_sass_lines(inner, worker, lines, Some(error));
                }
            }
        }
        Err(error) => {
            lines.push(SassLine {
                time_ms: crate::engine::sass_log_time(),
                level: crate::SassLogLevel::Error,
                message: error.clone(),
            });
            finish_sass_lines(inner, worker, lines, Some(error));
        }
    }
    *seen = Some(inventory);
}
fn install_sass_batch(
    inner: &Arc<Inner>,
    worker: &crate::sass_state::Lease,
    batch: crate::sass_compiler::Batch,
    mut lines: Vec<SassLine>,
) -> Vec<SassLine> {
    let root = PathBuf::from(&worker.root);
    lines.push(SassLine {
        time_ms: crate::engine::sass_log_time(),
        level: crate::SassLogLevel::Info,
        message: format!(
            "{}: compiling {} CSS output{}",
            batch.convention,
            batch.outputs.len(),
            if batch.outputs.len() == 1 { "" } else { "s" }
        ),
    });
    let mut first_error = None;
    for output in batch.outputs {
        if worker.cancel.is_cancelled() {
            break;
        }
        for warning in output.warnings {
            lines.push(SassLine {
                time_ms: crate::engine::sass_log_time(),
                level: crate::SassLogLevel::Warning,
                message: warning,
            });
            crate::sass_state::limit_lines(&mut lines);
        }
        let source = output
            .source
            .strip_prefix(&root)
            .unwrap_or(&output.source)
            .display();
        let result = if let Some(error) = output.error {
            Err(error)
        } else if let Some(css) = output.css {
            let write = worker.write_gate.lock().expect("Sass output gate");
            if !inner
                .sass_watches
                .lock()
                .expect("sass watches lock")
                .live(worker)
            {
                break;
            }
            let result = (|| {
                if let Some(map) = output.source_map {
                    let mut path = output.output.as_os_str().to_owned();
                    path.push(".map");
                    crate::sass_output::write_map(
                        &root,
                        &PathBuf::from(path),
                        &map,
                        &worker.cancel,
                    )?;
                }
                crate::sass_output::write_css(&root, &output.output, &css, &worker.cancel)
            })();
            drop(write);
            result
        } else {
            Err("Sass compiler returned no CSS".into())
        };
        if let Err(error) = &result {
            first_error.get_or_insert_with(|| error.clone());
        }
        lines.push(match result {
            Ok(()) => SassLine {
                time_ms: crate::engine::sass_log_time(),
                level: crate::SassLogLevel::Info,
                message: format!(
                    "{source} → {}",
                    output
                        .output
                        .strip_prefix(&root)
                        .unwrap_or(&output.output)
                        .display()
                ),
            },
            Err(error) => SassLine {
                time_ms: crate::engine::sass_log_time(),
                level: crate::SassLogLevel::Error,
                message: format!("{source}: {error}"),
            },
        });
        crate::sass_state::limit_lines(&mut lines);
    }
    // Prior error logs remain visible, but a successful pass clears status.
    finish_sass_lines(inner, worker, &mut lines, first_error);
    lines
}

fn emit_sass_failure(inner: &Arc<Inner>, session: &str, error: String) {
    let mut watches = inner.sass_watches.lock().expect("sass watches lock");
    for update in watches.fail(session, error) {
        inner.emit(update);
    }
}
fn fail_sass_worker(
    inner: &Arc<Inner>,
    worker: &crate::sass_state::Lease,
    lines: &mut Vec<SassLine>,
    error: String,
) {
    crate::sass_state::limit_lines(lines);
    let mut watches = inner.sass_watches.lock().expect("sass watches lock");
    for update in watches.failed_worker(worker, lines.clone(), error) {
        inner.emit(update);
    }
}
fn finish_sass_lines(
    inner: &Arc<Inner>,
    worker: &crate::sass_state::Lease,
    lines: &mut Vec<SassLine>,
    error: Option<String>,
) {
    crate::sass_state::limit_lines(lines);
    let mut watches = inner.sass_watches.lock().expect("sass watches lock");
    for update in watches.finish(worker, lines.clone(), error) {
        inner.emit(update);
    }
}

fn spawn_catalog(inner: &Arc<Inner>, client: Client, session_id: String) {
    let inner = inner.clone();
    tokio::spawn(async move {
        let Some(directory) = inner
            .read_view(&session_id)
            .map(|view| view.summary().directory)
            .filter(|directory| !directory.is_empty())
        else {
            return;
        };
        let (models, default_model, selected_model) = tokio::join!(
            client.list_models(&directory),
            client.default_model(&directory),
            client.selected_model(&session_id)
        );
        let (models, default_model, selected_model) = match (models, default_model, selected_model)
        {
            (Ok(models), Ok(default_model), Ok(selected_model)) => {
                (models, default_model, selected_model)
            }
            (models, default_model, selected_model) => {
                let error = models
                    .err()
                    .or_else(|| default_model.err())
                    .or_else(|| selected_model.err())
                    .map(|error| error.to_string());
                inner.emit(EngineUpdate::Catalog {
                    session_id,
                    models: Vec::new(),
                    agents: Vec::new(),
                    default_model: None,
                    error,
                });
                return;
            }
        };
        let agents = client.list_agents(&directory).await.unwrap_or_default();
        let default_model = selected_model.or(default_model).map(|selection| {
            if let Some(catalog) = models.iter().find(|model| {
                model.id == selection.id && model.provider_id == selection.provider_id
            }) {
                let mut catalog = catalog.clone();
                catalog.variant = selection.variant;
                catalog
            } else {
                selection
            }
        });
        inner.emit(EngineUpdate::Catalog {
            session_id,
            models,
            agents,
            default_model,
            error: None,
        });
    });
}

fn spawn_file_job(
    inner: &Arc<Inner>,
    session_id: String,
    job: impl FnOnce(std::path::PathBuf) -> EngineUpdate + Send + 'static,
) {
    let root = inner
        .read_view(&session_id)
        .map(|view| view.summary().directory)
        .filter(|directory| !directory.is_empty());
    let Some(root) = root else {
        set_error(inner, "This session has no workspace directory");
        return;
    };
    if let Err(error) = inner.observations.attach(&session_id, &root) {
        set_error(inner, &error);
        return;
    }
    let inner = inner.clone();
    tokio::task::spawn_blocking(move || {
        inner.emit(job(PathBuf::from(root)));
    });
}

fn write_file_update(
    workspace: &mut crate::observation::Workspace,
    request_id: u64,
    rel: String,
    content: String,
    expected: String,
    overwrite: bool,
) -> EngineUpdate {
    let result = workspace.write_file(&rel, &content, &expected, overwrite, request_id);
    let conflict = if matches!(&result, Err(crate::files::FileError::Changed(_))) {
        match crate::files::read_file(workspace.root(), &rel) {
            Ok(content) => Some(ExternalChange {
                content: Some(content),
                ..Default::default()
            }),
            Err(crate::files::FileError::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                Some(ExternalChange {
                    deleted: true,
                    ..Default::default()
                })
            }
            Err(error) => Some(ExternalChange {
                read_error: Some(error.to_string()),
                ..Default::default()
            }),
        }
    } else {
        None
    };
    EngineUpdate::FileSaved {
        request_id,
        rel,
        conflict,
        error: result.err().map(|error| error.to_string()),
    }
}

fn read_file_update(workspace: &mut crate::observation::Workspace, request_id: u64, rel: String) -> EngineUpdate {
    let result = workspace.read_file(&rel);
    let deleted = workspace.root().is_dir() && matches!(&result, Err(crate::files::FileError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound);
    EngineUpdate::FileContent {
        request_id, rel, deleted,
        content: result.as_ref().ok().cloned(),
        error: if deleted { None } else { result.err().map(|error| error.to_string()) },
    }
}

fn spawn_workspace_job(
    inner: &Arc<Inner>,
    session_id: String,
    job: impl FnOnce(&mut crate::observation::Workspace) -> EngineUpdate + Send + 'static,
) {
    let root = inner
        .read_view(&session_id)
        .map(|view| view.summary().directory)
        .filter(|directory| !directory.is_empty());
    let Some(root) = root else {
        inner.emit(EngineUpdate::Error(
            "This session has no workspace directory".into(),
        ));
        return;
    };
    if let Err(error) = inner.observations.submit(&session_id, &root, job) {
        set_error(inner, &error);
    }
}

fn start_terminal(inner: &Arc<Inner>, id: String, cwd: String, cols: u16, rows: u16) {
    start_terminal_with(inner, id, cwd, cols, rows, None);
}

struct ProcessCommand {
    program: String,
    args: Vec<String>,
    envs: Vec<(String, String)>,
}

fn start_terminal_command(inner: &Arc<Inner>, command: TerminalCommand) {
    start_terminal_with(
        inner,
        command.id,
        command.cwd,
        command.cols,
        command.rows,
        Some(ProcessCommand {
            program: command.program,
            args: command.args,
            envs: command.envs,
        }),
    );
}

fn start_terminal_with(
    inner: &Arc<Inner>,
    id: String,
    cwd: String,
    cols: u16,
    rows: u16,
    command: Option<ProcessCommand>,
) {
    if inner
        .terminals
        .lock()
        .map(|terminals| terminals.contains_key(&id))
        .unwrap_or(true)
    {
        set_error(inner, "terminal already exists or is unavailable");
        return;
    }
    let spawned = match command {
        Some(command) => vela_terminal::pty::spawn_command(
            &cwd,
            &command.program,
            &command.args,
            &command.envs,
            cols,
            rows,
        ),
        None => vela_terminal::pty::spawn(&cwd, cols, rows),
    };
    match spawned {
        Ok(spawned) => {
            let owner = Arc::downgrade(&spawned.session);
            if let Ok(mut terminals) = inner.terminals.lock() {
                if inner.cancel.is_cancelled() {
                    spawned.session.kill();
                    return;
                }
                terminals.insert(id.clone(), spawned.session);
            } else {
                spawned.session.kill();
                set_error(inner, "terminal registry unavailable");
                return;
            }
            let inner = inner.clone();
            tokio::spawn(async move {
                let code = crate::terminal_relay::relay(
                    spawned.output,
                    spawned.exit,
                    inner.cancel.clone(),
                    |bytes| {
                        if let Ok(terminals) = inner.terminals.lock()
                            && terminals.get(&id).is_some_and(|session| {
                                std::sync::Weak::ptr_eq(&Arc::downgrade(session), &owner)
                            })
                        {
                            inner.emit(EngineUpdate::TerminalData {
                                id: id.clone(),
                                data: Arc::new(bytes),
                            });
                        }
                    },
                )
                .await;
                if let Ok(mut terminals) = inner.terminals.lock()
                    && terminals.get(&id).is_some_and(|session| {
                        std::sync::Weak::ptr_eq(&Arc::downgrade(session), &owner)
                    })
                {
                    terminals.remove(&id);
                    inner.emit(EngineUpdate::TerminalExit { id, code });
                }
            });
        }
        Err(error) => {
            set_error(
                inner,
                &format!("Could not start terminal in {cwd}: {error}"),
            );
            inner.emit(EngineUpdate::TerminalExit { id, code: None });
        }
    }
}

fn kill_all_terminals(inner: &Arc<Inner>) {
    if let Ok(mut terminals) = inner.terminals.lock() {
        for (_, session) in terminals.drain() {
            session.kill();
        }
    }
}

fn set_error(inner: &Arc<Inner>, message: &str) {
    warn!(%message, "engine error");
    {
        let mut store = inner.store.write().expect("store lock");
        store.last_error = Some(message.to_string());
    }
    inner.emit(EngineUpdate::Error(message.to_string()));
}

fn is_critical(raw: &Value) -> bool {
    raw.get("type").and_then(Value::as_str).is_some_and(|kind| {
        kind.starts_with("permission.")
            || kind.starts_with("session.execution.")
            || kind == "session.error"
            || kind == "global.error"
            || kind == "session.idle"
            || kind == "session.created"
            || kind == "session.deleted"
            || kind == "server.connected"
    })
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tool_input_path_tests {
    use super::tool_input_file_paths;
    use serde_json::json;

    #[test]
    fn tool_input_paths_cover_common_keys_nested_arrays_and_http_values() {
        let input = json!({
            "filePath": "src/app.ts",
            "path": "src/app.ts",
            "nested": {
                "file_path": "src/lib.rs",
                "path": "https://example.com/ignored",
            },
            "edits": [
                { "filePath": "/tmp/absolute.ts" },
                { "file_path": "src/other.ts" },
            ],
            "command": "npm test",
        });
        let mut paths = tool_input_file_paths(&input);
        paths.sort();
        assert_eq!(
            paths,
            vec![
                "/tmp/absolute.ts".to_string(),
                "src/app.ts".to_string(),
                "src/lib.rs".to_string(),
                "src/other.ts".to_string(),
            ]
        );
    }

    #[test]
    fn tool_input_paths_keep_first_occurrence_and_ignore_non_objects() {
        let input = json!([
            { "path": "a.ts" },
            { "path": "b.ts" },
            { "path": "a.ts" },
            null,
            "path",
        ]);
        assert_eq!(
            tool_input_file_paths(&input),
            vec!["a.ts".to_string(), "b.ts".to_string()]
        );
        assert!(tool_input_file_paths(&json!(null)).is_empty());
    }
}
