use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS, PADDING, PADDING_TOP};
use crate::input::CellMetrics;

#[derive(Debug, Clone)]
pub struct WindowSettings {
    pub cols: u64,
    pub rows: u64,
    pub cell_metrics: CellMetrics,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            cell_metrics: CellMetrics {
                padding_x: PADDING as f64,
                padding_y: PADDING_TOP as f64,
                ..Default::default()
            },
        }
    }
}

impl WindowSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_padding(&mut self, scale_factor: f64) {
        self.cell_metrics.padding_x = (PADDING as f64 * scale_factor).round();
        self.cell_metrics.padding_y = (PADDING_TOP as f64 * scale_factor).round();
    }

    pub fn calculate_grid_size(&self, width: u32, height: u32) -> (u64, u64) {
        let cols = (width as f64 - 2.0 * self.cell_metrics.padding_x).max(0.0)
            / self.cell_metrics.cell_width;
        let rows = (height as f64 - (self.cell_metrics.padding_y + self.cell_metrics.padding_x))
            .max(0.0)
            / self.cell_metrics.cell_height;
        (cols.max(1.0) as u64, rows.max(1.0) as u64)
    }
}
