//! Tolerant normalization of OpenCode V2 events into Vela's domain vocabulary.
//!
//! OpenCode ships current `data` envelopes, older `properties` envelopes, and
//! occasional numbered event names. Vela must accept all of them without
//! letting wire-format churn leak into the transcript or UI.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::session::{SessionPhase, SessionSummary, SessionUsage};
use crate::transcript::{Message, Part, message_from_json, part_from_json, tool_content_text};

#[derive(Clone, Debug)]
pub struct EventEnvelope {
    pub event_type: String,
    pub data: Value,
    pub directory: Option<String>,
    pub created: i64,
}

impl EventEnvelope {
    pub fn parse(value: &Value) -> Option<Self> {
        let payload = value
            .get("payload")
            .filter(|payload| payload.get("type").is_some())
            .unwrap_or(value);
        let raw_type = payload.get("type").and_then(Value::as_str)?;
        let event_type = normalize_type(raw_type);
        let data = value
            .get("data")
            .or_else(|| value.get("properties"))
            .cloned()
            .or_else(|| payload.get("data").cloned())
            .unwrap_or_else(|| Value::Object(Default::default()));
        let directory = value
            .pointer("/location/directory")
            .and_then(Value::as_str)
            .or_else(|| {
                payload
                    .pointer("/location/directory")
                    .and_then(Value::as_str)
            })
            .or_else(|| data.get("directory").and_then(Value::as_str))
            .or_else(|| {
                data.pointer("/info/location/directory")
                    .and_then(Value::as_str)
            })
            .map(str::to_string);
        let created = value
            .get("created")
            .and_then(Value::as_i64)
            .or_else(|| payload.get("created").and_then(Value::as_i64))
            .unwrap_or_else(now_millis);
        Some(Self {
            event_type,
            data,
            directory,
            created,
        })
    }
}

fn normalize_type(value: &str) -> String {
    let trimmed = value.trim();
    if let Some((base, suffix)) = trimmed.rsplit_once('.')
        && !base.is_empty()
        && suffix.chars().all(|c| c.is_ascii_digit())
    {
        return base.to_string();
    }
    trimmed.to_string()
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub session_id: String,
    pub action: String,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub priority: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Started,
    Succeeded,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug)]
