//! Native overflow:auto geometry. GPUI/Taffy reserves scrollbar space for every
//! Overflow::Scroll node, so an initial full-width measurement decides whether
//! Vela's eight-point gutter is actually needed. Content/viewport changes probe
//! again, including removal of the final overflowing form.
use gpui::{prelude::*, *};
use std::{cell::RefCell, rc::Rc};

pub(crate) fn list_distance_from_bottom(list: &ListState) -> Pixels {
    (list.max_offset_for_scrollbar().y + list.scroll_px_offset_for_scrollbar().y).max(px(0.))
}

pub(crate) fn observe_list_scroll(
    list: &ListState,
    event: &ListScrollEvent,
    cx: &mut App,
    callback: impl FnOnce(bool, &mut App) + 'static,
) {
    if event.is_scrolled {
        // GPUI invokes this observer while its ListState RefCell is borrowed.
        // Read the settled position only after dispatch releases that borrow.
        let list = list.clone();
        let following = event.is_following_tail;
        cx.defer(move |cx| {
            callback(following || list_distance_from_bottom(&list) <= px(24.), cx);
        });
    }
}

#[derive(Clone, Default)]
pub(crate) struct AutoScrollHandle(Rc<RefCell<ScrollState>>);
#[derive(Default)]
struct ScrollState {
    scroll: ScrollHandle,
    gutter: bool,
    applied: bool,
    measured: Option<(Size<Pixels>, Pixels)>,
    dragging: Option<Pixels>,
    hovered: bool,
}
impl ScrollState {
    fn observe_geometry(&mut self, axis: Axis, viewport: Size<Pixels>, maximum: Pixels) -> bool {
        let measurement = (viewport, axis.extent(viewport) + maximum);
        if matches!(axis, Axis::Horizontal) {
            // This gutter reserves height, so it cannot create the horizontal
            // overflow being measured. A parent vertical-gutter width probe
            // must not remove it and restart both containers' layout loops.
            let needed = maximum > px(0.);
            let changed = self.gutter != needed;
            self.gutter = needed;
            self.applied = false;
            self.measured = Some(measurement);
            return changed;
        }
        if self.applied {
            self.applied = false;
            self.measured = Some(measurement);
        } else if self.measured != Some(measurement) {
            if self.gutter {
                self.gutter = false;
                self.measured = None;
                return true;
            } else if maximum > px(0.) {
                self.gutter = true;
                self.applied = true;
                return true;
            } else {
                self.measured = Some(measurement);
            }
        }
        false
    }
}
impl AutoScrollHandle {
    /// Share the native offset with virtual rows and keyboard navigation.
    pub(crate) fn native_handle(&self) -> ScrollHandle {
        self.0.borrow().scroll.clone()
    }
    pub fn reveal(&self, bounds: Bounds<Pixels>) {
        let state = self.0.borrow();
        let viewport = state.scroll.bounds();
        if bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom() {
            return;
        }
        // The reference exposes partially visible controls at the nearest edge,
        // and centers wholly hidden controls, clamped to the scroll range.
        let delta = if bounds.bottom() <= viewport.top() || bounds.top() >= viewport.bottom() {
            bounds.center().y - viewport.center().y
        } else if bounds.top() < viewport.top() {
            bounds.top() - viewport.top()
        } else {
            bounds.bottom() - viewport.bottom()
        };
        let max = state.scroll.max_offset().y;
        state.scroll.set_offset(point(
            px(0.),
            (state.scroll.offset().y - delta).clamp(-max, px(0.)),
        ));
    }
}

type FocusBuild = Box<dyn FnOnce(&FocusHandle) -> AnyElement>;
pub(crate) struct FocusState {
    focus: FocusHandle,
    was_focused: bool,
}
/// Keep focus ownership stable across renders and reveal only when focus enters.
/// Wheel scrolling must remain possible while a control retains keyboard focus.
pub(crate) struct FocusReveal {
    id: ElementId,
    scroll: AutoScrollHandle,
    focus: Option<FocusHandle>,
    enabled: bool,
    build: Option<FocusBuild>,
}
impl FocusReveal {
    pub fn input(
        id: impl Into<ElementId>,
        scroll: AutoScrollHandle,
        focus: FocusHandle,
        input: impl IntoElement + 'static,
    ) -> Self {
        Self::element(id, scroll, focus, input)
    }

