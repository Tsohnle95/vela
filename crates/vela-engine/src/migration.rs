//! Read-only import of Vela-owned data from the original Electron profile.
//!
//! Nothing in the legacy directory is written, moved, or deleted. The import is
//! idempotent: bookmarks are merged by directory, so repeated launches and
//! partial migrations converge without duplicating entries.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::persist::Settings;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBookmark {
    pub directory: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceLocation {
    pub directory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inode: Option<u64>,
    #[serde(default, rename = "movedTo", skip_serializing_if = "Option::is_none")]
    pub moved_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceResolution {
    pub directory: PathBuf,
    pub previous_root: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct MigrationReport {
    pub source: Option<PathBuf>,
    pub imported: usize,
    pub duplicates: usize,
    pub relocated: usize,
    pub ambiguous: usize,
    pub skipped: usize,
    pub window: Option<(f32, f32)>,
    pub error: Option<String>,
    /// Pins copied from Source's renderer localStorage (new ids only).
    pub pins_imported: usize,
    /// The one-time marker was set, so main should persist settings even when
    /// there were no workspaces or pins to add.
    pub pins_scanned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceLookup {
    Resolved(WorkspaceResolution),
    Ambiguous {
        previous_root: PathBuf,
        candidates: Vec<PathBuf>,
    },
    Missing,
}

impl WorkspaceLookup {
    pub fn resolved(self) -> Option<WorkspaceResolution> {
        match self {
            Self::Resolved(resolution) => Some(resolution),
            Self::Ambiguous { .. } | Self::Missing => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelocateOutcome {
    Resolved {
        directory: PathBuf,
        changed: bool,
    },
    Ambiguous {
        previous_root: PathBuf,
        candidates: Vec<PathBuf>,
    },
    Missing,
}

/// The Electron profile name has remained `Vela` across releases.
pub fn legacy_profile_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Vela")
        })
    }
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir().map(|base| base.join("Vela"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        dirs::config_dir().map(|base| base.join("Vela"))
    }
}

pub fn import_legacy(settings: &mut Settings) -> MigrationReport {
    if std::env::var("VELA_SKIP_LEGACY_IMPORT").as_deref() == Ok("1") {
        return MigrationReport::default();
    }
    if let Some(directory) = std::env::var_os("VELA_LEGACY_PROFILE_DIR") {
        return import_from(&PathBuf::from(directory), settings);
    }
    let Some(directory) = legacy_profile_dir() else {
        return MigrationReport::default();
    };
    import_from(&directory, settings)
}

/// Copy the original per-root Sass preference only before native preferences
/// exist. QA uses the same legacy-import opt-out as bookmark migration.
pub fn import_sass_preferences(destination: &Path) -> Result<bool, String> {
    if std::env::var("VELA_SKIP_LEGACY_IMPORT").as_deref() == Ok("1") {
        return Ok(false);
    }
    let source = std::env::var_os("VELA_LEGACY_PROFILE_DIR")
        .map(PathBuf::from)
        .or_else(legacy_profile_dir);
    match source {
        Some(source) => crate::sass_preferences::import_from(&source, destination),
        None => Ok(false),
    }
}

/// Import from an explicit legacy profile directory. Exposed for tests and for
/// users whose Electron profile lives at a nonstandard path.
pub fn import_from(directory: &Path, settings: &mut Settings) -> MigrationReport {
    let mut report = MigrationReport {
        source: Some(directory.to_path_buf()),
        ..Default::default()
    };
    let mut imported_locations = Vec::new();
    match read_legacy(&directory.join("saved-workspaces.json")) {
        Ok((workspaces, locations)) => {
            for mut workspace in workspaces {
                let original = PathBuf::from(&workspace.directory);
                match resolve_workspace_lookup(&original, &locations) {
                    WorkspaceLookup::Resolved(resolved) => {
                        let resolved = resolved.directory.to_string_lossy().into_owned();
                        if resolved != workspace.directory {
                            workspace.directory = resolved;
                            report.relocated += 1;
                        }
                    }
                    WorkspaceLookup::Ambiguous { .. } => report.ambiguous += 1,
                    WorkspaceLookup::Missing => {}
                }
                if settings
                    .workspaces
                    .iter()
                    .any(|existing| existing.directory == workspace.directory)
                {
                    report.duplicates += 1;
                } else {
                    settings.workspaces.push(workspace);
                    report.imported += 1;
                }
            }
            imported_locations = locations;
        }
        Err(error) => report.error = Some(error),
    }
    for location in imported_locations {
        merge_location(&mut settings.workspace_locations, location);
    }
    for workspace in &settings.workspaces {
        if let Some(location) = workspace_location(Path::new(&workspace.directory)) {
            merge_location(&mut settings.workspace_locations, location);
        }
    }
    // Source pins live only in Chromium's localStorage. Import them once,
    // after which native pin state is authoritative; rescanning would
    // resurrect sessions the user unpinned natively.
    if !settings.legacy_pins_imported
        && let Some(pins) = crate::legacy_pins::read(directory)
    {
        report.pins_scanned = true;
        for pin in pins {
            if settings.pinned_sessions.insert(pin) {
                report.pins_imported += 1;
            }
        }
        settings.legacy_pins_imported = true;
    }
    report.window = read_window(&directory.join("window-bounds.json"));
    report
}

fn read_legacy(path: &Path) -> Result<(Vec<WorkspaceBookmark>, Vec<WorkspaceLocation>), String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), Vec::new()));
        }
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let mut locations = Vec::new();
    let workspaces = match &value {
        Value::Array(items) => items,
        Value::Object(object) => {
            let version = object.get("version").and_then(Value::as_u64).unwrap_or(1);
            if version > 2 {
                return Err(format!("unsupported saved-workspace version {version}"));
            }
            if version == 2 {
                let entries = object
                    .get("locations")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        "saved workspace locations have an invalid format".to_string()
                    })?;
                for entry in entries {
                    let directory = entry
                        .get("directory")
                        .and_then(Value::as_str)
                        .filter(|directory| Path::new(directory).is_absolute())
                        .ok_or_else(|| "invalid saved workspace location".to_string())?;
                    let moved_to = entry
                        .get("movedTo")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    if moved_to
                        .as_deref()
                        .is_some_and(|target| !Path::new(target).is_absolute())
                    {
                        return Err("invalid workspace relocation".into());
                    }
                    locations.push(WorkspaceLocation {
                        directory: directory.to_string(),
                        device: entry.get("device").and_then(Value::as_u64),
                        inode: entry.get("inode").and_then(Value::as_u64),
                        moved_to,
                        project_id: None,
                    });
                }
            }
            object
                .get("workspaces")
                .and_then(Value::as_array)
                .ok_or_else(|| "saved-workspaces.json has no workspaces array".to_string())?
        }
        _ => return Err("saved-workspaces.json has an unsupported shape".into()),
    };
    let mut out = Vec::new();
    for item in workspaces {
        let Some(directory) = item.get("directory").and_then(Value::as_str) else {
            continue;
        };
        if directory.trim().is_empty() || !Path::new(directory).is_absolute() {
            continue;
        }
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                Path::new(directory)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| directory.to_string());
        if out
            .iter()
            .any(|entry: &WorkspaceBookmark| entry.directory == directory)
        {
            continue;
        }
        out.push(WorkspaceBookmark {
            directory: directory.to_string(),
            name,
        });
    }
    Ok((out, locations))
}

