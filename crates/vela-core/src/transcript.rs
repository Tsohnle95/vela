use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use serde_json::Value;

use crate::event::{AgentEvent, ExecutionOutcome};
use crate::markdown;
use crate::session::SessionPhase;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Shell,
    Compaction,
    Skill,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolStatus {
    #[default]
    None,
    Streaming,
    Running,
    Success,
    Failed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ToolFile {
    pub uri: String,
    pub name: Option<String>,
    pub mime: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ToolSubCall {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub status: ToolStatus,
    pub input: String,
    pub output: String,
    pub files: Vec<ToolFile>,
    pub sub_calls: Vec<ToolSubCall>,
}

#[derive(Clone, Debug)]
pub struct ToolPart {
    pub call_id: String,
    pub name: String,
    pub title: String,
    pub status: ToolStatus,
    pub input: String,
    pub output: String,
    pub detail: String,
    pub error: Option<String>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    /// Child session reported by a delegated task tool's metadata.
    pub child_session_id: Option<String>,
    /// File items carried by a tool result's `content` array.
    pub files: Vec<ToolFile>,
    /// Nested calls reported by `metadata.deepseek.subCalls`.
    pub sub_calls: Vec<ToolSubCall>,
}

impl Default for ToolPart {
    fn default() -> Self {
        Self {
            call_id: String::new(),
            name: String::new(),
            title: String::new(),
            status: ToolStatus::None,
            input: String::new(),
            output: String::new(),
            detail: String::new(),
            error: None,
            started_at: None,
            finished_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum PartKind {
    Text { text: String, streaming: bool },
    Reasoning { text: String, streaming: bool },
    Tool(Box<ToolPart>),
}

impl PartKind {
    pub fn text(&self) -> &str {
        match self {
            Self::Text { text, .. } | Self::Reasoning { text, .. } => text,
            Self::Tool(tool) => &tool.input,
        }
    }

    pub fn is_streaming(&self) -> bool {
        match self {
            Self::Text { streaming, .. } | Self::Reasoning { streaming, .. } => *streaming,
            Self::Tool(tool) => matches!(tool.status, ToolStatus::Streaming | ToolStatus::Running),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Part {
    pub id: String,
    pub ordinal: u32,
    pub kind: PartKind,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub parts: Vec<Part>,
    pub finish: Option<String>,
    pub error: Option<String>,
    pub retry: Option<RetryState>,
    pub skill_id: Option<String>,
    pub skill_name: Option<String>,
    pub attachments: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RetryState {
    pub attempt: u32,
    pub message: String,
    pub next: Option<i64>,
}

impl Message {
    pub fn new(id: impl Into<String>, role: Role, created_at: i64) -> Self {
        Self {
            id: id.into(),
            role,
            created_at,
            completed_at: None,
            agent: None,
            model: None,
            parts: Vec::new(),
            finish: None,
            error: None,
            retry: None,
            skill_id: None,
            skill_name: None,
            attachments: Vec::new(),
        }
    }

    pub fn text(
        id: impl Into<String>,
        role: Role,
        created_at: i64,
        text: impl Into<String>,
    ) -> Self {
        let id = id.into();
        let mut message = Self::new(id.clone(), role, created_at);
        message.parts.push(Part {
            id: format!("{id}:text:0"),
            ordinal: 0,
            kind: PartKind::Text {
                text: text.into(),
                streaming: false,
            },
        });
        message
    }

    pub fn part(&self, id: &str) -> Option<&Part> {
        self.parts.iter().find(|part| part.id == id)
    }

    pub fn part_mut(&mut self, id: &str) -> Option<&mut Part> {
        self.parts.iter_mut().find(|part| part.id == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockKind {
    User,
    Assistant,
    Reasoning,
    Tool,
    Shell,
    System,
    Compaction,
    Skill,
    Error,
    Retry,
    Activity,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: Arc<str>,
    pub message_id: Arc<str>,
    pub part_id: Arc<str>,
    pub kind: BlockKind,
    pub text: Arc<str>,
    /// Tool label / short detail (tool name, shell command).
    pub detail: Arc<str>,
    /// Tool output or status detail.
    pub output: Arc<str>,
    pub status: ToolStatus,
    pub streaming: bool,
    /// Agent id/name that produced this block, when known.
    pub agent: Option<Arc<str>>,
    /// Model id that produced this block, when known.
    pub model: Option<Arc<str>>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    /// Child session linked by a delegated task tool.
    pub child_session_id: Option<Arc<str>>,
    /// File items carried by a generic tool result.
    pub files: Vec<ToolFile>,
    /// Nested tool calls reported by tool metadata.
    pub sub_calls: Vec<ToolSubCall>,
    /// File names attached to a user prompt.
    pub attachments: Vec<Arc<str>>,
}

impl Block {
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.kind.hash(&mut hasher);
        self.text.hash(&mut hasher);
        self.detail.hash(&mut hasher);
        self.output.hash(&mut hasher);
        self.status.hash(&mut hasher);
        self.streaming.hash(&mut hasher);
        self.agent.hash(&mut hasher);
        self.model.hash(&mut hasher);
        self.created_at.hash(&mut hasher);
        self.completed_at.hash(&mut hasher);
        self.child_session_id.hash(&mut hasher);
        for file in &self.files {
            file.hash(&mut hasher);
        }
        for call in &self.sub_calls {
            call.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn is_user(&self) -> bool {
        self.kind == BlockKind::User
    }
}

#[derive(Clone, Debug, Default)]
struct PartBlocks {
    ranges: Vec<Range<usize>>,
    blocks: Vec<Block>,
    source_len: usize,
    /// Only populated for a streaming part. It lets the next delta prove the
    /// previous text is an unchanged prefix without storing a second copy of
    /// completed transcript text.
    source: String,
}

#[derive(Clone, Debug, Default)]
pub struct Transcript {
    messages: Vec<Message>,
    index: HashMap<String, usize>,
    part_blocks: HashMap<String, PartBlocks>,
    dirty_parts: HashSet<String>,
    blocks: Vec<Block>,
    block_spans: HashMap<String, Range<usize>>,
    revision: u64,
    needs_assemble: bool,
    phase: SessionPhase,
    busy: bool,
    execution_known: bool,
}

impl Transcript {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    pub fn phase(&self) -> SessionPhase {
        self.phase
    }

    pub fn set_phase(&mut self, phase: SessionPhase) {
        self.execution_known = true;
        if self.phase != phase {
            self.phase = phase;
            self.busy = phase.is_active();
            self.revision += 1;
        }
    }

    pub fn is_busy(&self) -> bool {
        self.busy || self.phase.is_active()
    }

    pub fn message(&self, id: &str) -> Option<&Message> {
        self.index.get(id).and_then(|ix| self.messages.get(*ix))
    }

    pub fn blocks(&mut self) -> &[Block] {
        self.flush_blocks();
        &self.blocks
    }

    /// Replace the transcript from an authoritative message replay.
    pub fn hydrate(&mut self, messages: Vec<Message>) {
        let live_compaction = self
            .messages
            .iter()
            .rev()
            .find(|message| message.role == Role::Compaction)
            .cloned();
        let live_skills = self
            .messages
            .iter()
            .filter(|message| message.role == Role::Skill)
            .cloned()
            .collect::<Vec<_>>();
        self.messages.clear();
        self.index.clear();
        for message in messages {
            self.index.insert(message.id.clone(), self.messages.len());
            self.messages.push(message);
        }
        if let Some(live) = live_compaction
            && (live.completed_at.is_none()
                || !self
                    .messages
                    .iter()
                    .any(|message| message.role == Role::Compaction))
        {
            self.index.insert(live.id.clone(), self.messages.len());
            self.messages.push(live);
        }
        for live in live_skills {
            if !self.index.contains_key(&live.id) {
                self.index.insert(live.id.clone(), self.messages.len());
                self.messages.push(live);
            }
        }
        if !self.execution_known {
            let trailing = self
                .messages
                .iter()
                .rev()
                .find(|message| message.role == Role::Assistant);
            self.phase = match trailing {
                Some(message) if message.retry.is_some() => SessionPhase::Retry,
                Some(message)
                    if message.completed_at.is_none()
                        && message.finish.is_none()
                        && message.error.is_none() =>
                {
                    SessionPhase::Busy
                }
                _ => SessionPhase::Idle,
            };
            self.busy = self.phase.is_active();
        }
        self.part_blocks.clear();
        self.dirty_parts.clear();
        self.blocks.clear();
        self.needs_assemble = true;
        self.revision += 1;
        for message in &self.messages {
            for part in &message.parts {
                self.dirty_parts.insert(part.id.clone());
            }
        }
        self.flush_blocks();
    }

    /// Insert an immediately visible user message while the prompt request is
    /// in flight. When the authoritative user message arrives, the local row is
    /// removed by matching text.
    pub fn upsert_local_user(
        &mut self,
        id: String,
        text: String,
        at: i64,
        attachments: Vec<String>,
    ) {
        let mut message = Message::text(id, Role::User, at, text);
        message.attachments = attachments;
        self.upsert_message(message);
        self.needs_assemble = true;
    }

    pub fn apply(&mut self, event: &AgentEvent) -> bool {
        let mut changed = true;
        match event {
            AgentEvent::StepStarted {
                session_id: _,
                message_id,
                agent,
                model,
                at,
            } => {
                if message_id.is_empty() {
                    return false;
                }
                let previous = self
                    .messages
                    .iter_mut()
                    .filter(|message| {
                        message.role == Role::Assistant
                            && message.id != *message_id
                            && message.completed_at.is_none()
                            && message.finish.is_none()
                    })
                    .map(|message| {
                        message.completed_at = Some(*at);
                        message.retry = None;
                        message.id.clone()
                    })
                    .collect::<Vec<_>>();
                for id in previous {
                    self.finish_message_parts(&id);
                }
                let message = self.ensure_message(message_id, Role::Assistant, *at);
                message.agent = agent.clone();
                message.model = model.clone();
                message.completed_at = None;
                message.finish = None;
                message.error = None;
                message.retry = None;
                self.needs_assemble = true;
                self.phase = SessionPhase::Busy;
                self.busy = true;
            }
            AgentEvent::StepEnded {
                session_id: _,
                message_id,
                finish,
                at,
            } => {
                let message = self.ensure_message(message_id, Role::Assistant, *at);
                message.completed_at = Some(*at);
                message.finish = finish.clone();
                message.retry = None;
                self.finish_message_parts(message_id);
                self.needs_assemble = true;
                if matches!(
                    finish.as_deref(),
                    Some("stop" | "length" | "content-filter" | "error" | "unknown")
                ) {
                    self.phase = SessionPhase::Idle;
                    self.busy = false;
                }
            }
            AgentEvent::TextStarted {
                message_id,
                part_id,
                ordinal,
                at,
                ..
            } => {
                self.ensure_text_part(message_id, part_id, *ordinal, *at, false, true);
            }
            AgentEvent::StepFailed {
                message_id,
                message: error,
                at,
                ..
            } => {
                if message_id.is_empty() {
                    return false;
                }
                let message = self.ensure_message(message_id, Role::Assistant, *at);
                message.completed_at = Some(*at);
                message.finish = Some("error".into());
                message.error = Some(error.clone());
                message.retry = None;
                self.finish_message_parts(message_id);
                self.needs_assemble = true;
                self.phase = SessionPhase::Error;
                self.busy = false;
            }
            AgentEvent::TextDelta {
                message_id,
                part_id,
                ordinal,
                delta,
                at,
                ..
            } => {
                self.push_text(message_id, part_id, *ordinal, *at, delta, false);
            }
            AgentEvent::TextEnded {
                message_id,
                part_id,
                ordinal,
                text,
                at,
                ..
            } => {
                self.finish_text(message_id, part_id, *ordinal, *at, text, false);
            }
            AgentEvent::ReasoningStarted {
                message_id,
                part_id,
                ordinal,
                at,
                ..
            } => {
                self.ensure_text_part(message_id, part_id, *ordinal, *at, true, true);
            }
            AgentEvent::ReasoningDelta {
                message_id,
                part_id,
                ordinal,
                delta,
                at,
                ..
            } => {
                self.push_text(message_id, part_id, *ordinal, *at, delta, true);
            }
            AgentEvent::ReasoningEnded {
                message_id,
                part_id,
                ordinal,
                text,
                at,
                ..
            } => {
                self.finish_text(message_id, part_id, *ordinal, *at, text, true);
            }
            AgentEvent::ToolInputStarted {
                message_id,
                call_id,
                name,
                at,
                ..
            } => {
                self.ensure_tool(message_id, call_id, *at, |tool| {
                    tool.name = name.clone();
                    tool.title = name.clone();
                    tool.status = ToolStatus::Streaming;
                });
            }
            AgentEvent::ToolInputDelta {
                message_id,
                call_id,
                delta,
                at,
                ..
            } => {
                self.ensure_tool(message_id, call_id, *at, |tool| {
                    tool.input.push_str(delta);
                    tool.status = ToolStatus::Streaming;
                });
            }
            AgentEvent::ToolInputEnded {
                message_id,
                call_id,
                input,
                at,
                ..
            } => {
                self.ensure_tool(message_id, call_id, *at, |tool| {
                    tool.input = input.clone();
                    tool.status = ToolStatus::Running;
                });
            }
            AgentEvent::ToolCalled {
                message_id,
                call_id,
                input,
                at,
                ..
            } => {
                self.ensure_tool(message_id, call_id, *at, |tool| {
                    tool.input = input.clone();
                    if tool.started_at.is_none() {
                        tool.started_at = Some(*at);
                    }
                    tool.status = ToolStatus::Running;
                });
            }
            AgentEvent::ToolProgress {
                message_id,
                call_id,
                detail,
                ..
            } => {
                self.ensure_tool(message_id, call_id, 0, |tool| {
                    tool.detail = detail.clone();
                    if let Ok(metadata) = serde_json::from_str::<Value>(detail) {
                        if let Some(id) = metadata
                            .get("sessionId")
                            .or_else(|| metadata.get("sessionID"))
                            .and_then(Value::as_str)
                            .filter(|id| !id.is_empty())
                        {
                            tool.child_session_id = Some(id.to_string());
                        }
                        let calls = tool_subcalls_from_metadata(&metadata);
                        if !calls.is_empty() {
                            tool.sub_calls = calls;
                        }
                    }
                });
            }
            AgentEvent::ToolFinished {
                message_id,
                call_id,
                success,
                output,
                files,
                error,
                at,
                ..
            } => {
                self.ensure_tool(message_id, call_id, *at, |tool| {
                    tool.status = if *success {
                        ToolStatus::Success
                    } else {
                        ToolStatus::Failed
                    };
                    tool.output = output.clone();
                    if !files.is_empty() {
                        tool.files = files.clone();
                    }
                    tool.error = error.clone();
                    tool.finished_at = Some(*at);
                    if tool.title.is_empty() {
                        tool.title = tool.name.clone();
                    }
                });
            }
            AgentEvent::MessageUpsert { message, .. } => {
                self.upsert_message(message.clone());
            }
            AgentEvent::MessageRemoved { message_id, .. } => {
                changed = self.remove_message(message_id);
            }
            AgentEvent::PartUpsert {
                message_id,
                part,
                at,
                ..
            } => {
                self.upsert_part(message_id, part.clone(), *at);
            }
            AgentEvent::ShellStarted {
                session_id,
                command,
                at,
            } => {
                let id = format!("shell:{session_id}:{at}");
                let mut message = Message::text(id.clone(), Role::Shell, *at, command.clone());
                message.error = None;
                self.upsert_message(message);
            }
            AgentEvent::ShellEnded {
                id,
                output,
                exit_code,
                ..
            } => {
                let message = Message::text(id.clone(), Role::Shell, 0, String::new());
                let mut message = message;
                message.parts.clear();
                message.parts.push(Part {
                    id: format!("{id}:shell"),
                    ordinal: 0,
                    kind: PartKind::Tool(Box::new(ToolPart {
                        call_id: id.clone(),
                        name: "shell".into(),
                        title: "shell".into(),
                        status: if *exit_code == 0 {
                            ToolStatus::Success
                        } else {
                            ToolStatus::Failed
                        },
                        input: String::new(),
                        output: output.clone(),
                        detail: String::new(),
                        error: None,
                        started_at: None,
                        finished_at: Some(0),
                        child_session_id: None,
                        files: Vec::new(),
                        sub_calls: Vec::new(),
                    })),
                });
                self.upsert_message(message);
            }
            AgentEvent::Notice {
                session_id,
                text,
                error,
                at,
            } => {
                let id = format!("notice:{session_id}:{at}:{}", self.messages.len());
                let mut message = Message::text(id, Role::System, *at, text.clone());
                if *error {
                    message.error = Some(text.clone());
                }
                self.upsert_message(message);
            }
            AgentEvent::RetryScheduled {
                message_id,
                attempt,
                message,
                next,
                at,
                ..
            } => {
                if message_id.is_empty() {
                    return false;
                }
                let assistant = self.ensure_message(message_id, Role::Assistant, *at);
                assistant.retry = Some(RetryState {
                    attempt: *attempt,
                    message: message.clone(),
                    next: *next,
                });
                self.needs_assemble = true;
                self.phase = SessionPhase::Retry;
                self.busy = true;
            }
            AgentEvent::CompactionStarted { session_id, at } => {
                self.start_compaction(session_id, *at);
            }
            AgentEvent::CompactionDelta { session_id, text } => {
                self.append_compaction(session_id, text, 0);
            }
            AgentEvent::CompactionEnded {
                session_id,
                text,
                at,
            } => {
                self.finish_compaction(session_id, *at, text, None);
            }
            AgentEvent::CompactionFailed {
                session_id,
                error,
                at,
            } => {
                self.finish_compaction(session_id, *at, "", Some(error));
            }
            AgentEvent::SkillActivated {
                session_id,
                id,
                name,
                text,
                at,
            } => {
                let message_id =
                    format!("skill:{session_id}:{at}:{}", self.messages.len());
                let mut message = Message::text(message_id, Role::Skill, *at, text.clone());
                message.completed_at = Some(*at);
                message.skill_id = Some(id.clone());
                message.skill_name = Some(name.clone());
                self.upsert_message(message);
            }
            AgentEvent::Execution {
                outcome,
                message,
                at,
                ..
            } => {
                self.phase = match outcome {
                    ExecutionOutcome::Started => SessionPhase::Busy,
                    ExecutionOutcome::Succeeded => SessionPhase::Idle,
                    ExecutionOutcome::Failed | ExecutionOutcome::Interrupted => SessionPhase::Error,
                };
                self.busy = self.phase.is_active();
                if !self.busy {
                    self.complete_latest_incomplete(*at, None);
                }
                if *outcome == ExecutionOutcome::Interrupted {
                    self.push_notice(message.as_deref().unwrap_or("Interrupted"), *at, false);
                }
            }
            AgentEvent::ExecutionSucceeded { at, .. } => {
                self.complete_latest_incomplete(*at, None);
                self.phase = SessionPhase::Idle;
                self.busy = false;
            }
            AgentEvent::Idle { at, .. } => {
                self.complete_latest_incomplete(*at, None);
                self.phase = SessionPhase::Idle;
                self.busy = false;
            }
            AgentEvent::ExecutionFailed {
                session_id,
                message,
                at,
            } => {
                let _ = session_id;
                self.phase = SessionPhase::Error;
                self.busy = false;
                if !self.complete_latest_incomplete(*at, Some(message)) {
                    self.push_notice(message, *at, true);
                }
            }
            AgentEvent::SessionStatus {
                phase,
                message,
                attempt,
                next,
                ..
            } => {
                self.phase = *phase;
                self.busy = phase.is_active();
                if let Some(assistant) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|message| message.role == Role::Assistant)
                {
                    if *phase == SessionPhase::Retry {
                        assistant.retry = Some(RetryState {
                            attempt: *attempt,
                            next: *next,
                            message: message
                                .clone()
                                .unwrap_or_else(|| "[VELA_RETRY_SCHEDULED] Retrying".into()),
                        });
                        self.needs_assemble = true;
                    } else if assistant.retry.take().is_some() {
                        self.needs_assemble = true;
                    }
                }
            }
            _ => changed = false,
        }
        if changed {
            if matches!(
                event,
                AgentEvent::StepStarted { .. }
                    | AgentEvent::StepEnded { .. }
                    | AgentEvent::StepFailed { .. }
                    | AgentEvent::Execution { .. }
                    | AgentEvent::ExecutionSucceeded { .. }
                    | AgentEvent::ExecutionFailed { .. }
                    | AgentEvent::Idle { .. }
                    | AgentEvent::SessionStatus { .. }
                    | AgentEvent::RetryScheduled { .. }
            ) {
                self.execution_known = true;
            }
            self.revision += 1;
        }
        changed
    }

    /// Source settlement thresholds: incomplete idle turns settle after 2s;
    /// silent busy turns recover after 60s. A near-future retry stays active.
    /// Returns whether a user-facing stalled-turn notification is needed.
    pub fn settle_quiet(&mut self, quiet_ms: u64, now: i64) -> Option<bool> {
        let busy = self.is_busy();
        let latest = self
            .messages
            .iter()
            .rev()
            .find(|message| message.role == Role::Assistant);
        let incomplete = latest.is_some_and(|message| {
            message.completed_at.is_none() && message.finish.is_none() && message.error.is_none()
        });
        if !incomplete {
            if !busy || quiet_ms < 60_000 {
                return None;
            }
        } else if !busy {
            if quiet_ms < 2_000 {
                return None;
            }
            self.complete_latest_incomplete(now, None);
        } else {
            if latest
                .and_then(|message| message.retry.as_ref())
                .and_then(|retry| retry.next)
                .is_some_and(|next| next > now && next.saturating_sub(now) < 60_000)
                || quiet_ms < 60_000
            {
                return None;
            }
            self.complete_latest_incomplete(now, None);
        }
        self.phase = SessionPhase::Idle;
        self.busy = false;
        self.execution_known = true;
        self.revision += 1;
        Some(busy)
    }

    fn push_notice(&mut self, text: &str, at: i64, error: bool) {
        let id = format!("notice:{}:{}", self.messages.len(), at);
        let mut message = Message::text(id, Role::System, at, text);
        if error {
            message.error = Some(text.to_string());
        }
        self.upsert_message(message);
    }

    fn complete_latest_incomplete(&mut self, at: i64, error: Option<&str>) -> bool {
        let Some(message) = self.messages.iter_mut().rev().find(|message| {
            message.role == Role::Assistant
                && ((message.completed_at.is_none() && message.finish.is_none())
                    || message.retry.is_some())
        }) else {
            return false;
        };
        let incomplete = message.completed_at.is_none() && message.finish.is_none();
        if incomplete {
            message.completed_at = Some(at);
        }
        if incomplete && let Some(error) = error {
            message.error = Some(error.to_owned());
            self.needs_assemble = true;
        }
        if message.retry.take().is_some() {
            self.needs_assemble = true;
        }
        let id = message.id.clone();
        self.finish_message_parts(&id);
        self.needs_assemble = true;
        true
    }

    fn finish_message_parts(&mut self, id: &str) {
        let Some(index) = self.index.get(id).copied() else {
            return;
        };
        for part in &mut self.messages[index].parts {
            match &mut part.kind {
                PartKind::Text { streaming, .. } | PartKind::Reasoning { streaming, .. } => {
                    *streaming = false
                }
                PartKind::Tool(_) => {}
            }
            self.dirty_parts.insert(part.id.clone());
        }
    }

    fn ensure_message(&mut self, id: &str, role: Role, at: i64) -> &mut Message {
        if let Some(ix) = self.index.get(id).copied() {
            return &mut self.messages[ix];
        }
        let message = Message::new(id, role, at);
        self.index.insert(id.to_string(), self.messages.len());
        self.messages.push(message);
        self.needs_assemble = true;
        self.messages.last_mut().expect("just pushed")
    }

    fn ensure_text_part(
        &mut self,
        message_id: &str,
        part_id: &str,
        ordinal: u32,
        at: i64,
        reasoning: bool,
        streaming: bool,
    ) {
        let missing = self
            .message(message_id)
            .and_then(|message| message.part(part_id))
            .is_none();
        if missing {
            self.needs_assemble = true;
        }
        let message = self.ensure_message(message_id, Role::Assistant, at);
        if message.part(part_id).is_none() {
            message.parts.push(Part {
                id: part_id.to_string(),
                ordinal,
                kind: if reasoning {
                    PartKind::Reasoning {
                        text: String::new(),
                        streaming,
                    }
                } else {
                    PartKind::Text {
                        text: String::new(),
                        streaming,
                    }
                },
            });
        }
        self.dirty_parts.insert(part_id.to_string());
    }

    fn push_text(
        &mut self,
        message_id: &str,
        part_id: &str,
        ordinal: u32,
        at: i64,
        delta: &str,
        reasoning: bool,
    ) {
        self.ensure_text_part(message_id, part_id, ordinal, at, reasoning, true);
        let message = self.ensure_message(message_id, Role::Assistant, at);
        if let Some(part) = message.part_mut(part_id) {
            match &mut part.kind {
                PartKind::Text { text, streaming } => {
                    text.push_str(delta);
                    *streaming = true;
                }
                PartKind::Reasoning { text, streaming } => {
                    text.push_str(delta);
                    *streaming = true;
                }
                _ => {}
            }
        }
        self.dirty_parts.insert(part_id.to_string());
    }

    fn finish_text(
        &mut self,
        message_id: &str,
        part_id: &str,
        ordinal: u32,
        at: i64,
        value: &str,
        reasoning: bool,
    ) {
        self.ensure_text_part(message_id, part_id, ordinal, at, reasoning, false);
        let message = self.ensure_message(message_id, Role::Assistant, at);
        if let Some(part) = message.part_mut(part_id) {
            match &mut part.kind {
                PartKind::Text { text, streaming } => {
                    *text = value.to_string();
                    *streaming = false;
                }
                PartKind::Reasoning { text, streaming } => {
                    *text = value.to_string();
                    *streaming = false;
                }
                _ => {}
            }
        }
        self.dirty_parts.insert(part_id.to_string());
    }

    fn start_compaction(&mut self, session_id: &str, at: i64) {
        let id = format!("compaction:{session_id}:{at}:{}", self.messages.len());
        let mut message = Message::text(id, Role::Compaction, at, "");
        if let Some(part) = message.parts.first_mut()
            && let PartKind::Text { streaming, .. } = &mut part.kind
        {
            *streaming = true;
        }
        self.upsert_message(message);
    }

    fn ensure_compaction(&mut self, session_id: &str, at: i64) -> usize {
        if let Some(index) = self
            .messages
            .iter()
            .rposition(|message| message.role == Role::Compaction)
        {
            return index;
        }
        let id = format!("compaction:{session_id}:{at}:{}", self.messages.len());
        let mut message = Message::text(id, Role::Compaction, at, "");
        if let Some(part) = message.parts.first_mut()
            && let PartKind::Text { streaming, .. } = &mut part.kind
        {
            *streaming = true;
        }
        self.upsert_message(message);
        self.messages.len() - 1
    }

    fn append_compaction(&mut self, session_id: &str, text: &str, at: i64) {
        let index = self.ensure_compaction(session_id, at);
        let mut message = self.messages[index].clone();
        if let Some(part) = message.parts.first_mut()
            && let PartKind::Text { text: body, .. } = &mut part.kind
        {
            body.push_str(text);
        }
        self.upsert_message(message);
    }

    fn finish_compaction(
        &mut self,
        session_id: &str,
        at: i64,
        text: &str,
        error: Option<&str>,
    ) {
        let index = self.ensure_compaction(session_id, at);
        let mut message = self.messages[index].clone();
        if let Some(part) = message.parts.first_mut()
            && let PartKind::Text {
                text: body,
                streaming,
            } = &mut part.kind
        {
            if !text.is_empty() {
                *body = text.to_string();
            }
            *streaming = false;
        }
        message.completed_at = Some(at);
        message.error = error.map(str::to_string);
        self.upsert_message(message);
    }

    fn ensure_tool(
        &mut self,
        message_id: &str,
        call_id: &str,
        at: i64,
        update: impl FnOnce(&mut ToolPart),
    ) {
        let message = self.ensure_message(message_id, Role::Assistant, at);
        let existing = message.parts.iter_mut().find(|part| {
            part.id == call_id
                || matches!(&part.kind, PartKind::Tool(tool) if tool.call_id == call_id)
        });
        let (part_id, inserted) = if let Some(part) = existing {
            if let PartKind::Tool(tool) = &mut part.kind {
                update(tool);
            }
            (part.id.clone(), false)
        } else {
            let mut tool = ToolPart {
                call_id: call_id.to_string(),
                ..Default::default()
            };
            update(&mut tool);
            message.parts.push(Part {
                id: call_id.to_string(),
                ordinal: message.parts.len() as u32,
                kind: PartKind::Tool(Box::new(tool)),
            });
            (call_id.to_string(), true)
        };
        self.needs_assemble |= inserted;
        self.dirty_parts.insert(part_id);
    }

    fn upsert_message(&mut self, message: Message) {
        let new_part_ids: Vec<String> = message.parts.iter().map(|part| part.id.clone()).collect();
        if message.role == Role::User && !message.id.starts_with("local:") {
            let text = message
                .parts
                .first()
                .map(|part| part.kind.text().to_string());
            if let Some(text) = text {
                let duplicates: Vec<String> = self
                    .messages
                    .iter()
                    .filter(|existing| {
                        existing.id.starts_with("local:")
                            && existing
                                .parts
                                .first()
                                .is_some_and(|part| part.kind.text() == text)
                    })
                    .map(|existing| existing.id.clone())
                    .collect();
                for id in duplicates {
                    self.remove_message(&id);
                }
            }
        }
        if let Some(ix) = self.index.get(&message.id).copied() {
            let old = std::mem::replace(&mut self.messages[ix], message);
            for part in old.parts {
                self.part_blocks.remove(&part.id);
                self.dirty_parts.remove(&part.id);
            }
        } else {
            self.index.insert(message.id.clone(), self.messages.len());
            self.messages.push(message);
        }
        for id in new_part_ids {
            self.dirty_parts.insert(id);
        }
        self.needs_assemble = true;
    }

    fn upsert_part(&mut self, message_id: &str, mut part: Part, at: i64) {
        let part_id = part.id.clone();
        let message = self.ensure_message(message_id, Role::Assistant, at);
        let old_id = if let Some(existing) = message.parts.iter_mut().find(|existing| {
            existing.id == part_id
                || matches!((&existing.kind, &part.kind),
                (PartKind::Tool(left), PartKind::Tool(right)) if left.call_id == right.call_id)
        }) {
            let old_id = existing.id.clone();
            part.ordinal = existing.ordinal;
            *existing = part;
            Some(old_id)
        } else {
            message.parts.push(part);
            None
        };
        if let Some(old_id) = old_id.filter(|old_id| *old_id != part_id) {
            self.part_blocks.remove(&old_id);
            self.dirty_parts.remove(&old_id);
        }
        self.dirty_parts.insert(part_id);
        self.needs_assemble = true;
    }

    fn remove_message(&mut self, message_id: &str) -> bool {
        let Some(ix) = self.index.get(message_id).copied() else {
            return false;
        };
        let removed = self.messages.remove(ix);
        for part in removed.parts {
            self.part_blocks.remove(&part.id);
            self.dirty_parts.remove(&part.id);
        }
        self.index.clear();
        for (ix, message) in self.messages.iter().enumerate() {
            self.index.insert(message.id.clone(), ix);
        }
        self.rebuild_all_blocks();
        self.needs_assemble = true;
        true
    }

    fn rebuild_all_blocks(&mut self) {
        for message in &self.messages {
            for part in &message.parts {
                self.dirty_parts.insert(part.id.clone());
            }
        }
        self.flush_blocks();
    }

    fn flush_blocks(&mut self) {
        if self.dirty_parts.is_empty() {
            if self.needs_assemble {
                self.assemble();
                self.needs_assemble = false;
            }
            return;
        }
        let dirty: Vec<(usize, usize)> = self
            .messages
            .iter()
            .enumerate()
            .flat_map(|(message_ix, message)| {
                message
                    .parts
                    .iter()
                    .enumerate()
                    .map(move |(part_ix, part)| (message_ix, part_ix, part.id.clone()))
            })
            .filter(|(_, _, id)| self.dirty_parts.contains(id))
            .map(|(message_ix, part_ix, _)| (message_ix, part_ix))
            .collect();
        let mut rebuilt = Vec::with_capacity(dirty.len());
        for (message_ix, part_ix) in dirty {
            let message = &self.messages[message_ix];
            let part = &message.parts[part_ix];
            let mut next = Self::build_part_blocks(message, part, self.part_blocks.get(&part.id));
            for block in &mut next.blocks {
                if !matches!(block.kind, BlockKind::Tool | BlockKind::Shell) {
                    block.completed_at = message.completed_at;
                    if message.completed_at.is_some() {
                        block.streaming = false;
                    }
                }
            }
            rebuilt.push((part.id.clone(), next));
        }
        let mut structural = self.needs_assemble;
        for (part_id, next) in rebuilt {
            let old_len = self
                .part_blocks
                .get(&part_id)
                .map(|cached| cached.blocks.len())
                .unwrap_or(0);
            let new_len = next.blocks.len();
            if (old_len == 0) != (new_len == 0) {
                structural = true;
            }
            self.part_blocks.insert(part_id.clone(), next);
            if structural {
                continue;
            }
            let Some(span) = self.block_spans.get(&part_id).cloned() else {
                structural = true;
                continue;
            };
            let replacement: Vec<Block> = self
                .part_blocks
                .get(&part_id)
                .map(|cached| cached.blocks.clone())
                .unwrap_or_default();
            self.blocks.splice(span.clone(), replacement);
            let delta = new_len as isize - old_len as isize;
            if delta != 0 {
                for (id, other) in self.block_spans.iter_mut() {
                    if id != &part_id && other.start >= span.end {
                        other.start = (other.start as isize + delta).max(0) as usize;
                        other.end = (other.end as isize + delta).max(0) as usize;
                    }
                }
            }
            self.block_spans.insert(
                part_id,
                span.start..(span.start as isize + new_len as isize).max(0) as usize,
            );
        }
        self.dirty_parts.clear();
        if structural {
            self.assemble();
        }
        self.needs_assemble = false;
    }

    fn assemble(&mut self) {
        let mut blocks = Vec::new();
        let mut spans = HashMap::new();
        let mut start = 0usize;
        for message in &self.messages {
            let before_parts = start;
            for part in &message.parts {
                if let Some(cached) = self.part_blocks.get(&part.id) {
                    spans.insert(part.id.clone(), start..start + cached.blocks.len());
                    start += cached.blocks.len();
                    blocks.extend(cached.blocks.iter().cloned());
                }
            }
            if message.role == Role::Assistant && start == before_parts {
                blocks.push(Block {
                    id: Arc::from(format!("{}:activity", message.id)),
                    message_id: Arc::from(message.id.as_str()),
                    part_id: Arc::from(format!("{}:activity", message.id)),
                    kind: BlockKind::Activity,
                    text: Arc::from(""),
                    detail: Arc::from(if message.parts.is_empty() {
                        ""
                    } else {
                        "parts"
                    }),
                    output: Arc::from(""),
                    status: ToolStatus::None,
                    streaming: message.completed_at.is_none(),
                    agent: None,
                    model: None,
                    created_at: message.created_at,
                    completed_at: message.completed_at,
                    child_session_id: None,
                    files: Vec::new(),
                    sub_calls: Vec::new(),
                    attachments: Vec::new(),
                });
                start += 1;
            }
            if message.role == Role::Assistant
                && let Some(error) = &message.error
            {
                blocks.push(Block {
                    id: Arc::from(format!("{}:error", message.id)),
                    message_id: Arc::from(message.id.as_str()),
                    part_id: Arc::from(format!("{}:error", message.id)),
                    kind: BlockKind::Error,
                    text: Arc::from(error.as_str()),
                    detail: Arc::from(""),
                    output: Arc::from(""),
                    status: ToolStatus::Failed,
                    streaming: false,
                    agent: None,
                    model: None,
                    created_at: message.created_at,
                    completed_at: message.completed_at,
                    child_session_id: None,
                    files: Vec::new(),
                    sub_calls: Vec::new(),
                    attachments: Vec::new(),
                });
                start += 1;
            }
            if let Some(retry) = &message.retry {
                blocks.push(Block {
                    id: Arc::from(format!("{}:retry", message.id)),
                    message_id: Arc::from(message.id.as_str()),
                    part_id: Arc::from(format!("{}:retry", message.id)),
                    kind: BlockKind::Retry,
                    text: Arc::from(retry.message.as_str()),
                    detail: Arc::from(format!("attempt {}", retry.attempt)),
                    output: Arc::from(""),
                    status: ToolStatus::None,
                    streaming: false,
                    agent: None,
                    model: None,
                    created_at: message.created_at,
                    completed_at: retry.next,
                    child_session_id: None,
                    files: Vec::new(),
                    sub_calls: Vec::new(),
                    attachments: Vec::new(),
                });
                start += 1;
            }
        }
        self.blocks = blocks;
        self.block_spans = spans;
    }

    fn build_part_blocks(
        message: &Message,
        part: &Part,
        existing: Option<&PartBlocks>,
    ) -> PartBlocks {
        let mut result = Self::build_part_blocks_inner(message, part, existing);
        for block in &mut result.blocks {
            block.agent = message.agent.as_deref().map(Arc::from);
            block.model = message.model.as_deref().map(Arc::from);
        }
        result
    }

    fn build_part_blocks_inner(
        message: &Message,
        part: &Part,
        existing: Option<&PartBlocks>,
    ) -> PartBlocks {
        match &part.kind {
            PartKind::Text { text, streaming } => {
                if message.role == Role::User {
                    let block = Block {
                        id: Arc::from(format!("{}#0", part.id)),
                        message_id: Arc::from(message.id.as_str()),
                        part_id: Arc::from(part.id.as_str()),
                        kind: BlockKind::User,
                        text: Arc::from(text.as_str()),
                        detail: Arc::from(""),
                        output: Arc::from(""),
                        status: ToolStatus::None,
                        streaming: false,
                        agent: None,
                        model: None,
                        created_at: message.created_at,
                        completed_at: None,
                        child_session_id: None,
                        files: Vec::new(),
                        sub_calls: Vec::new(),
                        attachments: message.attachments.iter().cloned().map(Arc::from).collect(),
                    };
                    return PartBlocks {
                        ranges: std::iter::once(0..text.len()).collect(),
                        blocks: vec![block],
                        source_len: text.len(),
                        source: String::new(),
                    };
                }
                if message.role == Role::Compaction {
                    let block = Block {
                        id: Arc::from(part.id.as_str()),
                        message_id: Arc::from(message.id.as_str()),
                        part_id: Arc::from(part.id.as_str()),
                        kind: BlockKind::Compaction,
                        text: Arc::from(text.as_str()),
                        detail: Arc::from(""),
                        output: Arc::from(message.error.as_deref().unwrap_or("")),
                        status: if message.error.is_some() {
                            ToolStatus::Failed
                        } else if message.completed_at.is_none() {
                            ToolStatus::Running
                        } else {
                            ToolStatus::Success
                        },
                        streaming: message.completed_at.is_none(),
                        agent: None,
                        model: None,
                        created_at: message.created_at,
                        completed_at: message.completed_at,
                        child_session_id: None,
                        files: Vec::new(),
                        sub_calls: Vec::new(),
                        attachments: Vec::new(),
                    };
                    return PartBlocks {
                        ranges: std::iter::once(0..text.len()).collect(),
                        blocks: vec![block],
                        source_len: text.len(),
                        source: String::new(),
                    };
                }
                if message.role == Role::Skill {
                    let block = Block {
                        id: Arc::from(part.id.as_str()),
                        message_id: Arc::from(message.id.as_str()),
                        part_id: Arc::from(part.id.as_str()),
                        kind: BlockKind::Skill,
                        text: Arc::from(text.as_str()),
                        detail: Arc::from(message.skill_id.as_deref().unwrap_or("")),
                        output: Arc::from(message.skill_name.as_deref().unwrap_or("")),
                        status: ToolStatus::None,
                        streaming: false,
                        agent: None,
                        model: None,
                        created_at: message.created_at,
                        completed_at: message.completed_at,
                        child_session_id: None,
                        files: Vec::new(),
                        sub_calls: Vec::new(),
                        attachments: Vec::new(),
                    };
                    return PartBlocks {
                        ranges: std::iter::once(0..text.len()).collect(),
                        blocks: vec![block],
                        source_len: text.len(),
                        source: String::new(),
                    };
                }
                let kind = if message.role == Role::System {
                    BlockKind::System
                } else {
                    BlockKind::Assistant
                };
                build_text_blocks(message, part, text, *streaming, kind, existing)
            }
            PartKind::Reasoning { text, streaming } => {
                let blocks = if text.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![Block {
                        id: Arc::from(part.id.as_str()),
                        message_id: Arc::from(message.id.as_str()),
                        part_id: Arc::from(part.id.as_str()),
                        kind: BlockKind::Reasoning,
                        text: Arc::from(text.as_str()),
                        detail: Arc::from("Thinking"),
                        output: Arc::from(""),
                        status: ToolStatus::None,
                        streaming: *streaming,
                        agent: None,
                        model: None,
                        created_at: message.created_at,
                        completed_at: None,
                        child_session_id: None,
                        files: Vec::new(),
                        sub_calls: Vec::new(),
                        attachments: Vec::new(),
                    }]
                };
                PartBlocks {
                    ranges: Vec::new(),
                    blocks,
                    source_len: text.len(),
                    source: String::new(),
                }
            }
            PartKind::Tool(tool) => {
                let kind = match message.role {
                    Role::Shell => BlockKind::Shell,
                    _ => BlockKind::Tool,
                };
                let detail = if !tool.title.is_empty() {
                    tool.title.clone()
                } else if !tool.name.is_empty() {
                    tool.name.clone()
                } else {
                    tool.call_id.clone()
                };
                let output = if !tool.output.is_empty() {
                    tool.output.clone()
                } else if let Some(error) = &tool.error {
                    error.clone()
                } else {
                    tool.detail.clone()
                };
                let blocks = vec![Block {
                    id: Arc::from(part.id.as_str()),
                    message_id: Arc::from(message.id.as_str()),
                    part_id: Arc::from(part.id.as_str()),
                    kind,
                    text: Arc::from(tool.input.as_str()),
                    detail: Arc::from(detail),
                    output: Arc::from(output),
                    status: tool.status,
                    streaming: matches!(tool.status, ToolStatus::Streaming | ToolStatus::Running),
                    agent: None,
                    model: None,
                    created_at: tool.started_at.unwrap_or(message.created_at),
                    completed_at: tool.finished_at,
                    child_session_id: tool.child_session_id.as_deref().map(Arc::from),
                    files: tool.files.clone(),
                    sub_calls: tool.sub_calls.clone(),
                    attachments: Vec::new(),
                }];
                PartBlocks {
                    ranges: Vec::new(),
                    blocks,
                    source_len: 0,
                    source: String::new(),
                }
            }
        }
    }
}

fn build_text_blocks(
    message: &Message,
    part: &Part,
    text: &str,
    streaming: bool,
    kind: BlockKind,
    existing: Option<&PartBlocks>,
) -> PartBlocks {
    let mut reuse = 0usize;
    if let Some(existing) = existing {
        let candidates = existing.ranges.len().saturating_sub(1);
        if existing.source_len <= text.len() {
            for (ix, range) in existing.ranges.iter().take(candidates).enumerate() {
                if range.end <= text.len()
                    && existing
                        .blocks
                        .get(ix)
                        .is_some_and(|block| &text[range.clone()] == block.text.as_ref())
                {
                    reuse += 1;
                } else {
                    break;
                }
            }
        }
    }

    // Streaming fast path: an append that cannot introduce a new top-level
    // boundary extends the last block in place. This keeps per-delta cost
    // proportional to the appended text plus one block copy, not a rescan of
    // the whole response.
    if streaming
        && let Some(cache) = existing
        && !cache.source.is_empty()
        && text.len() >= cache.source.len()
        && text.starts_with(cache.source.as_str())
        && !cache.blocks.is_empty()
    {
        let suffix = &text[cache.source.len()..];
        let structural = suffix.contains("\n\n")
            || suffix.contains("```")
            || suffix.contains("~~~")
            || suffix.lines().any(|line| {
                let line = line.trim_start();
                let hashes = line.chars().take_while(|c| *c == '#').count();
                (1..=6).contains(&hashes)
                    && line.chars().nth(hashes).is_some_and(char::is_whitespace)
            });
        if !structural {
            let mut ranges = cache.ranges.clone();
            let mut blocks = cache.blocks.clone();
            if let (Some(range), Some(block)) = (ranges.last_mut(), blocks.last_mut()) {
                *range = range.start..text.len();
                block.text = Arc::from(&text[range.clone()]);
            }
            return PartBlocks {
                ranges,
                blocks,
                source_len: text.len(),
                source: text.to_string(),
            };
        }
    }

    let parse_from = if reuse > 0 {
        existing
            .and_then(|cache| cache.ranges.get(reuse).map(|range| range.start))
            .unwrap_or(0)
    } else {
        0
    };

    let mut ranges = if let Some(existing) = existing {
        existing.ranges[..reuse].to_vec()
    } else {
        Vec::new()
    };
    for range in markdown::split_ranges(&text[parse_from..]) {
        ranges.push((range.start + parse_from)..(range.end + parse_from));
    }

    let mut blocks = Vec::with_capacity(ranges.len());
    if reuse > 0
        && let Some(existing) = existing
    {
        blocks.extend(existing.blocks[..reuse].iter().cloned());
    }
    for (ix, range) in ranges.iter().enumerate().skip(reuse) {
        let value = &text[range.clone()];
        if value.trim().is_empty() {
            continue;
        }
        blocks.push(Block {
            id: Arc::from(format!("{}#{}", part.id, ix)),
            message_id: Arc::from(message.id.as_str()),
            part_id: Arc::from(part.id.as_str()),
            kind,
            text: Arc::from(value),
            detail: Arc::from(""),
            output: Arc::from(""),
            status: ToolStatus::None,
            streaming,
            agent: None,
            model: None,
            created_at: message.created_at,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        });
    }

    PartBlocks {
        ranges,
        blocks,
        source_len: text.len(),
        source: if streaming {
            text.to_string()
        } else {
            String::new()
        },
    }
}

/// Convert one live OpenCode part snapshot into the domain part model.
pub fn part_from_json(value: &Value, message_id: &str) -> Option<Part> {
    let id = value.get("id").and_then(Value::as_str)?.to_string();
    match value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "text" => Some(Part {
            id,
            ordinal: 0,
            kind: PartKind::Text {
                text: value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                streaming: false,
            },
        }),
        "reasoning" => Some(Part {
            id,
            ordinal: 0,
            kind: PartKind::Reasoning {
                text: value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                streaming: false,
            },
        }),
        "tool" => tool_from_json(value).map(|tool| Part {
            id,
            ordinal: 0,
            kind: PartKind::Tool(Box::new(tool)),
        }),
        _ => {
            let _ = message_id;
            None
        }
    }
}

/// Convert a persisted OpenCode message into the domain model.
pub fn message_from_json(raw: &Value) -> Option<Message> {
    let value = raw.get("info").unwrap_or(raw);
    let content = raw
        .get("parts")
        .and_then(Value::as_array)
        .filter(|parts| !parts.is_empty())
        .or_else(|| value.get("content").and_then(Value::as_array));
    let id = value.get("id")?.as_str()?.to_string();
    let kind = value
        .get("type")
        .or_else(|| value.get("role"))
        .and_then(Value::as_str)
        .unwrap_or("assistant");
    let created = value
        .pointer("/time/created")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    match kind {
        "user" => {
            let text = value
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    content
                        .into_iter()
                        .flatten()
                        .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
                        .map(|part| part.get("text").and_then(Value::as_str).unwrap_or_default())
                        .collect::<Vec<_>>()
                        .join("\n")
                });
            let mut message = Message::text(id, Role::User, created, text);
            message.attachments = content
                .into_iter()
                .flatten()
                .filter(|part| part.get("type").and_then(Value::as_str) == Some("file"))
                .filter_map(|part| {
                    part.get("name")
                        .and_then(Value::as_str)
                        .filter(|name| !name.is_empty())
                        .map(str::to_string)
                        .or_else(|| {
                            part.get("path")
                                .and_then(Value::as_str)
                                .and_then(|path| {
                                    std::path::Path::new(path)
                                        .file_name()
                                        .map(|name| name.to_string_lossy().into_owned())
                                })
                        })
                })
                .collect();
            Some(message)
        }
        "assistant" => {
            let mut message = Message::new(id.clone(), Role::Assistant, created);
            message.agent = value
                .get("agent")
                .and_then(Value::as_str)
                .map(str::to_string);
            message.model = value
                .pointer("/model/id")
                .and_then(Value::as_str)
                .map(str::to_string);
            message.completed_at = value.pointer("/time/completed").and_then(Value::as_i64);
            message.finish = value
                .get("finish")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(error) = value.get("error") {
                message.error = Some(crate::event::failure_text(
                    Some(error),
                    "VELA_ASSISTANT_FAILED",
                    "Assistant failed",
                ));
            }
            if let Some(retry) = value.get("retry").filter(|value| value.is_object()) {
                message.retry = Some(RetryState {
                    attempt: retry
                        .get("attempt")
                        .and_then(Value::as_u64)
                        .unwrap_or(1)
                        .min(u32::MAX as u64) as u32,
                    message: crate::event::failure_text(
                        retry.get("error").or_else(|| retry.get("message")),
                        "VELA_RETRY_SCHEDULED",
                        "Retrying",
                    ),
                    next: retry
                        .get("at")
                        .or_else(|| retry.get("next"))
                        .and_then(Value::as_i64)
                        .filter(|at| *at != 0),
                });
            }
            let completed = message.completed_at.is_some()
                || message.finish.is_some()
                || message.error.is_some();
            if let Some(content) = content {
                let mut ordinals = [0u32; 2];
                for (index, part) in content.iter().enumerate() {
                    let kind = part.get("type").and_then(Value::as_str).unwrap_or_default();
                    let ordinal = match kind {
                        "text" | "reasoning" => {
                            let counter = &mut ordinals[usize::from(kind == "reasoning")];
                            let ordinal = *counter;
                            *counter += 1;
                            ordinal
                        }
                        _ => index as u32,
                    };
                    let part_id = part
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("{id}:{kind}:{ordinal}"));
                    let streaming = !completed
                        && part
                            .pointer("/time/end")
                            .or_else(|| part.pointer("/time/completed"))
                            .filter(|v| !v.is_null())
                            .is_none();
                    let text = || {
                        part.get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    };
                    let part_kind = match kind {
                        "text" => PartKind::Text {
                            text: text(),
                            streaming,
                        },
                        "reasoning" => PartKind::Reasoning {
                            text: text(),
                            streaming,
                        },
                        "tool" => {
                            let mut tool_value = part.clone();
                            if tool_value.get("id").is_none() {
                                tool_value["id"] = Value::String(part_id.clone());
                            }
                            let Some(mut tool) = tool_from_json(&tool_value) else {
                                continue;
                            };
                            // Reference replay -> viewPart -> partFromProjection drops
                            // state.error, then displays the default failure. Retain the
                            // original structured error here while matching that UI text.
                            if tool.status == ToolStatus::Failed {
                                tool.output = "[VELA_TOOL_FAILED] Tool failed".into();
                            }
                            if let Some(time) = tool_value
                                .get("time")
                                .or_else(|| tool_value.pointer("/state/time"))
                                && let Some(completed) =
                                    time.get("completed").and_then(Value::as_i64)
                                && let Some(created) = tool.started_at
                            {
                                let ran =
                                    time.get("ran").and_then(Value::as_i64).unwrap_or(created);
                                tool.finished_at = Some(
                                    created.saturating_add(completed.saturating_sub(ran).max(0)),
                                );
                            }
                            PartKind::Tool(Box::new(tool))
                        }
                        _ => continue,
                    };
                    message.parts.push(Part {
                        id: part_id,
                        ordinal,
                        kind: part_kind,
                    });
                }
            }
            Some(message)
        }
        "shell" => {
            let command = value
                .get("command")
                .and_then(Value::as_str)
                .or_else(|| value.get("text").and_then(Value::as_str))
                .unwrap_or_default();
            Some(Message::text(id, Role::Shell, created, command))
        }
        "synthetic" | "system" => {
            let text = value
                .get("text")
                .and_then(Value::as_str)
                .or_else(|| value.pointer("/payload/text").and_then(Value::as_str))
                .unwrap_or_default();
            Some(Message::text(id, Role::System, created, text))
        }
        "skill" => {
            let text = value
                .get("text")
                .and_then(Value::as_str)
                .or_else(|| value.pointer("/payload/text").and_then(Value::as_str))
                .unwrap_or_default();
            let skill_id = value
                .get("skill")
                .and_then(Value::as_str)
                .or_else(|| value.get("skillID").and_then(Value::as_str))
                .or_else(|| value.get("skillId").and_then(Value::as_str))
                .map(str::to_string)
                .unwrap_or_else(|| id.clone());
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| value.get("title").and_then(Value::as_str))
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    if skill_id.is_empty() {
                        "Skill".into()
                    } else {
                        skill_id.clone()
                    }
                });
            let mut message = Message::text(id, Role::Skill, created, text);
            message.completed_at = Some(created);
            message.skill_id = Some(skill_id);
            message.skill_name = Some(name);
            Some(message)
        }
        "compaction" => {
            let text = value
                .get("summary")
                .and_then(Value::as_str)
                .or_else(|| value.get("text").and_then(Value::as_str))
                .unwrap_or("Context compacted");
            let mut message = Message::text(id, Role::Compaction, created, text);
            message.completed_at = Some(
                value
                    .get("time")
                    .and_then(|time| time.get("completed"))
                    .and_then(Value::as_i64)
                    .unwrap_or(created),
            );
            if let Some(error) = value.get("error") {
                message.error = Some(crate::event::failure_text(
                    Some(error),
                    "VELA_COMPACTION_FAILED",
                    "Compaction failed",
                ));
            }
            Some(message)
        }
        _ => None,
    }
}

pub fn tool_files_from_json(value: &Value) -> Vec<ToolFile> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some("file") {
                return None;
            }
            let uri = item
                .get("uri")
                .or_else(|| item.get("url"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|uri| !uri.is_empty())?
                .to_string();
            Some(ToolFile {
                uri,
                name: item
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string),
                mime: item
                    .get("mime")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|mime| !mime.is_empty())
                    .map(str::to_string),
            })
        })
        .collect()
}

