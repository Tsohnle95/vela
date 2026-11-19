use super::*;
use crate::run_log::{EventLog, RunRecord};
use serde_json::json;

fn apply(transcript: &mut Transcript, kind: &str, data: Value, at: i64) {
    transcript.apply(
        &crate::event::parse_event(&json!({
            "type": kind, "created": at, "data": data,
        }))
        .unwrap(),
    );
}

fn failed_tool() -> Transcript {
    let mut transcript = Transcript::new();
    apply(
        &mut transcript,
        "session.tool.input.started",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call", "name":"read"
        }),
        1,
    );
    apply(
        &mut transcript,
        "session.tool.called",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call", "input":{"path":"old"}
        }),
        2,
    );
    apply(
        &mut transcript,
        "session.tool.progress",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call", "progress":"Old progress"
        }),
        3,
    );
    apply(
        &mut transcript,
        "session.tool.failed",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call",
            "content":[{"type":"text", "text":"Old output"}],
            "error":{"code":"denied", "message":"Permission denied"}
        }),
        4,
    );
    transcript
}

fn tool(transcript: &Transcript) -> &ToolPart {
    match &transcript.message("m").unwrap().parts[0].kind {
        PartKind::Tool(tool) => tool,
        _ => panic!("expected tool"),
    }
}

#[test]
fn tool_retry_preserves_failure_and_identity_but_clears_attempt_local_fields() {
    let mut transcript = failed_tool();
    let id = transcript.blocks()[0].id.clone();
    let started_at = tool(&transcript).started_at;
    apply(
        &mut transcript,
        "session.tool.input.started",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call", "name":"read"
        }),
        5,
    );
    let retry = tool(&transcript);
    assert!(retry.output.is_empty());
    assert!(retry.input.is_empty());
    assert!(retry.error.is_none());
    assert!(retry.progress.is_empty());
    assert!(retry.finished_at.is_none());
    assert_eq!(retry.started_at, started_at);
    assert_eq!(retry.failure_history.len(), 1);
    assert!(retry.failure_history[0].contains("Permission denied"));
    assert!(retry.failure_history[0].contains("Old output"));
    assert_eq!(transcript.blocks()[0].id, id);
    apply(
        &mut transcript,
        "session.tool.input.delta",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call", "delta":"{\"path\":\"new\"}"
        }),
        6,
    );
    apply(
        &mut transcript,
        "session.tool.called",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call", "input":{"path":"new"}
        }),
        7,
    );
    apply(
        &mut transcript,
        "session.tool.success",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "id":"call",
            "content":[{"type":"text", "text":"New result"}]
        }),
        8,
    );
    let current = tool(&transcript);
    assert_eq!(current.status, ToolStatus::Success);
    assert_eq!(current.output, "New result");
    assert_eq!(current.started_at, started_at);
    assert_eq!(current.failure_history.len(), 1);
    let blocks = transcript.blocks();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].id, id);
    assert!(blocks[0].output.contains("Old output"));
    assert!(blocks[0].output.contains("Permission denied"));
    assert!(blocks[0].output.contains("New result"));
}

#[test]
fn authoritative_tool_updates_keep_call_identity_and_failure_disclosure() {
    let mut transcript = failed_tool();
    let mut next = tool(&transcript).clone();
    next.status = ToolStatus::Running;
    next.started_at = Some(50);
    next.failure_history.clear();
    transcript.apply(&AgentEvent::PartUpsert {
        session_id: "s".into(),
        message_id: "m".into(),
        part: Part {
            id: "authoritative-part".into(),
            ordinal: 0,
            kind: PartKind::Tool(Box::new(next)),
        },
        at: 5,
    });
    let current = tool(&transcript);
    assert_eq!(current.started_at, Some(2));
    assert!(current.output.is_empty());
    assert!(current.progress.is_empty());
    assert!(current.error.is_none());
    assert!(current.finished_at.is_none());
    assert!(current.failure_history[0].contains("Permission denied"));
    assert_eq!(transcript.blocks()[0].id.as_ref(), "call");

    let mut authoritative = transcript.message("m").unwrap().clone();
    authoritative.failure_history.clear();
    let PartKind::Tool(next) = &mut authoritative.parts[0].kind else {
        unreachable!()
    };
    next.failure_history.clear();
    next.status = ToolStatus::Success;
    next.output = "New result".into();
    authoritative.parts[0].id = "yet-another-part".into();
    transcript.hydrate(vec![authoritative]);
    let blocks = transcript.blocks();
    assert_eq!(blocks[0].id.as_ref(), "call");
    assert!(blocks[0].output.contains("Permission denied"));
    assert!(blocks[0].output.contains("New result"));
}