fn within(root: &Path, directory: &Path) -> bool {
    directory.strip_prefix(root).is_ok()
}

pub fn resolve_workspace(
    directory: &Path,
    locations: &[WorkspaceLocation],
) -> Option<WorkspaceResolution> {
    resolve_workspace_lookup(directory, locations).resolved()
}

pub fn resolve_workspace_lookup(
    directory: &Path,
    locations: &[WorkspaceLocation],
) -> WorkspaceLookup {
    if let Ok(real) = fs::canonicalize(directory)
        && real.is_dir()
    {
        return WorkspaceLookup::Resolved(WorkspaceResolution {
            directory: real,
            previous_root: directory.to_path_buf(),
        });
    }
    let mut aliases = locations
        .iter()
        .filter(|entry| entry.moved_to.is_some())
        .filter(|entry| within(Path::new(&entry.directory), directory))
        .collect::<Vec<_>>();
    aliases.sort_by_key(|entry| std::cmp::Reverse(entry.directory.len()));
    for alias in aliases {
        let Some(moved_to) = alias.moved_to.as_ref() else {
            continue;
        };
        let mut target = PathBuf::from(moved_to);
        let mut visited = std::collections::HashSet::new();
        while visited.insert(target.clone()) {
            let Some(next) = locations
                .iter()
                .find(|entry| entry.moved_to.is_some() && Path::new(&entry.directory) == target)
            else {
                break;
            };
            let Some(next_target) = next.moved_to.as_ref() else {
                break;
            };
            target = PathBuf::from(next_target);
        }
        let Ok(relative) = directory.strip_prefix(&alias.directory) else {
            continue;
        };
        if let Ok(real) = fs::canonicalize(target.join(relative))
            && real.is_dir()
        {
            return WorkspaceLookup::Resolved(WorkspaceResolution {
                directory: real,
                previous_root: PathBuf::from(&alias.directory),
            });
        }
    }
    let Some(recorded) = locations.iter().find(|entry| {
        (entry.inode.is_some() || entry.project_id.is_some())
            && Path::new(&entry.directory) == directory
    }) else {
        return WorkspaceLookup::Missing;
    };
    let previous_root = Path::new(&recorded.directory).to_path_buf();
    match find_by_identity(directory, recorded) {
        IdentityLookup::Found(directory) => WorkspaceLookup::Resolved(WorkspaceResolution {
            directory,
            previous_root,
        }),
        IdentityLookup::Ambiguous(candidates) => WorkspaceLookup::Ambiguous {
            previous_root,
            candidates,
        },
        IdentityLookup::None => WorkspaceLookup::Missing,
    }
}

