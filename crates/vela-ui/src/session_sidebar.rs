//! Session sidebar filtering and pinning share one retained, virtualized row renderer.
use super::*;

impl VelaApp {
    pub(super) fn finish_session_deletion(
        &mut self,
        request_id: u64,
        id: &str,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if !self
            .session_deletions
            .get(id)
            .is_some_and(|(request, _)| *request == request_id)
        {
            return;
        }
        let (_, title) = self.session_deletions.remove(id).unwrap();
        if error.is_none()
            && self.settings.compaction_baselines.remove(id).is_some()
        {
            self.persist_preferences(cx);
        }
        self.toasts.update(cx, |toasts, cx| match error {
            Some(error) => toasts.push(error, true, cx),
            None => toasts.push(format!("Deleted session {title}"), false, cx),
        });
        cx.notify();
    }

    /// Source deletes immediately from the context menu; its menu labels the
    /// action as permanent and shows no confirmation prompt.
    pub(super) fn session_title_for_delete(&self, id: &str) -> String {
        self.panels
            .entries()
            .iter()
            .find(|panel| panel.session == id)
            .map(|panel| panel.title.clone())
            .or_else(|| {
                self.sessions
                    .iter()
                    .find(|session| session.id == id)
                    .map(|session| session.display_title().to_owned())
            })
            .unwrap_or_else(|| id.to_owned())
    }

