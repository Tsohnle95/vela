//! Multi-panel activation state. The engine retains transcript/draft data per
//! session; this model owns the UI-side selection, composer and scroll snapshot
//! so switching panels does not reset them.
use std::collections::HashSet;
use std::sync::Arc;

use gpui::ListOffset;
use vela_core::event::{PermissionRequest, TodoItem};
use vela_core::session::{SessionPhase, SessionUsage};
use vela_core::transcript::Block;
use vela_core::transcript::BlockKind;
use vela_opencode::{FormRequest, ModelInfo};

pub const MAX_PANELS: usize = 8;
/// Maximum transcript blocks kept in a tiled (inactive-panel) preview.
pub const PREVIEW_BLOCKS: usize = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelPreviewRow {
    /// Index into the panel's source block list, so the tiled renderer can
    /// expose the original tool payload rather than the condensed summary.
    pub source_index: usize,
    pub kind: BlockKind,
    pub label: &'static str,
    pub text: String,
    pub user: bool,
}

/// Condense the tail of a panel transcript for simultaneous tiled rendering.
/// The active panel uses the full interactive renderer; inactive tiles show this
/// snapshot, so every panel remains independently visible and scrollable.
pub fn preview_rows(blocks: &[Block], limit: usize) -> Vec<PanelPreviewRow> {
    let start = blocks.len().saturating_sub(limit);
    blocks[start..]
        .iter()
        .enumerate()
        .filter_map(|(offset, block)| {
            let text = block.text.trim();
            let text = if text.is_empty() {
                block.detail.trim()
            } else {
                text
            };
            if text.is_empty() && block.output.trim().is_empty() {
                return None;
            }
            let mut text = if text.is_empty() {
                block.output.trim().to_owned()
            } else {
                text.to_owned()
            };
            if text.chars().count() > 400 {
                text = text.chars().take(400).collect::<String>();
                text.push('…');
            }
            let label = match block.kind {
                BlockKind::User => "You",
                BlockKind::Assistant => "Agent",
                BlockKind::Reasoning => "Thinking",
                BlockKind::Tool => "Tool",
                BlockKind::Shell => "Shell",
                BlockKind::System => "System",
                BlockKind::Compaction => "Compacted",
                BlockKind::Skill => "Skill",
                BlockKind::Error => "Error",
                BlockKind::Retry => "Retry",
                BlockKind::Activity => "Activity",
            };
            Some(PanelPreviewRow {
                source_index: start + offset,
                kind: block.kind,
                label,
                text,
                user: block.is_user(),
            })
        })
        .collect()
}

#[derive(Clone)]
pub struct PanelState {
    pub session: String,
    pub title: String,
    pub composer: String,
    pub blocks: Arc<Vec<Block>>,
    pub phase: SessionPhase,
    pub permissions: Vec<PermissionRequest>,
    pub todos: Vec<TodoItem>,
    pub forms: Vec<FormRequest>,
    pub usage: Option<SessionUsage>,
    pub selected_model: Option<ModelInfo>,
    pub selected_agent: Option<String>,
    pub scroll: ListOffset,
    pub stick: bool,
    /// Expanded tool payloads in the inactive tile's compact transcript. Keys
    /// are block ids scoped to this panel.
    pub tool_open: HashSet<String>,
    /// Most recently copied tool block in the inactive tile, used for its
    /// transient "Copied" label.
    pub copied_tool: Option<String>,
    /// Expanded work groups in the inactive tile. Keys are tied to the first
    /// source block in each consecutive reasoning/activity run.
    pub work_open: HashSet<String>,
    /// Most recently copied user message in the inactive tile.
    pub copied_message: Option<String>,
    /// Explicit width fraction inside the panel's grid row. `None` shares the
    /// row evenly with other automatic panels.
    pub width: Option<f32>,
}

impl PanelState {
    pub fn new(session: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            session: session.into(),
            title: title.into(),
            composer: String::new(),
            blocks: Arc::new(Vec::new()),
            phase: SessionPhase::Idle,
            permissions: Vec::new(),
            todos: Vec::new(),
            forms: Vec::new(),
            usage: None,
            selected_model: None,
            selected_agent: None,
            scroll: ListOffset {
                item_ix: 0,
                offset_in_item: gpui::px(0.),
            },
            stick: true,
            tool_open: HashSet::new(),
            copied_tool: None,
            work_open: HashSet::new(),
            copied_message: None,
            width: None,
        }
    }
}

