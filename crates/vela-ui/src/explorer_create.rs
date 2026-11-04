//! Explorer creation retains the requesting workspace through asynchronous I/O.
use super::*;
use vela_engine::files::CreateEntryRequest;

pub(super) struct CreateForm {
    pub(super) workspace: String,
    pub(super) session: String,
    pub(super) parent: String,
    pub(super) is_dir: bool,
    pub(super) request: Option<u64>,
    pub(super) error: Option<String>,
    pub(super) rename_from: Option<String>,
    pub(super) waiting: bool,
}
impl VelaApp {
    pub(super) fn start_create_entry(
        &mut self,
        workspace: String,
        parent: String,
        is_dir: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_directory() != workspace {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            return;
        };
        self.file_trees.entry(workspace.clone()).or_default().show();
        if !parent.is_empty() {
            self.expanded.insert(parent.clone());
            self.request_directory(parent.clone());
        }
        self.pending_create = Some(CreateForm {
            workspace,
            session,
            parent,
            is_dir,
            request: None,
            error: None,
            rename_from: None,
            waiting: false,
        });
        self.focus_entry_name(
            if is_dir {
                "untitled folder"
            } else {
                "untitled.txt"
            },
            window,
            cx,
        );
    }

    pub(super) fn start_rename_entry(
        &mut self,
        workspace: String,
        rel: String,
        is_dir: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if rel.is_empty() || self.workspace_directory() != workspace {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            return;
        };
        let (parent, name) = rel.rsplit_once('/').unwrap_or(("", &rel));
        let initial = name.to_owned();
        self.pending_create = Some(CreateForm {
            workspace,
            session,
            parent: parent.to_owned(),
            is_dir,
            request: None,
            error: None,
            rename_from: Some(rel),
            waiting: false,
        });
        self.focus_entry_name(&initial, window, cx);
    }

    fn focus_entry_name(&mut self, initial: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.entry_name.update(cx, |input, cx| {
            input.set_text(initial.to_owned(), cx);
            input.buffer.select_all();
        });
        let focus = self.entry_name.focus_handle(cx);
        if self.create_blur.is_none() {
            self.create_blur =
                Some(cx.on_blur(&focus, window, |app, _, cx| app.commit_create_entry(cx)));
        }
        window.focus(&focus, cx);
        cx.notify();
    }

    fn commit_create_entry(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &self.pending_create else {
            return;
        };
        if form.request.is_some() {
            return;
        }
        if form.workspace != self.workspace_directory()
            || self.active_id.as_deref() != Some(form.session.as_str())
        {
            self.pending_create = None;
            cx.notify();
            return;
        }
        let name = self.entry_name.read(cx).buffer.text().trim().to_owned();
        if name.is_empty() {
            self.pending_create = None;
            cx.notify();
            return;
        }
        if name.contains('/')
            || name.contains('\\')
            || name.contains('\0')
            || name == "."
            || name == ".."
        {
            self.pending_create.as_mut().unwrap().error = Some("Invalid name".into());
            cx.notify();
            return;
        }
        if let Some(rel) = form.rename_from.clone() {
            self.file_request += 1;
            let request = vela_engine::files::RenameEntryRequest {
                id: self.file_request,
                target_parent: None,
                session: form.session.clone(),
                workspace: form.workspace.clone(),
                rel,
                name,
            };
            let form = self.pending_create.as_mut().unwrap();
            form.request = Some(request.id);
            form.waiting = true;
            form.error = None;
            self.rename_waiting.insert(request.id, request);
            self.pump_entry_renames(cx);
            cx.notify();
            return;
        }
        self.file_request += 1;
        let request = CreateEntryRequest {
            id: self.file_request,
            session: form.session.clone(),
            workspace: form.workspace.clone(),
            parent: form.parent.clone(),
            name,
            is_dir: form.is_dir,
        };
        self.pending_create.as_mut().unwrap().request = Some(request.id);
        self.create_requests.insert(request.id, request.clone());
        self.engine.create_entry(request);
        cx.notify();
    }

