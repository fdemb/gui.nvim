use crossfont::Size;

use super::atlas::GlyphAtlas;
use super::batch::RenderBatcher;
use super::color::u32_to_linear_rgba;
use super::font::FontSystem;
use super::GpuContext;
use crate::editor::{
    Cell, CursorShape, EditorState, HighlightAttributes, StyleFlags, UnderlineStyle,
};

pub struct GridRenderer {
    batcher: RenderBatcher,
    atlas: GlyphAtlas,
    font_system: FontSystem,
    cell_width: f32,
    cell_height: f32,
    font_size: Size,
}

impl GridRenderer {
    pub fn new(ctx: &GpuContext, scale_factor: f64) -> Result<Self, GridRendererError> {
        let font_config = super::font::FontConfig::with_scale_factor(scale_factor);
        let mut font_system = FontSystem::new(&font_config)?;

        let cell_width = font_system.cell_width();
        let cell_height = font_system.cell_height();
        let font_size = Size::new(font_config.scaled_size());

        let mut atlas = GlyphAtlas::new(ctx);
        atlas.prepopulate_ascii(ctx, &mut font_system, font_size);

        let batcher = RenderBatcher::new(ctx);

        Ok(Self {
            batcher,
            atlas,
            font_system,
            cell_width,
            cell_height,
            font_size,
        })
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }

    #[allow(dead_code)]
    pub fn font_system(&self) -> &FontSystem {
        &self.font_system
    }

    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    pub fn prepare(
        &mut self,
        ctx: &GpuContext,
        state: &EditorState,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
    ) {
        self.batcher.clear();
        self.prepare_grid_cells(ctx, state, default_bg, default_fg);
        self.prepare_cursor(ctx, state, default_bg, default_fg);
        self.batcher.upload(ctx);
    }

    fn prepare_grid_cells(
        &mut self,
        ctx: &GpuContext,
        state: &EditorState,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
    ) {
        let grid = state.main_grid();
        let highlights = &state.highlights;

        for (row_idx, row_cells) in grid.rows().enumerate() {
            let y = row_idx as f32 * self.cell_height;

            let mut last_hl_id = u64::MAX;
            let mut last_bg = default_bg;
            let mut last_fg = default_fg;
            let mut last_attrs = highlights.get(0);

            for (col_idx, cell) in row_cells.iter().enumerate() {
                // Cache color resolution
                if cell.highlight_id != last_hl_id {
                    last_hl_id = cell.highlight_id;
                    last_attrs = highlights.get(last_hl_id);
                    let (bg, fg) = self.resolve_colors(last_attrs, default_bg, default_fg);
                    last_bg = bg;
                    last_fg = fg;
                }

                let x = col_idx as f32 * self.cell_width;

                self.push_cell_background(x, y, last_bg, default_bg);
                self.push_cell_glyph(ctx, x, y, cell, last_attrs, last_fg);
                self.push_cell_decorations(x, y, last_attrs, last_fg);
            }
        }
    }

    #[inline(always)]
    fn push_cell_background(&mut self, x: f32, y: f32, bg: [f32; 4], default_bg: [f32; 4]) {
        if bg != default_bg {
            self.batcher
                .push_background(x, y, self.cell_width, self.cell_height, bg);
        }
    }

    #[inline(always)]
    fn push_cell_glyph(
        &mut self,
        ctx: &GpuContext,
        x: f32,
        y: f32,
        cell: &Cell,
        attrs: &HighlightAttributes,
        fg: [f32; 4],
    ) {
        if cell.is_empty() || cell.is_wide_spacer() {
            return;
        }

        if let Some(character) = cell.text.chars().next() {
            let font_key = self.font_system.font_key_for_style(
                attrs.style.contains(StyleFlags::BOLD),
                attrs.style.contains(StyleFlags::ITALIC),
            );

            if let Some(cached) = self.atlas.get_glyph(
                ctx,
                &mut self.font_system,
                character,
                font_key,
                self.font_size,
            ) {
                if cached.width > 0 && cached.height > 0 {
                    let atlas_size = self.atlas.atlas_size() as f32;
                    let uv_x = cached.atlas_x as f32 / atlas_size;
                    let uv_y = cached.atlas_y as f32 / atlas_size;
                    let uv_w = cached.width as f32 / atlas_size;
                    let uv_h = cached.height as f32 / atlas_size;

                    let glyph_x = x + cached.bearing_x as f32;
                    let glyph_y = y
                        + (self.cell_height
                            - self.font_system.descent().abs()
                            - cached.bearing_y as f32);

                    self.batcher.push_glyph(
                        glyph_x,
                        glyph_y,
                        cached.width as f32,
                        cached.height as f32,
                        uv_x,
                        uv_y,
                        uv_w,
                        uv_h,
                        fg,
                        cached.is_colored,
                    );
                }
            }
        }
    }

    #[inline(always)]
    fn push_cell_decorations(&mut self, x: f32, y: f32, attrs: &HighlightAttributes, fg: [f32; 4]) {
        let underline_style = attrs.underline_style();
        let has_strikethrough = attrs.has_strikethrough();

        if underline_style == UnderlineStyle::None && !has_strikethrough {
            return;
        }

        let special_color = attrs
            .special
            .map(|c| u32_to_linear_rgba(c.0 >> 8))
            .unwrap_or(fg);

        let geom = compute_decoration_geometry(
            x,
            y,
            self.cell_width,
            self.cell_height,
            self.font_system.descent(),
            self.font_system.underline_position(),
            self.font_system.underline_thickness(),
            self.font_system.strikeout_position(),
            self.font_system.strikeout_thickness(),
            underline_style,
            has_strikethrough,
        );

        let underline_count = match underline_style {
            UnderlineStyle::None => 0,
            UnderlineStyle::Double => 2,
            _ => 1,
        };

        for (i, line) in geom.lines.iter().enumerate() {
            let color = if i < underline_count {
                special_color
            } else {
                fg
            };
            self.batcher
                .push_decoration(line.x, line.y, line.width, line.height, color);
        }
    }

