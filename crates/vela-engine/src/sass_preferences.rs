//! Durable per-canonical-root settings, with a last valid backup.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
struct Saved {
    version: u32,
    enabled: BTreeSet<String>,
}
pub(crate) struct Preferences {
    path: Option<PathBuf>,
    enabled: Result<BTreeSet<String>, String>,
    primary_valid: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            path: None,
            enabled: Ok(BTreeSet::new()),
            primary_valid: false,
        }
    }
}
fn read(path: &Path) -> Result<BTreeSet<String>, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let saved: Saved = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if saved.version != 1
        || saved
            .enabled
            .iter()
            .any(|root| !Path::new(root).is_absolute())
    {
        return Err("Invalid Sass watcher preferences".into());
    }
    Ok(saved.enabled)
}
fn backup(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}
impl Preferences {
    pub fn load(directory: Option<&Path>) -> Self {
        let Some(directory) = directory else {
            return Self::default();
        };
        let path = directory.join("sass-watch.json");
        let (enabled, primary_valid) = match read(&path) {
            Ok(enabled) => (Ok(enabled), true),
            Err(_) => match read(&backup(&path)) {
                Ok(enabled) => (Ok(enabled), false),
                Err(_) if !path.exists() && !backup(&path).exists() => (Ok(BTreeSet::new()), false),
                Err(_) => (
                    Err("Could not read Sass watcher preferences or their backup".into()),
                    false,
                ),
            },
        };
        Self {
            path: Some(path),
            enabled,
            primary_valid,
        }
    }
    pub fn enabled(&self, root: &str) -> Result<bool, String> {
        self.enabled
            .as_ref()
            .map(|enabled| enabled.contains(root))
            .map_err(Clone::clone)
    }
    pub fn set(&mut self, root: &str, enabled: bool) -> Result<(), String> {
        let mut next = self.enabled.clone()?;
        if enabled {
            next.insert(root.into());
        } else {
            next.remove(root);
        }
        self.commit(next)
    }
    fn commit(&mut self, next: BTreeSet<String>) -> Result<(), String> {
        if let Some(path) = &self.path {
            let directory = path.parent().expect("preferences parent");
            fs::create_dir_all(directory).map_err(|error| error.to_string())?;
            let temporary = directory.join(format!(".sass-watch-{}.tmp", uuid::Uuid::new_v4()));
            let result = (|| -> Result<(), String> {
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)
                    .map_err(|error| error.to_string())?;
                let bytes = serde_json::to_vec(&Saved {
                    version: 1,
                    enabled: next.clone(),
                })
                .map_err(|error| error.to_string())?;
                file.write_all(&bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|error| error.to_string())?;
                // Never replace the valid backup with a corrupt primary from
                // which this instance already had to recover.
                if self.primary_valid {
                    fs::copy(path, backup(path)).map_err(|error| error.to_string())?;
                }
                fs::rename(&temporary, path).map_err(|error| error.to_string())?;
                Ok(())
            })();
            let _ = fs::remove_file(&temporary);
            result?;
            self.primary_valid = true;
        }
        self.enabled = Ok(next);
        Ok(())
    }
}

