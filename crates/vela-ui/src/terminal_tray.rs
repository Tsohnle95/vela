//! Tray input stays scoped to the QA/application window, including drag release.
use super::*;
impl VelaApp {
    pub(super) fn tray_drag_observer(&self, cx: &mut Context<Self>) -> AnyElement {
        let movement = cx.weak_entity();
        let release = movement.clone();
        gpui::canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let movement = movement.clone();
                window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, cx| {
                    if !phase.capture() {
                        return;
                    }
                    let viewport = f32::from(window.viewport_size().height);
                    let active = movement
                        .update(cx, |app, cx| {
                            if !app.tray.dragging() {
                                return app.update_terminal_selection(event, cx);
                            }
                            if event.pressed_button == Some(MouseButton::Left) {
                                app.tray.move_to(f32::from(event.position.y), viewport);
                            } else {
                                app.tray.finish();
                            }
                            cx.notify();
                            true
                        })
                        .unwrap_or(false);
                    if active {
                        cx.stop_propagation();
                    }
                });
                let release = release.clone();
                window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, _, cx| {
                    if !phase.capture() {
                        return;
                    }
                    if event.button != MouseButton::Left {
                        let handled = release
                            .update(cx, |app, cx| app.finish_terminal_mouse(event, cx))
                            .unwrap_or(false);
                        if handled {
                            cx.stop_propagation();
                        }
                        return;
                    }
                    let active = release
                        .update(cx, |app, cx| {
                            if app.finish_terminal_mouse(event, cx) {
                                return true;
                            }
                            if let Some(view) = app.terminals.active_mut()
                                && view.selecting
                            {
                                view.selecting = false;
                                cx.notify();
                                return true;
                            }
                            if app.tray.finish() {
                                cx.notify();
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);
                    if active {
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .size_full()
        .into_any_element()
    }
}