pub enum AgentEvent {
    SessionCreated {
        session: SessionSummary,
    },
    SessionRenamed {
        id: String,
        title: String,
        updated_at: i64,
    },
    SessionDeleted {
        id: String,
    },
    Execution {
        session_id: String,
        outcome: ExecutionOutcome,
        message: Option<String>,
        at: i64,
    },
    ExecutionSucceeded {
        session_id: String,
        at: i64,
    },
    ExecutionFailed {
        session_id: String,
        message: String,
        at: i64,
    },
    SessionStatus {
        session_id: String,
        phase: SessionPhase,
        message: Option<String>,
        attempt: u32,
        next: Option<i64>,
        at: i64,
    },
    RetryScheduled {
        session_id: String,
        message_id: String,
        attempt: u32,
        message: String,
        next: Option<i64>,
        at: i64,
    },
    CompactionStarted {
        session_id: String,
        at: i64,
    },
    CompactionDelta {
        session_id: String,
        text: String,
    },
    CompactionEnded {
        session_id: String,
        text: String,
        at: i64,
    },
    CompactionFailed {
        session_id: String,
        error: String,
        at: i64,
    },
    SkillActivated {
        session_id: String,
        id: String,
        name: String,
        text: String,
        at: i64,
    },
    Idle {
        session_id: String,
        at: i64,
    },
    StepStarted {
        session_id: String,
        message_id: String,
        agent: Option<String>,
        model: Option<String>,
        at: i64,
    },
    StepEnded {
        session_id: String,
        message_id: String,
        finish: Option<String>,
        at: i64,
    },
    StepFailed {
        session_id: String,
        message_id: String,
        message: String,
        at: i64,
    },
    TextStarted {
        session_id: String,
        message_id: String,
        part_id: String,
        ordinal: u32,
        at: i64,
    },
    TextDelta {
        session_id: String,
        message_id: String,
        part_id: String,
        ordinal: u32,
        delta: String,
        at: i64,
    },
    TextEnded {
        session_id: String,
        message_id: String,
        part_id: String,
        ordinal: u32,
        text: String,
        at: i64,
    },
    ReasoningStarted {
        session_id: String,
        message_id: String,
        part_id: String,
        ordinal: u32,
        at: i64,
    },
    ReasoningDelta {
        session_id: String,
        message_id: String,
        part_id: String,
        ordinal: u32,
        delta: String,
        at: i64,
    },
    ReasoningEnded {
        session_id: String,
        message_id: String,
        part_id: String,
        ordinal: u32,
        text: String,
        at: i64,
    },
    ToolInputStarted {
        session_id: String,
        message_id: String,
        call_id: String,
        name: String,
        at: i64,
    },
    ToolInputDelta {
        session_id: String,
        message_id: String,
        call_id: String,
        delta: String,
        at: i64,
    },
    ToolInputEnded {
        session_id: String,
        message_id: String,
        call_id: String,
        input: String,
        at: i64,
    },
    ToolCalled {
        session_id: String,
        message_id: String,
        call_id: String,
        input: String,
        at: i64,
    },
    ToolProgress {
        session_id: String,
        message_id: String,
        call_id: String,
        detail: String,
    },
    ToolFinished {
        session_id: String,
        message_id: String,
        call_id: String,
        success: bool,
        output: String,
        files: Vec<crate::transcript::ToolFile>,
        error: Option<String>,
        at: i64,
    },
    MessageUpsert {
        session_id: String,
        message: Message,
    },
    MessageRemoved {
        session_id: String,
        message_id: String,
    },
    PartUpsert {
        session_id: String,
        message_id: String,
        part: Part,
        at: i64,
    },
    PermissionAsked {
        request: PermissionRequest,
    },
    PermissionReplied {
        session_id: String,
        request_id: String,
        reply: String,
    },
    TodoUpdated {
        session_id: String,
        todos: Vec<TodoItem>,
    },
    UsageUpdated {
        session_id: String,
        usage: SessionUsage,
    },
    ShellStarted {
        session_id: String,
        command: String,
        at: i64,
    },
    ShellEnded {
        session_id: String,
        id: String,
        output: String,
        exit_code: i32,
    },
    Notice {
        session_id: String,
        text: String,
        error: bool,
        at: i64,
    },
    ServerConnected,
    Unknown {
        event_type: String,
    },
}

impl AgentEvent {
    /// Session this event is addressed to, when the event carries one. The SSE
    /// transport uses it for scoped dispatch, so background sessions do not
    /// invoke the focused panel's reducer.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::SessionCreated { session } => Some(&session.id),
            Self::SessionRenamed { id, .. } | Self::SessionDeleted { id } => Some(id),
            Self::Execution { session_id, .. }
            | Self::ExecutionSucceeded { session_id, .. }
            | Self::ExecutionFailed { session_id, .. }
            | Self::SessionStatus { session_id, .. }
            | Self::Idle { session_id, .. }
            | Self::RetryScheduled { session_id, .. }
            | Self::StepFailed { session_id, .. }
            | Self::StepStarted { session_id, .. }
            | Self::StepEnded { session_id, .. }
            | Self::TextStarted { session_id, .. }
            | Self::TextDelta { session_id, .. }
            | Self::TextEnded { session_id, .. }
            | Self::ReasoningStarted { session_id, .. }
            | Self::ReasoningDelta { session_id, .. }
            | Self::ReasoningEnded { session_id, .. }
            | Self::ToolInputStarted { session_id, .. }
            | Self::ToolInputDelta { session_id, .. }
            | Self::ToolInputEnded { session_id, .. }
            | Self::ToolCalled { session_id, .. }
            | Self::ToolProgress { session_id, .. }
            | Self::ToolFinished { session_id, .. }
            | Self::CompactionStarted { session_id, .. }
            | Self::CompactionDelta { session_id, .. }
            | Self::CompactionEnded { session_id, .. }
            | Self::CompactionFailed { session_id, .. }
            | Self::SkillActivated { session_id, .. }
            | Self::MessageUpsert { session_id, .. }
            | Self::MessageRemoved { session_id, .. }
            | Self::PartUpsert { session_id, .. }
            | Self::PermissionReplied { session_id, .. }
            | Self::TodoUpdated { session_id, .. }
            | Self::UsageUpdated { session_id, .. }
            | Self::ShellStarted { session_id, .. }
            | Self::ShellEnded { session_id, .. }
            | Self::Notice { session_id, .. } => Some(session_id),
            Self::PermissionAsked { request } => Some(&request.session_id),
            Self::ServerConnected | Self::Unknown { .. } => None,
        }
    }

    /// Coalescing key for high-frequency deltas. Events sharing a key within a
    /// frame are merged by the engine before they reach the transcript.
    pub fn coalesce_key(&self) -> Option<String> {
        match self {
            Self::TextDelta {
                message_id,
                part_id,
                ..
            } => Some(format!("text:{message_id}:{part_id}")),
            Self::ReasoningDelta {
                message_id,
                part_id,
                ..
            } => Some(format!("reasoning:{message_id}:{part_id}")),
            Self::ToolInputDelta { call_id, .. } => Some(format!("tool-input:{call_id}")),
            Self::CompactionDelta { session_id, .. } => Some(format!("compaction:{session_id}")),
            Self::TodoUpdated { session_id, .. } => Some(format!("todos:{session_id}")),
            _ => None,
        }
    }
}

