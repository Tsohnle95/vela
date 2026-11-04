//! Context-owned, lazy observed-change review. Diff work never runs in render.
use crate::{
    preformatted::PreformattedText,
    theme::{self, Theme},
};
use gpui::{prelude::*, *};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    ops::Range,
    rc::Rc,
    sync::Arc,
};
use vela_engine::{
    changes::{LineDiff, LineKind, Review, tree_rows},
    observation::{Baseline, ObservedFile},
};

pub(crate) enum Event {
    Changed,
    Close,
    Open(String),
}

/// Diff lines only lack a trailing newline when they are the final line of a
/// side, matching unified-diff EOF markers.
fn missing_final_newline(text: &str) -> bool {
    !text.is_empty() && !text.ends_with('\n')
}

fn display_text(text: &str) -> &str {
    text.strip_suffix('\n')
        .unwrap_or(text)
        .strip_suffix('\r')
        .unwrap_or(text.strip_suffix('\n').unwrap_or(text))
}

fn diff_line_aria(kind: LineKind, old: Option<usize>, new: Option<usize>, text: &str) -> String {
    let line = new.or(old).map(|line| line.to_string()).unwrap_or_default();
    let prefix = match kind {
        LineKind::Added => "Added line",
        LineKind::Removed => "Removed line",
        LineKind::Equal => "Line",
    };
    let content = if text.chars().count() > 160 {
        format!("{}…", text.chars().take(160).collect::<String>())
    } else {
        text.to_string()
    };
    if content.is_empty() {
        format!("{prefix} {line}")
    } else {
        format!("{prefix} {line}: {content}")
    }
}