pub fn workspace_location(directory: &Path) -> Option<WorkspaceLocation> {
    let metadata = fs::metadata(directory).ok()?;
    if !metadata.is_dir() {
        return None;
    }
    let directory = fs::canonicalize(directory).ok()?;
    let project_id = fs::read_to_string(directory.join(".git").join("opencode"))
        .ok()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(&directory).ok()?;
        Some(WorkspaceLocation {
            directory: directory.to_string_lossy().into_owned(),
            device: Some(metadata.dev()),
            inode: Some(metadata.ino()),
            moved_to: None,
            project_id,
        })
    }
    #[cfg(not(unix))]
    {
        Some(WorkspaceLocation {
            directory: directory.to_string_lossy().into_owned(),
            device: None,
            inode: None,
            moved_to: None,
            project_id,
        })
    }
}

pub fn merge_location(locations: &mut Vec<WorkspaceLocation>, location: WorkspaceLocation) {
    if let Some(existing) = locations
        .iter_mut()
        .find(|entry| entry.directory == location.directory)
    {
        *existing = location;
    } else {
        locations.push(location);
    }
    if locations.len() > 512 {
        locations.drain(..locations.len() - 512);
    }
}

/// Resolve a bookmark at click time, repair every bookmark under the moved root
/// and record the alias/current location for future launches.
pub fn resolve_and_relocate(
    settings: &mut crate::persist::Settings,
    requested: &Path,
) -> RelocateOutcome {
    let resolution = match resolve_workspace_lookup(requested, &settings.workspace_locations) {
        WorkspaceLookup::Resolved(resolution) => resolution,
        WorkspaceLookup::Ambiguous {
            previous_root,
            candidates,
        } => {
            return RelocateOutcome::Ambiguous {
                previous_root,
                candidates,
            };
        }
        WorkspaceLookup::Missing => return RelocateOutcome::Missing,
    };
    let mut changed = false;
    if resolution.previous_root != resolution.directory {
        for bookmark in &mut settings.workspaces {
            let Ok(relative) =
                Path::new(&bookmark.directory).strip_prefix(&resolution.previous_root)
            else {
                continue;
            };
            let candidate = resolution.directory.join(relative);
            let Ok(candidate) = fs::canonicalize(candidate) else {
                continue;
            };
            if !candidate.is_dir() {
                continue;
            }
            let candidate = candidate.to_string_lossy().into_owned();
            if bookmark.directory != candidate {
                bookmark.directory = candidate;
                changed = true;
            }
        }
        let before = settings.workspace_locations.clone();
        merge_location(
            &mut settings.workspace_locations,
            WorkspaceLocation {
                directory: resolution.previous_root.to_string_lossy().into_owned(),
                device: None,
                inode: None,
                moved_to: Some(resolution.directory.to_string_lossy().into_owned()),
                project_id: None,
            },
        );
        changed |= before != settings.workspace_locations;
    }
    if let Some(location) = workspace_location(&resolution.directory) {
        let before = settings.workspace_locations.clone();
        merge_location(&mut settings.workspace_locations, location);
        changed |= before != settings.workspace_locations;
    }
    let mut seen = std::collections::HashSet::new();
    let before = settings.workspaces.len();
    settings
        .workspaces
        .retain(|bookmark| seen.insert(bookmark.directory.clone()));
    changed |= settings.workspaces.len() != before;
    RelocateOutcome::Resolved {
        directory: resolution.directory,
        changed,
    }
}

