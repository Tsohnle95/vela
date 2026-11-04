//! Composer model catalog and its persistent, provider-qualified preferences.
use super::*;
use std::collections::BTreeSet;

fn model_key(model: &ModelInfo) -> String {
    format!("{}::{}", model.provider_id, model.id)
}
fn same_model(a: &ModelInfo, b: &ModelInfo) -> bool {
    a.id == b.id && a.provider_id == b.provider_id
}
fn visible(model: &ModelInfo, selected: Option<&ModelInfo>, settings: &Settings) -> bool {
    selected.is_some_and(|current| same_model(current, model))
        || (!settings.hidden_providers.contains(&model.provider_id)
            && !settings.hidden_models.contains(&model_key(model)))
}
fn toggle(keys: &mut BTreeSet<String>, key: String) {
    if !keys.remove(&key) {
        keys.insert(key);
    }
}

fn favorite_models(models: &[ModelInfo], settings: &Settings, query: &str) -> Vec<ModelInfo> {
    let mut favorites: Vec<_> = models
        .iter()
        .filter(|model| settings.favorite_models.contains(&model_key(model)))
        .filter(|model| composer::model_matches(model, query))
        .cloned()
        .collect();
    favorites.sort_by_key(|model| model.name.to_lowercase());
    favorites
}

impl VelaApp {
    pub(super) fn cycle_favorite_model(&mut self, cx: &mut Context<Self>) {
        let favorites = favorite_models(
            &self.models,
            &self.settings,
            &self.model_search.trim().to_lowercase(),
        );
        if !favorites.is_empty() {
            let next = self
                .selected_model
                .as_ref()
                .and_then(|selected| favorites.iter().position(|m| same_model(m, selected)))
                .map_or(0, |index| (index + 1) % favorites.len());
            self.choose_model(favorites[next].clone(), cx);
            self.picker_anchors.pending = None;
        }
    }