    /// Source asks `window.confirm` before deleting; use the native alert with
    /// the same title, detail and OK/Cancel order.
    pub(super) fn request_session_deletion(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session_deletions.contains_key(&id) || self.session_delete_prompt.is_some() {
            return;
        }
        let title = self.session_title_for_delete(&id);
        self.session_delete_prompt = Some(id.clone());
        let confirmation = crate::platform_prompt::document(
            window,
            &format!("Delete session \"{title}\"?"),
            Some("This permanently removes it from opencode and cannot be undone."),
            &[
                gpui::PromptButton::ok("OK"),
                gpui::PromptButton::cancel("Cancel"),
            ],
            cx,
        );
        cx.spawn(async move |this, cx| {
            let accepted = confirmation.await == 0;
            let _ = this.update(cx, |app, cx| {
                if app.session_delete_prompt.as_deref() != Some(id.as_str()) {
                    return;
                }
                app.session_delete_prompt = None;
                if accepted {
                    app.begin_session_deletion(id, cx);
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn begin_session_deletion(&mut self, id: String, cx: &mut Context<Self>) {
        if self.session_deletions.contains_key(&id) {
            return;
        }
        let title = self.session_title_for_delete(&id);
        if self.panels.contains(&id) {
            self.close_panel(id.clone(), cx);
        }
        self.file_request += 1;
        let request_id = self.file_request;
        self.session_deletions.insert(id.clone(), (request_id, title));
        self.engine.delete_session(request_id, id);
        cx.notify();
    }

    pub(super) fn navigate_session_context(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.session_context_menu.is_none() {
            return false;
        }
        match event.keystroke.key.as_str() {
            "escape" => {
                self.session_context_menu = None;
                window.focus(&self.session_filter_focus, cx);
            }
            "down" | "up" | "tab" => {
                let current = self
                    .session_context_focus
                    .iter()
                    .position(|focus| focus.is_focused(window))
                    .unwrap_or(0);
                window.focus(&self.session_context_focus[1 - current], cx);
            }
            _ => return false,
        }
        cx.notify();
        cx.stop_propagation();
        true
    }

    pub(super) fn render_session_context_menu(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some((id, position)) = &self.session_context_menu else {
            return div().into_any_element();
        };
        let theme = Theme::default();
        let size = window.viewport_size();
        let pinned = self.settings.pinned_sessions.contains(id);
        let pin_id = id.clone();
        let delete_id = id.clone();
        let pin_label = if pinned {
            "Unpin from Pinned"
        } else {
            "Pin chat"
        };
        let menu = div()
            .id("session-context-menu")
            .debug_selector(|| "session-context-menu".into())
            .role(Role::Menu)
            .absolute()
            .left(
                position
                    .x
                    .max(px(4.))
                    .min((size.width - px(194.)).max(px(4.))),
            )
            .top(
                position
                    .y
                    .max(px(4.))
                    .min((size.height - px(78.)).max(px(4.))),
            )
            .w(px(190.))
            .p(px(4.))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.elevated)
            .shadow(theme.large_shadow())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _, window, cx| {
                app.session_context_menu = None;
                window.focus(&app.session_filter_focus, cx);
                cx.notify();
            }))
            .child(
                div()
                    .id("session-menu-pin")
                    .role(Role::MenuItem)
                    .aria_label(pin_label)
                    .track_focus(&self.session_context_focus[0])
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(4.))
                    .text_size(px(11.5))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected))
                    .on_click(cx.listener(move |app, _, window, cx| {
                        cx.stop_propagation();
                        app.session_context_menu = None;
                        app.toggle_session_pin(&pin_id, cx);
                        window.focus(&app.session_filter_focus, cx);
                    }))
                    .child(pin_label),
            )
            .child(
                div()
                    .id("session-menu-delete")
                    .role(Role::MenuItem)
                    .aria_label("Delete session")
                    .track_focus(&self.session_context_focus[1])
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(4.))
                    .text_size(px(11.5))
                    .text_color(theme.red)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected))
                    .on_click(cx.listener(move |app, _, window, cx| {
                        cx.stop_propagation();
                        app.session_context_menu = None;
                        app.request_session_deletion(delete_id.clone(), window, cx);
                    }))
                    .child("Delete session"),
            );
        div()
            .id("session-context-layer")
            .absolute()
            .size_full()
            .occlude()
            .on_click(cx.listener(|app, _, _, cx| {
                app.session_context_menu = None;
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|app, _, _, cx| {
                    app.session_context_menu = None;
                    cx.notify();
                }),
            )
            .child(menu)
            .into_any_element()
    }

    pub(super) fn retire_session_ui(&mut self, id: &str, cx: &mut Context<Self>) {
        if self
            .session_context_menu
            .as_ref()
            .is_some_and(|(owner, _)| owner == id)
        {
            self.session_context_menu = None;
        }
        self.settings.pinned_sessions.remove(id);
        self.single_files.remove(id);
        self.revert_requests.retain(|_, (owner, _)| owner != id);
        self.staged_reverts.remove(id);
        self.commit_revert_requests.retain(|_, owner| owner != id);
        self.clear_revert_requests.retain(|_, owner| owner != id);
        self.form_requests.retain(|_, (owner, _)| owner != id);
        self.slash_requests.retain(|_, owner| owner != id);
        let live_requests = self
            .slash_requests
            .keys()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        self.slash_command_names
            .retain(|request, _| live_requests.contains(request));
        self.panel_close_saves.remove(id);
        if self
            .hidden_agent_anchor
            .as_ref()
            .is_some_and(|panel| panel.session == id)
        {
            self.hidden_agent_anchor = None;
        }
        self.clear_agent_menu_for(id);
        self.forget_panel_widgets(id);
        if self.session_delete_prompt.as_deref() == Some(id) {
            self.session_delete_prompt = None;
        }
        self.settings
            .recent_sessions
            .retain(|session| session != id);
        if self.settings.last_session.as_deref() == Some(id) {
            self.settings.last_session = None;
        }
        // Editor ownership outlives its conversation. Keep the workspace token
        // until Documents releases that workspace: Save All uses it even after
        // runtime deletion, and the retained observation lease saves by root.
        // Removing it here would also prevent release_unused_document_workspaces
        // from releasing the editor lease later.
        let was_active = self.active_id.as_deref() == Some(id);
        self.panels.close(id);
        self.clear_active_session_ui(id, cx);
        if self.agent_mode && self.panels.is_empty() {
            self.agent_mode = false;
            if let Some(open) = self.sidebar_before_agent_mode.take() {
                self.sidebar_open = open;
            }
        }
        // Workspace documents/recovery survive session deletion, including
        // dirty buffers; deleting a conversation must not discard source edits.
        self.sessions = Arc::new(self.engine.sessions());
        self.refresh_session_filter();
        self.persist_preferences(cx);
        if was_active && let Some(next) = self.panels.active().map(|panel| panel.session.clone()) {
            self.reopen_session(next, cx);
        }
        cx.notify();
    }

    pub(super) fn clear_active_session_ui(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
            self.workspace_hint = None;
            self.loading = false;
            self.attachments.clear();
            self.attachment_drag_over = false;
            self.composer.clear();
            self.composer_input
                .update(cx, |input, cx| input.set_text(String::new(), cx));
            self.blocks = Arc::new(Vec::new());
            self.transcript_rows = Arc::new(Vec::new());
            self.transcript_ui = Default::default();
            self.transcript_viewport = Default::default();
            self.transcript_list.reset(0);
            self.permissions.clear();
            self.todos.clear();
            self.usage = None;
            self.models.clear();
            self.agents.clear();
            self.selected_model = None;
            self.selected_agent = None;
            self.catalog_session = None;
            self.provider_catalog.clear();
            self.clear_provider_forms(cx);
            self.picker = None;
            self.inbox.clear();
            self.inbox_session = None;
            self.forms
                .update(cx, |dock, cx| dock.set_forms(Vec::new(), cx));
            self.forms_session = None;
            self.form_requests.clear();
            self.slash_requests.clear();
            self.command_catalog.clear();
            self.command_session = None;
            self.slash_selected = 0;
            self.reference_files.clear();
            self.reference_pending = None;
            self.reference_selected = 0;
            self.phase = SessionPhase::Idle;
            self.revision = u64::MAX;
            self.directory_requests.clear();
            self.status_request = None;
            self.diff_request = None;
            self.files.clear();
            self.children.clear();
            self.expanded.clear();
            self.changes.clear();
            self.diff = None;
            self.problems.clear();
            self.problem_path = None;
            self.validation_requests.clear();
            self.validation_summary = None;
            self.recovery_records.clear();
            self.usage_open = false;
            self.usage_anchor = None;
            self.sass = Default::default();
            self.sidebar_tab = SidebarTab::Sessions;
            self.composer_focus_pending = true;
        }
    }

    pub(super) fn choose_saved_workspace(&mut self, cx: &mut Context<Self>) {
        self.choose_workspace_dialog(true, cx);
    }

    pub(super) fn choose_workspace_dialog(&mut self, save: bool, cx: &mut Context<Self>) {
        if self.workspace_chooser_pending {
            return;
        }
        self.workspace_chooser_pending = true;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(
                if save {
                    "Save a workspace in Vela"
                } else {
                    "Open a workspace"
                }
                .into(),
            ),
        });
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let directory = match result {
                Ok(Ok(Some(paths))) => paths
                    .first()
                    .map(|path| path.to_string_lossy().to_string()),
                _ => None,
            };
            let _ = this.update(cx, |app, cx| {
                app.workspace_chooser_pending = false;
                app.finish_workspace_choice(save, directory, cx);
            });
        })
        .detach();
    }

    /// Applies a completed workspace chooser. A cancelled or dismissed chooser
    /// must clear the add-panel intent, otherwise the next workspace open
    /// silently attaches another agent panel.
    pub(super) fn finish_workspace_choice(
        &mut self,
        save: bool,
        directory: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(directory) = directory else {
            self.workspace_open_adds = false;
            return;
        };
        if save {
            let name = std::path::Path::new(&directory)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| directory.clone());
            if !self
                .settings
                .workspaces
                .iter()
                .any(|bookmark| bookmark.directory == directory)
            {
                self.settings
                    .workspaces
                    .push(vela_engine::migration::WorkspaceBookmark { directory, name });
                self.persist_preferences(cx);
            }
            // Saving a workspace never opens one; drop any stale add intent.
            self.workspace_open_adds = false;
            self.session_projects_open = true;
        } else {
            self.workspace_hint = Some(directory.clone());
            self.loading = true;
            self.engine.open_directory(directory);
        }
        cx.notify();
    }

    pub(super) fn refresh_session_filter(&mut self) {
        let needle = self.session_filter.trim().to_lowercase();
        let mut recent = Vec::new();
        let mut pinned = Vec::new();
        for (index, session) in self.sessions.iter().enumerate() {
            if !needle.is_empty() && !session.title.to_lowercase().contains(&needle) {
                continue;
            }
            if self.settings.pinned_sessions.contains(&session.id) {
                pinned.push(index);
            } else {
                recent.push(index);
            }
        }
        let open = self
            .panels
            .entries()
            .iter()
            .map(|panel| panel.session.clone())
            .collect::<HashSet<_>>();
        sort_session_indices(&self.sessions, &open, &mut recent);
        sort_session_indices(&self.sessions, &open, &mut pinned);
        self.filtered_sessions = Arc::new(recent);
        self.filtered_pinned_sessions = Arc::new(pinned);
        self.session_list.reset(self.filtered_sessions.len());
        self.pinned_session_list
            .reset(self.filtered_pinned_sessions.len());
    }

    fn toggle_session_pin(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.settings.pinned_sessions.remove(id) {
            self.settings.pinned_sessions.insert(id.to_owned());
        }
        self.persist_preferences(cx);
        self.refresh_session_filter();
        cx.notify();
    }

    pub(super) fn render_session_rows(
        &self,
        filtered: Arc<Vec<usize>>,
        state: ListState,
        cx: &mut Context<Self>,
    ) -> gpui::List {
        let sessions = self.sessions.clone();
        let active = self.active_id.clone();
        let weak = cx.weak_entity();
        let pinned = self.settings.pinned_sessions.clone();
        let open = self
            .panels
            .entries()
            .iter()
            .map(|panel| panel.session.clone())
            .collect::<HashSet<_>>();
        let pin_focus = self.session_pin_focus.clone();
        let codex = self.is_codex();
        list(state, move |ix, _window, cx| {
            let Some(session) = filtered.get(ix).and_then(|index| sessions.get(*index)) else {
                return div().into_any_element();
            };
            let focus = pin_focus
                .borrow_mut()
                .entry(session.id.clone())
                .or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            session_row(
                session,
                active.as_deref() == Some(session.id.as_str()),
                pinned.contains(&session.id),
                open.contains(&session.id),
                focus,
                None,
                codex,
                weak.clone(),
            )
        })
        .flex_1()
        .w_full()
    }
    pub(super) fn render_saved_workspace_menu(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some((directory, position)) = &self.saved_workspace_menu else {
            return div().into_any_element();
        };
        let directory = directory.clone();
        let theme = Theme::default();
        let size = window.viewport_size();
        let menu = div()
            .id("saved-workspace-menu")
            .debug_selector(|| "saved-workspace-menu".into())
            .role(Role::Menu)
            .absolute()
            .left(
                position
                    .x
                    .max(px(4.))
                    .min((size.width - px(194.)).max(px(4.))),
            )
            .top(
                position
                    .y
                    .max(px(4.))
                    .min((size.height - px(70.)).max(px(4.))),
            )
            .w(px(190.))
            .p(px(4.))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.elevated)
            .shadow(theme.large_shadow())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _, window, cx| {
                app.saved_workspace_menu = None;
                window.focus(&app.saved_workspace_menu_focus, cx);
                cx.notify();
            }))
            .child(
                div()
                    .id("remove-saved-workspace")
                    .role(Role::MenuItem)
                    .aria_label("Remove from Vela")
                    .track_focus(&self.saved_workspace_menu_focus)
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(4.))
                    .text_size(px(11.5))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected))
                    .on_click(cx.listener(move |app, _, window, cx| {
                        cx.stop_propagation();
                        app.settings
                            .workspaces
                            .retain(|bookmark| bookmark.directory != directory);
                        app.expanded_session_projects.remove(&directory);
                        app.saved_workspace_menu = None;
                        app.persist_preferences(cx);
                        window.focus(&app.session_filter_focus, cx);
                        cx.notify();
                    }))
                    .child("Remove from Vela"),
            );
        div()
            .id("saved-workspace-menu-layer")
            .absolute()
            .size_full()
            .occlude()
            .on_click(cx.listener(|app, _, _, cx| {
                app.saved_workspace_menu = None;
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|app, _, _, cx| {
                    app.saved_workspace_menu = None;
                    cx.notify();
                }),
            )
            .child(menu)
            .into_any_element()
    }

    pub(super) fn render_workspaces(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let weak = cx.weak_entity();
        let sessions = self.sessions.clone();
        let open_sessions = self
            .panels
            .entries()
            .iter()
            .map(|panel| panel.session.clone())
            .collect::<HashSet<_>>();
        let mut column = div().px(px(6.0)).flex().flex_col().gap(px(1.0));
        let mut project_height = self.settings.workspaces.len() as f32 * 27.;
        for (ix, bookmark) in self.settings.workspaces.iter().enumerate() {
            let directory = bookmark.directory.clone();
            let name = bookmark.name.clone();
            let mut history_indices = sessions
                .iter()
                .enumerate()
                .filter(|(_, session)| session.directory == directory)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            sort_session_indices(&sessions, &open_sessions, &mut history_indices);
            let history = history_indices
                .into_iter()
                .map(|index| &sessions[index])
                .collect::<Vec<_>>();
            let expanded = self.expanded_session_projects.contains(&directory);
            let toggle_directory = directory.clone();
            let row_toggle_directory = directory.clone();
            let menu_directory = directory.clone();
            let focus = self
                .session_project_focus
                .borrow_mut()
                .entry(directory.clone())
                .or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            let weak = weak.clone();
            column = column.child(crate::scroll::FocusReveal::element(
                format!("workspace-reveal-{ix}"),
                self.session_projects_scroll.clone(),
                focus.clone(),
                div()
                    .id(SharedString::from(format!("workspace-{ix}")))
                    .debug_selector({
                        let selector = format!("workspace-{ix}");
                        move || selector.clone()
                    })
                    .role(Role::Button)
                    .aria_label(SharedString::from(format!("{name} sessions")))
                    .track_focus(&focus)
                    .h(px(if self.is_codex() { 30.0 } else { 28.0 }))
                    .px(px(8.0))
                    .rounded(px(if self.is_codex() { 10.0 } else { 999.0 }))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .text_size(px(13.0))
                    .text_color(theme.dim)
                    .hover(|style| style.bg(theme.selected).text_color(theme.text))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |app, event: &gpui::MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            app.saved_workspace_menu =
                                Some((menu_directory.clone(), event.position));
                            window.focus(&app.saved_workspace_menu_focus, cx);
                            cx.notify();
                        }),
                    )
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if !app.expanded_session_projects.remove(&row_toggle_directory) {
                            app.expanded_session_projects
                                .insert(row_toggle_directory.clone());
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .id(SharedString::from(format!("project-open-{ix}")))
                            .debug_selector({
                                let selector = format!("project-open-{ix}");
                                move || selector.clone()
                            })
                            .role(Role::Button)
                            .aria_label(SharedString::from(format!(
                                "Open workspace {name}"
                            )))
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let directory = directory.clone();
                        let _ = weak.update(cx, |app, cx| {
                            let requested = std::path::Path::new(&directory);
                            let (directory, changed) =
                                match vela_engine::migration::resolve_and_relocate(
                                    &mut app.settings,
                                    requested,
                                ) {
                                    vela_engine::migration::RelocateOutcome::Resolved {
                                        directory,
                                        changed,
                                    } => (directory.to_string_lossy().into_owned(), changed),
                                    vela_engine::migration::RelocateOutcome::Ambiguous {
                                        previous_root,
                                        candidates,
                                    } => {
                                        let message =
                                            vela_engine::migration::ambiguous_relocation_message(
                                                &previous_root,
                                                &candidates,
                                            );
                                        app.toasts.update(cx, |toasts, cx| {
                                            toasts.push(message, true, cx)
                                        });
                                        cx.notify();
                                        return;
                                    }
                                    vela_engine::migration::RelocateOutcome::Missing => {
                                        (directory, false)
                                    }
                                };
                            if changed {
                                app.persist_preferences(cx);
                            }
                            let existing = app
                                .sessions
                                .iter()
                                .find(|session| session.directory == directory)
                                .map(|session| session.id.clone());
                            match existing {
                                // Opening a project swaps the focused panel, matching
                                // the folder-picker path; only the titlebar add control
                                // grows the grid.
                                Some(id) => app.reopen_session(id, cx),
                                None => {
                                    app.loading = true;
                                    // Opening a project replaces the focused panel.
                                    app.workspace_open_adds = false;
                                    app.engine.open_directory(directory);
                                    cx.notify();
                                }
                            }
                        });
                    })

                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .child(SharedString::from(name)),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(theme.faint)
                                    .child(history.len().to_string()),
                            ),
                    )
                    .child(
                        div()
                            .id(format!("project-history-toggle-{ix}"))
                            .role(Role::Button)
                            .tab_index(0)
                            .aria_label(SharedString::from(format!(
                                "{} {} sessions",
                                if expanded { "Collapse" } else { "Expand" },
                                bookmark.name
                            )))
                            .w(px(26.))
                            .h(px(26.))
                            .mr(px(4.))
                            .rounded(px(999.))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.selected).text_color(theme.text))
                            .child(if expanded { "⌄" } else { "›" })
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |app, _, _, cx| {
                                cx.stop_propagation();
                                if !app.expanded_session_projects.remove(&toggle_directory) {
                                    app.expanded_session_projects
                                        .insert(toggle_directory.clone());
                                }
                                cx.notify();
                            })),
                    )
                    .map(|row| {
                        crate::focus::outline(
                            row,
                            &focus,
                            7.,
                            1.,
                            0.,
                            theme.accent.into(),
                            &self.picker_anchors.keyboard,
                        )
                    }),
            ));
            if expanded {
                project_height += history.len().max(1) as f32 * 28.;
                let mut children = div().pl(px(12.)).flex().flex_col();
                if history.is_empty() {
                    children = children.child(
                        div()
                            .h(px(28.))
                            .text_size(px(12.))
                            .text_color(theme.faint)
                            .child("No sessions yet."),
                    );
                }
                for session in history {
                    let identity = format!("project:{ix}:{}", session.id);
                    let row_focus = self
                        .session_project_focus
                        .borrow_mut()
                        .entry(identity.clone())
                        .or_insert_with(|| cx.focus_handle().tab_stop(true))
                        .clone();
                    let pin_focus = self
                        .session_pin_focus
                        .borrow_mut()
                        .entry(identity.clone())
                        .or_insert_with(|| cx.focus_handle().tab_stop(true))
                        .clone();
                    let row = session_row(
                        session,
                        self.active_id.as_deref() == Some(session.id.as_str()),
                        self.settings.pinned_sessions.contains(&session.id),
                        self.panels.contains(&session.id),
                        pin_focus.clone(),
                        Some(row_focus.clone()),
                        self.is_codex(),
                        cx.weak_entity(),
                    );
                    let row = crate::scroll::FocusReveal::element(
                        format!("{identity}:pin"),
                        self.session_projects_scroll.clone(),
                        pin_focus,
                        row,
                    );
                    children = children.child(crate::scroll::FocusReveal::element(
                        identity,
                        self.session_projects_scroll.clone(),
                        row_focus,
                        row,
                    ));
                }
                column = column.child(children);
            }
        }
        div()
            .h(px(project_height.min(162.)))
            .flex_shrink_0()
            .child(crate::scroll::AutoScroll::new(
                self.session_projects_scroll.clone(),
                move |handle, gutter| {
                    div()
                        .id("saved-projects-scroll")
                        .h_full()
                        .w_full()
                        .overflow_y_scroll()
                        .track_scroll(&handle)
                        .scrollbar_width(gutter)
                        .child(column)
                        .into_any_element()
                },
            ))
            .into_any_element()
    }
}

