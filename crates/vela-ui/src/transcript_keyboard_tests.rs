//! Focused-owner keyboard regressions using an isolated engine and thin GPUI
//! fixture. No production profile, provider or live terminal is accessed.

use super::*;

struct KeyboardFixture {
    app: gpui::Entity<VelaApp>,
    active_control: gpui::FocusHandle,
    inactive_control: gpui::FocusHandle,
    routed: usize,
}

impl Render for KeyboardFixture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = self.app.read(cx);
        let active = app.transcript_list.clone();
        let inactive = app.panel_tile_lists["inactive"].clone();
        let composer = app.composer_input.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, event, window, cx| {
                if this
                    .app
                    .update(cx, |app, cx| app.route_transcript_key(event, window, cx))
                {
                    this.routed += 1;
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .id("active-control")
                    .track_focus(&self.active_control)
                    .h(px(20.))
                    .child("Active control"),
            )
            .child(
                div()
                    .id("inactive-control")
                    .track_focus(&self.inactive_control)
                    .h(px(20.))
                    .child("Inactive control"),
            )
            .child(composer)
            .child(
                div().h(px(120.)).flex_shrink_0().child(
                    list(active, |index, _, _| {
                        div()
                            .h(px(20.))
                            .child(format!("Active {index}"))
                            .into_any_element()
                    })
                    .size_full(),
                ),
            )
            .child(
                div().h(px(120.)).flex_shrink_0().child(
                    list(inactive, |index, _, _| {
                        div()
                            .h(px(20.))
                            .child(format!("Inactive {index}"))
                            .into_any_element()
                    })
                    .size_full(),
                ),
            )
    }
}

fn fixture(
    cx: &mut gpui::TestAppContext,
    user_text_focus: bool,
) -> (
    gpui::Entity<KeyboardFixture>,
    &mut gpui::VisualTestContext,
    tempfile::TempDir,
    tokio::runtime::Runtime,
) {
    let profile = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let engine = Engine::spawn(
        vela_engine::EngineConfig {
            data_dir: Some(profile.path().join("profile")),
            service_file: Some(profile.path().join("missing-service.json")),
            opencode_binary: Some(profile.path().join("missing-opencode")),
            ..Default::default()
        },
        runtime.handle(),
    );
    runtime.block_on(engine.shutdown_and_wait());
    let settings = engine.settings();
    let pending = Rc::new(RefCell::new(PendingOpenPaths::default()));
    let (view, cx) = cx.add_window_view(move |window, cx| {
        window.activate_window();
        let app = cx.new(|cx| {
            let mut app = VelaApp::new(engine, settings, None, pending, cx);
            app.active_id = Some("active".into());
            app.bootstrapped = true;
            app.loading = false;
            app.agent_panel_open = true;
            app.agent_mode = false;
            app.settings_open = false;
            app.composer = "active draft".into();
            app.composer_input
                .update(cx, |input, cx| input.set_text("active draft".into(), cx));
            app.panels
                .open(crate::panels::PanelState::new("active", "Active"))
                .unwrap();
            app.panels
                .open(crate::panels::PanelState::new("inactive", "Inactive"))
                .unwrap();
            app.panels.activate("active");
            app.transcript_list = ListState::new(100, ListAlignment::Top, px(100.)).measure_all();
            app.panel_tile_lists.insert(
                "inactive".into(),
                ListState::new(100, ListAlignment::Top, px(100.)).measure_all(),
            );
            app.panel_transcripts
                .insert("inactive".into(), Default::default());
            app
        });
        let active_control = cx.focus_handle().tab_stop(true);
        let inactive_control = cx.focus_handle().tab_stop(true);
        app.update(cx, |app, _| {
            app.transcript_ui
                .register_focus_for_test(active_control.clone(), false);
            app.panel_transcripts
                .get_mut("inactive")
                .unwrap()
                .ui
                .register_focus_for_test(inactive_control.clone(), user_text_focus);
            app.turn_anchors.request("inactive", &[], "pending", 100);
        });
        KeyboardFixture {
            app,
            active_control,
            inactive_control,
            routed: 0,
        }
    });
    cx.simulate_resize(size(px(400.), px(400.)));
    (view, cx, profile, runtime)
}

fn draw(view: &gpui::Entity<KeyboardFixture>, cx: &mut gpui::VisualTestContext) {
    cx.draw(point(px(0.), px(0.)), size(px(400.), px(400.)), |_, _| {
        view.clone().into_any_element()
    });
}

fn focus_inactive(view: &gpui::Entity<KeyboardFixture>, cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| window.focus(&view.read(cx).inactive_control.clone(), cx));
}