    pub fn element(
        id: impl Into<ElementId>,
        scroll: AutoScrollHandle,
        focus: FocusHandle,
        element: impl IntoElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            scroll,
            focus: Some(focus),
            enabled: true,
            build: Some(Box::new(move |_| element.into_any_element())),
        }
    }
}
pub(crate) fn focus_control<E>(
    id: impl Into<ElementId>,
    scroll: AutoScrollHandle,
    enabled: bool,
    control: E,
) -> FocusReveal
where
    E: StatefulInteractiveElement + IntoElement + 'static,
{
    control_reveal(id, scroll, None, enabled, control)
}
pub(crate) fn focused_control<E>(
    id: impl Into<ElementId>,
    scroll: AutoScrollHandle,
    focus: FocusHandle,
    control: E,
) -> FocusReveal
where
    E: StatefulInteractiveElement + IntoElement + 'static,
{
    control_reveal(id, scroll, Some(focus), true, control)
}
fn control_reveal<E>(
    id: impl Into<ElementId>,
    scroll: AutoScrollHandle,
    focus: Option<FocusHandle>,
    enabled: bool,
    control: E,
) -> FocusReveal
where
    E: StatefulInteractiveElement + IntoElement + 'static,
{
    FocusReveal {
        id: id.into(),
        scroll,
        focus,
        enabled,
        build: Some(Box::new(move |focus| {
            let theme = crate::theme::Theme::default();
            control
                .track_focus(focus)
                .focus_visible(move |style| {
                    style.shadow(vec![
                        BoxShadow::new(px(0.), px(0.), theme.accent.into()).spread_radius(px(2.)),
                        BoxShadow::new(px(0.), px(0.), theme.panel.into()).spread_radius(px(1.)),
                    ])
                })
                .into_any_element()
        })),
    }
}
impl IntoElement for FocusReveal {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for FocusReveal {
    type RequestLayoutState = (AnyElement, Rc<RefCell<FocusState>>);
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state =
            window.with_element_state(id.expect("focus reveal has a stable id"), |state, _| {
                let state: Rc<RefCell<FocusState>> = state.unwrap_or_else(|| {
                    Rc::new(RefCell::new(FocusState {
                        focus: self
                            .focus
                            .clone()
                            .unwrap_or_else(|| cx.focus_handle().tab_stop(self.enabled)),
                        was_focused: false,
                    }))
                });
                (state.clone(), state)
            });
        if self.focus.is_none() {
            let mut state = state.borrow_mut();
            state.focus = state.focus.clone().tab_stop(self.enabled);
        }
        let mut child =
            self.build.take().expect("focus control is laid out once")(&state.borrow().focus);
        (child.request_layout(window, cx), (child, state))
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let _ = state.0.prepaint(window, cx);
        let mut focus = state.1.borrow_mut();
        let focused = focus.focus.is_focused(window);
        if focused && !focus.was_focused {
            self.scroll.reveal(bounds);
            window.on_next_frame(|window, _| window.refresh());
        }
        focus.was_focused = focused;
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        state.0.paint(window, cx);
    }
}
/// The same overflow gutter and draggable thumb for independently virtualized
/// lists. Metrics are sampled after child prepaint, outside ListState's borrow.
#[derive(Clone, Default)]
pub(crate) struct ListViewportHandle(Rc<RefCell<ListViewportState>>);
impl ListViewportHandle {
    pub(crate) fn height(&self) -> Pixels {
        self.0.borrow().viewport_height.max(px(1.))
    }
}
#[derive(Default)]
struct ListViewportState {
    viewport_height: Pixels,
    gutter: bool,
    applied: bool,
    measured: Option<(Size<Pixels>, Pixels)>,
    dragging: Option<Pixels>,
    hovered: bool,
}
type ListBuild = Box<dyn FnOnce(Pixels) -> AnyElement>;
type ListObserver = Rc<dyn Fn(&ListState, &mut Window, &mut App)>;
pub(crate) struct ListViewport {
    handle: ListViewportHandle,
    list: ListState,
    build: Option<ListBuild>,
    on_scroll: Option<ListObserver>,
}
impl ListViewport {
    pub fn new(
        handle: ListViewportHandle,
        list: ListState,
        build: impl FnOnce(Pixels) -> AnyElement + 'static,
    ) -> Self {
        Self {
            handle,
            list,
            build: Some(Box::new(build)),
            on_scroll: None,
        }
    }
    pub fn on_scroll(
        mut self,
        callback: impl Fn(&ListState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_scroll = Some(Rc::new(callback));
        self
    }
}
impl IntoElement for ListViewport {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for ListViewport {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let gutter = if self.handle.0.borrow().gutter {
            8.
        } else {
            0.
        };
        let mut child = self.build.take().expect("list viewport is laid out once")(px(gutter));
        (child.request_layout(window, cx), child)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        let _ = child.prepaint(window, cx);
        let maximum = self.list.max_offset_for_scrollbar().y;
        let measurement = (bounds.size, bounds.size.height + maximum);
        let mut state = self.handle.0.borrow_mut();
        state.viewport_height = bounds.size.height;
        if state.applied {
            state.applied = false;
            state.measured = Some(measurement);
        } else if state.measured != Some(measurement) {
            if state.gutter {
                state.gutter = false;
                state.measured = None;
                window.on_next_frame(|window, _| window.refresh());
            } else if maximum > px(0.) {
                state.gutter = true;
                state.applied = true;
                window.on_next_frame(|window, _| window.refresh());
            } else {
                state.measured = Some(measurement);
            }
        }
        if state.gutter && maximum > px(0.) {
            let _ = window.insert_hitbox(
                Bounds::new(
                    point(bounds.right() - px(8.), bounds.top()),
                    size(px(8.), bounds.size.height),
                ),
                HitboxBehavior::Normal,
            );
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
        let maximum = self.list.max_offset_for_scrollbar().y;
        let state = self.handle.0.borrow();
        if !state.gutter || maximum <= px(0.) {
            return;
        }
        let track = Bounds::new(
            point(bounds.right() - px(8.), bounds.top()),
            size(px(8.), bounds.size.height),
        );
        let height = (bounds.size.height * (bounds.size.height / (bounds.size.height + maximum)))
            .max(px(18.))
            .min(bounds.size.height);
        let top = (-self.list.scroll_px_offset_for_scrollbar().y / maximum)
            * (track.size.height - height);
        let thumb = Bounds::new(
            point(track.left(), track.top() + top),
            size(track.size.width, height),
        );
        let hovered = state.hovered;
        drop(state);
        window.paint_quad(quad(
            thumb.inset(px(2.)),
            px(99.),
            rgba(if hovered { 0xffffff3d } else { 0xffffff24 }),
            Edges::all(px(0.)),
            transparent_black(),
            BorderStyle::Solid,
        ));
        let handle = self.handle.clone();
        let list = self.list.clone();
        let observer = self.on_scroll.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if !phase.bubble()
                || event.button != MouseButton::Left
                || !track.contains(&event.position)
            {
                return;
            }
            if thumb.contains(&event.position) {
                handle.0.borrow_mut().dragging = Some(event.position.y - thumb.top());
                list.scrollbar_drag_started();
            } else {
                let step = track.size.height
                    * 0.875
                    * if event.position.y < thumb.top() {
                        -1.
                    } else {
                        1.
                    };
                let offset =
                    (-list.scroll_px_offset_for_scrollbar().y + step).clamp(px(0.), maximum);
                list.set_offset_from_scrollbar(point(px(0.), -offset));
            }
            if let Some(observer) = &observer {
                observer(&list, window, cx);
            }
            window.refresh();
            cx.stop_propagation();
        });
        let handle = self.handle.clone();
        let list = self.list.clone();
        let observer = self.on_scroll.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            let mut state = handle.0.borrow_mut();
            let hovered = track.contains(&event.position);
            if state.hovered != hovered {
                state.hovered = hovered;
                window.refresh();
            }
            if let Some(grab) = state.dragging {
                let travel = track.size.height - thumb.size.height;
                if travel > px(0.) {
                    let fraction = ((event.position.y - track.top() - grab) / travel).clamp(0., 1.);
                    list.set_offset_from_scrollbar(point(px(0.), -maximum * fraction));
                    if let Some(observer) = &observer {
                        observer(&list, window, cx);
                    }
                    window.refresh();
                    cx.stop_propagation();
                }
            }
        });
        let handle = self.handle.clone();
        let list = self.list.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase.bubble()
                && event.button == MouseButton::Left
                && handle.0.borrow_mut().dragging.take().is_some()
            {
                list.scrollbar_drag_ended();
                window.refresh();
                cx.stop_propagation();
            }
        });
    }
}

