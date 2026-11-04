//! Native activity summaries and expandable execution/patch payloads.
use super::*;

const OUTPUT_LIMIT: usize = 6000;

/// Non-shell, non-patch tool output is truncated to Source's 6000-character
/// display cap. Shell output and patch diffs remain complete because Source
/// scrolls them in a bounded region instead of dropping content.
pub(super) fn display_output(text: &str) -> String {
    if text.chars().count() <= OUTPUT_LIMIT {
        return text.to_string();
    }
    let prefix = text.chars().take(OUTPUT_LIMIT).collect::<String>();
    format!("{prefix}\n… (truncated)")
}

/// Match Source's `formatDuration`: whole seconds below a minute, then
/// `Nm Ns`. Tool timestamps are optional, so incomplete calls show no duration.
fn tool_duration(started_at: i64, completed_at: Option<i64>) -> Option<String> {
    let completed_at = completed_at?;
    if completed_at < started_at {
        return None;
    }
    let seconds = (completed_at - started_at).max(0) / 1000;
    if seconds < 60 {
        Some(format!("{seconds}s"))
    } else {
        Some(format!("{}m {}s", seconds / 60, seconds % 60))
    }
}

fn is_shell(value: &str) -> bool {
    let key = value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || *c == '_')
        .collect::<String>();
    matches!(
        key.as_str(),
        "bash" | "shell" | "terminal" | "run_code" | "code"
    )
}