    pub(super) fn finish_create_entry(
        &mut self,
        request: CreateEntryRequest,
        rel: Option<String>,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.create_requests.remove(&request.id).is_none() {
            return;
        }
        let owns_form = self
            .pending_create
            .as_ref()
            .is_some_and(|form| form.request == Some(request.id));
        if let Some(error) = error {
            if owns_form {
                let form = self.pending_create.as_mut().unwrap();
                form.request = None;
                form.error = Some(error.clone());
            }
            self.toasts
                .update(cx, |toasts, cx| toasts.push(error, true, cx));
        } else {
            if owns_form {
                self.pending_create = None;
            }
            if self.workspace_directory() == request.workspace {
                self.request_directory(request.parent);
                if let Some(rel) = rel {
                    if let Some(scope) = self.document_scopes.get(&request.session)
                        && let Some(review) = self.change_reviews.get(scope)
                    {
                        review.update(cx, |view, cx| view.unhide(&rel, cx));
                    }
                    if request.is_dir {
                        self.expanded.insert(rel.clone());
                        self.request_directory(rel);
                    } else if owns_form
                        && self.explorer_menu.is_none()
                        && self.active_id.as_deref() == Some(request.session.as_str())
                        && self.sidebar_tab == SidebarTab::Files
                        && !self.settings_open
                    {
                        self.created_files_to_open.push(DocumentKey::in_workspace(
                            self.document_workspace(),
                            request.workspace,
                            rel,
                        ));
                    }
                }
            }
        }
        cx.notify();
    }

    pub(super) fn render_create_name(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let form = self.pending_create.as_ref()?;
        if form.workspace != self.workspace_directory() {
            return None;
        }
        let theme = Theme::default();
        Some(
            div()
                .id("explorer-name-input")
                .mx(px(8.))
                .p(px(5.))
                .border_1()
                .border_color(theme.accent)
                .rounded(px(4.))
                .child(self.entry_name.clone())
                .when(!form.parent.is_empty(), |row| {
                    row.child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.faint)
                            .child(form.parent.clone()),
                    )
                })
                .when(form.request.is_some(), |row| {
                    row.child(
                        div()
                            .id("entry-create-progress")
                            .role(Role::Label)
                            .aria_label(if form.rename_from.is_some() {
                                "Renaming entry"
                            } else {
                                "Creating entry"
                            })
                            .text_size(px(11.))
                            .child(if form.waiting {
                                "Waiting for file operations…"
                            } else if form.rename_from.is_some() {
                                "Renaming…"
                            } else {
                                "Creating…"
                            }),
                    )
                })
                .when_some(form.error.clone(), |row, error| {
                    row.child(
                        div()
                            .id("entry-create-error")
                            .role(Role::Label)
                            .aria_label(error.clone())
                            .text_size(px(11.))
                            .text_color(theme.red)
                            .child(error),
                    )
                })
                .on_key_down(cx.listener(|app, event: &KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "enter" => app.commit_create_entry(cx),
                        "escape" => {
                            app.pending_create = None;
                            window.focus(&app.composer_focus, cx);
                            cx.notify();
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .into_any_element(),
        )
    }
}

pub(super) fn create_action(
    workspace: String,
    parent: String,
    is_dir: bool,
    owner: gpui::WeakEntity<VelaApp>,
) -> AnyElement {
    let theme = Theme::default();
    div()
        .id(SharedString::from(format!(
            "create-{}-{parent}",
            if is_dir { "folder" } else { "file" }
        )))
        .role(Role::Button)
        .aria_label(if is_dir { "New Folder" } else { "New File" })
        .tab_index(0)
        .px(px(3.))
        .font_family("codicon")
        .text_size(px(14.))
        .text_color(theme.faint)
        .cursor_pointer()
        .hover(|style| style.text_color(theme.text))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            let _ = owner.update(cx, |app, cx| {
                app.start_create_entry(workspace.clone(), parent.clone(), is_dir, window, cx)
            });
        })
        .child(icon(if is_dir { "ea80" } else { "ea7f" }))
        .into_any_element()
}
