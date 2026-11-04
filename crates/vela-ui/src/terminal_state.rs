//! One workspace owns an ordered set of independently retained PTY views.
use gpui::{AppContext, Context, FocusHandle};
use std::{cell::Cell, rc::Rc, time::Duration};
use vela_terminal::emulator::Emulator;

pub(super) type Geometry = (gpui::Bounds<gpui::Pixels>, f32);

pub(super) const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(530);

/// Focus-aware blink phase. The timer exists only while the cursor is blinking
/// and its terminal owns focus; otherwise the cursor stays steadily visible.
pub(super) struct CursorBlink {
    visible: bool,
    blinking: bool,
    focused: bool,
    running: bool,
    generation: u64,
}

impl CursorBlink {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            visible: true,
            blinking: false,
            focused: false,
            running: false,
            generation: 0,
        }
    }

    pub fn update_state(&mut self, blinking: bool, focused: bool, cx: &mut Context<Self>) {
        self.blinking = blinking;
        self.focused = focused;
        if !(blinking && focused) {
            if self.running {
                self.running = false;
                self.generation = self.generation.wrapping_add(1);
            }
            if !self.visible {
                self.visible = true;
                cx.notify();
            }
            return;
        }
        if self.running {
            return;
        }
        self.running = true;
        self.visible = true;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CURSOR_BLINK_INTERVAL).await;
                let keep = this
                    .update(cx, |blink, cx| {
                        if !blink.running || blink.generation != generation {
                            return false;
                        }
                        blink.visible = !blink.visible;
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub fn cursor_visible(&self) -> bool {
        self.visible
    }

}

pub(super) struct DirectoryRequest {
    pub session: String,
    pub workspace: String,
    pub generation: u64,
    pub name: Option<String>,
}
impl DirectoryRequest {
    pub fn current(&self, tabs: &Tabs, workspace: &str, session: Option<&str>) -> bool {
        self.workspace == workspace
            && Some(self.session.as_str()) == session
            && tabs.owns(workspace, session)
            && self.generation == tabs.generation
    }
}

pub(super) struct View {
    pub id: String,
    pub name: Option<String>,
    pub emulator: Emulator,
    pub focus: FocusHandle,
    pub tab_focus: FocusHandle,
    pub close_focus: FocusHandle,
    focus_entered: Rc<Cell<bool>>,
    pub cols: u16,
    pub rows: u16,
    pub running: bool,
    pub selecting: bool,
    pub selection_pointer: Option<gpui::Point<gpui::Pixels>>,
    pub selection_generation: u64,
    pub scroll_pixels: f32,
    pub mouse_cell: Option<(usize, usize)>,
    pub mouse_button: Option<vela_terminal::emulator::MouseButton>,
    pub input: gpui::Entity<super::terminal_composition::Input>,
    pub input_subscriptions: Vec<gpui::Subscription>,
    pub blink: gpui::Entity<CursorBlink>,
    pub focus_reporting: bool,
    pub geometry: Rc<Cell<Option<Geometry>>>,
}
impl View {
    pub fn new<T: 'static>(id: String, cols: u16, rows: u16, cx: &mut Context<T>) -> Self {
        let geometry: Rc<Cell<Option<Geometry>>> = Rc::default();
        let input = cx.new(|_| super::terminal_composition::Input::new(geometry.clone()));
        let blink = cx.new(CursorBlink::new);
        Self {
            id,
            name: None,
            emulator: Emulator::new(cols, rows),
            focus: cx.focus_handle().tab_stop(true),
            tab_focus: cx.focus_handle().tab_stop(true),
            close_focus: cx.focus_handle().tab_stop(true),
            focus_entered: Rc::default(),
            cols,
            rows,
            running: true,
            selecting: false,
            selection_pointer: None,
            selection_generation: 0,
            scroll_pixels: 0.,
            mouse_cell: None,
            mouse_button: None,
            input,
            input_subscriptions: Vec::new(),
            blink,
            focus_reporting: false,
            geometry,
        }
    }
}
#[derive(Default)]
pub(super) struct Tabs {
    pub workspace: String,
    pub session: Option<String>,
    pub generation: u64,
    pub initialized: bool,
    pub views: Vec<View>,
    pub scroll: gpui::ScrollHandle,
    pub panel_focus: Option<[FocusHandle; 3]>,
    pub panel_entered: [Rc<Cell<bool>>; 3],
    active: Option<String>,
}
impl Tabs {
    pub fn owns(&self, workspace: &str, session: Option<&str>) -> bool {
        self.workspace == workspace && self.session.as_deref() == session
    }
    pub fn active(&self) -> Option<&View> {
        self.active
            .as_ref()
            .and_then(|id| self.views.iter().find(|view| view.id == *id))
    }
    pub fn active_mut(&mut self) -> Option<&mut View> {
        let id = self.active.as_ref()?;
        self.views.iter_mut().find(|view| view.id == *id)
    }
    pub fn get_mut(&mut self, id: &str) -> Option<&mut View> {
        self.views.iter_mut().find(|view| view.id == id)
    }
    pub fn insert(&mut self, view: View) {
        self.active = Some(view.id.clone());
        self.views.push(view);
        self.initialized = true;
        self.reveal_active();
    }
    pub fn select(&mut self, id: &str) -> bool {
        if self.views.iter().any(|view| view.id == id) {
            self.active = Some(id.to_owned());
            self.reveal_active();
            true
        } else {
            false
        }
    }
    pub fn remove(&mut self, id: &str) -> Option<View> {
        let index = self.views.iter().position(|view| view.id == id)?;
        let removed = self.views.remove(index);
        if self.active.as_deref() == Some(id) {
            self.active = self
                .views
                .get(index.min(self.views.len().saturating_sub(1)))
                .map(|view| view.id.clone());
        }
        self.reveal_active();
        Some(removed)
    }
    fn reveal_active(&self) {
        if let Some(index) = self
            .views
            .iter()
            .position(|view| Some(&view.id) == self.active.as_ref())
        {
            self.scroll
                .scroll_to_item(if index == 0 { 0 } else { index + 2 });
        }
    }
    pub fn reset(&mut self, workspace: String, session: Option<String>) -> Vec<View> {
        self.workspace = workspace;
        self.session = session;
        self.generation = self.generation.wrapping_add(1);
        self.active = None;
        self.initialized = false;
        self.scroll = gpui::ScrollHandle::new();
        std::mem::take(&mut self.views)
    }
}
pub(super) fn reveal_on_focus(
    row: gpui::Stateful<gpui::Div>,
    focus: Vec<FocusHandle>,
    scroll: gpui::ScrollHandle,
    index: usize,
    entered: Rc<Cell<bool>>,
) -> gpui::Stateful<gpui::Div> {
    use gpui::{prelude::*, *};
    row.relative().child(
        canvas(
            move |_, window, _| {
                let focused = focus.iter().any(|focus| focus.is_focused(window));
                if focused && !entered.replace(focused) {
                    scroll.scroll_to_item(index);
                    window.on_next_frame(|window, _| window.refresh());
                } else {
                    entered.set(focused);
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full(),
    )
}

impl super::VelaApp {
    pub(super) fn render_terminal_tabs(
        &self,
        panel_active: bool,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        use gpui::{prelude::*, *};
        let theme = crate::theme::Theme::default();
        self.terminals
            .views
            .iter()
            .enumerate()
            .map(|(index, view)| {
                let id = view.id.clone();
                let close_id = id.clone();
                let selected = panel_active
                    && self
                        .terminals
                        .active()
                        .is_some_and(|active| active.id == id);
                let name = view
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Terminal {}", index + 1));
                reveal_on_focus(
                    div()
                        .id(SharedString::from(format!("terminal-tab-{id}")))
                        .role(Role::Tab)
                        .aria_label(name.clone())
                        .aria_selected(selected)
                        .track_focus(&view.tab_focus)
                        .h(px(34.))
                        .px(px(9.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .border_b_1()
                        .border_color(if selected { theme.accent } else { theme.editor })
                        .text_size(px(10.))
                        .text_color(if selected { theme.accent } else { theme.dim })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.inset))
                        .on_click(cx.listener(move |app, _, window, cx| {
                            app.select_terminal(&id, window, cx)
                        }))
                        .child(name.clone())
                        .child(
                            div()
                                .id(SharedString::from(format!("terminal-tab-close-{close_id}")))
                                .role(Role::Button)
                                .aria_label(format!("Close {name}"))
                                .track_focus(&view.close_focus)
                                .px(px(3.))
                                .text_color(theme.faint)
                                .hover(|style| style.text_color(theme.red))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_click(cx.listener(move |app, _, _, cx| {
                                    cx.stop_propagation();
                                    app.close_terminal_tab(&close_id, cx);
                                }))
                                .child("×"),
                        ),
                    vec![view.tab_focus.clone(), view.close_focus.clone()],
                    self.terminals.scroll.clone(),
                    if index == 0 { 0 } else { index + 2 },
                    view.focus_entered.clone(),
                )
                .into_any_element()
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext;
    struct OverflowTabs {
        focus: Vec<FocusHandle>,
        entered: Vec<Rc<Cell<bool>>>,
        scroll: gpui::ScrollHandle,
    }
    impl gpui::Render for OverflowTabs {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut Context<Self>,
        ) -> impl gpui::IntoElement {
            use gpui::{prelude::*, *};
            div()
                .id("tabs-scroll")
                .w(px(180.))
                .h(px(34.))
                .flex()
                .overflow_x_scroll()
                .track_scroll(&self.scroll)
                .children(self.focus.iter().enumerate().map(|(index, focus)| {
                    reveal_on_focus(
                        div()
                            .id(("tab", index))
                            .role(Role::Button)
                            .track_focus(focus)
                            .w(px(100.))
                            .h(px(34.))
                            .flex_shrink_0()
                            .child(format!("Tab {index}")),
                        vec![focus.clone()],
                        self.scroll.clone(),
                        index,
                        self.entered[index].clone(),
                    )
                }))
        }
    }
    #[gpui::test]
    fn keyboard_focus_reveals_offscreen_tabs_without_fighting_manual_scroll(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::*;
        let (view, cx) = cx.add_window_view(|_, cx| OverflowTabs {
            focus: (0..12).map(|_| cx.focus_handle().tab_stop(true)).collect(),
            entered: (0..12).map(|_| Rc::default()).collect(),
            scroll: ScrollHandle::new(),
        });
        cx.simulate_resize(size(px(180.), px(34.)));
        let draw = |cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(180.), px(34.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        cx.update(|window, cx| {
            let focus = view.read(cx).focus[11].clone();
            window.focus(&focus, cx);
        });
        draw(cx);
        draw(cx);
        cx.update(|_, cx| {
            let state = view.read(cx);
            let bounds = state.scroll.bounds_for_item(11).unwrap();
            let offset = state.scroll.offset();
            let viewport = state.scroll.bounds();
            assert!(bounds.left() + offset.x >= viewport.left());
            assert!(bounds.right() + offset.x <= viewport.right());
            state.scroll.set_offset(point(px(0.), px(0.)));
        });
        draw(cx);
        draw(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).scroll.offset().x, px(0.)));
    }

    struct TestView {
        tabs: Tabs,
    }
    #[gpui::test]
    fn same_directory_conversations_retire_terminals_and_reject_previous_activation_results(
        cx: &mut gpui::TestAppContext,
    ) {
        let view = cx.new(|cx| {
            let mut tabs = Tabs::default();
            tabs.reset("/same".into(), Some("a".into()));
            tabs.insert(View::new("first".into(), 80, 24, cx));
            TestView { tabs }
        });
        view.update(cx,|view,cx| {
            let old=DirectoryRequest{session:"a".into(),workspace:"/same".into(),generation:view.tabs.generation,name:None};
            assert!(old.current(&view.tabs,"/same",Some("a")));
            assert!(!view.tabs.owns("/same",Some("b")));
            let retired=view.tabs.reset("/same".into(),Some("b".into()));
            assert_eq!(retired.len(),1);assert_eq!(retired[0].id,"first");assert!(view.tabs.get_mut("first").is_none());
            view.tabs.insert(View::new("second".into(),80,24,cx));
            assert!(!old.current(&view.tabs,"/same",Some("b")));
            let retired=view.tabs.reset("/same".into(),Some("a".into()));
            assert_eq!(retired[0].id,"second");
            assert!(!old.current(&view.tabs,"/same",Some("a")),"returning to the same conversation must not accept an old activation's directory result");
            assert!(view.tabs.active().is_none());assert!(!view.tabs.initialized);
        });
    }
    impl gpui::Render for TestView {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }
    #[gpui::test]
    fn cursor_blink_toggles_only_while_focused_and_blinking(cx: &mut gpui::TestAppContext) {
        let blink = cx.new(CursorBlink::new);
        blink.update(cx, |blink, cx| blink.update_state(true, true, cx));
        assert!(blink.read_with(cx, |blink, _| blink.cursor_visible()));
        cx.executor().advance_clock(CURSOR_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(!blink.read_with(cx, |blink, _| blink.cursor_visible()));
        cx.executor().advance_clock(CURSOR_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(blink.read_with(cx, |blink, _| blink.cursor_visible()));

        blink.update(cx, |blink, cx| blink.update_state(true, false, cx));
        assert!(
            blink.read_with(cx, |blink, _| blink.cursor_visible()),
            "unfocused cursor is steady"
        );
        cx.executor().advance_clock(CURSOR_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(blink.read_with(cx, |blink, _| blink.cursor_visible()));

        blink.update(cx, |blink, cx| blink.update_state(false, true, cx));
        cx.executor().advance_clock(CURSOR_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(
            blink.read_with(cx, |blink, _| blink.cursor_visible()),
            "steady cursor styles never blink"
        );

        blink.update(cx, |blink, cx| blink.update_state(true, true, cx));
        assert!(blink.read_with(cx, |blink, _| blink.cursor_visible()));
        cx.executor().advance_clock(CURSOR_BLINK_INTERVAL);
        cx.run_until_parked();
        assert!(!blink.read_with(cx, |blink, _| blink.cursor_visible()));
    }

    #[gpui::test]
    fn tabs_keep_inactive_buffers_choose_source_neighbor_and_reject_retired_ids(
        cx: &mut gpui::TestAppContext,
    ) {
        let view = cx.new(|cx| {
            let mut tabs = Tabs {
                workspace: "/one".into(),
                ..Default::default()
            };
            for id in ["one", "two", "three"] {
                tabs.insert(View::new(id.into(), 80, 24, cx));
            }
            TestView { tabs }
        });
        view.update(cx, |view, cx| {
            view.tabs
                .get_mut("one")
                .unwrap()
                .emulator
                .feed(b"background output");
            assert_eq!(view.tabs.active().unwrap().id, "three");
            assert!(view.tabs.select("one"));
            let text: String = view
                .tabs
                .active()
                .unwrap()
                .emulator
                .line(0)
                .iter()
                .map(|cell| cell.ch)
                .collect();
            assert!(text.starts_with("background output"));
            view.tabs.remove("two");
            assert_eq!(view.tabs.active().unwrap().id, "one");
            view.tabs.remove("one");
            assert_eq!(view.tabs.active().unwrap().id, "three");
            assert!(view.tabs.get_mut("one").is_none());
            assert!(!view.tabs.select("one"));
            let retired = view
                .tabs
                .reset("/two".into(), Some("second-session".into()));
            assert_eq!(retired.len(), 1);
            assert!(view.tabs.active().is_none());
            assert!(!view.tabs.initialized);
            view.tabs.insert(View::new("new-owner".into(), 80, 24, cx));
            assert!(view.tabs.get_mut("three").is_none());
            assert_eq!(view.tabs.active().unwrap().id, "new-owner");
        });
    }
}
