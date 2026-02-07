//! Windows font face implementation using DirectWrite.
//!
//! TODO: Implement using:
//! - `directwrite` crate for font loading and glyph rasterization
//! - `IDWriteFontFallback` for fallback discovery
//! - `hb_font_create` or `hb_directwrite_font_create` for HarfBuzz integration

use crate::renderer::font::{FaceError, FaceMetrics, FontFace, HbFontWrapper, RasterizedGlyph};

pub struct Face;

impl Clone for Face {
    fn clone(&self) -> Self {
        Self
    }
}

impl Face {
    pub fn new(_name: &str, size_pt: f32, dpi: f32) -> Result<Self, FaceError> {
        let _ = size_pt;
        let _ = dpi;

        // TODO: Use DirectWrite to find and load font
        Err(FaceError::NotImplemented)
    }

    pub fn from_bytes(_data: &'static [u8], _size_px: f32) -> Result<Self, FaceError> {
        // TODO: Use DirectWrite in-memory font loader
        Err(FaceError::NotImplemented)
    }

    pub fn create_style_variant(
        &self,
        _style: crate::renderer::font::collection::Style,
    ) -> Option<Self> {
        None
    }

    /// Returns the HarfBuzz font handle for text shaping.
    ///
    /// # Panics
    ///
    /// Panics because the Windows font backend is not yet implemented.
    /// This stub exists to satisfy the interface required by `Shaper`.
    pub fn hb_font(&self) -> &HbFontWrapper {
        unimplemented!("Windows HarfBuzz font integration not yet implemented")
    }
}

impl FontFace for Face {
    fn metrics(&self) -> &FaceMetrics {
        static DEFAULT_METRICS: FaceMetrics = FaceMetrics::default();
        &DEFAULT_METRICS
    }

    fn size_px(&self) -> f32 {
        0.0
    }

    fn has_codepoint(&self, _codepoint: u32) -> bool {
        false
    }

    fn glyph_index(&self, _codepoint: u32) -> Option<u32> {
        None
    }

    fn render_glyph(&self, _glyph_id: u32) -> Result<RasterizedGlyph, FaceError> {
        Err(FaceError::NotImplemented)
    }

    fn hb_font(&self) -> &HbFontWrapper {
        self.hb_font()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_creation_not_implemented() {
        let face = Face::new("Consolas", 14.0, 96.0);
        assert!(face.is_err());
        assert_eq!(face.unwrap_err(), FaceError::NotImplemented);
    }

    #[test]
    fn test_from_bytes_not_implemented() {
        let data: &'static [u8] = &[];
        let face = Face::from_bytes(data, 14.0);
        assert!(face.is_err());
        assert_eq!(face.unwrap_err(), FaceError::NotImplemented);
    }
}