/// Parse one wire event. Unknown event types become [`AgentEvent::Unknown`] so
/// diagnostics can report protocol drift instead of silently dropping frames.
pub fn parse_event(value: &Value) -> Option<AgentEvent> {
    let envelope = EventEnvelope::parse(value)?;
    Some(parse_envelope(&envelope))
}

pub fn parse_envelope(envelope: &EventEnvelope) -> AgentEvent {
    let data = &envelope.data;
    let at = envelope.created;
    let session_id = string_at(data, &["sessionID", "session_id", "sessionId"]);
    match envelope.event_type.as_str() {
        "server.connected" | "server.heartbeat" => AgentEvent::ServerConnected,
        "session.created" => {
            let info = data.get("info").unwrap_or(data);
            if let Some(session) = session_summary(info) {
                AgentEvent::SessionCreated { session }
            } else {
                AgentEvent::Unknown {
                    event_type: envelope.event_type.clone(),
                }
            }
        }
        "session.renamed" => AgentEvent::SessionRenamed {
            id: string_at(data, &["sessionID", "id"])
                .or(session_id.clone())
                .unwrap_or_default(),
            title: string_at(data, &["title"]).unwrap_or_default(),
            updated_at: at,
        },
        "session.deleted" => AgentEvent::SessionDeleted {
            id: string_at(data, &["sessionID", "id"]).unwrap_or_default(),
        },
        "session.retry.scheduled" => AgentEvent::RetryScheduled {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default(),
            attempt: data
                .get("attempt")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .min(u32::MAX as u64) as u32,
            message: failure_text(data.get("error"), "VELA_RETRY_SCHEDULED", "Retrying"),
            next: data
                .get("at")
                .or_else(|| data.get("next"))
                .and_then(Value::as_i64)
                .filter(|at| *at != 0),
            at,
        },
        "session.compaction.started" => AgentEvent::CompactionStarted {
            session_id: session_id.clone().unwrap_or_default(),
            at,
        },
        "session.compaction.delta" => AgentEvent::CompactionDelta {
            session_id: session_id.clone().unwrap_or_default(),
            text: string_at(data, &["text"]).unwrap_or_default(),
        },
        "session.compaction.ended" => AgentEvent::CompactionEnded {
            session_id: session_id.clone().unwrap_or_default(),
            text: string_at(data, &["text", "summary"]).unwrap_or_default(),
            at,
        },
        "session.compaction.failed" => AgentEvent::CompactionFailed {
            session_id: session_id.clone().unwrap_or_default(),
            error: failure_text(
                data.get("error"),
                "VELA_COMPACTION_FAILED",
                "Compaction failed",
            ),
            at,
        },
        "session.skill.activated" => {
            let id = string_at(data, &["id", "skillID", "skillId"]).unwrap_or_default();
            let name = string_at(data, &["name", "title"])
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| {
                    if id.is_empty() {
                        "Skill".into()
                    } else {
                        id.clone()
                    }
                });
            AgentEvent::SkillActivated {
                session_id: session_id.clone().unwrap_or_default(),
                id,
                name,
                text: string_at(data, &["text"]).unwrap_or_default(),
                at,
            }
        }
        "session.execution.started" => AgentEvent::Execution {
            session_id: session_id.clone().unwrap_or_default(),
            outcome: ExecutionOutcome::Started,
            message: None,
            at,
        },
        "session.execution.succeeded" => AgentEvent::ExecutionSucceeded {
            session_id: session_id.clone().unwrap_or_default(),
            at,
        },
        "session.execution.failed" => AgentEvent::ExecutionFailed {
            session_id: session_id.clone().unwrap_or_default(),
            message: failure_text(
                data.get("error"),
                "VELA_EXECUTION_FAILED",
                "Execution failed",
            ),
            at,
        },
        "session.step.failed" => AgentEvent::StepFailed {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default(),
            message: failure_text(data.get("error"), "VELA_STEP_FAILED", "Step failed"),
            at,
        },
        "session.error" => AgentEvent::ExecutionFailed {
            session_id: session_id.clone().unwrap_or_default(),
            message: failure_text(data.get("error"), "VELA_SESSION_ERROR", "Session failed"),
            at,
        },
        "session.execution.interrupted" => AgentEvent::Execution {
            session_id: session_id.clone().unwrap_or_default(),
            outcome: ExecutionOutcome::Interrupted,
            message: Some(
                data.get("error")
                    .filter(|error| !error.is_null())
                    .map(|error| {
                        failure_text(
                            Some(error),
                            "VELA_EXECUTION_INTERRUPTED",
                            "Execution interrupted",
                        )
                    })
                    .unwrap_or_else(|| "Interrupted".into()),
            ),
            at,
        },
        "session.idle" => AgentEvent::Idle {
            session_id: session_id.clone().unwrap_or_default(),
            at,
        },
        "session.status" => {
            let status = data.get("status").unwrap_or(data);
            let kind = status.get("type").and_then(Value::as_str).unwrap_or("idle");
            let phase = match kind {
                "busy" => SessionPhase::Busy,
                "retry" => SessionPhase::Retry,
                "error" => SessionPhase::Error,
                _ => SessionPhase::Idle,
            };
            AgentEvent::SessionStatus {
                session_id: session_id.clone().unwrap_or_default(),
                phase,
                message: if phase == SessionPhase::Retry {
                    Some(failure_text(
                        status.get("message"),
                        "VELA_RETRY_SCHEDULED",
                        "Retrying",
                    ))
                } else {
                    string_at(status, &["message"])
                },
                attempt: status
                    .get("attempt")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .min(u32::MAX as u64) as u32,
                next: status
                    .get("next")
                    .and_then(Value::as_i64)
                    .filter(|at| *at != 0),
                at,
            }
        }
        "session.step.started" => AgentEvent::StepStarted {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID", "id"])
                .unwrap_or_default(),
            agent: string_at(data, &["agent"]),
            model: string_at(data, &["model", "modelID"]),
            at,
        },
        "session.step.ended" => AgentEvent::StepEnded {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID", "id"])
                .unwrap_or_default(),
            finish: string_at(data, &["finish"]),
            at,
        },
        "session.text.started" | "session.text.delta" | "session.text.ended" => {
            let message_id =
                string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default();
            let ordinal = number_at(data, &["ordinal"]).unwrap_or(0) as u32;
            let part_id = string_at(data, &["partID"])
                .unwrap_or_else(|| format!("{message_id}:text:{ordinal}"));
            let session_id = session_id.clone().unwrap_or_default();
            match envelope.event_type.as_str() {
                "session.text.started" => AgentEvent::TextStarted {
                    session_id,
                    message_id,
                    part_id,
                    ordinal,
                    at,
                },
                "session.text.delta" => AgentEvent::TextDelta {
                    session_id,
                    message_id,
                    part_id,
                    ordinal,
                    delta: string_at(data, &["delta"]).unwrap_or_default(),
                    at,
                },
                _ => AgentEvent::TextEnded {
                    session_id,
                    message_id,
                    part_id,
                    ordinal,
                    text: string_at(data, &["text"]).unwrap_or_default(),
                    at,
                },
            }
        }
        "session.reasoning.started" | "session.reasoning.delta" | "session.reasoning.ended" => {
            let message_id =
                string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default();
            let ordinal = number_at(data, &["ordinal"]).unwrap_or(0) as u32;
            let part_id = string_at(data, &["partID"])
                .unwrap_or_else(|| format!("{message_id}:reasoning:{ordinal}"));
            let session_id = session_id.clone().unwrap_or_default();
            match envelope.event_type.as_str() {
                "session.reasoning.started" => AgentEvent::ReasoningStarted {
                    session_id,
                    message_id,
                    part_id,
                    ordinal,
                    at,
                },
                "session.reasoning.delta" => AgentEvent::ReasoningDelta {
                    session_id,
                    message_id,
                    part_id,
                    ordinal,
                    delta: string_at(data, &["delta"]).unwrap_or_default(),
                    at,
                },
                _ => AgentEvent::ReasoningEnded {
                    session_id,
                    message_id,
                    part_id,
                    ordinal,
                    text: string_at(data, &["text"]).unwrap_or_default(),
                    at,
                },
            }
        }
        "session.tool.input.started" | "session.tool.input.delta" | "session.tool.input.ended" => {
            let message_id =
                string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default();
            let call_id = string_at(data, &["id", "callID"]).unwrap_or_default();
            let session_id = session_id.clone().unwrap_or_default();
            match envelope.event_type.as_str() {
                "session.tool.input.started" => AgentEvent::ToolInputStarted {
                    session_id,
                    message_id,
                    call_id,
                    name: string_at(data, &["name"]).unwrap_or_default(),
                    at,
                },
                "session.tool.input.delta" => AgentEvent::ToolInputDelta {
                    session_id,
                    message_id,
                    call_id,
                    delta: string_at(data, &["delta"]).unwrap_or_default(),
                    at,
                },
                _ => AgentEvent::ToolInputEnded {
                    session_id,
                    message_id,
                    call_id,
                    input: string_at(data, &["text"]).unwrap_or_default(),
                    at,
                },
            }
        }
        "session.tool.called" => AgentEvent::ToolCalled {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default(),
            call_id: string_at(data, &["id", "callID"]).unwrap_or_default(),
            input: data
                .get("input")
                .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
                .unwrap_or_default(),
            at,
        },
        "session.tool.progress" => AgentEvent::ToolProgress {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default(),
            call_id: string_at(data, &["id", "callID"]).unwrap_or_default(),
            detail: data
                .get("metadata")
                .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
                .unwrap_or_default(),
        },
        "session.tool.success" => AgentEvent::ToolFinished {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default(),
            call_id: string_at(data, &["id", "callID"]).unwrap_or_default(),
            success: true,
            output: data
                .get("content")
                .map(tool_content_text)
                .unwrap_or_default(),
            files: data
                .get("content")
                .map(crate::transcript::tool_files_from_json)
                .unwrap_or_default(),
            error: None,
            at,
        },
        "session.tool.failed" => AgentEvent::ToolFinished {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["assistantMessageID", "messageID"]).unwrap_or_default(),
            call_id: string_at(data, &["id", "callID"]).unwrap_or_default(),
            success: false,
            output: data
                .get("content")
                .map(tool_content_text)
                .unwrap_or_default(),
            files: data
                .get("content")
                .map(crate::transcript::tool_files_from_json)
                .unwrap_or_default(),
            error: structured_message(data.get("error")),
            at,
        },
        "message.updated" => {
            let info = data.get("info").unwrap_or(data);
            match message_from_json(info) {
                Some(message) => AgentEvent::MessageUpsert {
                    session_id: session_id.clone().unwrap_or_default(),
                    message,
                },
                None => AgentEvent::Unknown {
                    event_type: envelope.event_type.clone(),
                },
            }
        }
        "message.removed" => AgentEvent::MessageRemoved {
            session_id: session_id.clone().unwrap_or_default(),
            message_id: string_at(data, &["messageID", "id"]).unwrap_or_default(),
        },
        "message.part.updated" => {
            let part_value = data.get("part").unwrap_or(data);
            let message_id =
                string_at(part_value, &["messageID", "message_id"]).unwrap_or_default();
            match part_from_json(part_value, &message_id) {
                Some(part) => AgentEvent::PartUpsert {
                    session_id: session_id.clone().unwrap_or_default(),
                    message_id,
                    part,
                    at,
                },
                None => AgentEvent::Unknown {
                    event_type: envelope.event_type.clone(),
                },
            }
        }
        "permission.asked" => {
            let request = data.get("request").unwrap_or(data);
            AgentEvent::PermissionAsked {
                request: PermissionRequest {
                    id: string_at(request, &["id", "requestID"]).unwrap_or_default(),
                    session_id: string_at(request, &["sessionID", "session_id"])
                        .unwrap_or_default(),
                    action: string_at(request, &["action", "permission"]).unwrap_or_default(),
                    resources: request
                        .get("resources")
                        .or_else(|| request.get("patterns"))
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    message: string_at(request, &["message"]),
                },
            }
        }
        "permission.replied" => AgentEvent::PermissionReplied {
            session_id: session_id.clone().unwrap_or_default(),
            request_id: string_at(data, &["requestID", "id"]).unwrap_or_default(),
            reply: string_at(data, &["reply", "decision"]).unwrap_or_default(),
        },
        "todo.updated" => {
            let todos = data
                .get("todos")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .enumerate()
                        .map(|(ix, item)| TodoItem {
                            id: item
                                .get("id")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                                .unwrap_or_else(|| format!("todo-{ix}")),
                            content: item
                                .get("content")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            status: item
                                .get("status")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            priority: item
                                .get("priority")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            AgentEvent::TodoUpdated {
                session_id: session_id.clone().unwrap_or_default(),
                todos,
            }
        }
        "session.usage.updated" | "session.usage.recorded" => AgentEvent::UsageUpdated {
            session_id: session_id.clone().unwrap_or_default(),
            usage: usage_from_json(data),
        },
        "session.shell.started" => AgentEvent::ShellStarted {
            session_id: session_id.clone().unwrap_or_default(),
            command: string_at(data, &["command"]).unwrap_or_default(),
            at,
        },
        "session.shell.ended" => AgentEvent::ShellEnded {
            session_id: session_id.clone().unwrap_or_default(),
            id: string_at(data, &["id", "shellID"]).unwrap_or_default(),
            output: string_at(data, &["output"]).unwrap_or_default(),
            exit_code: number_at(data, &["exitCode", "exit"]).unwrap_or(0) as i32,
        },
        "global.error" => AgentEvent::Notice {
            session_id: session_id.clone().unwrap_or_default(),
            text: structured_message(data.get("error"))
                .or_else(|| string_at(data, &["message"]))
                .unwrap_or_else(|| "Agent error".into()),
            error: true,
            at,
        },
        other => AgentEvent::Unknown {
            event_type: other.to_string(),
        },
    }
}

