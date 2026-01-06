use nvim_rs::Value;

use crate::editor::{Color, CursorShape, HighlightAttributes, ModeInfo, StyleFlags};

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

pub fn parse_redraw(args: Vec<Value>) -> Vec<RedrawEvent> {
    let mut events = Vec::new();

    for event_batch in args {
        if let Value::Array(batch) = event_batch {
            if batch.is_empty() {
                continue;
            }

            let event_name = match &batch[0] {
                Value::String(s) => s.as_str().unwrap_or(""),
                _ => continue,
            };

            for event_args in batch.iter().skip(1) {
                if let Value::Array(args) = event_args {
                    if let Some(event) = parse_single_event(event_name, args) {
                        events.push(event);
                    }
                }
            }
        }
    }

    events
}

fn parse_single_event(name: &str, args: &[Value]) -> Option<RedrawEvent> {
    match name {
        "grid_resize" => parse_grid_resize(args),
        "grid_clear" => parse_grid_clear(args),
        "grid_line" => parse_grid_line(args),
        "grid_scroll" => parse_grid_scroll(args),
        "grid_cursor_goto" => parse_grid_cursor_goto(args),
        "grid_destroy" => parse_grid_destroy(args),
        "hl_attr_define" => parse_hl_attr_define(args),
        "hl_group_set" => parse_hl_group_set(args),
        "default_colors_set" => parse_default_colors_set(args),
        "mode_info_set" => parse_mode_info_set(args),
        "mode_change" => parse_mode_change(args),
        "set_title" => parse_set_title(args),
        "set_icon" => parse_set_icon(args),
        "option_set" => parse_option_set(args),
        "flush" => Some(RedrawEvent::Flush),
        "busy_start" => Some(RedrawEvent::Busy { busy: true }),
        "busy_stop" => Some(RedrawEvent::Busy { busy: false }),
        "mouse_on" => Some(RedrawEvent::MouseOn),
        "mouse_off" => Some(RedrawEvent::MouseOff),
        _ => {
            log::trace!("Unhandled redraw event: {}", name);
            None
        }
    }
}

fn parse_grid_resize(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 3 {
        return None;
    }
    Some(RedrawEvent::GridResize {
        grid: as_u64(&args[0])?,
        width: as_usize(&args[1])?,
        height: as_usize(&args[2])?,
    })
}

fn parse_grid_clear(args: &[Value]) -> Option<RedrawEvent> {
    if args.is_empty() {
        return None;
    }
    Some(RedrawEvent::GridClear {
        grid: as_u64(&args[0])?,
    })
}

fn parse_grid_line(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 4 {
        return None;
    }

    let grid = as_u64(&args[0])?;
    let row = as_usize(&args[1])?;
    let col_start = as_usize(&args[2])?;
    let cells_array = args[3].as_array()?;

    let mut cells = Vec::new();
    for cell in cells_array {
        if let Value::Array(cell_data) = cell {
            if cell_data.is_empty() {
                continue;
            }

            let text = cell_data[0].as_str().unwrap_or(" ").to_string();

            let hl_id = if cell_data.len() > 1 {
                as_u64(&cell_data[1])
            } else {
                None
            };

            let repeat = if cell_data.len() > 2 {
                as_usize(&cell_data[2]).unwrap_or(1)
            } else {
                1
            };

            // Cells with repeat == 0 should be skipped. They are used by terminal Neovim
            // to distinguish between empty lines and lines ending with spaces.
            if repeat == 0 {
                continue;
            }

            cells.push(GridCell {
                text,
                hl_id,
                repeat,
            });
        }
    }

    Some(RedrawEvent::GridLine {
        grid,
        row,
        col_start,
        cells,
    })
}

fn parse_grid_scroll(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 6 {
        return None;
    }
    Some(RedrawEvent::GridScroll {
        grid: as_u64(&args[0])?,
        top: as_usize(&args[1])?,
        bot: as_usize(&args[2])?,
        left: as_usize(&args[3])?,
        right: as_usize(&args[4])?,
        rows: as_i64(&args[5])?,
    })
}

fn parse_grid_cursor_goto(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 3 {
        return None;
    }
    Some(RedrawEvent::GridCursorGoto {
        grid: as_u64(&args[0])?,
        row: as_usize(&args[1])?,
        col: as_usize(&args[2])?,
    })
}

