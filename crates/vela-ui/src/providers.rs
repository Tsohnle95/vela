//! Provider UI state belongs to the exact session and workspace that requested it.
use crate::input::{Changed, TextInput};
use crate::scroll::{AutoScrollHandle, FocusReveal, focus_control, focused_control};
use crate::theme::{self, Theme};
use gpui::{AnyElement, FontWeight, SharedString, div, prelude::*, px};
use gpui::{
    App, Context, Entity, EventEmitter, Focusable, KeyDownEvent, MouseButton, Render, Subscription,
    Window,
};
use serde_json::{Map, Value};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use vela_opencode::providers::ProviderIntegration;

const API_KEY_FIELD: &str = "$api-key";

type OpenProviderHandler = Rc<dyn Fn(ProviderIntegration, &mut Window, &mut App)>;
type RemoveProviderHandler = Rc<dyn Fn(RemoveCredential, &mut Window, &mut App)>;
fn numeric_value(text: &str) -> Option<Value> {
    let number = text.parse::<f64>().ok().filter(|value| value.is_finite())?;
    let number = if number.fract() == 0. && number >= i64::MIN as f64 && number <= i64::MAX as f64 {
        serde_json::Number::from(number as i64)
    } else {
        serde_json::Number::from_f64(number)?
    };
    Some(Value::Number(number))
}

fn same_primitive(actual: Option<&Value>, expected: &Value) -> bool {
    match (actual, expected) {
        (Some(Value::Number(actual)), Value::Number(expected)) => {
            actual.as_f64() == expected.as_f64()
        }
        (Some(actual), expected) => actual == expected,
        _ => false,
    }
}
#[derive(Clone)]
pub(crate) struct RemoveCredential {
    pub provider: String,
    pub name: String,
    pub credential: String,
}
pub(crate) struct ProviderHandlers {
    pub open: OpenProviderHandler,
    pub remove: RemoveProviderHandler,
    pub busy: std::collections::HashSet<String>,
    pub errors: HashMap<String, String>,
}
pub(crate) struct PendingRemoval {
    pub scope: ProviderScope,
    pub target: RemoveCredential,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderScope {
    pub session: String,
    pub directory: String,
}
#[derive(Default)]
pub(crate) struct ProviderCatalog {
    scope: Option<ProviderScope>,
    request: Option<u64>,
    pub providers: Vec<ProviderIntegration>,
    pub error: Option<String>,
    pub loading: bool,
    loaded: bool,
}
impl ProviderCatalog {
    pub fn needs_load(&self, scope: &ProviderScope) -> bool {
        self.scope.as_ref() != Some(scope) || (!self.loaded && !self.loading)
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn begin(&mut self, request: u64, scope: ProviderScope) {
        if self.scope.as_ref() != Some(&scope) {
            self.clear();
        }
        self.error = None;
        self.scope = Some(scope);
        self.request = Some(request);
        self.loading = true;
        self.loaded = false;
    }
    pub fn invalidate(&mut self) {
        self.request = None;
        self.loading = false;
        self.loaded = false;
        self.error = None;
    }
    pub fn finish(
        &mut self,
        request: u64,
        current: &ProviderScope,
        providers: Vec<ProviderIntegration>,
        error: Option<String>,
    ) -> bool {
        if self.request != Some(request) || self.scope.as_ref() != Some(current) {
            return false;
        }
        self.request = None;
        self.loading = false;
        self.loaded = true;
        self.providers = providers;
        self.error = error;
        true
    }
}

pub(crate) fn catalog_view(
    catalog: &ProviderCatalog,
    workspace_open: bool,
    width: f32,
    forms: &HashMap<String, ProviderFormHandle>,
    oauth_forms: &HashMap<String, crate::provider_oauth::OAuthHandle>,
    handlers: ProviderHandlers,
    scroll: AutoScrollHandle,
) -> AnyElement {
    let theme = Theme::default();
    let empty = |message: String| {
        div()
            .w_full()
            .px(px(20.))
            .py(px(32.))
            .text_center()
            .text_size(px(12.))
            .text_color(theme.faint)
            .whitespace_normal()
            .child(SharedString::from(message))
            .into_any_element()
    };
    if !workspace_open {
        return empty("Open a workspace to connect model providers.".into());
    }
    let mut content = div().w_full().min_w_0().flex().flex_col().gap(px(16.))
        .child(div().w_full().min_w_0().flex().flex_col().gap(px(3.))
            .child(div().text_size(px(13.)).font_weight(FontWeight::BOLD).child("Bring your own provider"))
            .child(div().text_size(px(10.5)).text_color(theme.faint).whitespace_normal()
                .child("Keys are stored by the active agent runtime and are never displayed again.")));
    if let Some(error) = &catalog.error {
        content = content.child(
            div()
                .w_full()
                .min_w_0()
                .p(px(26.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.panel)
                .text_color(theme.red)
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::BOLD)
                        .child("Providers unavailable"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .whitespace_normal()
                        .child(SharedString::from(error.clone())),
                ),
        );
    }
    if catalog.loading && catalog.providers.is_empty() {
        return content
            .child(empty("Loading providers...".into()))
            .into_any_element();
    }
    let mut grid = div()
        .w_full()
        .min_w_0()
        .grid()
        .grid_cols(if width <= 760. { 1 } else { 2 })
        .gap(px(10.));
    for provider in &catalog.providers {
        let connected = provider.connected();
        let busy = handlers.busy.contains(&provider.id);
        let mut card = div()
            .w_full()
            .min_w_0()
            .p(px(14.))
            .rounded(px(14.))
            .border_1()
            .border_color(if connected {
                theme.green.opacity(0.45)
            } else {
                theme.border
            })
            .bg(theme.panel)
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(32.))
                            .flex_shrink_0()
                            .rounded(px(10.))
                            .border_1()
                            .border_color(theme.accent.opacity(0.3))
                            .bg(theme.accent.opacity(0.18))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme.accent)
                            .text_size(px(10.))
                            .font_family(theme::FONT_MONO)
                            .font_weight(FontWeight::BOLD)
                            .child(SharedString::from(
                                provider
                                    .name
                                    .chars()
                                    .take(2)
                                    .collect::<String>()
                                    .to_uppercase(),
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::BOLD)
                                    .truncate()
                                    .child(SharedString::from(provider.name.clone())),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(9.))
                                    .font_family(theme::FONT_MONO)
                                    .text_color(theme.faint)
                                    .truncate()
                                    .child(SharedString::from(provider.id.clone())),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .px(px(8.))
                            .py(px(4.))
                            .rounded(px(999.))
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(9.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(if connected { theme.green } else { theme.dim })
                            .child(if connected { "CONNECTED" } else { "AVAILABLE" }),
                    ),
            );
        if !provider.credentials.is_empty() {
            let mut credentials = div()
                .mt(px(12.))
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(5.));
            for credential in &provider.credentials {
                let remove = handlers.remove.clone();
                let remove_key = remove.clone();
                let target = RemoveCredential {
                    provider: provider.id.clone(),
                    name: provider.name.clone(),
                    credential: credential.id.clone(),
                };
                let key_target = target.clone();
                credentials = credentials.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .px(px(9.))
                        .py(px(7.))
                        .rounded(px(12.))
                        .bg(theme.inset)
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .child(
                            div()
                                .size(px(6.))
                                .flex_shrink_0()
                                .rounded(px(99.))
                                .bg(theme.green),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(10.5))
                                .child(SharedString::from(credential.label.clone())),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("remove-{}", credential.id)))
                                .role(gpui::Role::Button)
                                .aria_label(SharedString::from(format!(
                                    "Remove {} credential: {}",
                                    provider.name, credential.label
                                )))
                                .focusable()
                                .tab_stop(!busy)
                                .border_1()
                                .border_color(theme.border_strong)
                                .rounded(px(999.))
                                .px(px(7.))
                                .py(px(3.))
                                .text_size(px(9.))
                                .text_color(theme.dim)
                                .opacity(if busy { 0.5 } else { 1. })
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    if !busy {
                                        remove(target.clone(), window, cx);
                                    }
                                    cx.stop_propagation();
                                })
                                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key == "enter"
                                        || event.keystroke.key == "space"
                                    {
                                        if !busy {
                                            remove_key(key_target.clone(), window, cx);
                                        }
                                        cx.stop_propagation();
                                    }
                                })
                                .child("Remove")
                                .map(|control| {
                                    focus_control(
                                        SharedString::from(format!(
                                            "reveal-remove-{}",
                                            credential.id
                                        )),
                                        scroll.clone(),
                                        !busy,
                                        control,
                                    )
                                }),
                        ),
                );
            }
            card = card.child(credentials);
        }
        if !provider.environment.connected.is_empty() {
            card = card.child(
                div()
                    .mt(px(9.))
                    .text_size(px(10.))
                    .text_color(theme.faint)
                    .whitespace_normal()
                    .child(SharedString::from(format!(
                        "Connected from {}",
                        provider.environment.connected.join(", ")
                    ))),
            );
        }
        if let Some(form) = forms.get(&provider.id) {
            grid = grid.child(card.child(form.form.clone()));
            continue;
        }
        if let Some(form) = oauth_forms.get(&provider.id) {
            grid = grid.child(card.child(form.form.clone()));
            continue;
        }
        let note = if !provider.environment.names.is_empty() {
            format!("Also supports {}", provider.environment.names.join(", "))
        } else if !provider.oauth.is_empty() {
            "Sign-in available through the provider".into()
        } else {
            "API key connection".into()
        };
        card = card.child(
            div()
                .mt_auto()
                .pt(px(14.))
                .w_full()
                .min_w_0()
                .flex()
                .items_end()
                .justify_between()
                .gap(px(12.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(9.))
                        .line_height(gpui::relative(1.4))
                        .text_color(theme.faint)
                        .child(SharedString::from(note)),
                )
                .when(
                    provider.key_method.is_some() || !provider.oauth.is_empty(),
                    |footer| {
                        let provider = provider.clone();
                        let has_key = provider.key_method.is_some();
                        let aria_label = if has_key {
                            format!("Add {} key", provider.name)
                        } else {
                            format!("Sign in to {}", provider.name)
                        };
                        let on_open = handlers.open.clone();
                        let key_open = on_open.clone();
                        let key_provider = provider.clone();
                        let reveal_id =
                            SharedString::from(format!("reveal-connect-{}", provider.id));
                        footer.child(
                            div()
                                .id(SharedString::from(format!("connect-{}", provider.id)))
                                .role(gpui::Role::Button)
                                .aria_label(aria_label)
                                .focusable()
                                .tab_stop(true)
                                .flex_shrink_0()
                                .px(px(9.))
                                .py(px(5.))
                                .border_1()
                                .border_color(theme.border_strong)
                                .rounded(px(999.))
                                .text_size(px(13.3333))
                                .text_color(theme.accent)
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    on_open(provider.clone(), window, cx)
                                })
                                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key == "enter"
                                        || event.keystroke.key == "space"
                                    {
                                        key_open(key_provider.clone(), window, cx);
                                        cx.stop_propagation();
                                    }
                                })
                                .child(if has_key { "Add key" } else { "Sign in" })
                                .map(|control| {
                                    focus_control(reveal_id, scroll.clone(), true, control)
                                }),
                        )
                    },
                ),
        );
        if let Some(error) = handlers.errors.get(&provider.id) {
            card = card.child(
                div()
                    .mt(px(8.))
                    .text_size(px(10.5))
                    .text_color(theme.red)
                    .whitespace_normal()
                    .child(SharedString::from(error.clone())),
            );
        }
        grid = grid.child(card);
    }
    content = content.child(grid);
    if catalog.providers.is_empty() && !catalog.loading {
        content = content.child(empty("No providers are available for this runtime.".into()));
    }
    content.into_any_element()
}