fn byte_column(text: &str, char_column: usize) -> usize {
    text.char_indices()
        .nth(char_column)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Row {
    Header(String),
    Pending(String),
    Unknown(String),
    Line(String, usize),
}
struct ViewDiff {
    data: LineDiff,
    width: Pixels,
}
impl std::ops::Deref for ViewDiff {
    type Target = LineDiff;
    fn deref(&self) -> &LineDiff {
        &self.data
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DiffPoint {
    path: String,
    line: usize,
    /// Byte offset inside the line's display text (newline excluded).
    column: usize,
}

#[derive(Clone, Debug)]
struct OrderedSelection {
    start: DiffPoint,
    end: DiffPoint,
    start_row: usize,
    /// Inclusive final row; a cursor at column zero of a later line folds back
    /// to a full selection of the previous line, matching editor selection.
    end_row: usize,
}

#[derive(Default)]
struct DiffSelection {
    anchor: Option<DiffPoint>,
    cursor: Option<DiffPoint>,
    dragging: bool,
}

type LineGeometry = Rc<RefCell<HashMap<(String, usize), Bounds<Pixels>>>>;

pub(crate) struct ChangesView {
    pub directory: String,
    review: Review,
    collapsed_dirs: BTreeSet<String>,
    collapsed_sections: BTreeSet<String>,
    selected: Option<String>,
    wrap: bool,
    diffs: HashMap<String, Arc<ViewDiff>>,
    pending: HashMap<String, uuid::Uuid>,
    rows: Vec<Row>,
    list: ListState,
    tree_list: ListState,
    tree_rows: Vec<vela_engine::changes::TreeRow>,
    focuses: HashMap<String, FocusHandle>,
    section_horizontal: HashMap<String, ScrollHandle>,
    selection: DiffSelection,
    ordered_selection: Option<OrderedSelection>,
    line_geometry: LineGeometry,
    char_width: Pixels,
    focus: FocusHandle,
}
impl EventEmitter<Event> for ChangesView {}
impl ChangesView {
    pub fn new(directory: String, seed: &[ObservedFile], cx: &mut Context<Self>) -> Self {
        let mut review = Review::default();
        review.receive(seed);
        let mut view = Self {
            directory,
            review,
            collapsed_dirs: BTreeSet::new(),
            collapsed_sections: BTreeSet::new(),
            selected: None,
            wrap: true,
            diffs: HashMap::new(),
            pending: HashMap::new(),
            rows: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(600.)),
            tree_list: ListState::new(0, ListAlignment::Top, px(120.)),
            tree_rows: Vec::new(),
            focuses: HashMap::new(),
            section_horizontal: HashMap::new(),
            selection: DiffSelection::default(),
            ordered_selection: None,
            line_geometry: Rc::new(RefCell::new(HashMap::new())),
            char_width: px(7.),
            focus: cx.focus_handle().tab_stop(true),
        };
        view.rebuild();
        view
    }
    pub fn receive(&mut self, changes: &[ObservedFile], cx: &mut Context<Self>) {
        for change in changes {
            self.diffs.remove(&change.rel);
            self.pending.remove(&change.rel);
            self.section_horizontal.remove(&change.rel);
        }
        self.clear_selection();
        self.review.receive(changes);
        self.rebuild();
        cx.emit(Event::Changed);
        cx.notify();
    }
    pub fn relocate(&mut self, from: &str, to: &str, cx: &mut Context<Self>) {
        self.review.relocate(from, to);
        self.diffs.clear();
        self.pending.clear();
        self.section_horizontal.clear();
        self.clear_selection();
        self.rebuild();
        cx.emit(Event::Changed);
        cx.notify();
    }
    pub fn hide(&mut self, prefix: String, cx: &mut Context<Self>) {
        self.review.hide(prefix);
        self.rebuild();
        cx.emit(Event::Changed);
        cx.notify();
    }
    pub fn unhide(&mut self, path: &str, cx: &mut Context<Self>) {
        self.review.unhide(path);
        self.rebuild();
        cx.emit(Event::Changed);
        cx.notify();
    }
    pub fn summary(&self) -> Vec<vela_engine::GitEntry> {
        self.review
            .files()
            .map(|file| vela_engine::GitEntry {
                path: file.rel.clone(),
                code: if file.deleted { "D" } else { "M" }.into(),
                label: if file.deleted {
                    "Deleted"
                } else if matches!(file.baseline, Baseline::Unknown) {
                    "Observed"
                } else {
                    "Modified"
                }
                .into(),
                staged: false,
            })
            .collect()
    }
    pub fn reveal(&mut self, path: String, cx: &mut Context<Self>) {
        self.selected = Some(path.clone());
        self.collapsed_sections.remove(&path);
        self.rebuild();
        if let Some(index) = self
            .rows
            .iter()
            .position(|row| matches!(row, Row::Header(p) if p == &path))
        {
            self.list.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: px(0.),
            });
        }
        cx.notify();
    }
    fn rebuild(&mut self) {
        let offset = self.list.logical_scroll_top();
        let anchor = self.rows.get(offset.item_ix).cloned();
        let tree = tree_rows(
            self.review.files().map(|f| f.rel.as_str()),
            &self.collapsed_dirs,
        );
        let common = tree
            .iter()
            .zip(&self.tree_rows)
            .take_while(|(a, b)| a == b)
            .count();
        if common != tree.len() || common != self.tree_rows.len() {
            self.tree_list
                .splice(common..self.tree_rows.len(), tree.len() - common);
            self.tree_rows = tree;
        }
        let mut rows = Vec::new();
        for file in self.review.files() {
            rows.push(Row::Header(file.rel.clone()));
            if self.collapsed_sections.contains(&file.rel) {
                continue;
            }
            if matches!(file.baseline, Baseline::Unknown) {
                rows.push(Row::Unknown(file.rel.clone()));
            } else if let Some(diff) = self.diffs.get(&file.rel) {
                rows.extend((0..diff.lines.len()).map(|index| Row::Line(file.rel.clone(), index)));
            } else {
                rows.push(Row::Pending(file.rel.clone()));
            }
        }
        self.list.splice(0..self.rows.len(), rows.len());
        if let Some(anchor) = anchor {
            let position = rows.iter().position(|row| *row == anchor).or_else(|| {
                let path = match &anchor {
                    Row::Header(path)
                    | Row::Pending(path)
                    | Row::Unknown(path)
                    | Row::Line(path, _) => path,
                };
                rows.iter()
                    .position(|row| matches!(row, Row::Header(candidate) if candidate == path))
            });
            if let Some(item_ix) = position {
                self.list.scroll_to(ListOffset {
                    item_ix,
                    offset_in_item: offset.offset_in_item,
                });
            }
        }
        self.rows = rows;
        self.refresh_selection();
    }
    fn compute(&mut self, path: String, cx: &mut Context<Self>) {
        if self.pending.contains_key(&path) || self.diffs.contains_key(&path) {
            return;
        }
        let Some(file) = self.review.get(&path) else {
            return;
        };
        let Baseline::Known {
            content: before, ..
        } = &file.baseline
        else {
            return;
        };
        let before = before.clone();
        let after = if file.deleted {
            Arc::<str>::from("")
        } else {
            file.content.clone().unwrap_or_default()
        };
        let token = uuid::Uuid::new_v4();
        self.pending.insert(path.clone(), token);
        let text_system = cx.text_system().clone();
        let work = cx.background_executor().spawn(async move {
            let text_system = WindowTextSystem::new(text_system);
            let data = vela_engine::changes::line_diff(&before, &after);
            let measure = |text: &str| {
                text_system
                    .shape_line(
                        text.to_owned().into(),
                        px(12.),
                        &[TextRun {
                            len: text.len(),
                            font: font(theme::FONT_MONO),
                            color: Theme::default().text.into(),
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }],
                        None,
                    )
                    .width()
            };
            let tab = (measure(" ") * 2.).max(px(0.01));
            let width = data
                .lines
                .iter()
                .map(|line| {
                    let mut width = px(0.);
                    for (index, part) in line
                        .text
                        .trim_end_matches(['\r', '\n'])
                        .split('\t')
                        .enumerate()
                    {
                        if index > 0 {
                            width = tab * ((f32::from(width) / f32::from(tab)).floor() + 1.);
                        }
                        width += measure(part);
                    }
                    width + px(76.)
                })
                .fold(px(0.), Pixels::max);
            ViewDiff { data, width }
        });
        cx.spawn(async move |owner, cx| {
            let diff = work.await;
            let _ = owner.update(cx, |view, cx| {
                if view.pending.get(&path) != Some(&token) {
                    return;
                }
                view.pending.remove(&path);
                view.diffs.insert(path, Arc::new(diff));
                view.rebuild();
                cx.notify();
            });
        })
        .detach();
    }
    fn button(
        &mut self,
        id: String,
        label: String,
        active: bool,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let focus = self
            .focuses
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let visible = if id.starts_with("clear-")
            || id.starts_with("sticky-clear-")
            || id == "change-close"
        {
            "×".to_owned()
        } else {
            label.clone()
        };
        let action = Arc::new(action);
        let key_action = action.clone();
        let click_focus = focus.clone();
        let selector = id.clone();
        div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .role(accesskit::Role::Button)
            .aria_label(label.clone())
            .aria_selected(active)
            .track_focus(&focus)
            .h(px(24.))
            .px(px(8.))
            .rounded(px(4.))
            .flex()
            .items_center()
            .text_size(px(11.))
            .cursor_pointer()
            .text_color(if active { theme.accent } else { theme.dim })
            .hover(|s| s.bg(theme.inset))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _, window, cx| {
                    window.focus(&click_focus, cx);
                    action(view, window, cx);
                    cx.emit(Event::Changed);
                    cx.stop_propagation();
                }),
            )
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if ["enter", "space"].contains(&event.keystroke.key.as_str()) {
                    key_action(view, window, cx);
                    cx.emit(Event::Changed);
                    cx.stop_propagation();
                }
            }))
            .child(SharedString::from(visible))
            .into_any_element()
    }
    fn render_tree_row(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.tree_rows.get(index).cloned() else {
            return div().into_any_element();
        };
        let path = row.path.clone();
        let directory = row.directory;
        let active = if directory {
            !self.collapsed_dirs.contains(&path)
        } else {
            self.selected.as_deref() == Some(&path)
        };
        let label = if directory {
            format!("{} {}", if active { "⌄" } else { "›" }, row.label)
        } else {
            row.label
        };
        div()
            .pl(px((8 + row.depth * 12) as f32))
            .child(self.button(
                format!("change-tree-{path}"),
                label,
                active,
                move |view, _, cx| {
                    if directory {
                        if !view.collapsed_dirs.remove(&path) {
                            view.collapsed_dirs.insert(path.clone());
                        }
                        view.rebuild();
                    } else {
                        view.selected = Some(path.clone());
                        view.collapsed_sections.remove(&path);
                        view.rebuild();
                        if let Some(index) = view
                            .rows
                            .iter()
                            .position(|row| matches!(row,Row::Header(p) if p==&path))
                        {
                            view.list.scroll_to(ListOffset {
                                item_ix: index,
                                offset_in_item: px(0.),
                            });
                        }
                    }
                    cx.notify();
                },
                cx,
            ))
            .into_any_element()
    }
    fn render_section_header(
        &mut self,
        path: &str,
        sticky: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let path = path.to_owned();
        self.compute(path.clone(), cx);
        let Some(file) = self.review.get(&path).cloned() else {
            return div().into_any_element();
        };
        let collapsed = self.collapsed_sections.contains(&path);
        let toggle = path.clone();
        let open = path.clone();
        let clear = path.clone();
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let prefix = if sticky { "sticky-" } else { "" };
        let status = if file.deleted {
            "Deleted"
        } else if matches!(file.baseline, Baseline::Unknown) {
            "Observed"
        } else {
            "Modified"
        };
        let section_id = if sticky {
            format!("change-sticky-{path}")
        } else {
            format!("change-section-{path}")
        };
        let debug_id = section_id.clone();
        let mut header = div()
            .id(SharedString::from(section_id))
            .debug_selector(move || debug_id.clone())
            .role(accesskit::Role::Group)
            .aria_label(path.clone())
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(10.))
            .py(px(6.))
            .border_b_1()
            .border_color(theme.border)
            .bg(if self.selected.as_deref() == Some(&path) {
                theme.inset
            } else {
                theme.editor
            })
            .child(self.button(
                format!("{prefix}collapse-{path}"),
                if collapsed {
                    "›".into()
                } else {
                    "⌄".into()
                },
                !collapsed,
                move |view, _, cx| {
                    if !view.collapsed_sections.remove(&toggle) {
                        view.collapsed_sections.insert(toggle.clone());
                    }
                    view.rebuild();
                    cx.notify();
                },
                cx,
            ))
            .child(self.button(
                format!("{prefix}open-{path}"),
                path.clone(),
                false,
                move |_, _, cx| cx.emit(Event::Open(open.clone())),
                cx,
            ))
            .child(crate::tree_icons::glyph(&name))
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme.faint)
                    .child(status),
            );
        if let Some(diff) = self.diffs.get(&path) {
            header = header
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.green)
                        .child(format!("+{}", diff.additions)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.red)
                        .child(format!("-{}", diff.deletions)),
                );
        }
        header
            .child(self.button(
                format!("{prefix}clear-{path}"),
                format!("Clear {path}"),
                false,
                move |view, _, cx| {
                    view.review.dismiss(&clear);
                    view.diffs.remove(&clear);
                    view.pending.remove(&clear);
                    view.section_horizontal.remove(&clear);
                    view.clear_selection();
                    view.rebuild();
                    cx.notify();
                },
                cx,
            ))
            .into_any_element()
    }

    /// Index of the section header that should stick above the visible diff.
    /// The real header supplies the sticky top while it is fully visible;
    /// this returns a header only once its row has scrolled under the top.
    fn sticky_section_index(&self) -> Option<usize> {
        if self.rows.is_empty() {
            return None;
        }
        let top = self.list.logical_scroll_top();
        Self::sticky_index_for(&self.rows, top.item_ix, top.offset_in_item)
    }

    fn sticky_index_for(rows: &[Row], top: usize, offset_in_item: Pixels) -> Option<usize> {
        if rows.is_empty() {
            return None;
        }
        let index = top.min(rows.len() - 1);
        if matches!(rows[index], Row::Header(_)) {
            return (offset_in_item > px(0.5)).then_some(index);
        }
        (0..=index)
            .rev()
            .find(|row| matches!(rows[*row], Row::Header(_)))
    }

    fn refresh_selection(&mut self) {
        self.ordered_selection = None;
        let (Some(anchor), Some(cursor)) =
            (self.selection.anchor.clone(), self.selection.cursor.clone())
        else {
            return;
        };
        let (Some(mut start_row), Some(mut end_row)) =
            (self.row_index(&anchor), self.row_index(&cursor))
        else {
            return;
        };
        let (mut start, mut end) = (anchor, cursor);
        if start_row > end_row || (start_row == end_row && start.column > end.column) {
            std::mem::swap(&mut start, &mut end);
            std::mem::swap(&mut start_row, &mut end_row);
        }
        if end.column == 0 && end_row > start_row {
            end_row -= 1;
            end.column = usize::MAX;
        }
        self.ordered_selection = Some(OrderedSelection {
            start,
            end,
            start_row,
            end_row,
        });
    }

    fn row_index(&self, point: &DiffPoint) -> Option<usize> {
        self.rows.iter().position(|row| {
            matches!(row, Row::Line(path, line) if path == &point.path && *line == point.line)
        })
    }

    fn selection_column(&self, path: &str, line: usize, x: Pixels) -> usize {
        let Some(bounds) = self
            .line_geometry
            .borrow()
            .get(&(path.to_owned(), line))
            .copied()
        else {
            return 0;
        };
        let Some(text) = self.diffs.get(path).and_then(|diff| diff.lines.get(line)) else {
            return 0;
        };
        let display = display_text(&text.text);
        let chars = display.chars().count();
        let column =
            ((f32::from(x - bounds.left()) / f32::from(self.char_width)) + 0.5).max(0.) as usize;
        byte_column(display, column.min(chars))
    }

    fn begin_diff_selection(
        &mut self,
        path: String,
        line: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        let point = DiffPoint {
            column: self.selection_column(&path, line, event.position.x),
            path,
            line,
        };
        if event.modifiers.shift && self.selection.anchor.is_some() {
            self.selection.cursor = Some(point);
        } else {
            self.selection.anchor = Some(point.clone());
            self.selection.cursor = Some(point);
        }
        self.selection.dragging = true;
        self.refresh_selection();
        cx.notify();
    }

    fn update_diff_selection(&mut self, path: String, line: usize, x: Pixels) {
        if !self.selection.dragging || self.selection.anchor.is_none() {
            return;
        }
        self.selection.cursor = Some(DiffPoint {
            column: self.selection_column(&path, line, x),
            path,
            line,
        });
        self.refresh_selection();
    }

    fn finish_diff_selection(&mut self) {
        self.selection.dragging = false;
    }

    fn line_selection(&self, row: usize, len: usize) -> Option<Range<usize>> {
        let ordered = self.ordered_selection.as_ref()?;
        if row < ordered.start_row || row > ordered.end_row {
            return None;
        }
        let start = if row == ordered.start_row {
            ordered.start.column.min(len)
        } else {
            0
        };
        let end = if row == ordered.end_row {
            ordered.end.column.min(len)
        } else {
            len
        };
        (end > start).then_some(start..end)
    }

    fn selection_text(&self) -> Option<String> {
        let ordered = self.ordered_selection.as_ref()?;
        let mut text = String::new();
        for row in ordered.start_row..=ordered.end_row {
            let Some(Row::Line(path, line)) = self.rows.get(row) else {
                continue;
            };
            let Some(diff) = self.diffs.get(path).and_then(|diff| diff.lines.get(*line)) else {
                continue;
            };
            let display = display_text(&diff.text);
            let Some(range) = self.line_selection(row, display.len()) else {
                continue;
            };
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&display[range]);
        }
        (!text.is_empty()).then_some(text)
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.selection_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    fn clear_selection(&mut self) {
        self.selection = DiffSelection::default();
        self.ordered_selection = None;
    }

    fn render_row(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        match self.rows.get(index).cloned() {
            Some(Row::Header(path)) => self.render_section_header(&path, false, cx),
            Some(Row::Pending(path)) => {
                self.compute(path, cx);
                div()
                    .h(px(80.))
                    .text_color(theme.faint)
                    .child("Loading diff…")
                    .into_any_element()
            }
            Some(Row::Unknown(_)) => div()
                .p(px(14.))
                .text_size(px(12.))
                .text_color(theme.dim)
                .child(
                    "Pre-change content wasn't observed for this file, so a diff isn't available.",
                )
                .into_any_element(),
            Some(Row::Line(path, line_index)) => {
                let Some(line) = self.diffs.get(&path).and_then(|d| d.lines.get(line_index)) else {
                    return div().into_any_element();
                };
                let color = match line.kind {
                    LineKind::Added => theme.green,
                    LineKind::Removed => theme.red,
                    LineKind::Equal => theme.dim,
                };
                let text = display_text(&line.text).to_owned();
                let selection = self.line_selection(index, text.len());
                let row_selected = selection.is_some();
                let emphasis = line
                    .emphasis
                    .clone()
                    .map(|range| range.start.min(text.len())..range.end.min(text.len()));
                let content = if (emphasis.is_some() || selection.is_some()) && !text.contains('\t')
                {
                    let mut cuts = vec![0, text.len()];
                    if let Some(range) = &emphasis {
                        cuts.push(range.start);
                        cuts.push(range.end);
                    }
                    if let Some(range) = &selection {
                        cuts.push(range.start);
                        cuts.push(range.end);
                    }
                    cuts.sort_unstable();
                    cuts.dedup();
                    let mut runs = Vec::new();
                    for pair in cuts.windows(2) {
                        let (start, end) = (pair[0], pair[1]);
                        if start >= end {
                            continue;
                        }
                        let selected = selection
                            .as_ref()
                            .is_some_and(|range| start >= range.start && end <= range.end);
                        let emphasized = emphasis
                            .as_ref()
                            .is_some_and(|range| start >= range.start && end <= range.end);
                        let background = if selected {
                            Some(theme.selected.into())
                        } else if emphasized {
                            Some(color.opacity(0.22).into())
                        } else {
                            None
                        };
                        runs.push(TextRun {
                            len: end - start,
                            font: theme::source_editor_font(),
                            color: theme.text.into(),
                            background_color: background,
                            underline: None,
                            strikethrough: None,
                        });
                    }
                    StyledText::new(SharedString::from(text.clone()))
                        .with_runs(runs)
                        .into_any_element()
                } else {
                    PreformattedText::new(text.clone(), self.wrap).into_any_element()
                };
                let geometry = self.line_geometry.clone();
                let geometry_path = path.clone();
                let geometry_line = line_index;
                let content_column = div()
                    .id(SharedString::from(format!(
                        "change-line-content-{path}-{line_index}"
                    )))
                    .debug_selector({
                        let selector = format!("change-line-content-{path}-{line_index}");
                        move || selector.clone()
                    })
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .text_color(theme.text)
                    .flex()
                    .flex_col()
                    .child(content)
                    .when(missing_final_newline(&line.text), |column| {
                        column.child(
                            div()
                                .text_size(px(10.))
                                .line_height(px(14.))
                                .text_color(theme.faint)
                                .child("\\ No newline at end of file"),
                        )
                    })
                    .child(
                        gpui::canvas(
                            move |bounds, _, _| {
                                geometry
                                    .borrow_mut()
                                    .insert((geometry_path.clone(), geometry_line), bounds);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    );
                let inner = div()
                    .w_full()
                    .min_w(if self.wrap {
                        px(0.)
                    } else {
                        self.diffs
                            .get(&path)
                            .map(|diff| diff.width)
                            .unwrap_or(px(0.))
                    })
                    .flex()
                    .bg(if row_selected {
                        theme.selected
                    } else if line.kind == LineKind::Equal {
                        theme.editor
                    } else {
                        color.opacity(0.08)
                    })
                    .font_family(theme::FONT_MONO)
                    .text_size(px(12.))
                    .line_height(px(20.))
                    .child(
                        div()
                            .w(px(38.))
                            .flex_shrink_0()
                            .text_right()
                            .pr(px(6.))
                            .text_color(theme.faint)
                            .child(line.old.map(|n| n.to_string()).unwrap_or_default()),
                    )
                    .child(
                        div()
                            .w(px(38.))
                            .flex_shrink_0()
                            .text_right()
                            .pr(px(6.))
                            .text_color(theme.faint)
                            .child(line.new.map(|n| n.to_string()).unwrap_or_default()),
                    )
                    .child(content_column);
                let handle = self
                    .section_horizontal
                    .entry(path.clone())
                    .or_default()
                    .clone();
                let row_id = format!("change-line-{path}-{line_index}");
                let row_selector = row_id.clone();
                let row_aria = diff_line_aria(line.kind, line.old, line.new, &text);
                let down_path = path.clone();
                let move_path = path.clone();
                let down_line = line_index;
                let move_line = line_index;
                div()
                    .id(SharedString::from(row_id))
                    .debug_selector(move || row_selector.clone())
                    .role(accesskit::Role::Label)
                    .aria_label(SharedString::from(row_aria))
                    .w_full()
                    .min_w_0()
                    .overflow_x_scroll()
                    .track_scroll(&handle)
                    .bg(if row_selected {
                        theme.selected
                    } else {
                        theme.editor
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                            view.begin_diff_selection(
                                down_path.clone(),
                                down_line,
                                event,
                                window,
                                cx,
                            );
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_move(cx.listener(move |view, event: &MouseMoveEvent, _, cx| {
                        if event.pressed_button == Some(MouseButton::Left) {
                            view.update_diff_selection(
                                move_path.clone(),
                                move_line,
                                event.position.x,
                            );
                            cx.notify();
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|view, _, _, cx| {
                            view.finish_diff_selection();
                            cx.notify();
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|view, _, _, cx| {
                            view.finish_diff_selection();
                            cx.notify();
                        }),
                    )
                    .child(inner)
                    .into_any_element()
            }
            None => div().into_any_element(),
        }
    }
}
impl Render for ChangesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        self.char_width = window
            .text_system()
            .shape_line(
                "W".into(),
                px(12.),
                &[TextRun {
                    len: 1,
                    font: font(theme::FONT_MONO),
                    color: theme.text.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            )
            .width()
            .max(px(1.));
        let count = self.review.files().count();
        let has_selection = self.selection_text().is_some();
        let all_collapsed = count > 0
            && self
                .review
                .files()
                .all(|file| self.collapsed_sections.contains(&file.rel));
        let mut toolbar = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(10.))
            .py(px(7.))
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("Changes"),
            )
            .child(
                div()
                    .px(px(7.))
                    .rounded(px(999.))
                    .bg(theme.selected)
                    .text_size(px(10.))
                    .text_color(theme.accent)
                    .child(count.to_string()),
            )
            .child(div().flex_1());
        if count > 0 {
            if has_selection {
                toolbar = toolbar.child(self.button(
                    "change-copy-selection".into(),
                    "Copy".into(),
                    false,
                    |view, _, cx| {
                        view.copy_selection(cx);
                    },
                    cx,
                ));
            }
            toolbar = toolbar
                .child(self.button(
                    "change-collapse-all".into(),
                    if all_collapsed {
                        "Expand all".into()
                    } else {
                        "Collapse all".into()
                    },
                    false,
                    move |view, _, cx| {
                        view.collapsed_sections = if all_collapsed {
                            BTreeSet::new()
                        } else {
                            view.review.files().map(|f| f.rel.clone()).collect()
                        };
                        view.rebuild();
                        cx.notify();
                    },
                    cx,
                ))
                .child(self.button(
                    "change-wrap".into(),
                    "Wrap".into(),
                    self.wrap,
                    |view, _, cx| {
                        view.wrap = !view.wrap;
                        for handle in view.section_horizontal.values() {
                            handle.set_offset(point(px(0.), px(0.)));
                        }
                        view.list.splice(0..view.rows.len(), view.rows.len());
                        cx.notify();
                    },
                    cx,
                ))
                .child(self.button(
                    "change-clear-all".into(),
                    "Clear all".into(),
                    false,
                    |view, _, cx| {
                        view.review.clear();
                        view.diffs.clear();
                        view.pending.clear();
                        view.selected = None;
                        view.clear_selection();
                        view.section_horizontal.clear();
                        view.rebuild();
                        cx.notify();
                    },
                    cx,
                ));
        }
        toolbar = toolbar.child(self.button(
            "change-close".into(),
            "Close changes view".into(),
            false,
            |_, _, cx| cx.emit(Event::Close),
            cx,
        ));
        let tree_entity = cx.entity();
        let tree = div()
            .id("changed-files-tree")
            .role(accesskit::Role::Navigation)
            .aria_label("Changed files")
            .w(px(220.))
            .flex_shrink_0()
            .min_h_0()
            .p(px(4.))
            .border_r_1()
            .border_color(theme.border)
            .child(
                list(self.tree_list.clone(), move |index, _, cx| {
                    tree_entity.update(cx, |view, cx| view.render_tree_row(index, cx))
                })
                .size_full(),
            );
        let entity = cx.entity();
        let sticky = self
            .sticky_section_index()
            .and_then(|index| match self.rows.get(index) {
                Some(Row::Header(path)) => Some(path.clone()),
                _ => None,
            })
            .map(|path| self.render_section_header(&path, true, cx));
        div()
            .id("observed-changes-view")
            .role(accesskit::Role::Region)
            .aria_label("Changes")
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(theme.editor)
            .track_focus(&self.focus)
            .on_action(cx.listener(|view, _: &crate::input::Copy, _, cx| {
                view.copy_selection(cx);
            }))
            .child(toolbar)
            .child(if count == 0 {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.))
                    .text_color(theme.faint)
                    .child("No changes yet. Files the agent edits appear here.")
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(tree)
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child({
                                let state = self.list.clone();
                                div()
                                    .id("changes-diff-scroll")
                                    .size_full()
                                    .min_w_0()
                                    .min_h_0()
                                    .overflow_hidden()
                                    .child(
                                        list(state, move |index, _, cx| {
                                            entity.update(cx, |view, cx| view.render_row(index, cx))
                                        })
                                        .size_full(),
                                    )
                                    .into_any_element()
                            })
                            .when_some(sticky, |pane, sticky| {
                                pane.child(
                                    div().absolute().top_0().left_0().right_0().child(sticky),
                                )
                            }),
                    )
                    .into_any_element()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Baseline, ChangesView, LineKind, ObservedFile, Row, diff_line_aria, missing_final_newline,
    };
    use gpui::{
        IntoElement, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size,
    };
    fn file(path: String) -> ObservedFile {
        ObservedFile {
            rel: path,
            baseline: Baseline::Known {
                content: "before\n".into(),
                existed: true,
            },
            content: Some("after\n".into()),
            deleted: false,
            moved_from: None,
            write_id: None,
        }
    }
    #[test]
    fn diff_line_aria_labels_distinguish_add_remove_and_context() {
        assert_eq!(
            diff_line_aria(LineKind::Added, None, Some(4), "new"),
            "Added line 4: new"
        );
        assert_eq!(
            diff_line_aria(LineKind::Removed, Some(9), None, "old"),
            "Removed line 9: old"
        );
        assert_eq!(
            diff_line_aria(LineKind::Equal, Some(3), Some(3), "same"),
            "Line 3: same"
        );
        assert_eq!(diff_line_aria(LineKind::Equal, None, None, ""), "Line ");
        let long = "x".repeat(200);
        let label = diff_line_aria(LineKind::Added, None, Some(1), &long);
        assert!(label.ends_with('…'));
        assert_eq!(
            label.chars().count(),
            "Added line 1: ".chars().count() + 161
        );
    }
    #[gpui::test]
    fn large_review_only_diffs_visible_sections_and_tree_reveal_reaches_the_last_file(
        cx: &mut TestAppContext,
    ) {
        let files = (0..1200)
            .map(|index| file(format!("src/file-{index:04}.txt")))
            .collect::<Vec<_>>();
        let (view, cx) =
            cx.add_window_view(|_, cx| ChangesView::new("/fixture".into(), &files, cx));
        let draw = |cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(1000.), px(350.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        cx.simulate_resize(size(px(1000.), px(350.)));
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let cached = view.read_with(cx, |view, _| view.diffs.len());
        assert!(
            cached > 0 && cached < 30,
            "opening review computed {cached} diffs for 1200 files"
        );
        assert!(
            view.read_with(cx, |view, _| view.focuses.len()) < 150,
            "tree mounted offscreen controls"
        );
        let sticky_target = "src/file-0600.txt";
        let sticky_line = view.read_with(cx, |view, _| {
            view.rows
                .iter()
                .position(|row| matches!(row, Row::Header(path) if path == sticky_target))
                .expect("target section exists")
        });
        view.update(cx, |view, cx| {
            view.list.scroll_to(gpui::ListOffset {
                item_ix: sticky_line,
                offset_in_item: px(20.),
            });
            cx.notify();
        });
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let (sticky, top, header_index, row_at_top) = view.read_with(cx, |view, _| {
            let header_index = view
                .rows
                .iter()
                .position(|row| matches!(row, Row::Header(path) if path == sticky_target));
            (
                view.sticky_section_index()
                    .map(|index| view.rows[index].clone()),
                view.list.logical_scroll_top(),
                header_index,
                view.rows
                    .get(view.list.logical_scroll_top().item_ix)
                    .cloned(),
            )
        });
        assert!(
            cx.debug_bounds("change-sticky-src/file-0600.txt").is_some(),
            "the section header above the viewport must stay visible: sticky={sticky:?} top={top:?} header={header_index:?} row={row_at_top:?}"
        );
        view.update(cx, |view, cx| view.reveal("src/file-1199.txt".into(), cx));
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        assert!(view.read_with(cx, |view, _| view.diffs.contains_key("src/file-1199.txt")));
        let position = cx
            .debug_bounds("clear-src/file-1199.txt")
            .expect("last file header must be rendered")
            .center();
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
        assert!(view.read_with(cx, |view, _| view.review.get("src/file-1199.txt").is_none()));
        // A subsequent actual event can reintroduce a locally cleared path.
        view.update(cx, |view, cx| {
            view.receive(&[file("src/file-1199.txt".into())], cx)
        });
        assert!(view.read_with(cx, |view, _| view.review.get("src/file-1199.txt").is_some()));
    }
    #[gpui::test]
    fn wrap_toggle_exposes_real_horizontal_scroll_and_releases_it_when_wrapped(
        cx: &mut TestAppContext,
    ) {
        let mut changed = file("long.txt".into());
        changed.content = Some(format!("\t{}界🌍\n", "long source ".repeat(100)).into());
        let (view, cx) =
            cx.add_window_view(|_, cx| ChangesView::new("/fixture".into(), &[changed], cx));
        cx.simulate_resize(size(px(900.), px(320.)));
        let draw = |cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(900.), px(320.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let button = cx.debug_bounds("change-wrap").unwrap().center();
        cx.simulate_click(button, Modifiers::default());
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let handle = view.read_with(cx, |view, _| {
            view.section_horizontal
                .get("long.txt")
                .cloned()
                .expect("section scroller is mounted")
        });
        assert!(
            handle.max_offset().x > px(1000.),
            "long line must be reachable: {:?}",
            handle.max_offset()
        );
        handle.set_offset(point(-handle.max_offset().x, px(0.)));
        draw(cx);
        assert!(handle.offset().x < px(-1000.));
        cx.simulate_click(button, Modifiers::default());
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        assert_eq!(handle.max_offset().x, px(0.));
        assert_eq!(handle.offset().x, px(0.));
    }

    #[gpui::test]
    fn sections_keep_independent_horizontal_offsets(cx: &mut TestAppContext) {
        let mut first = file("a-long.txt".into());
        first.content = Some(format!("{}\n", "a".repeat(400)).into());
        let mut second = file("b-long.txt".into());
        second.content = Some(format!("{}\n", "b".repeat(400)).into());
        let (view, cx) =
            cx.add_window_view(|_, cx| ChangesView::new("/fixture".into(), &[first, second], cx));
        cx.simulate_resize(size(px(900.), px(500.)));
        let draw = |cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(900.), px(500.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let wrap = cx.debug_bounds("change-wrap").unwrap().center();
        cx.simulate_click(wrap, Modifiers::default());
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let (first_handle, second_handle) = view.read_with(cx, |view, _| {
            (
                view.section_horizontal
                    .get("a-long.txt")
                    .cloned()
                    .expect("first section scroller"),
                view.section_horizontal
                    .get("b-long.txt")
                    .cloned()
                    .expect("second section scroller"),
            )
        });
        assert!(first_handle.max_offset().x > px(1000.));
        assert!(second_handle.max_offset().x > px(1000.));
        first_handle.set_offset(point(-first_handle.max_offset().x, px(0.)));
        draw(cx);
        assert!(first_handle.offset().x < px(-1000.));
        assert_eq!(second_handle.offset().x, px(0.));
        second_handle.set_offset(point(-second_handle.max_offset().x * 0.5, px(0.)));
        draw(cx);
        assert!(second_handle.offset().x < px(-500.));
        assert!(first_handle.offset().x < px(-1000.));
    }

    #[gpui::test]
    fn drag_selection_copies_only_the_selected_characters(cx: &mut TestAppContext) {
        let mut changed = file("select.txt".into());
        changed.baseline = Baseline::Known {
            content: "abcXYZ\n".into(),
            existed: true,
        };
        changed.content = Some("abcdef\n".into());
        let (view, cx) =
            cx.add_window_view(|_, cx| ChangesView::new("/fixture".into(), &[changed], cx));
        cx.simulate_resize(size(px(900.), px(320.)));
        let draw = |cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(900.), px(320.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        for _ in 0..4 {
            draw(cx);
            cx.run_until_parked();
        }
        let added_line = view.read_with(cx, |view, _| {
            let diff = &view.diffs["select.txt"];
            view.rows
                .iter()
                .find_map(|row| match row {
                    Row::Line(path, line) if path == "select.txt" => {
                        (diff.lines[*line].kind == LineKind::Added).then_some(*line)
                    }
                    _ => None,
                })
                .expect("added line is present")
        });
        let selector: &'static str =
            Box::leak(format!("change-line-content-select.txt-{added_line}").into_boxed_str());
        let content = cx.debug_bounds(selector).expect("line content is rendered");
        let char_width = view.read_with(cx, |view, _| view.char_width);
        let y = content.center().y;
        let start = point(content.left() + char_width * 1.2, y);
        let end = point(content.left() + char_width * 4.2, y);
        cx.simulate_event(gpui::MouseDownEvent {
            position: start,
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        draw(cx);
        cx.simulate_event(gpui::MouseMoveEvent {
            position: end,
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        draw(cx);
        cx.simulate_event(gpui::MouseUpEvent {
            position: end,
            button: MouseButton::Left,
            ..Default::default()
        });
        draw(cx);
        assert_eq!(
            view.read_with(cx, |view, _| view.selection_text()),
            Some("bcd".into())
        );
        assert!(
            cx.debug_bounds("change-copy-selection").is_some(),
            "a copy action appears while diff text is selected"
        );
    }

    #[gpui::test]
    fn stale_diff_jobs_cannot_replace_newer_file_state_and_controls_clear_without_mutating_content(
        cx: &mut TestAppContext,
    ) {
        let seed = vec![file("a.txt".into())];
        let (view, cx) = cx.add_window_view(|_, cx| ChangesView::new("/fixture".into(), &seed, cx));
        view.update(cx, |view, cx| {
            view.compute("a.txt".into(), cx);
            let mut next = file("a.txt".into());
            next.content = Some("newer\nsecond\n".into());
            view.receive(&[next], cx);
            view.compute("a.txt".into(), cx);
        });
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.diffs["a.txt"].additions),
            2
        );
        cx.simulate_resize(size(px(900.), px(400.)));
        let draw = |cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(900.), px(400.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        let collapse = cx.debug_bounds("change-collapse-all").unwrap().center();
        cx.simulate_click(collapse, Modifiers::default());
        draw(cx);
        assert!(view.read_with(cx, |view, _| {
            view.rows.iter().all(|row| matches!(row, Row::Header(_)))
        }));
        let wrap = cx.debug_bounds("change-wrap").unwrap().center();
        cx.simulate_click(wrap, Modifiers::default());
        draw(cx);
        assert!(!view.read_with(cx, |view, _| view.wrap));
        let clear = cx.debug_bounds("change-clear-all").unwrap().center();
        cx.simulate_click(clear, Modifiers::default());
        draw(cx);
        assert!(view.read_with(cx, |view, _| view.review.files().next().is_none()));
        assert_eq!(seed[0].content.as_deref(), Some("after\n"));
    }

    #[test]
    fn diff_eof_marker_only_applies_to_lines_without_trailing_newline() {
        assert!(missing_final_newline("last line"));
        assert!(missing_final_newline("last line\r"));
        assert!(!missing_final_newline("line\n"));
        assert!(!missing_final_newline("line\r\n"));
        assert!(!missing_final_newline(""));
    }

    #[test]
    fn sticky_header_follows_the_section_above_the_viewport() {
        let rows = vec![
            Row::Header("a.txt".into()),
            Row::Line("a.txt".into(), 0),
            Row::Pending("b.txt".into()),
            Row::Header("b.txt".into()),
            Row::Line("b.txt".into(), 0),
        ];
        assert_eq!(ChangesView::sticky_index_for(&rows, 0, px(0.)), None);
        assert_eq!(
            ChangesView::sticky_index_for(&rows, 0, px(2.)),
            Some(0),
            "a partially scrolled header stays pinned"
        );
        assert_eq!(ChangesView::sticky_index_for(&rows, 1, px(0.)), Some(0));
        assert_eq!(ChangesView::sticky_index_for(&rows, 2, px(0.)), Some(0));
        assert_eq!(ChangesView::sticky_index_for(&rows, 3, px(0.)), None);
        assert_eq!(ChangesView::sticky_index_for(&rows, 4, px(0.)), Some(3));
        assert_eq!(ChangesView::sticky_index_for(&[], 0, px(0.)), None);
    }
}
