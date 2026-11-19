//! Deterministic native interaction probes. All data and engine roots are synthetic.
use super::*;

use super::transcript_fixture as fixture;

const ACTIVE: &str = "stability-active";
const OTHER: &str = "stability-other";

struct TestEngine {
    runtime: tokio::runtime::Runtime,
    engine: Engine,
    _profile: tempfile::TempDir,
}

impl TestEngine {
    fn new() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let engine = Engine::spawn(
            vela_engine::EngineConfig {
                data_dir: Some(profile.path().join("profile")),
                service_file: Some(profile.path().join("absent-service.json")),
                opencode_binary: Some(profile.path().join("absent-opencode")),
                ..Default::default()
            },
            runtime.handle(),
        );
        Self {
            runtime,
            engine,
            _profile: profile,
        }
    }
}

impl Drop for TestEngine {
    fn drop(&mut self) {
        self.runtime.block_on(self.engine.shutdown_and_wait());
    }
}

fn block(id: &str, kind: BlockKind, text: &str) -> Block {
    Block {
        id: id.into(),
        message_id: id.into(),
        part_id: id.into(),
        kind,
        text: text.into(),
        detail: "".into(),
        output: "".into(),
        progress: "".into(),
        status: ToolStatus::None,
        streaming: false,
        interrupted: false,
        agent: None,
        model: None,
        created_at: 1000,
        completed_at: Some(2000),
        child_session_id: None,
        files: Vec::new(),
        sub_calls: Vec::new(),
        attachments: Vec::new(),
    }
}

fn history() -> Vec<Block> {
    let mut blocks = (0..64)
        .map(|index| {
            block(
                &format!("history-{index}"),
                BlockKind::User,
                &format!("Historical request {index}: retain this reading position."),
            )
        })
        .collect::<Vec<_>>();
    blocks.push(block(
        "current-request",
        BlockKind::User,
        "Inspect the synthetic workspace.",
    ));
    let mut live = block(
        "live-answer",
        BlockKind::Assistant,
        "I am inspecting the workspace.",
    );
    live.streaming = true;
    live.completed_at = None;
    blocks.push(live);
    blocks
}

fn initialize(app: &mut VelaApp, cx: &mut Context<VelaApp>) {
    app.bootstrapped = true;
    app.loading = false;
    app.workspace_closed = false;
    app.agent_mode = true;
    app.active_id = Some(ACTIVE.into());
    app.phase = SessionPhase::Busy;
    app.stick = true;
    for session in [ACTIVE, OTHER] {
        let mut panel = crate::panels::PanelState::new(session, session);
        panel.blocks = Arc::new(history());
        panel.phase = SessionPhase::Busy;
        panel.stick = true;
        panel.markdown = Arc::new(vela_core::markdown::PreparedMarkdown::refresh(
            &Default::default(),
            1,
            &panel.blocks,
            &[],
        ));
        if session == ACTIVE {
            app.blocks = panel.blocks.clone();
            app.transcript_ui.markdown.install(panel.markdown.clone());
        }
        app.panels.open(panel).unwrap();
        app.ensure_panel_input(session, cx);
    }
    app.panels.activate(ACTIVE);
    app.rebuild_conversation_rows(ACTIVE, cx);
}

fn draw(app: &gpui::Entity<VelaApp>, cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.draw(point(px(0.), px(0.)), size(px(1440.), px(900.)), |_, _| {
            app.clone().into_any_element()
        });
    }
}

fn wheel(cx: &mut gpui::VisualTestContext, bounds: Bounds<Pixels>, delta: f32) {
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: bounds.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(delta))),
        ..Default::default()
    });
}