pub fn tool_subcalls_from_metadata(value: &Value) -> Vec<ToolSubCall> {
    parse_tool_subcalls(value, 0)
}

fn parse_tool_subcalls(value: &Value, depth: usize) -> Vec<ToolSubCall> {
    const MAX_SUBCALL_DEPTH: usize = 8;
    if depth >= MAX_SUBCALL_DEPTH {
        return Vec::new();
    }
    let Some(calls) = value
        .get("subCalls")
        .and_then(Value::as_array)
        .or_else(|| value.pointer("/deepseek/subCalls").and_then(Value::as_array))
        .or_else(|| {
            value
                .pointer("/metadata/deepseek/subCalls")
                .and_then(Value::as_array)
        })
    else {
        return Vec::new();
    };
    calls
        .iter()
        .filter_map(|call| {
            let id = call.get("id").and_then(Value::as_str)?.to_string();
            let title = call.get("title").and_then(Value::as_str)?.to_string();
            let status = match call.get("status").and_then(Value::as_str) {
                Some("success") => ToolStatus::Success,
                Some("failed") => ToolStatus::Failed,
                _ => ToolStatus::Running,
            };
            let input = call
                .get("input")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    call.get("inputValue")
                        .filter(|value| !value.is_null())
                        .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
                })
                .unwrap_or_default();
            let files = call
                .get("content")
                .map(tool_files_from_json)
                .unwrap_or_default();
            Some(ToolSubCall {
                id,
                title,
                detail: call
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                status,
                input,
                output: call
                    .get("output")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                files,
                sub_calls: parse_tool_subcalls(call, depth + 1),
            })
        })
        .collect()
}

