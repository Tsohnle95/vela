//! Platform text composition is local until committed; preedit never reaches a PTY.
use super::*;
use gpui::{
    Context, Entity, EntityInputHandler, FocusHandle, Pixels, Point, Subscription, UTF16Selection,
    Window,
};
use std::ops::Range;
use vela_core::text::TextBuffer;

pub(super) struct Input {
    pub buffer: TextBuffer,
    pub geometry: Rc<std::cell::Cell<Option<terminal_state::Geometry>>>,
    pub anchor: Rc<std::cell::Cell<vela_terminal::emulator::Cursor>>,
}
pub(super) struct Committed(pub String);
impl gpui::EventEmitter<Committed> for Input {}
impl Input {
    fn layout(&self, window: &Window) -> gpui::ShapedLine {
        let text = self.buffer.text();
        window.text_system().shape_line(
            text.to_owned().into(),
            px(12.),
            &[TextRun {
                len: text.len(),
                font: theme::source_editor_font(),
                color: Theme::default().text.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        )
    }
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self.buffer = Default::default();
        cx.notify();
    }
    pub fn new(geometry: Rc<std::cell::Cell<Option<terminal_state::Geometry>>>) -> Self {
        Self {
            buffer: TextBuffer::default(),
            geometry,
            anchor: Rc::new(std::cell::Cell::new(vela_terminal::emulator::Cursor {
                row: 0,
                col: 0,
            })),
        }
    }
    fn commit(&mut self, cx: &mut Context<Self>) {
        let text = self.buffer.text().to_owned();
        self.buffer = Default::default();
        if !text.is_empty() {
            cx.emit(Committed(text));
        }
        cx.notify();
    }
}

pub(super) fn cancel_composition_on_blur<T: 'static>(
    focus: &FocusHandle,
    input: Entity<Input>,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Subscription {
    cx.on_blur(focus, window, move |_, _, cx| {
        input.update(cx, |input, cx| input.cancel(cx));
    })
}

pub(super) fn platform_text(event: &KeyDownEvent, composing: bool) -> bool {
    let stroke = &event.keystroke;
    if stroke.modifiers.platform || stroke.modifiers.control {
        return false;
    }
    if composing {
        return true;
    }
    if stroke.modifiers.alt && !cfg!(target_os = "macos") {
        return false;
    }
    stroke.key == "space"
        || stroke.key.chars().count() == 1
        || stroke
            .key_char
            .as_ref()
            .is_some_and(|text| !text.is_empty() && text.chars().all(|ch| !ch.is_control()))
}

impl EntityInputHandler for Input {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let buffer = &self.buffer;
        let range = buffer.byte_range(range);
        *actual = Some(buffer.utf16_range(range.clone()));
        Some(buffer.text()[range].to_owned())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let buffer = &self.buffer;
        Some(UTF16Selection {
            range: buffer.utf16_range(buffer.selection()),
            reversed: buffer.reversed(),
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let buffer = &self.buffer;
        buffer.marked.clone().map(|range| buffer.utf16_range(range))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.commit(cx);
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let buffer = &mut self.buffer;
        let range = range
            .map(|range| buffer.byte_range(range))
            .or(buffer.marked.clone())
            .unwrap_or(buffer.selection());
        buffer.replace(range, text);
        self.commit(cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selection: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let buffer = &mut self.buffer;
        let range = range
            .map(|range| buffer.byte_range(range))
            .or(buffer.marked.clone())
            .unwrap_or(buffer.selection());
        buffer.mark(range, text, selection);
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (bounds, width) = self.geometry.get()?;
        let cursor = self.anchor.get();
        let range = self.buffer.byte_range(range);
        let layout = self.layout(window);
        let start = layout.x_for_index(range.start);
        let end = layout.x_for_index(range.end);
        Some(Bounds::new(
            bounds.origin
                + point(
                    px(8. + cursor.col as f32 * width) + start,
                    px(6. + cursor.row as f32 * 15.),
                ),
            size((end - start).max(px(1.)), px(15.)),
        ))
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let (bounds, width) = self.geometry.get()?;
        let x = position.x - bounds.origin.x - px(8. + self.anchor.get().col as f32 * width);
        Some(
            self.buffer
                .to_utf16(self.layout(window).closest_index_for_x(x)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext, Bounds, Entity, Keystroke, Subscription, TestAppContext, point, px, size,
    };
    use std::cell::Cell;
    struct Harness {
        inputs: Vec<Entity<Input>>,
        committed: Vec<(usize, String)>,
        _subscriptions: Vec<Subscription>,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    #[gpui::test]
    fn preedit_stays_local_and_commits_once_to_its_own_terminal(cx: &mut TestAppContext) {
        let (harness, cx) = cx.add_window_view(|_, cx| {
            let inputs: Vec<_> = (0..2)
                .map(|_| cx.new(|_| Input::new(Rc::default())))
                .collect();
            let subscriptions = inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    cx.subscribe(
                        input,
                        move |harness: &mut Harness, _, event: &Committed, _| {
                            harness.committed.push((index, event.0.clone()))
                        },
                    )
                })
                .collect();
            Harness {
                inputs,
                committed: Vec::new(),
                _subscriptions: subscriptions,
            }
        });
        let inputs = harness.read_with(cx, |harness, _| harness.inputs.clone());
        inputs[0].update_in(cx, |input, window, cx| {
            input.replace_and_mark_text_in_range(None, "に", Some(1..1), window, cx);
            input.replace_and_mark_text_in_range(None, "日本🌍", Some(2..4), window, cx);
            assert_eq!(input.marked_text_range(window, cx), Some(0..4));
            assert_eq!(
                input.selected_text_range(false, window, cx).unwrap().range,
                2..4
            );
            let mut actual = None;
            assert_eq!(
                input
                    .text_for_range(0..4, &mut actual, window, cx)
                    .as_deref(),
                Some("日本🌍")
            );
            assert_eq!(actual, Some(0..4));
        });
        assert!(harness.read_with(cx, |harness, _| harness.committed.is_empty()));
        inputs[1].update_in(cx, |input, window, cx| {
            input.replace_text_in_range(None, "other", window, cx)
        });
        inputs[0].update_in(cx, |input, window, cx| {
            input.replace_text_in_range(Some(2..4), "界", window, cx);
            input.unmark_text(window, cx);
            assert!(input.buffer.text().is_empty());
            input.replace_and_mark_text_in_range(None, "cancel", None, window, cx);
            input.replace_and_mark_text_in_range(None, "", None, window, cx);
            input.unmark_text(window, cx);
            input.replace_and_mark_text_in_range(None, "discard", Some(7..7), window, cx);
            assert_eq!(input.marked_text_range(window, cx), Some(0..7));
            input.cancel(cx);
            assert!(input.buffer.text().is_empty());
            assert_eq!(input.marked_text_range(window, cx), None);
        });
        assert_eq!(
            harness.read_with(cx, |harness, _| harness.committed.clone()),
            [(1, "other".into()), (0, "日本界".into())]
        );
    }
    #[gpui::test]
    fn candidate_bounds_track_cursor_cells_and_astral_composition(cx: &mut TestAppContext) {
        let geometry = Rc::new(Cell::new(Some((
            Bounds::new(point(px(10.), px(20.)), size(px(160.), px(60.))),
            7.5,
        ))));
        let (harness, cx) = cx.add_window_view(|_, cx| Harness {
            inputs: vec![cx.new(|_| Input::new(geometry))],
            committed: Vec::new(),
            _subscriptions: Vec::new(),
        });
        let input = harness.read_with(cx, |harness, _| harness.inputs[0].clone());
        input.update_in(cx, |input, window, cx| {
            input
                .anchor
                .set(vela_terminal::emulator::Cursor { row: 1, col: 2 });
            input.replace_and_mark_text_in_range(None, "日本🌍", Some(4..4), window, cx);
            let bounds = input
                .bounds_for_range(
                    0..4,
                    Bounds::new(point(px(0.), px(0.)), size(px(1.), px(1.))),
                    window,
                    cx,
                )
                .unwrap();
            assert_eq!(
                bounds.origin,
                point(px(10. + 8. + 2. * 7.5), px(20. + 6. + 15.))
            );
            assert!(
                bounds.size.width > px(7.5),
                "the candidate anchor must span the composed glyphs: {bounds:?}"
            );
            assert_eq!(
                input.character_index_for_point(bounds.origin, window, cx),
                Some(0)
            );
            assert_eq!(
                input.character_index_for_point(
                    point(
                        bounds.origin.x + bounds.size.width + px(40.),
                        bounds.origin.y
                    ),
                    window,
                    cx
                ),
                Some(4),
                "astral composition is addressed in UTF-16"
            );
        });
    }

    #[test]
    fn printable_input_and_composition_are_separate_from_pty_protocol_keys() {
        let key = |key: &str| KeyDownEvent {
            keystroke: Keystroke::parse(key).unwrap(),
            is_held: false,
            prefer_character_input: false,
        };
        assert!(platform_text(&key("a"), false));
        assert!(platform_text(&key("space"), false));
        for name in ["enter", "backspace", "left", "f1", "ctrl-c", "cmd-c"] {
            assert!(!platform_text(&key(name), false), "{name}");
        }
        assert!(platform_text(&key("enter"), true));
        assert!(!platform_text(&key("ctrl-c"), true));
    }
}
