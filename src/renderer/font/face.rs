use objc2_core_foundation::{CFRange, CFRetained, CFString, CGFloat, CGPoint, CGRect};
use objc2_core_graphics::{CGBitmapInfo, CGColorSpace, CGGlyph};
use objc2_core_text::{CTFont, CTFontOrientation, CTFontSymbolicTraits};

use std::ptr::{self, NonNull};

use crate::config::FontSettings;

/// Rasterized glyph with positioning data.
#[derive(Clone)]
pub struct RasterizedGlyph {
    #[allow(dead_code)]
    pub character: char,
    pub width: u32,
    pub height: u32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub buffer: GlyphBuffer,
}

#[derive(Clone)]
pub enum GlyphBuffer {
    Rgb(Vec<u8>),
    Rgba(Vec<u8>),
}

impl GlyphBuffer {
    pub fn is_colored(&self) -> bool {
        matches!(self, GlyphBuffer::Rgba(_))
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            GlyphBuffer::Rgb(b) | GlyphBuffer::Rgba(b) => b,
        }
    }
}

/// Font configuration with fallback chain.
pub struct FontConfig {
    pub family: String,
    pub size_pt: f32,
    pub scale_factor: f32,
}

impl FontConfig {
    pub fn new(settings: &FontSettings, scale_factor: f64) -> Self {
        Self {
            family: settings.family.clone().unwrap_or_else(default_font_family),
            size_pt: settings.size.unwrap_or(14.0),
            scale_factor: scale_factor as f32,
        }
    }

    #[allow(dead_code)]
    pub fn scaled_size(&self) -> f32 {
        self.size_pt * self.scale_factor
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size_pt: 14.0,
            scale_factor: 1.0,
        }
    }
}

fn default_font_family() -> String {
    "Menlo".to_string()
}

pub struct HbFontWrapper {
    ptr: *mut harfbuzz_sys::hb_font_t,
}

mod hb_coretext_ffi {
    use std::ffi::c_void;

    extern "C" {
        pub fn hb_coretext_font_create(ct_font: *const c_void) -> *mut harfbuzz_sys::hb_font_t;
    }
}

impl HbFontWrapper {
    pub fn from_ct_font(ct_font: &CTFont, size_px: f32) -> Option<Self> {
        let ct_font_ptr = ct_font as *const CTFont as *const std::ffi::c_void;
        let hb_font = unsafe { hb_coretext_ffi::hb_coretext_font_create(ct_font_ptr) };
        if hb_font.is_null() {
            return None;
        }
        let scale = (size_px * 64.0) as i32;
        unsafe {
            harfbuzz_sys::hb_font_set_scale(hb_font, scale, scale);
        }
        Some(Self { ptr: hb_font })
    }

    #[allow(dead_code)]
    pub fn as_ptr(&self) -> *mut harfbuzz_sys::hb_font_t {
        self.ptr
    }
}

