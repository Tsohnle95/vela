//! Bounded external Explorer imports. Each source installs atomically without
//! replacing an existing destination; the selected originals remain untouched.
use crate::files::{self, FileError, Result};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Request {
    pub id: u64,
    pub session: String,
    pub workspace: String,
    pub parent: String,
    pub sources: Vec<PathBuf>,
}
#[derive(Clone, Debug)]
pub struct Outcome {
    pub name: String,
    pub rel: String,
    pub imported: bool,
    pub reason: Option<String>,
    pub files: Vec<String>,
}
#[derive(Clone, Copy)]
struct Limits {
    files: usize,
    bytes: u64,
}
const LIMITS: Limits = Limits {
    files: 10_000,
    bytes: 2 * 1024 * 1024 * 1024,
};
struct CopyBudget {
    files: Vec<String>,
    bytes: u64,
    limits: Limits,
}
fn invalid(message: &str) -> FileError {
    FileError::Invalid(message.into())
}
fn protected(rel: &str) -> bool {
    rel.split('/').any(|part| {
        matches!(
            part,
            ".git"
                | ".vela-recovery"
                | ".mira-recovery"
                | ".openshell-recovery"
                | ".orbit-recovery"
        )
    })
}
fn skip_directory(name: &str) -> bool {
    protected(name)
        || matches!(
            name,
            "node_modules"
                | ".next"
                | ".venv"
                | "__pycache__"
                | ".cache"
                | ".turbo"
                | ".nx"
                | ".svn"
                | ".hg"
                | ".idea"
                | ".vscode"
                | "dist"
                | "out"
                | "build"
                | "target"
                | "coverage"
                | ".pytest_cache"
                | ".opencode"
                | ".claude"
                | ".cursor"
                | ".aider"
                | ".windsurf"
                | ".codeium"
                | ".roo"
                | ".gemini"
                | ".kilocode"
                | ".continue"
        )
}
impl CopyBudget {
    fn copy(&mut self, source: &Path, target: &Path, rel: &str) -> Result<()> {
        let metadata = fs::symlink_metadata(source)?;
        if metadata.file_type().is_symlink() {
            return Ok(());
        }
        if metadata.is_dir() {
            fs::create_dir(target)?;
            for entry in fs::read_dir(source)? {
                let entry = entry?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| invalid("unsupported file name"))?;
                let kind = entry.file_type()?;
                if kind.is_symlink() || (kind.is_dir() && skip_directory(&name)) {
                    continue;
                }
                files::normalize_rel(&name)?;
                let child = if rel.is_empty() {
                    name.clone()
                } else {
                    format!("{rel}/{name}")
                };
                self.copy(&entry.path(), &target.join(name), &child)?;
            }
            #[cfg(unix)]
            {
                fs::File::open(target)?.sync_all()?;
            }
            return Ok(());
        }
        if !metadata.is_file() {
            return Ok(());
        }
        if self.files.len() >= self.limits.files {
            return Err(invalid("too many files to import"));
        }
        let remaining = self.limits.bytes.saturating_sub(self.bytes);
        if metadata.len() > remaining {
            return Err(invalid("import is too large"));
        }
        let mut input = fs::File::open(source)?.take(remaining + 1);
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        let copied = std::io::copy(&mut input, &mut output)?;
        if copied > remaining {
            return Err(invalid("import is too large"));
        }
        output.flush()?;
        output.set_permissions(metadata.permissions())?;
        output.sync_all()?;
        self.bytes += copied;
        self.files.push(rel.to_owned());
        Ok(())
    }
}

