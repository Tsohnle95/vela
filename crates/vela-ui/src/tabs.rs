//! Shared titlebar tabs, including the source strip's responsive overflow.
use super::*;

#[derive(Clone)]
enum Target {
    Document(DocumentKey),
    Diff,
    Browser(usize),
}

#[derive(Clone)]
pub(super) struct DraggedEditorTab {
    pub key: DocumentKey,
}

struct TabDragPreview {
    label: String,
}

impl Render for TabDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(5.))
            .bg(theme.selected)
            .text_color(theme.text)
            .text_size(px(12.5))
            .child(self.label.clone())
    }
}
#[derive(Clone)]
struct Tab {
    target: Target,
    name: String,
    active: bool,
    preview: bool,
    dirty: bool,
}

#[derive(Default)]
pub(super) struct Strip {
    width: f32,
    anchor: Option<Bounds<gpui::Pixels>>,
    open: bool,
    scroll: crate::scroll::AutoScrollHandle,
    focus: Option<gpui::FocusHandle>,
}

#[cfg(test)]
impl Strip {
    pub(super) fn set_width_for_test(&mut self, width: f32) {
        self.width = width;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BrowserBodyState {
    Empty,
    Loading,
    Error,
    Ready,
}

pub(super) fn browser_body_state(
    url: &str,
    loading: bool,
    error: Option<&str>,
) -> BrowserBodyState {
    if error.is_some() {
        BrowserBodyState::Error
    } else if url.is_empty() {
        BrowserBodyState::Empty
    } else if loading {
        BrowserBodyState::Loading
    } else {
        BrowserBodyState::Ready
    }
}

pub(super) fn browser_tab_label(url: &str, title: &str) -> String {
    let title = title.trim();
    if !title.is_empty() {
        return title.to_owned();
    }
    if url.is_empty() {
        return "New tab".into();
    }
    if let Ok(parsed) = url::Url::parse(url) {
        return parsed
            .host_str()
            .unwrap_or("")
            .trim_start_matches("www.")
            .to_owned();
    }
    url.to_owned()
}

pub(super) fn browser_active_after_close(
    len: usize,
    active: usize,
    closing: usize,
) -> Option<usize> {
    if len == 0 || active >= len || closing >= len {
        return None;
    }
    let remaining = len - 1;
    if remaining == 0 {
        return None;
    }
    if closing < active {
        Some(active - 1)
    } else if closing == active {
        Some(closing.min(remaining - 1))
    } else {
        Some(active)
    }
}

fn visible(width: f32, files: usize, browsers: usize) -> (usize, usize, usize) {
    let add = if browsers > 0 { 28. } else { 0. };
    let width = (width - add).max(0.);
    let overflow = files as f32 * 90. + browsers as f32 * 120. > width;
    let available = (width - if overflow { 30. } else { 0. }).max(0.);
    let shown_browsers = browsers.min((available / 120.).floor() as usize);
    let shown_files = if browsers > 0 && shown_browsers == 0 {
        0
    } else {
        files.min(
            ((available - shown_browsers as f32 * 120.) / 90.)
                .max(0.)
                .floor() as usize,
        )
    };
    (
        shown_files,
        shown_browsers,
        files + browsers - shown_files - shown_browsers,
    )
}

impl VelaApp {
    fn close_titlebar_tab(&mut self, target: &Target, window: &mut Window, cx: &mut Context<Self>) {
        match target {
            Target::Document(key) => self.close_document(key.clone(), window, cx),
            Target::Diff => self.close_diff(window, cx),
            Target::Browser(index) => self.close_browser_tab(*index, window, cx),
        }
    }
    fn titlebar_items(&self) -> Vec<Tab> {
        let workspace = self.document_workspace();
        let documents = self.documents.borrow();
        let mut items = documents
            .tabs(&workspace)
            .iter()
            .map(|tab| {
                let dirty = documents
                    .get(&tab.key)
                    .is_some_and(|document| document.dirty());
                Tab {
                    target: Target::Document(tab.key.clone()),
                    name: tab
                        .key
                        .rel
                        .rsplit('/')
                        .next()
                        .unwrap_or(&tab.key.rel)
                        .into(),
                    active: !self.browser_open
                        && !self.changes_open
                        && self.diff.is_none()
                        && self.active_document_key().as_ref() == Some(&tab.key),
                    preview: !tab.pinned,
                    dirty,
                }
            })
            .collect::<Vec<_>>();
        if let Some(diff) = &self.diff {
            items.push(Tab {
                target: Target::Diff,
                name: diff.rel.rsplit('/').next().unwrap_or(&diff.rel).into(),
                active: !self.browser_open && self.changes_open,
                preview: false,
                dirty: false,
            });
        }
        items.extend(
            self.browser_tabs
                .iter()
                .enumerate()
                .map(|(index, tab)| Tab {
                    target: Target::Browser(index),
                    name: browser_tab_label(&tab.url, &tab.title),
                    active: self.browser_open && self.browser_active == index,
                    preview: false,
                    dirty: false,
                }),
        );
        items
    }
    fn select_titlebar_tab(
        &mut self,
        target: &Target,
        pin: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab_strip.open = false;
        match target {
            Target::Browser(index) => self.activate_browser_tab(*index, window, cx),
            Target::Document(key) => {
                if key.workspace != self.document_workspace() {
                    return;
                }
                self.browser_open = false;
                self.changes_open = false;
                self.diff = None;
                if let Some(browser) = &self.browser {
                    browser.set_visible(false);
                }
                self.documents.borrow_mut().activate(key);
                if pin {
                    self.documents.borrow_mut().pin(key);
                }
                let editor = self.editor_view(key.clone(), cx);
                window.focus(&editor.focus_handle(cx), cx);
            }
            Target::Diff => {
                self.browser_open = false;
                self.changes_open = true;
                if let Some(browser) = &self.browser {
                    browser.set_visible(false);
                }
            }
        }
        cx.notify();
    }
    pub(super) fn render_titlebar_tabs(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let codex = self.is_codex();
        let items = self.titlebar_items();
        let files = items
            .iter()
            .filter(|tab| !matches!(tab.target, Target::Browser(_)))
            .count();
        let browsers = items.len() - files;
        let (shown_files, shown_browsers, hidden) = visible(self.tab_strip.width, files, browsers);
        let mut strip = div()
            .id("titlebar-tabbar")
            .relative()
            .w_full()
            .h_full()
            .min_w_0()
            .flex()
            .items_center()
            .overflow_hidden()
            .when(codex, |strip| strip.px(px(8.)).gap(px(4.)));
        let mut tab_list = div()
            .id("titlebar-tabs")
            .role(gpui::Role::TabList)
            .aria_label("Open editor tabs")
            .relative()
            .flex_1()
            .h_full()
            .min_w_0()
            .flex()
            .items_center()
            .overflow_hidden();
        let measured = cx.weak_entity();
        tab_list = tab_list.child(
            gpui::canvas(
                move |bounds, _, cx| {
                    let _ = measured.update(cx, |app, cx| {
                        // The reference observes the list itself, after the overflow
                        // sibling and its gap take space from the surrounding strip.
                        let width = f32::from(bounds.size.width).max(0.);
                        if (app.tab_strip.width - width).abs() > 0.25 {
                            app.tab_strip.width = width;
                            cx.notify();
                        }
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );
        for (index, tab) in items.iter().enumerate() {
            let shown = match tab.target {
                Target::Browser(browser) => browser < shown_browsers,
                _ => index < shown_files,
            };
            if !shown {
                continue;
            }
            let target = tab.target.clone();
            let close = target.clone();
            let keyboard_close = target.clone();
            let drag_key = match &tab.target {
                Target::Document(key) => Some(key.clone()),
                _ => None,
            };
            let drag_name = tab.name.clone();
            let active = tab.active;
            let browser = matches!(target, Target::Browser(_));
            let name = tab.name.clone();
            let chip_selector = format!("titlebar-tab-{index}");
            let mut chip = div()
                .id(SharedString::from(format!("titlebar-tab-{index}")))
                .debug_selector(move || chip_selector.clone())
                .group("titlebar-tab")
                .role(gpui::Role::Tab)
                .aria_label(SharedString::from(name.clone()))
                .aria_selected(active)
                .min_w_0()
                .max_w(px(if codex { 240. } else { 220. }))
                .h(px(if codex { 30. } else { 34. }))
                .pl(px(10.))
                .pr(px(if codex { 6. } else { 10. }))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(if codex { 12.5 } else { 11. }))
                .font_weight(if active && codex {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if active { theme.text } else { theme.dim })
                .cursor_pointer()
                .when(codex, |chip| {
                    chip.flex_1()
                        .min_w(px(if browser { 120. } else { 90. }))
                        .rounded(px(8.))
                })
                .when(!codex, |chip| {
                    chip.border_b_2().border_color(if active {
                        theme.accent
                    } else {
                        theme::rgba(0, 0, 0, 0.)
                    })
                })
                .bg(if active && codex {
                    theme.editor
                } else {
                    theme.top
                })
                .when(active && codex, |chip| {
                    chip.shadow(vec![
                        gpui::BoxShadow::new(px(0.), px(1.), theme::rgba(0, 0, 0, 0.08).into())
                            .blur_radius(px(3.)),
                    ])
                })
                .hover(|style| style.bg(theme.selected).text_color(theme.text))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |app, event: &gpui::MouseDownEvent, window, cx| {
                        app.select_titlebar_tab(&target, event.click_count >= 2, window, cx);
                    }),
                );
            if let Some(key) = drag_key {
                chip = chip.on_drag(
                    DraggedEditorTab { key },
                    move |_, _, _, cx| {
                        cx.new(|_| TabDragPreview {
                            label: drag_name.clone(),
                        })
                    },
                );
            }
            chip = chip.child(if browser {
                crate::icons::glyph("globe", 16., theme.faint).into_any_element()
            } else {
                crate::tree_icons::tab_glyph(&name)
                    .text_color(if active {
                        if codex { theme.text } else { theme.accent }
                    } else {
                        theme.faint
                    })
                    .into_any_element()
            });
            chip = chip.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when(tab.dirty, |label| {
                        label.child(div().size(px(7.)).rounded_full().bg(if codex {
                            theme.dim
                        } else {
                            theme.accent
                        }))
                    })
                    .child(
                        div()
                            .min_w_0()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .when(tab.preview, |label| label.italic())
                            .child(SharedString::from(name.clone())),
                    ),
            );
            tab_list = tab_list.child(
                chip.child(
                    div()
                        .id(SharedString::from(format!("titlebar-tab-close-{index}")))
                        .role(gpui::Role::Button)
                        .focusable()
                        .tab_stop(true)
                        .aria_label(SharedString::from(format!("Close {name}")))
                        .size(px(18.))
                        .flex_shrink_0()
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(16.))
                        .line_height(gpui::relative(1.))
                        .text_color(theme.faint)
                        .opacity(if active || !codex { 1. } else { 0. })
                        .group_hover("titlebar-tab", |style| style.opacity(1.))
                        .focus_visible(|style| style.opacity(1.))
                        .hover(|style| style.bg(theme.selected).text_color(theme.text))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |app, _, window, cx| {
                                app.close_titlebar_tab(&close, window, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .on_key_down(cx.listener(move |app, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                app.close_titlebar_tab(&keyboard_close, window, cx);
                                cx.stop_propagation();
                            }
                        }))
                        .child("×"),
                ),
            );
        }
        let mut add_browser = if browsers > 0 {
            Some(
                div()
                    .id("browser-tab-add")
                    .role(gpui::Role::Button)
                    .aria_label("New browser tab")
                    .w(px(28.))
                    .h(px(26.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(|app, _, window, cx| app.new_browser_tab(window, cx)))
                    .font_family("codicon")
                    .text_size(px(16.))
                    .text_color(theme.faint)
                    .child(icon("ea60"))
                    .into_any_element(),
            )
        } else {
            None
        };
        if shown_browsers > 0
            && let Some(add) = add_browser.take()
        {
            tab_list = tab_list.child(add);
        }
        strip = strip.child(tab_list);
        if let Some(add) = add_browser {
            strip = strip.child(add);
        }
        if hidden > 0 {
            let focus = self
                .tab_strip
                .focus
                .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            let anchor = cx.weak_entity();
            strip = strip.child(
                overflow_button(
                    &focus,
                    hidden,
                    self.tab_strip.open,
                    cx.listener(move |app, _, _, cx| {
                        app.tab_strip.open = !app.tab_strip.open;
                        cx.notify();
                    }),
                )
                .child(
                    gpui::canvas(
                        move |bounds, _, cx| {
                            let _ = anchor.update(cx, |app, _| app.tab_strip.anchor = Some(bounds));
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                ),
            );
        } else {
            self.tab_strip.open = false;
        }
        strip.into_any_element()
    }

    pub(super) fn render_tab_overflow(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !self.tab_strip.open {
            return div().into_any_element();
        }
        let Some(anchor) = self.tab_strip.anchor else {
            return div().into_any_element();
        };
        let theme = Theme::default();
        let items = self.titlebar_items();
        let metrics = items
            .iter()
            .map(|tab| {
                window.text_system().shape_line(
                    tab.name.clone().into(),
                    px(12.5),
                    &[TextRun {
                        len: tab.name.len(),
                        font: theme::ui_font(),
                        color: theme.text.into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
            })
            .collect::<Vec<_>>();
        let width = metrics
            .iter()
            .map(|line| f32::from(line.width()) + 46.)
            .fold(200_f32, f32::max)
            .min(300.);
        let mut menu = div()
            .id("tab-overflow-menu")
            .role(gpui::Role::Menu)
            .aria_label("All open tabs")
            .absolute()
            // The portal wrapper and the menu each add the reference's 6px
            // offset; their combined inset is visible in the real window.
            .top(anchor.bottom() + px(12.))
            .right((window.viewport_size().width - anchor.right()).max(px(0.)))
            .w(px(width))
            .max_h(px(320.))
            .overflow_y_scroll()
            .p(px(4.))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.elevated)
            .shadow(theme.large_shadow())
            .flex()
            .flex_col()
            .on_mouse_down_out(cx.listener(|app, event: &gpui::MouseDownEvent, _, cx| {
                if !app
                    .tab_strip
                    .anchor
                    .is_some_and(|bounds| bounds.contains(&event.position))
                {
                    app.tab_strip.open = false;
                    cx.notify();
                }
            }));
        for (index, tab) in items.into_iter().enumerate() {
            // CSS normal line height includes the fallback face, so emoji
            // labels can be taller than labels using only the UI font.
            let (line_height, baseline_delta) =
                crate::typography::normal_line_metrics(&metrics[index], window);
            let line_height = line_height.max(15.);
            let target = tab.target;
            let browser = matches!(target, Target::Browser(_));
            let item = div()
                .id(SharedString::from(format!("tab-overflow-item-{index}")))
                .role(gpui::Role::MenuItem)
                .aria_label(SharedString::from(tab.name.clone()))
                .h(px(line_height + 12.))
                .flex_shrink_0()
                .px(px(8.))
                .rounded(px(7.))
                .flex()
                .items_center()
                .gap(px(7.))
                .text_size(px(12.5))
                .line_height(px(line_height))
                .text_color(theme.text)
                .cursor_pointer()
                .bg(if tab.active {
                    theme.inset
                } else {
                    theme.elevated
                })
                .hover(|style| style.bg(theme.selected))
                .on_click(cx.listener(move |app, _, window, cx| {
                    app.select_titlebar_tab(&target, false, window, cx)
                }))
                .child(if browser {
                    crate::icons::glyph("globe", 16., theme.faint).into_any_element()
                } else {
                    crate::tree_icons::tab_glyph(&tab.name).into_any_element()
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .mt(px(baseline_delta * 2.))
                        .truncate()
                        .child(SharedString::from(tab.name)),
                );
            menu = menu.child(crate::scroll::focus_control(
                SharedString::from(format!("tab-overflow-focus-{index}")),
                self.tab_strip.scroll.clone(),
                true,
                item,
            ));
        }
        crate::scroll::AutoScroll::new(self.tab_strip.scroll.clone(), move |scroll, gutter| {
            menu.track_scroll(&scroll)
                .scrollbar_width(gutter)
                .into_any_element()
        })
        .into_any_element()
    }

    pub(super) fn dismiss_tab_overflow(&mut self) -> bool {
        std::mem::take(&mut self.tab_strip.open)
    }
}

// GPUI's focused clickable div already activates on Enter/Space key-up. Keep a
// single click path: an additional key-down toggle opens and closes in one key.
fn overflow_button(
    focus: &gpui::FocusHandle,
    hidden: usize,
    open: bool,
    activate: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    let theme = Theme::default();
    let click_focus = focus.clone();
    div()
        .id("titlebar-tab-overflow")
        .role(gpui::Role::Button)
        .track_focus(focus)
        .aria_label(SharedString::from(format!(
            "All open tabs ({hidden} hidden)"
        )))
        .aria_expanded(open)
        .relative()
        .w(px(26.))
        .h(px(26.))
        .flex_shrink_0()
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .font_family("codicon")
        .text_size(px(16.))
        .text_color(theme.dim)
        .on_click(move |event, window, cx| {
            window.focus(&click_focus, cx);
            activate(event, window, cx);
        })
        .child(icon("eab4"))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct OverflowFixture {
        focus: gpui::FocusHandle,
        open: bool,
        activations: usize,
    }
    impl Render for OverflowFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(overflow_button(
                &self.focus,
                4,
                self.open,
                cx.listener(|this, _, _, cx| {
                    this.open = !this.open;
                    this.activations += 1;
                    cx.notify();
                }),
            ))
        }
    }
    #[gpui::test]
    fn keyboard_overflow_activation_toggles_once_per_press_release(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle().tab_stop(true);
            window.focus(&focus, cx);
            OverflowFixture {
                focus,
                open: false,
                activations: 0,
            }
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(200.), px(100.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        for (key, count, open) in [
            ("enter", 1, true),
            ("space", 2, false),
            ("cmd-enter", 2, false),
        ] {
            let keystroke = gpui::Keystroke::parse(key).unwrap();
            cx.simulate_event(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            draw(cx);
            cx.update(|_, cx| {
                assert_eq!(
                    view.read(cx).activations,
                    count - usize::from(key != "cmd-enter")
                )
            });
            cx.simulate_event(gpui::KeyUpEvent { keystroke });
            draw(cx);
            cx.update(|_, cx| {
                assert_eq!(view.read(cx).activations, count);
                assert_eq!(view.read(cx).open, open);
            });
        }
    }
    #[test]
    fn browser_body_state_prefers_error_then_empty_then_loading() {
        assert_eq!(
            browser_body_state("", false, Some("failed")),
            BrowserBodyState::Error
        );
        assert_eq!(
            browser_body_state("https://example.com", false, Some("failed")),
            BrowserBodyState::Error
        );
        assert_eq!(browser_body_state("", true, None), BrowserBodyState::Empty);
        assert_eq!(
            browser_body_state("https://example.com", true, None),
            BrowserBodyState::Loading
        );
        assert_eq!(
            browser_body_state("https://example.com", false, None),
            BrowserBodyState::Ready
        );
    }

    #[test]
    fn browser_active_after_close_matches_source_index_rules() {
        assert_eq!(browser_active_after_close(3, 0, 0), Some(0));
        assert_eq!(browser_active_after_close(3, 1, 0), Some(0));
        assert_eq!(browser_active_after_close(3, 1, 1), Some(1));
        assert_eq!(browser_active_after_close(3, 2, 1), Some(1));
        assert_eq!(browser_active_after_close(3, 0, 2), Some(0));
        assert_eq!(browser_active_after_close(2, 1, 1), Some(0));
        assert_eq!(browser_active_after_close(1, 0, 0), None);
        assert_eq!(browser_active_after_close(0, 0, 0), None);
        assert_eq!(browser_active_after_close(2, 2, 0), None);
    }

    #[test]
    fn browser_tab_labels_match_source_title_hostname_and_empty_rules() {
        assert_eq!(browser_tab_label("", ""), "New tab");
        assert_eq!(
            browser_tab_label("https://www.example.com/path", "  Example  "),
            "Example"
        );
        assert_eq!(
            browser_tab_label("https://www.example.com/path?q=1", ""),
            "example.com"
        );
        assert_eq!(
            browser_tab_label("http://localhost:3000/app", ""),
            "localhost"
        );
        assert_eq!(browser_tab_label("not a url", ""), "not a url");
        assert_eq!(browser_tab_label("about:blank", ""), "");
    }

    #[test]
    fn overflow_always_leaves_access_to_every_file_and_browser() {
        assert_eq!(visible(720., 3, 0), (3, 0, 0));
        assert_eq!(visible(500., 7, 0), (5, 0, 2));
        assert_eq!(visible(500., 7, 2), (2, 2, 5));
        assert_eq!(visible(120., 2, 1), (0, 0, 3));
        assert_eq!(visible(0., 0, 0), (0, 0, 0));
        for width in [0., 50., 120., 500., 900.] {
            for files in 0..15 {
                for browsers in 0..8 {
                    let (f, b, hidden) = visible(width, files, browsers);
                    assert_eq!(f + b + hidden, files + browsers);
                    let reserved =
                        if browsers > 0 { 28. } else { 0. } + if hidden > 0 { 30. } else { 0. };
                    assert!(f as f32 * 90. + b as f32 * 120. <= (width - reserved).max(0.));
                }
            }
        }
    }
}