impl Drop for HbFontWrapper {
    fn drop(&mut self) {
        unsafe {
            harfbuzz_sys::hb_font_destroy(self.ptr);
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FaceMetrics {
    pub cell_width: f32,
    pub cell_height: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub underline_position: f32,
    pub underline_thickness: f32,
    pub strikeout_position: f32,
    pub strikeout_thickness: f32,
}

impl Default for FaceMetrics {
    fn default() -> Self {
        Self {
            cell_width: 8.0,
            cell_height: 16.0,
            ascent: 12.0,
            descent: 4.0,
            line_gap: 0.0,
            underline_position: 2.0,
            underline_thickness: 1.0,
            strikeout_position: 6.0,
            strikeout_thickness: 1.0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FaceError {
    #[error("Failed to create font with name: {0}")]
    FontNotFound(String),
    #[error("Failed to create graphics context")]
    ContextCreationFailed,
    #[error("Failed to create HarfBuzz face")]
    HarfBuzzFaceCreation,
    #[error("Failed to copy font table")]
    TableCopyFailed,
}

pub struct Face {
    ct_font: CFRetained<CTFont>,
    hb_font: HbFontWrapper,
    metrics: FaceMetrics,
    size_px: f32,
    has_color: bool,
}

impl Clone for Face {
    fn clone(&self) -> Self {
        // Clone the CTFont and create a new HarfBuzz font from it.
        let ct_font = self.ct_font.clone();
        let hb_font = HbFontWrapper::from_ct_font(&ct_font, self.size_px)
            .expect("Failed to create HarfBuzz font for cloned Face");
        Self {
            ct_font,
            hb_font,
            metrics: self.metrics,
            size_px: self.size_px,
            has_color: self.has_color,
        }
    }
}

impl Face {
    pub fn new(name: &str, size_pt: f32, dpi: f32) -> Result<Self, FaceError> {
        let cf_name = CFString::from_str(name);
        let size_px = size_pt * dpi / 72.0;

        let ct_font = unsafe { CTFont::with_name(&cf_name, size_px as CGFloat, ptr::null()) };

        Self::from_ct_font(ct_font, size_px)
    }

    pub fn from_ct_font(ct_font: CFRetained<CTFont>, size_px: f32) -> Result<Self, FaceError> {
        let hb_font = HbFontWrapper::from_ct_font(&ct_font, size_px)
            .ok_or(FaceError::HarfBuzzFaceCreation)?;
        let metrics = Self::compute_metrics(&ct_font, size_px);

        let traits = unsafe { ct_font.symbolic_traits() };
        let has_color = traits.contains(CTFontSymbolicTraits::TraitColorGlyphs);

        Ok(Self {
            ct_font,
            hb_font,
            metrics,
            size_px,
            has_color,
        })
    }

    fn compute_metrics(ct_font: &CTFont, _size_px: f32) -> FaceMetrics {
        let ascent = unsafe { ct_font.ascent() } as f32;
        let descent = unsafe { ct_font.descent() } as f32;
        let leading = unsafe { ct_font.leading() } as f32;

        let cell_height = ascent + descent + leading;

        let cell_width = Self::measure_advance(ct_font, 'M');

        let underline_position = unsafe { ct_font.underline_position() } as f32;
        let underline_thickness = (unsafe { ct_font.underline_thickness() } as f32).max(1.0);

        let strikeout_position = ascent / 3.0;
        let strikeout_thickness = underline_thickness;

        FaceMetrics {
            cell_width,
            cell_height,
            ascent,
            descent,
            line_gap: leading,
            underline_position,
            underline_thickness,
            strikeout_position,
            strikeout_thickness,
        }
    }

    fn measure_advance(ct_font: &CTFont, ch: char) -> f32 {
        let mut unichars = [0u16; 2];
        let len = ch.encode_utf16(&mut unichars).len();

        let mut glyphs = [0 as CGGlyph; 2];
        unsafe {
            ct_font.glyphs_for_characters(
                NonNull::new_unchecked(unichars.as_mut_ptr()),
                NonNull::new_unchecked(glyphs.as_mut_ptr()),
                len as isize,
            );
        }

        if glyphs[0] == 0 {
            return 8.0;
        }

        let advance = unsafe {
            ct_font.advances_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::new_unchecked(glyphs.as_mut_ptr()),
                ptr::null_mut(),
                1,
            )
        };

        advance as f32
    }

    pub fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    pub fn size_px(&self) -> f32 {
        self.size_px
    }

    #[allow(dead_code)]
    pub fn has_color(&self) -> bool {
        self.has_color
    }

    pub fn hb_font(&self) -> &HbFontWrapper {
        &self.hb_font
    }

    pub fn family_name(&self) -> Option<String> {
        let name = unsafe { self.ct_font.family_name() };
        Some(name.to_string())
    }

    pub fn ct_font(&self) -> &CFRetained<CTFont> {
        &self.ct_font
    }

    pub fn glyph_index(&self, codepoint: u32) -> Option<u32> {
        let ch = char::from_u32(codepoint)?;
        let mut unichars = [0u16; 2];
        let len = ch.encode_utf16(&mut unichars).len();

        let mut glyphs = [0 as CGGlyph; 2];
        let success = unsafe {
            self.ct_font.glyphs_for_characters(
                NonNull::new_unchecked(unichars.as_mut_ptr()),
                NonNull::new_unchecked(glyphs.as_mut_ptr()),
                len as isize,
            )
        };

        if success && glyphs[0] != 0 {
            Some(glyphs[0] as u32)
        } else {
            None
        }
    }

    pub fn has_codepoint(&self, codepoint: u32) -> bool {
        self.glyph_index(codepoint).is_some()
    }

    pub fn render_glyph(&self, glyph_id: u32) -> Result<RasterizedGlyph, FaceError> {
        let glyph = glyph_id as CGGlyph;
        let mut glyphs = [glyph];

        let rect = unsafe {
            self.ct_font.bounding_rects_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::new_unchecked(glyphs.as_mut_ptr()),
                ptr::null_mut(),
                1,
            )
        };

        // If the bounding rect is too small, return an empty glyph.
        if rect.size.width < 0.25 || rect.size.height < 0.25 {
            return Ok(RasterizedGlyph {
                character: '\0',
                width: 0,
                height: 0,
                bearing_x: 0,
                bearing_y: 0,
                buffer: GlyphBuffer::Rgba(Vec::new()),
            });
        }

        // Calculate integer pixel bearings using floor() consistently.
        // These represent the whole-pixel offset from the origin to the glyph.
        let bearing_x = rect.origin.x.floor() as i32;
        let bearing_y = (rect.origin.y + rect.size.height).floor() as i32;

        // Calculate the fractional part of the position. We will bake this
        // sub-pixel offset into the rasterized glyph by translating the
        // drawing context before rendering. This ensures consistent visual
        // positioning when glyphs are placed at integer pixel coordinates.
        let frac_x = rect.origin.x - rect.origin.x.floor();
        let frac_y = rect.origin.y - rect.origin.y.floor();

        // Expand the canvas to account for the fractional offset.
        // The glyph may extend slightly beyond the original bounding box
        // dimensions when rendered at the sub-pixel offset.
        let width = (rect.size.width + frac_x).ceil() as usize;
        let height = (rect.size.height + frac_y).ceil() as usize;

        if width == 0 || height == 0 {
            return Ok(RasterizedGlyph {
                character: '\0',
                width: 0,
                height: 0,
                bearing_x: 0,
                bearing_y: 0,
                buffer: GlyphBuffer::Rgba(Vec::new()),
            });
        }

        let (buffer, is_color) =
            self.render_to_buffer_subpixel(glyph, rect, width, height, frac_x, frac_y)?;

        Ok(RasterizedGlyph {
            character: '\0',
            width: width as u32,
            height: height as u32,
            bearing_x,
            bearing_y,
            buffer: if is_color {
                GlyphBuffer::Rgba(buffer)
            } else {
                Self::convert_gray_to_rgb(buffer, width, height)
            },
        })
    }

    /// Render a glyph to a buffer with sub-pixel positioning.
    ///
    /// The `frac_x` and `frac_y` parameters specify the fractional pixel offset
    /// to apply during rasterization. This offset is baked into the rendered bitmap
    /// so that when the glyph is positioned at integer pixel coordinates, it appears
    /// at the correct visual position with sub-pixel precision.
    fn render_to_buffer_subpixel(
        &self,
        glyph: CGGlyph,
        rect: CGRect,
        width: usize,
        height: usize,
        frac_x: f64,
        frac_y: f64,
    ) -> Result<(Vec<u8>, bool), FaceError> {
        let is_color = self.has_color;
        let bytes_per_pixel = if is_color { 4 } else { 1 };
        let bytes_per_row = width * bytes_per_pixel;

        let mut buffer = vec![0u8; height * bytes_per_row];

        let color_space = if is_color {
            unsafe { CGColorSpace::with_name(Some(objc2_core_graphics::kCGColorSpaceSRGB)) }
        } else {
            unsafe { CGColorSpace::with_name(Some(objc2_core_graphics::kCGColorSpaceLinearGray)) }
        };

        let color_space = color_space.ok_or(FaceError::ContextCreationFailed)?;

        #[allow(deprecated)]
        let bitmap_info = if is_color {
            CGBitmapInfo::ByteOrder32Little.0 | 1 // kCGImageAlphaPremultipliedFirst
        } else {
            0 // kCGImageAlphaOnly
        };

        let context = unsafe {
            objc2_core_graphics::CGBitmapContextCreate(
                buffer.as_mut_ptr() as *mut _,
                width,
                height,
                8,
                bytes_per_row,
                Some(&color_space),
                bitmap_info,
            )
        };

        let context = context.ok_or(FaceError::ContextCreationFailed)?;

        use objc2_core_graphics::CGContext;
        CGContext::set_allows_antialiasing(Some(&context), true);
        CGContext::set_should_antialias(Some(&context), true);
        CGContext::set_should_smooth_fonts(Some(&context), true);

        // Enable sub-pixel positioning for precise glyph placement.
        CGContext::set_allows_font_subpixel_positioning(Some(&context), true);
        CGContext::set_should_subpixel_position_fonts(Some(&context), true);

        // Disable sub-pixel quantization to maintain precise positioning.
        CGContext::set_allows_font_subpixel_quantization(Some(&context), false);
        CGContext::set_should_subpixel_quantize_fonts(Some(&context), false);

        if is_color {
            CGContext::set_rgb_fill_color(Some(&context), 1.0, 1.0, 1.0, 1.0);
        } else {
            CGContext::set_gray_fill_color(Some(&context), 1.0, 1.0);
        }

        // Apply the fractional offset via context translation.
        // This bakes the sub-pixel position into the rasterized glyph,
        // ensuring consistent visual positioning when placed at integer coordinates.
        CGContext::translate_ctm(Some(&context), frac_x, frac_y);

        // Draw the glyph at the negative of the bounding box origin.
        // Combined with the fractional translation above, this places the glyph
        // so that its visual position is correct when the bitmap is displayed
        // at the integer bearing coordinates.
        let positions = [CGPoint::new(-rect.origin.x, -rect.origin.y)];
        let glyphs = [glyph];

        unsafe {
            self.ct_font.draw_glyphs(
                NonNull::new_unchecked(glyphs.as_ptr() as *mut _),
                NonNull::new_unchecked(positions.as_ptr() as *mut _),
                1,
                &context,
            );
        }

        Ok((buffer, is_color))
    }

    fn convert_gray_to_rgb(gray_buffer: Vec<u8>, width: usize, height: usize) -> GlyphBuffer {
        let mut rgb = Vec::with_capacity(width * height * 3);
        for byte in gray_buffer {
            rgb.push(byte);
            rgb.push(byte);
            rgb.push(byte);
        }
        GlyphBuffer::Rgb(rgb)
    }

    #[allow(dead_code)]
    pub fn create_for_string(&self, text: &str) -> CFRetained<CTFont> {
        let cf_string = CFString::from_str(text);
        let range = CFRange::new(0, text.chars().count() as isize);

        unsafe { self.ct_font.for_string(&cf_string, range) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_creation() {
        let face = Face::new("Menlo", 14.0, 72.0);
        assert!(face.is_ok(), "Should create face from system font");
    }

    #[test]
    fn test_face_metrics() {
        let face = Face::new("Menlo", 14.0, 72.0).unwrap();
        let metrics = face.metrics();

        assert!(metrics.cell_width > 0.0, "Cell width should be positive");
        assert!(metrics.cell_height > 0.0, "Cell height should be positive");
        assert!(metrics.ascent > 0.0, "Ascent should be positive");
    }

    #[test]
    fn test_glyph_index() {
        let face = Face::new("Menlo", 14.0, 72.0).unwrap();

        assert!(
            face.glyph_index('A' as u32).is_some(),
            "Should find glyph for 'A'"
        );
        assert!(face.has_codepoint('Z' as u32), "Should have codepoint 'Z'");
    }

    #[test]
    fn test_render_glyph() {
        let face = Face::new("Menlo", 14.0, 72.0).unwrap();
        let glyph_id = face.glyph_index('A' as u32).unwrap();

        let result = face.render_glyph(glyph_id);
        assert!(result.is_ok(), "Should render glyph successfully");

        let glyph = result.unwrap();
        assert!(glyph.width > 0, "Glyph width should be positive");
        assert!(glyph.height > 0, "Glyph height should be positive");
    }

    #[test]
    fn test_face_metrics_default() {
        let metrics = FaceMetrics::default();
        assert!(metrics.cell_width > 0.0);
        assert!(metrics.cell_height > 0.0);
    }
}
