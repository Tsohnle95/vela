//! Event-driven Sass changes: coalesce bursts without queuing unbounded work.
use notify::{
    EventKind, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind},
};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::mpsc, time::Instant};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Pending {
    changed: Option<Instant>,
    errors: VecDeque<String>,
}
pub(crate) struct Changes {
    _watcher: WatcherLease,
    pending: Arc<Mutex<Pending>>,
    wake: mpsc::Receiver<()>,
    closed: bool,
}
struct WatcherLease(Option<notify::RecommendedWatcher>);
impl Drop for WatcherLease {
    fn drop(&mut self) {
        let watcher = self.0.take();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            // Some backends join an OS run loop on drop. Keep that join off
            // the runtime threads that process context and save acknowledgements.
            runtime.spawn_blocking(move || drop(watcher));
        } else {
            drop(watcher);
        }
    }
}
fn relevant(root: &Path, event: &notify::Event) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }
    if event.need_rescan() {
        return true;
    }
    let structural = matches!(
        event.kind,
        EventKind::Create(CreateKind::Folder | CreateKind::Any)
            | EventKind::Remove(RemoveKind::Folder | RemoveKind::Any)
            | EventKind::Modify(ModifyKind::Name(_))
    );
    event.paths.iter().any(|path| {
        !crate::sass_sources::ignored_path(root, path)
            && (structural || crate::sass_sources::relevant(root, path))
    })
}
impl Changes {
    pub fn start(root: &Path) -> Result<Self, String> {
        let pending = Arc::new(Mutex::new(Pending::default()));
        let collected = pending.clone();
        let watched_root = root.to_owned();
        let (tx, wake) = mpsc::channel(1);
        let mut watcher = notify::RecommendedWatcher::new(
            move |event: notify::Result<notify::Event>| {
                let mut pending = collected.lock().expect("Sass changes lock");
                match event {
                    Ok(event) if relevant(&watched_root, &event) => {
                        pending.changed = Some(Instant::now())
                    }
                    Ok(_) => return,
                    Err(error) => {
                        if pending.errors.len() == 100 {
                            pending.errors.pop_front();
                        }
                        pending.errors.push_back(error.to_string());
                    }
                }
                // Only one wake is needed; the shared latest instant keeps all
                // later changes even while a compilation is already in progress.
                let _ = tx.try_send(());
            },
            notify::Config::default().with_follow_symlinks(false),
        )
        .map_err(|error| error.to_string())?;
        watcher
            .watch(root, notify::RecursiveMode::Recursive)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            _watcher: WatcherLease(Some(watcher)),
            pending,
            wake,
            closed: false,
        })
    }
    pub async fn next(&mut self, cancel: &CancellationToken) -> Option<Result<(), String>> {
        if self.closed {
            return None;
        }
        loop {
            let changed = {
                let mut pending = self.pending.lock().expect("Sass changes lock");
                if let Some(error) = pending.errors.pop_front() {
                    return Some(Err(error));
                }
                pending.changed
            };
            if let Some(changed) = changed {
                tokio::select! {
                    _ = cancel.cancelled() => return None,
                    _ = tokio::time::sleep_until(changed + Duration::from_millis(250)) => {
                        let mut pending = self.pending.lock().expect("Sass changes lock");
                        if pending.changed == Some(changed) {
                            pending.changed = None;
                            return Some(Ok(()));
                        }
                    }
                    signal = self.wake.recv() => {
                        if signal.is_none() { self.closed = true; return Some(Err("Sass file watcher stopped".into())); }
                    }
                }
            } else {
                tokio::select! {
                    _ = cancel.cancelled() => return None,
                    signal = self.wake.recv() => {
                        if signal.is_none() { self.closed = true; return Some(Err("Sass file watcher stopped".into())); }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notifications_ignore_css_access_and_dependencies_but_include_partials_config_and_directories()
     {
        let root = Path::new("/fixture");
        let event = |name: &str, kind| notify::Event::new(kind).add_path(root.join(name));
        let modify = EventKind::Modify(ModifyKind::Any);
        for name in [
            "main.scss",
            "nested/_ink.SCSS",
            "vela-sass.json",
            ".vscode/settings.json",
            "package.json",
        ] {
            assert!(relevant(root, &event(name, modify)));
        }
        for name in [
            "main.css",
            "main.css.map",
            "README.md",
            "node_modules/a.scss",
            ".vela-recovery/a.scss",
        ] {
            assert!(!relevant(root, &event(name, modify)));
        }
        assert!(!relevant(
            root,
            &event(
                "main.scss",
                EventKind::Access(notify::event::AccessKind::Any)
            )
        ));
        assert!(relevant(
            root,
            &event("nested", EventKind::Create(CreateKind::Folder))
        ));
        assert!(relevant(
            root,
            &event("nested", EventKind::Remove(RemoveKind::Folder))
        ));
        assert!(!relevant(
            root,
            &event("node_modules/nested", EventKind::Create(CreateKind::Folder))
        ));
    }
    #[tokio::test]
    async fn actual_file_events_wake_once_after_a_burst_and_cancellation_stops_waiting() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut changes = Changes::start(&root).unwrap();
        let cancel = CancellationToken::new();
        for color in ["red", "blue", "green"] {
            std::fs::write(root.join("main.scss"), format!(".a {{ color: {color}; }}")).unwrap();
        }
        tokio::time::timeout(Duration::from_secs(5), changes.next(&cancel))
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            std::fs::read_to_string(root.join("main.scss"))
                .unwrap()
                .contains("green")
        );
        std::fs::write(root.join("main.css"), ".a{color:green}").unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(500), changes.next(&cancel))
                .await
                .is_err()
        );
        cancel.cancel();
        assert!(changes.next(&cancel).await.is_none());
    }
}
