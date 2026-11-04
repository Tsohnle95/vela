//! Rounded clipping for composed native surfaces. A rounded background alone
//! cannot contain opaque descendants; clipping must enclose their whole paint.
use gpui::{prelude::*, *};

pub struct RoundedSurface {
    content: AnyElement,
    radius: Pixels,
}
impl RoundedSurface {
    pub fn new(content: impl IntoElement, radius: Pixels) -> Self {
        Self {
            content: content.into_any_element(),
            radius,
        }
    }
}
impl IntoElement for RoundedSurface {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for RoundedSurface {
    type RequestLayoutState = ();
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
    ) -> (LayoutId, ()) {
        (self.content.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let _ = self.content.prepaint(window, cx);
        });
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_rounded_clip(bounds, self.radius, |window| self.content.paint(window, cx));
    }
}

/// Keep pane backgrounds inside the card's curved hairline.
pub(crate) fn codex_card(content: impl IntoElement) -> AnyElement {
    let theme = crate::theme::Theme::default();
    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .h_full()
        .rounded(px(10.))
        .border_t_1()
        .border_l_1()
        .border_color(theme.text.opacity(0.05))
        .bg(theme.top)
        .overflow_hidden()
        .flex()
        .child(RoundedSurface::new(content, px(9.)))
        .into_any_element()
}
