use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const SETTINGS_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub last_session: Option<String>,
    #[serde(default)]
    pub recent_sessions: Vec<String>,
    #[serde(default)]
    pub pinned_sessions: BTreeSet<String>,
    /// One-time marker: Source keeps pins only in renderer localStorage, so a
    /// completed scan must never be repeated or it would resurrect native
    /// unpins on the next launch.
    #[serde(default)]
    pub legacy_pins_imported: bool,
    #[serde(default = "default_warm_sessions")]
    pub warm_sessions: usize,
    #[serde(default = "default_warm_bytes")]
    pub warm_bytes: usize,
    #[serde(default)]
    pub appearance: String,
    #[serde(default = "default_shell")]
    pub shell: String,
    #[serde(default = "default_experimental_art")]
    pub experimental_art: bool,
    #[serde(default)]
    pub auto_approve_tools: bool,
    #[serde(default)]
    pub last_workspace: Option<String>,
    #[serde(default)]
    pub workspaces: Vec<crate::migration::WorkspaceBookmark>,
    #[serde(default)]
    pub workspace_locations: Vec<crate::migration::WorkspaceLocation>,
    #[serde(default)]
    pub favorite_models: BTreeSet<String>,
    #[serde(default)]
    pub hidden_models: BTreeSet<String>,
    #[serde(default)]
    pub hidden_providers: BTreeSet<String>,
    #[serde(default)]
    pub follow_up_behavior: crate::message_queue::FollowUpBehavior,
}

fn schema_version() -> u32 {
    SETTINGS_VERSION
}

fn default_shell() -> String {
    "codex".into()
}

fn default_experimental_art() -> bool {
    true
}

fn default_warm_sessions() -> usize {
    6
}

fn default_warm_bytes() -> usize {
    96 * 1024 * 1024
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_VERSION,
            last_session: None,
            recent_sessions: Vec::new(),
            pinned_sessions: BTreeSet::new(),
            legacy_pins_imported: false,
            warm_sessions: default_warm_sessions(),
            warm_bytes: default_warm_bytes(),
            appearance: "codex".into(),
            shell: default_shell(),
            experimental_art: true,
            auto_approve_tools: false,
            last_workspace: None,
            workspaces: Vec::new(),
            workspace_locations: Vec::new(),
            favorite_models: BTreeSet::new(),
            hidden_models: BTreeSet::new(),
            hidden_providers: BTreeSet::new(),
            follow_up_behavior: Default::default(),
        }
    }
}

pub fn default_data_dir() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("VELA_DATA_DIR") {
        return Some(PathBuf::from(directory));
    }
    dirs::data_dir().map(|base| base.join("vela-native"))
}

fn settings_path(directory: Option<&Path>) -> Option<PathBuf> {
    directory
        .map(Path::to_path_buf)
        .or_else(default_data_dir)
        .map(|dir| dir.join("settings.json"))
}

impl Settings {
    /// Clamp persisted values that would otherwise disable the warm cache or
    /// overflow the recent-session list. Shared by load and engine handoff so
    /// an in-memory update and the next launch behave identically.
    pub fn normalized(mut self) -> Self {
        if self.schema_version > SETTINGS_VERSION {
            return Self::default();
        }
        self.schema_version = SETTINGS_VERSION;
        if self.warm_sessions == 0 {
            self.warm_sessions = default_warm_sessions();
        }
        if self.warm_bytes < 8 * 1024 * 1024 {
            self.warm_bytes = default_warm_bytes();
        }
        self.recent_sessions.truncate(64);
        let mut seen = std::collections::HashSet::new();
        self.workspace_locations
            .retain(|location| seen.insert(location.directory.clone()));
        if self.workspace_locations.len() > 512 {
            self.workspace_locations
                .drain(..self.workspace_locations.len() - 512);
        }
        self
    }

    /// Load settings without ever rewriting an unreadable or newer file. A
    /// corrupt file falls back to defaults; the next explicit save repairs it.
    pub fn load(directory: Option<&Path>) -> Self {
        let Some(path) = settings_path(directory) else {
            return Self::default();
        };
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(settings) = serde_json::from_str::<Self>(&text) else {
            return Self::default();
        };
        settings.normalized()
    }

