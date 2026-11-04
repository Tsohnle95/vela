//! Confined workspace file access for the explorer and editor.
//!
//! All paths are session-relative, `/`-separated, and validated before use. The
//! resolver canonicalizes the workspace root and rejects traversal, absolute
//! paths, NUL/backslash components, symlinked targets, and writes outside the
//! root. I/O is bounded and runs on a blocking worker, never the GPUI thread.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_WRITE_BYTES: usize = 8 * 1024 * 1024;
const SKIP_NAMES: [&str; 3] = [".DS_Store", ".git", ".vela-recovery"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub rel: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Changed(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, FileError>;

pub(crate) fn normalize_rel(rel: &str) -> Result<&str> {
    if rel.is_empty() {
        return Ok("");
    }
    if rel.starts_with('/') || rel.contains('\\') || rel.contains('\0') {
        return Err(FileError::Invalid("invalid workspace path".into()));
    }
    for component in rel.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(FileError::Invalid("invalid workspace path".into()));
        }
    }
    Ok(rel)
}

pub(crate) fn confine(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel = normalize_rel(rel)?;
    let canonical_root = root.canonicalize()?;
    if !canonical_root.is_dir() {
        return Err(FileError::Invalid("workspace is not a directory".into()));
    }
    let joined = if rel.is_empty() {
        canonical_root.clone()
    } else {
        let path = canonical_root.join(rel);
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Err(FileError::Invalid("symlinks are not accessible".into()));
        }
        let canonical = path.canonicalize()?;
        if !canonical.starts_with(&canonical_root) {
            return Err(FileError::Invalid("path escapes the workspace".into()));
        }
        canonical
    };
    Ok(joined)
}

/// Resolve a user-selected entry for the platform file manager off the UI thread.
pub fn resolve_entry(root: &Path, rel: &str) -> Result<PathBuf> {
    confine(root, rel)
}

pub fn list_dir(root: &Path, rel: &str) -> Result<Vec<FileEntry>> {
    crate::recovery::initialize(root)?;
    let directory = confine(root, rel)?;
    normalize_rel(rel)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP_NAMES.contains(&name.as_str()) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let is_dir = metadata.is_dir();
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        entries.push(FileEntry {
            name,
            rel: child_rel,
            is_dir,
            size: if is_dir { 0 } else { metadata.len() },
        });
    }
    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

pub fn read_file(root: &Path, rel: &str) -> Result<String> {
    crate::recovery::initialize(root)?;
    let path = confine(root, rel)?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() {
        return Err(FileError::Invalid("not a file".into()));
    }
    if metadata.len() > MAX_READ_BYTES {
        return Err(FileError::Invalid(format!(
            "file is larger than {} MiB",
            MAX_READ_BYTES / 1_048_576
        )));
    }
    let bytes = fs::read(&path)?;
    String::from_utf8(bytes).map_err(|_| FileError::Invalid("file is not valid UTF-8".into()))
}

/// Recoverable compare-and-write. The held on-disk source must match the exact
/// snapshot the editor last acknowledged before a new inode is installed.
pub fn write_file(
    root: &Path,
    rel: &str,
    content: &str,
    expected: &str,
    overwrite: bool,
) -> Result<()> {
    crate::recovery::write_file(root, rel, content, expected, overwrite)
}

#[derive(Clone, Debug)]
pub struct RemoveEntryRequest {
    pub batch: u64,
    pub id: u64,
    pub session: String,
    pub workspace: String,
    pub rel: String,
}

#[derive(Clone, Debug)]
pub struct RenameSnapshot {
    pub rel: String,
    pub change: vela_core::document::ExternalChange,
    pub source_unchanged: bool,
}

#[derive(Clone, Debug)]
pub struct RenameEntryRequest {
    /// None renames in place; Some moves to this existing relative folder.
    pub target_parent: Option<String>,
    pub id: u64,
    pub session: String,
    pub workspace: String,
    pub rel: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct CreateEntryRequest {
    pub id: u64,
    pub session: String,
    pub workspace: String,
    pub parent: String,
    pub name: String,
    pub is_dir: bool,
}

/// Resolve an existing parent before creating a new final component. Both
/// creation operations fail atomically when the destination already exists.
pub(crate) fn new_entry_path(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel = normalize_rel(rel)?;
    if rel.is_empty() {
        return Err(FileError::Invalid("file path is empty".into()));
    }
    let (parent, name) = rel.rsplit_once('/').unwrap_or(("", rel));
    let parent = confine(root, parent)?;
    if !parent.is_dir() {
        return Err(FileError::Invalid("parent is not a directory".into()));
    }
    Ok(parent.join(name))
}

pub fn create_file(root: &Path, rel: &str) -> Result<()> {
    crate::recovery::initialize(root)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(new_entry_path(root, rel)?)?;
    Ok(())
}

pub fn create_directory(root: &Path, rel: &str) -> Result<()> {
    crate::recovery::initialize(root)?;
    fs::create_dir(new_entry_path(root, rel)?)?;
    Ok(())
}

pub fn create_entry(root: &Path, parent: &str, name: &str, is_dir: bool) -> Result<String> {
    normalize_rel(parent)?;
    let name = name.trim();
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name == "."
        || name == ".."
    {
        return Err(FileError::Invalid("Invalid name".into()));
    }
    let rel = if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    };
    if is_dir {
        create_directory(root, &rel)?;
    } else {
        create_file(root, &rel)?;
    }
    Ok(rel)
}

