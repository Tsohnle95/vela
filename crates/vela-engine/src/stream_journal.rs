//! Private, engine-owned append-before-reduce storage for normalized events.
//! The runtime remains authoritative; these files provide deterministic replay
//! of the native reducer, including its reconciled checkpoints.

use std::{
    collections::BTreeMap,
    fs,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vela_core::run_log::RunRecord;

const MAX_RECORD_BYTES: u64 = 32 * 1024 * 1024;
const MAX_REPLAY_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct StoredRecord {
    version: u32,
    record: RunRecord,
}

pub(crate) struct StreamJournal {
    root: Option<PathBuf>,
    instance: String,
    failed: bool,
}

impl StreamJournal {
    pub(crate) fn new(data_dir: Option<PathBuf>, instance: String) -> Self {
        Self {
            root: data_dir.map(|dir| dir.join("stream-journal")),
            instance,
            failed: false,
        }
    }

    pub(crate) fn is_failed(&self) -> bool {
        self.failed
    }

    /// Ordered OS append acknowledgement precedes reducer application.
    /// Blocking file work stays off GPUI. Drive flush barriers run at normal
    /// shutdown rather than on the frame path; OpenCode stays authoritative.
    pub(crate) async fn append(&mut self, records: Vec<RunRecord>) -> Result<(), String> {
        if self.failed {
            return Err(
                "Transcript journal is unavailable; reconnect after restarting Vela.".into(),
            );
        }
        if records.is_empty() {
            return Ok(());
        }
        let Some(root) = self.root.clone() else {
            self.failed = true;
            return Err("Transcript journal has no profile directory.".into());
        };
        let instance = self.instance.clone();
        let result = tokio::task::spawn_blocking(move || append_records(&root, &instance, records))
            .await.map_err(|_| "Transcript journal worker failed.".to_string())
            .and_then(|result| result.map_err(|_| "Could not save transcript events. Streaming is paused; free disk space and restart Vela.".to_string()));
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    pub(crate) async fn remove_session(&mut self, session: String) -> Result<(), String> {
        let Some(root) = self.root.clone() else {
            return Ok(());
        };
        tokio::task::spawn_blocking(move || {
            let name = session_filename(&session);
            let entries = match fs::read_dir(&root) {
                Ok(entries) => entries,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error),
            };
            for entry in entries {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    match fs::remove_file(entry.path().join(&name)) {
                        Ok(()) => {
                            fs::File::open(entry.path())?.sync_all()?;
                        }
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error),
                    }
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| "Transcript journal deletion worker failed.".to_string())?
        .map_err(|_| "Could not remove the deleted session's transcript journal.".to_string())
    }
    /// Finish normal shutdown with a durability barrier for this instance.
    pub(crate) async fn sync(&mut self) -> Result<(), String> {
        let Some(root) = self.root.clone() else {
            return Ok(());
        };
        let directory = root.join(&self.instance);
        tokio::task::spawn_blocking(move || {
            let entries = match fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error),
            };
            for entry in entries {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    fs::File::open(entry.path())?.sync_data()?;
                }
            }
            fs::File::open(&directory)?.sync_all()?;
            fs::File::open(&root)?.sync_all()?;
            if let Some(parent) = root.parent() {
                fs::File::open(parent)?.sync_all()?;
            }
            Ok(())
        })
        .await
        .map_err(|_| "Transcript journal shutdown worker failed.".to_string())?
        .map_err(|_| "Could not synchronize transcript journal at shutdown.".to_string())
    }
}

fn session_filename(session: &str) -> String {
    format!("{:x}.jsonl", Sha256::digest(session.as_bytes()))
}