#[gpui::test]
fn inactive_control_keyboard_scroll_preserves_active_panel_and_composer(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx, _profile, _runtime) = fixture(cx, false);
    draw(&view, cx);
    let app = view.read_with(cx, |view, _| view.app.clone());
    app.update(cx, |app, _| {
        app.transcript_list.scroll_to(ListOffset {
            item_ix: 5,
            offset_in_item: px(3.),
        })
    });
    draw(&view, cx);
    let active_before = app.read_with(cx, |app, _| app.transcript_list.logical_scroll_top());
    focus_inactive(&view, cx);
    for key in ["pagedown", "pageup", "end", "home"] {
        cx.simulate_keystrokes(key);
        draw(&view, cx);
        let top = app.read_with(cx, |app, _| {
            app.panel_tile_lists["inactive"]
                .logical_scroll_top()
                .item_ix
        });
        match key {
            "pagedown" => assert!(top > 0 && top < 20),
            "home" | "pageup" => assert_eq!(top, 0),
            "end" => assert!(top > 90),
            _ => unreachable!(),
        }
    }
    app.read_with(cx, |app, cx| {
        let active_after = app.transcript_list.logical_scroll_top();
        assert_eq!(
            (active_after.item_ix, active_after.offset_in_item),
            (active_before.item_ix, active_before.offset_in_item)
        );
        assert_eq!(app.active_id.as_deref(), Some("active"));
        assert_eq!(app.panels.active().unwrap().session, "active");
        assert_eq!(app.composer, "active draft");
        assert_eq!(app.composer_input.read(cx).buffer.text(), "active draft");
        assert!(
            app.stick,
            "reading an inactive panel must not suspend the active one"
        );
        assert!(
            !app.panels
                .entries()
                .iter()
                .find(|panel| panel.session == "inactive")
                .unwrap()
                .stick
        );
        assert!(!app.panel_tile_lists["inactive"].is_following_tail());
    });
    assert_eq!(view.read_with(cx, |view, _| view.routed), 4);
}

#[gpui::test]
fn user_text_focus_is_a_transcript_owner_and_navigation_cancels_pending_anchor(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx, _profile, _runtime) = fixture(cx, true);
    draw(&view, cx);
    focus_inactive(&view, cx);
    cx.simulate_keystrokes("pagedown");
    draw(&view, cx);
    let app = view.read_with(cx, |view, _| view.app.clone());
    app.update(cx, |app, _| {
        assert!(
            app.panel_tile_lists["inactive"]
                .logical_scroll_top()
                .item_ix
                > 0
        );
        let mut transcript = vela_core::transcript::Transcript::new();
        transcript.hydrate(vec![vela_core::transcript::Message::text(
            "canonical",
            vela_core::transcript::Role::User,
            101,
            "pending",
        )]);
        assert!(
            app.turn_anchors
                .take_match("inactive", transcript.blocks())
                .is_none()
        );
        assert_eq!(app.active_id.as_deref(), Some("active"));
    });
    assert_eq!(view.read_with(cx, |view, _| view.routed), 1);
}

#[gpui::test]
fn composer_and_hidden_settings_tui_or_closed_panels_do_not_route_transcript_keys(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx, _profile, _runtime) = fixture(cx, false);
    draw(&view, cx);
    let app = view.read_with(cx, |view, _| view.app.clone());
    cx.update(|window, cx| {
        let composer = app.read(cx).composer_input.read(cx).focus_handle(cx);
        window.focus(&composer, cx);
    });
    cx.simulate_keystrokes("end");
    draw(&view, cx);
    assert_eq!(view.read_with(cx, |view, _| view.routed), 0);
    focus_inactive(&view, cx);
    for state in ["settings", "hidden", "tui", "closed"] {
        app.update(cx, |app, _| {
            app.settings_open = state == "settings";
            app.agent_panel_open = state != "hidden";
            app.runtime_capabilities.tui = state == "tui";
            if state == "tui" {
                app.settings
                    .agent_panel_modes
                    .insert("inactive".into(), "tui".into());
            }
            if state == "closed" {
                app.panels.close("inactive");
            }
        });
        cx.simulate_keystrokes("pagedown");
        draw(&view, cx);
        assert_eq!(
            view.read_with(cx, |view, _| view.routed),
            0,
            "{state} must not own a visible GUI transcript"
        );
    }
    app.read_with(cx, |app, cx| {
        assert_eq!(
            app.panel_tile_lists["inactive"]
                .logical_scroll_top()
                .item_ix,
            0
        );
        assert_eq!(app.active_id.as_deref(), Some("active"));
        assert_eq!(app.composer_input.read(cx).buffer.text(), "active draft");
    });
}
