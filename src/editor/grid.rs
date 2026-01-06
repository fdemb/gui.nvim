use super::cell::Cell;

/// A 2D grid of cells representing a Neovim window or the main screen.
///
/// The grid stores cells in row-major order for efficient row-based operations,
/// which aligns with how Neovim sends `grid_line` events.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Grid ID (1 is the main grid, others are for multigrid extension).
    #[allow(dead_code)]
    pub id: u64,
    /// Number of columns.
    width: usize,
    /// Number of rows.
    height: usize,
    /// Cell storage in row-major order.
    cells: Vec<Cell>,
}

impl Grid {
    /// Creates a new grid with the given dimensions.
    pub fn new(id: u64, width: usize, height: usize) -> Self {
        let cells = vec![Cell::default(); width * height];
        Self {
            id,
            width,
            height,
            cells,
        }
    }

    /// Returns the width (number of columns) of the grid.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height (number of rows) of the grid.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the total number of cells in the grid.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Returns true if the grid is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Gets a reference to the cell at (row, col).
    /// Returns None if coordinates are out of bounds.
    pub fn get(&self, row: usize, col: usize) -> Option<&Cell> {
        if row < self.height && col < self.width {
            Some(&self.cells[row * self.width + col])
        } else {
            None
        }
    }

    /// Gets a mutable reference to the cell at (row, col).
    /// Returns None if coordinates are out of bounds.
    #[allow(dead_code)]
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        if row < self.height && col < self.width {
            Some(&mut self.cells[row * self.width + col])
        } else {
            None
        }
    }

    /// Returns a slice of the cells in the given row.
    #[allow(dead_code)]
    pub fn row(&self, row: usize) -> Option<&[Cell]> {
        if row < self.height {
            let start = row * self.width;
            Some(&self.cells[start..start + self.width])
        } else {
            None
        }
    }

    /// Returns a mutable slice of the cells in the given row.
    #[allow(dead_code)]
    pub fn row_mut(&mut self, row: usize) -> Option<&mut [Cell]> {
        if row < self.height {
            let start = row * self.width;
            Some(&mut self.cells[start..start + self.width])
        } else {
            None
        }
    }

    /// Returns an iterator over all rows.
    #[allow(dead_code)]
    pub fn rows(&self) -> impl Iterator<Item = &[Cell]> {
        self.cells.chunks(self.width)
    }

    /// Clears all cells to their default state.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }

    /// Resizes the grid to new dimensions.
    /// New cells are initialized to default, existing cells are preserved where possible.
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        if new_width == self.width && new_height == self.height {
            return;
        }

        let mut new_cells = vec![Cell::default(); new_width * new_height];

        // Copy existing cells that fit in the new dimensions
        let copy_width = self.width.min(new_width);
        let copy_height = self.height.min(new_height);

        for row in 0..copy_height {
            let old_start = row * self.width;
            let new_start = row * new_width;
            for col in 0..copy_width {
                new_cells[new_start + col] = self.cells[old_start + col].clone();
            }
        }

        self.width = new_width;
        self.height = new_height;
        self.cells = new_cells;
    }

    /// Processes a `grid_line` event from Neovim.
    ///
    /// Updates cells starting at (row, col_start) with the provided cell data.
    /// Each item in `cells` is (text, highlight_id, repeat_count).
    /// If highlight_id is None, the previous highlight is reused.
    pub fn update_line(
        &mut self,
        row: usize,
        col_start: usize,
        cells: &[(String, Option<u64>, usize)],
    ) {
        if row >= self.height {
            return;
        }

        let mut col = col_start;
        let mut last_hl_id: u64 = 0;

        for (text, hl_id, repeat) in cells {
            let hl_id = hl_id.unwrap_or(last_hl_id);
            last_hl_id = hl_id;

            let is_wide_spacer = text.is_empty();

            for _ in 0..*repeat.max(&1) {
                if col >= self.width {
                    break;
                }

                let cell = &mut self.cells[row * self.width + col];
                if is_wide_spacer {
                    cell.text.clear();
                    cell.set_wide_spacer(true);
                    cell.set_wide(false);
                } else {
                    cell.text.clone_from(text);
                    cell.flags = super::cell::CellFlags::empty();
                }
                cell.highlight_id = hl_id;
                col += 1;
            }
        }
    }

    pub fn scroll(&mut self, top: usize, bot: usize, left: usize, right: usize, rows: i64) {
        if top >= bot || left >= right || top >= self.height || left >= self.width {
            return;
        }

        let bot = bot.min(self.height);
        let right = right.min(self.width);
        let region_height = bot - top;

        if rows.unsigned_abs() as usize >= region_height {
            for row in top..bot {
                for col in left..right {
                    self.cells[row * self.width + col].clear();
                }
            }
            return;
        }

        if rows > 0 {
            let rows = rows as usize;
            for row in top..(bot - rows) {
                for col in left..right {
                    let src_idx = (row + rows) * self.width + col;
                    let dst_idx = row * self.width + col;
                    self.cells[dst_idx] = self.cells[src_idx].clone();
                }
            }
            for row in (bot - rows)..bot {
                for col in left..right {
                    self.cells[row * self.width + col].clear();
                }
            }
        } else {
            let rows = (-rows) as usize;
            for row in ((top + rows)..bot).rev() {
                for col in left..right {
                    let src_idx = (row - rows) * self.width + col;
                    let dst_idx = row * self.width + col;
                    self.cells[dst_idx] = self.cells[src_idx].clone();
                }
            }
            for row in top..(top + rows) {
                for col in left..right {
                    self.cells[row * self.width + col].clear();
                }
            }
        }
    }
}

