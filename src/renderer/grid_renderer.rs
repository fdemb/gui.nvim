use super::atlas::GlyphAtlas;
use super::batch::RenderBatcher;
use super::color::u32_to_linear_rgba;
use super::font::{
    Collection, FontConfig, GlyphCacheKey, RunIterator, ShapedGlyph, Shaper, ShapingCache,
    ShapingCacheKey, Style, TextRun,
};
use super::geometry::{compute_cursor_geometry, compute_decoration_geometry};
use super::GpuContext;
use crate::config::FontSettings;
use crate::editor::{CursorShape, EditorState, HighlightAttributes, StyleFlags, UnderlineStyle};
use std::time::{Duration, Instant};

/// Statistics collected during prepare() for performance analysis.
#[derive(Debug, Default, Clone, Copy)]
pub struct PrepareStats {
    pub cells_processed: usize,
    pub runs_processed: usize,
    pub shape_calls: usize,
    pub glyphs_shaped: usize,
    pub glyph_cache_hits: usize,
    pub glyph_cache_misses: usize,
    // Shaping cache stats
    pub shaping_cache_hits: usize,
    pub shaping_cache_misses: usize,
    // Timing breakdown
    pub time_backgrounds: Duration,
    pub time_shaping: Duration,
    pub time_glyph_lookup: Duration,
    pub time_batching: Duration,
}

pub struct GridRenderer {
    batcher: RenderBatcher,
    atlas: GlyphAtlas,
    collection: Collection,
    shaper: Shaper,
    /// Cache for shaped text runs to avoid redundant HarfBuzz calls.
    shaping_cache: ShapingCache,
    cell_width: f32,
    cell_height: f32,
    /// Distance from the top of the cell to the baseline.
    /// Computed as: ascent + (line_gap / 2) to center text vertically.
    baseline_offset: f32,
}

impl GridRenderer {
    pub fn new(
        ctx: &GpuContext,
        font_settings: &FontSettings,
        scale_factor: f64,
    ) -> Result<Self, GridRendererError> {
        let font_config = FontConfig::new(font_settings, scale_factor);
        let dpi = 72.0 * scale_factor as f32;
        let mut collection = Collection::new(&font_config.family, font_config.size_pt, dpi)?;
        let shaper = Shaper::new();

        let metrics = collection.metrics();
        let cell_width = metrics.cell_width;
        let cell_height = metrics.cell_height;
        // Compute baseline offset from top of cell.
        // We split the line_gap in half to center text vertically within the cell,
        // matching Ghostty's approach.
        let baseline_offset = metrics.ascent + (metrics.line_gap / 2.0);

        let mut atlas = GlyphAtlas::new(ctx);
        atlas.prepopulate_ascii(ctx, &mut collection, Style::Regular);

        let batcher = RenderBatcher::new(ctx);

        Ok(Self {
            batcher,
            atlas,
            collection,
            shaper,
            shaping_cache: ShapingCache::new(),
            cell_width,
            cell_height,
            baseline_offset,
        })
    }

    pub fn update_font(
        &mut self,
        ctx: &GpuContext,
        font_settings: &FontSettings,
        scale_factor: f64,
    ) -> Result<(), GridRendererError> {
        let font_config = FontConfig::new(font_settings, scale_factor);
        let dpi = 72.0 * scale_factor as f32;
        let mut collection = Collection::new(&font_config.family, font_config.size_pt, dpi)?;
        self.shaper = Shaper::new();

        let metrics = collection.metrics();
        self.cell_width = metrics.cell_width;
        self.cell_height = metrics.cell_height;
        self.baseline_offset = metrics.ascent + (metrics.line_gap / 2.0);

        self.atlas.clear(ctx);
        self.atlas
            .prepopulate_ascii(ctx, &mut collection, Style::Regular);
        self.collection = collection;

        // Clear shaping cache - cached results are invalid with new font
        self.shaping_cache.clear();

        Ok(())
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
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
    ) -> PrepareStats {
        self.batcher.clear();
        let stats = self.prepare_grid_cells(ctx, state, default_bg, default_fg, x_offset, y_offset);
        self.prepare_cursor(ctx, state, default_bg, default_fg, x_offset, y_offset);
        self.batcher.upload(ctx);
        stats
    }

