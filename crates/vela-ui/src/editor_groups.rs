//! Editor group layout for up to four simultaneous source panes.
//!
//! Mirrors the reference editor grid: two groups sit side by side, three use a
//! full-width second row, and four use a 2x2 grid. Percent cells keep the
//! layout deterministic and testable; dividers adjust the shared X/Y weights.

use vela_core::document::DocumentKey;

pub(super) const MAX_EDITOR_GROUPS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct GroupCell {
    /// Percent of the group area, 0..100.
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// Percent geometry for each group. Splits clamp in the caller, so cells stay
/// fractional even when a resize hits its minimum.
pub(super) fn group_cells(count: usize, split_x: f32, split_y: f32) -> Vec<GroupCell> {
    let x = split_x.clamp(0.1, 0.9) * 100.;
    let y = split_y.clamp(0.1, 0.9) * 100.;
    match count {
        0 | 1 => vec![GroupCell {
            left: 0.,
            top: 0.,
            width: 100.,
            height: 100.,
        }],
        2 => vec![
            GroupCell {
                left: 0.,
                top: 0.,
                width: x,
                height: 100.,
            },
            GroupCell {
                left: x,
                top: 0.,
                width: 100. - x,
                height: 100.,
            },
        ],
        3 => vec![
            GroupCell {
                left: 0.,
                top: 0.,
                width: x,
                height: y,
            },
            GroupCell {
                left: x,
                top: 0.,
                width: 100. - x,
                height: y,
            },
            GroupCell {
                left: 0.,
                top: y,
                width: 100.,
                height: 100. - y,
            },
        ],
        _ => vec![
            GroupCell {
                left: 0.,
                top: 0.,
                width: x,
                height: y,
            },
            GroupCell {
                left: x,
                top: 0.,
                width: 100. - x,
                height: y,
            },
            GroupCell {
                left: 0.,
                top: y,
                width: x,
                height: 100. - y,
            },
            GroupCell {
                left: x,
                top: y,
                width: 100. - x,
                height: 100. - y,
            },
        ],
    }
}

/// The next open tab after the active one that no editor group displays yet.
/// Returns `None` when every tab is already visible, so a new group starts
/// empty rather than duplicating a document.
pub(super) fn next_group_document(
    active: Option<&DocumentKey>,
    tabs: &[DocumentKey],
    assigned: &[DocumentKey],
) -> Option<DocumentKey> {
    let count = tabs.len();
    if count == 0 {
        return None;
    }
    let active_index = active.and_then(|active| tabs.iter().position(|key| key == active));
    for offset in 0..count {
        let index = match active_index {
            Some(active) => (active + 1 + offset) % count,
            None => offset,
        };
        let key = &tabs[index];
        if Some(key) == active || assigned.iter().any(|assigned| assigned == key) {
            continue;
        }
        return Some(key.clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> DocumentKey {
        DocumentKey::new("/workspace".into(), name.into())
    }

    #[test]
    fn grid_geometry_matches_the_reference_shapes() {
        assert_eq!(group_cells(1, 0.5, 0.5).len(), 1);
        let two = group_cells(2, 0.5, 0.5);
        assert_eq!(two.len(), 2);
        assert!((two[0].width - 50.).abs() < 1e-4);
        assert!((two[1].left - 50.).abs() < 1e-4);
        assert!((two.iter().map(|cell| cell.width).sum::<f32>() - 100.).abs() < 1e-4);

        let three = group_cells(3, 0.4, 0.6);
        assert_eq!(three.len(), 3);
        assert!((three[0].width - 40.).abs() < 1e-4);
        assert!((three[1].left - 40.).abs() < 1e-4);
        assert!((three[1].width - 60.).abs() < 1e-4);
        assert!((three[2].top - 60.).abs() < 1e-4);
        assert_eq!(three[2].width, 100., "the third group spans the full row");

        let four = group_cells(4, 0.55, 0.45);
        assert_eq!(four.len(), 4);
        assert!((four[3].left - 55.).abs() < 1e-4);
        assert!((four[3].top - 45.).abs() < 1e-4);
        assert!((four[3].width - 45.).abs() < 1e-4);
        assert!((four[3].height - 55.).abs() < 1e-4);
        assert_eq!(group_cells(0, 0.5, 0.5).len(), 1);
    }

    #[test]
    fn next_group_targets_the_following_unassigned_tab_and_wraps() {
        let (a, b, c) = (key("a.rs"), key("b.rs"), key("c.rs"));
        let tabs = vec![a.clone(), b.clone(), c.clone()];
        assert_eq!(next_group_document(Some(&a), &tabs, &[]), Some(b.clone()));
        assert_eq!(
            next_group_document(Some(&a), &tabs, std::slice::from_ref(&b)),
            Some(c.clone())
        );
        assert_eq!(
            next_group_document(Some(&a), &tabs, &[b.clone(), c.clone()]),
            None,
            "every tab is already visible"
        );
        assert_eq!(
            next_group_document(Some(&c), &tabs, &[]),
            Some(a),
            "targeting wraps after the final tab"
        );
        assert_eq!(
            next_group_document(None, std::slice::from_ref(&b), &[]),
            Some(b.clone()),
            "without an active tab the first unassigned tab wins"
        );
        assert_eq!(
            next_group_document(Some(&b), std::slice::from_ref(&b), &[]),
            None
        );
        assert_eq!(next_group_document(None, &[], &[]), None);
    }
}