pub fn copy(root: &Path, parent: &str, sources: &[PathBuf]) -> Result<Vec<Outcome>> {
    copy_with_limits(root, parent, sources, LIMITS, |_, _| Ok(()))
}
fn copy_with_limits(
    root: &Path,
    parent: &str,
    sources: &[PathBuf],
    limits: Limits,
    mut before_install: impl FnMut(&Path, &str) -> Result<()>,
) -> Result<Vec<Outcome>> {
    crate::recovery::initialize(root)?;
    let root = root.canonicalize()?;
    files::normalize_rel(parent)?;
    if protected(parent) {
        return Err(invalid("cannot import into workspace metadata"));
    }
    let destination = files::confine(&root, parent)?;
    if !destination.is_dir() {
        return Err(invalid("import destination is not a directory"));
    }
    let mut outcomes = Vec::with_capacity(sources.len());
    for source in sources {
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned();
        let rel = if parent.is_empty() {
            name.clone()
        } else {
            format!("{parent}/{name}")
        };
        let result = (|| {
            if name.is_empty()
                || !files::is_safe_rel(&name)
                || name.contains('/')
                || protected(&name)
            {
                return Err(invalid("invalid import name"));
            }
            let real = source.canonicalize().map_err(|_| invalid("not found"))?;
            if real.starts_with(&root) {
                return Err(invalid("already in the workspace"));
            }
            if root.starts_with(&real) {
                return Err(invalid("import source contains the workspace"));
            }
            let metadata = fs::metadata(&real)?;
            if !metadata.is_dir() && !metadata.is_file() {
                return Err(invalid("unsupported import entry"));
            }
            crate::recovery::install_import(&root, &rel, |stage| {
                let mut budget = CopyBudget {
                    files: Vec::new(),
                    bytes: 0,
                    limits,
                };
                budget.copy(&real, stage, "")?;
                before_install(&root, &rel)?;
                Ok(budget
                    .files
                    .into_iter()
                    .map(|child| {
                        if child.is_empty() {
                            rel.clone()
                        } else {
                            format!("{rel}/{child}")
                        }
                    })
                    .collect::<Vec<_>>())
            })
        })();
        let reason = result.as_ref().err().map(|error| match error {
            FileError::Io(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                "already exists".into()
            }
            other => other.to_string(),
        });
        outcomes.push(Outcome {
            name,
            rel,
            imported: result.is_ok(),
            reason,
            files: result.unwrap_or_default(),
        });
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_files_folders_binary_unicode_and_preserves_originals_and_collisions() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(outside.path().join("folder")).unwrap();
        fs::write(outside.path().join("folder/界🌍.bin"), [0, 255, 1, 2]).unwrap();
        fs::write(outside.path().join("plain.txt"), "original").unwrap();
        let sources = [
            outside.path().join("folder"),
            outside.path().join("plain.txt"),
        ];
        let outcomes = copy(root.path(), "", &sources).unwrap();
        assert!(outcomes.iter().all(|item| item.imported));
        assert_eq!(outcomes[0].files, ["folder/界🌍.bin"]);
        assert_eq!(
            fs::read(root.path().join("folder/界🌍.bin")).unwrap(),
            [0, 255, 1, 2]
        );
        fs::write(root.path().join("plain.txt"), "preserved target").unwrap();
        let repeated = copy(root.path(), "", &sources).unwrap();
        assert!(
            repeated
                .iter()
                .all(|item| item.reason.as_deref() == Some("already exists"))
        );
        assert_eq!(
            fs::read_to_string(root.path().join("plain.txt")).unwrap(),
            "preserved target"
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("plain.txt")).unwrap(),
            "original"
        );
    }
    #[test]
    fn skips_nested_links_dependencies_and_rejects_scope_and_resource_limits_without_partial_targets()
     {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let folder = outside.path().join("folder");
        fs::create_dir(&folder).unwrap();
        fs::create_dir(folder.join("node_modules")).unwrap();
        fs::write(folder.join("node_modules/ignored"), "skip").unwrap();
        fs::write(folder.join("a"), "abc").unwrap();
        fs::write(folder.join("b"), "def").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("a", folder.join("link")).unwrap();
        let sources = [folder.clone()];
        for limits in [
            Limits {
                files: 1,
                bytes: 100,
            },
            Limits {
                files: 10,
                bytes: 4,
            },
        ] {
            assert!(
                !copy_with_limits(root.path(), "", &sources, limits, |_, _| Ok(())).unwrap()[0]
                    .imported
            );
            assert!(!root.path().join("folder").exists());
        }
        assert!(copy(root.path(), "../escape", &sources).is_err());
        assert!(copy(root.path(), ".git", &sources).is_err());
        assert!(!copy(root.path(), "", &[root.path().to_path_buf()]).unwrap()[0].imported);
        assert!(!copy(root.path(), "", &[outside.path().join("missing")]).unwrap()[0].imported);
        assert!(copy(root.path(), "", &sources).unwrap()[0].imported);
        assert!(!root.path().join("folder/node_modules").exists());
        assert!(!root.path().join("folder/link").exists());
    }
    #[test]
    fn destination_created_during_copy_survives_atomic_install() {
        for directory in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            let source = outside.path().join("collision");
            if directory {
                fs::create_dir(&source).unwrap();
                fs::write(source.join("child"), "source").unwrap();
            } else {
                fs::write(&source, "source").unwrap();
            }
            let result =
                copy_with_limits(root.path(), "", std::slice::from_ref(&source), LIMITS, |root, rel| {
                    if directory {
                        fs::create_dir(root.join(rel))?;
                    } else {
                        fs::write(root.join(rel), "external target")?;
                    }
                    Ok(())
                })
                .unwrap();
            assert!(!result[0].imported);
            if directory {
                assert_eq!(
                    fs::read_dir(root.path().join("collision")).unwrap().count(),
                    0
                );
                assert!(source.join("child").exists());
            } else {
                assert_eq!(
                    fs::read_to_string(root.path().join("collision")).unwrap(),
                    "external target"
                );
                assert_eq!(fs::read_to_string(source).unwrap(), "source");
            }
        }
    }
}