impl std::ops::Index<(usize, usize)> for Grid {
    type Output = Cell;

    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        &self.cells[row * self.width + col]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Grid {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        &mut self.cells[row * self.width + col]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_new() {
        let grid = Grid::new(1, 80, 24);
        assert_eq!(grid.id, 1);
        assert_eq!(grid.width(), 80);
        assert_eq!(grid.height(), 24);
        assert_eq!(grid.len(), 80 * 24);
    }

    #[test]
    fn test_grid_get() {
        let grid = Grid::new(1, 10, 5);

        assert!(grid.get(0, 0).is_some());
        assert!(grid.get(4, 9).is_some());
        assert!(grid.get(5, 0).is_none()); // row out of bounds
        assert!(grid.get(0, 10).is_none()); // col out of bounds
    }

    #[test]
    fn test_grid_get_mut() {
        let mut grid = Grid::new(1, 10, 5);

        if let Some(cell) = grid.get_mut(0, 0) {
            cell.text = "x".into();
            cell.highlight_id = 5;
        }

        assert_eq!(grid.get(0, 0).unwrap().text, "x");
        assert_eq!(grid.get(0, 0).unwrap().highlight_id, 5);
    }

    #[test]
    fn test_grid_index() {
        let mut grid = Grid::new(1, 10, 5);
        grid[(2, 3)].text = "y".into();

        assert_eq!(grid[(2, 3)].text, "y");
    }

    #[test]
    fn test_grid_row() {
        let mut grid = Grid::new(1, 3, 2);
        grid[(0, 0)].text = "a".into();
        grid[(0, 1)].text = "b".into();
        grid[(0, 2)].text = "c".into();

        let row = grid.row(0).unwrap();
        assert_eq!(row.len(), 3);
        assert_eq!(row[0].text, "a");
        assert_eq!(row[1].text, "b");
        assert_eq!(row[2].text, "c");
    }

    #[test]
    fn test_grid_clear() {
        let mut grid = Grid::new(1, 5, 5);
        grid[(0, 0)].text = "x".into();
        grid[(0, 0)].highlight_id = 5;

        grid.clear();

        assert_eq!(grid[(0, 0)].text, " ");
        assert_eq!(grid[(0, 0)].highlight_id, 0);
    }

    #[test]
    fn test_grid_resize_larger() {
        let mut grid = Grid::new(1, 3, 2);
        grid[(0, 0)].text = "a".into();
        grid[(1, 2)].text = "b".into();

        grid.resize(5, 4);

        assert_eq!(grid.width(), 5);
        assert_eq!(grid.height(), 4);
        assert_eq!(grid[(0, 0)].text, "a");
        assert_eq!(grid[(1, 2)].text, "b");
        assert_eq!(grid[(3, 4)].text, " "); // New cell
    }

    #[test]
    fn test_grid_resize_smaller() {
        let mut grid = Grid::new(1, 5, 5);
        grid[(0, 0)].text = "a".into();
        grid[(4, 4)].text = "b".into();

        grid.resize(3, 3);

        assert_eq!(grid.width(), 3);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid[(0, 0)].text, "a");
        // (4, 4) is now out of bounds
    }

