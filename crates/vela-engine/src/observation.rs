//! Workspace-owned filesystem observation and serialized file jobs.
//!
//! Native notifications only enqueue bounded, debounced paths. One worker per
//! attached workspace owns reads, writes, identities and first-change baselines.
//! Its writes update the held snapshot before queued notifications are read, so
//! a save echo cannot hide a different external write that follows it.
use crate::files::file_identity;
use crate::{EngineUpdate, files};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const DEBOUNCE: Duration = Duration::from_millis(200);
const MAX_PENDING: usize = 4096;
const MAX_WATCH_BYTES: u64 = 2 * 1024 * 1024;
const SKIP: &[&str] = &[
    ".git",
    "node_modules",
    ".next",
    ".venv",
    "__pycache__",
    ".cache",
    ".turbo",
    ".nx",
    ".svn",
    ".hg",
    ".idea",
    ".vscode",
    ".vela-recovery",
    ".mira-recovery",
    ".openshell-recovery",
    ".orbit-recovery",
    "dist",
    "out",
    "build",
    "target",
    "coverage",
    ".pytest_cache",
    ".opencode",
    ".claude",
    ".cursor",
    ".aider",
    ".windsurf",
    ".codeium",
    ".roo",
    ".gemini",
    ".kilocode",
    ".continue",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Baseline {
    Unknown,
    Known { content: Arc<str>, existed: bool },
}

#[derive(Clone, Debug)]
pub struct ObservedFile {
    pub rel: String,
    pub content: Option<Arc<str>>,
    pub baseline: Baseline,
    pub deleted: bool,
    pub moved_from: Option<String>,
    pub write_id: Option<u64>,
}

type Snapshot = Arc<RwLock<Arc<Vec<ObservedFile>>>>;
type Job = Box<dyn FnOnce(&mut Workspace) -> EngineUpdate + Send>;
enum Work {
    Wake,
    Job(Job),
    CaptureBaselines(Vec<String>),
    WatcherReady(Result<RecommendedWatcher, String>),
    Stop,
}

struct Worker {
    generation: u64,
    active: Arc<AtomicBool>,
    tx: mpsc::Sender<Work>,
    owners: HashSet<String>,
    snapshot: Snapshot,
}
struct ActiveGuard(Arc<AtomicBool>);
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
#[derive(Default)]
struct Registry {
    workers: HashMap<String, Worker>,
    sessions: HashMap<String, String>,
    closed: bool,
}
struct Inner {
    registry: Mutex<Registry>,
    sequence: AtomicU64,
    updates: tokio::sync::broadcast::Sender<EngineUpdate>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        if let Ok(registry) = self.registry.get_mut() {
            for (_, worker) in registry.workers.drain() {
                worker.active.store(false, Ordering::Release);
                let _ = worker.tx.send(Work::Stop);
            }
        }
    }
}
#[derive(Clone)]
pub(crate) struct ObservationPool(Arc<Inner>);

impl ObservationPool {
    pub fn new(updates: tokio::sync::broadcast::Sender<EngineUpdate>) -> Self {
        Self(Arc::new(Inner {
            registry: Mutex::default(),
            sequence: AtomicU64::new(1),
            updates,
        }))
    }

    pub fn attach(&self, session: &str, directory: &str) -> Result<(), String> {
        self.sender(session, directory).map(|_| ())
    }

    fn sender(&self, session: &str, directory: &str) -> Result<mpsc::Sender<Work>, String> {
        if directory.is_empty() {
            return Err("This session has no workspace directory".into());
        }
        let mut registry = self.0.registry.lock().expect("observation registry");
        if registry.closed {
            return Err("Vela is shutting down".into());
        }
        if registry
            .sessions
            .get(session)
            .is_some_and(|old| old != directory)
        {
            detach(&mut registry, session);
        }
        if registry
            .workers
            .get(directory)
            .is_some_and(|worker| !worker.active.load(Ordering::Acquire))
        {
            registry.workers.remove(directory);
        }
        if !registry.workers.contains_key(directory) {
            let generation = self.0.sequence.fetch_add(1, Ordering::Relaxed);
            let (tx, rx) = mpsc::channel();
            let active = Arc::new(AtomicBool::new(true));
            let snapshot = Arc::new(RwLock::new(Arc::new(Vec::new())));
            let identity = Identity {
                directory: directory.to_owned(),
                generation,
                active: active.clone(),
                updates: self.0.updates.clone(),
            };
            let worker_snapshot = snapshot.clone();
            let wake = tx.clone();
            std::thread::Builder::new()
                .name("vela-workspace-files".into())
                .spawn(move || run(identity, rx, wake, worker_snapshot))
                .map_err(|error| error.to_string())?;
            let owners = registry
                .sessions
                .iter()
                .filter(|(_, root)| root.as_str() == directory)
                .map(|(session, _)| session.clone())
                .collect();
            registry.workers.insert(
                directory.to_owned(),
                Worker {
                    generation,
                    active,
                    tx,
                    owners,
                    snapshot,
                },
            );
        }
        registry
            .sessions
            .insert(session.to_owned(), directory.to_owned());
        let worker = registry
            .workers
            .get_mut(directory)
            .expect("attached worker");
        worker.owners.insert(session.to_owned());
        Ok(worker.tx.clone())
    }

    pub fn submit(
        &self,
        session: &str,
        directory: &str,
        job: impl FnOnce(&mut Workspace) -> EngineUpdate + Send + 'static,
    ) -> Result<(), String> {
        self.sender(session, directory)?
            .send(Work::Job(Box::new(job)))
            .map_err(|_| "Workspace observation stopped".into())
    }

    /// Record pre-tool content as a known baseline. Tool paths may be relative
    /// to the workspace or absolute; unreadable, outside-root and unsafe paths
    /// are ignored, and an existing baseline is never replaced.
    pub fn capture_baselines(&self, session: &str, directory: &str, paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        // Snapshot only an already-attached workspace, matching Source's
        // context lookup; a tool event must not create a new watcher.
        let sender = {
            let registry = self.0.registry.lock().expect("observation registry");
            registry
                .sessions
                .get(session)
                .filter(|root| root.as_str() == directory)
                .and_then(|_| registry.workers.get(directory))
                .filter(|worker| worker.active.load(Ordering::Acquire))
                .map(|worker| worker.tx.clone())
        };
        if let Some(tx) = sender {
            let _ = tx.send(Work::CaptureBaselines(paths));
        }
    }

    pub fn retain_editor(&self, session: &str, directory: &str) -> Result<(), String> {
        // Session authorization is checked by the engine before this lease is
        // granted. Its lifetime follows documents, independently of warm views.
        self.attach(session, directory)?;
        self.sender(&editor_owner(directory), directory).map(|_| ())
    }

