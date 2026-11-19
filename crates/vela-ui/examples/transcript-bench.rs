//! Native opt-in benchmark. The engine and OpenCode wire fixture are disposable.
#[path = "../../vela-engine/tests/support/streaming_publication.rs"]
#[allow(dead_code)]
mod fixture;

use serde_json::json;
use std::{sync::{Arc, Mutex}, time::{Duration, Instant}};
use vela_ui::{LiveProbe, run_native_transcript_bench};

#[derive(Clone, Copy)]
enum Payload {
    Ordinary,
    Paragraph,
    Fence,
    Table,
    Tool,
    Combined,
}

impl Payload {
    fn parse(value: &str) -> Self {
        match value {
            "ordinary" => Self::Ordinary,
            "paragraph" => Self::Paragraph,
            "fence" => Self::Fence,
            "table" => Self::Table,
            "tool" => Self::Tool,
            "combined" => Self::Combined,
            _ => panic!("use ordinary, paragraph, fence, table, tool, or combined"),
        }
    }

    fn source(self) -> String {
        let paragraph = || "a long paragraph ".repeat(4_096);
        let fence = || format!("```text\n{}```", "long fenced output\n".repeat(8_192));
        let table = || format!("| Field | Value |\n| --- | --- |\n{}",
            "| fixture | wide table cell data |\n".repeat(512));
        match self {
            Self::Ordinary | Self::Tool => "The streamed answer has finished.".into(),
            Self::Paragraph => paragraph(),
            Self::Fence => fence(),
            Self::Table => table(),
            Self::Combined => [paragraph(), fence(), table()].join("\n\n"),
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let count: usize = args.next().unwrap_or_else(|| "1".into()).parse().unwrap();
    assert!(matches!(count, 1 | 8), "use one or eight panels");
    let mode = args.next().unwrap_or_else(|| "combined".into());
    let payload = Payload::parse(&mode);
    let streaming_count = args.next().map_or(count, |value| value.parse().unwrap());
    assert!((1..=count).contains(&streaming_count), "streaming panels must fit in the window");
    eprintln!("workload: {mode}, {streaming_count}/{count} panels streaming");
    let ids = (0..count).map(|index| format!("ses_stress_{index}")).collect::<Vec<_>>();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().worker_threads(2).build().unwrap();
    let fixture = Arc::new(runtime.block_on(fixture::Fixture::stress(ids.clone(), 1_600)));
    let probes = Arc::new(Mutex::new(Vec::new()));
    let (start, started) = tokio::sync::oneshot::channel();
    let (finished, done) = tokio::sync::oneshot::channel();
    let sender = fixture.clone();
    let marks = probes.clone();
    let sessions = ids[..streaming_count].to_vec();
    let visual_check = std::env::var_os("VELA_TRANSCRIPT_VISUAL_CHECK").is_some();
    let producer = runtime.spawn(async move {
        const DELTA: &str = "a **streamed** word ";
        started.await.unwrap();
        for id in &sessions {
            if visual_check {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
                sender.user_arrived(id, "visual-check", "Show the changes and run the workspace checks.", now);
            }
            sender.send("session.step.started", json!({"sessionID":id,"assistantMessageID":"stress-live"}));
        }
        let mut pace = tokio::time::interval(Duration::from_millis(100));
        for burst in 1..=40 {
            pace.tick().await;
            marks.lock().unwrap().push(LiveProbe {
                started: Instant::now(), source_bytes: burst * 10 * DELTA.len() - 1, tool_bytes: 0,
            });
            for id in &sessions {
                for _ in 0..10 {
                    sender.send("session.text.delta", json!({"sessionID":id,"assistantMessageID":"stress-live",
                        "partID":"stress-line","ordinal":0,"delta":DELTA}));
                }
            }
        }
        pace.tick().await;
        let text = payload.source();
        let output = if matches!(payload, Payload::Combined | Payload::Tool) {
            "0123456789abcdef".repeat(65_536)
        } else {
            "Done".into()
        };
        marks.lock().unwrap().push(LiveProbe {
            started: Instant::now(), source_bytes: 0, tool_bytes: output.len(),
        });
        for id in &sessions {
            sender.send("session.text.ended", json!({"sessionID":id,"assistantMessageID":"stress-live",
                "partID":"stress-line","ordinal":0,"text":text}));
            sender.send("session.tool.input.started", json!({"sessionID":id,"assistantMessageID":"stress-live",
                "id":"stress-output","name":"bash"}));
            sender.send("session.tool.success", json!({"sessionID":id,"assistantMessageID":"stress-live",
                "id":"stress-output","content":[{"type":"text","text":output}]}));
            sender.send("session.execution.succeeded", json!({"sessionID":id}));
        }
        fixture::wait_for(|| sessions.iter().all(|id| {
            let snapshot = sender.engine.view(id).unwrap().presentation();
            !snapshot.phase.is_active() && snapshot.blocks.iter().any(|block| block.output.len() == output.len())
        })).await;
        finished.send(()).ok();
    });
    run_native_transcript_bench(
        fixture.engine.clone(), ids, streaming_count, probes, start, done,
        matches!(payload, Payload::Tool),
    );
    runtime.block_on(async {
        producer.await.unwrap();
        fixture.engine.shutdown_and_wait().await;
    });
}
