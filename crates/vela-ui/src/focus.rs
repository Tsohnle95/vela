//! CSS-compatible keyboard focus outlines, outside the control's border box.
use gpui::{
    BorderStyle, Div, FocusHandle, Hsla, IntoElement, MouseDownEvent, ScrollWheelEvent, Stateful,
    Window, canvas, prelude::*, px,
};
use std::{cell::Cell, rc::Rc};

/// Pointer motion changes hover state, but does not clear CSS :focus-visible.
#[derive(Clone, Default)]
pub(crate) struct KeyboardMode(Rc<Cell<bool>>);
impl KeyboardMode {
    pub(crate) fn key_down(&self) -> bool {
        !self.0.replace(true)
    }
    fn visible(&self, focus: &FocusHandle, window: &Window) -> bool {
        focus.is_focused(window) && self.0.get()
    }
    pub(crate) fn pointer_observer(&self) -> impl IntoElement {
        let mode = self.clone();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                window.on_mouse_event(|event: &ScrollWheelEvent, phase, _, _| {
                    if phase.capture() {
                        tracing::trace!(?event, "window scroll input");
                    }
                });
                let mode = mode.clone();
                window.on_mouse_event(move |_: &MouseDownEvent, phase, window, _| {
                    if phase.capture() && mode.0.replace(false) {
                        window.refresh();
                    }
                });
            },
        )
        .absolute()
        .size_full()
    }
}

pub(crate) fn outline(
    control: Stateful<Div>,
    focus: &FocusHandle,
    radius: f32,
    width: f32,
    border: f32,
    color: Hsla,
    keyboard: &KeyboardMode,
) -> Stateful<Div> {
    outline_style(
        control,
        focus,
        OutlineStyle {
            radius,
            width,
            border,
            offset: 1.,
        },
        color,
        keyboard,
    )
}

pub(crate) struct OutlineStyle {
    pub radius: f32,
    pub width: f32,
    pub border: f32,
    pub offset: f32,
}

pub(crate) fn outline_style(
    control: Stateful<Div>,
    focus: &FocusHandle,
    style: OutlineStyle,
    color: Hsla,
    keyboard: &KeyboardMode,
) -> Stateful<Div> {
    let OutlineStyle {
        radius,
        width,
        border,
        offset,
    } = style;
    let focus = focus.clone();
    let keyboard = keyboard.clone();
    // Absolute children start inside the parent border in GPUI.
    let outset = width + offset + border;
    control.relative().child(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                tracing::trace!(
                    ?bounds,
                    focused = focus.is_focused(window),
                    keyboard = keyboard.0.get(),
                    "keyboard outline paint"
                );
                if keyboard.visible(&focus, window) {
                    // Reference outline-offset is 1px; Attach overrides the 1px width to 2px.
                    window.paint_quad(
                        gpui::outline(bounds, color, BorderStyle::Solid)
                            .border_widths(px(width))
                            .corner_radii(px(if radius == 0. {
                                0.
                            } else {
                                radius + width + offset
                            })),
                    );
                }
            },
        )
        .absolute()
        .top(px(-outset))
        .bottom(px(-outset))
        .left(px(-outset))
        .right(px(-outset)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, MouseButton, MouseMoveEvent, Render, TestAppContext, VisualTestContext, div,
        point, rgb, size,
    };

    struct Controls {
        first: FocusHandle,
        second: FocusHandle,
        keyboard: KeyboardMode,
    }
    impl Render for Controls {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .relative()
                .capture_key_down(cx.listener(|this, _, _, cx| {
                    if this.keyboard.key_down() {
                        cx.notify();
                    }
                }))
                .on_key_down(crate::input::navigate_focus)
                .child(outline(
                    div().id("first").track_focus(&self.first).size(px(28.)),
                    &self.first,
                    8.,
                    2.,
                    0.,
                    rgb(0x0000ff).into(),
                    &self.keyboard,
                ))
                .child(outline(
                    div().id("second").track_focus(&self.second).size(px(28.)),
                    &self.second,
                    8.,
                    1.,
                    0.,
                    rgb(0x0000ff).into(),
                    &self.keyboard,
                ))
                .child(self.keyboard.pointer_observer())
        }
    }

    #[gpui::test]
    fn pointer_motion_preserves_keyboard_outline_until_pointer_down(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            window.activate_window();
            let first = cx.focus_handle().tab_stop(true);
            window.focus(&first, cx);
            Controls {
                first,
                second: cx.focus_handle().tab_stop(true),
                keyboard: KeyboardMode::default(),
            }
        });
        let draw = |cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(200.), px(100.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.keyboard.visible(&view.second, window));
        });
        cx.simulate_event(MouseMoveEvent {
            position: point(px(150.), px(80.)),
            ..Default::default()
        });
        draw(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!window.last_input_was_keyboard());
            assert!(view.keyboard.visible(&view.second, window));
        });
        cx.simulate_event(MouseDownEvent {
            position: point(px(150.), px(80.)),
            button: MouseButton::Left,
            ..Default::default()
        });
        draw(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.second.is_focused(window));
            assert!(!view.keyboard.visible(&view.second, window));
        });
    }
}