pub(crate) struct ProviderFormHandle {
    pub form: Entity<ProviderForm>,
    pub _subscription: Subscription,
}
pub(crate) struct ProviderPending {
    pub scope: ProviderScope,
    pub provider_id: String,
    pub form: gpui::WeakEntity<ProviderForm>,
}
pub(crate) enum ProviderFormEvent {
    Cancel,
    Submit {
        key: String,
        label: String,
        answers: Map<String, Value>,
    },
}
pub(crate) struct ProviderForm {
    provider: ProviderIntegration,
    scroll: AutoScrollHandle,
    key: Entity<TextInput>,
    label: Entity<TextInput>,
    inputs: HashMap<String, Entity<TextInput>>,
    values: Map<String, Value>,
    field_focus: HashMap<String, gpui::FocusHandle>,
    selections: HashMap<String, crate::multiple_selection::MultipleSelection>,
    open_options: Option<String>,
    option_cursor: usize,
    option_bounds: HashMap<String, Rc<Cell<Option<gpui::Bounds<gpui::Pixels>>>>>,
    busy: bool,
    error: Option<String>,
    error_field: Option<String>,
    invalid_focus: Option<(gpui::FocusHandle, Option<gpui::Bounds<gpui::Pixels>>)>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ProviderFormEvent> for ProviderForm {}
impl ProviderForm {
    pub fn busy(&self) -> bool {
        self.busy
    }
    pub fn clear_credentials(&mut self, cx: &mut Context<Self>) {
        self.key
            .update(cx, |input, cx| input.set_text(String::new(), cx));
        self.label
            .update(cx, |input, cx| input.set_text(String::new(), cx));
        for input in self.inputs.values() {
            input.update(cx, |input, cx| input.set_text(String::new(), cx));
        }
        self.values.clear();
        self.error = None;
        self.error_field = None;
        cx.notify();
    }
    pub fn new(
        provider: ProviderIntegration,
        scroll: AutoScrollHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        use vela_opencode::providers::FieldKind;
        let key_label = provider
            .key_method
            .as_ref()
            .and_then(|method| method.label.clone())
            .unwrap_or_else(|| "API key".into());
        let key = cx.new(|cx| {
            let mut input = TextInput::password(key_label, cx);
            input.use_settings_field();
            input
        });
        let label = cx.new(|cx| {
            let mut input = TextInput::new(false, "Work, personal, team...", cx);
            input.set_label("Label optional");
            input.use_settings_field();
            input
        });
        let mut subscriptions = vec![
            cx.subscribe(&key, |this, _, _: &Changed, cx| {
                this.error_field = None;
                cx.notify();
            }),
            cx.subscribe(&label, |this, _, _: &Changed, cx| {
                this.error_field = None;
                cx.notify();
            }),
        ];
        let mut values = Map::new();
        let mut inputs = HashMap::new();
        let mut option_bounds = HashMap::new();
        for field in provider.key_method.iter().flat_map(|method| &method.fields) {
            if field.options.is_some() || field.kind == FieldKind::Multiselect {
                option_bounds.insert(field.key.clone(), Rc::new(Cell::new(None)));
            }
            if let Some(default) = &field.default {
                values.insert(field.key.clone(), default.clone());
            }
            if field.options.is_none()
                && matches!(
                    field.kind,
                    FieldKind::String | FieldKind::Number | FieldKind::Integer
                )
            {
                let input = cx.new(|cx| {
                    let mut input =
                        TextInput::new(false, field.placeholder.clone().unwrap_or_default(), cx);
                    input.set_label(format!(
                        "{}{}",
                        field.title.as_deref().unwrap_or(&field.key),
                        if field.required == Some(true) {
                            " *"
                        } else {
                            ""
                        }
                    ));
                    input.use_settings_field();
                    if field.kind == FieldKind::String {
                        input.max_utf16_length = field
                            .max_length
                            .and_then(|length| usize::try_from(length).ok());
                    }
                    if let Some(value) = &field.default {
                        input.set_text(
                            value
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| value.to_string()),
                            cx,
                        );
                    }
                    input
                });
                let id = field.key.clone();
                let numeric = field.kind != FieldKind::String;
                subscriptions.push(cx.subscribe(&input, move |this, _, event: &Changed, cx| {
                    let value = if numeric && !event.0.is_empty() {
                        numeric_value(&event.0).unwrap_or_else(|| Value::String(event.0.clone()))
                    } else {
                        Value::String(event.0.clone())
                    };
                    this.values.insert(id.clone(), value);
                    this.error_field = None;
                    cx.notify();
                }));
                inputs.insert(field.key.clone(), input);
            }
        }
        let field_focus = provider
            .key_method
            .iter()
            .flat_map(|method| &method.fields)
            .map(|field| {
                let focus = inputs
                    .get(&field.key)
                    .map(|input| input.focus_handle(cx))
                    .unwrap_or_else(|| cx.focus_handle().tab_stop(true));
                (field.key.clone(), focus)
            })
            .collect();
        Self {
            provider,
            scroll,
            key,
            label,
            inputs,
            values,
            field_focus,
            selections: HashMap::new(),
            open_options: None,
            option_cursor: 0,
            option_bounds,
            busy: false,
            error: None,
            error_field: None,
            invalid_focus: None,
            _subscriptions: subscriptions,
        }
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) -> Option<gpui::Bounds<gpui::Pixels>> {
        let bounds = self.key.read(cx).outer_bounds();
        window.focus(&self.key.focus_handle(cx), cx);
        bounds
    }
    pub fn set_result(&mut self, error: Option<String>, cx: &mut Context<Self>) {
        self.busy = false;
        self.error = error;
        self.error_field = None;
        cx.notify();
    }
    pub fn set_busy(&mut self, cx: &mut Context<Self>) {
        self.busy = true;
        self.error = None;
        self.error_field = None;
        cx.notify();
    }
    fn field_visible(&self, field: &vela_opencode::providers::ProviderField) -> bool {
        field.when.iter().all(|condition| {
            let equal = same_primitive(self.values.get(&condition.key), &condition.value);
            match condition.op {
                vela_opencode::providers::ConditionOp::Eq => equal,
                vela_opencode::providers::ConditionOp::Neq => !equal,
            }
        })
    }
    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let key = self.key.read(cx).buffer.text().to_owned();
        if key.is_empty() {
            self.error = Some(crate::provider_validation::Invalid::Required.message());
            self.error_field = Some(API_KEY_FIELD.into());
            self.invalid_focus =
                Some((self.key.focus_handle(cx), self.key.read(cx).outer_bounds()));
            cx.notify();
            return;
        }
        let mut answers = Map::new();
        for field in self
            .provider
            .key_method
            .iter()
            .flat_map(|method| &method.fields)
        {
            if !self.field_visible(field)
                || field.kind == vela_opencode::providers::FieldKind::External
            {
                continue;
            }
            let input = self.inputs.get(&field.key).map(|input| input.read(cx));
            if let Some(invalid) = crate::provider_validation::validate(
                field,
                self.values.get(&field.key),
                input.map(|input| (input.buffer.text(), input.user_edited)),
            ) {
                self.error = Some(invalid.message());
                self.error_field = Some(field.key.clone());
                self.invalid_focus = self.field_focus.get(&field.key).map(|focus| {
                    (
                        focus.clone(),
                        self.inputs
                            .get(&field.key)
                            .and_then(|input| input.read(cx).outer_bounds())
                            .or_else(|| {
                                self.option_bounds
                                    .get(&field.key)
                                    .and_then(|bounds| bounds.get())
                            }),
                    )
                });
                cx.notify();
                return;
            }
            if let Some(value) = self.values.get(&field.key).filter(|value| {
                **value != Value::String(String::new())
                    && !value.as_array().is_some_and(Vec::is_empty)
            }) {
                answers.insert(field.key.clone(), value.clone());
            }
        }
        self.busy = true;
        self.error = None;
        self.error_field = None;
        cx.notify();
        cx.emit(ProviderFormEvent::Submit {
            key,
            label: self.label.read(cx).buffer.text().to_owned(),
            answers,
        });
    }
    fn choose_multiple(
        &mut self,
        id: &str,
        index: usize,
        shift: bool,
        additive: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(options) = self
            .provider
            .key_method
            .iter()
            .flat_map(|method| &method.fields)
            .find(|field| field.key == id)
            .and_then(|field| field.options.as_ref())
        else {
            return;
        };
        if index >= options.len() {
            return;
        }
        let values = self.values.get(id).and_then(Value::as_array);
        let mut selected = options
            .iter()
            .enumerate()
            .filter_map(|(index, option)| {
                values
                    .is_some_and(|values| values.contains(&Value::String(option.value.clone())))
                    .then_some(index)
            })
            .collect();
        self.selections.entry(id.to_owned()).or_default().choose(
            &mut selected,
            index,
            shift,
            additive,
        );
        self.values.insert(
            id.to_owned(),
            Value::Array(
                selected
                    .into_iter()
                    .map(|index| Value::String(options[index].value.clone()))
                    .collect(),
            ),
        );
        cx.notify();
    }
    fn multiple_key(&mut self, id: &str, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(options) = self
            .provider
            .key_method
            .iter()
            .flat_map(|method| &method.fields)
            .find(|field| field.key == id)
            .and_then(|field| field.options.as_ref())
        else {
            return;
        };
        if options.is_empty() {
            return;
        }
        let key = &event.keystroke;
        if key.key == "a" && (key.modifiers.platform || key.modifiers.control) {
            self.values.insert(
                id.to_owned(),
                Value::Array(
                    options
                        .iter()
                        .map(|option| Value::String(option.value.clone()))
                        .collect(),
                ),
            );
            cx.notify();
            cx.stop_propagation();
            return;
        }
        let active = self
            .selections
            .get(id)
            .and_then(|state| state.active)
            .or_else(|| {
                options.iter().position(|option| {
                    self.values
                        .get(id)
                        .and_then(Value::as_array)
                        .is_some_and(|values| values.contains(&Value::String(option.value.clone())))
                })
            })
            .unwrap_or(0);
        if key.key == "space" {
            self.choose_multiple(id, active, false, true, cx);
            cx.stop_propagation();
            return;
        }
        let command = key.modifiers.platform || key.modifiers.control;
        let next = match key.key.as_str() {
            "up" if command => 0,
            "down" if command => options.len() - 1,
            "up" => active.saturating_sub(1),
            "down" => (active + 1).min(options.len() - 1),
            "home" => 0,
            "end" => options.len() - 1,
            _ => return,
        };
        if key.modifiers.shift {
            self.choose_multiple(id, next, true, false, cx);
        } else {
            self.selections
                .entry(id.to_owned())
                .or_default()
                .focus(next);
            cx.notify();
        }
        cx.stop_propagation();
    }
    fn single_option_key(&mut self, id: &str, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(options) = self
            .provider
            .key_method
            .iter()
            .flat_map(|method| &method.fields)
            .find(|field| field.key == id)
            .and_then(|field| field.options.clone())
        else {
            return;
        };
        let command = event.keystroke.modifiers.platform || event.keystroke.modifiers.control;
        match event.keystroke.key.as_str() {
            "escape" => self.open_options = None,
            "up" if command => self.option_cursor = 0,
            "down" if command => self.option_cursor = options.len(),
            "up" => self.option_cursor = self.option_cursor.saturating_sub(1),
            "down" => self.option_cursor = (self.option_cursor + 1).min(options.len()),
            "home" => self.option_cursor = 0,
            "end" => self.option_cursor = options.len(),
            "enter" | "space" => {
                let value = self
                    .option_cursor
                    .checked_sub(1)
                    .and_then(|index| options.get(index))
                    .map(|option| option.value.clone())
                    .unwrap_or_default();
                self.values.insert(id.to_owned(), Value::String(value));
                self.open_options = None;
                self.error_field = None;
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn open_field_options(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        if let Some(bounds) = self.option_bounds.get(&id).and_then(|bounds| bounds.get())
            && let Some(field) = self
                .provider
                .key_method
                .iter()
                .flat_map(|method| &method.fields)
                .find(|field| field.key == id)
        {
            let options = field.options.clone().unwrap_or_default();
            let mut labels = vec!["Select".to_owned()];
            labels.extend(options.iter().map(|option| option.label.clone()));
            let selected = options
                .iter()
                .position(|option| {
                    self.values.get(&id).and_then(Value::as_str) == Some(option.value.as_str())
                })
                .map_or(0, |index| index + 1);
            if let Some(menu) =
                crate::platform_menu::Popup::new(window, bounds, &labels, selected, 11.)
            {
                cx.spawn(async move |this, cx| {
                    if let Some(index) = menu.show() {
                        let _ = this.update(cx, |this, cx| {
                            let value = if index == 0 {
                                Some(String::new())
                            } else {
                                options.get(index - 1).map(|option| option.value.clone())
                            };
                            if let Some(value) = value {
                                this.values.insert(id, Value::String(value));
                                cx.notify();
                            }
                        });
                    }
                })
                .detach();
                return;
            }
        }
        self.open_options = if self.open_options.as_deref() == Some(&id) {
            None
        } else {
            let selected = self
                .provider
                .key_method
                .iter()
                .flat_map(|method| &method.fields)
                .find(|field| field.key == id)
                .and_then(|field| field.options.as_ref())
                .and_then(|options| {
                    options.iter().position(|option| {
                        self.values.get(&id).and_then(Value::as_str) == Some(option.value.as_str())
                    })
                });
            self.option_cursor = selected.map_or(0, |index| index + 1);
            Some(id)
        };
        cx.notify();
    }
    fn step_number(
        &mut self,
        field: vela_opencode::providers::ProviderField,
        direction: i8,
        cx: &mut Context<Self>,
    ) {
        let Some(input) = self.inputs.get(&field.key).cloned() else {
            return;
        };
        let current = input.read(cx).buffer.text().to_owned();
        let next = crate::provider_validation::stepped_text(&field, &current, direction);
        if next == current {
            return;
        }
        input.update(cx, |input, cx| input.set_text(next.clone(), cx));
        let value = if field.kind == vela_opencode::providers::FieldKind::Integer {
            next.parse::<i64>()
                .ok()
                .map(|number| Value::Number(number.into()))
                .or_else(|| numeric_value(&next))
        } else {
            numeric_value(&next)
        }
        .unwrap_or_else(|| Value::String(next));
        self.values.insert(field.key, value);
        self.error = None;
        self.error_field = None;
        cx.notify();
    }

    fn number_stepper(
        &self,
        field: &vela_opencode::providers::ProviderField,
        direction: i8,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use vela_opencode::providers::FieldKind;
        let theme = Theme::default();
        let current = self
            .inputs
            .get(&field.key)
            .map(|input| input.read(cx).buffer.text().to_owned())
            .unwrap_or_default();
        let disabled =
            crate::provider_validation::stepped_text(field, &current, direction) == current;
        let forward = direction > 0;
        let id = format!(
            "provider-number-step-{}-{}",
            field.key,
            if forward { "up" } else { "down" }
        );
        let selector = id.clone();
        let field_value = field.clone();
        let numeric = matches!(field.kind, FieldKind::Number | FieldKind::Integer);
        let mut button = div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .role(gpui::Role::Button)
            .aria_label(format!(
                "{} {}",
                if forward { "Increase" } else { "Decrease" },
                field.title.as_deref().unwrap_or(&field.key)
            ))
            .size(px(22.))
            .flex_shrink_0()
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border_strong)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .text_color(theme.dim)
            .opacity(if numeric && disabled { 0.4 } else { 1. });
        if numeric && !disabled {
            button = button
                .cursor_pointer()
                .hover(|style| style.bg(theme.selected).text_color(theme.text))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.step_number(field_value.clone(), direction, cx);
                        cx.stop_propagation();
                    }),
                );
        }
        button
            .child(if forward { "+" } else { "−" })
            .into_any_element()
    }

    fn field_control(
        &self,
        field: &vela_opencode::providers::ProviderField,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use vela_opencode::providers::FieldKind;
        let theme = Theme::default();
        let id = field.key.clone();
        let focus = self.field_focus[&id].clone();
        match field.kind {
            FieldKind::External => {
                let Some(url) = field.url.clone() else {
                    return div().into_any_element();
                };
                let key_url = url.clone();
                div()
                    .id(SharedString::from(format!("external-{id}")))
                    .role(gpui::Role::Link)
                    .aria_label("Open setup instructions")
                    .text_size(px(10.5))
                    .text_color(theme.accent)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        if url != "about:blank" && vela_browser::allowed_url(&url) {
                            cx.open_url(&url);
                        }
                    })
                    .on_key_down(move |event: &KeyDownEvent, _, cx| {
                        if event.keystroke.key == "enter" {
                            if key_url != "about:blank" && vela_browser::allowed_url(&key_url) {
                                cx.open_url(&key_url);
                            }
                            cx.stop_propagation();
                        }
                    })
                    .child("Open setup instructions")
                    .map(|control| {
                        focused_control(
                            SharedString::from(format!("reveal-external-{id}")),
                            self.scroll.clone(),
                            focus.clone(),
                            control,
                        )
                    })
                    .into_any_element()
            }
            FieldKind::Boolean => {
                let checked = self.values.get(&id) == Some(&Value::Bool(true));
                let key_id = id.clone();
                let reveal_id = SharedString::from(format!("reveal-check-{id}"));
                div()
                    .id(SharedString::from(format!("check-{id}")))
                    .role(gpui::Role::CheckBox)
                    .aria_label(field.title.clone().unwrap_or_else(|| id.clone()))
                    .aria_toggled(if checked {
                        gpui::accesskit::Toggled::True
                    } else {
                        gpui::accesskit::Toggled::False
                    })
                    .self_center()
                    .my(px(3.))
                    .ml(px(1.))
                    .size(px(13.))
                    .rounded(px(2.))
                    .border_1()
                    .border_color(gpui::rgb(if theme.dark { 0x858585 } else { 0x767676 }))
                    .cursor_pointer()
                    .bg(if checked {
                        gpui::rgb(0x0075ff)
                    } else {
                        gpui::rgb(if theme.dark { 0x3b3b3b } else { 0xffffff })
                    })
                    .text_color(gpui::white())
                    .text_size(px(11.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.values.insert(id.clone(), Value::Bool(!checked));
                            cx.notify();
                        }),
                    )
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if event.keystroke.key == "space" {
                            this.values.insert(key_id.clone(), Value::Bool(!checked));
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }))
                    .child(if checked { "✓" } else { "" })
                    .map(|control| {
                        focused_control(reveal_id, self.scroll.clone(), focus.clone(), control)
                    })
                    .into_any_element()
            }
            _ if field.options.is_some() || field.kind == FieldKind::Multiselect => {
                let options = field.options.clone().unwrap_or_default();
                let multiple = field.kind == FieldKind::Multiselect;
                let selected = self.values.get(&id).cloned().unwrap_or(Value::Null);
                let (selected_background, selected_text) = crate::platform_menu::selection_colors(
                    focus.is_focused(window) && window.is_window_active(),
                );
                let selected_label = options
                    .iter()
                    .find(|o| selected.as_str() == Some(o.value.as_str()))
                    .map(|o| o.label.clone())
                    .unwrap_or_else(|| "Select".into());
                let cursor = self.option_cursor;
                let mut choices = div()
                    .id(SharedString::from(format!("choices-{id}")))
                    .debug_selector({
                        let selector = format!("provider-options-{id}");
                        move || selector.clone()
                    })
                    .role(gpui::Role::ListBox)
                    .w_full()
                    .max_h(px(180.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .bg(theme.inset);
                if !multiple {
                    let key_id = id.clone();
                    let placeholder_selected = selected.as_str().is_none_or(str::is_empty);
                    let placeholder_selector = format!("option-{id}-__select__");
                    choices = choices.child(
                        div()
                            .id(SharedString::from(format!("option-{id}-__select__")))
                            .debug_selector(move || placeholder_selector.clone())
                            .role(gpui::Role::ListBoxOption)
                            .aria_label("Select")
                            .aria_selected(placeholder_selected)
                            .w_full()
                            .px(px(6.))
                            .py(px(2.))
                            .text_size(px(11.))
                            .bg(if cursor == 0 {
                                selected_background
                            } else {
                                theme.inset
                            })
                            .text_color(if cursor == 0 {
                                selected_text
                            } else {
                                theme.text
                            })
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.values
                                        .insert(key_id.clone(), Value::String(String::new()));
                                    this.open_options = None;
                                    this.error_field = None;
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .child("Select"),
                    );
                }
                for (index, option) in options.into_iter().enumerate() {
                    let selected = if multiple {
                        selected
                            .as_array()
                            .is_some_and(|a| a.contains(&Value::String(option.value.clone())))
                    } else {
                        selected.as_str() == Some(option.value.as_str())
                    };
                    let row = index + usize::from(!multiple);
                    let id = id.clone();
                    let option_selector = format!("option-{id}-{}", option.value);
                    choices = choices.child(
                        div()
                            .id(SharedString::from(format!("option-{id}-{}", option.value)))
                            .debug_selector(move || option_selector.clone())
                            .role(gpui::Role::ListBoxOption)
                            .aria_label(option.label.clone())
                            .aria_selected(selected)
                            .w_full()
                            .px(px(if multiple { 0. } else { 6. }))
                            .py(px(if multiple { 0. } else { 2. }))
                            .text_size(px(11.))
                            .bg(if (!multiple && cursor == row) || selected {
                                selected_background
                            } else {
                                theme.inset
                            })
                            .text_color(if selected { selected_text } else { theme.text })
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(
                                    move |this, event: &gpui::MouseDownEvent, window, cx| {
                                        if multiple {
                                            window.focus(&this.field_focus[&id], cx);
                                            let additive = if cfg!(target_os = "macos") {
                                                event.modifiers.platform
                                            } else {
                                                event.modifiers.control
                                            };
                                            this.choose_multiple(
                                                &id,
                                                index,
                                                event.modifiers.shift,
                                                additive,
                                                cx,
                                            );
                                        } else {
                                            this.values.insert(
                                                id.clone(),
                                                Value::String(option.value.clone()),
                                            );
                                            this.open_options = None;
                                            this.error_field = None;
                                        }
                                        cx.stop_propagation();
                                        cx.notify();
                                    },
                                ),
                            )
                            .child(SharedString::from(option.label)),
                    );
                }
                if multiple {
                    let key_id = id.clone();
                    return choices
                        .role(gpui::Role::ListBox)
                        .aria_label(field.title.clone().unwrap_or_else(|| id.clone()))
                        .h(px(72.))
                        .px(px(9.))
                        .py(px(7.))
                        .border_1()
                        .border_color(theme.border_strong)
                        .rounded(px(12.))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                            this.multiple_key(&key_id, event, cx)
                        }))
                        .map(|control| {
                            focused_control(
                                SharedString::from(format!("reveal-multiple-{id}")),
                                self.scroll.clone(),
                                focus.clone(),
                                control,
                            )
                        })
                        .into_any_element();
                }
                let expanded = self.open_options.as_deref() == Some(&id);
                let bounds = self
                    .option_bounds
                    .get(&id)
                    .cloned()
                    .expect("option field has bounds");
                let key_id = id.clone();
                let reveal_id = SharedString::from(format!("reveal-select-{id}"));
                div()
                    .id(SharedString::from(format!("select-{id}")))
                    .debug_selector({
                        let selector = format!("provider-select-{id}");
                        move || selector.clone()
                    })
                    .role(gpui::Role::ComboBox)
                    .aria_label(format!(
                        "{}{}",
                        field.title.as_deref().unwrap_or(&id),
                        if field.required == Some(true) {
                            " *"
                        } else {
                            ""
                        }
                    ))
                    .aria_value(selected_label.clone())
                    .aria_expanded(expanded)
                    .relative()
                    .w_full()
                    .px(px(9.))
                    .py(px(7.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(theme.border_strong)
                    .bg(theme.inset)
                    .text_size(px(11.))
                    .text_color(theme.text)
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.open_field_options(id.clone(), window, cx)
                        }),
                    )
                    .on_key_down(cx.listener({
                        let key_id = key_id.clone();
                        move |this, event: &KeyDownEvent, window, cx| {
                            if this.open_options.as_deref() == Some(key_id.as_str()) {
                                this.single_option_key(&key_id, event, cx);
                                return;
                            }
                            if matches!(
                                event.keystroke.key.as_str(),
                                "enter" | "space" | "up" | "down"
                            ) {
                                this.open_field_options(key_id.clone(), window, cx);
                                cx.stop_propagation();
                            }
                        }
                    }))
                    .child(SharedString::from(selected_label))
                    .child(
                        gpui::canvas(
                            move |rect, _, _| bounds.set(Some(rect.dilate(px(1.)))),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(select_chevron(theme.text))
                    .when(expanded, |control| {
                        let single_id = key_id.clone();
                        let choices = choices.on_key_down(cx.listener(
                            move |this, event: &KeyDownEvent, _, cx| {
                                this.single_option_key(&single_id, event, cx)
                            },
                        ));
                        control.child(
                            choices
                                .absolute()
                                .left_0()
                                .top(gpui::relative(1.))
                                .border_1()
                                .border_color(theme.border_strong)
                                .rounded(px(8.)),
                        )
                    })
                    .map(|control| {
                        focused_control(reveal_id, self.scroll.clone(), focus.clone(), control)
                    })
                    .into_any_element()
            }
            _ => {
                let Some(input) = self.inputs.get(&id).cloned() else {
                    return div().into_any_element();
                };
                let boxed = input_box(input, self.scroll.clone(), cx);
                if !matches!(field.kind, FieldKind::Number | FieldKind::Integer) {
                    return boxed;
                }
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(div().flex_1().min_w_0().child(boxed))
                    .child(self.number_stepper(field, -1, cx))
                    .child(self.number_stepper(field, 1, cx))
                    .into_any_element()
            }
        }
    }
}
pub(crate) fn select_chevron(color: gpui::Rgba) -> AnyElement {
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut path = gpui::PathBuilder::stroke(px(1.));
            let at = |x, y| bounds.origin + gpui::point(px(x), px(y));
            path.move_to(at(1., 1.));
            path.line_to(at(5., 5.));
            path.line_to(at(9., 1.));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .absolute()
    .right(px(4.))
    .top(gpui::relative(0.5))
    .mt(px(-3.))
    .w(px(10.))
    .h(px(6.))
    .into_any_element()
}
fn validation_bubble(id: &str, message: String) -> AnyElement {
    let theme = Theme::default();
    let selector = format!("provider-validation-bubble-{id}");
    div()
        .id(SharedString::from(format!("provider-validation-{id}")))
        .debug_selector(move || selector.clone())
        .role(gpui::Role::Label)
        .aria_label(SharedString::from(message.clone()))
        .whitespace_normal()
        .text_size(px(10.5))
        .text_color(theme.red)
        .child(SharedString::from(message))
        .into_any_element()
}