fn tool_from_json(value: &Value) -> Option<ToolPart> {
    let call_id = value
        .get("callID")
        .or_else(|| value.get("id"))?
        .as_str()?
        .to_string();
    let state = value.get("state").unwrap_or(&Value::Null);
    let input_value = state.get("input").unwrap_or(&Value::Null);
    let name = value
        .get("name")
        .or_else(|| value.get("tool"))
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if let Some(name) = input_value
                .get("tool")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
            {
                name.to_string()
            } else if input_value.get("command").is_some() {
                "bash".into()
            } else if input_value
                .get("filePath")
                .or_else(|| input_value.get("file_path"))
                .is_some()
            {
                "file".into()
            } else if input_value.get("query").is_some() {
                "search".into()
            } else if input_value.get("url").is_some() {
                "web".into()
            } else if input_value.get("prompt").is_some() {
                "prompt".into()
            } else {
                "tool".into()
            }
        });
    let status = match state.get("status").and_then(Value::as_str) {
        Some("streaming") => ToolStatus::Streaming,
        Some("completed" | "success") => ToolStatus::Success,
        Some("error" | "failed" | "aborted" | "timeout" | "cancelled") => ToolStatus::Failed,
        _ => ToolStatus::Running,
    };
    let input = if input_value.is_null() {
        String::new()
    } else {
        serde_json::to_string_pretty(input_value).unwrap_or_default()
    };
    let error = state
        .get("error")
        .map(|error| crate::event::failure_text(Some(error), "VELA_TOOL_FAILED", "Tool failed"));
    let metadata = state
        .get("metadata")
        .or_else(|| value.get("metadata"));
    let shell = matches!(
        name.to_ascii_lowercase().as_str(),
        "bash" | "shell" | "terminal" | "run_code" | "code"
    );
    // Text content wins; otherwise the reference presents an error before output.
    let output = state
        .get("content")
        .map(tool_content_text)
        .filter(|text| !text.is_empty())
        .or_else(|| error.clone())
        .or_else(|| {
            state
                .get("output")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            shell
                .then(|| metadata.and_then(|metadata| metadata.get("output")))
                .flatten()
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    let detail = input_value
        .get("filePath")
        .or_else(|| input_value.get("file_path"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            input_value
                .get("command")
                .and_then(Value::as_str)
                .map(|command| format!("$ {command}"))
        })
        .or_else(|| {
            shell
                .then(|| metadata.and_then(|metadata| metadata.get("command")))
                .flatten()
                .and_then(Value::as_str)
                .map(|command| format!("$ {command}"))
        })
        .unwrap_or_default();
    let files = state
        .get("content")
        .or_else(|| value.get("content"))
        .map(tool_files_from_json)
        .unwrap_or_default();
    let sub_calls = metadata
        .map(tool_subcalls_from_metadata)
        .unwrap_or_default();
    let time = value.get("time").or_else(|| state.get("time"));
    let child_session_id = metadata
        .and_then(|metadata| {
            metadata
                .get("sessionId")
                .or_else(|| metadata.get("sessionID"))
        })
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_string);
    Some(ToolPart {
        call_id,
        name: name.clone(),
        title: name,
        status,
        input,
        output,
        detail,
        error,
        started_at: time
            .and_then(|time| time.get("created"))
            .and_then(Value::as_i64),
        finished_at: time
            .and_then(|time| time.get("completed"))
            .and_then(Value::as_i64),
        child_session_id,
        files,
        sub_calls,
    })
}