pub(crate) fn import_from(source: &Path, destination: &Path) -> Result<bool, String> {
    let target = destination.join("sass-watch.json");
    if target.exists() || backup(&target).exists() {
        return Ok(false);
    }
    let source = source.join("sass-watch.json");
    if !source.exists() && !backup(&source).exists() {
        return Ok(false);
    }
    let enabled = read(&source).or_else(|_| read(&backup(&source)))?;
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let temporary = destination.join(format!(".sass-import-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<bool, String> {
        {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| error.to_string())?;
            let bytes = serde_json::to_vec(&Saved {
                version: 1,
                enabled,
            })
            .map_err(|error| error.to_string())?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|error| error.to_string())?;
        }
        // Claim the destination atomically; another native instance may have
        // saved its own choices while the read-only legacy import was staging.
        match fs::hard_link(&temporary, &target) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(error) => Err(error.to_string()),
        }
    })();
    let _ = fs::remove_file(temporary);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_import_preserves_source_and_never_overrides_native_choices() {
        let legacy = tempfile::tempdir().unwrap();
        let native = tempfile::tempdir().unwrap();
        let root = legacy
            .path()
            .join("workspace")
            .to_string_lossy()
            .into_owned();
        let bytes = serde_json::to_vec(&Saved {
            version: 1,
            enabled: BTreeSet::from([root.clone()]),
        })
        .unwrap();
        let source = legacy.path().join("sass-watch.json");
        fs::write(&source, &bytes).unwrap();
        assert!(import_from(legacy.path(), native.path()).unwrap());
        let mut restored = Preferences::load(Some(native.path()));
        assert!(restored.enabled(&root).unwrap());
        restored.set(&root, false).unwrap();
        assert!(!import_from(legacy.path(), native.path()).unwrap());
        assert!(
            !Preferences::load(Some(native.path()))
                .enabled(&root)
                .unwrap()
        );
        assert_eq!(fs::read(source).unwrap(), bytes);
    }
    #[test]
    fn legacy_backup_recovery_and_invalid_source_leave_existing_bytes_intact() {
        let legacy = tempfile::tempdir().unwrap();
        let native = tempfile::tempdir().unwrap();
        let source = legacy.path().join("sass-watch.json");
        let bytes = br#"{"version":1,"enabled":[]}"#;
        fs::write(&source, "corrupt").unwrap();
        fs::write(backup(&source), bytes).unwrap();
        assert!(import_from(legacy.path(), native.path()).unwrap());
        assert_eq!(fs::read(&source).unwrap(), b"corrupt");
        assert_eq!(fs::read(backup(&source)).unwrap(), bytes);
        let other = tempfile::tempdir().unwrap();
        fs::write(backup(&source), "also corrupt").unwrap();
        assert!(import_from(legacy.path(), other.path()).is_err());
        assert!(!other.path().join("sass-watch.json").exists());
        fs::write(other.path().join("sass-watch.json"), "native corrupt").unwrap();
        assert!(!import_from(legacy.path(), other.path()).unwrap());
        assert_eq!(
            fs::read(other.path().join("sass-watch.json")).unwrap(),
            b"native corrupt"
        );
    }
    #[test]
    fn roots_survive_restart_and_toggles_preserve_other_roots() {
        let temp = tempfile::tempdir().unwrap();
        let mut preferences = Preferences::load(Some(temp.path()));
        preferences.set("/one", true).unwrap();
        preferences.set("/two/界", true).unwrap();
        let mut restored = Preferences::load(Some(temp.path()));
        assert!(restored.enabled("/one").unwrap());
        assert!(restored.enabled("/two/界").unwrap());
        restored.set("/one", false).unwrap();
        let restored = Preferences::load(Some(temp.path()));
        assert!(!restored.enabled("/one").unwrap());
        assert!(restored.enabled("/two/界").unwrap());
    }
    #[test]
    fn corrupt_primary_recovers_without_destroying_valid_backup() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sass-watch.json");
        let mut preferences = Preferences::load(Some(temp.path()));
        preferences.set("/one", true).unwrap();
        preferences.set("/two", true).unwrap();
        let saved_backup = fs::read(backup(&path)).unwrap();
        fs::write(&path, b"corrupt").unwrap();
        let mut recovered = Preferences::load(Some(temp.path()));
        assert!(recovered.enabled("/one").unwrap());
        recovered.set("/three", true).unwrap();
        assert_eq!(fs::read(backup(&path)).unwrap(), saved_backup);
        assert!(
            Preferences::load(Some(temp.path()))
                .enabled("/three")
                .unwrap()
        );
    }
    #[test]
    fn invalid_or_unwritable_preferences_are_not_silently_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sass-watch.json");
        for bytes in [
            br#"{"version":2,"enabled":[]}"#.as_slice(),
            br#"{"version":1,"enabled":["relative"]}"#,
        ] {
            fs::write(&path, bytes).unwrap();
            let mut preferences = Preferences::load(Some(temp.path()));
            assert!(preferences.enabled("/one").is_err());
            assert!(preferences.set("/one", true).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        let blocked = temp.path().join("blocked");
        fs::write(&blocked, "file").unwrap();
        let mut preferences = Preferences::load(Some(&blocked));
        assert!(preferences.set("/one", true).is_err());
        assert!(!preferences.enabled("/one").unwrap());
    }
}
