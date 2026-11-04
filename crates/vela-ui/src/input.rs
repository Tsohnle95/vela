//! Native editable text, including platform IME and UTF-16 accessibility ranges.
//! The platform handler follows GPUI's Apache-2.0 input example. Text editing
//! itself lives in vela-core so all fields share the same tested semantics.
use crate::theme::Theme;
use gpui::{prelude::*, *};
use std::{
    collections::HashSet,
    ops::Range,
    sync::Arc,
    time::Duration,
};
use vela_core::text::TextBuffer;

/// Source's global `::selection` rule paints selected text with the active
/// appearance's accent at 32% opacity. The native editor uses Monaco's
/// `editor.selectionBackground` instead, but every shared text input follows
/// this browser rule.
fn input_selection_fill(theme: &Theme) -> Rgba {
    Rgba {
        a: 0.32,
        ..theme.accent
    }
}

actions!(
    vela_edit,
    [
        Cut,
        Copy,
        Paste,
        SelectAll,
        Undo,
        Redo,
        RequestQuit,
        MinimizeWindow,
        ZoomWindow,
        BringWindowsToFront
    ]
);

/// OS edit selectors are also required by native child views and file dialogs.
pub fn install_menu(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", RequestQuit, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
    ]);
    cx.set_menus([
        Menu::new("Vela Native").items([
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Quit Vela", RequestQuit),
        ]),
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", Undo, OsAction::Undo),
            MenuItem::os_action("Redo", Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", Cut, OsAction::Cut),
            MenuItem::os_action("Copy", Copy, OsAction::Copy),
            MenuItem::os_action("Paste", Paste, OsAction::Paste),
            MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", MinimizeWindow),
            MenuItem::action("Zoom", ZoomWindow),
            MenuItem::separator(),
            MenuItem::action("Bring All to Front", BringWindowsToFront),
        ]),
    ]);
}