pub fn tool_content_text(value: &Value) -> String {
    let Some(items) = value.as_array() else {
        return value.as_str().unwrap_or_default().to_string();
    };
    let mut out = String::new();
    for item in items {
        if item.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text);
        }
    }
    out
}

pub fn structured_error(value: &Value) -> String {
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
        .unwrap_or_else(|| value.to_string())
}

impl Transcript {
    pub fn from_messages_json(value: &Value) -> Self {
        let messages = value
            .get("data")
            .or_else(|| value.get("messages"))
            .and_then(Value::as_array)
            .or_else(|| value.as_array())
            .map(|items| items.iter().filter_map(message_from_json).collect())
            .unwrap_or_default();
        let mut transcript = Self::new();
        transcript.hydrate(messages);
        transcript
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentEvent;

    fn event_session() -> &'static str {
        "ses_1"
    }

    #[test]
    fn v2_execution_events_drive_busy_and_terminal_state_without_step_events() {
        let mut transcript = Transcript::new();
        let event = |kind: &str| {
            crate::event::parse_event(&serde_json::json!({
                "type": kind, "data": {"sessionID":"ses_1"}
            }))
            .unwrap()
        };
        assert!(transcript.apply(&event("session.execution.started")));
        assert_eq!(transcript.phase, SessionPhase::Busy);
        assert!(transcript.busy);
        assert!(transcript.apply(&event("session.execution.interrupted")));
        assert_eq!(transcript.phase, SessionPhase::Error);
        assert!(!transcript.busy);
        transcript.apply(&event("session.execution.started"));
        transcript.apply(&event("session.execution.succeeded"));
        assert_eq!(transcript.phase, SessionPhase::Idle);
        assert!(!transcript.busy);
    }

