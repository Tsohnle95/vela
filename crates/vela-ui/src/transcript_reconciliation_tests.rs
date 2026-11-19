use super::*;
use serde_json::json;
use std::time::Instant;
use tokio::sync::broadcast::error::TryRecvError;

use super::transcript_fixture as fixture;

#[gpui::test]
#[ignore = "release stress measurement; run with --release --ignored --nocapture"]
fn streamed_transcript_release_stress(cx: &mut gpui::TestAppContext) {
    const HISTORY: usize = 1_600;
    const BURSTS: usize = 40;
    const DELTAS: usize = 10;
    const DELTA: &str = " a **streamed** word";
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let count = std::env::var("VELA_STRESS_PANELS").ok()
        .map(|count| count.parse::<usize>().expect("numeric panel count")).unwrap_or(1);
    assert!(matches!(count, 1 | 8));
    let ids = (0..count).map(|index| format!("ses_stress_{index}")).collect::<Vec<_>>();
    let fixture = runtime.block_on(fixture::Fixture::stress(ids.clone(), HISTORY));
    let app_ids = ids.clone();
    let engine = fixture.engine.clone();
    let (app, cx) = cx.add_window_view(move |_, cx| {
        let settings = engine.settings();
        let mut app = VelaApp::new(engine, settings, None, Default::default(), cx);
        app.bootstrapped = true;
        app.loading = false;
        app.workspace_closed = false;
        app.agent_mode = count > 1;
        for id in &app_ids {
            app.panels.open(crate::panels::PanelState::new(id, id)).unwrap();
            app.ensure_panel_input(id, cx);
        }
        app.panels.activate(&app_ids[0]);
        app.active_id = Some(app_ids[0].clone());
        app.refresh_active();
        app.refresh_panel_snapshots();
        app
    });
    let dimensions = size(px(1680.), px(1050.));
    cx.simulate_resize(dimensions);
    let draw = |cx: &mut gpui::VisualTestContext| {
        cx.draw(point(px(0.), px(0.)), dimensions, |_, _| app.clone().into_any_element());
    };
    for _ in 0..3 {
        cx.run_until_parked();
        draw(cx);
    }
    assert!(app.read_with(cx, |app, _| ids.iter().all(|id| app.list_for(id)
        .is_some_and(|list| list.viewport_bounds().size.height > px(0.)))));
    cx.update(|window, cx| app.update(cx, |app, cx| window.focus(&app.composer_focus, cx)));
    for id in &ids {
        fixture.send("session.step.started", json!({"sessionID":id,"assistantMessageID":"stress-live"}));
    }
    let views = ids.iter().map(|id| fixture.engine.view(id).unwrap()).collect::<Vec<_>>();
    let has_source = |len: usize| views.iter().all(|view| {
        view.presentation().blocks.iter().any(|block| block.part_id.as_ref() == "stress-line"
            && block.text.len() == len)
    });
    let mut publication = Vec::new();
    let mut ui_pump = Vec::new();
    let mut draw_samples = Vec::new();
    let mut end_to_draw = Vec::new();
    let mut input = Vec::new();
    for burst in 1..=BURSTS {
        let started = Instant::now();
        for id in &ids {
            for _ in 0..DELTAS {
                fixture.send("session.text.delta", json!({"sessionID":id,
                    "assistantMessageID":"stress-live","partID":"stress-line","ordinal":0,"delta":DELTA}));
            }
        }
        // The first Markdown segment discards initial leading whitespace.
        runtime.block_on(fixture::wait_for(|| {
            has_source(burst * DELTAS * DELTA.len()) || has_source(burst * DELTAS * DELTA.len() - 1)
        }));
        publication.push(started.elapsed());
        let pump_start = Instant::now();
        cx.run_until_parked();
        ui_pump.push(pump_start.elapsed());
        let draw_start = Instant::now();
        draw(cx);
        draw_samples.push(draw_start.elapsed());
        end_to_draw.push(started.elapsed());
        let input_start = Instant::now();
        cx.simulate_keystrokes("x");
        draw(cx);
        input.push(input_start.elapsed());
        assert_eq!(app.read_with(cx, |app, _| app.composer.len()), burst);
        // Ten deltas each 100ms per session when the renderer keeps pace.
        std::thread::sleep(std::time::Duration::from_millis(100).saturating_sub(started.elapsed()));
    }
    for view in &views {
        view.with_transcript(|source| {
            let message = source.message("stress-live").unwrap();
            assert_eq!(message.part("stress-line").unwrap().kind.text(), DELTA.repeat(BURSTS * DELTAS));
        });
    }
    let text = format!("{}\n\n```text\n{}```\n\n| Field | Value |\n| --- | --- |\n{}",
        "a long paragraph ".repeat(4_096), "long fenced output\n".repeat(8_192),
        "| fixture | wide table cell data |\n".repeat(512));
    let output = "0123456789abcdef".repeat(65_536);
    let edge_started = Instant::now();
    for id in &ids {
        fixture.send("session.text.ended", json!({"sessionID":id,"assistantMessageID":"stress-live",
            "partID":"stress-line","ordinal":0,"text":text}));
        fixture.send("session.tool.input.started", json!({"sessionID":id,"assistantMessageID":"stress-live",
            "id":"stress-output","name":"bash"}));
        fixture.send("session.tool.success", json!({"sessionID":id,"assistantMessageID":"stress-live",
            "id":"stress-output","content":[{"type":"text","text":output}]}));
        fixture.send("session.execution.succeeded", json!({"sessionID":id}));
    }
    runtime.block_on(fixture::wait_for(|| views.iter().all(|view| {
        let snapshot = view.presentation();
        !snapshot.phase.is_active() && snapshot.blocks.iter().any(|block| block.output.len() == output.len())
    })));
    let edge_publication = edge_started.elapsed();
    cx.run_until_parked();
    draw(cx);
    let edge_drawn = edge_started.elapsed();
    let scale = cx.update(|window, _| window.scale_factor());
    let report = |label: &str, mut samples: Vec<Duration>| {
        samples.sort_unstable();
        let percentile = |fraction: f64| samples[((samples.len() - 1) as f64 * fraction).round() as usize]
            .as_secs_f64() * 1_000.;
        eprintln!("{label}: p50={:.3} ms p95={:.3} ms max={:.3} ms",
            percentile(0.50), percentile(0.95), percentile(1.0));
    };
    eprintln!("panels={count}, history={HISTORY}/panel, 1680x1050 at {scale}x, {BURSTS} x {DELTAS} deltas/panel");
    report("SSE + publication", publication);
    report("GPUI scheduler/update drain", ui_pump);
    report("GPUI layout + record draw", draw_samples);
    report("SSE + publication + GPUI draw", end_to_draw);
    report("synthetic composer input + draw", input);
    eprintln!("long Markdown + 1 MiB tool per panel: publication={edge_publication:?}, through draw={edge_drawn:?}");
    runtime.block_on(fixture.engine.shutdown_and_wait());
}

