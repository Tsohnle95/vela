//! A workspace closes after its captured edits are acknowledged, never by
//! deleting its runtime history, bookmark, or independently owned documents.
use super::*;

struct Captured {
    key: DocumentKey,
    revision: u64,
    text: String,
}
struct Snapshot {
    workspace: String,
    files: Vec<Captured>,
}
impl Snapshot {
    fn capture(documents: &Documents, workspace: &str) -> Result<Self, &'static str> {
        let mut files = Vec::new();
        for key in documents.workspace_modified_keys(workspace) {
            let document = documents.get(&key).expect("retained document");
            if document.conflict.is_some() {
                return Err("Resolve external file conflicts before closing this workspace.");
            }
            files.push(Captured {
                key,
                revision: document.buffer.revision(),
                text: document.buffer.text().to_owned(),
            });
        }
        Ok(Self {
            workspace: workspace.into(),
            files,
        })
    }
    fn versions_current(&self, documents: &Documents) -> bool {
        self.files.iter().all(|captured| {
            documents.get(&captured.key).is_some_and(|document| {
                document.buffer.revision() == captured.revision
                    && document.buffer.text() == captured.text
                    && document.conflict.is_none()
            })
        }) && documents
            .workspace_modified_keys(&self.workspace)
            .iter()
            .all(|key| self.files.iter().any(|captured| captured.key == *key))
    }
    fn saved(&self, documents: &Documents) -> bool {
        self.versions_current(documents)
            && documents
                .workspace_modified_keys(&self.workspace)
                .is_empty()
    }
}
pub(super) struct Request {
    id: u64,
    snapshot: Snapshot,
    directory: String,
    pending: Option<HashSet<u64>>,
    failed: bool,
}

