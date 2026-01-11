//! Windows font face implementation using DirectWrite.
//!
//! TODO: Implement using:
//! - `dwrote` crate for font loading and glyph rasterization
//! - `hb_directwrite_font_create` or `hb_ft_font_create` for HarfBuzz integration

use super::{FaceError, FaceMetrics, GlyphBuffer, RasterizedGlyph};

pub struct HbFontWrapper {
    #[allow(dead_code)]
    ptr: *mut harfbuzz_sys::hb_font_t,
}

impl HbFontWrapper {
    #[allow(dead_code)]
    pub fn as_ptr(&self) -> *mut harfbuzz_sys::hb_font_t {
        self.ptr
    }
}

impl Drop for HbFontWrapper {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                harfbuzz_sys::hb_font_destroy(self.ptr);
            }
        }
    }
}

pub struct Face {
    metrics: FaceMetrics,
    size_px: f32,
    hb_font: HbFontWrapper,
}

impl Clone for Face {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics,
            size_px: self.size_px,
            hb_font: HbFontWrapper {
                ptr: std::ptr::null_mut(),
            },
        }
    }
}

impl Face {
    pub fn new(_name: &str, size_pt: f32, dpi: f32) -> Result<Self, FaceError> {
        let size_px = size_pt * dpi / 72.0;

        // TODO: Use DirectWrite to load font
        Err(FaceError::NotImplemented)
    }

    pub fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    pub fn size_px(&self) -> f32 {
        self.size_px
    }

    #[allow(dead_code)]
    pub fn has_color(&self) -> bool {
        false
    }

    pub fn hb_font(&self) -> &HbFontWrapper {
        &self.hb_font
    }

    #[allow(dead_code)]
    pub fn family_name(&self) -> Option<String> {
        None
    }

    pub fn glyph_index(&self, _codepoint: u32) -> Option<u32> {
        None
    }

    pub fn has_codepoint(&self, codepoint: u32) -> bool {
        self.glyph_index(codepoint).is_some()
    }

    pub fn render_glyph(&self, _glyph_id: u32) -> Result<RasterizedGlyph, FaceError> {
        Ok(RasterizedGlyph {
            character: '\0',
            width: 0,
            height: 0,
            bearing_x: 0,
            bearing_y: 0,
            buffer: GlyphBuffer::Rgba(Vec::new()),
        })
    }
}