    #[inline(always)]
    fn resolve_colors(
        &self,
        attrs: &HighlightAttributes,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
    ) -> ([f32; 4], [f32; 4]) {
        let mut bg = attrs
            .background
            .map(|c| u32_to_linear_rgba(c.0 >> 8))
            .unwrap_or(default_bg);
        let mut fg = attrs
            .foreground
            .map(|c| u32_to_linear_rgba(c.0 >> 8))
            .unwrap_or(default_fg);

        if attrs.style.contains(StyleFlags::REVERSE) {
            std::mem::swap(&mut bg, &mut fg);
        }

        (bg, fg)
    }

    pub fn batcher(&self) -> &RenderBatcher {
        &self.batcher
    }

    fn prepare_cursor(
        &mut self,
        ctx: &GpuContext,
        state: &EditorState,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
    ) {
        let cursor = &state.cursor;
        if !cursor.visible {
            return;
        }

        // Only draw cursor on the main grid (ID 1).
        if cursor.grid != 1 {
            return;
        }

        let mode = state.current_mode();
        let grid = state.main_grid();

        if cursor.row >= grid.height() || cursor.col >= grid.width() {
            return;
        }

        let geom = compute_cursor_geometry(
            mode.cursor_shape,
            cursor.row,
            cursor.col,
            self.cell_width,
            self.cell_height,
            mode.cell_percentage,
        );

        let cursor_color = if mode.attr_id > 0 {
            if let Some(fg) = state.highlights.get(mode.attr_id).foreground {
                u32_to_linear_rgba(fg.0 >> 8)
            } else {
                default_fg
            }
        } else {
            default_fg
        };

        self.batcher
            .push_background(geom.x, geom.y, geom.width, geom.height, cursor_color);

        // Block cursor: render character with inverted colors
        if mode.cursor_shape == CursorShape::Block {
            let cell = grid.get(cursor.row, cursor.col);
            let cell_attrs = cell.map(|c| state.highlights.get(c.highlight_id));

            if let Some(c) = cell {
                // Reuse existing push_cell_glyph logic but force colors
                // We manually construct a "fake" inverted style
                if !c.is_empty() && !c.is_wide_spacer() {
                    let text_color = cell_attrs
                        .and_then(|a| a.background)
                        .map(|c| u32_to_linear_rgba(c.0 >> 8))
                        .unwrap_or(default_bg);

                    let attrs = cell_attrs.unwrap_or_else(|| state.highlights.get(0));

                    self.push_cell_glyph(ctx, geom.x, geom.y, c, attrs, text_color);
                }
            }
        }
    }
}

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

#[derive(Debug, thiserror::Error)]
pub enum GridRendererError {
    #[error("Font error: {0}")]
    Font(#[from] super::font::FontError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_geometry_block() {
        let geom = compute_cursor_geometry(CursorShape::Block, 5, 10, 10.0, 20.0, 0);
        assert_eq!(geom.x, 100.0); // col 10 * cell_width 10
        assert_eq!(geom.y, 100.0); // row 5 * cell_height 20
        assert_eq!(geom.width, 10.0);
        assert_eq!(geom.height, 20.0);
    }

    #[test]
    fn test_cursor_geometry_vertical_default() {
        let geom = compute_cursor_geometry(CursorShape::Vertical, 0, 0, 10.0, 20.0, 0);
        assert_eq!(geom.x, 0.0);
        assert_eq!(geom.y, 0.0);
        assert_eq!(geom.width, 2.5); // 25% of 10.0
        assert_eq!(geom.height, 20.0);
    }

    #[test]
    fn test_cursor_geometry_vertical_custom_percentage() {
        let geom = compute_cursor_geometry(CursorShape::Vertical, 0, 0, 10.0, 20.0, 50);
        assert_eq!(geom.width, 5.0); // 50% of 10.0
    }

    #[test]
    fn test_cursor_geometry_horizontal_default() {
        let geom = compute_cursor_geometry(CursorShape::Horizontal, 0, 0, 10.0, 20.0, 0);
        assert_eq!(geom.x, 0.0);
        assert_eq!(geom.y, 15.0); // 20 - 5 (25% of 20)
        assert_eq!(geom.width, 10.0);
        assert_eq!(geom.height, 5.0); // 25% of 20.0
    }

    #[test]
    fn test_cursor_geometry_horizontal_custom_percentage() {
        let geom = compute_cursor_geometry(CursorShape::Horizontal, 2, 3, 10.0, 20.0, 10);
        assert_eq!(geom.x, 30.0); // col 3 * 10
        assert_eq!(geom.y, 58.0); // row 2 * 20 + 20 - 2 (bar at bottom)
        assert_eq!(geom.height, 2.0); // 10% of 20.0
    }

    #[test]
    fn test_cursor_geometry_minimum_size() {
        // Even with tiny percentage, minimum should be 1.0
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
