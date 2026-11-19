//! Vela turn presentation projected into independently virtualized rows.
//! A long answer remains a sequence of stable Markdown blocks; expanding a
//! process group never turns the whole conversation into one layout element.
use super::*;

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    hash::{Hash, Hasher},
    ops::Range,
    rc::Rc,
};
use unicode_segmentation::UnicodeSegmentation;

const TRANSCRIPT_BOTTOM_PADDING: Pixels = px(32.);

#[path = "work_summary_caption.rs"]
mod work_summary_caption;

type SpacingCache = HashMap<Arc<str>, SpacingEntry>;

struct SpacingEntry {
    source: Arc<str>,
    streaming: bool,
    dependency: u64,
    spacing: markdown_view::BlockSpacing,
}

fn cached_spacing(
    cache: &mut SpacingCache,
    block: &Block,
    dependency: u64,
    measure: impl Fn() -> markdown_view::BlockSpacing,
) -> markdown_view::BlockSpacing {
    let entry = || SpacingEntry {
        source: block.text.clone(),
        streaming: block.streaming,
        dependency,
        spacing: measure(),
    };
    let current = cache.entry(block.id.clone()).or_insert_with(entry);
    if current.streaming != block.streaming
        || current.dependency != dependency
        || (!Arc::ptr_eq(&current.source, &block.text) && current.source != block.text)
    {
        *current = entry();
    }
    current.spacing
}

/// A transcript-order selection spanning one or more user messages. Points are
/// `(message index, segment index, byte offset)` so virtualized rows never need
/// to be painted to contribute text.
pub(super) struct UserTextSelection {
    pub(super) anchor: markdown_view::UserSelectionPoint,
    pub(super) head: markdown_view::UserSelectionPoint,
    order: Vec<Arc<str>>,
}

impl UserTextSelection {
    fn ordered(
        &self,
    ) -> (
        markdown_view::UserSelectionPoint,
        markdown_view::UserSelectionPoint,
    ) {
        let a = (self.anchor.message, self.anchor.segment, self.anchor.offset);
        let b = (self.head.message, self.head.segment, self.head.offset);
        if a <= b {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    fn includes(&self, message: usize, segment: usize) -> bool {
        let (start, end) = self.ordered();
        let key = (message, segment);
        key >= (start.message, start.segment) && key <= (end.message, end.segment)
    }

    pub(super) fn selected_text(&self, segments: &HashMap<Arc<str>, Vec<SharedString>>) -> String {
        let (start, end) = self.ordered();
        let mut parts: Vec<String> = Vec::new();
        for message in start.message..=end.message {
            let Some(id) = self.order.get(message) else {
                continue;
            };
            let Some(message_segments) = segments.get(id) else {
                continue;
            };
            for (index, segment) in message_segments.iter().enumerate() {
                if !self.includes(message, index) {
                    continue;
                }
                let text: &str = segment;
                let from = if (message, index) == (start.message, start.segment) {
                    clamp_boundary(text, start.offset)
                } else {
                    0
                };
                let to = if (message, index) == (end.message, end.segment) {
                    clamp_boundary(text, end.offset)
                } else {
                    text.len()
                };
                if from < to {
                    parts.push(text[from..to].to_string());
                }
            }
        }
        parts.join("\n")
    }

    pub(super) fn selected_images(
        &self,
        segments: &HashMap<Arc<str>, Vec<SharedString>>,
        images: &HashMap<Arc<str>, Vec<(usize, Arc<gpui::Image>)>>,
    ) -> Vec<Arc<gpui::Image>> {
        let (start, end) = self.ordered();
        let mut selected = Vec::new();
        for message in start.message..=end.message {
            let Some(id) = self.order.get(message) else {
                continue;
            };
            if segments.get(id).is_none() {
                continue;
            }
            if let Some(entries) = images.get(id) {
                for (segment, image) in entries {
                    if self.includes(message, *segment) {
                        selected.push(image.clone());
                    }
                }
            }
        }
        selected
    }
}

fn clamp_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn message_text_points(
    ui: &TranscriptUi,
    message: usize,
) -> Option<(
    markdown_view::UserSelectionPoint,
    markdown_view::UserSelectionPoint,
)> {
    let block = ui.user_message_order.get(message)?;
    let texts = ui.user_message_segments.get(block)?;
    let last = texts.last()?;
    Some((
        markdown_view::UserSelectionPoint {
            message,
            segment: 0,
            offset: 0,
        },
        markdown_view::UserSelectionPoint {
            message,
            segment: texts.len().saturating_sub(1),
            offset: last.len(),
        },
    ))
}

/// The first and last selectable points across all rendered user messages.
fn all_user_text_points(
    ui: &TranscriptUi,
) -> Option<(
    markdown_view::UserSelectionPoint,
    markdown_view::UserSelectionPoint,
)> {
    let order = &ui.user_message_order;
    let segments = &ui.user_message_segments;
    let first = order.iter().enumerate().find_map(|(index, block)| {
        let texts = segments.get(block)?;
        (!texts.is_empty()).then_some(index)
    })?;
    let (last, segment, offset) = order.iter().enumerate().rev().find_map(|(index, block)| {
        let texts = segments.get(block)?;
        texts
            .last()
            .map(|text| (index, texts.len().saturating_sub(1), text.len()))
    })?;
    Some((
        markdown_view::UserSelectionPoint {
            message: first,
            segment: 0,
            offset: 0,
        },
        markdown_view::UserSelectionPoint {
            message: last,
            segment,
            offset,
        },
    ))
}

/// The word a double-click selects, using Unicode word boundaries. Whitespace
/// and punctuation fall back to the grapheme under the pointer.
pub(super) fn selected_word(text: &str, index: usize) -> Range<usize> {
    let index = index.min(text.len());
    for (start, word) in text.split_word_bound_indices() {
        let end = start + word.len();
        if index < end || (end == text.len() && index <= end) {
            return start..end;
        }
    }
    for (start, grapheme) in text.grapheme_indices(true) {
        let end = start + grapheme.len();
        if index < end || end == text.len() {
            return start..end;
        }
    }
    0..0
}

#[derive(Default)]
pub(super) struct TranscriptUi {
    spacing: RefCell<SpacingCache>,
    activity: RefCell<vela_core::activity::ActivityCache>,
    pub(super) tool_open: HashSet<String>,
    tool_groups: HashMap<SharedString, bool>,
    reasoning_open: HashMap<String, bool>,
    hovered_answer: Option<Arc<str>>,
    copied_answer: Option<Arc<str>>,
    answer_copy_generation: u64,
    answer_copy_owner: Arc<()>,
    focus: HashMap<SharedString, gpui::FocusHandle>,
    pub(super) user_text_selection: Option<UserTextSelection>,
    pub(super) user_text_session: Option<String>,
    user_text_drag: Rc<Cell<bool>>,
    pub(super) user_message_order: Vec<Arc<str>>,
    user_message_index: HashMap<Arc<str>, usize>,
    user_message_sources: HashMap<Arc<str>, (Arc<str>, u64)>,
    user_order_rows: std::sync::Weak<Vec<Row>>,
    pub(super) user_message_segments: HashMap<Arc<str>, Vec<SharedString>>,
    pub(super) user_message_images: HashMap<Arc<str>, Vec<(usize, Arc<gpui::Image>)>>,
    user_text_focus: HashMap<Arc<str>, gpui::FocusHandle>,
    pub(super) markdown: markdown_view::MarkdownUi,
    viewport: crate::scroll::ListViewportHandle,
    wheel_input: crate::scroll::TranscriptWheelInput,
    wheel_motion: crate::scroll::WheelMotion,
    pub(super) scroll_generation: u64,
    pub(super) scrolling_to_bottom: bool,
}
impl TranscriptUi {
    pub(super) fn cancel_wheel_scroll(&mut self) {
        self.wheel_motion.cancel();
    }

    pub(super) fn contains_focused(&self, window: &Window) -> bool {
        self.markdown.contains_focused(window)
            || self.focus.values().any(|focus| focus.is_focused(window))
            || self.user_text_focus.values().any(|focus| focus.is_focused(window))
    }

    #[cfg(test)]
    pub(super) fn register_focus_for_test(&mut self, focus: gpui::FocusHandle, user_text: bool) {
        if user_text {
            self.user_text_focus.insert("test-user".into(), focus);
        } else {
            self.focus.insert("test-control".into(), focus);
        }
    }

    fn answer_copy_is_current(&self, owner: &Arc<()>, generation: u64, key: &str) -> bool {
        Arc::ptr_eq(&self.answer_copy_owner, owner)
            && self.answer_copy_generation == generation
            && self.copied_answer.as_deref() == Some(key)
    }

    pub(super) fn answer_footer_visible(
        &self,
        answer: &str,
        footer: &gpui::FocusHandle,
        window: &Window,
    ) -> bool {
        self.hovered_answer.as_deref() == Some(answer)
            || footer.is_focused(window)
            || self.markdown.answer_contains_focused(answer, window)
    }

    fn user_text_focus(
        &mut self,
        session: &str,
        id: &Arc<str>,
        row: &Row,
        window: &mut Window,
        cx: &mut Context<VelaApp>,
    ) -> gpui::FocusHandle {
        if let Some(focus) = self.user_text_focus.get(id) {
            return focus.clone();
        }
        let focus = cx.focus_handle();
        let row_key = row.id.clone();
        let session = session.to_owned();
        let blur_session = session.clone();
        cx.on_focus(&focus, window, move |app, _, cx| {
            app.row_cache.invalidate_session(&session);
            if let Some(index) = app
                .rows_for(&session)
                .iter()
                .position(|row| row.id == row_key)
                && let Some(list) = app.list_for(&session)
            {
                list.scroll_to_reveal_item(index);
            }
            cx.notify();
        })
        .detach();
        cx.on_blur(&focus, window, move |app, _, cx| {
            app.row_cache.invalidate_session(&blur_session);
            cx.notify();
        }).detach();
        self.user_text_focus.insert(id.clone(), focus.clone());
        focus
    }
    pub(super) fn navigate(
        &mut self,
        event: &KeyDownEvent,
        list: &ListState,
        window: &Window,
    ) -> bool {
        if !self.contains_focused(window) {
            return false;
        }
        let key = &event.keystroke;
        if key.modifiers.alt || key.modifiers.control || key.modifiers.shift {
            return false;
        }
        let viewport_height = list.viewport_bounds().size.height;
        let page = if viewport_height > px(0.) { viewport_height } else { self.viewport.height() };
        match key.key.as_str() {
            "home" if !key.modifiers.platform => list.scroll_to(ListOffset::default()),
            "end" if !key.modifiers.platform => list.scroll_to_end(),
            "up" if key.modifiers.platform => list.scroll_to(ListOffset::default()),
            "down" if key.modifiers.platform => list.scroll_to_end(),
            "pageup" if !key.modifiers.platform => {
                list.scroll_by(-page * 0.875)
            }
            "pagedown" if !key.modifiers.platform => {
                list.scroll_by(page * 0.875)
            }
            "up" if !key.modifiers.platform => list.scroll_by(px(-40.)),
            "down" if !key.modifiers.platform => list.scroll_by(px(40.)),
            _ => return false,
        }
        self.scroll_generation += 1;
        self.scrolling_to_bottom = false;
        true
    }
}

#[derive(Clone)]
pub(super) struct Row {
    pub(super) id: SharedString,
    before: f32,
    after: f32,
    presentation: u64,
    /// Keep collapsed leaves in the same list/parent under their lifetime ID.
    /// Collapsing never changes their identity or parent.
    hidden: bool,
    summary: Option<Arc<str>>,
    kind: RowKind,
}

#[derive(Clone)]
enum RowKind {
    Content {
        block: Arc<Block>,
        answer: Option<Arc<str>>,
        reasoning: bool,
    },
    User(Arc<Block>),
    ScrollControl,
    Reasoning {
        key: String,
        running: bool,
        open: bool,
    },
    ToolGroup {
        title: Arc<str>,
        count: usize,
        running: usize,
        failed: usize,
        active: bool,
        open: bool,
    },
    Footer {
        key: Arc<str>,
        text: Arc<str>,
        duration: Option<i64>,
        interrupted: bool,
    },
    Status(Arc<Block>),
}

#[derive(Default)]
pub(super) struct PanelTranscript {
    pub(super) ui: TranscriptUi,
    pub(super) rows: Arc<Vec<Row>>,
    pub(super) blocks: Arc<Vec<Block>>,
    pub(super) phase: SessionPhase,
    pub(super) viewport: crate::transcript_viewport::TranscriptViewport,
    pub(super) dirty: bool,
}

/// Content identity for layout invalidation. `Block` owns immutable `Arc<str>`
/// payloads, so their pointers change exactly when the content does; hashing
/// pointers avoids re-reading a megabyte of tool output for every row on every
/// streamed delta. Equal content in a new allocation merely remeasures.
fn hash_block_revision(block: &Block, hash: &mut impl Hasher) {
    fn ptr(value: &Arc<str>, hash: &mut impl Hasher) {
        (Arc::as_ptr(value) as *const () as usize).hash(hash);
    }
    block.kind.hash(hash);
    ptr(&block.text, hash);
    ptr(&block.detail, hash);
    ptr(&block.output, hash);
    ptr(&block.progress, hash);
    block.status.hash(hash);
    block.streaming.hash(hash);
    block.interrupted.hash(hash);
    block.agent.as_deref().hash(hash);
    block.model.as_deref().hash(hash);
    block.created_at.hash(hash);
    block.completed_at.hash(hash);
    block.child_session_id.as_deref().hash(hash);
    for file in &block.files { file.hash(hash); }
    for call in &block.sub_calls { call.hash(hash); }
    for attachment in &block.attachments { ptr(attachment, hash); }
}

impl Row {
    pub(super) fn cacheable(&self) -> bool {
        !self.hidden && match &self.kind {
            RowKind::Content { block, .. } => !block.streaming || block.kind == BlockKind::Assistant
                || (block.kind == BlockKind::Tool && crate::subagent::is_task_tool(&block.detail)),
            RowKind::User(_) | RowKind::Footer { .. } | RowKind::ToolGroup { .. } => true,
            RowKind::Status(block) => !block.streaming,
            RowKind::Reasoning { running, .. } => !running,
            RowKind::ScrollControl => false,
        }
    }

    pub(super) fn revision(&self) -> crate::transcript_viewport::RowRevision {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.before.to_bits().hash(&mut hash);
        self.after.to_bits().hash(&mut hash);
        self.presentation.hash(&mut hash);
        match &self.kind {
            RowKind::Content {
                block,
                answer,
                reasoning,
            } => {
                hash_block_revision(block, &mut hash);
                answer.hash(&mut hash);
                reasoning.hash(&mut hash);
            }
            RowKind::User(block) | RowKind::Status(block) => {
                hash_block_revision(block, &mut hash);
            }
            RowKind::ScrollControl => {}
            RowKind::Reasoning { running, open, .. } => (running, open).hash(&mut hash),
            RowKind::ToolGroup { title, count, running, failed, active, open } => {
                (title, count, running, failed, active, open).hash(&mut hash);
            }
            RowKind::Footer {
                key,
                text,
                duration,
                interrupted,
            } => {
                (duration, interrupted).hash(&mut hash);
                key.hash(&mut hash);
                (Arc::as_ptr(text) as *const () as usize).hash(&mut hash);
            }
        }
        self.hidden.hash(&mut hash);
        self.summary.hash(&mut hash);
        crate::transcript_viewport::RowRevision::new(self.id.clone(), hash.finish())
    }
}

pub(super) fn configure_user_order(ui: &mut TranscriptUi, rows: &Arc<Vec<Row>>) {
    if ui.user_order_rows.as_ptr() == Arc::as_ptr(rows) {
        return;
    }
    ui.user_order_rows = Arc::downgrade(rows);
    let users = rows.iter().filter_map(|row| match &row.kind {
        RowKind::User(block) => Some((block, row.presentation)),
        _ => None,
    });
    let mut previous = ui.user_message_order.iter();
    if users.clone().all(|(block, stamp)| {
        previous.next() == Some(&block.id)
            && ui.user_message_sources.get(&block.id).is_some_and(|(text, old_stamp)| {
                *old_stamp == stamp && (Arc::ptr_eq(text, &block.text) || *text == block.text)
            })
    }) && previous.next().is_none() {
        return;
    }
    let mut order = Vec::new();
    let mut sources = HashMap::new();
    for (block, stamp) in users {
        order.push(block.id.clone());
        sources.insert(block.id.clone(), (block.text.clone(), stamp));
    }
    let unchanged = |id: &Arc<str>| {
        ui.user_message_sources.get(id).zip(sources.get(id))
            .is_some_and(|((old, old_stamp), (new, new_stamp))| {
                old_stamp == new_stamp && (Arc::ptr_eq(old, new) || old == new)
            })
    };
    ui.user_message_segments.retain(|id, _| unchanged(id));
    ui.user_message_images.retain(|id, _| unchanged(id));
    ui.user_message_sources = sources;
    ui.user_message_index = order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect();
    ui.user_message_order = order;
}

fn push(rows: &mut Vec<Row>, id: String, before: f32, kind: RowKind) {
    rows.push(Row {
        id: id.into(),
        before,
        after: 0.,
        presentation: 0,
        hidden: false,
        summary: None,
        kind,
    });
}

#[cfg(test)]
fn text_rows(
    rows: &mut Vec<Row>,
    blocks: &[Block],
    gap: f32,
    answer: Option<Arc<str>>,
    reasoning: bool,
    spacing: &mut SpacingCache,
) -> f32 {
    let mut previous = markdown_view::BlockSpacing::default();
    let mut tail_margin = 0.;
    for (index, block) in blocks.iter().enumerate() {
        let current = cached_spacing(spacing, block, 0, || {
            markdown_view::block_spacing(&block.text)
        });
        push(
            rows,
            format!("text:{}", block.id),
            if index == 0 {
                gap
            } else {
                previous.bottom.max(current.top_after(previous))
            },
            RowKind::Content {
                block: Arc::new(block.clone()),
                answer: answer.clone(),
                reasoning,
            },
        );
        tail_margin = current.bottom_at_document_end(previous);
        previous = current;
    }
    if reasoning && !blocks.is_empty() {
        rows.last_mut().unwrap().after = 6. + tail_margin;
    }
    tail_margin
}

/// Vela keeps one delegated-agent row per call or explicitly identified child.
/// Descriptions are correlation hints for unambiguous synthetic notices, never
/// identities: two simultaneous dispatches can have identical descriptions.
fn collapse_subagent_tools(blocks: &[Block]) -> Cow<'_, [Block]> {
    if !blocks.iter().any(|block| {
        block.kind == BlockKind::Tool && crate::subagent::is_task_tool(&block.detail)
    }) {
        return Cow::Borrowed(blocks);
    }
    fn merge(existing: &mut Block, latest: &Block) {
        if !latest.text.is_empty() {
            existing.text = latest.text.clone();
        }
        if !latest.detail.is_empty() {
            existing.detail = latest.detail.clone();
        }
        if !latest.output.is_empty() {
            existing.output = latest.output.clone();
        }
        existing.status = latest.status;
        existing.streaming = latest.streaming;
        existing.created_at = existing.created_at.min(latest.created_at);
        if latest.completed_at.is_some() {
            existing.completed_at = latest.completed_at;
        }
        if latest.child_session_id.is_some() {
            existing.child_session_id = latest.child_session_id.clone();
        }
    }

    let mut visible = Vec::with_capacity(blocks.len());
    let mut turn: Vec<&Block> = Vec::new();
    let flush = |turn: &mut Vec<&Block>, visible: &mut Vec<Block>| {
        let mut canonical: Vec<(usize, Block)> = Vec::with_capacity(turn.len());
        let mut represented: HashMap<String, usize> = HashMap::new();
        let mut signatures: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, block) in turn.iter().enumerate() {
            if block.kind == BlockKind::Tool && crate::subagent::is_task_tool(&block.detail) {
                let signature =
                    crate::subagent::tool_dispatch_signature(&block.detail, &block.text);
                let child = block.child_session_id.as_deref().unwrap_or("");
                let child_key = (!child.is_empty()).then(|| format!("child:{child}"));
                let call_key = format!("call:{}", block.id);
                let existing = represented.get(&call_key).copied().or_else(|| {
                    child_key.as_ref().and_then(|key| represented.get(key).copied())
                });
                let position = if let Some(position) = existing {
                    let previous_child = canonical[position].1.child_session_id.clone();
                    merge(&mut canonical[position].1, block);
                    if let Some(previous) = previous_child
                        && !child.is_empty() && previous.as_ref() != child
                    {
                        represented.remove(&format!("child:{previous}"));
                    }
                    position
                } else {
                    let position = canonical.len();
                    canonical.push((index, (*block).clone()));
                    position
                };
                represented.insert(call_key, position);
                if let Some(key) = child_key {
                    represented.insert(key, position);
                }
                if let Some(signature) = signature {
                    let candidates = signatures.entry(signature).or_default();
                    if !candidates.contains(&position) {
                        candidates.push(position);
                    }
                }
                continue;
            }
            if block.kind == BlockKind::System
                && let Some(reference) = crate::subagent::parse(&block.text)
            {
                let position = (!reference.id.is_empty())
                    .then(|| represented.get(&format!("child:{}", reference.id)).copied())
                    .flatten()
                    .or_else(|| {
                        let signature = crate::subagent::ref_dispatch_signature(&reference)?;
                        let mut candidates = signatures.get(&signature)?.iter().copied()
                            .filter(|position| {
                                let child = canonical[*position].1.child_session_id.as_deref()
                                    .unwrap_or_default();
                                reference.id.is_empty() || child.is_empty() || child == reference.id
                            });
                        let first = candidates.next()?;
                        candidates.next().is_none().then_some(first)
                    });
                if let Some(position) = position {
                    let card = &mut canonical[position].1;
                    if !reference.id.is_empty() {
                        card.child_session_id = Some(reference.id.clone().into());
                        represented.insert(format!("child:{}", reference.id), position);
                    }
                    if !reference.state.is_empty() {
                        card.status =
                            crate::subagent::settled_status(card.status, &reference.state);
                        if matches!(card.status, ToolStatus::Success | ToolStatus::Failed) {
                            card.streaming = false;
                            card.completed_at = card.completed_at.or(Some(
                                block.completed_at.unwrap_or(block.created_at).max(card.created_at),
                            ));
                        }
                    }
                    continue;
                }
            }
            canonical.push((index, (*block).clone()));
        }
        for (_, block) in canonical {
            visible.push(block);
        }
        turn.clear();
    };
    for block in blocks {
        if block.kind == BlockKind::User {
            flush(&mut turn, &mut visible);
            visible.push(block.clone());
        } else {
            turn.push(block);
        }
    }
    flush(&mut turn, &mut visible);
    Cow::Owned(visible)
}