/// Source orders Recents/Pinned with open sessions first, then by updatedAt.
fn sort_session_indices(
    sessions: &[SessionSummary],
    open: &HashSet<String>,
    indices: &mut [usize],
) {
    indices.sort_by(|left, right| {
        let left = &sessions[*left];
        let right = &sessions[*right];
        open.contains(&right.id)
            .cmp(&open.contains(&left.id))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
}

#[allow(clippy::too_many_arguments)]
fn session_row(
    session: &SessionSummary,
    is_active: bool,
    is_pinned: bool,
    is_open: bool,
    focus: gpui::FocusHandle,
    row_focus: Option<gpui::FocusHandle>,
    codex: bool,
    weak: gpui::WeakEntity<VelaApp>,
) -> AnyElement {
    let theme = Theme::default();
    let id = session.id.clone();
    let group = format!("session-row:{}", session.id);
    let weak = weak.clone();
    let pin_weak = weak.clone();
    let close_weak = weak.clone();
    let menu_weak = weak.clone();
    let menu_id = id.clone();
    let pin_id = id.clone();
    let close_id = id.clone();
    let close_label = format!("Close the {} session", session.display_title());
    let clicked_focus = focus.clone();
    let title = if session.title.trim().is_empty() {
        "Untitled session".to_string()
    } else {
        session.title.clone()
    };
    div()
        .id(SharedString::from(format!("session-{}", session.id)))
        .role(Role::Button)
        .aria_label(SharedString::from(title.clone()))
        .tab_index(0)
        .group(group.clone())
        .when_some(row_focus, |row, focus| row.track_focus(&focus))
        .w_full()
        .flex()
        .items_center()
        .gap(px(7.))
        .h(px(if codex { 30. } else { 28. }))
        .px(px(8.0))
        .rounded(px(if codex { 10. } else { 999. }))
        .cursor_pointer()
        .bg(if is_active {
            theme.selected
        } else {
            theme.panel
        })
        .text_size(px(12.5))
        .text_color(if is_active { theme.text } else { theme.dim })
        .hover(|style| style.bg(theme.selected).text_color(theme.text))
        .on_mouse_down(MouseButton::Right, move |event, window, cx| {
            cx.stop_propagation();
            let _ = menu_weak.update(cx, |app, cx| {
                app.saved_workspace_menu = None;
                app.session_context_menu = Some((menu_id.clone(), event.position));
                window.focus(&app.session_context_focus[0], cx);
                cx.notify();
            });
        })
        .on_click(move |_, _, cx| {
            let _ = weak.update(cx, |app, cx| app.reopen_session(id.clone(), cx));
        })
        .child(
            div()
                .debug_selector({
                    let title_id = session.id.clone();
                    move || format!("session-title:{title_id}")
                })
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(title)),
        )
        .child(
            div()
                .id(SharedString::from(format!("pin-session-{}", pin_id)))
                .role(Role::Button)
                .track_focus(&focus)
                .aria_label(if is_pinned {
                    "Unpin session"
                } else {
                    "Pin session"
                })
                .size(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_size(px(14.))
                .opacity(if is_pinned { 1. } else { 0. })
                .group_hover(group.clone(), |style| style.opacity(1.))
                .focus_visible(|style| style.opacity(1.))
                .hover(|style| {
                    style
                        .bg(theme.accent.opacity(0.12))
                        .text_color(theme.accent)
                })
                .text_color(if is_pinned { theme.accent } else { theme.faint })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    let _ = pin_weak.update(cx, |app, cx| {
                        app.toggle_session_pin(&pin_id, cx);
                        window.focus(&clicked_focus, cx);
                    });
                })
                .child("★"),
        )
        .when(is_open, |row| {
            row.child(
                div()
                    .id(SharedString::from(format!("close-session-{}", close_id)))
                    .debug_selector({
                        let close_id = close_id.clone();
                        move || format!("close-session:{close_id}")
                    })
                    .role(Role::Button)
                    .aria_label(SharedString::from(close_label))
                    .focusable()
                    .tab_stop(true)
                    .size(px(20.))
                    .flex_shrink_0()
                    .rounded(px(4.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .opacity(0.)
                    .group_hover(group.clone(), |style| style.opacity(1.))
                    .focus_visible(|style| style.opacity(1.))
                    .cursor_pointer()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(theme.faint)
                    .hover(|style| style.bg(theme.selected).text_color(theme.text))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let _ = close_weak.update(cx, |app, cx| {
                            app.close_panel(close_id.clone(), cx);
                        });
                    })
                    .child("×"),
            )
        })
        .into_any_element()
}


#[cfg(test)]
mod tests {
    use super::*;

    fn summary(id: &str, updated_at: i64) -> SessionSummary {
        SessionSummary {
            id: id.into(),
            title: id.into(),
            directory: "/workspace".into(),
            updated_at,
            parent_id: None,
            agent: None,
        }
    }

    #[test]
    fn session_order_puts_open_sessions_first_then_updated_at() {
        let sessions = vec![
            summary("newer", 300),
            summary("older-open", 100),
            summary("middle", 200),
            summary("newer-open", 50),
        ];
        let open = ["older-open", "newer-open"]
            .into_iter()
            .map(str::to_owned)
            .collect::<HashSet<_>>();
        let mut indices = vec![0, 1, 2, 3];
        sort_session_indices(&sessions, &open, &mut indices);
        let order = indices
            .into_iter()
            .map(|index| sessions[index].id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(order, ["older-open", "newer-open", "newer", "middle"]);
    }
}
