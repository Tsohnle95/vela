#[path = "support/streaming_publication.rs"]
mod fixture;

use fixture::{Fixture, wait_for};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;
use vela_core::{session::SessionPhase, transcript::BlockKind};
use vela_engine::EngineUpdate;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_thousand_wire_deltas_are_batched_deduplicated_and_replay_from_disk() {
    let fixture = Fixture::start().await;
    let view = fixture.engine.view("ses_stream").unwrap();
    let mut updates = fixture.engine.subscribe();
    let started = std::time::Instant::now();
    fixture.send_identified("start", "session.step.started", json!({"sessionID":"ses_stream","assistantMessageID":"live"}));
    for batch in 0..100 {
        for item in 0..100 {
            let id = format!("token-{}", batch * 100 + item);
            let data = json!({"sessionID":"ses_stream","assistantMessageID":"live","ordinal":0,"delta":"x"});
            fixture.send_identified(&id, "session.text.delta", data.clone());
            if item % 10 == 0 { fixture.send_identified(&id, "session.text.delta", data); }
        }
        // Advance at acknowledged publication boundaries rather than racing
        // the fixture's finite SSE buffer against machine/disk scheduling.
        // Every frame still contains 100 deltas plus duplicate deliveries.
        let expected = (batch + 1) * 100;
        wait_for(|| view.presentation().blocks.iter().any(|block|
            block.message_id.as_ref() == "live" && block.text.len() >= expected)).await;
        if batch % 25 == 24 { eprintln!("stream publication: {expected} deltas in {:?}", started.elapsed()); }
    }
    fixture.send_identified("end", "session.step.ended", json!({"sessionID":"ses_stream","assistantMessageID":"live","finish":"stop"}));
    wait_for(|| view.presentation().phase == SessionPhase::Idle && view.presentation().blocks.iter()
        .any(|block| block.message_id.as_ref() == "live" && block.text.len() == 10_000)).await;
    let mut publications = 0;
    while let Ok(update) = updates.try_recv() {
        publications += usize::from(matches!(update, EngineUpdate::Session(ref id) if id == "ses_stream"));
    }
    assert!(publications > 1 && publications < 200, "10k tokens must publish at frame cadence, got {publications}");
    let path = fixture.journal_path("ses_stream");
    let records = vela_engine::stream_journal::read_records(&path).unwrap();
    let wire = records.iter().filter_map(|record| match record {
        vela_core::run_log::RunRecord::Event(event) => Some(event), _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(wire.len(), 10_002);
    assert!(wire.iter().all(|event| event.run_id == wire[0].run_id));
    let mut replay = vela_engine::stream_journal::replay_file(&path, "ses_stream").unwrap();
    let replayed = replay.blocks();
    let snapshot = view.presentation();
    assert_eq!(replayed.len(), snapshot.blocks.len());
    for (restored, published) in replayed.iter().zip(snapshot.blocks.iter()) {
        assert_eq!(restored.id, published.id);
        assert_eq!(restored.text, published.text);
        assert_eq!(restored.status, published.status);
        assert_eq!(restored.streaming, published.streaming);
    }
    assert_eq!(replay.phase(), snapshot.phase);
    fixture.engine.shutdown_and_wait().await;
}

#[tokio::test]
async fn all_eight_open_panels_keep_publishing_with_the_default_warm_limit() {
    let ids = (0..vela_core::session::MAX_SIMULTANEOUS_PANELS)
        .map(|index| format!("ses_panel_{index}")).collect::<Vec<_>>();
    let fixture = Fixture::stress(ids.clone(), 0).await;
    for id in &ids {
        fixture.finish_text(id, "All visible panels stay live");
    }
    wait_for(|| ids.iter().all(|id| fixture.engine.view(id).is_some_and(|view| {
        view.presentation().blocks.iter().any(|block| block.text.as_ref() == "All visible panels stay live")
    }))).await;
    assert_eq!(fixture.engine.open_views(), ids.len());
    fixture.engine.shutdown_and_wait().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lagged_subscriber_reads_final_atomic_snapshot_and_retirement_without_another_event() {
    let fixture = Fixture::start().await;
    let view = fixture.engine.view("ses_stream").unwrap();
    let original = view.presentation();
    let mut slow = fixture.engine.subscribe();
    fixture.overflow_notifications("ses_stream");
    let final_text = "Final **Markdown** response 🌍";
    // Send the settling events explicitly without a trailing execution event,
    // so the wait predicate below cannot be satisfied before the last source
    // mutation has been applied.
    fixture.send(
        "session.step.started",
        json!({"sessionID":"ses_stream","assistantMessageID":"live"}),
    );
    fixture.send(
        "session.text.delta",
        json!({"sessionID":"ses_stream","assistantMessageID":"live","ordinal":0,"delta":final_text}),
    );
    fixture.send(
        "session.step.ended",
        json!({"sessionID":"ses_stream","assistantMessageID":"live","finish":"stop"}),
    );
    wait_for(|| {
        let snapshot = view.presentation();
        snapshot.phase == SessionPhase::Idle
            && snapshot.blocks.iter().any(|block| {
                block.kind == BlockKind::Assistant && block.text.as_ref() == final_text
            })
    })
    .await;

    assert!(
        matches!(
            slow.try_recv(),
            Err(broadcast::error::TryRecvError::Lagged(_))
        ),
        "the fixture must actually overflow a real subscriber"
    );
    // No more source events are sent. Pull the canonical revision immediately,
    // as a UI must do on Lagged rather than waiting for one more notification.
    let final_snapshot = view.presentation();
    assert!(final_snapshot.revision > original.revision);
    assert_eq!(final_snapshot.revision, final_snapshot.markdown.revision);
    assert_eq!(final_snapshot.revision, view.transcript_revision());
    assert_eq!(final_snapshot.phase, SessionPhase::Idle);
    let final_block = final_snapshot
        .blocks
        .iter()
        .find(|block| block.kind == BlockKind::Assistant && block.text.as_ref() == final_text)
        .unwrap();
    assert!(!final_block.streaming);
    assert!(final_block.completed_at.is_some());
    assert!(final_snapshot.markdown.find(final_text, false).is_some());
    assert!(
        original
            .blocks
            .iter()
            .zip(final_snapshot.blocks.iter())
            .all(|(previous, current)| {
                previous.id == current.id && previous.text == current.text
            }),
        "the previous revision must remain an immutable prefix of the new source"
    );
    assert!(final_snapshot.blocks.len() > original.blocks.len());
    tokio::task::yield_now().await;
    assert!(
        Arc::ptr_eq(&final_snapshot, &view.presentation()),
        "the final revision is already published without another event"
    );

    let mut deletion = fixture.engine.subscribe();
    fixture.send("session.deleted", json!({"sessionID":"ses_stream"}));
    fixture.overflow_notifications("ses_noise");
    fixture.finish_text("ses_noise", "Noise stream finished");
    wait_for(|| {
        fixture.engine.session_is_retired("ses_stream")
            && fixture.engine.view("ses_noise").is_some_and(|view| {
                view.presentation()
                    .blocks
                    .iter()
                    .any(|block| block.text.as_ref() == "Noise stream finished")
            })
    })
    .await;
    assert!(matches!(
        deletion.try_recv(),
        Err(broadcast::error::TryRecvError::Lagged(_))
    ));
    while let Ok(update) = deletion.try_recv() {
        assert!(
            !matches!(update, EngineUpdate::SessionRemoved(ref id) if id == "ses_stream"),
            "deletion must have fallen out of the retained notification buffer"
        );
    }
    assert!(fixture.engine.session_is_retired("ses_stream"));
    assert!(fixture.engine.view("ses_stream").is_none());
    assert!(
        !fixture
            .engine
            .sessions()
            .iter()
            .any(|session| session.id == "ses_stream")
    );
    fixture.engine.shutdown_and_wait().await;
}