#[gpui::test]
fn manual_scroll_to_live_edge_resumes_only_the_owning_panel(cx: &mut gpui::TestAppContext) {
    let isolated = TestEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx
        .add_window_view(move |_, cx| VelaApp::new(engine, settings, None, Default::default(), cx));
    cx.simulate_resize(size(px(1440.), px(900.)));
    app.update(cx, initialize);
    draw(&app, cx);
    let viewport = app.read_with(cx, |app, _| app.list_for(OTHER).unwrap().viewport_bounds());
    assert!(viewport.size.height > px(0.));
    wheel(cx, viewport, 300.);
    draw(&app, cx);
    app.read_with(cx, |app, _| {
        assert!(
            !app.panels
                .entries()
                .iter()
                .find(|panel| panel.session == OTHER)
                .unwrap()
                .stick
        );
        assert!(!app.list_for(OTHER).unwrap().is_following_tail());
        assert!(
            app.stick,
            "reading the inactive tile cannot release active following"
        );
        assert_eq!(app.active_id.as_deref(), Some(ACTIVE));
    });
    wheel(cx, viewport, -100_000.);
    draw(&app, cx);
    app.read_with(cx, |app, _| {
        assert!(
            app.panels
                .entries()
                .iter()
                .find(|panel| panel.session == OTHER)
                .unwrap()
                .stick
        );
        let list = app.list_for(OTHER).unwrap();
        assert!(
            list.is_following_tail(),
            "manual return to the edge must resume following"
        );
        assert!(crate::scroll::list_at_live_edge(&list, px(0.)));
        assert!(app.stick);
    });
    let (viewport, cached_id, height) = app.read_with(cx, |app, _| {
        let (id, height) = app.transcript_rows.iter().find_map(|row| {
            app.row_cache.measured_height(ACTIVE, &row.id).map(|height| (row.id.clone(), height))
        }).expect("settled rows are measured before scrolling");
        (app.transcript_list.viewport_bounds(), id, height)
    });
    wheel(cx, viewport, 300.);
    app.read_with(cx, |app, _| {
        assert_eq!(app.row_cache.measured_height(ACTIVE, &cached_id), Some(height),
            "wheel input must keep settled row measurements");
    });
    draw(&app, cx);
    assert!(!app.read_with(cx, |app, _| app.stick));
    wheel(cx, viewport, -100_000.);
    draw(&app, cx);
    assert!(app.read_with(cx, |app, _| app.stick
        && app.transcript_list.is_following_tail()));
    // The focused list has bottom padding. A one-pixel upward scroll inside
    // that padding is still manual reading intent and must not be stolen.
    wheel(cx, viewport, 1.);
    draw(&app, cx);
    assert!(!app.read_with(cx, |app, _| app.stick));
    cx.update(|window, cx| {
        app.update(cx, |app, cx| {
            let focus = cx.focus_handle();
            app.transcript_ui
                .register_focus_for_test(focus.clone(), false);
            window.focus(&focus, cx);
        })
    });
    for (key, platform, following) in [
        ("end", false, true),
        ("up", false, false),
        ("down", true, true),
    ] {
        cx.update(|window, cx| {
            app.update(cx, |app, cx| {
                let mut keystroke = gpui::Keystroke::parse(key).unwrap();
                keystroke.modifiers.platform = platform;
                let event = KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                };
                assert!(app.route_transcript_key(&event, window, cx));
            })
        });
        draw(&app, cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.stick),
            following,
            "keyboard navigation must apply the focused panel's live-edge intent"
        );
    }
}