    pub fn release_editor(&self, directory: &str) {
        self.detach(&editor_owner(directory));
    }

    pub fn submit_editor(
        &self,
        directory: &str,
        job: impl FnOnce(&mut Workspace) -> EngineUpdate + Send + 'static,
    ) -> Result<(), String> {
        let tx = {
            let registry = self.0.registry.lock().expect("observation registry");
            registry
                .workers
                .get(directory)
                .filter(|worker| {
                    worker.active.load(Ordering::Acquire)
                        && worker.owners.contains(&editor_owner(directory))
                })
                .map(|worker| worker.tx.clone())
        }
        .ok_or("This editor workspace is no longer open")?;
        tx.send(Work::Job(Box::new(job)))
            .map_err(|_| "Workspace observation stopped".into())
    }
    pub fn detach(&self, session: &str) {
        detach(
            &mut self.0.registry.lock().expect("observation registry"),
            session,
        );
    }
    pub fn close(&self) {
        let mut registry = self.0.registry.lock().expect("observation registry");
        registry.closed = true;
        registry.sessions.clear();
        for (_, worker) in registry.workers.drain() {
            worker.active.store(false, Ordering::Release);
            let _ = worker.tx.send(Work::Stop);
        }
    }
    pub fn is_current(&self, directory: &str, generation: u64) -> bool {
        self.0
            .registry
            .lock()
            .expect("observation registry")
            .workers
            .get(directory)
            .is_some_and(|worker| {
                worker.generation == generation && worker.active.load(Ordering::Acquire)
            })
    }
    pub fn snapshot(&self, directory: &str) -> Arc<Vec<ObservedFile>> {
        self.0
            .registry
            .lock()
            .expect("observation registry")
            .workers
            .get(directory)
            .map(|worker| {
                worker
                    .snapshot
                    .read()
                    .expect("observation snapshot")
                    .clone()
            })
            .unwrap_or_default()
    }
}
fn editor_owner(directory: &str) -> String {
    format!("editor:{directory}")
}
fn detach(registry: &mut Registry, session: &str) {
    if let Some(directory) = registry.sessions.remove(session)
        && let Some(worker) = registry.workers.get_mut(&directory)
    {
        worker.owners.remove(session);
        if worker.owners.is_empty() {
            let worker = registry.workers.remove(&directory).expect("unowned worker");
            worker.active.store(false, Ordering::Release);
            let _ = worker.tx.send(Work::Stop);
        }
    }
}

struct Identity {
    directory: String,
    generation: u64,
    active: Arc<AtomicBool>,
    updates: tokio::sync::broadcast::Sender<EngineUpdate>,
}
impl Identity {
    fn emit(
        &self,
        changes: Vec<ObservedFile>,
        tree_changed: bool,
        git_changed: bool,
        error: Option<String>,
    ) {
        if self.active.load(Ordering::Acquire) {
            let _ = self.updates.send(EngineUpdate::WorkspaceFiles {
                directory: self.directory.clone(),
                generation: self.generation,
                changes: Arc::new(changes),
                tree_changed,
                git_changed,
                error,
            });
        }
    }
}

#[derive(Default)]
struct Pending {
    paths: HashMap<PathBuf, Instant>,
    rescan: Option<Instant>,
    error: Option<String>,
}
impl Pending {
    fn event(&mut self, root: &Path, result: notify::Result<Event>) {
        let deadline = Instant::now() + DEBOUNCE;
        match result {
            Ok(event) => {
                if matches!(event.kind, EventKind::Access(_)) {
                    return;
                }
                if event.need_rescan() {
                    self.rescan.get_or_insert(deadline);
                }
                for path in event.paths {
                    if path
                        .strip_prefix(root)
                        .is_ok_and(|rel| !skipped(rel) || is_git(rel))
                    {
                        if self.paths.len() >= MAX_PENDING {
                            self.paths.clear();
                            self.rescan.get_or_insert(deadline);
                        }
                        self.paths.insert(path, deadline);
                    }
                }
            }
            Err(error) => {
                self.error = Some(error.to_string());
                self.rescan.get_or_insert(deadline);
            }
        }
    }
    fn wait(&self) -> Duration {
        self.paths
            .values()
            .copied()
            .chain(self.rescan)
            .min()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(1))
    }
}

fn skipped(rel: &Path) -> bool {
    rel.components()
        .any(|part| SKIP.iter().any(|skip| part.as_os_str() == *skip))
}
fn is_git(rel: &Path) -> bool {
    rel.components()
        .next()
        .is_some_and(|part| part.as_os_str() == ".git")
}

fn run(identity: Identity, rx: mpsc::Receiver<Work>, wake: mpsc::Sender<Work>, snapshot: Snapshot) {
    run_with_setup(identity, rx, wake, snapshot, |root, callback| {
        let mut watcher =
            RecommendedWatcher::new(callback, Config::default().with_follow_symlinks(false))?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(watcher)
    });
}

