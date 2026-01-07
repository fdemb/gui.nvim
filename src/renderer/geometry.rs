use crate::editor::{CursorShape, UnderlineStyle};

#[derive(Debug, Clone, PartialEq)]
pub struct CursorGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecorationGeometry {
    pub lines: Vec<DecorationLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecorationLine {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn compute_decoration_geometry(
    x: f32,
    y: f32,
    cell_width: f32,
    cell_height: f32,
    descent: f32,
    underline_pos: f32,
    underline_thickness: f32,
    strikeout_pos: f32,
    strikeout_thickness: f32,
    underline_style: UnderlineStyle,
    has_strikethrough: bool,
) -> DecorationGeometry {
    let mut lines = Vec::new();
    let baseline_y = y + cell_height - descent.abs();

    if underline_style != UnderlineStyle::None {
        let underline_y = baseline_y - underline_pos;

        match underline_style {
            UnderlineStyle::Single
            | UnderlineStyle::Curl
            | UnderlineStyle::Dotted
            | UnderlineStyle::Dashed => {
                lines.push(DecorationLine {
                    x,
                    y: underline_y,
                    width: cell_width,
                    height: underline_thickness,
                });
            }
            UnderlineStyle::Double => {
                let gap = underline_thickness;
                lines.push(DecorationLine {
                    x,
                    y: underline_y - gap,
                    width: cell_width,
                    height: underline_thickness,
                });
                lines.push(DecorationLine {
                    x,
                    y: underline_y + gap,
                    width: cell_width,
                    height: underline_thickness,
                });
            }
            UnderlineStyle::None => {}
        }
    }

    if has_strikethrough {
        let strikeout_y = baseline_y - strikeout_pos;
        lines.push(DecorationLine {
            x,
            y: strikeout_y,
            width: cell_width,
            height: strikeout_thickness,
        });
    }

    DecorationGeometry { lines }
}

pub fn compute_cursor_geometry(
    cursor_shape: CursorShape,
    row: usize,
    col: usize,
    cell_width: f32,
    cell_height: f32,
    cell_percentage: u8,
) -> CursorGeometry {
    let x = col as f32 * cell_width;
    let y = row as f32 * cell_height;

    let percentage = if cell_percentage > 0 {
        cell_percentage.min(100) as f32 / 100.0
    } else {
        match cursor_shape {
            CursorShape::Vertical => 0.25,
            CursorShape::Horizontal => 0.25,
            CursorShape::Block => 1.0,
        }
    };

    match cursor_shape {
        CursorShape::Block => CursorGeometry {
            x,
            y,
            width: cell_width,
            height: cell_height,
        },
        CursorShape::Vertical => {
            let bar_width = (cell_width * percentage).max(1.0);
            CursorGeometry {
                x,
                y,
                width: bar_width,
                height: cell_height,
            }
        }
        CursorShape::Horizontal => {
            let bar_height = (cell_height * percentage).max(1.0);
            CursorGeometry {
                x,
                y: y + cell_height - bar_height,
                width: cell_width,
                height: bar_height,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_geometry_block() {
        let geom = compute_cursor_geometry(CursorShape::Block, 5, 10, 10.0, 20.0, 0);
        assert_eq!(geom.x, 100.0);
        assert_eq!(geom.y, 100.0);
        assert_eq!(geom.width, 10.0);
        assert_eq!(geom.height, 20.0);
    }

    #[test]
    fn test_cursor_geometry_vertical_default() {
        let geom = compute_cursor_geometry(CursorShape::Vertical, 0, 0, 10.0, 20.0, 0);
        assert_eq!(geom.x, 0.0);
        assert_eq!(geom.y, 0.0);
        assert_eq!(geom.width, 2.5);
        assert_eq!(geom.height, 20.0);
    }

    #[test]
    fn test_cursor_geometry_vertical_custom_percentage() {
        let geom = compute_cursor_geometry(CursorShape::Vertical, 0, 0, 10.0, 20.0, 50);
        assert_eq!(geom.width, 5.0);
    }

    #[test]
    fn test_cursor_geometry_horizontal_default() {
        let geom = compute_cursor_geometry(CursorShape::Horizontal, 0, 0, 10.0, 20.0, 0);
        assert_eq!(geom.x, 0.0);
        assert_eq!(geom.y, 15.0);
        assert_eq!(geom.width, 10.0);
        assert_eq!(geom.height, 5.0);
    }

    #[test]
    fn test_cursor_geometry_horizontal_custom_percentage() {
        let geom = compute_cursor_geometry(CursorShape::Horizontal, 2, 3, 10.0, 20.0, 10);
        assert_eq!(geom.x, 30.0);
        assert_eq!(geom.y, 58.0);
        assert_eq!(geom.height, 2.0);
    }

    #[test]
    fn test_cursor_geometry_minimum_size() {
        let geom = compute_cursor_geometry(CursorShape::Vertical, 0, 0, 2.0, 2.0, 1);
        assert!(geom.width >= 1.0);

        let geom = compute_cursor_geometry(CursorShape::Horizontal, 0, 0, 2.0, 2.0, 1);
        assert!(geom.height >= 1.0);
    }

    #[test]
    fn test_decoration_geometry_single_underline() {
        let geom = compute_decoration_geometry(
            0.0,
            0.0,
            10.0,
            20.0,
            4.0,
            2.0,
            1.0,
            8.0,
            1.0,
            UnderlineStyle::Single,
            false,
        );
        assert_eq!(geom.lines.len(), 1);
        let line = &geom.lines[0];
        assert_eq!(line.x, 0.0);
        assert_eq!(line.width, 10.0);
        assert_eq!(line.height, 1.0);
    }

    #[test]
    fn test_decoration_geometry_double_underline() {
        let geom = compute_decoration_geometry(
            0.0,
            0.0,
            10.0,
            20.0,
            4.0,
            2.0,
            1.0,
            8.0,
            1.0,
            UnderlineStyle::Double,
            false,
        );
        assert_eq!(geom.lines.len(), 2);
        assert_eq!(geom.lines[0].width, 10.0);
        assert_eq!(geom.lines[1].width, 10.0);
        assert!(geom.lines[0].y != geom.lines[1].y);
    }

    #[test]
    fn test_decoration_geometry_strikethrough() {
        let geom = compute_decoration_geometry(
            5.0,
            10.0,
            10.0,
            20.0,
            4.0,
            2.0,
            1.0,
            8.0,
            1.5,
            UnderlineStyle::None,
            true,
        );
        assert_eq!(geom.lines.len(), 1);
        let line = &geom.lines[0];
        assert_eq!(line.x, 5.0);
        assert_eq!(line.width, 10.0);
        assert_eq!(line.height, 1.5);
    }

    #[test]
    fn test_decoration_geometry_underline_and_strikethrough() {
        let geom = compute_decoration_geometry(
            0.0,
            0.0,
            10.0,
            20.0,
            4.0,
            2.0,
            1.0,
            8.0,
            1.0,
            UnderlineStyle::Single,
            true,
        );
        assert_eq!(geom.lines.len(), 2);
    }

    #[test]
    fn test_decoration_geometry_no_decorations() {
        let geom = compute_decoration_geometry(
            0.0,
            0.0,
            10.0,
            20.0,
            4.0,
            2.0,
            1.0,
            8.0,
            1.0,
            UnderlineStyle::None,
            false,
        );
        assert!(geom.lines.is_empty());
    }
}

