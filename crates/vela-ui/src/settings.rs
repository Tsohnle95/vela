//! Shared native Settings layout and source-derived decoration.
use crate::surface::{RoundedSurface, codex_card};
use crate::theme::{self, PANEL_RADIUS, Theme};
use gpui::{prelude::*, *};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ThemeGrid(
    std::rc::Rc<std::cell::Cell<u16>>,
    std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, FocusHandle>>>,
);
impl Default for ThemeGrid {
    fn default() -> Self {
        Self(
            std::rc::Rc::new(std::cell::Cell::new(1)),
            Default::default(),
        )
    }
}
impl ThemeGrid {
    pub(crate) fn focus(&self, id: &str, cx: &mut App) -> FocusHandle {
        self.1
            .borrow_mut()
            .entry(id.to_owned())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }
    pub(crate) fn render(&self, cards: Vec<AnyElement>, owner: EntityId) -> AnyElement {
        let columns = self.0.clone();
        div()
            .w_full()
            .min_w_0()
            .child(
                div()
                    .debug_selector(|| "appearance-theme-grid".to_owned())
                    .w_full()
                    .min_w_0()
                    .grid()
                    .grid_cols(columns.get())
                    .gap(px(12.))
                    .children(cards),
            )
            .on_children_prepainted(move |bounds, _, cx| {
                let Some(bounds) = bounds.first() else { return };
                let count = ((f32::from(bounds.size.width) + 12.) / 162.)
                    .floor()
                    .max(1.) as u16;
                if columns.replace(count) != count {
                    cx.notify(owner);
                }
            })
            .id("appearance-themes")
            .role(Role::RadioGroup)
            .aria_label("Color theme")
            .into_any_element()
    }
}

pub(crate) fn appearance_page(
    header: AnyElement,
    body: AnyElement,
    padding: f32,
    scroll: crate::scroll::AutoScrollHandle,
) -> AnyElement {
    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .h_full()
        .bg(Theme::default().bg)
        .child(crate::scroll::AutoScroll::new(
            scroll,
            move |handle, gutter| {
                div()
                    .id("appearance-page-scroll")
                    .w_full()
                    .h_full()
                    .min_w_0()
                    .overflow_y_scroll()
                    .track_scroll(&handle)
                    .scrollbar_width(gutter)
                    .px(px(padding))
                    .py(px(32.))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(body)
                    .into_any_element()
            },
        ))
        .into_any_element()
}

pub(crate) fn shell(
    rail: AnyElement,
    nav: AnyElement,
    content: AnyElement,
    codex: bool,
) -> AnyElement {
    let row = div()
        .flex_1()
        .min_w_0()
        .w_full()
        .min_h_0()
        .h_full()
        .flex()
        .line_height(relative(1.2))
        .child(rail);
    if codex {
        row.bg(Theme::default().top)
            .child(codex_card(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .child(nav)
                    .child(content),
            ))
            .into_any_element()
    } else {
        row.gap(px(10.))
            .child(nav)
            .child(content)
            .into_any_element()
    }
}

pub(crate) fn page(
    header: AnyElement,
    body: AnyElement,
    padding: f32,
    scroll: crate::scroll::AutoScrollHandle,
    codex: bool,
) -> AnyElement {
    let theme = Theme::default();
    let content = div()
        .relative()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .h_full()
        .flex()
        .flex_col()
        .child(settings_background(theme))
        .child(crate::scroll::AutoScroll::new(
            scroll,
            move |handle, gutter| {
                div()
                    .id("settings-page-scroll")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .w_full()
                    .overflow_y_scroll()
                    .track_scroll(&handle)
                    .scrollbar_width(gutter)
                    .p(px(padding))
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(920.))
                            .min_w_0()
                            .flex_shrink_0()
                            .child(header),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_w(px(920.))
                            .min_w_0()
                            .flex_shrink_0()
                            .py(px(35.))
                            .child(body),
                    )
                    .into_any_element()
            },
        ));
    // Codex already clips the navigation and content inside one outer card.
    // An additional rounded, bordered page creates a notch and double seam at
    // their join. Keep that join flat; Classic owns separate floating cards.
    if codex {
        return content.bg(theme.inset).overflow_hidden().into_any_element();
    }
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .rounded(px(PANEL_RADIUS))
        .border_1()
        .border_color(theme.border)
        .bg(theme.inset)
        .overflow_hidden()
        .flex()
        .flex_col()
        .shadow(vec![
            gpui::BoxShadow::new(px(0.), px(10.4), theme::rgba(0, 0, 0, 0.2).into())
                .blur_radius(px(32.5)),
        ])
        .child(RoundedSurface::new(content, px(PANEL_RADIUS - 1.)))
        .into_any_element()
}

