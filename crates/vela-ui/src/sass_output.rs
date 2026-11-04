//! Scrollable, whitespace-preserving Sass output, separate from editor errors.
use super::*;
use gpui::{ClipboardItem, FocusHandle, MouseDownEvent};

struct LogClock {
    #[cfg(target_os = "macos")]
    formatter: objc2::rc::Retained<objc2_foundation::NSDateFormatter>,
}
impl LogClock {
    fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            use objc2_foundation::{NSDateFormatter, NSLocale, NSString};
            let formatter = NSDateFormatter::new();
            // `toLocaleTimeString()` follows the locale's preferred hour cycle
            // (`j`) and includes seconds (`ms`). A localized template matches
            // both a 12/24-hour system preference and locale field ordering.
            formatter.setLocale(Some(&NSLocale::currentLocale()));
            formatter.setLocalizedDateFormatFromTemplate(&NSString::from_str("jms"));
            Self { formatter }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {}
        }
    }
    fn format(&self, time_ms: u64) -> String {
        #[cfg(target_os = "macos")]
        {
            let date =
                objc2_foundation::NSDate::dateWithTimeIntervalSince1970(time_ms as f64 / 1000.);
            self.formatter.stringFromDate(&date).to_string()
        }
        #[cfg(not(target_os = "macos"))]
        {
            chrono::DateTime::from_timestamp_millis(time_ms.min(i64::MAX as u64) as i64)
                .map(|time| time.with_timezone(&chrono::Local).format("%X").to_string())
                .unwrap_or_default()
        }
    }

    #[cfg(all(test, target_os = "macos"))]
    fn with_locale(identifier: &str, seconds_from_gmt: i32) -> Self {
        use objc2_foundation::{NSDateFormatter, NSLocale, NSString, NSTimeZone};
        let formatter = NSDateFormatter::new();
        formatter.setLocale(Some(&NSLocale::localeWithLocaleIdentifier(
            &NSString::from_str(identifier),
        )));
        formatter.setTimeZone(Some(&NSTimeZone::timeZoneForSecondsFromGMT(
            seconds_from_gmt as isize,
        )));
        formatter.setLocalizedDateFormatFromTemplate(&NSString::from_str("jms"));
        Self { formatter }
    }
}

/// Sass output is a read-only log. Drag selection is line-based because each
/// rendered row is a preformatted message; the selected rows can be copied with
/// Cmd+C or the toolbar action without enabling any editing.
pub(super) struct SassOutput {
    state: vela_engine::SassWatchState,
    directory: String,
    scroll: crate::scroll::AutoScrollHandle,
    anchor: Option<usize>,
    cursor: Option<usize>,
    dragging: bool,
    focus: FocusHandle,
    clock: LogClock,
}

impl SassOutput {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: Default::default(),
            directory: String::new(),
            scroll: Default::default(),
            anchor: None,
            cursor: None,
            dragging: false,
            focus: cx.focus_handle().tab_stop(true),
            clock: LogClock::new(),
        }
    }

    pub(super) fn sync(
        &mut self,
        state: vela_engine::SassWatchState,
        directory: String,
        scroll: crate::scroll::AutoScrollHandle,
        cx: &mut Context<Self>,
    ) {
        let changed = self.state != state || self.directory != directory;
        self.state = state;
        self.directory = directory;
        self.scroll = scroll;
        if changed {
            self.anchor = None;
            self.cursor = None;
            self.dragging = false;
            cx.notify();
        }
    }

    fn directory(&self) -> &str {
        if self.state.directory.is_empty() {
            &self.directory
        } else {
            &self.state.directory
        }
    }

    fn log_count(&self) -> usize {
        self.state.lines.iter().take(100).count()
    }

    fn selection_range(&self) -> Option<(usize, usize)> {
        let (anchor, cursor) = (self.anchor?, self.cursor?);
        if self.log_count() == 0 {
            return None;
        }
        let last = self.log_count() - 1;
        Some((anchor.min(cursor).min(last), anchor.max(cursor).min(last)))
    }

    fn log_text(&self, index: usize) -> Option<String> {
        let line = self.state.lines.get(index)?;
        Some(format!(
            "{} · {}",
            self.clock.format(line.time_ms),
            line.message
        ))
    }

    pub(super) fn selection_text(&self) -> Option<String> {
        let (start, end) = self.selection_range()?;
        let text = (start..=end)
            .filter_map(|index| self.log_text(index))
            .collect::<Vec<_>>()
            .join("\n");
        (!text.is_empty()).then_some(text)
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.selection_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    fn begin_selection(
        &mut self,
        index: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        if event.modifiers.shift && self.anchor.is_some() {
            self.cursor = Some(index);
        } else {
            self.anchor = Some(index);
            self.cursor = Some(index);
        }
        self.dragging = true;
        cx.notify();
    }

    fn update_selection(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.dragging && self.anchor.is_some() {
            self.cursor = Some(index);
            cx.notify();
        }
    }
}

