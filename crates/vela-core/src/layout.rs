//! Row-layout arithmetic for virtualized transcripts.
//!
//! The list needs total height, offset lookup, and hit-testing without walking
//! every historical row. Heights are measured lazily and cached by stable block
//! id; only changed blocks lose their measurement.

use std::collections::HashMap;

use crate::transcript::Block;

const DEFAULT_ROW_HEIGHT: f32 = 24.0;

#[derive(Default)]
pub struct LayoutCache {
    order: Vec<String>,
    prints: HashMap<String, u64>,
    heights: HashMap<String, f32>,
    prefix: Vec<f32>,
    total: f32,
}

impl LayoutCache {
    /// Synchronize with the current block list. Returns true when the row order
    /// changed. Changed content invalidates that row's measured height, which
    /// the view repopulates during layout.
    pub fn sync(&mut self, blocks: &[Block]) -> bool {
        let mut changed = self.order.len() != blocks.len();
        if !changed {
            changed = self
                .order
                .iter()
                .zip(blocks)
                .any(|(id, block)| id != block.id.as_ref());
        }

        if changed {
            self.order = blocks.iter().map(|block| block.id.to_string()).collect();
        }

        for block in blocks {
            let print = block.fingerprint();
            if self.prints.get(block.id.as_ref()).copied() != Some(print) {
                self.prints.insert(block.id.to_string(), print);
                self.heights.remove(block.id.as_ref());
                changed = true;
            }
        }

        if changed {
            self.rebuild_prefix();
        }
        changed
    }

    pub fn set_height(&mut self, id: &str, height: f32) -> bool {
        let next = height.max(1.0);
        if self.heights.get(id).copied() == Some(next) {
            return false;
        }
        self.heights.insert(id.to_string(), next);
        self.rebuild_prefix();
        true
    }

    pub fn height(&self, id: &str) -> Option<f32> {
        self.heights.get(id).copied()
    }

    pub fn total_height(&self) -> f32 {
        self.total
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn row_offset(&self, index: usize) -> f32 {
        self.prefix.get(index).copied().unwrap_or(self.total)
    }

    pub fn row_height(&self, index: usize) -> f32 {
        self.heights
            .get(&self.order[index])
            .copied()
            .unwrap_or(DEFAULT_ROW_HEIGHT)
    }

    /// First row intersecting `offset` in document coordinates.
    pub fn index_at(&self, offset: f32) -> usize {
        if self.order.is_empty() {
            return 0;
        }
        let offset = offset.max(0.0);
        // prefix[i] is the top of row i; prefix[len] is the total.
        let mut lo = 0usize;
        let mut hi = self.order.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.prefix[mid + 1] <= offset {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo.min(self.order.len() - 1)
    }

    /// Rows intersecting a viewport, plus bounded overdraw.
    pub fn visible_range(&self, top: f32, height: f32, overdraw: f32) -> std::ops::Range<usize> {
        if self.order.is_empty() {
            return 0..0;
        }
        let start = self.index_at((top - overdraw).max(0.0));
        let end = self.index_at(top + height + overdraw) + 1;
        start..end.min(self.order.len())
    }

    fn rebuild_prefix(&mut self) {
        self.prefix.clear();
        self.prefix.reserve(self.order.len() + 1);
        let mut total = 0.0;
        self.prefix.push(0.0);
        for id in &self.order {
            total += self
                .heights
                .get(id)
                .copied()
                .unwrap_or(DEFAULT_ROW_HEIGHT)
                .max(1.0);
            self.prefix.push(total);
        }
        self.total = total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{Block, BlockKind, ToolStatus};
    use std::sync::Arc;

    fn block(id: &str, text: &str) -> Block {
        Block {
            id: Arc::from(id),
            message_id: Arc::from("m"),
            part_id: Arc::from("p"),
            kind: BlockKind::Assistant,
            text: Arc::from(text),
            detail: Arc::from(""),
            output: Arc::from(""),
            status: ToolStatus::None,
            streaming: false,
            agent: None,
            model: None,
            created_at: 0,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        }
    }

    #[test]
    fn offsets_and_hit_testing() {
        let blocks = vec![block("a", "one"), block("b", "two"), block("c", "three")];
        let mut layout = LayoutCache::default();
        assert!(layout.sync(&blocks));
        layout.set_height("a", 10.0);
        layout.set_height("b", 20.0);
        layout.set_height("c", 30.0);
        assert_eq!(layout.total_height(), 60.0);
        assert_eq!(layout.row_offset(1), 10.0);
        assert_eq!(layout.index_at(0.0), 0);
        assert_eq!(layout.index_at(9.9), 0);
        assert_eq!(layout.index_at(10.0), 1);
        assert_eq!(layout.index_at(55.0), 2);
        assert_eq!(layout.visible_range(10.0, 20.0, 0.0), 1..3);
    }

    #[test]
    fn content_change_invalidates_one_height() {
        let mut blocks = vec![block("a", "one"), block("b", "two")];
        let mut layout = LayoutCache::default();
        layout.sync(&blocks);
        layout.set_height("a", 10.0);
        layout.set_height("b", 20.0);
        blocks[1] = block("b", "two grew");
        assert!(layout.sync(&blocks));
        assert_eq!(layout.height("a"), Some(10.0));
        assert_eq!(layout.height("b"), None);
    }
}
