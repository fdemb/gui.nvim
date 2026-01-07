use gui_nvim::bridge::events::{GridCell, RedrawEvent};
use gui_nvim::editor::{CursorShape, EditorState};

#[test]
fn test_simulation_workflow() {
    // 1. Initialize EditorState
    let mut state = EditorState::new(80, 24);

    // Verify initial state
    assert_eq!(state.main_grid().width(), 80);
    assert_eq!(state.main_grid().height(), 24);
    assert_eq!(state.cursor.row, 0);
    assert_eq!(state.cursor.col, 0);

    // 2. Resize grid
    state.handle_redraw_event(&RedrawEvent::GridResize {
        grid: 1,
        width: 40,
        height: 10,
    });
    assert_eq!(state.main_grid().width(), 40);
    assert_eq!(state.main_grid().height(), 10);

    // 3. Define highlight attributes
    use gui_nvim::editor::{Color, HighlightAttributes, StyleFlags};
    let attrs = HighlightAttributes {
        foreground: Some(Color::from_rgb(255, 0, 0)),
        style: StyleFlags::BOLD,
        ..Default::default()
    };
    state.handle_redraw_event(&RedrawEvent::HlAttrDefine { id: 1, attrs });

    // Verify highlight was added
    assert!(state.highlights.get(1).is_bold());

    // 4. Draw text
    // "Hello " with hl_id 1
    // "World" with hl_id 0 (default)
    let cells = vec![
        GridCell {
            text: "H".into(),
            hl_id: Some(1),
            repeat: 1,
        },
        GridCell {
            text: "e".into(),
            hl_id: None,
            repeat: 1,
        },
        GridCell {
            text: "l".into(),
            hl_id: None,
            repeat: 2,
        }, // "ll"
        GridCell {
            text: "o".into(),
            hl_id: None,
            repeat: 1,
        },
        GridCell {
            text: " ".into(),
            hl_id: Some(0),
            repeat: 1,
        },
        GridCell {
            text: "W".into(),
            hl_id: None,
            repeat: 1,
        },
        GridCell {
            text: "o".into(),
            hl_id: None,
            repeat: 1,
        },
        GridCell {
            text: "r".into(),
            hl_id: None,
            repeat: 1,
        },
        GridCell {
            text: "l".into(),
            hl_id: None,
            repeat: 1,
        },
        GridCell {
            text: "d".into(),
            hl_id: None,
            repeat: 1,
        },
    ];
    state.handle_redraw_event(&RedrawEvent::GridLine {
        grid: 1,
        row: 0,
        col_start: 0,
        cells,
    });

    // Verify grid content
    assert_eq!(state.main_grid()[(0, 0)].text, "H");
    assert_eq!(state.main_grid()[(0, 0)].highlight_id, 1);

    assert_eq!(state.main_grid()[(0, 1)].text, "e");
    assert_eq!(state.main_grid()[(0, 1)].highlight_id, 1); // Inherited

    assert_eq!(state.main_grid()[(0, 2)].text, "l");
    assert_eq!(state.main_grid()[(0, 3)].text, "l");

    assert_eq!(state.main_grid()[(0, 5)].text, " ");
    assert_eq!(state.main_grid()[(0, 5)].highlight_id, 0);

    assert_eq!(state.main_grid()[(0, 6)].text, "W");
    assert_eq!(state.main_grid()[(0, 6)].highlight_id, 0); // Inherited

    // 5. Move cursor
    state.handle_redraw_event(&RedrawEvent::GridCursorGoto {
        grid: 1,
        row: 0,
        col: 5,
    });
    assert_eq!(state.cursor.row, 0);
    assert_eq!(state.cursor.col, 5);

    // 6. Set mode info
    use gui_nvim::editor::ModeInfo;
    let mode_info = ModeInfo {
        cursor_shape: CursorShape::Horizontal,
        cell_percentage: 50,
        ..Default::default()
    };
    state.handle_redraw_event(&RedrawEvent::ModeInfoSet {
        cursor_style_enabled: true,
        modes: vec![ModeInfo::default(), mode_info], // Mode 0 is default, Mode 1 is our custom
    });

    // Change to mode 1
    state.handle_redraw_event(&RedrawEvent::ModeChange {
        mode: "insert".into(),
        mode_idx: 1,
    });

    assert_eq!(state.current_mode().cursor_shape, CursorShape::Horizontal);

    // 7. Scroll
    // Move "Hello World" down by 1 line
    state.handle_redraw_event(&RedrawEvent::GridScroll {
        grid: 1,
        top: 0,
        bot: 10,
        left: 0,
        right: 40,
        rows: -1, // Down
    });

    // Row 0 should be empty (cleared)
    assert_eq!(state.main_grid()[(0, 0)].text, " ");
    // Row 1 should have "H"
    assert_eq!(state.main_grid()[(1, 0)].text, "H");
}