impl Render for SassOutput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        let directory = self.directory().to_owned();
        let selection = self.selection_range();
        let selected =
            |index: usize| selection.is_some_and(|(start, end)| index >= start && index <= end);
        let rows = self
            .state
            .lines
            .iter()
            .take(100)
            .enumerate()
            .map(|(index, line)| {
                let text = self.log_text(index).unwrap_or_else(|| line.message.clone());
                let select = selected(index);
                let down = index;
                let moved = index;
                div()
                    .id(SharedString::from(format!("sass-log-{index}")))
                    .debug_selector(move || format!("sass-log-{index}"))
                    .role(Role::Label)
                    .aria_label(text.clone())
                    .flex_shrink_0()
                    .w_full()
                    .min_w_0()
                    .mb(px(8.))
                    .font_family(theme::FONT_MONO)
                    .text_size(px(12.))
                    .line_height(px(19.2))
                    .text_color(match line.level {
                        vela_engine::SassLogLevel::Error => theme.red,
                        vela_engine::SassLogLevel::Warning => theme.yellow,
                        vela_engine::SassLogLevel::Info => theme.text,
                    })
                    .when(select, |row| row.bg(theme.selected))
                    .cursor(gpui::CursorStyle::IBeam)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |output, event: &MouseDownEvent, window, cx| {
                            output.begin_selection(down, event, window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_move(cx.listener(
                        move |output, event: &gpui::MouseMoveEvent, _, cx| {
                            if event.pressed_button == Some(MouseButton::Left) {
                                output.update_selection(moved, cx);
                            }
                        },
                    ))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|output, _, _, cx| {
                            output.dragging = false;
                            cx.notify();
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|output, _, _, cx| {
                            output.dragging = false;
                            cx.notify();
                        }),
                    )
                    .child(crate::preformatted::PreformattedText::new(text, true))
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        let has_selection = selection.is_some();
        let copy = has_selection.then(|| {
            div()
                .id("sass-copy-selection")
                .debug_selector(|| "sass-copy-selection".into())
                .role(Role::Button)
                .aria_label("Copy selected Sass output")
                .flex_shrink_0()
                .px(px(8.))
                .py(px(3.))
                .rounded(px(5.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.inset)
                .text_size(px(10.))
                .text_color(theme.text)
                .cursor_pointer()
                .hover(|style| style.bg(theme.selected))
                .on_click(cx.listener(|output, _, _, cx| {
                    output.copy_selection(cx);
                }))
                .child("Copy")
                .into_any_element()
        });
        let focus = self.focus.clone();
        let state = self.state.clone();
        let error_shown = state
            .lines
            .iter()
            .any(|line| line.level == vela_engine::SassLogLevel::Error);
        let copy_action = cx.listener(|output, _: &crate::input::Copy, _, cx| {
            output.copy_selection(cx);
        });
        crate::scroll::AutoScroll::new(self.scroll.clone(), move |handle, gutter| {
            div()
                .id("sass-output-panel")
                .role(Role::Log)
                .aria_label(format!("Sass output for {directory}"))
                .size_full()
                .min_w_0()
                .overflow_y_scroll()
                .track_scroll(&handle)
                .scrollbar_width(gutter)
                .track_focus(&focus)
                .on_action(copy_action)
                .px(px(14.))
                .py(px(10.))
                .flex()
                .flex_col()
                .font_family(theme::FONT_UI)
                .text_size(px(13.))
                .child(
                    div()
                        .flex_shrink_0()
                        .w_full()
                        .mb(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(theme.dim)
                                .child(crate::preformatted::PreformattedText::new(directory, true)),
                        )
                        .children(copy),
                )
                .when(state.lines.is_empty() && state.error.is_none(), |column| {
                    column.child(div().flex_shrink_0().child(
                        "Enable Watch Sass in the footer to compile saved Sass files to CSS.",
                    ))
                })
                .when(!error_shown, |column| {
                    column.when_some(state.error.clone(), |column, error| {
                        column.child(
                            div()
                                .id("sass-output-error")
                                .role(Role::Label)
                                .aria_label(error.clone())
                                .flex_shrink_0()
                                .text_color(theme.red)
                                .child(crate::preformatted::PreformattedText::new(error, true)),
                        )
                    })
                })
                .children(rows)
                .into_any_element()
        })
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(count: usize) -> vela_engine::SassWatchState {
        vela_engine::SassWatchState {
            lines: (0..count)
                .map(|index| vela_engine::SassLine {
                    time_ms: 0,
                    level: if index % 2 == 0 {
                        vela_engine::SassLogLevel::Error
                    } else {
                        vela_engine::SassLogLevel::Warning
                    },
                    message: format!(
                        "{index}: {}\n  kept indentation 🌍",
                        "long-path-".repeat(24)
                    ),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[gpui::test]
    fn hundred_wrapped_logs_scroll_to_the_last_row_and_contract_after_clear(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| SassOutput::new(cx));
        view.update(cx, |view, cx| {
            view.sync(state(100), "/fixture".into(), Default::default(), cx)
        });
        for width in [296., 448., 1200.] {
            cx.simulate_resize(size(px(width), px(180.)));
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(width), px(180.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
            let handle = view.read_with(cx, |view, _| view.scroll.native_handle());
            assert!(handle.max_offset().y > px(1000.));
            handle.set_offset(point(px(0.), -handle.max_offset().y));
            cx.draw(point(px(0.), px(0.)), size(px(width), px(180.)), |_, _| {
                view.clone().into_any_element()
            });
            let last = cx.debug_bounds("sass-log-99").unwrap();
            assert!(
                last.bottom() <= px(180.),
                "last log unreachable at width{width}: {last:?}"
            );
            assert!(last.bottom() > px(0.));
            assert!(
                last.size.width <= px(width - 28.),
                "wrapped output overflowed horizontally: {last:?}"
            );
        }
        view.update(cx, |view, cx| {
            view.state.lines.clear();
            cx.notify();
        });
        for _ in 0..4 {
            cx.draw(point(px(0.), px(0.)), size(px(1200.), px(180.)), |_, _| {
                view.clone().into_any_element()
            });
        }
        let handle = view.read_with(cx, |view, _| view.scroll.native_handle());
        assert_eq!(handle.max_offset().y, px(0.));
        assert_eq!(handle.offset().y, px(0.));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn localized_clock_follows_locale_hour_cycle_and_includes_seconds() {
        let us = LogClock::with_locale("en_US", 0);
        let us_midnight = us.format(0);
        assert!(
            us_midnight.contains("12:00:00"),
            "US clock should use a 12-hour cycle: {us_midnight:?}"
        );
        assert!(
            us_midnight.to_ascii_uppercase().contains("AM"),
            "US clock should include the AM marker: {us_midnight:?}"
        );

        let gb = LogClock::with_locale("en_GB", 0);
        let gb_midnight = gb.format(0);
        assert!(
            gb_midnight.contains("00:00:00"),
            "UK clock should use a 24-hour cycle: {gb_midnight:?}"
        );
        assert!(
            !gb_midnight.to_ascii_uppercase().contains("AM")
                && !gb_midnight.to_ascii_uppercase().contains("PM"),
            "UK clock should not include a meridiem marker: {gb_midnight:?}"
        );
        assert_ne!(us.format(90_000), us_midnight);
    }

    #[gpui::test]
    fn drag_selection_copies_selected_log_lines(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| SassOutput::new(cx));
        view.update(cx, |view, cx| {
            let mut state = state(3);
            for (index, line) in state.lines.iter_mut().enumerate() {
                line.message = format!("log {index}");
            }
            view.sync(state, "/fixture".into(), Default::default(), cx);
        });
        cx.simulate_resize(size(px(900.), px(260.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(900.), px(260.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let first = cx.debug_bounds("sass-log-0").unwrap().center();
        let last = cx.debug_bounds("sass-log-2").unwrap().center();
        cx.simulate_event(gpui::MouseDownEvent {
            position: first,
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        draw(cx);
        cx.simulate_event(gpui::MouseMoveEvent {
            position: last,
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        draw(cx);
        cx.simulate_event(gpui::MouseUpEvent {
            position: last,
            button: MouseButton::Left,
            ..Default::default()
        });
        draw(cx);
        let expected = view.read_with(cx, |view, _| {
            (0..3)
                .map(|index| view.log_text(index).unwrap())
                .collect::<Vec<_>>()
                .join("\n")
        });
        assert_eq!(
            view.read_with(cx, |view, _| view.selection_text()),
            Some(expected)
        );
        let copied = view.read_with(cx, |view, _| view.selection_text()).unwrap();
        assert!(
            copied
                .lines()
                .next()
                .is_some_and(|line| line.contains(':') && line.contains(" · ")),
            "selected logs keep their localized clock prefix: {copied:?}"
        );
        assert!(
            view.read_with(cx, |view, _| view.selection_range()) == Some((0, 2)),
            "dragging from the first to the third log selects the full range"
        );
        assert!(
            cx.debug_bounds("sass-copy-selection").is_some(),
            "a Copy action appears while log lines are selected"
        );
        assert!(view.update(cx, |view, cx| view.copy_selection(cx)));
    }
}
