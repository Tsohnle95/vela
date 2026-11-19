//! Rendered regressions for independent transcript interaction lifetimes.
use super::*;

const ACTIVE: &str = "scope_active";
const INACTIVE: &str = "scope_inactive";
const ACTIVE_CODE: &str = "code-body:scope-active-code:0";
const INACTIVE_CODE: &str = "code-body:scope-inactive-code:0";
const INACTIVE_WRAP: &str = "code-action:scope-inactive-code:0:wrap";
const INACTIVE_COPY: &str = "code-action:scope-inactive-code:0";

struct IsolatedEngine {
    runtime: tokio::runtime::Runtime,
    engine: Engine,
    directory: PathBuf,
}

impl IsolatedEngine {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("vela-transcript-scope-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let engine = Engine::spawn(vela_engine::EngineConfig {
            data_dir: Some(directory.join("profile")),
            service_file: Some(directory.join("absent-service.json")),
            opencode_binary: Some(directory.join("absent-opencode")),
            ..Default::default()
        }, runtime.handle());
        Self { runtime, engine, directory }
    }
}

impl Drop for IsolatedEngine {
    fn drop(&mut self) {
        self.runtime.block_on(self.engine.shutdown_and_wait());
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn code_block(id: &str, name: &str) -> Block {
    Block {
        id: id.into(), message_id: format!("{id}-message").into(), part_id: id.into(),
        kind: BlockKind::Assistant,
        text: format!("```text\n{}\n```", format!("{name} alpha beta gamma delta; ").repeat(8)).into(),
        detail: "".into(), output: "".into(), progress: "".into(),
        status: ToolStatus::None, streaming: false, interrupted: false,
        agent: None, model: None, created_at: 1000, completed_at: Some(2000),
        child_session_id: None, files: Vec::new(), sub_calls: Vec::new(), attachments: Vec::new(),
    }
}

fn panel(session: &str, block: Block) -> crate::panels::PanelState {
    let mut panel = crate::panels::PanelState::new(session, session);
    panel.stick = session != INACTIVE;
    panel.blocks = Arc::new(vec![block]);
    panel.markdown = Arc::new(vela_core::markdown::PreparedMarkdown::refresh(
        &Default::default(), 1, &panel.blocks, &[],
    ));
    panel
}

fn initialize(app: &mut VelaApp, cx: &mut Context<VelaApp>) {
    app.bootstrapped = true;
    app.loading = false;
    app.workspace_closed = false;
    app.active_id = Some(ACTIVE.into());
    app.composer = "Active draft remains untouched".into();
    let active = panel(ACTIVE, code_block("scope-active-code", "ACTIVE"));
    app.blocks = active.blocks.clone();
    app.transcript_ui.markdown.install(active.markdown.clone());
    app.panels.open(active).unwrap();
    app.panels.open(panel(INACTIVE, code_block("scope-inactive-code", "INACTIVE"))).unwrap();
    app.panels.activate(ACTIVE);
    app.rebuild_conversation_rows(ACTIVE, cx);
}

fn draw(app: &gpui::Entity<VelaApp>, cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.draw(point(px(0.), px(0.)), size(px(1200.), px(800.)), |_, _| app.clone().into_any_element());
        cx.run_until_parked();
    }
}

#[gpui::test]
fn inactive_code_wrap_and_copy_preserve_active_session_and_geometry(cx: &mut gpui::TestAppContext) {
    let isolated = IsolatedEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let pending = Rc::new(RefCell::new(PendingOpenPaths::default()));
    let (app, cx) = cx.add_window_view(move |_, cx| VelaApp::new(engine, settings, None, pending, cx));
    cx.simulate_resize(size(px(1200.), px(800.)));
    app.update(cx, initialize);
    draw(&app, cx);
    app.update(cx, |app, _| app.list_for(INACTIVE).unwrap().scroll_to(ListOffset::default()));
    draw(&app, cx);
    let active_before = cx.debug_bounds(ACTIVE_CODE).expect("active code renders");
    let inactive_before = cx.debug_bounds(INACTIVE_CODE).expect("inactive code renders");
    let active_offset = app.read_with(cx, |app, _| app.transcript_list.logical_scroll_top());
    let wrap = cx.debug_bounds(INACTIVE_WRAP).expect("inactive wrap control renders");
    assert!(wrap.center().y > px(48.), "wrap must be inside transcript viewport: {wrap:?}, code {inactive_before:?}");
    cx.simulate_click(wrap.center(), gpui::Modifiers::default());
    draw(&app, cx);
    assert_eq!(app.read_with(cx, |app, _| app.active_id.clone()).as_deref(), Some(ACTIVE),
        "a code control must not bubble into tile activation");
    let inactive_after = cx.debug_bounds(INACTIVE_CODE).unwrap();
    assert!(inactive_after.size.height < inactive_before.size.height,
        "the inactive list must remeasure its own wrapped row: {inactive_before:?} -> {inactive_after:?}");
    assert_eq!(cx.debug_bounds(ACTIVE_CODE).unwrap().size, active_before.size,
        "an inactive wrap cannot change the active code's layout");
    app.read_with(cx, |app, _| {
        assert_eq!(app.active_id.as_deref(), Some(ACTIVE));
        assert_eq!(app.panels.active().unwrap().session, ACTIVE);
        assert_eq!(app.composer, "Active draft remains untouched");
        let retained = app.transcript_list.logical_scroll_top();
        assert_eq!((retained.item_ix, retained.offset_in_item), (active_offset.item_ix, active_offset.offset_in_item));
        assert!(!app.panels.entries().iter().find(|panel| panel.session == INACTIVE).unwrap().stick,
            "the owning panel remains in manual reading mode");
        assert!(app.stick, "active following is independent");
    });
    let copy = cx.debug_bounds(INACTIVE_COPY).unwrap();
    cx.simulate_click(copy.center(), gpui::Modifiers::default());
    draw(&app, cx);
    assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()).as_deref(),
        Some("INACTIVE alpha beta gamma delta; ".repeat(8).as_str()));
    assert_eq!(app.read_with(cx, |app, _| app.active_id.clone()).as_deref(), Some(ACTIVE));
    assert_eq!(cx.debug_bounds(ACTIVE_CODE).unwrap().size, active_before.size);
}