/// Expanded shell output copies Source's `$ command` header plus the full
/// output; other tools copy their complete output (not the truncated display).
pub(super) fn copy_text(block: &Block) -> String {
    if !is_shell(&block.detail) {
        return block.output.to_string();
    }
    let command = serde_json::from_str::<serde_json::Value>(&block.text)
        .ok()
        .and_then(|input| {
            input
                .get("command")
                .or_else(|| input.get("cmd"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(str::to_string)
        })
        .unwrap_or_default();
    match (command.is_empty(), block.output.is_empty()) {
        (false, false) => format!("$ {command}\n\n{}", block.output),
        (false, true) => format!("$ {command}"),
        _ => block.output.to_string(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Exec,
    Read,
    Search,
    List,
    Patch,
    Web,
    Generic,
}
pub(super) struct Presentation {
    pub kind: Kind,
    pub summary: String,
    pub expandable: bool,
}
pub(super) struct RenderState<'a> {
    pub open: bool,
    pub focused: bool,
    pub copied: bool,
    pub copy_focused: bool,
    pub open_keys: &'a std::collections::HashSet<String>,
}
impl Presentation {
    pub fn expandable(&self) -> bool {
        self.expandable
    }
}
pub(super) fn presentation(block: &Block) -> Presentation {
    let key = block
        .detail
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || *c == '_')
        .collect::<String>();
    let kind = match key.as_str() {
        "bash" | "shell" | "terminal" | "run_code" | "code" => Kind::Exec,
        "read" | "read_image" => Kind::Read,
        "grep" | "search_code" => Kind::Search,
        "glob" | "list" | "list_files" => Kind::List,
        "write" | "edit" | "patch" | "apply_patch" => Kind::Patch,
        "websearch" | "web_search" | "search" | "webfetch" | "web_fetch" => Kind::Web,
        _ => Kind::Generic,
    };
    let input = serde_json::from_str::<serde_json::Value>(&block.text).unwrap_or_default();
    let field = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| {
                input
                    .get(*name)
                    .and_then(serde_json::Value::as_str)
                    .filter(|text| !text.trim().is_empty())
            })
            .unwrap_or("")
            .to_string()
    };
    let detail = input
        .get("filePath")
        .and_then(serde_json::Value::as_str)
        .or_else(|| input.get("file_path").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .or_else(|| {
            input
                .get("command")
                .and_then(serde_json::Value::as_str)
                .map(|text| format!("$ {text}"))
        })
        .unwrap_or_default();
    let with_fallback = |text: String| {
        if text.is_empty() {
            detail.clone()
        } else {
            text
        }
    };
    let running = matches!(block.status, ToolStatus::Running | ToolStatus::Streaming);
    let failed = block.status == ToolStatus::Failed;
    let command = with_fallback(field(&["command", "cmd"]))
        .trim()
        .trim_start_matches('$')
        .trim()
        .to_string();
    let path = with_fallback(field(&["filePath", "file_path", "path"]));
    let query = with_fallback(field(&["query", "pattern"]));
    let verb = match kind {
        Kind::Exec => {
            if running {
                "Running"
            } else if failed {
                "Stopped"
            } else {
                "Ran"
            }
        }
        Kind::Patch => {
            if running {
                "Editing"
            } else if failed {
                "Rejected"
            } else {
                "Edited"
            }
        }
        _ => "",
    };
    let summary = match kind {
        Kind::Exec => format!(
            "{verb} {}",
            if command.is_empty() {
                "command"
            } else {
                &command
            }
        ),
        Kind::Read => format!("Read {path}").trim().to_string(),
        Kind::Search => {
            if query.is_empty() {
                "Searched for files".into()
            } else {
                format!("Searched for {query}")
            }
        }
        Kind::List => {
            if path.is_empty() {
                "Listed files".into()
            } else {
                format!("Listed files in {path}")
            }
        }
        Kind::Patch => format!("{verb} {path}").trim().to_string(),
        Kind::Web => {
            if detail.trim().is_empty() {
                "Searched the web".into()
            } else {
                format!("Searched the web for {}", detail.trim())
            }
        }
        Kind::Generic => {
            if detail.trim().is_empty() {
                block.detail.to_string()
            } else {
                format!("{} {}", block.detail, detail.trim())
            }
        }
    };
    let expandable = matches!(kind, Kind::Exec | Kind::Patch)
        || !block.text.is_empty()
        || !block.output.is_empty()
        || !block.files.is_empty()
        || !block.sub_calls.is_empty();
    Presentation {
        kind,
        summary,
        expandable,
    }
}

fn tool_file_rows(
    files: &[vela_core::transcript::ToolFile],
    key_prefix: &str,
    theme: &Theme,
    cx: &mut Context<VelaApp>,
) -> AnyElement {
    let tertiary = theme.text.opacity(0.46);
    let secondary = theme.text.opacity(0.62);
    let mut rows = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(3.))
        .px(px(11.))
        .py(px(8.))
        .border_t_1()
        .border_color(theme.border_soft)
        .child(
            div()
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .text_color(tertiary)
                .child("FILES"),
        );
    for (index, file) in files.iter().enumerate() {
        let label = file.name.clone().unwrap_or_else(|| file.uri.clone());
        let mime = file.mime.clone();
        let url = vela_browser::safe_external_url(&file.uri);
        let debug = format!("tool-file:{key_prefix}:{index}");
        let selector = debug.clone();
        let aria = format!(
            "Open tool file {label}{}",
            mime.as_ref()
                .map(|mime| format!(" ({mime})"))
                .unwrap_or_default()
        );
        let mut row = div()
            .id(SharedString::from(debug))
            .debug_selector(move || selector.clone())
            .role(gpui::Role::Link)
            .aria_label(aria)
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(11.))
            .line_height(px(18.))
            .text_color(if url.is_some() {
                theme.accent
            } else {
                secondary
            })
            .child(crate::tree_icons::glyph(&label))
            .child(div().min_w_0().truncate().child(SharedString::from(label)));
        if let Some(mime) = mime {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(9.5))
                    .text_color(tertiary)
                    .child(SharedString::from(mime)),
            );
        }
        if let Some(url) = url {
            row = row.cursor_pointer().on_click(cx.listener(move |_, _, _, cx| {
                cx.open_url(&url);
            }));
        }
        rows = rows.child(row);
    }
    rows.into_any_element()
}

