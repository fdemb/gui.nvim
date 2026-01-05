use crossfont::Size;

use super::atlas::GlyphAtlas;
use super::batch::RenderBatcher;
use super::font::FontSystem;
use super::GpuContext;
use crate::editor::{EditorState, StyleFlags};

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

    /// Prepare render batches from editor state.
    pub fn prepare(
        &mut self,
        ctx: &GpuContext,
        state: &EditorState,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
    ) {
        self.batcher.clear();

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

        self.batcher.upload(ctx);
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
}

fn color_to_rgba(color: u32) -> [f32; 4] {
    let r = ((color >> 16) & 0xFF) as f32 / 255.0;
    let g = ((color >> 8) & 0xFF) as f32 / 255.0;
    let b = (color & 0xFF) as f32 / 255.0;
    [r, g, b, 1.0]
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
}
