pub const PADDING: u32 = 2;

#[cfg(target_os = "macos")]
pub const PADDING_TOP: u32 = 30;
#[cfg(not(target_os = "macos"))]
pub const PADDING_TOP: u32 = PADDING;

pub const DEFAULT_CELL_WIDTH: u32 = 10;
pub const DEFAULT_CELL_HEIGHT: u32 = 20;

// Bridge constants
pub const DEFAULT_COLS: u64 = 80;
pub const DEFAULT_ROWS: u64 = 24;
