//! Native composer controls share the acknowledged runtime model selection.
use super::*;

/// Menu rows are transient; their owning toolbar controls survive view changes.
pub(super) struct PickerAnchors {
    pub(super) keyboard: crate::focus::KeyboardMode,
    attach: gpui::FocusHandle,
    send: gpui::FocusHandle,
    agent: gpui::FocusHandle,
    model: gpui::FocusHandle,
    strength: gpui::FocusHandle,
    pub(super) scope: gpui::FocusHandle,
    pub(super) pending: Option<PickerKind>,
}
impl PickerAnchors {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            keyboard: crate::focus::KeyboardMode::default(),
            attach: cx.focus_handle().tab_stop(true),
            send: cx.focus_handle(),
            agent: cx.focus_handle().tab_stop(true),
            model: cx.focus_handle().tab_stop(true),
            strength: cx.focus_handle().tab_stop(true),
            scope: cx.focus_handle().tab_stop(false),
            pending: None,
        }
    }
    pub(super) fn handle(&self, kind: PickerKind) -> gpui::FocusHandle {
        match kind {
            PickerKind::Agent => self.agent.clone(),
            PickerKind::Model => self.model.clone(),
            PickerKind::Strength => self.strength.clone(),
        }
    }
    pub(super) fn take_pending(&mut self) -> Option<gpui::FocusHandle> {
        self.pending.take().map(|kind| self.handle(kind))
    }
}

