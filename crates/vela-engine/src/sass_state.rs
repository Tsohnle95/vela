//! Physical-root Sass workers shared by independently opened contexts.
use crate::{EngineUpdate, SassLine};
use std::collections::{HashMap, HashSet};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SassStatus {
    #[default]
    Off,
    Starting,
    On,
    Error,
}
impl SassStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Starting => "starting",
            Self::On => "on",
            Self::Error => "error",
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SassWatchState {
    pub directory: String,
    pub enabled: bool,
    pub status: SassStatus,
    pub lines: Vec<SassLine>,
    pub error: Option<String>,
}
pub(crate) fn limit_lines(lines: &mut Vec<SassLine>) {
    if lines.len() > 100 {
        lines.drain(..lines.len() - 100);
    }
    for line in lines {
        if line.message.len() > 8_192 {
            // Match the reference's JS slice(0, 8192), including a replacement
            // glyph when the boundary splits an astral character's surrogate.
            line.message = String::from_utf16_lossy(
                &line.message.encode_utf16().take(8_192).collect::<Vec<_>>(),
            );
        }
    }
}

#[derive(Clone)]
pub(crate) struct Lease {
    pub root: String,
    pub generation: uuid::Uuid,
    pub cancel: CancellationToken,
    pub write_gate: std::sync::Arc<std::sync::Mutex<()>>,
}
#[derive(Default)]
struct Root {
    owners: HashSet<String>,
    enabled: bool,
    worker: Option<Lease>,
    lines: Vec<SassLine>,
    status: SassStatus,
    error: Option<String>,
}
#[derive(Default)]
pub(crate) struct Watches {
    preferences: crate::sass_preferences::Preferences,
    owners: HashMap<String, String>,
    roots: HashMap<String, Root>,
    write_gates: HashMap<String, std::sync::Weak<std::sync::Mutex<()>>>,
}
impl Watches {
    pub fn registered(&self, session: &str) -> bool {
        self.owners.contains_key(session)
    }
    pub fn load(directory: Option<&std::path::Path>) -> Self {
        Self {
            preferences: crate::sass_preferences::Preferences::load(directory),
            ..Self::default()
        }
    }
    pub fn configured(&self, session: &str) -> Result<bool, String> {
        self.owners
            .get(session)
            .map(|root| self.preferences.enabled(root))
            .unwrap_or(Ok(false))
    }
    pub fn set_preference(&mut self, session: &str, enabled: bool) -> Result<(), String> {
        let root = self.owners.get(session).ok_or("Sass workspace is closed")?;
        self.preferences.set(root, enabled)
    }
    pub fn register(&mut self, session: &str, root: String) -> bool {
        if self
            .owners
            .get(session)
            .is_some_and(|previous| previous != &root)
        {
            self.unregister(session);
        }
        let fresh = !self.roots.contains_key(&root);
        self.owners.insert(session.into(), root.clone());
        self.roots
            .entry(root)
            .or_default()
            .owners
            .insert(session.into());
        fresh
    }
    pub fn unregister(&mut self, session: &str) {
        let Some(root) = self.owners.remove(session) else {
            return;
        };
        let Some(record) = self.roots.get_mut(&root) else {
            return;
        };
        record.owners.remove(session);
        if record.owners.is_empty()
            && let Some(record) = self.roots.remove(&root)
            && let Some(worker) = record.worker
        {
            worker.cancel.cancel();
        }
    }
    pub fn start(&mut self, session: &str, cancel: &CancellationToken) -> Option<Lease> {
        let root = self.owners.get(session)?;
        let record = self.roots.get_mut(root)?;
        if record
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.cancel.is_cancelled())
        {
            return None;
        }
        self.write_gates.retain(|_, gate| gate.strong_count() > 0);
        let gate = self.write_gates.entry(root.clone()).or_default();
        let write_gate = gate
            .upgrade()
            .unwrap_or_else(|| std::sync::Arc::new(std::sync::Mutex::new(())));
        *gate = std::sync::Arc::downgrade(&write_gate);
        let worker = Lease {
            root: root.clone(),
            generation: uuid::Uuid::new_v4(),
            cancel: cancel.child_token(),
            write_gate,
        };
        record.lines.push(SassLine {
            time_ms: crate::engine::sass_log_time(),
            level: crate::SassLogLevel::Info,
            message: format!("Starting bundled Dart Sass in {root}"),
        });
        limit_lines(&mut record.lines);
        record.worker = Some(worker.clone());
        record.enabled = true;
        record.status = SassStatus::Starting;
        record.error = None;
        Some(worker)
    }
    pub fn disable(&mut self, session: &str) -> Vec<EngineUpdate> {
        let Some(root) = self.owners.get(session).cloned() else {
            return Vec::new();
        };
        let record = self.roots.get_mut(&root).expect("registered root");
        if let Some(worker) = record.worker.take() {
            worker.cancel.cancel();
        }
        record.enabled = false;
        record.status = SassStatus::Off;
        record.error = None;
        record.lines.push(SassLine {
            time_ms: crate::engine::sass_log_time(),
            level: crate::SassLogLevel::Info,
            message: "Sass watching stopped".into(),
        });
        limit_lines(&mut record.lines);
        self.snapshots(&root)
    }
    pub fn enabled(&self, session: &str) -> bool {
        self.owners
            .get(session)
            .and_then(|root| self.roots.get(root))
            .is_some_and(|root| {
                root.enabled
                    && root
                        .worker
                        .as_ref()
                        .is_none_or(|worker| !worker.cancel.is_cancelled())
            })
    }
    pub fn lines(&self, session: &str) -> Vec<SassLine> {
        self.owners
            .get(session)
            .and_then(|root| self.roots.get(root))
            .map(|root| root.lines.clone())
            .unwrap_or_default()
    }
    pub fn snapshot(&self, session: &str) -> EngineUpdate {
        EngineUpdate::Sass {
            session_id: session.into(),
            state: self.state(session),
        }
    }
    pub fn state(&self, session: &str) -> SassWatchState {
        let Some(root) = self.owners.get(session) else {
            return SassWatchState::default();
        };
        let Some(record) = self.roots.get(root) else {
            return SassWatchState::default();
        };
        SassWatchState {
            directory: root.clone(),
            enabled: self.enabled(session),
            status: record.status,
            lines: record.lines.clone(),
            error: record.error.clone(),
        }
    }
    pub fn fail(&mut self, session: &str, error: String) -> Vec<EngineUpdate> {
        let Some(root) = self.owners.get(session).cloned() else {
            return vec![EngineUpdate::Sass {
                session_id: session.into(),
                state: SassWatchState {
                    status: SassStatus::Error,
                    error: Some(error),
                    ..Default::default()
                },
            }];
        };
        let record = self.roots.get_mut(&root).expect("registered root");
        record.status = SassStatus::Error;
        record.error = Some(error.clone());
        record.lines.push(SassLine {
            time_ms: crate::engine::sass_log_time(),
            level: crate::SassLogLevel::Error,
            message: error,
        });
        limit_lines(&mut record.lines);
        self.snapshots(&root)
    }
    pub fn finish(
        &mut self,
        worker: &Lease,
        lines: Vec<SassLine>,
        error: Option<String>,
    ) -> Vec<EngineUpdate> {
        if !self.live(worker) {
            return Vec::new();
        }
        let record = self.roots.get_mut(&worker.root).expect("live root");
        record.status = if error.is_some() {
            SassStatus::Error
        } else {
            SassStatus::On
        };
        record.error = error;
        self.publish(worker, lines)
    }
    pub fn failed_worker(
        &mut self,
        worker: &Lease,
        lines: Vec<SassLine>,
        error: String,
    ) -> Vec<EngineUpdate> {
        if !self.live(worker) {
            return Vec::new();
        }
        let updates = self.finish(worker, lines, Some(error));
        if let Some(stopped) = self
            .roots
            .get_mut(&worker.root)
            .and_then(|root| root.worker.take())
        {
            stopped.cancel.cancel();
        }
        updates
    }
    pub fn live(&self, worker: &Lease) -> bool {
        !worker.cancel.is_cancelled()
            && self
                .roots
                .get(&worker.root)
                .and_then(|root| root.worker.as_ref())
                .is_some_and(|current| current.generation == worker.generation)
    }
    pub fn worker_lines(&self, worker: &Lease) -> Option<Vec<SassLine>> {
        if !self.live(worker) {
            return None;
        }
        self.roots.get(&worker.root).map(|root| root.lines.clone())
    }
    pub fn publish(&mut self, worker: &Lease, mut lines: Vec<SassLine>) -> Vec<EngineUpdate> {
        if !self.live(worker) {
            return Vec::new();
        }
        limit_lines(&mut lines);
        self.roots
            .get_mut(&worker.root)
            .expect("live worker root")
            .lines = lines;
        self.snapshots(&worker.root)
    }
    fn snapshots(&self, root: &str) -> Vec<EngineUpdate> {
        let Some(record) = self.roots.get(root) else {
            return Vec::new();
        };
        let mut owners = record.owners.iter().collect::<Vec<_>>();
        owners.sort();
        owners
            .into_iter()
            .map(|owner| self.snapshot(owner))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_root_serializes_output_without_blocking_other_roots_or_state_reads() {
        let mut watches = Watches::default();
        let cancel = CancellationToken::new();
        watches.register("a", "/one".into());
        let old = watches.start("a", &cancel).unwrap();
        let held = old.write_gate.lock().unwrap();
        assert!(watches.state("a").enabled);
        watches.unregister("a");
        watches.register("b", "/one".into());
        let replacement = watches.start("b", &cancel).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &old.write_gate,
            &replacement.write_gate
        ));
        assert!(replacement.write_gate.try_lock().is_err());
        watches.register("c", "/two".into());
        let independent = watches.start("c", &cancel).unwrap();
        assert!(independent.write_gate.try_lock().is_ok());
        drop(held);
        assert!(replacement.write_gate.try_lock().is_ok());
    }
    #[test]
    fn log_limits_match_reference_utf16_slicing_and_keep_the_last_hundred_rows() {
        let message = format!("{}🌍tail", "a".repeat(8_191));
        let mut lines = (0..101)
            .map(|_| SassLine {
                time_ms: crate::engine::sass_log_time(),
                level: crate::SassLogLevel::Info,
                message: message.clone(),
            })
            .collect::<Vec<_>>();
        limit_lines(&mut lines);
        assert_eq!(lines.len(), 100);
        assert_eq!(lines[0].message.encode_utf16().count(), 8_192);
        assert!(lines[0].message.ends_with('�'));
    }
    #[test]
    fn failed_watcher_keeps_enabled_error_state_but_allows_an_explicit_restart() {
        let mut watches = Watches::default();
        let cancel = CancellationToken::new();
        watches.register("a", "/one".into());
        let worker = watches.start("a", &cancel).unwrap();
        let lines = watches.worker_lines(&worker).unwrap();
        watches.failed_worker(&worker, lines, "watcher failed".into());
        assert_eq!(watches.state("a").status, SassStatus::Error);
        assert!(watches.state("a").enabled);
        assert!(!watches.live(&worker));
        assert!(!watches.register("b", "/one".into()));
        assert_eq!(watches.state("b").status, SassStatus::Error);
        let restarted = watches.start("a", &cancel).unwrap();
        assert_ne!(worker.generation, restarted.generation);
        assert_eq!(watches.state("a").status, SassStatus::Starting);
        assert!(
            watches
                .failed_worker(&worker, Vec::new(), "late failure".into())
                .is_empty()
        );
        assert!(watches.live(&restarted));
    }
    #[test]
    fn shared_status_distinguishes_start_error_recovery_and_off_without_erasing_history() {
        let mut watches = Watches::default();
        let cancel = CancellationToken::new();
        watches.register("a", "/one".into());
        watches.register("b", "/one".into());
        let worker = watches.start("a", &cancel).unwrap();
        assert_eq!(watches.state("b").status, SassStatus::Starting);
        let failed = vec![SassLine {
            time_ms: crate::engine::sass_log_time(),
            level: crate::SassLogLevel::Error,
            message: "bad import".into(),
        }];
        watches.finish(&worker, failed.clone(), Some("bad import".into()));
        let state = watches.state("b");
        assert!(state.enabled);
        assert_eq!(state.status, SassStatus::Error);
        assert_eq!(state.error.as_deref(), Some("bad import"));
        let mut recovered = failed;
        recovered.push(SassLine {
            time_ms: crate::engine::sass_log_time(),
            level: crate::SassLogLevel::Info,
            message: "compiled".into(),
        });
        watches.finish(&worker, recovered, None);
        let state = watches.state("a");
        assert_eq!(state.status, SassStatus::On);
        assert!(state.error.is_none());
        assert_eq!(state.lines[0].level, crate::SassLogLevel::Error);
        watches.disable("b");
        assert_eq!(watches.state("a").status, SassStatus::Off);
        assert!(!watches.state("a").enabled);
        watches.fail("a", "preferences failed".into());
        assert_eq!(watches.state("b").status, SassStatus::Error);
        assert!(!watches.state("b").enabled);
        assert!(watches.finish(&worker, Vec::new(), None).is_empty());
        assert_eq!(watches.state("b").status, SassStatus::Error);
        let restarted = watches.start("b", &cancel).unwrap();
        assert_eq!(watches.state("a").status, SassStatus::Starting);
        assert!(watches.state("a").error.is_none());
        assert!(
            watches
                .worker_lines(&restarted)
                .unwrap()
                .iter()
                .any(|line| line.message == "compiled")
        );
        assert!(watches.worker_lines(&worker).is_none());
        assert!(watches.finish(&worker, Vec::new(), None).is_empty());
        assert_eq!(watches.state("b").status, SassStatus::Starting);
    }
    #[test]
    fn same_root_shares_worker_until_last_owner_closes() {
        let mut watches = Watches::default();
        let cancel = CancellationToken::new();
        watches.register("a", "/one".into());
        let worker = watches.start("a", &cancel).unwrap();
        watches.register("b", "/one".into());
        assert!(watches.start("b", &cancel).is_none());
        assert!(watches.enabled("b"));
        assert_eq!(
            watches
                .publish(
                    &worker,
                    vec![SassLine {
                        time_ms: crate::engine::sass_log_time(),
                        level: crate::SassLogLevel::Info,
                        message: "compiled".into()
                    }]
                )
                .len(),
            2
        );
        watches.unregister("a");
        assert!(watches.live(&worker));
        assert_eq!(watches.lines("b")[0].message, "compiled");
        watches.unregister("b");
        assert!(worker.cancel.is_cancelled());
        assert!(watches.publish(&worker, Vec::new()).is_empty());
    }
    #[test]
    fn roots_toggle_independently_but_all_owners_of_one_root_follow() {
        let mut watches = Watches::default();
        let cancel = CancellationToken::new();
        watches.register("a", "/one".into());
        watches.register("b", "/one".into());
        watches.register("c", "/two".into());
        let one = watches.start("a", &cancel).unwrap();
        let two = watches.start("c", &cancel).unwrap();
        assert_eq!(watches.disable("b").len(), 2);
        assert!(one.cancel.is_cancelled());
        assert!(!watches.enabled("a"));
        assert!(watches.live(&two));
        assert!(watches.enabled("c"));
        assert!(watches.disable("missing").is_empty());
        assert!(watches.live(&two));
    }
    #[test]
    fn relocation_and_reopen_reject_old_workers_and_output() {
        let mut watches = Watches::default();
        let cancel = CancellationToken::new();
        watches.register("a", "/one".into());
        let old = watches.start("a", &cancel).unwrap();
        watches.register("a", "/two".into());
        assert!(old.cancel.is_cancelled());
        let moved = watches.start("a", &cancel).unwrap();
        watches.unregister("a");
        watches.register("a", "/one".into());
        let reopened = watches.start("a", &cancel).unwrap();
        assert!(moved.cancel.is_cancelled());
        assert!(watches.publish(&old, Vec::new()).is_empty());
        assert!(watches.live(&reopened));
        cancel.cancel();
        assert!(!watches.live(&reopened));
    }
}
