//! Preserve code text while accumulating fractional CSS leading before layout.
use crate::app::inline_flow::{SelectionBinding, SelectionChange};
use gpui::{
    App, AvailableSpace, Bounds, CursorStyle, Element, ElementId, GlobalElementId, Hitbox,
    HitboxBehavior, InspectorElementId, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ShapedLine, SharedString, Size, Style, TextAlign, TextRun, Window,
    WrappedLine, fill, point, prelude::*, px, size,
};
use std::{cell::RefCell, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) struct PreformattedText {
    text: SharedString,
    wrap: bool,
    runs: Option<Vec<TextRun>>,
    selection: Option<SelectionBinding>,
}
impl PreformattedText {
    pub(crate) fn new(text: impl Into<SharedString>, wrap: bool) -> Self {
        Self {
            text: text.into(),
            wrap,
            runs: None,
            selection: None,
        }
    }

    pub(crate) fn with_selection(mut self, selection: Option<SelectionBinding>) -> Self {
        self.selection = selection;
        self
    }

    /// Preserve preformatted layout while painting syntax-colored runs. Runs
    /// must cover the complete text; tabs fall back to the plain text style.
    pub(crate) fn with_runs(
        text: impl Into<SharedString>,
        wrap: bool,
        runs: Vec<TextRun>,
    ) -> Self {
        Self {
            text: text.into(),
            wrap,
            runs: Some(runs),
            selection: None,
        }
    }
}
struct PlacedLine {
    line: ShapedLine,
    position: Point<Pixels>,
    range: std::ops::Range<usize>,
}

#[derive(Default)]
pub(crate) struct Layout {
    lines: Vec<WrappedLine>,
    placed: Vec<PlacedLine>,
    line_height: Pixels,
    size: Size<Pixels>,
}

fn wrapped_height(line: &WrappedLine, line_height: Pixels) -> Pixels {
    line_height * (line.wrap_boundaries().len() + 1) as f32
}

fn selection_index(
    layout: &Layout,
    bounds: Bounds<Pixels>,
    text_len: usize,
    position: Point<Pixels>,
) -> usize {
    if !layout.placed.is_empty() {
        let local_y = (position.y - bounds.origin.y).max(px(0.));
        if local_y >= layout.size.height {
            return text_len;
        }
        let local_x = position.x - bounds.origin.x;
        let mut row = layout
            .placed
            .iter()
            .filter(|placed| {
                local_y >= placed.position.y && local_y < placed.position.y + layout.line_height
            })
            .peekable();
        if row.peek().is_none() {
            return layout
                .placed
                .iter()
                .find(|placed| placed.position.y > local_y)
                .map_or(text_len, |placed| placed.range.start);
        }
        let mut last = None;
        for placed in row {
            let left = placed.position.x;
            let right = left + placed.line.width();
            if local_x <= left {
                return placed.range.start;
            }
            if local_x < right {
                let index = placed.line.closest_index_for_x(local_x - left);
                return (placed.range.start + index).min(placed.range.end);
            }
            last = Some(placed);
        }
        return last.map_or(text_len, |placed| placed.range.end);
    }
    if layout.lines.is_empty() {
        return 0;
    }
    let total = layout.lines.iter().fold(px(0.), |height, line| {
        height + wrapped_height(line, layout.line_height)
    });
    let local_y = (position.y - bounds.origin.y).max(px(0.));
    if local_y >= total {
        return text_len;
    }
    let mut top = px(0.);
    let mut text_start = 0;
    for (index, line) in layout.lines.iter().enumerate() {
        let height = wrapped_height(line, layout.line_height);
        if local_y < top + height || index + 1 == layout.lines.len() {
            let result = line.closest_index_for_position(
                point(position.x - bounds.origin.x, local_y - top),
                layout.line_height,
            );
            let index = result.unwrap_or_else(|index| index);
            return (text_start + index).min(text_len);
        }
        top += height;
        text_start += line.len() + 1;
    }
    text_len
}