impl VelaApp {
    pub(super) fn render_agent_picker(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let bottom = self
            .composer_bounds
            .map(|b| f32::from(b.size.height) + 8.)
            .unwrap_or(100.);
        let mut menu = div()
            .id("agent-picker-menu")
            .track_focus(&self.picker_anchors.scope)
            .role(gpui::Role::Menu)
            .aria_label("Agent selection")
            .absolute()
            .right(px(22.))
            .bottom(px(bottom.max(0.)))
            .w(self
                .composer_bounds
                .map(|b| (b.size.width - px(24.)).clamp(px(0.), px(310.)))
                .unwrap_or(px(310.)))
            .max_h(px(340.))
            .p(px(6.))
            .rounded(px(14.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.elevated)
            .shadow(theme.medium_shadow())
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .on_mouse_down_out(
                cx.listener(|app, event: &gpui::MouseDownEvent, window, cx| {
                    if !app
                        .composer_bounds
                        .is_some_and(|b| b.contains(&event.position))
                    {
                        if app.picker_anchors.scope.contains_focused(window, cx) {
                            window.focus(&app.picker_anchors.handle(PickerKind::Agent), cx);
                        }
                        app.picker = None;
                        cx.notify();
                    }
                }),
            );
        for agent in &self.agents {
            let selected = self.selected_agent.as_deref() == Some(agent.id.as_str());
            let choice = agent.clone();
            menu = menu.child(crate::scroll::focus_control(
                SharedString::from(format!("agent-focus-{}", agent.id)),
                self.agent_picker_scroll.clone(),
                true,
                div()
                    .id(SharedString::from(format!("agent-{}", agent.id)))
                    .role(gpui::Role::MenuItem)
                    .aria_label(SharedString::from(agent.name.clone()))
                    .aria_selected(selected)
                    .h(px(25.))
                    .flex_shrink_0()
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .line_height(px(15.))
                    .text_color(if selected { theme.text } else { theme.dim })
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.selected).text_color(theme.text))
                    .on_click(
                        cx.listener(move |app, _, _, cx| app.choose_agent(choice.clone(), cx)),
                    )
                    .child(
                        div()
                            .w(px(16.))
                            .flex_shrink_0()
                            .font_family("codicon")
                            .text_size(px(16.))
                            .text_color(theme.accent)
                            .child(if selected {
                                icon("eab2")
                            } else {
                                SharedString::default()
                            }),
                    )
                    .child(SharedString::from(agent.name.clone())),
            ));
        }
        if self.agents.is_empty() {
            menu = menu.child(
                div()
                    .py(px(10.))
                    .px(px(8.))
                    .text_center()
                    .text_size(px(12.))
                    .text_color(theme.faint)
                    .child("No agents available."),
            );
        }
        crate::scroll::AutoScroll::new(self.agent_picker_scroll.clone(), move |scroll, gutter| {
            menu.track_scroll(&scroll)
                .scrollbar_width(gutter)
                .into_any_element()
        })
        .into_any_element()
    }
    pub(super) fn render_strength_picker(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(model) = self
            .selected_model
            .clone()
            .filter(|model| !model.variants.is_empty())
        else {
            self.picker = None;
            return div().into_any_element();
        };
        let theme = Theme::default();
        let bottom = self
            .composer_bounds
            .map(|bounds| f32::from(bounds.size.height) + 8.)
            .unwrap_or(100.);
        let mut choices = div()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(theme.border)
            .pt(px(4.))
            .mb(px(4.));
        for (index, variant) in std::iter::once(None)
            .chain(model.variants.iter().cloned().map(Some))
            .enumerate()
        {
            let label = variant_label(variant.as_deref());
            let chosen = model.variant == variant;
            let mut selection = model.clone();
            selection.variant = variant;
            choices =
                choices.child(crate::scroll::focus_control(
                    SharedString::from(format!("strength-focus-{index}")),
                    self.strength_scroll.clone(),
                    true,
                    div()
                        .id(SharedString::from(format!("strength-option-{index}")))
                        .role(gpui::Role::MenuItem)
                        .aria_label(SharedString::from(label.clone()))
                        .aria_selected(chosen)
                        .h(px(25.))
                        .flex_shrink_0()
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .line_height(px(15.))
                        .text_color(if chosen { theme.text } else { theme.dim })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.selected).text_color(theme.text))
                        .on_click(cx.listener(move |app, _, _, cx| {
                            app.choose_strength(selection.clone(), cx)
                        }))
                        .child(
                            div()
                                .w(px(16.))
                                .flex_shrink_0()
                                .font_family("codicon")
                                .text_size(px(16.))
                                .text_color(theme.accent)
                                .child(if chosen {
                                    icon("eab2")
                                } else {
                                    SharedString::default()
                                }),
                        )
                        .child(label),
                ));
        }
        let menu = div()
            .id("response-strength-menu")
            .track_focus(&self.picker_anchors.scope)
            .role(gpui::Role::Menu)
            .aria_label("Response strength")
            .absolute()
            .right(px(22.))
            .bottom(px(bottom.max(0.)))
            .w(self
                .composer_bounds
                .map(|bounds| (bounds.size.width - px(24.)).max(px(0.)))
                .unwrap_or(px(AGENT_WIDTH - 45.)))
            .max_h(px(340.))
            .p(px(6.))
            .border_1()
            .border_color(theme.border_strong)
            .rounded(px(14.))
            .bg(theme.elevated)
            .shadow(theme.medium_shadow())
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .on_mouse_down_out(
                cx.listener(|app, event: &gpui::MouseDownEvent, window, cx| {
                    if !app
                        .composer_bounds
                        .is_some_and(|bounds| bounds.contains(&event.position))
                    {
                        if app.picker_anchors.scope.contains_focused(window, cx) {
                            window.focus(&app.picker_anchors.handle(PickerKind::Strength), cx);
                        }
                        app.picker = None;
                        cx.notify();
                    }
                }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .px(px(4.))
                    .pt(px(2.))
                    .pb(px(6.))
                    .mb(px(4.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.faint)
                            .child(crate::typography::TrackedText::new(
                                "RESPONSE STRENGTH",
                                px(0.6),
                            )),
                    )
                    .child(
                        div()
                            .id("strength-back")
                            .role(gpui::Role::Button)
                            .aria_label("Back to model list")
                            .focusable()
                            .tab_stop(true)
                            .size(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .font_family("codicon")
                            .text_size(px(16.))
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.picker = Some(PickerKind::Model);
                                app.picker_anchors.pending = Some(PickerKind::Model);
                                cx.notify();
                            }))
                            .child(icon("ea9b")),
                    ),
            )
            .child(choices);
        crate::scroll::AutoScroll::new(self.strength_scroll.clone(), move |scroll, gutter| {
            menu.track_scroll(&scroll)
                .scrollbar_width(gutter)
                .into_any_element()
        })
        .into_any_element()
    }
    fn choose_strength(&mut self, selection: ModelInfo, cx: &mut Context<Self>) {
        if let Some(id) = self.active_id.clone() {
            self.engine.switch_model(id, selection);
            self.picker = None;
            self.picker_anchors.pending = Some(PickerKind::Strength);
            cx.notify();
        }
    }

    fn render_reference_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.reference_files.is_empty() {
            return None;
        }
        let theme = Theme::default();
        let selected = self
            .reference_selected
            .min(self.reference_files.len().saturating_sub(1));
        let mut menu = div()
            .id("composer-reference-menu")
            .role(Role::Menu)
            .aria_label("File references")
            .w_full()
            .max_h(px(240.))
            .overflow_y_scroll()
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .p(px(4.))
            .flex()
            .flex_col()
            .gap(px(1.));
        for (index, path) in self.reference_files.iter().enumerate() {
            let path = path.clone();
            menu = menu.child(
                div()
                    .id(SharedString::from(format!("reference-{index}")))
                    .role(Role::MenuItem)
                    .aria_label(SharedString::from(format!("@{path}")))
                    .h(px(30.))
                    .px(px(8.))
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .bg(if index == selected {
                        theme.selected
                    } else {
                        theme::rgba(0, 0, 0, 0.)
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.apply_reference_selection(index, window, cx);
                    }))
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_family("codicon")
                            .text_size(px(13.))
                            .text_color(theme.faint)
                            .child(icon("ea7b")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.))
                            .text_color(theme.text)
                            .child(path),
                    ),
            );
        }
        Some(menu.into_any_element())
    }

    fn render_slash_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let items = self.slash_items();
        if items.is_empty() {
            return None;
        }
        let theme = Theme::default();
        let selected = self.slash_selected.min(items.len().saturating_sub(1));
        let mut menu = div()
            .id("composer-slash-menu")
            .debug_selector(|| "composer-slash-menu".into())
            .role(Role::Menu)
            .aria_label("Slash commands")
            .w_full()
            .max_h(px(220.))
            .overflow_y_scroll()
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.panel)
            .p(px(4.))
            .flex()
            .flex_col()
            .gap(px(1.));
        for (index, item) in items.iter().enumerate() {
            let name = item.name.clone();
            let kind = match item.kind {
                vela_opencode::CommandKind::Builtin => "built-in",
                vela_opencode::CommandKind::Command => "command",
                vela_opencode::CommandKind::Skill => "skill",
            };
            menu = menu.child(
                div()
                    .id(SharedString::from(format!("slash-command-{name}")))
                    .role(Role::MenuItem)
                    .aria_label(SharedString::from(format!("/{name}")))
                    .h(px(34.))
                    .px(px(8.))
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .bg(if index == selected {
                        theme.selected
                    } else {
                        theme::rgba(0, 0, 0, 0.)
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.apply_slash_selection(index, window, cx);
                    }))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(format!("/{name}")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(10.5))
                            .text_color(theme.faint)
                            .child(item.description.clone().unwrap_or_default()),
                    )
                    .child(div().text_size(px(9.5)).text_color(theme.faint).child(kind)),
            );
        }
        Some(menu.into_any_element())
    }

    fn render_slash_argument_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.slash_arg_files.is_empty() {
            return None;
        }
        let weak = cx.weak_entity();
        let on_select: crate::slash_args::SelectFn = std::rc::Rc::new(move |index, window, cx| {
            let _ = weak.update(cx, |app, cx| {
                app.apply_slash_argument(index, window, cx);
            });
        });
        Some(crate::slash_args::render_menu(
            &self.slash_arg_files,
            self.slash_arg_selected,
            on_select,
        ))
    }

    pub(super) fn render_composer(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let has_local_queue = self.active_id.as_ref().is_some_and(|session| {
            !self
                .engine
                .local_queue(&self.workspace_directory(), session)
                .is_empty()
        });
        let model_label = self
            .selected_model
            .as_ref()
            .map(|model| {
                if model.name.is_empty() {
                    model.id.clone()
                } else {
                    model.name.clone()
                }
            })
            .unwrap_or_else(|| "Model".into());
        let agent_label = self.selected_agent.as_ref().map(|id| {
            self.agents
                .iter()
                .find(|agent| &agent.id == id)
                .map(|agent| agent.name.clone())
                .unwrap_or_else(|| id.clone())
        });
        let can_send = self.can_send();
        let busy = self.phase.is_active();
        let can_stop = busy;
        let placeholder = "Ask about this repository or describe a change…";
        self.composer_input
            .update(cx, |input, _| input.set_placeholder(placeholder));
        let input = div()
            .id("composer")
            .key_context("VelaComposer")
            .on_key_down(cx.listener(Self::on_composer_key))
            .w_full()
            .min_w_0()
            .min_h(px(44.))
            .max_h(px(120.))
            .px(px(3.))
            .pt(px(2.))
            .text_size(px(if self.composer.is_empty() { 11. } else { 12.5 }))
            .line_height(px(18.))
            .text_color(theme.text)
            .child(self.composer_input.clone());
        let codex = self.is_codex();
        let strength = self
            .selected_model
            .as_ref()
            .filter(|model| !model.variants.is_empty());
        let strength_label = strength.map(|model| variant_label(model.variant.as_deref()));
        let measured = cx.weak_entity();
        let card_measurement = gpui::canvas(
            move |bounds, _, cx| {
                // This borderless group includes staged chips and the card,
                // matching the source composer's complete popup anchor.
                let _ = measured.update(cx, |app, cx| {
                    if app.composer_bounds != Some(bounds) {
                        app.composer_bounds = Some(bounds);
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let reference_menu = self.render_reference_menu(cx);
        let slash_menu = self.render_slash_menu(cx);
        let slash_argument_menu = self.render_slash_argument_menu(cx);
        div()
            .relative()
            .p(px(10.))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .when(!self.todos.is_empty(), |root| {
                root.child(self.render_todos())
            })
            .when(self.staged_revert_active(), |root| {
                root.child(self.render_staged_revert(cx))
            })
            .when(!self.permissions.is_empty(), |root| {
                root.child(self.render_permissions(cx))
            })
            .when_some(reference_menu, |root, menu| root.child(menu))
            .when_some(slash_menu, |root, menu| root.child(menu))
            .when_some(slash_argument_menu, |root, menu| root.child(menu))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(card_measurement)
                    .when(!self.attachments.is_empty(), |root| {
                        root.child(self.render_attachments(cx))
                    })
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .min_h(px(96.))
                            .rounded(px(if codex { 16.0 } else { 13.0 }))
                            .border_1()
                            .border_color(if codex {
                                theme.text.opacity(0.06)
                            } else {
                                theme.border_strong
                            })
                            .bg(theme.elevated)
                            .px(px(8.0))
                            .py(px(8.0))
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(input)
                            .child(
                                div()
                                    .h(px(28.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(3.0))
                                    .when(self.runtime_capabilities.attachments, |toolbar| {
                                        toolbar.child(
                                            div()
                                                .id("composer-add")
                                                .debug_selector(|| "composer-add".into())
                                                .role(gpui::Role::Button)
                                                .aria_label("Attach files")
                                                .track_focus(&self.picker_anchors.attach)
                                                .map(|button| {
                                                    crate::focus::outline(
                                                        button,
                                                        &self.picker_anchors.attach,
                                                        8.,
                                                        2.,
                                                        1.,
                                                        theme.accent.into(),
                                                        &self.picker_anchors.keyboard,
                                                    )
                                                })
                                                .on_click(cx.listener(|app, _, window, cx| {
                                                    app.attach_files(window, cx)
                                                }))
                                                .size(px(28.0))
                                                .flex_shrink_0()
                                                .border_1()
                                                .border_color(theme.border)
                                                .rounded(px(8.0))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .cursor_pointer()
                                                .text_size(px(16.0))
                                                .text_color(theme.dim)
                                                .hover(|style| style.bg(theme.inset))
                                                .font_family("codicon")
                                                .child(icon("ea60")),
                                        )
                                    })
                                    .when(self.runtime_capabilities.agents, |toolbar| {
                                        toolbar.child(
                                            div()
                                                .id("agent-picker")
                                                .debug_selector(|| "agent-picker".into())
                                                .role(gpui::Role::Button)
                                                .aria_label(SharedString::from(
                                                    agent_label
                                                        .map(|name| format!("Change agent: {name}"))
                                                        .unwrap_or_else(|| "Change agent".into()),
                                                ))
                                                .track_focus(
                                                    &self.picker_anchors.handle(PickerKind::Agent),
                                                )
                                                .map(|button| {
                                                    crate::focus::outline(
                                                        button,
                                                        &self
                                                            .picker_anchors
                                                            .handle(PickerKind::Agent),
                                                        8.,
                                                        1.,
                                                        0.,
                                                        theme.accent.into(),
                                                        &self.picker_anchors.keyboard,
                                                    )
                                                })
                                                .h(px(28.0))
                                                .w(px(26.))
                                                .flex_shrink_0()
                                                .justify_center()
                                                .rounded(px(8.0))
                                                .flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .cursor_pointer()
                                                .text_size(px(11.))
                                                .text_color(
                                                    if self.picker == Some(PickerKind::Agent) {
                                                        theme.accent
                                                    } else {
                                                        theme.dim
                                                    },
                                                )
                                                .hover(|style| style.bg(theme.inset))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.picker =
                                                        if this.picker == Some(PickerKind::Agent) {
                                                            None
                                                        } else {
                                                            Some(PickerKind::Agent)
                                                        };
                                                    this.picker_anchors.pending =
                                                        Some(PickerKind::Agent);
                                                    cx.notify();
                                                }))
                                                .font_family("codicon")
                                                .text_size(px(16.))
                                                .line_height(px(16.))
                                                .child(icon("ec6f")),
                                        )
                                    })
                                    .child(div().flex_1())
                                    .child(
                                        div()
                                            .id("model-picker")
                                            .role(gpui::Role::Button)
                                            .aria_label(SharedString::from(format!(
                                                "Change model: {model_label}"
                                            )))
                                            .track_focus(
                                                &self.picker_anchors.handle(PickerKind::Model),
                                            )
                                            .map(|button| {
                                                crate::focus::outline(
                                                    button,
                                                    &self.picker_anchors.handle(PickerKind::Model),
                                                    8.,
                                                    1.,
                                                    0.,
                                                    theme.accent.into(),
                                                    &self.picker_anchors.keyboard,
                                                )
                                            })
                                            .h(px(28.0))
                                            .min_w(px(52.))
                                            .max_w(px(90.))
                                            .px(px(5.0))
                                            .rounded(px(8.0))
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .cursor_pointer()
                                            .text_size(px(11.))
                                            .text_color(theme.text)
                                            .hover(|style| style.bg(theme.inset))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.picker =
                                                    if this.picker == Some(PickerKind::Model) {
                                                        None
                                                    } else {
                                                        Some(PickerKind::Model)
                                                    };
                                                this.picker_anchors.pending =
                                                    Some(PickerKind::Model);
                                                cx.notify();
                                            }))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .child(SharedString::from(model_label)),
                                            )
                                            .child(
                                                div()
                                                    .font_family("codicon")
                                                    .text_size(px(16.))
                                                    .line_height(px(16.))
                                                    .flex_shrink_0()
                                                    .child(icon("eab4")),
                                            ),
                                    )
                                    .when_some(strength_label, |row, label| {
                                        row.child(
                                            div()
                                                .id("strength-picker")
                                                .role(gpui::Role::Button)
                                                .track_focus(
                                                    &self
                                                        .picker_anchors
                                                        .handle(PickerKind::Strength),
                                                )
                                                .map(|button| {
                                                    crate::focus::outline(
                                                        button,
                                                        &self
                                                            .picker_anchors
                                                            .handle(PickerKind::Strength),
                                                        8.,
                                                        1.,
                                                        0.,
                                                        theme.accent.into(),
                                                        &self.picker_anchors.keyboard,
                                                    )
                                                })
                                                .aria_label(SharedString::from(format!(
                                                    "Change response strength: {label}"
                                                )))
                                                .h(px(28.0))
                                                .min_w(px(42.))
                                                .max_w(px(58.))
                                                .px(px(5.0))
                                                .rounded(px(8.0))
                                                .flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .text_size(px(11.))
                                                .text_color(
                                                    if self.picker == Some(PickerKind::Strength) {
                                                        theme.accent
                                                    } else {
                                                        theme.dim
                                                    },
                                                )
                                                .cursor_pointer()
                                                .hover(|style| style.bg(theme.inset))
                                                .on_click(cx.listener(|app, _, _, cx| {
                                                    app.picker = if app.picker
                                                        == Some(PickerKind::Strength)
                                                    {
                                                        None
                                                    } else {
                                                        Some(PickerKind::Strength)
                                                    };
                                                    app.picker_anchors.pending =
                                                        Some(PickerKind::Strength);
                                                    cx.notify();
                                                }))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .truncate()
                                                        .child(label),
                                                )
                                                .child(
                                                    div()
                                                        .font_family("codicon")
                                                        .text_size(px(16.))
                                                        .line_height(px(16.))
                                                        .flex_shrink_0()
                                                        .child(icon("eab4")),
                                                ),
                                        )
                                    })
                                    .child(
                                        div()
                                            .id("send")
                                            .role(gpui::Role::Button)
                                            .aria_label(if can_stop {
                                                "Stop the agent"
                                            } else {
                                                "Send prompt"
                                            })
                                            .size(px(28.0))
                                            .flex_shrink_0()
                                            .rounded(px(999.0))
                                            .bg(if can_stop { theme.red } else { theme.accent })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .text_size(px(if can_stop { 10.0 } else { 14.0 }))
                                            .text_color(if can_stop {
                                                gpui::rgb(0xffffff)
                                            } else {
                                                theme.send_ink
                                            })
                                            .opacity(if can_stop || busy || can_send {
                                                1.0
                                            } else {
                                                0.55
                                            })
                                            .track_focus(&self.picker_anchors.send)
                                            .tab_stop(can_stop || can_send)
                                            .map(|button| {
                                                crate::focus::outline(
                                                    button,
                                                    &self.picker_anchors.send,
                                                    14.,
                                                    1.,
                                                    0.,
                                                    theme.accent.into(),
                                                    &self.picker_anchors.keyboard,
                                                )
                                            })
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                if this.phase.is_active() {
                                                    this.interrupt(cx);
                                                } else {
                                                    this.send(window, cx);
                                                }
                                            }))
                                            .child(crate::icons::inherited_glyph(
                                                if can_stop { "stop" } else { "arrow-up" },
                                                16.,
                                            )),
                                    ),
                            )
                            .when(self.attachment_drag_over && cx.has_active_drag(), |card| {
                                card.child(self.attachment_drop_hint())
                            }),
                    )
                    .when(
                        self.active_id.is_some() && (!self.inbox.is_empty() || has_local_queue),
                        |root| root.child(self.render_inbox(cx)),
                    ),
            )
            .into_any_element()
    }
}