    pub fn save(&self, directory: Option<&Path>) -> io::Result<PathBuf> {
        let Some(path) = settings_path(directory) else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no application data directory",
            ));
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut value = self.clone();
        value.schema_version = SETTINGS_VERSION;
        value.recent_sessions.truncate(64);
        let bytes = serde_json::to_vec_pretty(&value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, bytes)?;
        // Atomic replace: a crash leaves either the old file or the new one,
        // never a truncated settings document.
        fs::rename(&temporary, &path)?;
        Ok(path)
    }

    pub fn record_session(&mut self, id: &str) {
        self.recent_sessions.retain(|existing| existing != id);
        self.recent_sessions.insert(0, id.to_string());
        self.recent_sessions.truncate(64);
        self.last_session = Some(id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_locations_survive_restart_with_aliases_and_identity() {
        let temp = tempfile::tempdir().unwrap();
        let settings = Settings {
            workspace_locations: vec![
                crate::migration::WorkspaceLocation {
                    directory: "/old".into(),
                    device: Some(7),
                    inode: Some(11),
                    moved_to: None,
                    project_id: None,
                },
                crate::migration::WorkspaceLocation {
                    directory: "/old/root".into(),
                    device: None,
                    inode: None,
                    moved_to: Some("/new".into()),
                    project_id: None,
                },
            ],
            ..Default::default()
        };
        settings.save(Some(temp.path())).unwrap();
        let loaded = Settings::load(Some(temp.path()));
        assert_eq!(loaded.workspace_locations, settings.workspace_locations);
        let text = fs::read_to_string(temp.path().join("settings.json")).unwrap();
        assert!(text.contains("\"movedTo\": \"/new\""));
    }

    #[test]
    fn pinned_sessions_survive_restart_without_losing_unavailable_ids() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("settings.json"), br#"{"shell":"classic"}"#).unwrap();
        let mut settings = Settings::load(Some(temp.path()));
        assert!(settings.pinned_sessions.is_empty());
        settings
            .pinned_sessions
            .extend(["session-unavailable".into(), "session-界".into()]);
        settings.save(Some(temp.path())).unwrap();
        assert_eq!(
            Settings::load(Some(temp.path())).pinned_sessions,
            settings.pinned_sessions
        );
        settings.pinned_sessions.remove("session-界");
        settings.save(Some(temp.path())).unwrap();
        assert_eq!(
            Settings::load(Some(temp.path()))
                .pinned_sessions
                .into_iter()
                .collect::<Vec<_>>(),
            ["session-unavailable"]
        );
    }

    #[test]
    fn model_preferences_survive_restart_and_older_settings_default_empty() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("settings.json"), br#"{"shell":"classic"}"#).unwrap();
        let mut settings = Settings::load(Some(temp.path()));
        assert!(settings.favorite_models.is_empty());
        assert!(settings.hidden_models.is_empty());
        assert!(settings.hidden_providers.is_empty());
        settings.favorite_models.insert("provider::model".into());
        settings.hidden_models.insert("other::model".into());
        settings.hidden_providers.insert("hidden-provider".into());
        settings.save(Some(temp.path())).unwrap();
        let loaded = Settings::load(Some(temp.path()));
        assert_eq!(loaded.favorite_models, settings.favorite_models);
        assert_eq!(loaded.hidden_models, settings.hidden_models);
        assert_eq!(loaded.hidden_providers, settings.hidden_providers);
        assert_eq!(loaded.shell, "classic");
    }

    #[test]
    fn round_trips_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        settings.record_session("ses_1");
        assert!(!settings.auto_approve_tools);
        settings.auto_approve_tools = true;
        let path = settings.save(Some(temp.path())).unwrap();
        assert!(path.exists());
        let loaded = Settings::load(Some(temp.path()));
        assert_eq!(loaded.last_session.as_deref(), Some("ses_1"));
        assert!(loaded.auto_approve_tools);
        assert_eq!(loaded.schema_version, SETTINGS_VERSION);
    }

    #[test]
    fn legacy_immediate_follow_up_aliases_steer() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("settings.json"),
            br#"{"follow_up_behavior":"immediate"}"#,
        )
        .unwrap();
        assert_eq!(
            Settings::load(Some(temp.path())).follow_up_behavior,
            crate::message_queue::FollowUpBehavior::Steer
        );
    }

    #[test]
    fn shell_preference_defaults_to_codex_and_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let defaults = Settings::load(Some(temp.path()));
        assert_eq!(defaults.shell, "codex");
        assert_eq!(
            defaults.follow_up_behavior,
            crate::message_queue::FollowUpBehavior::Queue
        );
        let mut settings = defaults;
        settings.shell = "classic".into();
        settings.follow_up_behavior = crate::message_queue::FollowUpBehavior::Steer;
        settings.save(Some(temp.path())).unwrap();
        assert_eq!(Settings::load(Some(temp.path())).shell, "classic");
        assert_eq!(
            Settings::load(Some(temp.path())).follow_up_behavior,
            crate::message_queue::FollowUpBehavior::Steer
        );
    }

    #[test]
    fn corrupt_file_falls_back_without_deleting_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, b"{not json").unwrap();
        let loaded = Settings::load(Some(temp.path()));
        assert_eq!(loaded.schema_version, SETTINGS_VERSION);
        assert!(path.exists());
    }

    #[test]
    fn future_schema_is_not_downgraded_silently() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, br#"{"schema_version": 99, "warm_sessions": 3}"#).unwrap();
        let loaded = Settings::load(Some(temp.path()));
        assert_eq!(loaded.schema_version, SETTINGS_VERSION);
        assert_eq!(loaded.warm_sessions, default_warm_sessions());
    }
}
