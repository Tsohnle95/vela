//! Preserve code text while accumulating fractional CSS leading before layout.
use gpui::{
    App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId,
    Pixels, Point, ShapedLine, SharedString, Size, Style, TextAlign, TextRun, Window, WrappedLine,
    point, prelude::*, px, size,
};
use std::{cell::RefCell, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) struct PreformattedText {
    text: SharedString,
    wrap: bool,
    runs: Option<Vec<TextRun>>,
}
impl PreformattedText {
    pub(crate) fn new(text: impl Into<SharedString>, wrap: bool) -> Self {
        Self {
            text: text.into(),
            wrap,
            runs: None,
        }
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
        }
    }
}
#[derive(Default)]
pub(crate) struct Layout {
    lines: Vec<WrappedLine>,
    placed: Vec<(ShapedLine, Point<Pixels>)>,
    line_height: Pixels,
    size: Size<Pixels>,
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
    for (line_index, logical) in text.split('\n').enumerate() {
        if line_index > 0 {
            maximum = maximum.max(x);
            x = px(0.);
            y += line_height;
        }
        for (tab_index, segment) in logical.split('\t').enumerate() {
            if tab_index > 0 {
                x = next_tab_stop(x, space);
            }
            let mut start = 0;
            for (end, _) in unicode_linebreak::linebreaks(segment) {
                let atom = &segment[start..end];
                start = end;
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
                                layout.placed.push((piece, point(px(0.), y)));
                                maximum = maximum.max(x.min(width));
                                x = px(0.);
                                y += line_height;
                                chunk_start = index;
                            }
                        }
                        let piece = shape(&atom[chunk_start..]);
                        let advance = piece.width();
                        layout.placed.push((piece, point(x, y)));
                        x += advance;
                        continue;
                    }
                }
                let advance = line.width();
                layout.placed.push((line, point(x, y)));
                x += advance;
            }
        }
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
    type PrepaintState = ();
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
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = state.borrow();
        for (line, position) in &layout.placed {
            let _ = line.paint(
                bounds.origin + *position,
                layout.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
        }
        let mut origin = bounds.origin;
        for line in &layout.lines {
            let _ = line.paint(
                origin,
                layout.line_height,
                TextAlign::Left,
                Some(bounds),
                window,
                cx,
            );
            origin.y += line.size(layout.line_height).height;
        }
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
}