impl VelaApp {
    pub(super) fn request_workspace_close(&mut self, workspace: String, cx: &mut Context<Self>) {
        if workspace != self.workspace_directory() || self.closing.is_some() {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            return;
        };
        if self.workspace_closures.contains_key(&session) {
            return;
        }
        let snapshot = match Snapshot::capture(&self.documents.borrow(), &self.document_workspace())
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.toasts
                    .update(cx, |toasts, cx| toasts.push(error, true, cx));
                return;
            }
        };
        self.file_request += 1;
        let id = self.file_request;
        self.workspace_closures.insert(
            session.clone(),
            Request {
                id,
                snapshot,
                directory: workspace,
                pending: None,
                failed: false,
            },
        );
        cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(30))
                .await;
            let _ = owner.update(cx, |app, cx| {
                if app
                    .workspace_closures
                    .get(&session)
                    .is_some_and(|request| request.id == id)
                {
                    app.workspace_closures.remove(&session);
                    app.toasts.update(cx, |toasts, cx| {
                        toasts.push(
                            "Workspace stayed open because file operations did not finish.",
                            true,
                            cx,
                        )
                    });
                    cx.notify();
                }
            });
        })
        .detach();
        self.advance_workspace_closes(cx);
        cx.notify();
    }
    pub(super) fn note_workspace_close_result(&mut self, update: &EngineUpdate) {
        if let EngineUpdate::FileSaved {
            request_id,
            error,
            conflict,
            ..
        } = update
        {
            for request in self.workspace_closures.values_mut() {
                if request
                    .pending
                    .as_mut()
                    .is_some_and(|pending| pending.remove(request_id))
                    && (error.is_some() || conflict.is_some())
                {
                    request.failed = true;
                }
            }
        }
    }
    fn workspace_explorer_busy(&self, workspace: &str) -> bool {
        self.create_requests
            .values()
            .any(|request| request.workspace == workspace)
            || self
                .import_requests
                .values()
                .any(|request| request.workspace == workspace)
            || self
                .rename_waiting
                .values()
                .chain(self.rename_requests.values())
                .any(|request| request.workspace == workspace)
            || self
                .remove_waiting
                .values()
                .chain(self.remove_requests.values())
                .any(|request| request.workspace == workspace)
            || self
                .detach_waiting
                .values()
                .chain(self.detach_requests.values())
                .any(|request| request.workspace == workspace)
    }
    pub(super) fn advance_workspace_closes(&mut self, cx: &mut Context<Self>) {
        let mut sessions = self.workspace_closures.keys().cloned().collect::<Vec<_>>();
        sessions.sort();
        for session in sessions {
            let mut request = self
                .workspace_closures
                .remove(&session)
                .expect("pending workspace close");
            if self.workspace_explorer_busy(&request.directory)
                || self
                    .documents
                    .borrow()
                    .workspace_busy(&request.snapshot.workspace)
            {
                self.workspace_closures.insert(session, request);
                continue;
            }
            if request.failed || !request.snapshot.versions_current(&self.documents.borrow()) {
                self.toasts.update(cx, |toasts, cx| {
                    toasts.push(
                        "Workspace stayed open because its files changed or could not be saved.",
                        true,
                        cx,
                    )
                });
                cx.notify();
                continue;
            }
            if request.pending.is_none() {
                let mut pending = HashSet::new();
                for captured in &request.snapshot.files {
                    if !self
                        .documents
                        .borrow()
                        .get(&captured.key)
                        .is_some_and(|document| document.dirty())
                    {
                        continue;
                    }
                    self.file_request += 1;
                    let id = self.file_request;
                    pending.insert(id);
                    let save = self
                        .documents
                        .borrow_mut()
                        .save(id, session.clone(), &captured.key);
                    if let Some(save) = save {
                        self.dispatch_save(save);
                    } else {
                        request.failed = true;
                    }
                }
                request.pending = Some(pending);
            }
            if request
                .pending
                .as_ref()
                .is_some_and(|pending| !pending.is_empty())
                || self
                    .documents
                    .borrow()
                    .workspace_busy(&request.snapshot.workspace)
            {
                self.workspace_closures.insert(session, request);
                cx.notify();
                continue;
            }
            if request.failed || !request.snapshot.saved(&self.documents.borrow()) {
                self.toasts.update(cx, |toasts, cx| {
                    toasts.push(
                        "Workspace stayed open because its edits were not saved.",
                        true,
                        cx,
                    )
                });
                cx.notify();
                continue;
            }
            if !self
                .documents
                .borrow_mut()
                .close_workspace(&request.snapshot.workspace)
            {
                self.workspace_closures.insert(session, request);
                continue;
            }
            self.editor_views
                .retain(|key, _| key.workspace != request.snapshot.workspace);
            self.close_after_save
                .retain(|key| key.workspace != request.snapshot.workspace);
            self.change_reviews.remove(&request.snapshot.workspace);
            self.document_scopes.remove(&session);
            self.release_unused_document_workspaces();
            self.finish_workspace_close(&session, &request.directory, cx);
        }
    }
    fn finish_workspace_close(&mut self, session: &str, workspace: &str, cx: &mut Context<Self>) {
        if self.active_id.as_deref() == Some(session) && self.workspace_directory() == workspace {
            self.save_view_state();
            self.workspace_closed = true;
            if self.terminals.session.as_deref() == Some(session) {
                for terminal in self.terminals.reset(String::new(), None) {
                    self.engine.stop_terminal(terminal.id);
                }
                self.tray.close();
                self.terminal_focus_pending = false;
            }
            self.terminal_directory_requests
                .retain(|_, request| request.session != session);
            self.preview_pending
                .retain(|_, owner| owner.as_deref() != Some(session));
            self.browser_open = false;
            if let Some(browser) = &self.browser {
                browser.set_visible(false);
            }
            let sidebar = self.sidebar_tab;
            self.clear_active_session_ui(session, cx);
            self.sidebar_tab = sidebar;
            if self.settings.last_session.as_deref() == Some(session) {
                self.settings.last_session = None;
            }
            self.persist_preferences(cx);
        }
        self.engine.close_session(session);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn load(documents: &mut Documents, id: u64, key: &DocumentKey, text: &str) {
        documents.open(id, key.clone(), true);
        documents
            .finish_load(id, &key.rel, Some(text.into()))
            .unwrap();
    }
    #[test]
    fn captured_close_requires_exact_acknowledged_edits_and_ignores_same_directory_context() {
        let mut documents = Documents::default();
        let a = DocumentKey::in_workspace("/a".into(), "/shared".into(), "one".into());
        let b = DocumentKey::in_workspace("/b".into(), "/shared".into(), "one".into());
        load(&mut documents, 1, &a, "before");
        load(&mut documents, 2, &b, "other");
        documents.edit(&a, |buffer| buffer.insert(" edit"));
        documents.edit(&b, |buffer| buffer.insert(" retained"));
        let snapshot = Snapshot::capture(&documents, "/a").unwrap();
        assert_eq!(snapshot.files.len(), 1);
        assert!(!snapshot.saved(&documents));
        let save = documents.save(3, "owner-a".into(), &a).unwrap();
        assert!(documents.workspace_busy("/a"));
        assert!(!documents.workspace_busy("/b"));
        documents.finish_save(3, &a.rel, true).unwrap();
        assert!(snapshot.saved(&documents));
        assert!(documents.get(&b).unwrap().dirty());
        assert_eq!(documents.get(&a).unwrap().buffer.text(), save.text);
        documents.edit(&a, |buffer| buffer.insert(" newer"));
        assert!(!snapshot.versions_current(&documents));
        documents.edit(&a, |buffer| buffer.undo());
        assert!(
            !snapshot.versions_current(&documents),
            "undo must not erase the intervening revision"
        );
    }
    #[test]
    fn conflicts_new_edits_and_failed_writes_cannot_close_a_workspace() {
        let mut documents = Documents::default();
        let a = DocumentKey::new("/a".into(), "one".into());
        let newer = DocumentKey::new("/a".into(), "newer".into());
        load(&mut documents, 1, &a, "before");
        documents.edit(&a, |buffer| buffer.insert(" edit"));
        let snapshot = Snapshot::capture(&documents, "/a").unwrap();
        documents.save(2, "owner".into(), &a).unwrap();
        documents.finish_save(2, &a.rel, false).unwrap();
        assert!(!snapshot.saved(&documents));
        load(&mut documents, 3, &newer, "new");
        documents.edit(&newer, |buffer| buffer.insert(" edit"));
        assert!(!snapshot.versions_current(&documents));
        documents.observe(
            &a,
            ExternalChange {
                deleted: true,
                ..Default::default()
            },
            None,
        );
        assert!(Snapshot::capture(&documents, "/a").is_err());
        assert!(documents.get(&a).unwrap().dirty());
        assert!(documents.get(&a).unwrap().deleted);
    }
}
