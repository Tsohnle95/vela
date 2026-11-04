use crate::{
    input::{Changed, TextInput},
    scroll::{FocusReveal, focus_control},
    theme::Theme,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use vela_engine::ProviderOAuthState;
use vela_opencode::providers::{OAuthAttempt, ProviderIntegration};

pub(crate) enum OAuthEvent {
    Start(String),
    Code(String),
    Cancel,
}
pub(crate) struct OAuthHandle {
    pub form: Entity<OAuthForm>,
    pub scope: crate::providers::ProviderScope,
    pub _subscription: Subscription,
}
pub(crate) struct OAuthForm {
    provider: ProviderIntegration,
    method: usize,
    menu: bool,
    method_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub flow_id: Option<u64>,
    starting: bool,
    busy: bool,
    attempt: Option<OAuthAttempt>,
    error: Option<String>,
    code: Entity<TextInput>,
    focus_code: bool,
    scroll: crate::scroll::AutoScrollHandle,
    _subscription: Subscription,
}
impl EventEmitter<OAuthEvent> for OAuthForm {}
impl OAuthForm {
    pub fn new(
        provider: ProviderIntegration,
        scroll: crate::scroll::AutoScrollHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let code = cx.new(|cx| {
            let mut input = TextInput::new(false, "Paste the code", cx);
            input.set_label("Authorization code");
            input.use_browser_field();
            input
        });
        let subscription = cx.subscribe(&code, |_, _, _: &Changed, cx| cx.notify());
        Self {
            provider,
            method: 0,
            menu: false,
            method_bounds: Rc::new(Cell::new(None)),
            flow_id: None,
            starting: false,
            busy: false,
            attempt: None,
            error: None,
            code,
            focus_code: false,
            scroll,
            _subscription: subscription,
        }
    }
    pub fn begin(&mut self, flow_id: u64, cx: &mut Context<Self>) {
        self.flow_id = Some(flow_id);
        self.starting = true;
        self.error = None;
        cx.notify();
    }
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.flow_id = None;
        self.starting = false;
        self.busy = false;
        self.attempt = None;
        self.code
            .update(cx, |input, cx| input.set_text(String::new(), cx));
        cx.notify();
    }
    pub fn apply(
        &mut self,
        flow_id: u64,
        state: ProviderOAuthState,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.flow_id != Some(flow_id) {
            return false;
        }
        match state {
            ProviderOAuthState::Pending(attempt) => {
                self.focus_code = attempt.code;
                self.attempt = Some(attempt);
                self.starting = false;
                self.busy = false;
            }
            ProviderOAuthState::Confirming => self.busy = true,
            ProviderOAuthState::CodeRejected(error) => {
                self.busy = false;
                self.error = Some(error);
            }
            ProviderOAuthState::Failed(error) => {
                self.reset(cx);
                self.error = Some(error);
            }
            ProviderOAuthState::Complete => {
                self.reset(cx);
                return true;
            }
        }
        cx.notify();
        false
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        if self.starting || self.flow_id.is_some() {
            return;
        }
        if let Some(method) = self.provider.oauth.get(self.method) {
            let method = method.id.clone();
            self.starting = true;
            self.error = None;
            cx.emit(OAuthEvent::Start(method));
            cx.notify();
        }
    }
    fn confirm(&mut self, cx: &mut Context<Self>) {
        if self.busy
            || self.flow_id.is_none()
            || !self.attempt.as_ref().is_some_and(|attempt| attempt.code)
        {
            return;
        }
        let code = self.code.read(cx).buffer.text().to_owned();
        if code.is_empty() {
            return;
        }
        self.busy = true;
        cx.emit(OAuthEvent::Code(code));
        cx.notify();
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(OAuthEvent::Cancel);
    }
    fn open_methods(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        if let Some(bounds) = self.method_bounds.get() {
            let labels = self
                .provider
                .oauth
                .iter()
                .map(|method| method.label.clone())
                .collect::<Vec<_>>();
            if let Some(menu) =
                crate::platform_menu::Popup::new(window, bounds, &labels, self.method, 13.)
            {
                cx.spawn(async move |this, cx| {
                    if let Some(index) = menu.show() {
                        let _ = this.update(cx, |this, cx| {
                            if index < this.provider.oauth.len() && this.attempt.is_none() {
                                this.method = index;
                                cx.notify();
                            }
                        });
                    }
                })
                .detach();
                return;
            }
        }
        self.menu = !self.menu;
        cx.notify();
    }
}
impl Render for OAuthForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::default();
        let method_bounds = self.method_bounds.clone();
        if std::mem::take(&mut self.focus_code) {
            let code = self.code.downgrade();
            let scroll = self.scroll.clone();
            window.on_next_frame(move |window, cx| {
                if let Some(code) = code.upgrade() {
                    window.focus(&code.focus_handle(cx), cx);
                    if let Some(bounds) = code.read(cx).outer_bounds() {
                        scroll.reveal(bounds);
                        window.refresh();
                    }
                }
            });
        }
        let mut root = div()
            .id("provider-oauth")
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "enter" && this.attempt.is_some() {
                    if this.code.read(cx).buffer.marked.is_some() {
                        return;
                    }
                    this.confirm(cx);
                    cx.stop_propagation();
                }
            }));
        if let Some(attempt) = &self.attempt {
            root = root.child(
                div()
                    .mt(px(9.))
                    .text_size(px(10.))
                    .text_color(theme.faint)
                    .whitespace_normal()
                    .child(SharedString::from(if attempt.instructions.is_empty() {
                        "Follow the authorization prompt".into()
                    } else {
                        attempt.instructions.clone()
                    })),
            );
            let mut inline = div().w_full().min_w_0().flex().flex_wrap().items_baseline();
            if !attempt.url.is_empty()
                && vela_browser::allowed_url(&attempt.url)
                && attempt.url != "about:blank"
            {
                let url = attempt.url.clone();
                let key_url = url.clone();
                inline = inline.child(
                    div()
                        .id("provider-authorization-link")
                        .role(Role::Link)
                        .aria_label("Open authorization page")
                        .focusable()
                        .tab_stop(true)
                        .relative()
                        .text_size(px(10.5))
                        .text_color(theme.accent)
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            cx.open_url(&url);
                            cx.stop_propagation();
                        })
                        .on_key_down(move |event: &KeyDownEvent, _, cx| {
                            if event.keystroke.key == "enter" {
                                cx.open_url(&key_url);
                                cx.stop_propagation();
                            }
                        })
                        .child("Open authorization page")
                        .map(|control| {
                            focus_control("reveal-oauth-link", self.scroll.clone(), true, control)
                        }),
                );
            }
            if attempt.code {
                // A label is an inline box in the reference. Its text and the
                // replaced input can wrap independently at the card edge.
                inline = inline
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.text)
                            .child("Authorization code"),
                    )
                    .child(FocusReveal::input(
                        "reveal-oauth-code",
                        self.scroll.clone(),
                        self.code.focus_handle(cx),
                        self.code.clone(),
                    ));
            }
            root = root.child(inline);
            let mut actions = div()
                .w_full()
                .pt(px(3.))
                .flex()
                .justify_end()
                .items_center()
                .gap(px(7.));
            if attempt.code {
                let enabled = !self.busy && !self.code.read(cx).buffer.text().is_empty();
                actions = actions.child(
                    div()
                        .id("provider-confirm-code")
                        .role(Role::Button)
                        .aria_label("Confirm code")
                        .focusable()
                        .tab_stop(enabled)
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(999.))
                        .border_1()
                        .border_color(theme.accent)
                        .bg(theme.accent)
                        .text_color(theme.bg)
                        .text_size(px(13.3333))
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.confirm(cx)),
                        )
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                                this.confirm(cx);
                                cx.stop_propagation();
                            }
                        }))
                        .child(if self.busy {
                            "Confirming..."
                        } else {
                            "Confirm code"
                        })
                        .map(|control| {
                            focus_control(
                                "reveal-oauth-confirm",
                                self.scroll.clone(),
                                enabled,
                                control,
                            )
                        }),
                );
            } else {
                actions = actions.child(
                    div()
                        .mt(px(9.))
                        .text_size(px(10.))
                        .text_color(theme.faint)
                        .child("Waiting for authorization…"),
                );
            }
            root = root.child(
                actions.child(
                    div()
                        .id("provider-oauth-cancel")
                        .role(Role::Button)
                        .aria_label("Cancel authorization")
                        .focusable()
                        .tab_stop(true)
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(999.))
                        .border_1()
                        .border_color(theme.border_strong)
                        .text_size(px(13.3333))
                        .text_color(theme.dim)
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.cancel(cx)),
                        )
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                                this.cancel(cx);
                                cx.stop_propagation();
                            }
                        }))
                        .child("Cancel")
                        .map(|control| {
                            focus_control("reveal-oauth-cancel", self.scroll.clone(), true, control)
                        }),
                ),
            );
        } else {
            let label = if self.starting {
                "Starting...".into()
            } else {
                self.provider
                    .oauth
                    .get(self.method)
                    .map(|method| format!("Connect with {}", method.label))
                    .unwrap_or_else(|| "Connect".into())
            };
            let mut controls = div()
                .relative()
                .w_full()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_baseline()
                .child(
                    div()
                        .id("provider-oauth-start")
                        .role(Role::Button)
                        .aria_label(SharedString::from(label.clone()))
                        .focusable()
                        .tab_stop(!self.starting)
                        .self_start()
                        .px(px(6.))
                        .py(px(2.))
                        .rounded(px(2.))
                        .border_1()
                        .border_color(rgb(if theme.dark { 0x858585 } else { 0x767676 }))
                        .bg(rgb(if theme.dark { 0x6b6b6b } else { 0xefefef }))
                        .text_size(px(13.3333))
                        .text_color(rgb(if theme.dark { 0xffffff } else { 0x000000 }))
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.start(cx)),
                        )
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                                this.start(cx);
                                cx.stop_propagation();
                            }
                        }))
                        .child(SharedString::from(label))
                        .map(|control| {
                            focus_control(
                                "reveal-oauth-start",
                                self.scroll.clone(),
                                !self.starting,
                                control,
                            )
                        }),
                );
            if self.provider.oauth.len() > 1 {
                controls = controls.child(
                    div()
                        .id("provider-oauth-method")
                        .relative()
                        .role(Role::ComboBox)
                        .aria_label("Authorization method")
                        .aria_value(SharedString::from(
                            self.provider.oauth[self.method].label.clone(),
                        ))
                        .aria_expanded(self.menu)
                        .focusable()
                        .tab_stop(true)
                        .self_start()
                        .px(px(6.))
                        .py(px(2.))
                        .border_1()
                        .border_color(rgb(if theme.dark { 0x858585 } else { 0x767676 }))
                        .rounded(px(2.))
                        .text_size(px(13.3333))
                        .bg(rgb(if theme.dark { 0x3b3b3b } else { 0xffffff }))
                        .text_color(rgb(if theme.dark { 0xffffff } else { 0x000000 }))
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.open_methods(window, cx);
                            }),
                        )
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            match event.keystroke.key.as_str() {
                                "up" => this.method = this.method.saturating_sub(1),
                                "down" => {
                                    this.method =
                                        (this.method + 1).min(this.provider.oauth.len() - 1)
                                }
                                "enter" | "space" => this.open_methods(window, cx),
                                "escape" => this.menu = false,
                                _ => return,
                            }
                            cx.notify();
                            cx.stop_propagation();
                        }))
                        .child(SharedString::from(format!(
                            "{} ▾",
                            self.provider.oauth[self.method].label
                        )))
                        .child(
                            canvas(
                                move |bounds, _, _| method_bounds.set(Some(bounds.dilate(px(1.)))),
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full(),
                        )
                        .map(|control| {
                            focus_control("reveal-oauth-method", self.scroll.clone(), true, control)
                        }),
                );
                if self.menu {
                    let mut options = div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .border_1()
                        .border_color(theme.border_strong)
                        .bg(theme.panel);
                    for (index, method) in self.provider.oauth.iter().enumerate() {
                        options = options.child(
                            div()
                                .id(("oauth-method", index))
                                .role(Role::ListBoxOption)
                                .aria_label(SharedString::from(method.label.clone()))
                                .aria_selected(index == self.method)
                                .focusable()
                                .tab_stop(true)
                                .p(px(6.))
                                .text_size(px(11.))
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.method = index;
                                        this.menu = false;
                                        cx.notify();
                                    }),
                                )
                                .on_key_down(cx.listener(
                                    move |this, event: &KeyDownEvent, _, cx| {
                                        if event.keystroke.key == "enter"
                                            || event.keystroke.key == "space"
                                        {
                                            this.method = index;
                                            this.menu = false;
                                            cx.notify();
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                                .child(SharedString::from(method.label.clone())),
                        );
                    }
                    controls = controls.child(
                        deferred(
                            div()
                                .absolute()
                                .top(px(22.))
                                .left_0()
                                .w_full()
                                .child(options),
                        )
                        .with_priority(10),
                    );
                }
            }
            root = root.child(controls);
        }
        root.when_some(self.error.clone(), |root, error| {
            root.child(
                div()
                    .mt(px(8.))
                    .text_size(px(10.5))
                    .text_color(theme.red)
                    .whitespace_normal()
                    .child(SharedString::from(error)),
            )
        })
    }
}
