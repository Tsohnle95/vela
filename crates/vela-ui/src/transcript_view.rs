//! Vela turn presentation projected into independently virtualized rows.
//! A long answer remains a sequence of stable Markdown blocks; expanding a
//! process group never turns the whole conversation into one layout element.
use super::*;

type SpacingCache = HashMap<Arc<str>, (Arc<str>, markdown_view::BlockSpacing)>;

const PACED_IMMEDIATE: usize = 512;
const PACED_INTERVAL: Duration = Duration::from_millis(24);

/// Source's reveal step: tiny tails advance slowly, large streams catch up in
/// larger chunks, and the end snaps forward to the next whitespace/punctuation.
fn next_paced_end(text: &str, start: usize) -> usize {
    let remaining = text.len().saturating_sub(start);
    let step = if remaining <= 12 {
        2
    } else if remaining <= 48 {
        4
    } else if remaining <= 96 {
        8
    } else {
        remaining.div_ceil(4).min(256)
    };
    let mut end = (start + step).min(text.len());
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    let max = (end + 8).min(text.len());
    let mut index = end;
    while index < max {
        let ch = text[index..].chars().next().unwrap();
        if ch.is_whitespace() || matches!(ch, '.' | ',' | '!' | '?' | ';' | ':' | ')' | ']') {
            return index + ch.len_utf8();
        }
        index += ch.len_utf8();
    }
    end
}

fn paced_blocks(blocks: &[Block], ui: &TranscriptUi) -> Vec<Block> {
    let pacing = ui.pacing.borrow();
    if pacing.is_empty() {
        return blocks.to_vec();
    }
    blocks
        .iter()
        .map(|block| {
            if block.kind == BlockKind::Assistant
                && block.streaming
                && let Some(shown) = pacing.get(&block.id)
                && shown.len() < block.text.len()
            {
                let mut block = block.clone();
                block.text = Arc::from(shown.as_str());
                return block;
            }
            block.clone()
        })
        .collect()
}

