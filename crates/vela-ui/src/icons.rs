//! Original Vela vector geometry, painted by GPUI rather than font substitutes.
use gpui::{prelude::*, *};
use std::borrow::Cow;

pub struct IconAssets;
const ICONS: &[(&str, &[u8])] = &[
    ("check", include_bytes!("../assets/icons/check.svg")),
    ("copy", include_bytes!("../assets/icons/copy.svg")),
    (
        "chevron-right",
        include_bytes!("../assets/icons/chevron-right.svg"),
    ),
    (
        "chevron-down",
        include_bytes!("../assets/icons/chevron-down.svg"),
    ),
    ("reasoning", include_bytes!("../assets/icons/reasoning.svg")),
    (
        "working-spinner",
        include_bytes!("../assets/icons/working-spinner.svg"),
    ),
    ("edit", include_bytes!("../assets/icons/edit.svg")),
    (
        "arrow-left",
        include_bytes!("../assets/icons/arrow-left.svg"),
    ),
    (
        "arrow-right",
        include_bytes!("../assets/icons/arrow-right.svg"),
    ),
    ("close", include_bytes!("../assets/icons/close.svg")),
    ("refresh", include_bytes!("../assets/icons/refresh.svg")),
    ("image", include_bytes!("../assets/icons/image.svg")),
    ("arrow-up", include_bytes!("../assets/icons/arrow-up.svg")),
    (
        "arrow-down",
        include_bytes!("../assets/icons/arrow-down.svg"),
    ),
    ("stop", include_bytes!("../assets/icons/stop.svg")),
    ("globe", include_bytes!("../assets/icons/globe.svg")),
    (
        "panel-left",
        include_bytes!("../assets/icons/panel-left.svg"),
    ),
    (
        "panel-left-dashed",
        include_bytes!("../assets/icons/panel-left-dashed.svg"),
    ),
    (
        "panel-right",
        include_bytes!("../assets/icons/panel-right.svg"),
    ),
    (
        "panel-right-dashed",
        include_bytes!("../assets/icons/panel-right-dashed.svg"),
    ),
    (
        "square-plus",
        include_bytes!("../assets/icons/square-plus.svg"),
    ),
    (
        "agent-mode",
        include_bytes!("../assets/icons/agent-mode.svg"),
    ),
    ("eye", include_bytes!("../assets/icons/eye.svg")),
    (
        "cloud-download",
        include_bytes!("../assets/icons/cloud-download.svg"),
    ),
    ("mic", include_bytes!("../assets/icons/mic.svg")),
    ("robot", include_bytes!("../assets/icons/robot.svg")),
    ("server", include_bytes!("../assets/icons/server.svg")),
    ("dashboard", include_bytes!("../assets/icons/dashboard.svg")),
];
impl AssetSource for IconAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = crate::tree_icons::asset(path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        Ok(ICONS
            .iter()
            .find(|(name, _)| path == format!("icons/{name}.svg"))
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(name, _)| SharedString::from(format!("icons/{name}.svg")))
            .chain(crate::tree_icons::paths())
            .filter(|name| name.starts_with(path))
            .collect())
    }
}
pub fn glyph(name: &'static str, diameter: f32, color: Rgba) -> Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .size(px(diameter))
        .flex_shrink_0()
        .text_color(color)
}

/// GPUI's Svg only paints with an explicit local color. Paint through the same
/// asset cache using the active text style so parent hover/focus colors apply.
pub fn inherited_glyph(name: &'static str, diameter: f32) -> AnyElement {
    let path: SharedString = format!("icons/{name}.svg").into();
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            if let Err(error) = window.paint_svg(
                bounds,
                path.clone(),
                None,
                TransformationMatrix::unit(),
                window.text_style().color,
                cx,
            ) {
                tracing::warn!(%error, "could not paint native icon");
            }
        },
    )
    .size(px(diameter))
    .flex_shrink_0()
    .into_any_element()
}

/// The two cubic paths and theme color mix in components/VelaMark.tsx.
pub fn vela_mark(diameter: f32, accent: Rgba, ink: Rgba) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let at = |x, y| bounds.origin + point(px(x * diameter / 96.), px(y * diameter / 96.));
            let mut large = PathBuilder::fill();
            large.move_to(at(48., 10.));
            for [ax, ay, bx, by, x, y] in [
                [53., 34., 58., 39., 86., 48.],
                [58., 57., 53., 62., 48., 86.],
                [43., 62., 38., 57., 10., 48.],
                [38., 39., 43., 34., 48., 10.],
            ] {
                large.cubic_bezier_to(at(x, y), at(ax, ay), at(bx, by));
            }
            large.close();
            if let Ok(path) = large.build() {
                window.paint_path(
                    path,
                    Rgba {
                        r: accent.r * 0.72 + ink.r * 0.28,
                        g: accent.g * 0.72 + ink.g * 0.28,
                        b: accent.b * 0.72 + ink.b * 0.28,
                        a: 1.,
                    },
                );
            }
            let mut small = PathBuilder::fill();
            small.move_to(at(70., 12.));
            for [ax, ay, bx, by, x, y] in [
                [72., 22., 74., 24., 84., 27.],
                [74., 30., 72., 32., 70., 42.],
                [68., 32., 66., 30., 56., 27.],
                [66., 24., 68., 22., 70., 12.],
            ] {
                small.cubic_bezier_to(at(x, y), at(ax, ay), at(bx, by));
            }
            small.close();
            if let Ok(path) = small.build() {
                window.paint_path(path, accent.opacity(0.75));
            }
        },
    )
    .size(px(diameter))
    .flex_shrink_0()
    .into_any_element()
}