#[gpui::test]
fn reading_anchor_stays_within_two_pixels_as_markdown_and_tool_status_grow(
    cx: &mut gpui::TestAppContext,
) {
    let isolated = TestEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx
        .add_window_view(move |_, cx| VelaApp::new(engine, settings, None, Default::default(), cx));
    cx.simulate_resize(size(px(1440.), px(900.)));
    app.update(cx, initialize);
    draw(&app, cx);
    let viewport = app.read_with(cx, |app, _| app.transcript_list.viewport_bounds());
    wheel(cx, viewport, 600.);
    draw(&app, cx);
    let selector = app.read_with(cx, |app, _| {
        assert!(!app.stick);
        let index = app.transcript_list.logical_scroll_top().item_ix;
        format!(
            "conversation-row:{ACTIVE}:{}",
            app.transcript_rows[index + 1].id
        )
    });
    let selector: &'static str = Box::leak(selector.into_boxed_str());
    let before = cx
        .debug_bounds(selector)
        .expect("reading anchor is painted");
    for step in 1..=20 {
        app.update(cx, |app, cx| {
            let mut blocks = (*app.blocks).clone();
            let live = blocks.iter_mut().find(|block| block.id.as_ref() == "live-answer").unwrap();
            live.text = format!("I am inspecting the workspace.\n\n```rust\n{}\n```\n\n| Item | Result |\n| --- | --- |\n{}",
                "let stable = true;\n".repeat(step), "| source | checked |\n".repeat(step)).into();
            if step == 1 {
                let mut tool = block("tail-tool", BlockKind::Tool, "{\"filePath\":\"src/main.rs\"}");
                tool.detail = "read".into();
                tool.status = ToolStatus::Running;
                tool.streaming = true;
                tool.completed_at = None;
                blocks.push(tool);
            }
            if step == 10 {
                let tool = blocks.iter_mut().find(|block| block.id.as_ref() == "tail-tool").unwrap();
                tool.status = ToolStatus::Failed;
                tool.streaming = false;
                tool.completed_at = Some(2500);
                tool.output = "Synthetic file is unavailable".into();
            }
            app.blocks = Arc::new(blocks);
            app.transcript_ui.markdown.install(Arc::new(vela_core::markdown::PreparedMarkdown::refresh(
                &Default::default(), step as u64 + 1, &app.blocks, &[],
            )));
            app.rebuild_conversation_rows(ACTIVE, cx);
        });
        draw(&app, cx);
        let after = cx
            .debug_bounds(selector)
            .expect("stream updates retain the reading anchor");
        let drift: f32 = (after.top() - before.top()).abs().into();
        assert!(drift <= 2., "step {step} moved the anchor by {drift}px");
        assert!(
            !app.read_with(cx, |app, _| app.stick),
            "source changes cannot re-arm following"
        );
    }
}

#[gpui::test]
fn explicit_tool_disclosure_and_focus_survive_streaming_retry_and_new_activity(
    cx: &mut gpui::TestAppContext,
) {
    let isolated = TestEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx
        .add_window_view(move |_, cx| VelaApp::new(engine, settings, None, Default::default(), cx));
    cx.simulate_resize(size(px(1440.), px(900.)));
    app.update(cx, |app, cx| {
        initialize(app, cx);
        let mut blocks = (*app.blocks).clone();
        let mut edit = block(
            "inspect-edit",
            BlockKind::Tool,
            "{\"filePath\":\"src/main.rs\"}",
        );
        edit.message_id = "live-answer".into();
        edit.detail = "edit".into();
        edit.status = ToolStatus::Running;
        edit.streaming = true;
        edit.completed_at = None;
        edit.output = "Synthetic execution detail".into();
        blocks.insert(blocks.len() - 1, edit);
        app.blocks = Arc::new(blocks);
        app.rebuild_conversation_rows(ACTIVE, cx);
    });
    draw(&app, cx);
    let trace = cx
        .debug_bounds("tool-group-trigger")
        .expect("running work has one compact disclosure line");
    cx.simulate_click(trace.center(), gpui::Modifiers::default());
    draw(&app, cx);
    let trigger = cx
        .debug_bounds("tool-trigger")
        .expect("the active operation is inspectable");
    cx.simulate_click(trigger.center(), gpui::Modifiers::default());
    draw(&app, cx);
    let expanded = cx
        .debug_bounds("tool-output-region")
        .expect("explicit tool details are painted");
    let focus = cx.update(|window, cx| window.focused(cx));
    assert!(focus.is_some());
    let selector = "conversation-row:stability-active:activity:inspect-edit";
    let anchor = cx.debug_bounds(selector).unwrap();
    for (step, status) in [ToolStatus::Failed, ToolStatus::Running, ToolStatus::Success]
        .into_iter()
        .enumerate()
    {
        app.update(cx, |app, cx| {
            let mut blocks = (*app.blocks).clone();
            let edit = blocks
                .iter_mut()
                .find(|block| block.id.as_ref() == "inspect-edit")
                .unwrap();
            edit.status = status;
            edit.streaming = status == ToolStatus::Running;
            edit.completed_at = (status != ToolStatus::Running).then_some(2600 + step as i64);
            let live = blocks
                .iter_mut()
                .find(|block| block.id.as_ref() == "live-answer")
                .unwrap();
            live.text = format!(
                "The answer continues while the operation is inspected. {}",
                "More detail. ".repeat(step + 1)
            )
            .into();
            if step == 0 {
                let mut next = block(
                    "later-read",
                    BlockKind::Tool,
                    "{\"filePath\":\"src/lib.rs\"}",
                );
                next.message_id = "live-answer".into();
                next.detail = "read".into();
                next.status = ToolStatus::Running;
                next.completed_at = None;
                blocks.push(next);
            }
            app.blocks = Arc::new(blocks);
            app.transcript_ui.markdown.install(Arc::new(
                vela_core::markdown::PreparedMarkdown::refresh(
                    &Default::default(),
                    step as u64 + 2,
                    &app.blocks,
                    &[],
                ),
            ));
            app.rebuild_conversation_rows(ACTIVE, cx);
        });
        draw(&app, cx);
        assert!(app.read_with(cx, |app, _| {
            app.transcript_ui
                .tool_open
                .contains("stability-active:inspect-edit")
        }));
        assert_eq!(
            cx.update(|window, cx| window.focused(cx)),
            focus,
            "stream state and sibling arrivals cannot replace the disclosure focus owner"
        );
        let body = cx
            .debug_bounds("tool-output-region")
            .expect("explicit expansion remains painted");
        assert_eq!(body.size.height, expanded.size.height);
        assert_eq!(
            cx.debug_bounds(selector).unwrap().top(),
            anchor.top(),
            "tool identity and reading geometry survive error, retry and success"
        );
    }
}

