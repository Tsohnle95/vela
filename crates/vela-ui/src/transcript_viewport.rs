//! Identity-aware updates for GPUI's measured transcript lists.
//!
//! GPUI remains the geometry authority. This index only retains presentation
//! identities and layout revisions so ordinary streaming never resets a list or
//! invalidates the measurements of unrelated rows.

use gpui::{FollowMode, ListOffset, ListState, Pixels, SharedString, px};
use std::{collections::HashMap, ops::Range};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowRevision {
    /// Stable presentation identity within the session-owned viewport.
    pub(crate) id: SharedString,
    /// Content, expansion, spacing and other height-affecting presentation state.
    pub(crate) layout: u64,
}

impl RowRevision {
    pub(crate) fn new(id: impl Into<SharedString>, layout: u64) -> Self {
        Self {
            id: id.into(),
            layout,
        }
    }
}

#[derive(Default)]
pub(crate) struct TranscriptViewport {
    rows: Vec<RowRevision>,
    turn_reservation: Option<TurnReservation>,
    bottom_padding: Pixels,
}

struct TurnReservation {
    id: SharedString,
    previous_context: Pixels,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ChangeSummary {
    pub(crate) splices: usize,
    pub(crate) remeasured: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct Edit {
    old: Range<usize>,
    new: Range<usize>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Changes {
    edits: Vec<Edit>,
    dirty: Vec<Range<usize>>,
}

impl TranscriptViewport {
    pub(crate) fn requires_natural_tail(&self) -> bool {
        self.turn_reservation.is_some()
    }

    #[cfg(test)]
    pub(crate) fn reserved_turn(&self) -> Option<&str> {
        self.turn_reservation.as_ref().map(|reservation| reservation.id.as_ref())
    }

    /// Anchor an actual newly submitted user row, after its first snapshot was
    /// reconciled. No state changes if the row has not arrived yet.
    ///
    /// Native tail following plus a viewport-sized reservation lets the reply
    /// grow below the prompt while keeping a little preceding context visible.
    /// As the reply fills that space, the same native follow mode follows its
    /// live edge without a second programmatic scroll or synthetic spacer row.
    pub(crate) fn anchor_turn(
        &mut self,
        list: &ListState,
        row_id: &str,
        previous_context: Pixels,
    ) -> bool {
        let Some(row) = self.rows.iter().find(|row| row.id.as_ref() == row_id) else {
            return false;
        };
        self.turn_reservation = Some(TurnReservation {
            id: row.id.clone(),
            previous_context: previous_context.max(px(0.)),
        });
        self.refresh_turn_reservation(list);
        list.set_follow_mode(FollowMode::Tail);
        true
    }

    /// Remove the current turn's remaining layout reservation. An explicit
    /// "latest" action should release it before requesting native tail follow.
    /// Manual reading should instead pass `follow = false` to `sync`: retaining
    /// the remaining geometry avoids clamping a short reply beneath the reader.
    pub(crate) fn release_turn_anchor(&mut self, list: &ListState) {
        self.turn_reservation = None;
        list.set_tail_reservation(None);
    }

    pub(crate) fn set_bottom_padding(&mut self, list: &ListState, padding: Pixels) {
        if self.bottom_padding != padding {
            self.bottom_padding = padding;
            self.refresh_turn_reservation(list);
        }
    }

    fn refresh_turn_reservation(&mut self, list: &ListState) {
        let reservation = self.turn_reservation.as_ref().and_then(|reservation| {
            self.rows
                .iter()
                .position(|row| row.id == reservation.id)
                .map(|index| (index, reservation.previous_context + self.bottom_padding))
        });
        if reservation.is_none() {
            self.turn_reservation = None;
        }
        list.set_tail_reservation(reservation);
    }

    /// Reconcile a list with the latest snapshot. Call outside list rendering or
    /// scroll observers, while GPUI does not hold the ListState borrow.
    ///
    /// Reading retains the same top row and pixel offset. If that row was
    /// removed, its nearest surviving successor (then predecessor) is used.
    /// Following delegates continuously growing rows to GPUI's native tail mode.
    /// Width/font reflow retains an item/pixel anchor, not a text/grapheme anchor.
    pub(crate) fn sync(
        &mut self,
        list: &ListState,
        next: Vec<RowRevision>,
        follow: bool,
    ) -> ChangeSummary {
        let offset = list.logical_scroll_top();
        // Retire only after layout has scrolled into the reserved turn. A
        // height estimate can report "filled" while the previous panel layout
        // still has the prompt near the top with preceding context.
        if self.turn_reservation.as_ref().is_some_and(|reservation| {
            self.rows.get(..=offset.item_ix)
                .is_some_and(|rows| rows.iter().any(|row| row.id == reservation.id))
        })
            && list.viewport_bounds().size.height > px(0.)
            && list.tail_reservation_filled()
        {
            self.release_turn_anchor(list);
        }
        if !follow {
            list.set_follow_mode(FollowMode::Normal);
        }

        // A new viewport can attach to a list restored by the shell/tests, and
        // a session reset may have changed its count independently. Bootstrap
        // structurally without assuming that old numerical indices identify rows.
        let bootstrap = list.item_count() != self.rows.len();
        if !bootstrap && self.rows == next {
            self.refresh_turn_reservation(list);
            if follow && !list.is_following_tail() {
                list.set_follow_mode(FollowMode::Tail);
            }
            return ChangeSummary::default();
        }
        let changes = if bootstrap {
            Changes {
                edits: vec![Edit {
                    old: 0..list.item_count(),
                    new: 0..next.len(),
                }],
                dirty: Vec::new(),
            }
        } else {
            reconcile(&self.rows, &next)
        };
        let anchor = (!follow).then(|| {
            if bootstrap {
                ListOffset {
                    item_ix: offset.item_ix.min(next.len().saturating_sub(1)),
                    offset_in_item: offset.offset_in_item,
                }
            } else {
                restore_anchor(&self.rows, &next, offset)
            }
        });

        // Reverse edits keep every range in old-list coordinates. Unchanged
        // survivors retain their measured height and registered focus handle.
        for edit in changes.edits.iter().rev() {
            list.splice(edit.old.clone(), edit.new.len());
        }
        for dirty in &changes.dirty {
            list.remeasure_items(dirty.clone());
        }
        let summary = ChangeSummary {
            splices: changes.edits.len(),
            remeasured: changes.dirty.iter().map(Range::len).sum(),
        };
        let changed = !changes.edits.is_empty() || !changes.dirty.is_empty();
        self.rows = next;
        // Splices work in list indices and cannot update this identity-owned
        // reservation themselves. Remap it after history prepends/corrections.
        self.refresh_turn_reservation(list);
        if follow {
            if changed || !list.is_following_tail() {
                list.set_follow_mode(FollowMode::Tail);
            }
        } else if changed && let Some(anchor) = anchor {
            // Splices can delete/rebase the numerical anchor; restore identity
            // after every mutation and let GPUI clamp the local offset at layout.
            list.scroll_to(anchor);
        }
        summary
    }
}

/// Retain the longest ordered subsequence of identities. The normal append and
/// middle-insert cases preserve every old row; authoritative reorder/correction
/// only rebuilds rows that genuinely moved. O(n log n), with no quadratic diff.
fn reconcile(old: &[RowRevision], next: &[RowRevision]) -> Changes {
    let shared = old
        .iter()
        .zip(next)
        .take_while(|(old, next)| old.id == next.id)
        .count();
    if shared == old.len().min(next.len()) {
        // A streamed update usually changes row content or extends the tail.
        // These edits already have an identity-preserving alignment.
        let mut changes = Changes::default();
        if old.len() != next.len() {
            changes.edits.push(Edit {
                old: shared..old.len(),
                new: shared..next.len(),
            });
        }
        for (index, (old, next)) in old.iter().zip(next).enumerate() {
            if old.layout != next.layout {
                mark_dirty(&mut changes.dirty, index);
            }
        }
        return changes;
    }
    let positions = old
        .iter()
        .enumerate()
        .map(|(index, row)| (row.id.as_ref(), index))
        .collect::<HashMap<_, _>>();
    debug_assert_eq!(positions.len(), old.len(), "duplicate transcript row id");
    let mut candidates = Vec::new();
    let mut tails: Vec<usize> = Vec::new();
    let mut predecessors = Vec::new();
    for (new_index, row) in next.iter().enumerate() {
        let Some(&old_index) = positions.get(row.id.as_ref()) else {
            continue;
        };
        let slot = tails.partition_point(|&candidate| {
            let (previous_old, _) = candidates[candidate];
            previous_old < old_index
        });
        let predecessor = slot.checked_sub(1).map(|slot| tails[slot]);
        let candidate = candidates.len();
        candidates.push((old_index, new_index));
        predecessors.push(predecessor);
        if slot == tails.len() {
            tails.push(candidate);
        } else {
            tails[slot] = candidate;
        }
    }
    let mut retained = Vec::with_capacity(tails.len());
    let mut cursor = tails.last().copied();
    while let Some(index) = cursor {
        retained.push(candidates[index]);
        cursor = predecessors[index];
    }
    retained.reverse();

    let mut changes = Changes::default();
    let mut old_start = 0;
    let mut new_start = 0;
    for (old_index, new_index) in retained
        .into_iter()
        .chain(std::iter::once((old.len(), next.len())))
    {
        if old_start != old_index || new_start != new_index {
            changes.edits.push(Edit {
                old: old_start..old_index,
                new: new_start..new_index,
            });
        }
        if old_index < old.len() && old[old_index].layout != next[new_index].layout {
            mark_dirty(&mut changes.dirty, new_index);
        }
        old_start = old_index + 1;
        new_start = new_index + 1;
    }
    changes
}

fn mark_dirty(dirty: &mut Vec<Range<usize>>, index: usize) {
    if let Some(last) = dirty.last_mut()
        && last.end == index
    {
        last.end += 1;
    } else {
        dirty.push(index..index + 1);
    }
}

fn restore_anchor(old: &[RowRevision], next: &[RowRevision], offset: ListOffset) -> ListOffset {
    if let Some(row) = old.get(offset.item_ix)
        && next.get(offset.item_ix).is_some_and(|next| next.id == row.id)
    {
        return offset;
    }
    let positions = next
        .iter()
        .enumerate()
        .map(|(index, row)| (row.id.as_ref(), index))
        .collect::<HashMap<_, _>>();
    let start = offset.item_ix.min(old.len());
    for (index, row) in old.iter().enumerate().skip(start) {
        if let Some(&item_ix) = positions.get(row.id.as_ref()) {
            return ListOffset {
                item_ix,
                offset_in_item: if index == offset.item_ix {
                    offset.offset_in_item
                } else {
                    px(0.)
                },
            };
        }
    }
    for row in old[..start].iter().rev() {
        if let Some(&item_ix) = positions.get(row.id.as_ref()) {
            return ListOffset {
                item_ix,
                offset_in_item: px(0.),
            };
        }
    }
    ListOffset::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, IntoElement, ListAlignment, Render, TestAppContext, Window, div, point,
        prelude::*, size,
    };

    fn rows(ids: &[&str]) -> Vec<RowRevision> {
        ids.iter().map(|id| RowRevision::new((*id).to_owned(), 0)).collect()
    }

    #[test]
    fn inserts_and_removes_at_the_actual_identity_ranges() {
        let old = rows(&["a", "b", "c", "d"]);
        let next = rows(&["history", "a", "tool", "b", "d", "tail"]);
        let changes = reconcile(&old, &next);
        assert_eq!(
            changes.edits,
            vec![
                Edit {
                    old: 0..0,
                    new: 0..1
                },
                Edit {
                    old: 1..1,
                    new: 2..3
                },
                Edit {
                    old: 2..3,
                    new: 4..4
                },
                Edit {
                    old: 4..4,
                    new: 5..6
                },
            ]
        );
        let mut actual = old;
        for edit in changes.edits.iter().rev() {
            actual.splice(edit.old.clone(), next[edit.new.clone()].iter().cloned());
        }
        assert_eq!(actual, next);
    }

    #[test]
    fn changed_content_only_invalidates_its_surviving_measurement() {
        let old = rows(&["a", "b", "c", "d"]);
        let mut next = old.clone();
        next[1].layout = 1;
        next[2].layout = 1;
        assert_eq!(
            reconcile(&old, &next),
            Changes {
                edits: Vec::new(),
                dirty: std::iter::once(1..3).collect()
            }
        );
        assert_eq!(reconcile(&next, &next), Changes::default());
    }

    #[test]
    fn same_count_replacement_and_reorder_preserve_survivors() {
        let old = rows(&["a", "b", "c", "d"]);
        for next in [rows(&["a", "new", "c", "d"]), rows(&["d", "b", "a", "c"])] {
            let changes = reconcile(&old, &next);
            let mut actual = old.clone();
            for edit in changes.edits.iter().rev() {
                actual.splice(edit.old.clone(), next[edit.new.clone()].iter().cloned());
            }
            assert_eq!(actual, next);
            assert!(
                changes
                    .edits
                    .iter()
                    .map(|edit| edit.old.len())
                    .sum::<usize>()
                    < old.len()
            );
        }
    }

    #[test]
    fn every_unique_reordering_and_subset_reconstructs_the_snapshot() {
        fn check(old: &[RowRevision], prefix: &mut Vec<&'static str>, remaining: &[&'static str]) {
            let next = rows(prefix);
            let changes = reconcile(old, &next);
            let mut actual = old.to_vec();
            for edit in changes.edits.iter().rev() {
                actual.splice(edit.old.clone(), next[edit.new.clone()].iter().cloned());
            }
            assert_eq!(actual, next);
            for (index, id) in remaining.iter().enumerate() {
                prefix.push(id);
                let mut tail = remaining.to_vec();
                tail.remove(index);
                check(old, prefix, &tail);
                prefix.pop();
            }
        }
        check(
            &rows(&["a", "b", "c", "d"]),
            &mut Vec::new(),
            &["a", "b", "c", "d", "new"],
        );
    }

    #[test]
    fn deleted_anchor_uses_successor_then_predecessor() {
        let old = rows(&["a", "b", "c", "d"]);
        let anchor = ListOffset {
            item_ix: 1,
            offset_in_item: px(9.),
        };
        let retained = restore_anchor(&old, &rows(&["history", "a", "b", "d"]), anchor);
        assert_eq!((retained.item_ix, retained.offset_in_item), (2, px(9.)));
        let successor = restore_anchor(&old, &rows(&["a", "d"]), anchor);
        assert_eq!((successor.item_ix, successor.offset_in_item), (1, px(0.)));
        let predecessor = restore_anchor(&old, &rows(&["a"]), anchor);
        assert_eq!(
            (predecessor.item_ix, predecessor.offset_in_item),
            (0, px(0.))
        );
        assert_eq!(restore_anchor(&old, &[], anchor).item_ix, 0);
    }

    struct Fixture {
        list: ListState,
        viewport: TranscriptViewport,
        rows: Vec<RowRevision>,
        heights: HashMap<SharedString, f32>,
    }
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let rows = self.rows.clone();
            let heights = self.heights.clone();
            div().size_full().child(
                gpui::list(self.list.clone(), move |index, _, _| {
                    div()
                        .h(px(*heights.get(&rows[index].id).unwrap_or(&40.)))
                        .child(rows[index].id.to_string())
                        .into_any_element()
                })
                .size_full(),
            )
        }
    }

    #[gpui::test]
    fn measured_list_retains_reading_anchor_through_growth_and_middle_edits(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| {
            let list = ListState::new(0, ListAlignment::Top, px(100.)).measure_all();
            let mut viewport = TranscriptViewport::default();
            let rows = rows(&["a", "b", "c", "d", "e", "f", "g", "h"]);
            viewport.sync(&list, rows.clone(), false);
            Fixture {
                list,
                viewport,
                rows,
                heights: HashMap::new(),
            }
        });
        cx.simulate_resize(size(px(300.), px(120.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(120.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        view.update(cx, |view, cx| {
            view.list.scroll_to(ListOffset {
                item_ix: 3,
                offset_in_item: px(7.),
            });
            cx.notify();
        });
        draw(cx);
        view.update(cx, |view, cx| {
            view.heights.insert("b".into(), 100.);
            view.rows[1].layout += 1;
            let changes = view.viewport.sync(&view.list, view.rows.clone(), false);
            assert_eq!(
                changes,
                ChangeSummary {
                    splices: 0,
                    remeasured: 1
                }
            );
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            let top = view.list.logical_scroll_top();
            assert_eq!((top.item_ix, top.offset_in_item), (3, px(7.)));
        });
        view.update(cx, |view, cx| {
            view.rows.insert(0, RowRevision::new("history", 0));
            view.rows.insert(3, RowRevision::new("tool", 0));
            view.viewport.sync(&view.list, view.rows.clone(), false);
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            let top = view.list.logical_scroll_top();
            assert_eq!(view.rows[top.item_ix].id.as_ref(), "d");
            assert_eq!(top.offset_in_item, px(7.));
        });
    }

    #[test]
    fn following_uses_native_tail_and_bootstrap_recovers_external_list_reset() {
        let list = ListState::new(12, ListAlignment::Top, px(100.));
        let mut viewport = TranscriptViewport::default();
        let next = rows(&["a", "b"]);
        viewport.sync(&list, next.clone(), true);
        assert_eq!(list.item_count(), 2);
        assert!(list.is_following_tail());
        list.reset(6);
        viewport.sync(&list, next, false);
        assert_eq!(list.item_count(), 2);
        assert!(!list.is_following_tail());
    }

    #[test]
    fn pending_user_row_does_not_anchor_or_change_existing_follow_intent() {
        let list = ListState::new(0, ListAlignment::Top, px(100.));
        let mut viewport = TranscriptViewport::default();
        viewport.sync(&list, rows(&["old-user", "old-reply"]), false);
        assert!(!viewport.anchor_turn(&list, "not-yet-received", px(24.)));
        assert!(viewport.turn_reservation.is_none());
        assert!(!list.is_following_tail());
    }

    #[gpui::test]
    fn new_turn_keeps_context_then_naturally_follows_a_long_reply(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| {
            let list = ListState::new(0, ListAlignment::Top, px(100.)).measure_all();
            let mut viewport = TranscriptViewport::default();
            let rows = rows(&["a", "b", "c", "new-user"]);
            viewport.sync(&list, rows.clone(), false);
            assert!(viewport.anchor_turn(&list, "new-user", px(24.)));
            // A second invalidation before first layout must not mistake an
            // unknown zero-sized viewport for a fully consumed reservation.
            viewport.sync(&list, rows.clone(), true);
            Fixture {
                list,
                viewport,
                rows,
                heights: HashMap::new(),
            }
        });
        cx.simulate_resize(size(px(300.), px(120.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(120.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        view.read_with(cx, |view, _| {
            let user = view.list.bounds_for_item(3).unwrap();
            assert_eq!(
                user.origin.y - view.list.viewport_bounds().origin.y,
                px(24.)
            );
            assert!(view.list.is_following_tail());
        });
        view.update(cx, |view, cx| {
            view.rows.insert(0, RowRevision::new("history", 0));
            view.rows.push(RowRevision::new("reply", 0));
            view.viewport.sync(&view.list, view.rows.clone(), true);
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            let user = view.list.bounds_for_item(4).unwrap();
            assert_eq!(
                user.origin.y - view.list.viewport_bounds().origin.y,
                px(24.)
            );
            assert!(view.viewport.turn_reservation.is_some());
        });
        view.update(cx, |view, cx| {
            view.heights.insert("reply".into(), 160.);
            view.rows.last_mut().unwrap().layout += 1;
            view.viewport.sync(&view.list, view.rows.clone(), true);
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            let reply = view.list.bounds_for_item(5).unwrap();
            assert_eq!(reply.bottom(), view.list.viewport_bounds().bottom());
            assert!(view.list.tail_reservation_filled());
        });
        view.update(cx, |view, cx| {
            view.viewport.sync(&view.list, view.rows.clone(), true);
            assert!(view.viewport.turn_reservation.is_none());
            assert!(view.list.is_following_tail());
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.list.bounds_for_item(5).unwrap().bottom(),
                view.list.viewport_bounds().bottom()
            );
        });
    }

    #[gpui::test]
    fn manual_reading_retains_short_turn_geometry_and_explicit_latest_releases_it(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| {
            let list = ListState::new(0, ListAlignment::Top, px(100.)).measure_all();
            let mut viewport = TranscriptViewport::default();
            let rows = rows(&["a", "b", "c", "new-user", "reply"]);
            viewport.sync(&list, rows.clone(), false);
            assert!(viewport.anchor_turn(&list, "new-user", px(24.)));
            Fixture {
                list,
                viewport,
                rows,
                heights: HashMap::new(),
            }
        });
        cx.simulate_resize(size(px(300.), px(120.)));
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(120.)), |_, _| {
                view.clone().into_any_element()
            });
        };
        draw(cx);
        view.update(cx, |view, cx| {
            view.list.scroll_to(ListOffset {
                item_ix: 2,
                offset_in_item: px(5.),
            });
            view.viewport.sync(&view.list, view.rows.clone(), false);
            cx.notify();
        });
        draw(cx);
        view.update(cx, |view, cx| {
            view.rows.insert(0, RowRevision::new("older", 0));
            view.viewport.sync(&view.list, view.rows.clone(), false);
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            let top = view.list.logical_scroll_top();
            assert_eq!(view.rows[top.item_ix].id.as_ref(), "c");
            assert_eq!(top.offset_in_item, px(5.));
            assert!(!view.list.is_following_tail());
            assert!(view.viewport.turn_reservation.is_some());
        });
        view.update(cx, |view, cx| {
            view.viewport.release_turn_anchor(&view.list);
            view.viewport.sync(&view.list, view.rows.clone(), true);
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            assert!(view.viewport.turn_reservation.is_none());
            assert!(view.list.is_following_tail());
            let user = view.list.bounds_for_item(4).unwrap();
            assert_eq!(
                user.origin.y - view.list.viewport_bounds().origin.y,
                px(40.)
            );
        });
        view.update(cx, |view, cx| {
            assert!(view.viewport.anchor_turn(&view.list, "new-user", px(24.)));
            view.rows.remove(4);
            view.viewport.sync(&view.list, view.rows.clone(), false);
            assert!(view.viewport.turn_reservation.is_none());
            cx.notify();
        });
        draw(cx);
    }
}
