//! Durable recovery for prompts the runtime did not accept. A queue belongs to
//! both the workspace directory and session; opening another panel cannot
//! accidentally deliver its files. Disk retains retries until runtime ACK.
use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vela_opencode::PromptFile;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FollowUpBehavior {
    #[default]
    Queue,
    #[serde(alias = "immediate")]
    Steer,
}
impl FollowUpBehavior {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queue => "queue",
            Self::Steer => "steer",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedPrompt {
    pub id: String,
    pub text: String,
    pub files: Vec<PromptFile>,
    pub created_at: i64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct State {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    queues: BTreeMap<String, Vec<QueuedPrompt>>,
    #[serde(default)]
    quarantined: BTreeMap<String, Vec<QueuedPrompt>>,
}

const STATE_VERSION: u32 = 3;

pub(crate) struct MessageQueues {
    state: State,
    path: Option<PathBuf>,
    pending: HashSet<String>,
    load_error: Option<String>,
}

fn key(directory: &str, session: &str) -> String {
    format!("{directory}\n{session}")
}

/// Match Source's queue migration: pre-v3 queues are not replayed; they move
/// into the quarantined map and the active queue starts empty.
fn migrate(state: &mut State) -> bool {
    if state.version >= STATE_VERSION {
        return false;
    }
    for (key, queue) in std::mem::take(&mut state.queues) {
        if !queue.is_empty() {
            state.quarantined.entry(key).or_default().extend(queue);
        }
    }
    state.version = STATE_VERSION;
    true
}

impl MessageQueues {
    pub fn load(directory: Option<&Path>) -> Self {
        let path = directory.map(|dir| dir.join("message-queues.json"));
        let loaded = path.as_ref().map(std::fs::read).transpose();
        let (mut state, load_error) = match loaded {
            Ok(Some(bytes)) => match serde_json::from_slice(&bytes) {
                Ok(state) => (state, None),
                Err(error) => (
                    State {
                        version: STATE_VERSION,
                        ..State::default()
                    },
                    Some(error.to_string()),
                ),
            },
            Ok(None) => (
                State {
                    version: STATE_VERSION,
                    ..State::default()
                },
                None,
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (
                State {
                    version: STATE_VERSION,
                    ..State::default()
                },
                None,
            ),
            Err(error) => (
                State {
                    version: STATE_VERSION,
                    ..State::default()
                },
                Some(error.to_string()),
            ),
        };
        let migrated = migrate(&mut state);
        let mut queues = Self {
            state,
            path,
            pending: HashSet::new(),
            load_error,
        };
        if migrated && let Err(error) = queues.save() {
            queues.load_error = Some(error.to_string());
        }
        queues
    }

    pub fn list(&self, directory: &str, session: &str) -> Vec<QueuedPrompt> {
        self.state
            .queues
            .get(&key(directory, session))
            .into_iter()
            .flatten()
            .filter(|item| !self.pending.contains(&item.id))
            .cloned()
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn quarantined(&self, directory: &str, session: &str) -> Vec<QueuedPrompt> {
        self.state
            .quarantined
            .get(&key(directory, session))
            .cloned()
            .unwrap_or_default()
    }

    pub fn enqueue(
        &mut self,
        directory: &str,
        session: &str,
        text: String,
        files: Vec<PromptFile>,
        created_at: i64,
    ) -> io::Result<()> {
        let queue = self
            .state
            .queues
            .entry(key(directory, session))
            .or_default();
        queue.push(QueuedPrompt {
            id: format!("queued-{}", uuid::Uuid::new_v4()),
            text,
            files,
            created_at,
        });
        // Match Vela's recovery limits: newest twenty prompts across fifty targets.
        if queue.len() > 20 {
            queue.drain(..queue.len() - 20);
        }
        if self.state.queues.len() > 50 {
            let oldest = self
                .state
                .queues
                .iter()
                .min_by_key(|(_, queue)| queue.first().map(|item| item.created_at).unwrap_or(0))
                .map(|(key, _)| key.clone());
            if let Some(key) = oldest {
                self.state.queues.remove(&key);
            }
        }
        self.save()
    }

    pub fn remove(&mut self, directory: &str, session: &str, id: &str) -> io::Result<()> {
        let key = key(directory, session);
        if let Some(queue) = self.state.queues.get_mut(&key) {
            queue.retain(|item| item.id != id);
            if queue.is_empty() {
                self.state.queues.remove(&key);
            }
        }
        self.pending.remove(id);
        self.save()
    }

    pub fn reorder(
        &mut self,
        directory: &str,
        session: &str,
        from: &str,
        to: &str,
    ) -> io::Result<()> {
        if from == to {
            return Ok(());
        }
        if let Some(queue) = self.state.queues.get_mut(&key(directory, session)) {
            let indexes = queue
                .iter()
                .position(|item| item.id == from)
                .zip(queue.iter().position(|item| item.id == to));
            if let Some((from, to)) = indexes {
                let item = queue.remove(from);
                queue.insert(to, item);
                return self.save();
            }
        }
        Ok(())
    }

    pub fn claim(&mut self, directory: &str, session: &str, id: &str) -> Option<QueuedPrompt> {
        if self.pending.contains(id) {
            return None;
        }
        let item = self
            .list(directory, session)
            .into_iter()
            .find(|item| item.id == id)?;
        self.pending.insert(id.to_owned());
        Some(item)
    }

    pub fn retry_failed(
        &mut self,
        directory: &str,
        session: &str,
        id: &str,
        at: i64,
    ) -> io::Result<()> {
        self.pending.remove(id);
        let Some(queue) = self.state.queues.get_mut(&key(directory, session)) else {
            return Ok(());
        };
        let Some(index) = queue.iter().position(|item| item.id == id) else {
            return Ok(());
        };
        let mut item = queue.remove(index);
        item.id = format!("queued-{}", uuid::Uuid::new_v4());
        item.created_at = at;
        queue.push(item);
        self.save()
    }

    fn save(&self) -> io::Result<()> {
        if let Some(error) = &self.load_error {
            // Preserve an unreadable original rather than replacing stored prompts.
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Stored queue could not be loaded: {error}"),
            ));
        }
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.state)?;
        let temporary = path.with_extension("json.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_keeps_attachments_order_and_session_workspace_ownership() {
        let temp = tempfile::tempdir().unwrap();
        let mut queues = MessageQueues::load(Some(temp.path()));
        queues
            .enqueue(
                "/one",
                "same",
                "First\n🌍".into(),
                vec![PromptFile {
                    path: "/tmp/image 🌍.png".into(),
                    name: None,
                    mention: None,
                }],
                1,
            )
            .unwrap();
        queues
            .enqueue("/one", "same", "Second".into(), vec![], 2)
            .unwrap();
        let original = queues.list("/one", "same");
        queues
            .reorder("/one", "same", &original[1].id, &original[0].id)
            .unwrap();
        assert!(queues.list("/two", "same").is_empty());
        assert!(queues.list("/one", "other").is_empty());
        let ordered = queues.list("/one", "same");
        assert_eq!(ordered[0].text, "Second");
        assert!(queues.claim("/one", "same", &original[0].id).is_some());
        assert!(queues.claim("/one", "same", &original[0].id).is_none());
        assert_eq!(queues.list("/one", "same").len(), 1);
        // A crash during HTTP delivery must leave the unacknowledged item on disk.
        let mut restarted = MessageQueues::load(Some(temp.path()));
        let restored = restarted.list("/one", "same");
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[1].text, "First\n🌍");
        assert_eq!(restored[1].files[0].path, "/tmp/image 🌍.png");
        restarted.remove("/one", "same", &original[0].id).unwrap();
        assert_eq!(
            MessageQueues::load(Some(temp.path()))
                .list("/one", "same")
                .len(),
            1
        );
    }

    #[test]
    fn unversioned_legacy_queues_are_quarantined_and_persisted_as_v3() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("message-queues.json");
        std::fs::write(
            &path,
            r#"{"queues":{"/one\nses":[{"id":"legacy-1","text":"Old prompt","files":[],"created_at":1}]}}"#,
        )
        .unwrap();
        let queues = MessageQueues::load(Some(temp.path()));
        assert!(queues.list("/one", "ses").is_empty());
        let quarantined = queues.quarantined("/one", "ses");
        assert_eq!(quarantined.len(), 1);
        assert_eq!(quarantined[0].text, "Old prompt");

        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["version"], STATE_VERSION);
        assert!(saved["queues"].as_object().unwrap().is_empty());
        assert_eq!(
            saved["quarantined"][format!("{}{}", "/one", "\nses")][0]["text"],
            "Old prompt"
        );

        let mut restarted = MessageQueues::load(Some(temp.path()));
        assert!(restarted.list("/one", "ses").is_empty());
        assert_eq!(restarted.quarantined("/one", "ses").len(), 1);
        restarted
            .enqueue("/one", "ses", "New prompt".into(), vec![], 2)
            .unwrap();
        let queued = restarted.list("/one", "ses");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].text, "New prompt");
    }

    #[test]
    fn invalid_saved_queue_is_preserved_and_failed_save_keeps_memory() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("message-queues.json");
        std::fs::write(&path, "invalid original").unwrap();
        let mut queues = MessageQueues::load(Some(temp.path()));
        assert!(
            queues
                .enqueue("/one", "ses", "recover me".into(), vec![], 0)
                .is_err()
        );
        assert_eq!(queues.list("/one", "ses")[0].text, "recover me");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "invalid original");
    }
}