#[gpui::test]
fn settled_prose_view_does_not_render_again_when_only_the_live_tail_changes(
    cx: &mut gpui::TestAppContext,
) {
    const SESSION: &str = "ses_stream";
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let fixture = runtime.block_on(fixture::Fixture::start());
    let engine = fixture.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx.add_window_view(move |_, cx| {
        let mut app = VelaApp::new(engine, settings, None, Default::default(), cx);
        app.bootstrapped = true;
        app.loading = false;
        app.workspace_closed = false;
        app.active_id = Some(SESSION.into());
        app.stick = false;
        app.panels
            .open(crate::panels::PanelState::new(SESSION, SESSION))
            .unwrap();
        app.panels.activate(SESSION);
        app.refresh_active();
        app
    });
    cx.simulate_resize(size(px(1440.), px(900.)));
    // Use the actual GPUI draw lifecycle. VisualTestContext::draw deliberately
    // calls window.refresh(), which invalidates every cached view and cannot
    // measure streaming-only invalidation behavior.
    let frame = |cx: &mut gpui::VisualTestContext| {
        cx.run_until_parked();
        cx.update(|window, cx| {
            drop(window.draw(cx));
        });
    };
    for _ in 0..5 {
        app.update(cx, |_, cx| cx.notify());
        frame(cx);
    }
    let count = app
        .read_with(cx, |app, _| {
            app.row_cache.render_count(SESSION, "text:seed_text#0")
        })
        .expect("the historical prose has a native view render counter");
    assert!(
        count > 0,
        "the measured view must have actually rendered before this assertion"
    );
    // Cached prepaint reuses the scene without repopulating debug selectors;
    // query the owning native list's measured row bounds instead.
    let bounds = |app: &VelaApp| {
        let index = app
            .transcript_rows
            .iter()
            .position(|row| row.id.as_ref() == "text:seed_text#0")?;
        app.transcript_list.bounds_for_item(index)
    };
    let before = app
        .read_with(cx, |app, _| bounds(app))
        .expect("the settled paragraph is measured");
    for step in 1..=12 {
        let view = fixture.engine.view(SESSION).unwrap();
        let revision = view.presentation().revision;
        runtime.block_on(async {
            fixture.send(
                "session.text.delta",
                serde_json::json!({
                    "sessionID":SESSION, "assistantMessageID":"live-answer", "partID":"live-tail",
                    "ordinal":0, "delta":" Another streamed word."
                }),
            );
            fixture::wait_for(|| view.presentation().revision > revision).await;
        });
        frame(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app
                .row_cache
                .render_count(SESSION, "text:seed_text#0")),
            Some(count),
            "a tail-only revision cannot render the settled paragraph again at step {step}"
        );
        assert_eq!(
            app.read_with(cx, |app, _| bounds(app)).unwrap(),
            before,
            "render reuse must retain the native paragraph bounds"
        );
    }
    runtime.block_on(fixture.engine.shutdown_and_wait());
}