#[gpui::test]
fn stale_copy_timer_cannot_clear_a_reopened_panel_copy(cx: &mut gpui::TestAppContext) {
    let isolated = IsolatedEngine::new();
    let engine = isolated.engine.clone();
    let settings = engine.settings();
    let pending = Rc::new(RefCell::new(PendingOpenPaths::default()));
    let (app, cx) = cx.add_window_view(move |_, cx| VelaApp::new(engine, settings, None, pending, cx));
    cx.simulate_resize(size(px(1200.), px(800.)));
    app.update(cx, initialize);
    draw(&app, cx);
    app.update(cx, |app, _| app.list_for(INACTIVE).unwrap().scroll_to(ListOffset::default()));
    draw(&app, cx);
    let initial_copy = cx.debug_bounds(INACTIVE_COPY).unwrap();
    assert!(initial_copy.center().y > px(48.), "copy must be inside transcript viewport: {initial_copy:?}");
    cx.simulate_click(initial_copy.center(), gpui::Modifiers::default());
    draw(&app, cx);
    let copied_width = cx.debug_bounds(INACTIVE_COPY).unwrap().size.width;
    assert!(copied_width > initial_copy.size.width, "Copied feedback must be visible");
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    app.update(cx, |app, cx| {
        app.finish_close_panel(INACTIVE.into(), cx);
        assert!(!app.panel_transcripts.contains_key(INACTIVE));
        app.panels.open(panel(INACTIVE, code_block("scope-inactive-code", "REOPENED"))).unwrap();
        let key = app.scroll_memory_key(INACTIVE);
        app.panel_scroll_memory.remove(&key);
    });
    draw(&app, cx);
    app.update(cx, |app, _| app.list_for(INACTIVE).unwrap().scroll_to(ListOffset::default()));
    draw(&app, cx);
    let reopened = cx.debug_bounds(INACTIVE_COPY).unwrap();
    assert!(reopened.center().y > px(48.), "reopened copy must be visible: {reopened:?}");
    assert_eq!(reopened.size.width, initial_copy.size.width, "replacement UI starts with Copy");
    cx.simulate_click(reopened.center(), gpui::Modifiers::default());
    draw(&app, cx);
    assert_eq!(cx.debug_bounds(INACTIVE_COPY).unwrap().size.width, copied_width);
    cx.executor().advance_clock(Duration::from_millis(1100));
    cx.run_until_parked();
    draw(&app, cx);
    assert_eq!(cx.debug_bounds(INACTIVE_COPY).unwrap().size.width, copied_width,
        "the closed panel's old timer must not reset the replacement panel's newer feedback");
    assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()).as_deref(),
        Some("REOPENED alpha beta gamma delta; ".repeat(8).as_str()));
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    draw(&app, cx);
    assert_eq!(cx.debug_bounds(INACTIVE_COPY).unwrap().size.width, initial_copy.size.width,
        "the replacement panel's own timer clears its feedback");
    assert_eq!(app.read_with(cx, |app, _| app.active_id.clone()).as_deref(), Some(ACTIVE));
}