    fn prepare_grid_cells(
        &mut self,
        ctx: &GpuContext,
        state: &EditorState,
        default_bg: [f32; 4],
        default_fg: [f32; 4],
        x_offset: f32,
        y_offset: f32,
    ) -> PrepareStats {
        let mut stats = PrepareStats::default();
        let grid = state.main_grid();
        let highlights = &state.highlights;

        for (row_idx, row_cells) in grid.rows().enumerate() {
            let y = row_idx as f32 * self.cell_height + y_offset;

            // First pass: backgrounds and decorations (cell by cell)
            let bg_start = Instant::now();
            let mut last_hl_id = u64::MAX;
            let mut last_bg = default_bg;
            let mut last_fg = default_fg;
            let mut last_attrs = highlights.get(0);

            for (col_idx, cell) in row_cells.iter().enumerate() {
                stats.cells_processed += 1;

                if cell.highlight_id != last_hl_id {
                    last_hl_id = cell.highlight_id;
                    last_attrs = highlights.get(last_hl_id);
                    let (bg, fg) = self.resolve_colors(last_attrs, default_bg, default_fg);
                    last_bg = bg;
                    last_fg = fg;
                }

                let x = col_idx as f32 * self.cell_width + x_offset;
                self.push_cell_background(x, y, last_bg, default_bg);
                self.push_cell_decorations(x, y, last_attrs, last_fg);
            }
            stats.time_backgrounds += bg_start.elapsed();

            // Second pass: text runs with shaping
            for run in RunIterator::new(row_cells, highlights) {
                if run.is_empty() {
                    continue;
                }

                stats.runs_processed += 1;

                let attrs = highlights.get(run.highlight_id);
                let (_, fg) = self.resolve_colors(attrs, default_bg, default_fg);

                // Try to get shaped glyphs from cache
                let cache_key = ShapingCacheKey::new(&run.text, run.style);
                let shape_start = Instant::now();

                // Check cache and clone if found, otherwise shape
                let shaped: Vec<ShapedGlyph> =
                    if let Some(cached) = self.shaping_cache.get(cache_key) {
                        stats.shaping_cache_hits += 1;
                        stats.glyphs_shaped += cached.glyphs.len();
                        cached.glyphs.clone()
                    } else {
                        stats.shaping_cache_misses += 1;
                        stats.shape_calls += 1;

                        let text_run = TextRun {
                            text: &run.text,
                            style: run.style,
                        };

                        let new_shaped = self
                            .shaper
                            .shape_with_collection(&text_run, &mut self.collection);
                        stats.glyphs_shaped += new_shaped.len();

                        // Insert into cache
                        self.shaping_cache.insert(cache_key, new_shaped.clone());

                        new_shaped
                    };
                stats.time_shaping += shape_start.elapsed();

                let run_x = run.start_col as f32 * self.cell_width + x_offset;

                self.push_shaped_run_with_stats(ctx, run_x, y, &shaped, fg, &mut stats);
            }
        }

        stats
    }

    fn push_shaped_run(
        &mut self,
        ctx: &GpuContext,
        run_x: f32,
        y: f32,
        shaped: &[ShapedGlyph],
        fg: [f32; 4],
    ) {
        let mut stats = PrepareStats::default();
        self.push_shaped_run_with_stats(ctx, run_x, y, shaped, fg, &mut stats);
    }

    fn push_shaped_run_with_stats(
        &mut self,
        ctx: &GpuContext,
        run_x: f32,
        y: f32,
        shaped: &[ShapedGlyph],
        fg: [f32; 4],
        stats: &mut PrepareStats,
    ) {
        let mut x = run_x;
        let baseline_y = y + self.baseline_offset;

        for glyph in shaped {
            let key = GlyphCacheKey::new(glyph.glyph_id, glyph.font_index);

            let lookup_start = Instant::now();
            let (cached_opt, was_cache_hit) =
                self.atlas
                    .get_glyph_by_id_with_stats(ctx, &self.collection, key);
            stats.time_glyph_lookup += lookup_start.elapsed();

            if was_cache_hit {
                stats.glyph_cache_hits += 1;
            } else {
                stats.glyph_cache_misses += 1;
            }

            if let Some(cached) = cached_opt {
                if cached.width > 0 && cached.height > 0 {
                    let batch_start = Instant::now();
                    let atlas_size = self.atlas.atlas_size() as f32;
                    let uv_x = cached.atlas_x as f32 / atlas_size;
                    let uv_y = cached.atlas_y as f32 / atlas_size;
                    let uv_w = cached.width as f32 / atlas_size;
                    let uv_h = cached.height as f32 / atlas_size;

                    // Apply HarfBuzz offsets (in 26.6 fixed-point).
                    // Use floating-point division to preserve fractional pixels.
                    let x_offset = glyph.x_offset as f32 / 64.0;
                    let y_offset = glyph.y_offset as f32 / 64.0;

                    let glyph_x = x + x_offset + cached.bearing_x as f32;
                    let glyph_y = baseline_y - y_offset - cached.bearing_y as f32;

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
                    stats.time_batching += batch_start.elapsed();
                }
            }

            // Advance by the shaped x_advance (26.6 fixed-point).
            // Use floating-point division to preserve sub-pixel precision and
            // prevent accumulated drift when rendering long runs of text.
            x += glyph.x_advance as f32 / 64.0;
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

        let metrics = self.collection.metrics();
        let geom = compute_decoration_geometry(
            x,
            y,
            self.cell_width,
            self.cell_height,
            metrics.descent,
            metrics.underline_position,
            metrics.underline_thickness,
            metrics.strikeout_position,
            metrics.strikeout_thickness,
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
                if !c.is_empty() && !c.is_wide_spacer() {
                    let text_color = cell_attrs
                        .and_then(|a| a.background)
                        .map(|c| u32_to_linear_rgba(c.0 >> 8))
                        .unwrap_or(default_bg);

                    let attrs = cell_attrs.unwrap_or_else(|| state.highlights.get(0));
                    let style = Style::from_flags(
                        attrs.style.contains(StyleFlags::BOLD),
                        attrs.style.contains(StyleFlags::ITALIC),
                    );

                    // Use shaped rendering for the cursor character.
                    // Disable ligatures so the standalone glyph matches the cell position.
                    // The cursor background covers the underlying ligature anyway.
                    let text_run = TextRun {
                        text: &c.text,
                        style,
                    };
                    let shaped = self
                        .shaper
                        .shape_without_ligatures(&text_run, &mut self.collection);
                    self.push_shaped_run(ctx, geom.x, geom.y, &shaped, text_color);
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GridRendererError {
    #[error("Face error: {0}")]
    Face(#[from] super::font::FaceError),
}
