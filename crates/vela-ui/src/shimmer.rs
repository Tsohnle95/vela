//! Running-text shimmer used by tool titles and the compaction notice.
//!
//! Source paints a narrow bright band across otherwise muted text on a 1200ms
//! loop. GPUI has no gradient text fill, so each character carries its own
//! color animation with a staggered phase; the band therefore sweeps across
//! the label without changing its measured text or accessibility label.

use super::*;
use gpui::{AnimationExt, Rgba};

const SHIMMER_MS: u64 = 1200;
const SPREAD_CHARS: f32 = 5.2;
const MAX_PER_CHAR: usize = 80;

pub(super) fn text(
    value: impl Into<SharedString>,
    active: bool,
    base: Rgba,
    peak: Rgba,
    selector: &'static str,
    id: SharedString,
) -> AnyElement {
    let value = value.into();
    if !active {
        return div()
            .debug_selector(move || selector.into())
            .whitespace_nowrap()
            .child(value)
            .into_any_element();
    }
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() > MAX_PER_CHAR {
        return div()
            .debug_selector(move || selector.into())
            .whitespace_nowrap()
            .with_animation(
                SharedString::from(format!("{id}:pulse")),
                gpui::Animation::new(Duration::from_millis(SHIMMER_MS)).repeat(),
                move |element, progress| element.text_color(color_at(progress, 0., 1., base, peak)),
            )
            .child(value)
            .into_any_element();
    }
    let len = chars.len().max(1) as f32;
    let spread = (SPREAD_CHARS / len).clamp(0.08, 0.5);
    div()
        .debug_selector(move || selector.into())
        .flex()
        .items_baseline()
        .whitespace_nowrap()
        .children(chars.into_iter().enumerate().map(|(index, ch)| {
            let phase = index as f32 / len;
            let ch = if ch == ' ' { '\u{00a0}' } else { ch };
            div().flex_shrink_0().child(ch.to_string()).with_animation(
                SharedString::from(format!("{id}:{index}")),
                gpui::Animation::new(Duration::from_millis(SHIMMER_MS)).repeat(),
                move |element, progress| {
                    element.text_color(color_at(progress, phase, spread, base, peak))
                },
            )
        }))
        .into_any_element()
}

/// Color of one character at `progress` through the 1200ms cycle. The bright
/// band trails the character's normalized position by `spread`, matching the
/// left-to-right sweep of Source's background animation.
pub(super) fn color_at(progress: f32, phase: f32, spread: f32, base: Rgba, peak: Rgba) -> Rgba {
    let wave = (progress - phase).rem_euclid(1.);
    let intensity = if wave < spread {
        (1. - wave / spread).powf(1.4)
    } else {
        0.
    };
    mix(base, peak, intensity)
}

fn mix(base: Rgba, peak: Rgba, amount: f32) -> Rgba {
    let amount = amount.clamp(0., 1.);
    Rgba {
        r: base.r + (peak.r - base.r) * amount,
        g: base.g + (peak.g - base.g) * amount,
        b: base.b + (peak.b - base.b) * amount,
        a: base.a + (peak.a - base.a) * amount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(r: f32) -> Rgba {
        Rgba {
            r,
            g: r,
            b: r,
            a: 1.,
        }
    }

    #[test]
    fn sweep_reaches_peak_then_returns_to_base() {
        let base = color(0.);
        let peak = color(1.);
        assert_eq!(color_at(0., 0., 0.2, base, peak).r, 1.);
        assert_eq!(color_at(0., 0.5, 0.2, base, peak).r, 0.);
        let between = color_at(0.1, 0., 0.2, base, peak).r;
        assert!(between > 0. && between < 1.);
    }

    #[test]
    fn spread_is_bounded_for_long_and_short_labels() {
        assert!((SPREAD_CHARS / 2.).clamp(0.08, 0.5) == 0.5);
        assert!((SPREAD_CHARS / 80.).clamp(0.08, 0.5) == 0.08);
    }
}
