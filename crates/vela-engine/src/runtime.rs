//! Installed CLI discovery and runtime management, away from the render thread.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAction {
    Sync,
    Update,
}

/// Capabilities reported by the connected runtime. Vela currently ships the
/// OpenCode runtime; keeping this explicit lets Settings gate controls that the
/// reference manifest disables (notably steering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub attachments: bool,
    pub commands: bool,
    pub models: bool,
    pub agents: bool,
    pub permissions: bool,
    pub provider_credentials: bool,
    pub session_fork: bool,
    pub session_resume: bool,
    pub steering: bool,
    pub tui: bool,
}

impl RuntimeCapabilities {
    pub fn opencode(tui: bool) -> Self {
        Self {
            attachments: true,
            commands: true,
            models: true,
            agents: true,
            permissions: true,
            provider_credentials: true,
            session_fork: true,
            session_resume: true,
            steering: false,
            tui,
        }
    }
}

impl Default for RuntimeCapabilities {
    fn default() -> Self {
        Self::opencode(false)
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeResult {
    pub version: String,
    pub previous_version: String,
    pub cli_updated: bool,
}

/// Connection credentials stay inside the engine and are never part of a UI
/// notification or a Debug representation.
pub struct RuntimeSync {
    pub version: String,
    pub previous_version: String,
    pub cli_updated: bool,
    pub endpoint: vela_opencode::Endpoint,
    pub service_file: PathBuf,
}

pub async fn manage(
    program: &Path,
    action: RuntimeAction,
    service_file: PathBuf,
) -> Result<RuntimeSync, String> {
    manage_with(program, action, service_file, Duration::from_secs(60)).await
}

async fn manage_with(
    program: &Path,
    action: RuntimeAction,
    service_file: PathBuf,
    ready_timeout: Duration,
) -> Result<RuntimeSync, String> {
    let previous_version = version_from(program, Duration::from_secs(5)).await
        .and_then(|version| vela_opencode::service::normalized_version(&version))
        .ok_or_else(|| "OpenCode was not found on Vela's PATH. Install it or correct Vela's PATH, then try again.".to_owned())?;
    let (managed_file, state_home) = vela_opencode::service::managed_profile(&service_file).await?;
    if let Ok(text) = tokio::fs::read_to_string(&managed_file).await
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(address) = value.get("url").and_then(serde_json::Value::as_str)
    {
        let address = url::Url::parse(address)
            .map_err(|_| "The OpenCode service address is invalid.".to_owned())?;
        if !matches!(
            address.host_str(),
            Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
        ) {
            return Err(
                "This OpenCode service is remote. Update or restart it on its host.".into(),
            );
        }
    }
    let upgrade_output = if action == RuntimeAction::Update {
        run_cli(
            program,
            &["upgrade"],
            &state_home,
            Duration::from_secs(300),
            "OpenCode's updater",
        )
        .await?
    } else {
        String::new()
    };
    let version = version_from(program, Duration::from_secs(5)).await
        .and_then(|version| vela_opencode::service::normalized_version(&version))
        .ok_or_else(|| "OpenCode was not found on Vela's PATH. Install it or correct Vela's PATH, then try again.".to_owned())?;
    if action == RuntimeAction::Update && version == previous_version {
        let output = upgrade_output.to_ascii_lowercase();
        if output.contains("may be managed by a package manager")
            || output.contains("install anyways?")
        {
            return Err("OpenCode could not identify its install method. Update it through its package manager, then use Sync installed version.".into());
        }
    }
    let normalized = vela_opencode::service::normalized_version(&version)
        .filter(|version| version.split('.').next() == Some("2"))
        .ok_or_else(|| {
            format!("OpenCode {version} is installed, but Vela currently requires OpenCode V2.")
        })?;
    let mut endpoint = vela_opencode::service::discover_at(&managed_file).await;
    let matches = |endpoint: &vela_opencode::Endpoint| {
        endpoint
            .version
            .as_deref()
            .and_then(vela_opencode::service::normalized_version)
            .as_deref()
            == Some(normalized.as_str())
    };
    if !endpoint.as_ref().is_some_and(matches) {
        // The public CLI owns service replacement and persistent-terminal
        // handoff. Bind its state root to the service we actually connected to,
        // rather than accidentally restarting a different inherited profile.
        let command = if tokio::fs::try_exists(&managed_file).await.unwrap_or(false) {
            "restart"
        } else {
            "start"
        };
        run_cli(
            program,
            &["service", command],
            &state_home,
            ready_timeout,
            "OpenCode service restart",
        )
        .await?;
        let deadline = tokio::time::Instant::now() + ready_timeout;
        loop {
            endpoint = vela_opencode::service::discover_at(&managed_file).await;
            if endpoint.as_ref().is_some_and(matches) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("Timed out waiting for the synced OpenCode service to start.".into());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    Ok(RuntimeSync {
        cli_updated: version != previous_version,
        version,
        previous_version,
        endpoint: endpoint.expect("matching service was discovered"),
        service_file: managed_file,
    })
}

async fn read_tail(mut input: impl tokio::io::AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let count = input.read(&mut buffer).await?;
        if count == 0 {
            return Ok(output);
        }
        output.extend_from_slice(&buffer[..count]);
        if output.len() > 12000 {
            output.drain(..output.len() - 12000);
        }
    }
}

async fn run_cli(
    program: &Path,
    arguments: &[&str],
    state_home: &Path,
    timeout: Duration,
    label: &str,
) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(arguments)
        .env("XDG_STATE_HOME", state_home)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not run {label}: {error}"))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let result = tokio::time::timeout(timeout, async {
        tokio::join!(child.wait(), read_tail(stdout), read_tail(stderr))
    })
    .await;
    let (status, stdout, stderr) = match result {
        Ok(result) => result,
        Err(_) => {
            let _ = child.kill().await;
            return Err(if timeout == Duration::from_secs(300) {
                "OpenCode's updater timed out after five minutes.".into()
            } else {
                format!("{label} timed out.")
            });
        }
    };
    let status = status.map_err(|error| error.to_string())?;
    let stdout = String::from_utf8_lossy(&stdout.map_err(|error| error.to_string())?).into_owned();
    let stderr = String::from_utf8_lossy(&stderr.map_err(|error| error.to_string())?).into_owned();
    if !status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        if !detail.is_empty() {
            return Err(detail
                .chars()
                .rev()
                .take(2400)
                .collect::<String>()
                .chars()
                .rev()
                .collect());
        }
        return Err(format!("{label} exited with {status}."));
    }
    Ok(format!("{stdout}\n{stderr}"))
}

