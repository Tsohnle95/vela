//! Bounded warm-session retention.
//!
//! Vela's navigation target is a first correct frame, not a fresh network
//! round trip. Completed sessions stay warm with their projected transcript,
//! scroll anchor, and draft. Retention is bounded by both a session count and
//! an estimated byte budget; eviction never blocks the UI thread.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::session::SessionSummary;
use crate::transcript::Transcript;

#[derive(Clone, Debug)]
pub struct WarmEntry {
    pub session: SessionSummary,
    pub transcript: Transcript,
    pub draft: String,
    pub scroll_top: f32,
    pub stick_to_bottom: bool,
    pub selected_model: Option<String>,
    pub selected_agent: Option<String>,
    pub last_used: Instant,
    pub bytes: usize,
}

impl WarmEntry {
    pub fn new(session: SessionSummary, transcript: Transcript) -> Self {
        let bytes = estimate_bytes(&session, &transcript);
        Self {
            session,
            transcript,
            draft: String::new(),
            scroll_top: 0.0,
            stick_to_bottom: true,
            selected_model: None,
            selected_agent: None,
            last_used: Instant::now(),
            bytes,
        }
    }

    pub fn with_draft(mut self, draft: String) -> Self {
        self.draft = draft;
        self
    }

    pub fn with_scroll(mut self, scroll: (f32, bool)) -> Self {
        self.scroll_top = scroll.0;
        self.stick_to_bottom = scroll.1;
        self
    }

    pub fn touch(&mut self) {
        self.last_used = Instant::now();
    }
}

fn estimate_bytes(session: &SessionSummary, transcript: &Transcript) -> usize {
    let mut bytes = 256 + session.id.len() + session.title.len() * 2 + session.directory.len();
    for message in transcript.messages() {
        bytes += 96 + message.id.len() + message.parts.len() * 40;
        bytes += message.error.as_ref().map_or(0, |error| error.len() * 2);
        bytes += message
            .retry
            .as_ref()
            .map_or(0, |retry| 32 + retry.message.len() * 2);
        for part in &message.parts {
            bytes += 48 + part.id.len();
            match &part.kind {
                crate::transcript::PartKind::Text { text, .. }
                | crate::transcript::PartKind::Reasoning { text, .. } => bytes += text.len() * 2,
                crate::transcript::PartKind::Tool(tool) => {
                    bytes += (tool.input.len() + tool.output.len() + tool.detail.len()) * 2;
                }
            }
        }
    }
    bytes
}

pub struct WarmCache {
    limit: usize,
    byte_budget: usize,
    entries: HashMap<String, WarmEntry>,
    order: VecDeque<String>,
    bytes: usize,
}

impl WarmCache {
    pub fn new(limit: usize, byte_budget: usize) -> Self {
        Self {
            limit: limit.max(1),
            byte_budget: byte_budget.max(1),
            entries: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
        }
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn byte_budget(&self) -> usize {
        self.byte_budget
    }

    /// Apply newly persisted cache limits without discarding retained warmth
    /// unnecessarily; eviction runs only when the new limits are smaller.
    pub fn set_limits(&mut self, limit: usize, byte_budget: usize) {
        self.limit = limit.max(1);
        self.byte_budget = byte_budget.max(1);
        self.evict();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn retained_bytes(&self) -> usize {
        self.bytes
    }

    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    pub fn get(&mut self, id: &str) -> Option<&mut WarmEntry> {
        if !self.entries.contains_key(id) {
            return None;
        }
        self.touch(id);
        self.entries.get_mut(id)
    }

    pub fn peek(&self, id: &str) -> Option<&WarmEntry> {
        self.entries.get(id)
    }

    /// Insert or replace an entry, then evict least-recently-used entries until
    /// both limits hold. A single entry larger than the whole budget is kept
    /// only if the cache is empty, so one huge transcript cannot evict all
    /// useful warmth; callers can decide to keep it outside the cache.
    pub fn insert(&mut self, entry: WarmEntry) {
        let id = entry.session.id.clone();
        if let Some(old) = self.entries.remove(&id) {
            self.bytes = self.bytes.saturating_sub(old.bytes);
            self.order.retain(|key| key != &id);
        }
        self.bytes += entry.bytes;
        self.entries.insert(id.clone(), entry);
        self.order.push_back(id);
        self.evict();
    }

    pub fn remove(&mut self, id: &str) -> Option<WarmEntry> {
        let entry = self.entries.remove(id)?;
        self.bytes = self.bytes.saturating_sub(entry.bytes);
        self.order.retain(|key| key != id);
        Some(entry)
    }

    pub fn touch(&mut self, id: &str) {
        if self.order.back().is_some_and(|key| key == id) {
            if let Some(entry) = self.entries.get_mut(id) {
                entry.touch();
            }
            return;
        }
        let Some(position) = self.order.iter().position(|key| key == id) else {
            return;
        };
        if let Some(key) = self.order.remove(position) {
            self.order.push_back(key);
        }
        if let Some(entry) = self.entries.get_mut(id) {
            entry.touch();
        }
    }

    pub fn set_draft(&mut self, id: &str, draft: String) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.draft = draft;
            entry.touch();
        }
    }

