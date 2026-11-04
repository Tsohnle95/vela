//! Composer-owned attachments; paths are sent to the runtime only on submission.
use super::*;
use std::path::PathBuf;

pub(super) fn cancel_drag(event: &KeyDownEvent, window: &mut Window, cx: &mut App) -> bool {
    event.keystroke.key == "escape" && cx.stop_active_drag(window)
}

/// GPUI clears its active drag on a platform Exited event without delivering
/// a final mouse move or invalidating this view. Observe that lifetime only
/// while a drag is active, then clear the target owner and request its frame.
#[derive(Default)]
pub(super) struct DragLifetime {
    running: Rc<std::cell::Cell<bool>>,
}
impl DragLifetime {
    pub fn observe<T: 'static>(
        &self,
        cx: &mut Context<T>,
        ended: impl Fn(&mut T, &mut Context<T>) + 'static,
    ) {
        if self.running.replace(true) {
            return;
        }
        let running = self.running.clone();
        cx.spawn(async move |owner, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let keep = owner
                    .update(cx, |owner, cx| {
                        if cx.has_active_drag() {
                            true
                        } else {
                            running.set(false);
                            ended(owner, cx);
                            false
                        }
                    })
                    .unwrap_or(false);
                if !keep {
                    running.set(false);
                    break;
                }
            }
        })
        .detach();
    }
}

#[derive(Clone, Debug)]
pub(super) struct StagedAttachment {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) mention: Option<vela_opencode::PromptMention>,
}

/// Source's `detectTrigger` mention rule: the trigger starts at the last `@`
/// that begins a word, runs to the caret without whitespace, and is ignored
/// while the caret is inside an existing mention span. Spans are derived from
/// the current text (`indexOf(token)`), not stale insertion offsets.
pub(super) fn mention_trigger_at(
    text: &str,
    caret: usize,
    spans: &[std::ops::Range<usize>],
) -> Option<(usize, String)> {
    if caret > text.len() {
        return None;
    }
    let before = &text[..caret];
    let token_start = before
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    if !before[token_start..].starts_with('@') {
        return None;
    }
    let at = token_start;
    let query = &before[at + 1..];
    if spans
        .iter()
        .any(|span| at < span.end && caret > span.start)
    {
        return None;
    }
    Some((at, query.to_owned()))
}

/// Rebuild prompt files from the current composer text. Source keeps a separate
/// mention list, looks up `@rel` with `indexOf`, and omits any mention whose
/// token was edited away. Plain staged files are always sent.
pub(super) fn prompt_files_from_staged(
    text: &str,
    files: Vec<StagedAttachment>,
) -> Vec<vela_opencode::PromptFile> {
    files
        .into_iter()
        .filter_map(|file| {
            let mention = match file.mention {
                Some(mut mention) => {
                    let start = text.find(&mention.text)?;
                    mention.start = start;
                    mention.end = start + mention.text.len();
                    Some(mention)
                }
                None => None,
            };
            Some(vela_opencode::PromptFile {
                path: file.path.to_string_lossy().into_owned(),
                name: None,
                mention,
            })
        })
        .collect()
}

pub(super) fn stage(
    files: &mut Vec<StagedAttachment>,
    paths: impl IntoIterator<Item = PathBuf>,
) {
    for path in paths {
        if !files.iter().any(|file| file.path == path) {
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned();
            files.push(StagedAttachment {
                path,
                name,
                mention: None,
            });
        }
    }
}
pub(super) fn image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "webp"
                    | "bmp"
                    | "svg"
                    | "avif"
                    | "ico"
                    | "tiff"
                    | "tif"
            )
        })
}

pub(super) fn attachment_mention_spans(
    attachments: &[StagedAttachment],
    text: &str,
) -> Vec<std::ops::Range<usize>> {
    attachments
        .iter()
        .filter_map(|attachment| attachment.mention.as_ref())
        .filter_map(|mention| {
            text.find(&mention.text)
                .map(|start| start..start + mention.text.len())
        })
        .collect()
}