pub fn executable() -> OsString {
    std::env::var_os("VELA_OPENCODE_BINARY").unwrap_or_else(|| "opencode".into())
}

pub async fn installed_version() -> Option<String> {
    version_from(Path::new(&executable()), Duration::from_secs(5)).await
}

pub async fn installed_version_at(program: &Path) -> Option<String> {
    version_from(program, Duration::from_secs(5)).await
}

async fn version_from(program: &Path, timeout: Duration) -> Option<String> {
    let output = tokio::time::timeout(
        timeout,
        Command::new(program)
            .arg("--version")
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    #[tokio::test]
    async fn runtime_sync_scopes_cli_to_connected_profile_and_rebinds_updated_service() {
        use axum::{Json, Router, routing::get};
        use serde_json::json;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("opencode")).unwrap();
        let service_file = root.join("opencode/service.json");
        let server_version = root.join("server-version");
        fs::write(&server_version, "2.1.4").unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let state = server_version.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().route("/api/info", get(move || {
                let state = state.clone();
                async move { Json(json!({"version":fs::read_to_string(state).unwrap().trim(),"pid":42})) }
            }))).await.unwrap();
        });
        let registration = |version: &str| {
            json!({"url":format!("http://{address}"),"pid":42,"version":version}).to_string()
        };
        fs::write(&service_file, registration("2.1.4")).unwrap();
        fs::write(root.join("target.json"), registration("2.1.5")).unwrap();
        fs::write(root.join("cli-version"), "opencode v2.1.4\n").unwrap();
        let executable = root.join("fake-opencode");
        let script = r#"#!/bin/sh
