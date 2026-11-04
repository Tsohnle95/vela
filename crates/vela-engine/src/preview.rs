//! App-owned Vite previews. Project tools remain child processes; the app shell,
//! lifecycle, readiness checks and inventory are owned by the native engine.
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewServer {
    pub id: String,
    pub workspace_id: String,
    pub directory: String,
    pub entry: String,
    pub url: String,
    pub port: u16,
}

/// Source sorts preview inventories with `localeCompare`, which is
/// case-insensitive and locale-aware. macOS keeps that behavior through
/// Foundation; other platforms fall back to Unicode case folding. Case variants
/// of the same text are ordered lowercase-first, matching JavaScript's default
/// collation in common locales.
fn locale_primary(left: &str, right: &str) -> std::cmp::Ordering {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::NSString;
        NSString::from_str(left)
            .localizedCaseInsensitiveCompare(&NSString::from_str(right))
            .into()
    }
    #[cfg(not(target_os = "macos"))]
    {
        left.to_lowercase().cmp(&right.to_lowercase())
    }
}

fn case_tiebreak(left: &str, right: &str) -> std::cmp::Ordering {
    for (left, right) in left.chars().zip(right.chars()) {
        if left == right {
            continue;
        }
        if left.to_lowercase().eq(right.to_lowercase()) {
            return if left.is_lowercase() {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        break;
    }
    std::cmp::Ordering::Equal
}

fn preview_order(left: &str, right: &str) -> std::cmp::Ordering {
    locale_primary(left, right)
        .then_with(|| case_tiebreak(left, right))
        .then_with(|| left.cmp(right))
}

fn sort_servers(servers: &mut [PreviewServer]) {
    servers.sort_by(|left, right| {
        preview_order(&left.directory, &right.directory)
            .then_with(|| preview_order(&left.entry, &right.entry))
    });
}

#[derive(Clone)]
pub struct PreviewProgram {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
}
impl PreviewProgram {
    pub fn discover() -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let bundled = executable
            .parent()
            .and_then(Path::parent)
            .map(|path| path.join("Resources/preview"));
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let node_override = std::env::var_os("VELA_PREVIEW_NODE");
        let vite_override = std::env::var_os("VELA_PREVIEW_VITE");
        let candidates = bundled
            .into_iter()
            .map(|root| {
                (
                    root.join("node/bin/node"),
                    root.join("node_modules/vite/bin/vite.js"),
                )
            })
            .chain(std::iter::once((
                source.join(".tools/preview/node/bin/node"),
                source.join("tools/preview/node_modules/vite/bin/vite.js"),
            )));
        for (node, vite) in candidates {
            let node = node_override.as_ref().map(PathBuf::from).unwrap_or(node);
            let vite = vite_override.as_ref().map(PathBuf::from).unwrap_or(vite);
            if node.is_file() && vite.is_file() {
                return Ok(Self {
                    executable: node,
                    arguments: vec![vite.into_os_string()],
                });
            }
        }
        Err("Vela's Vite preview tools are missing. Run scripts/setup-preview-toolchain.sh and rebuild the app.".into())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Target {
    directory: PathBuf,
    entry: String,
}
fn html_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
    })
}
fn target(root: &Path, requested: Option<&str>) -> Result<Target, String> {
    let root = crate::files::confine(root, "").map_err(|error| error.to_string())?;
    let page = if let Some(requested) = requested {
        let page = crate::files::confine(&root, requested).map_err(|error| error.to_string())?;
        if !page.is_file() || !html_file(&page) {
            return Err("the Vite preview target must be an existing HTML file".into());
        }
        Some(page)
    } else if root.join("index.html").is_file() {
        None
    } else {
        find_html(&root)
    };
    match page {
        Some(page) => {
            let name = page
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("the HTML entry name is not valid Unicode")?;
            Ok(Target {
                directory: page
                    .parent()
                    .expect("a workspace file has a parent")
                    .to_owned(),
                entry: if name.eq_ignore_ascii_case("index.html") {
                    String::new()
                } else {
                    name.to_owned()
                },
            })
        }
        None => Ok(Target {
            directory: root,
            entry: String::new(),
        }),
    }
}
fn find_html(root: &Path) -> Option<PathBuf> {
    let mut fallback = None;
    let mut level = vec![root.to_owned()];
    for _ in 0..=6 {
        let mut next = Vec::new();
        for directory in level {
            let Ok(entries) = std::fs::read_dir(directory) else {
                continue;
            };
            let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
            // Match Source's case-insensitive `localeCompare` before choosing
            // the shallowest HTML fallback, so uppercase files cannot win by
            // byte order.
            entries.sort_by(|left, right| {
                preview_order(
                    &left.file_name().to_string_lossy(),
                    &right.file_name().to_string_lossy(),
                )
            });
            for entry in entries {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.') || name == "node_modules" {
                    continue;
                }
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    next.push(entry.path());
                } else if kind.is_file() && html_file(&entry.path()) {
                    if name.eq_ignore_ascii_case("index.html") {
                        return Some(entry.path());
                    }
                    if fallback.is_none() {
                        fallback = Some(entry.path());
                    }
                }
            }
        }
        if next.is_empty() {
            break;
        }
        level = next;
    }
    fallback
}