/// Ensure a relative path only contains normal components. Exposed for tests
/// and for future mutation surfaces that do not touch the filesystem yet.
pub fn is_safe_rel(rel: &str) -> bool {
    if rel.is_empty() {
        return true;
    }
    if rel.starts_with('/') || rel.contains('\\') || rel.contains('\0') {
        return false;
    }
    rel.split('/')
        .all(|component| !component.is_empty() && component != "." && component != "..")
}

pub(crate) fn file_identity(metadata: &fs::Metadata) -> Option<(u64, u64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (metadata.ino() != 0).then(|| (metadata.dev(), metadata.ino()))
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Some((
            u64::from(metadata.volume_serial_number()?),
            metadata.file_index()?,
        ))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = metadata;
        None
    }
}

pub fn read_standalone(path: &str) -> Result<String> {
    let requested = Path::new(path);
    if !requested.is_absolute() {
        return Err(FileError::Invalid("standalone path must be absolute".into()));
    }
    let path = requested.canonicalize()?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() {
        return Err(FileError::Invalid("not a file".into()));
    }
    if metadata.len() > MAX_READ_BYTES {
        return Err(FileError::Invalid(format!(
            "file is larger than {} MiB",
            MAX_READ_BYTES / 1_048_576
        )));
    }
    Ok(fs::read_to_string(path)?)
}

