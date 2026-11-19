use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use vela_core::event::{PermissionRequest, TodoItem};
use vela_core::session::{SessionPhase, SessionSummary, SessionUsage};
use vela_core::transcript::{Block, Transcript};
use vela_core::markdown::PreparedMarkdown;
use vela_core::warm::WarmCache;
use vela_opencode::ModelInfo;

use crate::engine::ConnectionState;

/// One atomically published conversation revision. UI consumers must read
/// this snapshot rather than combining live source with prepared syntax from
/// independent locks.
#[derive(Clone, Debug)]
pub struct SessionPresentation {
    pub revision: u64,
    pub blocks: Arc<Vec<Block>>,
    pub phase: SessionPhase,
    pub markdown: Arc<PreparedMarkdown>,
}

impl Default for SessionPresentation {
    fn default() -> Self {
        Self {
            revision: 0, blocks: Arc::new(Vec::new()),
            phase: SessionPhase::default(), markdown: Arc::new(PreparedMarkdown::default()),
        }
    }
}

pub struct SessionView {
    pub summary: RwLock<SessionSummary>,
    transcript: RwLock<Transcript>,
    presentation: RwLock<Arc<SessionPresentation>>,
    markdown_preparation: Mutex<()>,
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
            presentation: RwLock::new(Arc::new(SessionPresentation::default())),
            markdown_preparation: Mutex::new(()),
            last_used: RwLock::new(Instant::now()),
            draft: RwLock::new(String::new()),
            scroll: RwLock::new((0.0, true)),
            selected_model: RwLock::new(None),
            selected_agent: RwLock::new(agent),
        }
    }

    pub fn from_warm(entry: vela_core::warm::WarmEntry) -> Self {
        let agent = entry.session.agent.clone();
        let view = Self {
            summary: RwLock::new(entry.session),
            transcript: RwLock::new(entry.transcript),
            presentation: RwLock::new(Arc::new(SessionPresentation::default())),
            markdown_preparation: Mutex::new(()),
            last_used: RwLock::new(Instant::now()),
            draft: RwLock::new(entry.draft),
            scroll: RwLock::new((entry.scroll_top, entry.stick_to_bottom)),
            selected_model: RwLock::new(None),
            selected_agent: RwLock::new(agent),
        };
        view.prepare_markdown();
        view
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

    /// Prepare only at engine publication boundaries. Publication work is
    /// serialized independently of the transcript, so GPUI snapshot readers
    /// never wait for CPU-heavy parsing and concurrent emits cannot overtake
    /// the final source mutation.
    pub fn prepare_markdown(&self) {
        let _preparation = self.markdown_preparation.lock().expect("markdown preparation lock");
        for _ in 0..2 {
            let previous = self.presentation();
            let (revision, blocks, phase, sources) = {
                let mut transcript = self.transcript.write().expect("transcript lock");
                let revision = transcript.revision();
                if previous.revision == revision { return; }
                let blocks = transcript.blocks().to_vec();
                let sources = PreparedMarkdown::reference_sources(&previous.markdown, transcript.messages());
                (revision, blocks, transcript.phase(), sources)
            };
            let markdown = Arc::new(PreparedMarkdown::refresh_sources(&previous.markdown, revision, &blocks, &sources));
            // A correction may have arrived while parsing. Briefly hold the
            // source read lock for the revision check + atomic publication.
            let transcript = self.transcript.read().expect("transcript lock");
            if transcript.revision() != revision { continue; }
            *self.presentation.write().expect("presentation lock") = Arc::new(SessionPresentation {
                revision, blocks: Arc::new(blocks), phase, markdown,
            });
            return;
        }
        // Every engine mutation publishes an update. If source keeps changing
        // through both attempts, that mutation's emitter is waiting on the
        // preparation mutex and will prepare the newest revision next. UI
        // readers retain the previous coherent snapshot in the meantime.
    }

    pub fn presentation(&self) -> Arc<SessionPresentation> {
        self.presentation.read().expect("presentation lock").clone()
    }

    /// Immutable parsed data from the same atomically published revision as
    /// presentation(). Prefer presentation() when reading blocks as well.
    pub fn markdown(&self) -> Arc<PreparedMarkdown> {
        self.presentation().markdown.clone()
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

#[cfg(test)]
mod markdown_tests {
    use super::*;
    use vela_core::transcript::{Message, Role};

    fn view() -> SessionView {
        SessionView::new(SessionSummary {
            id: "test".into(), title: "Test".into(), directory: String::new(),
            updated_at: 0, parent_id: None, agent: None,
        })
    }

    #[test]
    fn published_markdown_is_immutable_and_getters_never_prepare() {
        let view = view();
        let unpublished = view.markdown();
        view.hydrate(vec![Message::text("message", Role::Assistant, 1, "Original")]);
        assert!(Arc::ptr_eq(&unpublished, &view.markdown()), "snapshot getters cannot parse unpublished revisions");
        view.prepare_markdown();
        let published = view.markdown();
        assert!(published.find("Original", false).is_some());
        assert_eq!(published.revision, view.transcript_revision());
        view.prepare_markdown();
        assert!(Arc::ptr_eq(&published, &view.markdown()), "unchanged publication is constant work");
        view.update_transcript(|transcript| transcript.hydrate(vec![Message::text("message", Role::Assistant, 1, "Correction")]));
        view.prepare_markdown();
        let corrected = view.markdown();
        assert!(corrected.find("Correction", false).is_some());
        assert!(corrected.find("Original", false).is_none());
        assert!(published.find("Original", false).is_some(), "previous immutable snapshot remains valid");
    }

    #[test]
    fn concurrent_corrections_publish_coherent_monotonic_snapshots_and_converge() {
        use std::sync::Barrier;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        type Expected = HashMap<u64, (Arc<str>, SessionPhase)>;
        fn correct(
            view: &SessionView,
            expected: &Mutex<Expected>,
            writer: usize,
            index: usize,
        ) {
            let phase = if index.is_multiple_of(2) {
                SessionPhase::Busy
            } else {
                SessionPhase::Idle
            };
            let text: Arc<str> = format!(
                "writer-{writer} correction-{index} **content** {}",
                "fixture prose ".repeat(128)
            )
            .into();
            let mut message = Message::text("message", Role::Assistant, 1, text.to_string());
            if let Some(part) = message.parts.first_mut()
                && let vela_core::transcript::PartKind::Text { streaming, .. } = &mut part.kind
            {
                *streaming = phase.is_active();
            }
            message.completed_at = (!phase.is_active()).then_some(2);
            view.update_transcript(|transcript| {
                transcript.hydrate(vec![message]);
                transcript.set_phase(phase);
                // Register the expected state before releasing the source
                // lock. Every published revision must name one whole state.
                expected
                    .lock()
                    .unwrap()
                    .insert(transcript.revision(), (text, phase));
            });
        }

        fn assert_coherent(snapshot: &SessionPresentation, expected: &Mutex<Expected>) {
            let (source, phase) = expected
                .lock()
                .unwrap()
                .get(&snapshot.revision)
                .cloned()
                .expect("publication must correspond to a complete source mutation");
            assert_eq!(snapshot.phase, phase);
            assert_eq!(snapshot.markdown.revision, snapshot.revision);
            assert_eq!(snapshot.blocks.len(), 1);
            let block = &snapshot.blocks[0];
            assert_eq!(block.text, source);
            assert_eq!(block.streaming, phase.is_active());
            assert!(snapshot.markdown.get(&block.id, &source, block.streaming).is_some());
        }

        let view = Arc::new(view());
        let expected = Mutex::new(Expected::new());
        correct(&view, &expected, 0, 0);
        view.prepare_markdown();
        let original = view.presentation();
        let original_source = original.blocks[0].text.clone();
        let start = Barrier::new(3);
        let finished = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                start.wait();
                for index in 1..=24 {
                    correct(&view, &expected, 1, index);
                    view.prepare_markdown();
                }
                finished.fetch_add(1, Ordering::Release);
            });
            let second = scope.spawn(|| {
                start.wait();
                for index in 1..=24 {
                    correct(&view, &expected, 2, index);
                    view.prepare_markdown();
                }
                finished.fetch_add(1, Ordering::Release);
            });
            start.wait();
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut previous_revision = original.revision;
            loop {
                let snapshot = view.presentation();
                assert_coherent(&snapshot, &expected);
                assert!(snapshot.revision >= previous_revision, "stale parse replaced a newer revision");
                previous_revision = snapshot.revision;
                if finished.load(Ordering::Acquire) == 2 {
                    break;
                }
                assert!(Instant::now() < deadline, "bounded publishers did not finish");
                std::thread::yield_now();
            }
            first.join().unwrap();
            second.join().unwrap();
        });
        // No additional publisher or source event repairs the final state.
        let final_snapshot = view.presentation();
        assert_eq!(final_snapshot.revision, view.transcript_revision());
        assert_coherent(&final_snapshot, &expected);
        assert_eq!(original.blocks[0].text, original_source);
        assert!(original.markdown.find(&original_source, true).is_some());
    }

    #[test]
    fn readers_keep_the_previous_coherent_snapshot_while_a_publisher_is_queued() {
        use std::sync::mpsc;
        use std::time::Duration;

        let view = Arc::new(view());
        view.hydrate(vec![Message::text("message", Role::Assistant, 1, "Original")]);
        view.prepare_markdown();
        let original = view.presentation();
        // Use the actual serialization lock as a deterministic barrier. No
        // test-only production API or timing-dependent parser pause is needed.
        let preparation = view.markdown_preparation.lock().unwrap();
        view.hydrate(vec![Message::text("message", Role::Assistant, 1, "Correction")]);
        let (started_tx, started_rx) = mpsc::channel();
        let writer_view = view.clone();
        let writer = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            writer_view.prepare_markdown();
        });
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let reader_view = view.clone();
        let reader = std::thread::spawn(move || {
            snapshot_tx.send(reader_view.presentation()).unwrap();
        });
        let observed = snapshot_rx.recv_timeout(Duration::from_secs(1));
        // Always release the barrier before asserting/joining, including when
        // a regression makes the reader wait for the blocked publisher.
        drop(preparation);
        writer.join().unwrap();
        reader.join().unwrap();
        let observed = observed.expect("UI snapshot read waited for publication work");
        assert!(Arc::ptr_eq(&observed, &original));
        assert!(observed.markdown.find("Original", false).is_some());
        let final_snapshot = view.presentation();
        assert_eq!(final_snapshot.revision, view.transcript_revision());
        assert_eq!(final_snapshot.revision, final_snapshot.markdown.revision);
        assert!(final_snapshot.markdown.find("Correction", false).is_some());
    }
}
