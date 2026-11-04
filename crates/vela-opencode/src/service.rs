use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use serde::Deserialize;
use tokio::process::Command;

use crate::client::{Client, OpencodeError};

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub url: String,
    pub auth: Option<(String, String)>,
    pub version: Option<String>,
    pub pid: Option<u32>,
}

impl Endpoint {
    pub fn authorization(&self) -> Option<String> {
        self.auth.as_ref().map(|(user, password)| {
            let raw = format!("{user}:{password}");
            format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(raw)
            )
        })
    }

    pub fn basic_credentials(&self) -> Option<(&str, &str)> {
        self.auth
            .as_ref()
            .map(|(user, password)| (user.as_str(), password.as_str()))
    }
}

#[derive(Deserialize)]
struct Registration {
    url: String,
    pid: Option<u32>,
    version: Option<String>,
    password: Option<String>,
}

#[derive(Deserialize)]
struct ServerInfo {
    version: String,
    pid: u32,
}

pub fn service_file() -> PathBuf {
    if let Some(path) = std::env::var_os("VELA_OPENCODE_SERVICE_FILE") {
        return PathBuf::from(path);
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("state")))
        .unwrap_or_else(|| PathBuf::from("."));
    state.join("opencode").join("service.json")
}

/// Discover a healthy, V2-compatible service without starting one.
pub async fn discover() -> Option<Endpoint> {
    discover_at(&service_file()).await
}

/// Discover a healthy, V2-compatible service at an explicit registration path.
/// The engine passes its configured path so tests and isolated profiles never
/// depend on process-global environment state.
pub async fn discover_at(path: &std::path::Path) -> Option<Endpoint> {
    let text = tokio::fs::read_to_string(path).await.ok()?;
    let registration: Registration = serde_json::from_str(&text).ok()?;
    let mut endpoint = Endpoint {
        url: registration.url,
        auth: registration
            .password
            .map(|password| ("opencode".to_string(), password)),
        version: registration.version,
        pid: registration.pid,
    };
    if !version_is_v2(endpoint.version.as_deref()) {
        return None;
    }
    let info = probe(&endpoint).await?;
    if !version_is_v2(Some(&info.version)) {
        return None;
    }
    if let Some(expected) = endpoint.pid
        && info.pid != expected
    {
        return None;
    }
    if let Some(expected) = &endpoint.version
        && expected != &info.version
    {
        return None;
    }
    endpoint.version = Some(info.version);
    endpoint.pid = Some(info.pid);
    Some(endpoint)
}

pub async fn managed_profile(file: &Path) -> Result<(PathBuf, PathBuf), String> {
    let file = if file.is_absolute() {
        file.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(file)
    };
    if file.file_name().is_some_and(|name| name == "service.json")
        && file
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "opencode")
    {
        let state = file
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| "The OpenCode service profile is invalid.".to_owned())?;
        return Ok((file.clone(), state.to_owned()));
    }
    // Some read-only connection descriptors alias the standard CLI profile.
    // Follow one only when its complete registration identity agrees. Never
    // create, copy or rewrite a registration to make this test succeed.
    let alias = file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("opencode/service.json");
    if let (Ok(selected), Ok(aliased)) =
        (tokio::fs::read(&file).await, tokio::fs::read(&alias).await)
        && let (Ok(selected), Ok(aliased)) = (
            serde_json::from_slice::<serde_json::Value>(&selected),
            serde_json::from_slice::<serde_json::Value>(&aliased),
        )
        && ["url", "pid", "version", "password", "id"]
            .iter()
            .all(|key| selected.get(key) == aliased.get(key))
        && selected
            .get("url")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && selected
            .get("pid")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    {
        return Ok((
            alias,
            file.parent()
                .expect("absolute file has a parent")
                .to_owned(),
        ));
    }
    Err("The connected OpenCode service uses a custom registration location. Sync it using its owning application.".into())
}

/// Discover a service or start the shared `opencode serve --service` daemon and
/// wait for it to register. This mirrors the original app's lifecycle: the
/// service is shared with the CLI and is not owned by a Vela window.
pub async fn ensure() -> Result<Endpoint, OpencodeError> {
    ensure_at(&service_file()).await
}

pub async fn ensure_at(path: &std::path::Path) -> Result<Endpoint, OpencodeError> {
    let binary = std::env::var_os("VELA_OPENCODE_BINARY").unwrap_or_else(|| "opencode".into());
    ensure_at_with_program(path, Path::new(&binary)).await
}