#[gpui::test]
fn rich_history_and_sibling_agents_reuse_their_views_when_one_agent_finishes(
    cx: &mut gpui::TestAppContext,
) {
    let isolated = TestEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx
        .add_window_view(move |_, cx| VelaApp::new(engine, settings, None, Default::default(), cx));
    cx.simulate_resize(size(px(1440.), px(900.)));
    app.update(cx, |app, cx| {
        initialize(app, cx);
        let mut blocks = (*app.blocks).clone();
        blocks.push(block(
            "settled-rich",
            BlockKind::Assistant,
            "**Settled** history with `inline code` and emphasis.",
        ));
        for (id, description) in [
            ("agent-first", "Inspect files"),
            ("agent-second", "Check behavior"),
        ] {
            let mut agent = block(
                id,
                BlockKind::Tool,
                &serde_json::json!({
                    "agent":"review", "description":description
                })
                .to_string(),
            );
            agent.detail = "task".into();
            agent.message_id = "parallel-run".into();
            agent.status = ToolStatus::Running;
            agent.streaming = true;
            agent.completed_at = None;
            blocks.push(agent);
        }
        app.blocks = Arc::new(blocks);
        app.transcript_ui.markdown.install(Arc::new(
            vela_core::markdown::PreparedMarkdown::refresh(
                &Default::default(),
                1,
                &app.blocks,
                &[],
            ),
        ));
        app.rebuild_conversation_rows(ACTIVE, cx);
        app.transcript_list.scroll_to_end();
    });
    let frame = |cx: &mut gpui::VisualTestContext| {
        cx.run_until_parked();
        cx.update(|window, cx| {
            drop(window.draw(cx));
        });
    };
    for _ in 0..5 {
        // First measure naturally, then prime the native retained-view scene.
        app.update(cx, |_, cx| cx.notify());
        frame(cx);
    }
    let ids = [
        "text:settled-rich",
        "activity:agent-first",
        "activity:agent-second",
    ];
    let before = app.read_with(cx, |app, _| {
        ids.map(|id| {
            app.row_cache
                .render_count(ACTIVE, id)
                .expect("visible row has a render counter")
        })
    });
    app.update(cx, |app, cx| {
        let mut blocks = (*app.blocks).clone();
        let first = blocks
            .iter_mut()
            .find(|block| block.id.as_ref() == "agent-first")
            .unwrap();
        first.status = ToolStatus::Success;
        first.streaming = false;
        first.completed_at = Some(3000);
        app.blocks = Arc::new(blocks);
        app.rebuild_conversation_rows(ACTIVE, cx);
    });
    frame(cx);
    app.read_with(cx, |app, _| {
        let after = ids.map(|id| app.row_cache.render_count(ACTIVE, id).unwrap());
        assert_eq!(
            after[0], before[0],
            "settled rich Markdown does not render for agent progress"
        );
        assert!(
            after[1] > before[1],
            "the changed agent actually renders its final state"
        );
        assert_eq!(
            after[2], before[2],
            "finishing one agent cannot render its sibling again"
        );
    });
}

