//! Provider usage cards inside the session-usage popup.
//!
//! Snapshots are read by the engine from the short-lived bridge document; this
//! view owns formatting and the loading/empty/error presentation.

use crate::theme::{self, Theme};
use gpui::accesskit::Role;
use gpui::prelude::*;
use gpui::{AnyElement, FontWeight, IntoElement, Rgba, SharedString, div, px, relative};
use vela_engine::provider_usage::{ProviderUsageCredits, ProviderUsageResult, UsageWindow};

pub(super) fn format_provider_count(value: f64) -> String {
    let value = value.max(0.0);
    if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}k", value / 1_000.0)
    } else if (value - value.round()).abs() < f64::EPSILON {
        format!("{}", value.round() as u64)
    } else {
        format!("{value:.1}")
    }
}

pub(super) fn format_provider_credits(credits: &ProviderUsageCredits) -> String {
    if credits.unlimited {
        return "Unlimited".into();
    }
    if let (Some(used), Some(total)) = (credits.used, credits.total) {
        return format!(
            "{} / {}",
            format_provider_count(used),
            format_provider_count(total)
        );
    }
    if let Some(remaining) = credits.remaining {
        return format_provider_count(remaining);
    }
    credits.balance.clone().unwrap_or_default()
}

pub(super) fn format_provider_plan(plan: &str) -> String {
    let mut formatted = plan
        .replace(['_', '-'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    if formatted == "Self Serve Business" {
        formatted = "Self-serve Business".into();
    }
    formatted
}

pub(super) fn format_provider_resets(resets_at: i64, now_ms: i64) -> String {
    let remaining_ms = (resets_at as i128 * 1000 - now_ms as i128).max(0);
    if remaining_ms == 0 {
        return "resets any moment".into();
    }
    let minutes = ((remaining_ms + 59_999) / 60_000) as i64;
    if minutes < 60 {
        return format!("resets in {minutes}m");
    }
    let hours = minutes / 60;
    if hours >= 48 {
        let days = hours / 24;
        let rem_hours = hours % 24;
        return if rem_hours > 0 {
            format!("resets in {days}d {rem_hours}h")
        } else {
            format!("resets in {days}d")
        };
    }
    let rem_minutes = minutes % 60;
    if rem_minutes > 0 {
        format!("resets in {hours}h {rem_minutes}m")
    } else {
        format!("resets in {hours}h")
    }
}

fn window_tone(percent: f64) -> Rgba {
    let theme = Theme::default();
    if percent >= 90. {
        theme.red
    } else if percent >= 60. {
        theme::rgba(0xf0, 0xa6, 0x4b, 1.)
    } else {
        theme.accent
    }
}

fn window_row(provider: &str, index: usize, window: &UsageWindow, now_ms: i64) -> AnyElement {
    let theme = Theme::default();
    let used = window.used_percent.clamp(0., 100.);
    let left = (100. - used).round();
    let id = format!("provider-usage-{provider}-window-{index}");
    let selector = id.clone();
    div()
        .id(SharedString::from(id))
        .debug_selector(move || selector.clone())
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(10.5))
                .text_color(theme.dim)
                .child(SharedString::from(window.label.clone()))
                .child(format!("{left}% left")),
        )
        .child(
            div()
                .h(px(3.))
                .w_full()
                .rounded(px(999.))
                .bg(theme.inset)
                .child(
                    div()
                        .h_full()
                        .rounded(px(999.))
                        .bg(window_tone(used))
                        .w(relative((used / 100.) as f32)),
                ),
        )
        .when_some(window.resets_at, |row, resets| {
            row.child(
                div()
                    .text_size(px(9.))
                    .text_color(theme.faint)
                    .child(format_provider_resets(resets, now_ms)),
            )
        })
        .into_any_element()
}

