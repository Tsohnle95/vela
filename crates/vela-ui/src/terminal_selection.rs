//! Mouse selection and clipboard ownership remain inside the terminal's window.
use super::*;
use vela_terminal::emulator::{
    MouseAction, MouseButton as EmuMouseButton, MouseModifiers, MouseReportMode, SelectionKind,
};

fn emu_button(button: gpui::MouseButton) -> EmuMouseButton {
    match button {
        MouseButton::Left => EmuMouseButton::Left,
        MouseButton::Right => EmuMouseButton::Right,
        MouseButton::Middle => EmuMouseButton::Middle,
        MouseButton::Navigate(_) => EmuMouseButton::None,
    }
}

fn emu_modifiers(modifiers: &gpui::Modifiers) -> MouseModifiers {
    MouseModifiers {
        shift: modifiers.shift,
        alt: modifiers.alt,
        control: modifiers.control,
    }
}

// xterm SelectionService: 50px threshold, 15-line maximum, 50ms interval.
fn scroll_amount(y: f32, top: f32, bottom: f32) -> i32 {
    let offset = if y < top {
        y - top
    } else if y > bottom {
        y - bottom
    } else {
        return 0;
    };
    let normalized = offset.clamp(-50., 50.) / 50.;
    (normalized.signum() + (normalized * 14. + 0.5).floor()) as i32
}

fn cell_at(
    view: &TerminalView,
    position: gpui::Point<gpui::Pixels>,
) -> Option<(usize, usize, bool)> {
    let (bounds, width) = view.geometry.get()?;
    let x = (f32::from(position.x - bounds.origin.x) - 8.).max(0.) / width;
    let y = (f32::from(position.y - bounds.origin.y) - 6.).max(0.) / 15.;
    Some((
        (y.floor() as usize).min(view.emulator.rows() - 1),
        (x.floor() as usize).min(view.emulator.cols() - 1),
        x.fract() >= 0.5 || x >= view.emulator.cols() as f32,
    ))
}

fn auto_scroll(view: &mut TerminalView) -> bool {
    let Some(position) = view.selection_pointer else {
        return false;
    };
    let Some((bounds, _)) = view.geometry.get() else {
        return false;
    };
    let top = f32::from(bounds.top()) + 6.;
    let amount = scroll_amount(
        f32::from(position.y),
        top,
        top + view.emulator.rows() as f32 * 15.,
    );
    if amount != 0 {
        let before = view.emulator.display_offset();
        view.emulator.scroll(-amount);
        if let Some((row, col, right)) = selection_endpoint(view, position) {
            view.emulator.update_selection(row, col, right);
        }
        return before != view.emulator.display_offset();
    }
    false
}

fn selection_endpoint(
    view: &TerminalView,
    position: gpui::Point<gpui::Pixels>,
) -> Option<(usize, usize, bool)> {
    let (row, col, right) = cell_at(view, position)?;
    let (bounds, _) = view.geometry.get()?;
    let top = f32::from(bounds.top()) + 6.;
    if !view.emulator.block_selection() {
        if f32::from(position.y) < top {
            return Some((row, 0, false));
        }
        if f32::from(position.y) > top + view.emulator.rows() as f32 * 15. {
            return Some((row, view.emulator.cols() - 1, true));
        }
    }
    Some((row, col, right))
}

