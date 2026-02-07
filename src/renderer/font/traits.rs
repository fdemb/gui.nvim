use crate::renderer::font::{FaceError, FaceMetrics, HbFontWrapper, RasterizedGlyph};

pub trait FontFace {
    fn metrics(&self) -> &FaceMetrics;
    fn size_px(&self) -> f32;
    fn has_codepoint(&self, codepoint: u32) -> bool;
    fn glyph_index(&self, codepoint: u32) -> Option<u32>;
    fn render_glyph(&self, glyph_id: u32) -> Result<RasterizedGlyph, FaceError>;
    /// Returns the HarfBuzz font handle for text shaping.
    fn hb_font(&self) -> &HbFontWrapper;
}

pub trait SystemFallback<F: FontFace> {
    fn new(base_face: &F, size_px: f32) -> Self;
    fn discover(&self, codepoint: u32) -> Option<F>;
}