#[gpui::test]
fn live_activity_and_delayed_image_keep_preceding_prose_in_place(cx: &mut gpui::TestAppContext) {
    const SESSION: &str = "ses_stream";
    const FIRST: &str = "conversation-row:ses_stream:text:first-prose#0";
    const LAST: &str = "conversation-row:ses_stream:text:last-prose#0";
    const FOOTER: &str = "conversation-row:ses_stream:footer:layout-turn";
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let fixture = runtime.block_on(fixture::Fixture::start());
    let engine = fixture.engine.clone();
    let (app, cx) = cx.add_window_view(move |_, cx| {
        let settings = engine.settings();
        let mut app = VelaApp::new(engine, settings, None, Default::default(), cx);
        app.bootstrapped = true;
        app.loading = false;
        app.active_id = Some(SESSION.into());
        app.panels.open(crate::panels::PanelState::new(SESSION, SESSION)).unwrap();
        app.panels.open(crate::panels::PanelState::new("ses_noise", "Other panel")).unwrap();
        app.panels.activate(SESSION);
        app.stick = false;
        app.refresh_active();
        app.refresh_panel_snapshots();
        app
    });
    let draw = |cx: &mut gpui::VisualTestContext| {
        cx.draw(point(px(0.), px(0.)), size(px(1400.), px(1000.)), |_, _| app.clone().into_any_element());
    };
    let step = |cx: &mut gpui::VisualTestContext, kind: &str, fields: serde_json::Value| {
        let view = fixture.engine.view(SESSION).unwrap();
        let rev = view.presentation().revision;
        let mut data = fields.as_object().unwrap().clone();
        data.insert("sessionID".into(), json!(SESSION));
        data.insert("assistantMessageID".into(), json!("layout-turn"));
        runtime.block_on(async {
            fixture.send(kind, data.into());
            fixture::wait_for(|| view.presentation().revision > rev).await;
        });
        cx.run_until_parked();
        draw(cx);
    };
    let tall_prompt = (0..7)
        .map(|line| format!("Fixture requirement {}: preserve each prior line during the streamed answer.", line + 1))
        .collect::<Vec<_>>().join("\n\n");
    runtime.block_on(async {
        fixture.user_arrived(SESSION, "tall-prompt", &tall_prompt, crate::browser_downloads::now_ms());
        fixture::wait_for(|| fixture.engine.view(SESSION).unwrap().presentation().blocks
            .iter().any(|block| block.message_id.as_ref() == "tall-prompt")).await;
    });
    step(cx, "session.step.started", json!({}));
    assert!(cx.debug_bounds("conversation-row:ses_stream:user:tall-prompt").unwrap().size.height > px(130.));
    step(cx, "session.text.delta", json!({"partID":"first-prose","ordinal":0,"delta":"I will inspect the fixture."}));
    let first = cx.debug_bounds(FIRST).expect("the opening prose is painted");
    step(cx, "session.reasoning.started", json!({"partID":"reason","ordinal":1}));
    step(cx, "session.reasoning.delta", json!({"partID":"reason","ordinal":1,"delta":"Check the inputs."}));
    assert!(cx.debug_bounds("reasoning-trigger:ses_stream:reason").is_none(), "reasoning starts collapsed");
    let work = cx.debug_bounds("tool-group-trigger").unwrap();
    cx.simulate_click(work.center(), gpui::Modifiers::default());
    draw(cx);
    let reason = cx.debug_bounds("reasoning-trigger:ses_stream:reason").unwrap();
    cx.simulate_click(reason.center(), gpui::Modifiers::default());
    draw(cx);
    let reason_body = "conversation-row:ses_stream:reasoning-body:reason";
    assert!(cx.debug_bounds(reason_body).is_some());
    for (event, data) in [
        ("session.tool.input.started", json!({"id":"read-one","name":"read"})),
        ("session.tool.input.started", json!({"id":"read-two","name":"read"})),
        ("session.tool.called", json!({"id":"read-one","input":{"label":"one.txt"}})),
        ("session.tool.progress", json!({"id":"read-one","progress":"Still reading","metadata":{
            "deepseek":{"subCalls":[{"id":"nested-read","title":"Nested read","status":"running",
                "subCalls":[{"id":"nested-leaf","title":"Nested leaf","status":"success"}]}]}
        }})),
        ("session.tool.failed", json!({"id":"read-two","error":{"message":"Unavailable"}})),
        ("session.tool.success", json!({"id":"read-one","content":[{"type":"text","text":"Read complete"}]})),
    ] {
        step(cx, event, data);
        assert_eq!(cx.debug_bounds(FIRST).unwrap().top(), first.top(), "prose moved on {event}");
        assert!(cx.debug_bounds(FOOTER).is_none(), "premature footer on {event}");
        if event == "session.tool.progress" {
            app.read_with(cx, |app, _| {
                let tool = app.blocks.iter().find(|block| block.part_id.as_ref() == "read-one").unwrap();
                assert_eq!(tool.sub_calls[0].sub_calls[0].id, "nested-leaf");
            });
        }
    }
    assert!(cx.debug_bounds(reason_body).is_some(), "explicitly inspected reasoning stays open when tools arrive");
    step(cx, "session.reasoning.ended", json!({"partID":"reason","ordinal":1,"text":"Checked the inputs."}));
    assert!(cx.debug_bounds(reason_body).is_some());
    let permission = || fixture.engine.permissions(SESSION).iter().any(|request| request.id == "layout-approval");
    runtime.block_on(async {
        fixture.send("permission.asked", json!({"sessionID":SESSION,"id":"layout-approval","action":"read","resources":["fixture"]}));
        fixture::wait_for(permission).await;
    });
    cx.run_until_parked();
    draw(cx);
    assert!(cx.debug_bounds("permission-button-Deny").is_some());
    assert!(cx.debug_bounds(FOOTER).is_none());
    assert_eq!(cx.debug_bounds(FIRST).unwrap().top(), first.top());
    runtime.block_on(async {
        fixture.send("permission.replied", json!({"sessionID":SESSION,"requestID":"layout-approval","reply":"reject"}));
        fixture::wait_for(|| !permission()).await;
    });
    let focus = cx.update(|window, cx| window.focused(cx));
    fixture.forms.lock().unwrap().push(json!({
        "id":"layout-question","title":"Continue?","fields":[
            {"key":"confirm","type":"boolean","title":"Confirm","default":false}
        ]
    }));
    runtime.block_on(async {
        fixture.engine.refresh_forms(SESSION);
        fixture::wait_for(|| fixture.engine.forms(SESSION).iter().any(|form| form.id == "layout-question")).await;
    });
    cx.run_until_parked();
    draw(cx);
    assert!(cx.debug_bounds("form-bool-layout-question:confirm").is_some());
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focus);
    assert!(cx.debug_bounds(FOOTER).is_none());
    assert_eq!(cx.debug_bounds(FIRST).unwrap().top(), first.top());
    fixture.forms.lock().unwrap().clear();
    runtime.block_on(async {
        fixture.engine.refresh_forms(SESSION);
        fixture::wait_for(|| fixture.engine.forms(SESSION).is_empty()).await;
    });
    step(cx, "session.retry.scheduled", json!({"attempt":1,"error":{"message":"Retry requested"}}));
    assert!(cx.debug_bounds("retry-notice").is_some());
    assert!(cx.debug_bounds(FOOTER).is_none());
    assert_eq!(cx.debug_bounds(FIRST).unwrap().top(), first.top());
    step(cx, "session.execution.started", json!({}));
    step(cx, "session.text.delta", json!({"partID":"last-prose","ordinal":8,"delta":"Here is the result."}));
    assert!(cx.debug_bounds(LAST).unwrap().top() > first.bottom());
    assert_eq!(cx.debug_bounds(FIRST).unwrap().top(), first.top());
    assert!(cx.debug_bounds(FOOTER).is_none());
    step(cx, "session.execution.succeeded", json!({}));
    assert!(cx.debug_bounds(FOOTER).unwrap().top() > cx.debug_bounds(LAST).unwrap().top());
    assert!(cx.debug_bounds(reason_body).is_some());
    assert!(!app.read_with(cx, |app, _| app.stick));

    let other = app.read_with(cx, |app, _| app.panel_tile_lists["ses_noise"].logical_scroll_top());
    let image = "data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%20width=%22320%22%20height=%22180%22%3E%3Crect%20width=%22320%22%20height=%22180%22/%3E%3C/svg%3E";
    runtime.block_on(async {
        fixture.send("message.updated", json!({
            "sessionID":SESSION,"id":"illustration","type":"assistant","finish":"stop",
            "time":{"created":3,"completed":4},
            "content":[{"id":"image-prose","type":"text","text":format!("![Fixture image]({image})")}]
        }));
        fixture::wait_for(|| fixture.engine.view(SESSION).unwrap().presentation().blocks
            .iter().any(|block| block.part_id.as_ref() == "image-prose")).await;
    });
    // Install the published frame without advancing GPUI's executor: a
    // normal scheduler drain can both render and decode it before we sample.
    app.update(cx, |app, _| app.refresh_active());
    draw(cx);
    let selector = "conversation-row:ses_stream:text:image-prose#0";
    let unresolved = cx.debug_bounds(selector).unwrap();
    for _ in 0..4 {
        cx.run_until_parked();
        draw(cx);
    }
    let resolved = cx.debug_bounds(selector).unwrap();
    assert!(resolved.size.height > unresolved.size.height + px(50.),
        "fixture must exercise a real asynchronous height change: {unresolved:?} -> {resolved:?}");
    let scale = cx.update(|window, _| window.scale_factor());
    let drift = f32::from(cx.debug_bounds(FIRST).unwrap().top() - first.top()).abs() * scale;
    eprintln!("C16 image height {} -> {} logical px; anchor drift {drift} physical px at {scale}x",
        f32::from(unresolved.size.height), f32::from(resolved.size.height));
    assert!(drift <= 1., "delayed image displaced reading anchor by {drift} physical pixels");
    app.read_with(cx, |app, _| {
        let after = app.panel_tile_lists["ses_noise"].logical_scroll_top();
        assert_eq!((after.item_ix, after.offset_in_item), (other.item_ix, other.offset_in_item));
    });
    runtime.block_on(fixture.engine.shutdown_and_wait());
}