pub struct Changed(pub String);
pub fn navigate_focus(event: &KeyDownEvent, window: &mut Window, cx: &mut App) {
    let key = &event.keystroke;
    if key.key == "tab" && !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt {
        if key.modifiers.shift {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        cx.stop_propagation();
    }
}
/// Browsers blink the text caret at roughly this cadence; Source inherits the
/// platform textarea caret. Match the terminal's focus-aware phase so both
/// native caret surfaces feel the same.
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(530);

pub struct TextInput {
    pub buffer: TextBuffer,
    pub submit_on_enter: bool,
    pub max_utf16_length: Option<usize>,
    pub user_edited: bool,
    focus: FocusHandle,
    caret_visible: bool,
    caret_blink_running: bool,
    caret_blink_generation: u64,
    focus_observed_windows: HashSet<WindowId>,
    focus_subscriptions: Vec<Subscription>,
    placeholder: SharedString,
    label: SharedString,
    secure: bool,
    settings_field: bool,
    browser_field: bool,
    multiline: bool,
    selecting: bool,
    lines: Vec<PaintedLine>,
    bounds: Option<Bounds<Pixels>>,
    line_height: Pixels,
    scroll_y: Pixels,
    scroll_x: Pixels,
    pub key_guard: Option<std::rc::Rc<dyn Fn() -> bool>>,
    pub on_focus: Option<std::rc::Rc<dyn Fn()>>,
    pub native_undo: Option<std::rc::Rc<dyn Fn(bool) -> bool>>,
}
#[derive(Clone)]
struct PaintedLine {
    start: usize,
    origin: Point<Pixels>,
    layout: Arc<WrappedLine>,
}
impl EventEmitter<Changed> for TextInput {}
impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl TextInput {
    fn menu_key(&mut self, key: &str, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.key_guard.as_ref().is_some_and(|allowed| !allowed()) {
            if key == "z" && self.native_undo.as_ref().is_some_and(|undo| undo(shift)) {
                return;
            }
            cx.propagate();
            return;
        }
        self.on_key(
            &KeyDownEvent {
                keystroke: Keystroke {
                    key: key.into(),
                    key_char: None,
                    modifiers: Modifiers {
                        platform: true,
                        shift,
                        ..Default::default()
                    },
                },
                is_held: false,
                prefer_character_input: false,
            },
            window,
            cx,
        );
    }
    pub fn new(
        multiline: bool,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = placeholder.into();
        Self {
            buffer: TextBuffer::default(),
            submit_on_enter: false,
            max_utf16_length: None,
            user_edited: false,
            focus: cx.focus_handle().tab_stop(true),
            caret_visible: true,
            caret_blink_running: false,
            caret_blink_generation: 0,
            focus_observed_windows: HashSet::new(),
            focus_subscriptions: Vec::new(),
            label: placeholder.clone(),
            placeholder,
            secure: false,
            settings_field: false,
            browser_field: false,
            multiline,
            selecting: false,
            lines: vec![],
            bounds: None,
            line_height: px(17.),
            scroll_y: px(0.),
            scroll_x: px(0.),
            key_guard: None,
            on_focus: None,
            native_undo: None,
        }
    }
    pub fn password(label: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let mut input = Self::new(false, "", cx);
        input.label = label.into();
        input.secure = true;
        input.line_height = px(13.);
        input
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>) {
        self.label = label.into();
    }
    pub fn outer_bounds(&self) -> Option<Bounds<Pixels>> {
        self.bounds.map(|bounds| {
            let (x, y) = if self.settings_field {
                (10., 8.)
            } else if self.browser_field {
                (3., 2.)
            } else {
                (0., 0.)
            };
            Bounds::new(
                bounds.origin - point(px(x), px(y)),
                bounds.size + size(px(2. * x), px(2. * y)),
            )
        })
    }
    pub fn use_settings_field(&mut self) {
        self.settings_field = true;
        self.browser_field = false;
        self.line_height = px(13.);
    }
    pub fn use_browser_field(&mut self) {
        self.settings_field = false;
        self.browser_field = true;
        self.line_height = px(17.3333);
    }
    fn displayed_text(&self) -> SharedString {
        if self.secure {
            "•".repeat(self.buffer.text().encode_utf16().count()).into()
        } else {
            self.buffer.text().to_owned().into()
        }
    }
    fn display_index(&self, source: usize) -> usize {
        if self.secure {
            self.buffer.to_utf16(source) * "•".len()
        } else {
            source
        }
    }
    fn source_index(&self, display: usize) -> usize {
        if self.secure {
            self.buffer
                .byte_range(display / "•".len()..display / "•".len())
                .start
        } else {
            display.min(self.buffer.text().len())
        }
    }
    pub fn set_text(&mut self, text: String, cx: &mut Context<Self>) {
        self.user_edited = false;
        self.caret_visible = true;
        if self.buffer.text() != text {
            self.buffer = TextBuffer::new(text);
            self.scroll_y = px(0.);
            self.scroll_x = px(0.);
            cx.notify();
        }
    }

    /// Focus-aware caret phase. The timer exists only while this input owns
    /// focus in an active window; otherwise the caret stays steadily visible
    /// (and is not painted at all while blurred).
    fn sync_caret_blink(&mut self, blinking: bool, cx: &mut Context<Self>) {
        if !blinking {
            if self.caret_blink_running {
                self.caret_blink_running = false;
                self.caret_blink_generation = self.caret_blink_generation.wrapping_add(1);
            }
            self.caret_visible = true;
            return;
        }
        if self.caret_blink_running {
            return;
        }
        self.caret_blink_running = true;
        self.caret_visible = true;
        self.caret_blink_generation = self.caret_blink_generation.wrapping_add(1);
        let generation = self.caret_blink_generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CARET_BLINK_INTERVAL).await;
                let keep = this
                    .update(cx, |input, cx| {
                        if !input.caret_blink_running || input.caret_blink_generation != generation {
                            return false;
                        }
                        input.caret_visible = !input.caret_visible;
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
    }
    pub fn set_placeholder(&mut self, text: &'static str) {
        self.placeholder = text.into();
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        self.user_edited = true;
        self.caret_visible = true;
        cx.emit(Changed(self.buffer.text().to_owned()));
        cx.notify();
    }
    fn insertion(&self, range: Range<usize>, text: &str) -> String {
        // HTML single-line controls remove CR/LF; replacing them with spaces
        // changes pasted keys, addresses and provider answers.
        let text = if self.multiline {
            text.to_owned()
        } else {
            text.replace(['\n', '\r'], "")
        };
        let Some(maximum) = self.max_utf16_length else {
            return text;
        };
        let retained = self.buffer.text().encode_utf16().count()
            - self.buffer.text()[range].encode_utf16().count();
        let mut remaining = maximum.saturating_sub(retained);
        text.chars()
            .take_while(|character| {
                let fits = character.len_utf16() <= remaining;
                if fits {
                    remaining -= character.len_utf16();
                }
                fits
            })
            .collect()
    }
    fn index_at(&self, position: Point<Pixels>) -> usize {
        for line in &self.lines {
            if position.y < line.origin.y + line.layout.size(self.line_height).height {
                let local = point(
                    position.x - line.origin.x,
                    (position.y - line.origin.y).max(px(0.)),
                );
                let index = line
                    .layout
                    .closest_index_for_position(local, self.line_height)
                    .unwrap_or_else(|i| i);
                return self.source_index(line.start + index);
            }
        }
        self.buffer.text().len()
    }
    fn position_at(&self, index: usize) -> Option<Point<Pixels>> {
        let index = self.display_index(index);
        let line = self.lines.iter().rev().find(|line| index >= line.start)?;
        line.layout
            .position_for_index(
                (index - line.start).min(line.layout.len()),
                self.line_height,
            )
            .map(|p| line.origin + p)
    }
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.key_guard.as_ref().is_some_and(|allowed| !allowed()) {
            return;
        }
        // Any key press (including caret movement) restarts the visible phase.
        self.caret_visible = true;
        let key = &event.keystroke;
        let command = key.modifiers.platform || key.modifiers.control;
        let shift = key.modifiers.shift;
        // The containing composer/address field owns submission and Escape.
        if key.key == "escape"
            || (key.key == "enter"
                && (command || !self.multiline || (self.submit_on_enter && !shift)))
        {
            return;
        }
        let before = self.buffer.text().to_owned();
        match (key.key.as_str(), command) {
            ("a", true) => self.buffer.select_all(),
            ("c" | "x", true) if self.secure => {}
            ("c" | "x", true) => {
                let selection = self.buffer.selection();
                if !selection.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(
                        self.buffer.text()[selection].to_owned(),
                    ));
                    if key.key == "x" {
                        self.buffer.insert("");
                    }
                }
            }
            ("v", true) => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    let text = self.insertion(self.buffer.selection(), &text);
                    self.buffer.insert(&text);
                }
            }
            ("z", true) if shift => self.buffer.redo(),
            ("z", true) => self.buffer.undo(),
            ("y", true) => self.buffer.redo(),
            ("left" | "right", _) => {
                let right = key.key == "right";
                if key.modifiers.platform {
                    self.buffer.line_edge(right, false, shift);
                } else {
                    self.buffer.horizontal(
                        right,
                        shift,
                        key.modifiers.alt || key.modifiers.control,
                    );
                }
            }
            ("up" | "down", _) => {
                let down = key.key == "down";
                if command {
                    self.buffer.line_edge(down, true, shift);
                } else if let Some(pos) = self.position_at(self.buffer.caret()) {
                    let target = self.index_at(point(
                        pos.x,
                        pos.y
                            + if down {
                                self.line_height * 1.5
                            } else {
                                -self.line_height * 0.5
                            },
                    ));
                    self.buffer.move_to(target, shift);
                } else {
                    self.buffer.vertical(down, shift);
                }
            }
            ("home" | "end", _) => self.buffer.line_edge(key.key == "end", command, shift),
            ("backspace" | "delete", _) => {
                let forward = key.key == "delete";
                if self.buffer.selection().is_empty() {
                    if key.modifiers.platform {
                        self.buffer.line_edge(forward, false, true);
                    } else if key.modifiers.alt || key.modifiers.control {
                        self.buffer.horizontal(forward, true, true);
                    }
                }
                self.buffer.erase(forward);
            }
            ("enter", false) if self.buffer.marked.is_none() => self.buffer.insert("\n"),
            _ => return, // Printable text is inserted exactly once by the OS input handler.
        }
        if before != self.buffer.text() {
            self.changed(cx);
        } else {
            cx.notify();
        }
        let _ = window;
        cx.stop_propagation();
    }
}
impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.buffer.byte_range(range);
        *actual = Some(self.buffer.utf16_range(range.clone()));
        Some(if self.secure {
            "•".repeat(self.buffer.text()[range].encode_utf16().count())
        } else {
            self.buffer.text()[range].to_owned()
        })
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.buffer.utf16_range(self.buffer.selection()),
            reversed: self.buffer.reversed(),
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.buffer
            .marked
            .clone()
            .map(|r| self.buffer.utf16_range(r))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(marked) = self.buffer.marked.clone() {
            let text = self.insertion(marked.clone(), &self.buffer.text()[marked.clone()]);
            if text != self.buffer.text()[marked.clone()] {
                self.buffer.replace(marked, &text);
            }
        }
        self.buffer.unmark();
        self.changed(cx);
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| self.buffer.byte_range(r))
            .or(self.buffer.marked.clone())
            .unwrap_or(self.buffer.selection());
        let text = self.insertion(range.clone(), text);
        self.buffer.replace(range, &text);
        self.changed(cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selection: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| self.buffer.byte_range(r))
            .or(self.buffer.marked.clone())
            .unwrap_or(self.buffer.selection());
        self.buffer.mark(range, text, selection);
        self.changed(cx);
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.buffer.byte_range(range);
        let start = self.position_at(range.start)?;
        let end = self.position_at(range.end)?;
        Some(Bounds::new(
            start,
            size(
                if end.y == start.y {
                    (end.x - start.x).max(px(1.))
                } else {
                    px(1.)
                },
                self.line_height,
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.buffer.to_utf16(self.index_at(point)))
    }
}