    #[test]
    fn test_grid_update_line() {
        let mut grid = Grid::new(1, 10, 5);

        // Simulate: ["a", 1], ["b", 1], ["c", 2]
        let cells = vec![
            ("a".into(), Some(1), 1),
            ("b".into(), None, 1),    // reuses hl_id 1
            ("c".into(), Some(2), 1), // new hl_id
        ];

        grid.update_line(0, 0, &cells);

        assert_eq!(grid[(0, 0)].text, "a");
        assert_eq!(grid[(0, 0)].highlight_id, 1);
        assert_eq!(grid[(0, 1)].text, "b");
        assert_eq!(grid[(0, 1)].highlight_id, 1);
        assert_eq!(grid[(0, 2)].text, "c");
        assert_eq!(grid[(0, 2)].highlight_id, 2);
    }

    #[test]
    fn test_grid_update_line_with_repeat() {
        let mut grid = Grid::new(1, 10, 5);

        // Simulate: [" ", 0, 5] (5 spaces)
        let cells = vec![(" ".into(), Some(0), 5)];

        grid.update_line(0, 2, &cells);

        for col in 2..7 {
            assert_eq!(grid[(0, col)].text, " ");
            assert_eq!(grid[(0, col)].highlight_id, 0);
        }
    }

    #[test]
    fn test_grid_scroll_down() {
        let mut grid = Grid::new(1, 5, 5);
        grid[(0, 0)].text = "a".into();
        grid[(1, 0)].text = "b".into();
        grid[(2, 0)].text = "c".into();

        // Scroll down by 1 (content moves up)
        grid.scroll(0, 5, 0, 5, 1);

        assert_eq!(grid[(0, 0)].text, "b");
        assert_eq!(grid[(1, 0)].text, "c");
        assert_eq!(grid[(4, 0)].text, " "); // Cleared
    }

    #[test]
    fn test_grid_scroll_up() {
        let mut grid = Grid::new(1, 5, 5);
        grid[(2, 0)].text = "a".into();
        grid[(3, 0)].text = "b".into();
        grid[(4, 0)].text = "c".into();

        // Scroll up by 1 (content moves down)
        grid.scroll(0, 5, 0, 5, -1);

        assert_eq!(grid[(0, 0)].text, " "); // Cleared
        assert_eq!(grid[(3, 0)].text, "a");
        assert_eq!(grid[(4, 0)].text, "b");
    }

    #[test]
    fn test_grid_scroll_region() {
        let mut grid = Grid::new(1, 10, 10);
        grid[(2, 2)].text = "x".into();
        grid[(3, 2)].text = "y".into();

        // Scroll only a region
        grid.scroll(2, 5, 2, 8, 1);

        assert_eq!(grid[(2, 2)].text, "y");
        assert_eq!(grid[(4, 2)].text, " "); // Cleared
    }

    #[test]
    fn test_grid_rows_iterator() {
        let grid = Grid::new(1, 3, 2);
        let rows: Vec<_> = grid.rows().collect();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 3);
        assert_eq!(rows[1].len(), 3);
    }

    #[test]
    fn test_grid_update_line_clears_flags() {
        let mut grid = Grid::new(1, 10, 5);

        // First, simulate a wide spacer being set
        grid[(0, 1)].set_wide_spacer(true);
        assert!(grid[(0, 1)].is_wide_spacer());
        assert!(!grid[(0, 1)].is_empty()); // Spacer flag means not empty

        // Now update the line with regular text
        let cells = vec![("x".into(), Some(1), 1)];
        grid.update_line(0, 1, &cells);

        // After update, flags should be cleared
        assert!(!grid[(0, 1)].is_wide_spacer());
        assert_eq!(grid[(0, 1)].text, "x");
        // Cell with text "x" and hl_id 1 should not be empty
        assert!(!grid[(0, 1)].is_empty());
    }

    #[test]
    fn test_grid_update_line_space_with_cleared_flags() {
        let mut grid = Grid::new(1, 10, 5);

        // Set wide spacer flag
        grid[(0, 0)].set_wide_spacer(true);
        assert!(grid[(0, 0)].is_wide_spacer());

        // Update with a space and default highlight
        let cells = vec![(" ".into(), Some(0), 1)];
        grid.update_line(0, 0, &cells);

        // After update, cell should be empty (space with hl_id 0 and no flags)
        assert!(!grid[(0, 0)].is_wide_spacer());
        assert_eq!(grid[(0, 0)].text, " ");
        assert!(grid[(0, 0)].is_empty());
    }
}