#[gpui::test]
fn live_overflow_does_not_move_transcript_or_composer_sideways(cx: &mut gpui::TestAppContext) {
    const SESSION: &str = "ses_stream";
    const PROSE: &str = "label:steady-prose#0:0";
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let fixture = runtime.block_on(fixture::Fixture::start());
    let engine = fixture.engine.clone();
    let (app, cx) = cx.add_window_view(move |_, cx| {
        let settings = engine.settings();
        let mut app = VelaApp::new(engine, settings, None, Default::default(), cx);
        app.bootstrapped = true;
        app.loading = false;
        app.active_id = Some(SESSION.into());
        app.panels.open(crate::panels::PanelState::new(SESSION, SESSION)).unwrap();
        app.stick = false;
        app.refresh_active();
        app
    });
    let dimensions = size(px(1200.), px(480.));
    cx.simulate_resize(dimensions);
    let draw = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.draw(point(px(0.), px(0.)), dimensions, |_, _| app.clone().into_any_element());
        }
    };
    let send = |kind, fields| {
        let view = fixture.engine.view(SESSION).unwrap();
        let revision = view.presentation().revision;
        let mut data: serde_json::Map<String, serde_json::Value> = fields;
        data.insert("sessionID".into(), json!(SESSION));
        data.insert("assistantMessageID".into(), json!("width-turn"));
        runtime.block_on(async {
            fixture.send(kind, data.into());
            fixture::wait_for(|| view.presentation().revision > revision).await;
        });
    };
    let fields = |value: serde_json::Value| value.as_object().unwrap().clone();
    send("session.text.delta", fields(json!({
        "partID":"steady-prose", "ordinal":0, "delta":"Earlier prose stays in place."
    })));
    draw(cx);
    let prose = cx.debug_bounds(PROSE).expect("initial prose");
    let composer = cx.debug_bounds("composer-add").expect("composer");
    let viewport = app.read_with(cx, |app, _| {
        assert_eq!(app.transcript_list.max_offset_for_scrollbar().y, px(0.));
        app.transcript_list.viewport_bounds()
    });
    for _ in 0..3 {
        send("session.text.delta", fields(json!({
            "partID":"variable-prose", "ordinal":1,
            "delta": " This growing response crosses the viewport boundary.".repeat(80)
        })));
        draw(cx);
        app.read_with(cx, |app, _| {
            assert!(app.transcript_list.max_offset_for_scrollbar().y > px(0.));
            let current = app.transcript_list.viewport_bounds();
            assert_eq!((current.left(), current.size.width), (viewport.left(), viewport.size.width));
        });
        let current = cx.debug_bounds(PROSE).expect("prose remains visible");
        assert_eq!((current.left(), current.size.width), (prose.left(), prose.size.width));
        assert_eq!(cx.debug_bounds("composer-add").unwrap().left(), composer.left());
        send("session.text.ended", fields(json!({
            "partID":"variable-prose", "ordinal":1, "text":"Short final text."
        })));
        draw(cx);
        app.read_with(cx, |app, _| {
            assert_eq!(app.transcript_list.max_offset_for_scrollbar().y, px(0.));
            let current = app.transcript_list.viewport_bounds();
            assert_eq!((current.left(), current.size.width), (viewport.left(), viewport.size.width));
        });
        assert_eq!(cx.debug_bounds("composer-add").unwrap().left(), composer.left());
    }
    runtime.block_on(fixture.engine.shutdown_and_wait());
}