fn session_summary(value: &Value) -> Option<SessionSummary> {
    let id = string_at(value, &["id", "sessionID"])?;
    let directory = value
        .pointer("/location/directory")
        .and_then(Value::as_str)
        .or_else(|| value.get("directory").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    Some(SessionSummary {
        id,
        title: string_at(value, &["title"]).unwrap_or_default(),
        directory,
        updated_at: value
            .pointer("/time/updated")
            .and_then(Value::as_i64)
            .unwrap_or_else(now_millis),
        parent_id: string_at(value, &["parentID"]),
        agent: string_at(value, &["agent"]),
    })
}

fn usage_from_json(data: &Value) -> SessionUsage {
    let tokens = data.get("tokens").unwrap_or(data);
    SessionUsage {
        cost: data.get("cost").and_then(Value::as_f64).unwrap_or(0.0),
        input_tokens: number_at(tokens, &["input"]).unwrap_or(0) as u64,
        output_tokens: number_at(tokens, &["output"]).unwrap_or(0) as u64,
        reasoning_tokens: number_at(tokens, &["reasoning"]).unwrap_or(0) as u64,
        cache_read_tokens: number_at(tokens, &["cache", "read"])
            .or_else(|| number_at(tokens, &["cacheRead"]))
            .unwrap_or(0) as u64,
        cache_write_tokens: number_at(tokens, &["cache", "write"])
            .or_else(|| number_at(tokens, &["cacheWrite"]))
            .unwrap_or(0) as u64,
    }
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value.get(*key).and_then(Value::as_str)
            && !found.is_empty()
        {
            return Some(found.to_string());
        }
    }
    None
}

