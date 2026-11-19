use super::*;

fn dispatch(id: &str, child: Option<&str>) -> Block {
    Block {
        id: id.into(),
        message_id: "assistant".into(),
        part_id: id.into(),
        kind: BlockKind::Tool,
        text: r#"{"agent":"build","description":"Inspect parser"}"#.into(),
        detail: "task".into(),
        output: "".into(),
        progress: "".into(),
        interrupted: false,
        status: ToolStatus::Running,
        streaming: true,
        agent: None,
        model: None,
        created_at: 10,
        completed_at: None,
        child_session_id: child.map(Into::into),
        files: Vec::new(),
        sub_calls: Vec::new(),
        attachments: Vec::new(),
    }
}

fn notice(child: &str) -> Block {
    let mut notice = dispatch("notice", None);
    notice.kind = BlockKind::System;
    notice.text = format!(
        "<subagent id=\"{child}\" agent=\"build\" description=\"Inspect parser\" state=\"completed\"></subagent>"
    ).into();
    notice.streaming = false;
    notice
}

#[test]
fn different_unresolved_dispatch_ids_never_merge_by_description() {
    let mut blocks = vec![
        dispatch("spawn-first", None),
        dispatch("spawn-second", None),
    ];
    let first = collapse_subagent_tools(&blocks).into_owned();
    assert_eq!(first.len(), 2);
    blocks[1].status = ToolStatus::Success;
    blocks[1].streaming = false;
    blocks[1].child_session_id = Some("child-second".into());
    let second = collapse_subagent_tools(&blocks).into_owned();
    assert_eq!(
        second.iter().map(|block| &block.id).collect::<Vec<_>>(),
        first.iter().map(|block| &block.id).collect::<Vec<_>>()
    );
    assert_eq!(second[0].status, ToolStatus::Running);
    assert!(second[0].child_session_id.is_none());
    assert_eq!(second[1].status, ToolStatus::Success);
}

#[test]
fn ambiguous_synthetic_completion_does_not_settle_the_wrong_dispatch() {
    let blocks = [
        dispatch("spawn-first", None),
        dispatch("spawn-second", None),
        notice("child"),
    ];
    let projected = collapse_subagent_tools(&blocks);
    assert_eq!(
        projected.len(),
        3,
        "retain an uncorrelated notice instead of guessing"
    );
    assert_eq!(projected[0].status, ToolStatus::Running);
    assert_eq!(projected[1].status, ToolStatus::Running);
    assert_eq!(projected[2].id.as_ref(), "notice");
}

#[test]
fn explicit_child_completion_preserves_spawn_order_and_other_agent_state() {
    let mut blocks = vec![
        dispatch("spawn-first", Some("child-first")),
        dispatch("spawn-second", Some("child-second")),
        notice("child-second"),
    ];
    let projected = collapse_subagent_tools(&blocks);
    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].id.as_ref(), "spawn-first");
    assert_eq!(projected[0].status, ToolStatus::Running);
    assert_eq!(projected[1].id.as_ref(), "spawn-second");
    assert_eq!(projected[1].status, ToolStatus::Success);
    drop(projected);
    blocks.push(notice("child-first"));
    let projected = collapse_subagent_tools(&blocks);
    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].id.as_ref(), "spawn-first");
    assert_eq!(projected[1].id.as_ref(), "spawn-second");
    assert!(
        projected
            .iter()
            .all(|block| block.status == ToolStatus::Success && !block.streaming)
    );
}

#[test]
fn unique_synthetic_child_binds_once_and_does_not_override_a_different_resolved_child() {
    let blocks = [dispatch("spawn", None), notice("child")];
    let projected = collapse_subagent_tools(&blocks);
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].id.as_ref(), "spawn");
    assert_eq!(projected[0].child_session_id.as_deref(), Some("child"));
    assert_eq!(projected[0].status, ToolStatus::Success);
    let wrong = [dispatch("spawn", Some("another-child")), notice("child")];
    let projected = collapse_subagent_tools(&wrong);
    assert_eq!(projected.len(), 2);
    assert_eq!(
        projected[0].child_session_id.as_deref(),
        Some("another-child")
    );
    assert_eq!(projected[0].status, ToolStatus::Running);
}

#[test]
fn repeated_same_id_snapshots_preserve_first_row_and_start_time() {
    let first = dispatch("spawn", None);
    let mut latest = first.clone();
    latest.created_at = 50;
    latest.status = ToolStatus::Success;
    latest.streaming = false;
    latest.completed_at = Some(100);
    latest.child_session_id = Some("child".into());
    let blocks = [first, latest];
    let projected = collapse_subagent_tools(&blocks);
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].id.as_ref(), "spawn");
    assert_eq!(projected[0].created_at, 10);
    assert_eq!(projected[0].completed_at, Some(100));
}