#[derive(Default)]
pub struct Panels {
    states: Vec<PanelState>,
    active: Option<String>,
}

impl Panels {
    pub fn len(&self) -> usize {
        self.states.len()
    }
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
    pub fn active(&self) -> Option<&PanelState> {
        let id = self.active.as_ref()?;
        self.states.iter().find(|state| &state.session == id)
    }
    pub fn entries(&self) -> &[PanelState] {
        &self.states
    }
    pub fn contains(&self, session: &str) -> bool {
        self.states.iter().any(|state| state.session == session)
    }

    /// Add a panel or activate the existing session. Returns `true` when a new
    /// panel was inserted; the ninth distinct session is rejected.
    pub fn open(&mut self, state: PanelState) -> Result<bool, &'static str> {
        if self.contains(&state.session) {
            self.active = Some(state.session);
            return Ok(false);
        }
        if self.states.len() >= MAX_PANELS {
            return Err("Up to eight agent panels can be open");
        }
        self.active = Some(state.session.clone());
        self.states.push(state);
        Ok(true)
    }
    /// Replace the focused panel in place, matching the reference's
    /// `reopenSession` drill-down: the slot, its position and its width survive
    /// while the session changes. An already-visible session is only focused.
    pub fn replace_active(&mut self, mut state: PanelState) -> Result<bool, &'static str> {
        if self.contains(&state.session) {
            self.active = Some(state.session);
            return Ok(false);
        }
        let Some(index) = self
            .active
            .as_ref()
            .and_then(|active| self.states.iter().position(|panel| &panel.session == active))
        else {
            return self.open(state);
        };
        state.width = self.states[index].width;
        self.states[index] = state;
        self.active = Some(self.states[index].session.clone());
        Ok(true)
    }
    pub fn activate(&mut self, session: &str) -> bool {
        if self.contains(session) {
            self.active = Some(session.to_owned());
            true
        } else {
            false
        }
    }
    pub fn save(&mut self, state: PanelState) {
        if let Some(existing) = self
            .states
            .iter_mut()
            .find(|existing| existing.session == state.session)
        {
            *existing = state;
        }
    }
    /// Close one panel without touching its conversation. The neighboring panel
    /// becomes active; closing the final panel leaves no active session.
    pub fn close(&mut self, session: &str) -> Option<PanelState> {
        let index = self
            .states
            .iter()
            .position(|state| state.session == session)?;
        let removed = self.states.remove(index);
        if self.active.as_deref() == Some(session) {
            self.active = self
                .states
                .get(index.min(self.states.len().saturating_sub(1)))
                .map(|state| state.session.clone());
        }
        Some(removed)
    }
    pub fn replace_title(&mut self, session: &str, title: impl Into<String>) {
        if let Some(state) = self
            .states
            .iter_mut()
            .find(|state| state.session == session)
        {
            state.title = title.into();
        }
    }
    pub fn update(&mut self, session: &str, update: impl FnOnce(&mut PanelState)) {
        if let Some(state) = self
            .states
            .iter_mut()
            .find(|state| state.session == session)
        {
            update(state);
        }
    }

    /// Render rows with normalized width fractions. Indices refer to `entries`.
    pub fn rows(&self) -> Vec<Vec<(usize, f32)>> {
        crate::panel_layout::panel_rows(self.states.len())
            .into_iter()
            .map(|range| {
                let stored = &self.states[range.clone()];
                let widths = stored.iter().map(|state| state.width).collect::<Vec<_>>();
                let fractions = crate::panel_layout::normalize_widths(&widths);
                range
                    .enumerate()
                    .map(|(offset, index)| {
                        (index, fractions.get(offset).copied().unwrap_or_default())
                    })
                    .collect()
            })
            .collect()
    }

    /// Drag the boundary after panel `boundary` within its row. Widths are
    /// stored per panel so they survive activation and unrelated updates.
    pub fn resize_boundary(&mut self, boundary: usize, delta: f32) -> bool {
        let Some(row) = self
            .rows()
            .into_iter()
            .find(|row| row.iter().any(|(index, _)| *index == boundary))
            .filter(|row| row.last().is_some_and(|(index, _)| *index > boundary))
        else {
            return false;
        };
        let mut widths = row.iter().map(|(_, width)| *width).collect::<Vec<_>>();
        let Some(offset) = row.iter().position(|(index, _)| *index == boundary) else {
            return false;
        };
        if !crate::panel_layout::resize_widths(&mut widths, offset, delta) {
            return false;
        }
        for ((index, _), width) in row.into_iter().zip(widths) {
            self.states[index].width = Some(width);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Focusable as _, IntoElement as _, ParentElement as _, Styled as _,
    };

    #[test]
    fn panels_are_bounded_unique_and_close_to_a_neighbor() {
        let mut panels = Panels::default();
        for index in 0..MAX_PANELS {
            assert!(
                panels
                    .open(PanelState::new(format!("s{index}"), format!("S{index}")))
                    .unwrap()
            );
        }
        assert!(!panels.open(PanelState::new("s1", "duplicate")).unwrap());
        assert_eq!(panels.len(), MAX_PANELS);
        assert_eq!(panels.active().unwrap().title, "S1");
        assert!(panels.open(PanelState::new("s8", "S8")).is_err());

        assert!(panels.activate("s3"));
        let mut state = panels.active().unwrap().clone();
        state.composer = "retained draft".into();
        state.stick = false;
        panels.save(state);
        assert_eq!(panels.active().unwrap().composer, "retained draft");
        assert!(!panels.active().unwrap().stick);

        let removed = panels.close("s3").unwrap();
        assert_eq!(removed.session, "s3");
        assert_eq!(panels.active().unwrap().session, "s4");
        assert!(!panels.contains("s3"));

        panels.close("s4");
        panels.close("s5");
        for state in panels.states.clone() {
            panels.close(&state.session);
        }
        assert!(panels.is_empty());
        assert!(panels.active().is_none());
    }

    #[test]
    fn replace_active_keeps_the_slot_width_and_focuses_open_sessions() {
        let mut panels = Panels::default();
        let mut parent = PanelState::new("parent", "Parent");
        parent.width = Some(0.35);
        panels.open(parent).unwrap();
        let mut neighbor = PanelState::new("neighbor", "Neighbor");
        neighbor.width = Some(0.65);
        panels.open(neighbor).unwrap();
        panels.activate("parent");

        assert!(
            panels
                .replace_active(PanelState::new("child", "Child"))
                .unwrap()
        );
        assert_eq!(panels.len(), 2);
        assert_eq!(panels.entries()[0].session, "child");
        assert_eq!(panels.entries()[0].width, Some(0.35));
        assert_eq!(panels.entries()[1].session, "neighbor");
        assert_eq!(panels.entries()[1].width, Some(0.65));
        assert_eq!(panels.active().unwrap().session, "child");

        // Reopening a visible session must focus its existing slot, never
        // duplicate it inside the grid.
        assert!(
            !panels
                .replace_active(PanelState::new("neighbor", "Neighbor again"))
                .unwrap()
        );
        assert_eq!(panels.len(), 2);
        assert_eq!(panels.active().unwrap().session, "neighbor");
        assert_eq!(panels.active().unwrap().title, "Neighbor");
    }

    fn block(kind: BlockKind, text: &str) -> Block {
        Block {
            id: text.into(),
            message_id: text.into(),
            part_id: text.into(),
            kind,
            text: text.into(),
            detail: "".into(),
            output: "".into(),
            status: vela_core::transcript::ToolStatus::None,
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
    fn tiled_rows_normalize_and_resize_per_panel_widths() {
        let mut panels = Panels::default();
        panels.open(PanelState::new("a", "A")).unwrap();
        panels.open(PanelState::new("b", "B")).unwrap();
        assert_eq!(panels.rows(), vec![vec![(0, 0.5), (1, 0.5)]]);
        assert!(panels.resize_boundary(0, 0.1));
        let rows = panels.rows();
        assert!((rows[0][0].1 - 0.6).abs() < 1e-5);
        assert!((rows[0][1].1 - 0.4).abs() < 1e-5);
        // Widths are panel state, not activation state.
        panels.activate("b");
        assert!((panels.rows()[0][0].1 - 0.6).abs() < 1e-5);
        assert_eq!(panels.entries()[0].width, Some(0.6));

        // A five-panel grid has a full first row and a two-up final row.
        for index in 2..5 {
            panels
                .open(PanelState::new(format!("s{index}"), format!("S{index}")))
                .unwrap();
        }
        let rows = panels.rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 3);
        assert_eq!(rows[1].len(), 2);
        assert!((rows[0].iter().map(|(_, width)| width).sum::<f32>() - 1.0).abs() < 1e-5);
        assert!((rows[1].iter().map(|(_, width)| width).sum::<f32>() - 1.0).abs() < 1e-5);
        assert!((rows[1][0].1 - 0.5).abs() < 1e-5);
        // Boundaries at the end of a row cannot be dragged across rows.
        assert!(!panels.resize_boundary(2, 0.1));
    }

    #[test]
    fn tiled_preview_keeps_the_tail_and_labels_roles() {
        let blocks = vec![
            block(BlockKind::User, "first"),
            block(BlockKind::Assistant, "second"),
            block(BlockKind::Tool, "third"),
        ];
        let rows = preview_rows(&blocks, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "Agent");
        assert_eq!(rows[0].text, "second");
        assert_eq!(rows[1].label, "Tool");
        assert_eq!(rows[0].source_index, 1);
        assert_eq!(rows[0].kind, BlockKind::Assistant);
        assert_eq!(rows[1].source_index, 2);
        assert!(!rows[0].user);
        let rows = preview_rows(&blocks, 1);
        assert_eq!(rows[0].text, "third");
        assert_eq!(rows[0].source_index, 2);
        assert!(preview_rows(&[], 8).is_empty());
    }

    struct ComposerProbe {
        inputs: [gpui::Entity<crate::input::TextInput>; 2],
    }

    impl gpui::Render for ComposerProbe {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div().flex().children(self.inputs.clone())
        }
    }

    #[gpui::test]
    fn tiled_composers_retain_text_and_move_focus_independently(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            let inputs = [
                cx.new(|cx| {
                    let mut input = crate::input::TextInput::new(false, "First panel", cx);
                    input.set_text("alpha draft".into(), cx);
                    input
                }),
                cx.new(|cx| {
                    let mut input = crate::input::TextInput::new(false, "Second panel", cx);
                    input.set_text("beta draft".into(), cx);
                    input
                }),
            ];
            ComposerProbe { inputs }
        });
        cx.simulate_resize(gpui::size(gpui::px(420.), gpui::px(160.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(
                gpui::point(gpui::px(0.), gpui::px(0.)),
                gpui::size(gpui::px(420.), gpui::px(160.)),
                |_, _| view.clone().into_any_element(),
            );
        };
        draw(cx);
        let (first, second) = cx.update(|window, cx| {
            let view = view.read(cx);
            let first = view.inputs[0].read(cx).focus_handle(cx);
            let second = view.inputs[1].read(cx).focus_handle(cx);
            window.focus(&first, cx);
            (first, second)
        });
        draw(cx);
        cx.update(|window, cx| {
            assert!(first.is_focused(window));
            window.focus(&second, cx);
        });
        draw(cx);
        cx.update(|window, cx| {
            assert!(second.is_focused(window));
            assert!(!first.is_focused(window));
            let view = view.read(cx);
            assert_eq!(view.inputs[0].read(cx).buffer.text(), "alpha draft");
            assert_eq!(view.inputs[1].read(cx).buffer.text(), "beta draft");
        });
        // Focusing one tile must not clear the other draft, and switching back
        // restores the original input's text and focus.
        cx.update(|window, cx| window.focus(&first, cx));
        draw(cx);
        cx.update(|window, cx| {
            assert!(first.is_focused(window));
            assert_eq!(
                view.read(cx).inputs[0].read(cx).buffer.text(),
                "alpha draft"
            );
        });
    }
}
