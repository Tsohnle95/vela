//! Native pending-form dock. Form state lives here so periodic refreshes do not
//! discard in-progress answers or recreate focused text inputs.
use gpui::{prelude::*, *};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};
use vela_opencode::{FormField, FormKind, FormRequest};

use crate::{input::TextInput, theme::Theme};

pub enum Event {
    Submit {
        form_id: String,
        session_id: String,
        answers: serde_json::Map<String, serde_json::Value>,
    },
    Dismiss {
        form_id: String,
        session_id: String,
    },
    OpenExternal(String),
}
impl EventEmitter<Event> for FormDock {}

#[derive(Default)]
struct FormState {
    inputs: HashMap<String, Entity<TextInput>>,
    booleans: HashMap<String, bool>,
    multi: HashMap<String, BTreeSet<String>>,
    selections: HashMap<String, String>,
}

pub struct FormDock {
    forms: Vec<FormRequest>,
    states: HashMap<String, FormState>,
    submitting: HashSet<String>,
    focuses: HashMap<String, FocusHandle>,
}

fn state_key(form: &FormRequest, field: &FormField) -> String {
    format!("{}:{}", form.id, field.key)
}

fn text_default(field: &FormField) -> String {
    match &field.default {
        Some(serde_json::Value::String(value)) => value.clone(),
        Some(value) if value.is_number() => value.to_string(),
        _ => String::new(),
    }
}

