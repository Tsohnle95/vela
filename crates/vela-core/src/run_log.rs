//! Serializable input to the canonical transcript reducer.
//!
//! Recording owns receipt time, sequence allocation and live-ingress duplicate
//! suppression. Replaying these records never reads a clock. Disk ownership and
//! acknowledgement belong to the engine: it must persist a record before
//! applying it, and stop publication if persistence fails.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::{AgentEvent, EventEnvelope, parse_envelope};
use crate::session::SessionPhase;
use crate::transcript::{Message, Role, Transcript};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub id: String,
    pub seq: u64,
    pub run_id: String,
    pub timestamp: i64,
    pub event: AgentEvent,
}

/// Snapshots contain the fully reconciled canonical messages and phase, not an
/// HTTP response awaiting overlap reconciliation. This also records optimistic
/// prompts and timer-driven transitions that do not originate in transport.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunRecord {
    Event(Box<NormalizedEvent>),
    Snapshot {
        seq: u64,
        session_id: String,
        messages: Vec<Message>,
        phase: SessionPhase,
    },
    LocalUser {
        seq: u64,
        session_id: String,
        id: String,
        text: String,
        at: i64,
        attachments: Vec<String>,
    },
    Phase {
        seq: u64,
        session_id: String,
        phase: SessionPhase,
    },
    QuietSettlement {
        seq: u64,
        session_id: String,
        quiet_ms: u64,
        now: i64,
    },
}

impl RunRecord {
    pub fn seq(&self) -> u64 {
        match self {
            Self::Event(event) => event.seq,
            Self::Snapshot { seq, .. }
            | Self::LocalUser { seq, .. }
            | Self::Phase { seq, .. }
            | Self::QuietSettlement { seq, .. } => *seq,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::Event(event) => event.event.session_id(),
            Self::Snapshot { session_id, .. }
            | Self::LocalUser { session_id, .. }
            | Self::Phase { session_id, .. }
            | Self::QuietSettlement { session_id, .. } => Some(session_id),
        }
    }

