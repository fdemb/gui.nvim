use std::collections::HashMap;

use super::grid::Grid;
#[cfg(test)]
use super::highlight::StyleFlags;
use super::highlight::{Color, HighlightAttributes, HighlightMap};

/// Cursor shape as defined by Neovim's mode_info_set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorShape {
    #[default]
    Block,
    Horizontal,
    Vertical,
}

/// Information about a cursor mode.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModeInfo {
    pub cursor_shape: CursorShape,
    pub cell_percentage: u8,
    pub attr_id: u64,
    pub blink_wait: u32,
    pub blink_on: u32,
    pub blink_off: u32,
}

/// The cursor's current state.
#[derive(Debug, Clone)]
pub struct Cursor {
    pub grid: u64,
    pub row: usize,
    pub col: usize,
    pub visible: bool,
    /// Whether the cursor is currently visible due to blinking.
    pub blink_visible: bool,
    /// Timestamp (ms) of the last blink reset (e.g. cursor movement).
    pub last_blink_time: u64,
    /// Whether a blink reset is pending (waiting for the next update).
    pub blink_reset_pending: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            grid: 1,
            row: 0,
            col: 0,
            visible: true,
            blink_visible: true,
            last_blink_time: 0,
            blink_reset_pending: false,
        }
    }
}

/// Central container for all editor state.
///
/// This struct holds the complete state needed to render the Neovim UI:
/// - Grids: The 2D character grids (main grid and floating windows)
/// - Highlights: Color and style definitions from hl_attr_define
/// - Cursor: Position and mode information
#[derive(Debug)]
pub struct EditorState {
    /// All active grids (main grid is ID 1).
    grids: HashMap<u64, Grid>,
    /// Highlight definitions.
    pub highlights: HighlightMap,
    /// Cursor state.
    pub cursor: Cursor,
    /// Mode definitions from mode_info_set.
    modes: Vec<ModeInfo>,
    /// Current mode index.
    current_mode: usize,
    /// Default grid dimensions (columns x rows).
    #[allow(dead_code)]
    default_cols: usize,
    #[allow(dead_code)]
    default_rows: usize,
}

impl EditorState {
    /// Creates a new editor state with default dimensions.
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut grids = HashMap::new();
        grids.insert(1, Grid::new(1, cols, rows));

