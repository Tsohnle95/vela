//! Clipboard payloads that combine selected text with encoded image bytes.
//!
//! GPUI's macOS pasteboard backend only writes the string entries when a
//! `ClipboardItem` has more than one entry, so the macOS path writes the text
//! and image types to `NSPasteboard` directly. Other platforms use the
//! multi-entry `ClipboardItem` model.

#[cfg(any(not(target_os = "macos"), test))]
use gpui::ClipboardEntry;
use gpui::{App, ClipboardItem, Image};
use std::sync::Arc;

pub(super) fn write(cx: &mut App, text: &str, images: &[Arc<Image>]) {
    if images.is_empty() {
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
        return;
    }
    #[cfg(all(target_os = "macos", not(test)))]
    {
        let _ = cx;
        write_macos(text, images);
    }
    #[cfg(any(not(target_os = "macos"), test))]
    {
        let mut entries = Vec::new();
        if !text.is_empty() {
            entries.push(ClipboardEntry::String(gpui::ClipboardString::new(
                text.to_owned(),
            )));
        }
        entries.extend(
            images
                .iter()
                .map(|image| ClipboardEntry::Image((**image).clone())),
        );
        cx.write_to_clipboard(ClipboardItem { entries });
    }
}

#[cfg(all(target_os = "macos", not(test)))]
fn write_macos(text: &str, images: &[Arc<Image>]) {
    let pasteboard = objc2_app_kit::NSPasteboard::generalPasteboard();
    write_pasteboard(&pasteboard, text, images);
}

#[cfg(target_os = "macos")]
fn write_pasteboard(pasteboard: &objc2_app_kit::NSPasteboard, text: &str, images: &[Arc<Image>]) {
    use gpui::ImageFormat;
    use objc2_app_kit::{NSPasteboardTypePNG, NSPasteboardTypeString, NSPasteboardTypeTIFF};
    use objc2_foundation::{NSData, NSString};

    pasteboard.clearContents();
    if !text.is_empty() {
        let _ = pasteboard
            .setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
    }
    for image in images {
        let data = unsafe {
            NSData::dataWithBytes_length(image.bytes.as_ptr().cast(), image.bytes.len() as _)
        };
        unsafe {
            match image.format {
                ImageFormat::Png => {
                    pasteboard.setData_forType(Some(&data), NSPasteboardTypePNG);
                }
                ImageFormat::Tiff => {
                    pasteboard.setData_forType(Some(&data), NSPasteboardTypeTIFF);
                }
                format => {
                    if let Some(uti) = uti(format) {
                        let uti = NSString::from_str(uti);
                        pasteboard.setData_forType(Some(&data), &uti);
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn uti(format: gpui::ImageFormat) -> Option<&'static str> {
    use gpui::ImageFormat;
    match format {
        ImageFormat::Jpeg => Some("public.jpeg"),
        ImageFormat::Gif => Some("com.compuserve.gif"),
        ImageFormat::Webp => Some("org.webmproject.webp"),
        ImageFormat::Bmp => Some("com.microsoft.bmp"),
        ImageFormat::Svg => Some("public.svg-image"),
        ImageFormat::Ico => Some("com.microsoft.ico"),
        ImageFormat::Png | ImageFormat::Tiff | ImageFormat::Pnm => None,
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use core::ptr::NonNull;
    use gpui::ImageFormat;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeString};

    fn png_bytes() -> Vec<u8> {
        vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10]
    }

    /// The pasteboard service can apply `clearContents`/`setData` slightly
    /// after the call returns, so poll briefly before asserting absence or
    /// presence on the unique pasteboard.
    fn wait_for(condition: impl Fn() -> bool) -> bool {
        for _ in 0..500 {
            if condition() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        false
    }

    // The NSPasteboard service occasionally refuses writes when the parallel
    // test runner is creating/tearing down many GPUI windows; retries did not
    // make it deterministic. Run these two with `cargo test -- --ignored`
    // when touching the pasteboard path.
    #[test]
    #[ignore = "NSPasteboard is flaky under the parallel test runner"]
    fn macos_pasteboard_keeps_text_and_png_entries_together() {
        let pasteboard = NSPasteboard::pasteboardWithUniqueName();
        let png = png_bytes();
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, png.clone()));
        write_pasteboard(&pasteboard, "copied text", &[image]);

        let text = pasteboard
            .stringForType(unsafe { NSPasteboardTypeString })
            .map(|text| text.to_string());
        assert_eq!(text.as_deref(), Some("copied text"));

        assert!(
            wait_for(|| unsafe { pasteboard.dataForType(NSPasteboardTypePNG) }.is_some()),
            "PNG entry is present"
        );
        let data = pasteboard
            .dataForType(unsafe { NSPasteboardTypePNG })
            .expect("PNG entry is present");
        assert_eq!(data.length(), png.len());
        let mut bytes = vec![0u8; png.len()];
        unsafe {
            data.getBytes_length(NonNull::new_unchecked(bytes.as_mut_ptr().cast()), png.len());
        }
        assert_eq!(bytes, png);
    }

    #[test]
    #[ignore = "NSPasteboard is flaky under the parallel test runner"]
    fn macos_pasteboard_keeps_image_only_payloads() {
        let pasteboard = NSPasteboard::pasteboardWithUniqueName();
        let png = png_bytes();
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, png.clone()));
        let mut present = false;
        for _ in 0..3 {
            write_pasteboard(&pasteboard, "", std::slice::from_ref(&image));
            if wait_for(|| unsafe { pasteboard.dataForType(NSPasteboardTypePNG) }.is_some()) {
                present = true;
                break;
            }
        }
        assert!(present, "the image entry survives without text");
        assert!(
            pasteboard
                .stringForType(unsafe { NSPasteboardTypeString })
                .is_none(),
            "image-only payloads do not invent text"
        );
    }
}
