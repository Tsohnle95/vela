//! Persistent browser download history. Entries are newest-first and capped so
//! the tray stays bounded; the source profile is never involved.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_DOWNLOADS: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserDownload {
    pub id: u64,
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub success: bool,
    pub at: i64,
}

fn history_path(directory: &Path) -> PathBuf {
    directory.join("browser-downloads.json")
}

pub fn load(directory: &Path) -> Vec<BrowserDownload> {
    let Ok(text) = fs::read_to_string(history_path(directory)) else {
        return Vec::new();
    };
    let Ok(mut entries) = serde_json::from_str::<Vec<BrowserDownload>>(&text) else {
        return Vec::new();
    };
    entries.retain(|entry| !entry.name.trim().is_empty());
    entries.truncate(MAX_DOWNLOADS);
    entries
}

pub fn save(directory: &Path, entries: &[BrowserDownload]) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let path = history_path(directory);
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(entries).map_err(|error| error.to_string())?;
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    fs::rename(&temporary, &path).map_err(|error| error.to_string())
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

pub fn download_name(url: &str, path: Option<&Path>) -> String {
    path.and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| vela_browser::download_name(url))
}

/// Platform command used to reveal a completed download. Only absolute paths
/// are eligible; the returned arguments are testable without launching a
/// process.
pub fn reveal_command(path: &str) -> Option<(&'static str, Vec<String>)> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return None;
    }
    #[cfg(target_os = "macos")]
    {
        Some((
            "open",
            vec!["-R".into(), path.to_string_lossy().into_owned()],
        ))
    }
    #[cfg(target_os = "windows")]
    {
        Some((
            "explorer",
            vec![format!("/select,{}", path.to_string_lossy())],
        ))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path.parent()?;
        Some((
            "xdg-open",
            vec![parent.to_string_lossy().into_owned()],
        ))
    }
}

pub fn reveal(path: &str) -> bool {
    let Some((program, arguments)) = reveal_command(path) else {
        return false;
    };
    std::process::Command::new(program)
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

pub fn remove(entries: &mut Vec<BrowserDownload>, id: u64) -> bool {
    let before = entries.len();
    entries.retain(|entry| entry.id != id);
    entries.len() != before
}

pub fn push(
    entries: &mut Vec<BrowserDownload>,
    name: String,
    url: String,
    path: Option<&Path>,
    success: bool,
    at: i64,
) {
    let id = entries
        .iter()
        .map(|entry| entry.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    entries.insert(
        0,
        BrowserDownload {
            id,
            name,
            url,
            path: path.map(|path| path.to_string_lossy().into_owned()),
            success,
            at,
        },
    );
    entries.truncate(MAX_DOWNLOADS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_round_trips_newest_first_caps_and_ignores_corruption() {
        let temp = tempfile::tempdir().unwrap();
        assert!(load(temp.path()).is_empty());
        let mut entries = Vec::new();
        for index in 0..25 {
            push(
                &mut entries,
                format!("report-{index}.pdf"),
                format!("https://example.com/report-{index}.pdf"),
                None,
                index % 4 != 0,
                index as i64,
            );
        }
        assert_eq!(entries.len(), MAX_DOWNLOADS);
        assert_eq!(entries[0].name, "report-24.pdf");
        assert_eq!(entries[0].id, 25);
        assert!(!entries[0].success);
        save(temp.path(), &entries).unwrap();
        let loaded = load(temp.path());
        assert_eq!(loaded, entries);
        assert_eq!(loaded[0].name, "report-24.pdf");

        fs::write(history_path(temp.path()), b"{broken").unwrap();
        assert!(load(temp.path()).is_empty());
        save(temp.path(), &[]).unwrap();
        assert!(load(temp.path()).is_empty());
    }

    #[test]
    fn reveal_commands_require_absolute_paths_and_use_platform_arguments() {
        assert!(reveal_command("relative/report.pdf").is_none());
        let command = reveal_command("/Users/qa/Downloads/report.pdf").unwrap();
        #[cfg(target_os = "macos")]
        {
            assert_eq!(command.0, "open");
            assert_eq!(command.1, vec!["-R", "/Users/qa/Downloads/report.pdf"]);
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.0, "explorer");
            assert_eq!(command.1[0], "/select,\\Users\\qa\\Downloads\\report.pdf");
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            assert_eq!(command.0, "xdg-open");
            assert_eq!(command.1, vec!["/Users/qa/Downloads"]);
        }
    }

    #[test]
    fn individual_downloads_remove_by_id_and_unknown_ids_are_noops() {
        let mut entries = Vec::new();
        push(
            &mut entries,
            "one".into(),
            "https://example.com/one".into(),
            None,
            true,
            1,
        );
        push(
            &mut entries,
            "two".into(),
            "https://example.com/two".into(),
            None,
            false,
            2,
        );
        assert!(remove(&mut entries, 1));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, 2);
        assert!(!remove(&mut entries, 99));
        assert_eq!(entries.len(), 1);
        assert!(remove(&mut entries, 2));
        assert!(entries.is_empty());
    }

    #[test]
    fn names_prefer_saved_paths_and_fall_back_to_url_names() {
        assert_eq!(
            download_name(
                "https://example.com/other.bin",
                Some(Path::new("/Users/qa/Downloads/Report (1).pdf"))
            ),
            "Report (1).pdf"
        );
        assert_eq!(
            download_name(
                "https://example.com/files/%E4%B8%96%E7%95%8C.txt",
                None
            ),
            "世界.txt"
        );
        assert_eq!(
            download_name("https://example.com/files/", None),
            "download"
        );
    }
}