#[gpui::test]
fn queued_prompt_and_recovery_retry_anchor_only_on_canonical_arrival(cx: &mut gpui::TestAppContext) {
    const SESSION: &str = "ses_noise";
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let fixture = runtime.block_on(fixture::Fixture::start_with_workspace());
    let engine = fixture.engine.clone();
    let directory = engine.view(SESSION).unwrap().summary().directory;
    let (app, cx) = cx.add_window_view(move |_, cx| {
        let settings = engine.settings();
        let mut app = VelaApp::new(engine, settings, None, Default::default(), cx);
        app.bootstrapped = true;
        app.loading = false;
        app.workspace_closed = false;
        app.sidebar_tab = SidebarTab::Sessions;
        app.active_id = Some(SESSION.into());
        app.panels.open(crate::panels::PanelState::new("ses_stream", "Other panel")).unwrap();
        app.panels.open(crate::panels::PanelState::new(SESSION, SESSION)).unwrap();
        app.panels.activate(SESSION);
        app.settings.follow_up_behavior = vela_engine::message_queue::FollowUpBehavior::Queue;
        app.refresh_active();
        app
    });
    let draw = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.draw(point(px(0.), px(0.)), size(px(1400.), px(900.)), |_, _| {
                app.clone().into_any_element()
            });
            cx.run_until_parked();
        }
    };
    let start_run = || runtime.block_on(async {
        fixture.send("session.execution.started", json!({"sessionID":SESSION}));
        fixture::wait_for(|| fixture.engine.view(SESSION).unwrap().phase().is_active()).await;
    });
    let send = |cx: &mut gpui::VisualTestContext, text: &str| {
        cx.update(|window, cx| app.update(cx, |app, cx| {
            assert!(app.phase.is_active());
            app.composer = text.into();
            app.send(window, cx);
        }));
    };
    let canonical = |id: &str, text: &str| runtime.block_on(async {
        fixture.user_arrived(SESSION, id, text, crate::browser_downloads::now_ms());
        fixture::wait_for(|| fixture.engine.view(SESSION).unwrap().presentation().blocks
            .iter().any(|block| block.message_id.as_ref() == id)).await;
    });
    let assert_anchored = |cx: &mut gpui::VisualTestContext, key: &'static str| {
        draw(cx);
        let row = cx.debug_bounds(key).expect("canonical prompt is visible");
        let viewport = app.read_with(cx, |app, _| app.transcript_list.viewport_bounds());
        assert!(row.top() >= viewport.top() && row.top() <= viewport.top() + px(64.),
            "{key} must anchor near the viewport top: {row:?} {viewport:?}");
    };

    start_run();
    draw(cx);
    send(cx, "queued by runtime");
    runtime.block_on(fixture::wait_for(|| fixture.prompts.lock().unwrap().len() == 1
        && fixture.engine.view(SESSION).unwrap().presentation().blocks.iter()
            .any(|block| block.message_id.starts_with("local:") && block.text.as_ref() == "queued by runtime")));
    assert_eq!(fixture.prompts.lock().unwrap()[0]["delivery"], "queue");
    draw(cx);
    let viewport = app.read_with(cx, |app, _| {
        assert!(app.blocks.iter().any(|block| block.message_id.starts_with("local:")
            && block.text.as_ref() == "queued by runtime"), "active: {:?}; engine revision: {}; UI revision: {}",
            app.active_id, fixture.engine.view(SESSION).unwrap().presentation().revision, app.revision);
        app.transcript_list.viewport_bounds()
    });
    let preceding = cx.debug_bounds("conversation-row:ses_noise:user:user").unwrap();
    assert!(preceding.top() > viewport.top() + px(100.));
    app.update(cx, |app, cx| app.open_session("ses_stream".into(), cx));
    draw(cx);
    runtime.block_on(async {
        fixture.send("session.execution.interrupted", json!({"sessionID":SESSION}));
        fixture::wait_for(|| !fixture.engine.view(SESSION).unwrap().phase().is_active()).await;
    });
    cx.run_until_parked();
    app.read_with(cx, |app, _| assert_eq!(app.active_id.as_deref(), Some("ses_stream")));
    start_run();
    canonical("accepted-queue", "queued by runtime");
    draw(cx);
    app.read_with(cx, |app, _| {
        assert_eq!(app.active_id.as_deref(), Some("ses_stream"));
        assert_eq!(app.panel_transcripts[SESSION].viewport.reserved_turn(), Some("user:accepted-queue"));
    });
    app.update(cx, |app, cx| app.open_session(SESSION.into(), cx));
    app.read_with(cx, |app, _| {
        assert!(app.stick);
        assert_eq!(app.transcript_viewport.reserved_turn(), Some("user:accepted-queue"));
    });
    assert_anchored(cx, "conversation-row:ses_noise:user:accepted-queue");

    send(cx, "recover queued prompt");
    runtime.block_on(fixture::wait_for(|| {
        !fixture.engine.local_queue(&directory, SESSION).is_empty()
            && fixture.engine.view(SESSION).unwrap().presentation().blocks.iter()
                .any(|block| block.text.starts_with("Prompt failed:"))
    }));
    draw(cx);
    assert_eq!(fixture.prompts.lock().unwrap()[1]["delivery"], "queue");
    let queued = fixture.engine.local_queue(&directory, SESSION).pop().unwrap();
    app.read_with(cx, |app, _| {
        assert!(app.blocks.iter().any(|block| block.text.starts_with("Prompt failed:")));
        assert!(app.blocks.iter().filter(|block| block.text.as_ref() == queued.text)
            .all(|block| block.message_id.starts_with("local:")));
    });
    fixture.engine.retry_queued(directory.clone(), SESSION.into(), queued.id);
    runtime.block_on(fixture::wait_for(|| fixture.prompts.lock().unwrap().len() == 3
        && fixture.engine.local_queue(&directory, SESSION).is_empty()));
    assert!(fixture.prompts.lock().unwrap()[2].get("delivery").is_none());
    canonical("accepted-retry", "recover queued prompt");
    assert_anchored(cx, "conversation-row:ses_noise:user:accepted-retry");

    app.update(cx, |app, _| {
        app.transcript_viewport.release_turn_anchor(&app.transcript_list);
        app.transcript_list.scroll_to_end();
    });
    send(cx, "recover queued prompt");
    runtime.block_on(fixture::wait_for(|| !fixture.engine.local_queue(&directory, SESSION).is_empty()));
    let canceled = fixture.engine.local_queue(&directory, SESSION).pop().unwrap();
    app.update(cx, |app, _| app.discard_inbox_item(&InboxItem {
        id: canceled.id, session_id: SESSION.into(), text: canceled.text,
        kind: "local-recovery".into(), delivery: "queue".into(), attachment_count: 0,
    }));
    runtime.block_on(fixture::wait_for(|| fixture.engine.local_queue(&directory, SESSION).is_empty()));
    // A different submitter can now use the exact same text. Its message
    // must join the transcript without stealing the canceled send's anchor.
    canonical("external-after-remove", "recover queued prompt");
    draw(cx);
    let row = cx.debug_bounds("conversation-row:ses_noise:user:external-after-remove").unwrap();
    let viewport = app.read_with(cx, |app, _| app.transcript_list.viewport_bounds());
    assert!(row.top() > viewport.top() + px(100.));

    send(cx, "queued while selecting");
    runtime.block_on(fixture::wait_for(|| fixture.prompts.lock().unwrap().len() == 5));
    draw(cx);
    let bubble = cx.debug_bounds("user-bubble:external-after-remove:text:0#0").unwrap();
    let from = point(bubble.left() + px(14.), bubble.center().y);
    let to = point(bubble.right() - px(14.), bubble.center().y);
    cx.simulate_mouse_down(from, gpui::MouseButton::Left, gpui::Modifiers::default());
    cx.simulate_mouse_move(to, Some(gpui::MouseButton::Left), gpui::Modifiers::default());
    cx.simulate_mouse_up(to, gpui::MouseButton::Left, gpui::Modifiers::default());
    draw(cx);
    let reading = app.read_with(cx, |app, _| {
        assert!(app.transcript_ui.user_text_selection.is_some());
        assert!(!app.stick);
        app.transcript_list.logical_scroll_top()
    });
    canonical("accepted-after-selection", "queued while selecting");
    draw(cx);
    app.read_with(cx, |app, _| {
        assert!(app.transcript_ui.user_text_selection.is_some());
        assert!(!app.transcript_list.is_following_tail());
        let after = app.transcript_list.logical_scroll_top();
        assert_eq!((after.item_ix, after.offset_in_item), (reading.item_ix, reading.offset_in_item));
    });
    runtime.block_on(fixture.engine.shutdown_and_wait());
}