#[derive(Default)]
pub(super) struct TranscriptUi {
    spacing: RefCell<SpacingCache>,
    pub(super) work_open: HashSet<String>,
    pub(super) tool_open: HashSet<String>,
    reasoning_open: HashMap<String, bool>,
    hovered_answer: Option<Arc<str>>,
    copied_answer: Option<Arc<str>>,
    pub(super) copied_tool: Option<Arc<str>>,
    pacing: RefCell<HashMap<Arc<str>, String>>,
    pacing_timer: bool,
    pacing_last: Option<std::time::Instant>,
    clock_running: bool,
    clock_generation: u64,
    focus: HashMap<SharedString, gpui::FocusHandle>,
    pub(super) markdown: markdown_view::MarkdownUi,
    viewport: crate::scroll::ListViewportHandle,
    pub(super) scroll_generation: u64,
    pub(super) scrolling_to_bottom: bool,
}
impl TranscriptUi {
    /// Reveal newly appended assistant text at Source's pace. The first visible
    /// snapshot is shown in full (matching Source's initial state); later large
    /// deltas catch up over successive 24ms ticks.
    fn advance_pacing(&mut self, blocks: &[Block]) -> bool {
        let streaming = blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Assistant && block.streaming)
            .map(|block| block.id.clone())
            .collect::<HashSet<_>>();
        let mut pacing = self.pacing.borrow_mut();
        pacing.retain(|id, _| streaming.contains(id));
        let advance = self
            .pacing_last
            .is_none_or(|last| last.elapsed() >= PACED_INTERVAL);
        let mut active = false;
        for block in blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Assistant && block.streaming)
        {
            let shown = pacing
                .entry(block.id.clone())
                .or_insert_with(|| block.text.to_string());
            if !block.text.starts_with(shown.as_str()) {
                *shown = block.text.to_string();
            } else if block.text.len().saturating_sub(shown.len()) > PACED_IMMEDIATE {
                if advance {
                    let end = next_paced_end(&block.text, shown.len());
                    if end > shown.len() {
                        *shown = block.text[..end].to_string();
                    }
                }
            } else if shown.len() < block.text.len() {
                *shown = block.text.to_string();
            }
            if shown.len() < block.text.len() {
                active = true;
            }
        }
        if !active {
            self.pacing_last = None;
        } else if advance {
            self.pacing_last = Some(std::time::Instant::now());
        }
        active
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
    pub(super) fn navigate(
        &mut self,
        event: &KeyDownEvent,
        list: &ListState,
        window: &Window,
    ) -> bool {
        if !self.markdown.contains_focused(window)
            && !self.focus.values().any(|focus| focus.is_focused(window))
        {
            return false;
        }
        let key = &event.keystroke;
        if key.modifiers.alt || key.modifiers.control || key.modifiers.shift {
            return false;
        }
        match key.key.as_str() {
            "home" if !key.modifiers.platform => list.scroll_to(ListOffset::default()),
            "end" if !key.modifiers.platform => list.scroll_to_end(),
            "up" if key.modifiers.platform => list.scroll_to(ListOffset::default()),
            "down" if key.modifiers.platform => list.scroll_to_end(),
            "pageup" if !key.modifiers.platform => list.scroll_by(-self.viewport.height() * 0.875),
            "pagedown" if !key.modifiers.platform => list.scroll_by(self.viewport.height() * 0.875),
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
    id: SharedString,
    before: f32,
    after: f32,
    kind: RowKind,
}

#[derive(Clone)]
enum RowKind {
    Content {
        block: Block,
        answer: Option<Arc<str>>,
        reasoning: bool,
    },
    User(Block),
    Work {
        key: String,
        running: bool,
        open: bool,
        started: Option<i64>,
        duration: Option<i64>,
        previous: usize,
    },
    Reasoning {
        key: String,
        running: bool,
        open: bool,
    },
    Footer {
        key: Arc<str>,
        text: Arc<str>,
        duration: Option<i64>,
        agent: Option<String>,
        model: Option<String>,
    },
    Status(Block),
}

#[derive(Default)]
struct Turn {
    user: Option<Block>,
    body: Vec<Block>,
}

fn push(rows: &mut Vec<Row>, id: String, before: f32, kind: RowKind) {
    rows.push(Row {
        id: id.into(),
        before,
        after: 0.,
        kind,
    });
}

fn assistant(block: &Block) -> bool {
    matches!(
        block.kind,
        BlockKind::Assistant
            | BlockKind::Reasoning
            | BlockKind::Tool
            | BlockKind::Error
            | BlockKind::Retry
            | BlockKind::Activity
    )
}

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
        let current = spacing.entry(block.id.clone()).or_insert_with(|| {
            (
                block.text.clone(),
                markdown_view::block_spacing(&block.text),
            )
        });
        if !Arc::ptr_eq(&current.0, &block.text) && current.0 != block.text {
            *current = (
                block.text.clone(),
                markdown_view::block_spacing(&block.text),
            );
        }
        let current = current.1;
        push(
            rows,
            format!("text:{}", block.id),
            if index == 0 {
                gap
            } else {
                previous.bottom.max(current.top_after(previous))
            },
            RowKind::Content {
                block: block.clone(),
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

/// Vela keeps one delegated-agent card per dispatch. Repeated task-tool
/// snapshots merge into the first row, a later `<subagent>` completion notice
/// settles its status, and the synthetic row is suppressed once a task card
/// represents that child or dispatch.
fn collapse_subagent_tools(blocks: &[Block]) -> Vec<Block> {
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
        existing.created_at = latest.created_at;
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
        let mut synthetic = Vec::new();
        for (index, block) in turn.iter().enumerate() {
            if block.kind == BlockKind::Tool && crate::subagent::is_task_tool(&block.detail) {
                let signature =
                    crate::subagent::tool_dispatch_signature(&block.detail, &block.text);
                let child = block.child_session_id.as_deref().unwrap_or("");
                let child_key = (!child.is_empty()).then(|| format!("child:{child}"));
                let signature_key = signature.as_ref().map(|value| format!("sig:{value}"));
                let existing = child_key
                    .as_ref()
                    .and_then(|key| represented.get(key).copied())
                    .or_else(|| {
                        signature_key
                            .as_ref()
                            .and_then(|key| represented.get(key).copied())
                    });
                if let Some(position) = existing {
                    merge(&mut canonical[position].1, block);
                    if let Some(child) = block
                        .child_session_id
                        .as_deref()
                        .filter(|child| !child.is_empty())
                    {
                        represented.insert(format!("child:{child}"), position);
                    }
                } else {
                    let position = canonical.len();
                    canonical.push((index, (*block).clone()));
                    if let Some(key) = child_key {
                        represented.insert(key, position);
                    }
                    if let Some(key) = signature_key {
                        represented.insert(key, position);
                    }
                }
                continue;
            }
            if block.kind == BlockKind::System
                && let Some(reference) = crate::subagent::parse(&block.text)
            {
                let position = if reference.id.is_empty() {
                    crate::subagent::ref_dispatch_signature(&reference)
                        .and_then(|signature| represented.get(&format!("sig:{signature}")).copied())
                } else {
                    represented
                        .get(&format!("child:{}", reference.id))
                        .copied()
                        .or_else(|| {
                            crate::subagent::ref_dispatch_signature(&reference).and_then(
                                |signature| represented.get(&format!("sig:{signature}")).copied(),
                            )
                        })
                };
                if let Some(position) = position {
                    let card = &mut canonical[position].1;
                    if !reference.state.is_empty() {
                        card.status =
                            crate::subagent::settled_status(card.status, &reference.state);
                        if matches!(card.status, ToolStatus::Success | ToolStatus::Failed) {
                            card.streaming = false;
                            card.completed_at = card.completed_at.or(Some(card.created_at));
                        }
                    }
                    synthetic.push(index);
                    continue;
                }
            }
            canonical.push((index, (*block).clone()));
        }
        for (index, block) in canonical {
            if !synthetic.contains(&index) {
                visible.push(block);
            }
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
    visible
}

pub(super) fn project(blocks: &[Block], busy: bool, session: &str, ui: &TranscriptUi) -> Vec<Row> {
    let blocks = paced_blocks(&collapse_subagent_tools(blocks), ui);
    let mut spacing = ui.spacing.borrow_mut();
    // Revisions replace the same row entry; changing sessions cannot retain
    // an unbounded collection of old message text in the presentation cache.
    if spacing.len() > blocks.len().saturating_mul(2).max(64) {
        spacing.clear();
    }
    let mut turns: Vec<Turn> = Vec::new();
    for block in &blocks {
        if block.kind == BlockKind::User {
            turns.push(Turn {
                user: Some(block.clone()),
                body: Vec::new(),
            });
        } else {
            if turns.is_empty() {
                turns.push(Turn::default());
            }
            turns.last_mut().unwrap().body.push(block.clone());
        }
    }
    let mut rows = Vec::new();
    for (turn_index, turn) in turns.iter().enumerate() {
        let first = turn.user.as_ref().or_else(|| turn.body.first()).unwrap();
        let key = format!("{session}:{}", first.message_id);
        let mut gap = if rows.is_empty() { 0. } else { 16. };
        if let Some(user) = &turn.user {
            push(
                &mut rows,
                format!("user:{}", user.message_id),
                gap,
                RowKind::User(user.clone()),
            );
            gap = 16.;
        }
        let last = turn.body.iter().rev().find(|block| assistant(block));
        let last_id = last.map(|block| block.message_id.clone());
        let last_tool = turn.body.iter().rposition(|block| {
            Some(&block.message_id) == last_id.as_ref() && block.kind == BlockKind::Tool
        });
        let answer_start = turn
            .body
            .iter()
            .enumerate()
            .find(|(index, block)| {
                Some(&block.message_id) == last_id.as_ref()
                    && block.kind == BlockKind::Assistant
                    && !block.text.trim().is_empty()
                    && last_tool.is_none_or(|tool| *index > tool)
            })
            .map(|(index, _)| index);
        let answer = answer_start
            .map(|start| {
                turn.body[start..]
                    .iter()
                    .filter(|block| {
                        Some(&block.message_id) == last_id.as_ref()
                            && block.kind == BlockKind::Assistant
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let process = turn
            .body
            .iter()
            .enumerate()
            .filter(|(index, block)| {
                (assistant(block) || block.kind == BlockKind::Shell)
                    && !matches!(block.kind, BlockKind::Error | BlockKind::Retry)
                    && !(Some(&block.message_id) == last_id.as_ref()
                        && answer_start.is_some_and(|start| *index >= start))
            })
            .map(|(_, block)| block.clone())
            .collect::<Vec<_>>();
        let events = turn.body.iter().filter(|block| {
            matches!(
                block.kind,
                BlockKind::System | BlockKind::Compaction | BlockKind::Skill
            ) && !(block.kind == BlockKind::System
                && crate::subagent::is_internal_reminder(&block.text))
        });
        for event in events {
            push(
                &mut rows,
                format!("event:{}", event.id),
                gap,
                RowKind::Status(event.clone()),
            );
            gap = 16.;
        }
        let running = process.iter().any(|block| {
            matches!(block.kind, BlockKind::Tool | BlockKind::Shell)
                && matches!(block.status, ToolStatus::Running | ToolStatus::Streaming)
        }) || (busy && turn_index == turns.len() - 1 && answer.is_empty());
        let has_process = process
            .iter()
            .any(|block| block.kind != BlockKind::Activity || !block.detail.is_empty());
        if has_process || running {
            let open = running || ui.work_open.contains(&key);
            let duration = if busy {
                None
            } else {
                turn.user.as_ref().zip(last).map(|(user, last)| {
                    (last.completed_at.unwrap_or(last.created_at) - user.created_at).max(1000)
                })
            };
            let previous = process
                .iter()
                .map(|block| block.message_id.clone())
                .collect::<HashSet<_>>()
                .len();
            push(
                &mut rows,
                format!("work:{key}"),
                gap,
                RowKind::Work {
                    key: key.clone(),
                    running,
                    open,
                    started: turn
                        .user
                        .as_ref()
                        .map(|user| user.created_at)
                        .or_else(|| answer.first().map(|block| block.created_at)),
                    duration,
                    previous,
                },
            );
            gap = 16.;
            if open {
                // Work flex gap (4px) plus process-body top padding (8px).
                let mut process_gap = 12.;
                let process_blocks = process
                    .iter()
                    .filter(|block| block.kind != BlockKind::Activity)
                    .collect::<Vec<_>>();
                for (index, block) in process_blocks.iter().enumerate() {
                    if index > 0 && block.kind == BlockKind::Assistant {
                        let previous = process_blocks[index - 1];
                        if previous.kind == BlockKind::Assistant
                            && previous.part_id == block.part_id
                        {
                            process_gap = 14.;
                        }
                    }
                    if block.kind == BlockKind::Reasoning {
                        let reasoning_key = format!("{session}:{}", block.part_id);
                        let thinking = busy
                            && turn_index == turns.len() - 1
                            && Some(&block.message_id) == last_id.as_ref()
                            && block.streaming;
                        let open = ui
                            .reasoning_open
                            .get(&reasoning_key)
                            .copied()
                            .unwrap_or(thinking);
                        push(
                            &mut rows,
                            format!("reasoning:{reasoning_key}"),
                            process_gap,
                            RowKind::Reasoning {
                                key: reasoning_key,
                                running: thinking,
                                open,
                            },
                        );
                        if open {
                            let body = vela_core::markdown::split_ranges(&block.text)
                                .into_iter()
                                .enumerate()
                                .map(|(index, range)| {
                                    let mut paragraph = (*block).clone();
                                    paragraph.id =
                                        Arc::from(format!("{}:paragraph:{index}", block.id));
                                    paragraph.kind = BlockKind::Assistant;
                                    paragraph.text = Arc::from(&block.text[range]);
                                    paragraph
                                })
                                .collect::<Vec<_>>();
                            text_rows(&mut rows, &body, 4., None, true, &mut spacing);
                        }
                    } else {
                        push(
                            &mut rows,
                            format!("process:{}", block.id),
                            process_gap,
                            RowKind::Content {
                                block: (*block).clone(),
                                answer: None,
                                reasoning: false,
                            },
                        );
                        if block.kind == BlockKind::Assistant
                            && index + 1 < process_blocks.len()
                            && (process_blocks[index + 1].kind != BlockKind::Assistant
                                || process_blocks[index + 1].part_id != block.part_id)
                            && index > 0
                            && process_blocks[index - 1].kind == BlockKind::Assistant
                            && process_blocks[index - 1].part_id == block.part_id
                        {
                            rows.last_mut().unwrap().after = 14.;
                        }
                    }
                    process_gap = 8.;
                }
            }
        }
        if !answer.is_empty() {
            let message = last_id.clone().unwrap();
            let tail_margin = text_rows(
                &mut rows,
                &answer,
                gap,
                Some(message.clone()),
                false,
                &mut spacing,
            );
            let streaming = busy
                && turn_index == turns.len() - 1
                && answer
                    .last()
                    .is_some_and(|block| block.completed_at.is_none());
            if !streaming {
                let completed = answer.last().and_then(|block| block.completed_at);
                let duration = completed.map(|completed| {
                    (completed
                        - turn
                            .user
                            .as_ref()
                            .map(|user| user.created_at)
                            .unwrap_or(answer[0].created_at))
                    .max(0)
                });
                let text = answer
                    .iter()
                    .map(|block| block.text.as_ref())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let last_answer = answer.last();
                push(
                    &mut rows,
                    format!("footer:{message}"),
                    6. + tail_margin,
                    RowKind::Footer {
                        key: message,
                        text: text.into(),
                        duration,
                        agent: last_answer
                            .and_then(|block| block.agent.as_deref())
                            .map(str::to_string),
                        model: last_answer
                            .and_then(|block| block.model.as_deref())
                            .map(str::to_string),
                    },
                );
            }
            gap = 16.;
        }
        for note in turn.body.iter().filter(|block| {
            Some(&block.message_id) == last_id.as_ref()
                && matches!(block.kind, BlockKind::Error | BlockKind::Retry)
        }) {
            push(
                &mut rows,
                format!("note:{}", note.id),
                gap,
                RowKind::Status(note.clone()),
            );
            gap = 16.;
        }
    }
    rows
}

fn retry_text(block: &Block) -> (String, String) {
    let message = if block.text.trim().is_empty() {
        "Retrying".into()
    } else {
        block.text.chars().take(80).collect::<String>()
    };
    let info = if block.detail.trim().is_empty() {
        "Retrying".into()
    } else {
        format!("Retrying · {}", block.detail)
    };
    (message, info)
}

fn compaction_label(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::Running | ToolStatus::Streaming => "Compacting session",
        ToolStatus::Failed => "Compaction failed",
        _ => "Session compacted",
    }
}

fn meta_identity(
    agent: Option<&str>,
    model: Option<&str>,
    models: &[vela_opencode::ModelInfo],
) -> Vec<String> {
    let agent = agent.filter(|agent| !agent.trim().is_empty()).map(|agent| {
        let mut chars = agent.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    });
    let model = model.filter(|model| !model.trim().is_empty()).map(|id| {
        models
            .iter()
            .find(|candidate| candidate.id == id)
            .map(|candidate| candidate.name.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| id.to_string())
    });
    [agent, model]
        .into_iter()
        .flatten()
        .filter(|part| !part.trim().is_empty())
        .collect()
}

fn answer_meta(
    agent: Option<&str>,
    model: Option<&str>,
    models: &[vela_opencode::ModelInfo],
    duration_ms: Option<i64>,
) -> Option<String> {
    let mut parts = meta_identity(agent, model, models);
    if let Some(duration) = duration_ms.map(duration) {
        parts.push(duration);
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn user_message_meta(
    block: &Block,
    models: &[vela_opencode::ModelInfo],
) -> Option<String> {
    let parts = meta_identity(block.agent.as_deref(), block.model.as_deref(), models);
    (!parts.is_empty()).then(|| parts.join("\u{00A0}·\u{00A0}"))
}

fn duration(ms: i64) -> String {
    let seconds = ms.max(0) / 1000;
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m {}s", seconds / 60, seconds % 60)
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn answer_text(message: &vela_core::transcript::Message) -> String {
    use vela_core::transcript::PartKind;
    let start = message
        .parts
        .iter()
        .rposition(|part| matches!(part.kind, PartKind::Tool(_)))
        .map_or(0, |index| index + 1);
    message.parts[start..]
        .iter()
        .filter_map(|part| match &part.kind {
            PartKind::Text { text, .. } if !text.trim().is_empty() => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Source renders attached prompt files under the user text as a right-aligned
/// wrapping row of file chips. The chip is static: it shows the generic file
/// glyph and the attachment name, truncated to the 220px card width.
fn user_attachments(names: &[Arc<str>], key: &str, theme: &Theme) -> AnyElement {
    let key = key.to_string();
    div()
        .id(SharedString::from(format!("user-attachments:{key}")))
        .debug_selector({
            let key = key.clone();
            move || format!("user-attachments:{key}")
        })
        .mt(px(8.))
        .max_w(gpui::relative(0.82))
        .flex()
        .flex_wrap()
        .justify_end()
        .gap(px(8.))
        .children(names.iter().enumerate().map(|(index, name)| {
            let name = name.clone();
            let selector = format!("{key}:{index}");
            let label = format!("Attached file {name}");
            div()
                .id(SharedString::from(format!("user-attachment:{selector}")))
                .debug_selector(move || format!("user-attachment:{selector}"))
                .role(gpui::Role::Label)
                .aria_label(SharedString::from(label))
                .h(px(48.))
                .w(px(220.))
                .max_w_full()
                .min_w_0()
                .flex_shrink_0()
                .px(px(10.))
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.inset)
                .flex()
                .items_center()
                .gap(px(8.))
                .hover(|style| style.border_color(theme.border_strong))
                .child(
                    div()
                        .flex_shrink_0()
                        .font_family("codicon")
                        .text_size(px(20.))
                        .line_height(px(20.))
                        .text_color(theme.faint)
                        .child(icon("ea7b")),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(theme.dim)
                        .child(SharedString::from(name.to_string())),
                )
                .into_any_element()
        }))
        .into_any_element()
}

impl VelaApp {
    fn rebuild_transcript_rows(&mut self, cx: &mut Context<Self>) {
        let pacing = self.transcript_ui.advance_pacing(&self.blocks);
        let old_count = self.transcript_rows.len();
        self.transcript_rows = Arc::new(project(
            &self.blocks,
            self.phase.is_active(),
            self.active_id.as_deref().unwrap_or(""),
            &self.transcript_ui,
        ));
        let count = self.transcript_rows.len();
        if count > old_count {
            self.transcript_list
                .splice(old_count..old_count, count - old_count);
        }
        if count < old_count {
            self.transcript_list.splice(count..old_count, 0);
        }
        self.transcript_list.remeasure();
        if pacing && !self.transcript_ui.pacing_timer {
            self.transcript_ui.pacing_timer = true;
            let session = self.active_id.clone();
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(PACED_INTERVAL).await;
                let _ = this.update(cx, |app, cx| {
                    app.transcript_ui.pacing_timer = false;
                    if app.active_id == session {
                        app.rebuild_transcript_rows(cx);
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub(super) fn render_native_transcript(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.transcript_ui.markdown.keyboard = self.picker_anchors.keyboard.clone();
        let running = self
            .transcript_rows
            .iter()
            .any(|row| matches!(row.kind, RowKind::Work { running: true, .. }));
        if running != self.transcript_ui.clock_running {
            self.transcript_ui.clock_running = running;
            self.transcript_ui.clock_generation += 1;
            if running {
                let generation = self.transcript_ui.clock_generation;
                cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor().timer(Duration::from_secs(1)).await;
                        let keep = this
                            .update(cx, |app, cx| {
                                if app.transcript_ui.clock_generation != generation
                                    || !app.transcript_ui.clock_running
                                {
                                    return false;
                                }
                                if app.settings_open
                                    || !app.agent_panel_open
                                    || app.active_id.is_none()
                                    || !app.transcript_rows.iter().any(|row| {
                                        matches!(row.kind, RowKind::Work { running: true, .. })
                                    })
                                {
                                    app.transcript_ui.clock_running = false;
                                    return false;
                                }
                                app.transcript_list.remeasure();
                                cx.notify();
                                true
                            })
                            .unwrap_or(false);
                        if !keep {
                            break;
                        }
                    }
                })
                .detach();
            }
        }
        let rows = self.transcript_rows.clone();
        let owner = cx.weak_entity();
        let scroll_owner = cx.weak_entity();
        let list_state = self.transcript_list.clone();
        let viewport = crate::scroll::ListViewport::new(
            self.transcript_ui.viewport.clone(),
            list_state.clone(),
            move |gutter| {
                div()
                    .size_full()
                    .pr(gutter)
                    .child(
                        list(list_state, move |index, window, cx| {
                            let Some(row) = rows.get(index) else {
                                return div().into_any_element();
                            };
                            owner
                                .update(cx, |app, cx| app.render_transcript_row(row, window, cx))
                                .unwrap_or_else(|_| div().into_any_element())
                        })
                        .size_full()
                        .w_full()
                        // Reference .vela-column uses 16px at its top edge.
                        .pt(px(16.))
                        .pb(px(32.))
                        .into_any_element(),
                    )
                    .into_any_element()
            },
        );
        let viewport = viewport.on_scroll(move |list, _, cx| {
            let following = crate::scroll::list_distance_from_bottom(list) <= px(24.);
            let _ = scroll_owner.update(cx, |app, cx| {
                app.stick = following;
                app.transcript_ui.scroll_generation += 1;
                app.transcript_ui.scrolling_to_bottom = false;
                cx.notify();
            });
        });
        let show_bottom = !self.transcript_ui.scrolling_to_bottom
            && crate::scroll::list_distance_from_bottom(&self.transcript_list) > px(24.);
        let button = if show_bottom {
            let row = Row {
                id: "scroll-to-bottom".into(),
                before: 0.,
                after: 0.,
                kind: RowKind::Work {
                    key: String::new(),
                    running: false,
                    open: false,
                    started: None,
                    duration: None,
                    previous: 0,
                },
            };
            let (button, _) = self.transcript_button(
                &row,
                "Scroll to bottom".into(),
                "",
                window,
                cx,
                |app, _, cx| app.scroll_transcript_to_bottom(cx),
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
            .child(viewport)
            .children(button)
            .into_any_element()
    }

    fn scroll_transcript_to_bottom(&mut self, cx: &mut Context<Self>) {
        self.stick = true;
        self.transcript_ui.scroll_generation += 1;
        let generation = self.transcript_ui.scroll_generation;
        let distance = crate::scroll::list_distance_from_bottom(&self.transcript_list);
        if self.phase.is_active() || cx.reduce_motion() || distance <= px(24.) {
            self.transcript_list.scroll_to_end();
            cx.notify();
            return;
        }
        self.transcript_ui.scrolling_to_bottom = true;
        let session = self.active_id.clone();
        let started = std::time::Instant::now();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let progress = (started.elapsed().as_secs_f32() / 0.260).min(1.);
                let keep = this
                    .update(cx, |app, cx| {
                        if app.transcript_ui.scroll_generation != generation
                            || app.active_id != session
                        {
                            return false;
                        }
                        let maximum = app.transcript_list.max_offset_for_scrollbar().y;
                        let remaining = distance * (1. - progress).powi(3);
                        app.transcript_list.set_offset_from_scrollbar(point(
                            px(0.),
                            -(maximum - remaining).max(px(0.)),
                        ));
                        if progress >= 1. {
                            app.transcript_list.scroll_to_end();
                            app.transcript_ui.scrolling_to_bottom = false;
                        }
                        cx.notify();
                        progress < 1.
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    fn transcript_button(
        &mut self,
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
        let row_key = row.id.clone();
        let focus = if let Some(focus) = self.transcript_ui.focus.get(&key) {
            focus.clone()
        } else {
            let focus = cx.focus_handle().tab_stop(true);
            cx.on_focus(&focus, window, move |app, _, cx| {
                if let Some(index) = app.transcript_rows.iter().position(|row| row.id == row_key) {
                    app.transcript_list.scroll_to_reveal_item(index);
                }
                cx.notify();
            })
            .detach();
            cx.on_blur(&focus, window, |_, _, cx| cx.notify()).detach();
            self.transcript_ui.focus.insert(key.clone(), focus.clone());
            focus
        };
        let clicked = action;
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
            &self.transcript_ui.markdown.keyboard,
        );
        (control, focus)
    }

    fn render_delegated_card(
        &mut self,
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
        let icon = if card.tone == crate::subagent::Tone::Working {
            use gpui::AnimationExt;
            glyph("working-spinner", 14., icon_color)
                .with_animation(
                    SharedString::from(format!("subagent-spin:{block_id}")),
                    gpui::Animation::new(Duration::from_millis(700)).repeat(),
                    |svg, progress| {
                        svg.with_transformation(gpui::Transformation::rotate(gpui::percentage(
                            progress,
                        )))
                    },
                )
                .into_any_element()
        } else {
            glyph("robot", 14., icon_color).into_any_element()
        };
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(icon)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(17.))
                            .text_color(if card.tone == crate::subagent::Tone::Failed {
                                theme.red
                            } else {
                                theme.text
                            })
                            .truncate()
                            .child(card.agent.clone()),
                    )
                    .when(!card.description.is_empty(), |body| {
                        body.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(secondary)
                                .truncate()
                                .child(card.description.clone()),
                        )
                    }),
            );
        if card.child_id.is_empty() {
            return div()
                .debug_selector(|| "subagent-card".into())
                .w_full()
                .max_w(px(560.))
                .px(px(10.))
                .py(px(8.))
                .rounded(px(10.))
                .border_1()
                .border_color(theme.border_soft)
                .bg(theme.inset)
                .child(body)
                .into_any_element();
        }
        let open_id = card.child_id.clone();
        let label = format!("Open delegated agent session: {}", card.title);
        let (button, _) = self.transcript_button(
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
            .px(px(10.))
            .py(px(8.))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.inset)
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
        let theme = Theme::default();
        let secondary = theme.text.opacity(0.62);
        let tertiary = theme.text.opacity(0.46);
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
                                        self.active_id.as_deref().unwrap_or(""),
                                    )
                                })
                                .flatten();
                            if let Some(card) = card {
                                self.render_delegated_card(row, &block.id, card, window, cx)
                            } else {
                                let model = tool_view::presentation(&block);
                                let expandable = model.expandable();
                                let key = format!(
                                    "{}:{}",
                                    self.active_id.as_deref().unwrap_or(""),
                                    block.id
                                );
                                let open = self.transcript_ui.tool_open.contains(&key);
                                let copied = self.transcript_ui.copied_tool.as_deref()
                                    == Some(block.id.as_ref());
                                let (button, focus) = if expandable {
                                    let (button, focus) = self.transcript_button(
                                        row,
                                        model.summary.clone().into(),
                                        "",
                                        window,
                                        cx,
                                        move |app, _, cx| {
                                            if !app.transcript_ui.tool_open.remove(&key) {
                                                app.transcript_ui.tool_open.insert(key.clone());
                                            }
                                            app.transcript_list.remeasure();
                                            cx.notify();
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
                                let copy_value = tool_view::copy_text(&block);
                                let copy_block = block.id.clone();
                                let (copy, copy_focus) = self.transcript_button(
                                    row,
                                    if copied { "Copied" } else { "Copy" }.into(),
                                    "copy-output",
                                    window,
                                    cx,
                                    move |app, _, cx| {
                                        if copy_value.is_empty() {
                                            return;
                                        }
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            copy_value.clone(),
                                        ));
                                        app.transcript_ui.copied_tool = Some(copy_block.clone());
                                        let copied_key = copy_block.clone();
                                        cx.spawn(async move |this, cx| {
                                            cx.background_executor()
                                                .timer(Duration::from_secs(2))
                                                .await;
                                            let _ = this.update(cx, |app, cx| {
                                                if app.transcript_ui.copied_tool.as_deref()
                                                    == Some(copied_key.as_ref())
                                                {
                                                    app.transcript_ui.copied_tool = None;
                                                    cx.notify();
                                                }
                                            });
                                        })
                                        .detach();
                                        cx.notify();
                                    },
                                );
                                tool_view::render(
                                    &block,
                                    model,
                                    button,
                                    copy,
                                    tool_view::RenderState {
                                        open,
                                        focused: focus
                                            .as_ref()
                                            .is_some_and(|focus| focus.is_focused(window)),
                                        copied,
                                        copy_focused: copy_focus.is_focused(window),
                                        open_keys: &self.transcript_ui.tool_open,
                                    },
                                    &theme,
                                    cx,
                                )
                            }
                        } else {
                            markdown_view::block(
                                &block,
                                if *reasoning { secondary } else { theme.text },
                                &theme,
                                markdown_view::BlockContext {
                                    ui: &mut self.transcript_ui.markdown,
                                    list: self.transcript_list.clone(),
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
                let message_id = block.message_id.to_string();
                let prompt = block.text.to_string();
                let attachments = block.attachments.clone();
                let revert_id = message_id.clone();
                let edit_id = message_id.clone();
                let copy_text = prompt.clone();
                let (copy, _) = self.transcript_button(
                    row,
                    "Copy message".into(),
                    "copy",
                    window,
                    cx,
                    move |_, _, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_text.clone()));
                        cx.notify();
                    },
                );
                let (revert, _) = self.transcript_button(
                    row,
                    "Undo message".into(),
                    "revert",
                    window,
                    cx,
                    move |app, _, cx| {
                        app.revert_message(revert_id.clone(), false, String::new(), cx)
                    },
                );
                let (edit, _) = self.transcript_button(
                    row,
                    "Edit message".into(),
                    "edit",
                    window,
                    cx,
                    move |app, _, cx| app.revert_message(edit_id.clone(), true, prompt.clone(), cx),
                );
                let disabled = self.phase.is_active();
                let mut footer = div()
                    .mt(px(4.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.))
                    .opacity(if disabled { 0.5 } else { 1.0 });
                if let Some(meta) = user_message_meta(block, &self.models) {
                    let selector = format!("user-message-meta-{}", block.id);
                    footer = footer.child(
                        div()
                            .id(SharedString::from(selector.clone()))
                            .debug_selector(move || selector.clone())
                            .aria_label(meta.clone())
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_right()
                            .text_size(px(11.))
                            .line_height(px(15.))
                            .text_color(secondary)
                            .child(SharedString::from(meta)),
                    );
                }
                footer = footer
                    .child(
                        copy.px(px(6.))
                            .py(px(2.))
                            .rounded(px(6.))
                            .text_size(px(11.))
                            .text_color(secondary)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.inset))
                            .child("Copy"),
                    )
                    .child(
                        revert
                            .px(px(6.))
                            .py(px(2.))
                            .rounded(px(6.))
                            .text_size(px(11.))
                            .text_color(secondary)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.inset))
                            .child("Undo"),
                    )
                    .child(
                        edit.px(px(6.))
                            .py(px(2.))
                            .rounded(px(6.))
                            .text_size(px(11.))
                            .text_color(secondary)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.inset))
                            .child("Edit"),
                    );
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(markdown_view::user(
                        &block.text,
                        &block.id,
                        &theme,
                        &mut self.transcript_ui.markdown,
                        window,
                        cx,
                    ))
                    .when(!attachments.is_empty(), |row| {
                        row.child(user_attachments(&attachments, &message_id, &theme))
                    })
                    .child(footer)
                    .into_any_element()
            }
            RowKind::Work {
                key,
                running,
                open,
                started,
                duration: elapsed,
                previous,
            } => {
                let label = if *running {
                    format!(
                        "Working for {}",
                        duration(
                            started
                                .map(|start| now_ms() - start)
                                .unwrap_or(1000)
                                .max(1000)
                        )
                    )
                } else {
                    elapsed
                        .map(|ms| format!("Worked for {}", duration(ms)))
                        .unwrap_or_else(|| format!("{previous} previous messages"))
                };
                let target = key.clone();
                let was_open = *open;
                let (button, _) = self.transcript_button(
                    row,
                    label.clone().into(),
                    "",
                    window,
                    cx,
                    move |app, _, cx| {
                        if was_open {
                            app.transcript_ui.work_open.remove(&target);
                        } else {
                            app.transcript_ui.work_open.insert(target.clone());
                        }
                        app.rebuild_transcript_rows(cx);
                    },
                );
                div()
                    .w_full()
                    .pt(px(2.))
                    .flex()
                    .flex_col()
                    .child(
                        button
                            .aria_expanded(*open)
                            .py(px(2.))
                            .pr(px(6.))
                            .rounded(px(6.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(14.))
                            .line_height(px(17.))
                            .text_color(tertiary)
                            .hover(|style| style.bg(theme.text.opacity(0.05)))
                            .child(label)
                            .child(crate::icons::glyph(
                                if *open {
                                    "chevron-down"
                                } else {
                                    "chevron-right"
                                },
                                12.,
                                secondary,
                            )),
                    )
                    .child(div().mt(px(8.)).w_full().h(px(1.)).bg(theme.border_soft))
                    .into_any_element()
            }
            RowKind::Reasoning { key, running, open } => {
                let target = key.clone();
                let was_open = *open;
                let label = if *running { "Thinking" } else { "Thought" };
                let (button, _) =
                    self.transcript_button(row, label.into(), "", window, cx, move |app, _, cx| {
                        app.transcript_ui
                            .reasoning_open
                            .insert(target.clone(), !was_open);
                        app.rebuild_transcript_rows(cx);
                    });
                button
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
                    .when(*running, |button| {
                        use gpui::AnimationExt;
                        button.child(glyph("working-spinner", 11., secondary).with_animation(
                            SharedString::from(format!("spin:{}", row.id)),
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
                agent,
                model,
            } => {
                answer_hover = Some(key.clone());
                let copied = self.transcript_ui.copied_answer.as_ref() == Some(key);
                let label = if copied { "Copied" } else { "Copy" };
                let copy_key = key.clone();
                let copy_text = text.clone();
                let (button, focus) =
                    self.transcript_button(row, label.into(), "", window, cx, move |app, _, cx| {
                        let text = app
                            .active_id
                            .as_ref()
                            .and_then(|session| app.engine.view(session))
                            .and_then(|view| {
                                view.with_transcript(|transcript| {
                                    transcript.message(&copy_key).map(answer_text)
                                })
                            })
                            .unwrap_or_else(|| copy_text.to_string());
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                        app.transcript_ui.copied_answer = Some(copy_key.clone());
                        let key = copy_key.clone();
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_secs(2)).await;
                            let _ = this.update(cx, |app, cx| {
                                if app.transcript_ui.copied_answer.as_ref() == Some(&key) {
                                    app.transcript_ui.copied_answer = None;
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                        cx.notify();
                    });
                let visible = self
                    .transcript_ui
                    .answer_footer_visible(key, &focus, window);
                div()
                    .w_full()
                    .min_h(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .opacity(if visible { 1. } else { 0. })
                    .child(
                        button
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
                    .when_some(
                        answer_meta(
                            agent.as_deref(),
                            model.as_deref(),
                            &self.models,
                            *elapsed,
                        ),
                        |footer, meta| {
                            footer.child(
                                div()
                                    .id(SharedString::from(format!("answer-meta:{key}")))
                                    .debug_selector(|| "answer-meta".into())
                                    .role(gpui::Role::Label)
                                    .text_size(px(12.))
                                    .text_color(tertiary)
                                    .child(SharedString::from(meta)),
                            )
                        },
                    )
                    .into_any_element()
            }
            RowKind::Status(block) => {
                let card = if block.kind == BlockKind::System {
                    crate::subagent::parse(&block.text).and_then(|reference| {
                        crate::subagent::card(
                            &reference,
                            &self.sessions,
                            &self.agents,
                            self.active_id.as_deref().unwrap_or(""),
                        )
                    })
                } else {
                    None
                };
                if let Some(card) = card {
                    self.render_delegated_card(row, &block.id, card, window, cx)
                } else if block.kind == BlockKind::Retry {
                    let (message, info) = retry_text(block);
                    use gpui::AnimationExt;
                    div()
                        .id(SharedString::from(format!("retry:{}", block.id)))
                        .debug_selector(|| "retry-notice".into())
                        .w_full()
                        .min_w_0()
                        .flex()
                        .items_start()
                        .gap(px(8.))
                        .py(px(2.))
                        .text_color(secondary)
                        .role(gpui::Role::Label)
                        .aria_label(SharedString::from(format!("{message}. {info}")))
                        .child(
                            glyph("working-spinner", 11., secondary)
                                .mt(px(2.))
                                .with_animation(
                                    SharedString::from(format!("retry-spin:{}", block.id)),
                                    gpui::Animation::new(Duration::from_millis(700)).repeat(),
                                    |svg, progress| {
                                        svg.with_transformation(gpui::Transformation::rotate(
                                            gpui::percentage(progress),
                                        ))
                                    },
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .debug_selector(|| "retry-message".into())
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(secondary)
                                        .whitespace_normal()
                                        .child(message),
                                )
                                .child(
                                    div()
                                        .debug_selector(|| "retry-info".into())
                                        .text_size(px(11.))
                                        .line_height(px(16.))
                                        .text_color(tertiary)
                                        .child(info),
                                ),
                        )
                        .into_any_element()
                } else if block.kind == BlockKind::Compaction {
                    let failed = block.status == ToolStatus::Failed;
                    let label = compaction_label(block.status);
                    let label =
                        if matches!(block.status, ToolStatus::Running | ToolStatus::Streaming) {
                            shimmer::text(
                                label.to_string(),
                                true,
                                secondary,
                                theme.text,
                                "compaction-shimmer",
                                SharedString::from(format!("compaction-shimmer:{}", block.id)),
                            )
                        } else {
                            div().child(label).into_any_element()
                        };
                    let mut notice = div()
                        .id(SharedString::from(format!("compaction:{}", block.id)))
                        .debug_selector(|| "compaction-notice".into())
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .py(px(10.))
                                .child(div().flex_1().h(px(1.)).bg(theme.border_soft))
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(13.))
                                        .line_height(px(18.))
                                        .text_color(if failed { theme.red } else { secondary })
                                        .child(label),
                                )
                                .child(div().flex_1().h(px(1.)).bg(theme.border_soft)),
                        );
                    if !block.text.trim().is_empty() {
                        notice = notice.child(div().mt(px(6.)).child(markdown_view::block(
                            block,
                            secondary,
                            &theme,
                            markdown_view::BlockContext {
                                ui: &mut self.transcript_ui.markdown,
                                list: self.transcript_list.clone(),
                                answer: None,
                            },
                            window,
                            cx,
                        )));
                    }
                    if failed && !block.output.trim().is_empty() {
                        notice = notice.child(
                            div()
                                .debug_selector(|| "compaction-error".into())
                                .mt(px(6.))
                                .flex()
                                .items_start()
                                .gap(px(7.))
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(theme.red)
                                .child(glyph("close", 12., theme.red).mt(px(3.)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .whitespace_normal()
                                        .child(block.output.to_string()),
                                ),
                        );
                    }
                    notice.into_any_element()
                } else if block.kind == BlockKind::Skill {
                    let name = if block.output.trim().is_empty() {
                        "Skill"
                    } else {
                        block.output.as_ref()
                    };
                    let mut notice = div()
                        .id(SharedString::from(format!("skill:{}", block.id)))
                        .debug_selector(|| "skill-notice".into())
                        .w_full()
                        .min_w_0()
                        .pl(px(12.))
                        .py(px(1.))
                        .border_l_2()
                        .border_color(theme.border_soft)
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .debug_selector(|| "skill-label".into())
                                .text_size(px(11.))
                                .line_height(px(16.))
                                .text_color(tertiary)
                                .child(format!("Skill activated · {name}")),
                        );
                    if !block.detail.trim().is_empty() {
                        notice = notice.child(
                            div()
                                .debug_selector(|| "skill-detail".into())
                                .text_size(px(11.))
                                .line_height(px(16.))
                                .font_family(crate::theme::FONT_MONO)
                                .text_color(tertiary)
                                .child(block.detail.to_string()),
                        );
                    }
                    if !block.text.trim().is_empty() {
                        notice = notice.child(div().mt(px(6.)).child(markdown_view::block(
                            block,
                            secondary,
                            &theme,
                            markdown_view::BlockContext {
                                ui: &mut self.transcript_ui.markdown,
                                list: self.transcript_list.clone(),
                                answer: None,
                            },
                            window,
                            cx,
                        )));
                    }
                    notice.into_any_element()
                } else {
                    let error =
                        block.kind == BlockKind::Error || block.status == ToolStatus::Failed;
                    let text = block.text.to_string();
                    div()
                        .id(SharedString::from(format!("status-label:{}", block.id)))
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
        div()
            .id(SharedString::from(format!("row:{}", row.id)))
            .w_full()
            .flex()
            .justify_center()
            .when_some(answer_hover, |row, key| {
                row.on_hover(cx.listener(move |app, hovered: &bool, _, cx| {
                    if *hovered {
                        if app.transcript_ui.hovered_answer.as_ref() != Some(&key) {
                            app.transcript_ui.hovered_answer = Some(key.clone());
                            cx.notify();
                        }
                    } else if app.transcript_ui.hovered_answer.as_ref() == Some(&key) {
                        app.transcript_ui.hovered_answer = None;
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
    fn paced_reveal_uses_source_steps_and_character_boundaries() {
        assert_eq!(next_paced_end("abcdefghijklmnopqrstuvwxyz", 0), 4);
        assert_eq!(next_paced_end("aaaa.bbbb", 0), 5);
        assert_eq!(next_paced_end("a界界", 0), 4);
        assert_eq!(next_paced_end("small", 5), 5);
    }

    #[test]
    fn streaming_text_reveals_in_steps_and_snaps_on_completion() {
        let mut ui = TranscriptUi::default();
        let mut paced = block("paced", "answer", BlockKind::Assistant, &"a".repeat(500));
        paced.streaming = true;
        assert!(!ui.advance_pacing(&[paced.clone()]));
        let full = format!("{}{}", "a".repeat(500), "b".repeat(1500));
        paced.text = Arc::from(full.as_str());
        assert!(ui.advance_pacing(&[paced.clone()]));
        let visible = |ui: &TranscriptUi, block: &Block| {
            project(std::slice::from_ref(block), true, "session", ui)
                .iter()
                .find_map(|row| match &row.kind {
                    RowKind::Content { block, .. } if block.id.as_ref() == "paced" => {
                        Some(block.text.clone())
                    }
                    _ => None,
                })
                .unwrap()
        };
        let shown = visible(&ui, &paced);
        assert!(shown.len() > 500);
        assert!(shown.len() < full.len());
        let before = shown.len();
        assert!(ui.advance_pacing(&[paced.clone()]));
        assert_eq!(visible(&ui, &paced).len(), before);
        std::thread::sleep(Duration::from_millis(25));
        assert!(ui.advance_pacing(&[paced.clone()]));
        assert!(visible(&ui, &paced).len() > before);
        for _ in 0..1000 {
            ui.pacing_last = None;
            if !ui.advance_pacing(&[paced.clone()]) {
                break;
            }
        }
        assert_eq!(visible(&ui, &paced).as_ref(), full.as_str());
        paced.streaming = false;
        assert!(!ui.advance_pacing(&[paced.clone()]));
        assert!(ui.pacing.borrow().is_empty());
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
            (ToolStatus::Running, "Compacting session"),
            (ToolStatus::Failed, "Compaction failed"),
            (ToolStatus::Success, "Session compacted"),
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
    fn retry_notice_matches_source_two_line_text() {
        let mut retry = block("retry", "assistant", BlockKind::Retry, &"x".repeat(100));
        retry.detail = "attempt 4".into();
        let (message, info) = retry_text(&retry);
        assert_eq!(message.chars().count(), 80);
        assert_eq!(info, "Retrying · attempt 4");

        retry.detail = "".into();
        assert_eq!(retry_text(&retry).1, "Retrying");
        retry.text = "".into();
        assert_eq!(retry_text(&retry).0, "Retrying");
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
            rows.iter().any(|row| row.id.as_ref() == "process:tool"),
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
            "tool-second",
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
    struct AttachmentChipsFixture(Vec<Arc<str>>);
    impl Render for AttachmentChipsFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(600.))
                .flex()
                .flex_col()
                .items_end()
                .child(user_attachments(&self.0, "test", &Theme::default()))
        }
    }
    #[gpui::test]
    fn user_attachment_chips_render_file_cards_right_aligned_and_wrap(
        cx: &mut gpui::TestAppContext,
    ) {
        let names = vec![
            Arc::from("main.rs"),
            Arc::from("notes with spaces.md"),
            Arc::from("third.txt"),
        ];
        let (view, cx) = cx.add_window_view(move |_, _| AttachmentChipsFixture(names));
        cx.draw(point(px(0.), px(0.)), size(px(600.), px(300.)), |_, _| {
            view.clone().into_any_element()
        });
        let first = cx
            .debug_bounds("user-attachment:test:0")
            .expect("first attachment chip");
        let second = cx
            .debug_bounds("user-attachment:test:1")
            .expect("second attachment chip");
        let third = cx
            .debug_bounds("user-attachment:test:2")
            .expect("third attachment chip");
        assert_eq!(first.size, size(px(220.), px(48.)));
        assert_eq!(second.size, size(px(220.), px(48.)));
        assert_eq!(first.origin.y, second.origin.y);
        assert!(
            third.origin.y > first.origin.y + px(40.),
            "third chip wraps onto a second row: {third:?}"
        );
        assert_eq!(
            second.origin.x + second.size.width,
            px(600.),
            "chips stay right-aligned in the user column"
        );
        assert_eq!(
            third.origin.x + third.size.width,
            px(600.),
            "wrapped chips stay right-aligned"
        );
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
        }
    }
    #[test]
    fn final_text_survives_work_folding_and_footer_uses_turn_duration() {
        let user = block("user", "user", BlockKind::User, "Prompt");
        let reasoning = block("reason", "step", BlockKind::Reasoning, "Earlier reasoning");
        let tool = block("tool", "step", BlockKind::Tool, "input");
        let mut answer = block("answer", "step", BlockKind::Assistant, "Final answer");
        answer.agent = Some("build".into());
        answer.model = Some("claude".into());
        let error = block("error", "step", BlockKind::Error, "Failure");
        let blocks = vec![user, reasoning, tool, answer, error];
        let mut ui = TranscriptUi::default();
        let closed = project(&blocks, false, "session", &ui);
        assert!(matches!(closed[0].kind, RowKind::User(_)));
        assert!(
            closed
                .iter()
                .any(|row| matches!(row.kind, RowKind::Work { open: false, .. }))
        );
        assert!(
            !closed
                .iter()
                .any(|row| matches!(row.kind, RowKind::Reasoning { .. }))
        );
        assert!(closed.iter().any(|row|matches!(&row.kind,RowKind::Footer{text,duration:Some(2000),agent:Some(agent),model:Some(model),..} if text.as_ref()=="Final answer" && agent=="build" && model=="claude")));
        assert!(matches!(closed.last().unwrap().kind, RowKind::Status(_)));
        ui.work_open.insert("session:user".into());
        let open = project(&blocks, false, "session", &ui);
        assert!(
            open.iter()
                .any(|row| matches!(row.kind, RowKind::Reasoning { open: false, .. }))
        );
        assert!(open.iter().any(
            |row| matches!(&row.kind,RowKind::Content{block,..} if block.kind==BlockKind::Tool)
        ));
        assert_eq!(
            open.iter()
                .filter(|row| matches!(row.kind, RowKind::Footer { .. }))
                .count(),
            1
        );
    }
    #[test]
    fn answer_meta_joins_agent_model_and_duration_with_separators() {
        let models = vec![vela_opencode::ModelInfo {
            id: "claude".into(),
            provider_id: "anthropic".into(),
            name: "Claude Sonnet".into(),
            context_limit: None,
            variant: None,
            variants: Vec::new(),
        }];
        assert_eq!(
            answer_meta(
                Some("build"),
                Some("claude"),
                &models,
                Some(125_000)
            )
            .as_deref(),
            Some("Build · Claude Sonnet · 2m 5s")
        );
        assert_eq!(
            answer_meta(Some("plan"), None, &models, Some(1_000)).as_deref(),
            Some("Plan · 1s")
        );
        assert_eq!(
            answer_meta(None, Some("missing"), &models, None).as_deref(),
            Some("missing")
        );
        assert_eq!(answer_meta(None, None, &models, None), None);
    }

    #[test]
    fn user_message_meta_resolves_agent_and_catalog_model() {
        let models = vec![vela_opencode::ModelInfo {
            id: "claude".into(),
            provider_id: "anthropic".into(),
            name: "Claude Sonnet".into(),
            context_limit: None,
            variant: None,
            variants: Vec::new(),
        }];
        let mut block = Block {
            id: "user".into(),
            message_id: "message".into(),
            part_id: "user".into(),
            kind: BlockKind::User,
            text: "fix it".into(),
            detail: String::new().into(),
            output: String::new().into(),
            status: ToolStatus::None,
            streaming: false,
            agent: Some("build".into()),
            model: Some("claude".into()),
            created_at: 0,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        };
        assert_eq!(
            user_message_meta(&block, &models).as_deref(),
            Some("Build\u{00A0}·\u{00A0}Claude Sonnet")
        );
        block.agent = None;
        block.model = None;
        assert_eq!(user_message_meta(&block, &models), None);
    }

    #[test]
    fn reasoning_defaults_open_while_streaming_and_manual_choice_is_retained() {
        let mut reasoning = block("reason", "step", BlockKind::Reasoning, "First\n\nSecond");
        reasoning.streaming = true;
        reasoning.completed_at = None;
        let mut ui = TranscriptUi::default();
        let open = project(&[reasoning.clone()], true, "session", &ui);
        assert!(open.iter().any(|row| matches!(
            row.kind,
            RowKind::Reasoning {
                open: true,
                running: true,
                ..
            }
        )));
        assert_eq!(
            open.iter()
                .filter(|row| matches!(
                    row.kind,
                    RowKind::Content {
                        reasoning: true,
                        ..
                    }
                ))
                .count(),
            2
        );
        ui.reasoning_open.insert("session:reason".into(), false);
        assert!(
            !project(&[reasoning], true, "session", &ui)
                .iter()
                .any(|row| matches!(
                    row.kind,
                    RowKind::Content {
                        reasoning: true,
                        ..
                    }
                ))
        );
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
        assert_eq!(answer_text(&message), text);
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
