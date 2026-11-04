//! UTF-8 storage, UTF-16 platform ranges, grapheme navigation and undo for native
//! text fields. Selection offsets are always byte boundaries in `text`.
use std::collections::VecDeque;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Snapshot {
    text: String,
    anchor: usize,
    caret: usize,
}

#[derive(Clone, Debug)]
struct Edit {
    start: usize,
    deleted: VecDeque<u8>,
    inserted: String,
    before: (usize, usize),
    after: (usize, usize),
}

impl Edit {
    fn bytes(&self) -> usize {
        self.deleted.len() + self.inserted.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditKind {
    TypingOther,
    FirstSpace,
    ConsecutiveSpace,
    DeleteLeft,
    DeleteRight,
}
impl EditKind {
    fn typing(self) -> bool {
        matches!(
            self,
            Self::TypingOther | Self::FirstSpace | Self::ConsecutiveSpace
        )
    }
    fn space(self) -> bool {
        matches!(self, Self::FirstSpace | Self::ConsecutiveSpace)
    }
}

const HISTORY_BYTES: usize = 16 * 1024 * 1024;
const HISTORY_EDITS: usize = 4096;

#[derive(Clone, Debug, Default)]
pub struct TextBuffer {
    state: Snapshot,
    undo: VecDeque<Edit>,
    redo: Vec<Edit>,
    history_bytes: usize,
    revision: u64,
    group_kind: Option<EditKind>,
    composition: Option<Snapshot>,
    pub marked: Option<Range<usize>>,
}

impl TextBuffer {
    pub fn new(text: String) -> Self {
        let end = text.len();
        Self {
            state: Snapshot {
                text,
                anchor: end,
                caret: end,
            },
            ..Self::default()
        }
    }
    pub fn text(&self) -> &str {
        &self.state.text
    }
    pub fn caret(&self) -> usize {
        self.state.caret
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn reversed(&self) -> bool {
        self.state.caret < self.state.anchor
    }
    pub fn selection(&self) -> Range<usize> {
        self.state.anchor.min(self.state.caret)..self.state.anchor.max(self.state.caret)
    }
    pub fn boundary(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.text().len());
        while !self.text().is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }
    pub fn select(&mut self, anchor: usize, caret: usize) {
        let anchor = self.boundary(anchor);
        let caret = self.boundary(caret);
        if (anchor, caret) != (self.state.anchor, self.state.caret) {
            self.break_undo_group();
        }
        self.state.anchor = anchor;
        self.state.caret = caret;
    }
    pub fn move_to(&mut self, offset: usize, extend: bool) {
        self.select(if extend { self.state.anchor } else { offset }, offset);
    }
    pub fn select_all(&mut self) {
        self.select(0, self.text().len());
    }
    pub fn previous(&self, offset: usize) -> usize {
        self.text()
            .grapheme_indices(true)
            .rev()
            .find(|(i, _)| *i < offset)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
    pub fn next(&self, offset: usize) -> usize {
        self.text()
            .grapheme_indices(true)
            .find(|(i, _)| *i > offset)
            .map(|(i, _)| i)
            .unwrap_or(self.text().len())
    }
    pub fn horizontal(&mut self, right: bool, extend: bool, word: bool) {
        let selection = self.selection();
        let target = if !extend && !selection.is_empty() {
            if right {
                selection.end
            } else {
                selection.start
            }
        } else if word {
            let caret = self.caret();
            if right {
                self.text()
                    .unicode_word_indices()
                    .map(|(i, s)| i + s.len())
                    .find(|i| *i > caret)
                    .unwrap_or(self.text().len())
            } else {
                self.text()
                    .unicode_word_indices()
                    .map(|(i, _)| i)
                    .take_while(|i| *i < caret)
                    .last()
                    .unwrap_or(0)
            }
        } else if right {
            self.next(self.caret())
        } else {
            self.previous(self.caret())
        };
        self.move_to(target, extend);
    }
    pub fn line_edge(&mut self, end: bool, document: bool, extend: bool) {
        let caret = self.caret();
        let target = if document {
            if end { self.text().len() } else { 0 }
        } else if end {
            self.text()[caret..]
                .find('\n')
                .map(|n| caret + n)
                .unwrap_or(self.text().len())
        } else {
            self.text()[..caret].rfind('\n').map(|n| n + 1).unwrap_or(0)
        };
        self.move_to(target, extend);
    }
    pub fn vertical(&mut self, down: bool, extend: bool) {
        let caret = self.caret();
        let start = self.text()[..caret].rfind('\n').map(|n| n + 1).unwrap_or(0);
        let column = self.text()[start..caret].graphemes(true).count();
        let target_start = if down {
            match self.text()[caret..].find('\n') {
                Some(n) => caret + n + 1,
                None => self.text().len(),
            }
        } else if start == 0 {
            0
        } else {
            self.text()[..start - 1]
                .rfind('\n')
                .map(|n| n + 1)
                .unwrap_or(0)
        };
        let line = self.text()[target_start..].split('\n').next().unwrap_or("");
        let col = line
            .grapheme_indices(true)
            .nth(column)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        self.move_to(target_start + col, extend);
    }
    pub fn break_undo_group(&mut self) {
        self.group_kind = None;
    }
    fn checkpoint(&mut self, edit: Edit) {
        self.checkpoint_group(edit, None, false);
    }
    fn checkpoint_group(&mut self, mut edit: Edit, kind: Option<EditKind>, merge: bool) {
        self.history_bytes -= self.redo.iter().map(Edit::bytes).sum::<usize>();
        self.redo.clear();
        self.history_bytes += edit.bytes();
        if merge && let Some(previous) = self.undo.back_mut() {
            match kind {
                Some(EditKind::DeleteLeft) => {
                    for byte in edit.deleted.into_iter().rev() {
                        previous.deleted.push_front(byte);
                    }
                    previous.start = edit.start;
                }
                Some(EditKind::DeleteRight) => previous.deleted.append(&mut edit.deleted),
                _ => previous.inserted.push_str(&edit.inserted),
            }
            previous.after = edit.after;
        } else {
            self.undo.push_back(edit);
        }
        self.group_kind = kind;
        while self.undo.len() > 1
            && (self.undo.len() > HISTORY_EDITS || self.history_bytes > HISTORY_BYTES)
        {
            self.history_bytes -= self.undo.pop_front().unwrap().bytes();
        }
    }
    fn commit_composition(&mut self, previous: Snapshot) {
        // Composition is one transaction even if the platform repeatedly replaces
        // its marked range. Retain only the differing bytes in the undo log.
        let old = previous.text.as_str();
        let new = self.text();
        let mut start = old
            .bytes()
            .zip(new.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !old.is_char_boundary(start) || !new.is_char_boundary(start) {
            start -= 1;
        }
        let mut suffix = old[start..]
            .bytes()
            .rev()
            .zip(new[start..].bytes().rev())
            .take_while(|(a, b)| a == b)
            .count();
        while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix)
        {
            suffix -= 1;
        }
        if old != new {
            self.checkpoint(Edit {
                start,
                deleted: old[start..old.len() - suffix].bytes().collect(),
                inserted: new[start..new.len() - suffix].to_owned(),
                before: (previous.anchor, previous.caret),
                after: (self.state.anchor, self.state.caret),
            });
        }
    }
    /// Commit text, including the entire marked-text composition as one undo.
    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        self.break_undo_group();
        let start = self.boundary(range.start);
        let end = self.boundary(range.end).max(start);
        let previous = self.composition.take();
        let deleted = self.text()[start..end].to_owned();
        let before = (self.state.anchor, self.state.caret);
        self.replace_raw(start..end, text);
        self.marked = None;
        if let Some(previous) = previous {
            self.commit_composition(previous);
        } else if deleted != text {
            self.checkpoint(Edit {
                start,
                deleted: deleted.into_bytes().into(),
                inserted: text.to_owned(),
                before,
                after: (self.state.anchor, self.state.caret),
            });
        }
    }
    /// Replace several non-overlapping ranges as one atomic transaction. Returns
    /// the post-edit caret offset for each input range (same order). Undo is
    /// represented as one contiguous diff of the whole buffer so scattered
    /// multi-cursor edits collapse to a single history step.
    pub fn replace_many(
        &mut self,
        ranges: &[Range<usize>],
        text: &str,
        primary: usize,
    ) -> Vec<usize> {
        if ranges.is_empty() {
            return Vec::new();
        }
        if ranges.len() == 1 {
            self.replace(ranges[0].clone(), text);
            return vec![ranges[0].start + text.len()];
        }
        self.unmark();
        let old_text = self.state.text.clone();
        let before = (self.state.anchor, self.state.caret);
        let mut indexed = ranges
            .iter()
            .enumerate()
            .map(|(index, range)| {
                let start = self.boundary(range.start);
                let end = self.boundary(range.end).max(start);
                (index, start..end, start < end || !text.is_empty())
            })
            .filter(|(_, _, keep)| *keep)
            .collect::<Vec<_>>();
        indexed.sort_by_key(|(_, range, _)| range.start);
        let mut accepted = Vec::new();
        let mut last_end = 0;
        for entry in indexed {
            if entry.1.start < last_end {
                continue;
            }
            last_end = entry.1.end.max(entry.1.start);
            accepted.push(entry);
        }
        let mut positions = ranges.iter().map(|range| range.start).collect::<Vec<_>>();
        // Apply from the end so earlier offsets remain valid, then derive each
        // caret from the cumulative length delta of every preceding edit.
        for (_, range, _) in accepted.iter().rev() {
            self.replace_raw(range.clone(), text);
        }
        let mut delta = 0i64;
        for (index, range, _) in &accepted {
            positions[*index] = (range.start as i64 + delta + text.len() as i64) as usize;
            delta += text.len() as i64 - (range.end - range.start) as i64;
        }
        if let Some(primary) = positions.get(primary).copied() {
            self.state.anchor = primary;
            self.state.caret = primary;
        }
        self.marked = None;
        self.commit_composition(Snapshot {
            text: old_text,
            anchor: before.0,
            caret: before.1,
        });
        positions
    }

    /// Replace several non-overlapping ranges with independent replacement
    /// strings as one atomic transaction. Returns each accepted replacement's
    /// post-edit range in input order. Undo collapses to one whole-buffer diff,
    /// so multi-cursor formatting is a single history step.
    pub fn replace_many_with(
        &mut self,
        edits: &[(Range<usize>, String)],
        primary: usize,
    ) -> Vec<Range<usize>> {
        let mut positions = vec![0..0; edits.len()];
        if edits.is_empty() {
            return positions;
        }
        self.unmark();
        let old_text = self.state.text.clone();
        let before = (self.state.anchor, self.state.caret);
        let mut indexed = edits
            .iter()
            .enumerate()
            .map(|(index, (range, text))| {
                let start = self.boundary(range.start);
                let end = self.boundary(range.end).max(start);
                (index, start..end, text.as_str())
            })
            .filter(|(_, range, text)| !range.is_empty() || !text.is_empty())
            .collect::<Vec<_>>();
        indexed.sort_by_key(|(_, range, _)| range.start);
        let mut accepted = Vec::new();
        let mut last_end = 0;
        for entry in indexed {
            if entry.1.start < last_end {
                continue;
            }
            last_end = entry.1.end.max(entry.1.start);
            accepted.push(entry);
        }
        if accepted.is_empty() {
            return positions;
        }
        for (_, range, text) in accepted.iter().rev() {
            self.replace_raw(range.clone(), text);
        }
        let mut delta = 0i64;
        for (index, range, text) in &accepted {
            let start = (range.start as i64 + delta) as usize;
            positions[*index] = start..start + text.len();
            delta += text.len() as i64 - (range.end - range.start) as i64;
        }
        if let Some(primary) = positions.get(primary).cloned() {
            self.state.anchor = primary.start;
            self.state.caret = primary.end;
        }
        self.marked = None;
        self.commit_composition(Snapshot {
            text: old_text,
            anchor: before.0,
            caret: before.1,
        });
        positions
    }

    fn replace_raw(&mut self, range: Range<usize>, text: &str) {
        let start = self.boundary(range.start);
        let end = self.boundary(range.end).max(start);
        self.state.text.replace_range(start..end, text);
        self.revision = self.revision.wrapping_add(1);
        self.move_to(start + text.len(), false);
    }
    /// Monaco-style ordinary typing. Programmatic replace/insert remain
    /// independent transactions; only contiguous platform typing coalesces.
    pub fn type_text(&mut self, range: Range<usize>, text: &str) {
        if self.composition.is_some() || text.is_empty() || text.contains(['\n', '\r']) {
            self.replace(range, text);
            return;
        }
        let start = self.boundary(range.start);
        let end = self.boundary(range.end).max(start);
        let before = (self.state.anchor, self.state.caret);
        let previous = self.group_kind;
        let kind = if text == " " {
            if previous.is_some_and(EditKind::space) {
                EditKind::ConsecutiveSpace
            } else {
                EditKind::FirstSpace
            }
        } else {
            EditKind::TypingOther
        };
        let merge = start == end
            && self.selection() == (start..end)
            && previous.is_some_and(|previous| {
                previous.typing()
                    && (previous == EditKind::FirstSpace || previous.space() == kind.space())
            })
            && self.undo.back().is_some_and(|edit| {
                edit.after == before && edit.start + edit.inserted.len() == start
            });
        let deleted = self.text()[start..end].to_owned();
        self.replace_raw(start..end, text);
        self.marked = None;
        if deleted != text {
            self.checkpoint_group(
                Edit {
                    start,
                    deleted: deleted.into_bytes().into(),
                    inserted: text.to_owned(),
                    before,
                    after: (self.state.anchor, self.state.caret),
                },
                Some(kind),
                merge,
            );
        }
    }
    /// Enter starts a new typing transaction; subsequent ordinary text joins
    /// the newline and its auto-indent, matching Monaco's EnterOperation.
    pub fn enter_typed(&mut self, text: &str) {
        self.insert(text);
        self.group_kind = Some(EditKind::TypingOther);
    }

    pub fn erase_typed(&mut self, forward: bool) {
        let mut range = self.selection();
        if range.is_empty() {
            if forward {
                range.end = self.next(range.end);
            } else {
                range.start = self.previous(range.start);
            }
        }
        if range.is_empty() {
            self.break_undo_group();
            return;
        }
        if self.composition.is_some() {
            self.replace(range, "");
            return;
        }
        let kind = if forward {
            EditKind::DeleteRight
        } else {
            EditKind::DeleteLeft
        };
        let before = (self.state.anchor, self.state.caret);
        let deleted = self.text()[range.clone()].to_owned();
        let merge = !deleted.contains('\n')
            && self.group_kind == Some(kind)
            && self.undo.back().is_some_and(|edit| {
                edit.after == before
                    && edit.inserted.is_empty()
                    && if forward {
                        edit.start == range.start
                    } else {
                        range.end == edit.start
                    }
            });
        let start = range.start;
        self.replace_raw(range, "");
        self.marked = None;
        self.checkpoint_group(
            Edit {
                start,
                deleted: deleted.into_bytes().into(),
                inserted: String::new(),
                before,
                after: (self.state.anchor, self.state.caret),
            },
            Some(kind),
            merge,
        );
    }

    pub fn insert(&mut self, text: &str) {
        self.replace(self.selection(), text);
    }
    pub fn erase(&mut self, forward: bool) {
        let mut range = self.selection();
        if range.is_empty() {
            if forward {
                range.end = self.next(range.end);
            } else {
                range.start = self.previous(range.start);
            }
        }
        self.replace(range, "");
    }
    pub fn from_utf16(&self, offset: usize) -> usize {
        utf16_to_byte(self.text(), offset)
    }
    pub fn to_utf16(&self, offset: usize) -> usize {
        self.text()[..self.boundary(offset)].encode_utf16().count()
    }
    pub fn utf16_range(&self, range: Range<usize>) -> Range<usize> {
        self.to_utf16(range.start)..self.to_utf16(range.end)
    }
    pub fn byte_range(&self, range: Range<usize>) -> Range<usize> {
        self.from_utf16(range.start)..self.from_utf16(range.end)
    }
    pub fn mark(&mut self, range: Range<usize>, text: &str, selected_utf16: Option<Range<usize>>) {
        self.break_undo_group();
        if self.composition.is_none() {
            self.composition = Some(self.state.clone());
        }
        let start = self.boundary(range.start);
        self.replace_raw(range, text);
        self.marked = Some(start..start + text.len());
        if let Some(selection) = selected_utf16 {
            self.select(
                start + utf16_to_byte(text, selection.start),
                start + utf16_to_byte(text, selection.end),
            );
        }
    }
    pub fn unmark(&mut self) {
        self.marked = None;
        if let Some(previous) = self.composition.take() {
            self.commit_composition(previous);
        }
    }
    pub fn undo(&mut self) {
        self.unmark();
        self.break_undo_group();
        if let Some(mut edit) = self.undo.pop_back() {
            let deleted =
                std::str::from_utf8(edit.deleted.make_contiguous()).expect("undo text is UTF-8");
            self.state
                .text
                .replace_range(edit.start..edit.start + edit.inserted.len(), deleted);
            self.revision = self.revision.wrapping_add(1);
            self.select(edit.before.0, edit.before.1);
            self.redo.push(edit);
        }
    }
    pub fn redo(&mut self) {
        self.unmark();
        self.break_undo_group();
        if let Some(edit) = self.redo.pop() {
            self.state
                .text
                .replace_range(edit.start..edit.start + edit.deleted.len(), &edit.inserted);
            self.revision = self.revision.wrapping_add(1);
            self.select(edit.after.0, edit.after.1);
            self.undo.push_back(edit);
        }
    }
}

pub fn utf16_to_byte(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (i, ch) in text.char_indices() {
        if units + ch.len_utf16() > offset {
            return i;
        }
        units += ch.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn typed(buffer: &mut TextBuffer, text: &str) {
        for ch in text.chars() {
            buffer.type_text(buffer.selection(), &ch.to_string());
        }
    }
    #[test]
    fn typing_undo_matches_monaco_first_space_and_consecutive_space_boundaries() {
        let mut b = TextBuffer::default();
        typed(&mut b, "hello world");
        b.undo();
        assert_eq!(b.text(), "hello");
        b.undo();
        assert_eq!(b.text(), "");
        b.redo();
        assert_eq!(b.text(), "hello");
        b.redo();
        assert_eq!(b.text(), "hello world");
        let mut b = TextBuffer::default();
        typed(&mut b, "a  b");
        b.undo();
        assert_eq!(b.text(), "a  ");
        b.undo();
        assert_eq!(b.text(), "a");
        b.undo();
        assert_eq!(b.text(), "");
        typed(&mut b, "x.y");
        b.undo();
        assert_eq!(b.text(), "");
    }
    #[test]
    fn enter_starts_a_group_that_includes_following_text_and_indentation() {
        let mut b = TextBuffer::default();
        typed(&mut b, "line");
        b.enter_typed("\n  ");
        typed(&mut b, "tail");
        assert_eq!(b.text(), "line\n  tail");
        b.undo();
        assert_eq!(b.text(), "line");
        b.undo();
        assert_eq!(b.text(), "");
        b.redo();
        b.redo();
        assert_eq!(b.text(), "line\n  tail");
    }

    #[test]
    fn grouped_typing_respects_selection_navigation_paste_and_composition() {
        let mut b = TextBuffer::new("original".into());
        b.select(8, 0);
        typed(&mut b, "界!");
        b.undo();
        assert_eq!(b.text(), "original");
        assert!(b.reversed());
        assert_eq!(b.selection(), 0..8);
        b.redo();
        b.move_to(0, false);
        typed(&mut b, "a");
        b.undo();
        assert_eq!(b.text(), "界!");
        b.insert("paste");
        typed(&mut b, "suffix");
        b.undo();
        assert_eq!(b.text(), "paste界!");
        b.undo();
        assert_eq!(b.text(), "界!");
        let mut b = TextBuffer::default();
        typed(&mut b, "ab");
        b.mark(2..2, "n", None);
        b.mark(2..3, "ni", None);
        b.replace(2..4, "你");
        b.undo();
        assert_eq!(b.text(), "ab");
        b.undo();
        assert_eq!(b.text(), "");
    }
    #[test]
    fn contiguous_deletion_groups_preserve_graphemes_and_newline_stops() {
        let mut b = TextBuffer::new("ab👩‍💻界".into());
        let end = b.text().len();
        b.erase_typed(false);
        b.erase_typed(false);
        assert_eq!(b.text(), "ab");
        b.undo();
        assert_eq!(b.text(), "ab👩‍💻界");
        assert_eq!(b.caret(), end);
        b.redo();
        assert_eq!(b.text(), "ab");
        b.move_to(0, false);
        b.erase_typed(true);
        b.erase_typed(true);
        assert_eq!(b.text(), "");
        b.undo();
        assert_eq!(b.text(), "ab");
        assert_eq!(b.caret(), 0);
        let mut b = TextBuffer::new("ab\ncd".into());
        b.erase_typed(false);
        b.erase_typed(false);
        b.erase_typed(false);
        b.erase_typed(false);
        assert_eq!(b.text(), "a");
        b.undo();
        assert_eq!(b.text(), "ab\n");
        b.undo();
        assert_eq!(b.text(), "ab\ncd");
    }
    #[test]
    fn long_typing_and_backward_deletion_store_one_linear_delta_per_group() {
        let mut b = TextBuffer::default();
        typed(&mut b, &"x".repeat(10_000));
        assert_eq!(b.undo.len(), 1);
        assert_eq!(b.history_bytes, 10_000);
        b.undo();
        assert_eq!(b.text(), "");
        b.redo();
        for _ in 0..10_000 {
            b.erase_typed(false);
        }
        assert_eq!(b.text(), "");
        assert_eq!(b.undo.len(), 2);
        assert_eq!(b.history_bytes, 20_000);
        b.undo();
        assert_eq!(b.text(), "x".repeat(10_000));
    }

    #[test]
    fn large_documents_store_edit_deltas_and_restore_selection() {
        let text = "界".repeat(400_000);
        let mut b = TextBuffer::new(text.clone());
        b.select(6, 0);
        b.insert("🌍");
        b.undo();
        assert!(b.reversed());
        assert_eq!(b.selection(), 0..6);
        b.redo();
        b.move_to(b.text().len(), false);
        for _ in 0..1000 {
            b.insert("x");
        }
        assert!(b.history_bytes < 2000);
        for _ in 0..1000 {
            b.undo();
        }
        assert!(b.text().starts_with("🌍"));
        assert!(!b.text().ends_with('x'));
    }
    #[test]
    fn selection_replacement_undo_and_redo_preserve_unicode() {
        let mut b = TextBuffer::new("a👩‍💻e\u{301}界".into());
        b.horizontal(false, false, false);
        b.erase(false);
        assert_eq!(b.text(), "a👩‍💻界");
        b.undo();
        assert_eq!(b.text(), "a👩‍💻e\u{301}界");
        b.redo();
        assert_eq!(b.text(), "a👩‍💻界");
        b.select_all();
        b.insert("你好\nsecond line");
        assert_eq!(b.caret(), b.text().len());
        b.undo();
        assert_eq!(b.text(), "a👩‍💻界");
    }
    #[test]
    fn ime_replaces_marked_text_and_undoes_as_one_transaction() {
        let mut b = TextBuffer::new("hello 🌍 ".into());
        let start = b.text().len();
        b.mark(start..start, "ni", Some(0..2));
        b.mark(b.marked.clone().unwrap(), "你好", Some(2..2));
        assert_eq!(b.caret(), start + "你好".len());
        b.replace(b.marked.clone().unwrap(), "你好");
        assert_eq!(b.text(), "hello 🌍 你好");
        b.undo();
        assert_eq!(b.text(), "hello 🌍 ");
        b.redo();
        assert_eq!(b.text(), "hello 🌍 你好");
    }
    #[test]
    fn utf16_ranges_and_reversed_selections_are_byte_safe() {
        let mut b = TextBuffer::new("a🌍界".into());
        assert_eq!(b.byte_range(1..3), 1..5);
        assert_eq!(b.utf16_range(1..5), 1..3);
        assert_eq!(b.from_utf16(2), 1);
        b.select(b.text().len(), 1);
        assert!(b.reversed());
        b.insert("x");
        assert_eq!(b.text(), "ax");
    }
    #[test]
    fn independent_replace_many_edits_are_one_undo_and_return_new_ranges() {
        let mut b = TextBuffer::new("aa bb cc".into());
        let ranges = b.replace_many_with(
            &[
                (0..2, "X".into()),
                (3..5, "YY".into()),
                (6..8, "ZZZ".into()),
            ],
            1,
        );
        assert_eq!(b.text(), "X YY ZZZ");
        assert_eq!(ranges, vec![0..1, 2..4, 5..8]);
        assert_eq!(b.selection(), 2..4);
        b.undo();
        assert_eq!(b.text(), "aa bb cc");
        b.redo();
        assert_eq!(b.text(), "X YY ZZZ");

        let mut b = TextBuffer::new("abcdef".into());
        let ranges = b.replace_many_with(
            &[(0..2, "A".into()), (1..3, "B".into()), (4..5, "C".into())],
            0,
        );
        assert_eq!(b.text(), "AcdCf");
        assert_eq!(ranges, vec![0..1, 0..0, 3..4]);
    }

    #[test]
    fn scattered_replace_many_is_one_undo_and_returns_shifted_carets() {
        let mut b = TextBuffer::new("foo bar foo bar foo".into());
        let positions = b.replace_many(&[0..3, 8..11, 16..19], "x", 0);
        assert_eq!(b.text(), "x bar x bar x");
        assert_eq!(positions, vec![1, 7, 13]);
        assert_eq!(b.caret(), 1);
        b.undo();
        assert_eq!(b.text(), "foo bar foo bar foo");
        b.redo();
        assert_eq!(b.text(), "x bar x bar x");

        let mut b = TextBuffer::new("abcdef".into());
        let positions = b.replace_many(&[0..2, 1..3, 4..5], "Z", 0);
        assert_eq!(b.text(), "ZcdZf");
        assert_eq!(positions, vec![1, 1, 4]);
        assert_eq!(b.caret(), 1);
    }

    #[test]
    fn moving_and_deleting_respect_multiline_graphemes() {
        let mut b = TextBuffer::new("a👩‍💻\nb界\nc".into());
        b.move_to("a👩‍💻".len(), false);
        b.vertical(true, false);
        assert_eq!(b.caret(), "a👩‍💻\nb界".len());
        b.vertical(false, false);
        assert_eq!(b.caret(), "a👩‍💻".len());
        b.erase(false);
        assert_eq!(b.text(), "a\nb界\nc");
        b.move_to(2, false);
        b.erase(false);
        assert_eq!(b.text(), "ab界\nc");
    }
}