#[cfg(test)]
#[path = "transcript_activity_identity_tests.rs"]
mod activity_identity_tests;

pub(super) fn project(blocks: &[Block], busy: bool, session: &str, ui: &TranscriptUi) -> Vec<Row> {
    let blocks = collapse_subagent_tools(blocks);
    let conversation = vela_core::conversation::project(&blocks);
    let activity = ui.activity.borrow_mut().project(&conversation);
    let mut spacing = ui.spacing.borrow_mut();
    if spacing.len() > blocks.len().saturating_mul(2).max(64) {
        spacing.clear();
    }
    let mut rows = Vec::with_capacity(blocks.len());
    let mut previous_text: Option<(&str, markdown_view::BlockSpacing)> = None;
    let mut text_tail = 0.;
    for (index, node) in conversation.nodes.iter().enumerate() {
        let block = &node.block;
        let turn = &conversation.turns[node.turn_index];
        let mut gap: f32 = if rows.is_empty() { 0. } else { 12. };
        match block.kind {
            BlockKind::User => {
                previous_text = None;
                gap = if rows.is_empty() { 0. } else { 16. };
                push(
                    &mut rows,
                    format!("user:{}", block.message_id),
                    gap,
                    RowKind::User(block.clone()),
                );
            }
            BlockKind::Assistant => {
                if block.text.trim().is_empty() {
                    continue;
                }
                let dependency = ui.markdown.dependency_fingerprint(&block.id);
                let current = cached_spacing(&mut spacing, block, dependency, || {
                    markdown_view::block_spacing_cached(block, &ui.markdown)
                });
                if let Some((message, previous)) = previous_text
                    && message == block.message_id.as_ref()
                {
                    gap = previous.bottom.max(current.top_after(previous));
                }
                text_tail = current.bottom_at_document_end(
                    previous_text
                        .filter(|(message, _)| *message == block.message_id.as_ref())
                        .map(|(_, previous)| previous)
                        .unwrap_or_default(),
                );
                push(
                    &mut rows,
                    format!("text:{}", block.id),
                    gap,
                    RowKind::Content {
                        block: block.clone(),
                        answer: Some(block.message_id.clone()),
                        reasoning: false,
                    },
                );
                previous_text = Some((&block.message_id, current));
            }
            BlockKind::Reasoning => {
                previous_text = None;
                let key = format!("{session}:{}", block.part_id);
                let running = block.streaming;
                let open = ui.reasoning_open.get(&key).copied().unwrap_or(false);
                push(
                    &mut rows,
                    format!("reasoning:{key}"),
                    gap.min(8.),
                    RowKind::Reasoning { key, running, open },
                );
                if open && !block.text.trim().is_empty() {
                    let mut body = (**block).clone();
                    body.kind = BlockKind::Assistant;
                    push(
                        &mut rows,
                        format!("reasoning-body:{}", block.id),
                        4.,
                        RowKind::Content {
                            block: Arc::new(body),
                            answer: None,
                            reasoning: true,
                        },
                    );
                }
            }
            BlockKind::Tool | BlockKind::Shell => {
                previous_text = None;
                push(
                    &mut rows,
                    format!("activity:{}", block.id),
                    gap.min(8.),
                    RowKind::Content {
                        block: block.clone(),
                        answer: None,
                        reasoning: false,
                    },
                );
            }
            BlockKind::Activity => {
                // Running tools/reasoning are the activity timeline. Empty
                // synthetic activity rows must not appear and disappear on top.
                previous_text = None;
                if !block.detail.is_empty() && !block.text.trim().is_empty() {
                    push(
                        &mut rows,
                        format!("event:{}", block.id),
                        gap.min(8.),
                        RowKind::Status(block.clone()),
                    );
                }
            }
            _ => {
                previous_text = None;
                if block.kind == BlockKind::System
                    && crate::subagent::is_internal_reminder(&block.text)
                {
                    continue;
                }
                let prefix = if matches!(block.kind, BlockKind::Error | BlockKind::Retry) {
                    "note"
                } else {
                    "event"
                };
                push(
                    &mut rows,
                    format!("{prefix}:{}", block.id),
                    gap,
                    RowKind::Status(block.clone()),
                );
            }
        }
        if let Some(message) = turn.message(&block.message_id)
            && message.last_node == index
            && !message.text_blocks.is_empty()
            && !message.streaming
            && (message.completed_at.is_some() || !busy)
        {
            let tail = if previous_text.is_some() {
                text_tail
            } else {
                0.
            };
            push(
                &mut rows,
                format!("footer:{}", message.id),
                6. + tail,
                RowKind::Footer {
                    key: message.id.clone(),
                    text: match message.text_blocks.as_slice() {
                        [part] => part.text.clone(),
                        _ => message.text().into(),
                    },
                    duration: message.duration_since(turn.started_at),
                    interrupted: message.interrupted,
                },
            );
            previous_text = None;
        }
    }
    for row in &mut rows {
        if let RowKind::Content { block, .. } | RowKind::User(block) | RowKind::Status(block) =
            &row.kind
        {
            let dependency = ui.markdown.dependency_fingerprint(&block.id);
            row.presentation = dependency;
            if matches!(block.kind, BlockKind::Tool | BlockKind::Shell | BlockKind::Retry) {
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                dependency.hash(&mut hash);
                ui.tool_open
                    .contains(&format!("{session}:{}", block.id))
                    .hash(&mut hash);
                row.presentation = hash.finish();
            }
        }
    }
    project_activity_rows(rows, &activity, session, ui, busy)
}

