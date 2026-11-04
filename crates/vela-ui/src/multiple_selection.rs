//! Selection gestures preserve DOM option order, independently of click order.
use std::collections::BTreeSet;
#[derive(Default)]
pub(crate) struct MultipleSelection {
    pub active: Option<usize>,
    anchor: Option<usize>,
}
impl MultipleSelection {
    /// Move the focused option without altering the selection, matching the
    /// native multi-select arrow/Home/End behavior. The new position becomes the
    /// anchor for a following Shift extension.
    pub fn focus(&mut self, index: usize) {
        self.active = Some(index);
        self.anchor = Some(index);
    }
    pub fn choose(
        &mut self,
        selected: &mut BTreeSet<usize>,
        index: usize,
        shift: bool,
        additive: bool,
    ) {
        if shift {
            let anchor = *self
                .anchor
                .get_or_insert_with(|| selected.first().copied().unwrap_or(index));
            if !additive {
                selected.clear();
            }
            selected.extend(anchor.min(index)..=anchor.max(index));
        } else {
            self.anchor = Some(index);
            if additive {
                if !selected.remove(&index) {
                    selected.insert(index);
                }
            } else {
                selected.clear();
                selected.insert(index);
            }
        }
        self.active = Some(index);
    }
}
#[cfg(test)]
mod tests {
    use super::MultipleSelection;
    use std::collections::BTreeSet;
    #[test]
    fn ranges_keep_their_anchor_and_toggles_preserve_option_order() {
        let mut state = MultipleSelection::default();
        let mut selected = BTreeSet::from([0]);
        state.choose(&mut selected, 1, false, false);
        state.choose(&mut selected, 3, true, false);
        assert_eq!(selected, BTreeSet::from([1, 2, 3]));
        state.choose(&mut selected, 2, true, false);
        assert_eq!(selected, BTreeSet::from([1, 2]));
        state.choose(&mut selected, 0, false, true);
        state.choose(&mut selected, 2, false, true);
        assert_eq!(selected.into_iter().collect::<Vec<_>>(), [0, 1]);
    }

    #[test]
    fn focus_moves_without_selecting_and_anchors_shift_ranges() {
        let mut state = MultipleSelection::default();
        let mut selected = BTreeSet::from([0]);
        state.focus(2);
        assert_eq!(state.active, Some(2));
        assert_eq!(selected, BTreeSet::from([0]));
        state.choose(&mut selected, 3, true, false);
        assert_eq!(selected, BTreeSet::from([2, 3]));
        state.focus(1);
        state.choose(&mut selected, 1, false, true);
        assert_eq!(selected, BTreeSet::from([1, 2, 3]));
    }
}