fn provider_card(result: &ProviderUsageResult, now_ms: i64) -> AnyElement {
    let theme = Theme::default();
    let id = format!("provider-usage-{}", result.provider);
    let selector = id.clone();
    let name = if result.display_name.is_empty() {
        result.provider.clone()
    } else {
        result.display_name.clone()
    };
    let tone = match result.status.as_str() {
        "ok" => theme.green,
        "stale" => theme::rgba(0xf0, 0xa6, 0x4b, 1.),
        _ => theme.red,
    };
    let mut card = div()
        .id(SharedString::from(id))
        .debug_selector(move || selector.clone())
        .flex()
        .flex_col()
        .gap(px(5.))
        .p(px(7.))
        .rounded(px(7.))
        .bg(theme.inset)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(SharedString::from(name)),
                )
                .when_some(
                    result
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.plan_type.as_deref()),
                    |head, plan| {
                        head.child(
                            div()
                                .text_size(px(9.))
                                .text_color(theme.faint)
                                .child(format_provider_plan(plan)),
                        )
                    },
                )
                .child(div().size(px(6.)).flex_shrink_0().rounded_full().bg(tone)),
        );
    if let Some(snapshot) = &result.snapshot {
        for (index, window) in snapshot.windows.iter().enumerate() {
            card = card.child(window_row(&result.provider, index, window, now_ms));
        }
        if let Some(credits) = &snapshot.credits
            && (credits.has_credits
                || credits.unlimited
                || credits.used.is_some()
                || credits.remaining.is_some())
        {
            let id = format!("provider-usage-{}-credits", result.provider);
            let selector = id.clone();
            card = card.child(
                div()
                    .id(SharedString::from(id))
                    .debug_selector(move || selector.clone())
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.))
                    .text_color(theme.dim)
                    .child(SharedString::from(
                        credits.label.clone().unwrap_or_else(|| "Credits".into()),
                    ))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(format_provider_credits(credits)),
                    ),
            );
        }
    }
    if result.snapshot.is_none() {
        let message = result
            .error
            .as_ref()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "No usage data reported for this provider.".into());
        card = card.child(
            div()
                .text_size(px(10.))
                .text_color(theme.faint)
                .child(SharedString::from(message)),
        );
    }
    card.into_any_element()
}

