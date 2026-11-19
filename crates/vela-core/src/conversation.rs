//! Chronological conversation presentation shared by every transcript surface.
//!
//! A node keeps its source block identity throughout a run. In particular,
//! later tool calls do not turn already-visible prose into a different kind of
//! row or move it into a work group. Expansion and viewport state belong to the
//! consumer; this projection contains no display-dependent state.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use crate::transcript::{Block, BlockKind, ToolStatus};

/// An immutable projection of one source revision. The caller owns revision
/// and session/load-generation checks; block IDs are scoped to that session.
#[derive(Clone, Debug, Default)]
pub struct Conversation {
    pub turns: Vec<Turn>,
    /// Every source block appears once, in source order. Transport-specific
    /// synthetic-record normalization must happen before this projection.
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub block: Arc<Block>,
    pub source_index: usize,
    pub turn_index: usize,
}

impl Node {
    pub fn id(&self) -> &str {
        &self.block.id
    }
}

/// A user message followed by chronological agent/event nodes. A history that
/// starts before its user message has an orphan turn with `user == None`.
#[derive(Clone, Debug)]
pub struct Turn {
    pub id: Arc<str>,
    /// Range into `Conversation::nodes`, including the user node when present.
    pub range: Range<usize>,
    pub user: Option<Arc<Block>>,
    pub started_at: Option<i64>,
    pub latest_assistant_message: Option<Arc<str>>,
    pub messages: Vec<AssistantMessage>,
}

impl Turn {
    pub fn message(&self, id: &str) -> Option<&AssistantMessage> {
        self.messages
            .iter()
            .find(|message| message.id.as_ref() == id)
    }
}

/// Source-message metadata for actions and settled footers. It never decides
/// which message is the "final answer" and never changes the node sequence.
#[derive(Clone, Debug)]
pub struct AssistantMessage {
    pub id: Arc<str>,
    /// Index of the last node carrying this message ID in the source turn.
    pub last_node: usize,
    pub text_blocks: Vec<Arc<Block>>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub streaming: bool,
    pub interrupted: bool,
}

impl AssistantMessage {
    /// Materialize the source Markdown only when an action needs it. Keeping
    /// blocks shared avoids copying the full answer during every stream tick.
    pub fn text(&self) -> String {
        self.text_blocks
            .iter()
            .map(|block| block.text.as_ref())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn last_text_id(&self) -> Option<&str> {
        self.text_blocks.last().map(|block| block.id.as_ref())
    }

    pub fn duration_since(&self, started_at: Option<i64>) -> Option<i64> {
        self.completed_at.map(|completed| {
            completed
                .saturating_sub(started_at.unwrap_or(self.created_at))
                .max(0)
        })
    }
}

fn assistant_message_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Assistant
            | BlockKind::Reasoning
            | BlockKind::Tool
            | BlockKind::Error
            | BlockKind::Retry
            | BlockKind::Activity
    )
}

