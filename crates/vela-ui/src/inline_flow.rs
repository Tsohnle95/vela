//! Fractional inline layout for Markdown prose and codespans. Syntax stays in Markdown;
//! this element only shapes, wraps and paints the resulting styled text.
use gpui::{
    App, AvailableSpace, BorderStyle, Bounds, CursorStyle, Edges, Element, ElementId,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, LayoutId, MouseButton,
    MouseDownEvent, MouseUpEvent, Pixels, Point, Rgba, ShapedLine, SharedString, Size, Style,
    TextAlign, TextRun, Window, point, prelude::*, px, quad, size, transparent_black,
};
use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::Rc,
};
use unicode_segmentation::UnicodeSegmentation;

pub(super) struct InlineFlow {
    id: SharedString,
    text: SharedString,
    runs: Vec<TextRun>,
    code: Vec<Range<usize>>,
    links: Vec<(Range<usize>, String)>,
    code_background: Rgba,
}
impl InlineFlow {
    pub fn new(
        id: SharedString,
        text: SharedString,
        runs: Vec<TextRun>,
        code: Vec<Range<usize>>,
        links: Vec<(Range<usize>, String)>,
        code_background: Rgba,
    ) -> Self {
        Self {
            id,
            text,
            runs,
            code,
            links,
            code_background,
        }
    }
}
#[derive(Clone)]
struct Piece {
    line: ShapedLine,
    range: Range<usize>,
    code: Option<usize>,
    left: Pixels,
    right: Pixels,
}
impl Piece {
    fn width(&self) -> Pixels {
        self.left + self.line.width() + self.right
    }
}
#[derive(Clone)]
struct Atom {
    pieces: Vec<Piece>,
    newline: bool,
}
#[derive(Clone)]
struct Placed {
    piece: Piece,
    position: Point<Pixels>,
    row: usize,
}
#[derive(Default, Clone)]
pub(super) struct FlowLayout {
    placed: Vec<Placed>,
    size: Size<Pixels>,
    baseline: Pixels,
    line_height: Pixels,
}
#[derive(Default)]
struct LinkState {
    down: Rc<Cell<Option<usize>>>,
}

