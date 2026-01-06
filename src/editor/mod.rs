mod cell;
mod grid;
mod highlight;
mod state;

// Re-export public items for use by the renderer and other modules
#[allow(unused_imports)]
pub use cell::{Cell, CellFlags};
#[allow(unused_imports)]
pub use grid::Grid;
pub use highlight::{Color, HighlightAttributes, StyleFlags, UnderlineStyle};
#[allow(unused_imports)]
pub use highlight::{DefaultColors, HighlightMap};
#[allow(unused_imports)]
pub use state::Cursor;
pub use state::{CursorShape, EditorState, ModeInfo};
