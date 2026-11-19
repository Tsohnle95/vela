//! Embedded Agent TUI hosting. Source mounts `AgentTui` inside the agent
//! panel; its terminal is independent from the terminal tray's tab set.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AgentPanelMode {
    Gui,
    Tui,
}

impl AgentPanelMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Gui => "gui",
            Self::Tui => "tui",
        }
    }

    fn from_saved(value: Option<&String>) -> Self {
        match value.map(String::as_str) {
            Some("tui") => Self::Tui,
            _ => Self::Gui,
        }
    }
}

pub(super) struct AgentTuiView {
    pub view: TerminalView,
    pub workspace: String,
    sanitizer: BackgroundSanitizer,
}

#[derive(Default)]
pub(super) struct AgentTuiState {
    pub views: HashMap<String, AgentTuiView>,
    pub notices: HashMap<String, String>,
    pub menu: Option<String>,
}

impl AgentTuiState {
    fn get_mut(&mut self, session: &str) -> Option<&mut AgentTuiView> {
        self.views.get_mut(session)
    }

    #[cfg(test)]
    pub(super) fn insert_view(&mut self, session: &str, workspace: &str, view: TerminalView) {
        self.views.insert(
            session.to_owned(),
            AgentTuiView {
                view,
                workspace: workspace.to_owned(),
                sanitizer: BackgroundSanitizer::default(),
            },
        );
    }
}

/// Source strips SGR background colors from the Kitty glass theme so the
/// runtime TUI stays transparent. Escape sequences can be split across PTY
/// reads, so the last incomplete sequence is retained until the next chunk.
#[derive(Default)]
struct BackgroundSanitizer {
    pending: Vec<u8>,
}

impl BackgroundSanitizer {
    fn reset(&mut self) {
        self.pending.clear();
    }

    fn strip(&mut self, input: &[u8]) -> Vec<u8> {
        let mut value = std::mem::take(&mut self.pending);
        value.extend_from_slice(input);
        let mut output = Vec::with_capacity(value.len());
        let mut cursor = 0;
        while cursor < value.len() {
            let Some(relative) = value[cursor..]
                .windows(2)
                .position(|window| window == b"\x1b[")
            else {
                output.extend_from_slice(&value[cursor..]);
                break;
            };
            let start = cursor + relative;
            output.extend_from_slice(&value[cursor..start]);
            let mut final_index = start + 2;
            while final_index < value.len()
                && !(0x40..=0x7e).contains(&value[final_index])
            {
                final_index += 1;
            }
            if final_index >= value.len() {
                self.pending.extend_from_slice(&value[start..]);
                break;
            }
            let final_byte = value[final_index];
            if final_byte == b'm' {
                output.extend_from_slice(&strip_sgr_backgrounds(
                    &value[start + 2..final_index],
                ));
            } else {
                output.extend_from_slice(&value[start..=final_index]);
            }
            cursor = final_index + 1;
        }
        output
    }
}

fn strip_sgr_backgrounds(params: &[u8]) -> Vec<u8> {
    let parts = params
        .split(|byte| *byte == b';')
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect::<Vec<_>>();
    let mut kept = Vec::new();
    let mut index = 0;
    while index < parts.len() {
        let part = parts[index].as_str();
        if part.contains(':') {
            if part.split(':').next() != Some("48") {
                kept.push(part.to_owned());
            }
            index += 1;
            continue;
        }
        if part == "48" {
            match parts.get(index + 1).map(String::as_str) {
                Some("5") => index += 3,
                Some("2") => index += 5,
                _ => index += 1,
            }
            continue;
        }
        if let Ok(code) = part.parse::<u16>()
            && ((40..=47).contains(&code) || (100..=107).contains(&code))
        {
            index += 1;
            continue;
        }
        kept.push(part.to_owned());
        index += 1;
    }
    if kept.is_empty() {
        return Vec::new();
    }
    format!("\u{1b}[{}m", kept.join(";")).into_bytes()
}

/// Source scales the embedded TUI down inside narrow panels.
pub(super) fn tui_metrics_for_width(width: f32) -> (f32, f32) {
    if width > 0. && width <= 520. {
        (10., 11.5)
    } else if width > 0. && width <= 720. {
        (11., 13.2)
    } else {
        (12., 15.)
    }
}

pub(super) fn tui_args_for_session(session: &str) -> Vec<String> {
    vec!["--session".into(), session.into()]
}