#[gpui::test]
fn lagged_subscription_reconciles_final_conversation_and_retired_panels(
    cx: &mut gpui::TestAppContext,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let fixture = runtime.block_on(fixture::Fixture::start());
    let engine = fixture.engine.clone();
    let initial = engine.view("ses_stream").unwrap().presentation();
    let (app, cx) = cx.add_window_view(move |_, cx| {
        let settings = engine.settings();
        let mut app = VelaApp::new(engine, settings, None, Default::default(), cx);
        app.bootstrapped = true;
        app.loading = false;
        app.active_id = Some("ses_stream".into());
        for id in ["inventory-absent", "ses_noise", "ses_stream"] {
            app.panels.open(crate::panels::PanelState::new(id, id)).unwrap();
        }
        app.panels.activate("ses_stream");
        app.stick = false;
        app.refresh_active();
        app.refresh_panel_snapshots();
        app
    });
    cx.run_until_parked();
    let mut witness = fixture.engine.subscribe();
    runtime.block_on(async {
        fixture.finish_text("ses_stream", "Final **source** 🌍");
        fixture.send("permission.asked", json!({
            "id":"req-retained", "sessionID":"ses_stream", "action":"bash", "resources":["fixture"]
        }));
        fixture.send("session.deleted", json!({"sessionID":"ses_noise"}));
        // Ensure these notifications precede the flood's publication batch;
        // frame batching deliberately emits each session only once per tick.
        fixture::wait_for(|| fixture.engine.session_is_retired("ses_noise")
            && fixture.engine.view("ses_stream").unwrap().presentation().blocks.iter()
                .any(|block| block.text.as_ref() == "Final **source** 🌍")).await;
        fixture.overflow_notifications("ses_flood");
        fixture.finish_text("ses_flood", "Flood delivered");
        fixture::wait_for(|| fixture.engine.view("ses_flood").unwrap().presentation()
            .blocks.iter().any(|block| block.text.as_ref() == "Flood delivered")).await;
    });
    assert!(matches!(witness.try_recv(), Err(TryRecvError::Lagged(_))));
    while let Ok(event) = witness.try_recv() {
        assert!(!matches!(event, EngineUpdate::Session(ref id) if id == "ses_stream"));
        assert!(!matches!(event, EngineUpdate::SessionRemoved(ref id) if id == "ses_noise"));
    }
    cx.run_until_parked();
    app.read_with(cx, |app, _| {
        assert_eq!(app.active_id.as_deref(), Some("ses_stream"));
        assert!(app.revision > initial.revision);
        assert_eq!(app.revision, app.transcript_ui.markdown.snapshot().revision);
        assert!(app.blocks.iter().any(|block| block.text.as_ref() == "Final **source** 🌍"));
        assert!(app.permissions.iter().any(|permission| permission.id == "req-retained"));
        assert!(!app.stick);
        assert!(!app.panels.contains("ses_noise"));
        assert!(!app.panel_transcripts.contains_key("ses_noise"));
        assert!(app.panels.contains("inventory-absent"));
    });
    runtime.block_on(async {
        fixture.send("session.deleted", json!({"sessionID":"ses_stream"}));
        fixture.overflow_notifications("ses_flood");
        fixture.send("session.step.started", json!({"sessionID":"ses_flood", "assistantMessageID":"after-delete"}));
        fixture.send("session.text.delta", json!({"sessionID":"ses_flood", "assistantMessageID":"after-delete", "ordinal":0, "delta":"Second flood delivered"}));
        fixture.send("session.execution.succeeded", json!({"sessionID":"ses_flood"}));
        fixture::wait_for(|| fixture.engine.view("ses_flood").unwrap().presentation()
            .blocks.iter().any(|block| block.text.as_ref() == "Second flood delivered")).await;
    });
    cx.run_until_parked();
    app.read_with(cx, |app, _| {
        assert_ne!(app.active_id.as_deref(), Some("ses_stream"));
        assert!(!app.panels.contains("ses_stream"));
        assert!(app.panels.contains("inventory-absent"));
    });
    runtime.block_on(fixture.engine.shutdown_and_wait());
}
