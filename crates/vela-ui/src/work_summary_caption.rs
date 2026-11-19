//! Single-pass text highlight for a transcript work disclosure.

use std::{rc::Rc, sync::Arc, time::Instant};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId,
    Pixels, ShapedLine, SharedString, Style, TextRun, TruncateFrom, Window, div, point, prelude::*,
    px, relative, size,
};

const RADIUS: f32 = 25.;
const SPEED: f32 = 114.;
const MAX_FRAME_SECONDS: f32 = 1. / 30.;

struct Sweep {
    center: f32,
    last_frame: Option<Instant>,
}

impl Default for Sweep {
    fn default() -> Self {
        Self {
            center: -RADIUS,
            last_frame: None,
        }
    }
}

impl Sweep {
    fn advance(&mut self, now: Instant, width: f32) -> f32 {
        let seconds = self.last_frame.replace(now).map_or(0., |previous| {
            now.saturating_duration_since(previous)
                .as_secs_f32()
                .min(MAX_FRAME_SECONDS)
        });
        self.center += seconds * SPEED;
        // Restart only after the whole highlight has gone off the label. A
        // title change can affect when the sweep ends, never its velocity.
        if self.center >= width + RADIUS {
            self.center = -RADIUS;
        }
        self.center
    }
}

fn highlight(center: f32, x: f32) -> f32 {
    let distance = ((x - center).abs() / RADIUS).min(1.);
    (0.5 + 0.5 * (distance * std::f32::consts::PI).cos()).max(0.)
}

pub(super) fn render(title: &Arc<str>, animate: bool) -> AnyElement {
    let selector = format!("work-summary:{title}");
    div()
        .debug_selector(move || selector.clone())
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .child(
            div()
                .when(animate, |label| {
                    label.debug_selector(|| "work-summary-shimmer".into())
                })
                .child(Caption {
                    title: SharedString::from(title.clone()),
                    animate,
                }),
        )
        .into_any_element()
}

struct Caption {
    title: SharedString,
    animate: bool,
}

struct Memo {
    title: SharedString,
    run: TextRun,
    available: Pixels,
    line: Rc<ShapedLine>,
    sweep: Sweep,
}

impl Caption {
    fn prepare(&self, bounds: Bounds<Pixels>, previous: Option<Memo>, window: &Window) -> Memo {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let run = style.to_run(self.title.len());
        let available = bounds.size.width;
        let mut memo = match previous {
            Some(memo)
                if memo.title == self.title
                    && memo.run == run
                    && memo.available == available
                    && memo.line.font_size == font_size =>
            {
                memo
            }
            previous => {
                let text_system = window.text_system();
                let runs = std::slice::from_ref(&run);
                let mut line = text_system.shape_line(self.title.clone(), font_size, runs, None);
                if line.width() > available {
                    let (title, runs) = text_system
                        .line_wrapper(run.font.clone(), font_size)
                        .truncate_line(self.title.clone(), available, "…", runs, TruncateFrom::End);
                    line = text_system.shape_line(title, font_size, &runs, None);
                }
                Memo {
                    title: self.title.clone(),
                    run,
                    available,
                    line: Rc::new(line),
                    sweep: previous.map_or_else(Sweep::default, |memo| memo.sweep),
                }
            }
        };
        if !self.animate {
            memo.sweep = Sweep::default();
        }
        memo
    }
}

impl IntoElement for Caption {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Caption {
    type RequestLayoutState = ();
    type PrepaintState = Option<(Rc<ShapedLine>, gpui::Hsla, Option<f32>)>;

    fn id(&self) -> Option<ElementId> {
        Some("work-summary-caption".into())
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
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: size(relative(1.).into(), px(19.).into()),
            ..Default::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        if bounds.intersect(&window.content_mask().bounds).is_empty() {
            return None;
        }
        window.with_element_state(id.expect("caption has an id"), |previous, window| {
            let mut memo = self.prepare(bounds, previous, window);
            let center = self.animate.then(|| {
                memo.sweep.advance(
                    Instant::now(),
                    f32::from(memo.line.width().min(memo.available)),
                )
            });
            (Some((memo.line.clone(), memo.run.color, center)), memo)
        })
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        state: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let Some((line, ink, center)) = state else {
            return;
        };
        let baseline = bounds.origin
            + point(
                px(0.),
                (bounds.size.height - line.ascent - line.descent) / 2. + line.ascent,
            );
        window.paint_layer(bounds, |window| {
            for run in &line.runs {
                for glyph in &run.glyphs {
                    let mut color = *ink;
                    if let Some(center) = *center {
                        let x = f32::from(glyph.position.x + line.font_size * 0.24);
                        color.a += (1. - color.a) * highlight(center, x);
                    }
                    let origin = baseline + glyph.position;
                    if glyph.is_emoji {
                        let _ = window.paint_emoji(origin, run.font_id, glyph.id, line.font_size);
                    } else {
                        let _ = window.paint_glyph(
                            origin,
                            run.font_id,
                            glyph.id,
                            line.font_size,
                            color,
                        );
                    }
                }
            }
        });
        if center.is_some() {
            window.request_animation_frame();
        }
    }
}

#[test]
fn sweep_survives_title_growth_and_late_frames_without_jumping() {
    use std::time::Duration;

    let start = Instant::now();
    let mut sweep = Sweep::default();
    sweep.advance(start, 60.);
    let first = sweep.advance(start + Duration::from_millis(16), 60.);
    let wider = sweep.advance(start + Duration::from_millis(32), 150.);
    assert!((wider - first - SPEED * 0.016).abs() < 0.001);
    let late = sweep.advance(start + Duration::from_secs(1), 150.);
    assert!((late - wider - SPEED * MAX_FRAME_SECONDS).abs() < 0.001);

    sweep.center = 150. + RADIUS;
    assert_eq!(sweep.advance(start + Duration::from_secs(1), 150.), -RADIUS);
    assert_eq!(highlight(-RADIUS, 0.), 0.);
    assert_eq!(highlight(150. + RADIUS, 150.), 0.);
    assert_eq!(highlight(70., 70.), 1.);
}