/// Source passes a child-only OpenCode config with its system theme while
/// preserving any inline preferences. `OPENCODE_CLI_CONFIG_CONTENT` is JSON5
/// in Source; native accepts JSON and rejects invalid inline config the same
/// way the IPC command does.
pub(super) fn tui_environment(
    inline: Option<&str>,
) -> Result<Vec<(String, String)>, String> {
    let mut preferences = match inline.filter(|value| !value.trim().is_empty()) {
        Some(value) => serde_json::from_str::<serde_json::Value>(value)
            .map_err(|error| format!("Could not parse OPENCODE_CLI_CONFIG_CONTENT: {error}"))?,
        None => serde_json::json!({}),
    };
    let object = preferences
        .as_object_mut()
        .ok_or_else(|| "OPENCODE_CLI_CONFIG_CONTENT must be a JSON object".to_owned())?;
    object.insert(
        "theme".into(),
        serde_json::json!({ "name": "system", "mode": "system" }),
    );
    Ok(vec![(
        "OPENCODE_CLI_CONFIG_CONTENT".into(),
        serde_json::to_string(&preferences).map_err(|error| error.to_string())?,
    )])
}

/// Source's session-choice subtitle wording.
fn close_panel_detail(agent_mode: bool) -> &'static str {
    if agent_mode {
        "Remove this model panel"
    } else {
        "Hide the agent panel"
    }
}

fn agent_session_choice_detail(is_visible: bool, tiled: bool, disabled: bool) -> &'static str {
    if disabled {
        "Agent panel limit reached"
    } else if is_visible {
        if tiled {
            "In Agent Mode"
        } else {
            "Current coding workspace"
        }
    } else if tiled {
        "Add to Agent Mode"
    } else {
        "Switch workspace"
    }
}

impl VelaApp {
    pub(super) fn saved_agent_panel_mode(&self, session: &str) -> AgentPanelMode {
        AgentPanelMode::from_saved(self.settings.agent_panel_modes.get(session))
    }

    /// A saved TUI choice stays selected while the runtime capability is
    /// temporarily unavailable, but the panel renders GUI until it returns.
    pub(super) fn agent_panel_mode(&self, session: &str) -> AgentPanelMode {
        match self.saved_agent_panel_mode(session) {
            AgentPanelMode::Tui if self.runtime_capabilities.tui => AgentPanelMode::Tui,
            _ => AgentPanelMode::Gui,
        }
    }

    pub(super) fn agent_panel_menu_open(&self, session: &str) -> bool {
        self.agent_tui.menu.as_deref() == Some(session)
    }

    pub(super) fn toggle_agent_panel_menu(&mut self, session: &str, cx: &mut Context<Self>) {
        if self.agent_panel_menu_open(session) {
            self.agent_tui.menu = None;
        } else {
            self.agent_tui.menu = Some(session.to_owned());
        }
        cx.notify();
    }

    /// Source's `sessionChoices`: open views first, then this window's panels,
    /// deduplicated with the panel title and directory.
    pub(super) fn agent_panel_session_choices(&self) -> Vec<(String, String, String)> {
        let mut seen = std::collections::HashSet::new();
        let mut choices = Vec::new();
        for id in self.engine.open_session_ids() {
            if !seen.insert(id.clone()) {
                continue;
            }
            let directory = self
                .sessions
                .iter()
                .find(|session| session.id == id)
                .map(|session| session.directory.clone())
                .unwrap_or_default();
            choices.push((id.clone(), self.panel_title(&id), directory));
        }
        for panel in self.panels.entries() {
            if !seen.insert(panel.session.clone()) {
                continue;
            }
            let directory = self
                .sessions
                .iter()
                .find(|session| session.id == panel.session)
                .map(|session| session.directory.clone())
                .unwrap_or_default();
            choices.push((panel.session.clone(), panel.title.clone(), directory));
        }
        if let Some(anchor) = &self.hidden_agent_anchor
            && seen.insert(anchor.session.clone())
        {
            let directory = self
                .sessions
                .iter()
                .find(|session| session.id == anchor.session)
                .map(|session| session.directory.clone())
                .unwrap_or_default();
            choices.push((anchor.session.clone(), anchor.title.clone(), directory));
        }
        choices
    }

    /// Source's `selectAgentSession`: focus an attached session, otherwise
    /// replace the coding panel or add a panel while tiled.
    pub(super) fn choose_agent_panel_session(
        &mut self,
        session: String,
        cx: &mut Context<Self>,
    ) {
        if self.panels.contains(&session) {
            self.activate_panel(session, cx);
            return;
        }
        if self
            .hidden_agent_anchor
            .as_ref()
            .is_some_and(|panel| panel.session == session)
        {
            self.agent_panel_open = true;
            self.restore_hidden_agent_anchor();
            if self.panels.contains(&session) {
                self.panels.activate(&session);
                self.active_id = Some(session);
            }
            cx.notify();
            return;
        }
        if self.agent_mode {
            if self.panels.len() + usize::from(self.hidden_agent_anchor.is_some()) >= 8 {
                return;
            }
            self.show_session(session, false, cx);
            self.panels.even_widths();
        } else {
            self.show_session(session, true, cx);
        }
    }