fn tool_row(row: &Row) -> bool {
    matches!(&row.kind, RowKind::Content { block, .. }
        if matches!(block.kind, BlockKind::Tool | BlockKind::Shell))
}

fn work_row(row: &Row) -> bool {
    tool_row(row) || matches!(&row.kind,
        RowKind::Reasoning { .. } | RowKind::Content { reasoning: true, .. })
}

fn project_activity_rows(
    rows: Vec<Row>,
    activity: &vela_core::activity::ActivityProjection,
    session: &str,
    ui: &TranscriptUi,
    busy: bool,
) -> Vec<Row> {
    let tools = activity.tools.iter().map(|tool| (tool.id.as_ref(), tool)).collect::<HashMap<_, _>>();
    let mut compact = Vec::with_capacity(rows.len());
    let mut pending = rows.into_iter().peekable();
    while let Some(first) = pending.next() {
        if !work_row(&first) { compact.push(first); continue; }
        // Reasoning and tool phases share a single work sequence. Only actual
        // conversation content interrupts it; internal steps never add headings.
        let group_id = SharedString::from(format!("trace-line:{}", first.id));
        let open = ui.tool_groups.get(&group_id).copied().unwrap_or(false);
        let before = first.before;
        let mut members = vec![first];
        while let Some(row) = pending.next_if(|row| {
            work_row(row) || matches!(row.kind, RowKind::Footer { interrupted: false, .. })
        }) {
            members.push(row);
        }
        let has_tools = members.iter().any(tool_row);
        let mut categories = Vec::new();
        let mut count = 0;
        let mut running = 0;
        let mut failed = 0;
        let mut reasoning_running = false;
        for row in &mut members {
            match &row.kind {
                // A message's copy/timing footer can follow its tools. It is
                // internal to this work sequence, not another explanation.
                RowKind::Footer { .. } => row.hidden = true,
                RowKind::Reasoning { running, open: inspected, .. } => {
                    reasoning_running |= *running;
                    row.hidden = !*inspected && (!open || has_tools);
                }
                RowKind::Content { reasoning: true, .. } => {
                    // A body only exists after explicit reasoning disclosure.
                    row.hidden = false;
                }
                RowKind::Content { block, .. } => {
                    let Some(tool) = tools.get(block.id.as_ref()) else { continue; };
                    count += 1;
                    if !categories.contains(&tool.category) { categories.push(tool.category); }
                    running += usize::from(tool.running);
                    failed += usize::from(tool.failed);
                    row.hidden = !open && !tool.mandatory
                        && !ui.tool_open.contains(&format!("{session}:{}", block.id));
                    // Commands are details, not the default conversation text.
                    row.summary = (!open).then(|| tool.summary.clone());
                }
                _ => {}
            }
            row.before = if row.hidden { 0. } else { 4. };
        }
        let active = running > 0 || reasoning_running || (busy && pending.peek().is_none());
        let mut labels = categories.iter().map(|category| match category {
            vela_core::activity::Category::Exploring => if active { "Reading files" } else { "Read files" },
            vela_core::activity::Category::Changes => if active { "Editing files" } else { "Edited files" },
            vela_core::activity::Category::Verifying => if active { "Running checks" } else { "Ran checks" },
            vela_core::activity::Category::Browser => "Browser activity",
            vela_core::activity::Category::Other => if active { "Running commands" } else { "Ran commands" },
        }.to_owned()).collect::<Vec<_>>();
        for label in labels.iter_mut().skip(1) { label.make_ascii_lowercase(); }
        let title = if labels.is_empty() {
            if active { "Thinking" } else { "Thought" }.to_owned()
        } else { labels.join(", ") };
        let hidden = !open && !members.iter().any(|row| row.hidden && !matches!(row.kind, RowKind::Footer { .. }));
        compact.push(Row {
            id: group_id, before: if hidden { 0. } else { before }, after: 0.,
            presentation: 0, hidden, summary: None,
            kind: RowKind::ToolGroup { title: title.into(), count, running, failed, active, open },
        });
        if hidden && let Some(first) = members.iter_mut().find(|row| !row.hidden) { first.before = before; }
        compact.extend(members);
    }
    compact
}

fn retry_text(block: &Block) -> (String, String) {
    (block.text.to_string(), block.detail.to_string())
}

fn compaction_label(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::Running | ToolStatus::Streaming => "Compacting context…",
        ToolStatus::Failed => "Compaction failed",
        _ => "Context compacted",
    }
}

pub(super) fn format_answer_duration(ms: i64) -> String {
    let seconds = ms.max(0) / 1000;
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m {}s", seconds / 60, seconds % 60)
    }
}

