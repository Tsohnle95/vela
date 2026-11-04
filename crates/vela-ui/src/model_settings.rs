use super::*;

impl VelaApp {
    pub(super) fn render_settings_model(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let (model_labels, selected_model) = self.settings_model_options();
        let model_enabled = self.active_id.is_some() && !self.models.is_empty();
        self.settings_model_focus = self.settings_model_focus.clone().tab_stop(model_enabled);
        let bounds = self.settings_model_bounds.clone();
        let font = gpui::font(theme::FONT_UI);
        let font_id = window.text_system().resolve_font(&font);
        // font-kit uses a negative descender; CSS font height includes its
        // magnitude below the baseline.
        let font_height = window.text_system().ascent(font_id, px(11.)).round()
            + window.text_system().descent(font_id, px(11.)).abs().round();
        // Chromium 138 adds themed inner spacing to the CSS select padding:
        // 4 at the start, 1 + a 15px arrow at the end, and 1 above/below.
        // layout_box.cc::MenuListIntrinsicInlineSize also rounds text up.
        let width = model_labels
            .iter()
            .map(|label| {
                let text: SharedString = label.clone().into();
                let run = TextRun {
                    len: text.len(),
                    font: font.clone(),
                    color: theme.text.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                window
                    .text_system()
                    .shape_line(text, px(11.), &[run], None)
                    .width()
            })
            .max()
            .unwrap_or(px(0.))
            .ceil()
            + px(62.);
        let select = div()
            .id("settings-default-model")
            .role(gpui::Role::ComboBox)
            .aria_label("Default model")
            .aria_value(model_labels[selected_model].clone())
            .relative()
            .w(width.min(px(360.)))
            .max_w(gpui::relative(0.52))
            .min_w_0()
            .pl(px(14.))
            .pr(px(46.))
            .py(px(8.))
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.inset)
            .flex()
            .items_center()
            .text_size(px(11.0))
            .line_height(font_height)
            .text_color(theme.text)
            .when(model_enabled, |select| select.cursor_pointer())
            .when(!model_enabled, |select| select.opacity(0.5))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.open_settings_models(window, cx)),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if matches!(
                    event.keystroke.key.as_str(),
                    "space" | "enter" | "up" | "down"
                ) {
                    this.open_settings_models(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(model_labels[selected_model].clone())),
            )
            .child(
                gpui::canvas(
                    move |rect, _, _| bounds.set(Some(rect.dilate(px(1.)))),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(crate::providers::select_chevron(theme.text));
        let select = crate::scroll::focused_control(
            "reveal-settings-model",
            self.settings_scroll.clone(),
            self.settings_model_focus.clone(),
            select,
        );
        let permissions = div()
            .flex()
            .flex_shrink_0()
            .p(px(2.))
            .rounded(px(999.))
            .bg(theme.inset)
            .child(self.settings_approval_choice("Ask", false, cx))
            .child(self.settings_approval_choice("Approve", true, cx));
        let follow_up = div()
            .flex()
            .flex_shrink_0()
            .p(px(2.))
            .rounded(px(999.))
            .bg(theme.inset)
            .child(self.settings_follow_up_choice("Queue", false, cx))
            .child(self.settings_follow_up_choice("Steer", true, cx));
        let badge =
            crate::settings::badge(self.runtime_cli_version.as_deref().unwrap_or("Not found"));
        let sync = self.runtime_button(vela_engine::runtime::RuntimeAction::Sync, cx);
        let update = self.runtime_button(vela_engine::runtime::RuntimeAction::Update, cx);
        let callout = crate::settings::callout(
            "Vela uses the global OpenCode CLI.",
            "Sync and update restart the shared OpenCode service. Active runs in Vela or other OpenCode apps may be interrupted.",
        );
        let body = div().w_full().min_w_0().flex().flex_col()
            .child(settings_list(setting_row("Default model",
                if self.active_id.is_some() { format!("Used for new prompts in {}.", self.workspace_directory()) }
                else { "Open a workspace to choose its default model.".into() }, select)))
            .child(settings_group_title("SAFETY"))
            .when(self.runtime_capabilities.permissions, |body| {
                body.child(settings_list(
                    setting_row(
                        "Tool permissions",
                        "Choose whether agent tool actions need confirmation.",
                        permissions,
                    )
                    .debug_selector(|| "settings-permission-row".into()),
                ))
            })
            .when(self.runtime_capabilities.steering, |body| {
                body.child(settings_list(
                    setting_row(
                        "Follow-up behavior",
                        "Queue new prompts or use them to steer active work.",
                        follow_up,
                    )
                    .debug_selector(|| "settings-steering-row".into()),
                ))
            })
            .child(settings_group_title("OPENCODE"))
            .child(callout)
            .child(settings_list(div().flex().flex_col()
                .child(setting_row("Installed CLI", "The version resolved from Vela's PATH.", badge)
                    .border_b_1().border_color(theme.border))
                .child(setting_row("Sync installed version",
                    "Restart the shared service to use the CLI version already installed.", sync)
                    .border_b_1().border_color(theme.border))
                .child(setting_row("Update to latest",
                    "Run OpenCode's updater for its detected install method, then sync the service.", update))))
            .when(!self.runtime_feedback.is_empty(), |body| body.child(
                div().id("runtime-feedback").mt(px(12.)).mx(px(2.)).text_size(px(11.)).line_height(gpui::relative(1.5))
                    .text_color(theme.dim).whitespace_normal().role(gpui::Role::Label)
                    .aria_label(self.runtime_feedback.clone()).child(self.runtime_feedback.clone())
            ));
        self.render_settings_body(
            "Model",
            "Choose the model used by the current workspace.",
            body.into_any_element(),
            window,
            cx,
        )
    }

    fn runtime_button(
        &self,
        action: vela_engine::runtime::RuntimeAction,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use vela_engine::runtime::RuntimeAction;
        let theme = Theme::default();
        let id = match action {
            RuntimeAction::Sync => "runtime-sync",
            RuntimeAction::Update => "runtime-update",
        };
        let busy = self
            .runtime_action
            .is_some_and(|(_, pending)| pending == action);
        let enabled = self.runtime_cli_version.is_some()
            && self.runtime_action.is_none()
            && self.runtime_confirmation.is_none();
        let label = match (action, busy) {
            (RuntimeAction::Sync, false) => "Sync",
            (RuntimeAction::Sync, true) => "Syncing…",
            (RuntimeAction::Update, false) => "Update",
            (RuntimeAction::Update, true) => "Updating…",
        };
        let control = div()
            .id(id)
            .role(gpui::Role::Button)
            .aria_label(label)
            .flex_shrink_0()
            .px(px(12.))
            .py(px(7.))
            .border_1()
            .border_color(theme.border_strong)
            .rounded(px(999.))
            .bg(theme.inset)
            .text_size(px(10.5))
            .text_color(theme.text)
            .when(enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.accent).border_color(theme.accent))
            })
            .when(!enabled, |button| button.opacity(0.5))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.confirm_runtime_action(action, window, cx)
            }))
            .child(label);
        crate::scroll::focus_control(id, self.settings_scroll.clone(), enabled, control)
            .into_any_element()
    }

    fn confirm_runtime_action(
        &mut self,
        action: vela_engine::runtime::RuntimeAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.runtime_cli_version.is_none()
            || self.runtime_action.is_some()
            || self.runtime_confirmation.is_some()
        {
            return;
        }
        let intent = match action {
            vela_engine::runtime::RuntimeAction::Sync => {
                "Vela will restart the shared OpenCode service so it matches the installed CLI."
            }
            vela_engine::runtime::RuntimeAction::Update => {
                "OpenCode will update its global CLI using its own updater. Vela will then restart the shared OpenCode service to match."
            }
        };
        self.file_request += 1;
        let request_id = self.file_request;
        self.runtime_confirmation = Some(request_id);
        let confirmation = window.prompt(gpui::PromptLevel::Info,
            &format!("{intent}\n\nActive agent runs in Vela or other OpenCode apps may be interrupted. Continue?"),
            None, &[gpui::PromptButton::ok("OK"), gpui::PromptButton::cancel("Cancel")], cx);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let accepted = confirmation.await == Ok(0);
            let _ = this.update(cx, |this, cx| {
                if this.runtime_confirmation != Some(request_id) {
                    return;
                }
                this.runtime_confirmation = None;
                if accepted {
                    this.runtime_probe = None;
                    this.runtime_action = Some((request_id, action));
                    this.runtime_feedback.clear();
                    this.engine.manage_runtime(request_id, action);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn settings_approval_choice(
        &self,
        label: &'static str,
        approve: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let selected = self.settings.auto_approve_tools == approve;
        let control = div()
            .id(if approve {
                "tool-approval-approve"
            } else {
                "tool-approval-ask"
            })
            .role(gpui::Role::Button)
            .aria_label(label)
            .px(px(11.))
            .py(px(4.))
            .rounded(px(999.))
            .cursor_pointer()
            .text_size(px(10.5))
            .bg(if selected {
                theme.selected
            } else {
                theme::rgba(0, 0, 0, 0.)
            })
            .text_color(if selected { theme.text } else { theme.dim })
            .when(selected, |control| control.shadow(theme.small_shadow()))
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.settings.auto_approve_tools != approve {
                    this.settings.auto_approve_tools = approve;
                    this.persist_preferences(cx);
                    cx.notify();
                }
            }))
            .child(label);
        crate::scroll::focus_control(
            if approve {
                "reveal-approval-approve"
            } else {
                "reveal-approval-ask"
            },
            self.settings_scroll.clone(),
            true,
            control,
        )
        .into_any_element()
    }

    fn settings_follow_up_choice(
        &self,
        label: &'static str,
        steer: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use vela_engine::message_queue::FollowUpBehavior;
        let theme = Theme::default();
        let selected = self.settings.follow_up_behavior
            == if steer {
                FollowUpBehavior::Steer
            } else {
                FollowUpBehavior::Queue
            };
        let control = div()
            .id(if steer {
                "follow-up-steer"
            } else {
                "follow-up-queue"
            })
            .debug_selector(move || {
                if steer {
                    "follow-up-steer".into()
                } else {
                    "follow-up-queue".into()
                }
            })
            .role(gpui::Role::Button)
            .aria_label(label)
            .px(px(11.))
            .py(px(4.))
            .rounded(px(999.))
            .cursor_pointer()
            .text_size(px(10.5))
            .bg(if selected {
                theme.selected
            } else {
                theme::rgba(0, 0, 0, 0.)
            })
            .text_color(if selected { theme.text } else { theme.dim })
            .when(selected, |control| control.shadow(theme.small_shadow()))
            .on_click(cx.listener(move |this, _, _, cx| {
                let behavior = if steer {
                    FollowUpBehavior::Steer
                } else {
                    FollowUpBehavior::Queue
                };
                if this.settings.follow_up_behavior != behavior {
                    this.settings.follow_up_behavior = behavior;
                    this.persist_preferences(cx);
                    cx.notify();
                }
            }))
            .child(label);
        crate::scroll::focus_control(
            if steer {
                "reveal-follow-up-steer"
            } else {
                "reveal-follow-up-queue"
            },
            self.settings_scroll.clone(),
            true,
            control,
        )
        .into_any_element()
    }
}