        Self {
            grids,
            highlights: HighlightMap::new(),
            cursor: Cursor {
                grid: 1,
                visible: true,
                ..Default::default()
            },
            modes: vec![ModeInfo::default()],
            current_mode: 0,
            default_cols: cols,
            default_rows: rows,
        }
    }

    /// Returns the main grid (ID 1).
    pub fn main_grid(&self) -> &Grid {
        self.grids.get(&1).expect("main grid always exists")
    }

    /// Returns a mutable reference to the main grid.
    #[allow(dead_code)]
    pub fn main_grid_mut(&mut self) -> &mut Grid {
        self.grids.get_mut(&1).expect("main grid always exists")
    }

    /// Returns a grid by ID.
    #[allow(dead_code)]
    pub fn grid(&self, id: u64) -> Option<&Grid> {
        self.grids.get(&id)
    }

    /// Returns a mutable reference to a grid by ID.
    #[allow(dead_code)]
    pub fn grid_mut(&mut self, id: u64) -> Option<&mut Grid> {
        self.grids.get_mut(&id)
    }

    /// Returns the current mode info.
    pub fn current_mode(&self) -> &ModeInfo {
        self.modes.get(self.current_mode).unwrap_or(&self.modes[0])
    }

    /// Handles a grid_resize event.
    pub fn grid_resize(&mut self, grid_id: u64, width: usize, height: usize) {
        if let Some(grid) = self.grids.get_mut(&grid_id) {
            grid.resize(width, height);
        } else {
            self.grids
                .insert(grid_id, Grid::new(grid_id, width, height));
        }
    }

    /// Handles a grid_clear event.
    pub fn grid_clear(&mut self, grid_id: u64) {
        if let Some(grid) = self.grids.get_mut(&grid_id) {
            grid.clear();
        }
    }

    /// Handles a grid_line event.
    pub fn grid_line(
        &mut self,
        grid_id: u64,
        row: usize,
        col_start: usize,
        cells: &[(String, Option<u64>, usize)],
    ) {
        if let Some(grid) = self.grids.get_mut(&grid_id) {
            grid.update_line(row, col_start, cells);
        }
    }

    /// Handles a grid_scroll event.
    pub fn grid_scroll(
        &mut self,
        grid_id: u64,
        top: usize,
        bot: usize,
        left: usize,
        right: usize,
        rows: i64,
    ) {
        if let Some(grid) = self.grids.get_mut(&grid_id) {
            grid.scroll(top, bot, left, right, rows);
        }
    }

    /// Handles a grid_cursor_goto event.
    pub fn grid_cursor_goto(&mut self, grid_id: u64, row: usize, col: usize) {
        self.cursor.grid = grid_id;
        self.cursor.row = row;
        self.cursor.col = col;
        self.reset_blink();
    }

    /// Handles a hl_attr_define event.
    pub fn hl_attr_define(&mut self, id: u64, attrs: HighlightAttributes) {
        self.highlights.define(id, attrs);
    }

    /// Handles a default_colors_set event.
    pub fn default_colors_set(&mut self, fg: u32, bg: u32, sp: u32) {
        self.highlights.set_defaults(
            Color::from_u24(fg),
            Color::from_u24(bg),
            Color::from_u24(sp),
        );
    }

    /// Handles a mode_info_set event.
    pub fn mode_info_set(&mut self, modes: Vec<ModeInfo>) {
        self.modes = modes;
        if self.modes.is_empty() {
            self.modes.push(ModeInfo::default());
        }
        self.reset_blink();
    }

    /// Handles a mode_change event.
    pub fn mode_change(&mut self, _mode: &str, mode_idx: usize) {
        self.current_mode = mode_idx;
        self.reset_blink();
    }

    /// Resets the blink timer (e.g. on cursor move).
    pub fn reset_blink(&mut self) {
        self.cursor.blink_reset_pending = true;
        self.cursor.blink_visible = true;
    }

    /// Updates the cursor blink state based on the current time (ms).
    /// Returns true if the blink state changed (requiring a redraw).
    pub fn update_blink(&mut self, now: u64) -> bool {
        let old_visible = self.cursor.blink_visible;

        if self.cursor.blink_reset_pending {
            self.cursor.last_blink_time = now;
            self.cursor.blink_reset_pending = false;
            self.cursor.blink_visible = true;
            return old_visible != true;
        }

        let mode = self.current_mode();
        if mode.blink_on == 0 || mode.blink_off == 0 {
            self.cursor.blink_visible = true;
            return old_visible != true;
        }

        let elapsed = now.saturating_sub(self.cursor.last_blink_time);
        let wait = mode.blink_wait as u64;

        if elapsed < wait {
            self.cursor.blink_visible = true;
            return old_visible != true;
        }

        let blink_elapsed = elapsed - wait;
        let cycle = (mode.blink_on + mode.blink_off) as u64;

        if cycle == 0 {
            self.cursor.blink_visible = true;
            return old_visible != true;
        }

        let phase = blink_elapsed % cycle;
        self.cursor.blink_visible = phase < (mode.blink_on as u64);

        self.cursor.blink_visible != old_visible
    }

    /// Handles a flush event (marks end of a batch of updates).
    pub fn flush(&mut self) {
        // Currently a no-op, but could trigger a render request
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let state = EditorState::new(80, 24);
        assert_eq!(state.main_grid().width(), 80);
        assert_eq!(state.main_grid().height(), 24);
    }

    #[test]
    fn test_grid_resize() {
        let mut state = EditorState::new(80, 24);

        state.grid_resize(1, 100, 30);
        assert_eq!(state.main_grid().width(), 100);
        assert_eq!(state.main_grid().height(), 30);
    }

    #[test]
    fn test_grid_clear() {
        let mut state = EditorState::new(80, 24);
        state.main_grid_mut()[(0, 0)].text = "x".into();

        state.grid_clear(1);
        assert_eq!(state.main_grid()[(0, 0)].text, " ");
    }

    #[test]
    fn test_grid_line() {
        let mut state = EditorState::new(80, 24);

        let cells = vec![("a".into(), Some(0), 1), ("b".into(), None, 1)];
        state.grid_line(1, 5, 0, &cells);

        assert_eq!(state.main_grid()[(5, 0)].text, "a");
        assert_eq!(state.main_grid()[(5, 1)].text, "b");
    }

    #[test]
    fn test_grid_scroll() {
        let mut state = EditorState::new(80, 24);
        state.main_grid_mut()[(0, 0)].text = "a".into();
        state.main_grid_mut()[(1, 0)].text = "b".into();

        state.grid_scroll(1, 0, 24, 0, 80, 1);
        assert_eq!(state.main_grid()[(0, 0)].text, "b");
    }

    #[test]
    fn test_cursor_goto() {
        let mut state = EditorState::new(80, 24);

        state.grid_cursor_goto(1, 10, 20);
        assert_eq!(state.cursor.grid, 1);
        assert_eq!(state.cursor.row, 10);
        assert_eq!(state.cursor.col, 20);
    }

    #[test]
    fn test_cursor_move_marks_old_row() {
        let mut state = EditorState::new(80, 24);
        state.grid_cursor_goto(1, 5, 0);

        state.grid_cursor_goto(1, 10, 0);
    }

    #[test]
    fn test_hl_attr_define() {
        let mut state = EditorState::new(80, 24);

        let attrs = HighlightAttributes {
            foreground: Some(Color::from_rgb(255, 0, 0)),
            style: StyleFlags::BOLD,
            ..Default::default()
        };
        state.hl_attr_define(1, attrs);

        assert!(state.highlights.get(1).is_bold());
    }

    #[test]
    fn test_default_colors_set() {
        let mut state = EditorState::new(80, 24);

        state.default_colors_set(0xFFFFFF, 0x000000, 0xFF0000);
        assert_eq!(
            state.highlights.defaults.foreground,
            Color::from_rgb(255, 255, 255)
        );
    }

    #[test]
    fn test_mode_info_set() {
        let mut state = EditorState::new(80, 24);

        let modes = vec![
            ModeInfo {
                cursor_shape: CursorShape::Block,
                ..Default::default()
            },
            ModeInfo {
                cursor_shape: CursorShape::Vertical,
                cell_percentage: 25,
                ..Default::default()
            },
        ];
        state.mode_info_set(modes);
        assert_eq!(state.current_mode().cursor_shape, CursorShape::Block);
    }

    #[test]
    fn test_mode_change() {
        let mut state = EditorState::new(80, 24);

        let modes = vec![
            ModeInfo {
                cursor_shape: CursorShape::Block,
                ..Default::default()
            },
            ModeInfo {
                cursor_shape: CursorShape::Vertical,
                ..Default::default()
            },
        ];
        state.mode_info_set(modes);
        state.cursor.row = 5;

        state.mode_change("insert", 1);
        assert_eq!(state.current_mode().cursor_shape, CursorShape::Vertical);
    }

    #[test]
    fn test_multigrid() {
        let mut state = EditorState::new(80, 24);

        state.grid_resize(2, 40, 10);
        assert!(state.grid(2).is_some());
        assert_eq!(state.grid(2).unwrap().width(), 40);
    }

    #[test]
    fn test_cursor_blinking() {
        let mut state = EditorState::new(80, 24);
        let modes = vec![ModeInfo {
            blink_on: 100,
            blink_off: 100,
            blink_wait: 0,
            ..Default::default()
        }];
        state.mode_info_set(modes);

        // Initial state
        state.reset_blink();
        // Update with time 0 to process reset
        state.update_blink(0);
        assert!(state.cursor.blink_visible);

        // Advance time < blink_on
        state.update_blink(50);
        assert!(state.cursor.blink_visible);

        // Advance time > blink_on
        state.update_blink(150);
        assert!(!state.cursor.blink_visible);

        // Advance time > blink_on + blink_off
        state.update_blink(250);
        assert!(state.cursor.blink_visible);

        // Test reset
        state.reset_blink();
        state.update_blink(260); // Reset at 260
        assert!(state.cursor.blink_visible);

        // 260 + 150 = 410 -> should be hidden
        state.update_blink(410);
        assert!(!state.cursor.blink_visible);
    }
}