impl FormDock {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            forms: Vec::new(),
            states: HashMap::new(),
            submitting: HashSet::new(),
            focuses: HashMap::new(),
        }
    }

    pub fn set_forms(&mut self, forms: Vec<FormRequest>, cx: &mut Context<Self>) {
        let mut valid = HashSet::new();
        for form in &forms {
            valid.insert(format!("submit:{}", form.id));
            valid.insert(format!("dismiss:{}", form.id));
            let state = self.states.entry(form.id.clone()).or_default();
            for field in &form.fields {
                let key = state_key(form, field);
                valid.insert(key.clone());
                match field.kind {
                    FormKind::Text | FormKind::Number | FormKind::Integer => {
                        if field.options.is_empty() {
                            if let std::collections::hash_map::Entry::Vacant(entry) =
                                state.inputs.entry(key)
                            {
                                let placeholder = field
                                    .placeholder
                                    .clone()
                                    .or_else(|| field.title.clone())
                                    .unwrap_or_else(|| field.key.clone());
                                let default = text_default(field);
                                let input = cx.new(|cx| {
                                    let mut input = TextInput::new(false, &placeholder, cx);
                                    if !default.is_empty() {
                                        input.set_text(default, cx);
                                    }
                                    input
                                });
                                entry.insert(input);
                            }
                        } else {
                            let default = text_default(field);
                            state.selections.entry(key).or_insert(default);
                        }
                    }
                    FormKind::Boolean => {
                        if let Some(default) = field.default.as_ref().and_then(|v| v.as_bool()) {
                            state.booleans.entry(key).or_insert(default);
                        }
                    }
                    FormKind::MultiSelect => {
                        let entry = state.multi.entry(key).or_default();
                        if entry.is_empty()
                            && let Some(values) = field.default.as_ref().and_then(|v| v.as_array())
                        {
                            entry.extend(
                                values.iter().filter_map(|v| v.as_str().map(str::to_owned)),
                            );
                        }
                    }
                    FormKind::External => {}
                }
            }
        }
        for (id, state) in &mut self.states {
            state.inputs.retain(|key, _| valid.contains(key));
            state.booleans.retain(|key, _| valid.contains(key));
            state.multi.retain(|key, _| valid.contains(key));
            state.selections.retain(|key, _| valid.contains(key));
            if !forms.iter().any(|form| &form.id == id) {
                self.submitting.remove(id);
            }
        }
        self.states
            .retain(|id, _| forms.iter().any(|form| &form.id == id));
        self.focuses.retain(|key, _| valid.contains(key));
        self.forms = forms;
        cx.notify();
    }

    pub fn failed(&mut self, form_id: &str, cx: &mut Context<Self>) {
        self.submitting.remove(form_id);
        cx.notify();
    }

    fn focus(&mut self, key: String, cx: &mut Context<Self>) -> FocusHandle {
        self.focuses
            .entry(key)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    fn complete(&self, form: &FormRequest, cx: &App) -> bool {
        let Some(state) = self.states.get(&form.id) else {
            return false;
        };
        form.fields.iter().all(|field| {
            if field.kind == FormKind::External || !field.required {
                return true;
            }
            let key = state_key(form, field);
            match field.kind {
                FormKind::Text if !field.options.is_empty() => state
                    .selections
                    .get(&key)
                    .is_some_and(|value| !value.is_empty()),
                FormKind::Text => state
                    .inputs
                    .get(&key)
                    .is_some_and(|input| !input.read(cx).buffer.text().trim().is_empty()),
                FormKind::Number => state.inputs.get(&key).is_some_and(|input| {
                    let text = input.read(cx).buffer.text();
                    !text.trim().is_empty() && text.trim().parse::<f64>().is_ok()
                }),
                FormKind::Integer => state.inputs.get(&key).is_some_and(|input| {
                    let text = input.read(cx).buffer.text();
                    !text.trim().is_empty() && text.trim().parse::<i64>().is_ok()
                }),
                FormKind::Boolean => state.booleans.contains_key(&key),
                FormKind::MultiSelect => state.multi.get(&key).is_some_and(|set| !set.is_empty()),
                FormKind::External => true,
            }
        })
    }

    fn answers(&self, form: &FormRequest, cx: &App) -> serde_json::Map<String, serde_json::Value> {
        let mut answers = serde_json::Map::new();
        let Some(state) = self.states.get(&form.id) else {
            return answers;
        };
        for field in &form.fields {
            let key = state_key(form, field);
            match field.kind {
                FormKind::External => {}
                FormKind::Text if !field.options.is_empty() => {
                    if let Some(value) =
                        state.selections.get(&key).filter(|value| !value.is_empty())
                    {
                        answers.insert(field.key.clone(), serde_json::Value::String(value.clone()));
                    }
                }
                FormKind::Text => {
                    if let Some(input) = state.inputs.get(&key) {
                        let text = input.read(cx).buffer.text();
                        if !text.is_empty() {
                            answers.insert(
                                field.key.clone(),
                                serde_json::Value::String(text.to_string()),
                            );
                        }
                    }
                }
                FormKind::Number => {
                    if let Some(input) = state.inputs.get(&key) {
                        let text = input.read(cx).buffer.text();
                        if let Ok(value) = text.trim().parse::<f64>()
                            && let Some(number) = serde_json::Number::from_f64(value)
                        {
                            answers.insert(field.key.clone(), serde_json::Value::Number(number));
                        }
                    }
                }
                FormKind::Integer => {
                    if let Some(input) = state.inputs.get(&key) {
                        let text = input.read(cx).buffer.text();
                        if let Ok(value) = text.trim().parse::<i64>() {
                            answers
                                .insert(field.key.clone(), serde_json::Value::Number(value.into()));
                        }
                    }
                }
                FormKind::Boolean => {
                    if let Some(value) = state.booleans.get(&key) {
                        answers.insert(field.key.clone(), serde_json::Value::Bool(*value));
                    }
                }
                FormKind::MultiSelect => {
                    if let Some(values) = state.multi.get(&key).filter(|values| !values.is_empty())
                    {
                        answers.insert(
                            field.key.clone(),
                            serde_json::Value::Array(
                                values
                                    .iter()
                                    .map(|value| serde_json::Value::String(value.clone()))
                                    .collect(),
                            ),
                        );
                    }
                }
            }
        }
        answers
    }

    fn field_label(field: &FormField, theme: &Theme) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text)
                    .child(format!(
                        "{}{}",
                        field.title.as_deref().unwrap_or(&field.key),
                        if field.required && field.kind != FormKind::External {
                            " *"
                        } else {
                            ""
                        }
                    )),
            )
            .when_some(field.description.clone(), |label, description| {
                label.child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme.faint)
                        .child(description),
                )
            })
            .into_any_element()
    }

    fn chip(
        id: String,
        label: String,
        active: bool,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = Theme::default();
        let action = Rc::new(action);
        let clicked = action.clone();
        let selector = id.clone();
        div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_selected(active)
            .h(px(24.))
            .px(px(8.))
            .rounded(px(999.))
            .border_1()
            .border_color(if active {
                theme.accent.opacity(0.6)
            } else {
                theme.border
            })
            .bg(if active {
                theme.accent.opacity(0.14)
            } else {
                theme.inset
            })
            .flex()
            .items_center()
            .gap(px(4.))
            .text_size(px(10.5))
            .text_color(if active { theme.text } else { theme.dim })
            .cursor_pointer()
            .hover(|style| style.bg(theme.selected))
            .on_click(cx.listener(move |this, _, _, cx| {
                clicked(this, cx);
                cx.notify();
            }))
            .child(label)
            .into_any_element()
    }

    fn render_field(
        &mut self,
        form: &FormRequest,
        field: &FormField,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let key = state_key(form, field);
        let mut body = div().w_full();
        match field.kind {
            FormKind::Text if !field.options.is_empty() => {
                let selected = self
                    .states
                    .get(&form.id)
                    .and_then(|state| state.selections.get(&key).cloned());
                body = body.flex().flex_wrap().gap(px(5.));
                for option in &field.options {
                    let value = option.value.clone();
                    let key = key.clone();
                    let label = if selected.as_deref() == Some(option.value.as_str()) {
                        format!("✓ {}", option.label)
                    } else {
                        option.label.clone()
                    };
                    body = body.child(Self::chip(
                        format!("{key}:{}", option.value),
                        label,
                        selected.as_deref() == Some(option.value.as_str()),
                        cx,
                        move |this, _| {
                            this.states
                                .entry(form_id_for_state(&key))
                                .or_default()
                                .selections
                                .insert(key.clone(), value.clone());
                        },
                    ));
                }
            }
            FormKind::Text | FormKind::Number | FormKind::Integer => {
                if let Some(input) = self
                    .states
                    .get(&form.id)
                    .and_then(|state| state.inputs.get(&key))
                    .cloned()
                {
                    body = body.child(
                        div()
                            .id(SharedString::from(format!("form-input-wrap-{key}")))
                            .w_full()
                            .p(px(4.))
                            .rounded(px(5.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.inset)
                            .child(input),
                    );
                }
            }
            FormKind::Boolean => {
                let checked = self
                    .states
                    .get(&form.id)
                    .and_then(|state| state.booleans.get(&key))
                    .copied()
                    .unwrap_or(false);
                let form_id = form.id.clone();
                body = body.child(
                    div()
                        .id(SharedString::from(format!("form-bool-{key}")))
                        .debug_selector({
                            let selector = format!("form-bool-{key}");
                            move || selector.clone()
                        })
                        .role(Role::CheckBox)
                        .aria_label(field.title.as_deref().unwrap_or(&field.key))
                        .aria_toggled(if checked {
                            gpui::accesskit::Toggled::True
                        } else {
                            gpui::accesskit::Toggled::False
                        })
                        .h(px(24.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(if checked {
                            theme.accent.opacity(0.14)
                        } else {
                            theme.inset
                        })
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(11.))
                        .text_color(theme.text)
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let value = this
                                .states
                                .entry(form_id.clone())
                                .or_default()
                                .booleans
                                .entry(key.clone())
                                .or_default();
                            *value = !*value;
                            cx.notify();
                        }))
                        .child(if checked { "✓" } else { " " })
                        .child(if checked { "Enabled" } else { "Disabled" }),
                );
            }
            FormKind::MultiSelect => {
                let selected = self
                    .states
                    .get(&form.id)
                    .and_then(|state| state.multi.get(&key))
                    .cloned()
                    .unwrap_or_default();
                body = body.flex().flex_wrap().gap(px(5.));
                for option in &field.options {
                    let value = option.value.clone();
                    let key = key.clone();
                    let active = selected.contains(&option.value);
                    let label = if active {
                        format!("✓ {}", option.label)
                    } else {
                        option.label.clone()
                    };
                    body = body.child(Self::chip(
                        format!("{key}:{}", option.value),
                        label,
                        active,
                        cx,
                        move |this, _| {
                            let set = this
                                .states
                                .entry(form_id_for_state(&key))
                                .or_default()
                                .multi
                                .entry(key.clone())
                                .or_default();
                            if !set.remove(&value) {
                                set.insert(value.clone());
                            }
                        },
                    ));
                }
            }
            FormKind::External => {
                let url = field.url.clone().unwrap_or_default();
                body = body.child(
                    div()
                        .id(SharedString::from(format!("form-external-{key}")))
                        .role(Role::Button)
                        .aria_label("Open external step")
                        .h(px(24.))
                        .px(px(9.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.inset)
                        .flex()
                        .items_center()
                        .text_size(px(10.5))
                        .text_color(theme.accent)
                        .cursor_pointer()
                        .on_click(cx.listener(move |_, _, _, cx| {
                            if !url.is_empty() {
                                cx.emit(Event::OpenExternal(url.clone()));
                            }
                        }))
                        .child("Open external step"),
                );
            }
        }
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(Self::field_label(field, &theme))
            .child(body)
            .into_any_element()
    }
}

