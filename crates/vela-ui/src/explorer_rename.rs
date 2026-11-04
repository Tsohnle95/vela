//! Rename reserves document paths before serialized filesystem mutation.
use super::*;
use vela_core::document::RelocationError;
use vela_engine::files::RenameEntryRequest;

pub(super) struct Menu {
    workspace: String,
    rel: String,
    is_dir: bool,
    position: gpui::Point<gpui::Pixels>,
    pub(super) return_focus: Option<gpui::FocusHandle>,
}
fn remap(rel: &str, from: &str, to: &str) -> String {
    if rel == from
        || rel
            .strip_prefix(from)
            .is_some_and(|tail| tail.starts_with('/'))
    {
        format!("{to}{}", &rel[from.len()..])
    } else {
        rel.to_owned()
    }
}
fn remap_key(key: DocumentKey, workspace: &str, from: &str, to: &str) -> DocumentKey {
    if key.directory == workspace {
        DocumentKey::in_workspace(key.workspace, key.directory, remap(&key.rel, from, to))
    } else {
        key
    }
}
impl VelaApp {
    pub(super) fn save_document_key(
        &mut self,
        key: DocumentKey,
        session: String,
        cx: &mut Context<Self>,
    ) {
        if self.documents.borrow().relocation_contains(&key) {
            self.relocation_saves.insert(key, session);
            cx.notify();
            return;
        }
        self.file_request += 1;
        let request = self
            .documents
            .borrow_mut()
            .save(self.file_request, session, &key);
        if let Some(request) = request {
            self.dispatch_save(request);
        }
        cx.notify();
    }
    pub(super) fn drain_relocation_saves(&mut self, cx: &mut Context<Self>) {
        for (key, session) in std::mem::take(&mut self.relocation_saves) {
            self.save_document_key(key, session, cx);
        }
    }
    pub(super) fn pump_entry_renames(&mut self, cx: &mut Context<Self>) -> bool {
        let mut ids = self.rename_waiting.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        let mut changed = false;
        for id in ids {
            let request = self.rename_waiting[&id].clone();
            let parent = request.target_parent.as_deref().unwrap_or_else(|| {
                request
                    .rel
                    .rsplit_once('/')
                    .map(|(parent, _)| parent)
                    .unwrap_or("")
            });
            let name = if request.target_parent.is_some() {
                request.rel.rsplit('/').next().unwrap_or("")
            } else {
                &request.name
            };
            let target = if parent.is_empty() {
                name.to_owned()
            } else {
                format!("{parent}/{name}")
            };
            let result = self.documents.borrow_mut().begin_relocation(
                id,
                DocumentKey::new(request.workspace.clone(), request.rel.clone()),
                DocumentKey::new(request.workspace.clone(), target),
            );
            match result {
                Err(RelocationError::Busy) => continue,
                Err(error) => {
                    self.rename_waiting.remove(&id);
                    changed = true;
                    if let Some(form) = &mut self.pending_create
                        && form.request == Some(id)
                    {
                        form.request = None;
                        form.waiting = false;
                        form.error = Some(error.to_string());
                    }
                    self.toasts
                        .update(cx, |toasts, cx| toasts.push(error.to_string(), true, cx));
                }
                Ok(()) => {
                    self.rename_waiting.remove(&id);
                    changed = true;
                    if let Some(form) = &mut self.pending_create
                        && form.request == Some(id)
                    {
                        form.waiting = false;
                    }
                    self.rename_requests.insert(id, request.clone());
                    self.engine.rename_entry(request);
                }
            }
        }
        changed |= self.pump_entry_removals(cx);
        changed |= self.pump_entry_detachments(cx);
        if changed {
            cx.notify();
        }
        changed
    }
    pub(super) fn finish_entry_rename(
        &mut self,
        request: RenameEntryRequest,
        new_rel: Option<String>,
        snapshots: Vec<vela_engine::files::RenameSnapshot>,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.rename_requests.remove(&request.id).is_none() {
            return;
        }
        let moved = self
            .documents
            .borrow_mut()
            .finish_relocation(request.id, error.is_none());
        let owns_form = self
            .pending_create
            .as_ref()
            .is_some_and(|form| form.request == Some(request.id));
        if let Some(error) = error {
            if owns_form {
                let form = self.pending_create.as_mut().unwrap();
                form.request = None;
                form.waiting = false;
                form.error = Some(error.clone());
            }
            if self.active_id.as_deref() == Some(request.session.as_str()) {
                self.engine.load_recovery(request.session.clone());
            }
            self.toasts
                .update(cx, |toasts, cx| toasts.push(error, true, cx));
        } else if let Some(new_rel) = new_rel {
            for review in self.change_reviews.values() {
                if review.read(cx).directory == request.workspace {
                    review.update(cx, |view, cx| view.relocate(&request.rel, &new_rel, cx));
                }
            }
            if owns_form {
                self.pending_create = None;
            }
            if let Err(error) = moved {
                self.toasts.update(cx, |toasts, cx| {
                    toasts.push(
                        format!("Renamed on disk; editor contents retained: {error}"),
                        true,
                        cx,
                    )
                });
            } else {
                let keys = self
                    .editor_views
                    .keys()
                    .filter(|key| {
                        key.directory == request.workspace
                            && remap(&key.rel, &request.rel, &new_rel) != key.rel
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                for old in keys {
                    let new = remap_key(old.clone(), &request.workspace, &request.rel, &new_rel);
                    if let Some(editor) = self.editor_views.remove(&old) {
                        editor.update(cx, |editor, cx| {
                            editor.key = new.clone();
                            cx.notify();
                        });
                        self.editor_views.entry(new).or_insert(editor);
                    }
                }
                self.close_after_save = std::mem::take(&mut self.close_after_save)
                    .into_iter()
                    .map(|key| remap_key(key, &request.workspace, &request.rel, &new_rel))
                    .collect();
                self.relocation_saves = std::mem::take(&mut self.relocation_saves)
                    .into_iter()
                    .map(|(key, session)| {
                        (
                            remap_key(key, &request.workspace, &request.rel, &new_rel),
                            session,
                        )
                    })
                    .collect();
                if let Some(tree) = self.file_trees.get_mut(&request.workspace) {
                    tree.relocate(&request.rel, &new_rel);
                }
                self.directory_requests
                    .retain(|_, key| key.directory != request.workspace);
                if self.workspace_directory() == request.workspace {
                    self.expanded = std::mem::take(&mut self.expanded)
                        .into_iter()
                        .map(|rel| remap(&rel, &request.rel, &new_rel))
                        .collect();
                    self.children.clear();
                    self.request_directory(String::new());
                    if owns_form && self.sidebar_tab == SidebarTab::Files && !self.settings_open {
                        self.editor_focus_pending = true;
                    }
                }
                let missing = self
                    .documents
                    .borrow()
                    .unloaded_keys_in_directory(&request.workspace);
                for key in missing {
                    self.file_request += 1;
                    if self
                        .documents
                        .borrow_mut()
                        .load_if_missing(self.file_request, key.clone())
                    {
                        self.engine.read_document(self.file_request, key);
                    }
                }
            }
        }
        for snapshot in snapshots {
            let keys = self
                .documents
                .borrow()
                .keys_at_path(&request.workspace, &snapshot.rel);
            for key in keys {
                if self.documents.borrow_mut().observe_relocation(
                    &key,
                    snapshot.change.clone(),
                    snapshot.source_unchanged,
                ) && let Some(editor) = self.editor_views.get(&key)
                {
                    editor.update(cx, |_, cx| cx.notify());
                }
            }
        }
        self.drain_relocation_saves(cx);
        cx.notify();
    }
    pub(super) fn open_explorer_menu(
        &mut self,
        workspace: String,
        rel: String,
        is_dir: bool,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if workspace != self.workspace_directory() {
            return;
        }
        self.saved_workspace_menu = None;
        self.session_context_menu = None;
        self.preview_menu = None;
        self.dismiss_tab_overflow();
        self.explorer_menu = Some(Menu {
            workspace,
            rel,
            is_dir,
            position,
            return_focus: window.focused(cx),
        });
        window.focus(&self.explorer_menu_focus[0], cx);
        cx.notify();
    }
    pub(super) fn navigate_explorer_menu(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(menu) = &self.explorer_menu else {
            return false;
        };
        let count = if menu.rel.is_empty() { 5 } else { 8 };
        match event.keystroke.key.as_str() {
            "escape" => {
                let focus = self.explorer_menu.take().and_then(|menu| menu.return_focus);
                window.focus(focus.as_ref().unwrap_or(&self.composer_focus), cx);
            }
            "down" | "up" | "tab" => {
                let current = self
                    .explorer_menu_focus
                    .iter()
                    .position(|focus| focus.is_focused(window))
                    .unwrap_or(0);
                let backwards = event.keystroke.key == "up"
                    || (event.keystroke.key == "tab" && event.keystroke.modifiers.shift);
                let next = if backwards {
                    (current + count - 1) % count
                } else {
                    (current + 1) % count
                };
                window.focus(&self.explorer_menu_focus[next], cx);
            }
            _ => return false,
        }
        cx.notify();
        cx.stop_propagation();
        true
    }
    pub(super) fn render_explorer_menu(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(menu) = &self.explorer_menu else {
            return div().into_any_element();
        };
        let theme = Theme::default();
        let size = window.viewport_size();
        let mut body = div()
            .id("explorer-context-menu")
            .role(Role::Menu)
            .absolute()
            .left(
                menu.position
                    .x
                    .max(px(4.))
                    .min((size.width - px(194.)).max(px(4.))),
            )
            .top(
                menu.position
                    .y
                    .max(px(4.))
                    .min((size.height - px(210.)).max(px(4.))),
            )
            .w(px(190.))
            .p(px(5.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.elevated)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        for (index, label) in [
            "Open in Integrated Terminal",
            "New File",
            "New Folder",
            "Reveal in File Manager",
            "Remove Workspace",
            "Rename",
            "Remove from Workspace",
            "Delete",
        ]
        .into_iter()
        .enumerate()
        {
            if index >= 5 && menu.rel.is_empty() {
                continue;
            }
            let label = if index == 7 {
                let paths = self
                    .file_trees
                    .get(&menu.workspace)
                    .map(|tree| tree.drag_paths(&menu.rel))
                    .unwrap_or_else(|| vec![menu.rel.clone()]);
                let count = super::explorer::compact_paths(&paths).len();
                if count > 1 {
                    format!("Delete {count} Items")
                } else {
                    label.to_owned()
                }
            } else {
                label.to_owned()
            };
            let workspace = menu.workspace.clone();
            let rel = menu.rel.clone();
            let is_dir = menu.is_dir;
            body = body.child(
                div()
                    .id(format!("explorer-menu-{index}"))
                    .role(Role::MenuItem)
                    .aria_label(label.clone())
                    .track_focus(&self.explorer_menu_focus[index])
                    .px(px(10.))
                    .py(px(7.))
                    .rounded(px(4.))
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected))
                    .on_click(cx.listener(move |app, _, window, cx| {
                        cx.stop_propagation();
                        if index == 6 {
                            app.confirm_entry_detachment(
                                workspace.clone(),
                                rel.clone(),
                                window,
                                cx,
                            );
                            return;
                        }
                        if let Some(focus) = app
                            .explorer_menu
                            .as_ref()
                            .and_then(|menu| menu.return_focus.clone())
                        {
                            window.focus(&focus, cx);
                        }
                        app.explorer_menu = None;
                        if index == 0 {
                            if let Some(session) = app.active_id.clone() {
                                let parent = if is_dir {
                                    rel.clone()
                                } else {
                                    rel.rsplit_once('/')
                                        .map(|(parent, _)| parent)
                                        .unwrap_or("")
                                        .to_owned()
                                };
                                app.file_request += 1;
                                let name = Path::new(&parent)
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned());
                                app.terminal_directory_requests.insert(
                                    app.file_request,
                                    terminal_state::DirectoryRequest {
                                        session: session.clone(),
                                        workspace: workspace.clone(),
                                        generation: app.terminals.generation,
                                        name,
                                    },
                                );
                                app.engine.resolve_terminal_directory(
                                    app.file_request,
                                    session,
                                    workspace.clone(),
                                    parent,
                                );
                            }
                        } else if index == 3 {
                            if let Some(session) = app.active_id.clone() {
                                app.file_request += 1;
                                app.reveal_requests.insert(app.file_request);
                                app.engine.resolve_entry(
                                    app.file_request,
                                    session,
                                    workspace.clone(),
                                    rel.clone(),
                                );
                            }
                        } else if index == 7 {
                            app.start_remove_entry(workspace.clone(), rel.clone(), cx);
                        } else if index == 4 {
                            app.request_workspace_close(workspace.clone(), cx);
                        } else if index == 5 {
                            app.start_rename_entry(
                                workspace.clone(),
                                rel.clone(),
                                is_dir,
                                window,
                                cx,
                            );
                        } else {
                            let parent = if is_dir {
                                rel.clone()
                            } else {
                                rel.rsplit_once('/')
                                    .map(|(parent, _)| parent)
                                    .unwrap_or("")
                                    .to_owned()
                            };
                            app.start_create_entry(
                                workspace.clone(),
                                parent,
                                index == 2,
                                window,
                                cx,
                            );
                        }
                    }))
                    .child(label),
            );
        }
        div()
            .id("explorer-menu-layer")
            .absolute()
            .size_full()
            .occlude()
            .on_click(cx.listener(|app, _, _, cx| {
                app.explorer_menu = None;
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|app, _, _, cx| {
                    app.explorer_menu = None;
                    cx.notify();
                }),
            )
            .child(body)
            .into_any_element()
    }
}

/// A keyboard-reachable equivalent of the Source row's ellipsis button.
pub(super) fn menu_action(
    workspace: String,
    rel: String,
    is_dir: bool,
    focus: gpui::FocusHandle,
    owner: gpui::WeakEntity<VelaApp>,
) -> AnyElement {
    let theme = Theme::default();
    div()
        .id(SharedString::from(format!("explorer-more-{rel}")))
        .role(Role::Button)
        .aria_label("More actions…")
        .track_focus(&focus)
        .px(px(3.))
        .font_family("codicon")
        .text_size(px(14.))
        .text_color(theme.faint)
        .cursor_pointer()
        .hover(|style| style.text_color(theme.text))
        .focus(|style| style.bg(theme.selected).text_color(theme.text))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            window.focus(&focus, cx);
            let _ = owner.update(cx, |app, cx| {
                app.open_explorer_menu(
                    workspace.clone(),
                    rel.clone(),
                    is_dir,
                    event.position(),
                    window,
                    cx,
                );
            });
        })
        .child(icon("ea7c"))
        .into_any_element()
}