    /// Apply an accepted record. The caller must select the matching session;
    /// live-ingress deduplication belongs to `EventLog`, not `Transcript::apply`,
    /// because reconnect legitimately reapplies events after history hydration.
    pub fn apply_to(&self, transcript: &mut Transcript) {
        match self {
            Self::Event(event) => {
                transcript.apply(&event.event);
            }
            Self::Snapshot {
                messages, phase, ..
            } => {
                // A canonical snapshot replaces the entire state. Reusing
                // hydrate on an older reducer would retain live synthetic
                // messages that the snapshot may have intentionally removed.
                let mut replacement = Transcript::new();
                replacement.hydrate(messages.clone());
                replacement.set_phase(*phase);
                *transcript = replacement;
            }
            Self::LocalUser {
                id,
                text,
                at,
                attachments,
                ..
            } => {
                transcript.upsert_local_user(id.clone(), text.clone(), *at, attachments.clone());
            }
            Self::Phase { phase, .. } => transcript.set_phase(*phase),
            Self::QuietSettlement { quiet_ms, now, .. } => {
                transcript.settle_quiet(*quiet_ms, *now);
            }
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReplayError {
    #[error("record sequence must increase: {current} follows {previous}")]
    NonMonotonicSequence { previous: u64, current: u64 },
    #[error("event log sequence is exhausted")]
    SequenceExhausted,
    #[error("normalized event identity and run identity must be nonempty")]
    InvalidIdentity,
}

#[derive(Clone, Debug)]
struct CurrentRun {
    id: String,
    user: Option<(String, String)>,
}

/// Append-only pending records plus compact ingestion metadata. `drain_records`
/// releases acknowledged payloads without forgetting IDs, sequence numbers or
/// run ownership. A drained log is not a full-history replay source; pass the
/// persisted records to `from_records` for recovery.
#[derive(Debug)]
pub struct EventLog {
    namespace: String,
    last_seq: u64,
    records: Vec<RunRecord>,
    seen: HashSet<(String, String)>,
    current: HashMap<String, CurrentRun>,
    message_runs: HashMap<(String, String), String>,
}

impl EventLog {
    /// `namespace` should be a fresh journal-instance identifier. It prevents
    /// source-less event IDs colliding between independent recordings.
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            last_seq: 0,
            records: Vec::new(),
            seen: HashSet::new(),
            current: HashMap::new(),
            message_runs: HashMap::new(),
        }
    }

    pub fn records(&self) -> &[RunRecord] {
        &self.records
    }

    pub fn drain_records(&mut self) -> Vec<RunRecord> {
        std::mem::take(&mut self.records)
    }

    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// Release ingestion metadata after canonical session deletion. Pending
    /// records remain append-only; the engine owns flushing/removing files and
    /// rejecting subsequent events for a retired session ID.
    pub fn retire_session(&mut self, session_id: &str) {
        self.seen.retain(|(session, _)| session != session_id);
        self.message_runs
            .retain(|(session, _), _| session != session_id);
        self.current.remove(session_id);
    }

    fn next_seq(&mut self) -> u64 {
        self.last_seq = self
            .last_seq
            .checked_add(1)
            .expect("event log sequence exhausted");
        self.last_seq
    }

    /// Accept one wire event once. Protocol identity is read only from envelope
    /// metadata; `data.id` is commonly a tool-call ID. Identical ID-less payloads
    /// receive distinct local IDs, since repeated identical deltas are valid.
    pub fn append(&mut self, raw: &Value, received_at: i64) -> Option<NormalizedEvent> {
        let envelope = EventEnvelope::parse_at(raw, received_at)?;
        let event = parse_envelope(&envelope);
        self.append_normalized(raw, event, envelope.created)
    }

    /// Record an already-normalized event. The engine can inspect session
    /// ownership and record a seed before this call without parsing twice.
    /// `raw` supplies envelope identity only; the typed event is authoritative.
    pub fn append_normalized(
        &mut self,
        raw: &Value,
        event: AgentEvent,
        timestamp: i64,
    ) -> Option<NormalizedEvent> {
        let session = event.session_id().unwrap_or("").to_string();
        let source_id = source_id(raw);
        if let Some(id) = source_id
            && self.seen.contains(&(session.clone(), id.to_owned()))
        {
            return None;
        }
        let seq = self.next_seq();
        let id = source_id
            .map(str::to_owned)
            .unwrap_or_else(|| format!("vela:{}:{seq}", self.namespace));
        if source_id.is_some() {
            self.seen.insert((session, id.clone()));
        }
        // Locally generated IDs are unique by namespace and sequence. Retaining
        // them at live ingress would add one needless allocation per token;
        // from_records still deduplicates every persisted normalized ID.
        let run_id = self.run_for(&event, explicit_run_id(raw), &id);
        let normalized = NormalizedEvent {
            id,
            seq,
            run_id,
            timestamp,
            event,
        };
        self.records
            .push(RunRecord::Event(Box::new(normalized.clone())));
        Some(normalized)
    }

    pub fn record_snapshot(
        &mut self,
        session_id: &str,
        messages: Vec<Message>,
        phase: SessionPhase,
    ) -> RunRecord {
        let record = RunRecord::Snapshot {
            seq: self.next_seq(),
            session_id: session_id.into(),
            messages,
            phase,
        };
        self.remember_record(&record);
        self.records.push(record.clone());
        record
    }

    pub fn record_local_user(
        &mut self,
        session_id: &str,
        id: String,
        text: String,
        at: i64,
        attachments: Vec<String>,
    ) -> RunRecord {
        let record = RunRecord::LocalUser {
            seq: self.next_seq(),
            session_id: session_id.into(),
            id,
            text,
            at,
            attachments,
        };
        self.remember_record(&record);
        self.records.push(record.clone());
        record
    }

    pub fn record_phase(&mut self, session_id: &str, phase: SessionPhase) -> RunRecord {
        let record = RunRecord::Phase {
            seq: self.next_seq(),
            session_id: session_id.into(),
            phase,
        };
        self.records.push(record.clone());
        record
    }

    pub fn record_quiet_settlement(
        &mut self,
        session_id: &str,
        quiet_ms: u64,
        now: i64,
    ) -> RunRecord {
        let record = RunRecord::QuietSettlement {
            seq: self.next_seq(),
            session_id: session_id.into(),
            quiet_ms,
            now,
        };
        self.records.push(record.clone());
        record
    }

    /// Restore metadata and accepted records from persisted envelopes. Duplicate
    /// event IDs are idempotent even when a delivery repeats an older sequence;
    /// every newly accepted record must have a strictly increasing sequence.
    pub fn from_records(
        namespace: impl Into<String>,
        records: Vec<RunRecord>,
    ) -> Result<Self, ReplayError> {
        let mut log = Self::new(namespace);
        for record in records {
            if let RunRecord::Event(event) = &record {
                if event.id.is_empty() || event.run_id.is_empty() {
                    return Err(ReplayError::InvalidIdentity);
                }
                let key = (
                    event.event.session_id().unwrap_or("").to_string(),
                    event.id.clone(),
                );
                if log.seen.contains(&key) {
                    continue;
                }
            }
            let seq = record.seq();
            if seq <= log.last_seq {
                return Err(ReplayError::NonMonotonicSequence {
                    previous: log.last_seq,
                    current: seq,
                });
            }
            if seq == u64::MAX {
                return Err(ReplayError::SequenceExhausted);
            }
            log.last_seq = seq;
            log.remember_record(&record);
            log.records.push(record);
        }
        Ok(log)
    }

    pub fn replay(&self, session_id: &str) -> Transcript {
        let mut transcript = Transcript::new();
        for record in &self.records {
            if record.session_id() == Some(session_id) {
                record.apply_to(&mut transcript);
            }
        }
        transcript
    }

    fn bind_user(&mut self, session: &str, id: &str, text: &str) -> String {
        let key = (session.to_string(), id.to_string());
        let run = self
            .message_runs
            .get(&key)
            .cloned()
            .or_else(|| {
                // Canonical acknowledgement of an optimistic prompt must retain
                // the run that was already exposed before the source ID arrived.
                self.current.get(session).and_then(|current| {
                    current
                        .user
                        .as_ref()
                        .filter(|(old, old_text)| {
                            old.starts_with("local:")
                                && !id.starts_with("local:")
                                && old_text == text
                        })
                        .map(|_| current.id.clone())
                })
            })
            .unwrap_or_else(|| format!("run:{session}:user:{id}"));
        self.message_runs.insert(key, run.clone());
        self.current.insert(
            session.to_string(),
            CurrentRun {
                id: run.clone(),
                user: Some((id.to_string(), text.to_string())),
            },
        );
        run
    }

    fn run_for(&mut self, event: &AgentEvent, explicit: Option<&str>, fallback: &str) -> String {
        let session = event.session_id().unwrap_or("");
        if let AgentEvent::MessageUpsert { message, .. } = event
            && message.role == Role::User
        {
            return self.bind_user(session, &message.id, user_text(message));
        }
        let key = message_id(event)
            .filter(|id| !id.is_empty())
            .map(|id| (session.to_owned(), id.to_owned()));
        if let Some(run) = key.as_ref().and_then(|key| self.message_runs.get(key)) {
            return run.clone();
        }
        let run = self
            .current
            .get(session)
            .map(|current| current.id.clone())
            .or_else(|| explicit.map(str::to_owned))
            .unwrap_or_else(|| format!("run:{session}:event:{fallback}"));
        self.current
            .entry(session.to_string())
            .or_insert_with(|| CurrentRun {
                id: run.clone(),
                user: None,
            });
        if let Some(key) = key {
            self.message_runs.insert(key, run.clone());
        }
        run
    }

    fn remember_record(&mut self, record: &RunRecord) {
        match record {
            RunRecord::Event(normalized) => {
                let session = normalized.event.session_id().unwrap_or("");
                self.seen
                    .insert((session.to_string(), normalized.id.clone()));
                if let AgentEvent::MessageUpsert { message, .. } = &normalized.event
                    && message.role == Role::User
                {
                    self.message_runs.insert(
                        (session.to_string(), message.id.clone()),
                        normalized.run_id.clone(),
                    );
                    self.current.insert(
                        session.to_string(),
                        CurrentRun {
                            id: normalized.run_id.clone(),
                            user: Some((message.id.clone(), user_text(message).to_owned())),
                        },
                    );
                } else {
                    self.current
                        .entry(session.to_string())
                        .or_insert_with(|| CurrentRun {
                            id: normalized.run_id.clone(),
                            user: None,
                        });
                    if let Some(id) = message_id(&normalized.event).filter(|id| !id.is_empty()) {
                        self.message_runs.insert(
                            (session.to_string(), id.to_string()),
                            normalized.run_id.clone(),
                        );
                    }
                }
            }
            RunRecord::Snapshot {
                session_id,
                messages,
                ..
            } => {
                // Build the snapshot's assignments without walking the current
                // pointer through old user turns: that pointer may still name
                // an optimistic latest prompt awaiting its canonical ID.
                let previous = self.current.get(session_id).cloned();
                let last_user = messages
                    .iter()
                    .rfind(|message| message.role == Role::User)
                    .map(|message| message.id.as_str());
                let mut run = None;
                let mut latest_user = None;
                for message in messages {
                    let key = (session_id.clone(), message.id.clone());
                    if message.role == Role::User {
                        let user_run = self
                            .message_runs
                            .get(&key)
                            .cloned()
                            .or_else(|| {
                                previous.as_ref().and_then(|current| {
                                    current
                                        .user
                                        .as_ref()
                                        .filter(|(id, text)| {
                                            last_user == Some(message.id.as_str())
                                                && id.starts_with("local:")
                                                && text == user_text(message)
                                        })
                                        .map(|_| current.id.clone())
                                })
                            })
                            .unwrap_or_else(|| format!("run:{session_id}:user:{}", message.id));
                        self.message_runs.insert(key, user_run.clone());
                        run = Some(user_run);
                        latest_user = Some((message.id.clone(), user_text(message).to_owned()));
                    } else if message.role == Role::Assistant {
                        let message_run = self
                            .message_runs
                            .get(&key)
                            .cloned()
                            .or_else(|| run.clone())
                            .unwrap_or_else(|| format!("run:{session_id}:message:{}", message.id));
                        self.message_runs.insert(key, message_run.clone());
                        run = Some(message_run);
                    }
                }
                if let Some(id) = run {
                    self.current.insert(
                        session_id.clone(),
                        CurrentRun {
                            id,
                            user: latest_user,
                        },
                    );
                }
            }
            RunRecord::LocalUser {
                session_id,
                id,
                text,
                ..
            } => {
                self.bind_user(session_id, id, text);
            }
            RunRecord::Phase { .. } | RunRecord::QuietSettlement { .. } => {}
        }
    }
}

fn user_text(message: &Message) -> &str {
    message
        .parts
        .first()
        .map(|part| part.kind.text())
        .unwrap_or("")
}

fn metadata_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
    })
}

