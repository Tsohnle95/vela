//! Terminal tray gestures mirror Source's useTrayHeight without UI dependencies.
#[derive(Clone, Copy)]
struct Drag {
    y: f32,
    height: f32,
}
pub struct Tray {
    pub open: bool,
    pub height: f32,
    pub snapped: bool,
    last_full: f32,
    drag: Option<Drag>,
}
impl Default for Tray {
    fn default() -> Self {
        Self {
            open: false,
            height: 176.,
            snapped: false,
            last_full: 176.,
            drag: None,
        }
    }
}
impl Tray {
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }
    pub fn show(&mut self) {
        self.height = self.last_full;
        self.snapped = false;
        self.open = true;
    }
    pub fn toggle(&mut self) {
        if !self.open || self.snapped {
            self.show();
        } else {
            self.close();
        }
    }
    pub fn close(&mut self) {
        self.open = false;
    }
    pub fn begin(&mut self, y: f32) -> bool {
        if !self.open || self.drag.is_some() {
            return false;
        }
        self.drag = Some(Drag {
            y,
            height: self.height,
        });
        true
    }
    pub fn move_to(&mut self, y: f32, viewport_height: f32) {
        let Some(drag) = self.drag else {
            return;
        };
        if y >= viewport_height {
            self.drag = None;
            self.open = false;
            return;
        }
        self.height = (drag.height + drag.y - y).clamp(30., 520.);
        if self.height > 32. {
            self.last_full = self.height;
        }
    }
    pub fn finish(&mut self) -> bool {
        if self.drag.take().is_none() {
            return false;
        }
        if self.height <= 32. {
            self.height = 30.;
            self.snapped = true;
        } else {
            self.snapped = false;
        }
        self.open = true;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resize_snap_expand_and_toggle_follow_source_thresholds() {
        let mut tray = Tray::default();
        assert!(!tray.open);
        assert!(!tray.begin(300.));
        tray.toggle();
        assert_eq!(tray.height, 176.);
        assert!(tray.begin(300.));
        assert!(!tray.begin(300.));
        tray.move_to(100., 700.);
        assert_eq!(tray.height, 376.);
        tray.finish();
        tray.toggle();
        assert!(!tray.open);
        tray.toggle();
        assert_eq!(tray.height, 376.);
        tray.begin(300.);
        tray.move_to(644., 700.);
        assert_eq!(tray.height, 32.);
        tray.finish();
        assert!(tray.snapped);
        assert_eq!(tray.height, 30.);
        tray.toggle();
        assert!(tray.open);
        assert!(!tray.snapped);
        assert_eq!(tray.height, 376.);
        tray.begin(300.);
        tray.move_to(-1000., 700.);
        assert_eq!(tray.height, 520.);
        tray.finish();
    }
    #[test]
    fn dragging_below_window_hides_without_losing_the_last_full_size() {
        let mut tray = Tray::default();
        tray.show();
        tray.begin(400.);
        tray.move_to(300., 700.);
        assert_eq!(tray.height, 276.);
        tray.move_to(700., 700.);
        assert!(!tray.open);
        assert!(!tray.dragging());
        assert!(!tray.finish());
        tray.show();
        assert_eq!(tray.height, 276.);
        assert!(tray.open);
    }
}
