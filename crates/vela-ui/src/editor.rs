//! Native source editor. Workspace documents own text and history; each view owns
//! focus, selection geometry and a virtualized list of wrapped logical lines.
use crate::{
    input::{Copy, Cut, Paste, Redo, SelectAll, Undo},
    theme::{self, Theme},
};
use gpui::{prelude::*, *};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    rc::Rc,
    time::Duration,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use unicode_segmentation::UnicodeSegmentation;
use vela_core::{
    document::{DocumentKey, Documents},
    text::TextBuffer,
};

const LINE_HEIGHT: f32 = 20.;
static EDITOR_INSTANCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FoldRange {
    /// First logical line of the fold.
    start: usize,
    /// Last logical line hidden by the fold (the starting line stays visible).
    end: usize,
}

fn emmet_language_for(rel: &str) -> &'static str {
    let lower = rel.to_ascii_lowercase();
    match lower.rsplit('.').next().unwrap_or_default() {
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        _ => "plaintext",
    }
}

fn inside_script_tag(text: &str, caret: usize) -> bool {
    let before = &text[..caret.min(text.len())];
    let Some(open) = before.rfind("<script") else {
        return false;
    };
    before.rfind("</script>").is_none_or(|close| close < open)
}

fn line_indent_width(line: &str) -> usize {
    let mut width = 0;
    for ch in line.chars() {
        match ch {
            ' ' => width += 1,
            '\t' => width += 4,
            _ => break,
        }
    }
    width
}

/// Keep the caret on the same code position after indentation changes. The
/// formatter only edits whitespace, so mapping the old line and the column
/// relative to its first non-space character is sufficient.
fn remap_caret(old: &str, new: &str, caret: usize) -> usize {
    let caret = caret.min(old.len());
    let old_start = old[..caret].rfind('\n').map_or(0, |index| index + 1);
    let old_line = &old[old_start..caret];
    let old_lead = old_line.len() - old_line.trim_start().len();
    let old_relative = old_line.len().saturating_sub(old_lead);
    let line = old[..caret].bytes().filter(|byte| *byte == b'\n').count();
    let new_start = if line == 0 {
        0
    } else {
        new.match_indices('\n')
            .nth(line - 1)
            .map_or(new.len(), |(index, _)| index + 1)
    };
    let new_end = new[new_start..]
        .find('\n')
        .map_or(new.len(), |index| new_start + index);
    let new_line = &new[new_start..new_end];
    let new_lead = new_line.len() - new_line.trim_start().len();
    (new_start + new_lead + old_relative).min(new_end)
}

fn indentation_fold_ranges(text: &str) -> Vec<FoldRange> {
    let lines = text.split('\n').collect::<Vec<_>>();
    let indents = lines
        .iter()
        .map(|line| line_indent_width(line))
        .collect::<Vec<_>>();
    let mut ranges = Vec::new();
    for start in 0..lines.len() {
        if lines[start].trim().is_empty() {
            continue;
        }
        let base = indents[start];
        let mut next = start + 1;
        while next < lines.len() && lines[next].trim().is_empty() {
            next += 1;
        }
        if next >= lines.len() || indents[next] <= base {
            continue;
        }
        let mut end = start;
        let mut line = start + 1;
        while line < lines.len() {
            if lines[line].trim().is_empty() {
                line += 1;
                continue;
            }
            if indents[line] > base {
                end = line;
                line += 1;
            } else {
                break;
            }
        }
        if end > start {
            ranges.push(FoldRange { start, end });
        }
    }
    ranges
}

fn brace_fold_ranges(text: &str) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    let mut stack = Vec::<(char, usize)>::new();
    let mut line = 0usize;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut quote = None;
    let mut escaped = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\n' {
            line += 1;
            in_line_comment = false;
        }
        if in_line_comment {
            continue;
        }
        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            continue;
        }
        if ch == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    in_line_comment = true;
                    continue;
                }
                Some('*') => {
                    chars.next();
                    in_block_comment = true;
                    continue;
                }
                _ => {}
            }
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '{' | '[' | '(' => stack.push((ch, line)),
            '}' | ']' | ')' => {
                let Some((open, start)) = stack.pop() else {
                    continue;
                };
                let matches = matches!((open, ch), ('{', '}') | ('[', ']') | ('(', ')'));
                if matches && line > start {
                    ranges.push(FoldRange { start, end: line });
                }
            }
            _ => {}
        }
    }
    ranges
}

const MARKUP_VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

fn tag_end(text: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '>' => return Some(index),
            _ => {}
        }
    }
    None
}

/// HTML/XML element folds. Opening tags push a stack entry; matching closing
/// tags fold through the closing line. Comments, raw script/style content,
/// self-closing tags and void elements are ignored.
fn markup_fold_ranges(text: &str) -> Vec<FoldRange> {
    let mut stack = Vec::<(String, usize)>::new();
    let mut ranges = Vec::new();
    let mut in_comment = false;
    let mut raw: Option<String> = None;
    for (line_number, line) in text.split('\n').enumerate() {
        let mut rest = line;
        loop {
            if in_comment {
                if let Some(end) = rest.find("-->") {
                    rest = &rest[end + 3..];
                    in_comment = false;
                    continue;
                }
                break;
            }
            if let Some(tag) = raw.clone() {
                let close = format!("</{tag}");
                let lower = rest.to_ascii_lowercase();
                let Some(position) = lower.find(&close) else {
                    break;
                };
                rest = &rest[position + close.len()..];
                if let Some(end) = rest.find('>') {
                    rest = &rest[end + 1..];
                    if stack.last().is_some_and(|(open, _)| *open == tag)
                        && let Some((_, start)) = stack.pop()
                        && line_number > start
                    {
                        ranges.push(FoldRange {
                            start,
                            end: line_number,
                        });
                    }
                    raw = None;
                    continue;
                }
                break;
            }
            let Some(open) = rest.find('<') else {
                break;
            };
            rest = &rest[open..];
            if rest.starts_with("<!--") {
                rest = &rest[4..];
                in_comment = true;
                continue;
            }
            if rest.starts_with("<!") || rest.starts_with("<?") {
                if let Some(end) = tag_end(rest) {
                    rest = &rest[end + 1..];
                    continue;
                }
                break;
            }
            let closing = rest.starts_with("</");
            let mut signature = &rest[if closing { 2 } else { 1 }..];
            signature = signature.trim_start();
            let name_len = signature
                .char_indices()
                .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || matches!(ch, '-' | ':' | '_'))
                .map(|(index, ch)| index + ch.len_utf8())
                .last()
                .unwrap_or(0);
            if name_len == 0 {
                rest = &rest[1..];
                continue;
            }
            let name = signature[..name_len].to_ascii_lowercase();
            let Some(end) = tag_end(rest) else {
                break;
            };
            let inner = &rest[..end];
            let self_closing = inner.trim_end().ends_with('/');
            rest = &rest[end + 1..];
            if closing {
                if stack.last().is_some_and(|(open, _)| *open == name)
                    && let Some((_, start)) = stack.pop()
                    && line_number > start
                {
                    ranges.push(FoldRange {
                        start,
                        end: line_number,
                    });
                }
                continue;
            }
            if self_closing || MARKUP_VOID.contains(&name.as_str()) {
                continue;
            }
            stack.push((name.clone(), line_number));
            if matches!(name.as_str(), "script" | "style") {
                raw = Some(name);
            }
        }
    }
    ranges
}

/// Comment-marker folds used by JavaScript/TypeScript, CSS/SCSS, shell and
/// similar languages (`// #region`, `/* #region */`, `#region`). Starts and
/// ends are matched with a stack so nested regions fold independently.
fn region_fold_ranges(text: &str) -> Vec<FoldRange> {
    fn marker(line: &str) -> Option<bool> {
        let trimmed = line.trim();
        let content = trimmed
            .strip_prefix("//")
            .or_else(|| trimmed.strip_prefix("/*"))
            .or_else(|| trimmed.strip_prefix("<!--"))
            .or_else(|| trimmed.strip_prefix('#'))
            .or_else(|| trimmed.strip_prefix('*'))
            .unwrap_or(trimmed)
            .trim();
        let content = content
            .strip_suffix("*/")
            .or_else(|| content.strip_suffix("-->"))
            .unwrap_or(content)
            .trim();
        let lower = content.to_lowercase();
        let lower = lower.strip_prefix('#').unwrap_or(&lower).trim();
        if lower.starts_with("endregion") {
            Some(false)
        } else if lower.starts_with("region") {
            Some(true)
        } else {
            None
        }
    }

    let mut stack = Vec::new();
    let mut ranges = Vec::new();
    for (line, content) in text.split('\n').enumerate() {
        match marker(content) {
            Some(true) => stack.push(line),
            Some(false) => {
                if let Some(start) = stack.pop()
                    && line > start
                {
                    ranges.push(FoldRange { start, end: line });
                }
            }
            None => {}
        }
    }
    ranges
}

/// Multi-line block-comment folds emitted by Source's bundled language
/// providers. Only a comment whose opener starts its line creates the range;
/// single-line comments do not fold.
fn block_comment_fold_ranges(
    text: &str,
    block: Option<(&'static str, &'static str)>,
) -> Vec<FoldRange> {
    let Some((open, close)) = block else {
        return Vec::new();
    };
    let lines = text.split('\n').collect::<Vec<_>>();
    let mut ranges = Vec::new();
    let mut line = 0;
    while line < lines.len() {
        let Some(after_open) = lines[line].trim_start().strip_prefix(open) else {
            line += 1;
            continue;
        };
        if after_open.contains(close) {
            line += 1;
            continue;
        }
        let end = (line + 1..lines.len()).find(|end| lines[*end].contains(close));
        if let Some(end) = end {
            ranges.push(FoldRange { start: line, end });
            line = end + 1;
        } else {
            line += 1;
        }
    }
    ranges
}

/// Source's bundled language workers only classify comment folds for
/// JavaScript/TypeScript, JSON, CSS-family and HTML-family documents. Other
/// languages use indentation folding and must not invent comment ranges from
/// their block-comment tokens.
fn block_comment_fold_ranges_for(rel: &str, text: &str) -> Vec<FoldRange> {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "js" | "mjs"
            | "cjs"
            | "jsx"
            | "ts"
            | "mts"
            | "cts"
            | "tsx"
            | "json"
            | "jsonc"
            | "css"
            | "scss"
            | "less"
            | "html"
            | "htm"
            | "xml"
            | "svg"
    ) {
        return Vec::new();
    }
    block_comment_fold_ranges(text, comment_config(rel).block)
}

#[cfg(test)]
fn fold_ranges_for(text: &str) -> Vec<FoldRange> {
    fold_ranges_for_language("", text)
}

fn fold_ranges_for_language(rel: &str, text: &str) -> Vec<FoldRange> {
    let mut by_start = BTreeMap::<usize, usize>::new();
    for range in indentation_fold_ranges(text)
        .into_iter()
        .chain(brace_fold_ranges(text))
        .chain(region_fold_ranges(text))
        .chain(markup_fold_ranges(text))
        .chain(block_comment_fold_ranges_for(rel, text))
    {
        by_start
            .entry(range.start)
            .and_modify(|end| *end = (*end).max(range.end))
            .or_insert(range.end);
    }
    by_start
        .into_iter()
        .map(|(start, end)| FoldRange { start, end })
        .collect()
}

fn visible_lines_after_folds(
    count: usize,
    folds: &[FoldRange],
    folded: &BTreeSet<usize>,
) -> Vec<usize> {
    let mut hidden = BTreeSet::new();
    for range in folds.iter().filter(|range| folded.contains(&range.start)) {
        for line in range.start + 1..=range.end.min(count.saturating_sub(1)) {
            hidden.insert(line);
        }
    }
    (0..count).filter(|line| !hidden.contains(line)).collect()
}

/// Regions containing `line`, innermost first. This mirrors Monaco's
/// `getAllRegionsAtLine` ordering; the first entry is `getRegionAtLine`.
fn fold_region_starts_at_line(folds: &[FoldRange], line: usize) -> Vec<usize> {
    let mut starts = folds
        .iter()
        .filter(|range| range.start <= line && line <= range.end)
        .map(|range| range.start)
        .collect::<Vec<_>>();
    starts.sort_unstable_by(|left, right| right.cmp(left));
    starts
}

/// Child regions contained by the region starting at `start`.
fn fold_regions_inside(folds: &[FoldRange], start: usize) -> Vec<usize> {
    let Some(region) = folds.iter().find(|range| range.start == start) else {
        return Vec::new();
    };
    folds
        .iter()
        .filter(|range| range.start > region.start && range.end <= region.end)
        .map(|range| range.start)
        .collect()
}

/// Monaco's `setCollapseStateForRest` filter: keep regions that neither
/// contain nor are contained by any blocked region.
fn fold_regions_except(folds: &[FoldRange], blocked: &[usize]) -> Vec<usize> {
    let blocked = blocked
        .iter()
        .filter_map(|start| folds.iter().find(|range| range.start == *start))
        .collect::<Vec<_>>();
    folds
        .iter()
        .filter(|range| {
            blocked.iter().all(|blocked| {
                !(blocked.start <= range.start && range.end <= blocked.end)
                    && !(range.start <= blocked.start && blocked.end <= range.end)
            })
        })
        .map(|range| range.start)
        .collect()
}

pub struct Edited;
pub struct SaveRequested;
pub struct OpenSymbol {
    pub key: DocumentKey,
    pub offset: usize,
}
pub struct CodeEditor {
    pub key: DocumentKey,
    instance: u64,
    documents: Rc<RefCell<Documents>>,
    focus: FocusHandle,
    caret: gpui::Entity<SmoothCaret>,
    scroll_remaining: f32,
    scroll_running: bool,
    scroll_generation: u64,
    last_scroll_top: gpui::ListOffset,
    list: ListState,
    starts: Vec<usize>,
    revision: Option<u64>,
    workspace_revision: u64,
    painted: BTreeMap<usize, PaintedLine>,
    bounds: Option<Bounds<Pixels>>,
    folds: Vec<FoldRange>,
    folded: BTreeSet<usize>,
    visible_lines: Vec<usize>,
    extra_cursors: Vec<Range<usize>>,
    completion_open: bool,
    completion_items: Vec<String>,
    completion_selected: usize,
    completion_range: Range<usize>,
    completion_prefix: String,
    completion_generation: u64,
    diagnostics: Vec<crate::diagnostics::Diagnostic>,
    validation_diagnostics: Vec<crate::diagnostics::Diagnostic>,
    validation_revision: Option<u64>,
    symbols_open: bool,
    symbols_title: String,
    symbols: Vec<crate::symbols::SymbolLocation>,
    symbols_all: Vec<crate::symbols::SymbolLocation>,
    symbols_filter_active: bool,
    symbols_filter: Entity<crate::input::TextInput>,
    _symbols_filter_subscription: Subscription,
    symbol_selected: usize,
    workspace_symbols_open: bool,
    workspace_symbols_query: String,
    workspace_symbols_title: String,
    workspace_symbols_references: bool,
    workspace_symbols_filter_active: bool,
    workspace_symbols_filter: Entity<crate::input::TextInput>,
    _workspace_symbols_filter_subscription: Subscription,
    workspace_symbols: Vec<crate::symbols::WorkspaceSymbol>,
    workspace_symbol_selected: usize,
    hover_popup: Option<(crate::hover::Hover, Point<Pixels>)>,
    occurrences: Vec<Range<usize>>,
    brackets: Vec<Range<usize>>,
    bracket_colors: Vec<(Range<usize>, usize)>,
    rename_open: bool,
    rename_input: Entity<crate::input::TextInput>,
    rename_original: String,
    rename_targets: Vec<crate::rename::RenameFile>,
    rename_anchor: usize,
    rename_error: Option<String>,
    _rename_subscription: Subscription,
    code_actions_open: bool,
    code_actions: Vec<crate::code_actions::CodeAction>,
    code_action_selected: usize,
    selecting: bool,
    reveal_caret: bool,
    find_open: bool,
    replace_open: bool,
    find_query: Entity<crate::input::TextInput>,
    replacement: Entity<crate::input::TextInput>,
    find_options: vela_core::search::Options,
    find_pattern: Option<vela_core::search::Pattern>,
    find_scope: Option<Range<usize>>,
    find_matches: vela_core::search::Highlights,
    find_error: Option<String>,
    find_revision: Option<u64>,
    find_anchor: usize,
    goto_open: bool,
    goto_input: Entity<crate::input::TextInput>,
    _goto_subscription: Subscription,
    palette_open: bool,
    palette_query: Entity<crate::input::TextInput>,
    palette_selected: usize,
    _palette_subscription: Subscription,
    k_chord: bool,
    _find_subscription: Subscription,
    blur_subscription: Option<Subscription>,
}
#[derive(Clone)]
struct PaintedLine {
    start: usize,
    end: usize,
    origin: Point<Pixels>,
    layout: Arc<WrappedLine>,
    // Expanded tabs have several display byte offsets for one source byte.
    mapping: Vec<(usize, usize)>,
}
impl PaintedLine {
    fn source_index(&self, display: usize) -> usize {
        self.start
            + self
                .mapping
                .iter()
                .rev()
                .find(|(d, _)| *d <= display)
                .map(|(_, s)| *s)
                .unwrap_or(0)
    }
    fn display_index(&self, source: usize) -> usize {
        self.mapping
            .iter()
            .find(|(_, s)| self.start + *s >= source)
            .map(|(d, _)| *d)
            .unwrap_or(self.layout.len())
    }
}
fn quick_suggestion_trigger(rel: &str, typed: char) -> bool {
    if typed.is_alphanumeric() || matches!(typed, '_' | '$' | '.') {
        return true;
    }
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "css" | "scss" | "less" => typed == '-',
        "html" | "htm" | "xml" => matches!(typed, '-' | '>'),
        _ => false,
    }
}

fn quick_suggestion_in_comment(text: &str, caret: usize) -> bool {
    let comments = crate::semantic::mask_comments(text);
    let caret = caret.min(text.len());
    text[caret..].chars().next().is_some_and(|ch| !ch.is_whitespace())
        && comments[caret..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
}

const CARET_FRAME: Duration = Duration::from_millis(16);
const CARET_HOLD_FRAMES: u32 = 33;
const CARET_FADE_STEP: f32 = 16.0 / 140.0;

/// Monaco's `cursorBlinking: smooth` fades the caret instead of hard-toggling.
/// Opacity fades over ~140 ms after each ~530 ms hold; typing resets it fully
/// visible for the next hold.
struct SmoothCaret {
    opacity: f32,
    target: f32,
    remaining: u32,
    running: bool,
    generation: u64,
}

impl SmoothCaret {
    fn new(_: &mut Context<Self>) -> Self {
        Self {
            opacity: 1.,
            target: 1.,
            remaining: CARET_HOLD_FRAMES,
            running: false,
            generation: 0,
        }
    }

    fn tick(&mut self) {
        if self.remaining == 0 {
            self.target = if self.target > 0.5 { 0. } else { 1. };
            self.remaining = CARET_HOLD_FRAMES;
        } else {
            self.remaining -= 1;
        }
        if self.opacity < self.target {
            self.opacity = (self.opacity + CARET_FADE_STEP).min(self.target);
        } else if self.opacity > self.target {
            self.opacity = (self.opacity - CARET_FADE_STEP).max(self.target);
        }
    }

    fn update_state(&mut self, focused: bool, cx: &mut Context<Self>) {
        if !focused {
            if self.running {
                self.running = false;
                self.generation = self.generation.wrapping_add(1);
            }
            if self.opacity < 1. || self.target < 1. {
                self.opacity = 1.;
                self.target = 1.;
                self.remaining = CARET_HOLD_FRAMES;
                cx.notify();
            }
            return;
        }
        if self.running {
            return;
        }
        self.running = true;
        self.opacity = 1.;
        self.target = 1.;
        self.remaining = CARET_HOLD_FRAMES;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CARET_FRAME).await;
                let keep = this
                    .update(cx, |caret, cx| {
                        if !caret.running || caret.generation != generation {
                            return false;
                        }
                        caret.tick();
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
        cx.notify();
    }

    fn reset_values(&mut self) {
        self.opacity = 1.;
        self.target = 1.;
        self.remaining = CARET_HOLD_FRAMES;
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        self.reset_values();
        cx.notify();
    }

    fn opacity(&self) -> f32 {
        self.opacity
    }
}

fn comment_auto_close(rel: &str, text: &str, caret: usize) -> bool {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !matches!(extension.as_str(), "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx") {
        return false;
    }
    if !text[..caret].ends_with("/*") {
        return false;
    }
    let strings = crate::object_keys::mask_strings(text);
    let caret = caret.min(text.len());
    !(text[caret..].chars().next().is_some_and(|ch| !ch.is_whitespace())
        && strings[caret..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CommentConfig {
    line: Option<&'static str>,
    block: Option<(&'static str, &'static str)>,
}

/// Monaco's bundled language configurations for the file types Vela opens.
/// Languages without a line-comment token (CSS, HTML/XML, Markdown) fall back
/// to a block comment when `Cmd+/` is pressed, exactly like Monaco's
/// `toggleLineComment` command.
fn comment_config(rel: &str) -> CommentConfig {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let slash = CommentConfig {
        line: Some("//"),
        block: Some(("/*", "*/")),
    };
    match extension.as_str() {
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "mts" | "cts" | "tsx" | "json" | "jsonc"
        | "java" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "cs" | "go" | "rs" | "swift"
        | "kt" | "kts" | "scala" => slash,
        "php" => slash,
        "css" => CommentConfig {
            line: None,
            block: Some(("/*", "*/")),
        },
        "scss" | "less" => slash,
        "html" | "htm" | "xml" | "svg" | "md" | "markdown" => CommentConfig {
            line: None,
            block: Some(("<!--", "-->")),
        },
        "py" | "pyi" | "sh" | "bash" | "zsh" | "fish" | "yaml" | "yml" | "toml"
        | "dockerfile" | "makefile" => CommentConfig {
            line: Some("#"),
            block: None,
        },
        "ini" | "cfg" | "conf" => CommentConfig {
            line: Some(";"),
            block: None,
        },
        "sql" => CommentConfig {
            line: Some("--"),
            block: Some(("/*", "*/")),
        },
        "rb" => CommentConfig {
            line: Some("#"),
            block: None,
        },
        _ => CommentConfig::default(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommentToggle {
    edits: Vec<(Range<usize>, String)>,
    selection: Range<usize>,
}

fn comment_toggle_for(
    rel: &str,
    text: &str,
    selected: Range<usize>,
    block_only: bool,
) -> Option<CommentToggle> {
    let config = comment_config(rel);
    let selected = normalize_range(text, selected);
    if !block_only && let Some(line) = config.line {
        return Some(toggle_line_comments(text, selected, line));
    }
    let (start, end) = config.block?;
    toggle_block_comment(text, selected, start, end)
}

fn normalize_range(text: &str, selected: Range<usize>) -> Range<usize> {
    let start = selected.start.min(text.len());
    let end = selected.end.min(text.len()).max(start);
    start..end
}

/// (line start, content end) for the line containing `offset`.
fn comment_line_bounds(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let end = text[start..]
        .find('\n')
        .map_or(text.len(), |index| start + index);
    (start, end)
}

fn comment_line_spans(text: &str, selected: Range<usize>) -> Vec<(usize, usize)> {
    let Range { start, end } = normalize_range(text, selected);
    let (start_line, _) = comment_line_bounds(text, start);
    let end_probe = if end > start && (end == text.len() || text.as_bytes().get(end) == Some(&b'\n'))
    {
        end.saturating_sub(1)
    } else {
        end
    };
    let (end_line, _) = comment_line_bounds(text, end_probe);
    let mut spans = Vec::new();
    let mut cursor = start_line;
    loop {
        let (line_start, line_end) = comment_line_bounds(text, cursor);
        spans.push((line_start, line_end));
        if line_start >= end_line {
            break;
        }
        cursor = line_end + 1;
    }
    spans
}

fn comment_first_non_whitespace(text: &str, line_start: usize, line_end: usize) -> usize {
    text[line_start..line_end]
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map_or(line_end, |(index, _)| line_start + index)
}

fn comment_visual_column(line: &str, offset: usize, tab_size: usize) -> usize {
    let mut column = 0usize;
    for ch in line[..offset.min(line.len())].chars() {
        column = if ch == '\t' {
            column + (tab_size - column % tab_size)
        } else {
            column + 1
        };
    }
    column
}

fn comment_offset_at_column(line: &str, target: usize, tab_size: usize) -> usize {
    let mut column = 0usize;
    for (index, ch) in line.char_indices() {
        if column >= target {
            return index;
        }
        let next = if ch == '\t' {
            column + (tab_size - column % tab_size)
        } else {
            column + 1
        };
        if next > target {
            return index;
        }
        column = next;
        if column == target {
            return index + ch.len_utf8();
        }
    }
    line.len()
}

fn toggle_line_comments(text: &str, selected: Range<usize>, token: &str) -> CommentToggle {
    let collapsed = selected.start == selected.end;
    let caret_at_eol = collapsed && {
        let (_, end) = comment_line_bounds(text, selected.start);
        selected.start == end
    };
    let spans = comment_line_spans(text, selected.clone());
    let candidates = spans
        .iter()
        .copied()
        .filter(|(start, end)| !text[*start..*end].trim().is_empty())
        .collect::<Vec<_>>();
    let all_whitespace = candidates.is_empty();
    let remove = !all_whitespace
        && candidates.iter().all(|(start, end)| {
            let first = comment_first_non_whitespace(text, *start, *end);
            text[first..*end].starts_with(token)
        });

    let target_lines = if remove || !all_whitespace {
        candidates
    } else {
        spans.clone()
    };
    let columns = target_lines
        .iter()
        .map(|(start, end)| {
            let first = if remove || !all_whitespace {
                comment_first_non_whitespace(text, *start, *end)
            } else {
                *end
            };
            let line = &text[*start..*end];
            comment_visual_column(line, first - *start, 2)
        })
        .collect::<Vec<_>>();
    let target_column = columns
        .iter()
        .copied()
        .min()
        .map_or(0, |column| column - column % 2);

    let mut edits = Vec::new();
    for ((line_start, line_end), column) in target_lines.iter().zip(columns.iter()) {
        if remove {
            let first = comment_first_non_whitespace(text, *line_start, *line_end);
            let mut end = first + token.len();
            if text.as_bytes().get(end) == Some(&b' ') {
                end += 1;
            }
            edits.push((first..end, String::new()));
        } else {
            let offset = if *column == target_column && !all_whitespace {
                comment_first_non_whitespace(text, *line_start, *line_end)
            } else {
                let line = &text[*line_start..*line_end];
                *line_start + comment_offset_at_column(line, target_column, 2)
            };
            edits.push((offset..offset, format!("{token} ")));
        }
    }
    let selection = map_comment_range(text, selected, &edits, caret_at_eol);
    CommentToggle { edits, selection }
}

fn toggle_block_comment(
    text: &str,
    selected: Range<usize>,
    start_token: &str,
    end_token: &str,
) -> Option<CommentToggle> {
    let (start_line, start_end) = comment_line_bounds(text, selected.start);
    let end_probe = if selected.end > selected.start
        && (selected.end == text.len() || text.as_bytes().get(selected.end) == Some(&b'\n'))
    {
        selected.end.saturating_sub(1)
    } else {
        selected.end
    };
    let (end_line, end_end) = comment_line_bounds(text, end_probe);

    let start_line_text = &text[start_line..start_end];
    let relative_start = selected.start.saturating_sub(start_line);
    let start_search_end = (relative_start + start_token.len()).min(start_line_text.len());
    let start_index = start_line_text[..start_search_end].rfind(start_token);
    let end_line_text = &text[end_line..end_end];
    let relative_end = selected.end.saturating_sub(end_line).min(end_line_text.len());
    let end_search_start = relative_end.saturating_sub(end_token.len());
    let end_index = end_line_text[end_search_start..]
        .find(end_token)
        .map(|index| end_search_start + index);

    if let (Some(start_index), Some(end_index)) = (start_index, end_index) {
        let start_offset = start_line + start_index;
        let end_offset = end_line + end_index;
        if start_offset + start_token.len() <= end_offset {
            let mut start_delete_end = start_offset + start_token.len();
            if text.as_bytes().get(start_delete_end) == Some(&b' ') {
                start_delete_end += 1;
            }
            let mut end_delete_start = end_offset;
            if end_delete_start > 0 && text.as_bytes().get(end_delete_start - 1) == Some(&b' ') {
                end_delete_start -= 1;
            }
            let edits = vec![
                (start_offset..start_delete_end, String::new()),
                (end_delete_start..end_offset + end_token.len(), String::new()),
            ];
            let selection = map_comment_range(text, selected, &edits, false);
            return Some(CommentToggle { edits, selection });
        }
    }

    let wrap_start = comment_first_non_whitespace(text, start_line, start_end);
    let wrap_end = end_end;
    let edits = if wrap_start == wrap_end {
        vec![(wrap_start..wrap_end, format!("{start_token}  {end_token}"))]
    } else {
        vec![
            (wrap_start..wrap_start, format!("{start_token} ")),
            (wrap_end..wrap_end, format!(" {end_token}")),
        ]
    };
    let selection = if wrap_start == wrap_end {
        let caret = wrap_start + start_token.len() + 1;
        caret..caret
    } else {
        let mapped_start = map_comment_offset(text, wrap_start, &edits, false)
            + start_token.len()
            + 1;
        let mapped_end = map_comment_offset(text, wrap_end, &edits, false);
        mapped_start..mapped_end
    };
    Some(CommentToggle { edits, selection })
}

/// Literal find matches for Monaco's multi-cursor commands. A collapsed
/// invocation expands to the word and forces case-sensitive whole-word
/// matching; a non-empty selection uses the Find widget's case/whole-word
/// options, matching `MultiCursorSession.create`.
fn find_occurrences(
    text: &str,
    needle: &str,
    match_case: bool,
    whole_word: bool,
) -> Vec<Range<usize>> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while cursor <= text.len() {
        let Some(found) = find_literal(&text[cursor..], needle, match_case) else {
            break;
        };
        let start = cursor + found;
        let end = start + needle.len();
        if !whole_word || occurrence_is_whole_word(text, start, end) {
            ranges.push(start..end);
        }
        let next = end.max(start + text[start..].chars().next().map_or(1, char::len_utf8));
        if next <= cursor {
            break;
        }
        cursor = next;
    }
    ranges
}

fn find_literal(haystack: &str, needle: &str, match_case: bool) -> Option<usize> {
    if match_case || !needle.is_ascii() {
        return haystack.find(needle);
    }
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .find(|start| haystack[*start..*start + needle.len()].eq_ignore_ascii_case(needle))
}

/// Monaco's Add Selection to Previous Find Match searches backwards from the
/// earliest current selection start without wrapping.
fn previous_occurrence(
    text: &str,
    ranges: &[Range<usize>],
    needle: &str,
) -> Option<Range<usize>> {
    if needle.is_empty() {
        return None;
    }
    let start = ranges.iter().map(|range| range.start).min()?;
    text[..start]
        .rfind(needle)
        .map(|found| found..found + needle.len())
}

/// Monaco's Move Last Selection to Next Find Match searches forwards from the
/// latest current selection end without wrapping.
fn next_occurrence_after(
    text: &str,
    ranges: &[Range<usize>],
    needle: &str,
) -> Option<Range<usize>> {
    if needle.is_empty() {
        return None;
    }
    let end = ranges.iter().map(|range| range.end).max()?;
    text.get(end..)
        .and_then(|tail| tail.find(needle))
        .map(|found| end + found..end + found + needle.len())
}

fn occurrence_is_whole_word(text: &str, start: usize, end: usize) -> bool {
    let word = |ch: char| ch.is_alphanumeric() || ch == '_';
    !text[..start].chars().next_back().is_some_and(word)
        && !text[end..].chars().next().is_some_and(word)
}

/// Translate an offset one logical line up/down at the same grapheme column,
/// matching the editor's ordinary vertical caret movement.
fn translate_line_offset(text: &str, offset: usize, up: bool) -> Option<usize> {
    let offset = offset.min(text.len());
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let column = text[line_start..offset].graphemes(true).count();
    let (target_start, target_end) = if up {
        if line_start == 0 {
            return None;
        }
        let target_end = line_start - 1;
        let target_start = text[..target_end].rfind('\n').map_or(0, |index| index + 1);
        (target_start, target_end)
    } else {
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |index| line_start + index);
        if line_end >= text.len() {
            return None;
        }
        let target_start = line_end + 1;
        let target_end = text[target_start..]
            .find('\n')
            .map_or(text.len(), |index| target_start + index);
        (target_start, target_end)
    };
    let line = &text[target_start..target_end];
    let column = line
        .grapheme_indices(true)
        .nth(column)
        .map(|(index, _)| index)
        .unwrap_or(line.len());
    Some(target_start + column)
}

fn map_comment_range(
    text: &str,
    selected: Range<usize>,
    edits: &[(Range<usize>, String)],
    shift_equality: bool,
) -> Range<usize> {
    let start = map_comment_offset(text, selected.start, edits, shift_equality);
    let end = map_comment_offset(text, selected.end, edits, shift_equality);
    start..end.max(start)
}

fn map_comment_offset(
    _text: &str,
    offset: usize,
    edits: &[(Range<usize>, String)],
    shift_equality: bool,
) -> usize {
    let mut delta = 0i64;
    for (range, replacement) in edits {
        if range.is_empty() {
            if offset > range.start || (shift_equality && offset == range.start) {
                delta += replacement.len() as i64;
            }
        } else if offset >= range.end {
            delta -= (range.end - range.start) as i64;
        } else if offset > range.start {
            return (range.start as i64 + delta).max(0) as usize;
        }
    }
    (offset as i64 + delta).max(0) as usize
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PairAction {
    Insert(char),
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuoteRule {
    Always,
    NotString,
    NotStringOrComment,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PairPolicy {
    parens: bool,
    brackets: bool,
    braces: bool,
    angle: bool,
    surround_angle: bool,
    double_quote: QuoteRule,
    single_quote: QuoteRule,
    backtick: QuoteRule,
}

fn auto_pair_policy(rel: &str) -> PairPolicy {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let all = PairPolicy {
        parens: true,
        brackets: true,
        braces: true,
        angle: false,
        surround_angle: false,
        double_quote: QuoteRule::Always,
        single_quote: QuoteRule::Always,
        backtick: QuoteRule::Never,
    };
    match extension.as_str() {
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "typescript" | "javascript" => PairPolicy {
            double_quote: QuoteRule::NotString,
            single_quote: QuoteRule::NotStringOrComment,
            backtick: QuoteRule::NotStringOrComment,
            ..all
        },
        "json" => PairPolicy {
            parens: false,
            brackets: true,
            braces: true,
            angle: false,
            surround_angle: false,
            double_quote: QuoteRule::NotString,
            single_quote: QuoteRule::Never,
            backtick: QuoteRule::Never,
        },
        "html" | "htm" | "xml" => PairPolicy {
            angle: false,
            surround_angle: true,
            ..all
        },
        "css" | "scss" | "less" => PairPolicy {
            double_quote: QuoteRule::NotStringOrComment,
            single_quote: QuoteRule::NotStringOrComment,
            ..all
        },
        "md" | "markdown" => PairPolicy {
            angle: true,
            surround_angle: false,
            double_quote: QuoteRule::Never,
            single_quote: QuoteRule::Never,
            backtick: QuoteRule::Never,
            ..all
        },
        "py" | "python" => PairPolicy {
            double_quote: QuoteRule::NotString,
            single_quote: QuoteRule::NotStringOrComment,
            ..all
        },
        "sh" | "bash" | "zsh" | "shell" | "shellscript" => PairPolicy {
            backtick: QuoteRule::Always,
            ..all
        },
        _ => all,
    }
}

fn pair_context(text: &str, caret: usize) -> (bool, bool) {
    let comments = crate::semantic::mask_comments(text);
    let strings = crate::object_keys::mask_strings(text);
    let caret = caret.min(text.len());
    let original = text[caret..].chars().next();
    let hidden_in_string = strings[caret..].chars().next();
    let inside_string = original.is_some_and(|ch| !ch.is_whitespace())
        && hidden_in_string.is_some_and(char::is_whitespace);
    let hidden_in_comment = comments[caret..].chars().next();
    let inside_comment = !inside_string
        && original.is_some_and(|ch| !ch.is_whitespace())
        && hidden_in_comment.is_some_and(char::is_whitespace);
    (inside_string, inside_comment)
}

fn quote_allowed(rule: QuoteRule, inside_string: bool, inside_comment: bool) -> bool {
    match rule {
        QuoteRule::Always => true,
        QuoteRule::NotString => !inside_string,
        QuoteRule::NotStringOrComment => !inside_string && !inside_comment,
        QuoteRule::Never => false,
    }
}

fn pair_close_for_angle(typed: char, policy: PairPolicy, angle: bool) -> Option<char> {
    match typed {
        '(' if policy.parens => Some(')'),
        '[' if policy.brackets => Some(']'),
        '{' if policy.braces => Some('}'),
        '<' if angle => Some('>'),
        _ => None,
    }
}

fn pair_close_for(typed: char, policy: PairPolicy) -> Option<char> {
    pair_close_for_angle(typed, policy, policy.angle)
}

fn surround_close_for(typed: char, policy: PairPolicy) -> Option<char> {
    match typed {
        '\'' if policy.single_quote != QuoteRule::Never => Some('\''),
        '"' if policy.double_quote != QuoteRule::Never => Some('"'),
        '`' if policy.backtick != QuoteRule::Never => Some('`'),
        _ => pair_close_for_angle(typed, policy, policy.surround_angle),
    }
}

fn auto_pair_action(
    typed: char,
    text: &str,
    caret: usize,
    policy: PairPolicy,
    inside_string: bool,
    inside_comment: bool,
) -> Option<PairAction> {
    let close = match typed {
        ')' if policy.parens => return (text[caret..].starts_with(')')).then_some(PairAction::Skip),
        ']' if policy.brackets => return (text[caret..].starts_with(']')).then_some(PairAction::Skip),
        '}' if policy.braces => return (text[caret..].starts_with('}')).then_some(PairAction::Skip),
        '>' if policy.angle => return (text[caret..].starts_with('>')).then_some(PairAction::Skip),
        '\'' | '"' | '`' => {
            let rule = match typed {
                '\'' => policy.single_quote,
                '"' => policy.double_quote,
                _ => policy.backtick,
            };
            if !quote_allowed(rule, inside_string, inside_comment) {
                return text[caret..]
                    .starts_with(typed)
                    .then_some(PairAction::Skip);
            }
            typed
        }
        _ => pair_close_for(typed, policy)?,
    };
    if text[caret..].starts_with(close) {
        return Some(PairAction::Skip);
    }
    if matches!(typed, '\'' | '"' | '`')
        && text[..caret]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    Some(PairAction::Insert(close))
}

fn auto_pair_erase(text: &str, caret: usize, angle: bool) -> Option<std::ops::Range<usize>> {
    let previous = text[..caret].chars().next_back()?;
    let next = text[caret..].chars().next()?;
    let paired = matches!(
        (previous, next),
        ('(', ')') | ('[', ']') | ('{', '}') | ('"', '"') | ('\'', '\'') | ('`', '`')
    ) || (angle && (previous, next) == ('<', '>'));
    paired.then(|| caret - previous.len_utf8()..caret + next.len_utf8())
}

fn html_open_tag(line: &str) -> bool {
    let Some(start) = line.rfind('<') else {
        return false;
    };
    let tag = &line[start..];
    tag.ends_with('>')
        && !tag.starts_with("</")
        && !tag.starts_with("<!")
        && !tag.trim_end_matches('>').trim_end().ends_with('/')
}

fn enter_indent(rel: &str, text: &str, caret: usize) -> Option<String> {
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if matches!(extension.as_str(), "md" | "markdown" | "yaml" | "yml") {
        return None;
    }
    let caret = caret.min(text.len());
    let line_start = text[..caret].rfind('\n').map_or(0, |index| index + 1);
    let line = &text[line_start..caret];
    let base = &line[..line.len() - line.trim_start().len()];
    let code = line.trim_end().trim_start();
    let python = matches!(extension.as_str(), "py" | "python");
    let html = matches!(extension.as_str(), "html" | "htm" | "xml");
    let opens = code.ends_with(['{', '[', '('])
        || (python && code.ends_with(':'))
        || (html && html_open_tag(code));
    let next = text[caret..].chars().find(|ch| !ch.is_whitespace());
    let closes = matches!(next, Some('}' | ']' | ')'))
        || (html && text[caret..].trim_start().starts_with("</"));
    let step = format!("{base}  ");
    match (opens, closes) {
        (true, true) => Some(format!("\n{step}\n{base}")),
        (true, false) => Some(format!("\n{step}")),
        (false, true) => Some(format!("\n{base}")),
        (false, false) => None,
    }
}

/// Monaco's Go to Line parser: `line`, `line,col`, `line:col` and `line#col`.
/// Non-positive line numbers count back from the end (`-1` is the last line).
fn parse_goto_line(value: &str, line_count: usize) -> Option<(usize, usize)> {
    let value = value.trim();
    let value = value.strip_prefix(':').unwrap_or(value).trim();
    if value.is_empty() {
        return None;
    }
    let numbers = value
        .split([',', ':', '#'])
        .filter_map(|part| part.trim().parse::<i64>().ok())
        .collect::<Vec<_>>();
    let raw = *numbers.first()?;
    let line = if raw > 0 {
        raw
    } else {
        line_count as i64 + 1 + raw
    };
    if line <= 0 || line > line_count as i64 {
        return None;
    }
    let column = numbers.get(1).copied().filter(|value| *value > 0).unwrap_or(1);
    Some((line as usize, column as usize))
}

fn goto_line_offset(text: &str, value: &str) -> Option<usize> {
    let line_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let (line, column) = parse_goto_line(value, line_count)?;
    let line_start = if line == 1 {
        0
    } else {
        text.match_indices('\n').nth(line - 2).map(|(index, _)| index + 1)?
    };
    let line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |offset| line_start + offset);
    let line_text = &text[line_start..line_end];
    Some(
        line_text
            .grapheme_indices(true)
            .nth(column - 1)
            .map(|(index, _)| line_start + index)
            .unwrap_or(line_end),
    )
}

const PALETTE_COMMANDS: &[&str] = &[
    "Format Document",
    "Format Selection",
    "Find",
    "Replace",
    "Fold",
    "Unfold",
    "Toggle Fold",
    "Fold Recursively",
    "Unfold Recursively",
    "Toggle Fold Recursively",
    "Fold All",
    "Unfold All",
    "Fold All Block Comments",
    "Fold All Regions",
    "Unfold All Regions",
    "Fold All Except Selected",
    "Unfold All Except Selected",
    "Go to Line/Column",
    "Toggle Line Comment",
    "Toggle Block Comment",
    "Select All Occurrences",
    "Add Selection to Next Find Match",
    "Add Selection to Previous Find Match",
    "Move Last Selection to Next Find Match",
    "Move Last Selection to Previous Find Match",
    "Expand Line Selection",
    "Add Cursor Above",
    "Add Cursor Below",
    "Add Cursors to Line Ends",
    "Add Cursors to Top",
    "Add Cursors to Bottom",
    "Move Line Up",
    "Move Line Down",
    "Copy Line Up",
    "Copy Line Down",
    "Join Lines",
    "Indent Line",
    "Outdent Line",
    "Insert Line Above",
    "Insert Line Below",
    "Delete Line",
    "Trim Trailing Whitespace",
    "Go to Bracket",
    "Go to Next Problem",
    "Go to Previous Problem",
    "Sort Lines Ascending",
    "Sort Lines Descending",
    "Reverse Lines",
    "Delete Duplicate Lines",
    "Transform to Uppercase",
    "Transform to Lowercase",
    "Transform to Title Case",
    "Transform to Snake Case",
    "Transform to Camel Case",
    "Transform to Pascal Case",
    "Transform to Kebab Case",
    "Transpose Characters",
    "Duplicate Selection",
    "Select to Bracket",
    "Remove Brackets",
];

fn palette_matches(query: &str) -> Vec<&'static str> {
    let query = query.trim().to_lowercase();
    PALETTE_COMMANDS
        .iter()
        .copied()
        .filter(|command| query.is_empty() || command.to_lowercase().contains(&query))
        .collect()
}

fn line_block_for(
    text: &str,
    selected: Range<usize>,
    whole_doc_for_empty: bool,
) -> Option<(usize, usize, bool)> {
    let selected = normalize_range(text, selected);
    if selected.is_empty() && whole_doc_for_empty {
        let mut count = text.split('\n').count();
        if text.ends_with('\n') {
            count = count.saturating_sub(1);
        }
        return Some((0, count.saturating_sub(1), true));
    }
    let start = text[..selected.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let mut end = text[..selected.end]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    if selected.end > selected.start
        && text.as_bytes().get(selected.end - 1) == Some(&b'\n')
    {
        end = end.saturating_sub(1);
    }
    (start <= end).then_some((start, end, false))
}

fn sort_lines_at(
    text: &str,
    selected: Range<usize>,
    descending: bool,
) -> Option<(String, Range<usize>)> {
    let (start, end, _) = line_block_for(text, selected.clone(), true)?;
    let mut lines = text.split('\n').map(str::to_owned).collect::<Vec<_>>();
    if end >= lines.len() {
        return None;
    }
    let mut block = lines[start..=end].to_vec();
    block.sort();
    if descending {
        block.reverse();
    }
    if block == lines[start..=end] {
        return None;
    }
    lines.splice(start..=end, block);
    Some((lines.join("\n"), selected))
}

fn reverse_lines_at(
    text: &str,
    selected: Range<usize>,
) -> Option<(String, Range<usize>)> {
    let (start, end, _) = line_block_for(text, selected.clone(), true)?;
    let mut lines = text.split('\n').map(str::to_owned).collect::<Vec<_>>();
    if end >= lines.len() {
        return None;
    }
    let mut block = lines[start..=end].to_vec();
    block.reverse();
    if block == lines[start..=end] {
        return None;
    }
    lines.splice(start..=end, block);
    Some((lines.join("\n"), selected))
}

fn delete_duplicate_lines_at(
    text: &str,
    selected: Range<usize>,
) -> Option<(String, Range<usize>)> {
    let (start, end, whole) = line_block_for(text, selected.clone(), true)?;
    let mut lines = text.split('\n').map(str::to_owned).collect::<Vec<_>>();
    if end >= lines.len() {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    let block = lines[start..=end]
        .iter()
        .filter(|line| seen.insert((*line).clone()))
        .cloned()
        .collect::<Vec<_>>();
    if block.len() == end - start + 1 {
        return None;
    }
    lines.splice(start..=end, block);
    let joined = lines.join("\n");
    let selection = if whole {
        let caret = selected.start.min(joined.len());
        caret..caret
    } else {
        let start_offset = line_start_offset(&joined, start)?;
        let last = start + seen.len().saturating_sub(1);
        let end_offset = line_end_offset(&joined, last)?;
        start_offset..end_offset
    };
    Some((joined, selection))
}

fn map_replacement_range(
    range: Range<usize>,
    edits: &[(Range<usize>, String)],
) -> Range<usize> {
    let start = map_replacement_offset(range.start, edits);
    let end = map_replacement_offset(range.end, edits);
    start..end.max(start)
}

fn map_replacement_offset(offset: usize, edits: &[(Range<usize>, String)]) -> usize {
    let mut delta = 0i64;
    for (range, replacement) in edits {
        if offset >= range.end {
            delta += replacement.len() as i64 - (range.end - range.start) as i64;
        } else if offset > range.start {
            return (range.start as i64 + delta + (offset - range.start) as i64)
                .min(range.start as i64 + delta + replacement.len() as i64)
                .max(0) as usize;
        }
    }
    (offset as i64 + delta).max(0) as usize
}

fn spaces_to_next_tab_stop(text: &str, offset: usize) -> String {
    let offset = offset.min(text.len());
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let mut column = 0usize;
    for ch in text[line_start..offset].chars() {
        column = if ch == '\t' {
            column + (2 - column % 2)
        } else {
            column + 1
        };
    }
    " ".repeat(2 - column % 2)
}

fn filter_symbols(
    symbols: &[crate::symbols::SymbolLocation],
    query: &str,
) -> Vec<crate::symbols::SymbolLocation> {
    let query = query.trim().to_lowercase();
    symbols
        .iter()
        .filter(|symbol| query.is_empty() || symbol.preview.to_lowercase().contains(&query))
        .cloned()
        .collect()
}

fn remove_brackets_at(
    text: &str,
    selected: Range<usize>,
) -> Option<(String, Range<usize>)> {
    let selected = normalize_range(text, selected);
    let position = if selected.is_empty() {
        selected.start
    } else {
        selected.end
    };
    let (open, close) = crate::brackets::remove_target(text, position)?;
    let edits = vec![(open, String::new()), (close, String::new())];
    let selection = map_comment_range(text, selected, &edits, false);
    let mut removed = text.to_owned();
    for (range, _) in edits.iter().rev() {
        removed.replace_range(range.clone(), "");
    }
    Some((removed, selection))
}

fn duplicate_selection_at(
    text: &str,
    selected: Range<usize>,
) -> Option<(String, Range<usize>)> {
    let selected = normalize_range(text, selected);
    if selected.is_empty() {
        return copy_line_block(text, selected, true);
    }
    let value = &text[selected.clone()];
    if value.is_empty() {
        return None;
    }
    let at = selected.end;
    let duplicated = format!("{}{}{}", &text[..at], value, &text[at..]);
    Some((duplicated, at..at + value.len()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaseStyle {
    Upper,
    Lower,
    Title,
    Snake,
    Camel,
    Pascal,
    Kebab,
}

fn title_case(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut upper = true;
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            if upper {
                out.extend(ch.to_uppercase());
            } else {
                out.extend(ch.to_lowercase());
            }
            upper = false;
        } else {
            out.push(ch);
            upper = ch != '\'';
        }
    }
    out
}

/// Monaco's `AbstractCaseAction` word boundary used by Camel/Pascal Case.
fn case_words(value: &str) -> Vec<&str> {
    value
        .split(|ch: char| matches!(ch, '_' | '-') || ch.is_whitespace())
        .collect()
}

fn capitalize_case_word(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.extend(first.to_uppercase());
    out.push_str(chars.as_str());
    out
}

fn lowercase_camel_head(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    if !first.is_uppercase() {
        return word.to_owned();
    }
    let Some(second) = chars.next() else {
        return word.to_owned();
    };
    if second.is_uppercase() {
        return word.to_owned();
    }
    let mut out = String::new();
    out.extend(first.to_lowercase());
    out.extend(second.to_lowercase());
    out.push_str(chars.as_str());
    out
}

fn camel_case(value: &str) -> String {
    let mut words = case_words(value).into_iter();
    let mut out = words
        .next()
        .map(lowercase_camel_head)
        .unwrap_or_default();
    for word in words {
        out.push_str(&capitalize_case_word(word));
    }
    out
}

fn pascal_case(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for segment in value.split_inclusive('.') {
        for word in case_words(segment) {
            out.push_str(&capitalize_case_word(word));
        }
    }
    out
}

fn separate_case_boundaries(value: &str, separator: char) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(value.len() + chars.len() / 4);
    for (index, ch) in chars.iter().copied().enumerate() {
        if index > 0 {
            let previous = chars[index - 1];
            let lower_upper = previous.is_lowercase() && ch.is_uppercase();
            let acronym_boundary = ch.is_uppercase()
                && chars
                    .get(index + 1)
                    .is_some_and(|next| next.is_lowercase())
                && (previous.is_uppercase() || previous.is_numeric());
            if lower_upper || acronym_boundary {
                out.push(separator);
            }
        }
        out.push(ch);
    }
    out
}

fn replace_underscore_boundaries(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(value.len());
    let mut index = 0;
    while index < chars.len() {
        if index + 2 < chars.len()
            && !chars[index].is_whitespace()
            && chars[index + 1] == '_'
            && !chars[index + 2].is_whitespace()
        {
            out.push(chars[index]);
            out.push('-');
            out.push(chars[index + 2]);
            index += 3;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

fn snake_case(value: &str) -> String {
    separate_case_boundaries(value, '_').to_lowercase()
}

fn kebab_case(value: &str) -> String {
    separate_case_boundaries(&replace_underscore_boundaries(value), '-').to_lowercase()
}

fn apply_case_style(value: &str, style: CaseStyle) -> String {
    match style {
        CaseStyle::Upper => value.to_uppercase(),
        CaseStyle::Lower => value.to_lowercase(),
        CaseStyle::Title => title_case(value),
        CaseStyle::Snake => snake_case(value),
        CaseStyle::Camel => camel_case(value),
        CaseStyle::Pascal => pascal_case(value),
        CaseStyle::Kebab => kebab_case(value),
    }
}

fn transpose_at(text: &str, caret: usize) -> Option<(Range<usize>, String, usize)> {
    let caret = caret.min(text.len());
    let line_start = text[..caret].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |offset| line_start + offset);
    if caret == line_end {
        if line_end >= text.len() {
            return None;
        }
        let start = caret.saturating_sub(1).max(line_start);
        let end = line_end + 1;
        let reversed = text[start..end].chars().rev().collect::<String>();
        Some((start..end, reversed, end))
    } else {
        let start = caret.saturating_sub(1).max(line_start);
        let end = (caret + 1).min(line_end);
        let reversed = text[start..end].chars().rev().collect::<String>();
        Some((start..end, reversed, end))
    }
}

fn diagnostic_target(
    text: &str,
    caret: usize,
    diagnostics: &[crate::diagnostics::Diagnostic],
    forward: bool,
) -> Option<usize> {
    let mut offsets = diagnostics
        .iter()
        .filter_map(|diagnostic| {
            goto_line_offset(
                text,
                &format!("{}:{}", diagnostic.line.max(1), diagnostic.column.max(1)),
            )
        })
        .collect::<Vec<_>>();
    offsets.sort_unstable();
    offsets.dedup();
    if offsets.is_empty() {
        return None;
    }
    let index = if forward {
        offsets
            .iter()
            .position(|offset| *offset > caret)
            .unwrap_or(0)
    } else {
        offsets
            .iter()
            .rposition(|offset| *offset < caret)
            .unwrap_or(offsets.len() - 1)
    };
    offsets.get(index).copied()
}

fn line_column_at(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line = text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = text[line_start..offset].graphemes(true).count() + 1;
    (line, column)
}

/// Monaco's Join Lines whitespace rule: drop the following line's indent,
/// insert one space unless either side already has whitespace, and ignore
/// blank lines.
type ShiftEdits = Vec<(Range<usize>, String)>;

fn string_ranges(text: &str) -> Vec<Range<usize>> {
    let masked = crate::semantic::mask_comments(text);
    let mut ranges = Vec::new();
    let mut quote = None;
    let mut start = 0usize;
    let mut escaped = false;
    for (index, ch) in masked.char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
                ranges.push(start..index + ch.len_utf8());
            }
        } else if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            start = index;
        }
    }
    if quote.is_some() {
        ranges.push(start..masked.len());
    }
    ranges
}

fn trim_trailing_whitespace_at(
    text: &str,
    caret: usize,
    rel: &str,
) -> Option<(String, usize)> {
    let caret = caret.min(text.len());
    let caret_line = text[..caret].bytes().filter(|byte| *byte == b'\n').count();
    let caret_line_start = text[..caret].rfind('\n').map_or(0, |index| index + 1);
    let caret_column = caret - caret_line_start;
    let strings = string_ranges(text);
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let regexes = if matches!(extension.as_str(), "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx") {
        crate::syntax::regex_ranges(text)
    } else {
        Vec::new()
    };
    let mut edits = Vec::new();
    let mut line_start = 0usize;
    for (line, content) in text.split('\n').enumerate() {
        let line_end = line_start + content.len();
        if !content.is_empty() {
            let trailing = content.trim_end_matches([' ', '\t']).len();
            if trailing < content.len() {
                let trailing_start = line_start + trailing;
                let protected = strings
                    .iter()
                    .chain(regexes.iter())
                    .any(|range| range.start < trailing_start && trailing_start < range.end);
                if !protected {
                    let mut from = trailing_start;
                    if line == caret_line {
                        if caret_column == content.len() {
                            line_start = line_end + 1;
                            continue;
                        }
                        from = from.max(line_start + caret_column);
                    }
                    if from < line_end {
                        edits.push((from..line_end, String::new()));
                    }
                }
            }
        }
        line_start = line_end + 1;
    }
    if edits.is_empty() {
        return None;
    }
    let caret = shift_line_offset(caret, &edits, false);
    let mut trimmed = text.to_owned();
    for (range, _) in edits.iter().rev() {
        trimmed.replace_range(range.clone(), "");
    }
    Some((trimmed, caret))
}

fn delete_all_edits(
    text: &str,
    ranges: &[Range<usize>],
    left: bool,
) -> Option<(ShiftEdits, Vec<usize>)> {
    let mut deletions = Vec::new();
    let mut targets = Vec::with_capacity(ranges.len());
    for range in ranges {
        let range = normalize_range(text, range.clone());
        let line_start = text[..range.start].rfind('\n').map_or(0, |index| index + 1);
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |offset| line_start + offset);
        let (start, end, target) = if left {
            if range.is_empty() {
                if range.start == line_start {
                    if line_start == 0 {
                        targets.push(range.start);
                        continue;
                    }
                    (line_start - 1, line_start, line_start - 1)
                } else {
                    (line_start, range.start, line_start)
                }
            } else {
                (line_start, range.end, line_start)
            }
        } else if range.is_empty() {
            if range.start == line_end {
                if line_end >= text.len() {
                    targets.push(range.start);
                    continue;
                }
                (range.start, line_end + 1, range.start)
            } else {
                (range.start, line_end, range.start)
            }
        } else {
            (range.start, range.end, range.start)
        };
        deletions.push(start..end);
        targets.push(target);
    }
    if deletions.is_empty() {
        return None;
    }
    deletions.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in deletions {
        if let Some(last) = merged.last_mut()
            && range.start <= last.end
        {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    let edits = merged
        .iter()
        .cloned()
        .map(|range| (range, String::new()))
        .collect::<Vec<_>>();
    let mapped = targets
        .into_iter()
        .map(|target| shift_line_offset(target, &edits, false))
        .collect();
    Some((edits, mapped))
}

fn shift_line_edits(
    text: &str,
    ranges: &[Range<usize>],
    outdent: bool,
) -> Option<(ShiftEdits, Vec<Range<usize>>)> {
    let mut lines = BTreeSet::new();
    for range in ranges {
        let range = normalize_range(text, range.clone());
        let start_line = text[..range.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        let mut end_line = text[..range.end]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        if range.end > range.start
            && text.as_bytes().get(range.end - 1) == Some(&b'\n')
        {
            end_line = end_line.saturating_sub(1);
        }
        for line in start_line..=end_line {
            lines.insert(line);
        }
    }
    let multi_line = lines.len() > 1;
    let mut edits = Vec::new();
    for line in lines {
        let start = line_start_offset(text, line)?;
        let end = line_end_offset(text, line)?;
        if outdent {
            let line_text = &text[start..end];
            let remove = if line_text.starts_with('\t') {
                1
            } else {
                line_text.bytes().take_while(|byte| *byte == b' ').take(2).count()
            };
            if remove > 0 {
                edits.push((start..start + remove, String::new()));
            }
        } else if !(multi_line && end == start) {
            edits.push((start..start, "  ".to_owned()));
        }
    }
    if edits.is_empty() {
        return None;
    }
    let mapped = ranges
        .iter()
        .map(|range| {
            let range = normalize_range(text, range.clone());
            let empty = range.is_empty();
            let start = shift_line_offset(range.start, &edits, empty);
            let end = shift_line_offset(range.end, &edits, empty);
            start..end.max(start)
        })
        .collect();
    Some((edits, mapped))
}

fn shift_line_offset(
    offset: usize,
    edits: &[(Range<usize>, String)],
    shift_equality: bool,
) -> usize {
    let mut delta = 0i64;
    for (range, replacement) in edits {
        if range.is_empty() {
            if offset > range.start || (shift_equality && offset == range.start) {
                delta += replacement.len() as i64;
            }
        } else if offset >= range.end {
            delta -= (range.end - range.start) as i64;
        } else if offset > range.start {
            return (range.start as i64 + delta).max(0) as usize;
        }
    }
    (offset as i64 + delta).max(0) as usize
}

fn join_lines_at(
    text: &str,
    selected: Range<usize>,
) -> Option<(String, Range<usize>)> {
    let selected = normalize_range(text, selected);
    let start_line = text[..selected.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let selection_end_line = text[..selected.end]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let single_line = start_line == selection_end_line;
    let line_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let end_line = if single_line {
        if start_line + 1 >= line_count {
            return None;
        }
        start_line + 1
    } else {
        selection_end_line
    };
    if start_line >= end_line || end_line >= line_count {
        return None;
    }
    let start = line_start_offset(text, start_line)?;
    let end = line_end_offset(text, end_line)?;
    let mut joined = text
        [line_start_offset(text, start_line)?..line_end_offset(text, start_line)?]
        .to_owned();
    let mut column_delta = 0usize;
    for line in start_line + 1..=end_line {
        let line_start = line_start_offset(text, line)?;
        let line_end = line_end_offset(text, line)?;
        let line_text = &text[line_start..line_end];
        let Some(first) = line_text.find(|ch: char| !ch.is_whitespace()) else {
            column_delta = 0;
            continue;
        };
        let mut insert_space = !joined.is_empty();
        if insert_space
            && joined
                .chars()
                .next_back()
                .is_some_and(|ch| matches!(ch, ' ' | '\t'))
        {
            insert_space = false;
            joined = format!("{} ", joined.trim_end());
        }
        let without_indent = &line_text[first..];
        if insert_space {
            joined.push(' ');
        }
        joined.push_str(without_indent);
        column_delta = without_indent.len() + usize::from(insert_space);
    }
    let result = format!("{}{}{}", &text[..start], joined, &text[end..]);
    let selection = if selected.start == selected.end {
        let offset = start + joined.len().saturating_sub(column_delta);
        offset..offset
    } else if single_line {
        selected
    } else {
        let end_line_start = line_start_offset(text, selection_end_line)?;
        let end_line_end = line_end_offset(text, selection_end_line)?;
        let tail = text[end_line_start..end_line_end]
            .len()
            .saturating_sub(selected.end.saturating_sub(end_line_start));
        let end = start + joined.len().saturating_sub(tail);
        selected.start..end.max(selected.start)
    };
    Some((result, selection))
}

fn line_indent_after(rel: &str, line: &str) -> String {
    let base = &line[..line.len() - line.trim_start().len()];
    let code = line.trim_end();
    let extension = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let python = matches!(extension.as_str(), "py" | "python");
    let html = matches!(extension.as_str(), "html" | "htm" | "xml");
    let opens = code.ends_with(['{', '[', '('])
        || (python && code.ends_with(':'))
        || (html && html_open_tag(code));
    if opens {
        format!("{base}  ")
    } else {
        base.to_owned()
    }
}

fn insert_line_at(text: &str, caret: usize, before: bool, rel: &str) -> (String, usize) {
    let caret = caret.min(text.len());
    let line_start = text[..caret].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |offset| line_start + offset);
    let line = &text[line_start..line_end];
    if before {
        if line_start == 0 {
            (format!("\n{text}"), 0)
        } else {
            let previous_start = text[..line_start - 1]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let previous = &text[previous_start..line_start - 1];
            let indent = line_indent_after(rel, previous);
            let inserted = format!("{indent}\n");
            (
                format!(
                    "{}{}{}",
                    &text[..line_start],
                    inserted,
                    &text[line_start..]
                ),
                line_start + indent.len(),
            )
        }
    } else {
        let indent = line_indent_after(rel, line);
        let inserted = format!("\n{indent}");
        (
            format!("{}{}{}", &text[..line_end], inserted, &text[line_end..]),
            line_end + inserted.len(),
        )
    }
}

fn delete_lines_at(
    text: &str,
    selected: Range<usize>,
    caret: usize,
) -> Option<(String, usize)> {
    if text.is_empty() {
        return None;
    }
    let selected = normalize_range(text, selected);
    let start_line = text[..selected.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let mut end_line = text[..selected.end]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    if selected.end > selected.start
        && text.as_bytes().get(selected.end - 1) == Some(&b'\n')
    {
        end_line = end_line.saturating_sub(1);
    }
    if start_line > end_line {
        return None;
    }
    let line_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let start = line_start_offset(text, start_line)?;
    if end_line + 1 < line_count {
        let next_start = line_start_offset(text, end_line + 1)?;
        let next_end = line_end_offset(text, end_line + 1)?;
        let (_, column) = line_column_at(text, caret.min(text.len()));
        let next_line = &text[next_start..next_end];
        let offset = next_line
            .grapheme_indices(true)
            .nth(column.saturating_sub(1))
            .map(|(index, _)| index)
            .unwrap_or(next_line.len());
        Some((
            format!("{}{}", &text[..start], &text[next_start..]),
            start + offset,
        ))
    } else if start_line == 0 {
        Some((String::new(), 0))
    } else {
        let previous_end = line_end_offset(text, start_line - 1)?;
        Some((text[..previous_end].to_owned(), previous_end))
    }
}

fn line_start_offset(text: &str, line: usize) -> Option<usize> {
    if line == 0 {
        Some(0)
    } else {
        text.match_indices('\n')
            .nth(line - 1)
            .map(|(index, _)| index + 1)
    }
}

fn expand_line_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start_offset = range.start.min(text.len());
    let end_offset = range.end.min(text.len());
    let start = text[..start_offset]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let end = text[end_offset..]
        .find('\n')
        .map_or(text.len(), |index| end_offset + index + 1);
    start..end.max(start)
}

fn line_end_offset(text: &str, line: usize) -> Option<usize> {
    let start = line_start_offset(text, line)?;
    Some(
        text[start..]
            .find('\n')
            .map_or(text.len(), |offset| start + offset),
    )
}

fn line_grapheme_offset(
    text: &str,
    starts: &[usize],
    line: usize,
    column: usize,
    text_len: usize,
) -> usize {
    let start = starts[line];
    let end = starts.get(line + 1).map_or(text_len, |next| next - 1);
    text[start..end]
        .grapheme_indices(true)
        .nth(column)
        .map_or(end, |(offset, _)| start + offset)
}

/// Monaco's Add Cursors to Top/Bottom actions: replicate the primary
/// selection's start/end columns on every line from the current line to the
/// requested document edge.
fn line_edge_cursor_targets(
    text: &str,
    starts: &[usize],
    primary: Range<usize>,
    bottom: bool,
) -> Vec<Range<usize>> {
    let start_line = starts
        .partition_point(|start| *start <= primary.start)
        .saturating_sub(1);
    let end_line = starts
        .partition_point(|start| *start <= primary.end)
        .saturating_sub(1);
    let start_column = text[starts[start_line]..primary.start]
        .graphemes(true)
        .count();
    let end_column = text[starts[end_line]..primary.end]
        .graphemes(true)
        .count();
    let lines = if bottom {
        (start_line..starts.len()).collect::<Vec<_>>()
    } else {
        (0..=start_line).rev().collect::<Vec<_>>()
    };
    lines
        .into_iter()
        .map(|line| {
            let start =
                line_grapheme_offset(text, starts, line, start_column, text.len());
            let end = line_grapheme_offset(text, starts, line, end_column, text.len());
            start.min(end)..start.max(end)
        })
        .collect()
}

/// Monaco's Add Cursors to Line Ends (`Shift+Alt+I`): replace every non-empty
/// selection with a cursor at the end of each covered line, plus the exact
/// selection end when it sits past the final line's first column.
fn line_end_cursor_targets(
    starts: &[usize],
    text_len: usize,
    ranges: &[Range<usize>],
) -> Vec<usize> {
    let mut targets = Vec::new();
    for range in ranges.iter().filter(|range| !range.is_empty()) {
        let start_line = starts
            .partition_point(|start| *start <= range.start)
            .saturating_sub(1);
        let end_line = starts
            .partition_point(|start| *start <= range.end)
            .saturating_sub(1);
        if start_line >= starts.len() || end_line >= starts.len() {
            continue;
        }
        for line in start_line..end_line {
            targets.push(starts.get(line + 1).map_or(text_len, |start| start - 1));
        }
        if range.end > starts[end_line] {
            targets.push(range.end);
        }
    }
    let mut seen = BTreeSet::new();
    targets.retain(|target| seen.insert(*target));
    targets
}

/// Monaco's Copy Line Up/Down (`CopyLinesCommand`). Copy-down inserts the
/// source block before the first selected line and moves the selection with
/// it; copy-up inserts after the last selected line and leaves the selection
/// on the original block, as Monaco's own tests require.
fn copy_line_block(
    text: &str,
    selected: Range<usize>,
    down: bool,
) -> Option<(String, Range<usize>)> {
    let selected = normalize_range(text, selected);
    let lines = text.split('\n').collect::<Vec<_>>();
    let start_line = text[..selected.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let mut end_line = text[..selected.end]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    if selected.end > selected.start
        && text.as_bytes().get(selected.end - 1) == Some(&b'\n')
    {
        end_line = end_line.saturating_sub(1);
    }
    if start_line > end_line || end_line >= lines.len() {
        return None;
    }
    let source = lines[start_line..=end_line].join("\n");
    if down {
        let position = line_start_offset(text, start_line)?;
        let inserted = source.len() + 1;
        let moved = format!(
            "{}{}\n{}",
            &text[..position],
            source,
            &text[position..]
        );
        let start = (selected.start + inserted).min(moved.len());
        let end = (selected.end + inserted).min(moved.len());
        Some((moved, start..end))
    } else {
        let position = line_end_offset(text, end_line)?;
        let moved = format!(
            "{}\n{}{}",
            &text[..position],
            source,
            &text[position..]
        );
        Some((moved, selected))
    }
}

/// Monaco's Move Line Up/Down: move the caret's line or the selected line
/// block one line, preserving the block bytes and shifting its selection.
/// Context-sensitive re-indentation on the move remains a separate Monaco
/// behavior.
fn reindent_line(line: &str, delta: isize) -> (String, usize, usize) {
    let old_prefix = line.len() - line.trim_start().len();
    let prefix = if delta >= 0 {
        format!("{}{}", &line[..old_prefix], " ".repeat(delta as usize))
    } else {
        line[..old_prefix]
            .chars()
            .take((old_prefix as isize + delta).max(0) as usize)
            .collect()
    };
    (
        format!("{prefix}{}", &line[old_prefix..]),
        old_prefix,
        prefix.len(),
    )
}

fn move_line_block(
    text: &str,
    selected: Range<usize>,
    down: bool,
    rel: &str,
) -> Option<(String, Range<usize>)> {
    let selected = normalize_range(text, selected);
    let lines = text.split('\n').map(str::to_owned).collect::<Vec<_>>();
    let start_line = text[..selected.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let mut end_line = text[..selected.end]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    if selected.end > selected.start
        && text.as_bytes().get(selected.end - 1) == Some(&b'\n')
    {
        end_line = end_line.saturating_sub(1);
    }
    if start_line > end_line || end_line >= lines.len() {
        return None;
    }
    if down {
        if end_line + 1 >= lines.len() {
            return None;
        }
    } else if start_line == 0 {
        return None;
    }
    let block_len = end_line - start_line + 1;
    let desired = if down {
        line_indent_after(rel, &lines[end_line + 1])
    } else if start_line == 1 {
        String::new()
    } else {
        line_indent_after(rel, &lines[start_line - 2])
    };
    let first_prefix = lines[start_line].len() - lines[start_line].trim_start().len();
    let delta = desired.len() as isize - first_prefix as isize;
    let mut prefixes = Vec::with_capacity(block_len);
    let mut moved = Vec::with_capacity(block_len);
    for (index, line) in lines[start_line..=end_line].iter().enumerate() {
        if index == 0 {
            moved.push(format!("{desired}{}", line.trim_start()));
            prefixes.push((first_prefix, desired.len()));
        } else {
            let (line, old_prefix, new_prefix) = reindent_line(line, delta);
            moved.push(line);
            prefixes.push((old_prefix, new_prefix));
        }
    }
    let mut new_lines = lines.clone();
    if down {
        new_lines.splice(
            start_line..=end_line + 1,
            std::iter::once(lines[end_line + 1].clone()).chain(moved),
        );
    } else {
        new_lines.splice(
            start_line - 1..=end_line,
            moved
                .into_iter()
                .chain(std::iter::once(lines[start_line - 1].clone())),
        );
    }
    let moved_text = new_lines.join("\n");
    let map = |offset: usize, is_end: bool| {
        let line = text[..offset].bytes().filter(|byte| *byte == b'\n').count();
        let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
        let column = offset - line_start;
        let (new_line, new_column) = if line >= start_line && line <= end_line {
            let index = line - start_line;
            let (old_prefix, new_prefix) = prefixes[index];
            (
                if down {
                    start_line + index + 1
                } else {
                    start_line - 1 + index
                },
                if column <= old_prefix {
                    column.min(new_prefix)
                } else {
                    column - old_prefix + new_prefix
                },
            )
        } else if is_end && column == 0 && line == end_line + 1 {
            (
                if down {
                    start_line + block_len + 1
                } else {
                    start_line - 1 + block_len
                },
                0,
            )
        } else if down && line == end_line + 1 {
            (start_line, column)
        } else if !down && line == start_line - 1 {
            (end_line, column)
        } else {
            (line, column)
        };
        let start = line_start_offset(&moved_text, new_line).unwrap_or(moved_text.len());
        let end = line_end_offset(&moved_text, new_line).unwrap_or(moved_text.len());
        (start + new_column.min(end.saturating_sub(start))).min(moved_text.len())
    };
    let start = map(selected.start, false);
    let end = map(selected.end, true);
    Some((moved_text, start..end.max(start)))
}

fn workspace_fingerprint(documents: &Documents, key: &DocumentKey) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for tab in documents.tabs(&key.workspace) {
        let Some(document) = documents.get(&tab.key) else {
            continue;
        };
        hash ^= document.buffer.revision();
        hash = hash.wrapping_mul(0x100000001b3);
        for byte in tab.key.rel.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

impl EventEmitter<Edited> for CodeEditor {}
impl EventEmitter<SaveRequested> for CodeEditor {}
impl Focusable for CodeEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl CodeEditor {
    pub fn new(
        key: DocumentKey,
        documents: Rc<RefCell<Documents>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let find_query = cx.new(|cx| crate::input::TextInput::new(false, "Find", cx));
        let replacement = cx.new(|cx| crate::input::TextInput::new(false, "Replace", cx));
        let rename_input = cx.new(|cx| crate::input::TextInput::new(false, "New name", cx));
        let rename_subscription =
            cx.subscribe(&rename_input, |this, _, _: &crate::input::Changed, cx| {
                this.rename_error = None;
                cx.notify();
            });
        let subscription = cx.subscribe(&find_query, |this, _, _: &crate::input::Changed, cx| {
            this.compile_find(cx);
            this.select_find(this.find_anchor, false, cx);
        });
        let goto_input = cx.new(|cx| crate::input::TextInput::new(false, "Go to line", cx));
        let goto_subscription =
            cx.subscribe(&goto_input, |_this, _, _: &crate::input::Changed, cx| {
                cx.notify();
            });
        let palette_query =
            cx.new(|cx| crate::input::TextInput::new(false, "Type a command", cx));
        let palette_subscription =
            cx.subscribe(&palette_query, |this, _, _: &crate::input::Changed, cx| {
                this.palette_selected = 0;
                cx.notify();
            });
        let symbols_filter =
            cx.new(|cx| crate::input::TextInput::new(false, "Filter symbols", cx));
        let symbols_filter_subscription =
            cx.subscribe(&symbols_filter, |this, _, _: &crate::input::Changed, cx| {
                this.refresh_symbol_filter(cx);
                cx.notify();
            });
        let workspace_symbols_filter =
            cx.new(|cx| crate::input::TextInput::new(false, "Filter workspace symbols", cx));
        let workspace_symbols_filter_subscription = cx.subscribe(
            &workspace_symbols_filter,
            |this, _, _: &crate::input::Changed, cx| {
                this.refresh_workspace_symbols_filter(cx);
                cx.notify();
            },
        );
        Self {
            key,
            instance: EDITOR_INSTANCE.fetch_add(1, Ordering::Relaxed),
            documents,
            focus: cx.focus_handle(),
            caret: cx.new(SmoothCaret::new),
            scroll_remaining: 0.,
            scroll_running: false,
            scroll_generation: 0,
            last_scroll_top: gpui::ListOffset {
                item_ix: 0,
                offset_in_item: px(0.),
            },
            list: ListState::new(0, ListAlignment::Top, px(400.)),
            starts: vec![0],
            revision: None,
            workspace_revision: 0,
            painted: BTreeMap::new(),
            bounds: None,
            folds: Vec::new(),
            folded: BTreeSet::new(),
            visible_lines: Vec::new(),
            extra_cursors: Vec::new(),
            completion_open: false,
            completion_items: Vec::new(),
            completion_selected: 0,
            completion_range: 0..0,
            completion_prefix: String::new(),
            completion_generation: 0,
            diagnostics: Vec::new(),
            validation_diagnostics: Vec::new(),
            validation_revision: None,
            symbols_open: false,
            symbols_title: String::new(),
            symbols: Vec::new(),
            symbols_all: Vec::new(),
            symbols_filter_active: false,
            symbols_filter,
            _symbols_filter_subscription: symbols_filter_subscription,
            symbol_selected: 0,
            workspace_symbols_open: false,
            workspace_symbols_query: String::new(),
            workspace_symbols_title: String::new(),
            workspace_symbols_references: false,
            workspace_symbols_filter_active: false,
            workspace_symbols_filter,
            _workspace_symbols_filter_subscription: workspace_symbols_filter_subscription,
            workspace_symbols: Vec::new(),
            workspace_symbol_selected: 0,
            hover_popup: None,
            occurrences: Vec::new(),
            brackets: Vec::new(),
            bracket_colors: Vec::new(),
            rename_open: false,
            rename_input,
            rename_original: String::new(),
            rename_targets: Vec::new(),
            rename_anchor: 0,
            rename_error: None,
            _rename_subscription: rename_subscription,
            code_actions_open: false,
            code_actions: Vec::new(),
            code_action_selected: 0,
            selecting: false,
            reveal_caret: false,
            find_open: false,
            replace_open: false,
            find_query,
            replacement,
            find_options: Default::default(),
            find_pattern: None,
            find_scope: None,
            find_matches: Default::default(),
            find_error: None,
            find_revision: None,
            find_anchor: 0,
            goto_open: false,
            goto_input,
            _goto_subscription: goto_subscription,
            palette_open: false,
            palette_query,
            palette_selected: 0,
            _palette_subscription: palette_subscription,
            k_chord: false,
            _find_subscription: subscription,
            blur_subscription: None,
        }
    }
    fn compile_find(&mut self, cx: &mut Context<Self>) {
        let query = self.find_query.read(cx).buffer.text();
        match vela_core::search::Pattern::new_in(query, self.find_options, self.find_scope.clone())
        {
            Ok(pattern) => {
                self.find_pattern = Some(pattern);
                self.find_error = None;
            }
            Err(error) => {
                self.find_pattern = None;
                self.find_error = Some(error);
            }
        }
        self.find_revision = None;
        self.refresh_find();
        cx.notify();
    }
    fn refresh_find(&mut self) {
        if !self.find_open {
            return;
        }
        let revision = self.read(|buffer| buffer.revision());
        if self.find_revision == revision {
            return;
        }
        self.find_matches = self
            .read(|buffer| {
                self.find_pattern
                    .as_ref()
                    .map(|pattern| pattern.highlights(buffer.text()))
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        self.find_revision = revision;
    }
    fn select_find(&mut self, from: usize, previous: bool, cx: &mut Context<Self>) {
        let range = self
            .read(|buffer| {
                self.find_pattern.as_ref().and_then(|pattern| {
                    if previous {
                        pattern.previous(buffer.text(), from)
                    } else {
                        pattern.next(buffer.text(), from)
                    }
                })
            })
            .flatten();
        if let Some(range) = range {
            self.documents
                .borrow_mut()
                .edit(&self.key, |buffer| buffer.select(range.start, range.end));
            self.reveal_caret = true;
        }
        cx.notify();
    }
    fn open_find(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (selection, seed) = self
            .read(|buffer| {
                (
                    buffer.selection().start,
                    buffer.text()[buffer.selection()].to_owned(),
                )
            })
            .unwrap_or_default();
        self.find_anchor = selection;
        self.find_open = true;
        self.find_scope = None;
        self.replace_open |= replace;
        if !seed.is_empty() {
            self.find_query
                .update(cx, |input, cx| input.set_text(seed, cx));
        }
        self.find_query
            .update(cx, |input, _| input.buffer.select_all());
        self.compile_find(cx);
        self.select_find(self.find_anchor, false, cx);
        window.focus(&self.find_query.focus_handle(cx), cx);
    }
    fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_open = false;
        self.find_matches = Default::default();
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = true;
        self.palette_selected = 0;
        self.palette_query.update(cx, |input, cx| {
            input.set_text(String::new(), cx);
            input.buffer.select(0, 0);
        });
        self.close_completion(cx);
        window.focus(&self.palette_query.focus_handle(cx), cx);
        cx.notify();
    }
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn palette_items(&self, cx: &App) -> Vec<&'static str> {
        let query = self.palette_query.read(cx).buffer.text();
        palette_matches(query)
    }
    fn execute_palette(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items = self.palette_items(cx);
        let Some(command) = items.get(index).copied() else {
            return;
        };
        self.palette_open = false;
        window.focus(&self.focus, cx);
        match command {
            "Format Document" => self.format_document(cx),
            "Format Selection" => self.format_selection(cx),
            "Find" => self.open_find(false, window, cx),
            "Replace" => self.open_find(true, window, cx),
            "Fold" => self.fold_at_selection(false, cx),
            "Unfold" => self.unfold_at_selection(false, cx),
            "Toggle Fold" => self.toggle_fold_at_selection(false, cx),
            "Fold Recursively" => self.fold_at_selection(true, cx),
            "Unfold Recursively" => self.unfold_at_selection(true, cx),
            "Toggle Fold Recursively" => self.toggle_fold_at_selection(true, cx),
            "Fold All" => self.fold_all(cx),
            "Unfold All" => self.unfold_all(cx),
            "Fold All Block Comments" => self.fold_all_block_comments(cx),
            "Fold All Regions" => self.set_all_regions(true, cx),
            "Unfold All Regions" => self.set_all_regions(false, cx),
            "Fold All Except Selected" => self.set_all_except_selection(true, cx),
            "Unfold All Except Selected" => self.set_all_except_selection(false, cx),
            "Go to Line/Column" => self.open_goto_line(window, cx),
            "Toggle Line Comment" => self.toggle_comment(false, cx),
            "Toggle Block Comment" => self.toggle_comment(true, cx),
            "Select All Occurrences" => self.select_all_occurrences(cx),
            "Add Selection to Next Find Match" => self.add_next_occurrence(cx),
            "Add Selection to Previous Find Match" => self.add_previous_occurrence(cx),
            "Move Last Selection to Next Find Match" => {
                self.move_selection_to_next_find_match(cx)
            }
            "Move Last Selection to Previous Find Match" => {
                self.move_selection_to_previous_find_match(cx)
            }
            "Expand Line Selection" => self.expand_line_selection(cx),
            "Add Cursor Above" => self.add_cursor_line(true, cx),
            "Add Cursor Below" => self.add_cursor_line(false, cx),
            "Add Cursors to Line Ends" => self.add_cursors_to_line_ends(cx),
            "Add Cursors to Top" => self.add_cursors_to_edge(false, cx),
            "Add Cursors to Bottom" => self.add_cursors_to_edge(true, cx),
            "Move Line Up" => self.move_lines(false, cx),
            "Move Line Down" => self.move_lines(true, cx),
            "Copy Line Up" => self.copy_lines(false, cx),
            "Copy Line Down" => self.copy_lines(true, cx),
            "Join Lines" => self.join_lines(cx),
            "Indent Line" => self.shift_lines(false, cx),
            "Outdent Line" => self.shift_lines(true, cx),
            "Insert Line Above" => self.insert_line(true, cx),
            "Insert Line Below" => self.insert_line(false, cx),
            "Delete Line" => self.delete_selected_lines(cx),
            "Trim Trailing Whitespace" => self.trim_trailing_whitespace(cx),
            "Go to Bracket" => self.go_to_bracket(cx),
            "Go to Next Problem" => self.go_to_problem(true, cx),
            "Go to Previous Problem" => self.go_to_problem(false, cx),
            "Sort Lines Ascending" => self.apply_line_transform(
                |text, selection| sort_lines_at(text, selection, false),
                cx,
            ),
            "Sort Lines Descending" => self.apply_line_transform(
                |text, selection| sort_lines_at(text, selection, true),
                cx,
            ),
            "Reverse Lines" => self.apply_line_transform(reverse_lines_at, cx),
            "Delete Duplicate Lines" => {
                self.apply_line_transform(delete_duplicate_lines_at, cx)
            }
            "Transform to Uppercase" => {
                self.transform_selection_case(CaseStyle::Upper, cx)
            }
            "Transform to Lowercase" => {
                self.transform_selection_case(CaseStyle::Lower, cx)
            }
            "Transform to Title Case" => {
                self.transform_selection_case(CaseStyle::Title, cx)
            }
            "Transform to Snake Case" => {
                self.transform_selection_case(CaseStyle::Snake, cx)
            }
            "Transform to Camel Case" => {
                self.transform_selection_case(CaseStyle::Camel, cx)
            }
            "Transform to Pascal Case" => {
                self.transform_selection_case(CaseStyle::Pascal, cx)
            }
            "Transform to Kebab Case" => {
                self.transform_selection_case(CaseStyle::Kebab, cx)
            }
            "Transpose Characters" => self.transpose_characters(cx),
            "Duplicate Selection" => self.duplicate_selection(cx),
            "Select to Bracket" => self.select_to_bracket(cx),
            "Remove Brackets" => self.remove_brackets(cx),
            _ => {}
        }
        cx.notify();
    }
    fn transform_selection_case(&mut self, style: CaseStyle, cx: &mut Context<Self>) {
        let Some((text, primary)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let ranges = self.selection_ranges();
        let primary_index = ranges
            .iter()
            .position(|range| *range == primary)
            .unwrap_or(0);
        let mut edits = Vec::new();
        for range in &ranges {
            let range = if range.is_empty() {
                let caret = range.start;
                let Some((start, end)) = text
                    .unicode_word_indices()
                    .find(|(start, word)| *start <= caret && caret <= start + word.len())
                    .map(|(start, word)| (start, start + word.len()))
                else {
                    continue;
                };
                start..end
            } else {
                range.clone()
            };
            let value = &text[range.clone()];
            let transformed = apply_case_style(value, style);
            if transformed != value {
                edits.push((range, transformed));
            }
        }
        if edits.is_empty() {
            return;
        }
        let mapped = ranges
            .iter()
            .map(|range| map_replacement_range(range.clone(), &edits))
            .collect::<Vec<_>>();
        let selected = mapped
            .get(primary_index)
            .cloned()
            .unwrap_or(primary);
        self.edit(
            move |buffer| {
                buffer.replace_many_with(&edits, 0);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
        self.extra_cursors = mapped
            .into_iter()
            .enumerate()
            .filter(|(index, _)| *index != primary_index)
            .map(|(_, range)| range)
            .collect();
        self.extra_cursors.sort_by_key(|range| range.start);
        cx.notify();
    }
    fn tab_indent(&mut self, outdent: bool, cx: &mut Context<Self>) {
        if outdent {
            self.shift_lines(true, cx);
            return;
        }
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        if !selection.is_empty() {
            let start_line = text[..selection.start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count();
            let end_line = text[..selection.end]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count();
            let line_start = text[..selection.start]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let line_end = text[line_start..]
                .find('\n')
                .map_or(text.len(), |offset| line_start + offset);
            let whole_line = start_line != end_line
                || (selection.start <= line_start && selection.end >= line_end);
            if whole_line {
                self.shift_lines(false, cx);
                return;
            }
        }
        let spaces = spaces_to_next_tab_stop(&text, selection.start);
        self.edit(
            move |buffer| {
                buffer.replace(selection.clone(), &spaces);
                let caret = selection.start + spaces.len();
                buffer.select(caret, caret);
            },
            cx,
        );
    }
    fn duplicate_selection(&mut self, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some((duplicated, selected)) = duplicate_selection_at(&text, selection) else {
            return;
        };
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &duplicated);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
        self.extra_cursors.clear();
        cx.notify();
    }
    fn remove_brackets(&mut self, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some((removed, selected)) = remove_brackets_at(&text, selection) else {
            return;
        };
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &removed);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
        self.extra_cursors.clear();
        cx.notify();
    }
    fn select_to_bracket(&mut self, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some((open, close)) = crate::brackets::select_target(&text, selection.start) else {
            return;
        };
        let selected = open.start..close.end;
        self.documents.borrow_mut().edit(&self.key, |buffer| {
            buffer.select(selected.start, selected.end);
        });
        self.extra_cursors.clear();
        self.reveal_caret = true;
        cx.notify();
    }
    fn transpose_characters(&mut self, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        if !selection.is_empty() {
            return;
        }
        let Some((range, replacement, caret)) = transpose_at(&text, selection.start) else {
            return;
        };
        self.edit(
            move |buffer| {
                buffer.replace(range, &replacement);
                buffer.select(caret, caret);
            },
            cx,
        );
        self.extra_cursors.clear();
        cx.notify();
    }
    fn apply_line_transform(
        &mut self,
        transform: impl FnOnce(&str, Range<usize>) -> Option<(String, Range<usize>)>,
        cx: &mut Context<Self>,
    ) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some((changed, selected)) = transform(&text, selection) else {
            return;
        };
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &changed);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
        self.extra_cursors.clear();
        cx.notify();
    }
    fn on_palette_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.palette_items(cx);
        match event.keystroke.key.as_str() {
            "escape" => self.close_palette(window, cx),
            "up" => {
                if !items.is_empty() {
                    self.palette_selected =
                        (self.palette_selected + items.len() - 1) % items.len();
                    cx.notify();
                }
            }
            "down" => {
                if !items.is_empty() {
                    self.palette_selected = (self.palette_selected + 1) % items.len();
                    cx.notify();
                }
            }
            "enter" => self.execute_palette(self.palette_selected, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
    fn render_palette(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let items = self.palette_items(cx);
        let left = self
            .bounds
            .map(|bounds| ((f32::from(bounds.size.width) - 440.) / 2.).max(8.))
            .unwrap_or(12.);
        let mut menu = div()
            .id("editor-command-palette")
            .debug_selector(|| "editor-command-palette".into())
            .role(Role::Group)
            .aria_label("Command palette")
            .absolute()
            .left(px(left))
            .top(px(10.))
            .w(px(440.))
            .max_h(px(300.))
            .overflow_hidden()
            .p(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.22).into(),
            }])
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(self.palette_query.clone());
        if items.is_empty() {
            menu = menu.child(
                div()
                    .px(px(8.))
                    .py(px(6.))
                    .text_size(px(11.))
                    .text_color(theme.dim)
                    .child("No matching commands"),
            );
        }
        for (index, command) in items.iter().enumerate() {
            let selected = index == self.palette_selected;
            let id = format!("editor-palette-{index}");
            let selector = id.clone();
            menu = menu.child(
                div()
                    .id(SharedString::from(id))
                    .debug_selector(move || selector.clone())
                    .role(Role::ListBoxOption)
                    .aria_label(SharedString::from(*command))
                    .aria_selected(selected)
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(4.))
                    .text_size(px(11.5))
                    .text_color(if selected { theme.text } else { theme.dim })
                    .when(selected, |row| row.bg(theme.selected))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _, window, cx| {
                            editor.execute_palette(index, window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(SharedString::from(*command)),
            );
        }
        menu.on_key_down(cx.listener(Self::on_palette_key))
            .into_any_element()
    }
    fn open_goto_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.goto_open = true;
        self.goto_input.update(cx, |input, cx| {
            input.set_text(String::new(), cx);
            input.buffer.select(0, 0);
        });
        window.focus(&self.goto_input.focus_handle(cx), cx);
        cx.notify();
    }
    fn close_goto_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.goto_open = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn submit_goto_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.goto_input.read(cx).buffer.text().to_owned();
        let Some(offset) = self
            .read(|buffer| goto_line_offset(buffer.text(), &value))
            .flatten()
        else {
            cx.notify();
            return;
        };
        self.documents.borrow_mut().edit(&self.key, |buffer| {
            buffer.select(offset, offset);
        });
        self.reveal_caret = true;
        self.close_goto_line(window, cx);
    }
    fn on_goto_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.close_goto_line(window, cx),
            "enter" => self.submit_goto_line(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
    fn render_goto_line(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let value = self.goto_input.read(cx).buffer.text().to_owned();
        let text = self
            .read(|buffer| buffer.text().to_owned())
            .unwrap_or_default();
        let line_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let (current_line, current_column) = self
            .read(|buffer| line_column_at(buffer.text(), buffer.caret()))
            .unwrap_or((1, 1));
        let status = match parse_goto_line(&value, line_count) {
            Some((line, column)) => {
                let line_start = if line == 1 {
                    0
                } else {
                    text.match_indices('\n')
                        .nth(line - 2)
                        .map_or(0, |(index, _)| index + 1)
                };
                let line_end = text[line_start..]
                    .find('\n')
                    .map_or(text.len(), |offset| line_start + offset);
                let max_column = text[line_start..line_end].graphemes(true).count() + 1;
                if column <= max_column {
                    format!("Go to line {line} and character {column}.")
                } else {
                    format!("Go to line {line}.")
                }
            }
            None if line_count > 1 => format!(
                "Current Line: {current_line}, Character: {current_column}. Type a line number between 1 and {line_count} to navigate to."
            ),
            None => format!(
                "Current Line: {current_line}, Character: {current_column}. Type a line number to navigate to."
            ),
        };
        let left = self
            .bounds
            .map(|bounds| ((f32::from(bounds.size.width) - 360.) / 2.).max(8.))
            .unwrap_or(12.);
        let input = self.goto_input.clone();
        div()
            .id("editor-goto-line")
            .debug_selector(|| "editor-goto-line".into())
            .role(Role::Group)
            .aria_label(SharedString::from(status.clone()))
            .absolute()
            .left(px(left))
            .top(px(10.))
            .w(px(360.))
            .p(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.22).into(),
            }])
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(input)
            .child(
                div()
                    .id("editor-goto-status")
                    .debug_selector(|| "editor-goto-status".into())
                    .role(Role::Label)
                    .aria_label(SharedString::from(status.clone()))
                    .text_size(px(10.5))
                    .text_color(theme.dim)
                    .child(SharedString::from(status)),
            )
            .on_key_down(cx.listener(Self::on_goto_key))
            .into_any_element()
    }
    fn next_find(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let from = self.read(|buffer| buffer.selection().end).unwrap_or(0);
        self.select_find(from, false, cx);
    }
    fn previous_find(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let from = self.read(|buffer| buffer.selection().start).unwrap_or(0);
        self.select_find(from, true, cx);
    }
    fn toggle_case(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.find_options.match_case = !self.find_options.match_case;
        self.compile_find(cx);
        self.select_find(self.find_anchor, false, cx);
    }
    fn toggle_word(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.find_options.whole_word = !self.find_options.whole_word;
        self.compile_find(cx);
        self.select_find(self.find_anchor, false, cx);
    }
    fn toggle_regex(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.find_options.regex = !self.find_options.regex;
        self.compile_find(cx);
        self.select_find(self.find_anchor, false, cx);
    }
    fn toggle_preserve_case(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.find_options.preserve_case = !self.find_options.preserve_case;
        self.compile_find(cx);
        self.select_find(self.find_anchor, false, cx);
    }
    fn toggle_scope(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.find_scope.is_some() {
            self.find_scope = None;
        } else {
            let Some(selection) = self
                .read(|buffer| buffer.selection())
                .filter(|range| !range.is_empty())
            else {
                return;
            };
            self.find_scope = Some(selection);
        }
        self.compile_find(cx);
        self.select_find(self.find_anchor, false, cx);
    }
    fn toggle_replace(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.replace_open = !self.replace_open;
        cx.notify();
    }
    fn replace_one(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let replacement = self.replacement.read(cx).buffer.text().to_owned();
        let range = self
            .read(|buffer| {
                let selection = buffer.selection();
                self.find_pattern.as_ref().and_then(|pattern| {
                    pattern
                        .matches(buffer.text())
                        .find(|range| *range == selection)
                })
            })
            .flatten();
        if let Some(range) = range {
            let replacement = self
                .read(|buffer| {
                    self.find_pattern
                        .as_ref()
                        .map(|pattern| pattern.expand(buffer.text(), &range, &replacement))
                })
                .flatten()
                .unwrap_or(replacement);
            let next = range.start + replacement.len();
            self.edit(|buffer| buffer.replace(range, &replacement), cx);
            self.refresh_find();
            self.select_find(next, false, cx);
        } else {
            // Monaco selects the next match on the first Replace when the
            // selection no longer sits on a match. Only a subsequent click edits.
            let from = self.read(|buffer| buffer.selection().start).unwrap_or(0);
            self.select_find(from, false, cx);
        }
    }
    fn replace_all(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let replacement = self.replacement.read(cx).buffer.text().to_owned();
        let result = self
            .read(|buffer| {
                self.find_pattern
                    .as_ref()
                    .map(|pattern| pattern.replace_all(buffer.text(), &replacement))
            })
            .flatten();
        if let Some((text, count)) = result
            && count > 0
        {
            self.edit(|buffer| buffer.replace(0..buffer.text().len(), &text), cx);
            self.refresh_find();
        }
    }
    fn on_find_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        let replace_focused = self.replacement.focus_handle(cx).is_focused(window);
        match event.keystroke.key.as_str() {
            "escape" => self.close_find(window, cx),
            "enter" if modifiers.platform && (modifiers.alt || replace_focused) => {
                self.replace_all(window, cx);
            }
            "enter" if replace_focused && !modifiers.shift => self.replace_one(window, cx),
            "1" if modifiers.platform && modifiers.shift => self.replace_one(window, cx),
            "enter" if modifiers.shift => self.previous_find(window, cx),
            "enter" => self.next_find(window, cx),
            "r" if modifiers.alt => self.toggle_regex(window, cx),
            "c" if modifiers.alt => self.toggle_preserve_case(window, cx),
            "l" if modifiers.alt => self.toggle_scope(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
    fn render_find(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let selection = self.read(|buffer| buffer.selection());
        let current = self
            .find_matches
            .ranges
            .iter()
            .position(|range| Some(range) == selection.as_ref())
            .map(|index| index + 1);
        let count = self.find_matches.ranges.len();
        let status = if let Some(error) = &self.find_error {
            error.clone()
        } else if count == 0 {
            "No results".into()
        } else {
            format!(
                "{} of {}{}",
                current.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
                count,
                if self.find_matches.more { "+" } else { "" }
            )
        };
        let query = self.find_query.clone();
        let replacement = self.replacement.clone();
        let button = |id: &'static str,
                      label: &'static str,
                      text: &'static str,
                      active: bool,
                      action: fn(&mut Self, &mut Window, &mut Context<Self>)| {
            div()
                .id(id)
                .debug_selector(move || id.to_owned())
                .role(Role::Button)
                .aria_label(label)
                .tab_index(0)
                .px(px(6.))
                .py(px(3.))
                .rounded(px(3.))
                .cursor_pointer()
                .bg(if active {
                    theme.accent.opacity(0.2)
                } else {
                    theme.text.opacity(0.)
                })
                .hover(|style| style.bg(theme.text.opacity(0.08)))
                .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
                .child(text)
                .into_any_element()
        };
        let mut panel = div()
            .id("editor-find")
            .role(Role::Group)
            .aria_label("Find and Replace")
            .absolute()
            .top(px(8.))
            .right(px(8.))
            .w(px(460.))
            .max_w(relative(0.95))
            .p(px(6.))
            .rounded(px(5.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.elevated)
            .text_size(px(12.))
            .line_height(px(18.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .child(button(
                        "find-toggle-replace",
                        "Toggle Replace",
                        "↕",
                        self.replace_open,
                        Self::toggle_replace,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .p(px(4.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.inset)
                            .child(query),
                    )
                    .child(button(
                        "find-case",
                        "Match Case",
                        "Aa",
                        self.find_options.match_case,
                        Self::toggle_case,
                    ))
                    .child(button(
                        "find-word",
                        "Match Whole Word",
                        "ab",
                        self.find_options.whole_word,
                        Self::toggle_word,
                    ))
                    .child(button(
                        "find-regex",
                        "Use Regular Expression",
                        ".*",
                        self.find_options.regex,
                        Self::toggle_regex,
                    ))
                    .child(button(
                        "find-preserve-case",
                        "Preserve Case",
                        "AB",
                        self.find_options.preserve_case,
                        Self::toggle_preserve_case,
                    ))
                    .child(button(
                        "find-scope",
                        "Find in Selection",
                        "▭",
                        self.find_scope.is_some(),
                        Self::toggle_scope,
                    ))
                    .child(button(
                        "find-previous",
                        "Previous Match",
                        "↑",
                        false,
                        Self::previous_find,
                    ))
                    .child(button(
                        "find-next",
                        "Next Match",
                        "↓",
                        false,
                        Self::next_find,
                    ))
                    .child(button(
                        "find-close",
                        "Close Find",
                        "×",
                        false,
                        Self::close_find,
                    )),
            );
        if self.replace_open {
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .p(px(4.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.inset)
                            .child(replacement),
                    )
                    .child(button(
                        "replace-one",
                        "Replace",
                        "Replace",
                        false,
                        Self::replace_one,
                    ))
                    .child(button(
                        "replace-all",
                        "Replace All",
                        "All",
                        false,
                        Self::replace_all,
                    )),
            );
        }
        panel
            .child(
                div()
                    .id("find-status")
                    .role(Role::Label)
                    .aria_label(SharedString::from(status.clone()))
                    .text_color(theme.dim)
                    .child(status),
            )
            .on_key_down(cx.listener(Self::on_find_key))
            .into_any_element()
    }
    fn read<R>(&self, f: impl FnOnce(&TextBuffer) -> R) -> Option<R> {
        self.documents.borrow().get(&self.key).map(|d| f(&d.buffer))
    }
    fn edit(&mut self, f: impl FnOnce(&mut TextBuffer), cx: &mut Context<Self>) {
        self.documents.borrow_mut().edit(&self.key, f);
        self.caret.update(cx, |caret, cx| caret.reset(cx));
        self.sync_lines();
        self.reveal_caret = true;
        cx.emit(Edited);
        cx.notify();
    }
    fn format_selection(&mut self, cx: &mut Context<Self>) {
        let Some((text, primary)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let ranges = self.selection_ranges();
        let mut merged = ranges
            .iter()
            .map(|range| crate::formatting::line_range(&text, range.clone()))
            .collect::<Vec<_>>();
        merged.sort_by_key(|range| range.start);
        let mut merged_ranges: Vec<std::ops::Range<usize>> = Vec::new();
        for range in merged {
            if let Some(last) = merged_ranges.last_mut()
                && range.start <= last.end
            {
                last.end = last.end.max(range.end);
            } else {
                merged_ranges.push(range);
            }
        }
        let edits = merged_ranges
            .iter()
            .filter_map(|range| {
                crate::formatting::selection_edit(&self.key.rel, &text, range.clone())
            })
            .collect::<Vec<_>>();
        if edits.is_empty()
            || edits
                .iter()
                .all(|edit| edit.text == text[edit.range.clone()])
        {
            return;
        }
        let mut starts = Vec::with_capacity(edits.len());
        let mut delta = 0i64;
        for edit in &edits {
            starts.push((edit.range.start as i64 + delta) as usize);
            delta += edit.text.len() as i64 - (edit.range.end - edit.range.start) as i64;
        }
        let mut final_ranges = Vec::with_capacity(ranges.len());
        for original in &ranges {
            let found = edits.iter().enumerate().find(|(_, edit)| {
                edit.range.start <= original.start && original.start <= edit.range.end
            });
            if let Some((index, edit)) = found {
                let old = &text[edit.range.clone()];
                let start = starts[index];
                let relative_start = original
                    .start
                    .saturating_sub(edit.range.start)
                    .min(old.len());
                let relative_end = original.end.saturating_sub(edit.range.start).min(old.len());
                let mapped_start = start + remap_caret(old, &edit.text, relative_start);
                let mapped_end = if original.is_empty() {
                    mapped_start
                } else {
                    start + remap_caret(old, &edit.text, relative_end)
                };
                final_ranges.push(mapped_start..mapped_end);
            } else {
                let mut offset = original.start as i64;
                for edit in &edits {
                    if edit.range.end <= original.start {
                        offset +=
                            edit.text.len() as i64 - (edit.range.end - edit.range.start) as i64;
                    }
                }
                let offset = offset.clamp(0, text.len() as i64) as usize;
                final_ranges.push(offset..offset);
            }
        }
        if final_ranges.is_empty() {
            return;
        }
        let primary_index = ranges
            .iter()
            .position(|range| range == &primary)
            .unwrap_or(0)
            .min(final_ranges.len() - 1);
        let selected = final_ranges[primary_index].clone();
        let mut extra = final_ranges
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != primary_index)
            .map(|(_, range)| range.clone())
            .filter(|range| range != &selected)
            .collect::<Vec<_>>();
        extra.sort_by_key(|range| range.start);
        extra.dedup();
        let edits = edits
            .into_iter()
            .map(|edit| (edit.range, edit.text))
            .collect::<Vec<_>>();
        self.edit(
            move |buffer| {
                buffer.replace_many_with(&edits, 0);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
        self.extra_cursors = extra;
    }
    fn format_document(&mut self, cx: &mut Context<Self>) {
        let Some((text, caret)) = self
            .documents
            .borrow()
            .get(&self.key)
            .map(|document| (document.buffer.text().to_owned(), document.buffer.caret()))
        else {
            return;
        };
        let Some(formatted) = crate::formatting::document(&self.key.rel, &text) else {
            return;
        };
        let caret = remap_caret(&text, &formatted, caret);
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &formatted);
                buffer.select(caret, caret);
            },
            cx,
        );
    }
    fn toggle_comment(&mut self, block_only: bool, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some(toggle) =
            comment_toggle_for(&self.key.rel, &text, selection, block_only)
        else {
            return;
        };
        if toggle.edits.is_empty() {
            return;
        }
        let CommentToggle { edits, selection } = toggle;
        self.edit(
            move |buffer| {
                buffer.replace_many_with(&edits, 0);
                buffer.select(selection.start, selection.end);
            },
            cx,
        );
    }
    fn shift_lines(&mut self, outdent: bool, cx: &mut Context<Self>) {
        let Some((text, primary)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let ranges = self.selection_ranges();
        let primary_index = ranges
            .iter()
            .position(|range| *range == primary)
            .unwrap_or(0);
        let Some((edits, mapped)) = shift_line_edits(&text, &ranges, outdent) else {
            return;
        };
        let selected = mapped
            .get(primary_index)
            .cloned()
            .unwrap_or(primary.clone());
        self.edit(
            move |buffer| {
                buffer.replace_many_with(&edits, 0);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
        self.extra_cursors = mapped
            .into_iter()
            .enumerate()
            .filter(|(index, _)| *index != primary_index)
            .map(|(_, range)| range)
            .collect();
        self.extra_cursors.sort_by_key(|range| range.start);
        cx.notify();
    }
    fn go_to_problem(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some((text, caret)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.caret(),
            )
        }) else {
            return;
        };
        let Some(target) = diagnostic_target(&text, caret, &self.diagnostics, forward) else {
            return;
        };
        self.documents.borrow_mut().edit(&self.key, |buffer| {
            buffer.select(target, target);
        });
        self.extra_cursors.clear();
        self.reveal_caret = true;
        cx.notify();
    }
    fn go_to_bracket(&mut self, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some(target) = crate::brackets::jump_target(&text, selection.start) else {
            return;
        };
        self.documents.borrow_mut().edit(&self.key, |buffer| {
            buffer.select(target, target);
        });
        self.extra_cursors.clear();
        self.reveal_caret = true;
        cx.notify();
    }
    fn delete_all(&mut self, left: bool, cx: &mut Context<Self>) {
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            return;
        };
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        let ranges = self.selection_ranges();
        let primary_index = ranges
            .iter()
            .position(|range| *range == primary)
            .unwrap_or(0);
        let Some((edits, mapped)) = delete_all_edits(&text, &ranges, left) else {
            return;
        };
        let target = mapped
            .get(primary_index)
            .copied()
            .unwrap_or(primary.start);
        self.edit(
            move |buffer| {
                buffer.replace_many_with(&edits, 0);
                buffer.select(target, target);
            },
            cx,
        );
        self.extra_cursors = mapped
            .into_iter()
            .enumerate()
            .filter(|(index, _)| *index != primary_index)
            .map(|(_, target)| target..target)
            .collect();
        self.extra_cursors.sort_by_key(|range| range.start);
        cx.notify();
    }
    fn trim_trailing_whitespace(&mut self, cx: &mut Context<Self>) {
        let Some((text, caret)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.caret(),
            )
        }) else {
            return;
        };
        let Some((trimmed, caret)) = trim_trailing_whitespace_at(&text, caret, &self.key.rel)
        else {
            return;
        };
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &trimmed);
                buffer.select(caret, caret);
            },
            cx,
        );
    }
    fn join_lines(&mut self, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some((joined, selected)) = join_lines_at(&text, selection.clone()) else {
            return;
        };
        if joined == text && selected == selection {
            return;
        }
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &joined);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
    }
    fn insert_line(&mut self, before: bool, cx: &mut Context<Self>) {
        let Some((text, caret)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.caret(),
            )
        }) else {
            return;
        };
        let (inserted, caret) = insert_line_at(&text, caret, before, &self.key.rel);
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &inserted);
                buffer.select(caret, caret);
            },
            cx,
        );
    }
    fn delete_selected_lines(&mut self, cx: &mut Context<Self>) {
        let Some((text, selection, caret)) =
            self.documents.borrow().get(&self.key).map(|document| {
                (
                    document.buffer.text().to_owned(),
                    document.buffer.selection(),
                    document.buffer.caret(),
                )
            })
        else {
            return;
        };
        let Some((deleted, caret)) = delete_lines_at(&text, selection, caret) else {
            return;
        };
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &deleted);
                buffer.select(caret, caret);
            },
            cx,
        );
    }
    fn copy_lines(&mut self, down: bool, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some((copied, selected)) = copy_line_block(&text, selection, down) else {
            return;
        };
        if copied == text {
            return;
        }
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &copied);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
    }
    fn move_lines(&mut self, down: bool, cx: &mut Context<Self>) {
        let Some((text, selection)) = self.documents.borrow().get(&self.key).map(|document| {
            (
                document.buffer.text().to_owned(),
                document.buffer.selection(),
            )
        }) else {
            return;
        };
        let Some((moved, selected)) = move_line_block(&text, selection, down, &self.key.rel) else {
            return;
        };
        if moved == text {
            return;
        }
        self.edit(
            move |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, &moved);
                buffer.select(selected.start, selected.end);
            },
            cx,
        );
    }
    fn sync_lines(&mut self) {
        let (own_revision, fingerprint) = {
            let documents = self.documents.borrow();
            let Some(document) = documents.get(&self.key) else {
                return;
            };
            (
                document.buffer.revision(),
                workspace_fingerprint(&documents, &self.key),
            )
        };
        let own_changed = self.revision != Some(own_revision);
        let workspace_changed = self.workspace_revision != fingerprint;
        if !own_changed && !workspace_changed {
            return;
        }
        if !own_changed {
            let diagnostics = {
                let documents = self.documents.borrow();
                let Some(document) = documents.get(&self.key) else {
                    return;
                };
                let text = document.buffer.text();
                let mut diagnostics = crate::diagnostics::language_diagnostics(&self.key.rel, text);
                let files = documents
                    .tabs(&self.key.workspace)
                    .iter()
                    .filter_map(|tab| {
                        documents
                            .get(&tab.key)
                            .map(|document| (tab.key.clone(), document.buffer.text().to_owned()))
                    })
                    .collect::<Vec<_>>();
                diagnostics.extend(crate::auto_import::diagnostics(&self.key, text, &files));
                if self.validation_revision == Some(own_revision) {
                    diagnostics.extend(self.validation_diagnostics.iter().cloned());
                }
                diagnostics
            };
            self.diagnostics = diagnostics;
            self.workspace_revision = fingerprint;
            return;
        }
        let (starts, folds, bracket_colors, revision, diagnostics) = {
            let documents = self.documents.borrow();
            let Some(document) = documents.get(&self.key) else {
                return;
            };
            let buffer = &document.buffer;
            let text = buffer.text();
            let mut starts = vec![0];
            starts.extend(
                text.bytes()
                    .enumerate()
                    .filter_map(|(i, b)| (b == b'\n').then_some(i + 1)),
            );
            let mut diagnostics = crate::diagnostics::language_diagnostics(&self.key.rel, text);
            let files = documents
                .tabs(&self.key.workspace)
                .iter()
                .filter_map(|tab| {
                    documents
                        .get(&tab.key)
                        .map(|document| (tab.key.clone(), document.buffer.text().to_owned()))
                })
                .collect::<Vec<_>>();
            diagnostics.extend(crate::auto_import::diagnostics(&self.key, text, &files));
            if self.validation_revision == Some(buffer.revision()) {
                diagnostics.extend(self.validation_diagnostics.iter().cloned());
            } else {
                self.validation_diagnostics.clear();
                self.validation_revision = None;
            }
            (
                starts,
                fold_ranges_for_language(&self.key.rel, text),
                crate::brackets::colorized(text),
                buffer.revision(),
                diagnostics,
            )
        };
        let reset = self.revision.is_none();
        self.starts = starts;
        self.folds = folds;
        self.bracket_colors = bracket_colors;
        self.diagnostics = diagnostics;
        self.folded
            .retain(|start| self.folds.iter().any(|range| range.start == *start));
        self.rebuild_visible_lines(reset);
        self.revision = Some(revision);
        self.workspace_revision = fingerprint;
    }

    fn rebuild_visible_lines(&mut self, reset: bool) {
        let visible = visible_lines_after_folds(self.starts.len(), &self.folds, &self.folded);
        let old_len = self.visible_lines.len();
        if reset {
            self.list
                .reset_with_uniform_height(visible.len(), px(LINE_HEIGHT));
        } else if visible != self.visible_lines {
            let common = self
                .visible_lines
                .iter()
                .zip(&visible)
                .take_while(|(left, right)| left == right)
                .count();
            self.list
                .splice(common..old_len, visible.len().saturating_sub(common));
            self.list = self.list.clone().with_uniform_item_height(px(LINE_HEIGHT));
        }
        self.visible_lines = visible;
        self.painted.clear();
    }

    fn toggle_fold(&mut self, line: usize, cx: &mut Context<Self>) {
        let Some(start) = fold_region_starts_at_line(&self.folds, line)
            .first()
            .copied()
        else {
            return;
        };
        if !self.folded.remove(&start) {
            self.folded.insert(start);
        }
        self.rebuild_visible_lines(false);
        cx.notify();
    }

    fn unfold_line(&mut self, line: usize) -> bool {
        let mut changed = false;
        for range in &self.folds {
            if range.start <= line && line <= range.end && self.folded.remove(&range.start) {
                changed = true;
            }
        }
        if changed {
            self.rebuild_visible_lines(false);
        }
        changed
    }

    fn selected_lines(&self) -> Vec<usize> {
        self.selection_ranges()
            .into_iter()
            .map(|range| {
                self.starts
                    .partition_point(|start| *start <= range.start)
                    .saturating_sub(1)
            })
            .collect()
    }

    fn apply_fold_states(&mut self, states: BTreeMap<usize, bool>, cx: &mut Context<Self>) {
        if states.is_empty() {
            return;
        }
        let mut changed = false;
        for (start, collapsed) in states {
            changed |= if collapsed {
                self.folded.insert(start)
            } else {
                self.folded.remove(&start)
            };
        }
        if changed {
            self.rebuild_visible_lines(false);
            cx.notify();
        }
    }

    /// Monaco's `editor.fold`: with no arguments, fold the innermost
    /// uncollapsed region at each selection start, or its first uncollapsed
    /// parent. Recursive folding targets the innermost region and all of its
    /// descendants.
    fn fold_at_selection(&mut self, recursive: bool, cx: &mut Context<Self>) {
        let lines = self.selected_lines();
        let mut states = BTreeMap::new();
        for line in lines {
            let regions = fold_region_starts_at_line(&self.folds, line);
            let target = if recursive {
                regions.first().copied()
            } else {
                regions
                    .into_iter()
                    .find(|start| !self.folded.contains(start))
            };
            let Some(start) = target else {
                continue;
            };
            states.entry(start).or_insert(true);
            if recursive {
                for child in fold_regions_inside(&self.folds, start) {
                    states.entry(child).or_insert(true);
                }
            }
        }
        self.apply_fold_states(states, cx);
    }

    /// Monaco's `editor.unfold` behavior: unfold the innermost collapsed
    /// region at each selection start, or its first collapsed parent.
    fn unfold_at_selection(&mut self, recursive: bool, cx: &mut Context<Self>) {
        let lines = self.selected_lines();
        let mut states = BTreeMap::new();
        for line in lines {
            let Some(start) = fold_region_starts_at_line(&self.folds, line)
                .into_iter()
                .find(|start| self.folded.contains(start))
            else {
                continue;
            };
            states.insert(start, false);
            if recursive {
                for child in fold_regions_inside(&self.folds, start) {
                    if self.folded.contains(&child) {
                        states.insert(child, false);
                    }
                }
            }
        }
        self.apply_fold_states(states, cx);
    }

    /// Monaco's `editor.toggleFold`: toggle the innermost region at each
    /// selection start. Recursive toggling applies the same target state to
    /// every descendant.
    fn toggle_fold_at_selection(&mut self, recursive: bool, cx: &mut Context<Self>) {
        let lines = self.selected_lines();
        let mut states = BTreeMap::new();
        for line in lines {
            let Some(start) = fold_region_starts_at_line(&self.folds, line)
                .first()
                .copied()
            else {
                continue;
            };
            let collapse = !self.folded.contains(&start);
            states.entry(start).or_insert(collapse);
            if recursive {
                for child in fold_regions_inside(&self.folds, start) {
                    states.entry(child).or_insert(collapse);
                }
            }
        }
        self.apply_fold_states(states, cx);
    }

    fn fold_all(&mut self, cx: &mut Context<Self>) {
        let states = self
            .folds
            .iter()
            .map(|range| (range.start, true))
            .collect();
        self.apply_fold_states(states, cx);
    }

    fn unfold_all(&mut self, cx: &mut Context<Self>) {
        if self.folded.is_empty() {
            return;
        }
        self.folded.clear();
        self.rebuild_visible_lines(false);
        cx.notify();
    }

    /// Monaco's `editor.foldAllBlockComments`: fold every multi-line block
    /// comment range emitted for the current language.
    fn fold_all_block_comments(&mut self, cx: &mut Context<Self>) {
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            return;
        };
        let states = block_comment_fold_ranges_for(&self.key.rel, &text)
            .into_iter()
            .map(|range| (range.start, true))
            .collect();
        self.apply_fold_states(states, cx);
    }

    /// Monaco's Fold/Unfold All Regions actions for `#region` markers.
    fn set_all_regions(&mut self, collapse: bool, cx: &mut Context<Self>) {
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            return;
        };
        let states = region_fold_ranges(&text)
            .into_iter()
            .map(|range| (range.start, collapse))
            .collect();
        self.apply_fold_states(states, cx);
    }

    /// Monaco's Fold/Unfold All Except Selected: leave the innermost region at
    /// each selection start (and its ancestors/descendants) untouched.
    fn set_all_except_selection(&mut self, collapse: bool, cx: &mut Context<Self>) {
        let blocked = self
            .selected_lines()
            .into_iter()
            .filter_map(|line| {
                fold_region_starts_at_line(&self.folds, line)
                    .first()
                    .copied()
            })
            .collect::<Vec<_>>();
        let states = fold_regions_except(&self.folds, &blocked)
            .into_iter()
            .map(|start| (start, collapse))
            .collect();
        self.apply_fold_states(states, cx);
    }

    fn selection_ranges(&self) -> Vec<Range<usize>> {
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        let mut ranges = self.extra_cursors.clone();
        ranges.push(primary);
        ranges.sort_by_key(|range| range.start);
        ranges.dedup();
        ranges
    }

    fn clear_multi_cursor(&mut self) {
        self.extra_cursors.clear();
    }

    /// Monaco's `editor.action.insertCursorAbove/Below`: duplicate every
    /// selection one logical line up or down at the same grapheme column.
    fn add_cursor_line(&mut self, up: bool, cx: &mut Context<Self>) {
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            return;
        };
        let ranges = self.selection_ranges();
        if ranges.is_empty() {
            return;
        }
        let mut additions = Vec::new();
        for range in &ranges {
            let (Some(start), Some(end)) = (
                translate_line_offset(&text, range.start, up),
                translate_line_offset(&text, range.end, up),
            ) else {
                continue;
            };
            let target = start..end.max(start);
            if target != *range
                && !ranges.contains(&target)
                && !additions.contains(&target)
            {
                additions.push(target);
            }
        }
        if additions.is_empty() {
            return;
        }
        self.extra_cursors.extend(additions);
        self.extra_cursors.sort_by_key(|range| range.start);
        self.extra_cursors.dedup();
        self.reveal_caret = true;
        cx.notify();
    }

    /// Monaco's `editor.action.insertCursorAtEndOfEachLineSelected`
    /// (`Shift+Alt+I`): replace non-empty selections with cursors at the end
    /// of every covered line.
    fn add_cursors_to_line_ends(&mut self, cx: &mut Context<Self>) {
        let Some(text_len) = self.read(|buffer| buffer.text().len()) else {
            return;
        };
        let targets = line_end_cursor_targets(&self.starts, text_len, &self.selection_ranges());
        let Some(primary) = targets.first().copied() else {
            return;
        };
        self.edit(|buffer| buffer.select(primary, primary), cx);
        self.extra_cursors = targets[1..]
            .iter()
            .map(|target| *target..*target)
            .collect();
        self.reveal_caret = true;
        cx.notify();
    }

    /// Monaco's Add Cursors to Bottom/Top actions. They have no default key
    /// binding and are exposed through the command palette.
    fn add_cursors_to_edge(&mut self, bottom: bool, cx: &mut Context<Self>) {
        let Some((text, primary)) = self.read(|buffer| {
            (buffer.text().to_owned(), buffer.selection())
        }) else {
            return;
        };
        let targets = line_edge_cursor_targets(&text, &self.starts, primary, bottom);
        let Some(first) = targets.first() else {
            return;
        };
        let first = first.clone();
        self.edit(
            |buffer| buffer.select(first.start, first.end),
            cx,
        );
        self.extra_cursors = targets[1..].to_vec();
        self.reveal_caret = true;
        cx.notify();
    }

    /// Monaco's `editor.action.selectHighlights` (`Cmd+Shift+L`). A collapsed
    /// caret uses the word under it with case-sensitive whole-word matching; a
    /// non-empty selection follows the Find widget's case/whole-word options.
    fn select_all_occurrences(&mut self, cx: &mut Context<Self>) {
        let mut primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        let collapsed_word = primary.is_empty();
        if collapsed_word {
            let Some((start, end)) = self
                .read(|buffer| {
                    let caret = buffer.caret();
                    buffer
                        .text()
                        .unicode_word_indices()
                        .find(|(start, word)| *start <= caret && caret <= start + word.len())
                        .map(|(start, word)| (start, start + word.len()))
                })
                .flatten()
            else {
                return;
            };
            primary = start..end;
        }
        if primary.is_empty() {
            return;
        }
        let Some((text, needle)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.text()[primary.clone()].to_owned(),
            )
        }) else {
            return;
        };
        let (match_case, whole_word) = if collapsed_word {
            (true, true)
        } else {
            (
                self.find_options.match_case,
                self.find_options.whole_word,
            )
        };
        let matches = find_occurrences(&text, &needle, match_case, whole_word);
        if matches.is_empty() {
            return;
        }
        let primary_match = matches
            .iter()
            .find(|range| range.start < primary.end && primary.start < range.end)
            .cloned()
            .unwrap_or_else(|| matches[0].clone());
        let mut extras = matches
            .iter()
            .filter(|range| **range != primary_match)
            .cloned()
            .collect::<Vec<_>>();
        extras.sort_by_key(|range| range.start);
        self.documents.borrow_mut().edit(&self.key, |buffer| {
            buffer.select(primary_match.start, primary_match.end);
        });
        self.extra_cursors = extras;
        self.reveal_caret = true;
        cx.notify();
    }

    /// Monaco's `expandLineSelection` (`Cmd+L`): grow each selection to
    /// whole logical lines, adding one line per invocation and ending at the
    /// document end on the last line.
    fn expand_line_selection(&mut self, cx: &mut Context<Self>) {
        let Some((text, primary)) = self.read(|buffer| {
            (buffer.text().to_owned(), buffer.selection())
        }) else {
            return;
        };
        let primary = expand_line_range(&text, primary);
        let mut extras = self
            .extra_cursors
            .iter()
            .cloned()
            .map(|range| expand_line_range(&text, range))
            .filter(|range| range != &primary)
            .collect::<Vec<_>>();
        extras.sort_by_key(|range| range.start);
        extras.dedup();
        self.edit(|buffer| buffer.select(primary.start, primary.end), cx);
        self.extra_cursors = extras;
        self.reveal_caret = true;
        cx.notify();
    }

    fn add_next_occurrence(&mut self, cx: &mut Context<Self>) {
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        if primary.is_empty() {
            let Some((start, end)) = self
                .read(|buffer| {
                    let caret = buffer.caret();
                    buffer
                        .text()
                        .unicode_word_indices()
                        .find(|(start, word)| *start <= caret && caret <= start + word.len())
                        .map(|(start, word)| (start, start + word.len()))
                })
                .flatten()
            else {
                return;
            };
            self.documents
                .borrow_mut()
                .edit(&self.key, |buffer| buffer.select(start, end));
            cx.notify();
            return;
        }
        let Some((text, needle)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.text()[buffer.selection()].to_owned(),
            )
        }) else {
            return;
        };
        if needle.is_empty() {
            return;
        }
        let ranges = self.selection_ranges();
        if let Some(range) = next_occurrence_after(&text, &ranges, &needle)
            && !ranges.contains(&range)
        {
            self.extra_cursors.push(range);
            self.extra_cursors.sort_by_key(|range| range.start);
            cx.notify();
        }
    }

    fn add_previous_occurrence(&mut self, cx: &mut Context<Self>) {
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        if primary.is_empty() {
            let Some((start, end)) = self
                .read(|buffer| {
                    let caret = buffer.caret();
                    buffer
                        .text()
                        .unicode_word_indices()
                        .find(|(start, word)| *start <= caret && caret <= start + word.len())
                        .map(|(start, word)| (start, start + word.len()))
                })
                .flatten()
            else {
                return;
            };
            self.documents
                .borrow_mut()
                .edit(&self.key, |buffer| buffer.select(start, end));
            cx.notify();
            return;
        }
        let Some((text, needle)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.text()[primary.clone()].to_owned(),
            )
        }) else {
            return;
        };
        if needle.is_empty() {
            return;
        }
        let ranges = self.selection_ranges();
        let Some(range) = previous_occurrence(&text, &ranges, &needle) else {
            return;
        };
        if !ranges.contains(&range) {
            self.extra_cursors.push(range);
            self.extra_cursors.sort_by_key(|range| range.start);
            cx.notify();
        }
    }

    fn move_selection_to_next_find_match(&mut self, cx: &mut Context<Self>) {
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        if primary.is_empty() {
            let Some((start, end)) = self
                .read(|buffer| {
                    let caret = buffer.caret();
                    buffer
                        .text()
                        .unicode_word_indices()
                        .find(|(start, word)| *start <= caret && caret <= start + word.len())
                        .map(|(start, word)| (start, start + word.len()))
                })
                .flatten()
            else {
                return;
            };
            self.documents
                .borrow_mut()
                .edit(&self.key, |buffer| buffer.select(start, end));
            cx.notify();
            return;
        }
        let Some((text, needle)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.text()[primary.clone()].to_owned(),
            )
        }) else {
            return;
        };
        let ranges = self.selection_ranges();
        let Some(moving) = ranges.iter().max_by_key(|range| range.start).cloned() else {
            return;
        };
        let Some(next) = next_occurrence_after(&text, &ranges, &needle) else {
            return;
        };
        let mut remaining = ranges
            .iter()
            .filter(|range| **range != moving)
            .cloned()
            .collect::<Vec<_>>();
        remaining.push(next.clone());
        remaining.sort_by_key(|range| range.start);
        if moving == primary {
            self.edit(
                |buffer| buffer.select(next.start, next.end),
                cx,
            );
            self.extra_cursors = remaining
                .into_iter()
                .filter(|range| *range != next)
                .collect();
        } else {
            self.extra_cursors = remaining
                .into_iter()
                .filter(|range| *range != primary)
                .collect();
            cx.notify();
        }
    }

    fn move_selection_to_previous_find_match(&mut self, cx: &mut Context<Self>) {
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        if primary.is_empty() {
            let Some((start, end)) = self
                .read(|buffer| {
                    let caret = buffer.caret();
                    buffer
                        .text()
                        .unicode_word_indices()
                        .find(|(start, word)| *start <= caret && caret <= start + word.len())
                        .map(|(start, word)| (start, start + word.len()))
                })
                .flatten()
            else {
                return;
            };
            self.documents
                .borrow_mut()
                .edit(&self.key, |buffer| buffer.select(start, end));
            cx.notify();
            return;
        }
        let Some((text, needle)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.text()[primary.clone()].to_owned(),
            )
        }) else {
            return;
        };
        let ranges = self.selection_ranges();
        let Some(moving) = ranges.iter().min_by_key(|range| range.start).cloned() else {
            return;
        };
        let Some(previous) = previous_occurrence(&text, &ranges, &needle) else {
            return;
        };
        let mut remaining = ranges
            .iter()
            .filter(|range| **range != moving)
            .cloned()
            .collect::<Vec<_>>();
        remaining.push(previous.clone());
        remaining.sort_by_key(|range| range.start);
        if moving == primary {
            self.edit(
                |buffer| buffer.select(previous.start, previous.end),
                cx,
            );
            self.extra_cursors = remaining
                .into_iter()
                .filter(|range| *range != previous)
                .collect();
        } else {
            self.extra_cursors = remaining
                .into_iter()
                .filter(|range| *range != primary)
                .collect();
            cx.notify();
        }
    }

    fn multi_replace(&mut self, text: &str, cx: &mut Context<Self>) {
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        let ranges = self.selection_ranges();
        if ranges.len() < 2 {
            return;
        }
        let primary_index = ranges
            .iter()
            .position(|range| *range == primary)
            .unwrap_or(0);
        let mut positions = Vec::new();
        self.documents.borrow_mut().edit(&self.key, |buffer| {
            positions = buffer.replace_many(&ranges, text, primary_index);
        });
        self.extra_cursors = positions
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != primary_index)
            .map(|(_, position)| *position..*position)
            .collect();
        self.extra_cursors.sort_by_key(|range| range.start);
        cx.notify();
    }

    fn multi_erase(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            return;
        };
        let primary = self.read(|buffer| buffer.selection()).unwrap_or_default();
        let mut ranges = Vec::new();
        for range in self.selection_ranges() {
            if !range.is_empty() {
                ranges.push(range);
                continue;
            }
            let offset = range.start;
            if forward {
                let end = text
                    .get(offset..)
                    .and_then(|tail| tail.graphemes(true).next())
                    .map(|grapheme| offset + grapheme.len())
                    .unwrap_or(offset);
                if end > offset {
                    ranges.push(offset..end);
                }
            } else {
                let start = text
                    .get(..offset)
                    .and_then(|head| head.graphemes(true).next_back())
                    .map(|grapheme| offset - grapheme.len())
                    .unwrap_or(offset);
                if start < offset {
                    ranges.push(start..offset);
                }
            }
        }
        if ranges.is_empty() {
            return;
        }
        ranges.sort_by_key(|range| range.start);
        let mut unique = Vec::new();
        let mut last_end = 0;
        for range in ranges {
            if range.start >= last_end {
                last_end = range.end;
                unique.push(range);
            }
        }
        let primary_index = unique
            .iter()
            .position(|range| *range == primary || range.contains(&primary.start))
            .unwrap_or(0);
        let mut positions = Vec::new();
        self.documents.borrow_mut().edit(&self.key, |buffer| {
            positions = buffer.replace_many(&unique, "", primary_index);
        });
        self.extra_cursors = positions
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != primary_index)
            .map(|(_, position)| *position..*position)
            .collect();
        self.extra_cursors.sort_by_key(|range| range.start);
        cx.notify();
    }
    fn index_at(&self, position: Point<Pixels>) -> usize {
        let Some(line) = self
            .painted
            .values()
            .find(|l| position.y < l.origin.y + l.layout.size(px(LINE_HEIGHT)).height)
            .or_else(|| self.painted.values().next_back())
        else {
            return 0;
        };
        let local = point(
            (position.x - line.origin.x).max(px(0.)),
            (position.y - line.origin.y).max(px(0.)),
        );
        let index = line
            .layout
            .closest_index_for_position(local, px(LINE_HEIGHT))
            .unwrap_or_else(|i| i);
        line.source_index(index).min(line.end)
    }
    fn position_at(&self, index: usize) -> Option<Point<Pixels>> {
        let line = self
            .painted
            .values()
            .find(|line| index >= line.start && index <= line.end)?;
        line.layout
            .position_for_index(line.display_index(index), px(LINE_HEIGHT))
            .map(|p| line.origin + p)
    }
    fn menu_key(&mut self, key: &str, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
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
    fn close_completion(&mut self, cx: &mut Context<Self>) {
        if self.completion_open {
            self.completion_open = false;
            self.completion_items.clear();
            self.completion_selected = 0;
            cx.notify();
        }
    }

    fn open_completion(&mut self, cx: &mut Context<Self>) {
        if !self.extra_cursors.is_empty() {
            return;
        }
        let Some((text, caret, marked)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.caret(),
                buffer.marked.is_some(),
            )
        }) else {
            return;
        };
        if marked {
            return;
        }
        let range = crate::completion::word_range_at_lang(&text, caret, &self.key.rel);
        let prefix = text[range.start..caret].to_owned();
        let current = text[range.clone()].to_owned();
        let mut items = crate::completion::completions_at(
            &text,
            caret,
            &prefix,
            crate::completion::MAX_COMPLETIONS,
            &self.key.rel,
        );
        if prefix == current {
            items.retain(|item| item != &current);
        }
        if items.is_empty() {
            self.close_completion(cx);
            return;
        }
        self.completion_range = range;
        self.completion_prefix = prefix;
        self.completion_items = items;
        self.completion_selected = 0;
        self.completion_open = true;
        cx.notify();
    }

    fn schedule_quick_suggestion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.completion_open {
            return;
        }
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            return;
        };
        let Some(caret) = self.read(|buffer| buffer.caret()) else {
            return;
        };
        let Some(typed) = text[..caret].chars().next_back() else {
            return;
        };
        if !quick_suggestion_trigger(&self.key.rel, typed)
            || quick_suggestion_in_comment(&text, caret)
        {
            return;
        }
        let generation = self.completion_generation;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(25))
                .await;
            let _ = this.update(cx, |editor, cx| {
                if editor.completion_generation == generation && !editor.completion_open {
                    editor.open_completion(cx);
                }
            });
        })
        .detach();
    }

    fn refresh_completion(&mut self, cx: &mut Context<Self>) {
        if !self.completion_open {
            return;
        }
        let Some((text, caret, marked)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.caret(),
                buffer.marked.is_some(),
            )
        }) else {
            self.close_completion(cx);
            return;
        };
        if marked {
            self.close_completion(cx);
            return;
        }
        let range = crate::completion::word_range_at_lang(&text, caret, &self.key.rel);
        if caret < range.start || caret > range.end {
            self.close_completion(cx);
            return;
        }
        let prefix = text[range.start..caret].to_owned();
        let current = text[range.clone()].to_owned();
        let mut items = crate::completion::completions_at(
            &text,
            caret,
            &prefix,
            crate::completion::MAX_COMPLETIONS,
            &self.key.rel,
        );
        if prefix == current {
            items.retain(|item| item != &current);
        }
        if items.is_empty() {
            self.close_completion(cx);
            return;
        }
        self.completion_range = range;
        self.completion_prefix = prefix;
        self.completion_items = items;
        self.completion_selected = self
            .completion_selected
            .min(self.completion_items.len().saturating_sub(1));
        cx.notify();
    }

    fn move_completion(&mut self, delta: isize, cx: &mut Context<Self>) {
        if !self.completion_open || self.completion_items.is_empty() {
            return;
        }
        let len = self.completion_items.len() as isize;
        self.completion_selected =
            ((self.completion_selected as isize + delta).rem_euclid(len)) as usize;
        cx.notify();
    }

    fn accept_completion(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.completion_open {
            return false;
        }
        let Some(item) = self.completion_items.get(self.completion_selected).cloned() else {
            self.close_completion(cx);
            return false;
        };
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            self.close_completion(cx);
            return false;
        };
        let range = self.completion_range.clone();
        if range.end > text.len()
            || text[range.start..range.start + self.completion_prefix.len()]
                != self.completion_prefix
        {
            self.close_completion(cx);
            return false;
        }
        let caret = range.start + item.len();
        self.edit(
            move |buffer| {
                buffer.replace(range, &item);
                buffer.select(caret, caret);
            },
            cx,
        );
        self.close_completion(cx);
        true
    }

    fn close_symbols(&mut self, cx: &mut Context<Self>) {
        if self.symbols_open {
            self.symbols_open = false;
            self.symbols.clear();
            self.symbols_all.clear();
            self.symbols_filter_active = false;
            self.symbol_selected = 0;
            cx.notify();
        }
    }

    fn refresh_symbol_filter(&mut self, cx: &App) {
        if !self.symbols_filter_active {
            return;
        }
        let query = self.symbols_filter.read(cx).buffer.text();
        self.symbols = filter_symbols(&self.symbols_all, query);
        self.symbol_selected = 0;
    }

    fn open_document_symbols(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_completion(cx);
        self.close_workspace_symbols(cx);
        let Some(text) = self.read(|buffer| buffer.text().to_owned()) else {
            return;
        };
        self.symbols_all = crate::symbols::declarations(&text)
            .into_iter()
            .map(|declaration| declaration.location)
            .collect();
        self.symbols = self.symbols_all.clone();
        self.symbols_title = "Symbols".into();
        self.symbols_filter_active = true;
        self.symbol_selected = 0;
        self.symbols_open = true;
        self.symbols_filter.update(cx, |input, cx| {
            input.set_text(String::new(), cx);
            input.buffer.select(0, 0);
        });
        window.focus(&self.symbols_filter.focus_handle(cx), cx);
        cx.notify();
    }

    fn jump_to_symbol(&mut self, location: crate::symbols::SymbolLocation, cx: &mut Context<Self>) {
        let offset = location.offset;
        self.edit(move |buffer| buffer.select(offset, offset), cx);
        self.close_symbols(cx);
    }

    fn query_symbols(&mut self, references: bool, cx: &mut Context<Self>) {
        self.close_completion(cx);
        let Some((text, caret)) = self.read(|buffer| (buffer.text().to_owned(), buffer.caret()))
        else {
            return;
        };
        let Some((_, word)) = crate::symbols::word_at(&text, caret) else {
            return;
        };
        if references {
            let files = self.workspace_files();
            let found = crate::symbols::workspace_references(&files, &word, 100);
            self.close_symbols(cx);
            self.workspace_symbols_title = format!("References for {word}");
            self.workspace_symbols_references = true;
            self.workspace_symbols_query = word;
            self.workspace_symbols = found;
            self.workspace_symbols_filter_active = false;
            self.workspace_symbol_selected = 0;
            self.workspace_symbols_open = true;
            cx.notify();
            return;
        }
        let locations = crate::symbols::definitions(&text, &word);
        if locations.is_empty() {
            let matches = crate::symbols::workspace_definitions(&self.workspace_files(), &word);
            if matches.len() == 1 {
                self.close_symbols(cx);
                self.close_workspace_symbols(cx);
                cx.emit(OpenSymbol {
                    key: matches[0].key.clone(),
                    offset: matches[0].location.offset,
                });
                return;
            }
            if matches.len() > 1 {
                self.close_symbols(cx);
                self.workspace_symbols = matches;
                self.workspace_symbols_query = word.clone();
                self.workspace_symbols_title.clear();
                self.workspace_symbols_references = false;
                self.workspace_symbols_filter_active = false;
                self.workspace_symbol_selected = 0;
                self.workspace_symbols_open = true;
                cx.notify();
                return;
            }
        }
        if !references && locations.len() == 1 {
            self.jump_to_symbol(locations[0].clone(), cx);
            return;
        }
        self.close_workspace_symbols(cx);
        self.symbols_title = format!(
            "{} for {word}",
            if references {
                "References"
            } else {
                "Definitions"
            }
        );
        self.symbols = locations;
        self.symbols_all.clear();
        self.symbols_filter_active = false;
        self.symbol_selected = 0;
        self.symbols_open = true;
        cx.notify();
    }

    fn close_code_actions(&mut self, cx: &mut Context<Self>) {
        if self.code_actions_open {
            self.code_actions_open = false;
            self.code_actions.clear();
            self.code_action_selected = 0;
            cx.notify();
        }
    }

    fn open_code_actions(&mut self, cx: &mut Context<Self>) {
        self.close_completion(cx);
        self.close_symbols(cx);
        let Some((text, caret)) = self.read(|buffer| (buffer.text().to_owned(), buffer.caret()))
        else {
            return;
        };
        let line = text[..caret.min(text.len())]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let files = self.workspace_files();
        let actions = crate::code_actions::actions_with_files(
            &self.key,
            &text,
            &self.diagnostics,
            line,
            &files,
        );
        if actions.is_empty() {
            self.close_code_actions(cx);
            return;
        }
        self.code_actions = actions;
        self.code_action_selected = 0;
        self.code_actions_open = true;
        cx.notify();
    }

    fn apply_code_action(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(action) = self.code_actions.get(index).cloned() else {
            return;
        };
        self.code_actions_open = false;
        self.code_actions.clear();
        self.code_action_selected = 0;
        self.edit(
            move |buffer| buffer.replace(action.range, &action.replacement),
            cx,
        );
    }

    fn render_code_actions(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let mut menu = div()
            .id("editor-code-actions")
            .debug_selector(|| "editor-code-actions".into())
            .role(Role::Menu)
            .aria_label("Quick Fixes")
            .absolute()
            .left(px(12.))
            .top(px(12.))
            .w(px(300.))
            .p(px(5.))
            .rounded(px(7.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.22).into(),
            }])
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .px(px(7.))
                    .py(px(3.))
                    .text_size(px(9.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.faint)
                    .child("QUICK FIXES"),
            );
        for (index, action) in self.code_actions.iter().enumerate() {
            let selected = index == self.code_action_selected;
            let id = format!("editor-code-action-{index}");
            let selector = id.clone();
            let title = action.title.clone();
            menu = menu.child(
                div()
                    .id(SharedString::from(id))
                    .debug_selector(move || selector.clone())
                    .role(Role::ListBoxOption)
                    .aria_label(SharedString::from(title.clone()))
                    .aria_selected(selected)
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .text_size(px(11.))
                    .text_color(if selected { theme.text } else { theme.dim })
                    .when(selected, |row| row.bg(theme.selected))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset).text_color(theme.text))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _, _, cx| {
                            editor.apply_code_action(index, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(SharedString::from(title)),
            );
        }
        menu.into_any_element()
    }

    fn open_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_completion(cx);
        self.close_symbols(cx);
        self.close_workspace_symbols(cx);
        self.close_code_actions(cx);
        let Some((text, caret)) = self.read(|buffer| (buffer.text().to_owned(), buffer.caret()))
        else {
            return;
        };
        let Some((_, word)) = crate::symbols::word_at(&text, caret) else {
            return;
        };
        let files = self.workspace_files();
        let (targets, error) = match crate::rename::occurrences(&files, &word) {
            Ok(targets) => (targets, None),
            Err(crate::rename::RenameError::NoSymbol) => return,
            Err(error) => (Vec::new(), Some(error.message())),
        };
        self.rename_open = true;
        self.rename_original = word.clone();
        self.rename_anchor = caret;
        self.rename_targets = targets;
        self.rename_error = error;
        self.rename_input
            .update(cx, |input, cx| input.set_text(word, cx));
        self.rename_input
            .update(cx, |input, _| input.buffer.select_all());
        window.focus(&self.rename_input.focus_handle(cx), cx);
        cx.notify();
    }

    fn close_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.rename_open {
            self.rename_open = false;
            self.rename_targets.clear();
            self.rename_original.clear();
            self.rename_error = None;
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.rename_open {
            return;
        }
        let new_name = self.rename_input.read(cx).buffer.text().to_owned();
        let files = self.workspace_files();
        let targets = match crate::rename::plan(&files, &self.rename_original, &new_name) {
            Ok(targets) => targets,
            Err(error) => {
                self.rename_error = Some(error.message());
                cx.notify();
                return;
            }
        };
        let active_key = self.key.clone();
        for (key, ranges) in &targets {
            if *key != active_key {
                self.documents.borrow_mut().edit(key, |buffer| {
                    buffer.replace_many(ranges, &new_name, 0);
                });
            }
        }
        let active_ranges = targets
            .iter()
            .find(|(key, _)| *key == active_key)
            .map(|(_, ranges)| ranges.clone())
            .unwrap_or_default();
        if !active_ranges.is_empty() {
            let primary = active_ranges
                .iter()
                .position(|range| range.start >= self.rename_anchor)
                .unwrap_or(active_ranges.len() - 1);
            let name = new_name.clone();
            self.edit(
                move |buffer| {
                    let positions = buffer.replace_many(&active_ranges, &name, primary);
                    let caret = positions[primary];
                    buffer.select(caret, caret);
                },
                cx,
            );
        }
        self.rename_open = false;
        self.rename_targets.clear();
        self.rename_original.clear();
        self.rename_error = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn render_rename(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let occurrences = self
            .rename_targets
            .iter()
            .map(|(_, ranges)| ranges.len())
            .sum::<usize>();
        let files = self.rename_targets.len();
        let summary = self.rename_error.clone().unwrap_or_else(|| {
            format!(
                "{occurrences} occurrence{} in {files} document{}",
                if occurrences == 1 { "" } else { "s" },
                if files == 1 { "" } else { "s" }
            )
        });
        let summary_color = if self.rename_error.is_some() {
            theme.red
        } else {
            theme.faint
        };
        let input = self.rename_input.clone();
        div()
            .id("editor-rename")
            .debug_selector(|| "editor-rename".into())
            .role(Role::Group)
            .aria_label(SharedString::from(format!(
                "Rename {}",
                self.rename_original
            )))
            .absolute()
            .left(px(12.))
            .top(px(12.))
            .w(px(420.))
            .p(px(8.))
            .rounded(px(7.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.elevated)
            .flex()
            .flex_col()
            .gap(px(6.))
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.22).into(),
            }])
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.rename_input.read(cx).buffer.marked.is_some() {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "enter" => {
                        this.commit_rename(window, cx);
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.close_rename(window, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.dim)
                            .child(SharedString::from(format!(
                                "Rename {}",
                                self.rename_original
                            ))),
                    )
                    .child(
                        div()
                            .id("editor-rename-cancel")
                            .debug_selector(|| "editor-rename-cancel".into())
                            .role(Role::Button)
                            .aria_label("Cancel rename")
                            .px(px(6.))
                            .py(px(2.))
                            .rounded(px(4.))
                            .text_size(px(11.))
                            .text_color(theme.faint)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.selected).text_color(theme.text))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.close_rename(window, cx)),
                            )
                            .child("×"),
                    ),
            )
            .child(
                div()
                    .id("editor-rename-input")
                    .debug_selector(|| "editor-rename-input".into())
                    .p(px(4.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.inset)
                    .child(input),
            )
            .child(
                div()
                    .id("editor-rename-summary")
                    .debug_selector(|| "editor-rename-summary".into())
                    .text_size(px(9.5))
                    .text_color(summary_color)
                    .whitespace_normal()
                    .child(SharedString::from(summary)),
            )
            .into_any_element()
    }

    pub fn set_validation_diagnostics(
        &mut self,
        diagnostics: Vec<crate::diagnostics::Diagnostic>,
        cx: &mut Context<Self>,
    ) {
        let revision = self
            .documents
            .borrow()
            .get(&self.key)
            .map(|document| document.buffer.revision());
        self.validation_diagnostics = diagnostics;
        self.validation_revision = revision;
        self.diagnostics
            .retain(|diagnostic| diagnostic.kind != crate::diagnostics::DiagnosticKind::Validation);
        if self.validation_revision == revision {
            self.diagnostics
                .extend(self.validation_diagnostics.iter().cloned());
        }
        cx.notify();
    }

    pub fn select_offset(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.edit(move |buffer| buffer.select(offset, offset), cx);
    }

    fn close_workspace_symbols(&mut self, cx: &mut Context<Self>) {
        if self.workspace_symbols_open {
            self.workspace_symbols_open = false;
            self.workspace_symbols.clear();
            self.workspace_symbols_title.clear();
            self.workspace_symbols_references = false;
            self.workspace_symbols_filter_active = false;
            self.workspace_symbol_selected = 0;
            cx.notify();
        }
    }

    fn workspace_files(&self) -> Vec<(DocumentKey, String)> {
        let documents = self.documents.borrow();
        documents
            .tabs(&self.key.workspace)
            .iter()
            .filter_map(|tab| {
                documents
                    .get(&tab.key)
                    .map(|document| (tab.key.clone(), document.buffer.text().to_owned()))
            })
            .collect()
    }

    fn refresh_workspace_symbols_filter(&mut self, cx: &App) {
        if !self.workspace_symbols_filter_active {
            return;
        }
        let query = self.workspace_symbols_filter.read(cx).buffer.text();
        let files = self.workspace_files();
        self.workspace_symbols = crate::symbols::workspace_symbols(&files, query, 100);
        self.workspace_symbols_query = query.to_owned();
        self.workspace_symbol_selected = 0;
    }

    fn open_workspace_symbols(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_completion(cx);
        self.close_symbols(cx);
        let query = self
            .read(|buffer| crate::symbols::word_at(buffer.text(), buffer.caret()))
            .flatten()
            .map(|(_, word)| word)
            .unwrap_or_default();
        let files = self.workspace_files();
        self.workspace_symbols = crate::symbols::workspace_symbols(&files, &query, 100);
        self.workspace_symbols_query = query.clone();
        self.workspace_symbols_title.clear();
        self.workspace_symbols_references = false;
        self.workspace_symbols_filter_active = true;
        self.workspace_symbol_selected = 0;
        self.workspace_symbols_open = true;
        self.workspace_symbols_filter.update(cx, |input, cx| {
            input.set_text(query, cx);
            input.buffer.select_all();
        });
        window.focus(&self.workspace_symbols_filter.focus_handle(cx), cx);
        cx.notify();
    }

    fn jump_workspace_symbol(&mut self, cx: &mut Context<Self>) {
        let Some(symbol) = self
            .workspace_symbols
            .get(self.workspace_symbol_selected)
            .cloned()
        else {
            return;
        };
        self.close_workspace_symbols(cx);
        cx.emit(OpenSymbol {
            key: symbol.key,
            offset: symbol.location.offset,
        });
    }

    fn render_workspace_symbols(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let title = if !self.workspace_symbols_title.is_empty() {
            self.workspace_symbols_title.clone()
        } else if self.workspace_symbols_query.is_empty() {
            "Workspace symbols".to_owned()
        } else {
            format!("Workspace symbols · {}", self.workspace_symbols_query)
        };
        let mut panel = div()
            .id("editor-workspace-symbols")
            .debug_selector(|| "editor-workspace-symbols".into())
            .role(Role::ListBox)
            .aria_label(SharedString::from(title.clone()))
            .absolute()
            .left(px(12.))
            .right(px(12.))
            .top(px(12.))
            .max_h(px(260.))
            .overflow_hidden()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .p(px(5.))
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.22).into(),
            }])
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(7.))
                    .py(px(3.))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.dim)
                            .child(SharedString::from(title)),
                    )
                    .child(
                        div()
                            .id("editor-workspace-symbols-close")
                            .role(Role::Button)
                            .aria_label("Close workspace symbols")
                            .size(px(16.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.))
                            .text_size(px(12.))
                            .text_color(theme.faint)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.selected).text_color(theme.text))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|editor, _, _, cx| {
                                    editor.close_workspace_symbols(cx);
                                    cx.stop_propagation();
                                }),
                            )
                            .child("×"),
                    ),
            );
        if self.workspace_symbols_filter_active {
            panel = panel.child(self.workspace_symbols_filter.clone());
        }
        if self.workspace_symbols.is_empty() {
            let empty = if self.workspace_symbols_references {
                "No references in open documents."
            } else {
                "No workspace symbols in open documents."
            };
            panel = panel.child(
                div()
                    .id("editor-workspace-symbols-empty")
                    .debug_selector(|| "editor-workspace-symbols-empty".into())
                    .px(px(8.))
                    .py(px(8.))
                    .text_size(px(11.))
                    .text_color(theme.faint)
                    .child(empty),
            );
        }
        for (index, symbol) in self.workspace_symbols.iter().enumerate() {
            let selected = index == self.workspace_symbol_selected;
            let id = format!("editor-workspace-symbol-{index}");
            let selector = id.clone();
            panel = panel.child(
                div()
                    .id(SharedString::from(id))
                    .debug_selector(move || selector.clone())
                    .role(Role::ListBoxOption)
                    .aria_label(format!(
                        "{} {}:{}",
                        symbol.name, symbol.key.rel, symbol.location.line
                    ))
                    .aria_selected(selected)
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .text_size(px(11.))
                    .text_color(if selected { theme.text } else { theme.dim })
                    .when(selected, |row| row.bg(theme.selected))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset).text_color(theme.text))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _, _, cx| {
                            editor.workspace_symbol_selected = index;
                            editor.jump_workspace_symbol(cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(SharedString::from(if self.workspace_symbols_references {
                        format!(
                            "{}:{}  {}",
                            symbol.key.rel, symbol.location.line, symbol.location.preview
                        )
                    } else {
                        format!(
                            "{}  {}:{}",
                            symbol.name, symbol.key.rel, symbol.location.line
                        )
                    })),
            );
        }
        panel.into_any_element()
    }

    fn render_symbols(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let mut panel = div()
            .id("editor-symbol-panel")
            .debug_selector(|| "editor-symbol-panel".into())
            .role(Role::ListBox)
            .aria_label(SharedString::from(self.symbols_title.clone()))
            .absolute()
            .left(px(12.))
            .top(px(12.))
            .w(px(380.))
            .max_h(px(230.))
            .overflow_hidden()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .p(px(5.))
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.22).into(),
            }])
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(7.))
                    .py(px(3.))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.dim)
                            .child(SharedString::from(self.symbols_title.clone())),
                    )
                    .child(
                        div()
                            .id("editor-symbols-close")
                            .role(Role::Button)
                            .aria_label("Close symbol results")
                            .size(px(16.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.))
                            .text_size(px(12.))
                            .text_color(theme.faint)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.selected).text_color(theme.text))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|editor, _, _, cx| {
                                    editor.close_symbols(cx);
                                    cx.stop_propagation();
                                }),
                            )
                            .child("×"),
                    ),
            );
        if self.symbols_filter_active {
            panel = panel.child(self.symbols_filter.clone());
        }
        if self.symbols.is_empty() {
            panel = panel.child(
                div()
                    .id("editor-symbols-empty")
                    .debug_selector(|| "editor-symbols-empty".into())
                    .px(px(8.))
                    .py(px(8.))
                    .text_size(px(11.))
                    .text_color(theme.faint)
                    .child("No matching symbols in this document."),
            );
        }
        for (index, location) in self.symbols.iter().enumerate() {
            let selected = index == self.symbol_selected;
            let id = format!("editor-symbol-{index}");
            let selector = id.clone();
            let value = location.clone();
            panel = panel.child(
                div()
                    .id(SharedString::from(id))
                    .debug_selector(move || selector.clone())
                    .role(Role::ListBoxOption)
                    .aria_label(format!(
                        "{}:{} {}",
                        location.line, location.column, location.preview
                    ))
                    .aria_selected(selected)
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .text_size(px(11.))
                    .text_color(if selected { theme.text } else { theme.dim })
                    .when(selected, |row| row.bg(theme.selected))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset).text_color(theme.text))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _, _, cx| {
                            editor.symbol_selected = index;
                            editor.jump_to_symbol(value.clone(), cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(SharedString::from(format!(
                        "{}:{}  {}",
                        location.line, location.column, location.preview
                    ))),
            );
        }
        panel.into_any_element()
    }

    fn render_completion(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let caret = self.read(|buffer| buffer.caret()).unwrap_or(0);
        let anchor = self.position_at(caret).zip(self.bounds).map(|(position, bounds)| {
            let local = position - bounds.origin;
            (f32::from(local.x), f32::from(local.y), bounds)
        });
        let (left, top) = anchor.map_or((12., None), |(x, y, bounds)| {
            let height = 190.;
            let top = if y + LINE_HEIGHT + 2. + height <= f32::from(bounds.size.height) {
                y + LINE_HEIGHT + 2.
            } else {
                (y - height - 2.).max(8.)
            };
            (x.max(8.).min((f32::from(bounds.size.width) - 232.).max(8.)), Some(top))
        });
        let mut menu = div()
            .id("editor-completion")
            .debug_selector(|| "editor-completion".into())
            .role(Role::ListBox)
            .aria_label("Completions")
            .absolute()
            .left(px(left))
            .when_some(top, |menu, top| menu.top(px(top)))
            .when(top.is_none(), |menu| {
                menu.bottom(px(if self.diagnostics.is_empty() {
                    12.
                } else {
                    48.
                }))
            })
            .w(px(224.))
            .max_h(px(190.))
            .overflow_hidden()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .p(px(4.))
            .shadow(vec![gpui::BoxShadow {
                inset: false,
                offset: point(px(0.), px(4.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
                color: theme::rgba(0, 0, 0, 0.22).into(),
            }])
            .flex()
            .flex_col()
            .gap(px(1.));
        for (index, item) in self.completion_items.iter().enumerate() {
            let selected = index == self.completion_selected;
            let id = format!("editor-completion-{index}");
            let selector = id.clone();
            let value = item.clone();
            menu = menu.child(
                div()
                    .id(SharedString::from(id))
                    .debug_selector(move || selector.clone())
                    .role(Role::ListBoxOption)
                    .aria_label(value.clone())
                    .aria_selected(selected)
                    .px(px(8.))
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .rounded(px(4.))
                    .text_size(px(13.))
                    .text_color(if selected { theme.text } else { theme.dim })
                    .when(selected, |row| row.bg(theme.selected))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset).text_color(theme.text))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _, _, cx| {
                            editor.completion_selected = index;
                            editor.accept_completion(cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(SharedString::from(value)),
            );
        }
        menu.into_any_element()
    }

    fn try_emmet_expansion(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.extra_cursors.is_empty() {
            return false;
        }
        let Some((text, selection, marked)) = self.read(|buffer| {
            (
                buffer.text().to_owned(),
                buffer.selection(),
                buffer.marked.is_some(),
            )
        }) else {
            return false;
        };
        if marked || !selection.is_empty() {
            return false;
        }
        let language = emmet_language_for(&self.key.rel);
        if language == "plaintext" {
            return false;
        }
        if language == "html" && inside_script_tag(&text, selection.start) {
            return false;
        }
        let line_start = text[..selection.start]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let Some(expansion) =
            crate::emmet::emmet_expansion_at(&text[line_start..selection.start], language)
        else {
            return false;
        };
        let insertion = crate::emmet::emmet_insertion(&expansion.snippet);
        let range_start = line_start + expansion.start;
        let range_end = selection.start;
        let caret = range_start + insertion.caret;
        let inserted = insertion.text;
        self.edit(
            move |buffer| {
                buffer.replace(range_start..range_end, &inserted);
                buffer.select(caret, caret);
            },
            cx,
        );
        true
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.completion_generation = self.completion_generation.wrapping_add(1);
        if self.hover_popup.take().is_some() {
            cx.notify();
        }
        let key = &event.keystroke;
        let command = key.modifiers.platform || key.modifiers.control;
        let shift = key.modifiers.shift;
        let alt = key.modifiers.alt;
        if key.key == "f1" {
            self.open_palette(window, cx);
            cx.stop_propagation();
            return;
        }
        if cfg!(target_os = "macos")
            && key.key == "k"
            && key.modifiers.control
            && !key.modifiers.platform
            && !shift
            && !alt
        {
            self.delete_all(false, cx);
            cx.stop_propagation();
            return;
        }
        if cfg!(target_os = "macos")
            && key.key == "backspace"
            && key.modifiers.platform
            && !key.modifiers.control
            && !shift
            && alt
        {
            self.remove_brackets(cx);
            cx.stop_propagation();
            return;
        }
        if cfg!(target_os = "macos")
            && key.key == "backspace"
            && key.modifiers.platform
            && !key.modifiers.control
            && !shift
            && !alt
        {
            self.delete_all(true, cx);
            cx.stop_propagation();
            return;
        }
        if cfg!(target_os = "macos")
            && key.key == "delete"
            && key.modifiers.platform
            && !key.modifiers.control
            && !shift
            && !alt
        {
            self.delete_all(false, cx);
            cx.stop_propagation();
            return;
        }
        if command && !shift && !alt && key.key == "k" {
            self.k_chord = true;
            cx.stop_propagation();
            return;
        }
        if self.k_chord {
            self.k_chord = false;
            if command && !shift && !alt && key.key == "f" {
                self.format_selection(cx);
                cx.stop_propagation();
                return;
            }
            if command && !shift && !alt && key.key == "x" {
                self.trim_trailing_whitespace(cx);
                cx.stop_propagation();
                return;
            }
            if command && !shift && !alt && key.key == "l" {
                self.toggle_fold_at_selection(false, cx);
                cx.stop_propagation();
                return;
            }
            if command && !shift && !alt && key.key == "d" {
                self.move_selection_to_next_find_match(cx);
                cx.stop_propagation();
                return;
            }
            if command && shift && !alt && key.key == "l" {
                self.toggle_fold_at_selection(true, cx);
                cx.stop_propagation();
                return;
            }
            if command
                && !shift
                && !alt
                && matches!(key.key.as_str(), "0" | "digit0")
            {
                self.fold_all(cx);
                cx.stop_propagation();
                return;
            }
            if command && !shift && !alt && key.key == "j" {
                self.unfold_all(cx);
                cx.stop_propagation();
                return;
            }
            if command
                && !shift
                && !alt
                && matches!(key.key.as_str(), "/" | "slash")
            {
                self.fold_all_block_comments(cx);
                cx.stop_propagation();
                return;
            }
            if command
                && !shift
                && !alt
                && matches!(key.key.as_str(), "8" | "digit8")
            {
                self.set_all_regions(true, cx);
                cx.stop_propagation();
                return;
            }
            if command
                && !shift
                && !alt
                && matches!(key.key.as_str(), "9" | "digit9")
            {
                self.set_all_regions(false, cx);
                cx.stop_propagation();
                return;
            }
            if command
                && !shift
                && !alt
                && matches!(key.key.as_str(), "-" | "minus")
            {
                self.set_all_except_selection(true, cx);
                cx.stop_propagation();
                return;
            }
            if command
                && !shift
                && !alt
                && matches!(key.key.as_str(), "=" | "equal")
            {
                self.set_all_except_selection(false, cx);
                cx.stop_propagation();
                return;
            }
            if command && !shift && !alt && matches!(key.key.as_str(), "[" | "bracketleft") {
                self.fold_at_selection(true, cx);
                cx.stop_propagation();
                return;
            }
            if command && !shift && !alt && matches!(key.key.as_str(), "]" | "bracketright") {
                self.unfold_at_selection(true, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "escape" {
                cx.stop_propagation();
                return;
            }
        }
        if command && alt && !shift && matches!(key.key.as_str(), "[" | "bracketleft") {
            self.fold_at_selection(false, cx);
            cx.stop_propagation();
            return;
        }
        if command && alt && !shift && matches!(key.key.as_str(), "]" | "bracketright") {
            self.unfold_at_selection(false, cx);
            cx.stop_propagation();
            return;
        }
        if alt && shift && key.key == "f" {
            self.format_document(cx);
            cx.stop_propagation();
            return;
        }
        if command && !shift && !alt && key.key == "/" {
            self.toggle_comment(false, cx);
            cx.stop_propagation();
            return;
        }
        if alt && shift && key.key == "a" {
            self.toggle_comment(true, cx);
            cx.stop_propagation();
            return;
        }
        if alt && shift && key.key == "i" {
            self.add_cursors_to_line_ends(cx);
            cx.stop_propagation();
            return;
        }
        if command && !shift && !alt && matches!(key.key.as_str(), "]" | "bracketright") {
            self.shift_lines(false, cx);
            cx.stop_propagation();
            return;
        }
        if command && !shift && !alt && matches!(key.key.as_str(), "[" | "bracketleft") {
            self.shift_lines(true, cx);
            cx.stop_propagation();
            return;
        }
        if command && shift && !alt && matches!(key.key.as_str(), "\\" | "backslash") {
            self.go_to_bracket(cx);
            cx.stop_propagation();
            return;
        }
        if command && shift && !alt && key.key == "k" {
            self.delete_selected_lines(cx);
            cx.stop_propagation();
            return;
        }
        if command && shift && !alt && key.key == "enter" {
            self.insert_line(true, cx);
            cx.stop_propagation();
            return;
        }
        if command && !shift && !alt && key.key == "enter" {
            self.insert_line(false, cx);
            cx.stop_propagation();
            return;
        }
        if command && key.key == "f" && !shift {
            self.open_find(key.modifiers.alt, window, cx);
            cx.stop_propagation();
            return;
        }
        if key.key == "g"
            && key.modifiers.control
            && !shift
            && !alt
            && (!cfg!(target_os = "macos") || !key.modifiers.platform)
        {
            self.open_goto_line(window, cx);
            cx.stop_propagation();
            return;
        }
        if cfg!(target_os = "macos")
            && key.key == "j"
            && key.modifiers.control
            && !key.modifiers.platform
            && !shift
            && !alt
        {
            self.join_lines(cx);
            cx.stop_propagation();
            return;
        }
        if alt && shift && !command && matches!(key.key.as_str(), "up" | "down") {
            self.copy_lines(key.key == "down", cx);
            cx.stop_propagation();
            return;
        }
        if alt && !command && !shift && matches!(key.key.as_str(), "up" | "down") {
            self.move_lines(key.key == "down", cx);
            cx.stop_propagation();
            return;
        }
        if key.key == "f8" && !command && !alt {
            self.go_to_problem(!shift, cx);
            cx.stop_propagation();
            return;
        }
        if (command && key.key == "g") || key.key == "f3" {
            if !self.find_open {
                self.open_find(false, window, cx);
            }
            if shift {
                self.previous_find(window, cx);
            } else {
                self.next_find(window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if self.find_open && key.key == "escape" {
            self.close_find(window, cx);
            cx.stop_propagation();
            return;
        }
        if command && key.key == "." {
            self.open_code_actions(cx);
            cx.stop_propagation();
            return;
        }
        if self.code_actions_open {
            match key.key.as_str() {
                "escape" => {
                    self.close_code_actions(cx);
                    cx.stop_propagation();
                    return;
                }
                "up" => {
                    if !self.code_actions.is_empty() {
                        self.code_action_selected =
                            (self.code_action_selected + self.code_actions.len() - 1)
                                % self.code_actions.len();
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "down" => {
                    if !self.code_actions.is_empty() {
                        self.code_action_selected =
                            (self.code_action_selected + 1) % self.code_actions.len();
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "enter" | "tab" if !shift => {
                    self.apply_code_action(self.code_action_selected, cx);
                    cx.stop_propagation();
                    return;
                }
                _ => self.close_code_actions(cx),
            }
        }
        if key.modifiers.control && key.key == "space" {
            self.open_completion(cx);
            cx.stop_propagation();
            return;
        }
        if self.completion_open {
            match key.key.as_str() {
                "escape" => {
                    self.close_completion(cx);
                    cx.stop_propagation();
                    return;
                }
                "up" => {
                    self.move_completion(-1, cx);
                    cx.stop_propagation();
                    return;
                }
                "down" => {
                    self.move_completion(1, cx);
                    cx.stop_propagation();
                    return;
                }
                "enter" | "tab" if !shift => {
                    self.accept_completion(cx);
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }
        if key.key == "f2" && !command {
            self.open_rename(window, cx);
            cx.stop_propagation();
            return;
        }
        if key.key == "f12" && !command {
            self.query_symbols(shift, cx);
            cx.stop_propagation();
            return;
        }
        if command && shift && !alt && key.key == "o" {
            self.open_document_symbols(window, cx);
            cx.stop_propagation();
            return;
        }
        if command && key.key == "t" && !shift {
            self.open_workspace_symbols(window, cx);
            cx.stop_propagation();
            return;
        }
        if self.workspace_symbols_open {
            match key.key.as_str() {
                "escape" => {
                    self.close_workspace_symbols(cx);
                    window.focus(&self.focus, cx);
                    cx.stop_propagation();
                    return;
                }
                "up" => {
                    if !self.workspace_symbols.is_empty() {
                        self.workspace_symbol_selected =
                            (self.workspace_symbol_selected + self.workspace_symbols.len() - 1)
                                % self.workspace_symbols.len();
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "down" => {
                    if !self.workspace_symbols.is_empty() {
                        self.workspace_symbol_selected =
                            (self.workspace_symbol_selected + 1) % self.workspace_symbols.len();
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "enter" | "tab" if !shift => {
                    self.jump_workspace_symbol(cx);
                    window.focus(&self.focus, cx);
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }
        if self.symbols_open {
            match key.key.as_str() {
                "escape" => {
                    self.close_symbols(cx);
                    window.focus(&self.focus, cx);
                    cx.stop_propagation();
                    return;
                }
                "up" => {
                    if !self.symbols.is_empty() {
                        self.symbol_selected =
                            (self.symbol_selected + self.symbols.len() - 1) % self.symbols.len();
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "down" => {
                    if !self.symbols.is_empty() {
                        self.symbol_selected = (self.symbol_selected + 1) % self.symbols.len();
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "enter" | "tab" if !shift => {
                    if let Some(location) = self.symbols.get(self.symbol_selected).cloned() {
                        self.jump_to_symbol(location, cx);
                        window.focus(&self.focus, cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }
        if key.key == "escape" && !self.extra_cursors.is_empty() {
            self.clear_multi_cursor();
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if command && !shift && !alt && key.key == "l" {
            self.expand_line_selection(cx);
            cx.stop_propagation();
            return;
        }
        if command && shift && !alt && key.key == "l" {
            self.select_all_occurrences(cx);
            cx.stop_propagation();
            return;
        }
        if command
            && alt
            && !shift
            && matches!(key.key.as_str(), "up" | "down")
        {
            self.add_cursor_line(key.key == "up", cx);
            cx.stop_propagation();
            return;
        }
        if command && key.key == "d" && !shift {
            self.add_next_occurrence(cx);
            cx.stop_propagation();
            return;
        }
        if matches!(key.key.as_str(), "backspace" | "delete") && !self.extra_cursors.is_empty() {
            self.multi_erase(key.key == "delete", cx);
            cx.stop_propagation();
            return;
        }
        if !command
            && !shift
            && matches!(key.key.as_str(), "tab" | "enter")
            && self.try_emmet_expansion(cx)
        {
            cx.stop_propagation();
            return;
        }
        if !command && !alt && key.key == "tab" {
            self.tab_indent(shift, cx);
            cx.stop_propagation();
            return;
        }
        if !command && !self.extra_cursors.is_empty() {
            match key.key.as_str() {
                "enter" => {
                    self.multi_replace("\n", cx);
                    cx.stop_propagation();
                    return;
                }
                "tab" => {
                    self.multi_replace("  ", cx);
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }
        if self.completion_open
            && matches!(
                key.key.as_str(),
                "left" | "right" | "home" | "end" | "pageup" | "pagedown"
            )
        {
            self.close_completion(cx);
        }
        if self.symbols_open
            && matches!(
                key.key.as_str(),
                "left" | "right" | "home" | "end" | "pageup" | "pagedown"
            )
        {
            self.close_symbols(cx);
        }
        if matches!(
            key.key.as_str(),
            "left" | "right" | "up" | "down" | "home" | "end" | "pageup" | "pagedown"
        ) || (command && matches!(key.key.as_str(), "a" | "v" | "x" | "z" | "y"))
        {
            self.clear_multi_cursor();
        }
        if command && key.key == "s" {
            cx.emit(SaveRequested);
            cx.stop_propagation();
            return;
        }
        match (key.key.as_str(), command) {
            ("a", true) => self.edit(|b| b.select_all(), cx),
            ("c" | "x", true) => {
                if let Some(text) = self
                    .read(|b| b.text()[b.selection()].to_owned())
                    .filter(|s| !s.is_empty())
                {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    if key.key == "x" {
                        self.edit(|b| b.insert(""), cx);
                    }
                }
            }
            ("v", true) => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.edit(|b| b.insert(&text), cx);
                }
            }
            ("z", true) if shift => self.edit(TextBuffer::redo, cx),
            ("z", true) => self.edit(TextBuffer::undo, cx),
            ("y", true) => self.edit(TextBuffer::redo, cx),
            ("left" | "right", _) => self.edit(
                |b| {
                    if key.modifiers.platform {
                        b.line_edge(key.key == "right", false, shift);
                    } else {
                        b.horizontal(
                            key.key == "right",
                            shift,
                            key.modifiers.alt || key.modifiers.control,
                        );
                    }
                },
                cx,
            ),
            ("up" | "down", _) => {
                let down = key.key == "down";
                if command {
                    self.edit(|b| b.line_edge(down, true, shift), cx);
                } else {
                    let pos = self
                        .read(|b| b.caret())
                        .and_then(|index| self.position_at(index));
                    if let Some(pos) = pos {
                        let point = point(
                            pos.x,
                            pos.y
                                + px(if down {
                                    LINE_HEIGHT * 1.5
                                } else {
                                    -LINE_HEIGHT * 0.5
                                }),
                        );
                        let target = self.index_at(point);
                        self.edit(|b| b.move_to(target, shift), cx);
                    } else {
                        self.edit(|b| b.vertical(down, shift), cx);
                    }
                }
            }
            ("home" | "end", _) => self.edit(|b| b.line_edge(key.key == "end", command, shift), cx),
            ("pageup" | "pagedown", _) => {
                let down = key.key == "pagedown";
                let height = self.bounds.map(|b| b.size.height).unwrap_or(px(400.));
                self.list.scroll_by(if down { height } else { -height });
                if let Some(pos) = self.read(|b| b.caret()).and_then(|i| self.position_at(i)) {
                    let target =
                        self.index_at(point(pos.x, pos.y + if down { height } else { -height }));
                    self.edit(|b| b.move_to(target, shift), cx);
                }
            }
            ("backspace" | "delete", _) => {
                let rel = self.key.rel.clone();
                self.edit(
                move |b| {
                    let forward = key.key == "delete";
                    if !forward
                        && !key.modifiers.platform
                        && !key.modifiers.alt
                        && !key.modifiers.control
                        && b.selection().is_empty()
                    {
                        let caret = b.selection().start;
                        let erase = {
                            let current = b.text();
                            auto_pair_erase(current, caret, auto_pair_policy(&rel).angle)
                        };
                        if let Some(range) = erase {
                            b.replace(range, "");
                            return;
                        }
                    }
                    if b.selection().is_empty() {
                        if key.modifiers.platform {
                            b.line_edge(forward, false, true);
                        } else if key.modifiers.alt || key.modifiers.control {
                            b.horizontal(forward, true, true);
                        }
                    }
                    if key.modifiers.platform || key.modifiers.alt || key.modifiers.control {
                        b.erase(forward);
                    } else {
                        b.erase_typed(forward);
                    }
                },
                cx,
            )
            }
            ("enter", false) => {
                let rel = self.key.rel.clone();
                self.edit(
                    move |b| {
                        let caret = b.selection().start;
                        let insertion = enter_indent(&rel, b.text(), caret).unwrap_or_else(|| {
                            let start = b.text()[..caret]
                                .rfind('\n')
                                .map(|i| i + 1)
                                .unwrap_or(0);
                            let indent = b.text()[start..caret]
                                .chars()
                                .take_while(|c| *c == ' ' || *c == '\t')
                                .collect::<String>();
                            format!("\n{indent}")
                        });
                        let blank_line = insertion.matches('\n').count() >= 2;
                        let last_line = insertion.rsplit('\n').next().unwrap_or("").len();
                        b.enter_typed(&insertion);
                        if blank_line {
                            b.move_to(caret + insertion.len() - 1 - last_line, false);
                        }
                    },
                    cx,
                )
            }
            ("tab", false) => self.edit(|b| b.insert("  "), cx),
            _ => return, // Printable text and composition belong to the OS handler.
        }
        if matches!(key.key.as_str(), "backspace" | "delete") {
            self.refresh_completion(cx);
        }
        cx.stop_propagation();
    }
    fn scroll_editor(
        &mut self,
        event: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let native = self.list.logical_scroll_top();
        let native_pixels = self.list.offset_for_item(native.item_ix) + native.offset_in_item;
        let previous_pixels =
            self.list.offset_for_item(self.last_scroll_top.item_ix) + self.last_scroll_top.offset_in_item;
        let delta = if native_pixels != previous_pixels {
            self.list.scroll_to(self.last_scroll_top);
            f32::from(native_pixels - previous_pixels)
        } else {
            -f32::from(event.delta.pixel_delta(px(LINE_HEIGHT * 3.)).y)
        };
        if delta == 0. {
            return;
        }
        self.scroll_remaining += delta;
        if !self.scroll_running {
            self.scroll_running = true;
            self.scroll_generation = self.scroll_generation.wrapping_add(1);
            let generation = self.scroll_generation;
            cx.spawn(async move |this, cx| loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let keep = this
                    .update(cx, |editor, cx| {
                        if !editor.scroll_running || editor.scroll_generation != generation {
                            return false;
                        }
                        let step = if editor.scroll_remaining.abs() < 1. {
                            editor.scroll_remaining
                        } else {
                            editor.scroll_remaining * 0.35
                        };
                        if step.abs() > 0.01 {
                            editor.list.scroll_by(px(step));
                            editor.scroll_remaining -= step;
                            cx.notify();
                        }
                        if editor.scroll_remaining.abs() < 0.5 {
                            editor.scroll_remaining = 0.;
                            editor.scroll_running = false;
                            cx.notify();
                            return false;
                        }
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            })
            .detach();
        }
        cx.stop_propagation();
    }
}

impl gpui::EventEmitter<OpenSymbol> for CodeEditor {}

impl EntityInputHandler for CodeEditor {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        self.read(|b| {
            let range = b.byte_range(range);
            *actual = Some(b.utf16_range(range.clone()));
            b.text()[range].to_owned()
        })
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        self.read(|b| UTF16Selection {
            range: b.utf16_range(b.selection()),
            reversed: b.reversed(),
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.read(|b| b.marked.clone().map(|r| b.utf16_range(r)))
            .flatten()
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.edit(TextBuffer::unmark, cx);
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let refresh_completion = range.is_none();
        self.completion_generation = self.completion_generation.wrapping_add(1);
        if refresh_completion && !self.extra_cursors.is_empty() {
            self.multi_replace(text, cx);
            return;
        }
        let rel = self.key.rel.clone();
        let policy = auto_pair_policy(&rel);
        self.edit(
            move |b| {
                let typed_input = range.is_none();
                let range = range
                    .map(|r| b.byte_range(r))
                    .or(b.marked.clone())
                    .unwrap_or(b.selection());
                if typed_input && text.chars().count() == 1 {
                    let typed = text.chars().next().unwrap_or('\0');
                    let selection = b.selection();
                    if !selection.is_empty()
                        && let Some(close) = surround_close_for(typed, policy)
                    {
                        let selected = b.text()[selection.clone()].to_owned();
                        b.replace(selection.clone(), &format!("{typed}{selected}{close}"));
                        let start = selection.start + typed.len_utf8();
                        b.select(start, start + selected.len());
                        return;
                    }
                    let caret = b.selection().start;
                    let action = {
                        let current = b.text();
                        if typed == '*' && comment_auto_close(&rel, current, caret) {
                            b.replace(range, "* */");
                            b.move_to(caret + 1, false);
                            return;
                        }
                        let (inside_string, inside_comment) = pair_context(current, caret);
                        auto_pair_action(
                            typed,
                            current,
                            caret,
                            policy,
                            inside_string,
                            inside_comment,
                        )
                    };
                    match action {
                        Some(PairAction::Insert(close)) => {
                            b.insert(&format!("{typed}{close}"));
                            b.move_to(caret + typed.len_utf8(), false);
                        }
                        Some(PairAction::Skip) => {
                            b.move_to(caret + typed.len_utf8(), false);
                        }
                        None => b.type_text(range, text),
                    }
                } else {
                    b.type_text(range, text);
                }
            },
            cx,
        );
        if refresh_completion {
            self.refresh_completion(cx);
            self.schedule_quick_suggestion(window, cx);
        }
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selection: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_multi_cursor();
        self.edit(
            |b| {
                let range = range
                    .map(|r| b.byte_range(r))
                    .or(b.marked.clone())
                    .unwrap_or(b.selection());
                b.mark(range, text, selection);
            },
            cx,
        );
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.read(|b| b.byte_range(range))?;
        let start = self.position_at(range.start)?;
        let end = self.position_at(range.end).unwrap_or(start);
        Some(Bounds::new(
            start,
            size(
                if start.y == end.y {
                    (end.x - start.x).max(px(1.))
                } else {
                    px(1.)
                },
                px(LINE_HEIGHT),
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let index = self.index_at(point);
        self.read(|b| b.to_utf16(index))
    }
}

struct CodeLine {
    index: usize,
    editor: Entity<CodeEditor>,
    text: SharedString,
    start: usize,
    mapping: Vec<(usize, usize)>,
    end: usize,
}
impl IntoElement for CodeLine {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for CodeLine {
    type RequestLayoutState = ();
    type PrepaintState = PaintedLine;
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
        _: &mut App,
    ) -> (LayoutId, ()) {
        let text = self.text.clone();
        let style = window.text_style();
        let run = style.to_run(text.len());
        let font_size = style.font_size.to_pixels(window.rem_size());
        let id =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let width = known
                    .width
                    .or(match available.width {
                        AvailableSpace::Definite(w) => Some(w),
                        _ => None,
                    })
                    .unwrap_or(px(400.));
                let shaped = window
                    .text_system()
                    .shape_text(
                        text.clone(),
                        font_size,
                        std::slice::from_ref(&run),
                        Some((width - px(12.)).max(px(1.))),
                        None,
                    )
                    .unwrap_or_default();
                let height = shaped
                    .first()
                    .map(|l| l.size(px(LINE_HEIGHT)).height)
                    .unwrap_or(px(LINE_HEIGHT));
                size(width, height.max(px(LINE_HEIGHT)))
            });
        (id, ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PaintedLine {
        let style = window.text_style();
        let theme = Theme::default();
        let colors = self.editor.read(cx).bracket_colors.clone();
        let spans = bracket_display_runs(
            self.text.len(),
            &self.mapping,
            self.start,
            self.end,
            &colors,
        );
        let mut runs = Vec::new();
        for (offset, len, depth) in spans {
            if len == 0 {
                continue;
            }
            let mut run = style.to_run(len);
            run.color = match depth {
                Some(depth) => bracket_color(depth).into(),
                None => theme.text.into(),
            };
            let _ = offset;
            runs.push(run);
        }
        if runs.is_empty() {
            let mut run = style.to_run(self.text.len());
            run.color = theme.text.into();
            runs.push(run);
        }
        let layout = window
            .text_system()
            .shape_text(
                self.text.clone(),
                style.font_size.to_pixels(window.rem_size()),
                &runs,
                Some((bounds.size.width - px(12.)).max(px(1.))),
                None,
            )
            .unwrap_or_default()
            .into_iter()
            .next()
            .unwrap();
        PaintedLine {
            start: self.start,
            end: self.end,
            origin: bounds.origin,
            layout: Arc::new(layout),
            mapping: self.mapping.clone(),
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        line: &mut PaintedLine,
        window: &mut Window,
        cx: &mut App,
    ) {
        let editor = self.editor.read(cx);
        let focus = editor.focus.clone();
        let state = editor.read(|b| {
            (
                b.selection(),
                b.caret(),
                b.marked.clone(),
                editor.extra_cursors.clone(),
                editor.occurrences.clone(),
                editor.brackets.clone(),
                editor.caret.read(cx).opacity(),
            )
        });
        let theme = Theme::default();
        if let Some((selection, caret, marked, extras, occurrences, brackets, caret_opacity)) = state {
            if caret >= line.start && caret <= line.end {
                window.paint_quad(fill(bounds, editor_selection_fill(&theme)));
            }
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
                let source_start = line.source_index(pair[0]);
                let source_end = line.source_index(pair[1]);
                let first = editor
                    .find_matches
                    .ranges
                    .partition_point(|range| range.end <= source_start);
                for found in editor.find_matches.ranges[first..]
                    .iter()
                    .take_while(|range| range.start < source_end)
                {
                    let start = found.start.max(source_start);
                    let end = found.end.min(source_end);
                    if start < end {
                        let layout = &line.layout.unwrapped_layout;
                        let left = layout.x_for_index(line.display_index(start))
                            - layout.x_for_index(pair[0]);
                        let right = layout.x_for_index(line.display_index(end))
                            - layout.x_for_index(pair[0]);
                        window.paint_quad(fill(
                            Bounds::new(
                                line.origin + point(left, px(LINE_HEIGHT) * row),
                                size((right - left).max(px(1.)), px(LINE_HEIGHT)),
                            ),
                            theme.accent.opacity(0.22),
                        ));
                    }
                }
                for bracket in &brackets {
                    let start = bracket.start.max(source_start);
                    let end = bracket.end.min(source_end);
                    if start >= end {
                        continue;
                    }
                    let layout = &line.layout.unwrapped_layout;
                    let left =
                        layout.x_for_index(line.display_index(start)) - layout.x_for_index(pair[0]);
                    let right =
                        layout.x_for_index(line.display_index(end)) - layout.x_for_index(pair[0]);
                    window.paint_quad(fill(
                        Bounds::new(
                            line.origin + point(left, px(LINE_HEIGHT) * row),
                            size((right - left).max(px(1.)), px(LINE_HEIGHT)),
                        ),
                        theme.accent.opacity(0.24),
                    ));
                }
                for occurrence in &occurrences {
                    let start = occurrence.start.max(source_start);
                    let end = occurrence.end.min(source_end);
                    if start >= end {
                        continue;
                    }
                    let layout = &line.layout.unwrapped_layout;
                    let left =
                        layout.x_for_index(line.display_index(start)) - layout.x_for_index(pair[0]);
                    let right =
                        layout.x_for_index(line.display_index(end)) - layout.x_for_index(pair[0]);
                    window.paint_quad(fill(
                        Bounds::new(
                            line.origin + point(left, px(LINE_HEIGHT) * row),
                            size((right - left).max(px(1.)), px(LINE_HEIGHT)),
                        ),
                        theme.accent.opacity(0.13),
                    ));
                }
                let mut paint_range = |start: usize, end: usize| {
                    let start = start.max(source_start);
                    let end = end.min(source_end);
                    if start >= end {
                        return;
                    }
                    let layout = &line.layout.unwrapped_layout;
                    let left =
                        layout.x_for_index(line.display_index(start)) - layout.x_for_index(pair[0]);
                    let right =
                        layout.x_for_index(line.display_index(end)) - layout.x_for_index(pair[0]);
                    window.paint_quad(fill(
                        Bounds::new(
                            line.origin + point(left, px(LINE_HEIGHT) * row),
                            size((right - left).max(px(1.)), px(LINE_HEIGHT)),
                        ),
                        editor_selection_fill(&theme),
                    ));
                };
                paint_range(selection.start, selection.end);
                for extra in &extras {
                    paint_range(extra.start, extra.end);
                }
            }
            if let Some(marked) = marked {
                let start = marked.start.max(line.start);
                let end = marked.end.min(line.end);
                if start < end
                    && let Some(pos) = line
                        .layout
                        .position_for_index(line.display_index(start), px(LINE_HEIGHT))
                {
                    let end = line
                        .layout
                        .position_for_index(line.display_index(end), px(LINE_HEIGHT))
                        .unwrap_or(pos);
                    window.paint_quad(fill(
                        Bounds::new(
                            line.origin + pos + point(px(0.), px(LINE_HEIGHT - 1.)),
                            size((end.x - pos.x).max(px(2.)), px(1.)),
                        ),
                        theme.text,
                    ));
                }
            }
            let _ = line.layout.paint(
                line.origin,
                px(LINE_HEIGHT),
                TextAlign::Left,
                None,
                window,
                cx,
            );
            if focus.is_focused(window) && caret_opacity > 0.01 {
                let primary = selection.is_empty().then_some(caret);
                for caret in primary.into_iter().chain(
                    extras
                        .iter()
                        .filter(|range| range.is_empty())
                        .map(|range| range.start),
                ) {
                    if caret < line.start || caret > line.end {
                        continue;
                    }
                    if let Some(pos) = line
                        .layout
                        .position_for_index(line.display_index(caret), px(LINE_HEIGHT))
                    {
                        window.paint_quad(fill(
                            Bounds::new(line.origin + pos, size(px(1.), px(LINE_HEIGHT))),
                            theme.text.opacity(caret_opacity),
                        ));
                    }
                }
            }
        }
        self.editor.update(cx, |editor, _| {
            editor.painted.insert(self.index, line.clone());
        });
    }
}
const BRACKET_COLORS: [u32; 3] = [0xffd700, 0xda70d6, 0x179fff];

/// Source's Monaco appearance maps both `editor.lineHighlightBackground` and
/// `editor.selectionBackground` to the active profile's `selected` color.
fn editor_selection_fill(theme: &Theme) -> gpui::Rgba {
    theme.selected
}

fn bracket_color(depth: usize) -> gpui::Rgba {
    gpui::rgb(BRACKET_COLORS[depth % BRACKET_COLORS.len()])
}

/// Split a single display line into default/colored runs. Bracket spans are
/// source-order byte ranges from `brackets::colorized`; tabs in the display
/// text are already expanded, so spans map through the line's source mapping.
fn bracket_display_runs(
    text_len: usize,
    mapping: &[(usize, usize)],
    start: usize,
    end: usize,
    colors: &[(Range<usize>, usize)],
) -> Vec<(usize, usize, Option<usize>)> {
    let mut segments = Vec::new();
    let mut cursor = 0usize;
    for (range, depth) in colors {
        if range.start < start || range.start >= end {
            continue;
        }
        let local = range.start - start;
        let Some(display) = mapping
            .iter()
            .find(|(_, source)| *source >= local)
            .map(|(display, _)| *display)
        else {
            continue;
        };
        let end = display + 1;
        if end > text_len || display < cursor {
            continue;
        }
        if display > cursor {
            segments.push((cursor, display - cursor, None));
        }
        segments.push((display, 1, Some(*depth)));
        cursor = end;
    }
    if cursor < text_len {
        segments.push((cursor, text_len - cursor, None));
    }
    segments
}

fn expand_tabs(text: &str) -> (SharedString, Vec<(usize, usize)>) {
    let mut display = String::new();
    let mut mapping = Vec::new();
    let mut column = 0;
    for (source, c) in text.char_indices() {
        if c == '\t' {
            let spaces = 2 - column % 2;
            for _ in 0..spaces {
                mapping.push((display.len(), source));
                display.push(' ');
            }
            column += spaces;
        } else {
            mapping.push((display.len(), source));
            display.push(c);
            column += 1;
        }
    }
    mapping.push((display.len(), text.len()));
    (display.into(), mapping)
}
impl Render for CodeEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur_subscription.is_none() {
            self.blur_subscription = Some(cx.on_blur(&self.focus, window, |editor, _, cx| {
                editor.k_chord = false;
                editor.edit(TextBuffer::break_undo_group, cx)
            }));
        }
        let focused = self.focus.is_focused(window) && window.is_window_active();
        self.caret.update(cx, |caret, cx| caret.update_state(focused, cx));
        self.last_scroll_top = self.list.logical_scroll_top();
        self.sync_lines();
        self.refresh_find();
        let occurrences = self
            .read(|buffer| {
                if buffer.selection().is_empty() {
                    crate::highlights::word_occurrences(
                        buffer.text(),
                        buffer.caret(),
                        crate::highlights::MAX_OCCURRENCES,
                    )
                } else {
                    Vec::new()
                }
            })
            .unwrap_or_default();
        self.occurrences = occurrences;
        let brackets = self
            .read(|buffer| {
                crate::brackets::matching_pair(buffer.text(), buffer.caret())
                    .map(|(open, close)| vec![open, close])
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        self.brackets = brackets;
        let find_panel = self.find_open.then(|| self.render_find(cx));
        let goto_panel = self.goto_open.then(|| self.render_goto_line(cx));
        let palette_panel = self.palette_open.then(|| self.render_palette(cx));
        let completion = self.completion_open.then(|| self.render_completion(cx));
        let symbols_panel = self.symbols_open.then(|| self.render_symbols(cx));
        let workspace_symbols_panel = self
            .workspace_symbols_open
            .then(|| self.render_workspace_symbols(cx));
        let code_actions_panel = self.code_actions_open.then(|| self.render_code_actions(cx));
        let rename_panel = self.rename_open.then(|| self.render_rename(cx));
        let hover_panel = self.hover_popup.as_ref().map(|(hover, position)| {
            let theme = Theme::default();
            div()
                .id("editor-hover")
                .debug_selector(|| "editor-hover".into())
                .role(Role::Label)
                .aria_label(SharedString::from(format!(
                    "{}: {}",
                    hover.word, hover.detail
                )))
                .absolute()
                .left(position.x + px(12.))
                .top(position.y + px(18.))
                .max_w(px(360.))
                .p(px(8.))
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.panel)
                .shadow(vec![gpui::BoxShadow {
                    inset: false,
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(12.),
                    spread_radius: px(0.),
                    color: theme::rgba(0, 0, 0, 0.22).into(),
                }])
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(SharedString::from(format!(
                            "{} · {}",
                            hover.kind, hover.word
                        ))),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .font_family(theme::FONT_MONO)
                        .text_color(theme.dim)
                        .child(SharedString::from(hover.signature.clone())),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(theme.faint)
                        .child(SharedString::from(hover.detail.clone())),
                )
        });
        let signature_help =
            if self.completion_open || self.symbols_open || self.workspace_symbols_open {
                None
            } else {
                self.read(|buffer| crate::signature::signature_help(buffer.text(), buffer.caret()))
                    .flatten()
            };
        let signature_panel = signature_help.map(|signature| {
            let theme = Theme::default();
            let active = signature
                .active_label()
                .map(|parameter| format!("Parameter {}: {parameter}", signature.active + 1))
                .unwrap_or_else(|| "No parameters".into());
            div()
                .id("editor-signature-help")
                .debug_selector(|| "editor-signature-help".into())
                .role(Role::Label)
                .aria_label(SharedString::from(format!(
                    "{} — {active}",
                    signature.label()
                )))
                .absolute()
                .left(px(12.))
                .bottom(px(if self.diagnostics.is_empty() {
                    12.
                } else {
                    48.
                }))
                .max_w(px(420.))
                .p(px(8.))
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.panel)
                .shadow(vec![gpui::BoxShadow {
                    inset: false,
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(12.),
                    spread_radius: px(0.),
                    color: theme::rgba(0, 0, 0, 0.22).into(),
                }])
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(10.5))
                        .font_family(theme::FONT_MONO)
                        .text_color(theme.text)
                        .child(SharedString::from(signature.label())),
                )
                .child(
                    div()
                        .id("editor-signature-active")
                        .debug_selector(|| "editor-signature-active".into())
                        .text_size(px(9.5))
                        .text_color(theme.dim)
                        .child(SharedString::from(active)),
                )
        });
        let diagnostic_banner = self.diagnostics.first().cloned().map(|diagnostic| {
            let message = format!(
                "{} · Line {}, Column {}: {}",
                diagnostic.source, diagnostic.line, diagnostic.column, diagnostic.message
            );
            let label = message.clone();
            div()
                .id("editor-diagnostics-banner")
                .debug_selector(|| "editor-diagnostics-banner".into())
                .role(Role::Label)
                .aria_label(SharedString::from(label))
                .absolute()
                .left(px(12.))
                .right(px(12.))
                .bottom(px(10.))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(6.))
                .border_1()
                .border_color(Theme::default().red)
                .bg(Theme::default().red.opacity(0.12))
                .text_size(px(11.))
                .text_color(Theme::default().red)
                .child(SharedString::from(message))
        });
        let digit_width = f32::from(
            window
                .text_system()
                .shape_line(
                    "0".into(),
                    px(13.),
                    &[TextRun {
                        len: 1,
                        font: theme::source_editor_font(),
                        color: Theme::default().text.into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width(),
        );
        // Monaco reserves at least three digit cells plus the 16px folding
        // lane. Four-digit files grow that lane without moving the caret math.
        let number_width =
            (self.starts.len().to_string().len().max(3) as f32 * digit_width).round();
        let gutter = number_width + 16.;
        if self.reveal_caret
            && let Some(caret) = self.read(|b| b.caret())
        {
            let line = self
                .starts
                .partition_point(|s| *s <= caret)
                .saturating_sub(1);
            if !self.visible_lines.contains(&line) {
                self.unfold_line(line);
            }
            if self.position_at(caret).is_none() {
                let visible_index = self
                    .visible_lines
                    .iter()
                    .position(|visible| *visible == line)
                    .unwrap_or_else(|| line.min(self.visible_lines.len().saturating_sub(1)));
                // Anchor by visible row. Pixel-based reveal can only see
                // measured heights and otherwise walks through a long file
                // a viewport at a time instead of reaching the caret.
                self.list.scroll_to(ListOffset {
                    item_ix: visible_index,
                    offset_in_item: px(0.),
                });
            }
        }
        self.painted.clear();
        let entity = cx.entity();
        let input = entity.clone();
        let painted = entity.clone();
        let weak = cx.weak_entity();
        let label = SharedString::from(format!("Source editor: {}", self.key.rel));
        let instance = self.instance;
        div()
            .id(SharedString::from(format!(
                "native-code-editor-{}",
                self.instance
            )))
            .debug_selector(move || format!("native-code-editor-{instance}"))
            .size_full()
            .min_h_0()
            .relative()
            .overflow_hidden()
            .bg(Theme::default().editor)
            .font(theme::source_editor_font())
            .font_weight(FontWeight::NORMAL)
            .text_size(px(13.))
            .line_height(px(LINE_HEIGHT))
            .role(Role::MultilineTextInput)
            .aria_label(label)
            .aria_value(SharedString::from(
                self.read(|b| b.text().to_owned()).unwrap_or_default(),
            ))
            .tab_stop(true)
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(|this, _: &Cut, w, cx| this.menu_key("x", false, w, cx)))
            .on_action(cx.listener(|this, _: &Copy, w, cx| this.menu_key("c", false, w, cx)))
            .on_action(cx.listener(|this, _: &Paste, w, cx| this.menu_key("v", false, w, cx)))
            .on_action(cx.listener(|this, _: &SelectAll, w, cx| this.menu_key("a", false, w, cx)))
            .on_action(cx.listener(|this, _: &Undo, w, cx| this.menu_key("z", false, w, cx)))
            .on_action(cx.listener(|this, _: &Redo, w, cx| this.menu_key("z", true, w, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    this.hover_popup = None;
                    this.close_completion(cx);
                    this.close_symbols(cx);
                    this.close_workspace_symbols(cx);
                    this.close_code_actions(cx);
                    this.clear_multi_cursor();
                    this.selecting = true;
                    let index = this.index_at(event.position);
                    this.edit(
                        |b| {
                            if event.click_count >= 3 {
                                b.move_to(index, false);
                                b.line_edge(false, false, false);
                                b.line_edge(true, false, true);
                            } else if event.click_count == 2 {
                                b.move_to(index, false);
                                b.horizontal(false, false, true);
                                b.horizontal(true, true, true);
                            } else {
                                b.move_to(index, event.modifiers.shift);
                            }
                        },
                        cx,
                    );
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.selecting {
                    let index = this.index_at(event.position);
                    this.edit(|b| b.move_to(index, true), cx);
                } else if !this.completion_open
                    && !this.symbols_open
                    && !this.workspace_symbols_open
                {
                    let index = this.index_at(event.position);
                    let hover = this
                        .read(|buffer| crate::hover::hover_at(buffer.text(), index))
                        .flatten();
                    if let Some(hover) = hover {
                        this.hover_popup = Some((hover, event.position));
                        cx.notify();
                    } else if this.hover_popup.take().is_some() {
                        cx.notify();
                    }
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
            .on_a11y_action(AccessibleAction::SetValue, move |data, _, cx| {
                if let Some(accesskit::ActionData::Value(text)) = data {
                    let _ = weak.update(cx, |this, cx| {
                        this.edit(
                            |b| {
                                b.select_all();
                                b.insert(text);
                            },
                            cx,
                        )
                    });
                }
            })
            .child(
                div()
                    .size_full()
                    .on_scroll_wheel(cx.listener(Self::scroll_editor))
                    .child(list(self.list.clone(), move |index, _, cx| {
                    let editor = entity.read(cx);
                    let logical = editor.visible_lines.get(index).copied().unwrap_or(index);
                    let start = editor.starts.get(logical).copied().unwrap_or(0);
                    let source = editor
                        .read(|b| {
                            let end = editor
                                .starts
                                .get(logical + 1)
                                .map(|i| i - 1)
                                .unwrap_or(b.text().len());
                            (b.text()[start..end].to_owned(), end)
                        })
                        .unwrap_or_default();
                    let (text, mapping) = expand_tabs(&source.0);
                    let foldable = editor.folds.iter().find(|range| range.start == logical);
                    let folded = editor.folded.contains(&logical);
                    let line_error = editor
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.line == logical + 1);
                    let fold_entity = entity.clone();
                    let current_line = editor
                        .read(|buffer| buffer.caret())
                        .map(|caret| {
                            editor
                                .starts
                                .partition_point(|start| *start <= caret)
                                .saturating_sub(1)
                        })
                        == Some(logical);
                    let row_selector = format!("editor-row-{logical}");
                    let row = div()
                        .id(SharedString::from(format!("editor-row-{logical}")))
                        .debug_selector(move || row_selector.clone())
                        .relative()
                        .when(line_error, |row| {
                            let selector = format!("editor-error-marker-{logical}");
                            row.child(
                                div()
                                    .id(SharedString::from(format!(
                                        "editor-error-marker-{logical}"
                                    )))
                                    .debug_selector(move || selector.clone())
                                    .absolute()
                                    .left_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(px(2.))
                                    .bg(Theme::default().red),
                            )
                        })
                        .flex()
                        .w_full()
                        .child(
                            div()
                                .w(px(gutter))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .when(current_line, |gutter| {
                                    gutter.bg(Theme::default().selected.opacity(0.22))
                                })
                                .child(
                                    div()
                                        .w(px(16.))
                                        .flex_shrink_0()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .when_some(foldable, |lane, _| {
                                            let selector = format!("editor-fold-{logical}");
                                            lane.child(
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "editor-fold-{logical}"
                                                    )))
                                                    .debug_selector(move || selector.clone())
                                                    .role(Role::Button)
                                                    .aria_label(if folded {
                                                        "Expand folded lines"
                                                    } else {
                                                        "Collapse lines"
                                                    })
                                                    .size(px(16.))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(px(3.))
                                                    .text_size(px(11.))
                                                    .text_color(Theme::default().faint)
                                                    .cursor_pointer()
                                                    .hover(|style| {
                                                        style
                                                            .bg(Theme::default().inset)
                                                            .text_color(Theme::default().text)
                                                    })
                                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                        cx.stop_propagation()
                                                    })
                                                    .on_click(move |_, _, cx| {
                                                        fold_entity.update(cx, |editor, cx| {
                                                            editor.toggle_fold(logical, cx)
                                                        });
                                                    })
                                                    .child(if folded { "›" } else { "⌄" }),
                                            )
                                        }),
                                )
                                .child({
                                    let number_selector =
                                        format!("editor-current-line-number-{logical}");
                                    let number = div()
                                        .id(SharedString::from(format!(
                                            "editor-line-number-{logical}"
                                        )))
                                        .flex_1()
                                        .min_w_0()
                                        .pr(px(4.))
                                        .text_right()
                                        .text_size(px(13.))
                                        .text_color(if current_line {
                                            Theme::default().dim
                                        } else {
                                            Theme::default().faint
                                        })
                                        .child(SharedString::from((logical + 1).to_string()));
                                    number.when(current_line, |number| {
                                        number
                                            .debug_selector(move || number_selector.clone())
                                            .font_weight(FontWeight::MEDIUM)
                                    })
                                }),
                        )
                        .child({
                            let selector = format!("editor-line-{logical}");
                            div()
                                .id(SharedString::from(format!("editor-line-{logical}")))
                                .debug_selector(move || selector.clone())
                                .flex_1()
                                .min_w_0()
                                .child(CodeLine {
                                    editor: entity.clone(),
                                    index: logical,
                                    text,
                                    start,
                                    end: source.1,
                                    mapping,
                                })
                        });
                    row.into_any_element()
                })
                .size_full()
                .pt(px(10.))
                .pb(px(20.))),
            )
            .child(
                canvas(
                    move |bounds, _, cx| {
                        input.update(cx, |editor, _| editor.bounds = Some(bounds));
                    },
                    move |bounds, _, window, cx| {
                        let focus = painted.read(cx).focus.clone();
                        painted.update(cx, |editor, cx| {
                            if editor.reveal_caret
                                && let Some(position) = editor
                                    .read(|b| b.caret())
                                    .and_then(|i| editor.position_at(i))
                            {
                                let offset = if position.y < bounds.top() {
                                    position.y - bounds.top()
                                } else if position.y + px(LINE_HEIGHT) > bounds.bottom() {
                                    position.y + px(LINE_HEIGHT) - bounds.bottom()
                                } else {
                                    px(0.)
                                };
                                if offset == px(0.) {
                                    editor.reveal_caret = false;
                                } else {
                                    editor.list.scroll_by(offset);
                                    cx.notify();
                                }
                            }
                        });
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, painted.clone()),
                            cx,
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when_some(find_panel, |editor, panel| editor.child(panel))
            .when_some(goto_panel, |editor, panel| editor.child(panel))
            .when_some(palette_panel, |editor, panel| editor.child(panel))
            .when_some(completion, |editor, popup| editor.child(popup))
            .when_some(symbols_panel, |editor, panel| editor.child(panel))
            .when_some(workspace_symbols_panel, |editor, panel| editor.child(panel))
            .when_some(rename_panel, |editor, panel| editor.child(panel))
            .when_some(code_actions_panel, |editor, panel| editor.child(panel))
            .when_some(hover_panel, |editor, panel| editor.child(panel))
            .when_some(signature_panel, |editor, panel| editor.child(panel))
            .when_some(diagnostic_banner, |editor, banner| editor.child(banner))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CARET_FRAME, CARET_HOLD_FRAMES, CaseStyle, FoldRange, PairAction, QuoteRule, SmoothCaret,
        apply_case_style, auto_pair_action, auto_pair_erase, auto_pair_policy, bracket_display_runs,
        comment_auto_close, comment_config, comment_toggle_for, editor_selection_fill,
        copy_line_block, delete_all_edits, delete_lines_at, diagnostic_target, enter_indent,
        expand_tabs, find_occurrences, goto_line_offset, insert_line_at, join_lines_at,
        delete_duplicate_lines_at, duplicate_selection_at, filter_symbols, move_line_block,
        line_end_cursor_targets, line_edge_cursor_targets,
        pair_context, palette_matches, parse_goto_line, remove_brackets_at, reverse_lines_at,
        previous_occurrence, next_occurrence_after,
        shift_line_edits, sort_lines_at, spaces_to_next_tab_stop, title_case,
        translate_line_offset, transpose_at,
        trim_trailing_whitespace_at,
        quick_suggestion_in_comment, quick_suggestion_trigger, surround_close_for,
        fold_ranges_for, fold_ranges_for_language, fold_regions_except,
        visible_lines_after_folds,
    };
    use crate::theme::Theme;
    use gpui::AppContext;
    use std::collections::BTreeSet;

    #[test]
    fn fold_ranges_follow_indentation_and_hide_only_nested_bodies() {
        let text = "fn main() {\n  let a = 1;\n  if a {\n    work();\n  }\n}\ntail";
        let folds = fold_ranges_for(text);
        assert_eq!(
            folds,
            vec![
                FoldRange { start: 0, end: 5 },
                FoldRange { start: 2, end: 4 }
            ]
        );
        let all = (0..7).collect::<Vec<_>>();
        assert_eq!(visible_lines_after_folds(7, &folds, &BTreeSet::new()), all);
        assert_eq!(
            visible_lines_after_folds(7, &folds, &BTreeSet::from([0])),
            vec![0, 6]
        );
        assert_eq!(
            visible_lines_after_folds(7, &folds, &BTreeSet::from([0, 2])),
            vec![0, 6]
        );
        assert_eq!(
            visible_lines_after_folds(7, &folds, &BTreeSet::from([2])),
            vec![0, 1, 2, 5, 6]
        );
    }

    #[test]
    fn fold_categories_cover_block_comments_regions_and_except_selection() {
        let comments = "/* header\n * body\n */\nconst a = 1;\n// #region group\nconst b = 2;\n// #endregion\ntail";
        let folds = fold_ranges_for_language("x.ts", comments);
        assert!(folds.contains(&FoldRange { start: 0, end: 2 }));
        assert!(folds.contains(&FoldRange { start: 4, end: 6 }));
        let plain = "/* one\ntwo\n*/";
        assert!(
            !fold_ranges_for(plain).iter().any(|range| range.start == 0),
            "comment folds require the file language configuration"
        );
        assert!(
            fold_ranges_for_language("x.ts", plain).contains(&FoldRange { start: 0, end: 2 })
        );

        let css = "/* one\ntwo\n*/\n.a { color: red; }";
        assert!(
            fold_ranges_for_language("x.css", css).contains(&FoldRange { start: 0, end: 2 })
        );
        let html = "<!-- one\ntwo\n-->\n<div>\nx\n</div>";
        let html_folds = fold_ranges_for_language("page.html", html);
        assert!(html_folds.contains(&FoldRange { start: 0, end: 2 }));
        assert!(html_folds.contains(&FoldRange { start: 3, end: 5 }));
        assert!(
            !fold_ranges_for_language("x.py", "/* one\ntwo\n*/")
                .iter()
                .any(|range| range.start == 0),
            "languages without block comments never gain comment folds"
        );
        assert!(
            !fold_ranges_for_language("note.md", "<!-- one\ntwo\n-->")
                .iter()
                .any(|range| range.start == 0),
            "languages without a comment-folding provider stay unfurled"
        );

        let folds = vec![
            FoldRange { start: 0, end: 10 },
            FoldRange { start: 2, end: 4 },
            FoldRange { start: 6, end: 8 },
        ];
        assert_eq!(
            fold_regions_except(&folds, &[2]),
            vec![6],
            "selected regions and their ancestors are excluded, but siblings remain"
        );
        assert_eq!(
            fold_regions_except(&folds, &[]),
            vec![0, 2, 6],
            "no selected region leaves every fold eligible"
        );
    }

    #[test]
    fn markup_folding_matches_tags_and_ignores_comments_raw_text_and_voids() {
        let html = "<div class=\"a > b\">\n  <section>\n    <br>\n  </section>\n</div>\ntail";
        let folds = fold_ranges_for(html);
        assert!(folds.contains(&FoldRange { start: 0, end: 4 }));
        assert!(folds.contains(&FoldRange { start: 1, end: 3 }));

        let commented = "<!-- <div>\n</div> -->\n<p>\nx\n</p>";
        let folds = fold_ranges_for(commented);
        assert!(folds.contains(&FoldRange { start: 2, end: 4 }));
        assert!(!folds.iter().any(|range| range.start == 0));

        let script = "<script>\nif (a < b) {}\n</script>\n<div>\nx\n</div>";
        let folds = fold_ranges_for(script);
        assert!(folds.contains(&FoldRange { start: 0, end: 2 }));
        assert!(folds.contains(&FoldRange { start: 3, end: 5 }));
        assert!(!folds.iter().any(|range| range.start == 1));

        let uppercase = "<DIV>\nx\n</div>";
        assert!(fold_ranges_for(uppercase).contains(&FoldRange { start: 0, end: 2 }));

        assert!(
            !fold_ranges_for("<img src=\"x\">\ntext")
                .iter()
                .any(|range| range.start == 0)
        );
        assert!(
            !fold_ranges_for("<div>\nunclosed")
                .iter()
                .any(|range| range.start == 0)
        );
    }

    #[test]
    fn region_markers_fold_nested_language_blocks() {
        let text = "// #region outer\n// #region inner\nconst a = 1;\n// #endregion\nconst b = 2;\n// #endregion\ntail";
        let folds = fold_ranges_for(text);
        assert!(folds.contains(&FoldRange { start: 0, end: 5 }));
        assert!(folds.contains(&FoldRange { start: 1, end: 3 }));

        let css = "/* #region styles */\n.a { color: red; }\n/* #endregion */\ntail";
        assert!(
            fold_ranges_for(css).contains(&FoldRange { start: 0, end: 2 }),
            "CSS block-comment regions fold"
        );

        let hash = "#region setup\nwork\n# endregion\ntail";
        assert!(fold_ranges_for(hash).contains(&FoldRange { start: 0, end: 2 }));

        let unclosed = "// #region open\nwork\ntail";
        assert!(
            !fold_ranges_for(unclosed)
                .iter()
                .any(|range| range.start == 0),
            "an unmatched region start does not fold"
        );
        let extra_end = "work\n// #endregion\ntail";
        assert!(
            !fold_ranges_for(extra_end)
                .iter()
                .any(|range| range.start == 0)
        );
    }

    #[test]
    fn brace_folding_ignores_strings_comments_and_merges_with_indentation() {
        let text = "let s = \"{\";\n// }\n/* { */\nfunction f() {\nconst x = 1;\n}\nafter";
        let folds = fold_ranges_for(text);
        assert_eq!(folds, vec![FoldRange { start: 3, end: 5 }]);
        let unindented = ".a {\ncolor: red;\n.b {\ndisplay:block;\n}\n}\nafter";
        let folds = fold_ranges_for(unindented);
        assert_eq!(
            folds,
            vec![
                FoldRange { start: 0, end: 5 },
                FoldRange { start: 2, end: 4 }
            ]
        );
    }
    #[test]
    fn fold_command_targets_follow_monaco_inner_to_parent_rules() {
        use super::*;
        let folds = vec![FoldRange { start: 0, end: 5 }, FoldRange { start: 2, end: 4 }];
        assert_eq!(fold_region_starts_at_line(&folds, 1), vec![0]);
        assert_eq!(
            fold_region_starts_at_line(&folds, 3),
            vec![2, 0],
            "the innermost containing region comes first"
        );
        assert!(fold_region_starts_at_line(&folds, 6).is_empty());
        assert_eq!(fold_regions_inside(&folds, 0), vec![2]);
        assert!(fold_regions_inside(&folds, 2).is_empty());

        let fold_commands = palette_matches("fold");
        for command in [
            "Fold",
            "Unfold",
            "Toggle Fold",
            "Fold Recursively",
            "Unfold Recursively",
            "Toggle Fold Recursively",
            "Fold All",
            "Unfold All",
            "Fold All Block Comments",
            "Fold All Regions",
            "Unfold All Regions",
            "Fold All Except Selected",
            "Unfold All Except Selected",
        ] {
            assert!(fold_commands.contains(&command), "{command}");
        }
    }

    #[gpui::test]
    fn fold_controls_collapse_bodies_and_caret_reveal_expands_them(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "folds.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("fn main() {\n  let a = 1;\n  if a {\n    work();\n  }\n}\ntail".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                    editor.clone().into_any_element()
                });
            }
        };
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |editor, _| editor.visible_lines.clone()),
            (0..7).collect::<Vec<_>>()
        );
        let fold = cx.debug_bounds("editor-fold-0").expect("fold control");
        cx.simulate_click(fold.center(), Modifiers::default());
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.folded.contains(&0));
            assert_eq!(editor.visible_lines, vec![0, 6]);
        });
        assert!(cx.debug_bounds("editor-row-1").is_none());
        assert!(cx.debug_bounds("editor-row-5").is_none());
        assert!(cx.debug_bounds("editor-row-6").is_some());

        let caret = editor.read_with(cx, |editor, _| editor.starts[3] + 2);
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(caret, caret));
        editor.update(cx, |editor, cx| {
            editor.reveal_caret = true;
            cx.notify();
        });
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(!editor.folded.contains(&0));
            assert!(!editor.folded.contains(&2));
            assert!(editor.visible_lines.contains(&3));
        });
    }

    #[gpui::test]
    fn fold_commands_follow_monaco_key_bindings_and_selection_lines(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "folds.js".into());
        let source = "fn main() {\n  let a = 1;\n  if a {\n    work();\n  }\n}\ntail";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                    editor.clone().into_any_element()
                });
            }
        };
        let place = |offset: usize, cx: &mut gpui::VisualTestContext| {
            retained
                .borrow_mut()
                .edit(&key, |buffer| buffer.select(offset, offset));
            draw(cx);
        };
        let folded = |cx: &gpui::VisualTestContext| {
            editor.read_with(cx, |editor, _| editor.folded.clone())
        };
        let visible = |cx: &gpui::VisualTestContext| {
            editor.read_with(cx, |editor, _| editor.visible_lines.clone())
        };

        place(source.find("let a").unwrap(), cx);
        cx.simulate_keystrokes("cmd-alt-[");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0]));
        assert_eq!(visible(cx), vec![0, 6]);

        cx.simulate_keystrokes("cmd-alt-]");
        draw(cx);
        assert!(folded(cx).is_empty());
        assert_eq!(visible(cx), (0..7).collect::<Vec<_>>());

        place(source.find("work").unwrap(), cx);
        cx.simulate_keystrokes("cmd-alt-[");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([2]));
        assert_eq!(visible(cx), vec![0, 1, 2, 5, 6]);

        cx.simulate_keystrokes("cmd-alt-[");
        draw(cx);
        assert_eq!(
            folded(cx),
            BTreeSet::from([0, 2]),
            "folding an already collapsed inner region targets its first uncollapsed parent"
        );
        assert_eq!(visible(cx), vec![0, 6]);

        cx.simulate_keystrokes("cmd-alt-]");
        draw(cx);
        assert_eq!(
            folded(cx),
            BTreeSet::from([0]),
            "unfold targets the innermost collapsed region"
        );
        cx.simulate_keystrokes("cmd-alt-]");
        draw(cx);
        assert!(folded(cx).is_empty());

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-l");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([2]));
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-l");
        draw(cx);
        assert!(folded(cx).is_empty());

        place(source.find("let a").unwrap(), cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("shift-cmd-l");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0, 2]));
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("shift-cmd-l");
        draw(cx);
        assert!(folded(cx).is_empty());

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-[");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0, 2]));
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-]");
        draw(cx);
        assert!(folded(cx).is_empty());

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-0");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0, 2]));
        assert_eq!(visible(cx), vec![0, 6]);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-j");
        draw(cx);
        assert!(folded(cx).is_empty());

        place(source.find("let a").unwrap(), cx);
        let inner = source.find("work").unwrap();
        editor.update(cx, |editor, _| {
            editor.extra_cursors = std::iter::once(inner..inner).collect();
        });
        cx.simulate_keystrokes("cmd-alt-[");
        draw(cx);
        assert_eq!(
            folded(cx),
            BTreeSet::from([0, 2]),
            "each cursor folds the region at its own selection start line"
        );
        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-]");
        draw(cx);
        assert!(folded(cx).is_empty());

        cx.simulate_keystrokes("f1");
        draw(cx);
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("fold all".into(), cx));
        });
        draw(cx);
        editor.read_with(cx, |editor, cx| {
            assert_eq!(editor.palette_items(cx).first(), Some(&"Fold All"));
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0, 2]));

        cx.simulate_keystrokes("f1");
        draw(cx);
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("unfold all".into(), cx));
        });
        draw(cx);
        editor.read_with(cx, |editor, cx| {
            assert_eq!(editor.palette_items(cx).first(), Some(&"Unfold All"));
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert!(folded(cx).is_empty());
    }

    #[gpui::test]
    fn fold_category_commands_follow_source_targets(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "categories.js".into());
        let source = "/* header\n * comment\n */\nconst a = 1;\nfunction outer() {\n  const b = 2;\n}\n// #region group\n  const c = 3;\n// #endregion\ntail";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(360.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(675.), px(360.)), |_, _| {
                    editor.clone().into_any_element()
                });
            }
        };
        let folded = |cx: &gpui::VisualTestContext| {
            editor.read_with(cx, |editor, _| editor.folded.clone())
        };
        let place = |offset: usize, cx: &mut gpui::VisualTestContext| {
            retained
                .borrow_mut()
                .edit(&key, |buffer| buffer.select(offset, offset));
            draw(cx);
        };
        draw(cx);
        assert!(folded(cx).is_empty());

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-/");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0]));

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-j");
        draw(cx);
        assert!(folded(cx).is_empty());

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-8");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([7]));
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-9");
        draw(cx);
        assert!(folded(cx).is_empty());

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-0");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0, 4, 7]));

        place(source.find("const b").unwrap(), cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-=");
        draw(cx);
        assert_eq!(
            folded(cx),
            BTreeSet::from([4]),
            "unfold all except selected leaves the caret region folded"
        );
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd--");
        draw(cx);
        assert_eq!(
            folded(cx),
            BTreeSet::from([0, 4, 7]),
            "fold all except selected folds every unrelated region"
        );

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-j");
        draw(cx);
        cx.simulate_keystrokes("f1");
        draw(cx);
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("block comments".into(), cx));
        });
        draw(cx);
        editor.read_with(cx, |editor, cx| {
            assert_eq!(editor.palette_items(cx), vec!["Fold All Block Comments"]);
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(folded(cx), BTreeSet::from([0]));
    }

    #[gpui::test]
    fn format_document_reindents_typescript_and_undoes_as_one_edit(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "format.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function run() {\nreturn 1;\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("shift-alt-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function run() {\n  return 1;\n}\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function run() {\nreturn 1;\n}\n"
        );
    }

    #[gpui::test]
    fn format_selection_reindents_only_selected_lines_and_undoes_as_one_edit(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "selection.ts".into());
        let source = "function run() {\nconst a = 1;\nconst b = 2;\nreturn a + b;\n}\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let start = source.find("const a").unwrap();
        let end = source.find("return").unwrap();
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(start, end));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function run() {\n  const a = 1;\n  const b = 2;\nreturn a + b;\n}\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), source);
    }

    #[gpui::test]
    fn format_selection_with_a_caret_formats_the_current_line(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "caret.ts".into());
        let source = "function run() {\nconst a = 1;\nreturn a;\n}\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let caret = source.find("const a").unwrap() + 3;
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(caret, caret));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function run() {\n  const a = 1;\nreturn a;\n}\n"
        );
    }

    #[gpui::test]
    fn format_selection_formats_all_cursors_as_one_undo(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "multi-format.ts".into());
        let source = "function run() {\nconst a = 1;\nconst b = 2;\nreturn a + b;\n}\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let first = source.find("const a").unwrap()..source.find("\nconst b").unwrap();
        let second = source.find("const b").unwrap()..source.find("\nreturn").unwrap();
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(first.start, first.end));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        editor.update(cx, |editor, _| editor.extra_cursors = vec![second.clone()]);
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function run() {\n  const a = 1;\n  const b = 2;\nreturn a + b;\n}\n"
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.selection_ranges().len(), 2);
            assert_eq!(editor.extra_cursors.len(), 1);
        });
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), source);
    }

    #[gpui::test]
    fn format_selection_merges_same_line_cursors_without_losing_them(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "same-line.ts".into());
        let source = "const a=1,b=2;\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let primary = source.find('a').unwrap();
        let secondary = source.find('b').unwrap();
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(primary, primary));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        editor.update(cx, |editor, _| {
            editor.extra_cursors = std::iter::once(secondary..secondary).collect()
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const a = 1, b = 2;\n"
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.selection_ranges().len(), 2);
            assert_eq!(editor.extra_cursors.len(), 1);
        });
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), source);
    }

    #[gpui::test]
    fn format_document_indents_tsx_and_undoes_as_one_edit(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "component.tsx".into());
        let source = "function App(){\nreturn (\n<div>\n<span>Hi</span>\n</div>\n);\n}\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("shift-alt-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function App() {\n  return (\n    <div>\n      <span>Hi</span>\n    </div>\n  );\n}\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), source);
    }

    #[gpui::test]
    fn format_selection_reindents_tsx_against_surrounding_jsx(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "range.tsx".into());
        let source = "function App() {\nreturn (\n<div>\n<span>Hi</span>\n</div>\n);\n}\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let start = source.find("<div>").unwrap();
        let end = source.find("</div>").unwrap() + "</div>".len();
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(start, end));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function App() {\nreturn (\n    <div>\n      <span>Hi</span>\n    </div>\n);\n}\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), source);
    }

    #[gpui::test]
    fn format_selection_reindents_html_against_parent_depth(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "page.html".into());
        let source = "<html>\n<body>\n<div>\n<span>hi</span>\n</div>\n</body>\n</html>\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let start = source.find("<div>").unwrap();
        let end = source.find("</div>").unwrap() + "</div>".len();
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(start, end));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "<html>\n<body>\n    <div>\n      <span>hi</span>\n    </div>\n</body>\n</html>\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), source);
    }

    #[gpui::test]
    fn multi_cursor_adds_occurrences_types_and_undoes_as_one_step(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "multi.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("foo foo foo\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let press = |cx: &mut gpui::VisualTestContext, key: &str| {
            cx.simulate_keystrokes(key);
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 3));
        draw(cx);
        press(cx, "cmd-d");
        press(cx, "cmd-d");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.selection_ranges(), vec![0..3, 4..7, 8..11]);
            assert_eq!(editor.extra_cursors, vec![4..7, 8..11]);
        });
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "x", window, cx)
        });
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "x x x\n"
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![3..3, 5..5]);
            assert!(editor.selection_ranges()[0].is_empty());
        });
        press(cx, "cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "foo foo foo\n"
        );
        editor.read_with(cx, |editor, _| assert!(editor.extra_cursors.is_empty()));
    }

    #[gpui::test]
    fn multi_cursor_select_all_occurrences_uses_find_options_and_one_undo_step(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "select-all.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("foo foo Foo\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 3));
        draw(cx);
        cx.simulate_keystrokes("cmd-shift-l");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.selection_ranges(), vec![0..3, 4..7, 8..11]);
            assert_eq!(editor.extra_cursors, vec![4..7, 8..11]);
        });
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "x", window, cx)
        });
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "x x x\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "foo foo Foo\n"
        );
    }

    #[gpui::test]
    fn multi_cursor_select_all_from_collapsed_caret_is_case_sensitive_whole_word(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "select-word.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("Foo foo foobar foo\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(5, 5));
        draw(cx);
        cx.simulate_keystrokes("cmd-shift-l");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.selection_ranges(), vec![4..7, 15..18]);
            assert_eq!(editor.extra_cursors, vec![15..18]);
        });
    }

    #[test]
    fn previous_find_match_searches_before_the_earliest_selection_without_wrapping() {
        let text = "foo foo foo foo";
        let one =
            |range: std::ops::Range<usize>| std::iter::once(range).collect::<Vec<_>>();
        assert_eq!(previous_occurrence(text, &one(4..7), "foo"), Some(0..3));
        assert_eq!(previous_occurrence(text, &one(8..11), "foo"), Some(4..7));
        assert_eq!(
            previous_occurrence(text, &[8..11, 4..7], "foo"),
            Some(0..3),
            "the earliest current selection bounds the backward search"
        );
        assert_eq!(previous_occurrence(text, &one(0..3), "foo"), None);
        assert_eq!(previous_occurrence(text, &[], "foo"), None);
        assert_eq!(previous_occurrence(text, &one(4..7), ""), None);
        assert!(
            palette_matches("previous find match")
                .contains(&"Add Selection to Previous Find Match")
        );
    }

    #[test]
    fn next_find_match_search_uses_the_latest_selection_without_wrapping() {
        let text = "foo foo foo foo";
        let one =
            |range: std::ops::Range<usize>| std::iter::once(range).collect::<Vec<_>>();
        assert_eq!(next_occurrence_after(text, &one(0..3), "foo"), Some(4..7));
        assert_eq!(
            next_occurrence_after(text, &[4..7, 8..11], "foo"),
            Some(12..15),
            "the latest selection end bounds the forward search"
        );
        assert_eq!(next_occurrence_after(text, &one(12..15), "foo"), None);
        assert_eq!(next_occurrence_after(text, &[], "foo"), None);
        assert_eq!(next_occurrence_after(text, &one(0..3), ""), None);
        assert!(palette_matches("add selection to next").contains(
            &"Add Selection to Next Find Match"
        ));
        assert!(palette_matches("move last selection to next").contains(
            &"Move Last Selection to Next Find Match"
        ));
        assert!(palette_matches("move last selection to previous").contains(
            &"Move Last Selection to Previous Find Match"
        ));
    }

    #[test]
    fn add_cursors_to_line_ends_matches_monaco_selection_rules() {
        let text = "alpha\nbeta\ngamma\n";
        let starts = [0, 6, 11, 17];
        let one =
            |range: std::ops::Range<usize>| std::iter::once(range).collect::<Vec<_>>();
        assert_eq!(
            line_end_cursor_targets(&starts, text.len(), &one(2..2)),
            Vec::<usize>::new(),
            "collapsed selections produce no cursors"
        );
        assert_eq!(
            line_end_cursor_targets(&starts, text.len(), &one(2..4)),
            vec![4],
            "a single-line selection collapses to its end column"
        );
        assert_eq!(
            line_end_cursor_targets(&starts, text.len(), &one(0..6)),
            vec![5],
            "an end column of 1 does not add a cursor on the final line"
        );
        assert_eq!(
            line_end_cursor_targets(&starts, text.len(), &one(1..16)),
            vec![5, 10, 16]
        );
        assert_eq!(
            line_end_cursor_targets(&starts, text.len(), &one(0..8)),
            vec![5, 8]
        );
        assert_eq!(
            line_end_cursor_targets(&starts, text.len(), &[2..4, 6..11]),
            vec![4, 10]
        );
        assert!(palette_matches("line ends").contains(&"Add Cursors to Line Ends"));
    }

    #[gpui::test]
    fn expand_line_selection_shortcut_grows_whole_lines_for_all_cursors(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        assert_eq!(expand_line_range("one\ntwo\nthree", 1..1), 0..4);
        assert_eq!(expand_line_range("one\ntwo\nthree", 0..4), 0..8);
        assert_eq!(expand_line_range("one\ntwo\nthree", 0..8), 0..13);
        assert_eq!(expand_line_range("one\ntwo\nthree", 5..10), 4..13);

        let key = DocumentKey::new("/fixture".into(), "expand-line.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("one\ntwo\nthree".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(1, 1));
        draw(cx);
        cx.simulate_keystrokes("cmd-l");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            0..4
        );
        cx.simulate_keystrokes("cmd-l");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            0..8
        );
        cx.simulate_keystrokes("cmd-l");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            0..13
        );

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(5, 10));
        editor.update(cx, |editor, _| {
            editor.extra_cursors = std::iter::once(9..9).collect();
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-l");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            4..13
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![8..13]);
        });
    }

    #[gpui::test]
    fn add_selection_to_previous_find_match_palette_expands_leftward(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "previous-match.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("foo foo foo".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let palette = |cx: &mut gpui::VisualTestContext| {
            cx.simulate_keystrokes("f1");
            draw(cx);
            editor.update_in(cx, |editor, _window, cx| {
                editor.palette_query.update(cx, |input, cx| {
                    input.set_text("previous find match".into(), cx)
                });
            });
            draw(cx);
            cx.simulate_keystrokes("enter");
            draw(cx);
        };

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(8, 11));
        draw(cx);
        palette(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            8..11
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![4..7]);
        });

        palette(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![0..3, 4..7]);
        });

        palette(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(
                editor.extra_cursors,
                vec![0..3, 4..7],
                "the backward search does not wrap"
            );
        });

        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(9, 9));
        draw(cx);
        palette(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            8..11,
            "a collapsed caret first selects its word"
        );
        editor.read_with(cx, |editor, _| assert!(editor.extra_cursors.is_empty()));
    }

    #[gpui::test]
    fn add_selection_to_next_find_match_palette_expands_rightward(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "next-match.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("foo foo foo foo".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let palette = |cx: &mut gpui::VisualTestContext| {
            cx.simulate_keystrokes("f1");
            draw(cx);
            editor.update_in(cx, |editor, _window, cx| {
                editor.palette_query.update(cx, |input, cx| {
                    input.set_text("add selection to next".into(), cx)
                });
            });
            draw(cx);
            cx.simulate_keystrokes("enter");
            draw(cx);
        };

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 3));
        draw(cx);
        palette(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![4..7]);
        });
        palette(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![4..7, 8..11]);
        });
        palette(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![4..7, 8..11, 12..15]);
        });
        palette(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(
                editor.extra_cursors,
                vec![4..7, 8..11, 12..15],
                "the forward search does not wrap"
            );
        });

        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(5, 5));
        draw(cx);
        palette(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            4..7,
            "a collapsed caret first selects its word"
        );
        editor.read_with(cx, |editor, _| assert!(editor.extra_cursors.is_empty()));
        palette(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![8..11]);
        });
    }

    #[gpui::test]
    fn move_last_selection_to_find_match_shortcuts_and_palette(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "move-match.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("foo foo foo foo".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let extras =
            |range: std::ops::Range<usize>| std::iter::once(range).collect::<Vec<_>>();
        let palette = |query: &str, cx: &mut gpui::VisualTestContext| {
            cx.simulate_keystrokes("f1");
            draw(cx);
            editor.update_in(cx, |editor, _window, cx| {
                editor
                    .palette_query
                    .update(cx, |input, cx| input.set_text(query.into(), cx));
            });
            draw(cx);
            cx.simulate_keystrokes("enter");
            draw(cx);
        };

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 3));
        editor.update(cx, |editor, _| editor.extra_cursors = extras(4..7));
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-d");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            0..3
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![8..11]);
        });

        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-d");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![12..15]);
        });

        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 3));
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-d");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            4..7,
            "a lone primary selection moves itself"
        );
        editor.read_with(cx, |editor, _| assert!(editor.extra_cursors.is_empty()));

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(8, 11));
        draw(cx);
        palette("move last selection to previous", cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            4..7
        );
        editor.read_with(cx, |editor, _| assert!(editor.extra_cursors.is_empty()));

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(4, 7));
        editor.update(cx, |editor, _| editor.extra_cursors = extras(8..11));
        draw(cx);
        palette("move last selection to previous", cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            0..3
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(
                editor.extra_cursors,
                vec![8..11],
                "moving the earliest primary preserves later selections"
            );
        });

        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(5, 5));
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-d");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            4..7,
            "a collapsed caret first selects its word"
        );
    }

    #[gpui::test]
    fn add_cursors_to_line_ends_shortcut_replaces_selections_with_line_cursors(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "line-ends.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("alpha\nbeta\ngamma\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(1, 16));
        draw(cx);
        cx.simulate_keystrokes("shift-alt-i");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 5);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![10..10, 16..16]);
        });

        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(2, 4));
        draw(cx);
        cx.simulate_keystrokes("shift-alt-i");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 4);
        editor.read_with(cx, |editor, _| assert!(editor.extra_cursors.is_empty()));

        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(6, 6));
        draw(cx);
        cx.simulate_keystrokes("shift-alt-i");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 6);
        editor.read_with(cx, |editor, _| assert!(editor.extra_cursors.is_empty()));
    }

    #[test]
    fn add_cursors_to_top_and_bottom_replicate_primary_columns() {
        let text = "alpha\nbeta\ngamma";
        let starts = [0, 6, 11];
        assert_eq!(
            line_edge_cursor_targets(text, &starts, 2..4, true),
            vec![2..4, 8..10, 13..15]
        );
        assert_eq!(
            line_edge_cursor_targets(text, &starts, 2..4, false),
            vec![2..4],
            "the first line has no lines above it"
        );
        assert_eq!(
            line_edge_cursor_targets(text, &starts, 8..10, false),
            vec![8..10, 2..4],
            "top cursors are generated current-line first"
        );
        assert_eq!(
            line_edge_cursor_targets(text, &starts, 4..4, true),
            vec![4..4, 10..10, 15..15],
            "short lines clamp columns to the line end"
        );
        assert_eq!(
            line_edge_cursor_targets(text, &starts, 2..8, true),
            vec![2..2, 8..8, 13..13],
            "a multi-line primary replicates its column pair"
        );
        assert!(palette_matches("cursors to top").contains(&"Add Cursors to Top"));
        assert!(palette_matches("cursors to bottom").contains(&"Add Cursors to Bottom"));
    }

    #[gpui::test]
    fn add_cursors_to_top_and_bottom_palette_commands_replicate_columns(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "edge-cursors.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("alpha\nbeta\ngamma".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let palette = |query: &str, cx: &mut gpui::VisualTestContext| {
            cx.simulate_keystrokes("f1");
            draw(cx);
            editor.update_in(cx, |editor, _window, cx| {
                editor
                    .palette_query
                    .update(cx, |input, cx| input.set_text(query.into(), cx));
            });
            draw(cx);
            cx.simulate_keystrokes("enter");
            draw(cx);
        };

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(2, 4));
        draw(cx);
        palette("cursors to bottom", cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            2..4
        );
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![8..10, 13..15]);
        });

        editor.update(cx, |editor, _| editor.extra_cursors.clear());
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(8, 10));
        draw(cx);
        palette("cursors to top", cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            8..10
        );
        editor.read_with(cx, |editor, _| assert_eq!(editor.extra_cursors, vec![2..4]));
    }

    #[gpui::test]
    fn multi_cursor_add_above_and_below_duplicates_selections(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "add-cursor.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("one\ntwo\nthree\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(7, 7));
        draw(cx);
        cx.simulate_keystrokes("cmd-alt-down");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![11..11]);
        });
        cx.simulate_keystrokes("cmd-alt-up");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.selection_ranges(), vec![3..3, 7..7, 11..11]);
        });
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "X", window, cx)
        });
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "oneX\ntwoX\nthrXee\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "one\ntwo\nthree\n"
        );

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(4, 7));
        draw(cx);
        cx.simulate_keystrokes("cmd-alt-down");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.extra_cursors, vec![8..11]);
        });
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "x", window, cx)
        });
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "one\nx\nxee\n"
        );
    }

    #[gpui::test]
    fn problem_navigation_shortcuts_visit_and_wrap_diagnostics(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "problems.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("aaa\nbbb\nccc\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        editor.update(cx, |editor, cx| {
            editor.set_validation_diagnostics(
                vec![
                    crate::diagnostics::Diagnostic {
                        source: "W3C",
                        kind: crate::diagnostics::DiagnosticKind::Validation,
                        line: 1,
                        column: 2,
                        message: "first".into(),
                    },
                    crate::diagnostics::Diagnostic {
                        source: "W3C",
                        kind: crate::diagnostics::DiagnosticKind::Validation,
                        line: 3,
                        column: 2,
                        message: "second".into(),
                    },
                ],
                cx,
            );
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        draw(cx);
        cx.simulate_keystrokes("f8");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 1);
        cx.simulate_keystrokes("f8");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 9);
        cx.simulate_keystrokes("f8");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.caret(),
            1,
            "next wraps to the first diagnostic"
        );
        cx.simulate_keystrokes("shift-f8");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.caret(),
            9,
            "previous wraps to the last diagnostic"
        );
    }

    #[gpui::test]
    fn indent_and_outdent_shortcuts_shift_lines_and_follow_the_caret(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "indent.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("a\nb\nc\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        draw(cx);
        cx.simulate_keystrokes("cmd-]");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "  a\nb\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 2);
        cx.simulate_keystrokes("cmd-[");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 0);

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 4));
        draw(cx);
        cx.simulate_keystrokes("cmd-]");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "  a\n  b\nc\n"
        );
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            0..8
        );
    }

    #[gpui::test]
    fn command_palette_runs_sort_and_reverse_commands(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "palette.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents
            .borrow_mut()
            .open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("b\na\nc\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("f1");
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(editor.palette_open));
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("ascending".into(), cx));
        });
        draw(cx);
        editor.read_with(cx, |editor, cx| {
            assert_eq!(editor.palette_items(cx), vec!["Sort Lines Ascending"]);
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\nc\n"
        );

        cx.simulate_keystrokes("f1");
        draw(cx);
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("reverse".into(), cx));
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "c\nb\na\n"
        );
    }

    #[gpui::test]
    fn command_palette_runs_case_and_transpose_commands(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "palette-case.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("hello world".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 5));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("f1");
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("uppercase".into(), cx));
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "HELLO world"
        );

        cx.simulate_keystrokes("f1");
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("title case".into(), cx));
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "Hello world"
        );

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(6, 6));
        cx.simulate_keystrokes("f1");
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("transpose".into(), cx));
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "Hellow orld"
        );
    }

    #[gpui::test]
    fn command_palette_runs_snake_camel_pascal_and_kebab_case(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "palette-word-case.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        let source = "HelloWorld_test value";
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let mut run_case = |query: &str, expected: &str| {
            retained.borrow_mut().edit(&key, |buffer| {
                let length = buffer.text().len();
                buffer.replace(0..length, source);
                buffer.select(0, source.len());
            });
            draw(cx);
            cx.simulate_keystrokes("f1");
            editor.update_in(cx, |editor, _window, cx| {
                editor
                    .palette_query
                    .update(cx, |input, cx| input.set_text(query.into(), cx));
            });
            draw(cx);
            cx.simulate_keystrokes("enter");
            draw(cx);
            assert_eq!(
                retained.borrow().get(&key).unwrap().buffer.text(),
                expected
            );
        };
        run_case("snake case", "hello_world_test value");
        run_case("camel case", "helloWorldTestValue");
        run_case("pascal case", "HelloWorldTestValue");
        run_case("kebab case", "hello-world-test value");
    }

    #[gpui::test]
    fn tab_and_shift_tab_indent_lines_and_jump_to_tab_stops(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "tab.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("a\nb\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 4));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "  a\n  b\n"
        );
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\n"
        );

        retained.borrow_mut().edit(&key, |buffer| {
            let length = buffer.text().len();
            buffer.replace(0..length, "ab");
            buffer.select(1, 1);
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a b"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 2);

        retained.borrow_mut().edit(&key, |buffer| {
            let length = buffer.text().len();
            buffer.replace(0..length, "ab");
            buffer.select(0, 1);
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "  b"
        );
    }

    #[gpui::test]
    fn command_palette_runs_duplicate_and_select_to_bracket(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "palette-extra.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("abc\ndef".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 2));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("f1");
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("duplicate selection".into(), cx));
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "ababc\ndef"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.selection(), 2..4);

        retained.borrow_mut().edit(&key, |buffer| {
            let length = buffer.text().len();
            buffer.replace(0..length, "call(a)");
            buffer.select(4, 4);
        });
        draw(cx);
        cx.simulate_keystrokes("f1");
        editor.update_in(cx, |editor, _window, cx| {
            editor
                .palette_query
                .update(cx, |input, cx| input.set_text("select to bracket".into(), cx));
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            4..7
        );
    }

    #[gpui::test]
    fn go_to_symbol_in_file_filters_and_jumps(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "symbols-in-file.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        let source = "function alpha() {}\nconst beta = 1;\nclass Gamma {}";
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-shift-o");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.symbols_open);
            assert!(editor.symbols_filter_active);
            assert!(editor.symbols.len() >= 3);
        });
        for ch in ["b", "e", "t", "a"] {
            cx.simulate_keystrokes(ch);
        }
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.symbols.len(), 1);
            assert!(editor.symbols[0].preview.contains("beta"));
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(!editor.symbols_open));
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.caret(),
            source.find("beta").unwrap()
        );
    }

    #[gpui::test]
    fn workspace_symbols_filter_accepts_typing_and_jumps(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "workspace-symbols.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        let source = "function alpha() {}\nconst beta = 1;";
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        let seen = Rc::new(std::cell::Cell::new(None::<(DocumentKey, usize)>));
        let sink = seen.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&editor, move |_, event: &OpenSymbol, _| {
                sink.set(Some((event.key.clone(), event.offset)));
            })
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| {
                let caret = source.find("beta").unwrap() + 1;
                buffer.select(caret, caret);
            });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-t");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.workspace_symbols_open);
            assert!(editor.workspace_symbols_filter_active);
        });
        for ch in ["a", "l", "p", "h", "a"] {
            cx.simulate_keystrokes(ch);
        }
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.workspace_symbols.len(), 1);
            assert_eq!(editor.workspace_symbols[0].name, "alpha");
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(!editor.workspace_symbols_open));
        assert_eq!(
            seen.take(),
            Some((key, source.find("alpha").unwrap()))
        );
    }

    #[gpui::test]
    fn remove_brackets_shortcut_deletes_the_matching_pair(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "remove-brackets.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("call(a)".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(5, 5));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-alt-backspace");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "calla"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 4);
    }

    #[gpui::test]
    fn go_to_bracket_shortcut_jumps_to_partner_and_next_bracket(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "brackets.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("call(a[b]{c})".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(4, 4));
        draw(cx);
        cx.simulate_keystrokes("cmd-shift-\\");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 12);
        cx.simulate_keystrokes("cmd-shift-\\");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 4);

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        draw(cx);
        cx.simulate_keystrokes("cmd-shift-\\");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 4);
        cx.simulate_keystrokes("cmd-shift-\\");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 12);
    }

    #[gpui::test]
    fn delete_all_shortcuts_follow_line_boundaries(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "delete-all.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents
            .borrow_mut()
            .open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("abc\ndef".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(1, 1));
        draw(cx);
        cx.simulate_keystrokes("cmd-backspace");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "bc\ndef"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 0);

        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(3, 3));
        draw(cx);
        cx.simulate_keystrokes("ctrl-k");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "abcdef"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 3);

        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(4, 4));
        draw(cx);
        cx.simulate_keystrokes("cmd-backspace");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "abcdef"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 3);

        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(3, 3));
        draw(cx);
        cx.simulate_keystrokes("cmd-delete");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "abcdef"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 3);
    }

    #[gpui::test]
    fn trim_trailing_whitespace_chord_skips_regex_literals(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "trim-regex.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("const re = /abc  \nconst x = 1  ".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-x");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const re = /abc  \nconst x = 1"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 0);
    }

    #[gpui::test]
    fn trim_trailing_whitespace_chord_trims_the_document(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "trim.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("a  \nb\t\nc  ".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        draw(cx);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_keystrokes("cmd-x");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\nc"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 0);
    }

    #[gpui::test]
    fn join_lines_shortcut_joins_and_moves_the_caret(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "join.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("hello\nworld".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(1, 1));
        draw(cx);
        cx.simulate_keystrokes("ctrl-j");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "hello world"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 5);

        cx.simulate_keystrokes("ctrl-j");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "hello world",
            "joining on the last line is a no-op"
        );
    }

    #[gpui::test]
    fn insert_line_auto_indents_after_opening_lines(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "indent-insert.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("  if (ok) {\n  x\n}".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(11, 11));
        draw(cx);
        cx.simulate_keystrokes("cmd-enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "  if (ok) {\n    \n  x\n}"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 16);
    }

    #[gpui::test]
    fn move_line_reindents_into_the_destination_context(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "indent-move.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("if (x) {\n}\nb\nc\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(11, 11));
        draw(cx);
        cx.simulate_keystrokes("alt-up");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "if (x) {\n  b\n}\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 9);
    }

    #[gpui::test]
    fn insert_and_delete_line_shortcuts_use_monaco_bindings(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "lines.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("a\nb\nc\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(2, 2));
        draw(cx);
        cx.simulate_keystrokes("cmd-enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\n\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 4);

        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\nc\n"
        );
        cx.simulate_keystrokes("cmd-shift-enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\n\nb\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 2);

        cx.simulate_keystrokes("cmd-shift-k");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\nc\n"
        );
        cx.simulate_keystrokes("cmd-shift-k");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 2);
    }

    #[gpui::test]
    fn copy_line_shortcuts_duplicate_around_the_caret_line(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "copy.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("first\nsecond\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(2, 2));
        draw(cx);
        cx.simulate_keystrokes("shift-alt-down");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "first\nfirst\nsecond\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 8);

        cx.simulate_keystrokes("shift-alt-up");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "first\nfirst\nfirst\nsecond\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 8);
    }

    #[gpui::test]
    fn move_line_shortcuts_reorder_the_caret_line_and_follow_it(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "move.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("a\nb\nc\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(2, 2));
        draw(cx);
        cx.simulate_keystrokes("alt-down");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nc\nb\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 4);

        cx.simulate_keystrokes("alt-up");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "a\nb\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 2);

        cx.simulate_keystrokes("alt-up");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "b\na\nc\n"
        );
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 0);

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(4, 5));
        draw(cx);
        cx.simulate_keystrokes("alt-up");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "b\nc\na\n"
        );
        let selection = retained.borrow().get(&key).unwrap().buffer.selection();
        assert_eq!(selection, 2..3);
    }

    #[gpui::test]
    fn goto_line_shortcut_opens_moves_caret_and_escape_closes(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "goto.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("line 1\nline 2\nline 3\nline 4\nline 5\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-g");
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(editor.goto_open));

        editor.update_in(cx, |editor, _window, cx| {
            editor
                .goto_input
                .update(cx, |input, cx| input.set_text("3:6".into(), cx));
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(!editor.goto_open));
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 19);

        cx.simulate_keystrokes("ctrl-g");
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(editor.goto_open));
        cx.simulate_keystrokes("escape");
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(!editor.goto_open));
    }

    #[gpui::test]
    fn multi_cursor_backspace_deletes_all_selections_and_escape_clears_extras(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "multi-delete.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("foo foo foo\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let press = |cx: &mut gpui::VisualTestContext, key: &str| {
            cx.simulate_keystrokes(key);
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 3));
        draw(cx);
        press(cx, "cmd-d");
        press(cx, "cmd-d");
        press(cx, "backspace");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), "  \n");
        press(cx, "cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "foo foo foo\n"
        );
        press(cx, "cmd-d");
        press(cx, "escape");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.extra_cursors.is_empty());
            assert_eq!(editor.selection_ranges(), vec![0..3]);
        });
    }

    #[gpui::test]
    fn workspace_references_are_file_qualified_and_jump_across_documents(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let a = DocumentKey::new("/fixture".into(), "a.js".into());
        let b = DocumentKey::new("/fixture".into(), "b.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, a.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &a.rel, Some("function foo() {}\nfoo();\n".into()));
        documents.borrow_mut().open(2, b.clone(), true);
        documents
            .borrow_mut()
            .finish_load(2, &b.rel, Some("foo();\n".into()));
        documents.borrow_mut().activate(&a);
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(a.clone(), documents.clone(), cx);
            window.focus(&editor.focus, cx);
            editor
        });
        documents
            .borrow_mut()
            .edit(&a, |buffer| buffer.select(9, 9));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("shift-f12");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.workspace_symbols_open);
            assert!(editor.workspace_symbols_references);
            assert_eq!(editor.workspace_symbols_title, "References for foo");
            assert_eq!(editor.workspace_symbols.len(), 3);
            assert_eq!(editor.workspace_symbols[0].key, a);
            assert_eq!(editor.workspace_symbols[2].key, b);
        });
        assert!(cx.debug_bounds("editor-workspace-symbols").is_some());
        assert!(cx.debug_bounds("editor-workspace-symbol-0").is_some());
        assert!(cx.debug_bounds("editor-workspace-symbol-2").is_some());

        let seen = Rc::new(std::cell::Cell::new(None::<(DocumentKey, usize)>));
        let sink = seen.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&editor, move |_, event: &OpenSymbol, _| {
                sink.set(Some((event.key.clone(), event.offset)));
            })
        });
        cx.simulate_keystrokes("down down enter");
        draw(cx);
        assert_eq!(seen.take(), Some((b, 0)));
        editor.read_with(cx, |editor, _| assert!(!editor.workspace_symbols_open));
    }

    #[gpui::test]
    fn go_to_definition_falls_back_to_another_open_document(cx: &mut gpui::TestAppContext) {
        use super::*;
        let a = DocumentKey::new("/fixture".into(), "caller.js".into());
        let b = DocumentKey::new("/fixture".into(), "definition.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, a.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &a.rel, Some("gamma();\n".into()));
        documents.borrow_mut().open(2, b.clone(), true);
        documents
            .borrow_mut()
            .finish_load(2, &b.rel, Some("function gamma() {}\n".into()));
        documents.borrow_mut().activate(&a);
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(a.clone(), documents.clone(), cx);
            window.focus(&editor.focus, cx);
            editor
        });
        documents
            .borrow_mut()
            .edit(&a, |buffer| buffer.select(1, 1));
        let seen = Rc::new(std::cell::Cell::new(None::<(DocumentKey, usize)>));
        let sink = seen.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&editor, move |_, event: &OpenSymbol, _| {
                sink.set(Some((event.key.clone(), event.offset)));
            })
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("f12");
        draw(cx);
        assert_eq!(seen.take(), Some((b, 9)));
        editor.read_with(cx, |editor, _| {
            assert!(!editor.symbols_open);
            assert!(!editor.workspace_symbols_open);
        });
    }

    #[gpui::test]
    fn markup_fold_control_collapses_html_elements(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "page.html".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("<div>\n  <p>one</p>\n  <p>two</p>\n</div>\ntail".into()),
        );
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        let fold = cx.debug_bounds("editor-fold-0").expect("html fold control");
        cx.simulate_click(fold.center(), Modifiers::default());
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.folded.contains(&0));
            assert_eq!(editor.visible_lines, vec![0, 4]);
        });
        assert!(cx.debug_bounds("editor-row-1").is_none());
        assert!(cx.debug_bounds("editor-row-3").is_none());
        assert!(cx.debug_bounds("editor-row-4").is_some());
    }

    #[gpui::test]
    fn workspace_symbols_search_open_documents_and_emit_cross_file_targets(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let a = DocumentKey::new("/fixture".into(), "a.js".into());
        let b = DocumentKey::new("/fixture".into(), "b.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, a.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &a.rel,
            Some("gamma();\nfunction alpha() {}\n".into()),
        );
        documents.borrow_mut().open(2, b.clone(), true);
        documents
            .borrow_mut()
            .finish_load(2, &b.rel, Some("function gamma() {}\n".into()));
        documents.borrow_mut().activate(&a);
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(a.clone(), documents.clone(), cx);
            window.focus(&editor.focus, cx);
            editor
        });
        documents
            .borrow_mut()
            .edit(&a, |buffer| buffer.select(1, 1));
        let seen = Rc::new(std::cell::Cell::new(None::<(DocumentKey, usize)>));
        let sink = seen.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&editor, move |_, event: &OpenSymbol, _| {
                sink.set(Some((event.key.clone(), event.offset)));
            })
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-t");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.workspace_symbols_open);
            assert_eq!(editor.workspace_symbols.len(), 1);
            assert_eq!(editor.workspace_symbols[0].name, "gamma");
            assert_eq!(editor.workspace_symbols[0].key, b);
        });
        assert!(cx.debug_bounds("editor-workspace-symbols").is_some());
        assert!(cx.debug_bounds("editor-workspace-symbol-0").is_some());

        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(seen.take(), Some((b, 9)));
        editor.read_with(cx, |editor, _| assert!(!editor.workspace_symbols_open));
    }

    #[gpui::test]
    fn go_to_definition_and_find_references_use_document_symbols(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "symbols.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("function foo() {}\nfoo();\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(19, 19));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("f12");
        draw(cx);
        assert_eq!(
            retained
                .borrow()
                .get(&key)
                .unwrap()
                .buffer
                .selection()
                .start,
            9
        );
        editor.read_with(cx, |editor, _| assert!(!editor.symbols_open));

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(19, 19));
        draw(cx);
        let seen = Rc::new(std::cell::Cell::new(None::<(DocumentKey, usize)>));
        let sink = seen.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&editor, move |_, event: &OpenSymbol, _| {
                sink.set(Some((event.key.clone(), event.offset)));
            })
        });
        cx.simulate_keystrokes("shift-f12");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.workspace_symbols_open);
            assert!(editor.workspace_symbols_references);
            assert_eq!(editor.workspace_symbols.len(), 2);
            assert_eq!(editor.workspace_symbols_title, "References for foo");
        });
        assert!(cx.debug_bounds("editor-workspace-symbols").is_some());
        assert!(cx.debug_bounds("editor-workspace-symbol-0").is_some());
        assert!(cx.debug_bounds("editor-workspace-symbol-1").is_some());
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(seen.take(), Some((key.clone(), 9)));
        editor.read_with(cx, |editor, _| {
            assert!(!editor.workspace_symbols_open);
        });
    }

    #[gpui::test]
    fn region_fold_control_collapses_language_markers(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "region.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("// #region setup\nconst a = 1;\nconst b = 2;\n// #endregion\ntail".into()),
        );
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |editor, _| editor.visible_lines.clone()),
            (0..5).collect::<Vec<_>>()
        );
        let fold = cx
            .debug_bounds("editor-fold-0")
            .expect("region fold control");
        cx.simulate_click(fold.center(), Modifiers::default());
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.folded.contains(&0));
            assert_eq!(editor.visible_lines, vec![0, 4]);
        });
        assert!(cx.debug_bounds("editor-row-1").is_none());
        assert!(cx.debug_bounds("editor-row-3").is_none());
        assert!(cx.debug_bounds("editor-row-4").is_some());
    }

    #[gpui::test]
    fn validation_diagnostics_render_and_clear_after_edit(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "page.html".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("<div>ok</div>\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        editor.update(cx, |editor, cx| {
            editor.set_validation_diagnostics(
                vec![crate::diagnostics::Diagnostic {
                    source: "W3C",
                    kind: crate::diagnostics::DiagnosticKind::Validation,
                    line: 1,
                    column: 1,
                    message: "Bad markup".into(),
                }],
                cx,
            )
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert_eq!(
                editor.diagnostics[0].kind,
                crate::diagnostics::DiagnosticKind::Validation
            );
            assert_eq!(editor.diagnostics[0].source, "W3C");
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("<div>fixed</div>\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn unused_import_diagnostics_render_and_clear_after_use(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "imports.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("import { used, unused } from \"pkg\";\nconsole.log(used);\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(editor.diagnostics[0].message.contains("Import 'unused'"));
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import { used, unused } from \"pkg\";\nconsole.log(used, unused);\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn missing_return_diagnostics_render_and_clear_after_return(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "returns.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function greet(): string {\n  console.log('hi');\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("no return statement")
            );
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("function greet(): string {\n  return 'hi';\n}\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn quick_fixes_apply_unused_parameter_and_declaration_actions(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "quick.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function greet(name, age) {\n  return name;\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        assert!(cx.debug_bounds("editor-code-actions").is_some());
        assert!(cx.debug_bounds("editor-code-action-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert!(editor.code_actions_open);
            assert_eq!(editor.code_actions.len(), 1);
            assert!(editor.code_actions[0].title.contains("underscore"));
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function greet(name, _age) {\n  return name;\n}\n"
        );
        editor.read_with(cx, |editor, _| assert!(!editor.code_actions_open));
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("const unused = 1;\nconsole.log('x');\n");
            buffer.select(0, 0);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        assert!(cx.debug_bounds("editor-code-actions").is_some());
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "console.log('x');\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import { unused } from \"pkg\";\nconsole.log('x');\n");
            buffer.select(0, 0);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(
                editor.code_actions[0]
                    .title
                    .contains("Remove unused import")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "console.log('x');\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import { used, unused } from \"pkg\";\nconsole.log(used);\n");
            buffer.select(0, 0);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(
                editor.code_actions[0]
                    .title
                    .contains("Remove unused import")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import { used } from \"pkg\";\nconsole.log(used);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import Unused, { used } from \"pkg\";\nconsole.log(used);\n");
            buffer.select(0, 0);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(
                editor.code_actions[0]
                    .title
                    .contains("Remove unused import")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import { used } from \"pkg\";\nconsole.log(used);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import Default, * as unusedNs from \"pkg\";\nconsole.log(Default);\n");
            buffer.select(0, 0);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(
                editor.code_actions[0]
                    .title
                    .contains("Remove unused import")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import Default from \"pkg\";\nconsole.log(Default);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import {\n  used,\n  unused,\n} from \"pkg\";\nconsole.log(used);\n");
            let caret = buffer.text().find("unused").unwrap();
            buffer.select(caret, caret);
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(
                editor.code_actions[0]
                    .title
                    .contains("Remove unused import")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import {\n  used,\n} from \"pkg\";\nconsole.log(used);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import {\n  used, unused\n} from \"pkg\";\nconsole.log(used);\n");
            let caret = buffer.text().find("unused").unwrap();
            buffer.select(caret, caret);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import {\n  used\n} from \"pkg\";\nconsole.log(used);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert(
                "import Default,\n{\n  used,\n  unused,\n} from \"pkg\";\nconsole.log(Default, used);\n",
            );
            let caret = buffer.text().find("unused").unwrap();
            buffer.select(caret, caret);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import Default,\n{\n  used,\n} from \"pkg\";\nconsole.log(Default, used);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("import { used, unused }\nfrom \"pkg\";\nconsole.log(used);\n");
            let caret = buffer.text().find("unused").unwrap();
            buffer.select(caret, caret);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import { used }\nfrom \"pkg\";\nconsole.log(used);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn unused_parameter_diagnostics_render_and_clear_after_use(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "params.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function greet(name, age) {\n  return name;\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(editor.diagnostics[0].message.contains("Parameter 'age'"));
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("function greet(name, age) {\n  return name + age;\n}\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn unused_declaration_diagnostics_render_and_clear_after_use(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "unused.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("const answer = 42;\nconsole.log('ok');\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Unused declaration 'answer'")
            );
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("const answer = 42;\nconsole.log(answer);\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn lowercase_function_auto_import_inserts_a_named_import(cx: &mut gpui::TestAppContext) {
        use super::*;
        let app = DocumentKey::new("/fixture".into(), "lower-app.ts".into());
        let util = DocumentKey::new("/fixture".into(), "lower-util.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, app.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &app.rel,
            Some("export const value = helper();\n".into()),
        );
        documents.borrow_mut().open(2, util.clone(), true);
        documents.borrow_mut().finish_load(
            2,
            &util.rel,
            Some("export function helper() { return 1; }\n".into()),
        );
        documents.borrow_mut().activate(&app);
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(app.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&app, |buffer| {
            let caret = buffer.text().find("helper").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&app).unwrap().buffer.text(),
            "import { helper } from \"./lower-util\";\nexport const value = helper();\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_anonymous_function_type_parameter_offers_a_safe_removal(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-anon-type-param.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("export const f = function <T, T>(value: T) { return value; };\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("T, T").unwrap() + 3;
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(760.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(760.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate type parameter 'T'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export const f = function <T>(value: T) { return value; };\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_arrow_type_parameter_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-arrow-type-param.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("export const f = <T, T>(value: T) => value;\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("T, T").unwrap() + 3;
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate type parameter 'T'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export const f = <T>(value: T) => value;\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_type_parameter_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-type-param.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function f<T, T>(value: T) { return value; }\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("T, T").unwrap() + 3;
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate type parameter 'T'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate type parameter 'T'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function f<T>(value: T) { return value; }\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn nested_object_type_property_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-nested-type.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "type Options = {\n  outer: {\n    read: string;\n    write: string;\n    read: string;\n  };\n};\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(760.), px(400.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(760.), px(400.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-4").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate object type property 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "type Options = {\n  outer: {\n    read: string;\n    write: string;\n  };\n};\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn function_parameter_type_literal_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-param-type.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "function f(options: {\n  read: string;\n  write: string;\n  read: string;\n}) {\n  return options;\n}\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(760.), px(380.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(760.), px(380.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-3").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate object type property 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate object type property 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function f(options: {\n  read: string;\n  write: string;\n}) {\n  return options;\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn annotation_object_type_property_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-annotation-type.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "const options: {\n  read: string;\n  write: string;\n  read: string;\n} = value;\nconsole.log(options);\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(760.), px(380.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(760.), px(380.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-3").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate object type property 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate object type property 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const options: {\n  read: string;\n  write: string;\n} = value;\nconsole.log(options);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_object_type_property_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-type-literal.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "type Options = {\n  read: string;\n  write: string;\n  read: string;\n};\n".into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(360.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(360.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-3").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate object type property 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate object type property 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "type Options = {\n  read: string;\n  write: string;\n};\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_union_member_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-union.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("type Mode = \"read\" | \"write\" | \"read\";\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("\"read\"").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate union member '\"read\"'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate union member '\"read\"'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "type Mode = \"read\" | \"write\";\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_interface_property_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-interface.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "interface Reader {\n  read: string;\n  write: string;\n  read: string;\n}\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(360.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(360.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-3").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate interface property 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate interface property 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "interface Reader {\n  read: string;\n  write: string;\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_enum_member_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-enum.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("enum Direction {\n  Up = 1,\n  Down = 2,\n  Up = 1,\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("Up").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(360.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(360.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-3").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate enum member 'Up'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove later duplicate enum member 'Up'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "enum Direction {\n  Up = 1,\n  Down = 2,\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_switch_case_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-switch.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("switch (value) {\n  case 1:\n  case 1:\n  case 2:\n    work();\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("case 1").unwrap() + 5;
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(360.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(360.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-2").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate switch case label '1'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate switch case '1'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "switch (value) {\n  case 1:\n  case 2:\n    work();\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_multiline_class_member_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-multiline-class.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("class Reader {\n  read() { return 1; }\n  read() { return 2; }\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-2").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate class member 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "class Reader {\n  read() { return 2; }\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_class_expression_member_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-class-expr.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "const Reader = class { read() { return 1; } read() { return 2; } };\nconsole.log(Reader);\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(760.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(760.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate class member 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove earlier duplicate class member 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const Reader = class { read() { return 2; } };\nconsole.log(Reader);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_class_member_diagnostic_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-class.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("class Reader { read() { return 1; } read() { return 2; } }\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("read").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate class member 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove earlier duplicate class member 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "class Reader { read() { return 2; } }\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_destructuring_diagnostic_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-destructure.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("const { read, read } = source;\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("read }").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate destructuring binding 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate destructuring binding 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const { read } = source;\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_quoted_export_diagnostic_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-quoted-export.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "const read = 1;\nexport { read as \"value\", read as \"value\" };\nconsole.log(read);\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("\"value\"").unwrap() + 1;
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(760.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(760.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate export name 'value'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate export specifier 'value'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const read = 1;\nexport { read as \"value\" };\nconsole.log(read);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_export_diagnostic_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-export.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("const read = 1;\nexport { read, read };\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("read };").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate export name 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate export specifier 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const read = 1;\nexport { read };\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_quoted_import_diagnostic_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-quoted-import.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "import { \"a\" as read, \"a\" as read } from \"./pkg\";\nconsole.log(read);\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("as read }").unwrap() + 3;
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(760.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(760.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate import binding 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate import specifier 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import { \"a\" as read } from \"./pkg\";\nconsole.log(read);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_import_diagnostic_offers_a_safe_removal(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-import.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("import { read, read } from \"./io\";\nconsole.log(read);\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("read }").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate import binding 'read'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove duplicate import specifier 'read'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "import { read } from \"./io\";\nconsole.log(read);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn method_duplicate_parameter_offers_a_shadow_safe_quick_fix(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dup-method-param.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("class A { read(value, value) { return value; } }\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("value)").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(720.), px(340.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(720.), px(340.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate parameter 'value'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Prefix earlier duplicate 'value' with underscore"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "class A { read(_value, value) { return value; } }\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_parameter_diagnostic_offers_a_shadow_safe_quick_fix(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "params-dup.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function pick(value, value) {\n  return value;\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().find("value)").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate parameter 'value'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Prefix earlier duplicate 'value' with underscore"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function pick(_value, value) {\n  return value;\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn duplicate_object_key_diagnostic_offers_a_safe_quick_fix(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "objects.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some(
                "const options = {\n  size: 1,\n  color: 'red',\n  size: 2,\n};\nconsole.log(options);\n"
                    .into(),
            ),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("size").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-3").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate object key 'size'")
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(
                editor.code_actions[0].title,
                "Remove earlier duplicate key 'size'"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "const options = {\n  color: 'red',\n  size: 2,\n};\nconsole.log(options);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-3").is_none());
    }

    #[gpui::test]
    fn lowercase_arrow_auto_import_inserts_a_named_import(cx: &mut gpui::TestAppContext) {
        use super::*;
        let app = DocumentKey::new("/fixture".into(), "arrow-app.ts".into());
        let util = DocumentKey::new("/fixture".into(), "arrow-util.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, app.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &app.rel,
            Some("export const value = format();\n".into()),
        );
        documents.borrow_mut().open(2, util.clone(), true);
        documents.borrow_mut().finish_load(
            2,
            &util.rel,
            Some("export const format = (value: number) => `${value}`;\n".into()),
        );
        documents.borrow_mut().activate(&app);
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(app.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&app, |buffer| {
            let caret = buffer.text().find("format").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert!(editor.code_actions[0].title.contains("Import 'format'"));
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&app).unwrap().buffer.text(),
            "import { format } from \"./arrow-util\";\nexport const value = format();\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn auto_import_inserts_after_use_strict_and_refreshes_with_other_documents(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let app = DocumentKey::new("/fixture".into(), "strict-app.tsx".into());
        let widget = DocumentKey::new("/fixture".into(), "strict-widget.tsx".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, app.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &app.rel,
            Some("\"use strict\";\nexport const view = Widget();\n".into()),
        );
        documents.borrow_mut().open(2, widget.clone(), true);
        documents.borrow_mut().finish_load(
            2,
            &widget.rel,
            Some("export function Widget() { return null; }\n".into()),
        );
        documents.borrow_mut().activate(&app);
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(app.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&app, |buffer| {
            let caret = buffer.text().find("Widget").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&app).unwrap().buffer.text(),
            "\"use strict\";\nimport { Widget } from \"./strict-widget\";\nexport const view = Widget();\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&app, |buffer| {
            buffer.select_all();
            buffer.insert("\"use strict\";\nexport const view = Widget();\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        retained.borrow_mut().edit(&widget, |buffer| {
            buffer.select_all();
            buffer.insert("export function Other() { return null; }\n");
        });
        draw(cx);
        assert!(
            cx.debug_bounds("editor-diagnostics-banner").is_none(),
            "removing the export must refresh unresolved diagnostics"
        );
        retained.borrow_mut().edit(&widget, |buffer| {
            buffer.select_all();
            buffer.insert("export function Widget() { return null; }\n");
        });
        draw(cx);
        assert!(
            cx.debug_bounds("editor-diagnostics-banner").is_some(),
            "restoring the export must refresh unresolved diagnostics"
        );
    }

    #[gpui::test]
    fn unresolved_identifier_quick_fix_inserts_an_auto_import(cx: &mut gpui::TestAppContext) {
        use super::*;
        let app = DocumentKey::new("/fixture".into(), "app.tsx".into());
        let widget = DocumentKey::new("/fixture".into(), "widget.tsx".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, app.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &app.rel,
            Some("export const view = Widget();\n".into()),
        );
        documents.borrow_mut().open(2, widget.clone(), true);
        documents.borrow_mut().finish_load(
            2,
            &widget.rel,
            Some("export function Widget() { return null; }\n".into()),
        );
        documents.borrow_mut().activate(&app);
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(app.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&app, |buffer| {
            let caret = buffer.text().find("Widget").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        editor.read_with(cx, |editor, _| {
            assert!(
                editor
                    .diagnostics
                    .iter()
                    .any(|diagnostic| { diagnostic.message.contains("Cannot find name 'Widget'") })
            );
        });
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert!(editor.code_actions[0].title.contains("Import 'Widget'"));
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&app).unwrap().buffer.text(),
            "import { Widget } from \"./widget\";\nexport const view = Widget();\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn argument_count_quick_fix_fills_and_trims_calls(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "args-fix.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function f(a, b) { return a + b; }\nf(1);\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("f(1)").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(editor.code_actions[0].title, "Add 1 missing argument");
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function f(a, b) { return a + b; }\nf(1, undefined);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("function f(a, b) { return a + b; }\nf(1, 2, 3, 4);\n");
            let caret = buffer.text().rfind("f(1, 2, 3, 4)").unwrap();
            buffer.select(caret, caret);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions[0].title, "Remove 2 extra arguments");
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function f(a, b) { return a + b; }\nf(1, 2);\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn syntax_quick_fix_adds_or_removes_delimiters(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "syntax-fix.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("function f() {\n  work();\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 1);
            assert_eq!(editor.code_actions[0].title, "Add '}'");
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "function f() {\n  work();\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("export let x = 1;\n}\n");
            let caret = buffer.text().rfind('}').unwrap();
            buffer.select(caret, caret);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions[0].title, "Remove '}'");
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export let x = 1;\n\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn missing_return_quick_fix_changes_or_removes_the_annotation(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "returns-fix.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("export function greet(): string {\n  console.log('hi');\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 2);
            assert!(
                editor.code_actions[0]
                    .title
                    .contains("Change return type to 'void'")
            );
            assert_eq!(
                editor.code_actions[1].title,
                "Remove return type annotation"
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export function greet(): void {\n  console.log('hi');\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("export function greet(): string {\n  console.log('hi');\n}\n");
            buffer.select(0, 0);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("down enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export function greet() {\n  console.log('hi');\n}\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn type_mismatch_quick_fix_changes_or_removes_the_annotation(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "types-fix.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("export const name: string = 42;\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.code_actions.len(), 2);
            assert!(
                editor.code_actions[0]
                    .title
                    .contains("Change type to 'number'")
            );
            assert_eq!(editor.code_actions[1].title, "Remove type annotation");
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export const name: number = 42;\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("export const name: string = 42;\n");
            buffer.select(0, 0);
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        cx.simulate_keystrokes("down enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export const name = 42;\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_declaration_quick_fix_renames_the_second_declaration(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dupes-fix.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("export const value = 1;\nexport const value = 2;\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| {
            let caret = buffer.text().rfind("value").unwrap();
            buffer.select(caret, caret);
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        cx.simulate_keystrokes("cmd-.");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.code_actions[0].title.contains("Rename duplicate"));
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "export const value = 1;\nexport const value2 = 2;\n"
        );
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
    }

    #[gpui::test]
    fn duplicate_declaration_diagnostics_render_and_clear_after_fixing(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "dupes.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("const value = 1;\nconst value = 2;\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-1").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert_eq!(editor.diagnostics[0].source, "JavaScript");
            assert!(
                editor.diagnostics[0]
                    .message
                    .contains("Duplicate top-level")
            );
            assert!(editor.diagnostics[0].message.contains("line 1"));
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("const value = 1;\nconst other = value;\nconsole.log(other);\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-1").is_none());
    }

    #[gpui::test]
    fn typescript_literal_type_diagnostics_render_and_clear_after_fixing(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "types.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("const name: string = 42;\nname;\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert_eq!(editor.diagnostics[0].source, "TypeScript");
            assert!(editor.diagnostics[0].message.contains("not assignable"));
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("const name: string = \"ok\";\nname;\n");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn json_diagnostics_render_a_line_marker_and_clear_after_fixing(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "broken.json".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("{\n  \"a\": ,\n}\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-1").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert_eq!(editor.diagnostics[0].line, 2);
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("{\"a\": 1}");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-1").is_none());
    }

    #[gpui::test]
    fn javascript_diagnostics_render_and_clear_after_fixing(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "broken.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function f() {\n  return 1;\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_some());
        assert!(cx.debug_bounds("editor-error-marker-0").is_some());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.diagnostics.len(), 1);
            assert_eq!(editor.diagnostics[0].source, "JavaScript");
            assert!(editor.diagnostics[0].message.contains("Unclosed '{'"));
        });

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("function f() {}");
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-diagnostics-banner").is_none());
        assert!(cx.debug_bounds("editor-error-marker-0").is_none());
    }

    #[gpui::test]
    fn rename_validates_and_updates_open_workspace_documents(cx: &mut gpui::TestAppContext) {
        use super::*;
        use gpui::EntityInputHandler;
        let a = DocumentKey::new("/fixture".into(), "a.js".into());
        let b = DocumentKey::new("/fixture".into(), "b.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, a.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &a.rel, Some("function foo() {}\nfoo();\n".into()));
        documents.borrow_mut().open(2, b.clone(), true);
        documents
            .borrow_mut()
            .finish_load(2, &b.rel, Some("foo();\n".into()));
        documents.borrow_mut().activate(&a);
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(a.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&a, |buffer| buffer.select(9, 9));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("f2");
        draw(cx);
        let input = editor.read_with(cx, |editor, _| editor.rename_input.clone());
        editor.read_with(cx, |editor, _| {
            assert!(editor.rename_open);
            assert_eq!(editor.rename_original, "foo");
            assert_eq!(editor.rename_targets.len(), 2);
            assert_eq!(
                editor
                    .rename_targets
                    .iter()
                    .map(|(_, ranges)| ranges.len())
                    .sum::<usize>(),
                3
            );
        });
        assert!(cx.debug_bounds("editor-rename").is_some());
        assert!(cx.debug_bounds("editor-rename-input").is_some());

        input.update_in(cx, |input, window, cx| {
            input.replace_text_in_range(None, "1bad", window, cx)
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.rename_open);
            assert!(
                editor
                    .rename_error
                    .as_deref()
                    .is_some_and(|error| error.contains("letter"))
            );
        });
        assert_eq!(
            retained.borrow().get(&a).unwrap().buffer.text(),
            "function foo() {}\nfoo();\n"
        );

        input.update_in(cx, |input, window, cx| {
            input.buffer.select_all();
            input.replace_text_in_range(None, "bar", window, cx)
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&a).unwrap().buffer.text(),
            "function bar() {}\nbar();\n"
        );
        assert_eq!(retained.borrow().get(&b).unwrap().buffer.text(), "bar();\n");
        editor.read_with(cx, |editor, _| {
            assert!(!editor.rename_open);
            assert_eq!(editor.selection_ranges(), vec![12..12]);
        });

        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&a).unwrap().buffer.text(),
            "function foo() {}\nfoo();\n"
        );
        assert_eq!(
            retained.borrow().get(&b).unwrap().buffer.text(),
            "bar();\n",
            "cross-document undo remains document-local"
        );
    }

    #[test]
    fn enter_indent_follows_braces_python_colons_and_html_tags() {
        assert_eq!(
            enter_indent("app.ts", "if (ok) {", 9).as_deref(),
            Some("\n  ")
        );
        assert_eq!(
            enter_indent("app.ts", "if (ok) {}", 9).as_deref(),
            Some("\n  \n")
        );
        assert_eq!(
            enter_indent("app.ts", "  if (ok) {}", 11).as_deref(),
            Some("\n    \n  ")
        );
        assert_eq!(
            enter_indent("app.py", "if ok:", 6).as_deref(),
            Some("\n  ")
        );
        assert_eq!(
            enter_indent("page.html", "<div>", 5).as_deref(),
            Some("\n  ")
        );
        assert_eq!(
            enter_indent("page.html", "<div></div>", 5).as_deref(),
            Some("\n  \n")
        );
        assert_eq!(enter_indent("note.md", "text", 4), None);
        assert_eq!(enter_indent("config.yaml", "key: value", 10), None);
    }

    #[gpui::test]
    fn editor_wheel_scroll_animates_in_frames(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "scroll.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some((0..80).map(|line| format!("line {line}\n")).collect()),
        );
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(500.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(500.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        let before = editor.read_with(cx, |editor, _| editor.list.logical_scroll_top());
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: point(px(100.), px(100.)),
            delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-120.))),
            ..Default::default()
        });
        let immediate = editor.read_with(cx, |editor, _| editor.list.logical_scroll_top());
        assert_eq!(
            (immediate.item_ix, immediate.offset_in_item),
            (before.item_ix, before.offset_in_item),
            "native list scroll is rewound before the smooth animation"
        );
        cx.executor().advance_clock(Duration::from_millis(16));
        cx.run_until_parked();
        draw(cx);
        let first_frame = editor.read_with(cx, |editor, _| editor.list.logical_scroll_top());
        assert!(
            first_frame.item_ix > before.item_ix
                || first_frame.offset_in_item > before.offset_in_item,
            "the first animation frame advances the viewport"
        );
        cx.executor().advance_clock(Duration::from_millis(16 * 20));
        cx.run_until_parked();
        draw(cx);
        let after = editor.read_with(cx, |editor, _| editor.list.logical_scroll_top());
        assert!(
            after.item_ix >= 6,
            "the smooth animation converges on the native wheel target: {after:?}"
        );
    }

    #[test]
    fn comment_auto_close_only_applies_to_javascript_family() {
        assert!(comment_auto_close("x.ts", "/*", 2));
        assert!(comment_auto_close("x.jsx", "  /*", 4));
        assert!(!comment_auto_close("x.css", "/*", 2));
        assert!(!comment_auto_close("x.ts", "\"/*\"", 3));
    }

    fn apply_comment_edits(
        text: &str,
        edits: &[(std::ops::Range<usize>, String)],
    ) -> String {
        let mut output = text.to_owned();
        for (range, replacement) in edits.iter().rev() {
            output.replace_range(range.clone(), replacement);
        }
        output
    }

    #[test]
    fn line_comment_toggle_adds_removes_and_preserves_indentation() {
        let text = "const a = 1;\nconst b = 2;\n";
        let toggle = comment_toggle_for("x.ts", text, 0..text.len(), false).unwrap();
        assert_eq!(
            toggle.edits,
            vec![
                (0..0, "// ".to_owned()),
                (13..13, "// ".to_owned()),
            ]
        );
        let commented = apply_comment_edits(text, &toggle.edits);
        assert_eq!(commented, "// const a = 1;\n// const b = 2;\n");

        let second = comment_toggle_for("x.ts", &commented, 0..commented.len(), false).unwrap();
        assert_eq!(
            apply_comment_edits(&commented, &second.edits),
            text,
            "a fully commented selection toggles back"
        );

        let mixed = "// a\nb\n";
        let add = comment_toggle_for("x.ts", mixed, 0..mixed.len(), false).unwrap();
        assert_eq!(
            apply_comment_edits(mixed, &add.edits),
            "// // a\n// b\n",
            "one uncommented line makes the whole selection add comments"
        );

        let aligned = "  a\n    b\n";
        let add = comment_toggle_for("x.ts", aligned, 0..aligned.len(), false).unwrap();
        assert_eq!(
            apply_comment_edits(aligned, &add.edits),
            "  // a\n  //   b\n",
            "comments align at the selection's minimum indent column"
        );

        let blank = "a\n\nb\n";
        let add = comment_toggle_for("x.ts", blank, 0..blank.len(), false).unwrap();
        assert_eq!(apply_comment_edits(blank, &add.edits), "// a\n\n// b\n");
    }

    #[test]
    fn multi_cursor_occurrence_matching_follows_monaco_find_options() {
        let text = "Foo foo FOO foobar foo";
        assert_eq!(
            find_occurrences(text, "foo", false, false),
            vec![0..3, 4..7, 8..11, 12..15, 19..22]
        );
        assert_eq!(
            find_occurrences(text, "foo", true, false),
            vec![4..7, 12..15, 19..22]
        );
        assert_eq!(
            find_occurrences(text, "foo", true, true),
            vec![4..7, 19..22],
            "whole-word matching excludes the foobar prefix"
        );
        assert_eq!(
            find_occurrences(text, "foo", false, true),
            vec![0..3, 4..7, 8..11, 19..22]
        );
        assert!(find_occurrences(text, "", false, false).is_empty());
    }

    #[test]
    fn diagnostic_navigation_sorts_wraps_and_moves_to_next_or_previous() {
        let diagnostics = vec![
            crate::diagnostics::Diagnostic {
                source: "W3C",
                kind: crate::diagnostics::DiagnosticKind::Validation,
                line: 1,
                column: 2,
                message: "first".into(),
            },
            crate::diagnostics::Diagnostic {
                source: "W3C",
                kind: crate::diagnostics::DiagnosticKind::Validation,
                line: 3,
                column: 2,
                message: "second".into(),
            },
        ];
        let text = "aaa\nbbb\nccc\n";
        assert_eq!(diagnostic_target(text, 0, &diagnostics, true), Some(1));
        assert_eq!(diagnostic_target(text, 1, &diagnostics, true), Some(9));
        assert_eq!(diagnostic_target(text, 9, &diagnostics, true), Some(1));
        assert_eq!(diagnostic_target(text, 1, &diagnostics, false), Some(9));
        assert_eq!(diagnostic_target(text, 9, &diagnostics, false), Some(1));
        assert_eq!(
            diagnostic_target(text, 0, &[], true),
            None,
            "no diagnostics is a no-op"
        );
    }

    #[test]
    fn command_palette_filters_and_line_transforms_match_source_commands() {
        assert_eq!(
            palette_matches("sort"),
            vec!["Sort Lines Ascending", "Sort Lines Descending"]
        );
        assert!(palette_matches("comment").contains(&"Toggle Line Comment"));
        assert!(palette_matches("zzz").is_empty());

        let text = "b\na\nc\n";
        assert_eq!(
            sort_lines_at(text, 0..0, false),
            Some(("a\nb\nc\n".into(), 0..0))
        );
        assert_eq!(
            sort_lines_at(text, 0..0, true),
            Some(("c\nb\na\n".into(), 0..0))
        );
        assert_eq!(sort_lines_at("a\nb\n", 0..0, false), None);

        let selected = "z\nb\na\n";
        assert_eq!(
            sort_lines_at(selected, 2..5, false),
            Some(("z\na\nb\n".into(), 2..5))
        );
        assert_eq!(
            reverse_lines_at(text, 0..0),
            Some(("c\na\nb\n".into(), 0..0))
        );
        assert_eq!(
            delete_duplicate_lines_at("a\nb\na\nb\n", 0..0),
            Some(("a\nb\n".into(), 0..0))
        );

        assert_eq!(title_case("hello world"), "Hello World");
        assert_eq!(title_case("don't stop"), "Don't Stop");
        assert_eq!(apply_case_style("Hello", CaseStyle::Upper), "HELLO");
        assert_eq!(apply_case_style("Hello", CaseStyle::Lower), "hello");
        assert_eq!(
            transpose_at("hello world", 6),
            Some((5..7, "w ".into(), 7))
        );
        assert_eq!(
            transpose_at("hello world\nx", 11),
            Some((10..12, "\nd".into(), 12))
        );
        assert_eq!(transpose_at("abc", 0), Some((0..1, "a".into(), 1)));
        assert_eq!(transpose_at("abc", 3), None);
        assert_eq!(
            duplicate_selection_at("abc\ndef", 0..2),
            Some(("ababc\ndef".into(), 2..4))
        );
        assert_eq!(
            duplicate_selection_at("a\nb", 0..0),
            Some(("a\na\nb".into(), 2..2))
        );
        assert_eq!(spaces_to_next_tab_stop("ab", 0), "  ");
        assert_eq!(spaces_to_next_tab_stop("ab", 1), " ");
        assert_eq!(spaces_to_next_tab_stop("ab", 2), "  ");
        assert_eq!(spaces_to_next_tab_stop("\ta", 1), "  ");
        assert_eq!(
            remove_brackets_at("call(a)", 5..5),
            Some(("calla".into(), 4..4))
        );
        assert_eq!(
            remove_brackets_at("call(a)", 0..7),
            Some(("calla".into(), 0..5))
        );
        assert_eq!(remove_brackets_at("call(a)", 0..0), None);

        let symbols = vec![
            crate::symbols::SymbolLocation {
                offset: 0,
                line: 1,
                column: 1,
                preview: "function alpha() {}".into(),
            },
            crate::symbols::SymbolLocation {
                offset: 20,
                line: 2,
                column: 1,
                preview: "const beta = 1".into(),
            },
        ];
        assert_eq!(filter_symbols(&symbols, "bet").len(), 1);
        assert_eq!(filter_symbols(&symbols, "BET")[0].preview, "const beta = 1");
        assert_eq!(filter_symbols(&symbols, "").len(), 2);
    }

    #[test]
    fn case_styles_match_monaco_word_boundaries() {
        assert_eq!(
            apply_case_style("HelloWorld", CaseStyle::Snake),
            "hello_world"
        );
        assert_eq!(
            apply_case_style("HTTPServer", CaseStyle::Snake),
            "http_server"
        );
        assert_eq!(
            apply_case_style("version2Beta", CaseStyle::Snake),
            "version2_beta"
        );

        assert_eq!(
            apply_case_style("Hello world", CaseStyle::Camel),
            "helloWorld"
        );
        assert_eq!(
            apply_case_style("hello_world-test value", CaseStyle::Camel),
            "helloWorldTestValue"
        );
        assert_eq!(
            apply_case_style("foo__bar", CaseStyle::Camel),
            "fooBar"
        );

        assert_eq!(
            apply_case_style("hello_world-test value", CaseStyle::Pascal),
            "HelloWorldTestValue"
        );
        assert_eq!(
            apply_case_style("foo.bar baz", CaseStyle::Pascal),
            "Foo.BarBaz"
        );

        assert_eq!(
            apply_case_style("HelloWorld", CaseStyle::Kebab),
            "hello-world"
        );
        assert_eq!(
            apply_case_style("HTTP_Server", CaseStyle::Kebab),
            "http-server"
        );
        assert_eq!(
            apply_case_style("myURL", CaseStyle::Kebab),
            "my-url"
        );
        assert_eq!(
            apply_case_style("foo__bar", CaseStyle::Kebab),
            "foo-_bar",
            "underscore replacement follows Source's non-overlapping regex scan"
        );
        assert_eq!(
            palette_matches("snake case"),
            vec!["Transform to Snake Case"]
        );
        assert_eq!(
            palette_matches("kebab case"),
            vec!["Transform to Kebab Case"]
        );
    }

    #[test]
    fn delete_all_left_and_right_follow_monaco_line_boundaries() {
        let apply = |left: bool, range: std::ops::Range<usize>| {
            let (edits, mapped) =
                delete_all_edits("abc\ndef", std::slice::from_ref(&range), left).unwrap();
            (apply_comment_edits("abc\ndef", &edits), mapped)
        };
        assert_eq!(apply(true, 1..1), ("bc\ndef".into(), vec![0]));
        assert_eq!(apply(true, 4..4), ("abcdef".into(), vec![3]));
        assert_eq!(apply(true, 1..6), ("f".into(), vec![0]));
        let empty = 0..0;
        assert!(delete_all_edits("abc\ndef", std::slice::from_ref(&empty), true).is_none());
        assert_eq!(apply(false, 1..1), ("a\ndef".into(), vec![1]));
        assert_eq!(apply(false, 3..3), ("abcdef".into(), vec![3]));
        assert_eq!(apply(false, 1..2), ("ac\ndef".into(), vec![1]));
        let end = 7..7;
        assert!(delete_all_edits("abc\ndef", std::slice::from_ref(&end), false).is_none());
    }

    #[test]
    fn trim_trailing_whitespace_skips_cursor_lines_and_strings() {
        assert_eq!(
            trim_trailing_whitespace_at("a  \nb\t\nc  ", 0, ".ts"),
            Some(("a\nb\nc".into(), 0))
        );
        assert_eq!(
            trim_trailing_whitespace_at("a  \nb\t\nc  ", 3, ".ts"),
            Some(("a  \nb\nc".into(), 3)),
            "the caret line is skipped when the caret sits at its end"
        );
        assert_eq!(
            trim_trailing_whitespace_at("a  \nb\t\nc  ", 2, ".ts"),
            Some(("a \nb\nc".into(), 2)),
            "trimming starts at the caret inside trailing whitespace"
        );
        assert_eq!(trim_trailing_whitespace_at("a\nb", 0, ".ts"), None);
        assert_eq!(
            trim_trailing_whitespace_at("`a  \nb`\nc  ", 0, ".ts"),
            Some(("`a  \nb`\nc".into(), 0)),
            "whitespace inside a multiline string is not trimmed"
        );
        assert_eq!(
            trim_trailing_whitespace_at("a  // note  ", 0, ".ts"),
            Some(("a  // note".into(), 0))
        );
        assert_eq!(
            trim_trailing_whitespace_at("const re = /abc  \nconst x = 1  ", 0, ".ts"),
            Some(("const re = /abc  \nconst x = 1".into(), 0)),
            "an unterminated regex protects only its own line"
        );
        assert_eq!(
            trim_trailing_whitespace_at("const re = /abc  /   ", 0, ".ts"),
            Some(("const re = /abc  /".into(), 0)),
            "whitespace after a closed regex is trimmed"
        );
        assert_eq!(
            trim_trailing_whitespace_at("a = b / 2  ", 0, ".ts"),
            Some(("a = b / 2".into(), 0)),
            "division is not treated as a regex"
        );
    }

    #[test]
    fn indent_and_outdent_shift_selected_lines_and_preserve_selection() {
        fn one(start: usize, end: usize) -> Vec<std::ops::Range<usize>> {
            std::iter::once(start..end).collect()
        }
        let (edits, mapped) = shift_line_edits("a\nb", &one(0, 0), false).unwrap();
        assert_eq!(apply_comment_edits("a\nb", &edits), "  a\nb");
        assert_eq!(mapped, vec![2..2]);

        let (edits, mapped) = shift_line_edits("a\nb", &one(0, 3), false).unwrap();
        assert_eq!(apply_comment_edits("a\nb", &edits), "  a\n  b");
        assert_eq!(mapped, vec![0..7]);

        let (edits, mapped) = shift_line_edits("a\n\nb", &one(0, 4), false).unwrap();
        assert_eq!(apply_comment_edits("a\n\nb", &edits), "  a\n\n  b");
        assert_eq!(mapped, vec![0..8]);

        let (edits, mapped) = shift_line_edits("", &one(0, 0), false).unwrap();
        assert_eq!(apply_comment_edits("", &edits), "  ");
        assert_eq!(mapped, vec![2..2]);

        let (edits, mapped) = shift_line_edits("    a\n b", &one(0, 0), true).unwrap();
        assert_eq!(apply_comment_edits("    a\n b", &edits), "  a\n b");
        assert_eq!(mapped, vec![0..0]);

        let (edits, mapped) = shift_line_edits("    a", &one(2, 2), true).unwrap();
        assert_eq!(apply_comment_edits("    a", &edits), "  a");
        assert_eq!(mapped, vec![0..0]);

        let (edits, mapped) = shift_line_edits("\ta", &one(1, 1), true).unwrap();
        assert_eq!(apply_comment_edits("\ta", &edits), "a");
        assert_eq!(mapped, vec![0..0]);

        assert!(shift_line_edits("a", &one(0, 0), true).is_none());

        let (edits, mapped) = shift_line_edits("  a\n  b", &one(0, 7), true).unwrap();
        assert_eq!(apply_comment_edits("  a\n  b", &edits), "a\nb");
        assert_eq!(mapped, vec![0..3]);
    }

    #[test]
    fn join_lines_matches_monaco_whitespace_and_caret_rules() {
        assert_eq!(
            join_lines_at("hello\nworld", 1..1),
            Some(("hello world".into(), 5..5))
        );
        assert_eq!(join_lines_at("hello\nworld", 7..7), None);
        assert_eq!(
            join_lines_at("hello   \nworld", 0..0),
            Some(("hello world".into(), 6..6))
        );
        assert_eq!(
            join_lines_at("hello\n   world", 0..0),
            Some(("hello world".into(), 5..5))
        );
        assert_eq!(
            join_lines_at("hello\n\nworld", 0..0),
            Some(("hello\nworld".into(), 5..5))
        );
        assert_eq!(
            join_lines_at("hello\nworld", 1..3),
            Some(("hello world".into(), 1..3))
        );
        assert_eq!(
            join_lines_at("a\nb\nc", 0..3),
            Some(("a b\nc".into(), 0..3))
        );
        assert_eq!(
            join_lines_at("a\nb\nc", 0..5),
            Some(("a b c".into(), 0..5))
        );
        assert_eq!(join_lines_at("a\n", 0..0), Some(("a".into(), 1..1)));
        assert_eq!(
            join_lines_at("\nworld", 0..0),
            Some(("world".into(), 0..0))
        );
    }

    #[test]
    fn insert_and_delete_line_commands_match_monaco_line_boundaries() {
        assert_eq!(
            insert_line_at("a\n  b\nc", 5, false, ".ts"),
            ("a\n  b\n  \nc".into(), 8)
        );
        assert_eq!(
            insert_line_at("a\n  b\nc", 5, true, ".ts"),
            ("a\n\n  b\nc".into(), 2)
        );
        assert_eq!(
            insert_line_at("a\nb", 0, true, ".ts"),
            ("\na\nb".into(), 0)
        );
        assert_eq!(
            insert_line_at("  if (ok) {\n  x", 11, false, "x.ts"),
            ("  if (ok) {\n    \n  x".into(), 16)
        );
        assert_eq!(
            insert_line_at("if (x) {\n  y\n}", 9, true, "x.ts"),
            ("if (x) {\n  \n  y\n}".into(), 11)
        );
        assert_eq!(
            insert_line_at("if ok:\n  x", 6, false, "x.py"),
            ("if ok:\n  \n  x".into(), 9)
        );
        assert_eq!(
            insert_line_at("  <div>\n  x", 7, false, "x.html"),
            ("  <div>\n    \n  x".into(), 12)
        );

        assert_eq!(
            delete_lines_at("a\nb\nc\n", 2..2, 2),
            Some(("a\nc\n".into(), 2))
        );
        assert_eq!(
            delete_lines_at("a\nb\nc\n", 0..0, 0),
            Some(("b\nc\n".into(), 0))
        );
        assert_eq!(
            delete_lines_at("a\nb\nc\n", 4..4, 4),
            Some(("a\nb\n".into(), 4))
        );
        assert_eq!(
            delete_lines_at("a\nb", 2..2, 2),
            Some(("a".into(), 1))
        );
        assert_eq!(
            delete_lines_at("abc", 1..1, 1),
            Some((String::new(), 0))
        );
        assert_eq!(
            delete_lines_at("a\nb\nc\n", 2..4, 2),
            Some(("a\nc\n".into(), 2))
        );
        assert_eq!(
            delete_lines_at("a\n\nb", 2..2, 2),
            Some(("a\nb".into(), 2))
        );
        assert_eq!(
            delete_lines_at("a\nb\nc\n", 2..5, 2),
            Some(("a\n".into(), 2))
        );
    }

    #[test]
    fn copy_line_block_matches_monacos_copy_up_and_down_selection_rules() {
        let text = "first\nsecond line\nthird line\nfourth line\nfifth";
        assert_eq!(
            copy_line_block(text, 0..2, true),
            Some((
                "first\nfirst\nsecond line\nthird line\nfourth line\nfifth".into(),
                6..8
            ))
        );
        assert_eq!(
            copy_line_block(text, 0..2, false),
            Some((
                "first\nfirst\nsecond line\nthird line\nfourth line\nfifth".into(),
                0..2
            )),
            "copy up keeps the cursor on the original line"
        );
        assert_eq!(
            copy_line_block(text, 28..28, false),
            Some((
                "first\nsecond line\nthird line\nthird line\nfourth line\nfifth".into(),
                28..28
            ))
        );
        assert_eq!(
            copy_line_block(text, 41..43, true),
            Some((
                "first\nsecond line\nthird line\nfourth line\nfifth\nfifth".into(),
                47..49
            ))
        );
        assert_eq!(
            copy_line_block(text, 6..31, false),
            Some((
                "first\nsecond line\nthird line\nfourth line\nsecond line\nthird line\nfourth line\nfifth".into(),
                6..31
            ))
        );
        assert_eq!(
            copy_line_block("a\n\nb", 2..2, true),
            Some(("a\n\n\nb".into(), 3..3))
        );
        assert_eq!(
            copy_line_block("a\n\nb", 2..2, false),
            Some(("a\n\n\nb".into(), 2..2))
        );
    }

    #[test]
    fn move_line_block_swaps_lines_and_tracks_the_block_selection() {
        let text = "a\nb\nc\n";
        assert_eq!(
            move_line_block(text, 2..3, true, ".ts"),
            Some(("a\nc\nb\n".into(), 4..5))
        );
        assert_eq!(
            move_line_block(text, 2..3, false, ".ts"),
            Some(("b\na\nc\n".into(), 0..1))
        );
        assert_eq!(move_line_block(text, 0..1, false, ".ts"), None);
        assert_eq!(move_line_block("a\nb\nc", 4..5, true, ".ts"), None);

        let text = "a\nb\nc\nd\n";
        assert_eq!(
            move_line_block(text, 2..6, true, ".ts"),
            Some(("a\nd\nb\nc\n".into(), 4..8))
        );
        assert_eq!(
            move_line_block(text, 2..6, false, ".ts"),
            Some(("b\nc\na\nd\n".into(), 0..4))
        );
        assert_eq!(
            move_line_block(text, 2..2, true, ".ts"),
            Some(("a\nc\nb\nd\n".into(), 4..4))
        );
        assert_eq!(
            move_line_block("a\nb", 2..3, false, ".ts"),
            Some(("b\na".into(), 0..1))
        );

        assert_eq!(
            move_line_block("a\nif (x) {\n}\n", 0..1, true, ".ts"),
            Some(("if (x) {\n  a\n}\n".into(), 9..12))
        );
        assert_eq!(
            move_line_block("if (x) {\n}\nb\nc\n", 11..14, false, ".ts"),
            Some(("if (x) {\n  b\n  c\n}\n".into(), 9..16))
        );
        assert_eq!(
            move_line_block("a\n  b\n  c\nd\n", 4..4, true, ".ts"),
            Some(("a\n  c\n  b\nd\n".into(), 8..8))
        );
        assert_eq!(
            move_line_block("a\n  b\n  c\nd\n", 2..10, true, ".ts"),
            Some(("a\nd\nb\nc\n".into(), 4..8))
        );
    }

    #[test]
    fn goto_line_parses_monaco_formats_and_counts_back_from_the_end() {
        let text = "one\ntwo\nthree\n";
        assert_eq!(parse_goto_line("2", 4), Some((2, 1)));
        assert_eq!(parse_goto_line("2:3", 4), Some((2, 3)));
        assert_eq!(parse_goto_line("2,3", 4), Some((2, 3)));
        assert_eq!(parse_goto_line("2#3", 4), Some((2, 3)));
        assert_eq!(parse_goto_line(":-1", 4), Some((4, 1)));
        assert_eq!(parse_goto_line("-2", 4), Some((3, 1)));
        assert_eq!(parse_goto_line("0", 4), None);
        assert_eq!(parse_goto_line("9", 4), None);
        assert_eq!(parse_goto_line("abc", 4), None);
        assert_eq!(goto_line_offset(text, "1:1"), Some(0));
        assert_eq!(goto_line_offset(text, "2:3"), Some(6));
        assert_eq!(goto_line_offset(text, "3:100"), Some(13));
        assert_eq!(goto_line_offset(text, ":-1"), Some(14));
    }

    #[test]
    fn add_cursor_translation_preserves_grapheme_columns() {
        let text = "one\ntwo\nthree\n";
        assert_eq!(translate_line_offset(text, 3, false), Some(7));
        assert_eq!(translate_line_offset(text, 7, false), Some(11));
        assert_eq!(translate_line_offset(text, 11, true), Some(7));
        assert_eq!(translate_line_offset(text, 4, true), Some(0));
        assert_eq!(translate_line_offset(text, 0, true), None);
        assert_eq!(
            translate_line_offset(text, 13, false),
            Some(14),
            "the trailing newline creates an empty final logical line"
        );
        assert_eq!(translate_line_offset("one\ntwo", 7, false), None);
        assert_eq!(translate_line_offset("aa\néé\n", 2, false), Some(7));
    }

    #[test]
    fn comment_toggle_uses_monaco_language_tokens_and_block_fallback() {
        assert_eq!(comment_config("x.ts").line, Some("//"));
        assert_eq!(comment_config("x.css").line, None);
        assert_eq!(comment_config("page.html").block, Some(("<!--", "-->")));
        assert_eq!(comment_config("note.md").block, Some(("<!--", "-->")));
        assert_eq!(comment_config("script.py").line, Some("#"));
        assert_eq!(comment_config("query.sql").line, Some("--"));
        assert_eq!(comment_config("note.txt"), Default::default());

        let css = "a { color: red; }\n";
        let toggle = comment_toggle_for("x.css", css, 0..css.len(), false).unwrap();
        assert_eq!(
            apply_comment_edits(css, &toggle.edits),
            "/* a { color: red; } */\n"
        );
        let closed = apply_comment_edits(css, &toggle.edits);
        let remove = comment_toggle_for("x.css", &closed, 0..closed.len(), false).unwrap();
        assert_eq!(apply_comment_edits(&closed, &remove.edits), css);

        let html = "<div></div>\n";
        let toggle = comment_toggle_for("page.html", html, 0..html.len(), false).unwrap();
        assert_eq!(
            apply_comment_edits(html, &toggle.edits),
            "<!-- <div></div> -->\n"
        );

        let js = "const a = 1;\n";
        let block = comment_toggle_for("x.ts", js, 0..js.len(), true).unwrap();
        assert_eq!(
            apply_comment_edits(js, &block.edits),
            "/* const a = 1; */\n",
            "Shift+Alt+A forces the block-comment command"
        );

        let empty = "";
        let toggle = comment_toggle_for("x.ts", empty, 0..0, true).unwrap();
        assert_eq!(
            apply_comment_edits(empty, &toggle.edits),
            "/*  */",
            "a collapsed block toggle inserts both tokens"
        );
        assert_eq!(toggle.selection, 3..3, "the caret sits inside the block");

        assert!(comment_toggle_for("note.txt", "plain", 0..5, false).is_none());
    }

    #[gpui::test]
    fn editor_auto_closes_jsdoc_comment_pair(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "doc.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(1, &key.rel, Some(String::new()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        for ch in ["/", "*", "*"] {
            editor.update_in(cx, |editor, window, cx| {
                editor.replace_text_in_range(None, ch, window, cx);
            });
        }
        let document = retained.borrow();
        let buffer = &document.get(&key).unwrap().buffer;
        assert_eq!(buffer.text(), "/** */");
        assert_eq!(buffer.caret(), 3);
    }

    #[gpui::test]
    fn editor_toggles_line_comments_with_cmd_slash_and_undoes_once(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "comment.ts".into());
        let source = "const a = 1;\nconst b = 2;\n";
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, source.len()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-/");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "// const a = 1;\n// const b = 2;\n"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            source,
            "the whole comment toggle is one undo step"
        );
    }

    #[test]
    fn editor_selection_and_line_highlight_use_profile_selected_color() {
        let theme = Theme::default();
        assert_eq!(editor_selection_fill(&theme), theme.selected);
        assert_ne!(
            editor_selection_fill(&theme),
            gpui::rgba(0x3a83f740),
            "selection no longer uses the old hard-coded accent fill"
        );
    }

    #[test]
    fn smooth_caret_fades_and_resets_visibility() {
        let mut caret = SmoothCaret {
            opacity: 1.,
            target: 1.,
            remaining: CARET_HOLD_FRAMES,
            running: false,
            generation: 0,
        };
        for _ in 0..CARET_HOLD_FRAMES {
            caret.tick();
        }
        caret.tick();
        assert!(caret.opacity() < 1.0, "the hidden phase starts fading");
        for _ in 0..16 {
            caret.tick();
        }
        assert!(caret.opacity() <= 0.01, "fade reaches transparent");
        caret.reset_values();
        assert_eq!(caret.opacity(), 1.0);
    }

    #[gpui::test]
    fn smooth_caret_timer_fades_only_while_focused(cx: &mut gpui::TestAppContext) {
        let caret = cx.new(SmoothCaret::new);
        caret.update(cx, |caret, cx| caret.update_state(true, cx));
        cx.executor().advance_clock(
            CARET_FRAME * CARET_HOLD_FRAMES + CARET_FRAME * 16,
        );
        cx.run_until_parked();
        assert!(
            caret.read_with(cx, |caret, _| caret.opacity()) < 0.5,
            "focused caret fades through its blink cycle"
        );
        caret.update(cx, |caret, cx| caret.update_state(false, cx));
        assert_eq!(
            caret.read_with(cx, |caret, _| caret.opacity()),
            1.0,
            "blur restores a steady visible caret"
        );
        cx.executor().advance_clock(CARET_FRAME * 4);
        cx.run_until_parked();
        assert_eq!(caret.read_with(cx, |caret, _| caret.opacity()), 1.0);
    }

    #[test]
    fn auto_pair_helpers_close_openings_skip_closings_and_erase_pairs() {
        assert!(matches!(
            auto_pair_action('(', "", 0, auto_pair_policy("x.ts"), false, false),
            Some(PairAction::Insert(')'))
        ));
        assert!(matches!(
            auto_pair_action('(', ")", 0, auto_pair_policy("x.ts"), false, false),
            Some(PairAction::Skip)
        ));
        assert!(matches!(
            auto_pair_action(')', ")", 0, auto_pair_policy("x.ts"), false, false),
            Some(PairAction::Skip)
        ));
        assert!(
            auto_pair_action('\'', "don", 3, auto_pair_policy("x.ts"), false, false).is_none(),
            "quotes after a word do not auto-close"
        );
        assert_eq!(auto_pair_policy("note.md").double_quote, QuoteRule::Never);
        assert_eq!(auto_pair_policy("app.ts").double_quote, QuoteRule::NotString);
        assert_eq!(
            auto_pair_policy("app.ts").single_quote,
            QuoteRule::NotStringOrComment
        );
        assert!(!auto_pair_policy("data.json").parens);
        assert!(!auto_pair_policy("page.html").angle);
        assert!(auto_pair_policy("page.html").surround_angle);
        assert!(auto_pair_policy("note.md").angle);
        assert_eq!(surround_close_for('(', auto_pair_policy("x.ts")), Some(')'));
        assert_eq!(surround_close_for('"', auto_pair_policy("x.ts")), Some('"'));
        assert_eq!(surround_close_for('"', auto_pair_policy("note.md")), None);
        assert!(
            auto_pair_action('<', "", 0, auto_pair_policy("page.html"), false, false).is_none(),
            "HTML only surrounds < >, it does not auto-close them"
        );
        assert!(matches!(
            auto_pair_action('<', "", 0, auto_pair_policy("note.md"), false, false),
            Some(PairAction::Insert('>'))
        ));
        assert_eq!(
            surround_close_for('<', auto_pair_policy("page.html")),
            Some('>')
        );
        let (inside_string, inside_comment) = pair_context("let s = \"inside\";", 10);
        assert!(inside_string && !inside_comment);
        let (inside_string, inside_comment) = pair_context("// inside comment", 5);
        assert!(!inside_string && inside_comment);
        assert_eq!(auto_pair_erase("()", 1, false), Some(0..2));
        assert_eq!(auto_pair_erase("[]", 1, false), Some(0..2));
        assert_eq!(auto_pair_erase("(a)", 2, false), None);
        assert_eq!(auto_pair_erase("<>", 1, false), None);
        assert_eq!(auto_pair_erase("<>", 1, true), Some(0..2));
    }

    #[gpui::test]
    fn editor_auto_closes_pairs_overtypes_and_removes_empty_pairs(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "auto-pair.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("value".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "(", window, cx);
        });
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), "()value");
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 1);
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, ")", window, cx);
        });
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), "()value");
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.caret(), 2);
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "\"", window, cx);
        });
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "()\"\"value"
        );
        cx.simulate_keystrokes("backspace");
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), "()value");
    }

    #[gpui::test]
    fn editor_enter_smart_indents_between_braces(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "indent.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("if (ok) {}".into()));
        let retained = documents.clone();
        let (_editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained.borrow_mut().edit(&key, |buffer| buffer.select(9, 9));
        cx.simulate_keystrokes("enter");
        let document = retained.borrow();
        let buffer = &document.get(&key).unwrap().buffer;
        assert_eq!(buffer.text(), "if (ok) {\n  \n}");
        assert_eq!(buffer.caret(), 12);
    }

    #[gpui::test]
    fn editor_wraps_selection_when_typing_an_opening_pair(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "surround.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("value".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 5));
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "(", window, cx);
        });
        let document = retained.borrow();
        let buffer = &document.get(&key).unwrap().buffer;
        assert_eq!(buffer.text(), "(value)");
        assert_eq!(buffer.selection(), 1..6);
    }

    #[gpui::test]
    fn matching_bracket_highlight_follows_the_caret_and_skips_strings(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "brackets.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("function f() {\n  return (1 + 2);\n  // )\n}\n".into()),
        );
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        let text = retained
            .borrow()
            .get(&key)
            .unwrap()
            .buffer
            .text()
            .to_owned();
        let open = text.find("(1").unwrap();
        let close = text.find(");").unwrap();
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(open + 1, open + 1));
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.brackets, vec![open..open + 1, close..close + 1]);
            assert_eq!(
                editor.bracket_colors.len(),
                6,
                "structural brackets are colorized, the comment bracket is not"
            );
            let brace = text.find('{').unwrap();
            assert_eq!(
                editor
                    .bracket_colors
                    .iter()
                    .find(|(range, _)| range.start == brace)
                    .map(|(_, depth)| *depth),
                Some(0)
            );
            assert_eq!(
                editor
                    .bracket_colors
                    .iter()
                    .find(|(range, _)| range.start == open)
                    .map(|(_, depth)| *depth),
                Some(1)
            );
            let comment = text.find("// )").unwrap() + 3;
            assert!(
                !editor
                    .bracket_colors
                    .iter()
                    .any(|(range, _)| range.start == comment),
                "comment brackets never receive a depth color"
            );
        });

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(close, close));
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.brackets, vec![open..open + 1, close..close + 1]);
        });

        let comment = text.find("// )").unwrap() + 3;
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(comment, comment));
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(editor.brackets.is_empty()));

        let brace_open = text.find('{').unwrap();
        let brace_close = text.rfind('}').unwrap();
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(brace_close, brace_close));
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(
                editor.brackets,
                vec![brace_open..brace_open + 1, brace_close..brace_close + 1]
            );
        });

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(2, 2));
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(editor.brackets.is_empty()));
    }

    #[gpui::test]
    fn occurrence_highlights_follow_the_caret_word_and_clear_on_selection(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "highlights.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("foo bar foo foo\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(1, 1));
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.occurrences, vec![0..3, 8..11, 12..15]);
        });

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(5, 5));
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.occurrences, vec![4..7]);
        });

        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 3));
        draw(cx);
        editor.read_with(cx, |editor, _| assert!(editor.occurrences.is_empty()));
    }

    #[test]
    fn bracket_display_runs_map_source_offsets_through_tabs() {
        let text = "{\t[a]}";
        let (display, mapping) = expand_tabs(text);
        let colors = crate::brackets::colorized(text);
        let runs = bracket_display_runs(display.len(), &mapping, 0, text.len(), &colors);
        assert_eq!(
            runs.iter().map(|(_, len, _)| *len).sum::<usize>(),
            display.len(),
            "runs cover the entire display line"
        );
        assert_eq!(
            runs.iter()
                .filter(|(_, _, depth)| depth.is_some())
                .count(),
            4,
            "every structural bracket gets a colored run"
        );
        assert!(
            runs.iter().any(|(_, _, depth)| *depth == Some(0))
                && runs.iter().any(|(_, _, depth)| *depth == Some(1))
                && runs.iter().any(|(_, _, depth)| depth.is_none()),
            "outer, nested and default text runs all exist"
        );
        let first_only = bracket_display_runs(display.len(), &mapping, 0, 1, &colors);
        assert_eq!(
            first_only
                .iter()
                .filter(|(_, _, depth)| depth.is_some())
                .count(),
            1,
            "brackets from later lines are excluded"
        );
    }

    #[gpui::test]
    fn active_line_number_and_gutter_follow_the_caret(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "active-line.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("first\nsecond\nthird\n".into()),
        );
        let second = documents.borrow().get(&key).unwrap().buffer.text().find("second").unwrap();
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.move_to(second, false));
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents.clone(), cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(320.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(320.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-current-line-number-1").is_some());
        assert!(cx.debug_bounds("editor-current-line-number-0").is_none());
        assert!(cx.debug_bounds("editor-current-line-number-2").is_none());

        let third = documents.borrow().get(&key).unwrap().buffer.text().find("third").unwrap();
        documents
            .borrow_mut()
            .edit(&key, |buffer| buffer.move_to(third, false));
        draw(cx);
        assert!(cx.debug_bounds("editor-current-line-number-2").is_some());
        assert!(cx.debug_bounds("editor-current-line-number-1").is_none());
    }

    #[gpui::test]
    fn hover_popup_reports_inferred_declaration_type(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "hover.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents.borrow_mut().finish_load(
            1,
            &key.rel,
            Some("const answer = 42;\nanswer;\n".into()),
        );
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        let line = cx.debug_bounds("editor-line-1").expect("usage line");
        cx.simulate_event(gpui::MouseMoveEvent {
            position: point(line.left() + px(2.), line.center().y),
            ..Default::default()
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-hover").is_some());
        editor.read_with(cx, |editor, _| {
            let (hover, _) = editor.hover_popup.as_ref().expect("hover info");
            assert_eq!(hover.word, "answer");
            assert_eq!(hover.detail, "number · line 1");
        });

        cx.simulate_event(gpui::MouseMoveEvent {
            position: point(line.right() - px(2.), line.center().y),
            ..Default::default()
        });
        draw(cx);
        assert!(cx.debug_bounds("editor-hover").is_none());
        editor.read_with(cx, |editor, _| assert!(editor.hover_popup.is_none()));
    }

    #[gpui::test]
    fn signature_help_renders_active_parameter_inside_a_call(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "signature.js".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = "function greet(who, punctuation) {}\ngreet('hi', );\n";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        let caret = source.rfind(" )").unwrap() + 1;
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(caret, caret));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        assert!(cx.debug_bounds("editor-signature-help").is_some());
        assert!(cx.debug_bounds("editor-signature-active").is_some());

        let after = source.rfind(';').unwrap() + 1;
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(after, after));
        draw(cx);
        assert!(cx.debug_bounds("editor-signature-help").is_none());
    }

    #[test]
    fn quick_suggestion_trigger_and_comment_detection_match_source_rules() {
        for ch in ['a', 'Z', '0', '_', '$', '.'] {
            assert!(quick_suggestion_trigger("x.ts", ch), "{ch}");
        }
        for ch in [' ', '(', ')', '/', '-'] {
            assert!(!quick_suggestion_trigger("x.ts", ch), "{ch}");
        }
        assert!(quick_suggestion_trigger("x.css", '-'));
        assert!(quick_suggestion_trigger("x.html", '-'));
        assert!(quick_suggestion_trigger("x.html", '>'));
        assert!(!quick_suggestion_trigger("x.css", '>'));
        assert!(!quick_suggestion_trigger("x.ts", '-'));
        assert!(quick_suggestion_in_comment("// value", 4));
        assert!(!quick_suggestion_in_comment("value // note", 3));
        assert!(!quick_suggestion_in_comment("\"value\"", 2));
    }

    #[gpui::test]
    fn typing_a_word_schedules_quick_suggestions(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "quick.ts".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("alpha beta\n".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(11, 11));
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "a", window, cx);
        });
        cx.executor().advance_clock(Duration::from_millis(30));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open, "quick suggestions open after 25ms");
            assert!(
                editor.completion_items.iter().any(|item| item == "alpha"),
                "{:?}",
                editor.completion_items
            );
        });
    }

    #[gpui::test]
    fn completion_popup_filters_document_words_and_accepts_with_enter(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "completion.rs".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("al beta alpha".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(2, 2));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(editor.completion_items, vec!["alpha".to_owned()]);
            assert_eq!(editor.completion_selected, 0);
        });
        let popup = cx
            .debug_bounds("editor-completion")
            .expect("completion popup");
        let row = cx.debug_bounds("editor-row-0").expect("edited line");
        assert!(
            popup.top() >= row.bottom() - px(1.)
                && popup.top() <= row.bottom() + px(LINE_HEIGHT * 2.),
            "completion popup anchors below the caret line: {popup:?} vs {row:?}"
        );
        assert!(cx.debug_bounds("editor-completion-0").is_some());

        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "alpha beta alpha"
        );
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            5..5
        );
        editor.read_with(cx, |editor, _| assert!(!editor.completion_open));
    }

    #[gpui::test]
    fn completion_popup_offers_css_semantic_properties(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "styles.css".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = ".x {\n  dis";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(editor.completion_items.first().map(String::as_str), Some("display"));
        });
    }

    #[gpui::test]
    fn completion_popup_offers_css_property_values(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "value-complete.css".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = ".x { display: f";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("flex")
            );
        });
    }

    #[gpui::test]
    fn completion_popup_offers_css_value_functions(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "function-complete.css".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = ".x { color: rg";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("rgb(")
            );
        });
    }

    #[gpui::test]
    fn completion_popup_offers_css_custom_properties(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "custom-properties.css".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = ":root { --brand-color: red; --space: 1rem; }\n.x { color: var(--br";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("--brand-color")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            ":root { --brand-color: red; --space: 1rem; }\n.x { color: var(--brand-color"
        );
    }

    #[gpui::test]
    fn completion_popup_offers_css_media_features(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "media-complete.css".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = "@media (prefers-color-scheme: d";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("dark")
            );
        });
    }

    #[gpui::test]
    fn completion_popup_offers_css_at_rules(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "at-rule.css".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = "@med";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("media")
            );
        });
    }

    #[gpui::test]
    fn completion_popup_offers_html_closing_tags(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "closing-tag.html".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = "<div></di";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("div>")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "<div></div>"
        );
    }

    #[gpui::test]
    fn completion_popup_offers_html_auto_close_after_start_tag(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "auto-close.html".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = "<div>";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("</div>")
            );
        });
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "<div></div>"
        );
    }

    #[gpui::test]
    fn completion_popup_offers_html_attribute_values(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "value-complete.html".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        let source = "<link rel=\"sty";
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(source.len(), source.len()));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("ctrl-space");
        draw(cx);
        editor.read_with(cx, |editor, _| {
            assert!(editor.completion_open);
            assert_eq!(
                editor.completion_items.first().map(String::as_str),
                Some("stylesheet")
            );
        });
    }

    #[gpui::test]
    fn emmet_tab_expands_and_places_the_caret(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "page.html".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some("div".into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(3, 3));
        cx.simulate_resize(size(px(675.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(300.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "<div></div>"
        );
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            5..5
        );

        retained.borrow_mut().edit(&key, |buffer| {
            buffer.select_all();
            buffer.insert("ul>li*3");
        });
        editor.update(cx, |editor, cx| {
            editor.sync_lines();
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        let text = retained
            .borrow()
            .get(&key)
            .unwrap()
            .buffer
            .text()
            .to_owned();
        assert_eq!(text, "<ul>\n\t<li></li>\n\t<li></li>\n\t<li></li>\n</ul>");
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            text.rfind("<li>").unwrap() + 4..text.rfind("<li>").unwrap() + 4
        );
    }

    #[gpui::test]
    fn split_editor_views_keep_independent_documents_and_refresh(cx: &mut gpui::TestAppContext) {
        use super::*;
        struct SplitProbe {
            left: gpui::Entity<CodeEditor>,
            right: gpui::Entity<CodeEditor>,
        }
        impl Render for SplitProbe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .flex()
                    .size_full()
                    .child(
                        div()
                            .id("split-left")
                            .debug_selector(|| "split-left".into())
                            .w(px(300.))
                            .h_full()
                            .child(self.left.clone()),
                    )
                    .child(
                        div()
                            .id("split-right")
                            .debug_selector(|| "split-right".into())
                            .w(px(300.))
                            .h_full()
                            .child(self.right.clone()),
                    )
            }
        }
        let left_key = DocumentKey::new("/fixture".into(), "split-left.txt".into());
        let right_key = DocumentKey::new("/fixture".into(), "split-right.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, left_key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &left_key.rel, Some("one\ntwo\n".into()));
        documents.borrow_mut().open(2, right_key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(2, &right_key.rel, Some("alpha\n".into()));
        let retained = documents.clone();
        let (probe, cx) = cx.add_window_view(|window, cx| {
            let left = cx.new(|cx| CodeEditor::new(left_key.clone(), documents.clone(), cx));
            let right = cx.new(|cx| CodeEditor::new(right_key.clone(), documents, cx));
            let left_focus = left.read(cx).focus.clone();
            window.focus(&left_focus, cx);
            SplitProbe { left, right }
        });
        cx.simulate_resize(size(px(660.), px(280.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(660.), px(280.)), |_, _| {
                probe.clone().into_any_element()
            })
        };
        draw(cx);
        assert!(cx.debug_bounds("split-left").is_some());
        assert!(cx.debug_bounds("split-right").is_some());
        let (left, right) =
            probe.read_with(cx, |probe, _| (probe.left.clone(), probe.right.clone()));
        left.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "x", window, cx)
        });
        draw(cx);
        assert_eq!(
            retained.borrow().get(&left_key).unwrap().buffer.text(),
            "xone\ntwo\n"
        );
        assert_eq!(
            retained.borrow().get(&right_key).unwrap().buffer.text(),
            "alpha\n"
        );
        right.update_in(cx, |editor, window, cx| {
            editor.replace_text_in_range(None, "y", window, cx)
        });
        draw(cx);
        assert_eq!(
            retained.borrow().get(&right_key).unwrap().buffer.text(),
            "yalpha\n"
        );
        assert_eq!(
            retained.borrow().get(&left_key).unwrap().buffer.text(),
            "xone\ntwo\n"
        );
        assert!(probe.read_with(cx, |probe, cx| {
            probe.left.read(cx).revision.is_some() && probe.right.read(cx).revision.is_some()
        }));
    }

    #[gpui::test]
    fn platform_typing_and_backspace_use_source_undo_groups(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "undo.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(String::new()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(500.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(500.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        for ch in "hello world".chars() {
            cx.update(|window, cx| {
                editor.update(cx, |editor, cx| {
                    editor.replace_text_in_range(None, &ch.to_string(), window, cx)
                })
            });
            draw(cx);
        }
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), "hello");
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), "");
        cx.simulate_keystrokes("cmd-shift-z");
        cx.simulate_keystrokes("cmd-shift-z");
        draw(cx);
        cx.simulate_keystrokes("backspace");
        cx.simulate_keystrokes("backspace");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "hello wor"
        );
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "hello world"
        );
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.replace_text_in_range(None, "tail", window, cx)
            })
        });
        draw(cx);
        cx.simulate_keystrokes("cmd-z");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "hello world"
        );
    }

    #[gpui::test]
    fn find_reveals_a_match_beyond_the_virtualized_viewport(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "find-large.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        let source = format!("{}LAST FIND 🌍", "ordinary source line\n".repeat(6000));
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source));
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key, documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(500.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(675.), px(500.)), |_, _| {
                    editor.clone().into_any_element()
                });
            }
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        let query = editor.read_with(cx, |editor, _| editor.find_query.clone());
        cx.update(|window, cx| {
            query.update(cx, |input, cx| {
                input.replace_text_in_range(None, "LAST FIND 🌍", window, cx)
            })
        });
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                query.focus_handle(cx).is_focused(window),
                "Find retains input focus while revealing document matches"
            );
            editor.read_with(cx, |editor, _| {
                let start = editor.read(|buffer| buffer.selection().start).unwrap();
                let position = editor
                    .position_at(start)
                    .expect("matching logical line must be painted");
                assert!(editor.bounds.unwrap().contains(&position), "{position:?}");
                assert!(
                    editor.painted.len() < 100,
                    "Find must not render the entire file"
                );
                assert_eq!(editor.find_matches.ranges.len(), 1);
            });
        });
    }
    #[gpui::test]
    fn find_inputs_navigate_replace_atomically_and_leave_find_edits_out_of_document(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "find.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        let original = "prefix needle 界🌍\nNeedle second\nneedle_tail\nlast needle";
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(original.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(500.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..3 {
                cx.draw(point(px(0.), px(0.)), size(px(675.), px(500.)), |_, _| {
                    editor.clone().into_any_element()
                });
            }
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        let query = editor.read_with(cx, |editor, _| editor.find_query.clone());
        cx.update(|window, cx| {
            query.update(cx, |input, cx| {
                input.replace_text_in_range(None, "needle", window, cx)
            })
        });
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |editor, _| editor.find_matches.ranges.len()),
            4
        );
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            7..13
        );
        assert!(!retained.borrow().get(&key).unwrap().dirty());
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            editor
                .read_with(cx, |editor, _| editor
                    .read(|b| b.text()[b.selection()].to_owned()))
                .unwrap(),
            "Needle"
        );
        cx.simulate_keystrokes("shift-enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            7..13
        );
        let click_position = cx.debug_bounds("find-word").unwrap().center();
        cx.simulate_click(click_position, Modifiers::default());
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |editor, _| editor.find_matches.ranges.len()),
            3
        );
        let click_position = cx.debug_bounds("find-case").unwrap().center();
        cx.simulate_click(click_position, Modifiers::default());
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |editor, _| editor.find_matches.ranges.len()),
            2
        );
        let click_position = cx.debug_bounds("find-toggle-replace").unwrap().center();
        cx.simulate_click(click_position, Modifiers::default());
        draw(cx);
        // Moving the editor selection off a result must not make Replace inert.
        // Source selects the next result first, without modifying the document.
        retained
            .borrow_mut()
            .edit(&key, |buffer| buffer.select(0, 0));
        let click_position = cx.debug_bounds("replace-one").unwrap().center();
        cx.simulate_click(click_position, Modifiers::default());
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.selection(),
            7..13
        );
        assert!(!retained.borrow().get(&key).unwrap().dirty());
        let replacement = editor.read_with(cx, |editor, _| editor.replacement.clone());
        cx.update(|window, cx| {
            replacement.update(cx, |input, cx| {
                input.replace_text_in_range(None, "changed 🌍", window, cx)
            })
        });
        draw(cx);
        cx.update(|window, cx| window.focus(&replacement.focus_handle(cx), cx));
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "prefix changed 🌍 界🌍\nNeedle second\nneedle_tail\nlast needle"
        );
        cx.update(|_, cx| editor.update(cx, |editor, cx| editor.edit(TextBuffer::undo, cx)));
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), original);
        let click_position = cx.debug_bounds("replace-all").unwrap().center();
        cx.simulate_click(click_position, Modifiers::default());
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "prefix changed 🌍 界🌍\nNeedle second\nneedle_tail\nlast changed 🌍"
        );
        assert!(retained.borrow().get(&key).unwrap().dirty());
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.close_find(window, cx);
                editor.edit(TextBuffer::undo, cx);
            })
        });
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), original);
        assert!(!retained.borrow().get(&key).unwrap().dirty());
    }
    #[gpui::test]
    fn regex_find_replaces_capture_groups_with_undo(cx: &mut gpui::TestAppContext) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "regex.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        let original = "a1 b2 a3";
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(original.into()));
        let retained = documents.clone();
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key.clone(), documents, cx);
            window.focus(&editor.focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(500.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..3 {
                cx.draw(point(px(0.), px(0.)), size(px(675.), px(500.)), |_, _| {
                    editor.clone().into_any_element()
                });
            }
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-f");
        draw(cx);
        let regex = cx.debug_bounds("find-regex").unwrap().center();
        cx.simulate_click(regex, Modifiers::default());
        let query = editor.read_with(cx, |editor, _| editor.find_query.clone());
        cx.update(|window, cx| {
            query.update(cx, |input, cx| {
                input.replace_text_in_range(None, r"a(\d)", window, cx)
            })
        });
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |editor, _| editor.find_matches.ranges.len()),
            2
        );
        let toggle = cx.debug_bounds("find-toggle-replace").unwrap().center();
        cx.simulate_click(toggle, Modifiers::default());
        let replacement = editor.read_with(cx, |editor, _| editor.replacement.clone());
        cx.update(|window, cx| {
            replacement.update(cx, |input, cx| {
                input.replace_text_in_range(None, "<$1>", window, cx)
            })
        });
        draw(cx);
        let all = cx.debug_bounds("replace-all").unwrap().center();
        cx.simulate_click(all, Modifiers::default());
        draw(cx);
        assert_eq!(
            retained.borrow().get(&key).unwrap().buffer.text(),
            "<1> b2 <3>"
        );
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.close_find(window, cx);
                editor.edit(TextBuffer::undo, cx);
            })
        });
        draw(cx);
        assert_eq!(retained.borrow().get(&key).unwrap().buffer.text(), original);
    }

    #[::core::prelude::v1::test]
    fn expanded_tabs_preserve_unicode_source_offsets() {
        let (text, mapping) = expand_tabs("\t界\tx");
        assert_eq!(text.as_ref(), "  界 x");
        assert_eq!(
            mapping,
            vec![(0, 0), (1, 0), (2, 1), (5, 4), (6, 5), (7, 6)]
        );
    }

    #[gpui::test]
    fn document_end_reveals_last_line_without_rendering_the_whole_file(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::*;
        let key = DocumentKey::new("/fixture".into(), "large.txt".into());
        let documents = Rc::new(RefCell::new(Documents::default()));
        let mut lines = (1..=6000)
            .map(|i| format!("Line {i}: retained source text"))
            .collect::<Vec<_>>();
        lines[1] = "A long wrapped line with Unicode 界🌍. ".repeat(30);
        lines.push("PARITY END AT LINE 6001".into());
        let source = lines.join("\n");
        documents.borrow_mut().open(1, key.clone(), true);
        documents
            .borrow_mut()
            .finish_load(1, &key.rel, Some(source.clone()));
        let (editor, cx) = cx.add_window_view(|window, cx| {
            let editor = CodeEditor::new(key, documents, cx);
            let focus = editor.focus.clone();
            window.focus(&focus, cx);
            editor
        });
        cx.simulate_resize(size(px(675.), px(700.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(675.), px(700.)), |_, _| {
                editor.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("cmd-down");
        draw(cx);
        // A second frame permits a wrapped-row caret adjustment. It must not
        // walk through thousands of intervening rows to reach the endpoint.
        draw(cx);
        cx.update(|_, cx| {
            let editor = editor.read(cx);
            assert_eq!(editor.read(|b| b.caret()), Some(source.len()));
            let caret = editor
                .position_at(source.len())
                .expect("last line must be painted");
            let bounds = editor.bounds.unwrap();
            assert!(
                caret.y >= bounds.top() && caret.y + px(LINE_HEIGHT) <= bounds.bottom(),
                "caret={caret:?}, viewport={bounds:?}, scroll={:?}",
                editor.list.logical_scroll_top()
            );
            assert!(editor.list.logical_scroll_top().item_ix > 5900);
            assert!(
                editor.painted.len() < 200,
                "offscreen rows must remain virtualized"
            );
        });
    }
}
