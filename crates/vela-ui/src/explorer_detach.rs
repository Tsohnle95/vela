//! Detach reserves paths like Trash, but keeps editors, edits and undo.
use super::*;
use vela_core::document::RelocationError;
impl VelaApp {
    pub(super) fn confirm_entry_detachment(
        &mut self,
        workspace: String,
        rel: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if rel.is_empty()
            || workspace != self.workspace_directory()
            || self.detach_confirmation.is_some()
            || self.closing.is_some()
        {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            return;
        };
        self.file_request += 1;
        let request = vela_engine::detach::Request {
            id: self.file_request,
            session,
            workspace,
            rel,
        };
        self.detach_confirmation = Some(request.id);
        let cancel_focus = window.focused(cx);
        let confirmation = crate::platform_prompt::document(
            window,
            &format!(
                "Remove \"{}\" from Vela? The item will remain on disk.",
                request.rel
            ),
            None,
            &[
                gpui::PromptButton::ok("OK"),
                gpui::PromptButton::cancel("Cancel"),
            ],
            cx,
        );
        cx.spawn_in(window, async move |app, cx| {
            let accepted = confirmation.await == 0;
            let _ = app.update_in(cx, |app, window, cx| {
                if app.detach_confirmation != Some(request.id) {
                    return;
                }
                app.detach_confirmation = None;
                if accepted
                    && app.workspace_directory() == request.workspace
                    && app.active_id.as_deref() == Some(request.session.as_str())
                {
                    let focus = app.explorer_menu.take().and_then(|menu| menu.return_focus);
                    if let Some(focus) = focus {
                        window.focus(&focus, cx);
                    }
                    app.detach_waiting.insert(request.id, request);
                    app.pump_entry_detachments(cx);
                } else if let Some(focus) = cancel_focus {
                    window.focus(&focus, cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub(super) fn pump_entry_detachments(&mut self, cx: &mut Context<Self>) -> bool {
        let mut ids = self.detach_waiting.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        let mut changed = false;
        for id in ids {
            let request = self.detach_waiting[&id].clone();
            let result = self.documents.borrow_mut().begin_removal(
                id,
                DocumentKey::new(request.workspace.clone(), request.rel.clone()),
            );
            match result {
                Err(RelocationError::Busy) => continue,
                Err(error) => {
                    self.detach_waiting.remove(&id);
                    self.toasts
                        .update(cx, |toasts, cx| toasts.push(error.to_string(), true, cx));
                }
                Ok(()) => {
                    self.detach_waiting.remove(&id);
                    self.detach_requests.insert(id, request.clone());
                    self.engine.detach_entry(request);
                }
            }
            changed = true;
        }
        if changed {
            cx.notify();
        }
        changed
    }
    pub(super) fn finish_entry_detachment(
        &mut self,
        request: vela_engine::detach::Request,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.detach_requests.remove(&request.id).is_none() {
            return;
        }
        let success = error.is_none();
        let result = self
            .documents
            .borrow_mut()
            .finish_detachment(request.id, success);
        if let Some(error) = error {
            self.toasts
                .update(cx, |toasts, cx| toasts.push(error, true, cx));
        }
        match result {
            Err(error) => self
                .toasts
                .update(cx, |toasts, cx| toasts.push(error.to_string(), true, cx)),
            Ok(keys) => {
                for key in keys {
                    if let Some(editor) = self.editor_views.get(&key) {
                        editor.update(cx, |_, cx| cx.notify());
                    }
                }
            }
        }
        if success {
            if let Some(scope) = self.document_scopes.get(&request.session)
                && let Some(review) = self.change_reviews.get(scope)
            {
                review.update(cx, |view, cx| view.hide(request.rel.clone(), cx));
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
                self.refresh_files(cx);
            }
        }
        self.drain_relocation_saves(cx);
        cx.notify();
    }
}
