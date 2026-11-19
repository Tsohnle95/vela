//! Bounded correlation between local send intent and canonical user messages.
//!
//! Optimistic `local:` messages are also inserted for queued prompts, so their
//! appearance is not permission to move a viewport. Only a newly observed
//! canonical user message with the submitted text can consume an intent.
//!
//! OpenCode's current prompt API supplies no client request ID. Text, existing
//! identity and timestamp checks therefore cannot prove causality when another
//! client submits identical text. The timestamp check assumes comparable server
//! and request clocks: skew can miss an arrival, or make old identical history
//! with a previously unseen ID eligible. This is best-effort correlation.
//! This affects automatic scrolling only, never prompt delivery or transcript
//! contents. All state is transient and must be cancelled on manual reading or
//! panel closure by the caller.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use vela_core::transcript::{Block, BlockKind};

const MAX_SESSIONS: usize = 8;
const MAX_PENDING_PER_SESSION: usize = 32;
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_KNOWN_IDS: usize = 4096;
const MAX_BASELINE_BYTES: usize = 256 * 1024;
const MAX_ID_BYTES: usize = 256;

#[derive(Default)]
pub(crate) struct PendingTurnAnchors {
    sessions: HashMap<String, SessionIntents>,
}

#[derive(Default)]
struct SessionIntents {
    pending: VecDeque<Intent>,
    consumed: HashSet<Arc<str>>,
    last_baseline: Arc<HashSet<Arc<str>>>,
    /// Exhausting the identity tombstone bound disables this session until
    /// cancellation rather than forgetting IDs that could then be reused.
    exhausted: bool,
}

struct Intent {
    text: Arc<str>,
    requested_at: i64,
    existing: Arc<HashSet<Arc<str>>>,
}

fn canonical_user(block: &Block) -> bool {
    block.kind == BlockKind::User
        && !block.message_id.is_empty()
        && block.message_id.len() <= MAX_ID_BYTES
        && !block.message_id.starts_with("local:")
}

impl PendingTurnAnchors {
    /// Record the exact text passed to the engine immediately before sending.
    /// `false` means tracking was refused at a bound; the caller must still send
    /// the prompt normally. Existing pending intent is never evicted to make
    /// room, which would confuse the order of queued identical prompts.
    pub(crate) fn request(
        &mut self,
        session: &str,
        blocks: &[Block],
        text: &str,
        requested_at: i64,
    ) -> bool {
        if session.is_empty()
            || session.len() > MAX_ID_BYTES
            || text.is_empty()
            || text.len() > MAX_TEXT_BYTES
            || (!self.sessions.contains_key(session) && self.sessions.len() >= MAX_SESSIONS)
        {
            return false;
        }
        if self
            .sessions
            .get(session)
            .is_some_and(|state| state.exhausted || state.pending.len() >= MAX_PENDING_PER_SESSION)
        {
            return false;
        }
        let mut existing = HashSet::new();
        let mut baseline_bytes = 0usize;
        for block in blocks.iter().filter(|block| canonical_user(block)) {
            if existing.insert(block.message_id.clone()) {
                baseline_bytes += block.message_id.len();
                if existing.len() > MAX_KNOWN_IDS || baseline_bytes > MAX_BASELINE_BYTES {
                    return false;
                }
            }
        }
        let state = self.sessions.entry(session.to_owned()).or_default();
        // Queued sends commonly see the same canonical history. Share that
        // immutable set instead of retaining a copy per queued marker.
        if state.last_baseline.as_ref() != &existing {
            state.last_baseline = Arc::new(existing);
        }
        state.pending.push_back(Intent {
            text: Arc::from(text),
            requested_at,
            existing: state.last_baseline.clone(),
        });
        true
    }

    /// Consume at most one newly canonical user message in transcript order.
    /// Matching equal texts uses FIFO intent order. An unmatched steer that
    /// produces no user message does not block a later, differently worded send.
    pub(crate) fn take_match(&mut self, session: &str, blocks: &[Block]) -> Option<Arc<str>> {
        let state = self.sessions.get_mut(session)?;
        if state.exhausted || state.pending.is_empty() {
            return None;
        }
        for block in blocks.iter().filter(|block| canonical_user(block)) {
            if state.consumed.contains(&block.message_id) {
                continue;
            }
            let Some(index) = state.pending.iter().position(|intent| {
                block.created_at >= intent.requested_at
                    && block.text.as_ref() == intent.text.as_ref()
                    && !intent.existing.contains(&block.message_id)
            }) else {
                continue;
            };
            if state.consumed.len() >= MAX_KNOWN_IDS {
                state.exhausted = true;
                state.pending.clear();
                return None;
            }
            state.pending.remove(index);
            state.consumed.insert(block.message_id.clone());
            return Some(block.message_id.clone());
        }
        None
    }

