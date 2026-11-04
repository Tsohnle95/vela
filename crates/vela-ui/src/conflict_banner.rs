//! Match CSS's minimum word width so narrow conflict messages cannot collapse
//! to a single glyph column and push the retained editor off screen.
use super::*;
pub(super) fn render(
    change: ExternalChange,
    controls: impl IntoElement,
    window: &Window,
) -> AnyElement {
    let theme = Theme::default();
    let message = if change.deleted {
        "This file was deleted outside Vela. Your edits are safe and saving is paused."
    } else {
        "This file changed outside Vela. Your edits are safe and saving is paused."
    };
    let minimum = message
        .split_whitespace()
        .map(|word| {
            window
                .text_system()
                .shape_line(
                    word.to_owned().into(),
                    px(12.),
                    &[TextRun {
                        len: word.len(),
                        font: theme::ui_font(),
                        color: theme.text.into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width()
        })
        .fold(px(1.), |a, b| a.max(b));
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .px(px(12.))
        .py(px(8.))
        .border_b_1()
        .border_color(theme.yellow.opacity(0.45))
        .bg(theme.yellow.opacity(0.1))
        .text_color(theme.text)
        .text_size(px(12.))
        .child(
            div()
                .flex_1()
                .min_w(minimum)
                .child(message)
                .when_some(change.read_error, |text, error| text.child(error)),
        )
        .child(controls)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Banner {
        editor: Rc<std::cell::Cell<Option<Bounds<gpui::Pixels>>>>,
    }
    impl Render for Banner {
        fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let measured = self.editor.clone();
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(super::render(
                    ExternalChange {
                        deleted: true,
                        ..Default::default()
                    },
                    div().w(px(420.)).h(px(28.)).flex_shrink_0(),
                    window,
                ))
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .child("Editor remains reachable")
                        .child(
                            gpui::canvas(
                                move |bounds, _, _| measured.set(Some(bounds)),
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .inset_0(),
                        ),
                )
        }
    }
    #[gpui::test]
    fn narrow_conflict_message_keeps_editor_space(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| Banner {
            editor: Rc::default(),
        });
        for width in [296., 448., 1200.] {
            cx.simulate_resize(size(px(width), px(500.)));
            cx.draw(point(px(0.), px(0.)), size(px(width), px(500.)), |_, _| {
                view.clone().into_any_element()
            });
            cx.update(|_, cx| {
                let editor = view.read(cx).editor.get().unwrap();
                assert!(
                    editor.size.height > px(100.),
                    "conflict consumed editor at width{width}: {editor:?}"
                );
                assert!(editor.bottom() <= px(500.));
            });
        }
    }
}