pub(super) fn attach_staged_reference(
    files: &mut Vec<StagedAttachment>,
    path: PathBuf,
    rel: &str,
    mention: vela_opencode::PromptMention,
) {
    if let Some(existing) = files.iter_mut().find(|file| file.path == path) {
        existing.mention = Some(mention);
        return;
    }
    files.push(StagedAttachment {
        path,
        name: rel.rsplit('/').next().unwrap_or(rel).to_owned(),
        mention: Some(mention),
    });
}

impl VelaApp {
    pub(super) fn restore_prompt_files(&mut self, files: &[vela_opencode::PromptFile]) {
        self.attachments.clear();
        stage(
            &mut self.attachments,
            files.iter().map(|file| PathBuf::from(&file.path)),
        );
        for attachment in &mut self.attachments {
            attachment.mention = files
                .iter()
                .find(|file| Path::new(&file.path) == attachment.path)
                .and_then(|file| file.mention.clone());
        }
    }
    pub(super) fn attach_reference(
        &mut self,
        path: PathBuf,
        rel: &str,
        mention: vela_opencode::PromptMention,
    ) {
        if let Some(existing) = self.attachments.iter_mut().find(|file| file.path == path) {
            existing.mention = Some(mention);
            return;
        }
        self.attachments.push(StagedAttachment {
            path,
            name: rel.rsplit('/').next().unwrap_or(rel).to_owned(),
            mention: Some(mention),
        });
    }

