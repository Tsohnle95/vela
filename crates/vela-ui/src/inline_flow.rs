//! Fractional inline layout for Markdown prose and codespans. Syntax stays in Markdown;
//! this element only shapes, wraps and paints the resulting styled text.
use gpui::{
    App, AvailableSpace, BorderStyle, Bounds, CursorStyle, Edges, Element, ElementId,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Rgba, ShapedLine, SharedString,
    Size, Style, TextAlign, TextRun, Window, fill, point, prelude::*, px, quad, size,
    transparent_black,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ops::Range,
    rc::Rc,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

/// A selection gesture reported by an inline flow. `text` accompanies the
/// initial press so the owner can copy exactly the rendered characters.
#[derive(Clone)]
pub(crate) enum SelectionChange {
    Start {
        index: usize,
        text: SharedString,
        click_count: usize,
        shift: bool,
    },
    Extend {
        index: usize,
    },
    End,
}

pub(crate) type SelectionHandler = Rc<dyn Fn(SelectionChange, &mut Window, &mut App)>;

/// Optional browser-style text selection for a single inline flow. `drag` is
/// shared by every flow in one user document so a drag can cross segments.
#[derive(Clone)]
pub(crate) struct SelectionBinding {
    pub range: Option<Range<usize>>,
    pub fill: Rgba,
    pub drag: Rc<Cell<bool>>,
    pub on_change: SelectionHandler,
}

pub(super) struct InlineFlow {
    id: SharedString,
    text: SharedString,
    runs: Vec<TextRun>,
    code: Vec<Range<usize>>,
    links: Vec<(Range<usize>, String)>,
    code_background: Rgba,
    selection: Option<SelectionBinding>,
    stream_fade: Option<bool>,
}
impl InlineFlow {
    pub fn new(
        id: SharedString,
        text: SharedString,
        runs: Vec<TextRun>,
        code: Vec<Range<usize>>,
        links: Vec<(Range<usize>, String)>,
        code_background: Rgba,
        selection: Option<SelectionBinding>,
    ) -> Self {
        Self {
            id,
            text,
            runs,
            code,
            links,
            code_background,
            selection,
            stream_fade: None,
        }
    }

    pub(super) fn with_stream_fade(mut self, streaming: Option<bool>) -> Self {
        self.stream_fade = streaming;
        self
    }
}

/// Map a window-space pointer to the nearest byte boundary in the flow text.
fn index_at(
    layout: &FlowLayout,
    text: &str,
    bounds: Bounds<Pixels>,
    position: Point<Pixels>,
) -> usize {
    if layout.placed.is_empty() {
        return 0;
    }
    let local_y = position.y - bounds.origin.y;
    let line_height = layout.line_height.max(px(1.));
    let last_row = layout
        .placed
        .iter()
        .map(|piece| piece.row)
        .max()
        .unwrap_or(0);
    if local_y >= line_height * (last_row + 1) as f32 {
        return text.len();
    }
    if local_y < px(0.) {
        return 0;
    }
    let row = (f32::from(local_y) / f32::from(line_height)).floor() as usize;
    let mut pieces = layout
        .placed
        .iter()
        .filter(|piece| piece.row == row)
        .peekable();
    let Some(first) = pieces.peek().copied() else {
        // An empty visual row: snap to the first piece that follows it.
        return layout
            .placed
            .iter()
            .find(|piece| piece.row > row)
            .map_or(text.len(), |piece| piece.piece.range.start);
    };
    let local_x = position.x - bounds.origin.x;
    if local_x <= first.position.x + first.piece.left {
        return first.piece.range.start;
    }
    let mut last: Option<&Placed> = None;
    for piece in pieces {
        let text_x = piece.position.x + piece.piece.left;
        if local_x <= text_x + piece.piece.line.width() {
            let index = piece
                .piece
                .line
                .closest_index_for_x((local_x - text_x).max(px(0.)));
            return (piece.piece.range.start + index).min(piece.piece.range.end);
        }
        last = Some(piece);
    }
    last.map_or(0, |piece| piece.piece.range.end)
}

fn paint_selection(
    selection: &SelectionBinding,
    layout: &FlowLayout,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let Some(range) = &selection.range else {
        return;
    };
    if range.is_empty() {
        return;
    }
    for placed in &layout.placed {
        let start = range.start.max(placed.piece.range.start);
        let end = range.end.min(placed.piece.range.end);
        if start >= end {
            continue;
        }
        let left = placed
            .piece
            .line
            .x_for_index(start - placed.piece.range.start);
        let right = placed
            .piece
            .line
            .x_for_index(end - placed.piece.range.start);
        if right <= left {
            continue;
        }
        let origin = bounds.origin + placed.position + point(placed.piece.left, px(0.));
        window.paint_quad(fill(
            Bounds::new(
                point(origin.x + left, bounds.origin.y + placed.position.y),
                size(right - left, layout.line_height),
            ),
            selection.fill,
        ));
    }
}
#[derive(Clone)]
struct Piece {
    // GPUI's shaped line embeds a large decoration buffer. Layout positions
    // and reused prefix atoms all share the one shaped result.
    line: Rc<ShapedLine>,
    run: Rc<TextRun>,
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
    source: Range<usize>,
    pieces: Rc<[Piece]>,
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

const FADE_STEP: Duration = Duration::from_millis(11);
const FADE_STEPS: usize = 9;
const FADE_DURATION: Duration = Duration::from_millis(110);
const FADE_INITIAL_OPACITY: f32 = 0.20;

#[derive(Default)]
struct StreamFade {
    source: SharedString,
    /// Each range ends at the next range's start, or at the end of source.
    starts: Vec<(usize, Instant)>,
}

impl StreamFade {
    fn update(&mut self, source: &SharedString, now: Instant) {
        self.starts.retain(|&(_, start)| now < start + FADE_DURATION);
        if self.source == *source {
            return;
        }
        let Some(tail) = source.strip_prefix(self.source.as_ref()) else {
            // Markdown may reinterpret a delimiter and replace this flow.
            // Keep the replacement opaque rather than replaying old words.
            self.starts.clear();
            self.source = source.clone();
            return;
        };
        let offset = self.source.len();
        let latest = now + FADE_STEP * FADE_STEPS as u32;
        let start = self.starts.last().map_or(now, |&(_, at)| {
            (at + FADE_STEP).clamp(now, latest)
        });
        self.starts.push((offset, start));
        let breaks = unicode_linebreak::linebreaks(tail)
            .filter(|&(index, _)| index < tail.len())
            .take(FADE_STEPS + 1)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let count = breaks.len().min(FADE_STEPS);
        if count > 0 {
            let interval = FADE_STEP.min((latest - start) / count as u32);
            for slot in 1..=count {
                // For a burst larger than our frame budget, spread groups
                // across the entire suffix instead of releasing its tail together.
                let index = if breaks.len() > FADE_STEPS {
                    tail.floor_char_boundary(tail.len() * slot / (count + 1))
                } else {
                    breaks[slot - 1]
                };
                self.starts.push((offset + index, start + interval * slot as u32));
            }
        }
        self.source = source.clone();
    }

    fn opacity(&self, offset: usize, now: Instant) -> f32 {
        let index = self.starts.partition_point(|&(start, _)| start <= offset);
        let Some(&(_, start)) = index.checked_sub(1).and_then(|i| self.starts.get(i)) else {
            return 1.;
        };
        let t = (now.saturating_duration_since(start).as_secs_f32()
            / FADE_DURATION.as_secs_f32()).min(1.);
        FADE_INITIAL_OPACITY + (1. - FADE_INITIAL_OPACITY) * t * (2. - t)
    }
}

#[test]
fn streamed_words_keep_their_arrival_time_and_bursts_catch_up() {
    let now = Instant::now();
    let mut fade = StreamFade::default();
    fade.update(&"Already visible ".into(), now);
    let later = now + Duration::from_secs(1);
    let text: SharedString = format!("Already visible {}", "new word ".repeat(200)).into();
    fade.update(&text, later);
    assert_eq!(fade.opacity(0, later), 1.);
    assert_eq!(fade.opacity(16, later), FADE_INITIAL_OPACITY);
    let halfway = later + FADE_STEP * 2;
    assert!(fade.opacity(16, halfway) > FADE_INITIAL_OPACITY);
    assert_eq!(fade.opacity(text.len() - 1, halfway), FADE_INITIAL_OPACITY);
    let approaching_end = later + FADE_STEP * 8;
    assert!(fade.opacity(text.len() / 2, approaching_end) > FADE_INITIAL_OPACITY);
    assert_eq!(fade.opacity(text.len() - 1, approaching_end), FADE_INITIAL_OPACITY);
    fade.update(&text, halfway);
    assert!(fade.opacity(16, halfway) > FADE_INITIAL_OPACITY);
    let settled = later + FADE_STEP * FADE_STEPS as u32 + FADE_DURATION;
    fade.update(&text, settled);
    assert!(fade.starts.is_empty());
    assert_eq!(fade.opacity(text.len() - 1, settled), 1.);
}

#[test]
fn unchanged_style_extends_through_a_growing_text_run() {
    let old = TextRun { len: 12, ..Default::default() };
    let mut next = TextRun { len: 21, ..old.clone() };
    assert_eq!(shared_style_prefix(std::slice::from_ref(&next), std::slice::from_ref(&old)), 12);
    next.color = gpui::red();
    assert_eq!(shared_style_prefix(&[next], &[old]), 0);
}

fn pieces(
    text: &str,
    runs: &[TextRun],
    run_ends: &[usize],
    codes: &[Range<usize>],
    range: Range<usize>,
    font_size: Pixels,
    window: &Window,
) -> Vec<Piece> {
    let mut result = Vec::new();
    if range.is_empty() {
        return result;
    }
    let first_run = run_ends.partition_point(|end| *end <= range.start);
    let mut offset = first_run.checked_sub(1).map_or(0, |index| run_ends[index]);
    for (run, &run_end) in runs[first_run..].iter().zip(&run_ends[first_run..]) {
        if offset >= range.end {
            break;
        }
        let start = range.start.max(offset);
        let end = range.end.min(run_end);
        offset = run_end;
        if start >= end {
            continue;
        }
        let index = codes.partition_point(|code| code.end <= start);
        let code = codes.get(index)
            .filter(|code| code.start <= start && end <= code.end)
            .map(|_| index);
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
            std::slice::from_ref(&run),
            None,
        );
        result.push(Piece {
            line: Rc::new(line),
            run: Rc::new(run),
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

fn run_ends(runs: &[TextRun]) -> Vec<usize> {
    runs.iter().scan(0, |end, run| {
        *end += run.len;
        Some(*end)
    }).collect()
}

fn shared_style_prefix(runs: &[TextRun], previous: &[TextRun]) -> usize {
    let mut bytes = 0;
    for (current, old) in runs.iter().zip(previous) {
        if current == old {
            bytes += current.len;
            continue;
        }
        let mut overlap = current.clone();
        overlap.len = old.len;
        if overlap == *old {
            bytes += current.len.min(old.len);
        }
        break;
    }
    bytes
}

fn atoms(
    text: &str,
    runs: &[TextRun],
    run_ends: &[usize],
    codes: &[Range<usize>],
    font_size: Pixels,
    window: &Window,
    previous: Option<&FlowInput>,
) -> Vec<Atom> {
    // Markdown can revise an unclosed style or a Unicode break as text
    // arrives. Reuse shapes only for identical styled bytes with the same
    // actual break range, after running the line breaker on the new source.
    let previous = previous.filter(|prior| {
        prior.font_size == font_size && text.starts_with(prior.text.as_ref()) && prior.code == codes
    });
    let shared = previous.map_or(0, |prior| shared_style_prefix(runs, &prior.runs));
    let mut start = 0;
    unicode_linebreak::linebreaks(text)
        .enumerate()
        .map(|(index, (end, _))| {
            let source = start..end;
            if end <= shared
                && let Some(atom) = previous.and_then(|prior| prior.atoms.get(index))
                && atom.source == source
            {
                start = end;
                return atom.clone();
            }
            let newline = text[start..end].ends_with('\n');
            let content_end = if newline { end - 1 } else { end };
            // Break spaces do not make a fitting word overflow its line.
            // Keep them separate so a boundary space can be discarded without
            // splitting the word or moving it with its inline-code padding.
            let trimmed = start
                + text[start..content_end]
                    .trim_end_matches([' ', '\t', '\r'])
                    .len();
            let mut shaped = pieces(text, runs, run_ends, codes, start..trimmed, font_size, window);
            shaped.extend(pieces(
                text,
                runs,
                run_ends,
                codes,
                trimmed..content_end,
                font_size,
                window,
            ));
            let atom = Atom {
                source,
                pieces: shaped.into(),
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
    run_ends: Vec<usize>,
    code: Vec<Range<usize>>,
    atoms: Vec<Atom>,
    font_size: Pixels,
    line_height: Pixels,
    baseline: Pixels,
}

// Rows can be redrawn by the caret, header indicators, or another panel's
// stream. Reuse unchanged geometry without retaining any selection or handlers.
#[derive(Default)]
struct FlowCache {
    entries: HashMap<(gpui::WindowId, GlobalElementId), (u64, Rc<CachedFlow>)>,
    clock: u64,
    weight: usize,
}
#[derive(Default)]
struct FlowCacheStore(RefCell<FlowCache>);
impl gpui::Global for FlowCacheStore {}

type WidthLayout = Option<(Pixels, Rc<FlowLayout>)>;

struct CachedFlow {
    input: FlowInput,
    base_run: TextRun,
    scale: f32,
    width: Pixels,
    weight: usize,
    layouts: RefCell<[WidthLayout; 2]>,
}

impl CachedFlow {
    fn matches(&self, element: &InlineFlow, base_run: &TextRun, font: Pixels, line: Pixels, scale: f32) -> bool {
        self.input.text == element.text
            && self.input.runs == element.runs
            && self.input.code == element.code
            && self.input.font_size == font
            && self.input.line_height == line
            && self.base_run == *base_run
            && self.scale == scale
    }
}
impl FlowInput {
    #[cfg(test)]
    fn new(
        text: &SharedString,
        runs: &[TextRun],
        code: &[Range<usize>],
        font_size: Pixels,
        line_height: Pixels,
        baseline: Pixels,
        window: &Window,
    ) -> Self {
        Self::refresh(text, runs, code, (font_size, line_height, baseline), window, None)
    }

    fn refresh(
        text: &SharedString,
        runs: &[TextRun],
        code: &[Range<usize>],
        (font_size, line_height, baseline): (Pixels, Pixels, Pixels),
        window: &Window,
        previous: Option<&Self>,
    ) -> Self {
        let ends = run_ends(runs);
        debug_assert!(code.windows(2).all(|pair| pair[0].end <= pair[1].start));
        let atoms = atoms(text, runs, &ends, code, font_size, window, previous);
        Self {
            text: text.clone(),
            runs: runs.to_vec(),
            run_ends: ends,
            code: code.to_vec(),
            atoms,
            font_size,
            line_height,
            baseline,
        }
    }
}
fn intrinsic_width(input: &FlowInput) -> Pixels {
    let (mut advance, mut ink_end, mut maximum) = (px(0.), px(0.), px(0.));
    for atom in &input.atoms {
        for piece in atom.pieces.iter() {
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
        let fragmented = if word_width > width {
            Some(atom.pieces
                .iter()
                .flat_map(|piece| {
                    input.text[piece.range.clone()]
                        .grapheme_indices(true)
                        .flat_map(|(index, text)| {
                            pieces(
                                &input.text,
                                &input.runs,
                                &input.run_ends,
                                &input.code,
                                piece.range.start + index..piece.range.start + index + text.len(),
                                input.font_size,
                                window,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>())
        } else {
            None
        };
        let fragments = fragmented.as_deref().unwrap_or(&atom.pieces);
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
                piece: piece.clone(),
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
    type RequestLayoutState = Rc<RefCell<Rc<FlowLayout>>>;
    type PrepaintState = Option<Hitbox>;
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone().into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = style
            .line_height
            .to_pixels(style.font_size, window.rem_size());
        let scale = window.scale_factor();
        let base_run = style.to_run(1);
        let key = id.map(|id| (window.window_handle().window_id(), id.clone()));
        let previous = key.as_ref().and_then(|key| {
            let mut cache = cx.try_global::<FlowCacheStore>()?.0.borrow_mut();
            cache.clock += 1;
            let clock = cache.clock;
            let (seen, entry) = cache.entries.get_mut(key)?;
            *seen = clock;
            Some(entry.clone())
        });
        let cached = previous.as_ref()
            .filter(|entry| entry.matches(self, &base_run, font_size, line_height, scale)).cloned();
        let entry = cached.unwrap_or_else(|| {
            let base = window.text_system().shape_line("M".into(), font_size, std::slice::from_ref(&base_run), None);
            let previous = previous.as_ref().filter(|entry| entry.scale == scale);
            let input = FlowInput::refresh(
                &self.text, &self.runs, &self.code,
                (font_size, line_height, (line_height - base.ascent - base.descent) / 2. + base.ascent),
                window, previous.map(|entry| &entry.input),
            );
            let width = intrinsic_width(&input);
            let weight = input.text.len() + input.atoms.iter().map(|atom| 48
                + atom.pieces.len() * (std::mem::size_of::<ShapedLine>() + 192)).sum::<usize>();
            let entry = Rc::new(CachedFlow { input, base_run, scale, width, weight, layouts: Default::default() });
            // This cache is panel/window-local by ID and has a fixed admission
            // budget. One exceptional payload cannot displace the whole cache.
            const BUDGET: usize = 32 * 1024 * 1024;
            if let Some(key) = key
                && weight <= BUDGET / 8
            {
                if !cx.has_global::<FlowCacheStore>() { cx.set_global(FlowCacheStore::default()); }
                let mut cache = cx.global::<FlowCacheStore>().0.borrow_mut();
                if let Some((_, old)) = cache.entries.remove(&key) { cache.weight -= old.weight; }
                while cache.weight + weight > BUDGET || cache.entries.len() >= 1024 {
                    let Some(oldest) = cache.entries.iter()
                        .min_by_key(|(_, (seen, _))| *seen).map(|(key, _)| key.clone()) else { break; };
                    if let Some((_, old)) = cache.entries.remove(&oldest) { cache.weight -= old.weight; }
                }
                cache.clock += 1;
                let clock = cache.clock;
                cache.entries.insert(key, (clock, entry.clone()));
                cache.weight += weight;
            }
            entry
        });
        let measured = Rc::new(RefCell::new(Rc::new(FlowLayout::default())));
        let output = measured.clone();
        let intrinsic_width = entry.width;
        let layout =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let width = known.width.unwrap_or(match available.width {
                    AvailableSpace::Definite(width) => width,
                    _ => intrinsic_width,
                });
                // Flex probes with no inline space cannot paint any glyphs.
                // Defer wrapping until the parent supplies a positive width.
                if width <= px(0.) {
                    *output.borrow_mut() = Rc::new(FlowLayout::default());
                    return size(px(0.), entry.input.line_height);
                }
                let layout_width = width.max(px(1.));
                let mut cached = entry.layouts.borrow_mut();
                let layout = match cached.iter().position(|slot| slot.as_ref().is_some_and(|(previous, _)| *previous == layout_width)) {
                    Some(index) => {
                        cached.swap(0, index);
                        cached[0].as_ref().unwrap().1.clone()
                    }
                    None => {
                        let layout = Rc::new(arrange(&entry.input, layout_width, window));
                        let base_pieces = entry.input.atoms.iter().map(|atom| atom.pieces.len()).sum::<usize>();
                        // Very narrow columns can split unbreakable tokens
                        // into new shapes. Keep regular intrinsic and fitted
                        // layouts, but only one when they have extra pieces.
                        if layout.placed.len() <= base_pieces.saturating_mul(2) {
                            cached[1] = if layout.placed.len() <= base_pieces {
                                cached[0].take().filter(|(_, previous)| previous.placed.len() <= base_pieces)
                            } else { None };
                            cached[0] = Some((layout_width, layout.clone()));
                        }
                        layout
                    }
                };
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
        (!self.links.is_empty() || self.selection.is_some())
            .then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal))
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
        let clip = window.content_mask().bounds;
        // A single paragraph can exceed the entire viewport. Keep full
        // measurement and selection geometry; emit glyphs only near the clip.
        let above = clip.top() - bounds.top() - layout.line_height * 2.;
        let below = clip.bottom() - bounds.top() + layout.line_height;
        let from = layout.placed.partition_point(|placed| placed.position.y < above);
        let to = layout.placed.partition_point(|placed| placed.position.y <= below).max(from);
        let visible = &layout.placed[from..to];
        let mut backgrounds: Vec<(usize, usize, Bounds<Pixels>)> = Vec::new();
        let mut links = Vec::new();
        for placed in visible {
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
        if let Some(selection) = &self.selection {
            paint_selection(selection, &layout, bounds, window);
        }
        let now = Instant::now();
        let fade = if let Some(streaming) = self.stream_fade
            && !cx.reduce_motion()
            && let Some(id) = global_id
        {
            window.with_element_state(id, |state: Option<Option<Rc<RefCell<StreamFade>>>>, _| {
                let state = state.flatten().or_else(|| streaming.then(Default::default));
                let state = state.filter(|fade| {
                    let mut fade = fade.borrow_mut();
                    fade.update(&self.text, now);
                    streaming || !fade.starts.is_empty()
                });
                (state.clone(), state)
            })
        } else {
            None
        };
        let fade = fade.as_ref().map(|state| state.borrow());
        let mut needs_frame = false;
        for placed in visible {
            let line = &placed.piece.line;
            let natural_baseline =
                (layout.line_height - line.ascent - line.descent) / 2. + line.ascent;
            let origin = bounds.origin
                + placed.position
                + point(placed.piece.left, layout.baseline - natural_baseline);
            // Colour does not change shaping or any measured height. The
            // display clock redraws only while new, visible prose is fading.
            let opacity = fade.as_ref().map_or(1., |fade| fade.opacity(placed.piece.range.start, now));
            let faded;
            let line = if opacity < 1. && placed.piece.code.is_none() {
                needs_frame = true;
                let mut run = (*placed.piece.run).clone();
                run.color.a *= opacity;
                if let Some(underline) = &mut run.underline
                    && let Some(color) = &mut underline.color
                {
                    color.a *= opacity;
                }
                if let Some(strikethrough) = &mut run.strikethrough
                    && let Some(color) = &mut strikethrough.color
                {
                    color.a *= opacity;
                }
                faded = window.text_system().shape_line(
                    line.text.clone(), line.font_size, &[run], None,
                );
                &faded
            } else {
                line.as_ref()
            };
            let _ = line.paint(
                origin,
                layout.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
        }
        if needs_frame {
            window.request_animation_frame();
        }
        let selection_layout = self.selection.as_ref().map(|_| layout.clone());
        let selection_text = self.text.clone();
        drop(layout);
        if let Some(hitbox) = hitbox {
            if links
                .iter()
                .any(|(_, bounds)| bounds.contains(&window.mouse_position()))
            {
                window.set_cursor_style(CursorStyle::PointingHand, hitbox);
            } else if self.selection.is_some() && hitbox.is_hovered(window) {
                window.set_cursor_style(CursorStyle::IBeam, hitbox);
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
        if let (Some(selection), Some(hitbox), Some(layout)) = (self.selection.clone(), hitbox.clone(), selection_layout) {
            let text = selection_text;
            let down_hitbox = hitbox.clone();
            let down_text = text.clone();
            let down_layout = layout.clone();
            let down_selection = selection.clone();
            let down_drag = selection.drag.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase.bubble()
                    && event.button == MouseButton::Left
                    && down_hitbox.is_hovered(window)
                {
                    let index = index_at(&down_layout, &down_text, bounds, event.position);
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
            let move_text = text.clone();
            let move_layout = layout.clone();
            let move_selection = selection.clone();
            let move_drag = selection.drag.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase.bubble()
                    && move_drag.get()
                    && event.pressed_button == Some(MouseButton::Left)
                    && event.position.y >= bounds.top()
                    && event.position.y <= bounds.bottom()
                {
                    let index = index_at(&move_layout, &move_text, bounds, event.position);
                    (move_selection.on_change)(SelectionChange::Extend { index }, window, cx);
                    window.refresh();
                }
            });
            let up_selection = selection.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.bubble()
                    && event.button == MouseButton::Left
                    && selection.drag.replace(false)
                {
                    (up_selection.on_change)(SelectionChange::End, window, cx);
                    window.refresh();
                }
            });
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
            let runs=vec![run(7,crate::theme::agent_font()),run(text.len()-7,crate::theme::source_editor_font())];
            let ends = run_ends(&runs);
            let width_of=|range|pieces(&text,&runs,&ends,std::slice::from_ref(&code),range,px(14.),window).iter().fold(px(0.),|sum,piece|sum+piece.width());
            let width=width_of(0..7)+width_of(7..13)+px(1.);
            let input = FlowInput::new(&text, &runs, std::slice::from_ref(&code), px(14.), px(22.75), px(16.), window);
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
            let font = crate::theme::agent_font();
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
            let input = FlowInput::new(
                &text, &runs, std::slice::from_ref(&code_range), px(14.), px(22.75), px(16.), window,
            );
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

    #[gpui::test]
    fn pointer_hit_testing_maps_rows_and_columns_to_byte_boundaries(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, _| {
            let text: SharedString = "hello world".into();
            let run = |len| TextRun {
                len,
                font: crate::theme::agent_font(),
                color: black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs = vec![run(text.len())];
            let input = FlowInput::new(&text, &runs, &[], px(14.), px(22.), px(16.), window);
            let layout = arrange(&input, px(500.), window);
            let bounds = Bounds::new(point(px(10.), px(20.)), layout.size);
            assert_eq!(
                index_at(&layout, &text, bounds, point(px(10.), px(30.))),
                0,
                "the left edge selects the first byte"
            );
            let last = layout.placed.last().unwrap();
            let after = last.position.x + last.piece.left + last.piece.line.width() + px(20.);
            assert_eq!(
                index_at(&layout, &text, bounds, point(px(10.) + after, px(30.))),
                text.len(),
                "past the final glyph selects the end"
            );
            let first = layout.placed[0].clone();
            let middle = first.position.x + first.piece.left + first.piece.line.width() / 2.;
            let index = index_at(&layout, &text, bounds, point(px(10.) + middle, px(30.)));
            assert!(
                index > 0 && index < text.len(),
                "a mid-word pointer resolves inside the text: {index}"
            );
            assert!(
                index_at(&layout, &text, bounds, point(px(10.), px(100.))) == text.len(),
                "below the last row selects the end"
            );
        });
    }
    #[gpui::test]
    fn append_reuse_matches_cold_layout_at_unicode_and_style_boundaries(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, _| {
            let previous: SharedString = "Lead café ".into();
            let next: SharedString = "Lead café 👩‍💻 and 中文.".into();
            let run = |len, font| TextRun { len, font, color: black(), ..Default::default() };
            let lead = run(5, crate::theme::agent_font());
            let old_runs = [lead.clone(), run(previous.len() - 5, crate::theme::agent_font())];
            let new_runs = [lead, run(next.len() - 5, crate::theme::source_editor_font())];
            let geometry = (px(14.), px(22.75), px(16.));
            let old = FlowInput::refresh(&previous, &old_runs, &[], geometry, window, None);
            let warm = FlowInput::refresh(&next, &new_runs, &[], geometry, window, Some(&old));
            let cold = FlowInput::refresh(&next, &new_runs, &[], geometry, window, None);
            let warm = arrange(&warm, px(60.), window);
            let cold = arrange(&cold, px(60.), window);
            let placements = |layout: &FlowLayout| layout.placed.iter().map(|placed| {
                (placed.piece.range.clone(), placed.piece.width(), placed.position)
            }).collect::<Vec<_>>();
            assert_eq!(warm.size, cold.size);
            assert_eq!(placements(&warm), placements(&cold));
        });
    }
}
