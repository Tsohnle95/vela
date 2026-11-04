//! Runtime inbox plus durable local recovery, matching Vela's combined queue.
use super::*;

fn first_line(text: &str) -> String {
    let first = text.split('\n').next().unwrap_or_default();
    // JavaScript's substring limit is measured in UTF-16 code units. A split
    // surrogate displays as a replacement glyph in the reference as well.
    let mut units = first.encode_utf16();
    let prefix = units.by_ref().take(100).collect::<Vec<_>>();
    let long = units.next().is_some();
    let mut line = String::from_utf16_lossy(&prefix);
    if long || text.contains('\n') {
        line.push_str("...");
    }
    line
}

fn preview(item: &InboxItem, theme: Theme) -> StyledText {
    let mut text = first_line(&item.text);
    let prefix = text.len();
    if item.attachment_count > 0 {
        text.push_str(&format!(
            "  {} attachment{}",
            item.attachment_count,
            if item.attachment_count == 1 { "" } else { "s" }
        ));
    }
    let run = |len, color: gpui::Rgba| TextRun {
        len,
        font: gpui::font(theme::FONT_UI),
        color: color.into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let mut runs = Vec::new();
    if prefix > 0 {
        runs.push(run(prefix, theme.text));
    }
    if text.len() > prefix {
        runs.push(run(text.len() - prefix, theme.faint));
    }
    StyledText::new(SharedString::from(text)).with_runs(runs)
}

fn preview_label(item: &InboxItem) -> String {
    let mut label = first_line(&item.text);
    if item.attachment_count > 0 {
        label.push_str(&format!(
            " {} attachment{}",
            item.attachment_count,
            if item.attachment_count == 1 { "" } else { "s" }
        ));
    }
    label
}

fn action_button(
    id: String,
    label: &'static str,
    glyph: &'static str,
    action: impl Fn(&mut VelaApp, &mut Window, &mut Context<VelaApp>) + 'static,
    scroll: crate::scroll::AutoScrollHandle,
    cx: &mut Context<VelaApp>,
) -> AnyElement {
    let theme = Theme::default();
    let selector = id.clone();
    crate::scroll::focus_control(
        SharedString::from(format!("queue-focus-{id}")),
        scroll,
        true,
        div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .role(gpui::Role::Button)
            .aria_label(label)
            .size(px(20.))
            .flex_shrink_0()
            .rounded(px(6.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.faint)
            .cursor_pointer()
            .hover(|s| s.bg(theme.text.opacity(0.07)).text_color(theme.text))
            .on_click(cx.listener(move |app, _, window, cx| action(app, window, cx)))
            .child(crate::icons::inherited_glyph(glyph, 16.)),
    )
    .into_any_element()
}

impl VelaApp {
    fn edit_inbox(&mut self, item: InboxItem, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_id.as_deref() != Some(item.session_id.as_str()) {
            return;
        }
        // Match the source server-inbox edit: remove the runtime entry and put
        // its text in the composer immediately. The server exposes attachment
        // counts here, rather than re-attachable local paths.
        if item.kind == "local-recovery" {
            let directory = self.workspace_directory();
            let Some(queued) = self
                .engine
                .local_queue(&directory, &item.session_id)
                .into_iter()
                .find(|queued| queued.id == item.id)
            else {
                return;
            };
            self.restore_prompt_files(&queued.files);
            self.engine
                .remove_queued(directory, item.session_id.clone(), item.id.clone());
        } else {
            self.engine
                .cancel_inbox(item.session_id.clone(), item.id.clone());
            self.attachments.clear();
        }
        self.inbox.retain(|entry| entry.id != item.id);
        self.composer = item.text;
        self.composer_input
            .update(cx, |input, cx| input.set_text(self.composer.clone(), cx));
        self.engine
            .set_draft(item.session_id, self.composer.clone());
        self.picker = None;
        window.focus(&self.composer_focus, cx);
        cx.notify();
    }

    pub(super) fn render_inbox(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let Some(session) = self.active_id.as_ref() else {
            return div().into_any_element();
        };
        let directory = self.workspace_directory();
        let mut items = self.inbox.clone();
        items.extend(
            self.engine
                .local_queue(&directory, session)
                .into_iter()
                .map(|item| InboxItem {
                    id: item.id,
                    session_id: session.clone(),
                    text: item.text,
                    attachment_count: item.files.len(),
                    kind: "local-recovery".into(),
                    delivery: "queue".into(),
                }),
        );
        if items.is_empty() {
            return div().into_any_element();
        }
        let count = items.len();
        let mut rows = div()
            .id("queued-message-list")
            .debug_selector(|| "queued-message-list".into())
            .role(gpui::Role::List)
            .aria_label("Queued messages")
            .max_h(px(96.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.));
        for (index, item) in items.iter().enumerate() {
            let edit = item.clone();
            let steer = item.clone();
            let remove = item.clone();
            let mut row = div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(2.))
                .child(
                    div()
                        .id(SharedString::from(format!("queue-preview-{}", item.id)))
                        .debug_selector({
                            let selector = format!("queue-preview-{}", item.id);
                            move || selector.clone()
                        })
                        .role(gpui::Role::Label)
                        .aria_label(SharedString::from(preview_label(item)))
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.))
                        .line_height(px(13.2))
                        .text_color(theme.text)
                        .child(preview(item, theme)),
                );
            for (offset, label, glyph) in [
                (-1isize, "Move up", "arrow-up"),
                (1, "Move down", "arrow-down"),
            ] {
                let adjacent = index
                    .checked_add_signed(offset)
                    .and_then(|index| items.get(index));
                if item.kind == "local-recovery"
                    && let Some(adjacent) = adjacent.filter(|item| item.kind == "local-recovery")
                {
                    let from = item.id.clone();
                    let to = adjacent.id.clone();
                    let session = item.session_id.clone();
                    let directory = directory.clone();
                    row = row.child(action_button(
                        format!("queue-{offset}-{}", item.id),
                        label,
                        glyph,
                        move |app, _, _| {
                            app.engine.reorder_queued(
                                directory.clone(),
                                session.clone(),
                                from.clone(),
                                to.clone(),
                            )
                        },
                        self.inbox_scroll.clone(),
                        cx,
                    ));
                }
            }
            row = row.child(action_button(
                format!("queue-edit-{}", item.id),
                "Edit in composer",
                "edit",
                move |app, window, cx| app.edit_inbox(edit.clone(), window, cx),
                self.inbox_scroll.clone(),
                cx,
            ));
            row = row.child(action_button(
                format!("queue-steer-{}", item.id),
                "Send now",
                "arrow-right",
                move |app, _, _| {
                    if steer.kind == "local-recovery" {
                        app.engine.retry_queued(
                            app.workspace_directory(),
                            steer.session_id.clone(),
                            steer.id.clone(),
                        );
                    } else {
                        app.engine
                            .steer_inbox(steer.session_id.clone(), steer.id.clone());
                    }
                },
                self.inbox_scroll.clone(),
                cx,
            ));
            row = row.child(action_button(
                format!("queue-remove-{}", item.id),
                "Remove queued message",
                "close",
                move |app, _, _| {
                    if remove.kind == "local-recovery" {
                        app.engine.remove_queued(
                            app.workspace_directory(),
                            remove.session_id.clone(),
                            remove.id.clone(),
                        );
                    } else {
                        app.engine
                            .cancel_inbox(remove.session_id.clone(), remove.id.clone());
                    }
                },
                self.inbox_scroll.clone(),
                cx,
            ));
            rows = rows.child(row);
        }
        div()
            .w_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(4.))
            .px(px(6.))
            .pt(px(2.))
            .pb(px(6.))
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .id("queue-heading")
                    .role(gpui::Role::Label)
                    .aria_label(SharedString::from(format!("Queued messages {}", count)))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(10.))
                    .line_height(px(12.))
                    .text_color(theme.faint)
                    .child(crate::typography::TrackedText::new(
                        "QUEUED MESSAGES",
                        px(0.4),
                    ))
                    .child(
                        div()
                            .text_color(theme.yellow)
                            .child(SharedString::from(count.to_string())),
                    ),
            )
            .child(crate::scroll::AutoScroll::new(
                self.inbox_scroll.clone(),
                move |scroll, gutter| {
                    rows.track_scroll(&scroll)
                        .scrollbar_width(gutter)
                        .into_any_element()
                },
            ))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_preview_is_a_first_line_with_bounded_unicode_text() {
        assert_eq!(first_line(""), "");
        assert_eq!(first_line("first\nsecond"), "first...");
        assert_eq!(first_line("first\n"), "first...");
        assert_eq!(
            first_line(&"🌍".repeat(101)),
            format!("{}...", "🌍".repeat(50))
        );
        assert_eq!(
            first_line(&format!("{}🌍", "a".repeat(99))),
            format!("{}�...", "a".repeat(99))
        );
    }
}