/// Provider section with an optional host-owned refresh control. The control
/// sits in the section header exactly where Source places its refresh button.
pub(super) fn render_provider_usage_with_refresh(
    results: &[ProviderUsageResult],
    loading: bool,
    refresh: Option<AnyElement>,
) -> AnyElement {
    let theme = Theme::default();
    let mut section = div()
        .id("provider-usage-section")
        .role(Role::Group)
        .aria_label("Provider usage")
        .pt(px(8.))
        .mt(px(2.))
        .border_t_1()
        .border_color(theme.border)
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(9.5))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.dim)
                        .child("PROVIDER USAGE"),
                )
                .when_some(refresh, |head, refresh| head.child(refresh))
                .when(loading, |head| {
                    head.child(
                        div()
                            .text_size(px(9.))
                            .text_color(theme.faint)
                            .child("Refreshing…"),
                    )
                }),
        );
    if loading && results.is_empty() {
        return section
            .child(
                div()
                    .id("provider-usage-loading")
                    .debug_selector(|| "provider-usage-loading".into())
                    .text_size(px(10.5))
                    .text_color(theme.faint)
                    .child("Loading provider usage…"),
            )
            .into_any_element();
    }
    if results.is_empty() {
        return section
            .child(
                div()
                    .id("provider-usage-empty")
                    .debug_selector(|| "provider-usage-empty".into())
                    .text_size(px(10.5))
                    .text_color(theme.faint)
                    .child(
                        "No providers connected. Sign in with opencode auth login to see plan limits here.",
                    ),
            )
            .into_any_element();
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64;
    for result in results {
        section = section.child(provider_card(result, now_ms));
    }
    section.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Render, Window, point, size};
    use vela_engine::provider_usage::ProviderUsageSnapshot;

    #[test]
    fn provider_usage_formatters_match_reference_wording() {
        assert_eq!(format_provider_count(999.), "999");
        assert_eq!(format_provider_count(1_500.), "1.5k");
        assert_eq!(format_provider_count(2_000_000.), "2.0M");
        assert_eq!(
            format_provider_plan("self_serve_business"),
            "Self-serve Business"
        );
        assert_eq!(format_provider_plan("team-plan"), "Team Plan");
        assert_eq!(
            format_provider_resets(1_000, 1_000_000),
            "resets any moment"
        );
        assert_eq!(
            format_provider_resets(1_000 + 5 * 60, 1_000_000),
            "resets in 5m"
        );
        assert_eq!(
            format_provider_resets(1_000 + 125 * 60, 1_000_000),
            "resets in 2h 5m"
        );
        assert_eq!(
            format_provider_resets(1_000 + 49 * 3600, 1_000_000),
            "resets in 2d 1h"
        );
        assert_eq!(
            format_provider_credits(&ProviderUsageCredits {
                unlimited: true,
                ..Default::default()
            }),
            "Unlimited"
        );
        assert_eq!(
            format_provider_credits(&ProviderUsageCredits {
                used: Some(250.),
                total: Some(1_000.),
                ..Default::default()
            }),
            "250 / 1.0k"
        );
    }

    fn sample() -> Vec<ProviderUsageResult> {
        vec![
            ProviderUsageResult {
                provider: "openai".into(),
                display_name: "OpenAI".into(),
                status: "ok".into(),
                snapshot: Some(ProviderUsageSnapshot {
                    windows: vec![UsageWindow {
                        id: "5h".into(),
                        label: "5h".into(),
                        used_percent: 42.,
                        window_minutes: Some(300),
                        resets_at: None,
                    }],
                    credits: Some(ProviderUsageCredits {
                        has_credits: true,
                        used: Some(100.),
                        remaining: Some(900.),
                        ..Default::default()
                    }),
                    plan_type: Some("team".into()),
                    updated_at: 1,
                }),
                error: None,
            },
            ProviderUsageResult {
                provider: "copilot".into(),
                display_name: "GitHub Copilot".into(),
                status: "unauthenticated".into(),
                snapshot: None,
                error: Some(vela_engine::provider_usage::ProviderUsageError {
                    code: "missing_oauth".into(),
                    message: "GitHub Copilot is not authenticated.".into(),
                    retryable: false,
                }),
            },
        ]
    }

    struct UsageProbe {
        results: Vec<ProviderUsageResult>,
        loading: bool,
    }

    impl Render for UsageProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(render_provider_usage_with_refresh(&self.results, self.loading, None))
        }
    }

    struct RefreshProbe {
        activations: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl Render for RefreshProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let activations = self.activations.clone();
            let refresh = div()
                .id("test-provider-refresh")
                .debug_selector(|| "test-provider-refresh".into())
                .role(Role::Button)
                .aria_label("Refresh provider usage")
                .size(px(18.))
                .cursor_pointer()
                .on_click(move |_, _, _| activations.set(activations.get() + 1))
                .child("↻");
            div().size_full().child(render_provider_usage_with_refresh(
                &sample(),
                false,
                Some(refresh.into_any_element()),
            ))
        }
    }

    #[gpui::test]
    fn provider_usage_header_places_host_refresh_control(cx: &mut gpui::TestAppContext) {
        let activations = std::rc::Rc::new(std::cell::Cell::new(0));
        let (probe, cx) = cx.add_window_view(|_, _| RefreshProbe {
            activations: activations.clone(),
        });
        cx.simulate_resize(size(px(300.), px(500.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(500.)), |_, _| {
                probe.clone().into_any_element()
            })
        };
        draw(cx);
        let refresh = cx
            .debug_bounds("test-provider-refresh")
            .expect("provider section renders the host refresh control");
        cx.simulate_click(refresh.center(), gpui::Modifiers::default());
        draw(cx);
        assert_eq!(activations.get(), 1);
    }

    #[gpui::test]
    fn provider_usage_renders_loading_empty_and_snapshot_cards(cx: &mut gpui::TestAppContext) {
        for (loading, results, selectors) in [
            (true, Vec::new(), vec!["provider-usage-loading"]),
            (false, Vec::new(), vec!["provider-usage-empty"]),
            (
                false,
                sample(),
                vec![
                    "provider-usage-openai",
                    "provider-usage-openai-window-0",
                    "provider-usage-openai-credits",
                    "provider-usage-copilot",
                ],
            ),
        ] {
            let (probe, cx) = cx.add_window_view(|_, _| UsageProbe {
                results: results.clone(),
                loading,
            });
            cx.simulate_resize(size(px(300.), px(300.)));
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(300.)), |_, _| {
                probe.clone().into_any_element()
            });
            for selector in selectors {
                assert!(cx.debug_bounds(selector).is_some(), "missing {selector}");
            }
        }
    }
}
