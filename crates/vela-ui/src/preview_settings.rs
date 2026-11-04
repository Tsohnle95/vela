use super::*;
use vela_engine::preview::PreviewServer;
use vela_opencode::inventory::{McpServerInfo, PluginInfo, SkillInfo};

/// Workspace agent integrations shown in Settings > Servers. The request is
/// scoped to the active workspace so a late response cannot replace another
/// session's inventory.
#[derive(Clone, Debug, Default)]
pub(super) struct IntegrationInventory {
    pub session: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
    pub mcp_servers: Vec<McpServerInfo>,
    pub plugins: Vec<PluginInfo>,
    pub skills: Vec<SkillInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct InventoryRow {
    pub title: String,
    pub detail: String,
    pub status: String,
}

impl IntegrationInventory {
    pub fn finish(
        &mut self,
        session: String,
        mcp_servers: Vec<McpServerInfo>,
        plugins: Vec<PluginInfo>,
        skills: Vec<SkillInfo>,
        error: Option<String>,
    ) {
        self.session = Some(session);
        self.loading = false;
        self.error = error;
        if self.error.is_some() {
            self.mcp_servers.clear();
            self.plugins.clear();
            self.skills.clear();
        } else {
            self.mcp_servers = mcp_servers;
            self.plugins = plugins;
            self.skills = skills;
        }
    }

    pub fn mcp_rows(&self) -> Vec<InventoryRow> {
        self.mcp_servers
            .iter()
            .map(|server| InventoryRow {
                title: server.name.clone(),
                detail: format!("Status: {}", server.status),
                status: server.status.clone(),
            })
            .collect()
    }

    pub fn plugin_rows(&self) -> Vec<InventoryRow> {
        self.plugins
            .iter()
            .map(|plugin| InventoryRow {
                title: plugin.id.clone(),
                detail: format!("Source: {} · Status: {}", plugin.source, plugin.status),
                status: plugin.status.clone(),
            })
            .collect()
    }