fn private_directory(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(io::Error::other("journal directory is not a directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(path)?.permissions().mode() & 0o777 != 0o700 {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(())
}

fn append_records(root: &Path, instance: &str, records: Vec<RunRecord>) -> io::Result<()> {
    private_directory(root)?;
    let directory = root.join(instance);
    private_directory(&directory)?;
    let mut batches: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for record in records {
        let Some(session) = record.session_id() else {
            continue;
        };
        let name = session_filename(session);
        let encoded = serde_json::to_vec(&StoredRecord { version: 1, record })?;
        if encoded.len() as u64 + 1 > MAX_RECORD_BYTES {
            return Err(io::Error::other("journal record exceeds replay limit"));
        }
        let batch = batches.entry(name).or_default();
        batch.extend_from_slice(&encoded);
        batch.push(b'\n');
    }
    for (name, batch) in batches {
        let mut options = fs::OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let path = directory.join(name);
        let mut file = options.open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("journal is not a regular file"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            if file.metadata()?.nlink() != 1 {
                return Err(io::Error::other("journal has external hard links"));
            }
            if file.metadata()?.permissions().mode() & 0o777 != 0o600 {
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
        }
        file.write_all(&batch)?;
    }
    Ok(())
}

/// Read an explicitly selected journal for offline diagnostics/tests. Never
/// load arbitrary production logs at startup or print their payloads. A partial
/// final record is rejected; consumers must not guess which tokens were saved.
pub fn read_records(path: &Path) -> io::Result<Vec<RunRecord>> {
    let file = fs::File::open(path)?;
    let mut reader = io::BufReader::new(file.take(MAX_REPLAY_BYTES + 1));
    let mut records = Vec::new();
    let mut total = 0u64;
    loop {
        let mut bytes = Vec::new();
        let count = reader
            .by_ref()
            .take(MAX_RECORD_BYTES + 1)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_REPLAY_BYTES
            || count as u64 > MAX_RECORD_BYTES
            || bytes.last() != Some(&b'\n')
        {
            return Err(io::Error::other(
                "incomplete or oversized transcript journal",
            ));
        }
        let stored: StoredRecord = serde_json::from_slice(&bytes)?;
        if stored.version != 1 {
            return Err(io::Error::other("unsupported transcript journal version"));
        }
        records.push(stored.record);
    }
    Ok(records)
}

/// Replay arbitrarily long session files with bounded record parsing. Unlike
/// the diagnostic `read_records`, this never retains all event payloads.
pub fn replay_file(path: &Path, session: &str) -> io::Result<vela_core::transcript::Transcript> {
    let mut reader = io::BufReader::new(fs::File::open(path)?);
    let mut transcript = vela_core::transcript::Transcript::new();
    let mut seq = 0;
    let mut ids = std::collections::HashSet::new();
    loop {
        let mut bytes = Vec::new();
        let count = reader
            .by_ref()
            .take(MAX_RECORD_BYTES + 1)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            break;
        }
        if count as u64 > MAX_RECORD_BYTES || bytes.last() != Some(&b'\n') {
            return Err(io::Error::other(
                "incomplete or oversized transcript record",
            ));
        }
        let stored: StoredRecord = serde_json::from_slice(&bytes)?;
        if stored.version != 1 || stored.record.session_id() != Some(session) {
            return Err(io::Error::other(
                "unsupported or mismatched transcript record",
            ));
        }
        if let RunRecord::Event(event) = &stored.record {
            if event.id.is_empty() || event.run_id.is_empty() {
                return Err(io::Error::other("transcript event has no stable identity"));
            }
            if !ids.insert(event.id.clone()) {
                continue;
            }
        }
        if stored.record.seq() <= seq {
            return Err(io::Error::other("nonmonotonic transcript record"));
        }
        seq = stored.record.seq();
        stored.record.apply_to(&mut transcript);
    }
    Ok(transcript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vela_core::{run_log::EventLog, session::SessionPhase};

    #[tokio::test]
    async fn ordered_records_replay_ten_thousand_deltas_and_reject_duplicates() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = StreamJournal::new(Some(root.path().to_owned()), "fixture".into());
        let mut log = EventLog::new("fixture");
        log.record_snapshot("session", Vec::new(), SessionPhase::Idle);
        for index in 0..10_000 {
            let raw = json!({"id": format!("delta-{index}"), "type":"session.text.delta", "data": {
                "sessionID":"session", "assistantMessageID":"answer", "ordinal":0,"delta":"x"
            }});
            assert!(log.append(&raw, index).is_some());
            assert!(log.append(&raw, index).is_none());
        }
        writer.append(log.drain_records()).await.unwrap();
        writer.sync().await.unwrap();
        let path = root
            .path()
            .join("stream-journal/fixture")
            .join(session_filename("session"));
        let records = read_records(&path).unwrap();
        let recovered = EventLog::from_records("recovered", records).unwrap();
        let mut transcript = recovered.replay("session");
        assert_eq!(
            transcript
                .blocks()
                .iter()
                .find(|block| block.message_id.as_ref() == "answer")
                .unwrap()
                .text
                .len(),
            10_000
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"{\"partial\"")
            .unwrap();
        assert!(read_records(&path).is_err());
        writer.remove_session("session".into()).await.unwrap();
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn failed_append_latches_and_never_reports_success_after_losing_events() {
        let root = tempfile::tempdir().unwrap();
        let blocked = root.path().join("profile");
        fs::write(&blocked, "not a directory").unwrap();
        let mut writer = StreamJournal::new(Some(blocked), "fixture".into());
        let mut log = EventLog::new("fixture");
        log.record_phase("session", SessionPhase::Busy);
        assert!(writer.append(log.drain_records()).await.is_err());
        assert!(writer.is_failed());
        log.record_phase("session", SessionPhase::Idle);
        assert!(writer.append(log.drain_records()).await.is_err());
    }
}
