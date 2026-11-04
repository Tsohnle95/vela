//! Explicit Trash actions reserve document prefixes until the OS acknowledges.
use super::*;
use vela_core::document::RelocationError;
use vela_engine::files::RemoveEntryRequest;

impl VelaApp {
    pub(super) fn start_remove_entry(
        &mut self,
        workspace: String,
        rel: String,
        cx: &mut Context<Self>,
    ) {
        if rel.is_empty() || self.workspace_directory() != workspace {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            return;
        };
        let paths = self
            .file_trees
            .get(&workspace)
            .map(|tree| tree.drag_paths(&rel))
            .unwrap_or_else(|| vec![rel]);
        if let Some(tree) = self.file_trees.get_mut(&workspace) {
            tree.clear_selection();
        }
        let batch = self.file_request + 1;
        for rel in super::explorer::compact_paths(&paths) {
            self.file_request += 1;
            self.remove_waiting.insert(
                self.file_request,
                RemoveEntryRequest {
                    id: self.file_request,
                    batch,
                    session: session.clone(),
                    workspace: workspace.clone(),
                    rel,
                },
            );
        }
        self.pump_entry_removals(cx);
        cx.notify();
    }
    pub(super) fn pump_entry_removals(&mut self, cx: &mut Context<Self>) -> bool {
        let mut ids = self.remove_waiting.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        let mut changed = false;
        for id in ids {
            let request = self.remove_waiting[&id].clone();
            let result = self.documents.borrow_mut().begin_removal(
                id,
                DocumentKey::new(request.workspace.clone(), request.rel.clone()),
            );
            match result {
                Err(RelocationError::Busy) => continue,
                Err(error) => {
                    self.remove_waiting.remove(&id);
                    self.record_removal(request, Some(error.to_string()), false, cx);
                }
                Ok(()) => {
                    self.remove_waiting.remove(&id);
                    self.remove_requests.insert(id, request.clone());
                    self.engine.remove_entry(request);
                }
            }
            changed = true;
        }
        changed
    }
    pub(super) fn finish_entry_removal(
        &mut self,
        request: RemoveEntryRequest,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.remove_requests.remove(&request.id).is_none() {
            return;
        }
        self.record_removal(request, error, true, cx);
    }
    fn record_removal(
        &mut self,
        request: RemoveEntryRequest,
        error: Option<String>,
        reserved: bool,
        cx: &mut Context<Self>,
    ) {
        let batch = request.batch;
        let success = error.is_none();
        if let Some(error) = error {
            self.toasts
                .update(cx, |toasts, cx| toasts.push(error, true, cx));
        }
        self.removal_batches
            .entry(batch)
            .or_default()
            .push((request, success, reserved));
        if self
            .remove_waiting
            .values()
            .chain(self.remove_requests.values())
            .any(|request| request.batch == batch)
        {
            cx.notify();
            return;
        }
        let results = self.removal_batches.remove(&batch).unwrap();
        let acknowledgements = results
            .iter()
            .filter(|(_, _, reserved)| *reserved)
            .map(|(request, success, _)| (request.id, *success))
            .collect::<Vec<_>>();
        let completed = self
            .documents
            .borrow_mut()
            .finish_removals(&acknowledgements);
        match completed {
            Err(error) => self
                .toasts
                .update(cx, |toasts, cx| toasts.push(error.to_string(), true, cx)),
            Ok(keys) => {
                for key in keys {
                    self.editor_views.remove(&key);
                    self.close_after_save.remove(&key);
                }
                for (request, success, _) in results {
                    if !success {
                        continue;
                    }
                    let within = |rel: &str| {
                        rel == request.rel
                            || rel
                                .strip_prefix(&request.rel)
                                .is_some_and(|tail| tail.starts_with('/'))
                    };
                    self.relocation_saves
                        .retain(|key, _| key.directory != request.workspace || !within(&key.rel));
                    if let Some(tree) = self.file_trees.get_mut(&request.workspace) {
                        tree.remove_prefix(&request.rel);
                    }
                    self.directory_requests
                        .retain(|_, key| key.directory != request.workspace);
                    if self.workspace_directory() == request.workspace {
                        self.expanded.retain(|rel| !within(rel));
                        self.children.clear();
                        self.request_directory(String::new());
                    }
                }
                self.release_unused_document_workspaces();
            }
        }
        self.drain_relocation_saves(cx);
        cx.notify();
    }
}