    pub(super) fn cycle_response_strength(&mut self) {
        if let Some(mut model) = self
            .selected_model
            .clone()
            .filter(|m| !m.variants.is_empty())
            && let Some(session) = self.active_id.clone()
        {
            let index = model
                .variant
                .as_ref()
                .and_then(|v| model.variants.iter().position(|s| s == v))
                .map_or(0, |i| i + 1);
            model.variant = model.variants.get(index).cloned();
            self.engine.switch_model(session, model);
        }
    }
    fn model_option(
        &self,
        model: ModelInfo,
        favorite_row: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let key = model_key(&model);
        let favorite = self.settings.favorite_models.contains(&key);
        let selected = self
            .selected_model
            .as_ref()
            .is_some_and(|m| same_model(m, &model));
        let settings = self.model_menu_settings;
        let shown = !self.settings.hidden_providers.contains(&model.provider_id)
            && !self.settings.hidden_models.contains(&key);
        let name = if model.name.is_empty() {
            model.id.clone()
        } else {
            model.name.clone()
        };
        let selection = model.clone();
        let toggle_key = key.clone();
        let row_selector = format!("model-row-{key}");
        let favorite_selector = format!("favorite-{key}");
        let row = div()
            .id(SharedString::from(format!(
                "model-{}-{key}",
                if favorite_row { "favorite" } else { "catalog" }
            )))
            .debug_selector(move || row_selector.clone())
            .role(gpui::Role::Button)
            .aria_label(SharedString::from(if settings {
                format!("{} {name}", if shown { "Hide" } else { "Show" })
            } else {
                name.clone()
            }))
            .aria_selected(selected && !settings)
            .focusable()
            .tab_stop(true)
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
            .when(settings && !shown, |row| row.opacity(0.5))
            .cursor_pointer()
            .hover(|style| style.bg(theme.selected).text_color(theme.text))
            .on_click(cx.listener(move |app, _, _, cx| {
                if settings {
                    toggle(&mut app.settings.hidden_models, toggle_key.clone());
                    app.persist_preferences(cx);
                    cx.notify();
                } else {
                    app.choose_model(selection.clone(), cx);
                }
            }))
            .child(
                div()
                    .w(px(16.))
                    .flex_shrink_0()
                    .font_family("codicon")
                    .text_size(px(16.))
                    .text_color(if settings {
                        if shown { theme.green } else { theme.faint }
                    } else {
                        theme.accent
                    })
                    .child(if settings {
                        icon(if shown { "ea70" } else { "eae7" })
                    } else if selected {
                        icon("eab2")
                    } else {
                        SharedString::default()
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(name.clone())),
            )
            .when(!settings, |row| {
                row.child(crate::scroll::focus_control(
                    SharedString::from(format!("favorite-focus-{favorite_row}-{key}")),
                    self.model_picker_scroll.clone(),
                    true,
                    div()
                        .id(SharedString::from(format!(
                            "favorite-{}-{key}",
                            if favorite_row { "favorite" } else { "catalog" }
                        )))
                        .debug_selector(move || favorite_selector.clone())
                        .role(gpui::Role::Button)
                        .aria_label(SharedString::from(format!(
                            "{} {name} {} favorites",
                            if favorite { "Remove" } else { "Add" },
                            if favorite { "from" } else { "to" }
                        )))
                        .focusable()
                        .tab_stop(true)
                        .px(px(2.))
                        .text_size(px(13.))
                        .line_height(px(13.))
                        .text_color(if favorite { theme.yellow } else { theme.faint })
                        .hover(|style| style.text_color(theme.yellow))
                        .on_click(cx.listener(move |app, _, _, cx| {
                            cx.stop_propagation();
                            toggle(&mut app.settings.favorite_models, key.clone());
                            if favorite_row {
                                // Removing a favorite destroys this row and its focus node.
                                app.picker_anchors.pending = Some(PickerKind::Model);
                            }
                            app.persist_preferences(cx);
                            cx.notify();
                        }))
                        .child(if favorite { "★" } else { "☆" }),
                ))
            });
        crate::scroll::focus_control(
            SharedString::from(format!("model-focus-{favorite_row}-{}", model_key(&model))),
            self.model_picker_scroll.clone(),
            true,
            row,
        )
        .into_any_element()
    }

    fn model_group_heading(
        &self,
        provider: String,
        favorites: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let collapsed = self.model_collapsed.contains(&provider);
        let hidden = self.settings.hidden_providers.contains(&provider);
        let label = if favorites {
            "Favorites".to_owned()
        } else {
            provider.clone()
        };
        let group = provider.clone();
        let visibility_selector = format!("visibility-provider-{provider}");
        let visibility_provider = provider.clone();
        div()
            .flex()
            .flex_shrink_0()
            .line_height(px(16.))
            .items_center()
            .gap(px(4.))
            .px(px(8.))
            .pt(px(6.))
            .pb(px(3.))
            .child(
                div()
                    .id(SharedString::from(format!("collapse-models-{provider}")))
                    .role(gpui::Role::Button)
                    .aria_label(SharedString::from(format!(
                        "{} {label}",
                        if collapsed { "Expand" } else { "Collapse" }
                    )))
                    .focusable()
                    .tab_stop(true)
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .text_color(theme.faint)
                    .hover(|s| s.text_color(theme.dim))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if !app.model_collapsed.remove(&group) {
                            app.model_collapsed.insert(group.clone());
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .font_family("codicon")
                            .text_size(px(16.))
                            .child(icon(if collapsed { "eab6" } else { "eab4" })),
                    )
                    .when(favorites, |row| {
                        row.child(
                            div()
                                .font_family("codicon")
                                .text_size(px(16.))
                                .child(icon("eb59")),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight::BOLD)
                            .child(crate::typography::TrackedText::new(
                                label.to_uppercase(),
                                px(0.6),
                            )),
                    ),
            )
            .when(self.model_menu_settings && !favorites, |row| {
                row.child(
                    div()
                        .id(SharedString::from(format!(
                            "visibility-provider-{provider}"
                        )))
                        .debug_selector(move || visibility_selector.clone())
                        .role(gpui::Role::Button)
                        .aria_label(SharedString::from(format!(
                            "{} {provider}",
                            if hidden { "Show" } else { "Hide" }
                        )))
                        .focusable()
                        .tab_stop(true)
                        .size(px(20.))
                        .flex_shrink_0()
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .font_family("codicon")
                        .text_size(px(16.))
                        .text_color(theme.faint)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.selected).text_color(theme.text))
                        .on_click(cx.listener(move |app, _, _, cx| {
                            toggle(
                                &mut app.settings.hidden_providers,
                                visibility_provider.clone(),
                            );
                            app.persist_preferences(cx);
                            cx.notify();
                        }))
                        .child(icon(if hidden { "eae7" } else { "ea70" })),
                )
            })
            .into_any_element()
    }