fn number_at(value: &Value, keys: &[&str]) -> Option<i64> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    current
        .as_i64()
        .or_else(|| current.as_f64().map(|value| value as i64))
}

fn structured_message(value: Option<&Value>) -> Option<String> {
    let value = value?;
    value
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("message"))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
}

/// Vela's shared/errors format: structured runtime codes, nested payloads and
/// already formatted markers retain their identity instead of becoming JSON.
pub fn failure_text(value: Option<&Value>, fallback_code: &str, fallback_message: &str) -> String {
    let source = value.and_then(Value::as_object);
    let nested = source.and_then(|source| {
        source
            .get("error")
            .and_then(Value::as_object)
            .or_else(|| source.get("data").and_then(Value::as_object))
    });
    let field = |key: &str| {
        source
            .and_then(|source| source.get(key))
            .or_else(|| nested.and_then(|nested| nested.get(key)))
    };
    let raw = field("message")
        .and_then(Value::as_str)
        .or_else(|| value.and_then(Value::as_str))
        .unwrap_or("")
        .trim();
    let marker = raw
        .strip_prefix('[')
        .and_then(|raw| raw.split_once(']'))
        .filter(|(code, _)| {
            code.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && code
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        });
    let message = marker
        .map(|(_, message)| message.trim())
        .filter(|message| !message.is_empty())
        .unwrap_or(if raw.is_empty() {
            fallback_message
        } else {
            raw
        });
    let code = field("code")
        .and_then(Value::as_str)
        .or_else(|| field("_tag").and_then(Value::as_str))
        .or_else(|| marker.map(|(code, _)| code))
        .unwrap_or(fallback_code);
    let normalize = |code: &str| {
        code.trim()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
    };
    let code = normalize(code);
    let code = if code.is_empty() {
        normalize(fallback_code)
    } else {
        code
    };
    let details = field("details")
        .filter(|value| !value.is_null())
        .and_then(|value| {
            if let Some(text) = value.as_str() {
                return (!text.trim().is_empty()).then(|| text.to_owned());
            }
            let text = value.to_string();
            (text != "{}").then_some(text)
        });
    format!(
        "[{code}] {message}{}",
        details
            .map(|details| format!(" — {details}"))
            .unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_completion_content_keeps_text_and_file_items() {
        let event = parse_event(&serde_json::json!({
            "type":"session.tool.success",
            "created":12,
            "data":{
                "sessionID":"ses_tool",
                "assistantMessageID":"msg_tool",
                "id":"call_tool",
                "content":[
                    {"type":"text","text":"ok"},
                    {"type":"file","uri":"https://example.com/report.csv","name":"report.csv","mime":"text/csv"},
                    {"type":"file","text":"not a file"}
                ]
            }
        }));
        let Some(AgentEvent::ToolFinished {
            output, files, at, ..
        }) = event
        else {
            panic!("expected tool completion")
        };
        assert_eq!(output, "ok");
        assert_eq!(at, 12);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].uri, "https://example.com/report.csv");
        assert_eq!(files[0].name.as_deref(), Some("report.csv"));
        assert_eq!(files[0].mime.as_deref(), Some("text/csv"));
    }

    #[test]
    fn runtime_failure_copy_preserves_source_codes_markers_nested_errors_and_details() {
        for (value, expected) in [
            (
                serde_json::json!("Slow down"),
                "[VELA_RETRY_SCHEDULED] Slow down",
            ),
            (
                serde_json::json!({"error":{"_tag":"Rate Limit","message":"Slow down","details":{"retry":2}}}),
                "[RATE_LIMIT] Slow down — {\"retry\":2}",
            ),
            (
                serde_json::json!({"error":"ignored", "data":{"message":"Nested"}}),
                "[VELA_RETRY_SCHEDULED] Nested",
            ),
            (
                serde_json::json!("[provider.limit] Try later"),
                "[PROVIDER.LIMIT] Try later",
            ),
        ] {
            assert_eq!(
                failure_text(Some(&value), "VELA_RETRY_SCHEDULED", "Retrying"),
                expected
            );
        }
        assert_eq!(
            failure_text(None, "VELA_RETRY_SCHEDULED", "Retrying"),
            "[VELA_RETRY_SCHEDULED] Retrying"
        );
    }

    #[test]
    fn parses_skill_activation_event() {
        let parsed = parse_event(&serde_json::json!({
            "type":"session.skill.activated","created":10,
            "data":{"sessionID":"ses_1","id":"review","name":"Review","text":"Checked"}
        }))
        .unwrap();
        match parsed {
            AgentEvent::SkillActivated {
                id, name, text, ..
            } => {
                assert_eq!(id, "review");
                assert_eq!(name, "Review");
                assert_eq!(text, "Checked");
            }
            _ => panic!("skill activation"),
        }
        let fallback = parse_event(&serde_json::json!({
            "type":"session.skill.activated","created":11,
            "data":{"sessionID":"ses_1","id":"review"}
        }))
        .unwrap();
        assert!(matches!(
            fallback,
            AgentEvent::SkillActivated { ref name, .. } if name == "review"
        ));
        let unnamed = parse_event(&serde_json::json!({
            "type":"session.skill.activated","created":12,
            "data":{"sessionID":"ses_1"}
        }))
        .unwrap();
        assert!(matches!(
            unnamed,
            AgentEvent::SkillActivated { ref name, .. } if name == "Skill"
        ));
    }

    #[test]
    fn parses_compaction_lifecycle_events() {
        let started = parse_event(&serde_json::json!({
            "type":"session.compaction.started","created":10,
            "data":{"sessionID":"ses_1"}
        }))
        .unwrap();
        assert!(matches!(
            started,
            AgentEvent::CompactionStarted { at: 10, .. }
        ));
        let delta = parse_event(&serde_json::json!({
            "type":"session.compaction.delta","created":11,
            "data":{"sessionID":"ses_1","text":"Summary"}
        }))
        .unwrap();
        assert!(matches!(
            delta,
            AgentEvent::CompactionDelta { ref text, .. } if text == "Summary"
        ));
        let ended = parse_event(&serde_json::json!({
            "type":"session.compaction.ended","created":20,
            "data":{"sessionID":"ses_1","text":"Final"}
        }))
        .unwrap();
        assert!(matches!(
            ended,
            AgentEvent::CompactionEnded { ref text, at: 20, .. } if text == "Final"
        ));
        let failed = parse_event(&serde_json::json!({
            "type":"session.compaction.failed","created":30,
            "data":{"sessionID":"ses_1","error":"compaction boom"}
        }))
        .unwrap();
        assert!(matches!(
            failed,
            AgentEvent::CompactionFailed { ref error, at: 30, .. }
                if error.contains("compaction boom")
        ));
    }

    #[test]
    fn parses_stream_delta_and_derives_part_id() {
        let value = serde_json::json!({
            "id": "evt",
            "created": 12,
            "type": "session.text.delta",
            "location": { "directory": "/repo" },
            "data": {
                "sessionID": "s1",
                "assistantMessageID": "m1",
                "ordinal": 2,
                "delta": "hello"
            }
        });
        let event = parse_event(&value).unwrap();
        assert_eq!(event.session_id(), Some("s1"));
        match event {
            AgentEvent::TextDelta {
                part_id, delta, at, ..
            } => {
                assert_eq!(part_id, "m1:text:2");
                assert_eq!(delta, "hello");
                assert_eq!(at, 12);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn accepts_legacy_properties_envelope() {
        let value = serde_json::json!({
            "type": "session.status",
            "properties": {
                "sessionID": "s1",
                "status": { "type": "busy" }
            }
        });
        match parse_event(&value).unwrap() {
            AgentEvent::SessionStatus { phase, .. } => assert_eq!(phase, SessionPhase::Busy),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn strips_numbered_event_suffix() {
        let value = serde_json::json!({
            "type": "message.part.updated.4",
            "data": {
                "part": { "id": "p1", "messageID": "m1", "type": "text", "text": "hi" }
            }
        });
        match parse_event(&value).unwrap() {
            AgentEvent::PartUpsert { part, .. } => assert_eq!(part.id, "p1"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn permission_payload_normalizes() {
        let value = serde_json::json!({
            "type": "permission.asked",
            "data": {
                "id": "req1", "sessionID": "s1", "action": "bash",
                "resources": ["rm -rf build"], "message": "Allow?"
            }
        });
        match parse_event(&value).unwrap() {
            AgentEvent::PermissionAsked { request } => {
                assert_eq!(request.id, "req1");
                assert_eq!(request.resources, vec!["rm -rf build"]);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