fn answer_text(message: &vela_core::transcript::Message) -> String {
    use vela_core::transcript::PartKind;
    message
        .parts
        .iter()
        .filter_map(|part| match &part.kind {
            PartKind::Text { text, .. } if !text.trim().is_empty() => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

impl VelaApp {
    /// Route transcript navigation to the focused visible GUI panel. The
    /// active composer/session is independent from the panel being inspected.
    pub(super) fn route_transcript_key(
        &mut self,
        event: &KeyDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let panels_visible = self.agent_panel_open || (self.agent_mode && !self.panels.is_empty());
        if self.settings_open || !panels_visible {
            return false;
        }
        let active = self.active_id.as_ref().filter(|session| {
            self.agent_panel_mode(session) == AgentPanelMode::Gui
                && self.transcript_ui.contains_focused(window)
        }).cloned();
        let owner = active.or_else(|| {
            (self.panels.len() > 1).then(|| {
                self.panels.entries().iter().find_map(|panel| {
                    if self.active_id.as_deref() == Some(panel.session.as_str())
                        || self.agent_panel_mode(&panel.session) != AgentPanelMode::Gui
                    {
                        return None;
                    }
                    self.panel_transcripts.get(&panel.session)
                        .filter(|state| state.ui.contains_focused(window))
                        .map(|_| panel.session.clone())
                })
            }).flatten()
        });
        let Some(owner) = owner else { return false; };
        let Some(list) = self.list_for(&owner) else { return false; };
        let handled = self.transcript_ui_mut(&owner)
            .is_some_and(|ui| ui.navigate(event, &list, window));
        if handled {
            // Forward navigation can explicitly return to the live edge;
            // backward navigation always releases follow immediately.
            if matches!(event.keystroke.key.as_str(), "end" | "down" | "pagedown") {
                self.observe_conversation_scroll(&owner);
            } else {
                self.stop_conversation_follow(&owner);
            }
            cx.notify();
        }
        handled
    }

    /// Project selectable text for every user message once, including rows the
    /// virtualized transcript has not painted yet.
    pub(super) fn ensure_user_segment_texts(&mut self) {
        let theme = Theme::default();
        for block in self
            .blocks
            .iter()
            .filter(|block| block.kind == BlockKind::User)
        {
            if !self
                .transcript_ui
                .user_message_segments
                .contains_key(&block.id)
            {
                let projection = markdown_view::user_segment_texts_block_cached(
                    block,
                    theme.text,
                    &theme,
                    &self.transcript_ui.markdown,
                );
                self.transcript_ui
                    .user_message_segments
                    .insert(block.id.clone(), projection.texts);
                self.transcript_ui
                    .user_message_images
                    .insert(block.id.clone(), projection.images);
            }
        }
    }

    pub(super) fn ui_for_mut(&mut self, session: &str) -> Option<&mut TranscriptUi> {
        self.row_cache.invalidate_session(session);
        self.transcript_ui_mut(session)
    }

    fn transcript_ui_mut(&mut self, session: &str) -> Option<&mut TranscriptUi> {
        if self.active_id.as_deref() == Some(session) {
            Some(&mut self.transcript_ui)
        } else {
            self.panel_transcripts
                .get_mut(session)
                .map(|panel| &mut panel.ui)
        }
    }

    pub(super) fn list_for(&self, session: &str) -> Option<ListState> {
        if self.active_id.as_deref() == Some(session) {
            Some(self.transcript_list.clone())
        } else {
            self.panel_tile_lists.get(session).cloned()
        }
    }

    pub(super) fn rows_for(&self, session: &str) -> Arc<Vec<Row>> {
        if self.active_id.as_deref() == Some(session) {
            self.transcript_rows.clone()
        } else {
            self.panel_transcripts
                .get(session)
                .map(|panel| panel.rows.clone())
                .unwrap_or_default()
        }
    }

    pub(super) fn remeasure_conversation_row(&mut self, session: &str, row_key: &str) -> bool {
        self.row_cache.invalidate_session(session);
        let Some(list) = self.list_for(session) else { return false; };
        let rows = self.rows_for(session);
        let Some(index) = rows.iter().position(|row| row.id.as_ref() == row_key) else { return false; };
        if index >= list.item_count() {
            return false;
        }
        list.remeasure_items(index..index + 1);
        true
    }

    pub(super) fn rebuild_conversation_rows(&mut self, session: &str, cx: &mut Context<Self>) {
        if self.active_id.as_deref() != Some(session) {
            if let Some(panel) = self.panel_transcripts.get_mut(session) {
                panel.dirty = true;
            }
            cx.notify();
            return;
        }
        self.transcript_rows = Arc::new(project(
            &self.blocks,
            self.phase.is_active(),
            session,
            &self.transcript_ui,
        ));
        configure_user_order(&mut self.transcript_ui, &self.transcript_rows);
        self.transcript_viewport.sync(
            &self.transcript_list,
            self.transcript_rows
                .iter()
                .map(Row::revision)
                .collect(),
            self.stick,
        );
        cx.notify();
    }

    fn ui_for(&self, session: &str) -> Option<&TranscriptUi> {
        if self.active_id.as_deref() == Some(session) {
            Some(&self.transcript_ui)
        } else {
            self.panel_transcripts.get(session).map(|panel| &panel.ui)
        }
    }

    #[cfg(test)]
    pub(super) fn hovered_answer_for(&self, session: &str) -> Option<Arc<str>> {
        self.ui_for(session)
            .and_then(|ui| ui.hovered_answer.clone())
    }

    pub(super) fn user_clipboard_payload_for(
        &self,
        session: &str,
    ) -> Option<(String, Vec<Arc<gpui::Image>>)> {
        let ui = self.ui_for(session)?;
        let selection = ui.user_text_selection.as_ref()?;
        let text = selection.selected_text(&ui.user_message_segments);
        let images = selection.selected_images(&ui.user_message_segments, &ui.user_message_images);
        (!text.is_empty() || !images.is_empty()).then_some((text, images))
    }

    #[cfg(test)]
    pub(super) fn user_selection_text_for(&self, session: &str) -> String {
        self.user_clipboard_payload_for(session)
            .map(|(text, _)| text)
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(super) fn user_clipboard_payload(&self) -> Option<(String, Vec<Arc<gpui::Image>>)> {
        self.user_clipboard_payload_for(self.active_id.as_deref()?)
    }

    fn copy_user_text_selection_for(&mut self, session: &str, cx: &mut Context<Self>) {
        let Some((text, images)) = self.user_clipboard_payload_for(session) else {
            return;
        };
        super::selection_clipboard::write(cx, &text, &images);
        cx.notify();
    }

    fn select_all_user_text_for(&mut self, session: &str, cx: &mut Context<Self>) {
        let blocks = if self.active_id.as_deref() == Some(session) {
            self.blocks.clone()
        } else {
            self.panel_transcripts
                .get(session)
                .map(|panel| panel.blocks.clone())
                .unwrap_or_default()
        };
        self.stop_conversation_follow(session);
        let Some(ui) = self.ui_for_mut(session) else {
            return;
        };
        let theme = Theme::default();
        for block in blocks.iter().filter(|block| block.kind == BlockKind::User) {
            if !ui.user_message_segments.contains_key(&block.id) {
                let projection = markdown_view::user_segment_texts_block_cached(
                    block,
                    theme.text,
                    &theme,
                    &ui.markdown,
                );
                ui.user_message_segments
                    .insert(block.id.clone(), projection.texts);
                ui.user_message_images
                    .insert(block.id.clone(), projection.images);
            }
        }
        let Some((anchor, head)) = all_user_text_points(ui) else {
            return;
        };
        ui.user_text_selection = Some(UserTextSelection {
            anchor,
            head,
            order: ui.user_message_order.clone(),
        });
        cx.notify();
    }

    pub(super) fn stop_conversation_follow(&mut self, session: &str) {
        self.cancel_conversation_scroll_restore(session);
        self.turn_anchors.cancel(session);
        if self.active_id.as_deref() == Some(session) {
            self.stick = false;
        } else {
            self.panels.update(session, |panel| panel.stick = false);
        }
        if let Some(list) = self.list_for(session) {
            list.set_follow_mode(gpui::FollowMode::Normal);
        }
        if let Some(ui) = self.transcript_ui_mut(session) {
            ui.scroll_generation += 1;
            ui.scrolling_to_bottom = false;
            ui.cancel_wheel_scroll();
        }
    }

    /// Apply manual scroll intent after GPUI has updated its native geometry.
    /// Only this input path (or explicit latest navigation) can re-arm follow;
    /// growing Markdown, images and completed tools cannot do so.
    pub(super) fn observe_conversation_scroll(&mut self, session: &str) {
        let Some(list) = self.list_for(session) else { return; };
        let padding = self.conversation_scroll_padding(session);
        let at_edge = crate::scroll::list_at_live_edge(&list, padding);
        if at_edge && list.is_following_tail() {
            self.cancel_conversation_scroll_restore(session);
            self.turn_anchors.cancel(session);
            return;
        }
        self.stop_conversation_follow(session);
        if at_edge {
            if self.active_id.as_deref() == Some(session) {
                self.stick = true;
            } else {
                self.panels.update(session, |panel| panel.stick = true);
            }
            list.set_follow_mode(gpui::FollowMode::Tail);
        }
    }

    fn transcript_top_padding(&self) -> Pixels {
        px(if self.is_codex() { 16. } else { 48. })
    }

    pub(super) fn focused_transcript_padding(&self) -> Pixels {
        self.transcript_top_padding() + TRANSCRIPT_BOTTOM_PADDING
    }

    fn conversation_scroll_padding(&self, session: &str) -> Pixels {
        if self.active_id.as_deref() == Some(session) {
            self.focused_transcript_padding()
        } else {
            px(0.)
        }
    }

    pub(super) fn observe_conversation_wheel(
        &self,
        list: &ListState,
        session: String,
        cx: &mut Context<Self>,
    ) -> crate::scroll::TranscriptWheelInput {
        let input = self.ui_for(&session).expect("transcript UI").wheel_input.clone();
        let captured = input.clone();
        let owner = cx.weak_entity();
        list.set_scroll_handler(move |event, window, cx| {
            let owner = owner.clone();
            let session = session.clone();
            match captured.claim() {
                crate::scroll::WheelInput::Tick(tick) => {
                    window.defer(cx, move |window, cx| {
                        let _ = owner.update(cx, |app, cx| {
                            app.queue_transcript_wheel(&session, tick, window, cx);
                        });
                    });
                }
                crate::scroll::WheelInput::Native => {
                    crate::scroll::observe_list_scroll(event, cx, move |_, cx| {
                        let _ = owner.update(cx, |app, _| {
                            app.observe_conversation_scroll(&session);
                        });
                    });
                }
                crate::scroll::WheelInput::Queued => {}
            }
        });
        input
    }

    fn queue_transcript_wheel(
        &mut self,
        session: &str,
        tick: crate::scroll::WheelTick,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(list) = self.list_for(session) else { return; };
        // Roll back only the discrete list movement before the window paints.
        // The native wheel path also dispatches to scrollable tool/code rows.
        list.scroll_to(tick.anchor);
        if !self.ui_for(session).is_some_and(|ui| ui.wheel_motion.is_running()) {
            self.stop_conversation_follow(session);
        }
        let now = cx.background_executor().now();
        if let Some(token) = self
            .transcript_ui_mut(session)
            .and_then(|ui| ui.wheel_motion.queue(tick.distance, now))
        {
            Self::animate_transcript_wheel(cx.weak_entity(), session.to_owned(), token, window);
        }
    }

    fn animate_transcript_wheel(
        owner: gpui::WeakEntity<Self>,
        session: String,
        token: Rc<()>,
        window: &Window,
    ) {
        window.on_next_frame(move |window, cx| {
            let _ = owner.update(cx, |app, cx| {
                let Some(list) = app.list_for(&session) else { return; };
                let now = cx.background_executor().now();
                let Some(step) = app.transcript_ui_mut(&session)
                    .and_then(|ui| ui.wheel_motion.advance(&token, now))
                else { return; };
                let padding = app.conversation_scroll_padding(&session);
                let down = crate::scroll::list_distance_from_bottom(&list, padding);
                let up = (-list.scroll_px_offset_for_scrollbar().y).max(px(0.));
                list.scroll_by(step.clamp(-up, down));
                if step > px(0.) && step >= down {
                    app.observe_conversation_scroll(&session);
                } else if step < px(0.) && -step >= up
                    && let Some(ui) = app.transcript_ui_mut(&session)
                {
                    ui.cancel_wheel_scroll();
                }
                if app.ui_for(&session).is_some_and(|ui| ui.wheel_motion.is_running()) {
                    Self::animate_transcript_wheel(owner.clone(), session, token, window);
                }
                cx.notify();
            });
        });
    }

    pub(super) fn render_native_transcript(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.transcript_ui.user_text_session.as_deref() != self.active_id.as_deref() {
            self.transcript_ui.user_text_selection = None;
            self.transcript_ui.user_message_segments.clear();
            self.transcript_ui.user_message_images.clear();
            self.transcript_ui.user_text_session = self.active_id.clone();
        }
        self.transcript_ui.markdown.keyboard = self.picker_anchors.keyboard.clone();
        configure_user_order(&mut self.transcript_ui, &self.transcript_rows);
        let rows = self.transcript_rows.clone();
        let top_padding = self.transcript_top_padding();
        let owner = cx.weak_entity();
        let scroll_owner = cx.weak_entity();
        let natural_tail = self.transcript_viewport.requires_natural_tail();
        let scroll_session = self.active_id.clone();
        let list_state = self.transcript_list.clone();
        self.transcript_ui.markdown.set_viewport_width(list_state.viewport_bounds().size.width);
        let wheel_input = if let Some(session) = scroll_session.clone() {
            self.restore_measured_conversation_scroll(&session, &list_state);
            self.row_cache.capture_measurements(&session, &list_state, &self.transcript_rows, window);
            self.observe_conversation_wheel(&list_state, session, cx)
        } else {
            Default::default()
        };
        self.transcript_viewport.set_bottom_padding(&list_state, TRANSCRIPT_BOTTOM_PADDING);
        let viewport = crate::scroll::ListViewport::new(
            self.transcript_ui.viewport.clone(),
            list_state.clone(),
            move |gutter| {
                let transcript = list(list_state.clone(), move |index, window, cx| {
                    let Some(row) = rows.get(index) else {
                        return div().into_any_element();
                    };
                    owner
                        .update(cx, |app, cx| {
                            if row.cacheable() {
                                let session = app.active_id.clone().unwrap_or_default();
                                app.row_cache.element(
                                    cx.weak_entity(),
                                    &session,
                                    row,
                                    index,
                                    natural_tail && index + 1 == rows.len(),
                                    cx,
                                )
                            } else {
                                app.render_transcript_row(row, window, cx)
                            }
                        })
                        .unwrap_or_else(|_| div().into_any_element())
                })
                .size_full()
                .w_full()
                // The Codex shell puts the work row in flow; only the
                // classic shell reserves room for its absolute header.
                .pt(top_padding)
                .pb(TRANSCRIPT_BOTTOM_PADDING);
                div()
                    .size_full()
                    // Center the transcript on the same axis as the composer.
                    .px(gutter)
                    .child(wheel_input.wrap(list_state, transcript.into_any_element()))
                    .into_any_element()
            },
        );
        let viewport = viewport.with_padding(self.focused_transcript_padding()).on_scroll(move |_, _, cx| {
            let _ = scroll_owner.update(cx, |app, cx| {
                if let Some(session) = scroll_session.as_deref() && app.list_for(session).is_some() {
                    app.observe_conversation_scroll(session);
                    cx.notify();
                }
            });
        });
        let show_bottom = !self.transcript_ui.scrolling_to_bottom
            && crate::scroll::list_distance_from_bottom(&self.transcript_list, self.focused_transcript_padding()) > px(24.);
        let button = if show_bottom {
            let row = Row {
                id: "scroll-to-bottom".into(),
                before: 0.,
                after: 0.,
                presentation: 0,
                hidden: false,
                summary: None,
                kind: RowKind::ScrollControl,
            };
            let (button, _) = self.active_transcript_button(
                &row,
                "Scroll to bottom".into(),
                "",
                window,
                cx,
                |app, window, cx| app.scroll_transcript_to_bottom(window, cx),
            );
            Some(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(px(12.))
                    .flex()
                    .justify_center()
                    .child(
                        button
                            .size(px(32.))
                            .rounded_full()
                            .border_1()
                            .border_color(Theme::default().border_soft)
                            .bg(Theme::default().panel)
                            .shadow(Theme::default().small_shadow())
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(glyph(
                                "arrow-down",
                                16.,
                                Theme::default().text.opacity(0.62),
                            )),
                    ),
            )
        } else {
            None
        };
        div()
            .relative()
            .flex_1()
            .w_full()
            .min_h_0()
            .font(crate::theme::agent_font())
            .child(viewport)
            .children(button)
            .into_any_element()
    }

    pub(super) fn scroll_transcript_to_bottom(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some(session) = self.active_id.clone() else { return; };
        self.stop_conversation_follow(&session);
        self.transcript_viewport.release_turn_anchor(&self.transcript_list);
        let distance = crate::scroll::list_distance_from_bottom(
            &self.transcript_list, self.focused_transcript_padding(),
        );
        if self.phase.is_active() || cx.reduce_motion() || distance <= px(24.) {
            self.stick = true;
            self.transcript_list.set_follow_mode(gpui::FollowMode::Tail);
        } else {
            self.transcript_ui.scrolling_to_bottom = true;
            self.animate_transcript_bottom(
                window, cx, session, self.transcript_ui.scroll_generation,
                std::time::Instant::now(), 1.,
            );
        }
        cx.notify();
    }

    fn animate_transcript_bottom(
        &self,
        window: &Window,
        cx: &Context<Self>,
        session: String,
        generation: u64,
        started: std::time::Instant,
        previous_remaining: f32,
    ) {
        let owner = cx.weak_entity();
        window.on_next_frame(move |window, cx| {
            let _ = owner.update(cx, |app, cx| {
                if app.active_id.as_deref() != Some(&session)
                    || app.transcript_ui.scroll_generation != generation
                {
                    return;
                }
                let progress = (started.elapsed().as_secs_f32() / 0.260).min(1.);
                let remaining = (1. - progress).powi(3);
                if remaining == 0. {
                    app.stick = true;
                    app.transcript_list.set_follow_mode(gpui::FollowMode::Tail);
                    app.transcript_ui.scrolling_to_bottom = false;
                } else {
                    let distance = crate::scroll::list_distance_from_bottom(
                        &app.transcript_list, app.focused_transcript_padding(),
                    );
                    // Advance from the current row anchor. Discovering heights
                    // above it must not alter the animation's next step.
                    app.transcript_list.scroll_by(distance * (1. - remaining / previous_remaining));
                    app.animate_transcript_bottom(window, cx, session, generation, started, remaining);
                }
                cx.notify();
            });
        });
    }

    fn active_transcript_button(
        &mut self,
        row: &Row,
        label: SharedString,
        suffix: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: impl Fn(&mut VelaApp, &mut Window, &mut Context<VelaApp>) + 'static,
    ) -> (gpui::Stateful<gpui::Div>, gpui::FocusHandle) {
        let session = self.active_id.clone().unwrap_or_default();
        let mut ui = std::mem::take(&mut self.transcript_ui);
        let button =
            self.transcript_button(&session, &mut ui, row, label, suffix, window, cx, action);
        self.transcript_ui = ui;
        button
    }

    #[allow(clippy::too_many_arguments)]
    fn transcript_button(
        &mut self,
        session: &str,
        ui: &mut TranscriptUi,
        row: &Row,
        label: SharedString,
        suffix: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: impl Fn(&mut VelaApp, &mut Window, &mut Context<VelaApp>) + 'static,
    ) -> (gpui::Stateful<gpui::Div>, gpui::FocusHandle) {
        let key: SharedString = if suffix.is_empty() {
            row.id.clone()
        } else {
            format!("{}:{suffix}", row.id).into()
        };
        let key: SharedString = format!("{session}:{key}").into();
        let row_key = row.id.clone();
        let target_session = session.to_owned();
        let focus = if let Some(focus) = ui.focus.get(&key) {
            focus.clone()
        } else {
            let focus = cx.focus_handle().tab_stop(true);
            cx.on_focus(&focus, window, move |app, _, cx| {
                app.row_cache.invalidate_session(&target_session);
                if let Some(index) = app
                    .rows_for(&target_session)
                    .iter()
                    .position(|row| row.id == row_key)
                    && let Some(list) = app.list_for(&target_session)
                {
                    list.scroll_to_reveal_item(index);
                }
                cx.notify();
            })
            .detach();
            let blur_session = session.to_owned();
            cx.on_blur(&focus, window, move |app, _, cx| {
                app.row_cache.invalidate_session(&blur_session);
                cx.notify();
            }).detach();
            ui.focus.insert(key.clone(), focus.clone());
            focus
        };
        let clicked = action;
        let clicked_session = session.to_owned();
        let mouse_focus = focus.clone();
        let click_focus = focus.clone();
        let control = div()
            .id(key)
            .role(gpui::Role::Button)
            .aria_label(label)
            .track_focus(&focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, window, cx| window.focus(&mouse_focus, cx)),
            )
            .on_click(cx.listener(move |app, _, window, cx| {
                cx.stop_propagation();
                app.stop_conversation_follow(&clicked_session);
                window.focus(&click_focus, cx);
                clicked(app, window, cx);
            }));
        let theme = Theme::default();
        let control = crate::focus::outline(
            control,
            &focus,
            6.,
            1.,
            0.,
            theme.accent.into(),
            &ui.markdown.keyboard,
        );
        (control, focus)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_delegated_card(
        &mut self,
        session: &str,
        ui: &mut TranscriptUi,
        row: &Row,
        block_id: &str,
        card: crate::subagent::Card,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::default();
        let secondary = theme.text.opacity(0.62);
        let tertiary = theme.text.opacity(0.46);
        let icon_color = if card.tone == crate::subagent::Tone::Failed {
            theme.red
        } else {
            tertiary
        };
        let status = match card.tone {
            crate::subagent::Tone::Working => "working",
            crate::subagent::Tone::Complete => "finished",
            crate::subagent::Tone::Failed => "failed",
        };
        let selector = format!("subagent-summary:{block_id}");
        let body = div()
            .debug_selector(move || selector.clone())
            .flex_1().min_w_0().flex().items_center().gap(px(8.))
            .h(px(21.)).text_size(px(12.)).line_height(px(18.))
            .text_color(secondary)
            .child(glyph("robot", 13., icon_color))
            .child(div().flex_shrink_0().child(format!("{} {status}", card.agent)))
            .when(!card.description.is_empty(), |body| body.child(
                div().flex_1().min_w_0().truncate().text_color(tertiary)
                    .child(card.description.clone()),
            ));
        if card.child_id.is_empty() {
            return div()
                .debug_selector(|| "subagent-card".into())
                .w_full()
                .max_w(px(560.))
                .px(px(6.))
                .py(px(3.))
                .child(body)
                .into_any_element();
        }
        let open_id = card.child_id.clone();
        let label = format!("Open delegated agent session: {}", card.title);
        let (button, _) = self.transcript_button(
            session,
            ui,
            row,
            label.into(),
            "subagent",
            window,
            cx,
            move |app, _, cx| app.reopen_session(open_id.clone(), cx),
        );
        button
            .debug_selector(|| "subagent-link".into())
            .w_full()
            .max_w(px(560.))
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(6.))
            .py(px(3.))
            .rounded(px(6.))
            .cursor_pointer()
            .hover(|style| style.bg(theme.elevated))
            .child(body)
            .child(glyph("arrow-right", 13., tertiary))
            .into_any_element()
    }

    fn render_transcript_row(
        &mut self,
        row: &Row,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let session = self.active_id.clone().unwrap_or_default();
        let list = self.transcript_list.clone();
        let mut ui = std::mem::take(&mut self.transcript_ui);
        let element = self.render_conversation_row(&session, row, list, &mut ui, window, cx);
        self.transcript_ui = ui;
        element
    }

    pub(super) fn render_panel_transcript_row(
        &mut self,
        session: &str,
        row: &Row,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(list) = self.list_for(session) else {
            return div().into_any_element();
        };
        if self.active_id.as_deref() == Some(session) {
            return self.render_transcript_row(row, window, cx);
        }
        let Some(panel) = self.panel_transcripts.get_mut(session) else {
            return div().into_any_element();
        };
        let mut ui = std::mem::take(&mut panel.ui);
        let element = self.render_conversation_row(session, row, list, &mut ui, window, cx);
        if let Some(panel) = self.panel_transcripts.get_mut(session) {
            panel.ui = ui;
        }
        element
    }

    fn render_conversation_row(
        &mut self,
        session: &str,
        row: &Row,
        list: ListState,
        ui: &mut TranscriptUi,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if row.hidden {
            return div()
                .id(SharedString::from(format!("{session}:row:{}", row.id)))
                .w_full().h(px(0.)).overflow_hidden().into_any_element();
        }
        let theme = Theme::default();
        let secondary = theme.text.opacity(0.62);
        let tertiary = theme.text.opacity(0.46);
        ui.markdown.attach_row(row.id.clone());
        ui.markdown.attach_session(session);
        let mut answer_hover = None;
        let content = match &row.kind {
            RowKind::Content {
                block,
                answer,
                reasoning,
            } => {
                answer_hover = answer.clone();
                let block = block.clone();
                div()
                    .w_full()
                    .min_w_0()
                    .when(*reasoning, |body| body.pl(px(28.)).pr(px(6.)))
                    .child(
                        if matches!(block.kind, BlockKind::Tool | BlockKind::Shell) {
                            let card = (block.kind == BlockKind::Tool)
                                .then(|| {
                                    crate::subagent::tool_card(
                                        block.child_session_id.as_deref().unwrap_or(""),
                                        &block.detail,
                                        &block.text,
                                        block.status,
                                        &self.sessions,
                                        &self.agents,
                                        session,
                                    )
                                })
                                .flatten();
                            if let Some(card) = card {
                                self.render_delegated_card(
                                    session, ui, row, &block.id, card, window, cx,
                                )
                            } else {
                                let root = self.sessions.iter()
                                    .find(|entry| entry.id == session)
                                    .map(|entry| entry.directory.as_str());
                                let mut model = tool_view::presentation(&block, root);
                                if let Some(summary) = &row.summary { model.summary = summary.to_string(); }
                                let expandable = model.expandable();
                                let key = format!("{}:{}", session, block.id);
                                let open = ui.tool_open.contains(&key);
                                let target_session = session.to_owned();
                                let (button, focus) = if expandable {
                                    let (button, focus) = self.transcript_button(
                                        session,
                                        ui,
                                        row,
                                        model.summary.clone().into(),
                                        "",
                                        window,
                                        cx,
                                        move |app, _, cx| {
                                            if let Some(ui) = app.ui_for_mut(&target_session)
                                                && !ui.tool_open.remove(&key)
                                            {
                                                ui.tool_open.insert(key.clone());
                                            }
                                            app.rebuild_conversation_rows(&target_session, cx);
                                        },
                                    );
                                    (button, Some(focus))
                                } else {
                                    (
                                        div()
                                            .id(SharedString::from(format!(
                                                "tool-head-static:{}",
                                                row.id
                                            )))
                                            .role(gpui::Role::Label)
                                            .aria_label(model.summary.clone()),
                                        None,
                                    )
                                };
                                let tool = tool_view::render(
                                    &block,
                                    model,
                                    button,
                                    tool_view::RenderState {
                                        open,
                                        focused: focus
                                            .as_ref()
                                            .is_some_and(|focus| focus.is_focused(window)),
                                    },
                                    &theme,
                                    cx,
                                );
                                div()
                                    .min_w_0()
                                    .when(row.summary.is_none(), |row| row.pl(px(18.)))
                                    .child(tool)
                                    .into_any_element()
                            }
                        } else {
                            markdown_view::block(
                                &block,
                                if *reasoning { secondary } else { theme.text },
                                &theme,
                                markdown_view::BlockContext {
                                    ui: &mut ui.markdown,
                                    list: list.clone(),
                                    answer: answer.clone(),
                                },
                                window,
                                cx,
                            )
                        },
                    )
                    .into_any_element()
            }
            RowKind::User(block) => {
                if !ui.user_message_segments.contains_key(&block.id) {
                    let projection = markdown_view::user_segment_texts_block_cached(
                        block,
                        theme.text,
                        &theme,
                        &ui.markdown,
                    );
                    ui.user_message_segments
                        .insert(block.id.clone(), projection.texts);
                    ui.user_message_images
                        .insert(block.id.clone(), projection.images);
                }
                let segments: Rc<RefCell<Vec<SharedString>>> = Rc::new(RefCell::new(Vec::new()));
                let drag = ui.user_text_drag.clone();
                let message = ui.user_message_index.get(&block.id).copied().unwrap_or(0);
                let selection_focus = ui.user_text_focus(session, &block.id, row, window, cx);
                let current = ui
                    .user_text_selection
                    .as_ref()
                    .map(|selection| (selection.anchor, selection.head));
                let owner = cx.weak_entity();
                let focus = selection_focus.clone();
                let selection_session = session.to_owned();
                let on_change: markdown_view::UserSelectionHandler =
                    Rc::new(move |change, window, cx| match change {
                        markdown_view::UserSelectionChange::Start {
                            point,
                            text,
                            click_count,
                            shift,
                        } => {
                            window.focus(&focus, cx);
                            let _ = owner.update(cx, |app, cx| {
                                app.stop_conversation_follow(&selection_session);
                                let Some(ui) = app.ui_for_mut(&selection_session) else {
                                    return;
                                };
                                let existing_anchor =
                                    current.filter(|_| shift).map(|(anchor, _)| anchor);
                                let (anchor, head) = if click_count >= 3 {
                                    message_text_points(ui, point.message).unwrap_or((point, point))
                                } else if click_count == 2 {
                                    let word = selected_word(&text, point.offset);
                                    let start = markdown_view::UserSelectionPoint {
                                        message: point.message,
                                        segment: point.segment,
                                        offset: word.start,
                                    };
                                    let end = markdown_view::UserSelectionPoint {
                                        message: point.message,
                                        segment: point.segment,
                                        offset: word.end,
                                    };
                                    (existing_anchor.unwrap_or(start), end)
                                } else {
                                    (existing_anchor.unwrap_or(point), point)
                                };
                                ui.user_text_selection = Some(UserTextSelection {
                                    anchor,
                                    head,
                                    order: ui.user_message_order.clone(),
                                });
                                cx.notify();
                            });
                        }
                        markdown_view::UserSelectionChange::Extend { point } => {
                            let _ = owner.update(cx, |app, cx| {
                                if let Some(selection) = app
                                    .ui_for_mut(&selection_session)
                                    .and_then(|ui| ui.user_text_selection.as_mut())
                                {
                                    selection.head = point;
                                    cx.notify();
                                }
                            });
                        }
                        markdown_view::UserSelectionChange::End => {}
                    });
                let selection_spec = markdown_view::UserSelectionSpec {
                    message,
                    anchor: current.map(|(anchor, _)| anchor),
                    head: current.map(|(_, head)| head),
                    segments: segments.clone(),
                    drag,
                    fill: gpui::Rgba {
                        a: 0.32,
                        ..theme.accent
                    },
                    on_change,
                };
                let text_row = div()
                    .id(SharedString::from(format!(
                        "{session}:user-text:{}",
                        block.id
                    )))
                    .w_full()
                    .flex()
                    .justify_end()
                    .min_w_0()
                    .track_focus(&selection_focus);
                let user_content = markdown_view::user_block(
                    block,
                    &block.id,
                    &theme,
                    &mut ui.markdown,
                    window,
                    cx,
                    Some(selection_spec),
                );
                let copy_session = session.to_owned();
                let select_session = session.to_owned();
                let row = div()
                    .id(SharedString::from(format!("{session}:user-actions:{}", block.id)))
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_end()
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .on_action(cx.listener(move |app, _: &crate::input::Copy, _, cx| {
                        app.copy_user_text_selection_for(&copy_session, cx);
                    }))
                    .on_action(cx.listener(move |app, _: &crate::input::SelectAll, _, cx| {
                        app.select_all_user_text_for(&select_session, cx);
                    }));
                row.child(text_row.child(user_content)).into_any_element()
            }
            RowKind::ScrollControl => div().into_any_element(),
            RowKind::ToolGroup { title, count, running, failed, active, open } => {
                let status = if *running > 0 {
                    format!("{running} running")
                } else if *failed > 0 {
                    format!("{failed} failed")
                } else if *active {
                    "working".to_owned()
                } else {
                    format!("{count} completed")
                };
                let session = session.to_owned();
                let key = row.id.clone();
                let expanded = *open;
                let (button, _) = self.transcript_button(
                    &session,
                    ui,
                    row,
                    format!("{title}, {status}").into(),
                    "",
                    window,
                    cx,
                    {
                        let session = session.clone();
                        move |app, _, cx| {
                            if let Some(ui) = app.ui_for_mut(&session) {
                                ui.tool_groups.insert(key.clone(), !expanded);
                            }
                            app.rebuild_conversation_rows(&session, cx);
                        }
                    },
                );
                button
                    .debug_selector(|| "tool-group-trigger".into())
                    .aria_expanded(expanded)
                    .w_full()
                    .px(px(6.))
                    .h(px(27.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(secondary)
                    .hover(|style| style.bg(theme.text.opacity(0.05)))
                    .cursor_pointer()
                    .child(glyph(if expanded { "chevron-down" } else { "chevron-right" }, 12., secondary))
                    .child(work_summary_caption::render(title, *active && !cx.reduce_motion()))
                    .into_any_element()
            }
            RowKind::Reasoning { key, running, open } => {
                let target = key.clone();
                let was_open = *open;
                let label = if *running { "Thinking" } else { "Thought" };
                let target_session = session.to_owned();
                let (button, _) = self.transcript_button(
                    session,
                    ui,
                    row,
                    label.into(),
                    "",
                    window,
                    cx,
                    move |app, _, cx| {
                        if let Some(ui) = app.ui_for_mut(&target_session) {
                            ui.reasoning_open.insert(target.clone(), !was_open);
                        }
                        app.rebuild_conversation_rows(&target_session, cx);
                    },
                );
                let reasoning_selector = format!("reasoning-trigger:{key}");
                button
                    .debug_selector(move || reasoning_selector.clone())
                    .aria_expanded(*open)
                    .w_full()
                    .px(px(6.))
                    .pt(px(6.))
                    .pb(px(2.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(14.))
                    .line_height(px(17.))
                    .text_color(secondary)
                    .hover(|style| style.bg(theme.text.opacity(0.05)))
                    .cursor_pointer()
                    .child(glyph("reasoning", 14., secondary).opacity(0.7))
                    .child(div().text_color(theme.text).child(label))
                    .when(*running && !cx.reduce_motion(), |button| {
                        use gpui::AnimationExt;
                        button.child(glyph("working-spinner", 11., secondary).with_animation(
                            SharedString::from(format!("{session}:spin:{}", row.id)),
                            gpui::Animation::new(Duration::from_millis(700)).repeat(),
                            |svg, progress| {
                                svg.with_transformation(gpui::Transformation::rotate(
                                    gpui::percentage(progress),
                                ))
                            },
                        ))
                    })
                    .child(
                        glyph(
                            if *open {
                                "chevron-down"
                            } else {
                                "chevron-right"
                            },
                            12.,
                            secondary,
                        )
                        .opacity(if *open { 1. } else { 0. }),
                    )
                    .into_any_element()
            }
            RowKind::Footer {
                key,
                text,
                duration: elapsed,
                interrupted,
            } => {
                answer_hover = Some(key.clone());
                let copied = ui.copied_answer.as_ref() == Some(key);
                let label = if copied { "Copied" } else { "Copy" };
                let copy_key = key.clone();
                let copy_text = text.clone();
                let copy_session = session.to_owned();
                let (button, focus) = self.transcript_button(
                    session,
                    ui,
                    row,
                    label.into(),
                    "",
                    window,
                    cx,
                    move |app, _, cx| {
                        let text = app
                            .engine
                            .view(&copy_session)
                            .and_then(|view| {
                                view.with_transcript(|transcript| {
                                    transcript.message(&copy_key).map(answer_text)
                                })
                            })
                            .unwrap_or_else(|| copy_text.to_string());
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                        let Some(ui) = app.ui_for_mut(&copy_session) else { return; };
                        ui.copied_answer = Some(copy_key.clone());
                        ui.answer_copy_generation = ui.answer_copy_generation.wrapping_add(1);
                        let generation = ui.answer_copy_generation;
                        let copy_owner = ui.answer_copy_owner.clone();
                        let key = copy_key.clone();
                        let timer_session = copy_session.clone();
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_secs(2)).await;
                            let _ = this.update(cx, |app, cx| {
                                if let Some(ui) = app.ui_for_mut(&timer_session)
                                    && ui.answer_copy_is_current(&copy_owner, generation, &key)
                                {
                                    ui.copied_answer = None;
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                        cx.notify();
                    },
                );
                let visible = ui.answer_footer_visible(key, &focus, window);
                let copy_selector = format!("answer-copy:{session}:{key}");
                let controls = div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .opacity(if visible { 1. } else { 0. })
                    .child(
                        button
                            .debug_selector(move || copy_selector.clone())
                            .px(px(6.))
                            .py(px(2.))
                            .rounded(px(6.))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(secondary)
                            .cursor_pointer()
                            .hover(|style| {
                                style.bg(theme.text.opacity(0.05)).text_color(theme.text)
                            })
                            .child(label),
                    )
                    .when_some(elapsed.map(format_answer_duration), |footer, meta| {
                        footer.child(
                            div()
                                .id(SharedString::from(format!("{session}:answer-meta:{key}")))
                                .debug_selector(|| "answer-meta".into())
                                .role(gpui::Role::Label)
                                .text_size(px(12.))
                                .text_color(tertiary)
                                .child(SharedString::from(meta)),
                        )
                    });
                div()
                    .w_full()
                    .min_h(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .when(*interrupted, |footer| {
                        footer.child(
                            div()
                                .debug_selector(|| "answer-stopped".into())
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(tertiary)
                                .child("Stopped"),
                        )
                    })
                    .child(controls)
                    .into_any_element()
            }
            RowKind::Status(block) => {
                let card = if block.kind == BlockKind::System {
                    crate::subagent::parse(&block.text).and_then(|reference| {
                        crate::subagent::card(&reference, &self.sessions, &self.agents, session)
                    })
                } else {
                    None
                };
                if let Some(card) = card {
                    self.render_delegated_card(session, ui, row, &block.id, card, window, cx)
                } else if block.kind == BlockKind::Retry {
                    let (message, info) = retry_text(block);
                    let label = if info.is_empty() {
                        message.clone()
                    } else {
                        format!("{message} · {info}")
                    };
                    if !block.output.is_empty() {
                        let key = format!("{session}:{}", block.id);
                        let expanded = ui.tool_open.contains(&key);
                        let target_session = session.to_owned();
                        let (button, _) = self.transcript_button(
                            session, ui, row, label.clone().into(), "retry", window, cx,
                            move |app, _, cx| {
                                if let Some(ui) = app.ui_for_mut(&target_session)
                                    && !ui.tool_open.remove(&key) { ui.tool_open.insert(key.clone()); }
                                app.rebuild_conversation_rows(&target_session, cx);
                            },
                        );
                        div().w_full().min_w_0().flex().flex_col()
                            .child(button.debug_selector(|| "retry-notice".into())
                                .aria_expanded(expanded).py(px(2.)).flex().items_center().gap(px(6.))
                                .text_size(px(12.)).line_height(px(18.)).text_color(secondary)
                                .cursor_pointer()
                                .child(glyph(if expanded { "chevron-down" } else { "chevron-right" }, 12., secondary))
                                .child(label))
                            .when(expanded, |body| body.child(tool_view::card(block, &theme, false)))
                            .into_any_element()
                    } else {
                    div()
                        .id(SharedString::from(format!("{session}:retry:{}", block.id)))
                        .debug_selector(|| "retry-notice".into())
                        .w_full()
                        .min_w_0()
                        .py(px(2.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(secondary)
                        .whitespace_normal()
                        .role(gpui::Role::Label)
                        .aria_label(SharedString::from(label.clone()))
                        .child(label)
                        .into_any_element()
                    }
                } else if block.kind == BlockKind::Compaction {
                    let failed = block.status == ToolStatus::Failed;
                    div()
                        .id(SharedString::from(format!(
                            "{session}:compaction:{}",
                            block.id
                        )))
                        .debug_selector(|| "compaction-notice".into())
                        .w_full()
                        .min_w_0()
                        .py(px(2.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(if failed { theme.red } else { secondary })
                        .role(gpui::Role::Label)
                        .aria_label(compaction_label(block.status))
                        .child(compaction_label(block.status))
                        .into_any_element()
                } else if block.kind == BlockKind::Skill {
                    let name = if block.output.trim().is_empty() {
                        "Skill"
                    } else {
                        block.output.as_ref()
                    };
                    let mut notice = div()
                        .id(SharedString::from(format!("{session}:skill:{}", block.id)))
                        .debug_selector(|| "skill-notice".into())
                        .w_full()
                        .min_w_0()
                        .py(px(6.))
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .debug_selector(|| "skill-label".into())
                                .text_size(px(11.5))
                                .line_height(px(16.))
                                .text_color(tertiary)
                                .child(format!("SKILL · {name}")),
                        );
                    if !block.text.trim().is_empty() {
                        notice = notice.child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(secondary)
                                .child(markdown_view::block(
                                    block,
                                    secondary,
                                    &theme,
                                    markdown_view::BlockContext {
                                        ui: &mut ui.markdown,
                                        list: list.clone(),
                                        answer: None,
                                    },
                                    window,
                                    cx,
                                )),
                        );
                    }
                    notice.into_any_element()
                } else {
                    let error =
                        block.kind == BlockKind::Error || block.status == ToolStatus::Failed;
                    let text = block.text.to_string();
                    div()
                        .id(SharedString::from(format!(
                            "{session}:status-label:{}",
                            block.id
                        )))
                        .w_full()
                        .min_w_0()
                        .when(block.kind != BlockKind::Error, |note| note.py(px(2.)))
                        .text_size(px(if block.kind == BlockKind::Error {
                            12.5
                        } else {
                            12.
                        }))
                        .line_height(px(18.))
                        .text_color(if error { theme.red } else { secondary })
                        .whitespace_normal()
                        .role(gpui::Role::Label)
                        .aria_label(SharedString::from(text.clone()))
                        .child(text)
                        .into_any_element()
                }
            }
        };
        let hover_session = session.to_owned();
        let row_selector = format!("conversation-row:{session}:{}", row.id);
        div()
            .debug_selector(move || row_selector.clone())
            .id(SharedString::from(format!("{session}:row:{}", row.id)))
            .w_full()
            .flex()
            .justify_center()
            .when_some(answer_hover, |row, key| {
                row.on_hover(cx.listener(move |app, hovered: &bool, _, cx| {
                    let Some(ui) = app.ui_for_mut(&hover_session) else {
                        return;
                    };
                    if *hovered {
                        if ui.hovered_answer.as_ref() != Some(&key) {
                            ui.hovered_answer = Some(key.clone());
                            cx.notify();
                        }
                    } else if ui.hovered_answer.as_ref() == Some(&key) {
                        ui.hovered_answer = None;
                        cx.notify();
                    }
                }))
            })
            .child(
                div()
                    .w_full()
                    .max_w(px(768.))
                    .min_w_0()
                    .px(px(20.))
                    .pt(px(row.before))
                    .pb(px(row.after))
                    .child(content),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_reasoning_and_compound_read_steps_share_one_tool_disclosure() {
        let commands = [
            "ls -la back-end docs assets; cat package.json; cat .gitignore",
            "find back-end/src docs/js -maxdepth 3 -type f | head -50; echo '---'; cat package.json",
            "cat back-end/src/app.js; echo '--- routes'; cat back-end/src/routes/chatRoutes.js",
            "find assets -maxdepth 3 | head -40; git log --oneline -15; git status --short | head",
            "wc -l docs/js/main.js docs/index.html; head -40 docs/index.html",
            "grep -rn 'fetch\\|/api/chat\\|EventSource' docs --include='*.js' | head",
        ];
        let mut blocks = vec![block("user", "user", BlockKind::User, "Inspect this project.")];
        for (index, command) in commands.iter().enumerate() {
            blocks.push(block(&format!("thought-{index}"), &format!("step-{index}"), BlockKind::Reasoning, "Inspect the next part of the project."));
            let mut call = block(&format!("probe-{index}"), &format!("step-{index}"), BlockKind::Tool,
                &serde_json::json!({"command":command}).to_string());
            call.detail = "bash".into();
            call.status = ToolStatus::Success;
            blocks.push(call);
        }
        let mut ui = TranscriptUi::default();
        let closed = project(&blocks, false, "session", &ui);
        let visible = closed.iter().filter(|row| !row.hidden).collect::<Vec<_>>();
        assert_eq!(visible.len(), 2, "only the user prompt and one compact work line are visible");
        assert!(matches!(&visible[1].kind, RowKind::ToolGroup { title, count: 6, open: false, .. } if title.as_ref() == "Read files"));
        assert!(!closed.iter().any(|row| !row.hidden && matches!(row.kind, RowKind::Reasoning { .. })));
        assert!(!closed.iter().any(|row| !row.hidden && tool_row(row)));
        let key = visible[1].id.clone();
        let ids = closed.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
        ui.tool_groups.insert(key, true);
        let opened = project(&blocks, false, "session", &ui);
        assert_eq!(ids, opened.iter().map(|row| row.id.clone()).collect::<Vec<_>>());
        assert_eq!(opened.iter().filter(|row| !row.hidden && tool_row(row)).count(), 6);
        assert!(!opened.iter().any(|row| !row.hidden && matches!(row.kind, RowKind::Reasoning { .. })));
        assert!(opened.iter().filter(|row| tool_row(row)).all(|row| row.summary.is_none()), "raw command summaries require explicit expansion");
    }

    #[test]
    fn answer_copy_timers_cannot_clear_repeated_or_reopened_panel_copies() {
        let mut old = TranscriptUi {
            copied_answer: Some(Arc::from("answer")),
            answer_copy_generation: 1,
            ..Default::default()
        };
        let owner = old.answer_copy_owner.clone();
        assert!(old.answer_copy_is_current(&owner, 1, "answer"));
        old.answer_copy_generation = 2;
        assert!(!old.answer_copy_is_current(&owner, 1, "answer"), "earlier copy timer cannot clear a newer click");
        let replacement = TranscriptUi {
            copied_answer: Some(Arc::from("answer")),
            answer_copy_generation: 1,
            ..Default::default()
        };
        assert!(!replacement.answer_copy_is_current(&owner, 1, "answer"), "reopened session has a distinct presentation owner");
    }

    fn block(id: &str, message: &str, kind: BlockKind, text: &str) -> Block {
        Block {
            id: Arc::from(id),
            message_id: Arc::from(message),
            part_id: Arc::from(id),
            kind,
            text: Arc::from(text),
            detail: Arc::from(""),
            output: Arc::from(""),
            status: ToolStatus::None,
            streaming: false,
            agent: None,
            model: None,
            created_at: 1000,
            completed_at: Some(3000),
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
            progress: Arc::from(""),
            interrupted: false,
        }
    }
    #[test]
    fn answer_footer_uses_the_final_markdown_margin_instead_of_the_block_count() {
        for (parts, expected) in [
            (
                vec![
                    "## Heading",
                    "| A | B |\n|---|---|\n|1|2|",
                    "After the table.",
                ],
                6.,
            ),
            (vec!["First paragraph", "Last paragraph"], 20.),
            (vec!["Only paragraph"], 6.),
            (vec!["Paragraph", "```text\ncode\n```"], 6.),
            (
                vec!["Paragraph", "![Image](https://example.test/image.png)"],
                18.,
            ),
        ] {
            let blocks = parts
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    block(
                        &format!("part-{index}"),
                        "answer",
                        BlockKind::Assistant,
                        text,
                    )
                })
                .collect::<Vec<_>>();
            let rows = project(&blocks, false, "session", &TranscriptUi::default());
            let footer = rows
                .iter()
                .find(|row| matches!(row.kind, RowKind::Footer { .. }))
                .unwrap();
            assert_eq!(footer.before, expected, "{parts:?}");
        }
    }
    #[test]
    fn skill_notice_projects_as_one_event_row() {
        let mut skill = block("skill", "assistant", BlockKind::Skill, "Checked");
        skill.detail = "review".into();
        skill.output = "Review".into();
        let rows = project(&[skill], false, "session", &TranscriptUi::default());
        let rows = rows
            .iter()
            .filter(
                |row| matches!(&row.kind, RowKind::Status(block) if block.kind == BlockKind::Skill),
            )
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id.as_ref(), "event:skill");
    }

    #[test]
    fn compaction_notice_labels_and_projects() {
        for (status, label) in [
            (ToolStatus::Running, "Compacting context…"),
            (ToolStatus::Failed, "Compaction failed"),
            (ToolStatus::Success, "Context compacted"),
        ] {
            assert_eq!(compaction_label(status), label);
        }
        let compaction = block("compaction", "assistant", BlockKind::Compaction, "Summary");
        let rows = project(&[compaction], false, "session", &TranscriptUi::default());
        let row = rows
            .iter()
            .find(|row| {
                matches!(&row.kind, RowKind::Status(block) if block.kind == BlockKind::Compaction)
            })
            .unwrap();
        assert_eq!(row.id.as_ref(), "event:compaction");
    }

    #[test]
    fn retry_notice_matches_source_single_line_text() {
        let mut retry = block("retry", "assistant", BlockKind::Retry, &"x".repeat(100));
        retry.detail = "attempt 4".into();
        let (message, info) = retry_text(&retry);
        assert_eq!(message.chars().count(), 100);
        assert_eq!(info, "attempt 4");

        retry.detail = "".into();
        assert_eq!(retry_text(&retry).1, "");
        retry.text = "".into();
        assert_eq!(retry_text(&retry).0, "");
    }

    #[test]
    fn retry_notice_projects_as_one_status_row() {
        let retry = block(
            "retry",
            "assistant",
            BlockKind::Retry,
            "[VELA_RETRY_SCHEDULED] Rate limited",
        );
        let rows = project(&[retry], false, "session", &TranscriptUi::default());
        let rows = rows
            .iter()
            .filter(
                |row| matches!(&row.kind, RowKind::Status(block) if block.kind == BlockKind::Retry),
            )
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id.as_ref(), "note:retry");
    }

    #[test]
    fn matching_subagent_records_settle_one_task_card_and_are_absorbed() {
        let mut tool = block(
            "tool",
            "assistant",
            BlockKind::Tool,
            r#"{"agent":"build","description":"Run the tests"}"#,
        );
        tool.detail = "subagent".into();
        tool.status = ToolStatus::Running;
        tool.streaming = true;
        tool.child_session_id = Some("ses_child".into());
        let synthetic = block(
            "subagent",
            "assistant",
            BlockKind::System,
            "<subagent id=\"ses_child\" agent=\"build\" description=\"Run the tests\" state=\"completed\"></subagent>",
        );
        let rows = project(
            &[tool.clone(), synthetic.clone()],
            true,
            "session",
            &TranscriptUi::default(),
        );
        let cards = rows
            .iter()
            .filter_map(|row| match &row.kind {
                RowKind::Content { block, .. } if block.id.as_ref() == "tool" => Some(block),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(cards.len(), 1, "the task tool remains the single card");
        assert_eq!(cards[0].status, ToolStatus::Success);
        assert!(!cards[0].streaming);
        assert!(cards[0].completed_at.is_some());
        assert!(
            !rows.iter().any(|row| row.id.as_ref() == "event:subagent"),
            "the completion notice is represented by the settled card"
        );

        let other = block(
            "other",
            "assistant",
            BlockKind::System,
            "<subagent id=\"ses_other\" agent=\"build\" description=\"Inspect the renderer\" state=\"running\"></subagent>",
        );
        let rows = project(&[tool, other], true, "session", &TranscriptUi::default());
        assert!(
            rows.iter().any(|row| row.id.as_ref() == "activity:tool"),
            "a different dispatch keeps its tool row"
        );
        assert!(rows.iter().any(|row| row.id.as_ref() == "event:other"));
    }

    #[test]
    fn repeated_task_snapshots_merge_into_the_first_card() {
        let mut first = block(
            "tool-first",
            "assistant",
            BlockKind::Tool,
            r#"{"agent":"build","description":"Run the tests"}"#,
        );
        first.detail = "subagent".into();
        first.status = ToolStatus::Running;
        first.streaming = true;
        let mut second = block(
            "tool-first",
            "assistant",
            BlockKind::Tool,
            r#"{"agent":"build","description":"Run the tests"}"#,
        );
        second.detail = "subagent".into();
        second.status = ToolStatus::Success;
        second.completed_at = Some(first.created_at + 5);
        second.child_session_id = Some("ses_child".into());
        let expected_completion = second.completed_at;
        let rows = project(&[first, second], true, "session", &TranscriptUi::default());
        let cards = rows
            .iter()
            .filter_map(|row| match &row.kind {
                RowKind::Content { block, .. }
                    if matches!(block.id.as_ref(), "tool-first" | "tool-second") =>
                {
                    Some(block)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id.as_ref(), "tool-first");
        assert_eq!(cards[0].status, ToolStatus::Success);
        assert_eq!(cards[0].child_session_id.as_deref(), Some("ses_child"));
        assert_eq!(cards[0].completed_at, expected_completion);
    }
    #[test]
    fn distinct_resolved_task_dispatches_keep_their_own_cards() {
        let mut first = block(
            "tool-first",
            "assistant",
            BlockKind::Tool,
            r#"{"agent":"build","description":"Run the tests"}"#,
        );
        first.detail = "subagent".into();
        first.status = ToolStatus::Running;
        first.streaming = true;
        first.child_session_id = Some("ses_first".into());
        let mut second = block(
            "tool-second",
            "assistant",
            BlockKind::Tool,
            r#"{"agent":"build","description":"Run the tests"}"#,
        );
        second.detail = "subagent".into();
        second.status = ToolStatus::Success;
        second.completed_at = Some(first.created_at + 5);
        second.child_session_id = Some("ses_second".into());
        let before = block("before", "intro", BlockKind::Assistant, "Before the calls");
        let after = block("after", "outro", BlockKind::Assistant, "After the calls");
        let blocks = [before, first, second, after];
        let mut ui = TranscriptUi::default();
        let compact = project(&blocks, true, "session", &ui);
        assert_eq!(compact.iter().filter(|row| tool_row(row) && !row.hidden).count(), 2,
            "parallel delegated agents remain visible as compact stable rows");
        let group = compact.iter().position(|row| matches!(&row.kind,
            RowKind::ToolGroup { count: 2, running: 1, open: false, .. })).unwrap();
        assert!(compact[..group].iter().any(|row| row.id.as_ref() == "text:before"));
        assert!(compact[group + 1..].iter().any(|row| row.id.as_ref() == "text:after"));
        let mut settled = blocks.clone();
        settled[1].status = ToolStatus::Success;
        settled[1].streaming = false;
        let collapsed = project(&settled, false, "session", &ui);
        assert!(collapsed.iter().any(|row| row.id == compact[group].id
            && matches!(&row.kind, RowKind::ToolGroup { running: 0, open: false, .. })));
        ui.tool_groups.insert(compact[group].id.clone(), true);
        let rows = project(&blocks, true, "session", &ui);
        let cards = rows
            .iter()
            .filter_map(|row| match &row.kind {
                RowKind::Content { block, .. }
                    if matches!(block.id.as_ref(), "tool-first" | "tool-second") =>
                {
                    Some(block)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(cards.len(), 2, "a same-signature second dispatch is not the first one");
        assert_eq!(cards[0].id.as_ref(), "tool-first");
        assert_eq!(cards[0].status, ToolStatus::Running);
        assert_eq!(cards[0].child_session_id.as_deref(), Some("ses_first"));
        assert_eq!(cards[1].id.as_ref(), "tool-second");
        assert_eq!(cards[1].status, ToolStatus::Success);
        assert_eq!(cards[1].child_session_id.as_deref(), Some("ses_second"));
    }

    #[test]
    fn internal_reminders_are_hidden_while_subagent_records_remain_events() {
        let blocks = vec![
            block(
                "reminder",
                "note",
                BlockKind::System,
                "<system-reminder>\nprivate context\n</system-reminder>",
            ),
            block(
                "subagent",
                "note",
                BlockKind::System,
                "<subagent id=\"ses_child\" agent=\"build\" description=\"Run the tests\" state=\"completed\"></subagent>",
            ),
        ];
        let rows = project(&blocks, false, "session", &TranscriptUi::default());
        assert!(rows.iter().any(|row| row.id.as_ref() == "event:subagent"));
        assert!(
            !rows.iter().any(|row| row.id.as_ref() == "event:reminder"),
            "prompt scaffolding must not render as transcript content"
        );
    }
    #[test]
    fn streamed_spacing_revisions_replace_rows_and_session_changes_release_old_text() {
        let ui = TranscriptUi::default();
        let mut blocks = vec![
            block("first", "answer", BlockKind::Assistant, "## Heading"),
            block("tail", "answer", BlockKind::Assistant, "```text\ncode\n```"),
        ];
        let rows = project(&blocks, false, "session", &ui);
        assert_eq!(
            rows.iter()
                .find(|row| row.id.as_ref() == "text:tail")
                .unwrap()
                .before,
            7.
        );
        blocks[0].text = "Paragraph".into();
        let rows = project(&blocks, false, "session", &ui);
        assert_eq!(
            rows.iter()
                .find(|row| row.id.as_ref() == "text:tail")
                .unwrap()
                .before,
            14.
        );
        for index in 0..100 {
            blocks[1].text = format!("Streaming revision {index}").into();
            project(&blocks, false, "session", &ui);
            assert_eq!(ui.spacing.borrow().len(), 2);
        }
        let history = (0..100)
            .map(|index| {
                block(
                    &format!("old-{index}"),
                    "answer",
                    BlockKind::Assistant,
                    "Old history",
                )
            })
            .collect::<Vec<_>>();
        project(&history, false, "session", &ui);
        project(&blocks, false, "new-session", &ui);
        assert_eq!(ui.spacing.borrow().len(), 2);
    }
    #[test]
    fn virtual_blocks_collapse_reference_margins_at_their_actual_boundaries() {
        let source = [
            "## Heading",
            "- [x] Task",
            "| A | B |\n|---|---|\n|1|2|",
            "```text\ncode\n```",
            "Paragraph",
            "Next paragraph",
            "## Heading again",
            "---",
            "After rule",
        ];
        let blocks = source
            .iter()
            .enumerate()
            .map(|(index, text)| {
                block(
                    &format!("part-{index}"),
                    "answer",
                    BlockKind::Assistant,
                    text,
                )
            })
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        text_rows(&mut rows, &blocks, 0., None, false, &mut HashMap::new());
        assert_eq!(
            rows.iter().map(|row| row.before).collect::<Vec<_>>(),
            vec![0., 7., 7., 7., 7., 14., 14., 24.5, 24.5]
        );
    }
    struct NavigationFixture {
        ui: TranscriptUi,
        list: ListState,
        focus: gpui::FocusHandle,
        input: gpui::Entity<TextInput>,
    }
    impl Render for NavigationFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = self.list.clone();
            div()
                .size_full()
                .on_key_down(cx.listener(|this, event, window, cx| {
                    if this.ui.navigate(event, &this.list, window) {
                        cx.notify();
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .id("code-control")
                        .track_focus(&self.focus)
                        .h(px(20.))
                        .child("Copy"),
                )
                .child(self.input.clone())
                .child(div().h(px(100.)).child(crate::scroll::ListViewport::new(
                    self.ui.viewport.clone(),
                    state.clone(),
                    move |_| {
                        list(state, |index, _, _| {
                            div()
                                .h(px(20.))
                                .child(format!("Row {index}"))
                                .into_any_element()
                        })
                        .size_full()
                        .into_any_element()
                    },
                )))
        }
    }
    #[gpui::test]
    fn transcript_keyboard_navigation_pages_and_stays_out_of_text_inputs(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            window.activate_window();
            let focus = cx.focus_handle().tab_stop(true);
            window.focus(&focus, cx);
            let mut ui = TranscriptUi::default();
            ui.focus.insert("code-control".into(), focus.clone());
            NavigationFixture {
                ui,
                focus,
                list: ListState::new(100, ListAlignment::Top, px(100.)).measure_all(),
                input: cx.new(|cx| TextInput::new(true, "Draft", cx)),
            }
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(200.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        cx.simulate_keystrokes("end");
        draw(cx);
        assert!(view.read_with(cx, |view, _| view.list.logical_scroll_top().item_ix) > 90);
        cx.simulate_keystrokes("home");
        draw(cx);
        assert_eq!(
            view.read_with(cx, |view, _| view.list.logical_scroll_top().item_ix),
            0
        );
        cx.simulate_keystrokes("pagedown");
        draw(cx);
        let page = view.read_with(cx, |view, _| view.list.logical_scroll_top());
        assert!(page.item_ix > 0 && page.item_ix < 10);
        cx.simulate_keystrokes("pageup");
        draw(cx);
        assert_eq!(
            view.read_with(cx, |view, _| view.list.logical_scroll_top().item_ix),
            0
        );
        cx.simulate_keystrokes("cmd-down");
        draw(cx);
        assert!(view.read_with(cx, |view, _| view.list.logical_scroll_top().item_ix) > 90);
        cx.simulate_keystrokes("cmd-up");
        draw(cx);
        assert_eq!(
            view.read_with(cx, |view, _| view.list.logical_scroll_top().item_ix),
            0
        );
        cx.update(|window, cx| window.focus(&view.read(cx).input.read(cx).focus_handle(cx), cx));
        let before = view.read_with(cx, |view, _| view.list.logical_scroll_top());
        cx.simulate_keystrokes("end");
        draw(cx);
        let after = view.read_with(cx, |view, _| view.list.logical_scroll_top());
        assert_eq!(
            (after.item_ix, after.offset_in_item),
            (before.item_ix, before.offset_in_item)
        );
    }
    #[test]
    fn attachment_only_correction_remeasures_the_same_user_row() {
        let mut user = block("user", "user-message", BlockKind::User, "Review the files");
        let ui = TranscriptUi::default();
        let before = project(std::slice::from_ref(&user), false, "session", &ui);
        user.attachments.push(Arc::from("fixture.txt"));
        let after = project(&[user], false, "session", &ui);
        assert_eq!(before[0].id, after[0].id);
        assert_ne!(before[0].revision(), after[0].revision());
    }

    #[test]
    fn row_revision_uses_immutable_payload_identity() {
        let mut block = block("prose", "a", BlockKind::Assistant, "Original");
        let ui = TranscriptUi::default();
        let baseline = project(std::slice::from_ref(&block), false, "session", &ui)
            .remove(0);
        let baseline = baseline.revision().layout;
        let shared = block.clone();
        let reused = project(&[shared], false, "session", &ui).remove(0);
        assert_eq!(baseline, reused.revision().layout,
            "cloning shares the immutable payload Arcs");
        block.text = Arc::from("Original");
        let replaced = project(&[block.clone()], false, "session", &ui).remove(0);
        assert_ne!(baseline, replaced.revision().layout,
            "a new payload allocation conservatively remeasures the row");
        block.output = Arc::from("tool output");
        let output = project(&[block], false, "session", &ui).remove(0);
        assert_ne!(replaced.revision().layout, output.revision().layout);
    }

    #[test]
    fn tool_transitions_do_not_move_or_hide_preceding_prose() {
        let mut prose = block("prose", "a", BlockKind::Assistant, "Inspecting files.");
        prose.streaming = true;
        prose.completed_at = None;
        let before = project(&[prose.clone()], true, "session", &TranscriptUi::default());
        let mut tool = block("tool", "a", BlockKind::Tool, "{}");
        tool.status = ToolStatus::Running;
        tool.streaming = true;
        let mut blocks = vec![prose.clone(), tool.clone()];
        let during = project(&blocks, true, "session", &TranscriptUi::default());
        assert_eq!(before[0].id, during[0].id);
        assert_eq!(during[0].id.as_ref(), "text:prose");
        assert_eq!(during[1].id.as_ref(), "trace-line:activity:tool");
        assert_eq!(during[2].id.as_ref(), "activity:tool");
        assert!(
            matches!(&during[0].kind, RowKind::Content { block, .. } if block.text.as_ref() == "Inspecting files.")
        );
        blocks[0].streaming = false;
        blocks[0].completed_at = Some(3000);
        blocks[1].streaming = false;
        blocks[1].status = ToolStatus::Success;
        blocks.push(block("reply", "b", BlockKind::Assistant, "Found it."));
        let after = project(&blocks, false, "session", &TranscriptUi::default());
        assert_eq!(after[0].id, during[0].id);
        assert_eq!(after[1].id, during[1].id);
        assert!(after.iter().any(|row| row.id == during[2].id), "settling preserves the original leaf");
        assert!(after.iter().any(|row| row.id.as_ref() == "text:reply"));
    }

    #[test]
    fn reasoning_and_tools_remain_chronological_and_footer_uses_turn_duration() {
        let blocks = vec![
            block("user", "user", BlockKind::User, "Prompt"),
            block("reason", "step", BlockKind::Reasoning, "Earlier reasoning"),
            block("tool", "step", BlockKind::Tool, "input"),
            block("answer", "step", BlockKind::Assistant, "Final answer"),
            block("error", "step", BlockKind::Error, "Failure"),
        ];
        let mut ui = TranscriptUi::default();
        let closed = project(&blocks, false, "session", &ui).into_iter()
            .filter(|row| !matches!(row.kind, RowKind::ToolGroup { .. })).collect::<Vec<_>>();
        assert!(matches!(closed[0].kind, RowKind::User(_)));
        assert!(matches!(
            closed[1].kind,
            RowKind::Reasoning { open: false, .. }
        ));
        assert!(
            matches!(&closed[2].kind, RowKind::Content { block, .. } if block.kind == BlockKind::Tool)
        );
        assert!(
            matches!(&closed[3].kind, RowKind::Content { block, .. } if block.text.as_ref() == "Final answer")
        );
        assert!(closed.iter().any(|row|matches!(&row.kind,RowKind::Footer{text,duration:Some(2000),..} if text.as_ref()=="Final answer")));
        ui.reasoning_open.insert("session:reason".into(), true);
        let open = project(&blocks, false, "session", &ui);
        assert!(open.iter().any(|row| matches!(
            row.kind,
            RowKind::Content {
                reasoning: true,
                ..
            }
        )));
        assert_eq!(
            open.iter()
                .filter(|row| matches!(row.kind, RowKind::Footer { .. }))
                .count(),
            1
        );
    }
    #[test]
    fn double_click_word_selection_handles_unicode_and_whitespace() {
        let text = "fix the café bug";
        assert_eq!(&text[selected_word(text, 0)], "fix");
        assert_eq!(&text[selected_word(text, 9)], "café");
        assert_eq!(&text[selected_word(text, 3)], " ");
        assert_eq!(&text[selected_word(text, text.len())], "bug");
        assert_eq!(selected_word("", 0), 0..0);
    }

    #[test]
    fn reasoning_defaults_collapsed_and_manual_choice_survives_completion() {
        let mut reasoning = block("reason", "step", BlockKind::Reasoning, "First\n\nSecond");
        reasoning.streaming = true;
        reasoning.completed_at = None;
        let mut ui = TranscriptUi::default();
        let closed = project(&[reasoning.clone()], true, "session", &ui);
        assert_eq!(closed.iter().filter(|row| !row.hidden).count(), 1);
        assert!(matches!(&closed[0].kind, RowKind::ToolGroup { title, .. } if title.as_ref() == "Thinking"));
        assert!(closed[1].hidden && matches!(closed[1].kind, RowKind::Reasoning { open: false, running: true, .. }));
        ui.reasoning_open.insert("session:reason".into(), true);
        let open = project(&[reasoning.clone()], true, "session", &ui);
        assert_eq!(open.iter().filter(|row| !row.hidden).count(), 2);
        reasoning.streaming = false;
        reasoning.completed_at = Some(4000);
        let settled = project(&[reasoning], false, "session", &ui);
        assert!(matches!(settled[1].kind, RowKind::Reasoning { open: true, running: false, .. }));
        assert_eq!(settled.iter().map(|row| &row.id).collect::<Vec<_>>(), open.iter().map(|row| &row.id).collect::<Vec<_>>());
    }
    #[test]
    fn copy_preserves_full_original_markdown_after_last_tool_and_long_answers_stay_rows() {
        use vela_core::transcript::{Message, Part, PartKind, Role};
        let mut message = Message::text("answer", Role::Assistant, 0, "Commentary");
        message.parts.push(Part {
            id: "tool".into(),
            ordinal: 1,
            kind: PartKind::Tool(Box::default()),
        });
        let text = format!("**Exact**\n\n\n{}\n", "long 🌍 ".repeat(10_000));
        message.parts.push(Part {
            id: "final".into(),
            ordinal: 2,
            kind: PartKind::Text {
                text: text.clone(),
                streaming: false,
            },
        });
        assert_eq!(answer_text(&message), format!("Commentary\n\n{text}"));
        let blocks = (0..500)
            .map(|index| {
                block(
                    &format!("part-{index}"),
                    "answer",
                    BlockKind::Assistant,
                    &format!("Paragraph {index}"),
                )
            })
            .collect::<Vec<_>>();
        let rows = project(&blocks, false, "session", &TranscriptUi::default());
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row.kind, RowKind::Content { .. }))
                .count(),
            500
        );
        assert!(matches!(rows.last().unwrap().kind, RowKind::Footer { .. }));
    }
}
