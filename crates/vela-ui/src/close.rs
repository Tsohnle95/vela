//! The document owner decides when close is safe. Native window close and every
//! macOS termination route ask the same coordinator before teardown starts.
use super::*;

/// Global/AppKit actions have no visual context. Resolve the owner's window
/// explicitly instead of inferring it from whichever child last held focus.
pub(super) fn update_owner<T: 'static, C: gpui::AppContext, R>(
    window: gpui::AnyWindowHandle,
    owner: &gpui::WeakEntity<T>,
    cx: &mut C,
    update: impl FnOnce(&mut T, &mut Window, &mut Context<T>) -> R,
) -> gpui::Result<R> {
    window.update(cx, |_, window, cx| {
        owner.update(cx, |owner, cx| update(owner, window, cx))
    })?
}

pub(super) fn defer_owner<T: 'static>(
    window: gpui::AnyWindowHandle,
    owner: gpui::WeakEntity<T>,
    cx: &mut App,
    update: impl FnOnce(&mut T, &mut Window, &mut Context<T>) + 'static,
) {
    // A global action may still be bubbling out of a window update. Wait until
    // GPUI returns that window to its registry before borrowing it again.
    cx.defer(move |cx| {
        if let Err(error) = update_owner(window, &owner, cx, update) {
            tracing::error!(%error, "could not route deferred Quit to the document owner");
        }
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Intent {
    Window,
    Quit,
}
pub(super) enum Phase {
    Waiting,
    Prompt,
    Saving {
        requests: HashSet<u64>,
        failed: bool,
    },
}
pub(super) struct Request {
    id: u64,
    intent: Intent,
    phase: Phase,
}

#[derive(Debug, PartialEq, Eq)]
enum Readiness {
    Busy,
    Unsaved(Vec<DocumentKey>),
    Clean,
}
fn readiness(documents: &Documents) -> Readiness {
    if documents.relocation_pending() || !documents.saving_keys().is_empty() {
        return Readiness::Busy;
    }
    let keys = documents.unsaved_keys();
    if keys.is_empty() {
        Readiness::Clean
    } else {
        Readiness::Unsaved(keys)
    }
}

impl VelaApp {
    fn explorer_io_pending(&self) -> bool {
        !self.create_requests.is_empty()
            || !self.import_requests.is_empty()
            || !self.detach_waiting.is_empty()
            || !self.detach_requests.is_empty()
            || !self.rename_waiting.is_empty()
            || !self.rename_requests.is_empty()
            || !self.remove_waiting.is_empty()
            || !self.remove_requests.is_empty()
    }
    pub(super) fn quit_if_allowed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.request_close(Intent::Quit, window, cx) {
            cx.quit();
        }
    }

    pub(super) fn request_close(
        &mut self,
        intent: Intent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        crate::platform_prompt::cancel_files(window);
        if self.close_approved.is_some() {
            return true;
        }
        if let Some(request) = &mut self.closing {
            if intent == Intent::Quit {
                request.intent = intent;
            }
            return false;
        }
        if !self.explorer_io_pending() && readiness(&self.documents.borrow()) == Readiness::Clean {
            return true;
        }
        self.close_sequence += 1;
        self.closing = Some(Request {
            id: self.close_sequence,
            intent,
            phase: Phase::Waiting,
        });
        self.close_timeout(window, cx);
        self.advance_close(window, cx);
        false
    }

    pub(super) fn note_close_result(&mut self, update: &EngineUpdate) {
        if let EngineUpdate::FileSaved {
            request_id,
            error,
            conflict,
            ..
        } = update
            && (error.is_some() || conflict.is_some())
            && let Some(Request {
                phase: Phase::Saving { requests, failed },
                ..
            }) = &mut self.closing
            && requests.contains(request_id)
        {
            *failed = true;
        }
    }

    pub(super) fn advance_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.explorer_io_pending() {
            return;
        }
        let Some(request) = &self.closing else {
            return;
        };
        if matches!(request.phase, Phase::Prompt) {
            return;
        }
        let state = readiness(&self.documents.borrow());
        if state == Readiness::Busy {
            return;
        }
        if matches!(request.phase, Phase::Saving { failed: true, .. }) {
            self.close_failed(window, cx);
            return;
        }
        match state {
            Readiness::Clean => self.finish_close(window, cx),
            Readiness::Unsaved(keys) => {
                if matches!(request.phase, Phase::Saving { .. }) {
                    self.close_failed(window, cx);
                } else {
                    self.prompt_close(keys, window, cx);
                }
            }
            Readiness::Busy => {}
        }
    }

    fn prompt_close(
        &mut self,
        keys: Vec<DocumentKey>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_deadline = None;
        let Some(request) = &mut self.closing else {
            return;
        };
        request.phase = Phase::Prompt;
        let id = request.id;
        let action = if request.intent == Intent::Quit {
            "quitting"
        } else {
            "closing"
        };
        let mut lines = keys
            .iter()
            .take(16)
            .map(|key| {
                let path = Path::new(&key.directory)
                    .join(&key.rel)
                    .to_string_lossy()
                    .into_owned();
                format!("• {}", brief_path(&path))
            })
            .collect::<Vec<_>>();
        if keys.len() > 16 {
            lines.push(format!("• and {} more", keys.len() - 16));
        }
        let result = crate::platform_prompt::document(
            window,
            &format!("Do you want to save your changes before {action} Vela?"),
            Some(&lines.join("\n")),
            &[
                gpui::PromptButton::ok("Save All"),
                gpui::PromptButton::new("Don't Save"),
                gpui::PromptButton::cancel("Cancel"),
            ],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let choice = result.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if !this.closing.as_ref().is_some_and(|request| {
                    request.id == id && matches!(request.phase, Phase::Prompt)
                }) {
                    return;
                }
                match choice {
                    0 => this.save_before_close(window, cx),
                    1 => this.finish_close(window, cx),
                    _ => {
                        this.closing = None;
                        this.close_deadline = None;
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn save_before_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut requests = HashSet::new();
        let mut failed = false;
        let keys = self.documents.borrow().unsaved_keys();
        for key in keys {
            if self
                .documents
                .borrow()
                .get(&key)
                .is_some_and(|document| document.conflict.is_some())
            {
                failed = true;
                continue;
            }
            let Some(session) = self.document_sessions.get(&key.directory).cloned() else {
                failed = true;
                continue;
            };
            self.file_request += 1;
            requests.insert(self.file_request);
            let request = self
                .documents
                .borrow_mut()
                .save(self.file_request, session, &key);
            if let Some(request) = request {
                self.dispatch_save(request);
            } else {
                failed = true;
            }
        }
        if let Some(request) = &mut self.closing {
            request.phase = Phase::Saving { requests, failed };
        }
        self.close_timeout(window, cx);
        self.advance_close(window, cx);
        cx.notify();
    }

    fn close_timeout(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some(request) = &self.closing else {
            return;
        };
        let id = request.id;
        self.close_deadline = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(30))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.closing.as_ref().is_some_and(|request| {
                    request.id == id && !matches!(request.phase, Phase::Prompt)
                }) {
                    this.close_failed(window, cx);
                }
            });
        }));
    }

    fn close_failed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.closing = None;
        self.close_deadline = None;
        let result = crate::platform_prompt::document(
            window,
            "Vela stayed open because some files could not be saved.",
            Some("Your edits are still in the editor. Check the save error before trying again."),
            &[gpui::PromptButton::ok("OK")],
            cx,
        );
        cx.spawn(async move |_, _| {
            let _ = result.await;
        })
        .detach();
        cx.notify();
    }

    fn finish_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(request) = self.closing.take() else {
            return;
        };
        self.close_approved = Some(request.intent);
        self.close_deadline = None;
        match request.intent {
            Intent::Window => window.remove_window(),
            Intent::Quit => cx.quit(),
        }
    }
}

