//! Global, timed feedback shared by every application surface.
use crate::theme::{self, Theme};
use gpui::{prelude::*, *};
use std::time::Duration;

struct Toast {
    id: u64,
    text: SharedString,
    error: bool,
}

#[derive(Default)]
pub(crate) struct Toasts {
    sequence: u64,
    entries: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, text: impl Into<SharedString>, error: bool, cx: &mut Context<Self>) {
        self.sequence += 1;
        let id = self.sequence;
        if self.entries.len() >= 4 {
            self.entries.drain(..self.entries.len() - 3);
        }
        self.entries.push(Toast {
            id,
            text: text.into(),
            error,
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(4)).await;
            let _ = this.update(cx, |this, cx| {
                this.entries.retain(|toast| toast.id != id);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

#[cfg(test)]
impl Toasts {
    pub(crate) fn texts(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|toast| toast.text.to_string())
            .collect()
    }
}

impl Render for Toasts {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        div()
            .absolute()
            .bottom(px(18.))
            .left_0()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .children(self.entries.iter().map(|toast| {
                div()
                    .id(("toast", toast.id))
                    .role(Role::Label)
                    .aria_label(toast.text.clone())
                    .max_w(relative(0.7))
                    .min_w_0()
                    .px(px(14.))
                    .py(px(7.))
                    .border_1()
                    .border_color(if toast.error {
                        theme::rgba(226, 152, 138, 0.6)
                    } else {
                        theme.border_strong
                    })
                    .bg(theme.elevated)
                    .rounded(px(8.))
                    .shadow(theme.small_shadow())
                    .font_family(theme::FONT_UI)
                    .text_size(px(12.5))
                    .line_height(relative(1.2))
                    .text_color(if toast.error {
                        theme::rgba(226, 152, 138, 1.)
                    } else {
                        theme.text
                    })
                    .whitespace_normal()
                    .child(toast.text.clone())
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::Toasts;
    use gpui::{TestAppContext, prelude::*};
    use std::time::Duration;

    #[gpui::test]
    fn overlapping_expiry_never_removes_a_newer_toast(cx: &mut TestAppContext) {
        let toasts = cx.new(|_| Toasts::default());
        for index in 0..5 {
            toasts.update(cx, |toasts, cx| {
                toasts.push(format!("Old {index}"), true, cx)
            });
        }
        toasts.read_with(cx, |toasts, _| {
            assert_eq!(toasts.entries.len(), 4);
            assert_eq!(toasts.entries[0].text.as_ref(), "Old 1");
        });
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(2));
        toasts.update(cx, |toasts, cx| toasts.push("Newer", false, cx));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        toasts.read_with(cx, |toasts, _| {
            assert_eq!(toasts.entries.len(), 1);
            assert_eq!(toasts.entries[0].text.as_ref(), "Newer");
        });
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        toasts.read_with(cx, |toasts, _| assert!(toasts.entries.is_empty()));
    }
}