    pub(super) fn render_model_picker(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let query = self.model_search.trim().to_lowercase();
        let mut models: Vec<_> = self
            .models
            .iter()
            .filter(|m| composer::model_matches(m, &query))
            .cloned()
            .collect();
        models.sort_by_key(|m| (m.provider_id.to_lowercase(), m.name.to_lowercase()));
        let favorites: Vec<_> = if self.model_menu_settings {
            Vec::new()
        } else {
            favorite_models(&models, &self.settings, &query)
        };
        let mut groups: Vec<(String, Vec<ModelInfo>)> = Vec::new();
        for model in models.into_iter().filter(|m| {
            self.model_menu_settings || visible(m, self.selected_model.as_ref(), &self.settings)
        }) {
            if let Some((_, items)) = groups.last_mut().filter(|(p, _)| p == &model.provider_id) {
                items.push(model);
            } else {
                groups.push((model.provider_id.clone(), vec![model]));
            }
        }
        let settings = self.model_menu_settings;
        let bottom = self
            .composer_bounds
            .map(|b| f32::from(b.size.height) + 8.)
            .unwrap_or(100.);
        let menu = div()
            .id("model-picker-menu")
            .debug_selector(|| "model-picker-menu".into())
            .track_focus(&self.picker_anchors.scope)
            .role(gpui::Role::Menu)
            .aria_label(if settings {
                "Model settings"
            } else {
                "Model selection"
            })
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
                            window.focus(&app.picker_anchors.handle(PickerKind::Model), cx);
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
                            .px(px(4.))
                            .text_size(px(10.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.faint)
                            .child(crate::typography::TrackedText::new(
                                if settings {
                                    "MODEL SETTINGS"
                                } else {
                                    "MODEL SELECTION"
                                },
                                px(0.6),
                            )),
                    )
                    .child(
                        div()
                            .id("model-menu-settings")
                            .debug_selector(|| "model-menu-settings".into())
                            .role(gpui::Role::Button)
                            .aria_label(if settings {
                                "Back to model list"
                            } else {
                                "Choose which models appear here"
                            })
                            .focusable()
                            .tab_stop(true)
                            .size(px(24.))
                            .flex_shrink_0()
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_family("codicon")
                            .text_size(px(16.))
                            .text_color(theme.dim)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.selected).text_color(theme.text))
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.model_menu_settings = !app.model_menu_settings;
                                app.model_picker_scroll
                                    .native_handle()
                                    .set_offset(point(px(0.), px(0.)));
                                // The control remains mounted while its label and glyph change.
                                cx.notify();
                            }))
                            .child(icon(if settings { "ea9b" } else { "eb51" })),
                    ),
            )
            .child(
                div()
                    .h(px(28.))
                    .flex_shrink_0()
                    .mb(px(4.))
                    .px(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .rounded(px(6.))
                    .bg(theme.inset)
                    .flex()
                    .items_center()
                    .text_size(px(11.5))
                    .child(self.model_search_input.clone()),
            )
            .when_some(
                self.selected_model
                    .as_ref()
                    .filter(|m| !settings && !m.variants.is_empty())
                    .map(|m| composer::variant_label(m.variant.as_deref())),
                |root, label| {
                    root.child(
                        div()
                            .id("model-menu-strength")
                            .role(gpui::Role::Button)
                            .aria_label("Change response strength")
                            .focusable()
                            .tab_stop(true)
                            .h(px(25.))
                            .flex_shrink_0()
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(12.5))
                            .text_color(theme.dim)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.selected).text_color(theme.text))
                            .on_click(cx.listener(|app, _, _, cx| {
                                app.picker = Some(PickerKind::Strength);
                                app.picker_anchors.pending = Some(PickerKind::Strength);
                                cx.notify();
                            }))
                            .child(div().w(px(16.)).flex_shrink_0())
                            .child("Response strength")
                            .child(div().flex_1())
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.faint)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .font_family("codicon")
                                    .text_size(px(16.))
                                    .child(icon("eab6")),
                            ),
                    )
                },
            );
        let no_results = favorites.is_empty() && groups.is_empty();
        let mut content = menu;
        let mut first = true;
        if !favorites.is_empty() {
            let mut group = div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .mb(px(4.))
                .pb(px(4.))
                .child(self.model_group_heading("favorites".into(), true, cx));
            if !self.model_collapsed.contains("favorites") {
                for model in favorites {
                    group = group.child(self.model_option(model, true, cx));
                }
            }
            content = content.child(group);
            first = false;
        }
        for (provider, models) in groups {
            let mut group = div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .mb(px(4.))
                .when(!first, |g| {
                    g.border_t_1().border_color(theme.border).pt(px(4.))
                })
                .child(self.model_group_heading(provider.clone(), false, cx));
            first = false;
            if !self.model_collapsed.contains(&provider) {
                for model in models {
                    group = group.child(self.model_option(model, false, cx));
                }
            }
            content = content.child(group);
        }
        if no_results {
            content = content.child(
                div()
                    .py(px(10.))
                    .px(px(8.))
                    .text_size(px(12.))
                    .text_color(theme.faint)
                    .text_center()
                    .child("No models found."),
            );
        }
        crate::scroll::AutoScroll::new(self.model_picker_scroll.clone(), move |scroll, gutter| {
            content
                .track_scroll(&scroll)
                .scrollbar_width(gutter)
                .into_any_element()
        })
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model(provider: &str) -> ModelInfo {
        ModelInfo {
            id: "shared-id".into(),
            name: "Shared".into(),
            provider_id: provider.into(),
            context_limit: None,
            variant: None,
            variants: Vec::new(),
        }
    }
    #[test]
    fn visibility_preserves_current_model_but_qualifies_provider_and_favorites_are_independent() {
        let a = model("a");
        let b = model("b");
        let mut settings = Settings::default();
        toggle(&mut settings.hidden_models, model_key(&a));
        assert!(!visible(&a, None, &settings));
        assert!(visible(&b, None, &settings));
        assert!(visible(&a, Some(&a), &settings));
        assert!(!visible(&a, Some(&b), &settings));
        toggle(&mut settings.hidden_providers, "b".into());
        assert!(!visible(&b, None, &settings));
        assert!(visible(&b, Some(&b), &settings));
        toggle(&mut settings.favorite_models, model_key(&a));
        assert!(settings.favorite_models.contains(&model_key(&a)));
        assert!(!settings.favorite_models.contains(&model_key(&b)));
        assert!(!visible(&a, None, &settings));
        toggle(&mut settings.favorite_models, model_key(&a));
        assert!(settings.favorite_models.is_empty());
    }
}