fn nested_tool_row(
    call: &vela_core::transcript::ToolSubCall,
    key: String,
    debug_key: String,
    theme: &Theme,
    open_keys: &std::collections::HashSet<String>,
    cx: &mut Context<VelaApp>,
) -> AnyElement {
    let expandable = !call.input.trim().is_empty()
        || !call.output.trim().is_empty()
        || !call.files.is_empty()
        || !call.sub_calls.is_empty();
    let open = expandable && open_keys.contains(&key);
    let (status, color) = match call.status {
        ToolStatus::Success => ("Done", theme.green),
        ToolStatus::Failed => ("Failed", theme.red),
        ToolStatus::Running | ToolStatus::Streaming => ("Running", theme.yellow),
        ToolStatus::None => ("Queued", theme.text.opacity(0.46)),
    };
    let group = SharedString::from(format!("tool-subcall-head:{key}"));
    let toggle = key.clone();
    let head = div()
        .id(SharedString::from(format!("tool-subcall-head:{key}")))
        .debug_selector({
            let selector = format!("tool-subcall-head:{debug_key}");
            move || selector.clone()
        })
        .role(gpui::Role::Button)
        .aria_label(format!("{status}: {}", call.title))
        .aria_expanded(open)
        .focusable()
        .tab_stop(expandable)
        .group(group.clone())
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(5.))
        .cursor_pointer()
        .text_size(px(11.))
        .text_color(theme.text.opacity(0.62))
        .hover(|style| style.text_color(theme.text))
        .when(expandable, |head| {
            head.on_click(cx.listener(move |app, _, _, cx| {
                if !app.transcript_ui.tool_open.remove(&toggle) {
                    app.transcript_ui.tool_open.insert(toggle.clone());
                }
                cx.notify();
            }))
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(call.title.clone())),
        )
        .when(!call.detail.trim().is_empty(), |head| {
            head.child(
                div()
                    .min_w_0()
                    .max_w(px(180.))
                    .truncate()
                    .text_size(px(10.))
                    .text_color(theme.text.opacity(0.46))
                    .child(SharedString::from(call.detail.clone())),
            )
        })
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(9.5))
                .text_color(color)
                .child(status),
        )
        .when(expandable, |head| {
            head.child(
                div()
                    .flex_shrink_0()
                    .opacity(if open { 1. } else { 0. })
                    .group_hover(group, |style| style.opacity(1.))
                    .child(glyph(
                        if open {
                            "chevron-down"
                        } else {
                            "chevron-right"
                        },
                        10.,
                        theme.text.opacity(0.46),
                    )),
            )
        });
    let mut container = div()
        .id(SharedString::from(format!("tool-subcall:{key}")))
        .debug_selector({
            let selector = format!("tool-subcall:{debug_key}");
            move || selector.clone()
        })
        .role(gpui::Role::ListItem)
        .aria_label(format!("{status}: {}", call.title))
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .py(px(2.))
        .child(head);
    if open {
        let mut io = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(theme.border_soft);
        for (label, text) in [("IN", &call.input), ("OUT", &call.output)] {
            if text.trim().is_empty() {
                continue;
            }
            io = io.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .px(px(6.))
                    .py(px(5.))
                    .child(
                        div()
                            .w(px(20.))
                            .flex_shrink_0()
                            .text_size(px(9.))
                            .text_color(theme.text.opacity(0.46))
                            .child(label),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(10.5))
                            .line_height(px(16.))
                            .text_color(theme.text.opacity(0.62))
                            .whitespace_normal()
                            .child(display_output(text)),
                    ),
            );
        }
        if !call.files.is_empty() {
            io = io.child(tool_file_rows(
                &call.files,
                &format!("{debug_key}:files"),
                theme,
                cx,
            ));
        }
        if !call.sub_calls.is_empty() {
            let mut children = div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .px(px(6.))
                .py(px(5.))
                .border_t_1()
                .border_color(theme.border_soft)
                .child(
                    div()
                        .text_size(px(9.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.text.opacity(0.46))
                        .child(SharedString::from(format!(
                            "TOOL CALLS · {}",
                            call.sub_calls.len()
                        ))),
                );
            for (index, nested) in call.sub_calls.iter().enumerate() {
                let nested_key = format!("{key}::{index}");
                let nested_debug = format!("{debug_key}:{index}");
                children = children.child(nested_tool_row(
                    nested,
                    nested_key,
                    nested_debug,
                    theme,
                    open_keys,
                    cx,
                ));
            }
            io = io.child(children);
        }
        let io_debug = format!("tool-subcall-io:{debug_key}");
        container = container.child(
            io.id(SharedString::from(format!("tool-subcall-io:{key}")))
                .role(gpui::Role::Region)
                .aria_label("Nested tool input and output")
                .debug_selector(move || io_debug.clone()),
        );
    }
    container.into_any_element()
}

