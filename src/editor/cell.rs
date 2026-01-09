use bitflags::bitflags;
use compact_str::CompactString;

bitflags! {
    /// Cell attribute flags for efficient storage of boolean attributes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CellFlags: u16 {
        /// Cell contains a wide (double-width) character.
        const WIDE_CHAR = 0b0000_0001;
        /// Cell is a placeholder for the right half of a wide character.
        const WIDE_CHAR_SPACER = 0b0000_0010;
    }
}

/// A single cell in the terminal grid.
///
/// Cells are designed to be compact while storing all information needed
/// for rendering. The highlight_id refers to attributes defined by Neovim's
/// `hl_attr_define` events.
///
/// Uses `CompactString` for SSO (small string optimization) - strings up to
/// 24 bytes are stored inline without heap allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The character displayed in this cell.
    /// Empty cells contain a space character.
    pub text: CompactString,
    /// Highlight attribute ID from Neovim.
    /// ID 0 means default highlighting.
    pub highlight_id: u64,
    /// Cell-specific flags (wide char, etc.).
    pub flags: CellFlags,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            text: CompactString::const_new(" "),
            highlight_id: 0,
            flags: CellFlags::empty(),
        }
    }
}

impl Cell {
    /// Creates a new cell with the given text and highlight ID.
    #[allow(dead_code)]
    pub fn new(text: impl Into<CompactString>, highlight_id: u64) -> Self {
        Self {
            text: text.into(),
            highlight_id,
            flags: CellFlags::empty(),
        }
    }

    /// Returns true if this cell is empty (space with default highlight).
    pub fn is_empty(&self) -> bool {
        self.highlight_id == 0
            && (self.text == " " || self.text.is_empty())
            && self.flags.is_empty()
    }

    /// Returns true if this cell contains a wide character.
    #[allow(dead_code)]
    pub fn is_wide(&self) -> bool {
        self.flags.contains(CellFlags::WIDE_CHAR)
    }

    /// Returns true if this cell is a spacer for a wide character.
    pub fn is_wide_spacer(&self) -> bool {
        self.flags.contains(CellFlags::WIDE_CHAR_SPACER)
    }

    /// Sets the wide character flag.
    pub fn set_wide(&mut self, wide: bool) {
        self.flags.set(CellFlags::WIDE_CHAR, wide);
    }

    /// Sets the wide character spacer flag.
    pub fn set_wide_spacer(&mut self, spacer: bool) {
        self.flags.set(CellFlags::WIDE_CHAR_SPACER, spacer);
    }

    /// Resets the cell to its default state.
    pub fn clear(&mut self) {
        self.text = CompactString::const_new(" ");
        self.highlight_id = 0;
        self.flags = CellFlags::empty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert_eq!(cell.text, " ");
        assert_eq!(cell.highlight_id, 0);
        assert!(cell.flags.is_empty());
        assert!(cell.is_empty());
    }

    #[test]
    fn test_cell_new() {
        let cell = Cell::new("a", 5);
        assert_eq!(cell.text, "a");
        assert_eq!(cell.highlight_id, 5);
        assert!(!cell.is_empty());
    }

    #[test]
    fn test_cell_is_empty() {
        let empty = Cell::default();
        assert!(empty.is_empty());

        let with_text = Cell::new("x", 0);
        assert!(!with_text.is_empty());

        let with_highlight = Cell::new(" ", 1);
        assert!(!with_highlight.is_empty());
    }

    #[test]
    fn test_cell_wide_char() {
        let mut cell = Cell::new("あ", 0);
        assert!(!cell.is_wide());

        cell.set_wide(true);
        assert!(cell.is_wide());
        assert!(!cell.is_empty());

        cell.set_wide(false);
        assert!(!cell.is_wide());
    }

    #[test]
    fn test_cell_wide_spacer() {
        let mut cell = Cell::default();
        assert!(!cell.is_wide_spacer());

        cell.set_wide_spacer(true);
        assert!(cell.is_wide_spacer());
        assert!(!cell.is_empty());

        cell.set_wide_spacer(false);
        assert!(!cell.is_wide_spacer());
    }

    #[test]
    fn test_cell_clear() {
        let mut cell = Cell::new("x", 5);
        cell.set_wide(true);

        cell.clear();
        assert_eq!(cell.text, " ");
        assert_eq!(cell.highlight_id, 0);
        assert!(cell.flags.is_empty());
        assert!(cell.is_empty());
    }

    #[test]
    fn test_cell_size() {
        // CompactString is 24 bytes with inline storage for up to 24 bytes
        // text: CompactString (24) + highlight_id: u64 (8) + flags: u16 (2) + padding
        let size = mem::size_of::<Cell>();
        assert!(size <= 40, "Cell size {} is larger than expected", size);
    }

    #[test]
    fn test_cell_clone() {
        let cell = Cell::new("test", 42);
        let cloned = cell.clone();
        assert_eq!(cell, cloned);
    }
}