fn parse_grid_destroy(args: &[Value]) -> Option<RedrawEvent> {
    if args.is_empty() {
        return None;
    }
    Some(RedrawEvent::GridDestroy {
        grid: as_u64(&args[0])?,
    })
}

fn parse_hl_attr_define(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 2 {
        return None;
    }

    let id = as_u64(&args[0])?;
    let rgb_attrs = args[1].as_map()?;

    let mut attrs = HighlightAttributes::default();

    for (key, value) in rgb_attrs {
        let key_str = key.as_str().unwrap_or("");
        match key_str {
            "foreground" => {
                if let Some(fg) = as_u32(value) {
                    attrs.foreground = Some(Color::from_u24(fg));
                }
            }
            "background" => {
                if let Some(bg) = as_u32(value) {
                    attrs.background = Some(Color::from_u24(bg));
                }
            }
            "special" => {
                if let Some(sp) = as_u32(value) {
                    attrs.special = Some(Color::from_u24(sp));
                }
            }
            "bold" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::BOLD;
                }
            }
            "italic" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::ITALIC;
                }
            }
            "underline" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::UNDERLINE;
                }
            }
            "undercurl" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::UNDERCURL;
                }
            }
            "underdouble" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::UNDERDOUBLE;
                }
            }
            "underdotted" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::UNDERDOTTED;
                }
            }
            "underdashed" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::UNDERDASHED;
                }
            }
            "strikethrough" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::STRIKETHROUGH;
                }
            }
            "reverse" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::REVERSE;
                }
            }
            "altfont" => {
                if value.as_bool().unwrap_or(false) {
                    attrs.style |= StyleFlags::ALTFONT;
                }
            }
            "blend" => {
                if let Some(b) = as_u64(value) {
                    attrs.blend = b as u8;
                }
            }
            "url" => {
                if let Some(url) = value.as_str() {
                    attrs.url = Some(url.to_string());
                }
            }
            _ => {}
        }
    }

    Some(RedrawEvent::HlAttrDefine { id, attrs })
}

fn parse_hl_group_set(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 2 {
        return None;
    }
    Some(RedrawEvent::HlGroupSet {
        name: args[0].as_str()?.to_string(),
        id: as_u64(&args[1])?,
    })
}

fn parse_default_colors_set(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 3 {
        return None;
    }
    Some(RedrawEvent::DefaultColorsSet {
        fg: as_u32(&args[0])?,
        bg: as_u32(&args[1])?,
        sp: as_u32(&args[2])?,
    })
}

fn parse_mode_info_set(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 2 {
        return None;
    }

    let cursor_style_enabled = args[0].as_bool().unwrap_or(true);
    let mode_list = args[1].as_array()?;

    let mut modes = Vec::new();
    for mode_value in mode_list {
        let mode_map = mode_value.as_map()?;
        let mut mode_info = ModeInfo::default();

        for (key, value) in mode_map {
            let key_str = key.as_str().unwrap_or("");
            match key_str {
                "cursor_shape" => {
                    mode_info.cursor_shape = match value.as_str().unwrap_or("block") {
                        "block" => CursorShape::Block,
                        "horizontal" => CursorShape::Horizontal,
                        "vertical" => CursorShape::Vertical,
                        _ => CursorShape::Block,
                    };
                }
                "cell_percentage" => {
                    if let Some(pct) = as_u64(value) {
                        mode_info.cell_percentage = pct as u8;
                    }
                }
                "attr_id" => {
                    if let Some(id) = as_u64(value) {
                        mode_info.attr_id = id;
                    }
                }
                "blinkwait" => {
                    if let Some(ms) = as_u32(value) {
                        mode_info.blink_wait = ms;
                    }
                }
                "blinkon" => {
                    if let Some(ms) = as_u32(value) {
                        mode_info.blink_on = ms;
                    }
                }
                "blinkoff" => {
                    if let Some(ms) = as_u32(value) {
                        mode_info.blink_off = ms;
                    }
                }
                _ => {}
            }
        }

        modes.push(mode_info);
    }

    Some(RedrawEvent::ModeInfoSet {
        cursor_style_enabled,
        modes,
    })
}

fn parse_mode_change(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 2 {
        return None;
    }
    Some(RedrawEvent::ModeChange {
        mode: args[0].as_str()?.to_string(),
        mode_idx: as_usize(&args[1])?,
    })
}

fn parse_set_title(args: &[Value]) -> Option<RedrawEvent> {
    if args.is_empty() {
        return None;
    }
    Some(RedrawEvent::SetTitle {
        title: args[0].as_str()?.to_string(),
    })
}

