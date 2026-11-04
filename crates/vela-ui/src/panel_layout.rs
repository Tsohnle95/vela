//! Geometry for the simultaneous agent-panel grid.
//!
//! Mirrors the reference `agent-panels.ts` layout: one or two panels form a
//! single row, three or more form two rows with `ceil(count / 2)` columns. Rows
//! divide the area evenly. An incomplete final row divides the full width among
//! the panels it actually contains.
//!
//! Widths are fractions of the row. Automatic panels split the row evenly; a
//! user resize stores explicit fractions that are renormalized for rendering.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelGrid {
    pub columns: usize,
    pub rows: usize,
}

/// Grid shape for a panel count. Zero panels still report one column and row so
/// callers can render an empty placeholder without dividing by zero.
pub fn panel_grid(count: usize) -> PanelGrid {
    if count < 3 {
        return PanelGrid {
            columns: count.max(1),
            rows: 1,
        };
    }
    PanelGrid {
        columns: count.div_ceil(2),
        rows: 2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelSlot {
    /// Percentage of the tiled area, 0..100.
    pub left: f32,
    pub width: f32,
    pub top: f32,
    pub height: f32,
}

/// Slot geometry for `index` of `count` panels, in percentages.
pub fn panel_slot(count: usize, index: usize) -> Option<PanelSlot> {
    if index >= count || count == 0 {
        return None;
    }
    let grid = panel_grid(count);
    let row = index / grid.columns;
    let column = index % grid.columns;
    let row_columns = grid.columns.min(count - row * grid.columns).max(1);
    let left = 100. * column as f32 / row_columns as f32;
    let right = 100. * (column + 1) as f32 / row_columns as f32;
    Some(PanelSlot {
        left,
        width: right - left,
        top: 100. * row as f32 / grid.rows as f32,
        height: 100. / grid.rows as f32,
    })
}

/// Inclusive-exclusive index ranges for each grid row.
pub fn panel_rows(count: usize) -> Vec<std::ops::Range<usize>> {
    let grid = panel_grid(count);
    (0..grid.rows)
        .map(|row| {
            let start = (row * grid.columns).min(count);
            let end = ((row + 1) * grid.columns).min(count);
            start..end
        })
        .filter(|range| !range.is_empty())
        .collect()
}

/// Normalize stored fractions to exactly fill the row. Panels without an
/// explicit width are `None` and share the remaining space evenly.
pub fn normalize_widths(widths: &[Option<f32>]) -> Vec<f32> {
    let count = widths.len();
    if count == 0 {
        return Vec::new();
    }
    let explicit: f32 = widths.iter().flatten().map(|width| width.max(0.0)).sum();
    let automatic = widths.iter().filter(|width| width.is_none()).count();
    if explicit <= 0.0 {
        return vec![1.0 / count as f32; count];
    }
    if automatic == 0 {
        return widths
            .iter()
            .map(|width| width.unwrap_or(0.0).max(0.0) / explicit)
            .collect();
    }
    let remaining = (1.0 - explicit).max(0.0);
    let share = (remaining / automatic as f32).max(1.0 / count as f32);
    let explicit_target = (1.0 - share * automatic as f32).max(0.0);
    widths
        .iter()
        .map(|width| match width {
            Some(width) => width.max(0.0) / explicit * explicit_target,
            None => share,
        })
        .collect()
}

/// Drag the boundary between `boundary` and `boundary + 1` by `delta` row
/// fractions. Panels never shrink below `MIN_PANEL_FRACTION` of the row.
pub fn resize_widths(widths: &mut [f32], boundary: usize, delta: f32) -> bool {
    const MIN_PANEL_FRACTION: f32 = 0.15;
    if boundary >= widths.len().saturating_sub(1) {
        return false;
    }
    let total = widths[boundary] + widths[boundary + 1];
    if total <= 0.0 {
        return false;
    }
    let requested = (widths[boundary] + delta).clamp(
        MIN_PANEL_FRACTION.min(total / 2.0),
        total - MIN_PANEL_FRACTION.min(total / 2.0),
    );
    if (requested - widths[boundary]).abs() < f32::EPSILON {
        return false;
    }
    widths[boundary] = requested;
    widths[boundary + 1] = total - requested;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_matches_the_reference_shapes() {
        assert_eq!(
            panel_grid(0),
            PanelGrid {
                columns: 1,
                rows: 1
            }
        );
        assert_eq!(
            panel_grid(1),
            PanelGrid {
                columns: 1,
                rows: 1
            }
        );
        assert_eq!(
            panel_grid(2),
            PanelGrid {
                columns: 2,
                rows: 1
            }
        );
        assert_eq!(
            panel_grid(3),
            PanelGrid {
                columns: 2,
                rows: 2
            }
        );
        assert_eq!(
            panel_grid(4),
            PanelGrid {
                columns: 2,
                rows: 2
            }
        );
        assert_eq!(
            panel_grid(5),
            PanelGrid {
                columns: 3,
                rows: 2
            }
        );
        assert_eq!(
            panel_grid(8),
            PanelGrid {
                columns: 4,
                rows: 2
            }
        );
    }

    #[test]
    fn incomplete_rows_divide_the_full_width_without_overlap() {
        for count in 1..=8 {
            for row in panel_rows(count) {
                let mut left = 0.0;
                let mut width_sum = 0.0;
                for index in row.clone() {
                    let slot = panel_slot(count, index).unwrap();
                    assert!(
                        (slot.left - left).abs() < 1e-4,
                        "{count} panels row {row:?}"
                    );
                    assert!(slot.width > 0.0);
                    left += slot.width;
                    width_sum += slot.width;
                }
                assert!((width_sum - 100.0).abs() < 1e-3);
            }
        }
        let last = panel_slot(5, 4).unwrap();
        assert!((last.top - 50.0).abs() < f32::EPSILON);
        assert!((last.width - 100.0 / 2.0).abs() < 1e-3);
        assert!(panel_slot(5, 5).is_none());
    }

    #[test]
    fn automatic_and_explicit_widths_fill_the_row() {
        assert_eq!(normalize_widths(&[None, None]), vec![0.5, 0.5]);
        let widths = normalize_widths(&[Some(0.4), None]);
        assert!((widths.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        assert!((widths[0] - 0.4).abs() < 1e-5);
        assert!((widths[1] - 0.6).abs() < 1e-5);
        let widths = normalize_widths(&[Some(2.0), Some(2.0)]);
        assert_eq!(widths, vec![0.5, 0.5]);
    }

    #[test]
    fn resizing_respects_minimums_and_noops_at_the_edge() {
        let mut widths = vec![0.5, 0.5];
        assert!(resize_widths(&mut widths, 0, 0.1));
        assert!((widths[0] - 0.6).abs() < 1e-5);
        assert!((widths[1] - 0.4).abs() < 1e-5);
        assert!(resize_widths(&mut widths, 0, 10.0));
        assert!((widths[0] - 0.85).abs() < 1e-5);
        assert!((widths[1] - 0.15).abs() < 1e-5);
        assert!(!resize_widths(&mut widths, 0, 1.0));
        assert!(!resize_widths(&mut widths, 1, 0.1));
        assert!(!resize_widths(&mut widths, 5, 0.1));
    }
}