pub async fn ensure_at_with_program(path: &Path, binary: &Path) -> Result<Endpoint, OpencodeError> {
    if let Some(endpoint) = discover_at(path).await {
        return Ok(endpoint);
    }
    let (managed_file, state_home) = managed_profile(path)
        .await
        .map_err(OpencodeError::Service)?;
    let mut child = Command::new(binary)
        .args(["serve", "--service"])
        .env("XDG_STATE_HOME", state_home)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| OpencodeError::Service(format!("could not start opencode: {error}")))?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(endpoint) = discover_at(&managed_file).await {
            return Ok(endpoint);
        }
        if let Some(status) = child.try_wait().ok().flatten()
            && !status.success()
        {
            return Err(OpencodeError::Service(format!(
                "opencode serve --service exited with {status}"
            )));
        }
        if tokio::time::Instant::now() >= deadline {
            let _ = child.kill().await;
            return Err(OpencodeError::Service(
                "timed out waiting for the OpenCode service to start".into(),
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn probe(endpoint: &Endpoint) -> Option<ServerInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;
    let url = format!("{}/api/info", endpoint.url.trim_end_matches('/'));
    let mut request = client.get(url);
    if let Some(auth) = endpoint.authorization() {
        request = request.header("authorization", auth);
    }
    let response = request.send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<ServerInfo>().await.ok()
}

pub fn normalized_version(version: &str) -> Option<String> {
    version.split_whitespace().find_map(|part| {
        let part = part.strip_prefix('v').unwrap_or(part);
        let core = part.split(['-', '+']).next()?;
        let fields = core.split('.').collect::<Vec<_>>();
        (fields.len() == 3
            && fields
                .iter()
                .all(|field| !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit())))
        .then(|| part.to_owned())
    })
}

fn version_is_v2(version: Option<&str>) -> bool {
    match version {
        Some(version) => normalized_version(version)
            .as_deref()
            .unwrap_or_default()
            .split('.')
            .next()
            .and_then(|major| major.parse::<u32>().ok())
            .is_some_and(|major| major == 2),
        // A missing registration version is allowed; `/api/info` still has to
        // answer before the endpoint is trusted and must report V2.
        None => true,
    }
}

/// Connect through discovery/ensure and return an authenticated client.
pub async fn connect() -> Result<(Client, Endpoint), OpencodeError> {
    connect_at(&service_file()).await
}

pub async fn connect_at(path: &std::path::Path) -> Result<(Client, Endpoint), OpencodeError> {
    let endpoint = ensure_at(path).await?;
    let client = Client::new(endpoint.clone())?;
    Ok((client, endpoint))
}

pub async fn connect_at_with_program(
    path: &Path,
    binary: &Path,
) -> Result<(Client, Endpoint), OpencodeError> {
    let endpoint = ensure_at_with_program(path, binary).await?;
    let client = Client::new(endpoint.clone())?;
    Ok((client, endpoint))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn starting_service_uses_the_selected_profile_and_configured_binary() {
        use std::os::unix::fs::PermissionsExt;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("opencode")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            let length = socket.read(&mut request).await.unwrap();
            assert!(request[..length].starts_with(b"GET /api/info "));
            let body = r#"{"version":"2.1.4","pid":42}"#;
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
        });
        std::fs::write(
            root.join("target.json"),
            serde_json::json!({"url":format!("http://{address}"),"pid":42,"version":"2.1.4"})
                .to_string(),
        )
        .unwrap();
        let executable = root.join("fixture-cli");
        std::fs::write(
            &executable,
            r#"#!/bin/sh
base=${0%/*}
printf '%s|%s' "$XDG_STATE_HOME" "$*" > "$base/started"
cp "$base/target.json" "$XDG_STATE_HOME/opencode/service.json"
"#,
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let endpoint = ensure_at_with_program(&root.join("opencode/service.json"), &executable)
            .await
            .unwrap();
        assert_eq!(endpoint.version.as_deref(), Some("2.1.4"));
        assert_eq!(
            std::fs::read_to_string(root.join("started")).unwrap(),
            format!("{}|serve --service", root.display())
        );
        std::fs::remove_file(root.join("started")).unwrap();
        assert!(
            ensure_at_with_program(&root.join("unknown.json"), &executable)
                .await
                .is_err()
        );
        assert!(
            !root.join("started").exists(),
            "a custom descriptor must never start an unrelated inherited service"
        );
        server.await.unwrap();
    }

    #[test]
    fn recognizes_v2_versions() {
        assert!(version_is_v2(Some("2.1.4")));
        assert!(version_is_v2(Some("v2.0.0")));
        assert!(!version_is_v2(Some("1.9.0")));
        assert!(!version_is_v2(Some("3.0.0")));
        assert!(version_is_v2(Some("opencode v2.1.4")));
        assert_eq!(
            normalized_version("opencode v2.1.4-beta.1+build"),
            Some("2.1.4-beta.1+build".into())
        );
        assert!(normalized_version("not a version").is_none());
    }

    #[tokio::test]
    async fn discovery_validates_reported_major_when_registration_omits_version() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for reported in ["2.1.4", "3.0.0"] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let body = serde_json::json!({"version":reported,"pid":42}).to_string();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                let length = socket.read(&mut request).await.unwrap();
                assert!(request[..length].starts_with(b"GET /api/info "));
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            });
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("service.json");
            std::fs::write(
                &path,
                serde_json::json!({"url":format!("http://{address}"),"pid":42}).to_string(),
            )
            .unwrap();
            let discovered = discover_at(&path).await;
            if reported.starts_with('2') {
                let discovered = discovered.expect("V2 should be accepted");
                assert_eq!(discovered.version.as_deref(), Some(reported));
                assert_eq!(discovered.pid, Some(42));
            } else {
                assert!(
                    discovered.is_none(),
                    "an omitted registration version must not bypass the V2 boundary"
                );
            }
            server.await.unwrap();
        }
    }
}