struct Lease {
    server: PreviewServer,
    generation: uuid::Uuid,
    cancel: CancellationToken,
}
#[derive(Default)]
struct State {
    servers: HashMap<String, Lease>,
    workspaces: HashMap<String, CancellationToken>,
    ports: HashSet<u16>,
}
struct Inner {
    state: Mutex<State>,
    program: Result<PreviewProgram, String>,
    cancel: CancellationToken,
    changed: Box<dyn Fn(Vec<PreviewServer>) + Send + Sync>,
    first_port: u16,
    ready_timeout: Duration,
    #[cfg(unix)]
    processes: ProcessGroups,
}
#[derive(Clone)]
pub struct PreviewManager(Arc<Inner>);
impl PreviewManager {
    pub fn new(
        program: Result<PreviewProgram, String>,
        cancel: CancellationToken,
        changed: impl Fn(Vec<PreviewServer>) + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(Inner {
            state: Mutex::default(),
            program,
            cancel,
            changed: Box::new(changed),
            first_port: 5199,
            ready_timeout: Duration::from_secs(15),
            #[cfg(unix)]
            processes: ProcessGroups::default(),
        }))
    }
    #[cfg(unix)]
    pub(crate) fn with_processes(mut self, processes: ProcessGroups) -> Self {
        Arc::get_mut(&mut self.0)
            .expect("preview manager has not been shared")
            .processes = processes;
        self
    }
    pub async fn list(&self) -> Vec<PreviewServer> {
        let state = self.0.state.lock().await;
        let mut servers = state
            .servers
            .values()
            .map(|lease| lease.server.clone())
            .collect::<Vec<_>>();
        sort_servers(&mut servers);
        servers
    }
    async fn notify(&self) {
        (self.0.changed)(self.list().await);
    }
    pub async fn stop(&self, id: &str) {
        let mut state = self.0.state.lock().await;
        if let Some(lease) = state.servers.remove(id) {
            state.ports.remove(&lease.server.port);
            lease.cancel.cancel();
        }
        drop(state);
        self.notify().await;
    }
    pub async fn stop_workspace(&self, workspace: &str) {
        let mut state = self.0.state.lock().await;
        if let Some(scope) = state.workspaces.remove(workspace) {
            scope.cancel();
        }
        let ids = state
            .servers
            .iter()
            .filter(|(_, lease)| lease.server.workspace_id == workspace)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            if let Some(lease) = state.servers.remove(&id) {
                state.ports.remove(&lease.server.port);
                lease.cancel.cancel();
            }
        }
        drop(state);
        self.notify().await;
    }
    pub async fn stop_all(&self) {
        let mut state = self.0.state.lock().await;
        for (_, scope) in state.workspaces.drain() {
            scope.cancel();
        }
        for (_, lease) in state.servers.drain() {
            lease.cancel.cancel();
        }
        state.ports.clear();
        drop(state);
        self.notify().await;
    }
    async fn remove(&self, id: &str, generation: uuid::Uuid) {
        let mut state = self.0.state.lock().await;
        if state
            .servers
            .get(id)
            .is_some_and(|lease| lease.generation == generation)
            && let Some(lease) = state.servers.remove(id)
        {
            state.ports.remove(&lease.server.port);
        }
        drop(state);
        self.notify().await;
    }
    pub async fn toggle(
        &self,
        workspace: String,
        root: PathBuf,
        requested: Option<String>,
    ) -> Result<Option<PreviewServer>, String> {
        let scope = self.workspace_scope(&workspace).await;
        self.toggle_in_scope(workspace, root, requested, scope)
            .await
    }

    pub(crate) async fn workspace_scope(&self, workspace: &str) -> CancellationToken {
        let mut state = self.0.state.lock().await;
        state
            .workspaces
            .entry(workspace.to_owned())
            .or_insert_with(|| self.0.cancel.child_token())
            .clone()
    }

    pub(crate) async fn toggle_in_scope(
        &self,
        workspace: String,
        root: PathBuf,
        requested: Option<String>,
        scope: CancellationToken,
    ) -> Result<Option<PreviewServer>, String> {
        let target = tokio::select! {
            _ = scope.cancelled() => return Err("Vite preview canceled because the workspace closed".into()),
            result = tokio::task::spawn_blocking(move || target(&root,requested.as_deref())) => result.map_err(|error|error.to_string())??,
        };
        let id = serde_json::to_string(&(&workspace, &target.directory, &target.entry))
            .map_err(|error| error.to_string())?;
        let mut state = self.0.state.lock().await;
        if scope.is_cancelled() {
            return Err("Vite preview canceled because the workspace closed".into());
        }
        if let Some(previous) = state.servers.remove(&id) {
            state.ports.remove(&previous.server.port);
            previous.cancel.cancel();
            drop(state);
            self.notify().await;
            return Ok(None);
        }
        let program = self.0.program.as_ref().map_err(Clone::clone)?;
        let mut port = None;
        for offset in 0..20 {
            let Some(candidate) = self.0.first_port.checked_add(offset) else {
                break;
            };
            if state.ports.contains(&candidate) {
                continue;
            }
            if let Ok(listener) =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, candidate)).await
            {
                port = Some(
                    listener
                        .local_addr()
                        .map_err(|error| error.to_string())?
                        .port(),
                );
                break;
            }
        }
        let port = port.ok_or("no free port for the Vite preview server")?;
        let mut command = Command::new(&program.executable);
        command
            .args(&program.arguments)
            .arg("serve")
            .arg(&target.directory)
            .args([
                "--port",
                &port.to_string(),
                "--strictPort",
                "--host",
                "127.0.0.1",
            ])
            .current_dir(&target.directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        {
            command.process_group(0);
        }
        #[cfg(unix)]
        let child = self.0.processes.spawn(&mut command);
        #[cfg(not(unix))]
        let child = command.spawn().map(OwnedChild::new);
        let mut child = child.map_err(|error| format!("Could not start Vite: {error}"))?;
        let stderr = child.child.stderr.take().expect("stderr is piped");
        let tail = Arc::new(std::sync::Mutex::new(VecDeque::<u8>::new()));
        let reader_tail = tail.clone();
        let reader = tokio::spawn(async move {
            let mut stderr = stderr;
            let mut buffer = [0; 1024];
            while let Ok(count) = stderr.read(&mut buffer).await {
                if count == 0 {
                    break;
                }
                let mut tail = reader_tail.lock().expect("preview diagnostics lock");
                tail.extend(&buffer[..count]);
                while tail.len() > 4000 {
                    tail.pop_front();
                }
            }
        });
        let mut url = url::Url::parse(&format!("http://127.0.0.1:{port}/")).expect("loopback URL");
        if !target.entry.is_empty() {
            url.path_segments_mut()
                .expect("HTTP URL has segments")
                .clear()
                .push(&target.entry);
        }
        let server = PreviewServer {
            id: id.clone(),
            workspace_id: workspace,
            directory: target.directory.to_string_lossy().into_owned(),
            entry: target.entry,
            url: url.into(),
            port,
        };
        let generation = uuid::Uuid::new_v4();
        let cancel = scope.child_token();
        state.ports.insert(port);
        state.servers.insert(
            id.clone(),
            Lease {
                server: server.clone(),
                generation,
                cancel: cancel.clone(),
            },
        );
        drop(state);
        self.notify().await;
        let ready = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err("Vite preview canceled".into()),
            status = child.child.wait() => Err(format!("Vite preview server exited{} before becoming ready", match status { Ok(status)=>format!(" with {status}"),Err(error)=>format!(": {error}") })),
            result = wait_ready(&server.url,self.0.ready_timeout) => result,
        };
        if let Err(error) = ready {
            child.terminate().await;
            finish_reader(reader).await;
            self.remove(&id, generation).await;
            let bytes = tail
                .lock()
                .expect("preview diagnostics lock")
                .iter()
                .copied()
                .collect::<Vec<_>>();
            let detail = diagnostics(&String::from_utf8_lossy(&bytes));
            return Err(if detail.is_empty() {
                error
            } else {
                format!("{error}: {detail}")
            });
        }
        let manager = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = cancel.cancelled() => child.terminate().await,
                _ = child.child.wait() => {},
            }
            drop(child);
            finish_reader(reader).await;
            manager.remove(&id, generation).await;
        });
        Ok(Some(server))
    }
}