    pub fn skill_rows(&self) -> Vec<InventoryRow> {
        self.skills
            .iter()
            .map(|skill| InventoryRow {
                title: skill.name.clone(),
                detail: if skill.description.is_empty() {
                    "No description".into()
                } else {
                    skill.description.clone()
                },
                status: "Skill".into(),
            })
            .collect()
    }
}

/// Inventory card contents for one integration kind. Loading and error states
/// replace rows, while an empty successful list keeps its section heading.
pub(super) fn inventory_list(
    prefix: &'static str,
    rows: &[InventoryRow],
    empty: &'static str,
    loading: bool,
    error: Option<&str>,
) -> AnyElement {
    let theme = Theme::default();
    if loading {
        let id = format!("{prefix}-loading");
        let selector = id.clone();
        return div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .px(px(18.))
            .py(px(24.))
            .text_size(px(11.5))
            .text_color(theme.faint)
            .child("Loading…")
            .into_any_element();
    }
    if let Some(error) = error {
        let id = format!("{prefix}-error");
        let selector = id.clone();
        return div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .px(px(18.))
            .py(px(20.))
            .text_size(px(11.5))
            .text_color(theme.red)
            .child(SharedString::from(error.to_owned()))
            .into_any_element();
    }
    if rows.is_empty() {
        let id = format!("{prefix}-empty");
        let selector = id.clone();
        return div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .px(px(18.))
            .py(px(24.))
            .text_size(px(11.5))
            .text_color(theme.faint)
            .child(empty)
            .into_any_element();
    }
    let mut list = div().w_full().min_w_0().flex().flex_col();
    for (index, row) in rows.iter().enumerate() {
        let id = format!("{prefix}-row-{index}");
        let selector = id.clone();
        list = list.child(
            setting_row(
                row.title.clone(),
                row.detail.clone(),
                crate::settings::badge(&row.status),
            )
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .when(index + 1 < rows.len(), |row| {
                row.border_b_1().border_color(theme.border)
            }),
        );
    }
    list.into_any_element()
}

impl VelaApp {
    fn preview_toggle_busy(&self) -> bool {
        self.active_id.as_ref().is_some_and(|active| {
            self.preview_pending
                .values()
                .any(|owner| owner.as_ref() == Some(active))
        })
    }
    fn toggle_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview_toggle_busy() {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            return;
        };
        let entry = self
            .active_document_key()
            .map(|key| key.rel)
            .filter(|path| {
                Path::new(path).extension().is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
                }) && !self.browser_open
            });
        self.file_request += 1;
        let request = self.file_request;
        self.preview_pending.insert(request, Some(session.clone()));
        self.preview_notice = None;
        self.engine.toggle_preview(request, session, entry);
        cx.notify();
    }
    fn stop_preview(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        self.file_request += 1;
        let request = self.file_request;
        self.preview_pending.insert(request, None);
        self.preview_menu = None;
        self.engine.stop_preview(request, id);
        cx.notify();
    }
    pub(super) fn render_preview_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::default();
        let busy = self.preview_toggle_busy();
        let servers = self
            .preview_servers
            .iter()
            .filter(|server| Some(&server.workspace_id) == self.active_id.as_ref())
            .collect::<Vec<_>>();
        let count = servers.len();
        let label = if busy {
            "Starting the Vite server…".to_owned()
        } else if count == 1 {
            format!("{} — click to stop, right-click to manage", servers[0].url)
        } else if count > 1 {
            format!(
                "{count} Vite servers running — click to toggle this page, right-click to manage"
            )
        } else {
            "Serve this workspace with Vite and open it in a browser".to_owned()
        };
        div()
            .id("terminal-vite")
            .role(gpui::Role::Button)
            .aria_label(label)
            .focusable()
            .tab_stop(!busy)
            .size(px(22.))
            .flex_shrink_0()
            .rounded(px(999.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(if count > 0 { theme.green } else { theme.dim })
            .when(busy, |button| button.opacity(0.5))
            .when(!busy, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.inset).text_color(theme.text))
            })
            .on_click(cx.listener(|this, _, _, cx| this.toggle_preview(cx)))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    if count > 0 {
                        this.preview_menu = Some(event.position);
                        cx.notify();
                        cx.stop_propagation();
                    }
                }),
            )
            .child(crate::icons::inherited_glyph("globe", 16.))
            .into_any_element()
    }
    fn ensure_integrations_loaded(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.active_id.clone() else {
            if self.integrations.session.is_some() {
                self.integrations = Default::default();
                cx.notify();
            }
            return;
        };
        let same_session = self.integrations.session.as_deref() == Some(session.as_str());
        if same_session && (self.integration_request.is_some() || !self.integrations.loading) {
            return;
        }
        if !same_session {
            self.integrations = IntegrationInventory {
                session: Some(session),
                ..Default::default()
            };
        }
        self.load_integrations(cx);
    }

    fn load_integrations(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.active_id.clone() else {
            self.integrations.error =
                Some("Open a workspace to inspect MCP servers, plugins, and skills.".into());
            self.integrations.loading = false;
            cx.notify();
            return;
        };
        self.file_request += 1;
        let request = self.file_request;
        self.integration_request = Some(request);
        self.integrations.session = Some(session.clone());
        self.integrations.loading = true;
        self.integrations.error = None;
        self.engine.load_integrations(request, session);
        cx.notify();
    }

    pub(super) fn render_settings_servers(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        self.ensure_integrations_loaded(cx);
        let count = self.preview_servers.len();
        let busy = self.preview_pending.values().any(Option::is_none);
        let mut list = div().w_full().min_w_0().flex().flex_col();
        if count == 0 {
            list = list.child(
                div()
                    .px(px(20.))
                    .py(px(32.))
                    .text_size(px(12.))
                    .text_color(theme.faint)
                    .text_center()
                    .child("Start a preview from the server button in the terminal tray."),
            );
        }
        for (index, server) in self.preview_servers.iter().enumerate() {
            let id = server.id.clone();
            let label = server_label(server);
            let button = div()
                .id(SharedString::from(format!("preview-stop-{}", server.id)))
                .role(gpui::Role::Button)
                .aria_label(format!("Stop {label}"))
                .flex_shrink_0()
                .focusable()
                .tab_stop(!busy)
                .px(px(12.))
                .py(px(7.))
                .border_1()
                .border_color(theme.border_strong)
                .rounded(px(999.))
                .bg(theme.inset)
                .text_size(px(10.5))
                .text_color(theme.text)
                .when(busy, |button| button.opacity(0.5))
                .when(!busy, |button| {
                    button
                        .cursor_pointer()
                        .hover(|style| style.text_color(theme.accent).border_color(theme.accent))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !busy {
                        this.stop_preview(Some(id.clone()), cx);
                    }
                }))
                .child("Stop");
            let button = crate::scroll::focus_control(
                SharedString::from(format!("reveal-preview-stop-{}", server.id)),
                self.settings_scroll.clone(),
                !busy,
                button,
            );
            list = list.child(
                setting_row(
                    label,
                    format!("{} · {}", server.url, server.directory),
                    button,
                )
                .when(index + 1 < count, |row| {
                    row.border_b_1().border_color(theme.border)
                }),
            );
        }
        let loading = self.integrations.loading;
        let inventory_error = self.integrations.error.clone();
        let mcp_rows = self.integrations.mcp_rows();
        let plugin_rows = self.integrations.plugin_rows();
        let skill_rows = self.integrations.skill_rows();
        let mut inventory = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .child(settings_group_title("MCP SERVERS"))
            .child(settings_list(inventory_list(
                "inventory-mcp",
                &mcp_rows,
                "No MCP servers configured for this workspace.",
                loading,
                inventory_error.as_deref(),
            )))
            .child(settings_group_title("PLUGINS"))
            .child(settings_list(inventory_list(
                "inventory-plugin",
                &plugin_rows,
                "No plugins loaded for this workspace.",
                loading,
                inventory_error.as_deref(),
            )))
            .child(settings_group_title("SKILLS"))
            .child(settings_list(inventory_list(
                "inventory-skill",
                &skill_rows,
                "No skills are available in this workspace.",
                loading,
                inventory_error.as_deref(),
            )));
        if !loading && inventory_error.is_none() {
            inventory =
                inventory.child(div().text_size(px(10.5)).text_color(theme.faint).child(
                    "MCP servers, plugins, and skills are reported by the connected runtime.",
                ));
        }
        if let Some(error) = inventory_error {
            inventory = inventory.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(11.))
                            .text_color(theme.red)
                            .child(SharedString::from(error)),
                    )
                    .child(
                        div()
                            .id("integrations-retry")
                            .debug_selector(|| "integrations-retry".into())
                            .role(gpui::Role::Button)
                            .aria_label("Retry loading MCP, plugin, and skill inventories")
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.inset)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.selected))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_integrations(cx);
                            }))
                            .child("Retry"),
                    ),
            );
        }
        let body=div().w_full().min_w_0().flex().flex_col()
            .child(crate::settings::callout(if count==0 {"No servers running".to_owned()} else {
                format!("{count} server{} running",if count==1 {""}else{"s"})
            },"Vite previews run on loopback ports while Vela is open and stop when you quit. Stop one here to release its port immediately."))
            .child(settings_list(list))
            .when(count>1,|body|body.child(
                div().id("preview-stop-all").role(gpui::Role::Button).aria_label("Stop all servers").focusable().tab_stop(!busy).self_start()
                    .border_1().border_color(theme.red).rounded(px(12.)).px(px(14.)).py(px(6.))
                    .bg(theme::rgba(226,152,138,0.14)).text_color(theme::rgba(226,152,138,1.)).text_size(px(12.5)).font_weight(FontWeight::MEDIUM)
                    .when(busy,|button|button.opacity(0.45))
                    .when(!busy,|button|button.cursor_pointer().hover(|style|style.bg(theme::rgba(226,152,138,0.22))))
                    .on_click(cx.listener(move |this,_,_,cx| {if !busy {this.stop_preview(None,cx);}})).child("Stop all servers")
            ))
            .child(inventory);
        self.render_settings_body(
            "Servers",
            "See and stop the Vite preview servers running in this app.",
            body.into_any_element(),
            window,
            cx,
        )
    }
    pub(super) fn render_preview_menu(
        &self,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(position) = self.preview_menu else {
            return div().into_any_element();
        };
        let theme = Theme::default();
        let servers = self
            .preview_servers
            .iter()
            .filter(|server| Some(&server.workspace_id) == self.active_id.as_ref())
            .collect::<Vec<_>>();
        let count = servers.len();
        let height = 12. + 27. * (count + usize::from(count > 1)) as f32;
        let mut menu = div()
            .id("preview-context-menu")
            .role(gpui::Role::Menu)
            .absolute()
            .left(position.x)
            .top((position.y - px(height + 6.)).max(px(0.)))
            .min_w(px(176.))
            .p(px(5.))
            .border_1()
            .border_color(theme.border_strong)
            .rounded(px(12.))
            .bg(theme.elevated)
            .shadow(vec![
                gpui::BoxShadow::new(px(0.), px(2.), theme::rgba(0, 0, 0, 0.07).into())
                    .blur_radius(px(6.)),
                gpui::BoxShadow::new(px(0.), px(10.), theme::rgba(0, 0, 0, 0.09).into())
                    .blur_radius(px(24.)),
            ])
            .flex()
            .flex_col();
        for server in servers {
            let id = server.id.clone();
            let root = PathBuf::from(self.workspace_directory());
            let label = preview_stop_label(&root, server);
            menu = menu.child(
                div()
                    .id(SharedString::from(format!("preview-menu-{}", server.id)))
                    .role(gpui::Role::MenuItem)
                    .aria_label(label.clone())
                    .focusable()
                    .tab_stop(true)
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(4.))
                    .text_size(px(12.5))
                    .text_color(theme.text)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.stop_preview(Some(id.clone()), cx)),
                    )
                    .child(label),
            );
        }
        if count > 1 {
            menu = menu.child(
                div()
                    .id("preview-menu-stop-all")
                    .role(gpui::Role::MenuItem)
                    .aria_label("Stop all servers")
                    .focusable()
                    .tab_stop(true)
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(4.))
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected))
                    .on_click(cx.listener(|this, _, _, cx| this.stop_preview(None, cx)))
                    .child("Stop all servers"),
            );
        }
        div()
            .id("preview-menu-layer")
            .absolute()
            .size_full()
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| {
                this.preview_menu = None;
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.preview_menu = None;
                    cx.notify();
                }),
            )
            .child(menu)
            .into_any_element()
    }
}

