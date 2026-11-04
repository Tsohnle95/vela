//! A pure Alacritty-backed terminal state machine.
//!
//! PTY bytes go in; a renderable cell grid comes out. No I/O, timers, or GPUI,
//! so escape-sequence behavior is unit-testable. The view owns one emulator per
//! terminal panel and feeds it coalesced output batches.

use std::cell::RefCell;
use std::rc::Rc;

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{
    Color as AnsiColor, CursorShape as AnsiCursorShape, NamedColor, Processor, Rgb as AnsiRgb,
};

pub const SCROLLBACK_LINES: usize = 5_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
}

impl GridSize {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.max(2),
            rows: rows.max(1),
        }
    }
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.rows as usize
    }

    fn screen_lines(&self) -> usize {
        self.rows as usize
    }

    fn columns(&self) -> usize {
        self.cols as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseReportMode {
    Off,
    Click,
    Drag,
    Motion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
    Press,
    Release,
    Drag,
    Move,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseModifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UnderlineKind {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellColor {
    Foreground,
    Background,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub ch: char,
    /// Zero-width code points attached to this grid cell, in output order.
    pub combining: Vec<char>,
    pub selected: bool,
    pub fg: CellColor,
    pub bg: CellColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: UnderlineKind,
    pub underline_color: Option<CellColor>,
    pub hyperlink: Option<String>,
    pub strikethrough: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub wide: bool,
    pub wide_spacer: bool,
}

impl Cell {
    pub fn display_colors(&self) -> (CellColor, CellColor) {
        let (fg, bg) = if self.inverse {
            (self.bg, self.fg)
        } else {
            (self.fg, self.bg)
        };
        if self.hidden { (bg, bg) } else { (fg, bg) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Underline,
    Beam,
    HollowBlock,
    Hidden,
}

#[derive(Default, Clone)]
struct Capture {
    events: Rc<RefCell<Vec<Event>>>,
}

impl EventListener for Capture {
    fn send_event(&self, event: Event) {
        self.events.borrow_mut().push(event);
    }
}

fn map_color(color: AnsiColor) -> CellColor {
    match color {
        AnsiColor::Spec(AnsiRgb { r, g, b }) => CellColor::Rgb(r, g, b),
        AnsiColor::Indexed(ix) => CellColor::Indexed(ix),
        AnsiColor::Named(named) => {
            let index = named as usize;
            if index < 16 {
                return CellColor::Indexed(index as u8);
            }
            match named {
                NamedColor::Background => CellColor::Background,
                NamedColor::DimBlack
                | NamedColor::DimRed
                | NamedColor::DimGreen
                | NamedColor::DimYellow
                | NamedColor::DimBlue
                | NamedColor::DimMagenta
                | NamedColor::DimCyan
                | NamedColor::DimWhite => {
                    CellColor::Indexed((index - NamedColor::DimBlack as usize) as u8)
                }
                _ => CellColor::Foreground,
            }
        }
    }
}

pub struct Emulator {
    term: Term<Capture>,
    parser: Processor,
    capture: Capture,
    title: Option<String>,
}

#[derive(Clone, Copy)]
pub enum SelectionKind {
    Simple,
    Word,
    Line,
    Block,
}

impl Emulator {
    pub fn new(cols: u16, rows: u16) -> Self {
        let capture = Capture::default();
        let config = Config {
            scrolling_history: SCROLLBACK_LINES,
            ..Config::default()
        };
        let term = Term::new(config, &GridSize::new(cols, rows), capture.clone());
        Self {
            term,
            parser: Processor::new(),
            capture,
            title: None,
        }
    }

    /// Feed decoded PTY output. Returns bytes the program expects written back
    /// (DSR/DA query responses).
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.parser.advance(&mut self.term, bytes);
        let mut responses = Vec::new();
        for event in self.capture.events.borrow_mut().drain(..) {
            match event {
                Event::PtyWrite(text) => responses.extend_from_slice(text.as_bytes()),
                Event::Title(title) => self.title = Some(title),
                Event::ResetTitle => self.title = None,
                _ => {}
            }
        }
        responses
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.term.resize(GridSize::new(cols, rows));
    }

    pub fn cols(&self) -> usize {
        self.term.columns()
    }

    pub fn rows(&self) -> usize {
        self.term.screen_lines()
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn cursor(&self) -> Cursor {
        let point = self.term.grid().cursor.point;
        let mut col = point.column.0;
        // A wide glyph's second cell is a spacer; draw its cursor on the glyph.
        if col > 0 && self.term.grid()[point].flags.contains(Flags::WIDE_CHAR_SPACER) {
            col -= 1;
        }
        Cursor {
            row: (point.line.0 + self.display_offset() as i32).max(0) as usize,
            col,
        }
    }

    pub fn cursor_shape(&self) -> CursorShape {
        if !self.cursor_visible() {
            return CursorShape::Hidden;
        }
        match self.term.cursor_style().shape {
            AnsiCursorShape::Block => CursorShape::Block,
            AnsiCursorShape::Underline => CursorShape::Underline,
            AnsiCursorShape::Beam => CursorShape::Beam,
            AnsiCursorShape::HollowBlock => CursorShape::HollowBlock,
            AnsiCursorShape::Hidden => CursorShape::Hidden,
        }
    }

    pub fn cursor_blinking(&self) -> bool {
        self.cursor_visible() && self.term.cursor_style().blinking
    }

    pub fn cursor_cell_width(&self) -> usize {
        let mut point = self.term.grid().cursor.point;
        if point.column.0 > 0
            && self.term.grid()[point]
                .flags
                .contains(Flags::WIDE_CHAR_SPACER)
        {
            point.column = Column(point.column.0 - 1);
        }
        if self.term.grid()[point].flags.contains(Flags::WIDE_CHAR) {
            2
        } else {
            1
        }
    }

    pub fn app_cursor_mode(&self) -> bool {
        self.term.mode().contains(TermMode::APP_CURSOR)
    }

    pub fn cursor_visible(&self) -> bool {
        self.term.mode().contains(TermMode::SHOW_CURSOR)
    }

    pub fn bracketed_paste_mode(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    pub fn mouse_report_mode(&self) -> MouseReportMode {
        let mode = self.term.mode();
        if mode.contains(TermMode::MOUSE_MOTION) {
            MouseReportMode::Motion
        } else if mode.contains(TermMode::MOUSE_DRAG) {
            MouseReportMode::Drag
        } else if mode.contains(TermMode::MOUSE_REPORT_CLICK) {
            MouseReportMode::Click
        } else {
            MouseReportMode::Off
        }
    }

    pub fn sgr_mouse(&self) -> bool {
        self.term.mode().contains(TermMode::SGR_MOUSE)
    }

    pub fn focus_report_mode(&self) -> bool {
        self.term.mode().contains(TermMode::FOCUS_IN_OUT)
    }

    /// DECSET 1004 focus notifications.
    pub fn focus_report(&self, focused: bool) -> Option<Vec<u8>> {
        self.focus_report_mode().then(|| {
            if focused {
                b"\x1b[I".to_vec()
            } else {
                b"\x1b[O".to_vec()
            }
        })
    }

    /// Encode a mouse event using the terminal's active DEC mouse mode.
    /// Returns `None` when the application has not enabled reporting or the
    /// event is not meaningful for that mode. Coordinates are zero-based.
    pub fn encode_mouse(
        &self,
        action: MouseAction,
        button: MouseButton,
        row: usize,
        col: usize,
        modifiers: MouseModifiers,
    ) -> Option<Vec<u8>> {
        let mode = self.mouse_report_mode();
        if mode == MouseReportMode::Off {
            return None;
        }
        match action {
            MouseAction::Drag if !matches!(mode, MouseReportMode::Drag | MouseReportMode::Motion) => {
                return None;
            }
            MouseAction::Move if mode != MouseReportMode::Motion => return None,
            MouseAction::Release if button == MouseButton::WheelUp || button == MouseButton::WheelDown => {
                return None;
            }
            _ => {}
        }
        let mut code = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            MouseButton::WheelUp => 64,
            MouseButton::WheelDown => 65,
            MouseButton::None => 3,
        };
        if matches!(action, MouseAction::Drag | MouseAction::Move) {
            code += 32;
        }
        let modifier_bits = i32::from(modifiers.shift) * 4
            + i32::from(modifiers.alt) * 8
            + i32::from(modifiers.control) * 16;
        code += modifier_bits;
        let column = col.saturating_add(1);
        let row = row.saturating_add(1);
        if self.sgr_mouse() {
            let final_byte = if action == MouseAction::Release {
                'm'
            } else {
                'M'
            };
            return Some(format!("\x1b[<{code};{column};{row}{final_byte}").into_bytes());
        }
        // X10 reports a release as button 3 and clamps coordinates to 223.
        let code = if action == MouseAction::Release {
            3 + modifier_bits
        } else {
            code
        };
        Some(vec![
            0x1b,
            b'[',
            b'M',
            (32 + code).clamp(0, 255) as u8,
            column.min(223) as u8 + 32,
            row.min(223) as u8 + 32,
        ])
    }

    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    pub fn history_lines(&self) -> usize {
        self.term.grid().history_size()
    }

    pub fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    fn selection_point(&self, row: usize, col: usize) -> Point {
        Point::new(
            Line(row.min(self.rows() - 1) as i32 - self.display_offset() as i32),
            Column(col.min(self.cols() - 1)),
        )
    }

    pub fn begin_selection(
        &mut self,
        row: usize,
        col: usize,
        right: bool,
        kind: SelectionKind,
        extend: bool,
    ) {
        let point = self.selection_point(row, col);
        let side = if right { Side::Right } else { Side::Left };
        if extend && self.term.selection.is_some() {
            self.update_selection(row, col, right);
            return;
        }
        let kind = match kind {
            SelectionKind::Simple => SelectionType::Simple,
            SelectionKind::Word => SelectionType::Semantic,
            SelectionKind::Line => SelectionType::Lines,
            SelectionKind::Block => SelectionType::Block,
        };
        self.term.selection = Some(Selection::new(kind, point, side));
    }

    pub fn update_selection(&mut self, row: usize, col: usize, right: bool) {
        let point = self.selection_point(row, col);
        if let Some(selection) = &mut self.term.selection {
            selection.update(point, if right { Side::Right } else { Side::Left });
        }
    }

    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    pub fn selection_text(&self) -> Option<String> {
        self.term
            .selection_to_string()
            .filter(|text| !text.is_empty())
    }

    pub fn block_selection(&self) -> bool {
        self.term
            .selection
            .as_ref()
            .is_some_and(|selection| selection.ty == SelectionType::Block)
    }

    /// One viewport row, honoring the current scrollback offset.
    pub fn line(&self, row: usize) -> Vec<Cell> {
        let line = Line(row as i32 - self.display_offset() as i32);
        let grid = self.term.grid();
        let row = &grid[line];
        let selection = self
            .term
            .selection
            .as_ref()
            .and_then(|s| s.to_range(&self.term));
        (0..self.cols())
            .map(|col| {
                let cell = &row[Column(col)];
                Cell {
                    ch: cell.c,
                    combining: cell.zerowidth().unwrap_or_default().to_vec(),
                    selected: selection.is_some_and(|selection| {
                        selection.contains(Point::new(line, Column(col)))
                            || (cell.flags.contains(Flags::WIDE_CHAR)
                                && selection.contains(Point::new(line, Column(col + 1))))
                    }),
                    fg: map_color(cell.fg),
                    bg: map_color(cell.bg),
                    bold: cell.flags.intersects(Flags::BOLD),
                    dim: cell.flags.intersects(Flags::DIM),
                    italic: cell.flags.intersects(Flags::ITALIC),
                    underline: if cell.flags.intersects(Flags::DOUBLE_UNDERLINE) {
                        UnderlineKind::Double
                    } else if cell.flags.intersects(Flags::UNDERCURL) {
                        UnderlineKind::Curly
                    } else if cell.flags.intersects(Flags::DOTTED_UNDERLINE) {
                        UnderlineKind::Dotted
                    } else if cell.flags.intersects(Flags::DASHED_UNDERLINE) {
                        UnderlineKind::Dashed
                    } else if cell.flags.intersects(Flags::UNDERLINE) {
                        UnderlineKind::Single
                    } else {
                        UnderlineKind::None
                    },
                    underline_color: cell.underline_color().map(map_color),
                    hyperlink: cell.hyperlink().map(|link| link.uri().to_owned()),
                    strikethrough: cell.flags.intersects(Flags::STRIKEOUT),
                    inverse: cell.flags.intersects(Flags::INVERSE),
                    hidden: cell.flags.intersects(Flags::HIDDEN),
                    wide: cell.flags.intersects(Flags::WIDE_CHAR),
                    wide_spacer: cell.flags.intersects(Flags::WIDE_CHAR_SPACER),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_combining_marks_across_output_chunks_and_resize() {
        let mut emulator = Emulator::new(20, 4);
        for byte in "e\u{301}\u{327}界\u{301}X".as_bytes() {
            emulator.feed(&[*byte]);
        }
        emulator.resize(30, 4);
        let cells = emulator.line(0);
        assert_eq!(cells[0].ch, 'e');
        assert_eq!(cells[0].combining, ['\u{301}', '\u{327}']);
        assert_eq!(cells[1].ch, '界');
        assert_eq!(cells[1].combining, ['\u{301}']);
        assert!(cells[2].wide_spacer);
        assert_eq!(cells[3].ch, 'X');
        assert_eq!(emulator.cursor().col, 4);
    }

    #[test]
    fn applications_can_hide_and_restore_the_cursor() {
        let mut emulator = Emulator::new(20, 4);
        assert!(emulator.cursor_visible());
        emulator.feed(b"\x1b[?25l");
        assert!(!emulator.cursor_visible());
        emulator.resize(30, 5);
        assert!(!emulator.cursor_visible());
        emulator.feed(b"\x1b[?25h");
        assert!(emulator.cursor_visible());
    }

    #[test]
    fn cursor_shape_tracks_decscusr_and_wide_spacers() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"\x1b[4 q");
        assert_eq!(emulator.cursor_shape(), CursorShape::Underline);
        assert!(!emulator.cursor_blinking());
        emulator.feed(b"\x1b[5 q");
        assert_eq!(emulator.cursor_shape(), CursorShape::Beam);
        assert!(emulator.cursor_blinking());
        emulator.feed(b"\x1b[2 q");
        assert_eq!(emulator.cursor_shape(), CursorShape::Block);
        assert!(!emulator.cursor_blinking());
        emulator.feed(b"\x1b[6 q");
        assert_eq!(emulator.cursor_shape(), CursorShape::Beam);
        assert!(!emulator.cursor_blinking());
        emulator.feed(b"\x1b[3 q");
        assert_eq!(emulator.cursor_shape(), CursorShape::Underline);
        assert!(emulator.cursor_blinking());
        emulator.feed(b"\x1b[?25l");
        assert_eq!(emulator.cursor_shape(), CursorShape::Hidden);
        emulator.feed(b"\x1b[?25h");
        emulator.feed(b"\x1b[1 q");

        emulator.feed("界".as_bytes());
        emulator.feed(b"\x1b[1;2H");
        assert_eq!(
            emulator.cursor(),
            Cursor { row: 0, col: 0 },
            "the cursor on a wide glyph's spacer belongs to the glyph"
        );
        assert_eq!(emulator.cursor_cell_width(), 2);
        emulator.feed(b"\x1b[2;1H");
        assert_eq!(emulator.cursor_cell_width(), 1);
    }

    #[test]
    fn selection_copies_combining_wide_and_wrapped_text_without_spacers() {
        let mut emulator = Emulator::new(6, 3);
        emulator.feed("e\u{301}界abcDEF".as_bytes());
        emulator.begin_selection(0, 0, false, SelectionKind::Simple, false);
        assert!(emulator.selection_text().is_none());
        emulator.update_selection(1, 2, true);
        assert_eq!(
            emulator.selection_text().as_deref(),
            Some("e\u{301}界abcDEF")
        );
        assert!(emulator.line(0).iter().all(|cell| cell.selected));
        emulator.clear_selection();
        assert!(emulator.line(0).iter().all(|cell| !cell.selected));
        emulator.begin_selection(0, 2, false, SelectionKind::Simple, false);
        emulator.update_selection(0, 2, true);
        assert_eq!(emulator.selection_text().as_deref(), Some("界"));
        assert!(emulator.line(0)[1].selected);
    }

    #[test]
    fn selection_supports_words_lines_reverse_block_and_scrollback() {
        let mut emulator = Emulator::new(20, 3);
        emulator.feed(b"first word\r\nsecond word\r\nthird word\r\nfourth word");
        emulator.scroll(1);
        emulator.begin_selection(0, 7, false, SelectionKind::Word, false);
        assert_eq!(emulator.selection_text().as_deref(), Some("word"));
        emulator.begin_selection(1, 3, false, SelectionKind::Line, false);
        assert_eq!(emulator.selection_text().as_deref(), Some("second word\n"));
        emulator.begin_selection(1, 3, true, SelectionKind::Block, false);
        emulator.update_selection(0, 1, false);
        assert_eq!(emulator.selection_text().as_deref(), Some("irs\neco"));
        emulator.begin_selection(0, 0, false, SelectionKind::Simple, false);
        emulator.begin_selection(0, 4, true, SelectionKind::Simple, true);
        assert_eq!(emulator.selection_text().as_deref(), Some("first"));
    }

    #[test]
    fn writes_text_and_advances_cursor() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"hello");
        let line: String = emulator.line(0).iter().map(|cell| cell.ch).collect();
        assert!(line.starts_with("hello"));
        assert_eq!(emulator.cursor().col, 5);
    }

    #[test]
    fn handles_carriage_return_and_newline() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"one\r\ntwo");
        let first: String = emulator.line(0).iter().map(|cell| cell.ch).collect();
        let second: String = emulator.line(1).iter().map(|cell| cell.ch).collect();
        assert!(first.starts_with("one"));
        assert!(second.starts_with("two"));
    }

    #[test]
    fn applies_sgr_foreground_color() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"\x1b[31mred\x1b[0m");
        let cells = emulator.line(0);
        assert_eq!(cells[0].ch, 'r');
        assert_eq!(cells[0].fg, CellColor::Indexed(1));
        assert_eq!(cells[3].fg, CellColor::Foreground);
    }

    #[test]
    fn carries_dim_and_strikethrough_flags_until_reset() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"\x1b[2;9mghost\x1b[0m plain");
        let cells = emulator.line(0);
        assert!(cells[0].dim);
        assert!(cells[0].strikethrough);
        assert!(!cells[5].dim);
        assert!(!cells[5].strikethrough);
    }

    #[test]
    fn carries_all_underline_variants() {
        for (sequence, expected) in [
            ("\x1b[4m", UnderlineKind::Single),
            ("\x1b[4:2m", UnderlineKind::Double),
            ("\x1b[4:3m", UnderlineKind::Curly),
            ("\x1b[4:4m", UnderlineKind::Dotted),
            ("\x1b[4:5m", UnderlineKind::Dashed),
        ] {
            let mut emulator = Emulator::new(20, 4);
            emulator.feed(format!("{sequence}x\x1b[0m").as_bytes());
            assert_eq!(emulator.line(0)[0].underline, expected, "{sequence:?}");
            assert_eq!(
                emulator.line(0)[1].underline,
                UnderlineKind::None,
                "reset clears {sequence:?}"
            );
        }
    }

    #[test]
    fn carries_and_resets_sgr_underline_color() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"\x1b[4;58;2;10;20;30mX\x1b[59mY\x1b[0m");
        let cells = emulator.line(0);
        assert_eq!(cells[0].underline, UnderlineKind::Single);
        assert_eq!(cells[0].underline_color, Some(CellColor::Rgb(10, 20, 30)));
        assert_eq!(cells[1].underline, UnderlineKind::Single);
        assert_eq!(cells[1].underline_color, None);
    }

    #[test]
    fn carries_osc8_hyperlinks_until_reset() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"\x1b]8;;https://example.com\x07linked\x1b]8;;\x07 plain");
        let cells = emulator.line(0);
        assert_eq!(
            cells[0].hyperlink.as_deref(),
            Some("https://example.com"),
            "OSC 8 links attach to every covered cell"
        );
        assert_eq!(cells[5].hyperlink.as_deref(), Some("https://example.com"));
        assert_eq!(cells[6].hyperlink, None);
    }

    #[test]
    fn supports_alternate_screen_and_clear() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"primary");
        emulator.feed(b"\x1b[?1049h");
        emulator.feed(b"alt");
        let line: String = emulator.line(0).iter().map(|cell| cell.ch).collect();
        assert!(line.contains("alt"));
        emulator.feed(b"\x1b[?1049l");
        let restored: String = emulator.line(0).iter().map(|cell| cell.ch).collect();
        assert!(restored.starts_with("primary"));
    }

    #[test]
    fn mouse_reporting_tracks_declared_modes_and_encodes_sgr_and_x10() {
        let mut emulator = Emulator::new(20, 4);
        assert_eq!(emulator.mouse_report_mode(), MouseReportMode::Off);
        assert!(
            emulator
                .encode_mouse(
                    MouseAction::Press,
                    MouseButton::Left,
                    0,
                    0,
                    MouseModifiers::default()
                )
                .is_none()
        );

        emulator.feed(b"\x1b[?1000h\x1b[?1006h");
        assert_eq!(emulator.mouse_report_mode(), MouseReportMode::Click);
        assert!(emulator.sgr_mouse());
        assert_eq!(
            emulator
                .encode_mouse(
                    MouseAction::Press,
                    MouseButton::Left,
                    1,
                    2,
                    MouseModifiers::default()
                )
                .unwrap(),
            b"\x1b[<0;3;2M"
        );
        assert_eq!(
            emulator
                .encode_mouse(
                    MouseAction::Release,
                    MouseButton::Left,
                    1,
                    2,
                    MouseModifiers::default()
                )
                .unwrap(),
            b"\x1b[<0;3;2m"
        );
        assert!(
            emulator
                .encode_mouse(
                    MouseAction::Drag,
                    MouseButton::Left,
                    1,
                    2,
                    MouseModifiers::default()
                )
                .is_none(),
            "click-only mode has no drag reporting"
        );
        assert_eq!(
            emulator
                .encode_mouse(
                    MouseAction::Press,
                    MouseButton::WheelUp,
                    0,
                    0,
                    MouseModifiers {
                        shift: true,
                        control: true,
                        alt: false,
                    }
                )
                .unwrap(),
            b"\x1b[<84;1;1M"
        );

        emulator.feed(b"\x1b[?1002h");
        assert_eq!(emulator.mouse_report_mode(), MouseReportMode::Drag);
        assert_eq!(
            emulator
                .encode_mouse(
                    MouseAction::Drag,
                    MouseButton::Left,
                    0,
                    0,
                    MouseModifiers::default()
                )
                .unwrap(),
            b"\x1b[<32;1;1M"
        );
        assert!(
            emulator
                .encode_mouse(
                    MouseAction::Move,
                    MouseButton::None,
                    0,
                    0,
                    MouseModifiers::default()
                )
                .is_none()
        );

        emulator.feed(b"\x1b[?1003h");
        assert_eq!(emulator.mouse_report_mode(), MouseReportMode::Motion);
        assert_eq!(
            emulator
                .encode_mouse(
                    MouseAction::Move,
                    MouseButton::None,
                    3,
                    4,
                    MouseModifiers::default()
                )
                .unwrap(),
            b"\x1b[<35;5;4M"
        );
        assert!(
            emulator
                .encode_mouse(
                    MouseAction::Release,
                    MouseButton::WheelDown,
                    3,
                    4,
                    MouseModifiers::default()
                )
                .is_none()
        );

        emulator.feed(b"\x1b[?1003l\x1b[?1006l");
        assert_eq!(emulator.mouse_report_mode(), MouseReportMode::Off);
        assert!(!emulator.sgr_mouse());
        emulator.feed(b"\x1b[?1000h");
        assert_eq!(emulator.mouse_report_mode(), MouseReportMode::Click);
        assert_eq!(
            emulator
                .encode_mouse(
                    MouseAction::Press,
                    MouseButton::Left,
                    0,
                    0,
                    MouseModifiers::default()
                )
                .unwrap(),
            vec![0x1b, b'[', b'M', 32, 33, 33]
        );
        assert_eq!(
            emulator
                .encode_mouse(
                    MouseAction::Release,
                    MouseButton::Left,
                    0,
                    0,
                    MouseModifiers::default()
                )
                .unwrap(),
            vec![0x1b, b'[', b'M', 35, 33, 33]
        );
    }

    #[test]
    fn focus_reporting_follows_decset_1004() {
        let mut emulator = Emulator::new(20, 4);
        assert!(!emulator.focus_report_mode());
        assert!(emulator.focus_report(true).is_none());
        emulator.feed(b"\x1b[?1004h");
        assert!(emulator.focus_report_mode());
        assert_eq!(emulator.focus_report(true).unwrap(), b"\x1b[I");
        assert_eq!(emulator.focus_report(false).unwrap(), b"\x1b[O");
        emulator.feed(b"\x1b[?1004l");
        assert!(!emulator.focus_report_mode());
    }

    #[test]
    fn resize_preserves_a_usable_grid() {
        let mut emulator = Emulator::new(20, 4);
        emulator.feed(b"abcdef");
        emulator.resize(40, 8);
        assert_eq!(emulator.cols(), 40);
        assert_eq!(emulator.rows(), 8);
        let line: String = emulator.line(0).iter().map(|cell| cell.ch).collect();
        assert!(line.starts_with("abcdef"));
    }
}
