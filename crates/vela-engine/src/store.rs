use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use vela_core::event::{PermissionRequest, TodoItem};
use vela_core::session::{SessionPhase, SessionSummary, SessionUsage};
use vela_core::transcript::{Block, Transcript};
use vela_core::warm::WarmCache;
use vela_opencode::ModelInfo;

use crate::engine::ConnectionState;

pub struct SessionView {
    pub summary: RwLock<SessionSummary>,
    transcript: RwLock<Transcript>,
    pub last_used: RwLock<Instant>,
    draft: RwLock<String>,
    scroll: RwLock<(f32, bool)>,
    selected_model: RwLock<Option<ModelInfo>>,
    selected_agent: RwLock<Option<String>>,
}

impl SessionView {
    pub fn new(summary: SessionSummary) -> Self {
        let agent = summary.agent.clone();
        Self {
            summary: RwLock::new(summary),
            transcript: RwLock::new(Transcript::new()),
            last_used: RwLock::new(Instant::now()),
            draft: RwLock::new(String::new()),
            scroll: RwLock::new((0.0, true)),
            selected_model: RwLock::new(None),
            selected_agent: RwLock::new(agent),
        }
    }

    pub fn from_warm(entry: vela_core::warm::WarmEntry) -> Self {
        let agent = entry.session.agent.clone();
        Self {
            summary: RwLock::new(entry.session),
            transcript: RwLock::new(entry.transcript),
            last_used: RwLock::new(Instant::now()),
            draft: RwLock::new(entry.draft),
            scroll: RwLock::new((entry.scroll_top, entry.stick_to_bottom)),
            selected_model: RwLock::new(None),
            selected_agent: RwLock::new(agent),
        }
    }

    pub fn draft(&self) -> String {
        self.draft.read().expect("draft lock").clone()
    }

    pub fn set_draft(&self, draft: String) {
        *self.draft.write().expect("draft lock") = draft;
    }

    pub fn scroll(&self) -> (f32, bool) {
        *self.scroll.read().expect("scroll lock")
    }

    pub fn set_scroll(&self, top: f32, stick_to_bottom: bool) {
        *self.scroll.write().expect("scroll lock") = (top.max(0.0), stick_to_bottom);
    }

    pub fn selection(&self) -> (Option<ModelInfo>, Option<String>) {
        (
            self.selected_model.read().expect("model lock").clone(),
            self.selected_agent.read().expect("agent lock").clone(),
        )
    }

    pub fn set_selection(&self, model: Option<ModelInfo>, agent: Option<String>) {
        if let Some(model) = model {
            *self.selected_model.write().expect("model lock") = Some(model);
        }
        if let Some(agent) = agent {
            *self.selected_agent.write().expect("agent lock") = Some(agent);
        }
    }

    pub fn set_agent(&self, agent: String) {
        *self.selected_agent.write().expect("agent lock") = Some(agent);
    }

    pub fn summary(&self) -> SessionSummary {
        self.summary.read().expect("session summary lock").clone()
    }

    pub fn set_summary(&self, summary: SessionSummary) {
        *self.summary.write().expect("session summary lock") = summary;
    }

    pub fn touch(&self) {
        *self.last_used.write().expect("last used lock") = Instant::now();
    }

    pub fn last_used(&self) -> Instant {
        *self.last_used.read().expect("last used lock")
    }

    pub fn transcript_revision(&self) -> u64 {
        self.transcript.read().expect("transcript lock").revision()
    }

    pub fn phase(&self) -> SessionPhase {
        self.transcript.read().expect("transcript lock").phase()
    }

    pub fn blocks(&self) -> Vec<Block> {
        self.transcript
            .write()
            .expect("transcript lock")
            .blocks()
            .to_vec()
    }

    pub fn blocks_with_revision(&self) -> (u64, Vec<Block>) {
        let mut transcript = self.transcript.write().expect("transcript lock");
        let revision = transcript.revision();
        let blocks = transcript.blocks().to_vec();
        (revision, blocks)
    }

    pub fn with_transcript<R>(&self, read: impl FnOnce(&Transcript) -> R) -> R {
        read(&self.transcript.read().expect("transcript lock"))
    }

    pub fn update_transcript<R>(&self, write: impl FnOnce(&mut Transcript) -> R) -> R {
        write(&mut self.transcript.write().expect("transcript lock"))
    }

