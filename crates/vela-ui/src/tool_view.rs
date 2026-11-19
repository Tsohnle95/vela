//! Native activity summaries and expandable execution/patch payloads.
use super::*;

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
pub(super) struct RenderState {
    pub open: bool,
    pub focused: bool,
}
impl Presentation {
    pub fn expandable(&self) -> bool {
        self.expandable
    }
}

fn display_path(path: &str, workspace: Option<&str>) -> String {
    let path = std::path::Path::new(path);
    if let Some(root) = workspace.filter(|root| !root.is_empty())
        && let Ok(relative) = path.strip_prefix(root)
    {
        return if relative.as_os_str().is_empty() {
            ".".into()
        } else {
            relative.display().to_string()
        };
    }
    path.display().to_string()
}

pub(super) fn presentation(block: &Block, workspace: Option<&str>) -> Presentation {
    let key = block
        .detail
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || *c == '_')
        .collect::<String>();
    let kind = match key.as_str() {
        "bash" | "shell" | "terminal" | "exec_command" | "run_code" | "code" => Kind::Exec,
        "read" | "read_file" | "read_image" => Kind::Read,
        "grep" | "rg" | "search_code" | "search_files" => Kind::Search,
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
    let path = field(&["filePath", "file_path", "path", "directory"]);
    let path = display_path(&path, workspace);
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
            if command.is_empty() || command.len() > 64 || command.contains(['\n', '\r']) {
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
    // Every tool with details remains inspectable in the compact timeline.
    // Exec/patch can open before their first output arrives; ordinary empty
    // summary rows do not expose an inert disclosure control.
    let expandable = matches!(kind, Kind::Exec | Kind::Patch)
        || !block.output.is_empty()
        || !block.progress.is_empty()
        || !block.files.is_empty()
        || !block.sub_calls.is_empty();
    let expandable = expandable || !matches!(block.text.trim(), "" | "{}" | "null");
    Presentation {
        kind,
        summary,
        expandable,
    }
}

fn status_label(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::None => "",
        ToolStatus::Streaming => "Preparing tool input",
        ToolStatus::Running => "Running",
        ToolStatus::Success => "Success",
        ToolStatus::Failed => "Failed",
    }
}