type Build = Box<dyn FnOnce(ScrollHandle, Pixels) -> AnyElement>;
#[derive(Clone, Copy)]
enum Axis {
    Vertical,
    Horizontal,
}
impl Axis {
    fn extent(self, size: Size<Pixels>) -> Pixels {
        match self {
            Self::Vertical => size.height,
            Self::Horizontal => size.width,
        }
    }
    fn coordinate(self, point: Point<Pixels>) -> Pixels {
        match self {
            Self::Vertical => point.y,
            Self::Horizontal => point.x,
        }
    }
    fn offset(self, value: Pixels) -> Point<Pixels> {
        match self {
            Self::Vertical => point(px(0.), value),
            Self::Horizontal => point(value, px(0.)),
        }
    }
    fn track(self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        match self {
            Self::Vertical => Bounds::new(
                point(bounds.right() - px(8.), bounds.top()),
                size(px(8.), bounds.size.height),
            ),
            Self::Horizontal => Bounds::new(
                point(bounds.left(), bounds.bottom() - px(8.)),
                size(bounds.size.width, px(8.)),
            ),
        }
    }
    fn thumb(self, track: Bounds<Pixels>, offset: Pixels, length: Pixels) -> Bounds<Pixels> {
        match self {
            Self::Vertical => Bounds::new(
                track.origin + point(px(0.), offset),
                size(track.size.width, length),
            ),
            Self::Horizontal => Bounds::new(
                track.origin + point(offset, px(0.)),
                size(length, track.size.height),
            ),
        }
    }
}
type GutterChanged = Rc<dyn Fn(&mut App)>;
pub(crate) struct AutoScroll {
    state: AutoScrollHandle,
    build: Option<Build>,
    axis: Axis,
    gutter_changed: Option<GutterChanged>,
}
impl AutoScroll {
    pub fn new(
        state: AutoScrollHandle,
        build: impl FnOnce(ScrollHandle, Pixels) -> AnyElement + 'static,
    ) -> Self {
        Self {
            state,
            build: Some(Box::new(build)),
            axis: Axis::Vertical,
            gutter_changed: None,
        }
    }
    pub fn horizontal(
        state: AutoScrollHandle,
        build: impl FnOnce(ScrollHandle, Pixels) -> AnyElement + 'static,
    ) -> Self {
        Self {
            axis: Axis::Horizontal,
            ..Self::new(state, build)
        }
    }
    pub fn on_gutter_change(mut self, changed: impl Fn(&mut App) + 'static) -> Self {
        self.gutter_changed = Some(Rc::new(changed));
        self
    }
}
impl IntoElement for AutoScroll {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for AutoScroll {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let state = self.state.0.borrow();
        let mut child = self.build.take().expect("scroll content is laid out once")(
            state.scroll.clone(),
            px(if state.gutter { 8. } else { 0. }),
        );
        drop(state);
        (child.request_layout(window, cx), child)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        let _ = child.prepaint(window, cx);
        let mut state = self.state.0.borrow_mut();
        let max = self.axis.coordinate(state.scroll.max_offset());
        if matches!(self.axis, Axis::Horizontal) {
            tracing::trace!(
                ?bounds,
                ?max,
                gutter = state.gutter,
                applied = state.applied,
                "horizontal scroll geometry"
            );
        }
        let old_gutter = state.gutter;
        if state.observe_geometry(self.axis, bounds.size, max) {
            window.on_next_frame(|window, _| window.refresh());
        }
        if old_gutter != state.gutter
            && let Some(changed) = self.gutter_changed.clone()
        {
            cx.defer(move |cx| changed(cx));
        }
        if !state.gutter || max <= px(0.) {
            state.dragging = None;
            state.hovered = false;
        }
        if state.gutter && max > px(0.) {
            let track = self.axis.track(bounds);
            let _ = window.insert_hitbox(track, HitboxBehavior::Normal);
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
        let state = self.state.0.borrow();
        let axis = self.axis;
        let maximum = axis.coordinate(state.scroll.max_offset());
        if !state.gutter || maximum <= px(0.) {
            return;
        }
        let track = axis.track(bounds);
        let viewport = axis.extent(bounds.size);
        let height = (viewport * (viewport / (viewport + maximum)))
            .max(px(18.))
            .min(viewport);
        let top =
            -axis.coordinate(state.scroll.offset()) / maximum * (axis.extent(track.size) - height);
        let thumb = axis.thumb(track, top, height);
        let hovered = state.hovered;
        drop(state);
        window.paint_quad(quad(
            thumb.inset(px(2.)),
            px(99.),
            rgba(if hovered { 0xffffff3d } else { 0xffffff24 }),
            Edges::all(px(0.)),
            transparent_black(),
            BorderStyle::Solid,
        ));

        let down = self.state.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if !phase.bubble()
                || event.button != MouseButton::Left
                || !track.contains(&event.position)
            {
                return;
            }
            let mut state = down.0.borrow_mut();
            if thumb.contains(&event.position) {
                state.dragging =
                    Some(axis.coordinate(event.position) - axis.coordinate(thumb.origin));
            } else {
                let step = axis.extent(track.size)
                    * 0.875
                    * if axis.coordinate(event.position) < axis.coordinate(thumb.origin) {
                        -1.
                    } else {
                        1.
                    };
                let top = (-axis.coordinate(state.scroll.offset()) + step).clamp(px(0.), maximum);
                state.scroll.set_offset(axis.offset(-top));
            }
            window.refresh();
            cx.stop_propagation();
        });
        let moved = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            let mut state = moved.0.borrow_mut();
            let hovered = track.contains(&event.position);
            if state.hovered != hovered {
                state.hovered = hovered;
                window.refresh();
            }
            if let Some(grab) = state.dragging {
                let travel = axis.extent(track.size) - axis.extent(thumb.size);
                if travel > px(0.) {
                    let fraction =
                        ((axis.coordinate(event.position) - axis.coordinate(track.origin) - grab)
                            / travel)
                            .clamp(0., 1.);
                    state.scroll.set_offset(axis.offset(-maximum * fraction));
                    window.refresh();
                    cx.stop_propagation();
                }
            }
        });
        let up = self.state.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase.bubble()
                && event.button == MouseButton::Left
                && up.0.borrow_mut().dragging.take().is_some()
            {
                window.refresh();
                cx.stop_propagation();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{AutoScroll, AutoScrollHandle, focus_control};
    use gpui::{
        Bounds, Context, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
        Pixels, Render, TestAppContext, Window, canvas, div, point, prelude::*, px, size,
    };
    use std::{cell::RefCell, rc::Rc};
    struct ListFixture(gpui::ListState);
    struct ViewportFixture {
        list: gpui::ListState,
        handle: super::ListViewportHandle,
    }
    impl Render for ViewportFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let list = self.list.clone();
            div().size_full().child(super::ListViewport::new(
                self.handle.clone(),
                list.clone(),
                move |gutter| {
                    div()
                        .size_full()
                        .pr(gutter)
                        .child(
                            gpui::list(list, |index, _, _| {
                                div()
                                    .h(px(20.))
                                    .child(format!("row {index}"))
                                    .into_any_element()
                            })
                            .size_full(),
                        )
                        .into_any_element()
                },
            ))
        }
    }
    #[gpui::test]
    fn virtual_transcript_gutter_tracks_overflow_removal_and_resize(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| ViewportFixture {
            list: gpui::ListState::new(50, gpui::ListAlignment::Top, px(100.))
                .with_uniform_item_height(px(20.)),
            handle: super::ListViewportHandle::default(),
        });
        let draw = |cx: &mut gpui::VisualTestContext, height| {
            cx.simulate_resize(size(px(300.), height));
            for _ in 0..6 {
                cx.draw(point(px(0.), px(0.)), size(px(300.), height), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx, px(200.));
        view.read_with(cx, |view, _| {
            assert!(view.handle.0.borrow().gutter);
            assert_eq!(view.list.viewport_bounds().size.width, px(292.));
        });
        view.update(cx, |view, cx| {
            view.list.reset_with_uniform_height(2, px(20.));
            cx.notify();
        });
        draw(cx, px(200.));
        view.read_with(cx, |view, _| assert!(!view.handle.0.borrow().gutter));
        view.update(cx, |view, cx| {
            view.list.reset_with_uniform_height(50, px(20.));
            cx.notify();
        });
        draw(cx, px(200.));
        view.read_with(cx, |view, _| assert!(view.handle.0.borrow().gutter));
        draw(cx, px(1600.));
        view.read_with(cx, |view, _| assert!(!view.handle.0.borrow().gutter));
    }
    impl Render for ListFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                gpui::list(self.0.clone(), |index, _, _| {
                    div()
                        .h(px(40.))
                        .child(format!("row {index}"))
                        .into_any_element()
                })
                .size_full(),
            )
        }
    }
    #[gpui::test]
    fn transcript_wheel_observer_reads_tail_after_list_releases_its_mutable_borrow(
        cx: &mut TestAppContext,
    ) {
        let observed = Rc::new(RefCell::new(Vec::new()));
        let values = observed.clone();
        let (view, cx) = cx.add_window_view(move |_, _| {
            let state = gpui::ListState::new(50, gpui::ListAlignment::Top, px(100.)).measure_all();
            let scroll_state = state.clone();
            state.set_scroll_handler(move |event, _, cx| {
                let values = values.clone();
                super::observe_list_scroll(&scroll_state, event, cx, move |following, _| {
                    values.borrow_mut().push(following)
                });
            });
            ListFixture(state)
        });
        cx.simulate_resize(size(px(300.), px(200.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(200.)), |_, _| {
                view.clone().into_any_element()
            })
        };
        draw(cx);
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: point(px(150.), px(100.)),
            delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-200.))),
            ..Default::default()
        });
        draw(cx);
        assert_eq!(observed.borrow().last(), Some(&false));
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: point(px(150.), px(100.)),
            delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-10000.))),
            ..Default::default()
        });
        draw(cx);
        assert_eq!(observed.borrow().last(), Some(&true));
        // Match the reference's inclusive 24 px floor threshold.
        for (delta, following) in [(24., true), (1., false)] {
            cx.simulate_event(gpui::ScrollWheelEvent {
                position: point(px(150.), px(100.)),
                delta: gpui::ScrollDelta::Pixels(point(px(0.), px(delta))),
                ..Default::default()
            });
            draw(cx);
            assert_eq!(observed.borrow().last(), Some(&following));
        }
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: point(px(150.), px(100.)),
            delta: gpui::ScrollDelta::Pixels(point(px(0.), px(10000.))),
            ..Default::default()
        });
        draw(cx);
        assert_eq!(observed.borrow().last(), Some(&false));
    }

    struct FocusFixture {
        scroll: AutoScrollHandle,
        first: gpui::FocusHandle,
        last_enabled: bool,
        observed: Rc<RefCell<Option<Bounds<Pixels>>>>,
    }
    impl Render for FocusFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let first = self.first.clone();
            let scroll = self.scroll.clone();
            let observed = self.observed.clone();
            let enabled = self.last_enabled;
            div()
                .size_full()
                .flex()
                .flex_col()
                .on_key_down(crate::input::navigate_focus)
                .child(AutoScroll::new(
                    self.scroll.clone(),
                    move |handle, gutter| {
                        div()
                            .id("focus-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&handle)
                            .scrollbar_width(gutter)
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("first")
                                    .track_focus(&first)
                                    .h(px(30.))
                                    .flex_shrink_0()
                                    .child("First"),
                            )
                            .child(div().h(px(1000.)).flex_shrink_0())
                            .child(focus_control(
                                "reveal-last",
                                scroll,
                                enabled,
                                div()
                                    .id("last")
                                    .h(px(30.))
                                    .flex_shrink_0()
                                    .relative()
                                    .child("Last")
                                    .child(
                                        canvas(
                                            move |bounds, _, _| {
                                                *observed.borrow_mut() = Some(bounds)
                                            },
                                            |_, _, _, _| {},
                                        )
                                        .absolute()
                                        .size_full(),
                                    ),
                            ))
                            .child(div().h(px(80.)).flex_shrink_0())
                            .into_any_element()
                    },
                ))
        }
    }
    #[gpui::test]
    fn keyboard_focus_reveals_offscreen_controls_without_trapping_manual_scroll(
        cx: &mut TestAppContext,
    ) {
        let observed = Rc::new(RefCell::new(None));
        let measured = observed.clone();
        let (view, cx) = cx.add_window_view(|window, cx| {
            let first = cx.focus_handle().tab_stop(true);
            window.focus(&first, cx);
            FocusFixture {
                scroll: AutoScrollHandle::default(),
                first,
                last_enabled: true,
                observed: measured,
            }
        });
        cx.simulate_resize(size(px(300.), px(200.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(300.), px(200.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx);
        assert!(observed.borrow().unwrap().top() > px(200.));
        cx.simulate_keystrokes("tab");
        draw(cx);
        let bounds = observed.borrow().unwrap();
        assert!(
            bounds.top() >= px(0.) && bounds.bottom() <= px(200.),
            "focused control must be visible: {bounds:?}"
        );
        // A user can scroll away without the retained focus snapping it back.
        view.update(cx, |view, _| {
            view.scroll
                .0
                .borrow()
                .scroll
                .set_offset(point(px(0.), px(0.)))
        });
        draw(cx);
        assert!(observed.borrow().unwrap().top() > px(200.));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                view.read(cx).first.is_focused(window),
                "return to first before partial reveal"
            )
        });
        let hidden_top = observed.borrow().unwrap().top();
        view.update(cx, |view, _| {
            view.scroll
                .0
                .borrow()
                .scroll
                .set_offset(point(px(0.), px(185.) - hidden_top));
        });
        draw(cx);
        assert_eq!(observed.borrow().unwrap().top(), px(185.));
        let partially_visible = observed.borrow().unwrap();
        view.read_with(cx, |view, _| view.scroll.reveal(partially_visible));
        draw(cx);
        let partial = observed.borrow().unwrap();
        assert_eq!(
            partial.top(),
            px(170.),
            "partially visible focus reveals the nearest edge rather than centering"
        );
        assert_eq!(partial.bottom(), px(200.));
        // Disabled controls leave the traversal order even when their stable
        // focus handle was first created in the enabled state.
        view.update(cx, |view, cx| {
            view.last_enabled = false;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).first.is_focused(window)));
        view.update(cx, |view, cx| {
            view.last_enabled = true;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert!(observed.borrow().unwrap().bottom() <= px(200.));
    }
    struct Fixture {
        scroll: AutoScrollHandle,
        tall: bool,
        observed: Rc<RefCell<Option<Bounds<Pixels>>>>,
        requested: Option<Rc<std::cell::Cell<usize>>>,
    }
    struct HorizontalFixture {
        width: f32,
        scroll: AutoScrollHandle,
        changes: Rc<std::cell::Cell<usize>>,
    }
    impl Render for HorizontalFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            div().w(px(self.width)).child(
                AutoScroll::horizontal(self.scroll.clone(), move |handle, gutter| {
                    div()
                        .id("horizontal-fixture")
                        .w_full()
                        .min_w_0()
                        .overflow_x_scroll()
                        .track_scroll(&handle)
                        .p(px(12.))
                        .pb(px(12.) + gutter)
                        .child(div().w(px(1400.)).h(px(40.)).flex_shrink_0())
                        .into_any_element()
                })
                .on_gutter_change(move |_| changes.set(changes.get() + 1)),
            )
        }
    }
    #[test]
    fn recorded_parent_gutter_probes_do_not_clear_positive_horizontal_overflow() {
        let handle = AutoScrollHandle::default();
        let mut state = handle.0.borrow_mut();
        // Consecutive live QA measurements from the nested transcript/code
        // loop. Every width still overflows; only the parent gutter probes it.
        for _ in 0..3 {
            for (width, height, maximum) in [
                (274., 100., 1604.),
                (282., 108., 1596.),
                (274., 108., 1604.),
            ] {
                state.observe_geometry(
                    super::Axis::Horizontal,
                    size(px(width), px(height)),
                    px(maximum),
                );
                assert!(
                    state.gutter,
                    "a positive horizontal extent must retain its gutter at {width}x{height}"
                );
            }
        }
    }
    #[gpui::test]
    fn horizontal_gutter_stays_present_during_parent_width_probes(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| HorizontalFixture {
            width: 200.,
            scroll: AutoScrollHandle::default(),
            changes: Rc::new(std::cell::Cell::new(0)),
        });
        let draw = |cx: &mut gpui::VisualTestContext, width| {
            view.update(cx, |view, cx| {
                view.width = width;
                cx.notify();
            });
            cx.simulate_resize(size(px(width), px(200.)));
            cx.draw(point(px(0.), px(0.)), size(px(width), px(200.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        for _ in 0..4 {
            draw(cx, 200.);
        }
        let changes_before = view.read_with(cx, |view, _| view.changes.get());
        for width in [208., 200., 208., 200.] {
            draw(cx, width);
            view.read_with(cx,|view,_| {
                let state=view.scroll.0.borrow();
                assert!(state.scroll.max_offset().x>px(1000.));
                assert_eq!(state.scroll.bounds().size.width,px(width));
                assert!(state.gutter,"positive horizontal overflow must survive the outer vertical gutter's width probes");
                assert_eq!(state.scroll.bounds().size.height,px(72.));
                assert_eq!(view.changes.get(),changes_before,"width probing must not remove and reinsert a still-needed horizontal gutter");
            });
        }
    }
    #[gpui::test]
    fn horizontal_overflow_reserves_height_drags_and_releases_gutter_after_resize(
        cx: &mut TestAppContext,
    ) {
        let changes = Rc::new(std::cell::Cell::new(0));
        let recorded = changes.clone();
        let (view, cx) = cx.add_window_view(|_, _| HorizontalFixture {
            width: 200.,
            scroll: AutoScrollHandle::default(),
            changes: recorded,
        });
        let draw = |cx: &mut gpui::VisualTestContext, width| {
            view.update(cx, |view, cx| {
                view.width = width;
                cx.notify();
            });
            cx.simulate_resize(size(px(width), px(200.)));
            for _ in 0..4 {
                cx.draw(point(px(0.), px(0.)), size(px(width), px(200.)), |_, _| {
                    view.clone().into_any_element()
                });
            }
        };
        draw(cx, 200.);
        view.read_with(cx, |view, _| {
            let state = view.scroll.0.borrow();
            assert!(state.gutter);
            assert_eq!(state.scroll.bounds().size.height, px(72.));
            assert!(state.scroll.max_offset().x > px(1000.));
        });
        cx.simulate_event(MouseDownEvent {
            position: point(px(10.), px(68.)),
            button: MouseButton::Left,
            ..Default::default()
        });
        cx.simulate_event(MouseMoveEvent {
            position: point(px(180.), px(68.)),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        cx.simulate_event(MouseUpEvent {
            position: point(px(180.), px(68.)),
            button: MouseButton::Left,
            ..Default::default()
        });
        draw(cx, 200.);
        view.read_with(cx, |view, _| {
            let state = view.scroll.0.borrow();
            assert!(state.scroll.offset().x < -state.scroll.max_offset().x * 0.9);
            assert_eq!(state.scroll.offset().y, px(0.));
            assert!(state.dragging.is_none());
        });
        cx.simulate_event(MouseDownEvent {
            position: point(px(180.), px(68.)),
            button: MouseButton::Left,
            ..Default::default()
        });
        assert!(view.read_with(cx, |view, _| view.scroll.0.borrow().dragging.is_some()));
        draw(cx, 2000.);
        view.read_with(cx, |view, _| {
            let state = view.scroll.0.borrow();
            assert!(!state.gutter);
            assert_eq!(state.scroll.bounds().size.height, px(64.));
            assert_eq!(state.scroll.offset().x, px(0.));
            assert!(
                state.dragging.is_none(),
                "removing overflow during a drag must release its owner"
            );
        });
        assert!(
            changes.get() >= 2,
            "virtual row owner must hear gutter insertion and removal"
        );
    }
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let tall = self.tall;
            let observed = self.observed.clone();
            let requested_before = self.requested.clone();
            let requested_after = self.requested.clone();
            div().size_full().flex().flex_col().child(AutoScroll::new(
                self.scroll.clone(),
                move |handle, gutter| {
                    div()
                        .id("fixture-scroll")
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .overflow_y_scroll()
                        .track_scroll(&handle)
                        .scrollbar_width(gutter)
                        .p(px(20.))
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .relative()
                                .w_full()
                                .flex_shrink_0()
                                .h(px(if tall { 1000. } else { 100. }))
                                .child(
                                    canvas(
                                        move |bounds, window, cx| {
                                            *observed.borrow_mut() = Some(bounds);
                                            if requested_before.is_some() {
                                                window.simulate_next_frame(cx);
                                            }
                                        },
                                        move |_, _, window, cx| {
                                            if let Some(requested) = &requested_after {
                                                let count = window.simulate_next_frame(cx);
                                                requested.set(requested.get().max(count));
                                            }
                                        },
                                    )
                                    .absolute()
                                    .size_full(),
                                ),
                        )
                        .into_any_element()
                },
            ))
        }
    }
    #[gpui::test]
    fn gutter_follows_overflow_after_content_removal_and_viewport_resize(cx: &mut TestAppContext) {
        let observed = Rc::new(RefCell::new(None));
        let measured = observed.clone();
        let (view, cx) = cx.add_window_view(|_, _| Fixture {
            scroll: AutoScrollHandle::default(),
            tall: true,
            observed: measured,
            requested: None,
        });
        for (tall, width, height, expected_width) in [
            (true, 200., 200., 152.),
            (false, 200., 200., 160.),
            (true, 500., 200., 452.),
            (true, 500., 1200., 460.),
        ] {
            view.update(cx, |view, cx| {
                view.tall = tall;
                cx.notify();
            });
            cx.simulate_resize(size(px(width), px(height)));
            for _ in 0..4 {
                cx.draw(
                    point(px(0.), px(0.)),
                    size(px(width), px(height)),
                    |_, _| view.clone().into_any_element(),
                );
            }
            let bounds = observed.borrow().expect("content must be measured");
            assert!(
                (f32::from(bounds.size.width) - expected_width).abs() < 0.1,
                "only overflowing content reserves the eight-point gutter: tall={tall}, viewport={width}x{height}, bounds={bounds:?}"
            );
        }
    }
    #[gpui::test]
    fn overflow_probe_requests_a_followup_frame_without_another_input(cx: &mut TestAppContext) {
        let requested = Rc::new(std::cell::Cell::new(0));
        let captured = requested.clone();
        let (view, cx) = cx.add_window_view(|_, _| Fixture {
            scroll: AutoScrollHandle::default(),
            tall: true,
            observed: Rc::default(),
            requested: Some(captured),
        });
        cx.simulate_resize(size(px(200.), px(200.)));
        cx.draw(point(px(0.), px(0.)), size(px(200.), px(200.)), |_, _| {
            view.clone().into_any_element()
        });
        // Check inside the draw, before the test harness adds its own refresh.
        // Window::refresh is ignored during drawing; a next-frame callback
        // must settle the gutter probe without an unrelated input event.
        assert!(
            requested.get() > 0,
            "an overflow probe must schedule its followup independently of user input"
        );
    }
}