impl VelaApp {
    pub(super) fn scroll_terminal(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.terminals.active_mut() else {
            return;
        };
        if view.emulator.mouse_report_mode() != MouseReportMode::Off {
            let delta = event.delta.pixel_delta(px(15.)).y;
            if delta == px(0.) {
                return;
            }
            let button = if delta > px(0.) {
                EmuMouseButton::WheelUp
            } else {
                EmuMouseButton::WheelDown
            };
            let Some((row, col, _)) = cell_at(view, event.position) else {
                return;
            };
            let Some(bytes) = view.emulator.encode_mouse(
                MouseAction::Press,
                button,
                row,
                col,
                emu_modifiers(&event.modifiers),
            ) else {
                return;
            };
            let id = view.id.clone();
            self.engine.terminal_input(id, bytes);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        view.scroll_pixels += f32::from(event.delta.pixel_delta(px(15.)).y);
        let lines = (view.scroll_pixels / 15.).trunc() as i32;
        if lines != 0 {
            view.scroll_pixels -= lines as f32 * 15.;
            view.emulator.scroll(lines);
            if view.selecting
                && let Some((row, col, right)) = selection_endpoint(view, event.position)
            {
                view.emulator.update_selection(row, col, right);
            }
            cx.notify();
        }
        cx.stop_propagation();
    }

    pub(super) fn install_terminal_focus_reporting(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pending = self
            .terminals
            .views
            .iter()
            .enumerate()
            .filter(|(_, view)| !view.focus_reporting)
            .map(|(index, view)| {
                (
                    index,
                    view.id.clone(),
                    view.focus.clone(),
                    view.input.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (index, id, focus, input) in pending {
            let entered = id.clone();
            let enter = cx.on_focus(&focus, window, move |app, _, cx| {
                app.report_terminal_focus(&entered, true);
                cx.notify();
            });
            let left = id.clone();
            let leave = cx.on_blur(&focus, window, move |app, _, cx| {
                app.report_terminal_focus(&left, false);
                cx.notify();
            });
            let cancel =
                super::terminal_composition::cancel_composition_on_blur(&focus, input, window, cx);
            if let Some(view) = self.terminals.views.get_mut(index) {
                view.focus_reporting = true;
                view.input_subscriptions.push(enter);
                view.input_subscriptions.push(leave);
                view.input_subscriptions.push(cancel);
            }
        }
    }

    pub(super) fn report_terminal_focus(&mut self, id: &str, gained: bool) {
        let Some(bytes) = self
            .terminals
            .views
            .iter()
            .find(|view| view.id == id)
            .and_then(|view| view.emulator.focus_report(gained))
        else {
            return;
        };
        self.engine.terminal_input(id.to_owned(), bytes);
    }

    pub(super) fn finish_terminal_mouse(
        &mut self,
        event: &gpui::MouseUpEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(view) = self.terminals.active_mut() else {
            return false;
        };
        let Some(button) = view.mouse_button.take() else {
            return false;
        };
        view.mouse_cell = None;
        let Some((row, col, _)) = cell_at(view, event.position) else {
            return true;
        };
        let Some(bytes) = view.emulator.encode_mouse(
            MouseAction::Release,
            button,
            row,
            col,
            emu_modifiers(&event.modifiers),
        ) else {
            return true;
        };
        let id = view.id.clone();
        self.engine.terminal_input(id, bytes);
        cx.notify();
        true
    }

    pub(super) fn copy_terminal(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self
            .terminals
            .active()
            .and_then(|view| view.emulator.selection_text())
        {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
        }
        cx.stop_propagation();
    }

    pub(super) fn begin_terminal_selection(
        &mut self,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.terminals.active_mut() else {
            return;
        };
        if event.button != MouseButton::Left
            && view.emulator.mouse_report_mode() == MouseReportMode::Off
        {
            return;
        }
        let Some((row, col, right)) = cell_at(view, event.position) else {
            return;
        };
        if view.emulator.mouse_report_mode() != MouseReportMode::Off {
            let button = emu_button(event.button);
            let Some(bytes) = view.emulator.encode_mouse(
                MouseAction::Press,
                button,
                row,
                col,
                emu_modifiers(&event.modifiers),
            ) else {
                return;
            };
            view.mouse_button = Some(button);
            view.mouse_cell = Some((row, col));
            let id = view.id.clone();
            let focus = view.focus.clone();
            window.focus(&focus, cx);
            self.engine.terminal_input(id, bytes);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        window.focus(&view.focus, cx);
        let kind = if event.click_count >= 3 {
            SelectionKind::Line
        } else if event.click_count == 2 {
            SelectionKind::Word
        } else if event.modifiers.alt {
            SelectionKind::Block
        } else {
            SelectionKind::Simple
        };
        view.emulator
            .begin_selection(row, col, right, kind, event.modifiers.shift);
        view.selecting = true;
        view.selection_pointer = Some(event.position);
        view.selection_generation = view.selection_generation.wrapping_add(1);
        let generation = view.selection_generation;
        let id = view.id.clone();
        let handle = window.window_handle();
        cx.spawn(async move |app, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                let keep = handle
                    .update(cx, |_, window, cx| {
                        app.update(cx, |app, cx| {
                            app.tick_terminal_selection(
                                &id,
                                generation,
                                window.is_window_active(),
                                cx,
                            )
                        })
                    })
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
        cx.stop_propagation();
        cx.notify();
    }

    pub(super) fn update_terminal_selection(
        &mut self,
        event: &gpui::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(view) = self.terminals.active_mut() else {
            return false;
        };
        if view.emulator.mouse_report_mode() != MouseReportMode::Off {
            let action = if event.pressed_button.is_some() {
                MouseAction::Drag
            } else {
                MouseAction::Move
            };
            let button = event
                .pressed_button
                .map(emu_button)
                .unwrap_or(EmuMouseButton::None);
            let Some((row, col, _)) = cell_at(view, event.position) else {
                return false;
            };
            if view.mouse_cell == Some((row, col)) {
                return true;
            }
            let Some(bytes) = view.emulator.encode_mouse(
                action,
                button,
                row,
                col,
                emu_modifiers(&event.modifiers),
            ) else {
                return false;
            };
            view.mouse_cell = Some((row, col));
            let id = view.id.clone();
            self.engine.terminal_input(id, bytes);
            cx.notify();
            return true;
        }
        if !view.selecting {
            return false;
        }
        if let Some((row, col, right)) = selection_endpoint(view, event.position) {
            view.selection_pointer = Some(event.position);
            view.emulator.update_selection(row, col, right);
            cx.notify();
        }
        true
    }

    fn tick_terminal_selection(
        &mut self,
        id: &str,
        generation: u64,
        window_active: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let current = self.tray.open
            && !self.tray.snapped
            && self.terminal_panel == TerminalPanel::Terminal
            && self.terminals.active().is_some_and(|view| view.id == id);
        let Some(view) = self.terminals.get_mut(id) else {
            return false;
        };
        if view.selection_generation != generation || !view.selecting {
            return false;
        }
        if !current || !window_active {
            view.selecting = false;
            return false;
        }
        if auto_scroll(view) {
            cx.notify();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext;
    struct Owner {
        view: TerminalView,
    }
    #[gpui::test]
    fn outside_drag_scrolls_history_extends_selection_and_preserves_block_columns(
        cx: &mut gpui::TestAppContext,
    ) {
        let owner = cx.new(|cx| Owner {
            view: TerminalView::new("selection".into(), 20, 5, cx),
        });
        owner.update(cx, |owner, _| {
            let view = &mut owner.view;
            view.geometry.set(Some((
                Bounds::new(point(px(100.), px(100.)), size(px(200.), px(87.))),
                7.2,
            )));
            let output = (0..100)
                .map(|index| format!("ROW_{index:03}"))
                .collect::<Vec<_>>()
                .join("\r\n");
            view.emulator.feed(output.as_bytes());
            view.emulator
                .begin_selection(4, 6, true, SelectionKind::Simple, false);
            view.selection_pointer = Some(point(px(150.), px(50.)));
            assert!(auto_scroll(view));
            assert_eq!(view.emulator.display_offset(), 15);
            let text = view.emulator.selection_text().unwrap();
            assert!(text.contains("ROW_080") && text.contains("ROW_099"));
            view.selection_pointer = Some(point(px(150.), px(130.)));
            assert!(!auto_scroll(view), "inside pointer must stop scrolling");
            view.selection_pointer = Some(point(px(150.), px(240.)));
            assert!(auto_scroll(view));
            assert_eq!(view.emulator.display_offset(), 0);
            view.emulator
                .begin_selection(4, 4, false, SelectionKind::Block, false);
            view.selection_pointer = Some(point(px(150.), px(50.)));
            assert!(auto_scroll(view));
            let text = view.emulator.selection_text().unwrap();
            assert!(
                text.lines().all(|line| line.len() == 2),
                "block drag must retain its columns: {text:?}"
            );
        });
    }
    #[test]
    fn selection_autoscroll_matches_xterm_distance_rounding_and_clamps() {
        for (y, expected) in [
            (100., 0),
            (200., 0),
            (99., -1),
            (98., -2),
            (50., -15),
            (-500., -15),
            (201., 1),
            (202., 2),
            (250., 15),
            (900., 15),
        ] {
            assert_eq!(scroll_amount(y, 100., 200.), expected, "pointer y={y}");
        }
    }
}