pub fn ambiguous_relocation_message(previous_root: &Path, candidates: &[PathBuf]) -> String {
    let mut names = candidates
        .iter()
        .take(3)
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    if candidates.len() > 3 {
        names.push(format!("and {} more", candidates.len() - 3));
    }
    format!(
        "Workspace {} matches more than one moved location: {}. Update the bookmark manually.",
        previous_root.display(),
        names.join(", ")
    )
}

enum IdentityLookup {
    Found(PathBuf),
    Ambiguous(Vec<PathBuf>),
    None,
}

fn find_by_identity(directory: &Path, recorded: &WorkspaceLocation) -> IdentityLookup {
    let Some(mut parent) = directory.parent() else {
        return IdentityLookup::None;
    };
    let mut nearby = Vec::new();
    for _ in 0..4 {
        if let Ok(entries) = fs::read_dir(parent) {
            let entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
            if entries.len() <= 500 {
                nearby.extend(
                    entries
                        .into_iter()
                        .map(|entry| entry.path())
                        .filter(|path| fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())),
                );
            }
        }
        let Some(next) = parent.parent() else {
            break;
        };
        if next == parent {
            break;
        }
        parent = next;
    }
    let mut ambiguous = Vec::new();
    #[cfg(unix)]
    if let (Some(device), Some(inode)) = (recorded.device, recorded.inode) {
        use std::os::unix::fs::MetadataExt;
        let matches = nearby
            .iter()
            .filter(|path| {
                fs::metadata(path).is_ok_and(|metadata| {
                    metadata.is_dir() && metadata.dev() == device && metadata.ino() == inode
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return fs::canonicalize(&matches[0])
                .map_or(IdentityLookup::None, IdentityLookup::Found);
        }
        ambiguous.extend(matches);
    }
    if let Some(project_id) = recorded.project_id.as_deref() {
        let Some(name) = directory.file_name() else {
            return IdentityLookup::Ambiguous(ambiguous);
        };
        let matches = nearby
            .iter()
            .filter(|path| path.file_name() == Some(name))
            .filter(|path| {
                fs::read_to_string(path.join(".git").join("opencode"))
                    .ok()
                    .map(|id| id.trim().to_string())
                    .as_deref()
                    == Some(project_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return fs::canonicalize(&matches[0])
                .map_or(IdentityLookup::None, IdentityLookup::Found);
        }
        ambiguous.extend(matches);
    }
    ambiguous.sort();
    ambiguous.dedup();
    if ambiguous.is_empty() {
        IdentityLookup::None
    } else {
        IdentityLookup::Ambiguous(ambiguous)
    }
}

fn read_window(path: &Path) -> Option<(f32, f32)> {
    let text = fs::read_to_string(path).ok()?;
    let size = crate::window_state::WindowSize::parse(&text)?;
    Some((size.width as f32, size.height as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved_outcome(outcome: RelocateOutcome) -> (PathBuf, bool) {
        match outcome {
            RelocateOutcome::Resolved { directory, changed } => (directory, changed),
            other => panic!("expected resolved relocation, got {other:?}"),
        }
    }

    fn renderer_pin_log(pins: &[&str]) -> Vec<u8> {
        let key = b"_file://\x00\x01vela.pinnedSessions";
        let mut value = vec![1];
        value.extend_from_slice(serde_json::to_string(pins).unwrap().as_bytes());
        let mut batch = Vec::new();
        batch.extend_from_slice(&1u64.to_le_bytes());
        batch.extend_from_slice(&1u32.to_le_bytes());
        batch.push(1);
        batch.push(key.len() as u8);
        batch.extend_from_slice(key);
        assert!(value.len() < 128);
        batch.push(value.len() as u8);
        batch.extend_from_slice(&value);
        let mut log = vec![0, 0, 0, 0];
        log.extend_from_slice(&(batch.len() as u16).to_le_bytes());
        log.push(1);
        log.extend_from_slice(&batch);
        log
    }

    #[test]
    fn imports_source_pins_once_and_never_resurrects_native_unpins() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path();
        let database = legacy.join("Local Storage").join("leveldb");
        fs::create_dir_all(&database).unwrap();
        let log = renderer_pin_log(&["ses_parent", "ses_child"]);
        fs::write(database.join("000003.log"), &log).unwrap();

        let mut settings = Settings::default();
        let report = import_from(legacy, &mut settings);
        assert_eq!(report.pins_imported, 2);
        assert!(report.pins_scanned);
        assert!(settings.legacy_pins_imported);
        assert!(settings.pinned_sessions.contains("ses_parent"));
        assert!(settings.pinned_sessions.contains("ses_child"));

        // Unpin in native, then relaunch: Source's stale value is not replayed.
        settings.pinned_sessions.remove("ses_child");
        let second = import_from(legacy, &mut settings);
        assert_eq!(second.pins_imported, 0);
        assert!(!second.pins_scanned);
        assert!(settings.pinned_sessions.contains("ses_parent"));
        assert!(!settings.pinned_sessions.contains("ses_child"));
        assert_eq!(fs::read(database.join("000003.log")).unwrap(), log);
    }

    #[test]
    fn a_profile_without_local_storage_keeps_the_pin_scan_pending() {
        let temp = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        let report = import_from(temp.path(), &mut settings);
        assert_eq!(report.pins_imported, 0);
        assert!(!report.pins_scanned);
        assert!(!settings.legacy_pins_imported);
    }

    #[test]
    fn imports_v2_bookmarks_idempotently_and_leaves_the_source_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path();
        let source = b"{\n  \"version\": 2,\n  \"workspaces\": [\n    {\"directory\": \"/tmp/one\", \"name\": \"One\"},\n    {\"directory\": \"/tmp/two\", \"name\": \"Two\"},\n    {\"directory\": \"relative\", \"name\": \"Bad\"}\n  ],\n  \"locations\": []\n}";
        fs::write(legacy.join("saved-workspaces.json"), source).unwrap();
        fs::write(
            legacy.join("window-bounds.json"),
            br#"{"version":2,"session":{"width":1426,"height":890}}"#,
        )
        .unwrap();

        let mut settings = Settings::default();
        let report = import_from(legacy, &mut settings);
        assert_eq!(report.imported, 2);
        assert_eq!(settings.workspaces.len(), 2);
        assert_eq!(report.window, Some((1426.0, 890.0)));
        assert_eq!(
            fs::read(legacy.join("saved-workspaces.json")).unwrap(),
            source
        );

        let second = import_from(legacy, &mut settings);
        assert_eq!(second.imported, 0);
        assert_eq!(second.duplicates, 2);
        assert_eq!(settings.workspaces.len(), 2);
    }

    #[test]
    fn tolerates_missing_and_corrupt_legacy_files() {
        let temp = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        let missing = import_from(temp.path(), &mut settings);
        assert_eq!(missing.imported, 0);
        assert!(missing.error.is_none());

        fs::write(temp.path().join("saved-workspaces.json"), b"{broken").unwrap();
        let corrupt = import_from(temp.path(), &mut settings);
        assert!(corrupt.error.is_some());
        assert!(settings.workspaces.is_empty());
    }

    #[test]
    fn refuses_a_future_schema_instead_of_guessing() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("saved-workspaces.json"),
            br#"{"version":99,"workspaces":[{"directory":"/tmp/x","name":"X"}]}"#,
        )
        .unwrap();
        let mut settings = Settings::default();
        let report = import_from(temp.path(), &mut settings);
        assert!(report.error.is_some());
        assert!(settings.workspaces.is_empty());
    }

    #[test]
    fn accepts_the_legacy_array_shape() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("saved-workspaces.json"),
            br#"[{"directory":"/tmp/array","name":"Array"}]"#,
        )
        .unwrap();
        let mut settings = Settings::default();
        let report = import_from(temp.path(), &mut settings);
        assert_eq!(report.imported, 1);
        assert_eq!(settings.workspaces[0].name, "Array");
    }

    #[test]
    fn relocates_aliases_and_nested_bookmarks_without_writing_the_source() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        fs::create_dir(&legacy).unwrap();
        let moved = temp.path().join("moved");
        fs::create_dir_all(moved.join("child")).unwrap();
        let old = temp.path().join("old");
        let data = serde_json::json!({
            "version": 2,
            "workspaces": [
                {"directory": old, "name": "Old"},
                {"directory": old.join("child"), "name": "Child"}
            ],
            "locations": [
                {"directory": old, "movedTo": moved}
            ]
        });
        let serialized = serde_json::to_string_pretty(&data).unwrap();
        fs::write(legacy.join("saved-workspaces.json"), &serialized).unwrap();

        let mut settings = Settings::default();
        let report = import_from(&legacy, &mut settings);
        assert_eq!(report.imported, 2);
        assert_eq!(report.relocated, 2);
        assert_eq!(
            settings.workspaces[0].directory,
            fs::canonicalize(&moved).unwrap().to_string_lossy()
        );
        assert_eq!(
            settings.workspaces[1].directory,
            fs::canonicalize(moved.join("child"))
                .unwrap()
                .to_string_lossy()
        );
        assert_eq!(
            fs::read_to_string(legacy.join("saved-workspaces.json")).unwrap(),
            serialized
        );
    }

    #[test]
    fn follows_multi_step_relocation_chains() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        fs::create_dir(&legacy).unwrap();
        let final_dir = temp.path().join("final");
        fs::create_dir(&final_dir).unwrap();
        let old = temp.path().join("old");
        let middle = temp.path().join("middle");
        let data = serde_json::json!({
            "version": 2,
            "workspaces": [{"directory": old, "name": "Moved"}],
            "locations": [
                {"directory": old, "movedTo": middle},
                {"directory": middle, "movedTo": final_dir}
            ]
        });
        fs::write(
            legacy.join("saved-workspaces.json"),
            serde_json::to_string(&data).unwrap(),
        )
        .unwrap();
        let mut settings = Settings::default();
        let report = import_from(&legacy, &mut settings);
        assert_eq!(report.relocated, 1);
        assert_eq!(
            settings.workspaces[0].directory,
            fs::canonicalize(final_dir).unwrap().to_string_lossy()
        );
    }

    #[test]
    fn relocates_by_opencode_project_id_when_the_inode_changed() {
        let temp = tempfile::tempdir().unwrap();
        let old_parent = temp.path().join("a");
        let old = old_parent.join("root");
        fs::create_dir_all(old.join(".git")).unwrap();
        fs::write(old.join(".git").join("opencode"), "proj_123\n").unwrap();
        let old = fs::canonicalize(&old).unwrap();
        let location = workspace_location(&old).unwrap();
        assert_eq!(location.project_id.as_deref(), Some("proj_123"));
        fs::remove_dir_all(&old).unwrap();
        let moved = temp.path().join("root");
        fs::create_dir_all(moved.join(".git")).unwrap();
        fs::write(moved.join(".git").join("opencode"), "proj_123\n").unwrap();

        let mut settings = Settings {
            workspaces: vec![WorkspaceBookmark {
                directory: old.to_string_lossy().into_owned(),
                name: "Moved".into(),
            }],
            workspace_locations: vec![location],
            ..Default::default()
        };
        let (resolved, changed) = resolved_outcome(resolve_and_relocate(&mut settings, &old));
        assert!(changed);
        assert_eq!(resolved, fs::canonicalize(moved).unwrap());
        assert_eq!(settings.workspaces[0].directory, resolved.to_string_lossy());
    }

    #[cfg(unix)]
    #[test]
    fn relocates_by_device_and_inode_when_no_alias_exists() {
        use std::os::unix::fs::MetadataExt;

        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        fs::create_dir(&legacy).unwrap();
        let old = temp.path().join("old");
        fs::create_dir(&old).unwrap();
        let metadata = fs::metadata(&old).unwrap();
        let new = temp.path().join("new");
        fs::rename(&old, &new).unwrap();
        let data = serde_json::json!({
            "version": 2,
            "workspaces": [{"directory": old, "name": "Moved"}],
            "locations": [{
                "directory": old,
                "device": metadata.dev(),
                "inode": metadata.ino()
            }]
        });
        fs::write(
            legacy.join("saved-workspaces.json"),
            serde_json::to_string(&data).unwrap(),
        )
        .unwrap();

        let mut settings = Settings::default();
        let report = import_from(&legacy, &mut settings);
        assert_eq!(report.relocated, 1);
        assert_eq!(
            settings.workspaces[0].directory,
            fs::canonicalize(new).unwrap().to_string_lossy()
        );
    }

    #[cfg(unix)]
    #[test]
    fn ambiguous_project_matches_report_candidates_without_relocating() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("shared");
        fs::create_dir_all(target.join(".git")).unwrap();
        fs::write(target.join(".git").join("opencode"), "proj_same\n").unwrap();
        let first = temp.path().join("root");
        symlink(&target, &first).unwrap();
        let a = temp.path().join("a");
        fs::create_dir(&a).unwrap();
        let second = a.join("root");
        symlink(&target, &second).unwrap();
        let old = a.join("missing").join("root");
        let mut settings = Settings {
            workspaces: vec![WorkspaceBookmark {
                directory: old.to_string_lossy().into_owned(),
                name: "Ambiguous".into(),
            }],
            workspace_locations: vec![WorkspaceLocation {
                directory: old.to_string_lossy().into_owned(),
                device: None,
                inode: None,
                moved_to: None,
                project_id: Some("proj_same".into()),
            }],
            ..Default::default()
        };
        let outcome = resolve_and_relocate(&mut settings, &old);
        let RelocateOutcome::Ambiguous {
            previous_root,
            candidates,
        } = outcome
        else {
            panic!("expected ambiguous relocation");
        };
        assert_eq!(previous_root, old);
        let mut expected = vec![first.clone(), second.clone()];
        expected.sort();
        assert_eq!(candidates, expected);
        assert_eq!(settings.workspaces[0].directory, old.to_string_lossy());
        let message = ambiguous_relocation_message(&previous_root, &candidates);
        assert!(message.contains(&first.display().to_string()));
        assert!(message.contains(&second.display().to_string()));
        assert!(message.contains("more than one"));
    }

    #[test]
    fn runtime_relocation_repairs_bookmarks_and_records_alias_idempotently() {
        let temp = tempfile::tempdir().unwrap();
        let moved = temp.path().join("moved");
        fs::create_dir_all(moved.join("child")).unwrap();
        let old = temp.path().join("old");
        let mut settings = Settings {
            workspaces: vec![
                WorkspaceBookmark {
                    directory: old.to_string_lossy().into_owned(),
                    name: "Root".into(),
                },
                WorkspaceBookmark {
                    directory: old.join("child").to_string_lossy().into_owned(),
                    name: "Child".into(),
                },
            ],
            workspace_locations: vec![WorkspaceLocation {
                directory: old.to_string_lossy().into_owned(),
                device: None,
                inode: None,
                moved_to: Some(moved.to_string_lossy().into_owned()),
                project_id: None,
            }],
            ..Default::default()
        };

        let (resolved, changed) = resolved_outcome(resolve_and_relocate(&mut settings, &old));
        assert!(changed);
        assert_eq!(resolved, fs::canonicalize(&moved).unwrap());
        let resolved_string = resolved.to_string_lossy().into_owned();
        assert_eq!(settings.workspaces[0].directory, resolved_string);
        assert_eq!(
            settings.workspaces[1].directory,
            fs::canonicalize(moved.join("child"))
                .unwrap()
                .to_string_lossy()
        );
        assert!(settings.workspace_locations.iter().any(|location| {
            Path::new(&location.directory) == old
                && location.moved_to.as_deref() == Some(resolved_string.as_str())
        }));
        assert!(
            settings
                .workspace_locations
                .iter()
                .any(|location| { Path::new(&location.directory) == resolved })
        );

        let (_, changed_again) = resolved_outcome(resolve_and_relocate(&mut settings, &old));
        assert!(!changed_again);
    }
}