    pub(super) fn attach_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let session = self.active_id.clone();
        let receiver = crate::platform_prompt::files(window, cx);
        cx.spawn(async move |this, cx| {
            let Some(paths) = receiver.await else {
                return;
            };
            let _ = this.update(cx, |app, cx| {
                // A late system dialog must not attach files to another workspace.
                if app.active_id == session {
                    stage(&mut app.attachments, paths);
                    app.composer_focus_pending = true;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn stage_dropped_files(
        &mut self,
        paths: &gpui::ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.attachment_drag_over = false;
        stage(&mut self.attachments, paths.paths().iter().cloned());
        cx.stop_propagation();
        window.focus(&self.composer_focus, cx);
        cx.notify();
    }

    pub(super) fn attachment_drop_hint(&self) -> AnyElement {
        let theme = Theme::default();
        div()
            .absolute()
            .inset(px(-1.))
            .rounded(px(if self.is_codex() { 16. } else { 13. }))
            .border(px(1.5))
            .border_dashed()
            .border_color(theme.accent)
            .bg(theme.inset.opacity(0.78))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(7.))
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.accent)
            .child(crate::icons::inherited_glyph("image", 16.))
            .child("Drop images to attach")
            .into_any_element()
    }

    pub(super) fn attachment_mention_spans(&self, text: &str) -> Vec<std::ops::Range<usize>> {
        attachment_mention_spans(&self.attachments, text)
    }

    pub(super) fn take_prompt_files(&mut self, text: &str) -> Vec<vela_opencode::PromptFile> {
        prompt_files_from_staged(text, std::mem::take(&mut self.attachments))
    }

    pub(super) fn render_attachments(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let mut files = div().w_full().px(px(2.)).flex().flex_wrap().gap(px(4.));
        for file in &self.attachments {
            let path = file.path.clone();
            files = files.child(
                div()
                    .max_w_full()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .pl(px(6.))
                    .pr(px(4.))
                    .py(px(3.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.border_strong)
                    .bg(if self.is_codex() {
                        theme.text.opacity(0.07)
                    } else {
                        theme.accent.opacity(0.07)
                    })
                    .text_size(px(10.))
                    .line_height(px(12.))
                    .text_color(theme.dim)
                    .child(if image_path(&file.path) {
                        gpui::img(file.path.clone())
                            .size(px(18.))
                            .flex_shrink_0()
                            .rounded(px(4.))
                            .object_fit(gpui::ObjectFit::Cover)
                            .with_fallback(|| {
                                div()
                                    .font_family("codicon")
                                    .text_size(px(16.))
                                    .child(icon("ea7b"))
                                    .into_any_element()
                            })
                            .into_any_element()
                    } else {
                        div()
                            .font_family("codicon")
                            .text_size(px(16.))
                            .line_height(px(16.))
                            .child(icon("ea7b"))
                            .into_any_element()
                    })
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .child(SharedString::from(file.name.clone())),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "remove-attachment-{}",
                                file.path.display()
                            )))
                            .role(gpui::Role::Button)
                            .aria_label(SharedString::from(format!("Remove {}", file.name)))
                            .focusable()
                            .tab_stop(true)
                            .px(px(2.))
                            .text_size(px(14.))
                            .line_height(px(14.))
                            .text_color(theme.faint)
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text))
                            .on_click(cx.listener(move |app, _, _, cx| {
                                app.attachments.retain(|file| file.path != path);
                                app.composer_focus_pending = true;
                                cx.notify();
                            }))
                            .child("×"),
                    ),
            );
        }
        files.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct DragFixture {
        over: bool,
        lifetime: DragLifetime,
        focus: gpui::FocusHandle,
        child_escaped: bool,
    }
    impl Render for DragFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("attachment-drag-target")
                .w(px(200.))
                .h(px(100.))
                .capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                    if cancel_drag(event, window, cx) {
                        view.over = false;
                        cx.notify();
                        cx.stop_propagation();
                    }
                }))
                .on_drag_move(cx.listener(
                    |view, event: &gpui::DragMoveEvent<gpui::ExternalPaths>, _, cx| {
                        view.over = event.bounds.contains(&event.event.position);
                        view.lifetime.observe(cx, |view, cx| {
                            view.over = false;
                            cx.notify();
                        });
                        cx.notify();
                    },
                ))
                .child(
                    div()
                        .id("focused-child")
                        .track_focus(&self.focus)
                        .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                            if event.keystroke.key == "escape" {
                                view.child_escaped = true;
                                cx.stop_propagation();
                            }
                        }))
                        .child(if self.over && cx.has_active_drag() {
                            "Drop images to attach"
                        } else {
                            "Ready"
                        }),
                )
        }
    }
    #[gpui::test]
    fn external_drag_exit_clears_attachment_owner_without_another_mouse_event(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            DragFixture {
                over: false,
                lifetime: DragLifetime::default(),
                focus,
                child_escaped: false,
            }
        });
        cx.draw(point(px(0.), px(0.)), size(px(200.), px(100.)), |_, _| {
            view.clone().into_any_element()
        });
        cx.update(|window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::FileDrop(gpui::FileDropEvent::Entered {
                    position: point(px(10.), px(10.)),
                    paths: gpui::ExternalPaths(
                        [PathBuf::from("/fixture/image.png")].into_iter().collect(),
                    ),
                }),
                cx,
            )
        });
        view.read_with(cx, |view, _| assert!(view.over));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.over, "the hint follows the active drag, not a timeout")
        });
        cx.update(|window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::FileDrop(gpui::FileDropEvent::Exited),
                cx,
            )
        });
        assert!(
            !cx.update(|_, cx| cx.has_active_drag()),
            "framework drag ends before the owner is notified"
        );
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        view.read_with(cx,|view,_|assert!(!view.over,"an external exit must clear the attachment owner without waiting for a later mouse move"));
    }

    #[gpui::test]
    fn escape_cancels_external_drag_before_a_focused_child_can_consume_it(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            DragFixture {
                over: false,
                lifetime: DragLifetime::default(),
                focus,
                child_escaped: false,
            }
        });
        cx.draw(point(px(0.), px(0.)), size(px(200.), px(100.)), |_, _| {
            view.clone().into_any_element()
        });
        cx.update(|window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::FileDrop(gpui::FileDropEvent::Entered {
                    position: point(px(10.), px(10.)),
                    paths: gpui::ExternalPaths(
                        [PathBuf::from("/fixture/image.png")].into_iter().collect(),
                    ),
                }),
                cx,
            )
        });
        cx.simulate_keystrokes("escape");
        assert!(!cx.update(|_, cx| cx.has_active_drag()));
        view.read_with(cx, |view, _| {
            assert!(!view.over);
            assert!(!view.child_escaped);
        });
    }

    #[test]
    fn mention_trigger_matches_source_word_and_current_span_rules() {
        assert_eq!(
            mention_trigger_at("@src/foo.ts", 11, &[]),
            Some((0, "src/foo.ts".into()))
        );
        assert_eq!(
            mention_trigger_at("hi @src/foo.ts", 14, &[]),
            Some((3, "src/foo.ts".into()))
        );
        assert_eq!(mention_trigger_at("foo@bar", 7, &[]), None);
        assert_eq!(
            mention_trigger_at("a @foo@bar", 10, &[]),
            Some((2, "foo@bar".into())),
            "the first @ after whitespace owns the whole token"
        );
        assert_eq!(mention_trigger_at("@foo bar", 8, &[]), None);
        assert_eq!(
            mention_trigger_at("a\n@bar", 6, &[]),
            Some((2, "bar".into()))
        );

        let spans = std::iter::once(3..14).collect::<Vec<_>>();
        assert_eq!(
            mention_trigger_at("hi @src/foo.tsx", 15, &spans),
            None,
            "typing immediately after a mention stays suppressed"
        );
        assert_eq!(
            mention_trigger_at("hi @src/foo.ts @bar", 19, &spans),
            Some((15, "bar".into())),
            "a new whitespace-separated mention is not suppressed"
        );
        assert_eq!(mention_trigger_at("hi @src/foo", 10, &spans), None);
    }

    #[test]
    fn prompt_files_rebuild_mentions_from_current_text_and_drop_removed_tokens() {
        let files = vec![
            StagedAttachment {
                path: PathBuf::from("/w/src/foo.ts"),
                name: "foo.ts".into(),
                mention: Some(vela_opencode::PromptMention {
                    start: 0,
                    end: 11,
                    text: "@src/foo.ts".into(),
                }),
            },
            StagedAttachment {
                path: PathBuf::from("/w/src/bar.ts"),
                name: "bar.ts".into(),
                mention: Some(vela_opencode::PromptMention {
                    start: 20,
                    end: 31,
                    text: "@src/bar.ts".into(),
                }),
            },
            StagedAttachment {
                path: PathBuf::from("/w/notes.txt"),
                name: "notes.txt".into(),
                mention: None,
            },
        ];
        let out = prompt_files_from_staged("use @src/foo.ts now", files);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].path, "/w/src/foo.ts");
        let mention = out[0].mention.as_ref().unwrap();
        assert_eq!((mention.start, mention.end), (4, 15));
        assert_eq!(mention.text, "@src/foo.ts");
        assert!(
            out.iter().all(|file| file.path != "/w/src/bar.ts"),
            "a mention whose token was edited away is not sent"
        );
        assert_eq!(out[1].path, "/w/notes.txt");
        assert!(out[1].mention.is_none());
    }

    #[test]
    fn staging_deduplicates_full_paths_and_preserves_unicode_and_same_names() {
        let mut files = Vec::new();
        stage(
            &mut files,
            [
                PathBuf::from("/one/🌍.PNG"),
                PathBuf::from("/two/🌍.PNG"),
                PathBuf::from("/one/🌍.PNG"),
            ],
        );
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "🌍.PNG");
        assert_eq!(files[1].path, PathBuf::from("/two/🌍.PNG"));
        assert!(image_path(&files[0].path));
        assert!(!image_path(Path::new("/one/notes.txt")));
    }
}
