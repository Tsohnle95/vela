use super::*;
use vela_engine::preview::PreviewServer;
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
    pub(super) fn render_settings_servers(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
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
            ));
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

}