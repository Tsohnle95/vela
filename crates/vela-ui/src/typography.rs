//! Tracking for Vela's static single-line Settings labels. Shape the whole line
//! before applying tracking so kerning and fractional advances are preserved;
//! separate character elements round every advance and accumulate width error.
use gpui::{prelude::*, *};
use std::sync::Arc;

/// Include the actual fallback faces in a CSS normal inline box. GPUI's stored
/// line ascent/descent describe its declared runs; resolved glyph fonts can be
/// taller, particularly the full-size Apple Color Emoji face.
pub(crate) fn normal_line_metrics(line: &ShapedLine, window: &Window) -> (f32, f32) {
    let mut ascent = f32::from(line.ascent);
    let mut descent = f32::from(line.descent);
    for run in &line.runs {
        ascent = ascent.max(f32::from(
            window.text_system().ascent(run.font_id, line.font_size),
        ));
        descent =
            descent.max(f32::from(window.text_system().descent(run.font_id, line.font_size)).abs());
    }
    let height = (ascent + descent).ceil();
    let baseline_delta = (ascent - descent - f32::from(line.ascent - line.descent)) / 2.;
    #[cfg(debug_assertions)]
    if line
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .any(|glyph| glyph.is_emoji)
    {
        static LOGGED: std::sync::Once = std::sync::Once::new();
        LOGGED.call_once(|| {
            let metrics=line.runs.iter().map(|run| (
                f32::from(window.text_system().ascent(run.font_id,line.font_size)),
                f32::from(window.text_system().descent(run.font_id,line.font_size)),
                run.glyphs.iter().any(|glyph|glyph.is_emoji),
            )).collect::<Vec<_>>();
            tracing::debug!(target:"vela_ui::theme",?metrics,height,baseline_delta,"resolved emoji inline metrics");
        });
    }
    (height, baseline_delta)
}

pub(crate) struct TrackedText {
    text: SharedString,
    spacing: Pixels,
}
impl TrackedText {
    pub fn new(text: impl Into<SharedString>, spacing: Pixels) -> Self {
        Self {
            text: text.into(),
            spacing,
        }
    }
}
impl IntoElement for TrackedText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for TrackedText {
    type RequestLayoutState = (ShapedLine, Pixels);
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
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let height = style.line_height_in_pixels(window.rem_size());
        let mut line = window.text_system().shape_line(
            self.text.clone(),
            font_size,
            &[style.to_run(self.text.len())],
            None,
        );
        let mut runs = line.runs.clone();
        let indices = self
            .text
            .char_indices()
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        for run in &mut runs {
            for glyph in &mut run.glyphs {
                let index = indices.partition_point(|index| *index < glyph.index);
                glyph.position.x += self.spacing * index;
            }
        }
        let width = (line.width() + self.spacing * indices.len()).max(px(0.));
        *line = Arc::new(LineLayout {
            font_size: line.font_size,
            width,
            ascent: line.ascent,
            descent: line.descent,
            runs,
            len: line.len(),
        });
        let layout = window.request_measured_layout(Style::default(), move |known, _, _, _| {
            size(known.width.unwrap_or(width), height)
        });
        (layout, (line, height))
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
        let _ = state
            .0
            .paint(bounds.origin, state.1, TextAlign::Left, None, window, cx);
    }
}
