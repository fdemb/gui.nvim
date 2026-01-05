use crossfont::Size;

use super::atlas::GlyphAtlas;
use super::batch::RenderBatcher;
use super::font::FontSystem;
use super::GpuContext;
use crate::editor::{CursorShape, EditorState, StyleFlags};

pub struct GridRenderer {
    batcher: RenderBatcher,
    atlas: GlyphAtlas,
    font_system: FontSystem,
    cell_width: f32,
    cell_height: f32,
    font_size: Size,
}

impl GridRenderer {
    pub fn new(ctx: &GpuContext) -> Result<Self, GridRendererError> {
        let font_config = super::font::FontConfig::default();
        let mut font_system = FontSystem::new(&font_config)?;

        let cell_width = font_system.cell_width();
        let cell_height = font_system.cell_height();
        let font_size = Size::new(font_config.size_pt);

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

    pub fn font_system(&self) -> &FontSystem {
        &self.font_system
    }

    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    /// Prepare render batches from editor state, including cursor.
    /// This is the main entry point for preparing a frame for rendering.
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

    /// Prepare grid cells for rendering (backgrounds and glyphs).
    fn prepare_grid_cells(
        &mut self,
        ctx: &GpuContext,
        state: &EditorState,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
    ) {
        let grid = state.main_grid();
        let highlights = &state.highlights;

        for row in 0..grid.height() {
            for col in 0..grid.width() {
                if let Some(cell) = grid.get(row, col) {
                    let x = col as f32 * self.cell_width;
                    let y = row as f32 * self.cell_height;

                    // Get highlight attributes
                    let attrs = highlights.get(cell.highlight_id);

                    let (bg, fg) = self.resolve_colors(attrs, default_bg, default_fg);

                    // Push background if non-default
                    if bg != default_bg {
                        self.batcher
                            .push_background(x, y, self.cell_width, self.cell_height, bg);
                    }

                    // Push glyph if cell has content
                    if !cell.is_empty() && !cell.is_wide_spacer() {
                        // Get first character from text
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

                                    // Position glyph using bearing
                                    let glyph_x = x + cached.bearing_x as f32;
                                    let glyph_y = y
                                        + (self.cell_height
                                            - self.font_system.descent()
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
                }
            }
        }
    }

    fn resolve_colors(
        &self,
        attrs: &crate::editor::HighlightAttributes,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
    ) -> ([f32; 4], [f32; 4]) {
        let mut bg = attrs
            .background
            .map(|c| color_to_rgba(c.0 >> 8))
            .unwrap_or(default_bg);
        let mut fg = attrs
            .foreground
            .map(|c| color_to_rgba(c.0 >> 8))
            .unwrap_or(default_fg);

        if attrs.style.contains(StyleFlags::REVERSE) {
            std::mem::swap(&mut bg, &mut fg);
        }

        (bg, fg)
    }

    pub fn batcher(&self) -> &RenderBatcher {
        &self.batcher
    }

    /// Prepare the cursor for rendering (adds to batch).
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

        let mode = state.current_mode();
        let grid = state.main_grid();

        // Verify cursor is within grid bounds
        if cursor.row >= grid.height() || cursor.col >= grid.width() {
            return;
        }

        let x = cursor.col as f32 * self.cell_width;
        let y = cursor.row as f32 * self.cell_height;

        // Get cursor color from mode's attr_id or use default
        let cursor_color = if mode.attr_id > 0 {
            if let Some(fg) = state.highlights.get(mode.attr_id).foreground {
                color_to_rgba(fg.0 >> 8)
            } else {
                default_fg
            }
        } else {
            default_fg
        };

        // Get the cell under cursor for block cursor rendering
        let cell = grid.get(cursor.row, cursor.col);
        let cell_attrs = cell.map(|c| state.highlights.get(c.highlight_id));

        // Determine percentage for bar cursor thickness
        let percentage = if mode.cell_percentage > 0 {
            mode.cell_percentage.min(100) as f32 / 100.0
        } else {
            // Default percentages if not specified
            match mode.cursor_shape {
                CursorShape::Vertical => 0.25,   // 25% cell width
                CursorShape::Horizontal => 0.25, // 25% cell height
                CursorShape::Block => 1.0,
            }
        };

        match mode.cursor_shape {
            CursorShape::Block => {
                // Block cursor: render inverted cell
                // First, render the cursor background
                self.batcher
                    .push_background(x, y, self.cell_width, self.cell_height, cursor_color);

                // Then render the character with inverted colors if cell has content
                if let Some(c) = cell {
                    if !c.is_empty() && !c.is_wide_spacer() {
                        if let Some(character) = c.text.chars().next() {
                            // Use background as text color for inversion
                            let text_color = cell_attrs
                                .and_then(|a| a.background)
                                .map(|c| color_to_rgba(c.0 >> 8))
                                .unwrap_or(default_bg);

                            let attrs = cell_attrs.unwrap_or_else(|| state.highlights.get(0));
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
                                            - self.font_system.descent()
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
                                        text_color,
                                        cached.is_colored,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            CursorShape::Vertical => {
                // Vertical bar on left edge of cell
                let bar_width = (self.cell_width * percentage).max(1.0);
                self.batcher
                    .push_background(x, y, bar_width, self.cell_height, cursor_color);
            }

            CursorShape::Horizontal => {
                // Horizontal bar at bottom of cell
                let bar_height = (self.cell_height * percentage).max(1.0);
                let bar_y = y + self.cell_height - bar_height;
                self.batcher
                    .push_background(x, bar_y, self.cell_width, bar_height, cursor_color);
            }
        }
    }
}

fn color_to_rgba(color: u32) -> [f32; 4] {
    let r = ((color >> 16) & 0xFF) as f32 / 255.0;
    let g = ((color >> 8) & 0xFF) as f32 / 255.0;
    let b = (color & 0xFF) as f32 / 255.0;
    [r, g, b, 1.0]
}

/// Cursor geometry for rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Computes the cursor geometry based on shape, position, and cell dimensions.
/// Returns the bounding box for the cursor.
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
    fn test_color_to_rgba() {
        let white = color_to_rgba(0xFFFFFF);
        assert!((white[0] - 1.0).abs() < 0.001);
        assert!((white[1] - 1.0).abs() < 0.001);
        assert!((white[2] - 1.0).abs() < 0.001);

        let black = color_to_rgba(0x000000);
        assert!((black[0]).abs() < 0.001);
        assert!((black[1]).abs() < 0.001);
        assert!((black[2]).abs() < 0.001);

        let red = color_to_rgba(0xFF0000);
        assert!((red[0] - 1.0).abs() < 0.001);
        assert!((red[1]).abs() < 0.001);
        assert!((red[2]).abs() < 0.001);
    }

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
}