/// A fixed-size status gutter keeps the summary in place as the call settles.
/// Static indicators deliberately work identically with reduced motion: no
/// per-character animation, flashing label or animation backlog is needed to
/// convey a tool's state.
fn status_indicator(status: ToolStatus, theme: &Theme) -> AnyElement {
    let tertiary = theme.text.opacity(0.46);
    let selector = match status {
        ToolStatus::None => "tool-status-none",
        ToolStatus::Streaming => "tool-status-streaming",
        ToolStatus::Running => "tool-status-running",
        ToolStatus::Success => "tool-status-success",
        ToolStatus::Failed => "tool-status-failed",
    };
    div()
        .id("tool-status")
        .debug_selector(move || selector.into())
        .w(px(16.))
        .h(px(21.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .when(status != ToolStatus::None, |indicator| {
            indicator
                .role(gpui::Role::Image)
                .aria_label(status_label(status))
        })
        .child(match status {
            ToolStatus::Running | ToolStatus::Streaming => {
                glyph("working-spinner", 11., tertiary).into_any_element()
            }
            ToolStatus::Success => glyph("check", 11., tertiary).into_any_element(),
            ToolStatus::Failed => glyph("close", 11., theme.red).into_any_element(),
            ToolStatus::None => div()
                .size(px(4.))
                .rounded_full()
                .bg(tertiary)
                .into_any_element(),
        })
        .into_any_element()
}

fn output_text(block: &Block, patch: bool) -> String {
    fn append_files(output: &mut String, files: &[vela_core::transcript::ToolFile]) {
        for file in files {
            if !output.is_empty() {
                output.push('\n');
            }
            if let Some(name) = &file.name {
                output.push_str(name);
                output.push_str(" · ");
            }
            output.push_str(&file.uri);
        }
    }
    fn append_calls(output: &mut String, calls: &[vela_core::transcript::ToolSubCall]) {
        for call in calls {
            for field in [
                call.title.as_str(),
                call.detail.as_str(),
                status_label(call.status),
                call.input.as_str(),
                call.output.as_str(),
            ] {
                if !field.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(field);
                }
            }
            append_files(output, &call.files);
            append_calls(output, &call.sub_calls);
        }
    }
    let mut output = if block.output.is_empty() {
        block.progress.to_string()
    } else {
        block.output.to_string()
    };
    append_files(&mut output, &block.files);
    append_calls(&mut output, &block.sub_calls);
    if output.is_empty() {
        (if patch { "No changes" } else { "No output" }).into()
    } else {
        output
    }
}

/// A bounded output card shared by every compact activity row. Its stable
/// output element ID retains the native scroll offset while output updates.
pub(super) fn card(block: &Block, theme: &Theme, patch: bool) -> AnyElement {
    let tertiary = theme.text.opacity(0.46);
    let output = output_text(block, patch);
    let diff = patch
        && output.lines().any(|line| {
            ["@@ ", "+++ ", "--- "]
                .iter()
                .any(|prefix| line.starts_with(prefix))
        });
    let content = if diff {
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
                    row = row.text_color(theme.text.opacity(0.46));
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
        } else {
            144.
        }))
        .overflow_x_scroll()
        .overflow_y_scroll()
        .px(px(if patch { 10. } else { 8. }))
        .py(px(8.))
        .font(theme::source_editor_font())
        .text_size(px(12.))
        .line_height(px(18.))
        .text_color(theme.text)
        .when(patch && !diff, |body| body.whitespace_normal())
        .when(!patch || diff, |body| body.whitespace_nowrap())
        .when(!block.text.is_empty(), |body| body.child(
            div().debug_selector(|| "tool-input-details".into())
                .mb(px(8.)).text_color(tertiary)
                .child("Input\n").child(block.text.to_string())
        ))
        .child(content);
    let card = div()
        .id(SharedString::from(format!("tool-card:{}", block.id)))
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
        .child(body)
        .when(!patch, |card| {
            card.child(
                div()
                    .debug_selector(|| "tool-output-status".into())
                    .flex()
                    .h(px(21.))
                    .flex_shrink_0()
                    .justify_end()
                    .items_center()
                    .gap(px(4.))
                    .px(px(10.))
                    .pt(px(2.))
                    .pb(px(4.))
                    .text_size(px(12.))
                    .line_height(px(15.))
                    .text_color(tertiary)
                    .when(block.status == ToolStatus::Success, |footer| {
                        footer.child(glyph("check", 11., tertiary))
                    })
                    .child(status_label(block.status)),
            )
        });
    card.into_any_element()
}