fn brief_path(path: &str) -> String {
    let utf16 = path.encode_utf16().collect::<Vec<_>>();
    if utf16.len() <= 384 {
        path.to_owned()
    } else {
        format!("…{}", String::from_utf16_lossy(&utf16[utf16.len() - 383..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct DetachedFocusOwner {
        calls: usize,
        dirty: bool,
        allow: bool,
    }
    impl Render for DetachedFocusOwner {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child("No focused child")
        }
    }
    #[gpui::test]
    fn global_quit_reaches_owner_without_a_focused_child_and_preserves_guard(
        cx: &mut gpui::TestAppContext,
    ) {
        let (owner, visual) = cx.add_window_view(|_, _| DetachedFocusOwner {
            calls: 0,
            dirty: true,
            allow: false,
        });
        let weak = owner.downgrade();
        visual.update(|window, cx| {
            let handle = window.window_handle();
            cx.on_action(move |_: &crate::input::RequestQuit, cx| {
                defer_owner(handle, weak.clone(), cx, |owner, _, _| {
                    owner.calls += 1;
                    owner.allow = !owner.dirty;
                });
            });
        });
        visual.draw(point(px(0.), px(0.)), size(px(300.), px(160.)), |_, _| {
            owner.clone().into_any_element()
        });
        visual.update(|_, cx| cx.dispatch_action(&crate::input::RequestQuit));
        visual.update(|_, cx| {
            let state = owner.read(cx);
            assert_eq!(state.calls, 1);
            assert!(!state.allow, "global Quit must not bypass dirty documents");
            owner.update(cx, |owner, _| owner.dirty = false);
        });
        visual.update(|_, cx| cx.dispatch_action(&crate::input::RequestQuit));
        visual.update(|_, cx| {
            let state = owner.read(cx);
            assert_eq!(state.calls, 2);
            assert!(state.allow);
        });
    }
    fn edit(documents: &mut Documents, id: u64, workspace: &str) -> DocumentKey {
        let key = DocumentKey::new(workspace.into(), "file.txt".into());
        documents.open(id, key.clone(), true);
        documents.finish_load(id, "file.txt", Some("disk".into()));
        documents.edit(&key, |buffer| buffer.insert("local "));
        key
    }
    #[test]
    fn close_waits_for_all_workspace_snapshots_and_rejects_intervening_edits() {
        let mut documents = Documents::default();
        let first = edit(&mut documents, 1, "/one");
        let second = edit(&mut documents, 2, "/two");
        assert_eq!(
            readiness(&documents),
            Readiness::Unsaved(vec![first.clone(), second.clone()])
        );
        documents.save(3, "one".into(), &first);
        documents.save(4, "two".into(), &second);
        assert_eq!(readiness(&documents), Readiness::Busy);
        documents.finish_save(3, "file.txt", true);
        assert_eq!(readiness(&documents), Readiness::Busy);
        documents.edit(&second, |buffer| buffer.insert("newer "));
        documents.finish_save(4, "file.txt", true);
        assert_eq!(
            readiness(&documents),
            Readiness::Unsaved(vec![second.clone()])
        );
        documents.save(5, "two".into(), &second);
        documents.finish_save(5, "file.txt", true);
        assert_eq!(readiness(&documents), Readiness::Clean);
    }
    #[test]
    fn clean_pending_save_is_not_safe_and_conflicting_edits_remain_unsaved() {
        let mut documents = Documents::default();
        let key = edit(&mut documents, 1, "/one");
        documents.save(2, "one".into(), &key);
        documents.finish_save(2, "file.txt", true);
        documents.save(3, "one".into(), &key);
        assert_eq!(readiness(&documents), Readiness::Busy);
        documents.finish_save(3, "file.txt", true);
        documents.edit(&key, |buffer| buffer.insert("edit "));
        documents.mark_conflict(
            &key,
            ExternalChange {
                content: Some("external".into()),
                ..Default::default()
            },
        );
        assert!(documents.save(4, "one".into(), &key).is_none());
        assert_eq!(readiness(&documents), Readiness::Unsaved(vec![key]));
    }
    #[test]
    fn close_waits_for_reserved_relocation_and_then_reports_the_new_dirty_path() {
        let mut documents = Documents::default();
        let old = edit(&mut documents, 1, "/one");
        let new = DocumentKey::new(old.workspace.clone(), "renamed.txt".into());
        documents.begin_relocation(10, old, new.clone()).unwrap();
        assert_eq!(readiness(&documents), Readiness::Busy);
        documents.finish_relocation(10, true).unwrap();
        assert_eq!(readiness(&documents), Readiness::Unsaved(vec![new]));
    }

    #[test]
    fn long_close_paths_match_reference_utf16_tail_limit() {
        assert_eq!(brief_path("short/path"), "short/path");
        let long = format!("{}🌍tail", "a".repeat(400));
        let shortened = brief_path(&long);
        assert_eq!(shortened.encode_utf16().count(), 384);
        assert!(shortened.ends_with("🌍tail"));
    }
}
