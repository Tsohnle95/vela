//! Recoverable OS Trash only. Never fall back to permanent filesystem removal.
use crate::files::{self, FileError, Result};
use std::path::Path;

pub fn entry(root: &Path, rel: &str) -> Result<()> {
    entry_with(root, rel, platform)
}
fn entry_with(root: &Path, rel: &str, trash: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    files::normalize_rel(rel)?;
    if rel.is_empty()
        || rel.split('/').any(|part| {
            matches!(
                part,
                ".git"
                    | ".vela-recovery"
                    | ".mira-recovery"
                    | ".openshell-recovery"
                    | ".orbit-recovery"
            )
        })
    {
        return Err(FileError::Invalid(
            "cannot remove workspace or recovery storage".into(),
        ));
    }
    crate::recovery::initialize(root)?;
    let source = files::confine(root, rel)?;
    trash(&source)
}
#[cfg(target_os = "macos")]
fn platform(path: &Path) -> Result<()> {
    use objc2_foundation::{NSFileManager, NSURL};
    use std::os::unix::ffi::OsStrExt;
    let bytes = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| FileError::Invalid("invalid file path".into()))?;
    objc2::rc::autoreleasepool(|_| {
        // SAFETY: bytes is a live NUL-terminated filesystem representation. The
        // URL is retained and used synchronously on this workspace I/O worker.
        let url = unsafe {
            NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
                std::ptr::NonNull::new(bytes.as_ptr().cast_mut()).expect("CString pointer"),
                path.is_dir(),
                None,
            )
        };
        NSFileManager::defaultManager()
            .trashItemAtURL_resultingItemURL_error(&url, None)
            .map_err(|error| FileError::Invalid(error.localizedDescription().to_string()))
    })
}
#[cfg(not(target_os = "macos"))]
fn platform(_: &Path) -> Result<()> {
    Err(FileError::Invalid(
        "OS Trash integration is unavailable on this platform".into(),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trash_rejects_root_traversal_and_recovery_and_preserves_source_on_failure() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("keep.txt"), "retain bytes").unwrap();
        for rel in [
            "",
            "../outside",
            "/absolute",
            ".git",
            ".git/vela-recovery/held",
            ".vela-recovery/held",
        ] {
            assert!(entry_with(root.path(), rel, |_| panic!("unsafe trash invocation")).is_err());
        }
        assert!(
            entry_with(root.path(), "keep.txt", |_| Err(FileError::Invalid(
                "denied".into()
            )))
            .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("keep.txt")).unwrap(),
            "retain bytes"
        );
    }
    #[test]
    fn trash_passes_exact_unicode_path_to_recoverable_platform_operation() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("界🌍")).unwrap();
        entry_with(root.path(), "界🌍", |path| {
            assert_eq!(path, root.path().canonicalize().unwrap().join("界🌍"));
            Ok(())
        })
        .unwrap();
        assert!(root.path().join("界🌍").is_dir());
    }
}