base=${0%/*}
if [ "$1" = '--version' ]; then cat "$base/cli-version"; exit; fi
printf '%s|%s\n' "$XDG_STATE_HOME" "$*" >> "$base/commands"
if [ "$1" = 'upgrade' ]; then
  printf 'opencode v2.1.5\n' > "$base/cli-version"
elif [ "$1 $2" = 'service restart' ] || [ "$1 $2" = 'service start' ]; then
  cp "$base/target.json" "$XDG_STATE_HOME/opencode/service.json"
  printf '2.1.5' > "$base/server-version"
else exit 19; fi
"#;
        fs::write(&executable, script).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let run = |action| {
            manage_with(
                &executable,
                action,
                service_file.clone(),
                Duration::from_secs(1),
            )
        };
        let synced = run(RuntimeAction::Sync).await.unwrap();
        assert_eq!(synced.version, "2.1.4");
        assert!(
            !root.join("commands").exists(),
            "a matching healthy service needs no restart"
        );

        let updated = run(RuntimeAction::Update).await.unwrap();
        assert!(updated.cli_updated);
        assert_eq!(updated.previous_version, "2.1.4");
        assert_eq!(updated.version, "2.1.5");
        assert_eq!(updated.endpoint.version.as_deref(), Some("2.1.5"));
        assert_eq!(updated.service_file, service_file);
        let commands = fs::read_to_string(root.join("commands")).unwrap();
        assert_eq!(
            commands,
            format!(
                "{}|upgrade\n{}|service restart\n",
                root.display(),
                root.display()
            )
        );

        fs::write(root.join("commands"), "").unwrap();
        fs::write(&server_version, "2.1.4").unwrap();
        fs::write(&service_file, registration("2.1.4")).unwrap();
        let synced = run(RuntimeAction::Sync).await.unwrap();
        assert_eq!(synced.endpoint.version.as_deref(), Some("2.1.5"));
        assert!(!synced.cli_updated);
        assert_eq!(
            fs::read_to_string(root.join("commands")).unwrap(),
            format!("{}|service restart\n", root.display())
        );

        fs::write(root.join("commands"), "").unwrap();
        fs::write(root.join("cli-version"), "opencode v3.0.0\n").unwrap();
        assert!(
            run(RuntimeAction::Sync)
                .await
                .err()
                .unwrap()
                .contains("requires OpenCode V2")
        );
        assert_eq!(fs::read_to_string(root.join("commands")).unwrap(), "");
        fs::write(root.join("cli-version"), "opencode v2.1.5\n").unwrap();
        fs::write(
            &service_file,
            json!({"url":"https://example.invalid","pid":42,"version":"2.1.5"}).to_string(),
        )
        .unwrap();
        assert!(
            run(RuntimeAction::Update)
                .await
                .err()
                .unwrap()
                .contains("remote")
        );
        assert_eq!(
            fs::read_to_string(root.join("commands")).unwrap(),
            "",
            "remote services must not invoke the local updater"
        );
        server.abort();
    }

    #[tokio::test]
    async fn runtime_profile_alias_requires_complete_identity_and_cli_errors_are_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("opencode")).unwrap();
        let selected = root.join("service.json");
        let alias = root.join("opencode/service.json");
        let registration =
            r#"{"url":"http://127.0.0.1:1","pid":42,"password":"fixture-only","version":"2.1.4"}"#;
        fs::write(&selected, registration).unwrap();
        fs::write(&alias, registration).unwrap();
        assert_eq!(
            vela_opencode::service::managed_profile(&selected)
                .await
                .unwrap(),
            (alias.clone(), root.to_owned())
        );
        fs::write(&alias, registration.replace("fixture-only", "different")).unwrap();
        assert!(
            vela_opencode::service::managed_profile(&selected)
                .await
                .is_err()
        );
        let executable = root.join("fake-opencode");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf 'fixture failure' >&2\nexit 9\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            run_cli(
                &executable,
                &["upgrade"],
                root,
                Duration::from_secs(1),
                "fixture updater"
            )
            .await
            .err()
            .unwrap(),
            "fixture failure"
        );
        fs::write(&executable, "#!/bin/sh\nexec sleep 2\n").unwrap();
        let start = Instant::now();
        assert!(
            run_cli(
                &executable,
                &["upgrade"],
                root,
                Duration::from_millis(30),
                "fixture updater"
            )
            .await
            .err()
            .unwrap()
            .contains("timed out")
        );
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn cli_probe_uses_successful_first_line_and_bounds_a_hung_process() {
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("opencode");
        let write = |script: &str| {
            fs::write(&executable, script).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        };
        write("#!/bin/sh\nprintf 'opencode v2.1.4\\nextra output\\n'\n");
        assert_eq!(
            version_from(&executable, Duration::from_secs(1))
                .await
                .as_deref(),
            Some("opencode v2.1.4")
        );
        write("#!/bin/sh\nprintf 'unreliable output\\n'\nexit 1\n");
        assert!(
            version_from(&executable, Duration::from_secs(1))
                .await
                .is_none()
        );
        write("#!/bin/sh\nexec sleep 2\n");
        let started = Instant::now();
        assert!(
            version_from(&executable, Duration::from_millis(30))
                .await
                .is_none()
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "CLI discovery must not wait for the hung process"
        );
        assert!(
            version_from(&temp.path().join("missing"), Duration::from_secs(1))
                .await
                .is_none()
        );
    }

    #[test]
    fn opencode_capabilities_match_the_reference_manifest() {
        let ready = RuntimeCapabilities::opencode(true);
        assert!(ready.attachments && ready.commands && ready.models && ready.agents);
        assert!(ready.permissions && ready.provider_credentials);
        assert!(ready.session_fork && ready.session_resume);
        assert!(!ready.steering, "OpenCode steering stays disabled");
        assert!(ready.tui);
        assert!(!RuntimeCapabilities::opencode(false).tui);
        assert!(!RuntimeCapabilities::default().tui);
    }
}