fn input_box(input: Entity<TextInput>, scroll: AutoScrollHandle, cx: &App) -> AnyElement {
    FocusReveal::input(
        SharedString::from(format!("reveal-input-{:?}", input.entity_id())),
        scroll,
        input.focus_handle(cx),
        input,
    )
    .into_any_element()
}
impl Render for ProviderForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        if let Some((focus, bounds)) = self.invalid_focus.take() {
            let scroll = self.scroll.clone();
            window.on_next_frame(move |window, cx| {
                window.focus(&focus, cx);
                if let Some(bounds) = bounds {
                    scroll.reveal(bounds);
                    window.refresh();
                }
            });
        }
        let theme = Theme::default();
        let key_label = self
            .provider
            .key_method
            .as_ref()
            .and_then(|m| m.label.clone())
            .unwrap_or_else(|| "API key".into());
        let mut form = div()
            .id("provider-key-form")
            .w_full()
            .min_w_0()
            .mt(px(13.))
            .pt(px(12.))
            .border_t_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap(px(9.))
            .text_color(theme.dim)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "enter" {
                    if this.key.read(cx).buffer.marked.is_some()
                        || this.label.read(cx).buffer.marked.is_some()
                        || this
                            .inputs
                            .values()
                            .any(|input| input.read(cx).buffer.marked.is_some())
                    {
                        return;
                    }
                    this.submit(cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SharedString::from(key_label)),
                    )
                    .child(input_box(self.key.clone(), self.scroll.clone(), cx))
                    .when(
                        self.error_field.as_deref() == Some(API_KEY_FIELD),
                        |block| {
                            block.child(validation_bubble(
                                "api-key",
                                self.error
                                    .clone()
                                    .unwrap_or_else(|| "Please fill out this field.".into()),
                            ))
                        },
                    ),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(3.))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Label"),
                            )
                            .child(
                                div()
                                    .text_size(px(8.33))
                                    .text_color(theme.faint)
                                    .child("optional"),
                            ),
                    )
                    .child(input_box(self.label.clone(), self.scroll.clone(), cx)),
            );
        for field in self
            .provider
            .key_method
            .as_ref()
            .map(|m| m.fields.clone())
            .unwrap_or_default()
        {
            if !self.field_visible(&field) {
                continue;
            }
            let title = field.title.clone().unwrap_or_else(|| field.key.clone());
            let control = self.field_control(&field, window, cx);
            let invalid = self.error_field.as_deref() == Some(field.key.as_str());
            let validation = self.error.clone().unwrap_or_default();
            form = form.child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SharedString::from(format!(
                                "{title}{}",
                                if field.required == Some(true) {
                                    " *"
                                } else {
                                    ""
                                }
                            ))),
                    )
                    .child(control)
                    .when(invalid, |row| {
                        row.child(validation_bubble(&field.key, validation))
                    })
                    .when_some(field.description, |row, description| {
                        row.child(
                            div()
                                .whitespace_normal()
                                .text_size(px(10.))
                                .text_color(theme.faint)
                                .child(SharedString::from(description)),
                        )
                    }),
            );
        }
        if self.error_field.is_none()
            && let Some(error) = &self.error
        {
            form = form.child(
                div()
                    .text_size(px(10.5))
                    .text_color(theme.red)
                    .whitespace_normal()
                    .child(SharedString::from(error.clone())),
            );
        }
        form.child(
            div()
                .w_full()
                .pt(px(3.))
                .flex()
                .justify_end()
                .gap(px(7.))
                .child(
                    div()
                        .id("provider-cancel")
                        .role(gpui::Role::Button)
                        .aria_label("Cancel")
                        .focusable()
                        .tab_stop(true)
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(999.))
                        .border_1()
                        .border_color(theme.border_strong)
                        .text_size(px(13.3333))
                        .cursor_pointer()
                        .on_key_down(cx.listener(|_, event: &KeyDownEvent, _, cx| {
                            if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                                cx.emit(ProviderFormEvent::Cancel);
                                cx.stop_propagation();
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| cx.emit(ProviderFormEvent::Cancel)),
                        )
                        .child("Cancel")
                        .map(|control| {
                            focus_control(
                                "reveal-provider-cancel",
                                self.scroll.clone(),
                                true,
                                control,
                            )
                        }),
                )
                .child(
                    div()
                        .id("provider-connect")
                        .role(gpui::Role::Button)
                        .aria_label("Connect provider")
                        .focusable()
                        .tab_stop(true)
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(999.))
                        .bg(theme.accent)
                        .text_color(theme.bg)
                        .text_size(px(13.3333))
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                                this.submit(cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.submit(cx)),
                        )
                        .child(if self.busy {
                            "Connecting..."
                        } else {
                            "Connect provider"
                        })
                        .map(|control| {
                            focus_control(
                                "reveal-provider-connect",
                                self.scroll.clone(),
                                !self.busy,
                                control,
                            )
                        }),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn invalid_dynamic_fields_block_submission_and_hidden_fields_do_not(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::EntityInputHandler;
        use std::cell::RefCell;
        let provider: ProviderIntegration = serde_json::from_value(serde_json::json!({
            "id":"openai", "name":"OpenAI", "credentials":[], "environment":{"names":[],"connected":[]}, "oauth":[],
            "key_method":{"label":"API key", "fields":[
                {"key":"region","type":"string","required":true,"minLength":3,"maxLength":6,"pattern":"[a-z]+"},
                {"key":"count","type":"number","minimum":0.5,"maximum":3.5,"default":1.5},
                {"key":"enabled","type":"boolean","default":false},
                {"key":"scopes","type":"multiselect","default":["read"],"options":[
                    {"value":"read","label":"Read"},{"value":"write","label":"Write"},{"value":"admin","label":"Admin"}
                ]},
                {"key":"hidden","type":"string","required":true,"when":[{"key":"enabled","op":"eq","value":true}]}
            ]}
        })).unwrap();
        let (form, cx) = cx
            .add_window_view(|_, cx| ProviderForm::new(provider, AutoScrollHandle::default(), cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let recorded = events.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&form, move |_, event, _| {
                if let ProviderFormEvent::Submit { key, answers, .. } = event {
                    recorded.borrow_mut().push((key.clone(), answers.clone()));
                }
            })
        });
        form.update(cx, |form, cx| form.submit(cx));
        assert!(
            events.borrow().is_empty(),
            "empty keys cannot reach the service"
        );
        let (key, region, count) = form.read_with(cx, |form, _| {
            (
                form.key.clone(),
                form.inputs["region"].clone(),
                form.inputs["count"].clone(),
            )
        });
        key.update_in(cx, |input, window, cx| {
            input.replace_text_in_range(None, "disposable-fixture-key", window, cx)
        });
        for invalid in ["ab", "abc1"] {
            region.update_in(cx, |input, window, cx| {
                input.buffer.select_all();
                input.replace_text_in_range(None, invalid, window, cx);
            });
            form.update(cx, |form, cx| form.submit(cx));
            assert!(
                events.borrow().is_empty(),
                "invalid length/pattern cannot emit a request"
            );
        }
        region.update_in(cx, |input, window, cx| {
            input.buffer.select_all();
            input.replace_text_in_range(None, "abcdefghi", window, cx);
            assert_eq!(
                input.buffer.text(),
                "abcdef",
                "maxlength limits actual inserted text"
            );
        });
        count.update_in(cx, |input, window, cx| {
            input.buffer.select_all();
            input.replace_text_in_range(None, "1", window, cx);
        });
        form.update(cx, |form, cx| form.submit(cx));
        assert!(
            events.borrow().is_empty(),
            "step mismatch cannot emit a request"
        );
        count.update_in(cx, |input, window, cx| {
            input.buffer.select_all();
            input.replace_text_in_range(None, "2.5", window, cx);
        });
        cx.simulate_resize(gpui::size(px(400.), px(800.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(
                gpui::point(px(0.), px(0.)),
                gpui::size(px(400.), px(800.)),
                |_, _| form.clone().into_any_element(),
            );
        };
        draw(cx);
        cx.update(|window, cx| window.focus(&form.read(cx).field_focus["enabled"].clone(), cx));
        draw(cx);
        cx.simulate_keystrokes("space");
        form.update(cx, |form, cx| form.submit(cx));
        assert!(
            events.borrow().is_empty(),
            "keyboard checkbox changes reveal and validate dependent fields"
        );
        cx.simulate_keystrokes("space");
        draw(cx);
        cx.update(|window, cx| window.focus(&form.read(cx).field_focus["scopes"].clone(), cx));
        draw(cx);
        cx.simulate_keystrokes("down");
        cx.simulate_keystrokes("shift-down");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["scopes"], serde_json::json!(["write", "admin"]))
        });
        cx.simulate_keystrokes("cmd-a");
        form.read_with(cx, |form, _| {
            assert_eq!(
                form.values["scopes"],
                serde_json::json!(["read", "write", "admin"])
            )
        });
        form.update(cx, |form, cx| form.submit(cx));
        let events = events.borrow();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "disposable-fixture-key");
        assert_eq!(
            events[0].1,
            serde_json::from_value::<Map<String, Value>>(
                serde_json::json!({"region":"abcdef","count":2.5,"enabled":false,"scopes":["read","write","admin"]})
            )
            .unwrap()
        );
    }
    #[gpui::test]
    fn numeric_steppers_and_validation_bubbles_update_the_provider_form(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::EntityInputHandler;
        use std::cell::RefCell;
        let provider: ProviderIntegration = serde_json::from_value(serde_json::json!({
            "id":"numeric", "name":"Numeric", "credentials":[], "environment":{"names":[],"connected":[]}, "oauth":[],
            "key_method":{"label":"API key", "fields":[
                {"key":"count","type":"number","minimum":0,"maximum":3,"default":1},
                {"key":"level","type":"integer","minimum":1,"maximum":2,"default":1}
            ]}
        })).unwrap();
        let (form, cx) = cx
            .add_window_view(|_, cx| ProviderForm::new(provider, AutoScrollHandle::default(), cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let recorded = events.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&form, move |_, event, _| {
                if let ProviderFormEvent::Submit { key, answers, .. } = event {
                    recorded.borrow_mut().push((key.clone(), answers.clone()));
                }
            })
        });
        cx.simulate_resize(gpui::size(px(420.), px(600.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(
                gpui::point(px(0.), px(0.)),
                gpui::size(px(420.), px(600.)),
                |_, _| form.clone().into_any_element(),
            );
        };
        draw(cx);
        form.update(cx, |form, cx| form.submit(cx));
        draw(cx);
        assert!(
            cx.debug_bounds("provider-validation-bubble-api-key")
                .is_some()
        );

        let (key, count) = form.read_with(cx, |form, _| {
            (form.key.clone(), form.inputs["count"].clone())
        });
        key.update_in(cx, |input, window, cx| {
            input.replace_text_in_range(None, "fixture-key", window, cx)
        });
        draw(cx);
        assert!(
            cx.debug_bounds("provider-validation-bubble-api-key")
                .is_none()
        );

        let up = cx
            .debug_bounds("provider-number-step-count-up")
            .expect("count up");
        cx.simulate_click(up.center(), gpui::Modifiers::default());
        draw(cx);
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["count"], serde_json::json!(2));
        });
        let up = cx
            .debug_bounds("provider-number-step-count-up")
            .expect("count up");
        cx.simulate_click(up.center(), gpui::Modifiers::default());
        draw(cx);
        let up = cx
            .debug_bounds("provider-number-step-count-up")
            .expect("count up");
        cx.simulate_click(up.center(), gpui::Modifiers::default());
        draw(cx);
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["count"], serde_json::json!(3));
        });

        let level_up = cx
            .debug_bounds("provider-number-step-level-up")
            .expect("level up");
        cx.simulate_click(level_up.center(), gpui::Modifiers::default());
        draw(cx);
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["level"], serde_json::json!(2));
        });
        let level_up = cx
            .debug_bounds("provider-number-step-level-up")
            .expect("level up");
        cx.simulate_click(level_up.center(), gpui::Modifiers::default());
        draw(cx);
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["level"], serde_json::json!(2));
        });

        count.update_in(cx, |input, window, cx| {
            input.buffer.select_all();
            input.replace_text_in_range(None, "9", window, cx);
        });
        form.update(cx, |form, cx| form.submit(cx));
        draw(cx);
        assert!(
            cx.debug_bounds("provider-validation-bubble-count")
                .is_some()
        );
        assert!(events.borrow().is_empty());

        count.update_in(cx, |input, window, cx| {
            input.buffer.select_all();
            input.replace_text_in_range(None, "2", window, cx);
        });
        form.update(cx, |form, cx| form.submit(cx));
        draw(cx);
        assert!(
            cx.debug_bounds("provider-validation-bubble-count")
                .is_none()
        );
        let events = events.borrow();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "fixture-key");
        assert_eq!(
            events[0].1,
            serde_json::from_value::<Map<String, Value>>(serde_json::json!({"count":2,"level":2}))
                .unwrap()
        );
    }

    #[gpui::test]
    fn single_and_multiple_option_menus_support_keyboard_selection(cx: &mut gpui::TestAppContext) {
        let provider: ProviderIntegration = serde_json::from_value(serde_json::json!({
            "id":"options", "name":"Options", "credentials":[], "environment":{"names":[],"connected":[]}, "oauth":[],
            "key_method":{"label":"API key", "fields":[
                {"key":"region","type":"string","options":[
                    {"value":"read","label":"Read"},{"value":"write","label":"Write"}
                ]},
                {"key":"scopes","type":"multiselect","default":["read"],"options":[
                    {"value":"read","label":"Read"},{"value":"write","label":"Write"},{"value":"admin","label":"Admin"}
                ]}
            ]}
        })).unwrap();
        let (form, cx) = cx
            .add_window_view(|_, cx| ProviderForm::new(provider, AutoScrollHandle::default(), cx));
        cx.simulate_resize(gpui::size(px(420.), px(700.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(
                gpui::point(px(0.), px(0.)),
                gpui::size(px(420.), px(700.)),
                |_, _| form.clone().into_any_element(),
            );
        };
        draw(cx);
        form.update(cx, |form, cx| {
            form.open_options = Some("region".into());
            form.option_cursor = 0;
            cx.notify();
        });
        draw(cx);
        assert!(cx.debug_bounds("provider-options-region").is_some());
        assert!(cx.debug_bounds("option-region-__select__").is_some());
        assert!(cx.debug_bounds("option-region-read").is_some());
        cx.update(|window, cx| {
            let focus = form.read(cx).field_focus["region"].clone();
            window.focus(&focus, cx);
        });
        draw(cx);
        cx.simulate_keystrokes("down enter");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["region"], serde_json::json!("read"));
            assert!(form.open_options.is_none());
        });
        draw(cx);
        assert!(cx.debug_bounds("provider-options-region").is_none());

        form.update(cx, |form, cx| {
            form.open_options = Some("region".into());
            form.option_cursor = 1;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("home enter");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["region"], serde_json::json!(""));
        });

        cx.update(|window, cx| {
            let focus = form.read(cx).field_focus["scopes"].clone();
            window.focus(&focus, cx);
        });
        draw(cx);
        cx.simulate_keystrokes("space");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["scopes"], serde_json::json!([]));
        });
        cx.simulate_keystrokes("space");
        cx.simulate_keystrokes("down");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["scopes"], serde_json::json!(["read"]));
        });
        cx.simulate_keystrokes("space");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["scopes"], serde_json::json!(["read", "write"]));
        });
        cx.simulate_keystrokes("shift-down");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["scopes"], serde_json::json!(["write", "admin"]));
        });
        cx.simulate_keystrokes("cmd-down space");
        form.read_with(cx, |form, _| {
            assert_eq!(form.values["scopes"], serde_json::json!(["write"]));
        });
        cx.simulate_keystrokes("cmd-a");
        form.read_with(cx, |form, _| {
            assert_eq!(
                form.values["scopes"],
                serde_json::json!(["read", "write", "admin"])
            );
        });
    }

    #[test]
    fn numeric_conditions_match_browser_numbers_without_string_coercion() {
        assert!(same_primitive(
            Some(&serde_json::json!(2.0)),
            &serde_json::json!(2)
        ));
        assert!(!same_primitive(
            Some(&serde_json::json!("2")),
            &serde_json::json!(2)
        ));
        assert!(!same_primitive(
            Some(&serde_json::json!(false)),
            &serde_json::json!(0)
        ));
        assert!(!same_primitive(None, &serde_json::json!(false)));
    }
    #[test]
    fn late_catalog_cannot_replace_another_workspace_or_a_newer_refresh() {
        let scope = |directory: &str| ProviderScope {
            session: "same-session".into(),
            directory: directory.into(),
        };
        let mut state = ProviderCatalog::default();
        state.begin(1, scope("/first"));
        assert!(!state.finish(1, &scope("/moved"), Vec::new(), None));
        state.begin(2, scope("/moved"));
        assert!(!state.finish(1, &scope("/moved"), Vec::new(), Some("stale".into())));
        assert!(state.loading);
        assert!(state.finish(2, &scope("/moved"), Vec::new(), Some("current".into())));
        assert!(!state.loading);
        assert_eq!(state.error.as_deref(), Some("current"));
        state.clear();
        assert!(!state.finish(2, &scope("/moved"), Vec::new(), None));
        assert!(state.needs_load(&scope("/moved")));
    }
}