fn paint_selection(
    selection: &SelectionBinding,
    layout: &Layout,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let Some(range) = &selection.range else {
        return;
    };
    if range.is_empty() {
        return;
    }
    if !layout.placed.is_empty() {
        for placed in &layout.placed {
            let start = range.start.max(placed.range.start);
            let end = range.end.min(placed.range.end);
            if start >= end {
                continue;
            }
            let left = placed.line.x_for_index(start - placed.range.start);
            let right = placed.line.x_for_index(end - placed.range.start);
            if right > left {
                window.paint_quad(fill(
                    Bounds::new(
                        point(
                            bounds.origin.x + placed.position.x + left,
                            bounds.origin.y + placed.position.y,
                        ),
                        size(right - left, layout.line_height),
                    ),
                    selection.fill,
                ));
            }
        }
        return;
    }
    let mut top = bounds.origin.y;
    let mut text_start = 0;
    for line in &layout.lines {
        let boundaries = std::iter::once(0)
            .chain(
                line.wrap_boundaries()
                    .iter()
                    .map(|boundary| line.runs()[boundary.run_ix].glyphs[boundary.glyph_ix].index),
            )
            .chain(std::iter::once(line.len()))
            .collect::<Vec<_>>();
        for pair in boundaries.windows(2) {
            let start = range.start.max(text_start + pair[0]) - text_start;
            let end = range.end.min(text_start + pair[1]) - text_start;
            if start >= end {
                continue;
            }
            let (Some(start), Some(end)) = (
                line.position_for_index(start, layout.line_height),
                line.position_for_index(end, layout.line_height),
            ) else {
                continue;
            };
            let left = start.x;
            let right = end.x;
            if right > left {
                window.paint_quad(fill(
                    Bounds::new(
                        point(bounds.origin.x + left, top + start.y),
                        size(right - left, layout.line_height),
                    ),
                    selection.fill,
                ));
            }
        }
        top += wrapped_height(line, layout.line_height);
        text_start += line.len() + 1;
    }
}

fn next_tab_stop(x: Pixels, space: Pixels) -> Pixels {
    let step = f32::from(space) * 2.;
    let x = f32::from(x);
    let mut stop = (x / step).floor().mul_add(step, step);
    // CSS Text: a preserved tab advances at least half a character.
    if stop - x < f32::from(space) * 0.5 {
        stop += step;
    }
    px(stop)
}

fn tabbed_layout(
    text: &str,
    font_size: Pixels,
    run: &TextRun,
    width: Option<Pixels>,
    line_height: Pixels,
    window: &Window,
) -> Layout {
    let shape = |text: &str| {
        let mut run = run.clone();
        run.len = text.len();
        window
            .text_system()
            .shape_line(text.to_owned().into(), font_size, &[run], None)
    };
    let space = shape(" ").width().max(px(0.01));
    let mut layout = Layout {
        line_height,
        ..Default::default()
    };
    let (mut x, mut y, mut maximum) = (px(0.), px(0.), px(0.));
    let mut line_start = 0;
    for (line_index, logical) in text.split('\n').enumerate() {
        if line_index > 0 {
            maximum = maximum.max(x);
            x = px(0.);
            y += line_height;
        }
        let mut segment_start = 0;
        for (tab_index, segment) in logical.split('\t').enumerate() {
            if tab_index > 0 {
                x = next_tab_stop(x, space);
                segment_start += 1;
            }
            let segment_global = line_start + segment_start;
            let mut start = 0;
            for (end, _) in unicode_linebreak::linebreaks(segment) {
                let atom = &segment[start..end];
                let atom_global = segment_global + start;
                let atom_end = segment_global + end;
                start = end;
                if atom.is_empty() {
                    continue;
                }
                let line = shape(atom);
                let ink_width = line.x_for_index(atom.trim_end_matches(' ').len());
                if let Some(width) = width {
                    if x > px(0.) && x + ink_width > width && ink_width > px(0.) {
                        maximum = maximum.max(x.min(width));
                        x = px(0.);
                        y += line_height;
                    }
                    if ink_width > width {
                        let mut chunk_start = 0;
                        for (index, grapheme) in atom.grapheme_indices(true) {
                            let end = index + grapheme.len();
                            let advance = line.x_for_index(end) - line.x_for_index(chunk_start);
                            if index > chunk_start && x + advance > width {
                                let piece = shape(&atom[chunk_start..index]);
                                x += piece.width();
                                layout.placed.push(PlacedLine {
                                    line: piece,
                                    position: point(px(0.), y),
                                    range: atom_global + chunk_start..atom_global + index,
                                });
                                maximum = maximum.max(x.min(width));
                                x = px(0.);
                                y += line_height;
                                chunk_start = index;
                            }
                        }
                        let piece = shape(&atom[chunk_start..]);
                        let advance = piece.width();
                        layout.placed.push(PlacedLine {
                            line: piece,
                            position: point(x, y),
                            range: atom_global + chunk_start..atom_end,
                        });
                        x += advance;
                        continue;
                    }
                }
                let advance = line.width();
                layout.placed.push(PlacedLine {
                    line,
                    position: point(x, y),
                    range: atom_global..atom_end,
                });
                x += advance;
            }
            segment_start += segment.len();
        }
        line_start += logical.len() + 1;
    }
    layout.size = size(
        width.map_or(maximum.max(x), |width| maximum.max(x).min(width)),
        y + line_height,
    );
    layout
}