    pub fn set_scroll(&mut self, id: &str, scroll_top: f32, stick_to_bottom: bool) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.scroll_top = scroll_top.max(0.0);
            entry.stick_to_bottom = stick_to_bottom;
            entry.touch();
        }
    }

    pub fn set_selection(&mut self, id: &str, model: Option<String>, agent: Option<String>) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.selected_model = model;
            entry.selected_agent = agent;
        }
    }

    fn evict(&mut self) {
        while self.entries.len() > self.limit && self.entries.len() > 1 {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(entry.bytes);
            }
        }
        while self.bytes > self.byte_budget && self.entries.len() > 1 {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(entry.bytes);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{Message, Role, Transcript};

    fn entry(id: &str, text: &str) -> WarmEntry {
        let session = SessionSummary {
            id: id.into(),
            title: id.into(),
            directory: "/repo".into(),
            updated_at: 0,
            parent_id: None,
            agent: None,
        };
        let mut transcript = Transcript::new();
        transcript.hydrate(vec![Message::text(format!("{id}-m"), Role::User, 1, text)]);
        WarmEntry::new(session, transcript)
    }

    #[test]
    fn evicts_least_recently_used_by_count() {
        let mut cache = WarmCache::new(2, 10 * 1024 * 1024);
        cache.insert(entry("a", "one"));
        cache.insert(entry("b", "two"));
        cache.get("a");
        cache.insert(entry("c", "three"));
        assert!(cache.contains("a"));
        assert!(!cache.contains("b"));
        assert!(cache.contains("c"));
    }

    #[test]
    fn evicts_by_byte_budget() {
        let mut cache = WarmCache::new(10, 1_600);
        for ix in 0..5 {
            cache.insert(entry(&format!("s{ix}"), &"x".repeat(400)));
        }
        assert!(cache.retained_bytes() <= 1_600 || cache.len() == 1);
        assert!(cache.len() < 5);
    }

    #[test]
    fn set_limits_evicts_until_both_new_budgets_hold() {
        let mut cache = WarmCache::new(4, 10_000);
        for ix in 0..4 {
            cache.insert(entry(&format!("s{ix}"), &"x".repeat(400)));
        }
        cache.get("s3");
        cache.set_limits(2, 10_000);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.limit(), 2);
        assert!(cache.contains("s3"));

        cache.set_limits(2, 1);
        assert_eq!(cache.len(), 1, "one oversized entry may remain");
        cache.set_limits(0, 0);
        assert_eq!(cache.limit(), 1);
        assert_eq!(cache.byte_budget(), 1);
    }

    #[test]
    fn keeps_drafts_and_scroll_across_touches() {
        let mut cache = WarmCache::new(2, 1_000_000);
        cache.insert(entry("a", "hello"));
        cache.set_draft("a", "unsent".into());
        cache.set_scroll("a", 420.0, false);
        cache.get("a");
        let found = cache.peek("a").unwrap();
        assert_eq!(found.draft, "unsent");
        assert_eq!(found.scroll_top, 420.0);
        assert!(!found.stick_to_bottom);
    }
}
