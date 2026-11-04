use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use vela_core::event::PermissionRequest;
use vela_core::session::{SessionSummary, SessionUsage};

use crate::service::Endpoint;

#[derive(Debug, thiserror::Error)]
pub enum OpencodeError {
    #[error("{0}")]
    Service(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("OpenCode returned HTTP {status}: {body}")]
    Status { status: u16, body: String },
    #[error("invalid OpenCode response: {0}")]
    Protocol(String),
    #[error("invalid OpenCode endpoint: {0}")]
    Url(String),
}

pub type Result<T> = std::result::Result<T, OpencodeError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptFile {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mention: Option<PromptMention>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptMention {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

async fn prompt_file_spec(file: &PromptFile) -> Result<Value> {
    let path = std::path::Path::new(&file.path);
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|error| OpencodeError::Service(format!("Could not attach {name}: {error}")))?;
    if !metadata.is_file() {
        return Err(OpencodeError::Service(format!("{name} is not a file")));
    }
    if metadata.len() > 10 * 1024 * 1024 {
        return Err(OpencodeError::Service(format!(
            "{name} is larger than 10 MB"
        )));
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| OpencodeError::Service(error.to_string()))?
            .join(path)
    };
    let uri = Url::from_file_path(absolute)
        .map_err(|_| OpencodeError::Protocol("Invalid attachment path".into()))?;
    let mut spec = json!({"uri": uri.as_str(), "name": name});
    if let Some(mention) = &file.mention {
        spec["mention"] = serde_json::to_value(mention)
            .map_err(|error| OpencodeError::Protocol(error.to_string()))?;
    }
    Ok(spec)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionReply {
    Allow,
    AllowAlways,
    Deny,
}

impl PermissionReply {
    pub fn decision(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::AllowAlways => "always",
            Self::Deny => "deny",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub context_limit: Option<u64>,
    pub variant: Option<String>,
    pub variants: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct InboxItem {
    pub id: String,
    pub session_id: String,
    pub text: String,
    pub delivery: String,
    pub kind: String,
    pub attachment_count: usize,
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base: Url,
    auth: Option<String>,
}

impl Client {
    pub fn new(endpoint: Endpoint) -> Result<Self> {
        let mut base =
            Url::parse(&endpoint.url).map_err(|error| OpencodeError::Url(error.to_string()))?;
        if !base.path().ends_with('/') {
            let path = format!("{}/", base.path());
            base.set_path(&path);
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self {
            http,
            base,
            auth: endpoint.authorization(),
        })
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    fn url(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base.clone();
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| OpencodeError::Url("endpoint cannot be a base URL".into()))?;
            path.pop_if_empty();
            for segment in segments {
                path.push(segment);
            }
        }
        Ok(url)
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let request = if let Some(auth) = &self.auth {
            request.header("authorization", auth)
        } else {
            request
        };
        let response = request.timeout(Duration::from_secs(120)).send().await?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(OpencodeError::Status {
                status,
                body: body.chars().take(2_000).collect(),
            })
        }
    }

    async fn get_json(&self, segments: &[&str], query: &[(&str, String)]) -> Result<Value> {
        let url = self.url(segments)?;
        let request = self.http.get(url).query(query);
        Ok(self.send(request).await?.json().await?)
    }

    async fn post_json(&self, segments: &[&str], body: Value) -> Result<Value> {
        let url = self.url(segments)?;
        let request = self.http.post(url).json(&body);
        let response = self.send(request).await?;
        if response.content_length() == Some(0) {
            return Ok(Value::Null);
        }
        Ok(response.json().await.unwrap_or(Value::Null))
    }

    /// Open the SSE endpoint without an overall request timeout. The stream is
    /// kept alive by the server's heartbeat and bounded by the caller's
    /// cancellation token.
    pub(crate) async fn open_event_stream(&self) -> Result<reqwest::Response> {
        let url = self.url(&["api", "event"])?;
        let mut request = self
            .http
            .get(url)
            .header("accept", "text/event-stream")
            .header("cache-control", "no-cache");
        if let Some(auth) = &self.auth {
            request = request.header("authorization", auth);
        }
        let response = request.send().await?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(OpencodeError::Status {
                status,
                body: body.chars().take(2_000).collect(),
            })
        }
    }

    pub async fn server_info(&self) -> Result<Value> {
        self.get_json(&["api", "info"], &[]).await
    }

    pub async fn list_provider_integrations(
        &self,
        directory: &str,
    ) -> Result<Vec<crate::providers::ProviderIntegration>> {
        let value = self
            .get_json(
                &["api", "integration"],
                &[("location[directory]", directory.to_owned())],
            )
            .await?;
        crate::providers::parse_catalog(&value)
    }

    pub async fn connect_provider_key(
        &self,
        directory: &str,
        integration: &str,
        key: String,
        label: String,
        answers: serde_json::Map<String, Value>,
    ) -> Result<()> {
        if !crate::providers::SUPPORTED_PROVIDER_IDS.contains(&integration) {
            return Err(OpencodeError::Protocol(
                "unsupported provider integration".into(),
            ));
        }
        let url = self.url(&["api", "integration", integration, "connect", "key"])?;
        let mut body = json!({"key": key});
        if !label.is_empty() {
            body["label"] = Value::String(label);
        }
        if !answers.is_empty() {
            body["answer"] = Value::Object(answers);
        }
        // A runtime error body can echo submitted credentials. Never forward it
        // into engine updates, UI errors, Debug output or logs.
        self.send(
            self.http
                .post(url)
                .query(&[("location[directory]", directory)])
                .json(&body),
        )
        .await
        .map_err(|error| match error {
            OpencodeError::Status { status, .. } => {
                OpencodeError::Service(format!("Provider connection failed (HTTP {status})"))
            }
            _ => OpencodeError::Service("Could not connect to the provider service".into()),
        })?;
        Ok(())
    }

    pub async fn remove_provider_credential(
        &self,
        directory: &str,
        integration: &str,
        credential: &str,
    ) -> Result<()> {
        // Removal is global in the runtime API. Verify membership in the
        // captured workspace catalog before issuing that global mutation.
        let catalog = self
            .list_provider_integrations(directory)
            .await
            .map_err(|_| {
                OpencodeError::Service("Could not verify the provider credential".into())
            })?;
        if !catalog.iter().any(|provider| {
            provider.id == integration
                && provider
                    .credentials
                    .iter()
                    .any(|item| item.id == credential)
        }) {
            return Err(OpencodeError::Service(
                "The provider credential is no longer available in this workspace".into(),
            ));
        }
        let url = self.url(&["api", "credential", credential])?;
        self.send(self.http.delete(url))
            .await
            .map_err(|error| match error {
                OpencodeError::Status { status, .. } => OpencodeError::Service(format!(
                    "Could not remove provider credential (HTTP {status})"
                )),
                _ => OpencodeError::Service("Could not remove provider credential".into()),
            })?;
        Ok(())
    }

    pub async fn start_provider_oauth(
        &self,
        directory: &str,
        integration: &str,
        method: &str,
    ) -> Result<crate::providers::OAuthAttempt> {
        if !crate::providers::SUPPORTED_PROVIDER_IDS.contains(&integration) {
            return Err(OpencodeError::Protocol(
                "unsupported provider integration".into(),
            ));
        }
        let url = self.url(&["api", "integration", integration, "connect", "oauth"])?;
        let response = self
            .send(
                self.http
                    .post(url)
                    .query(&[("location[directory]", directory)])
                    .json(&json!({"methodID":method})),
            )
            .await
            .map_err(|_| OpencodeError::Service("Could not start provider authorization".into()))?;
        let value: Value = response
            .json()
            .await
            .map_err(|_| OpencodeError::Protocol("invalid authorization attempt".into()))?;
        let value = value.get("data").unwrap_or(&value);
        let id = value
            .get("attemptID")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                OpencodeError::Protocol("authorization service returned no attempt".into())
            })?;
        Ok(crate::providers::OAuthAttempt {
            id: id.into(),
            url: value
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            instructions: value
                .get("instructions")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            code: value.get("mode").and_then(Value::as_str) == Some("code"),
        })
    }

    pub async fn poll_provider_oauth(
        &self,
        directory: &str,
        integration: &str,
        attempt: &str,
    ) -> Result<crate::providers::OAuthStatus> {
        let value = self
            .get_json(
                &[
                    "api",
                    "integration",
                    integration,
                    "connect",
                    "oauth",
                    attempt,
                ],
                &[("location[directory]", directory.into())],
            )
            .await
            .map_err(|_| {
                OpencodeError::Service("Lost contact with the authorization service".into())
            })?;
        let value = value.get("data").unwrap_or(&value);
        Ok(match value.get("status").and_then(Value::as_str) {
            Some("complete") => crate::providers::OAuthStatus::Complete,
            Some("expired") => crate::providers::OAuthStatus::Expired,
            Some("failed") => crate::providers::OAuthStatus::Failed,
            _ => crate::providers::OAuthStatus::Pending,
        })
    }

    pub async fn complete_provider_oauth(
        &self,
        directory: &str,
        integration: &str,
        attempt: &str,
        code: String,
    ) -> Result<()> {
        let url = self.url(&[
            "api",
            "integration",
            integration,
            "connect",
            "oauth",
            attempt,
            "complete",
        ])?;
        self.send(
            self.http
                .post(url)
                .query(&[("location[directory]", directory)])
                .json(&json!({"code":code})),
        )
        .await
        .map_err(|_| OpencodeError::Service("Could not complete provider authorization".into()))?;
        Ok(())
    }

    pub async fn cancel_provider_oauth(
        &self,
        directory: &str,
        integration: &str,
        attempt: &str,
    ) -> Result<()> {
        let url = self.url(&[
            "api",
            "integration",
            integration,
            "connect",
            "oauth",
            attempt,
        ])?;
        self.send(
            self.http
                .delete(url)
                .query(&[("location[directory]", directory)]),
        )
        .await
        .map_err(|_| OpencodeError::Service("Could not cancel provider authorization".into()))?;
        Ok(())
    }

    pub async fn list_sessions(&self, directory: Option<&str>) -> Result<Vec<SessionSummary>> {
        let mut query: Vec<(&str, String)> = vec![("limit", "5000".to_string())];
        if let Some(directory) = directory {
            query.push(("directory", directory.to_string()));
        }
        let value = self.get_json(&["api", "session"], &query).await?;
        let items = value
            .get("data")
            .and_then(Value::as_array)
            .or_else(|| value.as_array())
            .ok_or_else(|| OpencodeError::Protocol("session list is not an array".into()))?;
        Ok(items.iter().filter_map(session_from_value).collect())
    }

    pub async fn create_session(&self, directory: &str) -> Result<SessionSummary> {
        let value = self
            .post_json(
                &["api", "session"],
                json!({ "location": { "directory": directory } }),
            )
            .await?;
        session_from_value(value.get("data").unwrap_or(&value))
            .ok_or_else(|| OpencodeError::Protocol("session.create returned no session".into()))
    }

    pub async fn get_session(&self, session_id: &str) -> Result<SessionSummary> {
        let value = self.get_json(&["api", "session", session_id], &[]).await?;
        session_from_value(value.get("data").unwrap_or(&value))
            .ok_or_else(|| OpencodeError::Protocol("session.get returned no session".into()))
    }

    /// Load the full persisted conversation, following every continuation page.
    pub async fn load_messages(&self, session_id: &str) -> Result<Value> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        for page in 0..200 {
            let mut query: Vec<(&str, String)> = vec![("limit", "200".to_string())];
            if page == 0 {
                query.push(("order", "asc".to_string()));
            }
            if let Some(cursor) = &cursor {
                query.push(("cursor", cursor.clone()));
            }
            let value = self
                .get_json(&["api", "session", session_id, "message"], &query)
                .await?;
            if let Some(messages) = value.as_array() {
                all.extend(messages.iter().cloned());
                break;
            }
            let data = value
                .get("data")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            all.extend(data);
            cursor = value
                .pointer("/cursor/next")
                .and_then(Value::as_str)
                .filter(|cursor| !cursor.is_empty())
                .map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        Ok(Value::Array(all))
    }

    pub async fn prompt(
        &self,
        session_id: &str,
        text: &str,
        files: &[PromptFile],
        delivery: Option<&str>,
    ) -> Result<Value> {
        let mut body = json!({ "text": text });
        if !files.is_empty() {
            let mut specs = Vec::with_capacity(files.len());
            for file in files {
                specs.push(prompt_file_spec(file).await?);
            }
            body["files"] = Value::Array(specs);
        }
        if let Some(delivery) = delivery {
            body["delivery"] = json!(delivery);
        }
        self.post_json(&["api", "session", session_id, "prompt"], body)
            .await
    }

    pub async fn interrupt(&self, session_id: &str) -> Result<()> {
        self.post_json(&["api", "session", session_id, "interrupt"], json!({}))
            .await?;
        Ok(())
    }

    pub async fn compact(&self, session_id: &str) -> Result<()> {
        self.post_json(&["api", "session", session_id, "compact"], json!({}))
            .await?;
        Ok(())
    }

    pub async fn run_command(&self, session_id: &str, name: &str, text: &str) -> Result<()> {
        self.post_json(
            &["api", "session", session_id, "command"],
            json!({ "name": name, "text": text }),
        )
        .await?;
        Ok(())
    }

    pub async fn list_skills(&self, directory: &str) -> Result<Value> {
        self.get_json(
            &["api", "skill"],
            &[("location[directory]", directory.to_owned())],
        )
        .await
    }

    pub async fn list_mcp_servers(
        &self,
        directory: &str,
    ) -> Result<Vec<crate::inventory::McpServerInfo>> {
        let value = self
            .get_json(
                &["api", "mcp"],
                &[("location[directory]", directory.to_owned())],
            )
            .await?;
        Ok(crate::inventory::parse_mcp_servers(&value))
    }

    pub async fn list_plugins(
        &self,
        directory: &str,
    ) -> Result<Vec<crate::inventory::PluginInfo>> {
        let value = self
            .get_json(
                &["api", "plugin"],
                &[("location[directory]", directory.to_owned())],
            )
            .await?;
        Ok(crate::inventory::parse_plugins(&value))
    }

    pub async fn list_skill_inventory(
        &self,
        directory: &str,
    ) -> Result<Vec<crate::inventory::SkillInfo>> {
        let value = self.list_skills(directory).await?;
        Ok(crate::inventory::parse_skills(&value))
    }

    pub async fn command_catalog(
        &self,
        directory: &str,
    ) -> Result<Vec<crate::commands::CommandOption>> {
        let query = [("location[directory]", directory.to_owned())];
        let commands = self
            .get_json(&["api", "command"], &query)
            .await
            .unwrap_or(Value::Null);
        let skills = self
            .get_json(&["api", "skill"], &query)
            .await
            .unwrap_or(Value::Null);
        Ok(crate::commands::parse_catalog(&commands, &skills))
    }

    pub async fn find_files(&self, directory: &str, query: &str) -> Result<Vec<String>> {
        let value = self
            .get_json(
                &["api", "fs", "find"],
                &[
                    ("location[directory]", directory.to_owned()),
                    ("query", query.to_owned()),
                    ("type", "file".into()),
                    ("limit", "50".into()),
                ],
            )
            .await?;
        let rows = value
            .get("data")
            .and_then(Value::as_array)
            .or_else(|| value.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(rows
            .iter()
            .filter_map(|row| row.get("path").and_then(Value::as_str))
            .filter(|path| {
                !["vela-recovery", "mira-recovery", "openshell-recovery", "orbit-recovery"]
                    .iter()
                    .any(|part| path.split('/').any(|component| component.ends_with(part)))
            })
            .map(str::to_owned)
            .collect())
    }

    pub async fn activate_skill(&self, session_id: &str, id: &str) -> Result<()> {
        self.post_json(
            &["api", "experimental", "session", session_id, "skill"],
            json!({ "id": id }),
        )
        .await?;
        Ok(())
    }

    pub async fn stage_revert(
        &self,
        session_id: &str,
        message_id: &str,
        files: bool,
    ) -> Result<Value> {
        self.post_json(
            &["api", "session", session_id, "revert", "stage"],
            json!({ "messageID": message_id, "files": files }),
        )
        .await
    }

    pub async fn commit_revert(&self, session_id: &str) -> Result<()> {
        self.post_json(
            &["api", "session", session_id, "revert", "commit"],
            json!({}),
        )
        .await?;
        Ok(())
    }

    pub async fn clear_revert(&self, session_id: &str) -> Result<()> {
        let url = self.url(&["api", "session", session_id, "revert"])?;
        let request = self.http.delete(url);
        self.send(request).await?;
        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let url = self.url(&["api", "session", session_id])?;
        let request = self.http.delete(url);
        self.send(request).await?;
        Ok(())
    }

    pub async fn list_permissions(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<PermissionRequest>> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(directory) = directory {
            query.push(("location[directory]", directory.to_string()));
        }
        let value = self
            .get_json(&["api", "permission", "request"], &query)
            .await?;
        let items = value
            .get("data")
            .and_then(Value::as_array)
            .or_else(|| value.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(items
            .iter()
            .filter_map(|item| {
                Some(PermissionRequest {
                    id: item.get("id")?.as_str()?.to_string(),
                    session_id: item.get("sessionID")?.as_str()?.to_string(),
                    action: item
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("permission")
                        .to_string(),
                    resources: item
                        .get("resources")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    message: item
                        .get("message")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
            })
            .collect())
    }

    pub async fn reply_permission(
        &self,
        session_id: &str,
        request_id: &str,
        reply: PermissionReply,
    ) -> Result<()> {
        self.post_json(
            &[
                "api",
                "session",
                session_id,
                "permission",
                request_id,
                "reply",
            ],
            json!({ "decision": reply.decision() }),
        )
        .await?;
        Ok(())
    }

    pub async fn list_inbox(&self, session_id: &str) -> Result<Vec<InboxItem>> {
        let value = self
            .get_json(&["api", "session", session_id, "inbox"], &[])
            .await?;
        let items = value
            .get("data")
            .and_then(Value::as_array)
            .or_else(|| value.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(items.iter().filter_map(inbox_from_value).collect())
    }

    pub async fn cancel_inbox(&self, session_id: &str, inbox_id: &str) -> Result<()> {
        let url = self.url(&["api", "session", session_id, "inbox", inbox_id])?;
        self.send(self.http.delete(url)).await?;
        Ok(())
    }

    pub async fn steer_inbox(&self, session_id: &str, inbox_id: &str) -> Result<()> {
        let url = self.url(&["api", "session", session_id, "inbox", inbox_id])?;
        let request = self.http.patch(url).json(&json!({ "delivery": "steer" }));
        self.send(request).await?;
        Ok(())
    }

    pub async fn list_forms(&self, session_id: &str) -> Result<Vec<crate::forms::FormRequest>> {
        let value = self
            .get_json(&["api", "session", session_id, "form"], &[])
            .await?;
        Ok(crate::forms::parse_forms(&value, session_id))
    }

    pub async fn list_location_forms(
        &self,
        directory: &str,
    ) -> Result<Vec<crate::forms::FormRequest>> {
        let value = self
            .get_json(
                &["api", "form"],
                &[("location[directory]", directory.to_owned())],
            )
            .await?;
        Ok(crate::forms::parse_forms(&value, "global"))
    }

    fn form_directory_header(
        request: reqwest::RequestBuilder,
        directory: Option<&str>,
    ) -> reqwest::RequestBuilder {
        if let Some(directory) = directory {
            let encoded = url::form_urlencoded::byte_serialize(directory.as_bytes())
                .collect::<String>()
                .replace('+', "%20");
            request.header("x-opencode-directory", encoded)
        } else {
            request
        }
    }

    pub async fn reply_form(
        &self,
        session_id: &str,
        form_id: &str,
        answer: serde_json::Map<String, Value>,
        directory: Option<&str>,
    ) -> Result<()> {
        let url = self.url(&["api", "session", session_id, "form", form_id, "reply"])?;
        let request = self.http.post(url).json(&json!({ "answer": answer }));
        self.send(Self::form_directory_header(request, directory))
            .await?;
        Ok(())
    }

    pub async fn cancel_form(
        &self,
        session_id: &str,
        form_id: &str,
        directory: Option<&str>,
    ) -> Result<()> {
        let url = self.url(&["api", "session", session_id, "form", form_id])?;
        let request = self.http.delete(url);
        self.send(Self::form_directory_header(request, directory))
            .await?;
        Ok(())
    }

    pub async fn list_models(&self, directory: &str) -> Result<Vec<ModelInfo>> {
        let value = self
            .get_json(
                &["api", "model"],
                &[("location[directory]", directory.to_string())],
            )
            .await?;
        let items = value
            .get("data")
            .and_then(Value::as_array)
            .or_else(|| value.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(items
            .iter()
            .filter(|value| value.get("enabled") != Some(&Value::Bool(false)))
            .filter_map(model_from_value)
            .filter(|model| !model.id.is_empty() && !model.provider_id.is_empty())
            .collect())
    }

    pub async fn selected_model(&self, session_id: &str) -> Result<Option<ModelInfo>> {
        let value = self.get_json(&["api", "session", session_id], &[]).await?;
        let session = value.get("data").unwrap_or(&value);
        Ok(session.get("model").and_then(model_from_value))
    }

    pub async fn default_model(&self, directory: &str) -> Result<Option<ModelInfo>> {
        let value = self
            .get_json(
                &["api", "model", "default"],
                &[("location[directory]", directory.to_string())],
            )
            .await?;
        let value = value.get("data").unwrap_or(&value);
        Ok(model_from_value(value))
    }

    pub async fn switch_model(&self, session_id: &str, model: &ModelInfo) -> Result<()> {
        let mut selection = json!({ "id": model.id, "providerID": model.provider_id });
        if let Some(variant) = &model.variant {
            selection["variant"] = Value::String(variant.clone());
        }
        self.post_json(
            &["api", "session", session_id, "model"],
            json!({ "model": selection }),
        )
        .await?;
        Ok(())
    }

    pub async fn switch_agent(&self, session_id: &str, agent_id: &str) -> Result<()> {
        self.post_json(
            &["api", "session", session_id, "agent"],
            json!({ "agent": agent_id }),
        )
        .await?;
        Ok(())
    }

    pub async fn list_agents(&self, directory: &str) -> Result<Vec<AgentInfo>> {
        let value = self
            .get_json(
                &["api", "agent"],
                &[("location[directory]", directory.to_string())],
            )
            .await?;
        let items = value
            .get("data")
            .and_then(Value::as_array)
            .or_else(|| value.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(items
            .iter()
            .filter_map(|item| {
                let id = item.get("id")?.as_str()?.to_string();
                Some(AgentInfo {
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or(&id)
                        .to_string(),
                    id,
                })
            })
            .collect())
    }

    pub async fn session_usage(&self, session_id: &str) -> Result<Option<SessionUsage>> {
        let value = self.get_session(session_id).await?;
        let _ = value;
        Ok(None)
    }

    pub async fn health(&self) -> bool {
        self.send(
            self.http.get(
                self.url(&["api", "info"])
                    .unwrap_or_else(|_| self.base.clone()),
            ),
        )
        .await
        .is_ok()
    }
}

pub fn inbox_from_value(value: &Value) -> Option<InboxItem> {
    let id = value.get("id")?.as_str()?.to_string();
    let session_id = value.get("sessionID")?.as_str()?.to_string();
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("user")
        .to_string();
    let text = value
        .pointer("/payload/text")
        .and_then(Value::as_str)
        .or_else(|| value.get("text").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    Some(InboxItem {
        id,
        session_id,
        text,
        delivery: value
            .get("delivery")
            .and_then(Value::as_str)
            .unwrap_or("queue")
            .to_string(),
        kind,
        attachment_count: value
            .pointer("/payload/files")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
    })
}

pub fn model_from_value(value: &Value) -> Option<ModelInfo> {
    let id = value
        .get("id")
        .or_else(|| value.get("modelID"))?
        .as_str()?
        .to_string();
    Some(ModelInfo {
        id: id.clone(),
        provider_id: value
            .get("providerID")
            .or_else(|| value.get("providerId"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .unwrap_or(&id)
            .to_string(),
        context_limit: value.pointer("/limit/context").and_then(Value::as_u64),
        variant: value
            .get("variant")
            .and_then(Value::as_str)
            .map(str::to_owned),
        variants: value
            .get("variants")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|variant| {
                variant
                    .as_str()
                    .or_else(|| variant.get("id").and_then(Value::as_str))
            })
            .filter(|variant| !variant.is_empty())
            .map(str::to_owned)
            .collect(),
    })
}

pub fn session_from_value(value: &Value) -> Option<SessionSummary> {
    let id = value.get("id").and_then(Value::as_str)?.to_string();
    let directory = value
        .pointer("/location/directory")
        .and_then(Value::as_str)
        .or_else(|| value.get("directory").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    Some(SessionSummary {
        id,
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        directory,
        updated_at: value
            .pointer("/time/updated")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        parent_id: value
            .get("parentID")
            .and_then(Value::as_str)
            .map(str::to_string),
        agent: value
            .get("agent")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn attachment_wire_contract_encodes_uri_and_validates_files_and_limit() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("image 🌍 #?.png");
        std::fs::write(&path, b"fixture").unwrap();
        let mut file = PromptFile {
            path: path.to_string_lossy().into_owned(),
            name: Some("ignored display override".into()),
            mention: Some(PromptMention {
                start: 8,
                end: 19,
                text: "@src/foo.ts".into(),
            }),
        };
        let spec = prompt_file_spec(&file).await.unwrap();
        assert_eq!(spec["name"], "image 🌍 #?.png");
        let uri = spec["uri"].as_str().unwrap();
        assert!(uri.contains("%20") && uri.contains("%23") && uri.contains("%3F"));
        assert_eq!(Url::parse(uri).unwrap().to_file_path().unwrap(), path);
        assert_eq!(
            spec["mention"],
            json!({"start":8,"end":19,"text":"@src/foo.ts"})
        );
        assert!(spec.get("path").is_none());
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(10 * 1024 * 1024)
            .unwrap();
        assert!(
            prompt_file_spec(&file).await.is_ok(),
            "exactly 10 MB is accepted by Vela"
        );
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(10 * 1024 * 1024 + 1)
            .unwrap();
        assert!(
            prompt_file_spec(&file)
                .await
                .unwrap_err()
                .to_string()
                .contains("larger than 10 MB")
        );
        file.path = temp.path().to_string_lossy().into_owned();
        assert!(
            prompt_file_spec(&file)
                .await
                .unwrap_err()
                .to_string()
                .contains("not a file")
        );
        file.path = temp.path().join("missing").to_string_lossy().into_owned();
        assert!(prompt_file_spec(&file).await.is_err());
    }

    #[test]
    fn file_only_inbox_entries_keep_attachment_counts_without_requiring_local_paths() {
        let entry = inbox_from_value(&json!({
            "id":"inbox-files", "sessionID":"session", "type":"user", "delivery":"queue",
            "payload":{"text":"", "files":[{"name":"🌍.png","uri":"file:///fixture/image.png"},{"name":"notes.txt"}]}
        })).unwrap();
        assert!(entry.text.is_empty());
        assert_eq!(entry.attachment_count, 2);
        assert_eq!(entry.session_id, "session");
        assert_eq!(
            inbox_from_value(&json!({"id":"text","sessionID":"session","text":"legacy text"}))
                .unwrap()
                .attachment_count,
            0
        );
    }

    #[test]
    fn parses_session_location_and_title() {
        let value = json!({
            "id": "ses_1",
            "title": "Fix the build",
            "location": { "directory": "/repo" },
            "time": { "created": 1, "updated": 9 },
            "parentID": "ses_0"
        });
        let session = session_from_value(&value).unwrap();
        assert_eq!(session.id, "ses_1");
        assert_eq!(session.directory, "/repo");
        assert_eq!(session.updated_at, 9);
        assert_eq!(session.parent_id.as_deref(), Some("ses_0"));
    }

    #[test]
    fn permission_decisions_match_protocol() {
        assert_eq!(PermissionReply::Allow.decision(), "allow");
        assert_eq!(PermissionReply::AllowAlways.decision(), "always");
        assert_eq!(PermissionReply::Deny.decision(), "deny");
    }

    #[test]
    fn model_variants_preserve_runtime_order_and_selected_strength() {
        let model = model_from_value(&json!({
            "id":"reasoning", "providerID":"fixture", "variant":"extra_high",
            "variants":["low", {"id":"extra_high"}, null, {}, "", {"id": 5}],
        }))
        .unwrap();
        assert_eq!(model.variant.as_deref(), Some("extra_high"));
        assert_eq!(model.variants, ["low", "extra_high"]);
        assert!(
            model_from_value(
                &json!({"id":"plain", "providerID":"fixture", "variants":{ "high":{} }})
            )
            .unwrap()
            .variants
            .is_empty()
        );
    }

    #[test]
    fn base_url_keeps_a_trailing_slash_for_joining() {
        let endpoint = Endpoint {
            url: "http://127.0.0.1:4096".into(),
            auth: None,
            version: Some("2.1.4".into()),
            pid: None,
        };
        let client = Client::new(endpoint).unwrap();
        assert!(client.base_url().as_str().ends_with('/'));
    }
}
