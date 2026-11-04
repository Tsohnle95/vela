use gpui::Window;
use vela_engine::window_state::WindowSize;

pub(crate) fn normal_size(window: &Window) -> Option<WindowSize> {
    if window.is_fullscreen() || window.is_maximized() || platform_excludes_size(window) {
        return None;
    }
    let size = window.bounds().size;
    WindowSize::new(
        f64::from(f32::from(size.width)),
        f64::from(f32::from(size.height)),
    )
}

#[cfg(target_os = "macos")]
fn platform_excludes_size(window: &Window) -> bool {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return true;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return true;
    };
    // SAFETY: GPUI's AppKit handle points to its retained NSView; this call is
    // made on the UI thread while the Window is borrowed and cannot be closed.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    view.window()
        .is_none_or(|window| window.isMiniaturized() || window.isZoomed())
}

#[cfg(not(target_os = "macos"))]
fn platform_excludes_size(_: &Window) -> bool {
    false
}