    #[test]
    fn skill_activation_renders_a_skill_block() {
        let mut transcript = Transcript::new();
        transcript.apply(
            &crate::event::parse_event(&serde_json::json!({
                "type":"session.skill.activated","created":10,
                "data":{"sessionID":"ses_1","id":"review","name":"Review","text":"Checked"}
            }))
            .unwrap(),
        );
        let skill = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::Skill)
            .cloned()
            .unwrap();
        assert_eq!(skill.detail.as_ref(), "review");
        assert_eq!(skill.output.as_ref(), "Review");
        assert_eq!(skill.text.as_ref(), "Checked");
        assert_eq!(skill.status, ToolStatus::None);
        assert_eq!(skill.completed_at, Some(10));
    }

    #[test]
    fn hydrate_keeps_live_skill_records() {
        let mut transcript = Transcript::new();
        transcript.apply(
            &crate::event::parse_event(&serde_json::json!({
                "type":"session.skill.activated","created":10,
                "data":{"sessionID":"ses_1","id":"review","name":"Review","text":"Checked"}
            }))
            .unwrap(),
        );
        transcript.hydrate(Vec::new());
        let skills = transcript
            .blocks()
            .iter()
            .filter(|block| block.kind == BlockKind::Skill)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].detail.as_ref(), "review");
        assert_eq!(skills[0].output.as_ref(), "Review");
        assert_eq!(skills[0].text.as_ref(), "Checked");
    }

    #[test]
    fn replay_skill_records_preserve_identity_name_and_text() {
        let mut transcript =
            Transcript::from_messages_json(&serde_json::json!({"data":[{
                "info":{"id":"event","type":"skill","time":{"created":5},
                    "skill":"review","name":"Review","text":"Checked"}
            }]}));
        let skill = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::Skill)
            .cloned()
            .unwrap();
        assert_eq!(skill.detail.as_ref(), "review");
        assert_eq!(skill.output.as_ref(), "Review");
        assert_eq!(skill.text.as_ref(), "Checked");
        assert_eq!(skill.completed_at, Some(5));
    }

    #[test]
    fn live_compaction_aggregates_one_notice_and_finishes() {
        let mut transcript = Transcript::new();
        for value in [
            serde_json::json!({"type":"session.compaction.started","created":10,
                "data":{"sessionID":"ses_1"}}),
            serde_json::json!({"type":"session.compaction.delta","created":11,
                "data":{"sessionID":"ses_1","text":"First"}}),
            serde_json::json!({"type":"session.compaction.delta","created":12,
                "data":{"sessionID":"ses_1","text":" summary"}}),
        ] {
            transcript.apply(&crate::event::parse_event(&value).unwrap());
        }
        let notices = transcript
            .blocks()
            .iter()
            .filter(|block| block.kind == BlockKind::Compaction)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].status, ToolStatus::Running);
        assert_eq!(notices[0].text.as_ref(), "First summary");
        assert!(notices[0].streaming);
        assert_eq!(notices[0].completed_at, None);

        transcript.apply(
            &crate::event::parse_event(&serde_json::json!({
                "type":"session.compaction.ended","created":20,
                "data":{"sessionID":"ses_1","text":"Final summary"}
            }))
            .unwrap(),
        );
        let notices = transcript
            .blocks()
            .iter()
            .filter(|block| block.kind == BlockKind::Compaction)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].status, ToolStatus::Success);
        assert_eq!(notices[0].text.as_ref(), "Final summary");
        assert!(!notices[0].streaming);
        assert_eq!(notices[0].completed_at, Some(20));
    }

    #[test]
    fn failed_compaction_keeps_the_summary_and_error() {
        let mut transcript = Transcript::new();
        for value in [
            serde_json::json!({"type":"session.compaction.started","created":10,
                "data":{"sessionID":"ses_1"}}),
            serde_json::json!({"type":"session.compaction.delta","created":11,
                "data":{"sessionID":"ses_1","text":"Partial summary"}}),
            serde_json::json!({"type":"session.compaction.failed","created":20,
                "data":{"sessionID":"ses_1","error":"compaction boom"}}),
        ] {
            transcript.apply(&crate::event::parse_event(&value).unwrap());
        }
        let notice = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::Compaction)
            .cloned()
            .unwrap();
        assert_eq!(notice.status, ToolStatus::Failed);
        assert_eq!(notice.text.as_ref(), "Partial summary");
        assert!(notice.output.contains("compaction boom"));
        assert_eq!(notice.completed_at, Some(20));
    }

    #[test]
    fn replay_compaction_is_a_completed_notice() {
        let mut transcript = Transcript::from_messages_json(&serde_json::json!({"data":[
            {"info":{"id":"compact","type":"compaction","time":{"created":5},
                "summary":"Kept summary"}}
        ]}));
        let notice = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::Compaction)
            .cloned()
            .unwrap();
        assert_eq!(notice.status, ToolStatus::Success);
        assert_eq!(notice.text.as_ref(), "Kept summary");
        assert_eq!(notice.completed_at, Some(5));
    }

    #[test]
    fn hydrate_keeps_a_running_compaction_notice() {
        let mut transcript = Transcript::new();
        transcript.apply(
            &crate::event::parse_event(&serde_json::json!({
                "type":"session.compaction.started","created":10,
                "data":{"sessionID":"ses_1"}
            }))
            .unwrap(),
        );
        transcript.hydrate(Vec::new());
        let notices = transcript
            .blocks()
            .iter()
            .filter(|block| block.kind == BlockKind::Compaction)
            .collect::<Vec<_>>();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].status, ToolStatus::Running);
    }

    #[test]
    fn v2_retry_keeps_deadline_and_replay_state_then_clears_on_restart_or_interrupt() {
        let mut transcript = Transcript::new();
        let event = |kind: &str, data: Value| {
            crate::event::parse_event(&serde_json::json!({
                "type":kind,"created":10,"data":data
            }))
            .unwrap()
        };
        transcript.apply(&event(
            "session.retry.scheduled",
            serde_json::json!({
                "sessionID":"ses", "assistantMessageID":"retry", "attempt":3,
                "error":{"code":"provider.limit","message":"Try later"},"at":50000
            }),
        ));
        assert_eq!(transcript.phase(), SessionPhase::Retry);
        let retry = transcript.message("retry").unwrap().retry.as_ref().unwrap();
        assert_eq!(retry.next, Some(50000));
        assert_eq!(retry.attempt, 3);
        assert_eq!(retry.message, "[PROVIDER.LIMIT] Try later");
        assert_eq!(
            transcript
                .blocks()
                .iter()
                .filter(|block| block.kind == BlockKind::Retry)
                .count(),
            1
        );
        transcript.apply(&event(
            "session.step.started",
            serde_json::json!({"sessionID":"ses","assistantMessageID":"retry"}),
        ));
        assert!(transcript.message("retry").unwrap().retry.is_none());
        assert!(
            !transcript
                .blocks()
                .iter()
                .any(|block| block.kind == BlockKind::Retry)
        );
        let replay = serde_json::json!({"id":"replay", "type":"assistant", "time":{"created":1,"completed":2},
            "retry":{"attempt":2,"message":"Wait","next":40},"content":[]});
        transcript.hydrate(vec![message_from_json(&replay).unwrap()]);
        assert!(transcript.message("replay").unwrap().retry.is_some());
        transcript.apply(&event(
            "session.execution.interrupted",
            serde_json::json!({"sessionID":"ses"}),
        ));
        assert!(transcript.message("replay").unwrap().retry.is_none());
        assert_eq!(transcript.message("replay").unwrap().completed_at, Some(2));
        assert!(
            !transcript
                .blocks()
                .iter()
                .any(|block| block.kind == BlockKind::Retry)
        );
    }

    #[test]
    fn retry_settles_on_step_end_and_non_retry_session_status() {
        let event = |kind: &str, data: Value| {
            crate::event::parse_event(&serde_json::json!({
                "type":kind,"created":10,"data":data
            }))
            .unwrap()
        };
        let scheduled = || {
            event(
                "session.retry.scheduled",
                serde_json::json!({
                    "sessionID":"ses", "assistantMessageID":"retry", "attempt":2,
                    "error":{"message":"Wait"},"at":50000
                }),
            )
        };
        let mut ended = Transcript::new();
        ended.apply(&scheduled());
        assert!(ended.message("retry").unwrap().retry.is_some());
        ended.apply(&event(
            "session.step.ended",
            serde_json::json!({"sessionID":"ses","assistantMessageID":"retry","finish":"stop"}),
        ));
        assert!(ended.message("retry").unwrap().retry.is_none());
        assert_eq!(ended.message("retry").unwrap().completed_at, Some(10));
        assert_eq!(ended.phase(), SessionPhase::Idle);
        assert!(
            !ended
                .blocks()
                .iter()
                .any(|block| block.kind == BlockKind::Retry)
        );

        let mut status = Transcript::new();
        status.apply(&scheduled());
        assert!(status.message("retry").unwrap().retry.is_some());
        status.apply(&event(
            "session.status",
            serde_json::json!({"sessionID":"ses","status":{"type":"busy"}}),
        ));
        assert!(status.message("retry").unwrap().retry.is_none());
        assert_eq!(status.phase(), SessionPhase::Busy);
        assert!(
            !status
                .blocks()
                .iter()
                .any(|block| block.kind == BlockKind::Retry)
        );
    }

    #[test]
    fn quiet_settlement_matches_source_thresholds_and_near_future_retry_exception() {
        let started = || {
            crate::event::parse_event(&serde_json::json!({
                "type":"session.execution.started","created":1,"data":{"sessionID":"ses"}
            }))
            .unwrap()
        };
        let mut empty = Transcript::new();
        empty.apply(&started());
        assert_eq!(empty.settle_quiet(59_999, 60_000), None);
        assert_eq!(empty.settle_quiet(60_000, 60_001), Some(true));
        assert_eq!(empty.phase(), SessionPhase::Idle);
        let mut idle = Transcript::new();
        idle.hydrate(vec![Message::text(
            "unfinished",
            Role::Assistant,
            1,
            "Partial response",
        )]);
        assert_eq!(
            idle.phase(),
            SessionPhase::Busy,
            "initial replay must seed unfinished work"
        );
        idle.set_phase(SessionPhase::Idle);
        assert_eq!(idle.settle_quiet(1_999, 3_000), None);
        assert_eq!(idle.settle_quiet(2_000, 3_001), Some(false));
        assert_eq!(
            idle.message("unfinished").unwrap().completed_at,
            Some(3_001)
        );
        let retry = |next| {
            let mut transcript = Transcript::new();
            transcript.apply(&crate::event::parse_event(&serde_json::json!({
                "type":"session.retry.scheduled","created":1,
                "data":{"sessionID":"ses","assistantMessageID":"retry","at":next,"error":{"message":"Wait"}}
            })).unwrap());
            transcript
        };
        let mut pending = retry(100_001);
        assert_eq!(pending.settle_quiet(60_000, 60_001), None);
        assert_eq!(pending.settle_quiet(100_001, 100_001), Some(true));
        assert!(pending.message("retry").unwrap().retry.is_none());
        // The reference's exception is strictly less than sixty seconds ahead.
        assert_eq!(retry(120_001).settle_quiet(60_000, 60_001), Some(true));
        let mut known_idle = Transcript::new();
        known_idle.set_phase(SessionPhase::Idle);
        known_idle.hydrate(vec![Message::text(
            "late",
            Role::Assistant,
            1,
            "Late replay",
        )]);
        assert_eq!(
            known_idle.phase(),
            SessionPhase::Idle,
            "a replay cannot resurrect a known terminal state"
        );
    }

    #[test]
    fn empty_assistant_activity_marker_is_replaced_when_the_first_part_becomes_visible() {
        let mut transcript = Transcript::new();
        let event = |kind: &str, extra: Value| {
            crate::event::parse_event(&serde_json::json!({
                "type":kind,"created":20,"data":extra
            }))
            .unwrap()
        };
        transcript.apply(&event(
            "session.step.started",
            serde_json::json!({"sessionID":"ses","assistantMessageID":"message"}),
        ));
        assert_eq!(transcript.blocks().len(), 1);
        assert_eq!(transcript.blocks()[0].kind, BlockKind::Activity);
        transcript.apply(&event(
            "session.text.started",
            serde_json::json!({"sessionID":"ses","assistantMessageID":"message","ordinal":0}),
        ));
        assert_eq!(transcript.blocks().len(), 1);
        assert_eq!(transcript.blocks()[0].detail.as_ref(), "parts");
        transcript.apply(&event("session.text.delta",serde_json::json!({"sessionID":"ses","assistantMessageID":"message","ordinal":0,"delta":"Visible text"})));
        assert_eq!(transcript.blocks().len(), 1);
        assert_eq!(transcript.blocks()[0].kind, BlockKind::Assistant);
        transcript.apply(&event("session.text.ended",serde_json::json!({"sessionID":"ses","assistantMessageID":"message","ordinal":0,"text":""})));
        assert_eq!(transcript.blocks().len(), 1);
        assert_eq!(transcript.blocks()[0].kind, BlockKind::Activity);
        transcript.apply(&event(
            "session.execution.succeeded",
            serde_json::json!({"sessionID":"ses"}),
        ));
        assert_eq!(transcript.blocks()[0].completed_at, Some(20));
        assert!(!transcript.blocks()[0].streaming);
    }

    #[test]
    fn terminal_execution_completes_latest_assistant_and_updates_cached_stream_blocks() {
        for kind in [
            "session.execution.succeeded",
            "session.execution.interrupted",
            "session.execution.failed",
            "session.step.failed",
            "session.error",
        ] {
            let mut transcript = Transcript::new();
            // Keep an earlier incomplete message to check that terminal events
            // settle the latest assistant, as Vela's completeLatestIncomplete does.
            for id in ["earlier", "latest"] {
                transcript.apply(
                    &crate::event::parse_event(&serde_json::json!({
                        "type":"session.step.started", "created":10,
                        "data":{"sessionID":"ses", "assistantMessageID":id}
                    }))
                    .unwrap(),
                );
                transcript.apply(&crate::event::parse_event(&serde_json::json!({
                    "type":"session.text.delta", "created":11,
                    "data":{"sessionID":"ses", "assistantMessageID":id, "ordinal":0,"delta":"First paragraph\n\nSecond paragraph"}
                })).unwrap());
                transcript.apply(&crate::event::parse_event(&serde_json::json!({
                    "type":"session.reasoning.delta", "created":12,
                    "data":{"sessionID":"ses", "assistantMessageID":id, "ordinal":1,"delta":"Reasoning still streaming"}
                })).unwrap());
            }
            let before = transcript
                .blocks()
                .iter()
                .filter(|block| block.message_id.as_ref() == "latest")
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(before.len(), 3);
            assert!(before.iter().any(|block| block.streaming));
            transcript.apply(&crate::event::parse_event(&serde_json::json!({
                "type":kind,"created":99,"data":{"sessionID":"ses","assistantMessageID":"latest","error":{"message":"Failure fixture"}}
            })).unwrap());
            assert_eq!(transcript.message("latest").unwrap().completed_at, Some(99));
            assert_eq!(
                transcript.message("earlier").unwrap().completed_at,
                Some(10)
            );
            if kind.ends_with("failed") || kind == "session.error" {
                assert!(
                    transcript
                        .message("latest")
                        .unwrap()
                        .error
                        .as_deref()
                        .unwrap()
                        .ends_with("Failure fixture")
                );
            }
            if kind.ends_with("failed") || kind == "session.error" {
                let errors = transcript
                    .blocks()
                    .iter()
                    .filter(|block| block.kind == BlockKind::Error)
                    .collect::<Vec<_>>();
                assert_eq!(errors.len(), 1);
                assert!(errors[0].text.ends_with("Failure fixture"));
            }
            let blocks = transcript
                .blocks()
                .iter()
                .filter(|block| {
                    block.message_id.as_ref() == "latest" && block.kind != BlockKind::Error
                })
                .collect::<Vec<_>>();
            assert_eq!(blocks.len(), before.len());
            for (before, after) in before.iter().zip(blocks) {
                assert_eq!(before.id, after.id);
                assert_eq!(before.text, after.text);
                assert!(!after.streaming);
                assert_eq!(after.completed_at, Some(99));
            }
        }
    }

    #[test]
    fn streams_text_without_rebuilding_completed_blocks() {
        let mut transcript = Transcript::new();
        transcript.apply(&AgentEvent::StepStarted {
            session_id: event_session().into(),
            message_id: "m1".into(),
            agent: Some("build".into()),
            model: Some("claude".into()),
            at: 1,
        });
        transcript.apply(&AgentEvent::TextStarted {
            session_id: event_session().into(),
            message_id: "m1".into(),
            part_id: "m1:text:0".into(),
            ordinal: 0,
            at: 2,
        });
        transcript.apply(&AgentEvent::TextDelta {
            session_id: event_session().into(),
            message_id: "m1".into(),
            part_id: "m1:text:0".into(),
            ordinal: 0,
            delta: "hello\n\nworld".into(),
            at: 3,
        });
        let blocks = transcript.blocks().to_vec();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].text.as_ref(), "hello");
        assert_eq!(blocks[1].text.as_ref(), "world");
        assert!(blocks[1].streaming);

        let first_id = blocks[0].id.clone();
        transcript.apply(&AgentEvent::TextDelta {
            session_id: event_session().into(),
            message_id: "m1".into(),
            part_id: "m1:text:0".into(),
            ordinal: 0,
            delta: "\n\nthird".into(),
            at: 4,
        });
        let blocks = transcript.blocks().to_vec();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].id, first_id);
        assert_eq!(blocks[0].text.as_ref(), "hello");
    }

    #[test]
    fn completed_text_replaces_streamed_text() {
        let mut transcript = Transcript::new();
        transcript.apply(&AgentEvent::TextDelta {
            session_id: event_session().into(),
            message_id: "m1".into(),
            part_id: "m1:text:0".into(),
            ordinal: 0,
            delta: "hel".into(),
            at: 1,
        });
        transcript.apply(&AgentEvent::TextEnded {
            session_id: event_session().into(),
            message_id: "m1".into(),
            part_id: "m1:text:0".into(),
            ordinal: 0,
            text: "hello".into(),
            at: 2,
        });
        assert_eq!(transcript.blocks()[0].text.as_ref(), "hello");
        assert!(!transcript.blocks()[0].streaming);
    }

    #[test]
    fn tool_lifecycle_is_terminal_and_non_regressing() {
        let mut transcript = Transcript::new();
        transcript.apply(&AgentEvent::ToolInputStarted {
            session_id: event_session().into(),
            message_id: "m1".into(),
            call_id: "call_1".into(),
            name: "bash".into(),
            at: 1,
        });
        transcript.apply(&AgentEvent::ToolCalled {
            session_id: event_session().into(),
            message_id: "m1".into(),
            call_id: "call_1".into(),
            input: "{\"command\":\"ls\"}".into(),
            at: 2,
        });
        transcript.apply(&AgentEvent::ToolFinished {
            session_id: event_session().into(),
            message_id: "m1".into(),
            call_id: "call_1".into(),
            success: true,
            output: "file.txt".into(),
            files: Vec::new(),
            error: None,
            at: 3,
        });
        let blocks = transcript.blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].status, ToolStatus::Success);
        assert_eq!(blocks[0].output.as_ref(), "file.txt");
    }

    #[test]
    fn interrupted_execution_retains_partial_answer_and_adds_informational_notice() {
        for (error, expected) in [
            (Value::Null, "Interrupted"),
            (
                serde_json::json!({"code":"fixture.cancel", "message":"Stopped"}),
                "[FIXTURE.CANCEL] Stopped",
            ),
        ] {
            let mut transcript = Transcript::new();
            transcript.hydrate(vec![Message::text(
                "a",
                Role::Assistant,
                1,
                "Partial answer",
            )]);
            transcript.apply(
                &crate::event::parse_event(&serde_json::json!({
                    "type":"session.execution.interrupted", "created":2,
                    "data":{"sessionID":"s", "error":error}
                }))
                .unwrap(),
            );
            assert!(!transcript.busy);
            assert_eq!(
                transcript.message("a").unwrap().parts[0].kind.text(),
                "Partial answer"
            );
            let notice = transcript.messages().last().unwrap();
            assert_eq!(notice.role, Role::System);
            assert_eq!(notice.parts[0].kind.text(), expected);
            assert!(
                notice.error.is_none(),
                "interruption has informational tone"
            );
        }
    }

    #[test]
    fn canonical_replay_preserves_roles_part_ids_completion_and_retry_contract() {
        let value = serde_json::json!({"data":[
            {"info":{"id":"u","role":"user","time":{"created":1}},
             "parts":[{"id":"user-text","type":"text","text":"First"},
                      {"id":"user-more","type":"text","text":"Second 🌍"}]},
            {"info":{"id":"a","role":"assistant","time":{"created":2},
                     "retry":{"attempt":3,"at":5000,"error":{"code":"fixture.limit","message":"Wait"}}},
             "parts":[{"id":"r","type":"reasoning","text":"Plan","time":{"end":3}},
                      {"id":"t","type":"text","text":"Partial"},
                      {"id":"t2","type":"text","text":"More","time":{"completed":4}}]},
            {"info":{"id":"done","role":"assistant","time":{"created":5},"finish":"stop"},
             "parts":[{"id":"final","type":"text","text":"Done"}]},
            {"info":{"id":"failed","role":"assistant","time":{"created":6},
                     "error":{"code":"fixture.failed","message":"Failed"}},"parts":[]}
        ]});
        let transcript = Transcript::from_messages_json(&value);
        assert_eq!(transcript.messages().len(), 4);
        let user = transcript.message("u").unwrap();
        assert_eq!(user.role, Role::User);
        assert_eq!(user.parts[0].kind.text(), "First\nSecond 🌍");
        let assistant = transcript.message("a").unwrap();
        assert_eq!(
            assistant
                .parts
                .iter()
                .map(|part| part.id.as_str())
                .collect::<Vec<_>>(),
            vec!["r", "t", "t2"]
        );
        assert_eq!(
            assistant
                .parts
                .iter()
                .map(|part| part.ordinal)
                .collect::<Vec<_>>(),
            vec![0, 0, 1]
        );
        assert!(!assistant.parts[0].kind.is_streaming());
        assert!(assistant.parts[1].kind.is_streaming());
        assert!(!assistant.parts[2].kind.is_streaming());
        let retry = assistant.retry.as_ref().unwrap();
        assert_eq!(retry.attempt, 3);
        assert_eq!(retry.next, Some(5000));
        assert_eq!(retry.message, "[FIXTURE.LIMIT] Wait");
        assert!(
            !transcript.message("done").unwrap().parts[0]
                .kind
                .is_streaming()
        );
        assert_eq!(
            transcript.message("failed").unwrap().error.as_deref(),
            Some("[FIXTURE.FAILED] Failed")
        );
    }

    #[test]
    fn canonical_tool_replay_preserves_identity_output_status_and_state_times() {
        for (status, expected) in [
            ("completed", ToolStatus::Success),
            ("success", ToolStatus::Success),
            ("error", ToolStatus::Failed),
            ("failed", ToolStatus::Failed),
            ("aborted", ToolStatus::Failed),
            ("timeout", ToolStatus::Failed),
            ("cancelled", ToolStatus::Failed),
            ("pending", ToolStatus::Running),
        ] {
            let part = serde_json::json!({"id":"prt_tool", "callID":"call_tool", "type":"tool", "tool":"bash",
                "state":{"status":status,"input":{"command":"printf hello"},"output":"hello 🌍",
                    "time":{"created":10,"ran":12,"completed":20}}});
            let parsed = part_from_json(&part, "a").unwrap();
            assert_eq!(parsed.id, "prt_tool");
            let PartKind::Tool(tool) = parsed.kind else {
                panic!("tool part")
            };
            assert_eq!(tool.call_id, "call_tool");
            assert_eq!(tool.name, "bash");
            assert_eq!(tool.detail, "$ printf hello");
            assert_eq!(tool.status, expected, "{status}");
            assert_eq!(tool.output, "hello 🌍");
            assert_eq!(tool.started_at, Some(10));
            assert_eq!(tool.finished_at, Some(20));
        }
        let tool=tool_from_json(&serde_json::json!({"id":"p", "type":"tool", "state":{
            "input":{"file_path":"/tmp/a.txt"},"status":"error","output":"fallback",
            "content":[{"type":"file","uri":"https://example.com/report.csv","name":"report.csv","mime":"text/csv"},{"type":"file","text":"not a file"},{"type":"text","text":"first"},{"type":"text","text":"second"}],
            "metadata":{"deepseek":{"subCalls":[
                {"id":"sub-1","title":"Inspect","detail":"src/a.ts","status":"success","input":"{\"path\":\"src/a.ts\"}","output":"ok",
                    "content":[{"type":"file","uri":"https://example.com/nested.txt","name":"nested.txt","mime":"text/plain"}],
                    "subCalls":[{"id":"sub-1-1","title":"Child","status":"success","input":"{}","output":"child"}]},
                {"id":"sub-2","title":"Skip","status":"failed"}
            ]}},
            "error":{"code":"fixture.tool","message":"Failed"}}})).unwrap();
        assert_eq!(tool.title, "file");
        assert_eq!(tool.detail, "/tmp/a.txt");
        assert_eq!(tool.output, "first\nsecond");
        assert_eq!(tool.files.len(), 1);
        assert_eq!(tool.files[0].uri, "https://example.com/report.csv");
        assert_eq!(tool.files[0].name.as_deref(), Some("report.csv"));
        assert_eq!(tool.files[0].mime.as_deref(), Some("text/csv"));
        assert_eq!(tool.sub_calls.len(), 2);
        assert_eq!(tool.sub_calls[0].id, "sub-1");
        assert_eq!(tool.sub_calls[0].status, ToolStatus::Success);
        assert_eq!(tool.sub_calls[0].detail, "src/a.ts");
        assert_eq!(tool.sub_calls[0].files.len(), 1);
        assert_eq!(
            tool.sub_calls[0].files[0].name.as_deref(),
            Some("nested.txt")
        );
        assert_eq!(tool.sub_calls[0].sub_calls.len(), 1);
        assert_eq!(tool.sub_calls[0].sub_calls[0].id, "sub-1-1");
        assert_eq!(tool.sub_calls[0].sub_calls[0].output, "child");
        assert_eq!(tool.sub_calls[1].status, ToolStatus::Failed);
        assert_eq!(tool.error.as_deref(), Some("[FIXTURE.TOOL] Failed"));
    }

    #[test]
    fn task_tool_metadata_links_the_child_session_and_reaches_blocks() {
        for metadata in [
            serde_json::json!({"sessionId":"ses_child"}),
            serde_json::json!({"sessionID":"ses_child"}),
        ] {
            let part = serde_json::json!({"id":"p", "callID":"c", "type":"tool", "tool":"task",
                "state":{"status":"running","input":{"agent":"build","description":"Run the tests"},
                    "metadata":metadata}});
            let parsed = part_from_json(&part, "a").unwrap();
            let PartKind::Tool(tool) = parsed.kind else {
                panic!("tool")
            };
            assert_eq!(
                tool.child_session_id.as_deref(),
                Some("ses_child"),
                "{metadata}"
            );
        }
        let mut transcript = Transcript::from_messages_json(&serde_json::json!({"data":[
            {"info":{"id":"a","role":"assistant","time":{"created":1}},"parts":[
                {"id":"p","callID":"c","type":"tool","tool":"task","state":{
                    "status":"running","input":{"agent":"build","description":"Run the tests"},
                    "metadata":{"sessionId":"ses_child"}}}]}
        ]}));
        let linked = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::Tool)
            .unwrap();
        assert_eq!(linked.child_session_id.as_deref(), Some("ses_child"));
        let top_level = tool_from_json(&serde_json::json!({"id":"p","type":"tool","tool":"task",
            "metadata":{"sessionID":"ses_top"},"state":{"status":"running","input":{}}}))
        .unwrap();
        assert_eq!(top_level.child_session_id.as_deref(), Some("ses_top"));
        let plain = tool_from_json(&serde_json::json!({"id":"p","type":"tool","tool":"task",
            "state":{"status":"running","input":{}}}))
        .unwrap();
        assert_eq!(plain.child_session_id, None);
    }

    #[test]
    fn nested_subcall_parsing_is_bounded() {
        let mut call = serde_json::json!({"id":"leaf","title":"Leaf","status":"success"});
        for index in (0..12).rev() {
            call = serde_json::json!({
                "id":format!("level-{index}"),
                "title":"Level",
                "status":"success",
                "subCalls":[call]
            });
        }
        let mut calls = tool_subcalls_from_metadata(&serde_json::json!({
            "deepseek":{"subCalls":[call]}
        }));
        let mut depth = 0;
        while let Some(call) = calls.first() {
            depth += 1;
            calls = call.sub_calls.clone();
        }
        assert!(depth <= 8, "recursive metadata must be depth-bounded");
        assert!(depth > 1, "valid nested calls still recurse");
    }

    #[test]
    fn task_progress_metadata_links_the_child_session() {
        let mut transcript = Transcript::new();
        transcript.apply(
            &crate::event::parse_event(&serde_json::json!({
                "type":"session.tool.progress","created":10,
                "data":{"sessionID":"ses_parent","assistantMessageID":"a","id":"c",
                    "metadata":{"sessionId":"ses_child","deepseek":{"subCalls":[
                        {"id":"sub-a","title":"Nested","status":"running"}
                    ]}}}
            }))
            .unwrap(),
        );
        let linked = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::Tool)
            .unwrap();
        assert_eq!(linked.child_session_id.as_deref(), Some("ses_child"));
        assert_eq!(linked.sub_calls.len(), 1);
        assert_eq!(linked.sub_calls[0].title, "Nested");
        assert_eq!(linked.sub_calls[0].status, ToolStatus::Running);
    }

    #[test]
    fn shell_metadata_command_and_output_fill_missing_input_fields() {
        let tool = tool_from_json(&serde_json::json!({
            "id":"p","type":"tool","tool":"bash",
            "state":{"status":"success","input":{},
                "metadata":{"command":"printf hi","output":"hello"}}
        }))
        .unwrap();
        assert_eq!(tool.detail, "$ printf hi");
        assert_eq!(tool.output, "hello");

        let file = tool_from_json(&serde_json::json!({
            "id":"p","type":"tool","tool":"read",
            "state":{"status":"success","input":{"file_path":"/tmp/a"},
                "metadata":{"command":"ignored","output":"ignored"}}
        }))
        .unwrap();
        assert_eq!(file.detail, "/tmp/a");
        assert!(file.output.is_empty());
    }

    #[test]
    fn assembled_blocks_carry_message_agent_and_model_metadata() {
        let mut transcript = Transcript::new();
        transcript.apply(&AgentEvent::StepStarted {
            session_id: event_session().into(),
            message_id: "m1".into(),
            agent: Some("build".into()),
            model: Some("claude".into()),
            at: 1,
        });
        transcript.apply(&AgentEvent::TextStarted {
            session_id: event_session().into(),
            message_id: "m1".into(),
            part_id: "m1:text:0".into(),
            ordinal: 0,
            at: 2,
        });
        transcript.apply(&AgentEvent::TextDelta {
            session_id: event_session().into(),
            message_id: "m1".into(),
            part_id: "m1:text:0".into(),
            ordinal: 0,
            delta: "hello".into(),
            at: 3,
        });
        let blocks = transcript.blocks();
        assert_eq!(blocks[0].agent.as_deref(), Some("build"));
        assert_eq!(blocks[0].model.as_deref(), Some("claude"));
    }

    #[test]
    fn restored_failed_tool_matches_reference_hydration_and_retains_original_error() {
        let raw = serde_json::json!({"info":{"id":"a","role":"assistant","time":{"created":1}},
            "parts":[{"id":"p","callID":"c","type":"tool","name":"bash", "time":{"created":10,"ran":12,"completed":20},
                "state":{"status":"error","input":{},"error":{"code":"fixture.tool","message":"Saved failure"}}}]});
        let message = message_from_json(&raw).unwrap();
        let PartKind::Tool(tool) = &message.parts[0].kind else {
            panic!("tool")
        };
        assert_eq!(tool.output, "[VELA_TOOL_FAILED] Tool failed");
        assert_eq!(tool.error.as_deref(), Some("[FIXTURE.TOOL] Saved failure"));
        assert_eq!(tool.finished_at, Some(18));
        let live = part_from_json(&raw["parts"][0], "a").unwrap();
        let PartKind::Tool(tool) = live.kind else {
            panic!("tool")
        };
        assert_eq!(tool.output, "[FIXTURE.TOOL] Saved failure");
        assert_eq!(tool.finished_at, Some(20));
    }

    #[test]
    fn replay_and_live_tool_updates_share_one_part_across_distinct_call_and_part_ids() {
        let raw = serde_json::json!({"info":{"id":"a","role":"assistant","time":{"created":1}},
            "parts":[{"id":"prt_tool","callID":"call_tool","type":"tool","tool":"bash",
                "state":{"status":"running","input":{"command":"ls"}}}]});
        let mut transcript = Transcript::new();
        transcript.hydrate(vec![message_from_json(&raw).unwrap()]);
        transcript.apply(&AgentEvent::ToolFinished {
            session_id: event_session().into(),
            message_id: "a".into(),
            call_id: "call_tool".into(),
            success: true,
            output: "done".into(),
            files: vec![ToolFile {
                uri: "https://example.com/log.txt".into(),
                name: Some("log.txt".into()),
                mime: Some("text/plain".into()),
            }],
            error: None,
            at: 3,
        });
        assert_eq!(transcript.message("a").unwrap().parts.len(), 1);
        let blocks = transcript.blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id.as_ref(), "prt_tool");
        assert_eq!(blocks[0].output.as_ref(), "done");
        assert_eq!(blocks[0].files.len(), 1);
        assert_eq!(blocks[0].files[0].name.as_deref(), Some("log.txt"));
        assert_eq!(blocks[0].status, ToolStatus::Success);

        let mut transcript = Transcript::new();
        transcript.apply(&AgentEvent::ToolInputStarted {
            session_id: event_session().into(),
            message_id: "a".into(),
            call_id: "call_tool".into(),
            name: "bash".into(),
            at: 1,
        });
        assert_eq!(transcript.blocks().len(), 1);
        transcript.apply(&AgentEvent::PartUpsert {
            session_id: event_session().into(),
            message_id: "a".into(),
            part: part_from_json(&raw["parts"][0], "a").unwrap(),
            at: 2,
        });
        assert_eq!(transcript.message("a").unwrap().parts.len(), 1);
        let blocks = transcript.blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id.as_ref(), "prt_tool");
    }

    #[test]
    fn local_user_prompt_attachments_survive_block_assembly() {
        let mut transcript = Transcript::new();
        transcript.upsert_local_user(
            "local:1".into(),
            "Review these".into(),
            1,
            vec!["main.rs".into(), "notes.md".into()],
        );
        let user = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::User)
            .expect("user block");
        assert_eq!(
            user.attachments
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>(),
            vec!["main.rs", "notes.md"]
        );
        assert_eq!(user.text.as_ref(), "Review these");
    }

    #[test]
    fn replay_user_file_parts_become_attachment_names() {
        let value = serde_json::json!({"data":[{
            "id":"u1","type":"user","time":{"created":1},"text":"See files",
            "content":[
                {"type":"text","text":"See files"},
                {"type":"file","path":"/tmp/secret-plan.md"},
                {"type":"file","name":"src/main.rs","path":"/tmp/src/main.rs"},
                {"type":"file","name":"","path":"/tmp/no-name.txt"}
            ]
        }]});
        let mut transcript = Transcript::from_messages_json(&value);
        let user = transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::User)
            .expect("user block");
        assert_eq!(
            user.attachments
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>(),
            vec!["secret-plan.md", "src/main.rs", "no-name.txt"]
        );
        assert_eq!(user.text.as_ref(), "See files");
    }

    #[test]
    fn hydrates_replay_messages() {
        let value = serde_json::json!({
            "data": [
                { "id": "u1", "type": "user", "time": { "created": 1 }, "text": "do it" },
                { "id": "a1", "type": "assistant", "time": { "created": 2, "completed": 3 },
                  "agent": "build", "model": { "id": "gpt" }, "finish": "stop",
                  "content": [
                    { "type": "reasoning", "text": "plan" },
                    { "type": "text", "text": "done" },
                    { "type": "tool", "id": "t1", "name": "bash", "time": { "created": 2 },
                      "state": { "status": "completed", "input": { "command": "ls" },
                        "content": [ { "type": "text", "text": "a.txt" } ] } }
                  ] }
            ]
        });
        let mut transcript = Transcript::from_messages_json(&value);
        let kinds: Vec<_> = transcript.blocks().iter().map(|b| b.kind).collect();
        assert_eq!(
            kinds,
            vec![
                BlockKind::User,
                BlockKind::Reasoning,
                BlockKind::Assistant,
                BlockKind::Tool
            ]
        );
        assert_eq!(
            transcript.message("a1").unwrap().agent.as_deref(),
            Some("build")
        );
    }
}
