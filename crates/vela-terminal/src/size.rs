//! Source-compatible terminal viewport fitting, bounded by its IPC dimensions.
pub const MAX_COLUMNS: u16 = 1000;
pub const MAX_ROWS: u16 = 500;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}
pub fn fit(width: f32, height: f32, cell_width: f32, cell_height: f32) -> Option<Size> {
    if ![width, height, cell_width, cell_height]
        .iter()
        .all(|value| value.is_finite() && *value > 0.)
    {
        return None;
    }
    Some(Size {
        cols: (((width - 30.) / cell_width).floor() - 1.).clamp(2., MAX_COLUMNS as f32) as u16,
        rows: ((height - 12.) / cell_height)
            .floor()
            .clamp(1., MAX_ROWS as f32) as u16,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn viewport_fitting_matches_source_padding_ruler_and_spare_column() {
        assert_eq!(
            fit(900., 400., 7.2, 15.),
            Some(Size {
                cols: 119,
                rows: 25
            })
        );
        assert_eq!(fit(294., 88., 7.2, 15.), Some(Size { cols: 35, rows: 5 }));
        assert_eq!(fit(20., 5., 7.2, 15.), Some(Size { cols: 2, rows: 1 }));
        assert_eq!(
            fit(100_000., 100_000., 7.2, 15.),
            Some(Size {
                cols: 1000,
                rows: 500
            })
        );
    }
    #[test]
    fn hidden_or_unmeasurable_viewports_do_not_resize_the_shell() {
        for values in [
            [0., 88., 7.2, 15.],
            [294., 0., 7.2, 15.],
            [294., 88., 0., 15.],
            [f32::NAN, 88., 7.2, 15.],
            [294., f32::INFINITY, 7.2, 15.],
        ] {
            assert!(fit(values[0], values[1], values[2], values[3]).is_none());
        }
    }
}
