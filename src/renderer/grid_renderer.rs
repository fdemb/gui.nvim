use crossfont::Size;

use super::atlas::GlyphAtlas;
use super::batch::RenderBatcher;
use super::color::u32_to_linear_rgba;
use super::font::FontSystem;
use super::geometry::{compute_cursor_geometry, compute_decoration_geometry};
use super::GpuContext;
use crate::config::FontSettings;
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
    pub fn new(
        ctx: &GpuContext,
        font_settings: &FontSettings,
        scale_factor: f64,
    ) -> Result<Self, GridRendererError> {
        let font_config = super::font::FontConfig::new(font_settings, scale_factor);
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

    pub fn update_font(
        &mut self,
        ctx: &GpuContext,
        font_settings: &FontSettings,
        scale_factor: f64,
    ) -> Result<(), GridRendererError> {
        let font_config = super::font::FontConfig::new(font_settings, scale_factor);
        let mut font_system = FontSystem::new(&font_config)?;

        let cell_width = font_system.cell_width();
        let cell_height = font_system.cell_height();
        let font_size = Size::new(font_config.scaled_size());

        self.atlas.clear(ctx);
        self.atlas
            .prepopulate_ascii(ctx, &mut font_system, font_size);

        self.font_system = font_system;
        self.cell_width = cell_width;
        self.cell_height = cell_height;
        self.font_size = font_size;

        Ok(())
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
        x_offset: f32,
        y_offset: f32,
    ) {
        self.batcher.clear();
        self.prepare_grid_cells(ctx, state, default_bg, default_fg, x_offset, y_offset);
        self.prepare_cursor(ctx, state, default_bg, default_fg, x_offset, y_offset);
        self.batcher.upload(ctx);
    }

    fn prepare_grid_cells(
        &mut self,
        ctx: &GpuContext,
        state: &EditorState,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
        x_offset: f32,
        y_offset: f32,
    ) {
        let grid = state.main_grid();
        let highlights = &state.highlights;

        for (row_idx, row_cells) in grid.rows().enumerate() {
            let y = row_idx as f32 * self.cell_height + y_offset;

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

                let x = col_idx as f32 * self.cell_width + x_offset;

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
        x_offset: f32,
        y_offset: f32,
    ) {
        let cursor = &state.cursor;
        if !cursor.visible || !cursor.blink_visible {
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

        let mut geom = compute_cursor_geometry(
            mode.cursor_shape,
            cursor.row,
            cursor.col,
            self.cell_width,
            self.cell_height,
            mode.cell_percentage,
        );

        geom.x += x_offset;
        geom.y += y_offset;

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


#[derive(Debug, thiserror::Error)]
pub enum GridRendererError {
    #[error("Font error: {0}")]
    Font(#[from] super::font::FontError),
}

