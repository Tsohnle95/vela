//! The reference's HTML select opens a macOS menu. Keep the collapsed control
//! in GPUI and use AppKit for the actual menu, including its tracking loop,
//! keyboard navigation, dismissal, selection checkmark and accessibility.
#[cfg(target_os = "macos")]
mod macos {
    use gpui::{Bounds, Pixels, Window};
    use objc2::{
        AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
        rc::Retained, sel,
    };
    use objc2_app_kit::{
        NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSFont, NSFontAttributeName,
        NSLineBreakMode, NSMenu, NSMenuItem, NSMutableParagraphStyle,
        NSParagraphStyleAttributeName, NSPopUpButtonCell, NSTextAlignment, NSView,
        NSWritingDirection,
    };
    use objc2_foundation::{
        NSAttributedString, NSDictionary, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
        NSString,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::cell::Cell;

    define_class!(
        // SAFETY: NSObject has no subclass requirements. AppKit invokes the
        // selector on the main thread while the target is retained by Popup.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[ivars = Cell<Option<usize>>]
        struct Selection;
        unsafe impl NSObjectProtocol for Selection {}
        impl Selection {
            #[unsafe(method(choose:))]
            fn choose(&self, item: &NSMenuItem) { self.ivars().set(usize::try_from(item.tag()).ok()); }
        }
    );
    impl Selection {
        fn new(main: MainThreadMarker) -> Retained<Self> {
            let this = Self::alloc(main).set_ivars(Cell::new(None));
            // SAFETY: NSObject's designated initializer initializes this subclass.
            unsafe { msg_send![super(this), init] }
        }
    }
    pub(crate) struct Popup {
        cell: Retained<NSPopUpButtonCell>,
        control: Retained<NSView>,
        target: Retained<Selection>,
        view: Retained<NSView>,
    }
    impl Popup {
        pub fn new(
            window: &Window,
            bounds: Bounds<Pixels>,
            labels: &[String],
            selected: usize,
            font_size: f64,
        ) -> Option<Self> {
            let main = MainThreadMarker::new()?;
            let RawWindowHandle::AppKit(handle) =
                HasWindowHandle::window_handle(window).ok()?.as_raw()
            else {
                return None;
            };
            // SAFETY: GPUI's raw AppKit handle is an NSView. Retain it through
            // tracking; the menu and target never leave the main thread.
            let view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>())? };
            let menu = NSMenu::initWithTitle(NSMenu::alloc(main), &NSString::from_str(""));
            menu.setAutoenablesItems(false);
            let target = Selection::new(main);
            for (index, label) in labels.iter().enumerate() {
                // SAFETY: choose: has the NSMenuItem target/action signature,
                // and Popup retains its target throughout the tracking loop.
                let item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(main),
                        &NSString::from_str(label),
                        Some(sel!(choose:)),
                        &NSString::from_str(""),
                    )
                };
                let paragraph = NSMutableParagraphStyle::new();
                paragraph.setAlignment(NSTextAlignment::Left);
                paragraph.setBaseWritingDirection(NSWritingDirection::LeftToRight);
                paragraph.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
                let font = NSFont::menuFontOfSize(font_size);
                let attributes = NSDictionary::from_slices(
                    &[unsafe { NSParagraphStyleAttributeName }, unsafe {
                        NSFontAttributeName
                    }],
                    &[
                        &*paragraph as &objc2::runtime::AnyObject,
                        &*font as &objc2::runtime::AnyObject,
                    ],
                );
                // SAFETY: the attributes contain the required AppKit types:
                // NSParagraphStyle and NSFont, as used by Chromium 138.
                let title = unsafe {
                    NSAttributedString::initWithString_attributes(
                        NSAttributedString::alloc(),
                        &NSString::from_str(label),
                        Some(&attributes),
                    )
                };
                item.setAttributedTitle(Some(&title));
                item.setTitle(&NSString::from_str(label.trim()));
                item.setTag(index as isize);
                unsafe {
                    item.setTarget(Some(&target));
                }
                menu.addItem(&item);
            }
            let x = f64::from(f32::from(bounds.left()));
            let y = if view.isFlipped() {
                f64::from(f32::from(bounds.top()))
            } else {
                view.bounds().size.height - f64::from(f32::from(bounds.bottom()))
            };
            let frame = NSRect::new(
                NSPoint::new(x, y),
                NSSize::new(
                    f64::from(f32::from(bounds.size.width)),
                    f64::from(f32::from(bounds.size.height)),
                ),
            );
            let control = NSView::initWithFrame(NSView::alloc(main), frame);
            let cell = NSPopUpButtonCell::initTextCell_pullsDown(
                NSPopUpButtonCell::alloc(main),
                &NSString::from_str(""),
                false,
            );
            cell.setMenu(Some(&menu));
            cell.selectItemWithTag(selected as isize);
            Some(Self {
                cell,
                control,
                target,
                view,
            })
        }
        /// Use the same public popup-cell path as Chromium. Direct NSMenu
        /// tracking has different sizing and selection rendering on macOS.
        /// https://raw.githubusercontent.com/chromium/chromium/138.0.7204.251/content/app_shim_remote_cocoa/web_menu_runner_mac.mm
        /// Run outside App/Entity updates because AppKit runs a nested loop.
        pub fn show(self) -> Option<usize> {
            self.view.addSubview(&self.control);
            self.cell
                .attachPopUpWithFrame_inView(self.control.bounds(), &self.control);
            self.cell
                .performClickWithFrame_inView(self.control.bounds(), &self.control);
            self.control.removeFromSuperview();
            self.target.ivars().get()
        }
    }
    pub fn sync_appearance(dark: bool) {
        let Some(main) = MainThreadMarker::new() else {
            return;
        };
        let name = unsafe {
            if dark {
                NSAppearanceNameDarkAqua
            } else {
                NSAppearanceNameAqua
            }
        };
        objc2_app_kit::NSApplication::sharedApplication(main)
            .setAppearance(NSAppearance::appearanceNamed(name).as_deref());
    }
    pub fn selection_colors(focused: bool) -> (gpui::Rgba, gpui::Rgba) {
        use objc2_app_kit::{NSColor, NSColorSpace};
        let theme = crate::theme::Theme::default();
        let result = Cell::new((theme.accent, theme.text));
        let name = unsafe {
            if theme.dark {
                NSAppearanceNameDarkAqua
            } else {
                NSAppearanceNameAqua
            }
        };
        if let Some(appearance) = NSAppearance::appearanceNamed(name) {
            appearance.performAsCurrentDrawingAppearance(&block2::StackBlock::new(|| {
                let background = if focused {
                    NSColor::selectedTextBackgroundColor()
                } else {
                    NSColor::unemphasizedSelectedTextBackgroundColor()
                };
                let foreground = if focused {
                    NSColor::selectedTextColor()
                } else {
                    NSColor::unemphasizedSelectedTextColor()
                };
                let rgba = |color: &NSColor| {
                    let color = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
                    Some(gpui::Rgba {
                        r: color.redComponent() as f32,
                        g: color.greenComponent() as f32,
                        b: color.blueComponent() as f32,
                        a: 1.,
                    })
                };
                if let (Some(background), Some(foreground)) = (rgba(&background), rgba(&foreground))
                {
                    result.set((background, foreground));
                }
            }));
        }
        result.get()
    }
}
#[cfg(target_os = "macos")]
pub(crate) use macos::{Popup, selection_colors, sync_appearance};
#[cfg(not(target_os = "macos"))]
pub(crate) fn selection_colors(focused: bool) -> (gpui::Rgba, gpui::Rgba) {
    let theme = crate::theme::Theme::default();
    (
        if focused {
            theme.accent
        } else {
            theme.selected
        },
        theme.text,
    )
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn sync_appearance(_: bool) {}