type WatchCallback = Box<dyn FnMut(notify::Result<notify::Event>) + Send>;
fn run_with_setup(
    identity: Identity,
    rx: mpsc::Receiver<Work>,
    wake: mpsc::Sender<Work>,
    snapshot: Snapshot,
    setup: impl FnOnce(&Path, WatchCallback) -> notify::Result<RecommendedWatcher> + Send + 'static,
) {
    let _active = ActiveGuard(identity.active.clone());
    let root = match fs::canonicalize(&identity.directory) {
        Ok(root) if root.is_dir() => root,
        result => {
            identity.emit(
                Vec::new(),
                false,
                false,
                Some(format!(
                    "Could not watch workspace: {}",
                    result
                        .err()
                        .map(|error| error.to_string())
                        .unwrap_or_else(|| "not a directory".into())
                )),
            );
            return;
        }
    };
    let pending = Arc::new(Mutex::new(Pending::default()));
    let callback_pending = pending.clone();
    let callback_root = root.clone();
    let callback_active = identity.active.clone();
    let waking = Arc::new(AtomicBool::new(false));
    let callback_waking = waking.clone();
    let ready = wake.clone();
    let watch_root = root.clone();
    let startup = std::thread::Builder::new()
        .name("vela-file-watch-start".into())
        .spawn(move || {
            let result = setup(
                &watch_root,
                Box::new(move |event| {
                    if !callback_active.load(Ordering::Acquire) {
                        return;
                    }
                    callback_pending
                        .lock()
                        .expect("watch event queue")
                        .event(&callback_root, event);
                    if !callback_waking.swap(true, Ordering::AcqRel) {
                        let _ = wake.send(Work::Wake);
                    }
                }),
            )
            .map_err(|error| format!("Live file updates are unavailable: {error}"));
            // If this generation closed during setup, send failure drops the OS
            // watcher here, off the actor and file-job threads.
            let _ = ready.send(Work::WatcherReady(result));
        });
    let error = startup
        .err()
        .map(|error| format!("Live file updates are unavailable: {error}"));
    let mut watcher = None;
    let mut workspace = Workspace {
        root,
        identity,
        entries: HashMap::new(),
        baselines: HashMap::new(),
        observed: HashMap::new(),
        snapshot,
        has_git: None,
    };
    workspace.identity.emit(Vec::new(), false, false, error);
    while workspace.identity.active.load(Ordering::Acquire) {
        let wait = pending.lock().expect("watch event queue").wait();
        match rx.recv_timeout(wait) {
            Ok(Work::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(Work::Wake) => {
                waking.store(false, Ordering::Release);
            }
            Ok(Work::WatcherReady(result)) => match result {
                Ok(ready) => {
                    watcher = Some(ready);
                    // Reconcile only already-observed files read during startup.
                    // Scanning the entire root here invents changes for untouched
                    // files with unknown baselines as soon as a workspace opens.
                    let mut pending = pending.lock().expect("watch event queue");
                    let now = Instant::now();
                    for rel in workspace.entries.keys() {
                        pending.paths.insert(workspace.root.join(rel), now);
                    }
                    drop(pending);
                    workspace.identity.emit(Vec::new(), true, false, None);
                }
                Err(error) => workspace
                    .identity
                    .emit(Vec::new(), false, false, Some(error)),
            },
            Ok(Work::Job(job)) => {
                let update = job(&mut workspace);
                let _ = workspace.identity.updates.send(update);
            }
            Ok(Work::CaptureBaselines(paths)) => workspace.capture_baselines(&paths),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        workspace.flush(&pending);
    }
    drop(watcher);
}

#[derive(Clone)]
struct Entry {
    content: Option<Arc<str>>,
    identity: Option<(u64, u64)>,
    deleted: bool,
}

pub(crate) struct Workspace {
    root: PathBuf,
    identity: Identity,
    entries: HashMap<String, Entry>,
    baselines: HashMap<String, Baseline>,
    observed: HashMap<String, ObservedFile>,
    snapshot: Snapshot,
    has_git: Option<bool>,
}
impl Workspace {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn import_external(
        &mut self,
        parent: &str,
        sources: &[PathBuf],
    ) -> files::Result<Vec<crate::imports::Outcome>> {
        let outcomes = crate::imports::copy(&self.root, parent, sources)?;
        let mut changes = Vec::new();
        for rel in outcomes
            .iter()
            .filter(|outcome| outcome.imported)
            .flat_map(|outcome| &outcome.files)
        {
            let Ok(path) = files::confine(&self.root, rel) else {
                continue;
            };
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            if metadata.len() > MAX_WATCH_BYTES {
                continue;
            }
            let Ok(bytes) = fs::read(path) else {
                continue;
            };
            if bytes.len() as u64 > MAX_WATCH_BYTES {
                continue;
            }
            let Ok(content) = String::from_utf8(bytes) else {
                continue;
            };
            let content: Arc<str> = Arc::from(content);
            self.baselines.insert(
                rel.clone(),
                Baseline::Known {
                    content: content.clone(),
                    existed: true,
                },
            );
            self.entries.insert(
                rel.clone(),
                Entry {
                    content: Some(content),
                    identity: file_identity(&metadata),
                    deleted: false,
                },
            );
            changes.push(self.change(rel, None, None));
        }
        self.publish(changes, true, true, None);
        Ok(outcomes)
    }
    pub fn capture_baselines(&mut self, inputs: &[String]) {
        for input in inputs {
            let path = Path::new(input);
            let absolute = if path.is_absolute() {
                path.to_owned()
            } else {
                self.root.join(path)
            };
            let Some(rel) = absolute.strip_prefix(&self.root).ok().and_then(Path::to_str) else {
                continue;
            };
            if rel.is_empty()
                || !files::is_safe_rel(rel)
                || self.baselines.contains_key(rel)
                || skipped(Path::new(rel))
            {
                continue;
            }
            let Ok(path) = files::confine(&self.root, rel) else {
                continue;
            };
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            if !metadata.is_file() || metadata.len() > MAX_WATCH_BYTES {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            if bytes.len() as u64 > MAX_WATCH_BYTES {
                continue;
            }
            let Ok(content) = String::from_utf8(bytes) else {
                continue;
            };
            self.baselines.insert(
                rel.to_owned(),
                Baseline::Known {
                    content: Arc::from(content),
                    existed: true,
                },
            );
        }
    }
    pub fn read_file(&mut self, rel: &str) -> files::Result<String> {
        let content = files::read_file(&self.root, rel)?;
        let metadata = files::confine(&self.root, rel)
            .and_then(|path| fs::metadata(path).map_err(Into::into))
            .ok();
        self.entries.insert(
            rel.to_owned(),
            Entry {
                content: Some(Arc::from(content.as_str())),
                identity: metadata.as_ref().and_then(file_identity),
                deleted: false,
            },
        );
        Ok(content)
    }
    pub fn remove_entry(&mut self, rel: &str) -> files::Result<()> {
        self.remove_entry_with(rel, crate::trash::entry)
    }
    pub fn detach_entry(&mut self, rel: &str) -> files::Result<PathBuf> {
        self.detach_entry_with(rel, crate::detach::entry)
    }
    fn detach_entry_with(
        &mut self,
        rel: &str,
        detach: impl FnOnce(&Path, &str) -> files::Result<PathBuf>,
    ) -> files::Result<PathBuf> {
        let target = detach(&self.root, rel)?;
        let keys = self
            .entries
            .keys()
            .filter(|key| {
                key.as_str() == rel
                    || key
                        .strip_prefix(rel)
                        .is_some_and(|tail| tail.starts_with('/'))
            })
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            self.baselines.remove(&key);
            self.entries.insert(
                key.clone(),
                Entry {
                    content: None,
                    identity: None,
                    deleted: true,
                },
            );
            self.change(&key, None, None);
        }
        let mut all = self.observed.values().cloned().collect::<Vec<_>>();
        all.sort_by(|a, b| a.rel.cmp(&b.rel));
        *self.snapshot.write().expect("observation snapshot") = Arc::new(all);
        self.identity.emit(Vec::new(), true, true, None);
        Ok(target)
    }
    fn remove_entry_with(
        &mut self,
        rel: &str,
        trash: impl FnOnce(&Path, &str) -> files::Result<()>,
    ) -> files::Result<()> {
        let keys = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                !entry.deleted
                    && (key.as_str() == rel
                        || key
                            .strip_prefix(rel)
                            .is_some_and(|tail| tail.starts_with('/')))
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in &keys {
            if !self.baselines.contains_key(key) {
                let baseline = self.observed_baseline(key);
                self.baselines.insert(key.clone(), baseline);
            }
        }
        trash(&self.root, rel)?;
        for key in keys {
            self.entries.insert(
                key.clone(),
                Entry {
                    content: None,
                    identity: None,
                    deleted: true,
                },
            );
            self.change(&key, None, None);
        }
        let mut all = self.observed.values().cloned().collect::<Vec<_>>();
        all.sort_by(|a, b| a.rel.cmp(&b.rel));
        *self.snapshot.write().expect("observation snapshot") = Arc::new(all);
        self.identity.emit(Vec::new(), true, true, None);
        Ok(())
    }
    #[cfg(test)]
    pub fn rename_entry(
        &mut self,
        rel: &str,
        name: &str,
    ) -> files::Result<(String, Vec<files::RenameSnapshot>)> {
        self.relocate_entry(rel, name, None)
    }
    pub fn relocate_entry(
        &mut self,
        rel: &str,
        name: &str,
        parent: Option<&str>,
    ) -> files::Result<(String, Vec<files::RenameSnapshot>)> {
        let keys = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                !entry.deleted
                    && (key.as_str() == rel
                        || key
                            .strip_prefix(rel)
                            .is_some_and(|tail| tail.starts_with('/')))
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in &keys {
            if !self.baselines.contains_key(key) {
                let baseline = self.observed_baseline(key);
                self.baselines.insert(key.clone(), baseline);
            }
        }
        let new_rel = match parent {
            Some(parent) => crate::recovery::move_entry(&self.root, rel, parent)?,
            None => crate::recovery::rename_entry(&self.root, rel, name)?,
        };
        if new_rel == rel {
            return Ok((new_rel, Vec::new()));
        }
        let mut snapshots = Vec::new();
        for old in keys {
            let target = format!("{new_rel}{}", &old[rel.len()..]);
            let before = self
                .entries
                .get(&old)
                .cloned()
                .expect("captured rename source");
            let result = files::read_file(&self.root, &target);
            let metadata = files::confine(&self.root, &target)
                .and_then(|path| fs::metadata(path).map_err(Into::into))
                .ok();
            let identity = metadata.as_ref().and_then(file_identity);
            let deleted = matches!(&result,Err(files::FileError::Io(error)) if error.kind()==std::io::ErrorKind::NotFound);
            let change = vela_core::document::ExternalChange {
                content: result.as_ref().ok().cloned(),
                deleted,
                read_error: if deleted {
                    None
                } else {
                    result.as_ref().err().map(|error| error.to_string())
                },
                merging: false,
            };
            let source_unchanged = !before.deleted
                && before.identity.is_some()
                && before.identity == identity
                && before.content.as_deref() == change.content.as_deref();
            self.entries.insert(
                old.clone(),
                Entry {
                    content: None,
                    identity: None,
                    deleted: true,
                },
            );
            self.entries.insert(
                target.clone(),
                Entry {
                    content: change.content.as_ref().map(|text| Arc::from(text.as_str())),
                    identity,
                    deleted,
                },
            );
            if let Some(baseline) = self.baselines.get(&old).cloned() {
                self.baselines.insert(target.clone(), baseline);
            }
            self.change(&old, None, None);
            self.change(&target, Some(old), None);
            snapshots.push(files::RenameSnapshot {
                rel: target,
                change,
                source_unchanged,
            });
        }
        // Update the observer before any queued OS notification can run. The
        // request ACK carries these identified snapshots; broadcasting them as
        // arbitrary external writes would falsely conflict with dirty buffers.
        let mut all = self.observed.values().cloned().collect::<Vec<_>>();
        all.sort_by(|a, b| a.rel.cmp(&b.rel));
        *self.snapshot.write().expect("observation snapshot") = Arc::new(all);
        self.identity.emit(Vec::new(), true, true, None);
        Ok((new_rel, snapshots))
    }

    pub fn write_file(
        &mut self,
        rel: &str,
        content: &str,
        expected: &str,
        overwrite: bool,
        write_id: u64,
    ) -> files::Result<()> {
        let source =
            crate::recovery::write_file_with_source(&self.root, rel, content, expected, overwrite)?;
        self.baselines
            .entry(rel.to_owned())
            .or_insert_with(|| match source.content {
                Some(content) => Baseline::Known {
                    content: Arc::from(content),
                    existed: source.existed,
                },
                None => Baseline::Unknown,
            });
        let identity = files::confine(&self.root, rel)
            .and_then(|path| fs::metadata(path).map_err(Into::into))
            .ok()
            .as_ref()
            .and_then(file_identity);
        self.entries.insert(
            rel.to_owned(),
            Entry {
                content: Some(Arc::from(content)),
                identity,
                deleted: false,
            },
        );
        let change = self.change(rel, None, Some(write_id));
        self.publish(vec![change], true, false, None);
        Ok(())
    }
    fn change(
        &mut self,
        rel: &str,
        moved_from: Option<String>,
        write_id: Option<u64>,
    ) -> ObservedFile {
        let entry = self.entries.get(rel).expect("observed file");
        let change = ObservedFile {
            rel: rel.to_owned(),
            content: entry.content.clone(),
            baseline: self
                .baselines
                .get(rel)
                .cloned()
                .unwrap_or(Baseline::Unknown),
            deleted: entry.deleted,
            moved_from,
            write_id,
        };
        self.observed.insert(rel.to_owned(), change.clone());
        change
    }
    fn publish(&self, changes: Vec<ObservedFile>, tree: bool, git: bool, error: Option<String>) {
        if !changes.is_empty() {
            let mut all = self.observed.values().cloned().collect::<Vec<_>>();
            all.sort_by(|a, b| a.rel.cmp(&b.rel));
            *self.snapshot.write().expect("observation snapshot") = Arc::new(all);
        }
        self.identity.emit(changes, tree, git, error);
    }
    fn observed_baseline(&mut self, rel: &str) -> Baseline {
        let has_git = *self
            .has_git
            .get_or_insert_with(|| crate::git::is_worktree(&self.root));
        if !has_git {
            return Baseline::Unknown;
        }
        match crate::git::head_file(&self.root, rel) {
            Some(content) => Baseline::Known {
                content: Arc::from(content),
                existed: true,
            },
            None => Baseline::Known {
                content: Arc::from(""),
                existed: false,
            },
        }
    }
    fn flush(&mut self, pending: &Mutex<Pending>) {
        let now = Instant::now();
        let (mut paths, rescan, error) = {
            let mut pending = pending.lock().expect("watch event queue");
            let paths = pending
                .paths
                .iter()
                .filter(|(_, deadline)| **deadline <= now)
                .map(|(path, _)| path.clone())
                .collect::<Vec<_>>();
            for path in &paths {
                pending.paths.remove(path);
            }
            let rescan = pending.rescan.is_some_and(|deadline| deadline <= now);
            let error = if rescan {
                pending.rescan = None;
                pending.error.take()
            } else {
                None
            };
            (paths, rescan, error)
        };
        if paths.is_empty() && !rescan {
            return;
        }
        let git_changed = paths
            .iter()
            .any(|path| path.strip_prefix(&self.root).is_ok_and(is_git));
        let mut changed_baselines = HashSet::new();
        if git_changed {
            self.has_git = None;
            for rel in self.baselines.keys().cloned().collect::<Vec<_>>() {
                let baseline = self.observed_baseline(&rel);
                if self.baselines.get(&rel) != Some(&baseline) {
                    changed_baselines.insert(rel.clone());
                }
                self.baselines.insert(rel, baseline);
            }
            paths.extend(self.entries.keys().map(|rel| self.root.join(rel)));
        }
        if rescan {
            paths.push(self.root.clone());
            paths.extend(self.entries.keys().map(|rel| self.root.join(rel)));
        }
        let mut files = HashSet::new();
        for path in paths {
            if path.strip_prefix(&self.root).is_err() {
                continue;
            }
            self.collect(&path, &mut files);
        }
        let mut files = files.into_iter().collect::<Vec<_>>();
        // Destinations must be correlated while the old opened identities are
        // still present. Missing source paths are processed after destinations.
        files.sort_by_key(|path| !path.is_file());
        let mut changes = Vec::new();
        let mut moved = HashSet::new();
        for path in files {
            let Some(rel) = path
                .strip_prefix(&self.root)
                .ok()
                .and_then(|rel| rel.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            if rel.is_empty()
                || skipped(Path::new(&rel))
                || !files::is_safe_rel(&rel)
                || moved.contains(&rel)
            {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).ok();
            if metadata
                .as_ref()
                .is_some_and(|meta| meta.file_type().is_symlink())
            {
                continue;
            }
            let entry = if let Some(meta) = metadata.filter(|meta| meta.is_file()) {
                if meta.len() > MAX_WATCH_BYTES {
                    continue;
                }
                let Ok(path) = files::confine(&self.root, &rel) else {
                    continue;
                };
                let Ok(bytes) = fs::read(path) else {
                    continue;
                };
                if bytes.len() as u64 > MAX_WATCH_BYTES {
                    continue;
                }
                let Ok(content) = String::from_utf8(bytes) else {
                    continue;
                };
                Entry {
                    content: Some(Arc::from(content)),
                    identity: file_identity(&meta),
                    deleted: false,
                }
            } else if self.entries.contains_key(&rel) {
                Entry {
                    content: None,
                    identity: self.entries.get(&rel).and_then(|entry| entry.identity),
                    deleted: true,
                }
            } else {
                continue;
            };
            let moved_from = entry.identity.and_then(|identity| {
                let missing = self
                    .entries
                    .iter()
                    .filter(|(old, old_entry)| {
                        *old != &rel
                            && old_entry.identity == Some(identity)
                            && !self.root.join(old).exists()
                    })
                    .map(|(old, _)| old.clone())
                    .collect::<Vec<_>>();
                (missing.len() == 1).then(|| missing[0].clone())
            });
            let unchanged = self
                .entries
                .get(&rel)
                .is_some_and(|old| old.content == entry.content && old.deleted == entry.deleted);
            self.entries.insert(rel.clone(), entry);
            if unchanged && moved_from.is_none() && !changed_baselines.contains(&rel) {
                continue;
            }
            if !self.baselines.contains_key(&rel) {
                let baseline = self.observed_baseline(&rel);
                self.baselines.insert(rel.clone(), baseline);
            }
            if let Some(old) = &moved_from {
                if !self.baselines.contains_key(old) {
                    let baseline = self.observed_baseline(old);
                    self.baselines.insert(old.clone(), baseline);
                }
                self.entries.insert(
                    old.clone(),
                    Entry {
                        content: None,
                        identity: None,
                        deleted: true,
                    },
                );
                changes.push(self.change(old, None, None));
                moved.insert(old.clone());
            }
            changes.push(self.change(&rel, moved_from, None));
        }
        self.publish(changes, true, git_changed, error);
    }
    fn collect(&self, path: &Path, files: &mut HashSet<PathBuf>) {
        let Ok(rel) = path.strip_prefix(&self.root) else {
            return;
        };
        if skipped(rel) {
            return;
        }
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.is_dir() => {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        self.collect(&entry.path(), files);
                    }
                }
            }
            Ok(meta) if meta.file_type().is_symlink() => {}
            _ => {
                files.insert(path.to_owned());
            }
        }
        // A removed/renamed directory also invalidates its opened descendants.
        if !path.exists() {
            for old in self.entries.keys() {
                let candidate = self.root.join(old);
                if candidate.starts_with(path) {
                    files.insert(candidate);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[test]
    fn file_jobs_do_not_wait_for_os_watcher_startup_or_failure() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file.txt"), "available before watcher").unwrap();
        let (updates, mut notifications) = broadcast::channel(32);
        let identity = Identity {
            directory: root.path().to_string_lossy().into_owned(),
            generation: 1,
            active: Arc::new(AtomicBool::new(true)),
            updates,
        };
        let (tx, rx) = mpsc::channel();
        let wake = tx.clone();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            run_with_setup(
                identity,
                rx,
                wake,
                Arc::new(RwLock::new(Arc::new(Vec::new()))),
                move |_, _| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Err(notify::Error::generic("held watcher unavailable"))
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let read = |tx: &mpsc::Sender<Work>| {
            let (reply, result) = mpsc::channel();
            tx.send(Work::Job(Box::new(move |workspace| {
                reply.send(workspace.read_file("file.txt")).unwrap();
                EngineUpdate::Sessions
            })))
            .unwrap();
            result.recv_timeout(Duration::from_secs(2))
        };
        let before_ready = read(&tx);
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut failure_reported = false;
        while Instant::now() < deadline {
            if let Ok(EngineUpdate::WorkspaceFiles {
                error: Some(error), ..
            }) = notifications.try_recv()
                && error.contains("held watcher unavailable")
            {
                failure_reported = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let after_failure = read(&tx);
        tx.send(Work::Stop).unwrap();
        worker.join().unwrap();
        assert!(
            failure_reported,
            "watcher failure must be reported without stopping file jobs"
        );
        assert_eq!(before_ready.unwrap().unwrap(), "available before watcher");
        assert_eq!(after_failure.unwrap().unwrap(), "available before watcher");
    }

    #[tokio::test]
    async fn watcher_readiness_reconciles_edits_that_happened_during_startup() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file.txt");
        fs::write(&path, "before").unwrap();
        fs::write(root.path().join("untouched.txt"), "must not appear as changed").unwrap();
        let (updates, mut notifications) = broadcast::channel(32);
        let identity = Identity {
            directory: root.path().to_string_lossy().into_owned(),
            generation: 1,
            active: Arc::new(AtomicBool::new(true)),
            updates,
        };
        let (tx, rx) = mpsc::channel();
        let wake = tx.clone();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            run_with_setup(
                identity,
                rx,
                wake,
                Arc::new(RwLock::new(Arc::new(Vec::new()))),
                move |root, callback| {
                    release_rx.recv().unwrap();
                    let mut watcher = RecommendedWatcher::new(
                        callback,
                        Config::default().with_follow_symlinks(false),
                    )?;
                    watcher.watch(root, RecursiveMode::Recursive)?;
                    Ok(watcher)
                },
            )
        });
        let (reply, read) = mpsc::channel();
        tx.send(Work::Job(Box::new(move |workspace| {
            reply.send(workspace.read_file("file.txt")).unwrap();
            EngineUpdate::Sessions
        })))
        .unwrap();
        let initial = read.recv_timeout(Duration::from_secs(2));
        fs::write(&path, "changed before watching").unwrap();
        release_tx.send(()).unwrap();
        // OS watcher startup may take several seconds; the independent read
        // above still has its strict two-second deadline while setup is held.
        // Full-suite concurrency can delay watcher startup beyond 15s; keep the
        // bound generous so slow setup is not mistaken for a lost event.
        let reconciled = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let update = notifications.recv().await.unwrap();
                if let EngineUpdate::WorkspaceFiles { changes, error, .. } = &update {
                    assert!(error.is_none(), "watch startup error: {error:?}");
                    assert!(changes.iter().all(|change| change.rel != "untouched.txt"), "startup must not invent edits to untouched files");
                }
                if let EngineUpdate::WorkspaceFiles { changes, .. } = update
                    && changes.iter().any(|change| {
                        change.rel == "file.txt"
                            && change.content.as_deref() == Some("changed before watching")
                    })
                {
                    break;
                }
            }
        })
        .await;
        tx.send(Work::Stop).unwrap();
        worker.join().unwrap();
        assert_eq!(initial.unwrap().unwrap(), "before");
        reconciled.expect("startup-gap file change must be reconciled without an OS event");
    }

    fn workspace(root: &Path) -> (Workspace, broadcast::Receiver<EngineUpdate>) {
        let root = root.canonicalize().unwrap();
        let (updates, rx) = broadcast::channel(128);
        (
            Workspace {
                root: root.clone(),
                identity: Identity {
                    directory: root.to_string_lossy().into_owned(),
                    generation: 1,
                    active: Arc::new(AtomicBool::new(true)),
                    updates,
                },
                entries: HashMap::new(),
                baselines: HashMap::new(),
                observed: HashMap::new(),
                snapshot: Arc::new(RwLock::new(Arc::new(Vec::new()))),
                has_git: None,
            },
            rx,
        )
    }
    fn flush(workspace: &mut Workspace, paths: &[&str]) {
        let pending = Pending {
            paths: paths
                .iter()
                .map(|path| (workspace.root.join(path), Instant::now()))
                .collect(),
            ..Default::default()
        };
        workspace.flush(&Mutex::new(pending));
    }

    #[test]
    fn captured_tool_input_paths_become_known_baselines_before_the_first_write() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("app.ts"), "before tool").unwrap();
        let (mut workspace, _updates) = workspace(root.path());
        workspace.capture_baselines(&[
            "app.ts".into(),
            root.path().join("app.ts").to_string_lossy().into_owned(),
            root.path().join("app.ts").to_string_lossy().into_owned(),
            "../outside.ts".into(),
        ]);
        fs::write(root.path().join("app.ts"), "after tool").unwrap();
        flush(&mut workspace, &["app.ts"]);
        let change = workspace
            .snapshot
            .read()
            .unwrap()
            .iter()
            .find(|change| change.rel == "app.ts")
            .cloned()
            .expect("tool edit is observed");
        assert_eq!(change.content.as_deref(), Some("after tool"));
        assert_eq!(
            change.baseline,
            Baseline::Known {
                content: Arc::from("before tool"),
                existed: true,
            }
        );
    }

    #[test]
    fn captured_tool_input_paths_do_not_replace_an_existing_baseline() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("app.ts"), "current tool input").unwrap();
        let (mut workspace, _updates) = workspace(root.path());
        workspace.baselines.insert(
            "app.ts".into(),
            Baseline::Known {
                content: Arc::from("original baseline"),
                existed: true,
            },
        );
        workspace.capture_baselines(&["app.ts".into()]);
        fs::write(root.path().join("app.ts"), "after tool").unwrap();
        flush(&mut workspace, &["app.ts"]);
        let change = workspace
            .snapshot
            .read()
            .unwrap()
            .iter()
            .find(|change| change.rel == "app.ts")
            .cloned()
            .expect("tool edit is observed");
        assert_eq!(
            change.baseline,
            Baseline::Known {
                content: Arc::from("original baseline"),
                existed: true,
            }
        );
    }

    #[test]
    fn imported_text_becomes_its_baseline_and_owned_watcher_echo_is_suppressed() {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("import.txt"), "imported baseline").unwrap();
        let (mut workspace, mut updates) = workspace(root.path());
        let outcomes = workspace
            .import_external("", &[source.path().join("import.txt")])
            .unwrap();
        assert!(outcomes[0].imported);
        let change = workspace.snapshot.read().unwrap()[0].clone();
        assert_eq!(change.content.as_deref(), Some("imported baseline"));
        assert_eq!(
            change.baseline,
            Baseline::Known {
                content: Arc::from("imported baseline"),
                existed: true
            }
        );
        while updates.try_recv().is_ok() {}
        flush(&mut workspace, &["import.txt"]);
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::WorkspaceFiles { changes, .. } = update {
                assert!(
                    changes.is_empty(),
                    "owned import echo must not become a new external edit"
                );
            }
        }
        fs::write(root.path().join("import.txt"), "later external edit").unwrap();
        flush(&mut workspace, &["import.txt"]);
        assert!(updates.try_recv().is_ok());
        assert_eq!(
            workspace.snapshot.read().unwrap()[0].content.as_deref(),
            Some("later external edit")
        );
    }

    #[test]
    fn detaching_a_folder_forgets_its_baseline_and_suppresses_owned_delete_echoes() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("folder")).unwrap();
        fs::write(root.path().join("folder/child"), "retain").unwrap();
        let (mut observer, mut updates) = workspace(root.path());
        observer.read_file("folder/child").unwrap();
        observer.baselines.insert(
            "folder/child".into(),
            Baseline::Known {
                content: Arc::from("before"),
                existed: true,
            },
        );
        let target = outside.path().join("retained");
        observer
            .detach_entry_with("folder", |root, rel| {
                fs::rename(root.join(rel), &target)?;
                Ok(target.clone())
            })
            .unwrap();
        assert_eq!(fs::read_to_string(target.join("child")).unwrap(), "retain");
        assert!(observer.snapshot.read().unwrap()[0].deleted);
        assert_eq!(
            observer.snapshot.read().unwrap()[0].baseline,
            Baseline::Unknown
        );
        while updates.try_recv().is_ok() {}
        flush(&mut observer, &["folder/child"]);
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::WorkspaceFiles { changes, .. } = update {
                assert!(changes.is_empty());
            }
        }
    }
    fn git(root: &Path, args: &[&str]) {
        let result = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "user.name=Vela fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&result.stderr)
        );
    }
    #[test]
    fn owned_removal_records_deleted_children_without_echo_and_keeps_baselines_for_recreation() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let trash = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.join("folder/sub")).unwrap();
        fs::write(root.join("folder/sub/file.txt"), "baseline").unwrap();
        git(root, &["init", "-q"]);
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "baseline"]);
        let (mut observer, mut updates) = workspace(root);
        observer.read_file("folder/sub/file.txt").unwrap();
        observer
            .remove_entry_with("folder", |root, rel| {
                fs::rename(root.join(rel), trash.path().join("retained-folder"))?;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            fs::read_to_string(trash.path().join("retained-folder/sub/file.txt")).unwrap(),
            "baseline"
        );
        flush(&mut observer, &["folder/sub/file.txt"]);
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::WorkspaceFiles { changes, .. } = update {
                assert!(changes.is_empty());
            }
        }
        assert!(observer.entries["folder/sub/file.txt"].deleted);
        fs::create_dir_all(root.join("folder/sub")).unwrap();
        fs::write(root.join("folder/sub/file.txt"), "recreated").unwrap();
        flush(&mut observer, &["folder/sub/file.txt"]);
        assert!(!observer.entries["folder/sub/file.txt"].deleted);
        assert_eq!(
            observer.entries["folder/sub/file.txt"].content.as_deref(),
            Some("recreated")
        );
        assert!(matches!(
            observer.observed["folder/sub/file.txt"].baseline,
            Baseline::Known { .. }
        ));
    }

    #[test]
    fn owned_folder_move_receipts_rebase_tracked_children_and_ignore_os_echoes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("before/sub")).unwrap();
        fs::create_dir(root.join("destination")).unwrap();
        fs::write(root.join("before/sub/open.txt"), "baseline").unwrap();
        let (mut observer, mut updates) = workspace(root);
        observer.read_file("before/sub/open.txt").unwrap();
        let (new, snapshots) = observer
            .relocate_entry("before", "before", Some("destination"))
            .unwrap();
        assert_eq!(new, "destination/before");
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].rel, "destination/before/sub/open.txt");
        assert!(snapshots[0].source_unchanged);
        assert_eq!(snapshots[0].change.content.as_deref(), Some("baseline"));
        flush(
            &mut observer,
            &["before/sub/open.txt", "destination/before/sub/open.txt"],
        );
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::WorkspaceFiles { changes, .. } = update {
                assert!(changes.is_empty());
            }
        }
        assert!(observer.entries["before/sub/open.txt"].deleted);
        assert_eq!(
            observer.entries["destination/before/sub/open.txt"]
                .content
                .as_deref(),
            Some("baseline")
        );
    }

    #[test]
    fn owned_rename_receipts_preserve_identity_and_suppress_later_notification_echoes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join("before.txt"), "baseline").unwrap();
        let (mut observer, mut updates) = workspace(root);
        observer.read_file("before.txt").unwrap();
        let (new, snapshots) = observer.rename_entry("before.txt", "after.txt").unwrap();
        assert_eq!(new, "after.txt");
        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].source_unchanged);
        assert_eq!(snapshots[0].change.content.as_deref(), Some("baseline"));
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::WorkspaceFiles { changes, .. } = update {
                assert!(
                    changes.is_empty(),
                    "an owned rename must not masquerade as an external write"
                );
            }
        }
        flush(&mut observer, &["before.txt", "after.txt"]);
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::WorkspaceFiles { changes, .. } = update {
                assert!(
                    changes.is_empty(),
                    "OS rename echoes must not generate a second write"
                );
            }
        }
        assert_eq!(
            observer.entries["after.txt"].content.as_deref(),
            Some("baseline")
        );
        fs::write(root.join("after.txt"), "real external change").unwrap();
        let (_, snapshots) = observer.rename_entry("after.txt", "last.txt").unwrap();
        assert!(!snapshots[0].source_unchanged);
        assert_eq!(
            snapshots[0].change.content.as_deref(),
            Some("real external change")
        );
    }

    #[test]
    fn observations_preserve_known_and_unknown_baselines_refresh_git_and_ignore_unsafe_or_large_files()
     {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("opened.txt"), "opened before").unwrap();
        fs::write(root.join("saved.txt"), "saved before").unwrap();
        let (mut observer, _) = workspace(root);
        observer.read_file("opened.txt").unwrap();
        fs::write(root.join("opened.txt"), "external").unwrap();
        flush(&mut observer, &["opened.txt"]);
        assert_eq!(observer.observed["opened.txt"].baseline, Baseline::Unknown);
        observer
            .write_file("saved.txt", "saved after", "saved before", false, 42)
            .unwrap();
        assert_eq!(observer.observed["saved.txt"].write_id, Some(42));
        fs::write(root.join("saved.txt"), "external after save").unwrap();
        flush(&mut observer, &["saved.txt"]);
        assert_eq!(
            observer.observed["saved.txt"].baseline,
            Baseline::Known {
                content: Arc::from("saved before"),
                existed: true
            }
        );
        assert_eq!(observer.observed["saved.txt"].write_id, None);
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/ignored.txt"), "ignored").unwrap();
        fs::write(
            root.join("large.txt"),
            vec![b'x'; MAX_WATCH_BYTES as usize + 1],
        )
        .unwrap();
        fs::write(root.join("binary.bin"), [0xff, 0xfe]).unwrap();
        flush(
            &mut observer,
            &["node_modules/ignored.txt", "large.txt", "binary.bin"],
        );
        assert_eq!(observer.observed.len(), 2);
        git(root, &["init", "--quiet"]);
        git(root, &["add", "opened.txt"]);
        git(root, &["commit", "--quiet", "-m", "fixture baseline"]);
        flush(&mut observer, &[".git/HEAD"]);
        assert_eq!(
            observer.observed["opened.txt"].baseline,
            Baseline::Known {
                content: Arc::from("external"),
                existed: true
            }
        );
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/unicode 🌍.txt"), "nested baseline").unwrap();
        git(root, &["add", "nested"]);
        git(root, &["commit", "--quiet", "-m", "nested fixture"]);
        assert_eq!(
            crate::git::head_file(&root.join("nested"), "unicode 🌍.txt").as_deref(),
            Some("nested baseline")
        );
        fs::write(root.join("new.txt"), "new").unwrap();
        flush(&mut observer, &["new.txt"]);
        assert_eq!(
            observer.observed["new.txt"].baseline,
            Baseline::Known {
                content: Arc::from(""),
                existed: false
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn rename_correlation_survives_separate_batches_and_rejects_ambiguous_hardlinks_and_symlinks() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("before.txt"), "same inode").unwrap();
        let (mut observer, _) = workspace(root);
        observer.read_file("before.txt").unwrap();
        fs::rename(root.join("before.txt"), root.join("after.txt")).unwrap();
        flush(&mut observer, &["before.txt"]);
        assert!(observer.observed["before.txt"].deleted);
        flush(&mut observer, &["after.txt"]);
        assert_eq!(
            observer.observed["after.txt"].moved_from.as_deref(),
            Some("before.txt")
        );
        fs::write(root.join("a.txt"), "hardlinked").unwrap();
        fs::hard_link(root.join("a.txt"), root.join("b.txt")).unwrap();
        fs::hard_link(root.join("a.txt"), root.join("destination.txt")).unwrap();
        observer.read_file("a.txt").unwrap();
        observer.read_file("b.txt").unwrap();
        fs::remove_file(root.join("a.txt")).unwrap();
        fs::remove_file(root.join("b.txt")).unwrap();
        flush(&mut observer, &["destination.txt"]);
        assert!(observer.observed["destination.txt"].moved_from.is_none());
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), "outside").unwrap();
        symlink(outside.path(), root.join("linked")).unwrap();
        flush(&mut observer, &["linked", "linked/secret.txt"]);
        assert!(!observer.observed.contains_key("linked/secret.txt"));
    }

    #[test]
    fn notification_queue_is_bounded_and_overflow_rescan_deadline_does_not_starve() {
        let root = Path::new("/workspace");
        let mut pending = Pending::default();
        for index in 0..MAX_PENDING + 1 {
            pending.event(
                root,
                Ok(Event::new(EventKind::Any).add_path(root.join(format!("{index}.txt")))),
            );
        }
        assert!(pending.paths.len() <= MAX_PENDING);
        let deadline = pending.rescan.unwrap();
        for index in 0..MAX_PENDING + 1 {
            pending.event(
                root,
                Ok(Event::new(EventKind::Any).add_path(root.join(format!("next-{index}.txt")))),
            );
        }
        assert_eq!(pending.rescan, Some(deadline));
        let count = pending.paths.len();
        pending.event(
            root,
            Ok(Event::new(EventKind::Any).add_path(PathBuf::from("/outside/secret"))),
        );
        pending.event(
            root,
            Ok(Event::new(EventKind::Any).add_path(root.join("target/ignored"))),
        );
        assert_eq!(pending.paths.len(), count);
    }

    async fn next(
        rx: &mut broadcast::Receiver<EngineUpdate>,
        mut matches: impl FnMut(&EngineUpdate) -> bool,
    ) -> EngineUpdate {
        // Native FSEvents can be delayed under concurrent test load; keep the
        // bound generous so a slow delivery is not mistaken for a lost event.
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                let update = rx.recv().await.expect("observation channel");
                if matches(&update) {
                    return update;
                }
            }
        })
        .await
        .expect("native filesystem notification did not arrive")
    }
    #[tokio::test]
    async fn native_events_follow_external_writes_save_echoes_multiple_owners_and_new_generations()
    {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let directory = root.to_string_lossy().into_owned();
        fs::write(root.join("live.txt"), "before").unwrap();
        let (updates, mut rx) = broadcast::channel(128);
        let pool = ObservationPool::new(updates);
        pool.attach("one", &directory).unwrap();
        pool.attach("two", &directory).unwrap();
        let ready = next(&mut rx, |update| {
            matches!(
                update,
                EngineUpdate::WorkspaceFiles {
                    tree_changed: false,
                    ..
                }
            )
        })
        .await;
        let EngineUpdate::WorkspaceFiles {
            generation, error, ..
        } = ready
        else {
            unreachable!()
        };
        assert!(error.is_none(), "{error:?}");
        pool.submit("one", &directory, |workspace| EngineUpdate::FileContent {
            request_id: 1,
            deleted: false,
            rel: "live.txt".into(),
            content: Some(workspace.read_file("live.txt").unwrap()),
            error: None,
        })
        .unwrap();
        next(&mut rx, |update| {
            matches!(update, EngineUpdate::FileContent { request_id: 1, .. })
        })
        .await;
        fs::write(root.join("live.txt"), "external 🌍").unwrap();
        next(&mut rx,|update|matches!(update,EngineUpdate::WorkspaceFiles {changes,..} if changes.iter().any(|file|file.rel=="live.txt" && file.content.as_deref()==Some("external 🌍") && file.write_id.is_none()))).await;
        pool.detach("one");
        assert!(pool.is_current(&directory, generation));
        pool.submit("two", &directory, |workspace| {
            workspace
                .write_file("live.txt", "native save", "external 🌍", false, 2)
                .unwrap();
            EngineUpdate::FileSaved {
                request_id: 2,
                rel: "live.txt".into(),
                error: None,
                conflict: None,
            }
        })
        .unwrap();
        next(&mut rx,|update|matches!(update,EngineUpdate::WorkspaceFiles {changes,..} if changes.iter().any(|file|file.write_id==Some(2)))).await;
        next(&mut rx, |update| {
            matches!(update, EngineUpdate::FileSaved { request_id: 2, .. })
        })
        .await;
        fs::write(root.join("live.txt"), "external after own save").unwrap();
        next(&mut rx,|update|matches!(update,EngineUpdate::WorkspaceFiles {changes,..} if changes.iter().any(|file|file.content.as_deref()==Some("external after own save") && file.write_id.is_none()))).await;
        assert_eq!(
            pool.snapshot(&directory)
                .iter()
                .find(|file| file.rel == "live.txt")
                .unwrap()
                .content
                .as_deref(),
            Some("external after own save")
        );
        pool.detach("two");
        assert!(!pool.is_current(&directory, generation));
        pool.attach("new", &directory).unwrap();
        let ready=next(&mut rx,|update|matches!(update,EngineUpdate::WorkspaceFiles {generation:current,tree_changed:false,..} if *current!=generation)).await;
        let EngineUpdate::WorkspaceFiles {
            generation: new,
            error,
            ..
        } = ready
        else {
            unreachable!()
        };
        assert!(error.is_none());
        assert!(pool.is_current(&directory, new));
        pool.close();
        assert!(!pool.is_current(&directory, new));
        assert!(pool.attach("late", &directory).is_err());
    }
}
