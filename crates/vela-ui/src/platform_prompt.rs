//! Match the reference's AppKit document alerts, including its initial button
//! focus. GPUI otherwise focuses Don't Save, changing what Space does.
use gpui::{App, PromptButton, PromptLevel, Window};

pub(crate) fn document(
    window: &mut Window,
    message: &str,
    detail: Option<&str>,
    answers: &[PromptButton],
    cx: &mut App,
) -> impl Future<Output = usize> + use<> {
    #[cfg(target_os = "macos")]
    let native = macos::prompt(window, message, detail, answers);
    #[cfg(not(target_os = "macos"))]
    let native: Option<tokio::sync::oneshot::Receiver<usize>> = None;
    let fallback = native
        .is_none()
        .then(|| window.prompt(PromptLevel::Critical, message, detail, answers, cx));
    let cancel = answers
        .iter()
        .position(PromptButton::is_cancel)
        .unwrap_or(0);
    async move {
        if let Some(result) = native {
            result.await.unwrap_or(cancel)
        } else if let Some(result) = fallback {
            result.await.unwrap_or(cancel)
        } else {
            cancel
        }
    }
}

/// The reference's file input opens a sheet belonging to its window. Keep that
/// ownership so the parent cannot receive input while the picker is open.
pub(crate) fn files(
    window: &mut Window,
    cx: &mut App,
) -> impl Future<Output = Option<Vec<std::path::PathBuf>>> + use<> {
    #[cfg(target_os = "macos")]
    let native = macos::files(window);
    #[cfg(not(target_os = "macos"))]
    let native: Option<tokio::sync::oneshot::Receiver<Option<Vec<std::path::PathBuf>>>> = None;
    let fallback = native.is_none().then(|| {
        cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Open".into()),
        })
    });
    async move {
        if let Some(result) = native {
            result.await.ok().flatten()
        } else if let Some(result) = fallback {
            result.await.ok().and_then(Result::ok).flatten()
        } else {
            None
        }
    }
}

pub(crate) fn cancel_files(window: &Window) {
    #[cfg(target_os = "macos")]
    macos::cancel_files(window);
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use block2::RcBlock;
    use objc2::{MainThreadMarker, rc::Retained};
    use objc2_app_kit::{NSAlert, NSAlertStyle, NSView};
    use objc2_foundation::NSString;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::cell::{Cell, RefCell};

    type FilePicker = (usize, Retained<objc2_app_kit::NSOpenPanel>);
    thread_local! {static FILE_PICKER: RefCell<Option<FilePicker>> = const {RefCell::new(None)};}

    pub(super) fn cancel_files(window: &Window) {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return;
        };
        // SAFETY: this is the main-thread NSView retained by GPUI's window.
        let Some(view) = (unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) })
        else {
            return;
        };
        let Some(parent) = view.window() else {
            return;
        };
        let owner = Retained::as_ptr(&parent) as usize;
        // Release the slot borrow before AppKit invokes the completion block.
        let picker = FILE_PICKER.with(|slot| {
            slot.borrow()
                .as_ref()
                .filter(|(parent, _)| *parent == owner)
                .map(|(_, panel)| panel.clone())
        });
        if let Some(panel) = picker {
            // SAFETY: this is our live main-thread panel; cancel: accepts nil
            // as its sender and invokes the retained completion block.
            unsafe {
                panel.cancel(None);
            }
        }
    }

    pub(super) fn files(
        window: &Window,
    ) -> Option<tokio::sync::oneshot::Receiver<Option<Vec<std::path::PathBuf>>>> {
        let main = MainThreadMarker::new()?;
        let RawWindowHandle::AppKit(handle) = HasWindowHandle::window_handle(window).ok()?.as_raw()
        else {
            return None;
        };
        // SAFETY: the main-thread GPUI AppKit handle owns an NSView; retain its
        // parent during setup. AppKit retains the sheet and completion block.
        let view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>())? };
        let parent = view.window()?;
        let panel = objc2_app_kit::NSOpenPanel::openPanel(main);
        panel.setCanChooseFiles(true);
        panel.setCanChooseDirectories(false);
        panel.setAllowsMultipleSelection(true);
        panel.setCanCreateDirectories(false);
        panel.setResolvesAliases(false);
        let (send, receive) = tokio::sync::oneshot::channel();
        let send = Cell::new(Some(send));
        let retained_panel = panel.clone();
        let owner = Retained::as_ptr(&parent) as usize;
        FILE_PICKER.with(|slot| *slot.borrow_mut() = Some((owner, panel.clone())));
        let completion = RcBlock::new(move |response: isize| {
            FILE_PICKER.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot.as_ref().is_some_and(|(_, panel)| {
                    Retained::as_ptr(panel) == Retained::as_ptr(&retained_panel)
                }) {
                    slot.take();
                }
            });
            let files = (response == 1).then(|| {
                retained_panel
                    .URLs()
                    .iter()
                    .filter(|url| url.isFileURL())
                    .filter_map(|url| url.path())
                    .map(|path| std::path::PathBuf::from(path.to_string()))
                    .collect()
            });
            if let Some(send) = send.take() {
                let _ = send.send(files);
            }
        });
        panel.beginSheetModalForWindow_completionHandler(&parent, &completion);
        Some(receive)
    }

    pub(super) fn prompt(
        window: &Window,
        message: &str,
        detail: Option<&str>,
        answers: &[PromptButton],
    ) -> Option<tokio::sync::oneshot::Receiver<usize>> {
        let main = MainThreadMarker::new()?;
        let RawWindowHandle::AppKit(handle) = HasWindowHandle::window_handle(window).ok()?.as_raw()
        else {
            return None;
        };
        // SAFETY: GPUI's main-thread AppKit handle owns an NSView. Keep its
        // window alive through setup; AppKit retains the modal sheet afterward.
        let view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>())? };
        let parent = view.window()?;
        let alert = NSAlert::new(main);
        // Electron uses Critical for both warning and error message boxes.
        alert.setAlertStyle(NSAlertStyle::Critical);
        alert.setMessageText(&NSString::from_str(message));
        if let Some(detail) = detail {
            alert.setInformativeText(&NSString::from_str(detail));
        }
        for (index, answer) in answers.iter().enumerate() {
            let button = alert.addButtonWithTitle(&NSString::from_str(answer.label()));
            button.setTag(index as isize);
            if index == 0 {
                button.setKeyEquivalent(&NSString::from_str("\r"));
            }
            if answer.is_cancel() && answers.len() > 1 {
                button.setKeyEquivalent(&NSString::from_str("\u{1b}"));
            }
        }
        let (send, receive) = tokio::sync::oneshot::channel();
        let send = Cell::new(Some(send));
        let retained_alert = alert.clone();
        let completion = RcBlock::new(move |response: isize| {
            let _keep_alive = &retained_alert;
            if let Some(send) = send.take() {
                let _ = send.send(usize::try_from(response).unwrap_or(usize::MAX));
            }
        });
        alert.beginSheetModalForWindow_completionHandler(&parent, Some(&completion));
        Some(receive)
    }
}