fn parse_set_icon(args: &[Value]) -> Option<RedrawEvent> {
    if args.is_empty() {
        return None;
    }
    Some(RedrawEvent::SetIcon {
        icon: args[0].as_str()?.to_string(),
    })
}

fn parse_option_set(args: &[Value]) -> Option<RedrawEvent> {
    if args.len() < 2 {
        return None;
    }
    Some(RedrawEvent::OptionSet {
        name: args[0].as_str()?.to_string(),
        value: args[1].clone(),
    })
}

fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Integer(i) => i.as_u64(),
        _ => None,
    }
}

fn as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Integer(i) => i.as_i64(),
        _ => None,
    }
}

fn as_u32(value: &Value) -> Option<u32> {
    as_u64(value).map(|v| v as u32)
}

fn as_usize(value: &Value) -> Option<usize> {
    as_u64(value).map(|v| v as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid_resize_event(grid: u64, width: u64, height: u64) -> Vec<Value> {
        vec![Value::Array(vec![
            Value::from("grid_resize"),
            Value::Array(vec![
                Value::from(grid),
                Value::from(width),
                Value::from(height),
            ]),
        ])]
    }

    #[test]
    fn test_parse_grid_resize() {
        let args = make_grid_resize_event(1, 80, 24);
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            RedrawEvent::GridResize {
                grid: 1,
                width: 80,
                height: 24,
            }
        );
    }

    #[test]
    fn test_parse_grid_clear() {
        let args = vec![Value::Array(vec![
            Value::from("grid_clear"),
            Value::Array(vec![Value::from(1u64)]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], RedrawEvent::GridClear { grid: 1 });
    }

    #[test]
    fn test_parse_grid_line() {
        let args = vec![Value::Array(vec![
            Value::from("grid_line"),
            Value::Array(vec![
                Value::from(1u64), // grid
                Value::from(0u64), // row
                Value::from(0u64), // col_start
                Value::Array(vec![
                    // cells
                    Value::Array(vec![Value::from("H"), Value::from(1u64)]),
                    Value::Array(vec![Value::from("i")]), // hl_id inherited
                    Value::Array(vec![Value::from(" "), Value::from(0u64), Value::from(3u64)]), // repeat 3
                ]),
            ]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        if let RedrawEvent::GridLine {
            grid,
            row,
            col_start,
            cells,
        } = &events[0]
        {
            assert_eq!(*grid, 1);
            assert_eq!(*row, 0);
            assert_eq!(*col_start, 0);
            assert_eq!(cells.len(), 3);
            assert_eq!(cells[0].text, "H");
            assert_eq!(cells[0].hl_id, Some(1));
            assert_eq!(cells[0].repeat, 1);
            assert_eq!(cells[1].text, "i");
            assert_eq!(cells[1].hl_id, None); // inherited
            assert_eq!(cells[2].text, " ");
            assert_eq!(cells[2].hl_id, Some(0));
            assert_eq!(cells[2].repeat, 3);
        } else {
            panic!("Expected GridLine event");
        }
    }

    #[test]
    fn test_parse_grid_scroll() {
        let args = vec![Value::Array(vec![
            Value::from("grid_scroll"),
            Value::Array(vec![
                Value::from(1u64),  // grid
                Value::from(0u64),  // top
                Value::from(24u64), // bot
                Value::from(0u64),  // left
                Value::from(80u64), // right
                Value::from(1i64),  // rows (positive = scroll up)
            ]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            RedrawEvent::GridScroll {
                grid: 1,
                top: 0,
                bot: 24,
                left: 0,
                right: 80,
                rows: 1,
            }
        );
    }

    #[test]
    fn test_parse_grid_cursor_goto() {
        let args = vec![Value::Array(vec![
            Value::from("grid_cursor_goto"),
            Value::Array(vec![
                Value::from(1u64),
                Value::from(10u64),
                Value::from(5u64),
            ]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            RedrawEvent::GridCursorGoto {
                grid: 1,
                row: 10,
                col: 5,
            }
        );
    }

    #[test]
    fn test_parse_hl_attr_define() {
        let rgb_attrs = vec![
            (Value::from("foreground"), Value::from(0xFF0000u64)),
            (Value::from("background"), Value::from(0x00FF00u64)),
            (Value::from("bold"), Value::from(true)),
            (Value::from("italic"), Value::from(true)),
        ];

        let args = vec![Value::Array(vec![
            Value::from("hl_attr_define"),
            Value::Array(vec![
                Value::from(1u64),
                Value::Map(rgb_attrs),
                Value::Map(vec![]),   // cterm attrs (ignored)
                Value::Array(vec![]), // info (ignored)
            ]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        if let RedrawEvent::HlAttrDefine { id, attrs } = &events[0] {
            assert_eq!(*id, 1);
            assert_eq!(attrs.foreground, Some(Color::from_u24(0xFF0000)));
            assert_eq!(attrs.background, Some(Color::from_u24(0x00FF00)));
            assert!(attrs.is_bold());
            assert!(attrs.is_italic());
        } else {
            panic!("Expected HlAttrDefine event");
        }
    }

    #[test]
    fn test_parse_default_colors_set() {
        let args = vec![Value::Array(vec![
            Value::from("default_colors_set"),
            Value::Array(vec![
                Value::from(0xFFFFFFu64), // fg
                Value::from(0x000000u64), // bg
                Value::from(0xFF0000u64), // sp
                Value::from(-1i64),       // cterm_fg (ignored)
                Value::from(-1i64),       // cterm_bg (ignored)
            ]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            RedrawEvent::DefaultColorsSet {
                fg: 0xFFFFFF,
                bg: 0x000000,
                sp: 0xFF0000,
            }
        );
    }

    #[test]
    fn test_parse_mode_info_set() {
        let mode_attrs = vec![
            (Value::from("cursor_shape"), Value::from("vertical")),
            (Value::from("cell_percentage"), Value::from(25u64)),
            (Value::from("attr_id"), Value::from(0u64)),
        ];

        let args = vec![Value::Array(vec![
            Value::from("mode_info_set"),
            Value::Array(vec![
                Value::from(true), // cursor_style_enabled
                Value::Array(vec![Value::Map(mode_attrs)]),
            ]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        if let RedrawEvent::ModeInfoSet {
            cursor_style_enabled,
            modes,
        } = &events[0]
        {
            assert!(*cursor_style_enabled);
            assert_eq!(modes.len(), 1);
            assert_eq!(modes[0].cursor_shape, CursorShape::Vertical);
            assert_eq!(modes[0].cell_percentage, 25);
        } else {
            panic!("Expected ModeInfoSet event");
        }
    }

    #[test]
    fn test_parse_mode_change() {
        let args = vec![Value::Array(vec![
            Value::from("mode_change"),
            Value::Array(vec![Value::from("insert"), Value::from(1u64)]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            RedrawEvent::ModeChange {
                mode: "insert".to_string(),
                mode_idx: 1,
            }
        );
    }

    #[test]
    fn test_parse_flush() {
        let args = vec![Value::Array(vec![
            Value::from("flush"),
            Value::Array(vec![]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], RedrawEvent::Flush);
    }

    #[test]
    fn test_parse_set_title() {
        let args = vec![Value::Array(vec![
            Value::from("set_title"),
            Value::Array(vec![Value::from("My Title")]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            RedrawEvent::SetTitle {
                title: "My Title".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_multiple_events_in_batch() {
        let args = vec![Value::Array(vec![
            Value::from("grid_resize"),
            Value::Array(vec![
                Value::from(1u64),
                Value::from(80u64),
                Value::from(24u64),
            ]),
            Value::Array(vec![
                Value::from(2u64),
                Value::from(40u64),
                Value::from(10u64),
            ]),
        ])];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            RedrawEvent::GridResize {
                grid: 1,
                width: 80,
                height: 24,
            }
        );
        assert_eq!(
            events[1],
            RedrawEvent::GridResize {
                grid: 2,
                width: 40,
                height: 10,
            }
        );
    }

    #[test]
    fn test_parse_empty_args() {
        let events = parse_redraw(vec![]);
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_unknown_event() {
        let args = vec![Value::Array(vec![
            Value::from("unknown_event"),
            Value::Array(vec![Value::from(1u64)]),
        ])];
        let events = parse_redraw(args);

        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_busy_events() {
        let args = vec![
            Value::Array(vec![Value::from("busy_start"), Value::Array(vec![])]),
            Value::Array(vec![Value::from("busy_stop"), Value::Array(vec![])]),
        ];
        let events = parse_redraw(args);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0], RedrawEvent::Busy { busy: true });
        assert_eq!(events[1], RedrawEvent::Busy { busy: false });
    }
}