    pub(super) fn close_agent_panel_menu(&mut self, cx: &mut Context<Self>) {
        if self.agent_tui.menu.take().is_some() {
            cx.notify();
        }
    }

    pub(super) fn choose_agent_panel_mode(
        &mut self,
        session: &str,
        mode: AgentPanelMode,
        cx: &mut Context<Self>,
    ) {
        self.agent_tui.menu = None;
        if mode == AgentPanelMode::Tui && !self.runtime_capabilities.tui {
            self.agent_tui
                .notices
                .insert(session.to_owned(), "TUI is unavailable for this runtime.".into());
            cx.notify();
            return;
        }
        self.agent_tui.notices.remove(session);
        self.settings
            .agent_panel_modes
            .insert(session.to_owned(), mode.as_str().to_owned());
        self.persist_preferences(cx);
        match mode {
            AgentPanelMode::Gui => self.stop_agent_tui(session),
            AgentPanelMode::Tui => self.ensure_agent_tui(session, cx),
        }
        cx.notify();
    }

    /// Source keeps the GUI/TUI switch, the runtime capability reason and the
    /// close action in one options menu opened from the panel header.
    pub(super) fn render_agent_panel_options(
        &mut self,
        session: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let open = self.agent_panel_menu_open(session);
        let owner = session.to_owned();
        let selector = format!("agent-panel-options-{session}");
        let mut root = div().relative().flex_shrink_0().child(
            div()
                .id(SharedString::from(format!("agent-panel-options-{session}")))
                .debug_selector(move || selector.clone())
                .role(Role::Button)
                .aria_label("Agent panel options")
                .aria_expanded(open)
                .focusable()
                .tab_stop(true)
                .size(px(28.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.))
                .text_color(if open { theme.accent } else { theme.dim })
                .cursor_pointer()
                .hover(|style| style.bg(theme.selected).text_color(theme.text))
                .on_click(cx.listener(move |app, _, _, cx| {
                    cx.stop_propagation();
                    app.toggle_agent_panel_menu(&owner, cx);
                }))
                .on_key_down(cx.listener(move |app, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        app.close_agent_panel_menu(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(crate::icons::inherited_glyph("more-horizontal", 18.)),
        );
        if open {
            let menu_selector = format!("agent-panel-options-menu-{session}");
            let mut menu = div()
                .id(SharedString::from(format!(
                    "agent-panel-options-menu-{session}"
                )))
                .debug_selector(move || menu_selector.clone())
                .role(Role::Menu)
                .aria_label("Agent panel options")
                .absolute()
                .top(px(32.))
                .right(px(0.))
                .w(px(260.))
                .p(px(4.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                .on_mouse_down_out(cx.listener(|app, _, _, cx| {
                    app.close_agent_panel_menu(cx);
                }))
                .bg(if theme.is_kitty() {
                    theme::rgba(16, 22, 36, 0.96)
                } else {
                    theme.elevated
                })
                .border_1()
                .border_color(theme.border_strong)
                .rounded(px(10.))
                .shadow(theme.small_shadow())
                .flex()
                .flex_col();
            menu = menu.child(
                self.agent_mode_menu_item(session, AgentPanelMode::Gui, cx),
            );
            menu = menu.child(
                self.agent_mode_menu_item(session, AgentPanelMode::Tui, cx),
            );
            menu = menu.child(
                div()
                    .h(px(1.))
                    .mx(px(2.))
                    .my(px(4.))
                    .bg(theme.border),
            );
            menu = menu.child(self.render_usage_button(session, cx));
            let choices = self.agent_panel_session_choices();
            if !choices.is_empty() {
                let agent_mode = self.agent_mode;
                let limit_reached =
                    self.panels.len() + usize::from(self.hidden_agent_anchor.is_some()) >= 8;
                let mut visible = self
                    .panels
                    .entries()
                    .iter()
                    .map(|panel| panel.session.clone())
                    .collect::<std::collections::HashSet<_>>();
                if let Some(anchor) = &self.hidden_agent_anchor {
                    visible.insert(anchor.session.clone());
                }
                menu = menu.child(
                    div()
                        .h(px(1.))
                        .mx(px(2.))
                        .my(px(4.))
                        .bg(theme.border),
                );
                menu = menu.child(
                    div()
                        .debug_selector(|| "agent-panel-sessions-heading".into())
                        .px(px(8.))
                        .pt(px(4.))
                        .pb(px(3.))
                        .text_size(px(10.))
                        .text_color(theme.faint)
                        .child("Active sessions"),
                );
                for (id, title, _directory) in choices {
                    let is_visible = visible.contains(&id);
                    let disabled = limit_reached && !is_visible && agent_mode;
                    let detail = agent_session_choice_detail(is_visible, agent_mode, disabled);
                    let row_id = format!("agent-panel-session-choice-{id}");
                    let debug_id = row_id.clone();
                    let choose = id.clone();
                    menu = menu.child(
                        div()
                            .id(SharedString::from(row_id))
                            .debug_selector(move || debug_id.clone())
                            .role(Role::MenuItem)
                            .aria_label(SharedString::from(title.clone()))
                            .aria_toggled(if is_visible {
                                gpui::Toggled::True
                            } else {
                                gpui::Toggled::False
                            })
                            .w_full()
                            .px(px(8.))
                            .py(px(6.))
                            .rounded(px(6.))
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap(px(1.))
                            .text_size(px(11.))
                            .text_color(if disabled {
                                theme.faint
                            } else if is_visible {
                                theme.accent
                            } else {
                                theme.text
                            })
                            .when(disabled, |row| row.opacity(0.5))
                            .when(!disabled, |row| {
                                row.cursor_pointer()
                                    .hover(|style| style.bg(theme.selected))
                                    .on_click(cx.listener(move |app, _, _, cx| {
                                        cx.stop_propagation();
                                        app.close_agent_panel_menu(cx);
                                        app.choose_agent_panel_session(choose.clone(), cx);
                                    }))
                            })
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .truncate()
                                    .child(SharedString::from(title)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(if disabled { theme.faint } else { theme.dim })
                                    .child(detail),
                            ),
                    );
                }
            }
            menu = menu.child(
                div()
                    .h(px(1.))
                    .mx(px(2.))
                    .my(px(4.))
                    .bg(theme.border),
            );
            let close_session = session.to_owned();
            let close_selector = format!("agent-panel-options-close-{session}");
            menu = menu.child(
                div()
                    .id(SharedString::from(format!(
                        "agent-panel-options-close-{session}"
                    )))
                    .debug_selector(move || close_selector.clone())
                    .role(Role::MenuItem)
                    .aria_label("Close panel")
                    .w_full()
                    .px(px(8.))
                    .py(px(7.))
                    .rounded(px(6.))
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(2.))
                    .text_size(px(11.))
                    .text_color(theme.text)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        cx.stop_propagation();
                        app.close_agent_panel_menu(cx);
                        app.close_panel(close_session.clone(), cx);
                    }))
                    .child("Close panel")
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme.faint)
                            .child(close_panel_detail(self.agent_mode)),
                    ),
            );
            root = root.child(menu);
        }
        root.into_any_element()
    }

    fn agent_mode_menu_item(
        &self,
        session: &str,
        mode: AgentPanelMode,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let title = match mode {
            AgentPanelMode::Gui => "GUI",
            AgentPanelMode::Tui => "TUI",
        };
        let selected = self.agent_panel_mode(session) == mode;
        let disabled = mode == AgentPanelMode::Tui && !self.runtime_capabilities.tui;
        let detail = match mode {
            AgentPanelMode::Gui => "Vela agent panel",
            AgentPanelMode::Tui if disabled => "Unavailable",
            AgentPanelMode::Tui => "Terminal interface",
        };
        let owner = session.to_owned();
        let selector = format!("agent-panel-mode-{}-{session}", mode.as_str());
        let mut item = div()
            .id(SharedString::from(format!(
                "agent-panel-mode-{}-{session}",
                mode.as_str()
            )))
            .debug_selector(move || selector.clone())
            .role(Role::MenuItemRadio)
            .aria_label(title)
            .aria_selected(selected)
            .w_full()
            .px(px(8.))
            .py(px(7.))
            .rounded(px(6.))
            .flex()
            .flex_col()
            .items_start()
            .gap(px(2.))
            .text_size(px(11.))
            .text_color(if disabled { theme.faint } else { theme.text })
            .when(selected, |item| item.bg(theme.selected))
            .when(disabled, |item| item.opacity(0.55));
        // Source styles the TUI row disabled but leaves the click handler in
        // place so it can explain why the runtime cannot host a TUI.
        item = item.cursor_pointer().hover(|style| style.bg(theme.selected)).on_click(
            cx.listener(move |app, _, _, cx| {
                cx.stop_propagation();
                app.choose_agent_panel_mode(&owner, mode, cx);
            }),
        );
        item.child(title)
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(if selected { theme.accent } else { theme.faint })
                    .child(detail),
            )
            .into_any_element()
    }

    pub(super) fn ensure_agent_tui_views(&mut self, cx: &mut Context<Self>) {
        let sessions = self
            .panels
            .entries()
            .iter()
            .map(|panel| panel.session.clone())
            .collect::<HashSet<_>>();
        let stale = self
            .agent_tui
            .views
            .keys()
            .filter(|session| !sessions.contains(*session))
            .cloned()
            .collect::<Vec<_>>();
        for session in stale {
            self.stop_agent_tui(&session);
        }
        for session in sessions {
            self.ensure_agent_tui(&session, cx);
        }
    }

    fn ensure_agent_tui(&mut self, session: &str, cx: &mut Context<Self>) {
        if self.agent_panel_mode(session) != AgentPanelMode::Tui {
            self.stop_agent_tui(session);
            return;
        }
        let Some(view) = self.engine.view(session) else {
            return;
        };
        let workspace = view.summary().directory.clone();
        if workspace.is_empty() {
            return;
        }
        if let Some(existing) = self.agent_tui.views.get(session)
            && existing.workspace != workspace
        {
            self.stop_agent_tui(session);
        }
        if self.agent_tui.views.contains_key(session) {
            return;
        }
        if !self.start_agent_tui(session, &workspace, cx) {
            let notice = self
                .agent_tui
                .notices
                .get(session)
                .cloned()
                .unwrap_or_else(|| "Could not start the agent TUI".into());
            self.agent_tui.notices.insert(session.to_owned(), notice);
            self.settings
                .agent_panel_modes
                .insert(session.to_owned(), AgentPanelMode::Gui.as_str().into());
            self.persist_preferences(cx);
        }
    }

    fn start_agent_tui(
        &mut self,
        session: &str,
        workspace: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        let configured = std::env::var_os("VELA_OPENCODE_BINARY");
        let path_var = std::env::var_os("PATH");
        let Some(program) = find_tui_program(configured.as_deref(), path_var.as_deref()) else {
            self.agent_tui.notices.insert(
                session.to_owned(),
                "OpenCode CLI was not found. Install OpenCode or set VELA_OPENCODE_BINARY.".into(),
            );
            return false;
        };
        let program = program.to_string_lossy().into_owned();
        let envs = match tui_environment(
            std::env::var("OPENCODE_CLI_CONFIG_CONTENT")
                .ok()
                .as_deref(),
        ) {
            Ok(envs) => envs,
            Err(error) => {
                self.agent_tui.notices.insert(session.to_owned(), error);
                return false;
            }
        };
        let id = format!("term-{}", uuid::Uuid::new_v4());
        let mut terminal = TerminalView::new(id.clone(), 100, 24, cx);
        let input_id = id.clone();
        terminal.input_subscriptions.push(cx.subscribe(
            &terminal.input,
            move |app, _, event: &terminal_composition::Committed, cx| {
                let Some(view) = app.agent_tui.get_mut(&input_id) else {
                    return;
                };
                if !view.view.running {
                    return;
                }
                view.view.emulator.clear_selection();
                view.view.emulator.scroll_to_bottom();
                view.view.selecting = false;
                app.engine
                    .terminal_input(view.view.id.clone(), event.0.as_bytes().to_vec());
                cx.notify();
            },
        ));
        let composition_id = id.clone();
        terminal.input_subscriptions.push(cx.observe(
            &terminal.input,
            move |app, input, cx| {
                if input.read(cx).buffer.marked.is_some()
                    && let Some(entry) = app.agent_tui.views.get_mut(&composition_id)
                {
                    entry.view.emulator.scroll_to_bottom();
                }
                cx.notify();
            },
        ));
        terminal
            .input_subscriptions
            .push(cx.observe(&terminal.blink, |_, _, cx| {
                cx.notify();
            }));
        self.agent_tui.views.insert(
            session.to_owned(),
            AgentTuiView {
                view: terminal,
                workspace: workspace.to_owned(),
                sanitizer: BackgroundSanitizer::default(),
            },
        );
        self.engine
            .start_terminal_command(vela_engine::TerminalCommand {
                id,
                cwd: workspace.to_owned(),
                program,
                args: tui_args_for_session(session),
                envs,
                cols: 100,
                rows: 24,
            });
        true
    }

    pub(super) fn stop_agent_tui(&mut self, session: &str) {
        if let Some(entry) = self.agent_tui.views.remove(session) {
            self.engine.stop_terminal(entry.view.id);
        }
    }

    /// Feed terminal output into the embedded TUI when the id belongs to it.
    /// Returns `false` so the tray handler can try its own registry.
    pub(super) fn feed_agent_tui(
        &mut self,
        id: &str,
        data: &[u8],
        cx: &mut Context<Self>,
    ) -> bool {
        let kitty = Theme::default().is_kitty();
        for entry in self.agent_tui.views.values_mut() {
            if entry.view.id != id {
                continue;
            }
            let bytes = if kitty {
                entry.sanitizer.strip(data)
            } else {
                entry.sanitizer.reset();
                data.to_vec()
            };
            let replies = entry.view.emulator.feed(&bytes);
            if !replies.is_empty() {
                self.engine.terminal_input(id.to_owned(), replies);
            }
            cx.notify();
            return true;
        }
        false
    }

    pub(super) fn handle_agent_tui_exit(
        &mut self,
        id: &str,
        code: Option<i32>,
        cx: &mut Context<Self>,
    ) -> bool {
        let session = self
            .agent_tui
            .views
            .iter()
            .find(|(_, entry)| entry.view.id == id)
            .map(|(session, _)| session.clone());
        let Some(session) = session else {
            return false;
        };
        self.stop_agent_tui(&session);
        self.settings
            .agent_panel_modes
            .insert(session.clone(), AgentPanelMode::Gui.as_str().into());
        self.persist_preferences(cx);
        self.agent_tui
            .notices
            .insert(session.clone(), tui_exit_notice(code));
        if self.active_id.as_deref() == Some(session.as_str()) {
            self.composer_focus_pending = true;
        }
        cx.notify();
        true
    }

    pub(super) fn agent_tui_visible(&self, id: &str) -> bool {
        self.agent_tui.views.iter().any(|(session, entry)| {
            entry.view.id == id
                && self.agent_panel_mode(session) == AgentPanelMode::Tui
        })
    }

    pub(super) fn install_agent_tui_focus_reporting(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pending = self
            .agent_tui
            .views
            .iter()
            .filter(|(_, entry)| !entry.view.focus_reporting)
            .map(|(session, entry)| {
                (
                    session.clone(),
                    entry.view.id.clone(),
                    entry.view.focus.clone(),
                    entry.view.input.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (session, id, focus, input) in pending {
            let entered = id.clone();
            let enter = cx.on_focus(&focus, window, move |app, _, cx| {
                app.report_terminal_focus(&entered, true);
                cx.notify();
            });
            let left = id.clone();
            let leave = cx.on_blur(&focus, window, move |app, _, cx| {
                app.report_terminal_focus(&left, false);
                cx.notify();
            });
            let cancel =
                terminal_composition::cancel_composition_on_blur(&focus, input, window, cx);
            if let Some(entry) = self.agent_tui.views.get_mut(&session) {
                entry.view.focus_reporting = true;
                entry.view.input_subscriptions.push(enter);
                entry.view.input_subscriptions.push(leave);
                entry.view.input_subscriptions.push(cancel);
            }
        }
    }

    pub(super) fn render_agent_tui(
        &mut self,
        session: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.install_agent_tui_focus_reporting(window, cx);
        let Some(entry) = self.agent_tui.views.get_mut(session) else {
            return div().into_any_element();
        };
        let terminal = &mut entry.view;
        let focused = terminal.focus.is_focused(window) && window.is_window_active();
        terminal.blink.update(cx, |blink, cx| {
            blink.update_state(terminal.emulator.cursor_blinking(), focused, cx)
        });
        let cursor_blink_visible = terminal.blink.read(cx).cursor_visible();
        let width = terminal
            .geometry
            .get()
            .map(|(bounds, _)| f32::from(bounds.size.width))
            .unwrap_or(AGENT_WIDTH);
        let (font_size, row_height) = tui_metrics_for_width(width);
        let cell_width = f32::from(
            window
                .text_system()
                .shape_line(
                    "W".into(),
                    px(font_size),
                    &[TextRun {
                        len: 1,
                        font: theme::source_editor_font(),
                        color: Theme::default().text.into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width(),
        )
        .max(1.);
        let rows = terminal.rows as usize;
        let cols = terminal.cols as usize;
        let geometry = terminal.geometry.clone();
        terminal
            .input
            .read(cx)
            .anchor
            .set(terminal.emulator.cursor());
        let input_entity = terminal.input.clone();
        let input_focus = terminal.focus.clone();
        let focus = terminal.focus.clone();
        let terminal_mouse = terminal.emulator.mouse_report_mode()
            != vela_terminal::emulator::MouseReportMode::Off;
        let id = terminal.id.clone();
        let workspace = entry.workspace.clone();
        let session_for_fit = session.to_owned();
        let weak = cx.weak_entity();
        let grid = render_terminal_grid(
            terminal,
            cell_width,
            (rows, cols),
            cursor_blink_visible,
            (font_size, row_height),
            &Theme::default(),
            cx,
        );
        let owner = session.to_owned();
        div()
            .id(SharedString::from(format!("agent-tui-{session}")))
            .debug_selector({
                let selector = format!("agent-tui-{session}");
                move || selector.clone()
            })
            .role(Role::Group)
            .aria_label("Agent TUI")
            .key_context("VelaTerminal")
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .px(px(8.))
            .pt(px(6.))
            .bg(Theme::default().agent)
            .overflow_hidden()
            .track_focus(&focus)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&focus, cx)
            })
            .on_mouse_down(MouseButton::Left, {
                let owner = owner.clone();
                cx.listener(move |app, event: &MouseDownEvent, window, cx| {
                    app.begin_agent_tui_selection(&owner, event, window, cx)
                })
            })
            .on_mouse_down(MouseButton::Right, {
                let owner = owner.clone();
                cx.listener(move |app, event: &MouseDownEvent, window, cx| {
                    app.begin_agent_tui_selection(&owner, event, window, cx)
                })
            })
            .on_mouse_down(MouseButton::Middle, {
                let owner = owner.clone();
                cx.listener(move |app, event: &MouseDownEvent, window, cx| {
                    app.begin_agent_tui_selection(&owner, event, window, cx)
                })
            })
            .on_scroll_wheel(cx.listener({
                let owner = owner.clone();
                move |app, event: &gpui::ScrollWheelEvent, _, cx| {
                    app.scroll_agent_tui(&owner, event, cx)
                }
            }))
            .on_key_down(cx.listener({
                let owner = owner.clone();
                move |app, event: &KeyDownEvent, window, cx| {
                    app.on_agent_tui_key(&owner, event, window, cx)
                }
            }))
            .on_action(cx.listener({
                let owner = owner.clone();
                move |app, _: &crate::input::Copy, _, cx| {
                    app.copy_agent_tui(&owner, cx)
                }
            }))
            .on_action(cx.listener({
                let owner = owner.clone();
                move |app, _: &crate::input::Paste, _, cx| {
                    app.paste_agent_tui(&owner, cx)
                }
            }))
            .cursor(if terminal_mouse {
                gpui::CursorStyle::Arrow
            } else {
                gpui::CursorStyle::IBeam
            })
            .child(grid)
            .child(
                gpui::canvas(
                    move |bounds, window, _| {
                        geometry.set(Some((bounds, cell_width)));
                        let Some(size) = vela_terminal::size::fit(
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                            cell_width,
                            row_height,
                        ) else {
                            return;
                        };
                        let id = id.clone();
                        let workspace = workspace.clone();
                        let weak = weak.clone();
                        let owner = session_for_fit.clone();
                        window.on_next_frame(move |_, cx| {
                            let _ = weak.update(cx, |app, cx| {
                                app.fit_agent_tui(&owner, &id, &workspace, size, cx)
                            });
                        });
                    },
                    move |bounds, _, window, cx| {
                        window.handle_input(
                            &input_focus,
                            gpui::ElementInputHandler::new(bounds, input_entity.clone()),
                            cx,
                        );
                    },
                )
                .absolute()
                .inset_0(),
            )
            .into_any_element()
    }

    fn fit_agent_tui(
        &mut self,
        session: &str,
        id: &str,
        workspace: &str,
        size: vela_terminal::size::Size,
        cx: &mut Context<Self>,
    ) {
        let visible = self.agent_panel_mode(session) == AgentPanelMode::Tui;
        let Some(entry) = self.agent_tui.views.get_mut(session) else {
            return;
        };
        if entry.workspace != workspace || entry.view.id != id || !visible {
            return;
        }
        if entry.view.cols == size.cols && entry.view.rows == size.rows {
            return;
        }
        entry.view.cols = size.cols;
        entry.view.rows = size.rows;
        entry.view.emulator.resize(size.cols, size.rows);
        self.engine.terminal_resize(id, size.cols, size.rows);
        cx.notify();
    }

    pub(super) fn on_agent_tui_key(
        &mut self,
        session: &str,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.agent_tui.views.contains_key(session) {
            return;
        }
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.platform && key == "c" {
            self.copy_agent_tui(session, cx);
            return;
        }
        if matches!(key, "pageup" | "pagedown") && event.keystroke.modifiers.shift {
            let lines = if key == "pageup" { 8 } else { -8 };
            if let Some(entry) = self.agent_tui.views.get_mut(session) {
                entry.view.emulator.scroll(lines);
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let paste = (event.keystroke.modifiers.platform && key == "v")
            || (event.keystroke.modifiers.shift && key == "insert");
        if paste {
            self.paste_agent_tui(session, cx);
            return;
        }
        let Some(entry) = self.agent_tui.views.get_mut(session) else {
            return;
        };
        if !entry.view.running {
            return;
        }
        if terminal_composition::platform_text(
            event,
            entry.view.input.read(cx).buffer.marked.is_some(),
        ) {
            return;
        }
        let bytes = encode_terminal_key(event, entry.view.emulator.app_cursor_mode());
        if !bytes.is_empty() {
            entry.view.input.update(cx, |input, cx| input.cancel(cx));
            entry.view.emulator.clear_selection();
            entry.view.selecting = false;
            entry.view.emulator.scroll_to_bottom();
            self.engine.terminal_input(entry.view.id.clone(), bytes);
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub(super) fn paste_agent_tui(&mut self, session: &str, cx: &mut Context<Self>) {
        let Some(entry) = self.agent_tui.views.get_mut(session) else {
            return;
        };
        if !entry.view.running {
            return;
        }
        let Some(text) = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
        else {
            return;
        };
        let bytes =
            vela_terminal::input::paste_bytes(&text, entry.view.emulator.bracketed_paste_mode());
        entry.view.emulator.scroll_to_bottom();
        entry.view.emulator.clear_selection();
        entry.view.selecting = false;
        entry.view.input.update(cx, |input, cx| input.cancel(cx));
        self.engine.terminal_input(entry.view.id.clone(), bytes);
        cx.stop_propagation();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_panel_detail_follows_agent_mode() {
        assert_eq!(close_panel_detail(true), "Remove this model panel");
        assert_eq!(close_panel_detail(false), "Hide the agent panel");
    }

    #[test]
    fn agent_session_choice_details_match_source_wording() {
        assert_eq!(
            agent_session_choice_detail(true, false, false),
            "Current coding workspace"
        );
        assert_eq!(agent_session_choice_detail(true, true, false), "In Agent Mode");
        assert_eq!(agent_session_choice_detail(false, false, false), "Switch workspace");
        assert_eq!(agent_session_choice_detail(false, true, false), "Add to Agent Mode");
        assert_eq!(
            agent_session_choice_detail(false, true, true),
            "Agent panel limit reached"
        );
    }

    #[test]
    fn sanitizer_strips_kitty_backgrounds_and_keeps_other_sgr() {
        let mut state = BackgroundSanitizer::default();
        assert_eq!(
            state.strip(b"\x1b[101mbright"),
            b"bright".to_vec(),
            "bright background is removed while the text remains"
        );
        assert_eq!(
            state.strip(b"\x1b[1;48;5;0mglass"),
            b"\x1b[1mglass".to_vec(),
            "background color arguments are removed with the 48 parameter"
        );
        assert_eq!(
            state.strip(b"\x1b[48;2;12;80;35mdiff\x1b[49m"),
            b"diff\x1b[49m".to_vec()
        );
        assert_eq!(
            state.strip(b"\x1b[48:2::12:80:35mcolon"),
            b"colon".to_vec()
        );
    }

    #[test]
    fn sanitizer_handles_background_sequences_split_across_reads() {
        let mut state = BackgroundSanitizer::default();
        assert!(state.strip(b"\x1b[48;2;2;").is_empty());
        assert_eq!(state.strip(b"2;4mglass"), b"glass".to_vec());
        assert_eq!(state.strip(b"\x1b[1mbold"), b"\x1b[1mbold".to_vec());
    }

    #[test]
    fn tui_metrics_match_source_width_breakpoints() {
        assert_eq!(tui_metrics_for_width(480.), (10., 11.5));
        assert_eq!(tui_metrics_for_width(520.), (10., 11.5));
        assert_eq!(tui_metrics_for_width(521.), (11., 13.2));
        assert_eq!(tui_metrics_for_width(720.), (11., 13.2));
        assert_eq!(tui_metrics_for_width(721.), (12., 15.));
    }

    #[test]
    fn tui_command_matches_source_session_and_theme_contract() {
        assert_eq!(
            tui_args_for_session("session-1"),
            vec!["--session".to_owned(), "session-1".to_owned()]
        );
        let envs = tui_environment(None).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].0, "OPENCODE_CLI_CONFIG_CONTENT");
        let value: serde_json::Value = serde_json::from_str(&envs[0].1).unwrap();
        assert_eq!(
            value["theme"],
            serde_json::json!({ "name": "system", "mode": "system" })
        );
    }

    #[test]
    fn tui_environment_preserves_inline_preferences_and_rejects_invalid_json() {
        let envs = tui_environment(Some(
            r#"{"theme":{"name":"catppuccin"},"animations":true}"#,
        ))
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&envs[0].1).unwrap();
        assert_eq!(value["animations"], true);
        assert_eq!(
            value["theme"],
            serde_json::json!({ "name": "system", "mode": "system" })
        );
        assert!(tui_environment(Some("{not json")).is_err());
    }
}