fn relative_server_directory(root: &Path, directory: &str) -> String {
    let path = Path::new(directory);
    if path == root {
        return String::new();
    }
    if let Ok(relative) = path.strip_prefix(root)
        && !relative.as_os_str().is_empty()
    {
        return format!("{}/", relative.display());
    }
    // A relocated workspace can leave a running server under its old root.
    // Source keeps the full directory in that case instead of pretending the
    // entry belongs to the current workspace.
    format!("{}/", directory.trim_end_matches(['/', '\\']))
}

fn preview_stop_label(root: &Path, server: &PreviewServer) -> String {
    let directory = relative_server_directory(root, &server.directory);
    let entry = if server.entry.is_empty() {
        "index.html"
    } else {
        &server.entry
    };
    format!("Stop {directory}{entry}")
}

fn server_label(server: &PreviewServer) -> String {
    let folder = Path::new(&server.directory)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| server.directory.clone());
    format!(
        "{folder}/{}",
        if server.entry.is_empty() {
            "index.html"
        } else {
            &server.entry
        }
    )
}

use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    fn preview_server(directory: &str, entry: &str) -> PreviewServer {
        PreviewServer {
            id: format!("{directory}:{entry}"),
            workspace_id: "ws".into(),
            directory: directory.into(),
            entry: entry.into(),
            url: "http://127.0.0.1:5199/".into(),
            port: 5199,
        }
    }

    #[test]
    fn preview_stop_labels_keep_stale_relocated_directories_visible() {
        let root = Path::new("/workspace");
        assert_eq!(
            preview_stop_label(root, &preview_server("/workspace", "")),
            "Stop index.html"
        );
        assert_eq!(
            preview_stop_label(root, &preview_server("/workspace", "page.html")),
            "Stop page.html"
        );
        assert_eq!(
            preview_stop_label(root, &preview_server("/workspace/docs", "page.html")),
            "Stop docs/page.html"
        );
        assert_eq!(
            preview_stop_label(root, &preview_server("/workspace2/docs", "page.html")),
            "Stop /workspace2/docs/page.html",
            "a sibling path must not be mistaken for a workspace-relative path"
        );
        assert_eq!(
            preview_stop_label(root, &preview_server("/old/workspace", "")),
            "Stop /old/workspace/index.html",
            "relocated previews keep their old directory visible"
        );
        assert_eq!(
            preview_stop_label(root, &preview_server("/workspace/docs/", "page.html")),
            "Stop docs/page.html"
        );
    }

    fn sample_rows() -> Vec<InventoryRow> {
        vec![
            InventoryRow {
                title: "filesystem".into(),
                detail: "Status: connected".into(),
                status: "connected".into(),
            },
            InventoryRow {
                title: "review".into(),
                detail: "Review a change".into(),
                status: "Skill".into(),
            },
        ]
    }

    #[test]
    fn inventory_rows_map_source_fields_and_errors_clear_payloads() {
        let mut inventory = IntegrationInventory::default();
        inventory.finish(
            "session".into(),
            vec![McpServerInfo {
                name: "filesystem".into(),
                status: "connected".into(),
            }],
            vec![PluginInfo {
                id: "alpha".into(),
                source: "npm".into(),
                status: "loaded".into(),
            }],
            vec![SkillInfo {
                id: "review".into(),
                name: "review".into(),
                description: "Review a change".into(),
            }],
            None,
        );
        assert_eq!(inventory.mcp_rows()[0].title, "filesystem");
        assert_eq!(inventory.mcp_rows()[0].detail, "Status: connected");
        assert_eq!(inventory.plugin_rows()[0].title, "alpha");
        assert_eq!(
            inventory.plugin_rows()[0].detail,
            "Source: npm · Status: loaded"
        );
        assert_eq!(inventory.skill_rows()[0].detail, "Review a change");
        assert!(!inventory.loading);
        assert!(inventory.error.is_none());

        inventory.finish(
            "session".into(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("inventory failed".into()),
        );
        assert!(inventory.mcp_rows().is_empty());
        assert!(inventory.plugin_rows().is_empty());
        assert!(inventory.skill_rows().is_empty());
        assert_eq!(inventory.error.as_deref(), Some("inventory failed"));
    }

    struct InventoryProbe {
        rows: Vec<InventoryRow>,
        loading: bool,
        error: Option<String>,
    }

    impl Render for InventoryProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(inventory_list(
                "inventory-test",
                &self.rows,
                "No entries",
                self.loading,
                self.error.as_deref(),
            ))
        }
    }

    #[gpui::test]
    fn inventory_lists_render_loading_error_empty_and_populated_states(
        cx: &mut gpui::TestAppContext,
    ) {
        for (loading, error, rows, selector) in [
            (true, None, Vec::new(), "inventory-test-loading"),
            (
                false,
                Some("inventory failed".to_string()),
                Vec::new(),
                "inventory-test-error",
            ),
            (false, None, Vec::new(), "inventory-test-empty"),
            (false, None, sample_rows(), "inventory-test-row-0"),
        ] {
            let (probe, cx) = cx.add_window_view(|_, _| InventoryProbe {
                rows,
                loading,
                error,
            });
            cx.simulate_resize(size(px(560.), px(240.)));
            cx.draw(point(px(0.), px(0.)), size(px(560.), px(240.)), |_, _| {
                probe.clone().into_any_element()
            });
            assert!(
                cx.debug_bounds(selector).is_some(),
                "missing inventory state {selector}"
            );
        }
    }
}
