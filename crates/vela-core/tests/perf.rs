use std::time::{Duration, Instant};

use vela_core::layout::LayoutCache;
use vela_core::session::SessionSummary;
use vela_core::transcript::{Message, Part, PartKind, Role, ToolPart, ToolStatus, Transcript};
use vela_core::warm::{WarmCache, WarmEntry};

const SESSIONS: usize = 6;
const MESSAGES: usize = 400;

fn paragraph(ix: usize) -> String {
    format!(
        "Paragraph {ix}. Vela renders long conversations natively. This block is intentionally \
         representative of prose with **markdown**, `inline code`, and enough words to wrap onto \
         several lines in the transcript column without requiring a full-document reparse."
    )
}

fn session_fixture(index: usize) -> Transcript {
    let mut messages = Vec::with_capacity(MESSAGES);
    for message_ix in 0..MESSAGES {
        let created = (index * MESSAGES + message_ix) as i64;
        if message_ix % 2 == 0 {
            messages.push(Message::text(
                format!("m{index}-{message_ix}"),
                Role::User,
                created,
                format!("Request {message_ix}: adjust the workspace and explain the change."),
            ));
        } else {
            let id = format!("a{index}-{message_ix}");
            let mut message = Message::new(&id, Role::Assistant, created);
            message.agent = Some("build".into());
            message.completed_at = Some(created + 400);
            message.parts.push(Part {
                id: format!("{id}:reasoning:0"),
                ordinal: 0,
                kind: PartKind::Reasoning {
                    text: "Inspect the workspace, then apply the requested change.".into(),
                    streaming: false,
                },
            });
            message.parts.push(Part {
                id: format!("{id}:text:0"),
                ordinal: 1,
                kind: PartKind::Text {
                    text: format!(
                        "## Result\n\n{}\n\n{}\n\n```rust\nfn demo() -> usize {{ {} }}\n```\n\nDone.",
                        paragraph(message_ix),
                        paragraph(message_ix + 1),
                        message_ix
                    ),
                    streaming: false,
                },
            });
            message.parts.push(Part {
                id: format!("{id}:call"),
                ordinal: 2,
                kind: PartKind::Tool(Box::new(ToolPart {
                    call_id: format!("{id}:call"),
                    name: "bash".into(),
                    title: "bash".into(),
                    status: ToolStatus::Success,
                    input: "{\"command\":\"cargo test\"}".into(),
                    output: "test result: ok. 27 passed".into(),
                    detail: String::new(),
                    error: None,
                    started_at: Some(created + 50),
                    finished_at: Some(created + 100),
                    child_session_id: None,
                    files: Vec::new(),
                    sub_calls: Vec::new(),
                })),
            });
            messages.push(message);
        }
    }
    let mut transcript = Transcript::new();
    transcript.hydrate(messages);
    let _ = transcript.blocks();
    transcript
}

fn summary(index: usize) -> SessionSummary {
    SessionSummary {
        id: format!("ses_{index}"),
        title: format!("Large session {index}"),
        directory: "/tmp/perf-workspace".into(),
        updated_at: index as i64,
        parent_id: None,
        agent: Some("build".into()),
    }
}

fn percentile(mut samples: Vec<Duration>, p: f64) -> Duration {
    samples.sort();
    let index = ((samples.len() as f64 - 1.0) * p).round() as usize;
    samples[index]
}

#[test]
#[ignore = "performance measurement; run with --release --ignored --nocapture"]
fn transcript_and_warm_navigation() {
    let build_start = Instant::now();
    let fixtures: Vec<Transcript> = (0..SESSIONS).map(session_fixture).collect();
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;

    let block_counts: Vec<usize> = fixtures
        .iter()
        .map(|transcript| {
            let mut transcript = transcript.clone();
            transcript.blocks().len()
        })
        .collect();

    let mut cache = WarmCache::new(SESSIONS, 256 * 1024 * 1024);
    for (index, transcript) in fixtures.into_iter().enumerate() {
        cache.insert(WarmEntry::new(summary(index), transcript));
    }
    let retained_mb = cache.retained_bytes() as f64 / 1_048_576.0;

    let mut layout = LayoutCache::default();
    let mut samples = Vec::new();
    for round in 0..120 {
        let id = format!("ses_{}", round % SESSIONS);
        let start = Instant::now();
        let entry = cache.get(&id).expect("warm session");
        let blocks = entry.transcript.blocks();
        layout.sync(blocks);
        samples.push(start.elapsed());
    }
    let p50 = percentile(samples.clone(), 0.50);
    let p95 = percentile(samples, 0.95);

    // Streaming cost must depend on the changed part, not on the other 399
    // messages. Apply 500 deltas to one assistant part and time the projection.
    let mut streaming = session_fixture(0);
    let stream_start = Instant::now();
    for ix in 0..500 {
        streaming.apply(&vela_core::event::AgentEvent::TextDelta {
            session_id: "ses_0".into(),
            message_id: "a0-399".into(),
            part_id: "a0-399:text:0".into(),
            ordinal: 1,
            delta: format!(" token{ix}"),
            at: 1_000 + ix,
        });
        let _ = streaming.blocks();
    }
    let streaming_ms = stream_start.elapsed().as_secs_f64() * 1000.0;

    println!(
        "{{\"sessions\":{SESSIONS},\"messages_per_session\":{MESSAGES},\"blocks\":{:?},\
         \"fixture_build_ms\":{build_ms:.2},\"warm_retained_mb\":{retained_mb:.2},\
         \"switch_p50_ms\":{:.3},\"switch_p95_ms\":{:.3},\"stream_500_deltas_ms\":{streaming_ms:.2}}}",
        block_counts,
        p50.as_secs_f64() * 1000.0,
        p95.as_secs_f64() * 1000.0,
    );
}