/// Project existing blocks without inferring presentation roles from future
/// events. Corrections update payloads under the same IDs; prepended history
/// may change turn membership but never replaces a node's source identity.
pub fn project(blocks: &[Block]) -> Conversation {
    let mut conversation = Conversation {
        turns: Vec::new(),
        nodes: Vec::with_capacity(blocks.len()),
    };
    let mut message_indices: HashMap<Arc<str>, usize> = HashMap::new();
    for (source_index, source) in blocks.iter().enumerate() {
        let block = Arc::new(source.clone());
        let user = block.kind == BlockKind::User;
        if user || conversation.turns.is_empty() {
            message_indices.clear();
            conversation.turns.push(Turn {
                id: block.message_id.clone(),
                range: conversation.nodes.len()..conversation.nodes.len(),
                user: user.then(|| block.clone()),
                started_at: Some(block.created_at),
                latest_assistant_message: None,
                messages: Vec::new(),
            });
        }
        let turn_index = conversation.turns.len() - 1;
        let turn = &mut conversation.turns[turn_index];
        if assistant_message_kind(block.kind) {
            turn.latest_assistant_message = Some(block.message_id.clone());
            let index = *message_indices
                .entry(block.message_id.clone())
                .or_insert_with(|| {
                    let index = turn.messages.len();
                    turn.messages.push(AssistantMessage {
                        id: block.message_id.clone(),
                        last_node: source_index,
                        text_blocks: Vec::new(),
                        created_at: block.created_at,
                        completed_at: None,
                        streaming: false,
                        interrupted: false,
                    });
                    index
                });
            let message = &mut turn.messages[index];
            message.last_node = source_index;
            message.created_at = message.created_at.min(block.created_at);
            // Tool completion timestamps describe the call, while Retry's
            // timestamp is its next attempt. Neither proves that the source
            // assistant message has finished and may reveal its footer.
            if matches!(
                block.kind,
                BlockKind::Assistant
                    | BlockKind::Reasoning
                    | BlockKind::Error
                    | BlockKind::Activity
            ) {
                message.completed_at = message.completed_at.max(block.completed_at);
            }
            // A tool call's own running/finished state describes the call, not
            // the assistant message. Letting it keep the message streaming
            // suppresses the settled footer forever when a call is still
            // running in a completed replay or an interrupted turn.
            if !matches!(block.kind, BlockKind::Tool | BlockKind::Shell) {
                message.streaming |= block.streaming
                    || matches!(block.status, ToolStatus::Streaming | ToolStatus::Running);
            }
            message.interrupted |= block.interrupted;
            if block.kind == BlockKind::Assistant && !block.text.trim().is_empty() {
                message.text_blocks.push(block.clone());
            }
        } else if let Some(&index) = message_indices.get(&block.message_id) {
            turn.messages[index].last_node = source_index;
        }
        conversation.nodes.push(Node {
            block,
            source_index,
            turn_index,
        });
        turn.range.end = conversation.nodes.len();
    }
    conversation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentEvent;
    use crate::transcript::{Message, Role, Transcript};

    fn block(id: &str, message: &str, kind: BlockKind, text: &str) -> Block {
        Block {
            id: id.into(),
            message_id: message.into(),
            part_id: id.into(),
            kind,
            text: text.into(),
            detail: "".into(),
            output: "".into(),
            progress: "".into(),
            interrupted: false,
            status: ToolStatus::None,
            streaming: false,
            agent: None,
            model: None,
            created_at: 1,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        }
    }

    fn ids(conversation: &Conversation) -> Vec<&str> {
        conversation.nodes.iter().map(Node::id).collect()
    }

    #[test]
    fn later_tools_and_text_do_not_reclassify_existing_prose() {
        let user = block("user", "u", BlockKind::User, "Inspect this");
        let prose = block("p#0", "a", BlockKind::Assistant, "I will inspect it.");
        let tool = block("tool", "a", BlockKind::Tool, r#"{"path":"src"}"#);
        let answer = block("q#0", "b", BlockKind::Assistant, "Here is the result.");
        let before = project(&[user.clone(), prose.clone()]);
        let after_tool = project(&[user.clone(), prose.clone(), tool.clone()]);
        let after = project(&[user, prose, tool, answer]);

        assert_eq!(ids(&before), vec!["user", "p#0"]);
        assert_eq!(&ids(&after_tool)[..2], ids(&before));
        assert_eq!(&ids(&after)[..3], ids(&after_tool));
        assert_eq!(after.nodes[1].block.kind, BlockKind::Assistant);
        assert_eq!(after.nodes[1].turn_index, before.nodes[1].turn_index);
        assert_eq!(
            after.turns[0].message("a").unwrap().text(),
            "I will inspect it."
        );
        assert_eq!(
            after.turns[0].message("a").unwrap().last_text_id(),
            Some("p#0")
        );
        assert_eq!(
            after.turns[0].latest_assistant_message.as_deref(),
            Some("b")
        );
    }

    #[test]
    fn each_typed_event_and_parallel_call_preserves_source_order() {
        let kinds = [
            BlockKind::Reasoning,
            BlockKind::Tool,
            BlockKind::Assistant,
            BlockKind::Tool,
            BlockKind::Shell,
            BlockKind::System,
            BlockKind::Compaction,
            BlockKind::Skill,
            BlockKind::Error,
            BlockKind::Retry,
            BlockKind::Activity,
        ];
        let mut source = kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| block(&format!("node-{index}"), "a", *kind, "payload"))
            .collect::<Vec<_>>();
        source[1].child_session_id = Some("child-1".into());
        source[3].child_session_id = Some("child-2".into());
        let conversation = project(&source);
        assert_eq!(conversation.turns.len(), 1);
        assert!(conversation.turns[0].user.is_none());
        assert_eq!(conversation.turns[0].range, 0..source.len());
        for (index, node) in conversation.nodes.iter().enumerate() {
            assert_eq!(node.source_index, index);
            assert_eq!(node.id(), source[index].id.as_ref());
            assert_eq!(node.block.kind, kinds[index]);
        }
        assert_eq!(
            conversation.nodes[1].block.child_session_id.as_deref(),
            Some("child-1")
        );
        assert_eq!(
            conversation.nodes[3].block.child_session_id.as_deref(),
            Some("child-2")
        );
    }

    #[test]
    fn empty_and_consecutive_user_turns_keep_independent_ranges() {
        assert!(project(&[]).turns.is_empty());
        let conversation = project(&[
            block("orphan", "a0", BlockKind::Assistant, "Earlier response"),
            block("u1#0", "u1", BlockKind::User, "First"),
            block("u2#0", "u2", BlockKind::User, "Second"),
            block("a2#0", "a2", BlockKind::Assistant, "Reply"),
        ]);
        assert_eq!(
            conversation
                .turns
                .iter()
                .map(|turn| turn.range.clone())
                .collect::<Vec<_>>(),
            vec![0..1, 1..2, 2..4]
        );
        assert!(conversation.turns[0].user.is_none());
        assert_eq!(
            conversation.turns[1].user.as_ref().unwrap().id.as_ref(),
            "u1#0"
        );
        assert_eq!(
            conversation.turns[2].user.as_ref().unwrap().id.as_ref(),
            "u2#0"
        );
        assert_eq!(
            conversation
                .nodes
                .iter()
                .map(|node| node.turn_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 2]
        );
    }

    #[test]
    fn correction_and_terminal_status_update_payload_without_changing_identity() {
        let mut first = block("p#0", "a", BlockKind::Assistant, "Initial **text");
        first.streaming = true;
        let before = project(&[first.clone()]);
        first.text = "Corrected **text**".into();
        first.streaming = false;
        first.interrupted = true;
        first.created_at = 200;
        first.completed_at = Some(350);
        let after = project(&[first.clone()]);
        assert_eq!(ids(&before), ids(&after));
        let message = after.turns[0].message("a").unwrap();
        assert!(!message.streaming);
        assert!(message.interrupted);
        assert_eq!(message.text(), "Corrected **text**");
        assert_eq!(message.duration_since(Some(100)), Some(250));
        assert_eq!(message.duration_since(None), Some(150));
        assert!(Arc::ptr_eq(&message.text_blocks[0].text, &first.text));
        assert!(Arc::ptr_eq(&after.nodes[0].block, &message.text_blocks[0]));
        assert_eq!(before.nodes[0].block.text.as_ref(), "Initial **text");
    }

    #[test]
    fn finished_tool_and_scheduled_retry_do_not_complete_an_assistant_message() {
        let prose = block("p#0", "a", BlockKind::Assistant, "I will inspect this.");
        let mut tool = block("tool", "a", BlockKind::Tool, "{}");
        tool.status = ToolStatus::Success;
        tool.completed_at = Some(200);
        let mut retry = block("retry", "a", BlockKind::Retry, "Trying again");
        retry.completed_at = Some(900);
        let conversation = project(&[prose, tool, retry]);
        let message = conversation.turns[0].message("a").unwrap();
        assert_eq!(message.completed_at, None);
        assert_eq!(message.duration_since(Some(1)), None);
        assert_eq!(message.text(), "I will inspect this.");
    }

    #[test]
    fn running_tool_does_not_keep_a_completed_message_streaming() {
        let mut prose = block("p#0", "a", BlockKind::Assistant, "Done.");
        prose.completed_at = Some(100);
        let mut tool = block("tool", "a", BlockKind::Tool, "{}");
        tool.status = ToolStatus::Running;
        tool.streaming = true;
        let conversation = project(&[prose, tool]);
        let message = conversation.turns[0].message("a").unwrap();
        assert!(!message.streaming, "a tool's running state is not message progress");
        assert_eq!(message.completed_at, Some(100));
    }

    #[test]
    fn live_interleaved_execution_and_hydrated_replay_have_identical_nodes() {
        let mut live = Transcript::new();
        live.hydrate(vec![Message::text("u", Role::User, 1, "Inspect")]);
        live.apply(&AgentEvent::StepStarted {
            session_id: "s".into(),
            message_id: "a".into(),
            agent: None,
            model: None,
            at: 2,
        });
        live.apply(&AgentEvent::TextDelta {
            session_id: "s".into(),
            message_id: "a".into(),
            part_id: "p".into(),
            ordinal: 0,
            delta: "Inspecting.".into(),
            at: 3,
        });
        let before_tool = project(live.blocks());
        live.apply(&AgentEvent::ToolInputStarted {
            session_id: "s".into(),
            message_id: "a".into(),
            call_id: "t".into(),
            name: "read".into(),
            at: 4,
        });
        live.apply(&AgentEvent::ToolCalled {
            session_id: "s".into(),
            message_id: "a".into(),
            call_id: "t".into(),
            input: "{}".into(),
            at: 5,
        });
        live.apply(&AgentEvent::ToolFinished {
            session_id: "s".into(),
            message_id: "a".into(),
            call_id: "t".into(),
            success: true,
            output: "contents".into(),
            files: Vec::new(),
            error: None,
            at: 6,
        });
        live.apply(&AgentEvent::TextDelta {
            session_id: "s".into(),
            message_id: "a".into(),
            part_id: "q".into(),
            ordinal: 2,
            delta: "Found it.".into(),
            at: 7,
        });
        live.apply(&AgentEvent::StepEnded {
            session_id: "s".into(),
            message_id: "a".into(),
            finish: Some("stop".into()),
            at: 8,
        });
        let after = project(live.blocks());
        assert_eq!(&ids(&after)[..before_tool.nodes.len()], ids(&before_tool));
        let mut replay = Transcript::new();
        replay.hydrate(live.messages().to_vec());
        let replay = project(replay.blocks());
        assert_eq!(ids(&after), ids(&replay));
        assert_eq!(
            after.turns[0].message("a").unwrap().text(),
            replay.turns[0].message("a").unwrap().text()
        );
        assert_eq!(
            after
                .nodes
                .iter()
                .map(|node| node.block.fingerprint())
                .collect::<Vec<_>>(),
            replay
                .nodes
                .iter()
                .map(|node| node.block.fingerprint())
                .collect::<Vec<_>>()
        );
    }
}
