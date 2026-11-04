//! Remove from Workspace retains the item in Vela's detached-item store.
use crate::files::{self, FileError, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Request {
    pub id: u64,
    pub session: String,
    pub workspace: String,
    pub rel: String,
}

pub fn entry(root: &Path, rel: &str) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| FileError::Invalid("home directory is unavailable".into()))?;
    entry_to(root, rel, &home.join(".vela-detached"))
}
fn entry_to(root: &Path, rel: &str, store: &Path) -> Result<PathBuf> {
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
            "cannot detach workspace or recovery storage".into(),
        ));
    }
    let source = files::confine(root, rel)?;
    match fs::symlink_metadata(store) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            return Err(FileError::Invalid(
                "detached-item store is not a directory".into(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(store)?;
        }
        Err(error) => return Err(error.into()),
    }
    let store = store.canonicalize()?;
    if store.starts_with(&source) {
        return Err(FileError::Invalid(
            "cannot detach the detached-item store".into(),
        ));
    }
    let name = source
        .file_name()
        .ok_or_else(|| FileError::Invalid("invalid detached entry".into()))?;
    let target = store.join(format!(
        "{}-{}",
        uuid::Uuid::new_v4(),
        name.to_string_lossy()
    ));
    crate::recovery::detach_entry(root, rel, &target)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detaching_file_and_folder_retains_disk_bytes_and_rejects_unsafe_paths() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let store = outside.path().join("store");
        fs::write(root.path().join("界🌍.txt"), "retained file").unwrap();
        fs::create_dir(root.path().join("folder")).unwrap();
        fs::write(root.path().join("folder/child"), "retained child").unwrap();
        let file = entry_to(root.path(), "界🌍.txt", &store).unwrap();
        let folder = entry_to(root.path(), "folder", &store).unwrap();
        assert_eq!(fs::read_to_string(file).unwrap(), "retained file");
        assert_eq!(
            fs::read_to_string(folder.join("child")).unwrap(),
            "retained child"
        );
        assert!(!root.path().join("界🌍.txt").exists());
        assert!(!root.path().join("folder").exists());
        for rel in ["", "../outside", ".git", ".vela-recovery/held", "missing"] {
            assert!(entry_to(root.path(), rel, &store).is_err());
        }
    }
    #[test]
    fn invalid_store_and_parent_failure_preserve_the_source() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(root.path().join("keep"), "source").unwrap();
        fs::write(outside.path().join("file"), "obstacle").unwrap();
        assert!(entry_to(root.path(), "keep", &outside.path().join("file")).is_err());
        assert!(entry_to(root.path(), "keep", &outside.path().join("missing/store")).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("keep")).unwrap(),
            "source"
        );
        fs::write(outside.path().join("occupied"), "external destination").unwrap();
        assert!(
            crate::recovery::detach_entry(root.path(), "keep", &outside.path().join("occupied"))
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("occupied")).unwrap(),
            "external destination"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("keep")).unwrap(),
            "source"
        );
    }
}
