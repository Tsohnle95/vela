//! Read-only Git status and diff for the active workspace.
//!
//! Commands run directly (`git` with `-C`, never through a shell), paths are
//! validated as workspace-relative, and output is bounded. Mutating Git
//! operations remain out of scope until they can reuse Vela's recovery and
//! confirmation policies.

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::files::{self, FileError};

pub const MAX_GIT_OUTPUT: usize = 2 * 1024 * 1024;

/// Baseline reads may run for paths outside a repository. They never invoke a
/// shell or fetch missing objects, and both output and process lifetime are
/// bounded. `HEAD:./` keeps nested-workspace paths relative to the Git cwd.
pub(crate) fn head_file(root: &Path, rel: &str) -> Option<String> {
    if !files::is_safe_rel(rel) || rel.is_empty() {
        return None;
    }
    String::from_utf8(baseline_git(root, &["show", &format!("HEAD:./{rel}")])?).ok()
}

pub(crate) fn is_worktree(root: &Path) -> bool {
    baseline_git(root, &["rev-parse", "--is-inside-work-tree"])
        .is_some_and(|output| output.starts_with(b"true"))
}

fn baseline_git(root: &Path, arguments: &[&str]) -> Option<Vec<u8>> {
    use std::{
        io::Read,
        process::Stdio,
        time::{Duration, Instant},
    };
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::Builder::new()
        .name("vela-git-baseline".into())
        .spawn(move || {
            let mut bytes = Vec::new();
            let result = stdout
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .ok()
                .filter(|_| bytes.len() <= 16 * 1024 * 1024)
                .map(|_| bytes);
            let _ = tx.send(result);
        });
    let deadline = Instant::now() + Duration::from_secs(10);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline && reader.is_ok() => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => {
                #[cfg(unix)]
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let output = rx.recv_timeout(Duration::from_millis(100)).ok().flatten();
    if success { output } else { None }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitEntry {
    pub path: String,
    pub code: String,
    pub label: String,
    pub staged: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("{0}")]
    Invalid(String),
    #[error("git is unavailable: {0}")]
    Unavailable(String),
    #[error("git command failed: {0}")]
    Command(String),
    #[error(transparent)]
    File(#[from] FileError),
}

pub type Result<T> = std::result::Result<T, GitError>;

fn git(root: &Path, arguments: &[&str]) -> std::result::Result<std::process::Output, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "core.quotepath=false"])
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|error| GitError::Unavailable(error.to_string()))?;
    if output.stdout.len() > MAX_GIT_OUTPUT {
        return Err(GitError::Invalid("git output is too large".into()));
    }
    Ok(output)
}

pub fn status(root: &Path, rel: &str) -> Result<Vec<GitEntry>> {
    if !rel.is_empty() && !files::is_safe_rel(rel) {
        return Err(GitError::Invalid("invalid workspace path".into()));
    }
    let output = git(
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    if !output.status.success() {
        return Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(2_000)
                .collect(),
        ));
    }
    let mut parts = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let mut entries = Vec::new();
    while let Some(record) = parts.next() {
        if record.len() < 4 {
            continue;
        }
        let code = String::from_utf8_lossy(&record[..2]).to_string();
        let path = String::from_utf8_lossy(&record[3..]).to_string();
        // Rename/copy records are followed by the original path; keep the new
        // path and consume the source.
        if code.contains('R') || code.contains('C') {
            let _ = parts.next();
        }
        if !files::is_safe_rel(&path) {
            continue;
        }
        if !rel.is_empty() && !path.starts_with(rel.trim_end_matches('/')) {
            continue;
        }
        let staged = !code.starts_with(' ') && !code.starts_with('?');
        let label = match code.as_str() {
            "??" => "untracked",
            "!!" => "ignored",
            "MM" | " M" | "AM" => "modified",
            "M " => "staged",
            "A " => "added",
            " D" | "D " => "deleted",
            "R " | " R" | "RM" => "renamed",
            _ => "changed",
        };
        entries.push(GitEntry {
            path,
            code,
            label: label.into(),
            staged,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

pub fn diff(root: &Path, rel: &str) -> Result<String> {
    if !files::is_safe_rel(rel) || rel.is_empty() {
        return Err(GitError::Invalid("invalid workspace path".into()));
    }
    let entries = status(root, rel)?;
    let untracked = entries
        .iter()
        .any(|entry| entry.code == "??" && entry.path == rel);
    if untracked {
        let content = files::read_file(root, rel)?;
        let mut out = format!(
            "diff --git a/{rel} b/{rel}\nnew file mode 100644\n--- /dev/null\n+++ b/{rel}\n@@ -0,0 +1,{} @@\n",
            content.lines().count()
        );
        for line in content.lines() {
            out.push('+');
            out.push_str(line);
            out.push('\n');
        }
        return Ok(out);
    }
    let mut output = git(
        root,
        &["diff", "--no-color", "--unified=3", "HEAD", "--", rel],
    )?;
    if !output.status.success() || output.stdout.is_empty() {
        output = git(root, &["diff", "--no-color", "--unified=3", "--", rel])?;
    }
    if !output.status.success() {
        return Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(2_000)
                .collect(),
        ));
    }
    if output.stdout.is_empty() {
        output = git(
            root,
            &["diff", "--cached", "--no-color", "--unified=3", "--", rel],
        )?;
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(temp.path())
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        std::fs::write(temp.path().join("a.txt"), "one\n").unwrap();
        run(&["add", "a.txt"]);
        run(&["commit", "-qm", "init"]);
        std::fs::write(temp.path().join("a.txt"), "one\ntwo\n").unwrap();
        std::fs::write(temp.path().join("new.txt"), "hello\n").unwrap();
        temp
    }

    #[test]
    fn reports_modified_and_untracked_files() {
        let temp = repo();
        let entries = status(temp.path(), "").unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.path == "a.txt" && entry.label == "modified")
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.path == "new.txt" && entry.code == "??")
        );
    }

    #[test]
    fn produces_tracked_and_untracked_diffs() {
        let temp = repo();
        let tracked = diff(temp.path(), "a.txt").unwrap();
        assert!(tracked.contains("+two"));
        let untracked = diff(temp.path(), "new.txt").unwrap();
        assert!(untracked.contains("new file mode"));
        assert!(untracked.contains("+hello"));
    }

    #[test]
    fn rejects_paths_outside_the_workspace() {
        let temp = repo();
        assert!(diff(temp.path(), "../etc/passwd").is_err());
    }
}