#[gpui::test]
fn hundred_twenty_tools_render_as_one_disclosure_and_keep_all_leaf_ids(
    cx: &mut gpui::TestAppContext,
) {
    let isolated = TestEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx
        .add_window_view(move |_, cx| VelaApp::new(engine, settings, None, Default::default(), cx));
    cx.simulate_resize(size(px(1440.), px(900.)));
    app.update(cx, |app, cx| {
        initialize(app, cx);
        let mut blocks = (*app.blocks).clone();
        for index in 0..120 {
            let mut call = block(
                &format!("bulk-{index}"),
                BlockKind::Tool,
                &serde_json::json!({"filePath":format!("src/file-{index}.rs")}).to_string(),
            );
            call.message_id = "live-answer".into();
            call.detail = "read".into();
            call.status = ToolStatus::Success;
            call.completed_at = Some(call.created_at + 100);
            call.output = "Synthetic file contents".into();
            blocks.push(call);
        }
        app.blocks = Arc::new(blocks);
        app.rebuild_conversation_rows(ACTIVE, cx);
        app.transcript_list.scroll_to_end();
    });
    draw(&app, cx);
    assert!(
        cx.debug_bounds("tool-trigger").is_none(),
        "quiet calls must not become 120 default cards"
    );
    assert!(
        cx.debug_bounds("tool-input-details").is_none(),
        "raw arguments remain behind explicit disclosure"
    );
    let ids = app.read_with(cx, |app, _| {
        app.transcript_rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        ids.iter()
            .filter(|id| id.starts_with("trace-line:"))
            .count(),
        1
    );
    assert_eq!(
        ids.iter()
            .filter(|id| id.starts_with("activity:bulk-"))
            .count(),
        120
    );
    let trace = cx
        .debug_bounds("tool-group-trigger")
        .expect("one compact work summary is visible");
    cx.simulate_click(trace.center(), gpui::Modifiers::default());
    draw(&app, cx);
    assert!(
        cx.debug_bounds("tool-trigger").is_some(),
        "expanded work exposes inspectable calls"
    );
    assert!(
        cx.debug_bounds("tool-input-details").is_none(),
        "call details still require explicit inspection"
    );
    assert_eq!(
        app.read_with(cx, |app, _| app
            .transcript_rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>()),
        ids,
        "opening a trace changes neither leaf keys nor their sibling positions"
    );
}

#[gpui::test]
fn screenshot_reasoning_and_read_commands_are_one_closed_native_line(
    cx: &mut gpui::TestAppContext,
) {
    let isolated = TestEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx
        .add_window_view(move |_, cx| VelaApp::new(engine, settings, None, Default::default(), cx));
    cx.simulate_resize(size(px(1440.), px(900.)));
    app.update(cx, |app, cx| {
        initialize(app, cx);
        let mut blocks = vec![block("request", BlockKind::User, "Inspect the project and explain its structure.")];
        let mut commentary = block("commentary", BlockKind::Assistant, "I'll run the checks now.");
        commentary.message_id = "step-0".into();
        blocks.push(commentary);
        let commands = [
            "ls -la back-end docs assets; cat package.json; find docs -type f | head -40; echo '---'; git status --short",
            "git log --oneline -15; git status --short | head -20; git branch -a",
            "wc -l docs/*.js 2>/dev/null; echo '---'; head -60 docs/main.js",
            "for f in $(find assets/backups -name main.js | head -3); do echo \"$f: $(wc -l < \"$f\") lines\"; grep -c fetch \"$f\"; done",
            "cat back-end/src/app.js; echo '--- routes'; cat back-end/src/routes/chatRoutes.js",
            "grep -rn fetch docs --include=*.js | head",
        ];
        for (index, command) in commands.iter().enumerate() {
            let mut thought = block(&format!("thinking-{index}"), BlockKind::Reasoning, "Inspecting the next part of the project.");
            thought.message_id = format!("step-{index}").into();
            blocks.push(thought);
            let mut call = block(&format!("probe-{index}"), BlockKind::Tool,
                &serde_json::json!({"command": command}).to_string());
            call.message_id = format!("step-{index}").into();
            call.detail = "bash".into();
            call.status = ToolStatus::Success;
            call.output = "warnings.md\nconst warning = 'warning: text';\nconsole.warn('warning');\n// warning colors".into();
            blocks.push(call);
        }
        blocks.push(block("answer", BlockKind::Assistant, "The project has a frontend, a backend, and shared assets."));
        app.blocks = Arc::new(blocks);
        app.phase = SessionPhase::Idle;
        app.rebuild_conversation_rows(ACTIVE, cx);
    });
    draw(&app, cx);
    assert_eq!(
        app.read_with(cx, |app, _| app
            .transcript_rows
            .iter()
            .filter(|row| row.id.starts_with("trace-line:"))
            .count()),
        1,
        "internal message footers do not split consecutive work"
    );
    assert!(
        cx.debug_bounds("work-summary:Read files, ran commands")
            .is_some()
    );
    assert!(
        cx.debug_bounds("tool-trigger").is_none(),
        "commands are hidden by default"
    );
    for index in 0..6 {
        let selector: &'static str =
            Box::leak(format!("reasoning-trigger:{ACTIVE}:thinking-{index}").into_boxed_str());
        assert!(
            cx.debug_bounds(selector).is_none(),
            "internal thoughts do not create default transcript rows"
        );
    }
    let trace = cx.debug_bounds("tool-group-trigger").unwrap();
    cx.simulate_click(trace.center(), gpui::Modifiers::default());
    draw(&app, cx);
    assert!(
        cx.debug_bounds("tool-trigger").is_some(),
        "all underlying calls remain inspectable"
    );
    assert!(
        cx.debug_bounds("reasoning-trigger:stability-active:thinking-0")
            .is_none(),
        "expanded tool work does not interleave internal thought headers"
    );
    assert!(
        cx.debug_bounds("conversation-row:stability-active:footer:step-0").is_none(),
        "commentary copy controls do not appear inside tool work"
    );
}

#[gpui::test]
fn mouse_wheel_ticks_move_the_transcript_across_multiple_frames(cx: &mut gpui::TestAppContext) {
    let isolated = TestEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let (app, cx) = cx
        .add_window_view(move |_, cx| VelaApp::new(engine, settings, None, Default::default(), cx));
    cx.simulate_resize(size(px(1440.), px(900.)));
    app.update(cx, initialize);
    draw(&app, cx);

    app.update(cx, |app, cx| {
        for session in [ACTIVE, OTHER] {
            app.list_for(session).unwrap().measure_all();
        }
        cx.notify();
    });
    draw(&app, cx);

    let detent = |cx: &mut gpui::VisualTestContext, list: &ListState, lines| {
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: list.viewport_bounds().center(),
            delta: gpui::ScrollDelta::Lines(point(0., lines)),
            ..Default::default()
        });
    };
    let advance = |cx: &mut gpui::VisualTestContext| {
        cx.background_executor.advance_clock(Duration::from_millis(16));
        cx.update(|window, cx| window.simulate_next_frame(cx));
        draw(&app, cx);
    };

    for session in [ACTIVE, OTHER] {
        let list = app.read_with(cx, |app, _| app.list_for(session).unwrap());
        let before = list.scroll_px_offset_for_scrollbar().y;
        detent(cx, &list, 3.);
        assert_eq!(
            list.scroll_px_offset_for_scrollbar().y, before,
            "a detent must not move the transcript by its full distance before the first frame in {session}",
        );
        assert!(!list.is_following_tail(), "wheel intent releases follow immediately in {session}");

        let mut previous = before;
        let mut moving_frames = 0;
        for _ in 0..24 {
            advance(cx);
            let current = list.scroll_px_offset_for_scrollbar().y;
            assert!(current >= previous, "scroll must move monotonically");
            assert!(current - previous < px(30.), "a frame must not repeat the detent bump");
            moving_frames += usize::from(current > previous);
            previous = current;
        }
        assert!(moving_frames >= 4, "a detent must be spread across several frames");
        assert!((previous - before - px(60.)).abs() < px(0.1));
    }

    let list = app.read_with(cx, |app, _| app.transcript_list.clone());
    let before = list.scroll_px_offset_for_scrollbar().y;
    detent(cx, &list, 1.);
    detent(cx, &list, 2.);
    for _ in 0..24 {
        advance(cx);
    }
    assert!((list.scroll_px_offset_for_scrollbar().y - before - px(60.)).abs() < px(0.1),
        "a burst must retain the distance of each tick");

    detent(cx, &list, 3.);
    advance(cx);
    let before = list.scroll_px_offset_for_scrollbar().y;
    detent(cx, &list, -1.);
    advance(cx);
    assert!(list.scroll_px_offset_for_scrollbar().y < before,
        "reversal must change direction on the next frame");

    let before = list.scroll_px_offset_for_scrollbar().y;
    wheel(cx, list.viewport_bounds(), 12.);
    let precise = list.scroll_px_offset_for_scrollbar().y;
    assert!(precise > before, "precise input moves immediately");
    for _ in 0..24 {
        advance(cx);
    }
    assert_eq!(list.scroll_px_offset_for_scrollbar().y, precise,
        "precise input cancels any pending mouse-wheel animation");

    cx.update(|_, cx| cx.set_reduce_motion(true));
    detent(cx, &list, 1.);
    assert!(list.scroll_px_offset_for_scrollbar().y > precise,
        "reduced motion uses immediate native scrolling");
    cx.update(|_, cx| cx.set_reduce_motion(false));
    draw(&app, cx);
    detent(cx, &list, -20.);
    for _ in 0..24 {
        advance(cx);
    }
    assert!(list.is_following_tail(), "a detent reaching bottom resumes follow");
    assert!(!app.read_with(cx, |app, _| app.list_for(OTHER).unwrap().is_following_tail()));
}