pub(super) fn render(
    block: &Block,
    model: Presentation,
    button: gpui::Stateful<gpui::Div>,
    state: RenderState,
    theme: &Theme,
    _cx: &mut Context<VelaApp>,
) -> AnyElement {
    let RenderState { open, focused } = state;
    let expandable = model.expandable();
    let tertiary = theme.text.opacity(0.46);
    let summary = div()
        .debug_selector(|| "tool-summary".into())
        .min_w_0()
        .truncate()
        .child(model.summary);
    let group = SharedString::from(format!("tool-head:{}", block.id));
    let head = button
        .group(group.clone())
        .self_start()
        .max_w_full()
        .min_w_0()
        .flex()
        .items_center()
        .min_h(px(21.))
        .gap(px(6.))
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
        .child(status_indicator(block.status, theme))
        .child(summary)
        .child(
            div()
                .w(px(12.))
                .h(px(21.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .opacity(if !expandable { 0. } else if open || focused { 1. } else { 0.48 })
                .when(expandable, |chevron| {
                    chevron.group_hover(group, |style| style.opacity(1.))
                })
                .child(glyph(
                    if open {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    },
                    12.,
                    tertiary,
                )),
        );
    let mut container = div().w_full().min_w_0().flex().flex_col().child(head);
    if open && expandable {
        let patch = model.kind == Kind::Patch;
        container = container.child(div().pt(px(8.)).min_w_0().child(card(block, theme, patch)));
    }
    container.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_summaries_and_payload_disclosures_cover_tool_kinds_and_states() {
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
                progress: Arc::from(""),
                interrupted: false,
            };
            let model = presentation(&block, None);
            assert_eq!(model.summary, summary);
            assert_eq!(model.expandable(), expandable);
        }
    }

    fn fixture(status: ToolStatus) -> Block {
        Block {
            id: "payload".into(),
            message_id: "assistant".into(),
            part_id: "payload".into(),
            kind: BlockKind::Tool,
            text: "{}".into(),
            detail: "custom".into(),
            output: "".into(),
            progress: "".into(),
            interrupted: false,
            status,
            streaming: matches!(status, ToolStatus::Running | ToolStatus::Streaming),
            agent: None,
            model: None,
            created_at: 1,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        }
    }

    #[test]
    fn compact_file_tool_rows_use_paths_relative_to_their_workspace() {
        let mut block = fixture(ToolStatus::Success);
        block.detail = "read".into();
        block.text = r#"{"filePath":"/fixture/project/public/includes/header.php"}"#.into();
        assert_eq!(
            presentation(&block, Some("/fixture/project")).summary,
            "Read public/includes/header.php"
        );
        assert_eq!(
            presentation(&block, Some("/fixture/pro")).summary,
            "Read /fixture/project/public/includes/header.php"
        );
        assert!(presentation(&block, Some("/fixture/project")).expandable());
    }

    #[test]
    fn empty_generic_rows_stay_compact_and_actual_details_become_inspectable() {
        let mut block = fixture(ToolStatus::Running);
        assert!(!presentation(&block, None).expandable());
        block.progress = "Reading the next page".into();
        assert!(presentation(&block, None).expandable());
        assert_eq!(output_text(&block, false), "Reading the next page");
        block.progress = "".into();
        block.output = "Full output\nwith all lines\n".into();
        assert!(presentation(&block, None).expandable());
        assert_eq!(output_text(&block, false), block.output.as_ref());
        block.output = "".into();
        block.files.push(vela_core::transcript::ToolFile {
            uri: "file:///fixture/report.txt".into(),
            name: Some("report.txt".into()),
            mime: None,
        });
        assert!(presentation(&block, None).expandable());
        assert!(output_text(&block, false).contains("file:///fixture/report.txt"));
    }

    #[test]
    fn nested_tool_details_retain_all_input_output_and_file_payloads() {
        let mut block = fixture(ToolStatus::Success);
        block.output = "top-level".into();
        block.sub_calls.push(vela_core::transcript::ToolSubCall {
            id: "nested".into(),
            title: "read".into(),
            input: r#"{"path":"fixture"}"#.into(),
            output: "nested output".into(),
            status: ToolStatus::Success,
            sub_calls: vec![vela_core::transcript::ToolSubCall {
                id: "deep".into(),
                output: "deep output".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        let output = output_text(&block, false);
        for expected in [
            "top-level",
            "read",
            r#"{"path":"fixture"}"#,
            "nested output",
            "deep output",
        ] {
            assert!(output.contains(expected), "missing payload {expected}");
        }
    }

    struct CardFixture {
        block: Block,
    }

    impl Render for CardFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_start()
                .child(status_indicator(self.block.status, &theme))
                .child(card(
                    &self.block,
                    &theme,
                    presentation(&self.block, None).kind == Kind::Patch,
                ))
        }
    }

    #[gpui::test]
    fn status_changes_keep_gutter_and_large_output_geometry_stable(cx: &mut gpui::TestAppContext) {
        let mut block = fixture(ToolStatus::Running);
        block.output = "fixture output\n".repeat(500).into();
        let (view, cx) = cx.add_window_view(move |_, _| CardFixture { block });
        cx.simulate_resize(size(px(400.), px(400.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(400.), px(400.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        let gutter = cx.debug_bounds("tool-status-running").unwrap();
        let body = cx.debug_bounds("tool-output-region").unwrap();
        let footer = cx.debug_bounds("tool-output-status").unwrap();
        assert_eq!(gutter.size, size(px(16.), px(21.)));
        assert!(body.size.height <= px(144.));
        assert!(cx.debug_bounds("tool-shimmer").is_none());
        for (status, selector) in [
            (ToolStatus::Streaming, "tool-status-streaming"),
            (ToolStatus::Success, "tool-status-success"),
            (ToolStatus::Failed, "tool-status-failed"),
            (ToolStatus::None, "tool-status-none"),
        ] {
            view.update(cx, |view, cx| {
                view.block.status = status;
                cx.notify();
            });
            draw(cx);
            assert_eq!(cx.debug_bounds(selector).unwrap(), gutter);
            assert_eq!(cx.debug_bounds("tool-output-region").unwrap(), body);
            assert_eq!(cx.debug_bounds("tool-output-status").unwrap(), footer);
            assert!(cx.debug_bounds("tool-shimmer").is_none());
        }
    }
}