/// Write a standalone external file through a private sibling temporary. This
/// deliberately bypasses workspace recovery: recovery belongs to the active
/// workspace and must never be created in an arbitrary external parent.
pub fn write_standalone(
    path: &str,
    content: &str,
    expected: &str,
    overwrite: bool,
) -> Result<()> {
    if content.len() > MAX_WRITE_BYTES {
        return Err(FileError::Invalid(format!(
            "file is larger than {} MiB",
            MAX_WRITE_BYTES / 1_048_576
        )));
    }
    let requested = Path::new(path);
    if !requested.is_absolute() {
        return Err(FileError::Invalid("standalone path must be absolute".into()));
    }
    let target = match requested.canonicalize() {
        Ok(target) => target,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = requested
                .parent()
                .ok_or_else(|| FileError::Invalid("file has no parent".into()))?;
            fs::create_dir_all(parent)?;
            let parent = parent.canonicalize()?;
            let name = requested
                .file_name()
                .ok_or_else(|| FileError::Invalid("file has no name".into()))?;
            parent.join(name)
        }
        Err(error) => return Err(error.into()),
    };
    if fs::symlink_metadata(&target).is_ok_and(|metadata| !metadata.is_file()) {
        return Err(FileError::Invalid("not a file".into()));
    }
    let current = fs::read_to_string(&target).ok();
    if !overwrite && current.as_deref().is_some_and(|current| current != expected) {
        return Err(FileError::Changed(
            current.unwrap_or_else(|| "file changed on disk".into()),
        ));
    }
    let parent = target
        .parent()
        .ok_or_else(|| FileError::Invalid("file has no parent".into()))?;
    let temporary = parent.join(format!(".vela-{}.tmp", uuid::Uuid::new_v4()));
    struct Stage(PathBuf);
    impl Drop for Stage {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let stage = Stage(temporary.clone());
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    use std::io::Write;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    drop(file);
    #[cfg(windows)]
    if target.exists() {
        fs::remove_file(&target)?;
    }
    fs::rename(&temporary, &target)?;
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    drop(stage);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        // Keep recoverable write artifacts inside this disposable fixture.
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("README.md"), "# Demo\n").unwrap();
        fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        temp
    }

    #[test]
    fn new_entries_preserve_existing_data_and_confine_parents() {
        let temp = fixture();
        assert_eq!(
            create_entry(temp.path(), "", "  界 folder  ", true).unwrap(),
            "界 folder"
        );
        assert_eq!(
            create_entry(temp.path(), "界 folder", "🌍.txt", false).unwrap(),
            "界 folder/🌍.txt"
        );
        fs::write(temp.path().join("界 folder/🌍.txt"), "retain me").unwrap();
        assert!(create_entry(temp.path(), "界 folder", "🌍.txt", false).is_err());
        assert!(create_entry(temp.path(), "界 folder", "🌍.txt", true).is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join("界 folder/🌍.txt")).unwrap(),
            "retain me"
        );
        for name in [
            "",
            " ",
            ".",
            "..",
            "../escape",
            "/absolute",
            "a/b",
            "a\\b",
            "bad\0name",
        ] {
            assert!(
                create_entry(temp.path(), "", name, false).is_err(),
                "{name:?}"
            );
        }
        assert!(create_entry(temp.path(), "../outside", "file", false).is_err());
        assert!(create_entry(temp.path(), "README.md", "file", false).is_err());
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(outside.path(), temp.path().join("outside")).unwrap();
            assert!(create_entry(temp.path(), "outside", "escape", false).is_err());
            assert!(!outside.path().join("escape").exists());
        }
    }

    #[test]
    fn lists_directories_first_and_hides_metadata() {
        let temp = fixture();
        fs::write(temp.path().join(".DS_Store"), b"x").unwrap();
        let entries = list_dir(temp.path(), "").unwrap();
        assert_eq!(entries[0].name, "src");
        assert!(entries[0].is_dir);
        assert!(entries.iter().all(|entry| entry.name != ".DS_Store"));
        assert_eq!(entries[1].name, "README.md");
    }

    #[test]
    fn directory_inventory_does_not_silently_drop_entries_after_one_thousand() {
        let temp = fixture();
        fs::create_dir(temp.path().join("many")).unwrap();
        for index in 0..1205 {
            fs::write(temp.path().join(format!("many/file-{index:04}.txt")), "").unwrap();
        }
        let entries = list_dir(temp.path(), "many").unwrap();
        assert_eq!(entries.len(), 1205);
        assert_eq!(entries.first().unwrap().name, "file-0000.txt");
        assert_eq!(entries.last().unwrap().name, "file-1204.txt");
    }

    #[test]
    fn reads_and_writes_files_atomically() {
        let temp = fixture();
        let before = read_file(temp.path(), "README.md").unwrap();
        assert_eq!(before, "# Demo\n");
        write_file(temp.path(), "README.md", "updated\n", "# Demo\n", false).unwrap();
        assert_eq!(read_file(temp.path(), "README.md").unwrap(), "updated\n");
        assert!(!temp.path().join("README.md").read_dir().is_ok());
    }

    #[test]
    fn rejects_traversal_absolute_and_backslashes() {
        let temp = fixture();
        for rel in [
            "../secret",
            "/etc/passwd",
            "src\\main.rs",
            "src//main.rs",
            "./README.md",
            "",
        ] {
            if rel.is_empty() {
                continue;
            }
            assert!(read_file(temp.path(), rel).is_err(), "accepted {rel}");
        }
    }

    #[test]
    fn rejects_symlinks() {
        let temp = fixture();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), "no").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path().join("secret"), temp.path().join("link"))
            .unwrap();
        #[cfg(unix)]
        assert!(read_file(temp.path(), "link").is_err());
    }

    #[test]
    fn creates_new_files_without_overwriting() {
        let temp = fixture();
        create_file(temp.path(), "new.txt").unwrap();
        assert!(create_file(temp.path(), "new.txt").is_err());
        assert!(read_file(temp.path(), "new.txt").unwrap().is_empty());
    }

    #[test]
    fn safe_rel_matches_validation() {
        assert!(is_safe_rel("src/main.rs"));
        assert!(!is_safe_rel("../x"));
        assert!(!is_safe_rel("/x"));
        assert!(!is_safe_rel("src//x"));
    }

    #[test]
    fn standalone_files_read_write_conflict_and_recreate_without_workspace_recovery() {
        let outside = tempfile::tempdir().unwrap();
        let path = outside.path().join("界 notes.txt");
        let path_text = path.to_str().unwrap();
        fs::write(&path, "original").unwrap();
        assert_eq!(read_standalone(path_text).unwrap(), "original");
        write_standalone(path_text, "updated", "original", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "updated");
        assert!(matches!(
            write_standalone(path_text, "lost", "original", false),
            Err(FileError::Changed(_))
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), "updated");
        write_standalone(path_text, "forced", "original", true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "forced");
        fs::remove_file(&path).unwrap();
        write_standalone(path_text, "recreated", "forced", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "recreated");
        assert!(!outside.path().join(".vela-recovery").exists());
        assert_eq!(
            fs::read_dir(outside.path())
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".vela-")
                })
                .count(),
            0
        );
        assert!(read_standalone("relative.txt").is_err());
    }
}