    pub fn hydrate(&self, messages: Vec<vela_core::transcript::Message>) {
        self.transcript
            .write()
            .expect("transcript lock")
            .hydrate(messages);
    }

    pub fn transcript_clone(&self) -> Transcript {
        self.transcript.read().expect("transcript lock").clone()
    }
}

/// All UI-readable state. The lock is held only for short, non-blocking reads
/// or mutations; no network, disk, or PTY work runs under it.
pub struct Store {
    pub connection: ConnectionState,
    pub sessions: Vec<SessionSummary>,
    /// Runtime deletion is final for a session ID. In-flight replay/inventory
    /// can still finish after its deletion event.
    pub(crate) retired_sessions: HashSet<String>,
    pub views: HashMap<String, Arc<SessionView>>,
    pub warm: WarmCache,
    pub permissions: HashMap<String, Vec<PermissionRequest>>,
    pub todos: HashMap<String, Vec<TodoItem>>,
    pub usage: HashMap<String, SessionUsage>,
    pub active_session: Option<String>,
    pub last_error: Option<String>,
    pub endpoint: Option<String>,
    pub protocol_drift: HashMap<String, u64>,
    pub preview_servers: Vec<crate::preview::PreviewServer>,
}

impl Store {
    pub fn new(warm_sessions: usize, warm_bytes: usize) -> Self {
        Self {
            connection: ConnectionState::Connecting,
            sessions: Vec::new(),
            retired_sessions: HashSet::new(),
            views: HashMap::new(),
            warm: WarmCache::new(warm_sessions, warm_bytes),
            permissions: HashMap::new(),
            todos: HashMap::new(),
            usage: HashMap::new(),
            active_session: None,
            last_error: None,
            endpoint: None,
            protocol_drift: HashMap::new(),
            preview_servers: Vec::new(),
        }
    }

    pub fn view(&self, id: &str) -> Option<Arc<SessionView>> {
        self.views.get(id).cloned()
    }

    pub fn active_view(&self) -> Option<Arc<SessionView>> {
        let id = self.active_session.as_deref()?;
        self.view(id)
    }

    pub fn sessions_sorted(&self) -> Vec<SessionSummary> {
        self.sessions.clone()
    }

    pub fn upsert_summary(&mut self, summary: SessionSummary) {
        if self.retired_sessions.contains(&summary.id) {
            return;
        }
        if let Some(existing) = self.sessions.iter_mut().find(|item| item.id == summary.id) {
            *existing = summary.clone();
        } else {
            self.sessions.push(summary.clone());
        }
        self.sessions.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.title.cmp(&right.title))
        });
        if let Some(view) = self.views.get(&summary.id) {
            view.set_summary(summary);
        }
    }

    pub fn insert_view(&mut self, view: Arc<SessionView>) {
        let summary = view.summary();
        if self.retired_sessions.contains(&summary.id) {
            return;
        }
        // A live view always supersedes a retained copy; keeping both would
        // double-count memory and risk restoring a stale transcript.
        self.warm.remove(&summary.id);
        self.active_session = Some(summary.id.clone());
        self.sessions.iter_mut().for_each(|item| {
            if item.id == summary.id {
                *item = summary.clone();
            }
        });
        if !self.sessions.iter().any(|item| item.id == summary.id) {
            self.sessions.push(summary);
        }
        self.views.insert(view.summary().id, view);
    }

    pub fn touch(&mut self, id: &str) {
        if let Some(view) = self.views.get(id) {
            view.touch();
        }
        self.active_session = Some(id.to_string());
    }

    pub(crate) fn retire_session(&mut self, id: &str) {
        self.retired_sessions.insert(id.to_owned());
        self.sessions.retain(|session| session.id != id);
        self.views.remove(id);
        self.warm.remove(id);
        self.permissions.remove(id);
        self.todos.remove(id);
        self.usage.remove(id);
        if self.active_session.as_deref() == Some(id) {
            self.active_session = None;
        }
    }

    pub fn remove_view(&mut self, id: &str) -> Option<Arc<SessionView>> {
        if self.active_session.as_deref() == Some(id) {
            self.active_session = None;
        }
        self.views.remove(id)
    }
}