async fn finish_reader(mut reader: tokio::task::JoinHandle<()>) {
    if tokio::time::timeout(Duration::from_millis(500), &mut reader)
        .await
        .is_err()
    {
        reader.abort();
    }
}

async fn wait_ready(address: &str, timeout: Duration) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| error.to_string())?;
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Ok(response) = client.get(address).send().await {
            let status = response.status().as_u16();
            if (200..400).contains(&status) {
                return Ok(());
            }
            let path = url::Url::parse(address)
                .map_err(|error| error.to_string())?
                .path()
                .to_owned();
            return Err(if status == 404 {
                format!(
                    "Vite is running, but no page was found at {path}. Open an HTML file and try again, or add index.html at the workspace root."
                )
            } else {
                format!("Vite returned HTTP {status} while loading {path}")
            });
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("Vite preview server did not become ready".into());
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}
fn diagnostics(value: &str) -> String {
    let mut output = String::new();
    let mut escape = false;
    for character in value.chars() {
        if character == '\u{1b}' {
            escape = true;
        } else if escape {
            if character.is_ascii_alphabetic() {
                escape = false;
            }
        } else {
            output.push(character);
        }
    }
    output.trim().to_owned()
}
struct OwnedChild {
    child: Child,
    #[cfg(unix)]
    group: i32,
    #[cfg(unix)]
    processes: ProcessGroups,
}
impl OwnedChild {
    #[cfg(not(unix))]
    fn new(child: Child) -> Self {
        Self { child }
    }
    async fn terminate(&mut self) {
        #[cfg(unix)]
        {
            // SAFETY: this is a fresh, app-owned process group established at
            // spawn; a negative ID signals that group, never the app's group.
            unsafe {
                libc::kill(-self.group, libc::SIGTERM);
            }
            if tokio::time::timeout(Duration::from_secs(2), self.child.wait())
                .await
                .is_ok()
            {
                return;
            }
        }
        let _ = self.child.kill().await;
    }
}
#[cfg(unix)]
impl Drop for OwnedChild {
    fn drop(&mut self) {
        // SAFETY: clean up any remaining app-owned Vite helpers as well when a
        // worker is canceled or the parent exits before graceful termination.
        unsafe {
            libc::kill(-self.group, libc::SIGKILL);
        }
        if let Some(groups) = self
            .processes
            .0
            .lock()
            .expect("preview process lock")
            .as_mut()
        {
            groups.remove(&self.group);
        }
    }
}