fn form_id_for_state(key: &str) -> String {
    key.split_once(':')
        .map(|(form, _)| form.to_owned())
        .unwrap_or_default()
}

impl Render for FormDock {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        if self.forms.is_empty() {
            return div().into_any_element();
        }
        let forms = self.forms.clone();
        let mut dock = div()
            .id("agent-form-dock")
            .role(Role::Region)
            .aria_label("Agent forms")
            .w_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(8.));
        for form in forms {
            let complete = self.complete(&form, cx);
            let submitting = self.submitting.contains(&form.id);
            let mut fields = div().w_full().flex().flex_col().gap(px(8.));
            for field in &form.fields {
                fields = fields.child(self.render_field(&form, field, cx));
            }
            let submit_key = format!("submit:{}", form.id);
            let dismiss_key = format!("dismiss:{}", form.id);
            let submit_selector = format!("form-submit-{}", form.id);
            let dismiss_selector = format!("form-dismiss-{}", form.id);
            let submit_focus = self.focus(submit_key, cx);
            let dismiss_focus = self.focus(dismiss_key, cx);
            let form_for_submit = form.clone();
            let form_id_for_dismiss = form.id.clone();
            let session_for_dismiss = form.session_id.clone();
            dock = dock.child(
                div()
                    .id(SharedString::from(format!("agent-form-{}", form.id)))
                    .w_full()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(theme.border_strong)
                    .bg(theme.elevated)
                    .p(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(form.title.clone()),
                    )
                    .child(fields)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .id(SharedString::from(format!("form-submit-{}", form.id)))
                                    .debug_selector(move || submit_selector.clone())
                                    .role(Role::Button)
                                    .aria_label("Submit form")
                                    .track_focus(&submit_focus)
                                    .h(px(26.))
                                    .px(px(10.))
                                    .rounded(px(6.))
                                    .bg(theme.accent)
                                    .text_size(px(11.))
                                    .text_color(theme.send_ink)
                                    .opacity(if complete && !submitting { 1. } else { 0.45 })
                                    .when(complete && !submitting, |button| {
                                        button.cursor_pointer().on_click(cx.listener(
                                            move |this, _, _, cx| {
                                                if this.submitting.contains(&form_for_submit.id) {
                                                    return;
                                                }
                                                let answers = this.answers(&form_for_submit, cx);
                                                this.submitting.insert(form_for_submit.id.clone());
                                                cx.emit(Event::Submit {
                                                    form_id: form_for_submit.id.clone(),
                                                    session_id: form_for_submit.session_id.clone(),
                                                    answers,
                                                });
                                                cx.notify();
                                            },
                                        ))
                                    })
                                    .child("Submit"),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("form-dismiss-{}", form.id)))
                                    .debug_selector(move || dismiss_selector.clone())
                                    .role(Role::Button)
                                    .aria_label("Dismiss form")
                                    .track_focus(&dismiss_focus)
                                    .h(px(26.))
                                    .px(px(10.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.inset)
                                    .text_size(px(11.))
                                    .text_color(theme.red)
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(Event::Dismiss {
                                            form_id: form_id_for_dismiss.clone(),
                                            session_id: session_for_dismiss.clone(),
                                        });
                                    }))
                                    .child("Dismiss"),
                            ),
                    ),
            );
        }
        dock.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, FormDock, FormField, FormKind};
    use gpui::{
        EntityInputHandler, IntoElement, Modifiers, TestAppContext, VisualTestContext, point, px,
        size,
    };
    use std::{cell::RefCell, rc::Rc};
    use vela_opencode::FormRequest;

    fn field(key: &str, kind: FormKind, required: bool) -> FormField {
        FormField {
            key: key.into(),
            title: Some(key.into()),
            description: None,
            required,
            kind,
            placeholder: None,
            url: None,
            options: Vec::new(),
            default: None,
        }
    }

    #[gpui::test]
    fn required_answers_gate_submission_and_survive_refresh(cx: &mut TestAppContext) {
        let mut text = field("name", FormKind::Text, true);
        text.placeholder = Some("service".into());
        let mut number = field("count", FormKind::Integer, true);
        number.default = Some(serde_json::json!(1));
        let mut multi = field("targets", FormKind::MultiSelect, true);
        multi.options = vec![
            vela_opencode::FormOption {
                value: "a".into(),
                label: "Alpha".into(),
                description: None,
            },
            vela_opencode::FormOption {
                value: "b".into(),
                label: "Beta".into(),
                description: None,
            },
        ];
        let boolean = field("enabled", FormKind::Boolean, true);
        let form = FormRequest {
            id: "form-1".into(),
            session_id: "session-1".into(),
            title: "Deploy".into(),
            fields: vec![text, number, multi, boolean],
        };
        let (dock, cx) = cx.add_window_view(|_, cx| FormDock::new(cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();
        cx.update(|_, cx| {
            cx.subscribe(&dock, move |_, event: &Event, _| match event {
                Event::Submit {
                    form_id,
                    session_id,
                    answers,
                } => captured
                    .borrow_mut()
                    .push(format!("submit:{form_id}:{session_id}:{answers:?}")),
                Event::Dismiss { form_id, .. } => {
                    captured.borrow_mut().push(format!("dismiss:{form_id}"))
                }
                Event::OpenExternal(url) => captured.borrow_mut().push(format!("external:{url}")),
            })
            .detach();
        });
        dock.update(cx, |dock, cx| dock.set_forms(vec![form.clone()], cx));
        cx.simulate_resize(size(px(420.), px(600.)));
        let draw = |cx: &mut VisualTestContext| {
            for _ in 0..3 {
                cx.draw(point(px(0.), px(0.)), size(px(420.), px(600.)), |_, _| {
                    dock.clone().into_any_element()
                });
            }
        };
        draw(cx);
        let submit = cx.debug_bounds("form-submit-form-1").unwrap().center();
        cx.simulate_click(submit, Modifiers::default());
        draw(cx);
        assert!(events.borrow().is_empty());

        let text_input = dock.read_with(cx, |dock, _| {
            dock.states["form-1"].inputs["form-1:name"].clone()
        });
        cx.update(|window, cx| {
            text_input.update(cx, |input, cx| {
                input.replace_text_in_range(None, "api", window, cx)
            })
        });
        let multi_a = cx
            .debug_bounds("form-1:targets:a")
            .expect("multiselect option")
            .center();
        cx.simulate_click(multi_a, Modifiers::default());
        let boolean = cx
            .debug_bounds("form-bool-form-1:enabled")
            .expect("boolean control")
            .center();
        cx.simulate_click(boolean, Modifiers::default());
        draw(cx);
        let submit = cx.debug_bounds("form-submit-form-1").unwrap().center();
        cx.simulate_click(submit, Modifiers::default());
        draw(cx);
        let submitted = events.borrow().join("|");
        assert!(submitted.contains("submit:form-1:"), "{submitted}");
        assert!(
            submitted.contains("\"name\": String(\"api\")"),
            "{submitted}"
        );
        assert!(submitted.contains("\"count\": Number(1)"), "{submitted}");
        assert!(
            submitted.contains("\"targets\": Array [String(\"a\")]"),
            "{submitted}"
        );
        assert!(submitted.contains("\"enabled\": Bool(true)"), "{submitted}");

        // A periodic refresh with identical forms must keep typed answers.
        dock.update(cx, |dock, cx| dock.set_forms(vec![form], cx));
        draw(cx);
        assert_eq!(
            dock.read_with(cx, |dock, cx| {
                dock.states["form-1"].inputs["form-1:name"]
                    .read(cx)
                    .buffer
                    .text()
                    .to_owned()
            }),
            "api"
        );
        let dismiss = cx.debug_bounds("form-dismiss-form-1").unwrap().center();
        cx.simulate_click(dismiss, Modifiers::default());
        draw(cx);
        assert!(
            events
                .borrow()
                .iter()
                .any(|event| event == "dismiss:form-1")
        );
    }
}