pub(super) fn render(
    block: &Block,
    model: Presentation,
    button: gpui::Stateful<gpui::Div>,
    copy: gpui::Stateful<gpui::Div>,
    state: RenderState<'_>,
    theme: &Theme,
    cx: &mut Context<VelaApp>,
) -> AnyElement {
    let RenderState {
        open,
        focused,
        copied,
        copy_focused,
        open_keys,
    } = state;
    let expandable = model.expandable();
    let tertiary = theme.text.opacity(0.46);
    let secondary = theme.text.opacity(0.62);
    let running = matches!(block.status, ToolStatus::Running | ToolStatus::Streaming);
    let summary = if running {
        div()
            .min_w_0()
            .max_w_full()
            .overflow_hidden()
            .child(shimmer::text(
                model.summary.clone(),
                true,
                tertiary,
                theme.text,
                "tool-shimmer",
                SharedString::from(format!("tool-shimmer:{}", block.id)),
            ))
            .into_any_element()
    } else {
        div()
            .min_w_0()
            .truncate()
            .child(model.summary)
            .into_any_element()
    };
    let group = SharedString::from(format!("tool-head:{}", block.id));
    let head = button
        .group(group.clone())
        .self_start()
        .max_w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(4.))
        .text_size(px(14.))
        .line_height(px(21.))
        .text_color(tertiary)
        .when(expandable, |head| {
            head.aria_expanded(open)
                .debug_selector(|| "tool-trigger".into())
                .cursor_pointer()
                .hover(|head| head.text_color(theme.text))
        })
        .when(!expandable, |head| {
            head.debug_selector(|| "tool-disabled".into())
        })
        .child(summary)
        .when(expandable, |head| {
            head.child(
                div()
                    .flex_shrink_0()
                    .opacity(if open || focused { 1. } else { 0. })
                    .group_hover(group, |style| style.opacity(1.))
                    .child(glyph(
                        if open {
                            "chevron-down"
                        } else {
                            "chevron-right"
                        },
                        12.,
                        tertiary,
                    )),
            )
        });
    let mut container = div().w_full().min_w_0().flex().flex_col().child(head);
    if open && expandable {
        let patch = model.kind == Kind::Patch;
        let generic = !patch && model.kind != Kind::Exec;
        let output = if block.output.is_empty() {
            (if patch { "No changes" } else { "No output" }).to_string()
        } else {
            block.output.to_string()
        };
        let diff = patch
            && output.lines().any(|line| {
                ["@@ ", "+++ ", "--- "]
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
            });
        let mut content = if generic {
            let mut io = div().w_full().min_w_0().flex().flex_col();
            if !block.text.trim().is_empty() {
                io = io.child(
                    div()
                        .debug_selector(|| "tool-io-input".into())
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .px(px(11.))
                        .py(px(9.))
                        .child(
                            div()
                                .w(px(24.))
                                .flex_shrink_0()
                                .text_size(px(10.))
                                .line_height(px(18.))
                                .text_color(tertiary)
                                .child("IN"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(secondary)
                                .whitespace_normal()
                                .child(display_output(&block.text)),
                        ),
                );
            }
            if !block.text.trim().is_empty() && !block.output.trim().is_empty() {
                io = io.child(div().h(px(1.)).w_full().bg(theme.border_soft));
            }
            if !block.output.trim().is_empty() {
                io = io.child(
                    div()
                        .debug_selector(|| "tool-io-output".into())
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .px(px(11.))
                        .py(px(9.))
                        .child(
                            div()
                                .w(px(24.))
                                .flex_shrink_0()
                                .text_size(px(10.))
                                .line_height(px(18.))
                                .text_color(tertiary)
                                .child("OUT"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(secondary)
                                .whitespace_normal()
                                .child(display_output(&block.output)),
                        ),
                );
            }
            io.into_any_element()
        } else if diff {
            div()
                .flex()
                .flex_col()
                .children(output.split('\n').map(|line| {
                    let mut row = div().min_h(px(18.)).child(if line.is_empty() {
                        " ".to_string()
                    } else {
                        line.to_string()
                    });
                    if line.starts_with("@@") {
                        row = row.text_color(tertiary);
                    } else if line.starts_with('+') && !line.starts_with("+++") {
                        row = row.text_color(theme.green).bg(theme.green.opacity(0.1));
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        row = row.text_color(theme.red).bg(theme.red.opacity(0.1));
                    }
                    row
                }))
                .into_any_element()
        } else {
            div().child(output).into_any_element()
        };
        if !block.files.is_empty() {
            let files = tool_file_rows(&block.files, block.id.as_ref(), theme, cx);
            content = div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .child(content)
                .child(files)
                .into_any_element();
        }
        if !block.sub_calls.is_empty() {
            let mut calls = div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(3.))
                .px(px(11.))
                .py(px(8.))
                .border_t_1()
                .border_color(theme.border_soft)
                .child(
                    div()
                        .text_size(px(9.5))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tertiary)
                        .child(SharedString::from(format!(
                            "TOOL CALLS · {}",
                            block.sub_calls.len()
                        ))),
                );
            for (index, call) in block.sub_calls.iter().enumerate() {
                let key = format!("{}::sub::{index}", block.id);
                let debug_key = format!("{}:{index}", block.id);
                calls = calls.child(nested_tool_row(call, key, debug_key, theme, open_keys, cx));
            }
            content = div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .child(content)
                .child(calls)
                .into_any_element();
        }
        if !running {
            let (label, color, icon) = if block.status == ToolStatus::Failed {
                ("Failed", theme.red, "close")
            } else {
                ("Success", theme.green, "check")
            };
            let footer_debug = format!("tool-status-footer:{}", block.id);
            let duration = tool_duration(block.created_at, block.completed_at);
            content = div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .child(content)
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "tool-status-footer:{}",
                            block.id
                        )))
                        .debug_selector(move || footer_debug.clone())
                        .role(gpui::Role::Status)
                        .aria_label(label)
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .px(px(11.))
                        .py(px(6.))
                        .border_t_1()
                        .border_color(theme.border_soft)
                        .text_size(px(10.))
                        .text_color(color)
                        .child(glyph(icon, 11., color))
                        .child(label)
                        .when_some(duration, |footer, duration| {
                            footer.child(
                                div()
                                    .ml_auto()
                                    .text_color(theme.text.opacity(0.46))
                                    .child(SharedString::from(duration)),
                            )
                        }),
                )
                .into_any_element();
        }
        let body = div()
            .id(SharedString::from(format!("tool-output:{}", block.id)))
            .debug_selector(move || {
                if diff {
                    "tool-output-diff".into()
                } else {
                    "tool-output-region".into()
                }
            })
            .role(gpui::Role::Region)
            .aria_label("Scrollable content")
            .tab_index(0)
            .max_h(px(if patch {
                if diff { 240. } else { 160. }
            } else if model.kind == Kind::Exec {
                240.
            } else {
                280.
            }))
            .overflow_x_scroll()
            .overflow_y_scroll()
            .px(px(if generic {
                0.
            } else if patch {
                10.
            } else {
                8.
            }))
            .py(px(if generic { 0. } else { 8. }))
            .font(theme::source_editor_font())
            .text_size(px(12.))
            .line_height(px(18.))
            .text_color(theme.text)
            .when(!generic && (model.kind == Kind::Exec || diff), |body| {
                body.whitespace_nowrap()
            })
            .when(!generic && !(model.kind == Kind::Exec || diff), |body| {
                body.whitespace_normal()
            })
            .child(content);
        let copy_value = copy_text(block);
        let copy_available = !copy_value.is_empty();
        let copy_group = SharedString::from(format!("tool-card:{}", block.id));
        let copy_selector = if copied || copy_focused {
            "tool-copy-visible"
        } else {
            "tool-copy"
        };
        let copy = copy
            .debug_selector(move || copy_selector.into())
            .rounded(px(4.))
            .p(px(4.))
            .opacity(if copied || copy_focused { 1. } else { 0. })
            .group_hover(copy_group.clone(), |style| style.opacity(1.))
            .cursor_pointer()
            .hover(|style| style.bg(theme.text.opacity(0.06)))
            .child(glyph(if copied { "check" } else { "copy" }, 14., tertiary));
        let card = div()
            .group(copy_group)
            .relative()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .rounded(px(8.))
            .border_1()
            .border_color(if patch {
                theme.border_soft
            } else {
                theme.border_strong
            })
            .bg(if patch {
                theme.text.opacity(0.04)
            } else {
                theme.inset
            })
            .flex()
            .flex_col()
            .child(body);
        let card = if copy_available {
            card.child(div().absolute().top(px(4.)).right(px(4.)).child(copy))
        } else {
            card
        };
        let card = card.when(
            !patch && !matches!(block.status, ToolStatus::Running | ToolStatus::Streaming),
            |card| {
                card.child(
                    div()
                        .flex()
                        .justify_end()
                        .items_center()
                        .gap(px(4.))
                        .px(px(10.))
                        .pt(px(2.))
                        .pb(px(4.))
                        .text_size(px(12.))
                        .line_height(px(15.))
                        .text_color(tertiary)
                        .when(block.status != ToolStatus::Failed, |footer| {
                            footer.child(glyph("check", 11., tertiary))
                        })
                        .child(if block.status == ToolStatus::Failed {
                            "Failed"
                        } else {
                            "Success"
                        }),
                )
            },
        );
        container = container.child(div().pt(px(8.)).min_w_0().child(card));
    }
    container.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activity_summaries_match_reference_tool_kinds_and_terminal_states() {
        for (name, input, status, summary, expandable) in [
            (
                "bash",
                r#"{"command":"printf hello"}"#,
                ToolStatus::Success,
                "Ran printf hello",
                true,
            ),
            (
                "shell",
                r#"{"cmd":"$ sleep 1"}"#,
                ToolStatus::Running,
                "Running sleep 1",
                true,
            ),
            (
                "bash",
                r#"{"command":"fixture fail"}"#,
                ToolStatus::Failed,
                "Stopped fixture fail",
                true,
            ),
            (
                "read",
                r#"{"filePath":"/fixture/a.txt"}"#,
                ToolStatus::Success,
                "Read /fixture/a.txt",
                true,
            ),
            (
                "grep",
                r#"{"pattern":"🌍"}"#,
                ToolStatus::Success,
                "Searched for 🌍",
                true,
            ),
            ("glob", "{}", ToolStatus::Success, "Listed files", true),
            (
                "edit",
                r#"{"path":"src/a.rs"}"#,
                ToolStatus::Failed,
                "Rejected src/a.rs",
                true,
            ),
            (
                "websearch",
                r#"{"query":"fixture"}"#,
                ToolStatus::Success,
                "Searched the web",
                true,
            ),
            (
                "custom",
                r#"{"file_path":"/fixture/a"}"#,
                ToolStatus::Success,
                "custom /fixture/a",
                true,
            ),
        ] {
            let block = Block {
                id: "p".into(),
                message_id: "m".into(),
                part_id: "p".into(),
                kind: BlockKind::Tool,
                text: input.into(),
                detail: name.into(),
                output: "output".into(),
                status,
                streaming: false,
                agent: None,
                model: None,
                created_at: 1,
                completed_at: Some(2),
                child_session_id: None,
                files: Vec::new(),
                sub_calls: Vec::new(),
                attachments: Vec::new(),
            };
            let model = presentation(&block);
            assert_eq!(model.summary, summary);
            assert_eq!(model.expandable(), expandable);
        }
    }

    #[test]
    fn copy_text_includes_the_shell_command_and_untruncated_output() {
        let mut shell = Block {
            id: "p".into(),
            message_id: "m".into(),
            part_id: "p".into(),
            kind: BlockKind::Tool,
            text: r#"{"command":"printf hi"}"#.into(),
            detail: "bash".into(),
            output: "界".repeat(OUTPUT_LIMIT + 100).into(),
            status: ToolStatus::Success,
            streaming: false,
            agent: None,
            model: None,
            created_at: 1,
            completed_at: Some(2),
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        let copied = copy_text(&shell);
        assert!(copied.starts_with("$ printf hi\n\n"));
        assert!(copied.chars().count() > OUTPUT_LIMIT + 100);
        shell.detail = "read".into();
        assert_eq!(copy_text(&shell), shell.output.as_ref());
    }

    #[test]
    fn tool_duration_matches_source_seconds_and_minutes_wording() {
        assert_eq!(tool_duration(1_000, Some(1_000)), Some("0s".into()));
        assert_eq!(tool_duration(1_000, Some(1_999)), Some("0s".into()));
        assert_eq!(tool_duration(1_000, Some(12_000)), Some("11s".into()));
        assert_eq!(tool_duration(1_000, Some(61_000)), Some("1m 0s".into()));
        assert_eq!(tool_duration(1_000, Some(125_000)), Some("2m 4s".into()));
        assert_eq!(tool_duration(1_000, None), None);
        assert_eq!(tool_duration(2_000, Some(1_000)), None);
    }

    #[test]
    fn large_output_truncates_at_source_limit_and_preserves_unicode() {
        assert_eq!(display_output("short"), "short");
        let exact = "x".repeat(OUTPUT_LIMIT);
        assert_eq!(display_output(&exact), exact);
        let large = "界".repeat(OUTPUT_LIMIT + 1);
        let prefix = "界".repeat(OUTPUT_LIMIT);
        assert_eq!(display_output(&large), format!("{prefix}\n… (truncated)"));
    }
}