    /// Cancel automatic movement after manual intent or remove closed sessions.
    pub(crate) fn cancel(&mut self, session: &str) {
        self.sessions.remove(session);
    }

    /// The queue ID is not present on the eventual user message. When removing
    /// an item, disarm preexisting intents for its text so an identical arrival
    /// cannot move the viewport on behalf of the removed item.
    pub(crate) fn discard_queued(&mut self, session: &str, text: &str) {
        if let Some(state) = self.sessions.get_mut(session) {
            state.pending.retain(|intent| intent.text.as_ref() != text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vela_core::transcript::ToolStatus;

    fn user(id: &str, text: &str, created_at: i64) -> Block {
        Block {
            id: format!("{id}:text#0").into(),
            message_id: id.into(),
            part_id: format!("{id}:text").into(),
            kind: BlockKind::User,
            text: text.into(),
            detail: "".into(),
            output: "".into(),
            progress: "".into(),
            interrupted: false,
            status: ToolStatus::None,
            streaming: false,
            agent: None,
            model: None,
            created_at,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        }
    }

    #[test]
    fn waits_for_canonical_user_instead_of_queued_optimistic_rows() {
        let history = vec![user("earlier", "same prompt", 100)];
        let mut pending = PendingTurnAnchors::default();
        assert!(pending.request("session", &history, "same prompt", 200));
        let mut snapshot = history;
        snapshot.push(user("local:optimistic", "same prompt", 200));
        assert!(pending.take_match("session", &snapshot).is_none());
        snapshot.push(user("canonical", "same prompt", 200));
        assert_eq!(
            pending.take_match("session", &snapshot).as_deref(),
            Some("canonical")
        );
        assert!(pending.take_match("session", &snapshot).is_none());
    }

    #[test]
    fn known_ids_and_older_rehydrated_history_do_not_match_corrections() {
        let history = vec![user("known", "old text", 100)];
        let mut pending = PendingTurnAnchors::default();
        assert!(pending.request("session", &history, "submitted", 200));
        let snapshot = vec![
            user("known", "submitted", 300),
            user("replayed", "submitted", 199),
        ];
        assert!(pending.take_match("session", &snapshot).is_none());
        assert_eq!(pending.sessions["session"].pending.len(), 1);
    }

    #[test]
    fn identical_queued_sends_consume_distinct_canonical_ids_once() {
        let mut pending = PendingTurnAnchors::default();
        assert!(pending.request("session", &[], "repeat", 200));
        assert!(pending.request("session", &[], "repeat", 200));
        let first = user("canonical-1", "repeat", 200);
        assert_eq!(
            pending.take_match("session", std::slice::from_ref(&first)).as_deref(),
            Some("canonical-1")
        );
        assert!(pending.take_match("session", std::slice::from_ref(&first)).is_none());
        let second = user("canonical-2", "repeat", 200);
        assert_eq!(
            pending.take_match("session", &[first, second]).as_deref(),
            Some("canonical-2")
        );
        assert!(pending.sessions["session"].pending.is_empty());
    }

    #[test]
    fn no_user_event_means_no_anchor_and_a_later_distinct_send_can_match() {
        let mut pending = PendingTurnAnchors::default();
        assert!(pending.request("session", &[], "steer only", 200));
        assert!(pending.request("session", &[], "ordinary prompt", 201));
        let mut assistant = user("assistant", "steer only", 202);
        assistant.kind = BlockKind::Assistant;
        assert!(pending.take_match("session", &[assistant]).is_none());
        assert_eq!(
            pending
                .take_match("session", &[user("ordinary", "ordinary prompt", 202)])
                .as_deref(),
            Some("ordinary")
        );
        assert_eq!(
            pending.sessions["session"]
                .pending
                .front()
                .unwrap()
                .text
                .as_ref(),
            "steer only"
        );
    }

    #[test]
    fn exact_text_session_scope_and_cancellation_control_matching() {
        let mut pending = PendingTurnAnchors::default();
        assert!(pending.request("first", &[], "  exact\ntext  ", 200));
        assert!(pending.request("second", &[], "other", 200));
        assert!(
            pending
                .take_match("first", &[user("changed", "exact\ntext", 201)])
                .is_none()
        );
        assert!(
            pending
                .take_match("second", &[user("wrong-session", "  exact\ntext  ", 201)])
                .is_none()
        );
        pending.cancel("first");
        assert!(
            pending
                .take_match("first", &[user("after-close", "  exact\ntext  ", 201)])
                .is_none()
        );
        assert_eq!(
            pending
                .take_match("second", &[user("other", "other", 201)])
                .as_deref(),
            Some("other")
        );
    }

    #[test]
    fn discarded_queue_text_disarms_ambiguous_matches_without_touching_other_intents() {
        let mut pending = PendingTurnAnchors::default();
        for (session, text) in [("first", "same"), ("first", "same"), ("first", "different"), ("second", "same")] {
            assert!(pending.request(session, &[], text, 200));
        }
        pending.discard_queued("first", "same");
        assert!(pending.take_match("first", &[user("late", "same", 201)]).is_none());
        assert_eq!(pending.take_match("first", &[user("other", "different", 201)]).as_deref(), Some("other"));
        assert_eq!(pending.take_match("second", &[user("elsewhere", "same", 201)]).as_deref(), Some("elsewhere"));
    }

    #[test]
    fn bounds_refuse_tracking_without_evicting_existing_fifo_intents() {
        let mut pending = PendingTurnAnchors::default();
        for _ in 0..MAX_PENDING_PER_SESSION {
            assert!(pending.request("first", &[], "repeat", 200));
        }
        assert!(!pending.request("first", &[], "overflow", 200));
        assert_eq!(
            pending.sessions["first"].pending.len(),
            MAX_PENDING_PER_SESSION
        );
        for index in 1..MAX_SESSIONS {
            assert!(pending.request(&format!("session-{index}"), &[], "text", 200));
        }
        assert!(!pending.request("ninth", &[], "text", 200));
        assert!(!pending.request("first", &[], &"x".repeat(MAX_TEXT_BYTES + 1), 200));
        assert_eq!(pending.sessions.len(), MAX_SESSIONS);
    }

    #[test]
    fn captures_all_existing_user_ids_and_shares_identical_baselines() {
        let history = vec![user("known", "old", 100), user("local:queued", "new", 200)];
        let mut pending = PendingTurnAnchors::default();
        assert!(pending.request("session", &history, "new", 200));
        assert!(pending.request("session", &history, "new", 201));
        let state = &pending.sessions["session"];
        assert!(state.pending[0].existing.contains("known"));
        assert!(!state.pending[0].existing.contains("local:queued"));
        assert!(Arc::ptr_eq(
            &state.pending[0].existing,
            &state.pending[1].existing
        ));
    }

    #[test]
    fn large_baselines_and_exhausted_tombstones_fail_without_forgetting_matches() {
        let mut pending = PendingTurnAnchors::default();
        let history = (0..=MAX_KNOWN_IDS)
            .map(|index| user(&format!("existing-{index}"), "old", 100))
            .collect::<Vec<_>>();
        assert!(!pending.request("session", &history, "new", 200));
        assert!(pending.sessions.is_empty());
        assert!(!pending.request("session", &[], &"x".repeat(MAX_TEXT_BYTES + 1), 200));
        assert!(pending.request("session", &[], "new", 200));
        let state = pending.sessions.get_mut("session").unwrap();
        state
            .consumed
            .extend((0..MAX_KNOWN_IDS).map(|index| Arc::from(format!("used-{index}"))));
        assert!(
            pending
                .take_match("session", &[user("new", "new", 201)])
                .is_none()
        );
        let state = &pending.sessions["session"];
        assert!(state.exhausted);
        assert!(state.pending.is_empty());
        assert_eq!(state.consumed.len(), MAX_KNOWN_IDS);
        assert!(!pending.request("session", &[], "later", 300));
        pending.cancel("session");
        assert!(pending.request("session", &[], "later", 300));
    }
}