pub(crate) fn tracked_label(
    text: &str,
    font_size: f32,
    tracking: f32,
    color: gpui::Rgba,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("tracked-label-{text}")))
        .role(gpui::Role::Label)
        .aria_label(SharedString::from(text.to_owned()))
        .text_size(px(font_size))
        .font_weight(FontWeight::BOLD)
        .text_color(color)
        .child(crate::typography::TrackedText::new(
            SharedString::from(text.to_owned()),
            px(font_size * tracking),
        ))
        .into_any_element()
}

pub(crate) fn settings_list(content: impl IntoElement) -> AnyElement {
    let theme = Theme::default();
    div()
        .w_full()
        .min_w_0()
        .rounded(px(12.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .overflow_hidden()
        .child(content)
        .into_any_element()
}

pub(crate) fn settings_group_title(text: &'static str) -> AnyElement {
    let theme = Theme::default();
    div()
        .id(SharedString::from(format!("settings-group-{text}")))
        .role(Role::Heading)
        .aria_label(text)
        .mt(px(20.))
        .mb(px(8.))
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.faint)
        .child(crate::typography::TrackedText::new(text, px(0.48)))
        .into_any_element()
}

pub(crate) fn badge(text: &str) -> AnyElement {
    let theme = Theme::default();
    let text = text.to_uppercase();
    div()
        .id(SharedString::from(format!("settings-badge-{text}")))
        .role(Role::Label)
        .aria_label(text.clone())
        .flex_shrink_0()
        .px(px(8.))
        .py(px(4.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.inset)
        .rounded(px(999.))
        .text_size(px(9.))
        .font_weight(FontWeight::BOLD)
        .text_color(theme.dim)
        .child(crate::typography::TrackedText::new(text, px(0.54)))
        .into_any_element()
}

pub(crate) fn setting_row(
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    let theme = Theme::default();
    div()
        .w_full()
        .min_w_0()
        .min_h(px(72.))
        .px(px(18.))
        .py(px(15.))
        .flex()
        .items_center()
        .gap(px(24.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .whitespace_normal()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::BOLD)
                        .child(title.into()),
                )
                .child(
                    div()
                        .mt(px(3.))
                        .whitespace_normal()
                        .text_size(px(10.5))
                        .text_color(theme.faint)
                        .child(detail.into()),
                ),
        )
        .child(control)
}

pub(crate) fn callout(title: impl Into<SharedString>, detail: impl Into<SharedString>) -> Div {
    let theme = Theme::default();
    div()
        .w_full()
        .min_w_0()
        .p(px(26.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .rounded(px(12.))
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::BOLD)
                .child(title.into()),
        )
        .child(
            div()
                .mt(px(8.))
                .max_w(px(650.))
                .text_size(px(12.))
                .line_height(relative(1.65))
                .text_color(theme.dim)
                .whitespace_normal()
                .child(detail.into()),
        )
}

/// CSS's 720×440 elliptical radial gradient at 88% -8%, ending at 68%.
/// GPUI decodes this small SVG through its native image pipeline; the shell
/// remains native. Intrinsic pixels match the window's backing scale.
fn settings_background(theme: Theme) -> AnyElement {
    gpui::canvas(move |bounds, window, cx| {
    let w = f32::from(bounds.size.width).max(1.);
    let h = f32::from(bounds.size.height).max(1.);
    let scale = window.scale_factor();
    let color = |c: gpui::Rgba| {
        format!(
            "#{:02x}{:02x}{:02x}",
            (c.r * 255.).round() as u8,
            (c.g * 255.).round() as u8,
            (c.b * 255.).round() as u8
        )
    };
    let accent = color(theme.accent);
    let source = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {w} {h}"><defs><radialGradient id="g" gradientUnits="userSpaceOnUse" cx="0" cy="0" r="1" gradientTransform="translate({} {}) scale(720 440)" color-interpolation="sRGB"><stop offset="0" stop-color="{accent}" stop-opacity="0.1"/><stop offset="0.68" stop-color="{accent}" stop-opacity="0"/></radialGradient></defs><rect width="{w}" height="{h}" fill="url(#g)"/></svg>"##,
        (w * scale).ceil(),
        (h * scale).ceil(),
        w * 0.88,
        h * -0.08
    );
    let image = Arc::new(gpui::Image::from_bytes(
        gpui::ImageFormat::Svg,
        source.into_bytes(),
    ));
    image.use_render_image(window, cx)
    }, |bounds, image, window, _| {
        if let Some(image) = image {
            let _ = window.paint_image(bounds, gpui::Corners::default(), image, 0, false);
        }
    }).absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{page, setting_row, settings_list, shell};
    use gpui::{
        Bounds, Context, IntoElement, Pixels, Render, TestAppContext, Window, canvas, div, point,
        prelude::*, px, size,
    };
    use std::{cell::RefCell, rc::Rc};

    struct ThemeGridFixture {
        grid: super::ThemeGrid,
    }
    struct ThemeFocusFixture {
        grid: super::ThemeGrid,
        scroll: crate::scroll::AutoScrollHandle,
    }
    impl Render for ThemeFocusFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let cards = (0..15)
                .map(|i| {
                    let handle = self.grid.focus(&format!("focus-theme-{i}"), cx);
                    crate::scroll::FocusReveal::element(
                        format!("theme-reveal-{i}"),
                        self.scroll.clone(),
                        handle.clone(),
                        div()
                            .id(format!("theme-focus-{i}"))
                            .debug_selector(move || format!("theme-focus-{i}"))
                            .track_focus(&handle)
                            .role(gpui::Role::RadioButton)
                            .w_full()
                            .h(px(110.)),
                    )
                    .into_any_element()
                })
                .collect();
            let grid = self.grid.render(cards, cx.entity_id());
            div()
                .size_full()
                .on_key_down(crate::input::navigate_focus)
                .child(super::appearance_page(
                    div().h(px(0.)).into_any_element(),
                    div()
                        .w_full()
                        .flex_shrink_0()
                        .child(grid)
                        .into_any_element(),
                    0.,
                    self.scroll.clone(),
                ))
        }
    }
    #[gpui::test]
    fn theme_card_keyboard_order_reveals_the_last_card(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let grid = super::ThemeGrid::default();
            let first = grid.focus("focus-theme-0", cx);
            window.focus(&first, cx);
            ThemeFocusFixture {
                grid,
                scroll: Default::default(),
            }
        });
        cx.simulate_resize(size(px(300.), px(300.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(300.), px(300.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx);
        for _ in 0..14 {
            cx.simulate_keystrokes("tab");
            draw(cx);
        }
        cx.update(|window, cx| {
            let grid = view.read(cx).grid.clone();
            assert!(
                grid.focus("focus-theme-14", cx).is_focused(window),
                "every retained theme handle must be a tab stop"
            );
        });
        let last = cx.debug_bounds("theme-focus-14").unwrap();
        assert!(
            last.top() >= px(0.) && last.bottom() <= px(300.),
            "{last:?}"
        );
    }
    impl Render for ThemeGridFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.grid.render(
                (0..15)
                    .map(|i| {
                        div()
                            .debug_selector(move || format!("theme-card-{i}"))
                            .w_full()
                            .min_w_0()
                            .h(px(110.))
                            .into_any_element()
                    })
                    .collect(),
                cx.entity_id(),
            )
        }
    }
    #[gpui::test]
    fn theme_grid_fits_source_minimum_tracks_and_reflows_after_resize(cx: &mut TestAppContext) {
        let selectors = [
            "theme-card-0",
            "theme-card-1",
            "theme-card-2",
            "theme-card-3",
            "theme-card-4",
        ];
        let (view, cx) = cx.add_window_view(|_, _| ThemeGridFixture {
            grid: super::ThemeGrid::default(),
        });
        for (width, columns) in [
            (540., 3),
            (760., 4),
            (300., 1),
            (312., 2),
            (149., 1),
            (540., 3),
        ] {
            cx.simulate_resize(size(px(width), px(1000.)));
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(width), px(1000.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
            let first = cx.debug_bounds("theme-card-0").unwrap();
            let next_row = cx.debug_bounds(selectors[columns]).unwrap();
            assert!(
                next_row.top() >= first.bottom(),
                "width {width}: {first:?}, {next_row:?}"
            );
            if columns > 1 {
                let last = cx.debug_bounds(selectors[columns - 1]).unwrap();
                assert_eq!(last.top(), first.top());
                assert!((f32::from(last.right()) - width).abs() <= 1., "{last:?}");
            }
        }
    }

    struct SettingsFixture {
        width: f32,
        measured: Rc<RefCell<Option<Bounds<Pixels>>>>,
        scroll: crate::scroll::AutoScrollHandle,
    }
    impl Render for SettingsFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let padding = (self.width * 0.06).clamp(34., 76.);
            let measured = self.measured.clone();
            let body = div().relative().w_full().min_w_0()
                .child(settings_list(setting_row("Development build",
                    "GitHub app updates are disabled here. Develop and test this local checkout; source edits do not update the installed app through the release updater.",
                    div().w(px(80.)).flex_shrink_0().child("Updater disabled"))))
                .child(canvas(move |bounds, _, _| *measured.borrow_mut() = Some(bounds), |_, _, _, _| {})
                    .absolute().top_0().left_0().size_full());
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(div().h(px(44.)).flex_shrink_0())
                .child(
                    div().flex_1().min_h_0().w_full().flex().child(shell(
                        div().w(px(50.)).h_full().flex_shrink_0().into_any_element(),
                        div()
                            .w(px(229.))
                            .h_full()
                            .flex_shrink_0()
                            .into_any_element(),
                        page(
                            div().h(px(100.)).into_any_element(),
                            body.into_any_element(),
                            padding,
                            self.scroll.clone(),
                            true,
                        ),
                        true,
                    )),
                )
                .into_any_element()
        }
    }
    #[gpui::test]
    fn settings_content_stays_inside_viewport_with_long_update_copy(cx: &mut TestAppContext) {
        let observed = Rc::new(RefCell::new(None));
        let measured = observed.clone();
        let (view, cx) = cx.add_window_view(|_, _| SettingsFixture {
            width: 900.,
            measured,
            scroll: crate::scroll::AutoScrollHandle::default(),
        });
        for width in [900., 1280.] {
            let padding = (width * 0.06_f32).clamp(34., 76.);
            view.update(cx, |view, cx| {
                view.width = width;
                cx.notify();
            });
            cx.simulate_resize(size(px(width), px(800.)));
            cx.draw(point(px(0.), px(0.)), size(px(width), px(800.)), |_, _| {
                view.clone().into_any_element()
            });
            let bounds = observed
                .borrow()
                .expect("Settings content should be painted");
            assert!(
                (f32::from(bounds.left()) - (280. + padding)).abs() <= 1.,
                "{bounds:?}"
            );
            assert!(
                f32::from(bounds.right()) <= width - 1. - padding + 1.,
                "content must fit inside the right padding: width={width}, bounds={bounds:?}"
            );
        }
    }
}
