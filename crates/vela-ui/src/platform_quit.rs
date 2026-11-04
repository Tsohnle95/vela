//! AppKit termination (including Dock Quit) must consult the document guard
//! before GPUI's non-cancelable applicationWillTerminate notification.
#[cfg(target_os = "macos")]
pub(crate) fn install(callback: impl FnMut() -> bool + 'static) {
    use objc2::{
        MainThreadMarker,
        runtime::{AnyObject, ClassBuilder, Sel},
        sel,
    };
    use objc2_app_kit::{NSApplication, NSApplicationTerminateReply};
    use std::cell::RefCell;
    thread_local! {static GUARD:RefCell<Option<Box<dyn FnMut()->bool>>>=RefCell::new(None);}
    extern "C-unwind" fn should_terminate(
        _: &AnyObject,
        _: Sel,
        _: &NSApplication,
    ) -> NSApplicationTerminateReply {
        let allow = GUARD.with(|slot| {
            let Ok(mut callback) = slot.try_borrow_mut() else {
                return false;
            };
            callback.as_mut().is_none_or(|callback| callback())
        });
        if allow {
            NSApplicationTerminateReply::TerminateNow
        } else {
            NSApplicationTerminateReply::TerminateCancel
        }
    }
    GUARD.with(|slot| *slot.borrow_mut() = Some(Box::new(callback)));
    let main = MainThreadMarker::new().expect("quit guard must be installed on the AppKit thread");
    let delegate = NSApplication::sharedApplication(main)
        .delegate()
        .expect("GPUI application delegate");
    let object: &AnyObject = (*delegate).as_ref();
    let current = object.class();
    if current.name() == c"VelaDocumentQuitDelegate" {
        return;
    }
    assert_eq!(
        current.name(),
        c"GPUIApplicationDelegate",
        "unexpected GPUI application delegate"
    );
    assert!(
        current
            .instance_method(sel!(applicationShouldTerminate:))
            .is_none(),
        "GPUI already owns termination approval"
    );
    let mut builder = ClassBuilder::new(c"VelaDocumentQuitDelegate", current)
        .expect("unique quit delegate class");
    // SAFETY: AppKit specifies (id,SEL,NSApplication*) -> NSUInteger for this
    // selector. The typed implementation uses that exact Objective-C encoding.
    unsafe {
        builder.add_method(
            sel!(applicationShouldTerminate:),
            should_terminate as extern "C-unwind" fn(_, _, _) -> _,
        );
    }
    let class = builder.register();
    assert_eq!(current.instance_size(), class.instance_size());
    // SAFETY: this is an ivar-free subclass of this exact main-thread delegate;
    // existing GPUI ivars and methods retain their layout and implementation.
    let previous = unsafe { AnyObject::set_class(object, class) };
    assert!(std::ptr::eq(previous, current));
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn install(_: impl FnMut() -> bool + 'static) {}