#[test]
fn assistant_retry_keeps_one_stable_disclosure_until_success() {
    let mut transcript = Transcript::new();
    apply(
        &mut transcript,
        "session.step.failed",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "error":{"message":"Provider unavailable"}
        }),
        1,
    );
    apply(
        &mut transcript,
        "session.retry.scheduled",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "attempt":2,
            "error":{"message":"Retry after a delay"}, "at":10
        }),
        2,
    );
    let blocks = transcript.blocks();
    assert!(!blocks.iter().any(|block| block.kind == BlockKind::Error));
    let retry = blocks
        .iter()
        .find(|block| block.kind == BlockKind::Retry)
        .unwrap();
    assert_eq!(retry.id.as_ref(), "m:retry");
    assert!(retry.output.contains("Provider unavailable"));
    assert!(retry.output.contains("Retry after a delay"));
    apply(
        &mut transcript,
        "session.step.started",
        json!({
            "sessionID":"s", "assistantMessageID":"m"
        }),
        3,
    );
    let retry = transcript
        .blocks()
        .iter()
        .find(|block| block.kind == BlockKind::Retry)
        .unwrap();
    assert_eq!(retry.id.as_ref(), "m:retry");
    assert_eq!(retry.status, ToolStatus::Running);
    apply(
        &mut transcript,
        "session.step.ended",
        json!({
            "sessionID":"s", "assistantMessageID":"m", "finish":"stop"
        }),
        4,
    );
    let retry = transcript
        .blocks()
        .iter()
        .find(|block| block.kind == BlockKind::Retry)
        .unwrap();
    assert_eq!(retry.id.as_ref(), "m:retry");
    assert_eq!(retry.status, ToolStatus::Success);
    assert_eq!(retry.text.as_ref(), "Recovered after retry");
    assert!(retry.output.contains("Provider unavailable"));
    assert!(transcript.message("m").unwrap().error.is_none());

    let mut replacement = Message::new("m", Role::Assistant, 1);
    replacement.completed_at = Some(4);
    replacement.finish = Some("stop".into());
    transcript.apply(&AgentEvent::MessageUpsert {
        session_id: "s".into(),
        message: replacement,
    });
    assert!(
        transcript
            .blocks()
            .iter()
            .find(|block| block.kind == BlockKind::Retry)
            .unwrap()
            .output
            .contains("Provider unavailable")
    );
}

#[test]
fn serialized_retry_events_and_canonical_messages_replay_identically() {
    let mut log = EventLog::new("retry-fixture");
    let events = [
        (
            "session.step.failed",
            json!({"sessionID":"s","assistantMessageID":"m","error":{"message":"Failure"}}),
        ),
        (
            "session.retry.scheduled",
            json!({"sessionID":"s","assistantMessageID":"m","attempt":2,"error":{"message":"Retrying provider"}}),
        ),
        (
            "session.step.started",
            json!({"sessionID":"s","assistantMessageID":"m"}),
        ),
        (
            "session.step.ended",
            json!({"sessionID":"s","assistantMessageID":"m","finish":"stop"}),
        ),
    ];
    for (index, (kind, data)) in events.into_iter().enumerate() {
        log.append(
            &json!({"id":format!("event-{index}"),"type":kind,"data":data}),
            index as i64 + 1,
        )
        .unwrap();
    }
    let serialized = serde_json::to_vec(log.records()).unwrap();
    let records: Vec<RunRecord> = serde_json::from_slice(&serialized).unwrap();
    let restored = EventLog::from_records("restarted", records).unwrap();
    let mut live = log.replay("s");
    let mut replay = restored.replay("s");
    assert_eq!(
        live.blocks()
            .iter()
            .map(Block::fingerprint)
            .collect::<Vec<_>>(),
        replay
            .blocks()
            .iter()
            .map(Block::fingerprint)
            .collect::<Vec<_>>()
    );
    let messages = serde_json::to_vec(live.messages()).unwrap();
    let mut snapshot = Transcript::new();
    snapshot.hydrate(serde_json::from_slice(&messages).unwrap());
    assert_eq!(
        live.blocks()
            .iter()
            .map(Block::fingerprint)
            .collect::<Vec<_>>(),
        snapshot
            .blocks()
            .iter()
            .map(Block::fingerprint)
            .collect::<Vec<_>>()
    );
}

#[test]
fn history_fields_default_for_older_serialized_domain_records() {
    let mut tool = serde_json::to_value(ToolPart::default()).unwrap();
    tool.as_object_mut().unwrap().remove("failure_history");
    assert!(
        serde_json::from_value::<ToolPart>(tool)
            .unwrap()
            .failure_history
            .is_empty()
    );
    let mut message = serde_json::to_value(Message::new("m", Role::Assistant, 1)).unwrap();
    message.as_object_mut().unwrap().remove("failure_history");
    assert!(
        serde_json::from_value::<Message>(message)
            .unwrap()
            .failure_history
            .is_empty()
    );
}

#[test]
fn canonical_tool_name_is_preserved_when_title_is_a_human_summary() {
    let mut transcript = Transcript::new();
    let mut message = Message::new("m", Role::Assistant, 1);
    message.parts.push(Part {
        id: "call".into(),
        ordinal: 0,
        kind: PartKind::Tool(Box::new(ToolPart {
            call_id: "call".into(),
            name: "read".into(),
            title: "Inspecting the architecture".into(),
            status: ToolStatus::Running,
            ..Default::default()
        })),
    });
    transcript.hydrate(vec![message]);
    assert_eq!(transcript.blocks()[0].detail.as_ref(), "read");
}