/// AppKit can exit without returning from GPUI's application loop. Keep child
/// ownership outside the async actor so its quit hook can terminate process
/// groups synchronously, within GPUI's 200ms shutdown allowance. Registration
/// and closing share one lock: a concurrent spawn cannot escape shutdown.
#[cfg(unix)]
#[derive(Clone)]
pub(crate) struct ProcessGroups(Arc<std::sync::Mutex<Option<HashSet<i32>>>>);

#[cfg(unix)]
impl Default for ProcessGroups {
    fn default() -> Self {
        Self(Arc::new(std::sync::Mutex::new(Some(HashSet::new()))))
    }
}

#[cfg(unix)]
impl ProcessGroups {
    fn spawn(&self, command: &mut Command) -> std::io::Result<OwnedChild> {
        let mut state = self.0.lock().expect("preview process lock");
        let groups = state.as_mut().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Interrupted, "Vela is shutting down")
        })?;
        let child = command.spawn()?;
        let group = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .expect("new child has a process ID");
        groups.insert(group);
        Ok(OwnedChild {
            child,
            group,
            processes: self.clone(),
        })
    }

    pub(crate) fn close(&self) {
        if let Some(groups) = self.0.lock().expect("preview process lock").take() {
            for group in groups {
                // SAFETY: every group was established by process_group(0) on
                // an app-owned child. Never signal the application's group.
                unsafe {
                    libc::kill(-group, libc::SIGKILL);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn preview_server(directory: &str, entry: &str) -> PreviewServer {
        PreviewServer {
            id: format!("{directory}:{entry}"),
            workspace_id: "ws".into(),
            directory: directory.into(),
            entry: entry.into(),
            url: "http://127.0.0.1:1/".into(),
            port: 1,
        }
    }

    #[test]
    fn preview_servers_sort_with_locale_compare_and_lowercase_case_tiebreaks() {
        assert_eq!(preview_order("a", "B"), std::cmp::Ordering::Less);
        assert_eq!(
            preview_order("Alpha", "alpha"),
            std::cmp::Ordering::Greater
        );
        let mut servers = vec![
            preview_server("/w/zeta", "z.html"),
            preview_server("/w/Alpha", "b.html"),
            preview_server("/w/alpha", "a.html"),
            preview_server("/w/beta", "a.html"),
            preview_server("/w/docs", "zeta.html"),
            preview_server("/w/docs", "Alpha.html"),
            preview_server("/w/docs", "beta.html"),
        ];
        sort_servers(&mut servers);
        assert_eq!(
            servers
                .iter()
                .map(|server| (server.directory.as_str(), server.entry.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("/w/alpha", "a.html"),
                ("/w/Alpha", "b.html"),
                ("/w/beta", "a.html"),
                ("/w/docs", "Alpha.html"),
                ("/w/docs", "beta.html"),
                ("/w/docs", "zeta.html"),
                ("/w/zeta", "z.html"),
            ]
        );
        let sorted = servers.clone();
        sort_servers(&mut servers);
        assert_eq!(servers, sorted, "sorting is stable and idempotent");
    }

    #[test]
    fn html_entry_search_uses_locale_case_insensitive_order() {
        let temp = tempfile::tempdir().unwrap();
        let canonical = temp.path().canonicalize().unwrap();
        let root = canonical.as_path();
        fs::write(root.join("Zeta.html"), "uppercase").unwrap();
        fs::write(root.join("alpha.html"), "lowercase").unwrap();
        fs::write(root.join("beta.html"), "middle").unwrap();
        assert_eq!(
            target(root, None).unwrap(),
            Target {
                directory: root.to_owned(),
                entry: "alpha.html".into()
            }
        );

        for name in ["Zeta.html", "alpha.html", "beta.html"] {
            fs::remove_file(root.join(name)).unwrap();
        }
        fs::create_dir_all(root.join("Zoo")).unwrap();
        fs::create_dir_all(root.join("alpha")).unwrap();
        fs::write(root.join("Zoo/page.html"), "uppercase directory").unwrap();
        fs::write(root.join("alpha/page.html"), "lowercase directory").unwrap();
        assert_eq!(
            target(root, None).unwrap(),
            Target {
                directory: root.join("alpha"),
                entry: "page.html".into()
            }
        );
    }

    #[test]
    fn html_targets_match_reference_search_depth_entries_and_confinement() {
        let temp = tempfile::tempdir().unwrap();
        let canonical = temp.path().canonicalize().unwrap();
        let root = canonical.as_path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join(".hidden/index.html"), "hidden").unwrap();
        fs::write(root.join("node_modules/index.html"), "dependency").unwrap();
        fs::write(root.join("about.htm"), "fallback").unwrap();
        assert_eq!(target(root, None).unwrap().entry, "about.htm");
        fs::write(root.join("docs/index.html"), "nested index").unwrap();
        assert_eq!(
            target(root, None).unwrap(),
            Target {
                directory: root.join("docs"),
                entry: String::new()
            }
        );
        fs::write(root.join("index.html"), "root index").unwrap();
        assert_eq!(
            target(root, None).unwrap(),
            Target {
                directory: root.to_owned(),
                entry: String::new()
            }
        );
        fs::write(root.join("docs/My demo 🌍.html"), "Unicode entry").unwrap();
        assert_eq!(
            target(root, Some("docs/My demo 🌍.html")).unwrap(),
            Target {
                directory: root.join("docs"),
                entry: "My demo 🌍.html".into()
            }
        );
        fs::write(root.join("text.txt"), "not HTML").unwrap();
        for invalid in [
            "../outside.html",
            "/absolute.html",
            "docs/../index.html",
            "text.txt",
            "",
        ] {
            assert!(
                target(root, Some(invalid)).is_err(),
                "must reject {invalid}"
            );
        }
        fs::remove_file(root.join("index.html")).unwrap();
        fs::remove_file(root.join("docs/index.html")).unwrap();
        let mut deepest = root.to_owned();
        for _ in 0..7 {
            deepest = deepest.join("deep");
            fs::create_dir(&deepest).unwrap();
        }
        fs::write(deepest.join("index.html"), "beyond scan depth").unwrap();
        assert_eq!(target(root, None).unwrap().entry, "about.htm");
    }

    #[cfg(unix)]
    fn fixture(root: &Path, timeout: Duration) -> PreviewManager {
        let script = root.join("fixture-server.py");
        fs::write(
            &script,
            r#"import http.server, pathlib, signal, subprocess, sys, time
root = pathlib.Path.cwd()
if (root / 'ignore-terminate').exists(): signal.signal(signal.SIGTERM, signal.SIG_IGN)
if (root / 'spawn-helper').exists():
    subprocess.Popen([sys.executable, '-c', '''import http.server, pathlib, signal
signal.signal(signal.SIGTERM, signal.SIG_IGN)
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), http.server.SimpleHTTPRequestHandler)
pathlib.Path('helper-port').write_text(str(server.server_port))
server.serve_forever()
'''], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
if (root / 'delay-start').exists(): time.sleep(30)
class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        if (root / 'reject-ready').exists(): self.send_error(404); return
        super().do_GET()
port = int(sys.argv[sys.argv.index('--port') + 1])
http.server.ThreadingHTTPServer(('127.0.0.1', port), Handler).serve_forever()
"#,
        )
        .unwrap();
        let mut manager = PreviewManager::new(
            Ok(PreviewProgram {
                executable: "/usr/bin/python3".into(),
                arguments: vec![script.into_os_string()],
            }),
            CancellationToken::new(),
            |_| {},
        );
        let inner = Arc::get_mut(&mut manager.0).unwrap();
        inner.first_port = 0;
        inner.ready_timeout = timeout;
        manager
    }

    #[cfg(unix)]
    async fn closed(port: u16) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .is_ok()
        {
            assert!(
                tokio::time::Instant::now() < deadline,
                "app-owned preview port must be released"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn previews_serve_encoded_entries_toggle_independently_and_release_ports() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("index.html"), "root preview").unwrap();
        fs::write(root.join("My demo 🌍.html"), "Unicode preview").unwrap();
        let manager = fixture(root, Duration::from_secs(3));
        let first = manager
            .toggle("ws".into(), root.to_owned(), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            reqwest::get(&first.url)
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "root preview"
        );
        let second = manager
            .toggle("ws".into(), root.to_owned(), Some("My demo 🌍.html".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(second.url.ends_with("/My%20demo%20%F0%9F%8C%8D.html"));
        assert_ne!(first.port, second.port);
        assert_eq!(
            reqwest::get(&second.url)
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "Unicode preview"
        );
        assert_eq!(manager.list().await.len(), 2);
        manager.stop(&first.id).await;
        closed(first.port).await;
        assert_eq!(manager.list().await.len(), 1);
        assert!(
            manager
                .toggle("ws".into(), root.to_owned(), Some("My demo 🌍.html".into()))
                .await
                .unwrap()
                .is_none()
        );
        closed(second.port).await;
        assert!(manager.list().await.is_empty());

        // A closed workspace cancels pending startup, and its late cleanup must
        // not remove a newer lease with the same stable server ID.
        fs::write(root.join("delay-start"), "").unwrap();
        let background = manager.clone();
        let directory = root.to_owned();
        let pending =
            tokio::spawn(async move { background.toggle("ws".into(), directory, None).await });
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let pending_port = loop {
            if let Some(server) = manager.list().await.first() {
                break server.port;
            }
            assert!(tokio::time::Instant::now() < deadline);
            tokio::task::yield_now().await;
        };
        manager.stop_workspace("ws").await;
        fs::remove_file(root.join("delay-start")).unwrap();
        let replacement = manager
            .toggle("ws".into(), root.to_owned(), None)
            .await
            .unwrap()
            .unwrap();
        assert!(pending.await.unwrap().unwrap_err().contains("canceled"));
        closed(pending_port).await;
        assert_eq!(manager.list().await[0].id, replacement.id);
        manager.stop_all().await;
        closed(replacement.port).await;
        assert!(manager.list().await.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn application_quit_kills_preview_and_helpers_without_polling_async_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("index.html"), "quit fixture").unwrap();
        fs::write(root.join("ignore-terminate"), "").unwrap();
        fs::write(root.join("spawn-helper"), "").unwrap();
        let processes = ProcessGroups::default();
        let manager = fixture(root, Duration::from_secs(3)).with_processes(processes.clone());
        let running = manager
            .toggle("ws".into(), root.to_owned(), None)
            .await
            .unwrap()
            .unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let helper_port: u16 = loop {
            if let Ok(port) = fs::read_to_string(root.join("helper-port"))
                && let Ok(port) = port.parse()
            {
                break port;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "preview helper did not start"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        // Do not yield to Tokio after closing. AppKit's short quit allowance
        // cannot depend on the engine actor or a graceful SIGTERM being polled.
        processes.close();
        let deadline = std::time::Instant::now() + Duration::from_millis(150);
        loop {
            let parent =
                std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, running.port));
            let helper = std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, helper_port));
            if parent.is_err() && helper.is_err() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "owned preview survived application quit"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let error = manager
            .toggle("another".into(), root.to_owned(), None)
            .await
            .unwrap_err();
        assert!(
            error.contains("shutting down"),
            "late spawn escaped quit: {error}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn preview_readiness_failures_exit_diagnostics_and_shutdown_clean_up_children() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("index.html"), "preview").unwrap();
        fs::write(root.join("reject-ready"), "").unwrap();
        let manager = fixture(root, Duration::from_secs(3));
        let failure = manager
            .toggle("ws".into(), root.to_owned(), None)
            .await
            .unwrap_err();
        assert!(failure.contains("no page was found at /"), "{failure}");
        assert!(manager.list().await.is_empty());
        fs::remove_file(root.join("reject-ready")).unwrap();
        fs::write(root.join("delay-start"), "").unwrap();
        let manager = fixture(root, Duration::from_millis(100));
        let began = tokio::time::Instant::now();
        assert!(
            manager
                .toggle("ws".into(), root.to_owned(), None)
                .await
                .unwrap_err()
                .contains("did not become ready")
        );
        assert!(began.elapsed() < Duration::from_secs(2));
        assert!(manager.list().await.is_empty());
        fs::remove_file(root.join("delay-start")).unwrap();
        let executable = root.join("failure-cli");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '\\033[31mDeliberate Vite fixture failure\\033[0m' >&2\nexit 7\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let mut manager = PreviewManager::new(
            Ok(PreviewProgram {
                executable,
                arguments: Vec::new(),
            }),
            CancellationToken::new(),
            |_| {},
        );
        Arc::get_mut(&mut manager.0).unwrap().first_port = 0;
        let failure = manager
            .toggle("ws".into(), root.to_owned(), None)
            .await
            .unwrap_err();
        assert!(
            failure.contains("Deliberate Vite fixture failure"),
            "{failure}"
        );
        assert!(!failure.contains('\u{1b}'));
        assert!(manager.list().await.is_empty());
        let manager = fixture(root, Duration::from_secs(3));
        let running = manager
            .toggle("ws".into(), root.to_owned(), None)
            .await
            .unwrap()
            .unwrap();
        manager.0.cancel.cancel();
        closed(running.port).await;
    }
}
