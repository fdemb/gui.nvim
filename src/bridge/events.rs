use nvim_rs::Value;

use crate::editor::{HighlightAttributes, ModeInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum RedrawEvent {
    GridResize {
        grid: u64,
        width: usize,
        height: usize,
    },
    GridClear {
        grid: u64,
    },
    GridLine {
        grid: u64,
        row: usize,
        col_start: usize,
        cells: Vec<GridCell>,
    },
    GridScroll {
        grid: u64,
        top: usize,
        bot: usize,
        left: usize,
        right: usize,
        rows: i64,
    },
    GridCursorGoto {
        grid: u64,
        row: usize,
        col: usize,
    },
    GridDestroy {
        grid: u64,
    },
    HlAttrDefine {
        id: u64,
        attrs: HighlightAttributes,
    },
    HlGroupSet {
        name: String,
        id: u64,
    },
    DefaultColorsSet {
        fg: u32,
        bg: u32,
        sp: u32,
    },
    ModeInfoSet {
        cursor_style_enabled: bool,
        modes: Vec<ModeInfo>,
    },
    ModeChange {
        mode: String,
        mode_idx: usize,
    },
    SetTitle {
        title: String,
    },
    SetIcon {
        icon: String,
    },
    OptionSet {
        name: String,
        value: Value,
    },
    Flush,
    Busy {
        busy: bool,
    },
    MouseOn,
    MouseOff,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridCell {
    pub text: String,
    pub hl_id: Option<u64>,
    pub repeat: usize,
}