struct InputElement(Entity<TextInput>);
impl IntoElement for InputElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for InputElement {
    type RequestLayoutState = ();
    type PrepaintState = Vec<PaintedLine>;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let input = self.0.read(cx);
        let text: SharedString = if input.buffer.text().is_empty() {
            input.placeholder.clone()
        } else {
            input.displayed_text()
        };
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let run = style.to_run(text.len());
        let multiline = input.multiline;
        let line_height = input.line_height;
        let layout =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let width = known
                    .width
                    .or(match available.width {
                        AvailableSpace::Definite(width) => Some(width),
                        _ => None,
                    })
                    .unwrap_or(px(280.));
                let lines = window
                    .text_system()
                    .shape_text(
                        text.clone(),
                        font_size,
                        std::slice::from_ref(&run),
                        multiline.then_some(width.max(px(1.))),
                        None,
                    )
                    .unwrap_or_default();
                let height = lines
                    .iter()
                    .map(|line| f32::from(line.size(line_height).height))
                    .sum::<f32>();
                size(
                    width,
                    px(if multiline {
                        height.clamp(28., 120.)
                    } else {
                        f32::from(line_height)
                    }),
                )
            });
        (layout, ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<PaintedLine> {
        let input = self.0.read(cx);
        // Placeholder is a separate visual layer, never editable text or part
        // of the platform selection/IME layout.
        let text = input.displayed_text();
        let style = window.text_style();
        let run = style.to_run(text.len());
        let runs = if let Some(marked) = &input.buffer.marked {
            let marked = input.display_index(marked.start)..input.display_index(marked.end);
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.len(),
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: text.len() - marked.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|r| r.len > 0)
            .collect::<Vec<_>>()
        } else {
            vec![run]
        };
        let shaped = window
            .text_system()
            .shape_text(
                text,
                style.font_size.to_pixels(window.rem_size()),
                &runs,
                input.multiline.then_some(bounds.size.width.max(px(1.))),
                None,
            )
            .unwrap_or_default();
        let mut y = bounds.top() - input.scroll_y;
        let mut start = 0;
        shaped
            .into_iter()
            .map(|layout| {
                let line = PaintedLine {
                    start,
                    origin: point(bounds.left() - input.scroll_x, y),
                    layout: Arc::new(layout),
                };
                start += line.layout.len() + 1;
                y += line.layout.size(input.line_height).height;
                line
            })
            .collect()
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        lines: &mut Vec<PaintedLine>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focused = self.0.read(cx).focus.is_focused(window);
        let caret_blinking = focused && window.is_window_active();
        self.0.update(cx, |input, cx| {
            input.bounds = Some(bounds);
            input.lines = std::mem::take(lines);
            input.sync_caret_blink(caret_blinking, cx);
        });
        let input = self.0.read(cx);
        let theme = Theme::default();
        let selection_fill = input_selection_fill(&theme);
        let focus = input.focus.clone();
        let line_height = input.line_height;
        let multiline = input.multiline;
        let selection = input.buffer.selection();
        let selection = input.display_index(selection.start)..input.display_index(selection.end);
        let caret = input.position_at(input.buffer.caret());
        let painted_lines = input.lines.clone();
        let caret_color = window.text_style().color;
        let caret_visible = input.caret_visible;
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if focused {
                for line in &painted_lines {
                    // Paint selection one visual row at a time, including soft wraps.
                    let boundaries = std::iter::once(0)
                        .chain(
                            line.layout
                                .wrap_boundaries
                                .iter()
                                .map(|b| line.layout.runs()[b.run_ix].glyphs[b.glyph_ix].index),
                        )
                        .chain(std::iter::once(line.layout.len()))
                        .collect::<Vec<_>>();
                    for (row, pair) in boundaries.windows(2).enumerate() {
                        let start = selection.start.max(line.start + pair[0]);
                        let end = selection.end.min(line.start + pair[1]);
                        if start < end {
                            let layout = &line.layout.unwrapped_layout;
                            let left = layout.x_for_index(start - line.start)
                                - layout.x_for_index(pair[0]);
                            let right =
                                layout.x_for_index(end - line.start) - layout.x_for_index(pair[0]);
                            window.paint_quad(fill(
                                Bounds::new(
                                    line.origin + point(left, line_height * row),
                                    size(right - left, line_height),
                                ),
                                selection_fill,
                            ));
                        }
                    }
                }
            }
            for line in &painted_lines {
                let _ =
                    line.layout
                        .paint(line.origin, line_height, TextAlign::Left, None, window, cx);
            }
            if focused
                && window.is_window_active()
                && caret_visible
                && selection.is_empty()
                && let Some(pos) = caret
            {
                window.paint_quad(fill(
                    Bounds::new(pos, size(px(1.), line_height)),
                    caret_color,
                ));
            }
        });
        // Keep the caret inside the bounded scroll surface while pasting or typing.
        if let Some(caret) = caret {
            let offset = if caret.y < bounds.top() {
                caret.y - bounds.top()
            } else if caret.y + line_height > bounds.bottom() {
                caret.y + line_height - bounds.bottom()
            } else {
                px(0.)
            };
            if offset != px(0.) {
                self.0.update(cx, |input, cx| {
                    input.scroll_y = (input.scroll_y + offset).max(px(0.));
                    cx.notify();
                });
            }
            if !multiline && focus.is_focused(window) {
                let horizontal = if caret.x < bounds.left() {
                    caret.x - bounds.left()
                } else if caret.x + px(1.) > bounds.right() {
                    caret.x + px(1.) - bounds.right()
                } else {
                    px(0.)
                };
                if horizontal != px(0.) {
                    self.0.update(cx, |input, cx| {
                        input.scroll_x = (input.scroll_x + horizontal).max(px(0.));
                        cx.notify();
                    });
                }
            }
        }
        window.handle_input(&focus, ElementInputHandler::new(bounds, self.0.clone()), cx);
    }
}
impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self
            .focus_observed_windows
            .insert(window.window_handle().window_id())
        {
            // A retained input can reuse its cached draw commands after another
            // control gains focus. Invalidate the input owner, not just its window.
            let focus = self.focus.clone();
            let mut previously_focused = focus.is_focused(window);
            let pending = cx.observe_pending_input(window, move |_, window, cx| {
                let focused = focus.is_focused(window);
                if focused != previously_focused {
                    previously_focused = focused;
                    cx.notify();
                }
            });
            let activation = cx.observe_window_activation(window, |_, _, cx| cx.notify());
            let id = cx.entity_id();
            let exited = window.on_focus_out(&self.focus, cx, move |_, _, cx| cx.notify(id));
            self.focus_subscriptions
                .extend([pending, activation, exited]);
        }
        let weak = cx.weak_entity();
        let theme = Theme::default();
        div()
            .id("native-text-input")
            .role(if self.secure {
                Role::PasswordInput
            } else if self.multiline {
                Role::MultilineTextInput
            } else {
                Role::TextInput
            })
            .aria_label(self.label.clone())
            .aria_placeholder(self.placeholder.clone())
            .aria_value(self.displayed_text())
            .tab_stop(true)
            .on_a11y_action(AccessibleAction::SetValue, move |data, _, cx| {
                if let Some(gpui::accesskit::ActionData::Value(value)) = data {
                    let _ = weak.update(cx, |input, cx| {
                        input.buffer.select_all();
                        input.buffer.insert(value);
                        input.changed(cx);
                    });
                }
            })
            .relative()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .track_focus(&self.focus)
            .when(self.settings_field, |input| {
                input
                    .px(px(9.))
                    .py(px(7.))
                    .border_1()
                    .border_color(theme.border_strong)
                    .rounded(px(12.))
                    .bg(theme.inset)
                    .text_size(px(11.))
                    .focus(move |style| {
                        style.shadow(vec![
                            BoxShadow::new(px(0.), px(0.), theme.accent.into())
                                .spread_radius(px(2.)),
                            BoxShadow::new(px(0.), px(0.), theme.panel.into())
                                .spread_radius(px(1.)),
                        ])
                    })
            })
            .when(self.browser_field, |input| {
                input
                    .w(px(147.3333))
                    .flex_shrink_0()
                    .px(px(2.))
                    .py(px(1.))
                    .border_1()
                    .border_color(rgb(if theme.dark { 0x858585 } else { 0x767676 }))
                    .bg(rgb(if theme.dark { 0x3b3b3b } else { 0xffffff }))
                    .text_color(rgb(if theme.dark { 0xffffff } else { 0x000000 }))
                    .font_family("Arial")
                    .text_size(px(13.3333))
                    .focus(move |style| {
                        style.shadow(vec![
                            BoxShadow::new(px(0.), px(0.), theme.accent.into())
                                .spread_radius(px(2.)),
                            BoxShadow::new(px(0.), px(0.), theme.panel.into())
                                .spread_radius(px(1.)),
                        ])
                    })
            })
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(
                cx.listener(|this, _: &Cut, window, cx| this.menu_key("x", false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Copy, window, cx| this.menu_key("c", false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Paste, window, cx| this.menu_key("v", false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &SelectAll, window, cx| {
                    this.menu_key("a", false, window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &Undo, window, cx| this.menu_key("z", false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Redo, window, cx| this.menu_key("z", true, window, cx)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if let Some(on_focus) = &this.on_focus {
                        on_focus();
                    }
                    window.focus(&this.focus, cx);
                    this.selecting = true;
                    this.caret_visible = true;
                    let index = this.index_at(event.position);
                    if event.click_count >= 3 {
                        this.buffer.select_all();
                    } else if event.click_count == 2 {
                        this.buffer.move_to(index, false);
                        this.buffer.horizontal(false, false, true);
                        this.buffer.horizontal(true, true, true);
                    } else {
                        this.buffer.move_to(index, event.modifiers.shift);
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.selecting {
                    this.buffer.move_to(this.index_at(event.position), true);
                    this.caret_visible = true;
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.selecting = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.selecting = false),
            )
            .when(self.buffer.text().is_empty(), |input| {
                input.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .when(self.settings_field, |placeholder| {
                            placeholder.top(px(7.)).left(px(9.))
                        })
                        .when(self.browser_field, |placeholder| {
                            placeholder
                                .top(px(1.))
                                .left(px(2.))
                                .text_color(rgb(0x757575))
                        })
                        .w_full()
                        .text_color(if self.browser_field {
                            rgb(if theme.dark { 0xaaaaaa } else { 0x757575 })
                        } else {
                            theme.faint
                        })
                        .child(self.placeholder.clone()),
                )
            })
            .child(InputElement(cx.entity()))
    }
}

#[cfg(test)]
mod tests {
    use super::{CARET_BLINK_INTERVAL, TextInput, input_selection_fill, navigate_focus};
    use crate::theme::Theme;
    use gpui::{ClipboardItem, EntityInputHandler, TestAppContext, point, prelude::*, px, size};

    #[test]
    fn text_selection_uses_profile_accent_at_source_alpha() {
        let codex = Theme::default();
        let fill = input_selection_fill(&codex);
        assert_eq!((fill.r, fill.g, fill.b), (codex.accent.r, codex.accent.g, codex.accent.b));
        assert!((fill.a - 0.32).abs() < f32::EPSILON);

        let kitty = Theme::from_profile(vela_core::appearance::profile("kitty"));
        let kitty_fill = input_selection_fill(&kitty);
        assert_eq!((kitty_fill.r, kitty_fill.g, kitty_fill.b), (kitty.accent.r, kitty.accent.g, kitty.accent.b));
        assert_ne!(
            (kitty_fill.r, kitty_fill.g, kitty_fill.b),
            (fill.r, fill.g, fill.b),
            "the selection follows the active palette instead of a fixed blue"
        );
    }

    #[gpui::test]
    fn caret_blinks_only_while_focused_and_blur_restores_a_steady_caret(
        cx: &mut TestAppContext,
    ) {
        let input = cx.new(|cx| TextInput::new(false, "Blink", cx));
        input.update(cx, |input, cx| input.sync_caret_blink(true, cx));
        assert!(input.read_with(cx, |input, _| input.caret_visible));
        cx.executor().advance_clock(CARET_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(!input.read_with(cx, |input, _| input.caret_visible));
        cx.executor().advance_clock(CARET_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(input.read_with(cx, |input, _| input.caret_visible));

        input.update(cx, |input, cx| input.sync_caret_blink(false, cx));
        assert!(input.read_with(cx, |input, _| input.caret_visible));
        cx.executor().advance_clock(CARET_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(
            input.read_with(cx, |input, _| input.caret_visible),
            "a blurred caret is steady and never repainted off"
        );

        // Refocusing starts a fresh visible phase.
        input.update(cx, |input, cx| input.sync_caret_blink(true, cx));
        assert!(input.read_with(cx, |input, _| input.caret_visible));
        input.update(cx, |input, cx| input.sync_caret_blink(false, cx));
    }

    #[gpui::test]
    fn input_invalidates_cached_paint_on_focus_loss_and_regain(cx: &mut TestAppContext) {
        struct Fields {
            first: gpui::Entity<TextInput>,
            second: gpui::Entity<TextInput>,
        }
        impl gpui::Render for Fields {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                gpui::div()
                    .w(px(300.))
                    .flex()
                    .flex_col()
                    .child(self.first.clone())
                    .child(self.second.clone())
            }
        }
        let (fields, cx) = cx.add_window_view(|window, cx| {
            let first = cx.new(|cx| TextInput::new(false, "Focus repaint", cx));
            let second = cx.new(|cx| TextInput::new(false, "Other field", cx));
            window.focus(&first.read(cx).focus.clone(), cx);
            Fields { first, second }
        });
        let (input, other) = fields.read_with(cx, |fields, _| {
            (fields.first.clone(), fields.second.clone())
        });
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear());
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(window.is_window_active());
            assert!(input.read(cx).focus.is_focused(window));
            assert_eq!(input.read(cx).focus_observed_windows.len(), 1);
        });
        let notified = std::rc::Rc::new(std::cell::Cell::new(0));
        let counter = notified.clone();
        cx.update(|_, cx| cx.observe(&input, move |_, _| counter.set(counter.get() + 1)))
            .detach();
        input.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        assert!(notified.get() > 0, "observer is active");
        notified.set(0);
        cx.update(|window, cx| window.focus(&other.read(cx).focus.clone(), cx));
        cx.update(|window, cx| window.draw(cx).clear());
        cx.run_until_parked();
        assert!(
            notified.get() > 0,
            "blur must invalidate the cached caret/selection drawing"
        );
        notified.set(0);
        cx.update(|window, cx| window.focus(&input.read(cx).focus.clone(), cx));
        cx.update(|window, cx| window.draw(cx).clear());
        cx.run_until_parked();
        assert!(
            notified.get() > 0,
            "focus must invalidate the cached caret/selection drawing"
        );
        notified.set(0);
        cx.deactivate_window();
        cx.run_until_parked();
        assert!(
            notified.get() > 0,
            "window deactivation must invalidate the cached caret drawing"
        );
    }

    #[gpui::test]
    fn length_limits_apply_to_utf16_replacements_and_committed_composition(
        cx: &mut TestAppContext,
    ) {
        let (input, cx) = cx.add_window_view(|window, cx| {
            let mut input = TextInput::new(false, "Limited", cx);
            input.max_utf16_length = Some(4);
            window.focus(&input.focus, cx);
            input
        });
        input.update_in(cx, |input, window, cx| {
            input.replace_text_in_range(None, "a🌍bc\r\n", window, cx);
            assert_eq!(input.buffer.text(), "a🌍b");
            input.replace_text_in_range(Some(1..3), "界界界", window, cx);
            assert_eq!(input.buffer.text(), "a界界b");
            input.buffer.undo();
            assert_eq!(input.buffer.text(), "a🌍b");
            input.set_text("ab".into(), cx);
            assert!(!input.user_edited);
            input.buffer.move_to(2, false);
            input.replace_and_mark_text_in_range(None, "🌍界", Some(3..3), window, cx);
            input.unmark_text(window, cx);
            assert_eq!(input.buffer.text(), "ab🌍");
            input.buffer.undo();
            assert_eq!(
                input.buffer.text(),
                "ab",
                "one undo restores the entire pre-composition value"
            );
            input.max_utf16_length = None;
            input.replace_text_in_range(Some(0..2), "aa\r\nbb", window, cx);
            assert_eq!(input.buffer.text(), "aabb");
        });
    }

    #[gpui::test]
    fn password_input_masks_unicode_and_blocks_copy_cut_and_platform_context_leaks(
        cx: &mut TestAppContext,
    ) {
        let source = "a🌍e\u{301}界";
        let (input, cx) = cx.add_window_view(|window, cx| {
            let mut input = TextInput::password("API key", cx);
            input.buffer.insert(source);
            window.focus(&input.focus.clone(), cx);
            input
        });
        cx.simulate_resize(size(px(300.), px(30.)));
        cx.draw(point(px(0.), px(0.)), size(px(300.), px(30.)), |_, _| {
            input.clone().into_any_element()
        });
        cx.update(|_, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("clipboard sentinel".into()))
        });
        for key in ["cmd-a", "cmd-c", "cmd-x"] {
            cx.simulate_keystrokes(key);
        }
        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                assert_eq!(input.buffer.text(), source);
                assert_eq!(input.displayed_text().as_ref(), "••••••");
                for source_index in source
                    .char_indices()
                    .map(|(i, _)| i)
                    .chain(std::iter::once(source.len()))
                {
                    assert_eq!(
                        input.source_index(input.display_index(source_index)),
                        source_index
                    );
                }
                let mut actual = None;
                assert_eq!(
                    input
                        .text_for_range(0..6, &mut actual, window, cx)
                        .as_deref(),
                    Some("••••••")
                );
                assert_eq!(actual, Some(0..6));
                assert_eq!(
                    cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
                    Some("clipboard sentinel")
                );
            });
        });
    }
    #[gpui::test]
    fn long_single_line_unicode_input_keeps_caret_inside_its_content_viewport(
        cx: &mut TestAppContext,
    ) {
        let source = "long-key-🌍-界-".repeat(50);
        let (input, cx) = cx.add_window_view(|window, cx| {
            let mut input = TextInput::password("API key", cx);
            input.use_settings_field();
            input.buffer.insert(&source);
            window.focus(&input.focus.clone(), cx);
            input
        });
        cx.simulate_resize(size(px(220.), px(40.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(220.), px(40.)), |_, _| {
                input.clone().into_any_element()
            });
        };
        draw(cx);
        draw(cx);
        input.read_with(cx, |input, _| {
            let bounds = input.bounds.unwrap();
            let caret = input.position_at(input.buffer.caret()).unwrap();
            assert!(
                caret.x >= bounds.left() && caret.x + px(1.) <= bounds.right(),
                "long input caret must be inside its content viewport"
            );
            assert!(input.scroll_x > px(0.));
            assert_eq!(input.buffer.text(), source);
        });
        cx.simulate_keystrokes("cmd-left");
        draw(cx);
        draw(cx);
        input.read_with(cx, |input, _| {
            assert_eq!(input.scroll_x, px(0.));
            assert_eq!(input.buffer.caret(), 0);
        });
        cx.simulate_keystrokes("cmd-right");
        draw(cx);
        draw(cx);
        input.read_with(cx, |input, _| {
            let bounds = input.bounds.unwrap();
            let caret = input.position_at(input.buffer.caret()).unwrap();
            assert!(caret.x + px(1.) <= bounds.right());
            assert_eq!(input.buffer.text(), source);
        });
    }
    #[gpui::test]
    fn native_fields_traverse_tabs_and_preserve_submit_newline_and_composition_intent(
        cx: &mut TestAppContext,
    ) {
        use std::{cell::Cell, rc::Rc};
        struct Fields {
            first: gpui::Entity<TextInput>,
            second: gpui::Entity<TextInput>,
            submitted: Rc<Cell<usize>>,
        }
        impl gpui::Render for Fields {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let first = self.first.clone();
                let submitted = self.submitted.clone();
                gpui::div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .on_key_down(move |event: &gpui::KeyDownEvent, window, cx| {
                        if event.keystroke.key == "enter" && !event.keystroke.modifiers.shift {
                            if first.read(cx).buffer.marked.is_some() {
                                return;
                            }
                            submitted.set(submitted.get() + 1);
                            cx.stop_propagation();
                        } else {
                            navigate_focus(event, window, cx);
                        }
                    })
                    .child(self.first.clone())
                    .child(self.second.clone())
            }
        }
        let submitted = Rc::new(Cell::new(0));
        let count = submitted.clone();
        let (fields, cx) = cx.add_window_view(|window, cx| {
            let first = cx.new(|cx| {
                let mut input = TextInput::new(true, "Composer", cx);
                input.submit_on_enter = true;
                input
            });
            let second = cx.new(|cx| TextInput::new(false, "Label", cx));
            window.focus(&first.read(cx).focus.clone(), cx);
            Fields {
                first,
                second,
                submitted: count,
            }
        });
        cx.simulate_resize(size(px(300.), px(100.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(100.)), |_, _| {
                fields.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("enter");
        assert_eq!(submitted.get(), 1);
        fields.read_with(cx, |fields, cx| {
            assert_eq!(fields.first.read(cx).buffer.text(), "")
        });
        cx.simulate_keystrokes("shift-enter");
        draw(cx);
        fields.read_with(cx, |fields, cx| {
            assert_eq!(fields.first.read(cx).buffer.text(), "\n")
        });
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            assert!(fields.read(cx).second.read(cx).focus.is_focused(window));
        });
        cx.simulate_keystrokes("shift-tab");
        cx.update(|window, cx| {
            let first = fields.read(cx).first.clone();
            assert!(first.read(cx).focus.is_focused(window));
            first.update(cx, |input, cx| {
                input.replace_and_mark_text_in_range(Some(0..1), "é", Some(1..1), window, cx)
            });
        });
        // Dispatch the physical key separately from the IME commit. The GPUI
        // keystroke convenience method simulates a literal newline insertion.
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        assert_eq!(
            submitted.get(),
            1,
            "composition Enter must not submit the form"
        );
        cx.update(|window, cx| {
            let first = fields.read(cx).first.clone();
            first.update(cx, |input, cx| input.unmark_text(window, cx));
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(submitted.get(), 2);
        fields.read_with(cx, |fields, cx| {
            assert_eq!(fields.first.read(cx).buffer.text(), "é")
        });
    }
}