fn pieces(
    text: &str,
    runs: &[TextRun],
    codes: &[Range<usize>],
    range: Range<usize>,
    font_size: Pixels,
    window: &Window,
) -> Vec<Piece> {
    let mut offset = 0;
    let mut result = Vec::new();
    for run in runs {
        let start = range.start.max(offset);
        let end = range.end.min(offset + run.len);
        offset += run.len;
        if start >= end {
            continue;
        }
        let code = codes
            .iter()
            .position(|code| code.start <= start && end <= code.end);
        let mut run = run.clone();
        run.len = end - start;
        run.background_color = None;
        let line = window.text_system().shape_line(
            SharedString::from(text[start..end].to_owned()),
            if code.is_some() {
                font_size * 0.875
            } else {
                font_size
            },
            &[run],
            None,
        );
        result.push(Piece {
            line,
            range: start..end,
            code,
            left: px(if code.is_some_and(|index| codes[index].start == start) {
                5.
            } else {
                0.
            }),
            right: px(if code.is_some_and(|index| codes[index].end == end) {
                5.
            } else {
                0.
            }),
        });
    }
    result
}
fn atoms(
    text: &str,
    runs: &[TextRun],
    codes: &[Range<usize>],
    font_size: Pixels,
    window: &Window,
) -> Vec<Atom> {
    let mut start = 0;
    unicode_linebreak::linebreaks(text)
        .map(|(end, _)| {
            let newline = text[start..end].ends_with('\n');
            let content_end = if newline { end - 1 } else { end };
            // Break spaces do not make a fitting word overflow its line.
            // Keep them separate so a boundary space can be discarded without
            // splitting the word or moving it with its inline-code padding.
            let trimmed = start
                + text[start..content_end]
                    .trim_end_matches([' ', '\t', '\r'])
                    .len();
            let mut shaped = pieces(text, runs, codes, start..trimmed, font_size, window);
            shaped.extend(pieces(
                text,
                runs,
                codes,
                trimmed..content_end,
                font_size,
                window,
            ));
            let atom = Atom {
                pieces: shaped,
                newline,
            };
            start = end;
            atom
        })
        .collect()
}
struct FlowInput {
    text: SharedString,
    runs: Vec<TextRun>,
    code: Vec<Range<usize>>,
    atoms: Vec<Atom>,
    font_size: Pixels,
    line_height: Pixels,
    baseline: Pixels,
}
fn intrinsic_width(input: &FlowInput) -> Pixels {
    let (mut advance, mut ink_end, mut maximum) = (px(0.), px(0.), px(0.));
    for atom in &input.atoms {
        for piece in &atom.pieces {
            advance += piece.width();
            if piece.code.is_some()
                || input.text[piece.range.clone()]
                    .chars()
                    .any(|ch| !ch.is_whitespace())
            {
                ink_end = advance;
            }
        }
        if atom.newline {
            maximum = maximum.max(ink_end);
            advance = px(0.);
            ink_end = px(0.);
        }
    }
    maximum.max(ink_end)
}
fn arrange(input: &FlowInput, width: Pixels, window: &Window) -> FlowLayout {
    let width = width.max(px(1.));
    let mut layout = FlowLayout {
        baseline: input.baseline,
        line_height: input.line_height,
        ..Default::default()
    };
    let (mut x, mut row, mut maximum) = (px(0.), 0, px(0.));
    for atom in &input.atoms {
        let word_width = atom
            .pieces
            .iter()
            .rev()
            .skip_while(|piece| {
                input.text[piece.range.clone()]
                    .chars()
                    .all(|ch| matches!(ch, ' ' | '\t' | '\r'))
            })
            .fold(px(0.), |sum, piece| sum + piece.width());
        if x > px(0.) && x + word_width > width {
            maximum = maximum.max(x);
            row += 1;
            x = px(0.);
        }
        let fragments = if word_width > width {
            atom.pieces
                .iter()
                .flat_map(|piece| {
                    input.text[piece.range.clone()]
                        .grapheme_indices(true)
                        .flat_map(|(index, text)| {
                            pieces(
                                &input.text,
                                &input.runs,
                                &input.code,
                                piece.range.start + index..piece.range.start + index + text.len(),
                                input.font_size,
                                window,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        } else {
            atom.pieces.clone()
        };
        for piece in fragments {
            let space = input.text[piece.range.clone()]
                .chars()
                .all(|ch| matches!(ch, ' ' | '\t' | '\r'));
            if space && (x == px(0.) || x + piece.width() > width) {
                continue;
            }
            if x > px(0.) && x + piece.width() > width {
                maximum = maximum.max(x);
                row += 1;
                x = px(0.);
            }
            let advance = piece.width();
            layout.placed.push(Placed {
                piece,
                position: point(x, input.line_height * row as f32),
                row,
            });
            x += advance;
        }
        if atom.newline {
            maximum = maximum.max(x);
            row += 1;
            x = px(0.);
        }
    }
    layout.size = size(
        maximum.max(x).min(width),
        input.line_height * (row + 1) as f32,
    );
    layout
}
impl IntoElement for InlineFlow {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for InlineFlow {
    type RequestLayoutState = Rc<RefCell<FlowLayout>>;
    type PrepaintState = Option<Hitbox>;
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone().into())
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
        let base = window
            .text_system()
            .shape_line("M".into(), font_size, &[style.to_run(1)], None);
        let input = FlowInput {
            text: self.text.clone(),
            runs: self.runs.clone(),
            code: self.code.clone(),
            atoms: atoms(&self.text, &self.runs, &self.code, font_size, window),
            font_size,
            line_height,
            baseline: (line_height - base.ascent - base.descent) / 2. + base.ascent,
        };
        let measured = Rc::new(RefCell::new(FlowLayout::default()));
        let output = measured.clone();
        let intrinsic_width = intrinsic_width(&input);
        let layout =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let width = known.width.unwrap_or(match available.width {
                    AvailableSpace::Definite(width) => width,
                    _ => intrinsic_width,
                });
                let layout = arrange(&input, width, window);
                // A wrapped shrink-to-fit box keeps its available width;
                // its final short line must not shrink the box a second time.
                let size = size(intrinsic_width.min(width), layout.size.height);
                *output.borrow_mut() = layout;
                size
            });
        (layout, measured)
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
        (!self.links.is_empty()).then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal))
    }
    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = state.borrow();
        let mut backgrounds: Vec<(usize, usize, Bounds<Pixels>)> = Vec::new();
        let mut links = Vec::new();
        for placed in &layout.placed {
            let line = &placed.piece.line;
            let origin = bounds.origin + placed.position;
            if let Some(code) = placed.piece.code {
                let box_bounds = Bounds::new(
                    point(origin.x, origin.y + layout.baseline - line.ascent - px(1.)),
                    size(placed.piece.width(), line.ascent + line.descent + px(2.)),
                );
                if let Some((previous_row, previous_code, previous)) = backgrounds.last_mut()
                    && *previous_row == placed.row
                    && *previous_code == code
                {
                    previous.size.width = box_bounds.right() - previous.left();
                } else {
                    backgrounds.push((placed.row, code, box_bounds));
                }
            }
            for (index, (range, _)) in self.links.iter().enumerate() {
                if range.start < placed.piece.range.end && placed.piece.range.start < range.end {
                    links.push((
                        index,
                        Bounds::new(origin, size(placed.piece.width(), layout.line_height)),
                    ));
                }
            }
        }
        for (_, _, background) in backgrounds {
            window.paint_quad(quad(
                background,
                px(4.),
                self.code_background,
                Edges::all(px(0.)),
                transparent_black(),
                BorderStyle::Solid,
            ));
        }
        for placed in &layout.placed {
            let line = &placed.piece.line;
            let natural_baseline =
                (layout.line_height - line.ascent - line.descent) / 2. + line.ascent;
            let origin = bounds.origin
                + placed.position
                + point(placed.piece.left, layout.baseline - natural_baseline);
            let _ = line.paint(
                origin,
                layout.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
        }
        drop(layout);
        if let Some(hitbox) = hitbox {
            if links
                .iter()
                .any(|(_, bounds)| bounds.contains(&window.mouse_position()))
            {
                window.set_cursor_style(CursorStyle::PointingHand, hitbox);
            }
            let urls = self
                .links
                .iter()
                .map(|(_, url)| url.clone())
                .collect::<Vec<_>>();
            window.with_element_state::<LinkState, _>(
                global_id.expect("inline flow has an id"),
                |state, window| {
                    let state = state.unwrap_or_default();
                    let down = state.down.clone();
                    let hitbox = hitbox.clone();
                    let down_links = links.clone();
                    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, _| {
                        if phase.bubble()
                            && event.button == MouseButton::Left
                            && hitbox.is_hovered(window)
                        {
                            down.set(
                                down_links
                                    .iter()
                                    .find(|(_, bounds)| bounds.contains(&event.position))
                                    .map(|(index, _)| *index),
                            );
                            window.refresh();
                        }
                    });
                    let down = state.down.clone();
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                        if phase.bubble() && event.button == MouseButton::Left {
                            let previous = down.take();
                            if let Some(index) = previous
                                && links.iter().any(|(link, bounds)| {
                                    *link == index && bounds.contains(&event.position)
                                })
                            {
                                if let Some(url) = vela_browser::safe_external_url(&urls[index]) {
                                    cx.open_url(&url);
                                }
                                window.refresh();
                            }
                        }
                    });
                    ((), state)
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, black};
    #[gpui::test]
    fn trailing_code_space_does_not_push_a_fitting_word_to_the_next_line(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window,_| {
            let text:SharedString="Before inline code".into();
            let code=7..text.len();
            let run=|len,font|TextRun {len,font,color:black(),background_color:None,underline:None,strikethrough:None};
            let runs=vec![run(7,crate::theme::ui_font()),run(text.len()-7,crate::theme::source_editor_font())];
            let width_of=|range|pieces(&text,&runs,std::slice::from_ref(&code),range,px(14.),window).iter().fold(px(0.),|sum,piece|sum+piece.width());
            let width=width_of(0..7)+width_of(7..13)+px(1.);
            let input=FlowInput {text:text.clone(),runs:runs.clone(),code:vec![code.clone()],atoms:atoms(&text,&runs,std::slice::from_ref(&code),px(14.),window),font_size:px(14.),line_height:px(22.75),baseline:px(16.)};
            let layout=arrange(&input,width,window);
            assert_eq!(layout.placed.iter().find(|piece|piece.piece.code.is_some()).unwrap().row,0,"the code word fits when its trailing break space is excluded");
            assert!(layout.placed.iter().any(|piece|piece.row==1&&text[piece.piece.range.clone()].contains("code")));
        });
    }

    #[gpui::test]
    fn inline_code_uses_its_own_size_padding_and_unicode_safe_wrapping(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, _| {
            let code = "literal **code** 👩‍💻";
            let text: SharedString = format!("Before {code} after\n\nlast").into();
            let code_range = 7..7 + code.len();
            let font = crate::theme::ui_font();
            let run = |len, font| TextRun {
                len,
                font,
                color: black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs = vec![
                run(7, font.clone()),
                run(code.len(), crate::theme::source_editor_font()),
                run(text.len() - code_range.end, font),
            ];
            let input = FlowInput {
                text: text.clone(),
                runs: runs.clone(),
                code: vec![code_range.clone()],
                atoms: atoms(
                    &text,
                    &runs,
                    std::slice::from_ref(&code_range),
                    px(14.),
                    window,
                ),
                font_size: px(14.),
                line_height: px(22.75),
                baseline: px(16.),
            };
            let wide = arrange(&input, px(2000.), window);
            assert_eq!(
                wide.size.height,
                px(22.75 * 3.),
                "explicit blank lines must survive"
            );
            assert!(
                wide.placed
                    .iter()
                    .filter(|piece| piece.piece.code.is_some())
                    .all(|piece| piece.piece.line.font_size == px(12.25))
            );
            assert!(
                wide.placed
                    .iter()
                    .filter(|piece| piece.piece.code.is_none())
                    .all(|piece| piece.piece.line.font_size == px(14.))
            );
            let padding = wide
                .placed
                .iter()
                .filter(|piece| piece.piece.code.is_some())
                .fold(px(0.), |sum, piece| {
                    sum + piece.piece.left + piece.piece.right
                });
            assert_eq!(padding, px(10.));
            let narrow = arrange(&input, px(45.), window);
            assert!(narrow.size.height > wide.size.height);
            let actual = narrow
                .placed
                .iter()
                .map(|placed| &text[placed.piece.range.clone()])
                .collect::<String>();
            let nonspace = |text: &str| {
                text.chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>()
            };
            assert_eq!(
                nonspace(&actual),
                nonspace(&text),
                "reflow must not drop or duplicate source text"
            );
            assert!(
                narrow
                    .placed
                    .iter()
                    .any(|piece| piece.piece.line.text.as_ref() == "👩‍💻")
            );
        });
    }
}