pub(crate) fn unwrapped_width(
    text: &str,
    font_size: Pixels,
    run: &TextRun,
    window: &Window,
) -> Pixels {
    if !text.contains('\t') {
        return text
            .lines()
            .map(|line| {
                let mut run = run.clone();
                run.len = line.len();
                window
                    .text_system()
                    .shape_line(line.to_owned().into(), font_size, &[run], None)
                    .width()
            })
            .fold(px(0.), Pixels::max);
    }
    tabbed_layout(text, font_size, run, None, px(1.), window)
        .size
        .width
}

impl IntoElement for PreformattedText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for PreformattedText {
    type RequestLayoutState = Rc<RefCell<Layout>>;
    type PrepaintState = Option<Hitbox>;
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
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = style
            .line_height
            .to_pixels(style.font_size, window.rem_size());
        let plain_run = style.to_run(self.text.len());
        let runs = self
            .runs
            .clone()
            .unwrap_or_else(|| vec![plain_run.clone()]);
        let text = self.text.clone();
        let wrap = self.wrap;
        let state = Rc::new(RefCell::new(Layout::default()));
        let measured = state.clone();
        let layout =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let width = if wrap {
                    known
                        .width
                        .or(match available.width {
                            AvailableSpace::Definite(width) => Some(width),
                            _ => None,
                        })
                        .map(|width| width.max(px(1.)))
                } else {
                    None
                };
                if text.contains('\t') {
                    let result =
                        tabbed_layout(&text, font_size, &plain_run, width, line_height, window);
                    let size = result.size;
                    *measured.borrow_mut() = result;
                    return size;
                }
                let lines = match window.text_system().shape_text(
                    text.clone(),
                    font_size,
                    &runs,
                    width,
                    None,
                ) {
                    Ok(lines) => lines.into_iter().collect::<Vec<_>>(),
                    Err(_) => {
                        tracing::warn!("could not shape preformatted text");
                        Vec::new()
                    }
                };
                let size = lines.iter().fold(size(px(0.), px(0.)), |mut size, line| {
                    let line = line.size(line_height);
                    size.width = size.width.max(line.width);
                    size.height += line.height;
                    size
                });
                *measured.borrow_mut() = Layout {
                    lines,
                    line_height,
                    size,
                    ..Default::default()
                };
                size
            });
        (layout, state)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        self.selection
            .as_ref()
            .map(|_| window.insert_hitbox(bounds, HitboxBehavior::Normal))
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = state.borrow();
        if let Some(selection) = &self.selection {
            paint_selection(selection, &layout, bounds, window);
        }
        let clip = window.content_mask().bounds;
        let top = clip.top() - layout.line_height;
        let bottom = clip.bottom() + layout.line_height;
        for placed in &layout.placed {
            let y = bounds.origin.y + placed.position.y;
            if y + layout.line_height < top || y > bottom {
                continue;
            }
            let _ = placed.line.paint(
                bounds.origin + placed.position,
                layout.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
        }
        let mut origin = bounds.origin;
        for line in &layout.lines {
            if origin.y > bottom {
                break;
            }
            let height = line.size(layout.line_height).height;
            if origin.y + height >= top {
                let _ = line.paint(
                    origin, layout.line_height, TextAlign::Left, Some(bounds), window, cx,
                );
            }
            origin.y += height;
        }
        drop(layout);
        let (Some(selection), Some(hitbox)) = (self.selection.clone(), hitbox.clone()) else {
            return;
        };
        if hitbox.is_hovered(window) {
            window.set_cursor_style(CursorStyle::IBeam, &hitbox);
        }
        let text = self.text.clone();
        let text_len = text.len();
        let down_hitbox = hitbox.clone();
        let down_selection = selection.clone();
        let down_layout = state.clone();
        let down_text = text.clone();
        let down_drag = selection.drag.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase.bubble() && event.button == MouseButton::Left && down_hitbox.is_hovered(window)
            {
                let layout = down_layout.borrow();
                let index = selection_index(&layout, bounds, text_len, event.position);
                (down_selection.on_change)(
                    SelectionChange::Start {
                        index,
                        text: down_text.clone(),
                        click_count: event.click_count,
                        shift: event.modifiers.shift,
                    },
                    window,
                    cx,
                );
                down_drag.set(true);
                window.refresh();
            }
        });
        let move_selection = selection.clone();
        let move_layout = state.clone();
        let move_drag = selection.drag.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase.bubble()
                && move_drag.get()
                && event.pressed_button == Some(MouseButton::Left)
                && event.position.y >= bounds.top()
                && event.position.y <= bounds.bottom()
            {
                let layout = move_layout.borrow();
                let index = selection_index(&layout, bounds, text_len, event.position);
                (move_selection.on_change)(SelectionChange::Extend { index }, window, cx);
                window.refresh();
            }
        });
        let up_selection = selection.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase.bubble() && event.button == MouseButton::Left && selection.drag.replace(false)
            {
                (up_selection.on_change)(SelectionChange::End, window, cx);
                window.refresh();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn css_tabs_use_measured_spaces_and_advance_at_least_half_a_character() {
        assert_eq!(next_tab_stop(px(0.), px(7.5)), px(15.));
        assert_eq!(next_tab_stop(px(7.5), px(7.5)), px(15.));
        assert_eq!(next_tab_stop(px(15.), px(7.5)), px(30.));
        assert_eq!(next_tab_stop(px(14.9), px(7.5)), px(30.));
        assert_eq!(next_tab_stop(px(11.25), px(7.5)), px(15.));
    }

    #[gpui::test]
    fn tab_layout_records_byte_ranges_and_maps_pointer_positions(cx: &mut gpui::TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, _| {
            let text: SharedString = "a\tb\n\tlet x = 1;".into();
            let run = TextRun {
                len: text.len(),
                font: crate::theme::source_editor_font(),
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let layout = tabbed_layout(&text, px(14.), &run, Some(px(500.)), px(18.), window);
            let ranges = layout
                .placed
                .iter()
                .map(|placed| placed.range.clone())
                .collect::<Vec<_>>();
            assert!(ranges.contains(&(0..1)), "first segment keeps its range");
            assert!(
                ranges.contains(&(2..3)),
                "post-tab segment is offset past the tab"
            );
            let second_line = ranges
                .iter()
                .filter(|range| range.start >= 5)
                .collect::<Vec<_>>();
            assert_eq!(
                second_line.first().map(|range| range.start),
                Some(5),
                "the next line starts after its tab: {ranges:?}"
            );
            assert_eq!(
                second_line.last().map(|range| range.end),
                Some(15),
                "the line ranges cover the text through its final byte: {ranges:?}"
            );
            assert!(
                ranges.windows(2).all(|pair| pair[0].end <= pair[1].start),
                "ranges stay ordered: {ranges:?}"
            );
            for range in &ranges {
                assert!(!text[range.clone()].is_empty());
            }
            let wrapped = tabbed_layout(&text, px(14.), &run, Some(px(20.)), px(18.), window);
            let wrapped_ranges = wrapped
                .placed
                .iter()
                .map(|placed| placed.range.clone())
                .collect::<Vec<_>>();
            assert!(
                wrapped_ranges.len() > ranges.len(),
                "narrow widths split long atoms into more chunks"
            );
            assert_eq!(
                wrapped_ranges.first().map(|range| range.start),
                Some(0)
            );
            assert_eq!(
                wrapped_ranges.last().map(|range| range.end),
                Some(text.len())
            );
            assert!(
                wrapped_ranges
                    .windows(2)
                    .all(|pair| pair[0].end <= pair[1].start),
                "wrapped ranges stay ordered: {wrapped_ranges:?}"
            );
            for range in &wrapped_ranges {
                assert!(!text[range.clone()].is_empty());
            }
            let bounds = Bounds::new(point(px(0.), px(0.)), layout.size);
            let second = layout
                .placed
                .iter()
                .find(|placed| placed.range == (2..3))
                .unwrap();
            let index = selection_index(
                &layout,
                bounds,
                text.len(),
                point(second.position.x + px(1.), px(1.)),
            );
            assert!(
                (2..=3).contains(&index),
                "pointer hit testing returns a byte inside the post-tab chunk: {index}"
            );
        });
    }
}