pub(super) fn model_matches(model: &ModelInfo, query: &str) -> bool {
    query.is_empty()
        || format!("{} {} {}", model.name, model.id, model.provider_id)
            .to_lowercase()
            .contains(query)
}

pub(super) fn variant_label(variant: Option<&str>) -> String {
    let Some(variant) = variant.filter(|variant| !variant.is_empty()) else {
        return "Auto".into();
    };
    let mut result = String::new();
    let mut separator = false;
    let mut previous_word = false;
    for character in variant.chars() {
        if character == '-' || character == '_' {
            if !separator {
                result.push(' ');
            }
            separator = true;
            previous_word = false;
        } else {
            separator = false;
            result.push(if !previous_word {
                character.to_ascii_uppercase()
            } else {
                character
            });
            previous_word = character.is_ascii_alphanumeric() || character == '_';
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    struct PickerSwap {
        anchors: PickerAnchors,
        back: gpui::FocusHandle,
        open: Option<PickerKind>,
        escaped: usize,
        quit: usize,
    }
    impl Render for PickerSwap {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            // The pre-fix render path left focus on the removed Back control.
            // Restoring the shared owner also keeps global actions reachable.
            if let Some(anchor) = self.anchors.take_pending() {
                window.focus(&anchor, cx);
            }
            div()
                .size_full()
                .flex()
                .flex_col()
                .on_action(cx.listener(|this, _: &crate::input::RequestQuit, _, cx| {
                    this.quit += 1;
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape"
                        && let Some(kind) = this.open.take()
                    {
                        this.anchors.pending = Some(kind);
                        this.escaped += 1;
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .id("permanent-model-anchor")
                        .track_focus(&self.anchors.handle(PickerKind::Model))
                        .h(px(24.))
                        .child("Model"),
                )
                .when(self.open.is_some(), |root| {
                    root.child(
                        div()
                            .id("picker-swap-popup")
                            .track_focus(&self.anchors.scope)
                            .when(self.open == Some(PickerKind::Strength), |popup| {
                                popup.child(
                                    div()
                                        .id("transient-back")
                                        .track_focus(&self.back)
                                        .h(px(24.))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open = Some(PickerKind::Model);
                                            this.anchors.pending = Some(PickerKind::Model);
                                            cx.notify();
                                        }))
                                        .child("Back to model list"),
                                )
                            })
                            .when(self.open == Some(PickerKind::Model), |popup| {
                                popup.child("Model choices")
                            }),
                    )
                })
        }
    }
    #[gpui::test]
    fn replacing_picker_restores_owner_and_keeps_escape_and_quit_reachable(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            cx.bind_keys([gpui::KeyBinding::new(
                "cmd-q",
                crate::input::RequestQuit,
                None,
            )])
        });
        let (view, cx) = cx.add_window_view(|window, cx| {
            let back = cx.focus_handle().tab_stop(true);
            window.focus(&back, cx);
            PickerSwap {
                anchors: PickerAnchors::new(cx),
                back,
                open: Some(PickerKind::Strength),
                escaped: 0,
                quit: 0,
            }
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(160.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        cx.simulate_resize(size(px(300.), px(160.)));
        draw(cx);
        let enter = gpui::Keystroke::parse("enter").unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: enter.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke: enter });
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx);
            assert!(state.open == Some(PickerKind::Model));
            assert!(
                state.anchors.handle(PickerKind::Model).is_focused(window),
                "focus must leave the unmounted Back control"
            );
        });
        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.simulate_keystrokes("cmd-q");
        cx.update(|_, cx| {
            let state = view.read(cx);
            assert!(state.open.is_none());
            assert_eq!(state.escaped, 1);
            assert_eq!(state.quit, 1);
        });
    }
    #[test]
    fn model_search_matches_name_id_provider_and_unicode() {
        let model = vela_opencode::client::model_from_value(&serde_json::json!({"id":"reasoning-29", "providerID":"Provider-X", "name":"Long Unicode 🌍 Model"})).unwrap();
        for query in ["", "unicode", "🌍", "reasoning-29", "provider-x"] {
            assert!(model_matches(&model, query));
        }
        assert!(!model_matches(&model, "missing-model"));
    }
    #[test]
    fn strength_labels_match_reference_auto_and_ascii_word_formatting() {
        for (variant, label) in [
            (None, "Auto"),
            (Some(""), "Auto"),
            (Some("extra_high"), "Extra High"),
            (Some("very--low"), "Very Low"),
            (Some("level2_fast"), "Level2 Fast"),
            (Some("🌍high"), "🌍High"),
        ] {
            assert_eq!(variant_label(variant), label);
        }
    }
}