fn source_id(raw: &Value) -> Option<&str> {
    // `_velaEventId` is transport-owned metadata for an SSE `id:` field.
    metadata_string(
        raw,
        &["_velaEventId", "eventID", "eventId", "event_id", "id"],
    )
    .or_else(|| {
        raw.get("payload")
            .and_then(|payload| metadata_string(payload, &["eventID", "eventId", "event_id", "id"]))
    })
}

fn explicit_run_id(raw: &Value) -> Option<&str> {
    let keys = ["runID", "runId", "run_id", "executionID", "executionId"];
    metadata_string(raw, &keys)
        .or_else(|| {
            raw.get("payload")
                .and_then(|value| metadata_string(value, &keys))
        })
        .or_else(|| {
            raw.get("data")
                .and_then(|value| metadata_string(value, &keys))
        })
}

fn message_id(event: &AgentEvent) -> Option<&str> {
    match event {
        AgentEvent::MessageUpsert { message, .. } => Some(&message.id),
        AgentEvent::RetryScheduled { message_id, .. }
        | AgentEvent::StepStarted { message_id, .. }
        | AgentEvent::StepEnded { message_id, .. }
        | AgentEvent::StepFailed { message_id, .. }
        | AgentEvent::TextStarted { message_id, .. }
        | AgentEvent::TextDelta { message_id, .. }
        | AgentEvent::TextEnded { message_id, .. }
        | AgentEvent::ReasoningStarted { message_id, .. }
        | AgentEvent::ReasoningDelta { message_id, .. }
        | AgentEvent::ReasoningEnded { message_id, .. }
        | AgentEvent::ToolInputStarted { message_id, .. }
        | AgentEvent::ToolInputDelta { message_id, .. }
        | AgentEvent::ToolInputEnded { message_id, .. }
        | AgentEvent::ToolCalled { message_id, .. }
        | AgentEvent::ToolProgress { message_id, .. }
        | AgentEvent::ToolFinished { message_id, .. }
        | AgentEvent::MessageRemoved { message_id, .. }
        | AgentEvent::PartUpsert { message_id, .. } => Some(message_id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::ToolStatus;
    use serde_json::json;

    fn wire(id: &str, kind: &str, data: Value) -> Value {
        json!({"id":id,"type":kind,"data":data})
    }

    fn send(log: &mut EventLog, id: &str, kind: &str, data: Value, at: i64) -> NormalizedEvent {
        log.append(&wire(id, kind, data), at).unwrap()
    }

    #[test]
    fn ten_thousand_deltas_round_trip_and_replay_without_reading_a_clock() {
        let mut log = EventLog::new("fixture");
        log.record_snapshot(
            "s",
            vec![Message::text("u", Role::User, 1, "Inspect")],
            SessionPhase::Idle,
        );
        send(
            &mut log,
            "start",
            "session.step.started",
            json!({"sessionID":"s","assistantMessageID":"a"}),
            2,
        );
        for index in 0..10_000 {
            send(
                &mut log,
                &format!("delta-{index}"),
                "session.text.delta",
                json!({
                    "sessionID":"s","assistantMessageID":"a","ordinal":0,"delta":"x"
                }),
                3 + index,
            );
        }
        send(
            &mut log,
            "end",
            "session.step.ended",
            json!({"sessionID":"s","assistantMessageID":"a","finish":"stop"}),
            10_004,
        );
        let bytes = serde_json::to_vec(log.records()).unwrap();
        let records: Vec<RunRecord> = serde_json::from_slice(&bytes).unwrap();
        let restored = EventLog::from_records("another-process", records).unwrap();
        let mut transcript = restored.replay("s");
        let blocks = transcript.blocks();
        assert_eq!(blocks.last().unwrap().text.as_ref(), "x".repeat(10_000));
        assert_eq!(blocks.last().unwrap().completed_at, Some(10_004));
        assert!(!blocks.last().unwrap().streaming);
        assert_eq!(log.last_seq(), restored.last_seq());
        let fingerprints = |mut transcript: Transcript| {
            transcript
                .blocks()
                .iter()
                .map(|block| block.fingerprint())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            fingerprints(log.replay("s")),
            fingerprints(restored.replay("s"))
        );
    }

    #[test]
    fn duplicates_are_scoped_to_session_but_identical_idless_deltas_are_distinct() {
        let mut log = EventLog::new("fixture");
        let data = json!({"sessionID":"s","assistantMessageID":"a","delta":"ha"});
        let explicit = wire("one", "session.text.delta", data.clone());
        let first = log.append(&explicit, 10).unwrap();
        assert!(log.append(&explicit, 20).is_none());
        let idless = json!({"type":"session.text.delta","data":data});
        let second = log.append(&idless, 30).unwrap();
        let third = log.append(&idless, 40).unwrap();
        assert_ne!(second.id, third.id);
        assert_eq!(first.timestamp, 10);
        assert_eq!(second.timestamp, 30);
        assert!(
            log.append(
                &wire(
                    "one",
                    "session.text.delta",
                    json!({"sessionID":"other","assistantMessageID":"b","delta":"ha"})
                ),
                50
            )
            .is_some()
        );
        assert_eq!(log.replay("s").messages()[0].parts[0].kind.text(), "hahaha");
        let mut persisted = log.records().to_vec();
        persisted.push(RunRecord::Event(Box::new(first)));
        let restored = EventLog::from_records("new", persisted).unwrap();
        assert_eq!(
            restored.replay("s").messages()[0].parts[0].kind.text(),
            "hahaha"
        );
    }

    #[test]
    fn run_survives_steps_retry_and_late_previous_run_events() {
        let mut log = EventLog::new("fixture");
        log.record_local_user("s", "local:u".into(), "Inspect".into(), 1, Vec::new());
        let first = send(
            &mut log,
            "a",
            "session.step.started",
            json!({"sessionID":"s","assistantMessageID":"a"}),
            2,
        );
        let ack = send(
            &mut log,
            "user-ack",
            "message.updated",
            json!({"sessionID":"s","info":{"id":"u","role":"user","time":{"created":1},"parts":[{"id":"up","type":"text","text":"Inspect"}]}}),
            3,
        );
        assert_eq!(first.run_id, ack.run_id);
        let retry = send(
            &mut log,
            "retry",
            "session.retry.scheduled",
            json!({"sessionID":"s","assistantMessageID":"a","attempt":2}),
            4,
        );
        let second_step = send(
            &mut log,
            "b",
            "session.step.started",
            json!({"sessionID":"s","assistantMessageID":"b"}),
            5,
        );
        assert_eq!(first.run_id, retry.run_id);
        assert_eq!(first.run_id, second_step.run_id);
        log.record_local_user(
            "s",
            "local:next".into(),
            "Next request".into(),
            6,
            Vec::new(),
        );
        let next = send(
            &mut log,
            "c",
            "session.step.started",
            json!({"sessionID":"s","assistantMessageID":"c"}),
            7,
        );
        assert_ne!(next.run_id, first.run_id);
        let late = send(
            &mut log,
            "late",
            "session.tool.success",
            json!({"sessionID":"s","assistantMessageID":"a","id":"old-call"}),
            8,
        );
        assert_eq!(late.run_id, first.run_id);
        let restored = EventLog::from_records("restart", log.records().to_vec()).unwrap();
        assert_eq!(restored.current["s"].id, next.run_id);
    }

    #[test]
    fn parallel_calls_keep_spawn_order_when_completions_arrive_out_of_order() {
        let mut log = EventLog::new("fixture");
        for (index, call) in ["one", "two", "three"].iter().enumerate() {
            send(
                &mut log,
                &format!("spawn-{call}"),
                "session.tool.input.started",
                json!({"sessionID":"s","assistantMessageID":"a","id":call,"name":"task"}),
                index as i64 + 1,
            );
        }
        for call in ["three", "one", "two"] {
            send(
                &mut log,
                &format!("done-{call}"),
                "session.tool.success",
                json!({"sessionID":"s","assistantMessageID":"a","id":call}),
                10,
            );
        }
        let mut transcript = log.replay("s");
        let blocks = transcript.blocks();
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.id.as_ref())
                .collect::<Vec<_>>(),
            ["one", "two", "three"]
        );
        assert!(
            blocks
                .iter()
                .all(|block| block.status == ToolStatus::Success)
        );
        let input_delta = json!({"type":"session.tool.input.delta","data":{"sessionID":"s","assistantMessageID":"a","id":"one","delta":"x"}});
        assert!(log.append(&input_delta, 11).is_some());
        assert!(
            log.append(&input_delta, 12).is_some(),
            "tool data.id must not deduplicate event deliveries"
        );
    }

    #[test]
    fn snapshot_phase_and_quiet_settlement_survive_serialization() {
        let mut log = EventLog::new("fixture");
        log.record_snapshot(
            "s",
            vec![Message::new("a", Role::Assistant, 10)],
            SessionPhase::Idle,
        );
        assert_eq!(log.replay("s").phase(), SessionPhase::Idle);
        log.record_phase("s", SessionPhase::Busy);
        log.record_quiet_settlement("s", 60_001, 70_000);
        let bytes = serde_json::to_vec(log.records()).unwrap();
        let restored =
            EventLog::from_records("restored", serde_json::from_slice(&bytes).unwrap()).unwrap();
        let transcript = restored.replay("s");
        assert_eq!(transcript.phase(), SessionPhase::Idle);
        assert_eq!(transcript.messages()[0].completed_at, Some(70_000));
    }

    #[test]
    fn draining_payloads_retains_duplicate_and_run_metadata() {
        let mut log = EventLog::new("fixture");
        let raw = wire(
            "a",
            "session.text.delta",
            json!({"sessionID":"s","assistantMessageID":"a","delta":"one"}),
        );
        let first = log.append(&raw, 10).unwrap();
        let persisted = log.drain_records();
        assert!(log.records().is_empty());
        assert!(log.append(&raw, 20).is_none());
        let next = send(
            &mut log,
            "b",
            "session.text.delta",
            json!({"sessionID":"s","assistantMessageID":"a","delta":"two"}),
            30,
        );
        assert!(next.seq > first.seq);
        assert_eq!(next.run_id, first.run_id);
        assert_eq!(
            EventLog::from_records("restore", persisted)
                .unwrap()
                .last_seq(),
            first.seq
        );
    }

    #[test]
    fn canonical_snapshot_keeps_the_run_of_an_optimistic_prompt() {
        let mut log = EventLog::new("fixture");
        log.record_local_user("s", "local:prompt".into(), "Inspect".into(), 1, Vec::new());
        let first = send(
            &mut log,
            "first",
            "session.step.started",
            json!({"sessionID":"s","assistantMessageID":"a"}),
            2,
        );
        log.record_snapshot(
            "s",
            vec![
                Message::text("canonical-user", Role::User, 1, "Inspect"),
                Message::new("a", Role::Assistant, 2),
            ],
            SessionPhase::Busy,
        );
        let mut restored = EventLog::from_records("restarted", log.records().to_vec()).unwrap();
        let next = send(
            &mut restored,
            "next",
            "session.step.started",
            json!({"sessionID":"s","assistantMessageID":"b"}),
            3,
        );
        assert_eq!(first.run_id, next.run_id);
    }

    #[test]
    fn transport_identity_and_source_timestamp_survive_recording() {
        let mut log = EventLog::new("fixture");
        let raw = json!({
            "_velaEventId":"frame-1", "created":123,
            "type":"session.text.delta",
            "data":{"id":"not-an-event-id","sessionID":"s","assistantMessageID":"a","delta":"one"}
        });
        let event = log.append(&raw, 999).unwrap();
        assert_eq!(event.id, "frame-1");
        assert_eq!(event.timestamp, 123);
        assert!(matches!(event.event, AgentEvent::TextDelta { at: 123, .. }));
        assert!(log.append(&raw, 1_000).is_none());
    }

    #[test]
    fn snapshot_does_not_assign_an_optimistic_run_to_older_identical_prompts() {
        let mut log = EventLog::new("fixture");
        log.record_local_user("s", "local:latest".into(), "Again".into(), 10, Vec::new());
        log.record_snapshot(
            "s",
            vec![
                Message::text("old-user", Role::User, 1, "Again"),
                Message::new("old-answer", Role::Assistant, 2),
                Message::text("new-user", Role::User, 10, "Again"),
            ],
            SessionPhase::Busy,
        );
        let old = send(
            &mut log,
            "old",
            "session.text.delta",
            json!({"sessionID":"s","assistantMessageID":"old-answer","delta":"old"}),
            11,
        );
        let new = send(
            &mut log,
            "new",
            "session.text.delta",
            json!({"sessionID":"s","assistantMessageID":"new-answer","delta":"new"}),
            12,
        );
        assert_ne!(old.run_id, new.run_id);
        assert_eq!(new.run_id, "run:s:user:local:latest");
    }

    #[test]
    fn idless_ingress_does_not_retain_token_ids_but_persisted_duplicates_are_idempotent() {
        let mut log = EventLog::new("fixture");
        let raw = json!({"type":"session.text.delta","data":{
            "sessionID":"s","assistantMessageID":"a","delta":"x"
        }});
        let first = log.append(&raw, 1).unwrap();
        for index in 0..10_000 {
            log.append(&raw, index + 2).unwrap();
        }
        assert!(log.seen.is_empty());
        let mut persisted = log.drain_records();
        persisted.push(RunRecord::Event(Box::new(first)));
        let restored = EventLog::from_records("new", persisted).unwrap();
        assert_eq!(restored.records().len(), 10_001);
        assert_eq!(
            restored.replay("s").messages()[0].parts[0]
                .kind
                .text()
                .len(),
            10_001
        );
    }

    #[test]
    fn canonical_retirement_releases_only_the_deleted_sessions_metadata() {
        let mut log = EventLog::new("fixture");
        for session in ["s", "other"] {
            send(
                &mut log,
                "start",
                "session.step.started",
                json!({
                    "sessionID":session,"assistantMessageID":"a"
                }),
                1,
            );
        }
        let count = log.records().len();
        log.retire_session("s");
        assert_eq!(log.records().len(), count);
        assert!(log.seen.iter().all(|(session, _)| session == "other"));
        assert!(
            log.message_runs
                .keys()
                .all(|(session, _)| session == "other")
        );
        assert!(!log.current.contains_key("s"));
        assert!(log.current.contains_key("other"));
    }

    #[test]
    fn replay_rejects_new_records_with_regressing_sequences() {
        let records = vec![
            RunRecord::Phase {
                seq: 2,
                session_id: "s".into(),
                phase: SessionPhase::Busy,
            },
            RunRecord::Phase {
                seq: 1,
                session_id: "s".into(),
                phase: SessionPhase::Idle,
            },
        ];
        assert!(matches!(
            EventLog::from_records("x", records),
            Err(ReplayError::NonMonotonicSequence { .. })
        ));
    }
}
